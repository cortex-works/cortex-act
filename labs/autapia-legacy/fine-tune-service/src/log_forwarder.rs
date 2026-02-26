use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{Event, Subscriber};
use tracing_subscriber::Layer;
use serde_json::json;
use reqwest::Client;
use chrono::Utc;

/// HTTP Log Forwarder that sends logs to the centralized logging service
pub struct LogForwarder {
    sender: mpsc::UnboundedSender<LogEntry>,
}

#[derive(Debug, Clone)]
struct LogEntry {
    timestamp: chrono::DateTime<chrono::Utc>,
    level: String,
    service: String,
    message: String,
    metadata: HashMap<String, serde_json::Value>,
    #[allow(dead_code)]
    target: String,
}

impl LogForwarder {
    pub fn new(logging_service_url: String, _service_name: String) -> Self {
        let (sender, mut receiver) = mpsc::unbounded_channel::<LogEntry>();
        
        // Spawn background task to forward logs
        tokio::spawn(async move {
            let client = Client::new();
            let ingest_url = format!("{}/api/v1/ingest", logging_service_url);
            let mut batch = Vec::new();
            let mut last_flush = std::time::Instant::now();
            
            while let Some(log_entry) = receiver.recv().await {
                batch.push(log_entry);
                
                // Flush batch if it's large enough or enough time has passed
                if batch.len() >= 10 || last_flush.elapsed().as_secs() >= 2 {
                    if !batch.is_empty() {
                        Self::send_batch(&client, &ingest_url, &batch).await;
                        batch.clear();
                        last_flush = std::time::Instant::now();
                    }
                }
            }
            
            // Send any remaining logs
            if !batch.is_empty() {
                Self::send_batch(&client, &ingest_url, &batch).await;
            }
        });
        
        Self { sender }
    }
    
    async fn send_batch(client: &Client, url: &str, batch: &[LogEntry]) {
        let logs: Vec<serde_json::Value> = batch.iter().map(|entry| {
            json!({
                "id": uuid::Uuid::new_v4().to_string(),
                "timestamp": entry.timestamp,
                "level": entry.level,
                "service": entry.service,
                "message": entry.message,
                "metadata": entry.metadata,
                "trace_id": entry.metadata.get("trace_id").and_then(|v| v.as_str()),
                "span_id": entry.metadata.get("span_id").and_then(|v| v.as_str()),
                "environment": "development"
            })
        }).collect();
        
        let payload = json!({
            "logs": logs
        });
        
        match client.post(url).json(&payload).send().await {
            Ok(response) => {
                if !response.status().is_success() {
                    eprintln!("Failed to send logs: {}", response.status());
                }
            }
            Err(e) => {
                eprintln!("Failed to send logs: {}", e);
            }
        }
    }
    
    fn forward_log(&self, log_entry: LogEntry) {
        if let Err(_) = self.sender.send(log_entry) {
            eprintln!("Failed to forward log - channel closed");
        }
    }
}

impl<S> Layer<S> for LogForwarder
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let metadata = event.metadata();
        
        // Extract the message from the event
        let mut visitor = LogVisitor::new();
        event.record(&mut visitor);
        
        let log_entry = LogEntry {
            timestamp: Utc::now(),
            level: metadata.level().to_string().to_uppercase(),
            service: "fine-tune-service".to_string(),
            message: visitor.message,
            metadata: visitor.fields,
            target: metadata.target().to_string(),
        };
        
        self.forward_log(log_entry);
    }
}

/// Visitor to extract fields from tracing events
struct LogVisitor {
    message: String,
    fields: HashMap<String, serde_json::Value>,
}

impl LogVisitor {
    fn new() -> Self {
        Self {
            message: String::new(),
            fields: HashMap::new(),
        }
    }
}

impl tracing::field::Visit for LogVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let value_str = format!("{:?}", value);
        
        if field.name() == "message" {
            self.message = value_str.trim_matches('"').to_string();
        } else {
            self.fields.insert(field.name().to_string(), json!(value_str));
        }
    }
    
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else {
            self.fields.insert(field.name().to_string(), json!(value));
        }
    }
    
    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields.insert(field.name().to_string(), json!(value));
    }
    
    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.fields.insert(field.name().to_string(), json!(value));
    }
    
    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields.insert(field.name().to_string(), json!(value));
    }
} 