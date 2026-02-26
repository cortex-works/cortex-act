use crate::{
    client::{
        chat_client::ChatServiceClient,
        embedding_client::EmbeddingServiceClient,
    },
    config::Config,
    database::FineTuneDatabase,
    engine::{EngineFactory, ProgressUpdate, FineTuneRequest, TrainingConfig},
    error::{FineTuneError, Result},
    fine_tune::{SubmitRequest, VectorDatasetConfig, LocalDatasetConfig, DatasetSplit},
};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::Arc,
};
use tokio::sync::{mpsc, RwLock};
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};

#[derive(Clone)]
pub struct FineTuningOrchestrator {
    config: Config,
    database: FineTuneDatabase,
}

#[derive(Debug, Serialize, Deserialize)]
struct TrainingExample {
    input: String,
    output: String,
    metadata: Option<serde_json::Value>,
}

// Define missing types for compatibility
#[derive(Debug, Clone)]
pub struct TrainingChunk {
    pub id: String,
    pub content: String,
    pub source: String,
    pub score: f64,
}

#[derive(Debug, Clone)]
pub struct DatasetFilters {
    pub min_content_length: Option<usize>,
    pub max_content_length: Option<usize>,
    pub min_score: Option<f64>,
    pub allowed_sources: Vec<String>,
    pub excluded_sources: Vec<String>,
}

impl FineTuningOrchestrator {
    pub async fn new(config: Config, database: FineTuneDatabase) -> Result<Self> {
        info!("Initializing Fine-Tuning Orchestrator");
        Ok(Self { config, database })
    }

    /// Enhanced dataset path resolution with intelligent path search
    fn resolve_dataset_path(&self, configured_path: &str) -> Result<PathBuf> {
        let path = std::path::Path::new(configured_path);
        
        // If it's an absolute path and exists, use it directly
        if path.is_absolute() {
            if path.exists() {
                info!("✅ Found dataset at absolute path: {}", path.display());
                return Ok(path.to_path_buf());
            } else {
                return Err(FineTuneError::Dataset(format!(
                    "Dataset file not found at absolute path: {}", 
                    configured_path
                )));
            }
        }

        // For relative paths, search in multiple locations
        let search_locations = vec![
            // 1. Current working directory
            std::env::current_dir().unwrap_or_default().join(configured_path),
            // 2. Workspace root data/datasets/
            std::env::current_dir().unwrap_or_default().join("data/datasets").join(configured_path),
            // 3. Fine-tune service data directory
            std::env::current_dir().unwrap_or_default().join("services/fine-tune-service/data/datasets").join(configured_path),
            // 4. Dataset generator service directory 
            std::env::current_dir().unwrap_or_default().join("services/dataset-generator-service/datasets").join(configured_path),
            // 5. Relative to path as-is
            path.to_path_buf(),
        ];

        debug!("🔍 Searching for dataset file: {}", configured_path);
        for (i, candidate_path) in search_locations.iter().enumerate() {
            debug!("  {} Checking: {}", i + 1, candidate_path.display());
            if candidate_path.exists() {
                info!("✅ Found dataset at location {}: {}", i + 1, candidate_path.display());
                return Ok(candidate_path.clone());
            }
        }

        // If not found, provide detailed error with all searched locations
        let searched_locations: Vec<String> = search_locations
            .iter()
            .map(|p| p.display().to_string())
            .collect();

        Err(FineTuneError::Dataset(format!(
            "Dataset file '{}' not found. Searched in:\n{}",
            configured_path,
            searched_locations.join("\n")
        )))
    }

