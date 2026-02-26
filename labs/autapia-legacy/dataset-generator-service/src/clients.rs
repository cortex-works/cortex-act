use crate::error::{DatasetError, Result, RetryOperation, RetryConfig, ResultExt};
use crate::types_minimal::{EmbeddingRequest, EmbeddingResponse, ChatRequest, ChatResponse};
use log::{debug, info, warn};
use reqwest::Client;
use serde_json::json;
use std::time::Duration;

pub struct ServiceClients {
    http_client: Client,
    vector_service_addr: String,
    embedding_service_addr: String,
    chat_service_addr: String,
    retry_operation: RetryOperation,
}

impl ServiceClients {
    pub async fn new(
        vector_service_addr: String,
        embedding_service_addr: String,
        chat_service_addr: String,
    ) -> Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(300)) // 5 minute timeout for long operations
            .build()
            .map_err(|e| DatasetError::service_connection("http_client", e.to_string()))?;

        let clients = Self {
            http_client,
            vector_service_addr,
            embedding_service_addr,
            chat_service_addr,
            retry_operation: RetryOperation::with_defaults(),
        };

        // Test connectivity to all services
        clients.test_connectivity().await
            .with_operation("service_connectivity_test")?;

        Ok(clients)
    }

    async fn test_connectivity(&self) -> Result<()> {
        info!("Testing connectivity to microservices...");

        // Test embedding service
        let embedding_test = self.test_service_health(&self.embedding_service_addr, "embedding-service").await;
        if let Err(e) = embedding_test {
            warn!("Embedding service connectivity test failed: {}", e);
            // Don't fail startup, just warn - service might not be ready yet
        }

        // Test chat service  
        let chat_test = self.test_service_health(&self.chat_service_addr, "chat-service").await;
        if let Err(e) = chat_test {
            warn!("Chat service connectivity test failed: {}", e);
        }

        // Test vector service
        let vector_test = self.test_service_health(&self.vector_service_addr, "vector-service").await;
        if let Err(e) = vector_test {
            warn!("Vector service connectivity test failed: {}", e);
        }

        info!("Service connectivity tests completed");
        Ok(())
    }

    async fn test_service_health(&self, service_addr: &str, service_name: &str) -> Result<()> {
        let health_url = if service_addr.ends_with('/') {
            format!("{}health", service_addr)
        } else {
            format!("{}/health", service_addr)
        };

        let response = self.http_client
            .get(&health_url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    DatasetError::network_timeout(service_name, 5000)
                } else if e.is_connect() {
                    DatasetError::service_connection(service_name, format!("Connection failed: {}", e))
                } else {
                    DatasetError::service_connection(service_name, e.to_string())
                }
            })?;

        if response.status().is_success() {
            debug!("Health check passed for {}", service_name);
            Ok(())
        } else {
            Err(DatasetError::service_connection(
                service_name,
                format!("Health check failed with status: {}", response.status()),
            ))
        }
    }

    pub async fn generate_embedding(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        let operation = || async {
            self.generate_embedding_internal(request.clone()).await
        };

        self.retry_operation
            .execute(operation)
            .await
            .with_operation("generate_embedding")
    }

    async fn generate_embedding_internal(&self, request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        debug!("Generating embedding for text of length: {}", request.text.len());

        let url = format!("{}/embeddings", self.embedding_service_addr);
        let payload = json!({
            "text": request.text,
            "model": request.model,
            "provider": request.provider
        });

        let response = self.http_client
            .post(&url)
            .json(&payload)
            .timeout(Duration::from_secs(60))
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    DatasetError::network_timeout("embedding-service", 60000)
                } else if e.is_connect() {
                    DatasetError::service_connection("embedding-service", format!("Connection failed: {}", e))
                } else {
                    DatasetError::service_connection("embedding-service", e.to_string())
                }
            })?;

        if response.status().is_success() {
            let embedding_response: EmbeddingResponse = response
                .json()
                .await
                .map_err(|e| DatasetError::parsing("json", format!("Failed to parse embedding response: {}", e)))?;
            
            debug!("Successfully generated embedding of dimension: {}", embedding_response.embedding.len());
            Ok(embedding_response)
        } else if response.status().as_u16() == 429 {
            // Rate limit
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(5000); // Default 5 seconds
            
            Err(DatasetError::rate_limit("embedding-service", retry_after * 1000))
        } else {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            
            Err(DatasetError::service_connection(
                "embedding-service",
                format!("Request failed with status {}: {}", status, error_text),
            ))
        }
    }

    pub async fn generate_chat_completion(&self, request: ChatRequest) -> Result<ChatResponse> {
        let operation = || async {
            self.generate_chat_completion_internal(request.clone()).await
        };

        self.retry_operation
            .execute(operation)
            .await
            .with_operation("generate_chat_completion")
    }

    async fn generate_chat_completion_internal(&self, request: ChatRequest) -> Result<ChatResponse> {
        debug!("Generating chat completion for prompt of length: {}", request.prompt.len());

        let url = format!("{}/chat/completions", self.chat_service_addr);
        let payload = json!({
            "prompt": request.prompt,
            "model": request.model,
            "provider": request.provider,
            "temperature": request.temperature,
            "max_tokens": request.max_tokens
        });

        let response = self.http_client
            .post(&url)
            .json(&payload)
            .timeout(Duration::from_secs(180)) // 3 minute timeout for chat completions
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    DatasetError::network_timeout("chat-service", 180000)
                } else if e.is_connect() {
                    DatasetError::service_connection("chat-service", format!("Connection failed: {}", e))
                } else {
                    DatasetError::service_connection("chat-service", e.to_string())
                }
            })?;

        if response.status().is_success() {
            let chat_response: ChatResponse = response
                .json()
                .await
                .map_err(|e| DatasetError::parsing("json", format!("Failed to parse chat response: {}", e)))?;
            
            debug!("Successfully generated chat completion of length: {}", chat_response.response.len());
            Ok(chat_response)
        } else if response.status().as_u16() == 429 {
            // Rate limit
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(10000); // Default 10 seconds for chat
            
            Err(DatasetError::rate_limit("chat-service", retry_after * 1000))
        } else {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            
            Err(DatasetError::service_connection(
                "chat-service",
                format!("Request failed with status {}: {}", status, error_text),
            ))
        }
    }

    pub async fn search_vectors(&self, query: Vec<f32>, collection: &str, limit: usize) -> Result<Vec<serde_json::Value>> {
        let operation = || async {
            self.search_vectors_internal(query.clone(), collection, limit).await
        };

        self.retry_operation
            .execute(operation)
            .await
            .with_operation("search_vectors")
    }

    async fn search_vectors_internal(&self, query: Vec<f32>, collection: &str, limit: usize) -> Result<Vec<serde_json::Value>> {
        debug!("Searching vectors in collection: {} with limit: {}", collection, limit);

        let url = format!("{}/vectors/search", self.vector_service_addr);
        let payload = json!({
            "vector": query,
            "collection": collection,
            "limit": limit
        });

        let response = self.http_client
            .post(&url)
            .json(&payload)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    DatasetError::network_timeout("vector-service", 30000)
                } else if e.is_connect() {
                    DatasetError::service_connection("vector-service", format!("Connection failed: {}", e))
                } else {
                    DatasetError::service_connection("vector-service", e.to_string())
                }
            })?;

        if response.status().is_success() {
            let results: Vec<serde_json::Value> = response
                .json()
                .await
                .map_err(|e| DatasetError::parsing("json", format!("Failed to parse vector search response: {}", e)))?;
            
            debug!("Successfully searched vectors, found {} results", results.len());
            Ok(results)
        } else if response.status().as_u16() == 429 {
            // Rate limit
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(2000); // Default 2 seconds for vector search
            
            Err(DatasetError::rate_limit("vector-service", retry_after * 1000))
        } else {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            
            Err(DatasetError::service_connection(
                "vector-service",
                format!("Vector search failed with status {}: {}", status, error_text),
            ))
        }
    }

    /// Update retry configuration
    pub fn with_retry_config(&mut self, config: RetryConfig) -> &mut Self {
        self.retry_operation = RetryOperation::new(config);
        self
    }

    /// Get current service addresses for debugging
    pub fn get_service_addresses(&self) -> (String, String, String) {
        (
            self.vector_service_addr.clone(),
            self.embedding_service_addr.clone(),
            self.chat_service_addr.clone(),
        )
    }

    /// Validate all service connections
    pub async fn validate_connections(&self) -> Result<()> {
        info!("Validating all service connections...");
        
        let services = [
            ("vector-service", &self.vector_service_addr),
            ("embedding-service", &self.embedding_service_addr),
            ("chat-service", &self.chat_service_addr),
        ];

        let mut errors = Vec::new();

        for (service_name, addr) in &services {
            if let Err(e) = self.test_service_health(addr, service_name).await {
                errors.push(format!("{}: {}", service_name, e));
            }
        }

        if !errors.is_empty() {
            return Err(DatasetError::service_connection(
                "multiple_services",
                format!("Connection validation failed for: {}", errors.join(", ")),
            ));
        }

        info!("All service connections validated successfully");
        Ok(())
    }

    /// Create a mock ServiceClients instance for testing
    pub async fn mock_for_testing() -> Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| DatasetError::service_connection("mock_http_client", e.to_string()))?;

        Ok(Self {
            http_client,
            vector_service_addr: "http://mock:20030".to_string(),
            embedding_service_addr: "http://mock:20020".to_string(),
            chat_service_addr: "http://mock:20010".to_string(),
            retry_operation: RetryOperation::with_defaults(),
        })
    }
}