    pub async fn run_fine_tuning_job(
        &self,
        job_id: &str,
        request: SubmitRequest,
        _chat_client: Arc<RwLock<ChatServiceClient>>,
        _embedding_client: Arc<RwLock<EmbeddingServiceClient>>,
        cancel_token: CancellationToken,
    ) -> Result<()> {
        let job_id_short = &job_id[..8.min(job_id.len())];
        
        info!("🔄 Job {}: Starting fine-tuning job execution", job_id_short);

        // Update status to PREPARING
        self.update_job_status(job_id, "PREPARING", 10.0, &["Starting dataset preparation".to_string()]).await?;
        info!("📊 Job {}: PREPARING (10%) - Starting dataset preparation", job_id_short);

        // Process the dataset based on the request configuration
        let dataset_path = if let Some(dataset_config) = &request.dataset_config {
            match &dataset_config.dataset_source {
                Some(crate::fine_tune::dataset_config::DatasetSource::LocalConfig(local_config)) => {
                    self.resolve_dataset_path(&local_config.dataset_path)?
                }
                Some(crate::fine_tune::dataset_config::DatasetSource::VectorConfig(_)) => {
                    return Err(FineTuneError::Dataset("Vector datasets not yet supported".to_string()));
                }
                None => {
                    return Err(FineTuneError::Dataset("No dataset configuration provided".to_string()));
                }
            }
        } else {
            return Err(FineTuneError::Dataset("No dataset configuration provided".to_string()));
        };
        info!("🔍 Job {}: Processing local dataset: {}", job_id_short, dataset_path.display());

        // For single-turn API, use the dataset directly without processing
        self.update_job_status(job_id, "PREPARING", 50.0, &[format!("Using existing dataset: {}", dataset_path.display())]).await?;
        info!("📊 Job {}: PREPARING (50%) - Using existing dataset", job_id_short);

        if cancel_token.is_cancelled() {
            return Err(FineTuneError::Cancellation("Job cancelled during dataset preparation".to_string()));
        }

        self.update_job_status(job_id, "PREPARING", 70.0, &[format!("Dataset ready: {}", dataset_path.display())]).await?;
        info!("📊 Job {}: PREPARING (70%) - Dataset ready: {}", job_id_short, dataset_path.display());

        // Start training
        self.update_job_status(job_id, "TRAINING", 80.0, &["Starting model training with LLaMA Factory engine".to_string()]).await?;
        info!("📊 Job {}: TRAINING (80%) - Starting model training with LLaMA Factory engine", job_id_short);
        info!("⚡ Real-time progress will be shown below...");

        let model_path = self.train_model(
            job_id,
            &request,
            &dataset_path,
            &cancel_token,
        ).await?;

        if cancel_token.is_cancelled() {
            return Err(FineTuneError::Cancellation("Job cancelled during training".to_string()));
        }

        // Complete job
        self.update_job_status(job_id, "COMPLETED", 100.0, &[format!("Training completed. Model saved to: {}", model_path.display())]).await?;
        info!("📊 Job {}: COMPLETED (100%) - Training completed. Model saved to: {}", job_id_short, model_path.display());

        info!("✅ Fine-tuning job {} completed successfully", job_id_short);
        Ok(())
    }

    async fn process_vector_dataset(
        &self,
        _request: &SubmitRequest,
        vector_config: &VectorDatasetConfig,
        _chat_client: &Arc<RwLock<ChatServiceClient>>,
        _embedding_client: &Arc<RwLock<EmbeddingServiceClient>>,
        _cancel_token: &CancellationToken,
    ) -> Result<Vec<TrainingExample>> {
        debug!("Processing vector-based dataset with seed prompt: {}", vector_config.seed_prompt);

        // For now, create a simple training example from the seed prompt
        // TODO: Implement proper vector dataset retrieval when vector service is available
        let training_examples = vec![
            TrainingExample {
                input: format!("Process: {}", vector_config.seed_prompt),
                output: vector_config.seed_prompt.clone(),
                metadata: Some(serde_json::json!({
                    "type": "seed_prompt",
                    "collection": vector_config.collection_name
                })),
            }
        ];

        debug!("Generated {} training examples from seed prompt", training_examples.len());
        Ok(training_examples)
    }

    async fn process_local_dataset(
        &self,
        local_config: &LocalDatasetConfig,
        split_config: Option<&DatasetSplit>,
        dataset_type: Option<i32>,
        cancel_token: &CancellationToken,
    ) -> Result<Vec<TrainingExample>> {
        debug!("Processing local dataset from path: {}", local_config.dataset_path);

        // Enhanced dataset path resolution with multiple search locations
        let dataset_path = self.resolve_dataset_path(&local_config.dataset_path)?;

        let file_content = tokio::fs::read_to_string(&dataset_path).await
            .map_err(|e| FineTuneError::Dataset(format!("Failed to read dataset file: {}", e)))?;

        if cancel_token.is_cancelled() {
            return Err(FineTuneError::Cancellation("Job cancelled during file reading".to_string()));
        }

        // Parse based on file format
        let format = local_config.format.as_deref().unwrap_or_else(|| {
            // Auto-detect format from file extension
            dataset_path.extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("jsonl")
        });

        let mut examples = Vec::new();

        // Convert format to lowercase for case-insensitive matching
        match format.to_lowercase().as_str() {
            "jsonl" => {
                for line in file_content.lines() {
                    if line.trim().is_empty() {
                        continue;
                    }
                    
                    let json_value: serde_json::Value = serde_json::from_str(line)
                        .map_err(|e| FineTuneError::Dataset(format!("Failed to parse JSONL line: {}", e)))?;
                    
                    let input = json_value.get(&local_config.input_field)
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| FineTuneError::Dataset(format!("Missing input field: {}", local_config.input_field)))?
                        .to_string();
                    
                    let output = json_value.get(&local_config.output_field)
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| FineTuneError::Dataset(format!("Missing output field: {}", local_config.output_field)))?
                        .to_string();
                    
                    examples.push(TrainingExample {
                        input,
                        output,
                        metadata: Some(json_value),
                    });
                }
            }
            "json" | "api_docs" | "single_turn_api" => {
                // Parse as API documentation dataset (JSON with data array containing conversation objects)
                let json_data: serde_json::Value = serde_json::from_str(&file_content)
                    .map_err(|e| FineTuneError::Dataset(format!("Failed to parse API docs JSON: {}", e)))?;
                
                // For single_turn_api format, process directly
                if format.to_lowercase() == "single_turn_api" {
                    info!("🎯 Processing single_turn_api format dataset explicitly");
                    self.process_single_turn_api_format(&json_data, &mut examples)?;
                    return Ok(examples);
                }
                
                // Detect if this is a single_turn_api or multi_turn_api format (auto-detection for json/api_docs)
                if let Some(first_item) = json_data.as_array().and_then(|arr| arr.first()) {
                    if first_item.get("query").is_some() && first_item.get("tools").is_some() {
                        // This is single_turn_api or multi_turn_api format
                        info!("🎯 Detected single_turn_api format dataset");
                        self.process_single_turn_api_format(&json_data, &mut examples)?;
                        return Ok(examples);
                    }
                }
                
                // Process the dataset based on the dataset type setting (for standard conversation format)
                let dataset_type_enum = dataset_type.unwrap_or(0); // Default to SINGLE_TURN
                
                // Check if it's the new API docs dataset format with "data" wrapper
                if let Some(data_array) = json_data.get("data").and_then(|d| d.as_array()) {
                    info!("📊 Processing API docs dataset with {} items (type: {})", data_array.len(), 
                          match dataset_type_enum { 0 => "SINGLE_TURN", 1 => "MULTI_TURN", 2 => "MIXED", _ => "UNKNOWN" });
                    
                    // Handle the dataset-generator-service format: {"data": [{"conversations": [...], "tools": [...]}]}
                    for (item_idx, item) in data_array.iter().enumerate() {
                        if let Some(conversations) = item.get("conversations").and_then(|c| c.as_array()) {
                            info!("🔍 Processing item {} with {} conversations", item_idx + 1, conversations.len());
                            
                            // Process conversations based on dataset type
                            match dataset_type_enum {
                                0 => { // SINGLE_TURN
                                    self.process_single_turn_conversations(conversations, item, &mut examples)?;
                                }
                                1 => { // MULTI_TURN
                                    self.process_multi_turn_conversations(conversations, item, &mut examples)?;
                                }
                                2 => { // MIXED
                                    // For mixed datasets, automatically detect conversation type
                                    if conversations.len() <= 2 {
                                        self.process_single_turn_conversations(conversations, item, &mut examples)?;
                                    } else {
                                        self.process_multi_turn_conversations(conversations, item, &mut examples)?;
                                    }
                                }
                                _ => {
                                    // Default to single-turn for unknown types
                                    self.process_single_turn_conversations(conversations, item, &mut examples)?;
                                }
                            }
                        }
                    }
                } else if let Some(instances) = json_data.get("instances").and_then(|i| i.as_array()) {
                    // Handle the test dataset format: {"instances": [{"conversations": [...]}]}
                    info!("📊 Processing test dataset with {} instances (type: {})", instances.len(),
                          match dataset_type_enum { 0 => "SINGLE_TURN", 1 => "MULTI_TURN", 2 => "MIXED", _ => "UNKNOWN" });
                    
                    for (item_idx, item) in instances.iter().enumerate() {
                        if let Some(conversations) = item.get("conversations").and_then(|c| c.as_array()) {
                            info!("🔍 Processing instance {} with {} conversations", item_idx + 1, conversations.len());
                            
                            // Process conversations based on dataset type
                            match dataset_type_enum {
                                0 => { // SINGLE_TURN
                                    self.process_single_turn_conversations(conversations, item, &mut examples)?;
                                }
                                1 => { // MULTI_TURN
                                    self.process_multi_turn_conversations(conversations, item, &mut examples)?;
                                }
                                2 => { // MIXED
                                    // For mixed datasets, automatically detect conversation type
                                    if conversations.len() <= 2 {
                                        self.process_single_turn_conversations(conversations, item, &mut examples)?;
                                    } else {
                                        self.process_multi_turn_conversations(conversations, item, &mut examples)?;
                                    }
                                }
                                _ => {
                                    // Default to single-turn for unknown types
                                    self.process_single_turn_conversations(conversations, item, &mut examples)?;
                                }
                            }
                        }
                    }
                } else if let Some(conversations_array) = json_data.as_array() {
                    // Handle the original format: [{"messages": [...]}]
                    for conversation in conversations_array {
                        if let Some(messages) = conversation.get("messages").and_then(|m| m.as_array()) {
                            // Convert conversation messages to input/output pairs
                            match dataset_type_enum {
                                0 => { // SINGLE_TURN
                                    self.process_single_turn_conversations(messages, conversation, &mut examples)?;
                                }
                                1 => { // MULTI_TURN
                                    self.process_multi_turn_conversations(messages, conversation, &mut examples)?;
                                }
                                _ => {
                                    // Default processing for backward compatibility
                                    for i in (0..messages.len().saturating_sub(1)).step_by(2) {
                                        if let (Some(user_msg), Some(assistant_msg)) = (messages.get(i), messages.get(i + 1)) {
                                            if let (Some(input), Some(output)) = (
                                                user_msg.get("content").and_then(|c| c.as_str()),
                                                assistant_msg.get("content").and_then(|c| c.as_str())
                                            ) {
                                                examples.push(TrainingExample {
                                                    input: input.to_string(),
                                                    output: output.to_string(),
                                                    metadata: Some(conversation.clone()),
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    return Err(FineTuneError::Dataset("API docs dataset should be either a JSON array or have a 'data' or 'instances' field with an array".to_string()));
                }
            }
            "csv" => {
                let mut reader = csv::Reader::from_reader(file_content.as_bytes());
                for result in reader.records() {
                    let record = result
                        .map_err(|e| FineTuneError::Dataset(format!("Failed to parse CSV record: {}", e)))?;
                    
                    // For CSV, we assume the input and output fields are column names or indices
                    let input = record.get(0)
                        .ok_or_else(|| FineTuneError::Dataset("Missing input column in CSV".to_string()))?
                        .to_string();
                    
                    let output = record.get(1)
                        .ok_or_else(|| FineTuneError::Dataset("Missing output column in CSV".to_string()))?
                        .to_string();
                    
                    examples.push(TrainingExample {
                        input,
                        output,
                        metadata: None,
                    });
                }
            }
            _ => {
                return Err(FineTuneError::Dataset(format!("Unsupported file format: {}", format)));
            }
        }

        if cancel_token.is_cancelled() {
            return Err(FineTuneError::Cancellation("Job cancelled during data parsing".to_string()));
        }

        // Apply dataset split if configured
        if let Some(split_config) = split_config {
            examples = self.apply_dataset_split(examples, split_config).await?;
        }

        debug!("Loaded {} training examples from local dataset", examples.len());
        Ok(examples)
    }

    async fn apply_dataset_split(
        &self,
        mut examples: Vec<TrainingExample>,
        split_config: &DatasetSplit,
    ) -> Result<Vec<TrainingExample>> {
        match &split_config.split_type {
            Some(crate::fine_tune::dataset_split::SplitType::TrainRatio(ratio)) => {
                let train_size = (examples.len() as f32 * ratio) as usize;
                examples.truncate(train_size);
                debug!("Applied train ratio {}, keeping {} examples", ratio, examples.len());
            }
            Some(crate::fine_tune::dataset_split::SplitType::Paths(paths)) => {
                // If separate paths are specified, we would load the train path here
                // For now, we'll just use the existing examples as training data
                debug!("Using separate dataset paths (train: {})", paths.train_path);
            }
            None => {
                // No split configuration
            }
        }
        Ok(examples)
    }

    async fn create_dataset_file(
        &self,
        job_id: &str,
        examples: &[TrainingExample],
    ) -> Result<PathBuf> {
        let dataset_dir = std::path::Path::new(&self.config.data_dir).join("datasets");
        tokio::fs::create_dir_all(&dataset_dir).await
            .map_err(|e| FineTuneError::Storage(e.to_string()))?;

        let dataset_path = dataset_dir.join(format!("{}_dataset.jsonl", job_id));
        
        let mut file_content = String::new();
        for example in examples {
            let json_line = serde_json::to_string(example)
                .map_err(|e| FineTuneError::Dataset(format!("Failed to serialize training example: {}", e)))?;
            file_content.push_str(&json_line);
            file_content.push('\n');
        }

        tokio::fs::write(&dataset_path, file_content).await
            .map_err(|e| FineTuneError::Storage(e.to_string()))?;

        info!("Created dataset file with {} examples: {}", examples.len(), dataset_path.display());
        Ok(dataset_path)
    }

    async fn train_model(
        &self,
        job_id: &str,
        request: &SubmitRequest,
        dataset_path: &PathBuf,
        _cancel_token: &CancellationToken,
    ) -> Result<PathBuf> {
        info!("Starting model training for job: {}", job_id);
        
        // Extract model name from request
        let model_name = match &request.model_config {
            Some(config) => match &config.model_source {
                Some(crate::fine_tune::model_config::ModelSource::ModelName(name)) => name.clone(),
                Some(crate::fine_tune::model_config::ModelSource::ModelPath(path)) => {
                    std::path::Path::new(path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Salesforce/xLAM-2-1b-fc-r")
                        .to_string()
                }
                None => "Salesforce/xLAM-2-1b-fc-r".to_string(),
            },
            None => "Salesforce/xLAM-2-1b-fc-r".to_string(),
         };
         
         info!("🚀 Using MLX engine for fine-tuning model: {}", model_name);
         
        // Create the MLX engine (only supported engine)
        let engine = EngineFactory::create_engine(&model_name, true).await;
        
        // Create progress channel
        let (progress_tx, mut progress_rx) = mpsc::unbounded_channel::<ProgressUpdate>();
        
        // Spawn progress monitoring task
        let database = self.database.clone();
        let job_id_clone = job_id.to_string();
        let progress_task = tokio::spawn(async move {
            while let Some(progress) = progress_rx.recv().await {
                let logs = vec![progress.message.clone()];
                if let Err(e) = database.update_job_status(
                    &job_id_clone,
                    &progress.stage,
                    progress.progress,
                    &logs,
                ).await {
                    tracing::error!("Failed to update job progress: {}", e);
                }
                
                // Log progress to terminal
                info!("🔥 Job {}: {:.1}% | {} | {}", 
                    &job_id_clone[..8.min(job_id_clone.len())],
                    progress.progress,
                    progress.stage,
                    progress.message
                );
            }
        });
        
        // Convert SubmitRequest to FineTuneRequest
        let fine_tune_request = FineTuneRequest {
            job_id: job_id.to_string(),
            model_name: model_name.clone(),
            dataset_path: dataset_path.to_string_lossy().to_string(),
            training_config: request.training_config.as_ref().map(|tc| TrainingConfig {
                epochs: tc.epochs as u32,
                batch_size: tc.batch_size as u32,
                learning_rate: tc.learning_rate as f64,
            }),
            job_name: request.job_name.clone(),
            description: None,
        };
        
        // Run fine-tuning
        let result = engine.fine_tune(fine_tune_request, progress_tx).await?;
        
        // Wait for progress task to complete
        progress_task.abort();
        
        if result.success {
            Ok(PathBuf::from(result.model_path))
        } else {
            Err(FineTuneError::EngineError(result.message))
        }
    }

    async fn update_job_status(
        &self,
        job_id: &str,
        status: &str,
        progress: f32,
        logs: &[String],
    ) -> Result<()> {
        Self::update_job_status_static(&self.database, job_id, status, progress.into(), logs).await
    }

    pub async fn update_job_status_static(
        database: &FineTuneDatabase,
        job_id: &str,
        status: &str,
        progress: f32,
        logs: &[String],
    ) -> Result<()> {
        database.update_job_status(job_id, status, progress, logs).await
            .map_err(|e| FineTuneError::Database(format!("Failed to update job status: {}", e)))
    }

    /// Process conversations as single-turn Q&A pairs
    fn process_single_turn_conversations(
        &self,
        conversations: &[serde_json::Value],
        item: &serde_json::Value,
        examples: &mut Vec<TrainingExample>,
    ) -> Result<()> {
        debug!("Processing {} conversations as single-turn", conversations.len());
        
        let mut current_input = String::new();
        let mut current_output = String::new();
        
        for conversation in conversations {
            // Handle new format (role/content)
            if let (Some(role), Some(content)) = (
                conversation.get("role").and_then(|r| r.as_str()),
                conversation.get("content").and_then(|c| c.as_str())
            ) {
                match role {
                    "user" | "human" => {
                        // If we have a previous pair, save it
                        if !current_input.is_empty() && !current_output.is_empty() {
                            examples.push(TrainingExample {
                                input: current_input.clone(),
                                output: current_output.clone(),
                                metadata: Some(item.clone()),
                            });
                        }
                        current_input = content.to_string();
                        current_output.clear();
                    }
                    "assistant" => {
                        // Handle function calls in assistant messages
                        if let Some(tool_calls) = conversation.get("tool_calls").and_then(|tc| tc.as_array()) {
                            let mut tool_parts = Vec::new();
                            for tool_call in tool_calls {
                                if let Some(function) = tool_call.get("function") {
                                    if let Some(name) = function.get("name").and_then(|n| n.as_str()) {
                                        tool_parts.push(name.to_string());
                                    }
                                }
                            }
                            if !tool_parts.is_empty() {
                                current_output = format!("{}\nTool Calls: {}", content, tool_parts.join(", "));
                            } else {
                                current_output = content.to_string();
                            }
                        } else {
                            current_output = content.to_string();
                        }
                    }
                    "tool" => {
                        // Append tool response to current output
                        if !current_output.is_empty() {
                            current_output.push_str(&format!("\nTool Response: {}", content));
                        }
                    }
                    _ => {
                        // Other roles, skip or handle as needed
                    }
                }
            }
            // Handle old format (from/value) for backward compatibility
            else if let (Some(from), Some(value)) = (
                conversation.get("from").and_then(|f| f.as_str()),
                conversation.get("value").and_then(|v| v.as_str())
            ) {
                match from {
                    "human" => {
                        // If we have a previous pair, save it
                        if !current_input.is_empty() && !current_output.is_empty() {
                            examples.push(TrainingExample {
                                input: current_input.clone(),
                                output: current_output.clone(),
                                metadata: Some(item.clone()),
                            });
                        }
                        current_input = value.to_string();
                        current_output.clear();
                    }
                    "gpt" | "assistant" => {
                        current_output = value.to_string();
                    }
                    "function_call" => {
                        current_output.push_str(&format!("\nFunction Call: {}", value));
                    }
                    "observation" => {
                        current_output.push_str(&format!("\nObservation: {}", value));
                    }
                    _ => {
                        // Other roles, skip or handle as needed
                    }
                }
            }
        }
        
        // Add the final pair if we have both input and output
        if !current_input.is_empty() && !current_output.is_empty() {
            examples.push(TrainingExample {
                input: current_input,
                output: current_output,
                metadata: Some(item.clone()),
            });
        }
        
        Ok(())
    }

    /// Process conversations as multi-turn dialogue
    fn process_multi_turn_conversations(
        &self,
        conversations: &[serde_json::Value],
        item: &serde_json::Value,
        examples: &mut Vec<TrainingExample>,
    ) -> Result<()> {
        debug!("Processing {} conversations as multi-turn", conversations.len());
        
        let mut conversation_parts = Vec::new();
        let mut last_user_input = String::new();
        
        for conversation in conversations {
            // Handle new format (role/content)
            if let (Some(role), Some(content)) = (
                conversation.get("role").and_then(|r| r.as_str()),
                conversation.get("content").and_then(|c| c.as_str())
            ) {
                match role {
                    "user" | "human" => {
                        let formatted_content = format!("User: {}", content);
                        conversation_parts.push(formatted_content.clone());
                        last_user_input = content.to_string();
                    }
                    "assistant" => {
                        let mut assistant_content = content.to_string();
                        
                        // Handle function calls in assistant messages
                        if let Some(tool_calls) = conversation.get("tool_calls").and_then(|tc| tc.as_array()) {
                            let mut tool_parts = Vec::new();
                            for tool_call in tool_calls {
                                if let Some(function) = tool_call.get("function") {
                                    if let Some(name) = function.get("name").and_then(|n| n.as_str()) {
                                        tool_parts.push(name.to_string());
                                    }
                                }
                            }
                            if !tool_parts.is_empty() {
                                assistant_content.push_str(&format!("\nTool Calls: {}", tool_parts.join(", ")));
                            }
                        }
                        
                        let formatted_content = format!("Assistant: {}", assistant_content);
                        conversation_parts.push(formatted_content);
                        
                        // Create a training example for this turn
                        if !last_user_input.is_empty() {
                            let context = if conversation_parts.len() > 2 {
                                // Include previous context for multi-turn
                                let context_parts = &conversation_parts[..conversation_parts.len()-2];
                                format!("{}\nUser: {}", context_parts.join("\n"), last_user_input)
                            } else {
                                last_user_input.clone()
                            };
                            
                            examples.push(TrainingExample {
                                input: context,
                                output: assistant_content,
                                metadata: Some(item.clone()),
                            });
                        }
                    }
                    "tool" => {
                        let formatted_content = format!("Tool: {}", content);
                        conversation_parts.push(formatted_content);
                    }
                    _ => {
                        let formatted_content = format!("{}: {}", role, content);
                        conversation_parts.push(formatted_content);
                    }
                }
            }
            // Handle old format (from/value) for backward compatibility
            else if let (Some(from), Some(value)) = (
                conversation.get("from").and_then(|f| f.as_str()),
                conversation.get("value").and_then(|v| v.as_str())
            ) {
                match from {
                    "human" => {
                        let formatted_content = format!("Human: {}", value);
                        conversation_parts.push(formatted_content);
                        last_user_input = value.to_string();
                    }
                    "gpt" | "assistant" => {
                        let formatted_content = format!("Assistant: {}", value);
                        conversation_parts.push(formatted_content);
                        
                        // Create a training example for this turn
                        if !last_user_input.is_empty() {
                            let context = if conversation_parts.len() > 2 {
                                // Include previous context for multi-turn
                                let context_parts = &conversation_parts[..conversation_parts.len()-2];
                                format!("{}\nHuman: {}", context_parts.join("\n"), last_user_input)
                            } else {
                                last_user_input.clone()
                            };
                            
                            examples.push(TrainingExample {
                                input: context,
                                output: value.to_string(),
                                metadata: Some(item.clone()),
                            });
                        }
                    }
                    "function_call" => {
                        let formatted_content = format!("Function Call: {}", value);
                        conversation_parts.push(formatted_content);
                    }
                    "observation" => {
                        let formatted_content = format!("Observation: {}", value);
                        conversation_parts.push(formatted_content);
                    }
                    _ => {
                        let formatted_content = format!("{}: {}", from, value);
                        conversation_parts.push(formatted_content);
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Process single_turn_api format dataset from dataset-generator-service
    fn process_single_turn_api_format(
        &self,
        json_data: &serde_json::Value,
        examples: &mut Vec<TrainingExample>,
    ) -> Result<()> {
        info!("📊 Processing single_turn_api format dataset");
        
        // Single turn API format: [{"query": "...", "answers": "...", "tools": [...]}]
        if let Some(data_array) = json_data.as_array() {
            for (item_idx, item) in data_array.iter().enumerate() {
                // Handle both array and string formats for query and answers
                let user_query = if let Some(query_value) = item.get("query") {
                    match query_value {
                        // Handle query as array: ["question"]
                        serde_json::Value::Array(arr) => {
                            arr.get(0)
                                .and_then(|q| q.as_str())
                                .unwrap_or("Unknown query")
                                .to_string()
                        }
                        // Handle query as string: "question"
                        serde_json::Value::String(s) => s.clone(),
                        _ => "Unknown query".to_string(),
                    }
                } else {
                    continue; // Skip items without query
                };
                
                let assistant_response = if let Some(answers_value) = item.get("answers") {
                    match answers_value {
                        // Handle answers as array: ["response"]
                        serde_json::Value::Array(arr) => {
                            let answer = arr.get(0)
                                .and_then(|a| a.as_str())
                                .unwrap_or("");
                            
                            // If empty answer, create a response from tools
                            if answer.is_empty() {
                                self.create_tool_response_from_item(item)?
                            } else {
                                answer.to_string()
                            }
                        }
                        // Handle answers as string: "response"
                        serde_json::Value::String(s) => {
                            if s.is_empty() {
                                self.create_tool_response_from_item(item)?
                            } else {
                                s.clone()
                            }
                        }
                        _ => self.create_tool_response_from_item(item)?,
                    }
                } else {
                    // No answers field, create response from tools
                    self.create_tool_response_from_item(item)?
                };
                
                // Skip examples with very short queries or responses to avoid noise
                if user_query.len() < 5 || assistant_response.len() < 10 {
                    debug!("Skipping item {} due to short content", item_idx + 1);
                    continue;
                }
                
                // Create training example
                examples.push(TrainingExample {
                    input: user_query,
                    output: assistant_response,
                    metadata: Some(item.clone()),
                });
                
                debug!("Processed single-turn API item {}: '{}' -> '{}'", 
                       item_idx + 1, 
                       if examples.last().unwrap().input.len() > 50 { 
                           format!("{}...", &examples.last().unwrap().input[..50])
                       } else { 
                           examples.last().unwrap().input.clone() 
                       },
                       if examples.last().unwrap().output.len() > 50 { 
                           format!("{}...", &examples.last().unwrap().output[..50])
                       } else { 
                           examples.last().unwrap().output.clone() 
                       });
            }
            
            info!("✅ Processed {} single-turn API examples", examples.len());
        } else {
            return Err(FineTuneError::Dataset("Invalid single_turn_api format: expected array".to_string()));
        }
        
        Ok(())
    }

    /// Create a tool-based response from available tools in the item
    fn create_tool_response_from_item(&self, item: &serde_json::Value) -> Result<String> {
        if let Some(tools_value) = item.get("tools") {
            match tools_value {
                // Handle tools as array of strings
                serde_json::Value::Array(tools_array) => {
                    let mut tool_names = Vec::new();
                    
                    for tool in tools_array {
                        if let Some(tool_str) = tool.as_str() {
                            // Try to parse as JSON to extract function name
                            if let Ok(tool_json) = serde_json::from_str::<serde_json::Value>(tool_str) {
                                if let Some(function) = tool_json.get("function") {
                                    if let Some(name) = function.get("name").and_then(|n| n.as_str()) {
                                        tool_names.push(name.to_string());
                                    }
                                }
                            } else {
                                // If not JSON, treat as simple tool name
                                tool_names.push(tool_str.to_string());
                            }
                        }
                    }
                    
                    if !tool_names.is_empty() {
                        Ok(format!("You can use the following tools: {}", tool_names.join(", ")))
                    } else {
                        Ok("I can help you with that request.".to_string())
                    }
                }
                _ => Ok("I can help you with that request.".to_string()),
            }
        } else {
            Ok("I can help you with that request.".to_string())
        }
    }

}
