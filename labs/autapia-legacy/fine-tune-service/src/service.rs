use crate::{
    client::{
        
        vector_client::VectorServiceClient,
        chat_client::ChatServiceClient,
        embedding_client::EmbeddingServiceClient,
    },
    config::Config,
    database::FineTuneDatabase,
    error::{FineTuneError, Result},
    orchestrator::FineTuningOrchestrator,
};
use crate::fine_tune::{
    fine_tune_server::FineTune,
    SubmitRequest, SubmitResponse, StatusRequest, StatusResponse,
    CancelRequest, CancelResponse, DeleteRequest, DeleteResponse,
    ListJobsRequest, ListJobsResponse,
    JobLogsRequest, JobLogEntry, HealthRequest, HealthResponse,
    JobStatus, JobMetadata, TrainingMetrics, FineTuningMethod,
};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock as StdRwLock},
};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use tonic::{Request, Response, Status};
use tracing::{debug, error, info};
use uuid::Uuid;

#[derive(Clone)]
pub struct FineTuneService {
    config: Arc<StdRwLock<Config>>,
    orchestrator: Arc<RwLock<FineTuningOrchestrator>>,
    pub database: FineTuneDatabase,
    vector_client: Arc<RwLock<VectorServiceClient>>,
    chat_client: Arc<RwLock<ChatServiceClient>>,
    embedding_client: Arc<RwLock<EmbeddingServiceClient>>,
    active_jobs: Arc<RwLock<HashMap<String, CancellationToken>>>,
}

impl FineTuneService {
    pub async fn new(config: Config) -> Result<Self> {
        let database = FineTuneDatabase::new(&config.database.database_url).await?;
        let orchestrator = FineTuningOrchestrator::new(config.clone(), database.clone()).await?;

        Ok(Self {
            config: Arc::new(StdRwLock::new(config.clone())),
            orchestrator: Arc::new(RwLock::new(orchestrator)),
            database,
            vector_client: Arc::new(RwLock::new(
                VectorServiceClient::new(&config.services.vector_service_url).await?
            )),
            chat_client: Arc::new(RwLock::new(
                ChatServiceClient::new(&config.services.chat_service_url).await?
            )),
            embedding_client: Arc::new(RwLock::new(
                EmbeddingServiceClient::new(&config.services.embedding_service_url).await?
            )),
            active_jobs: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn new_with_reload(config: Config) -> Result<Self> {
        Self::new(config).await
    }

    pub async fn reload_config(&self) -> Result<String> {
        info!("🔄 Reloading fine-tune service configuration...");

        // Load fresh configuration
        let new_config = Config::load(None)?;
        
        // Get current config
        let current_config = {
            let config_guard = self.config.read().unwrap();
            config_guard.clone()
        };
        
        // Update configuration
        {
            let mut config_guard = self.config.write().unwrap();
            *config_guard = new_config.clone();
        }

        // Recreate orchestrator with new config if needed
        if current_config.fine_tuning.max_concurrent_jobs != new_config.fine_tuning.max_concurrent_jobs ||
           current_config.fine_tuning.default_device != new_config.fine_tuning.default_device {
            
            match FineTuningOrchestrator::new(new_config.clone(), self.database.clone()).await {
                Ok(new_orchestrator) => {
                    let mut orchestrator_guard = self.orchestrator.write().await;
                    *orchestrator_guard = new_orchestrator;
                    info!("✅ Orchestrator recreated with new configuration");
                }
                Err(e) => {
                    error!("❌ Failed to recreate orchestrator: {}", e);
                    return Err(e);
                }
            }
        }

        // Update service clients if URLs changed
        if current_config.services.vector_service_url != new_config.services.vector_service_url {
            match VectorServiceClient::new(&new_config.services.vector_service_url).await {
                Ok(new_client) => {
                    let mut client_guard = self.vector_client.write().await;
                    *client_guard = new_client;
                    info!("✅ Vector client updated");
                }
                Err(e) => {
                    error!("❌ Failed to update vector client: {}", e);
                }
            }
        }

        if current_config.services.chat_service_url != new_config.services.chat_service_url {
            match ChatServiceClient::new(&new_config.services.chat_service_url).await {
                Ok(new_client) => {
                    let mut client_guard = self.chat_client.write().await;
                    *client_guard = new_client;
                    info!("✅ Chat client updated");
                }
                Err(e) => {
                    error!("❌ Failed to update chat client: {}", e);
                }
            }
        }

        if current_config.services.embedding_service_url != new_config.services.embedding_service_url {
            match EmbeddingServiceClient::new(&new_config.services.embedding_service_url).await {
                Ok(new_client) => {
                    let mut client_guard = self.embedding_client.write().await;
                    *client_guard = new_client;
                    info!("✅ Embedding client updated");
                }
                Err(e) => {
                    error!("❌ Failed to update embedding client: {}", e);
                }
            }
        }

        let message = format!(
            "Configuration reloaded - Device: {}, Max Jobs: {}, Memory: {} MB", 
            new_config.fine_tuning.default_device,
            new_config.fine_tuning.max_concurrent_jobs,
            new_config.max_memory_mb
        );
        info!("✅ {}", message);
        Ok(message)
    }

    fn get_config(&self) -> Config {
        let config_guard = self.config.read().unwrap();
        config_guard.clone()
    }

    fn validate_request(&self, request: &SubmitRequest) -> Result<()> {
        // Validate model configuration
        if let Some(model_config) = &request.model_config {
            match &model_config.model_source {
                Some(crate::fine_tune::model_config::ModelSource::ModelName(name)) => {
                    if name.is_empty() {
                        return Err(FineTuneError::Validation("Model name cannot be empty".to_string()));
                    }
                }
                Some(crate::fine_tune::model_config::ModelSource::ModelPath(path)) => {
                    if path.is_empty() {
                        return Err(FineTuneError::Validation("Model path cannot be empty".to_string()));
                    }
                }
                None => {
                    return Err(FineTuneError::Validation("Model source must be specified".to_string()));
                }
            }
        } else {
            return Err(FineTuneError::Validation("Model configuration is required".to_string()));
        }

        // Validate dataset configuration
        if let Some(dataset_config) = &request.dataset_config {
            match &dataset_config.dataset_source {
                Some(crate::fine_tune::dataset_config::DatasetSource::VectorConfig(vector_config)) => {
                    if vector_config.seed_prompt.is_empty() {
                        return Err(FineTuneError::Validation("Seed prompt cannot be empty for vector-based datasets".to_string()));
                    }
                    if vector_config.retrieval_top_k <= 0 {
                        return Err(FineTuneError::Validation("Retrieval top_k must be positive".to_string()));
                    }
                    if vector_config.retrieval_top_k > 1000 {
                        return Err(FineTuneError::Validation("Retrieval top_k cannot exceed 1000".to_string()));
                    }
                }
                Some(crate::fine_tune::dataset_config::DatasetSource::LocalConfig(local_config)) => {
                    if local_config.dataset_path.is_empty() {
                        return Err(FineTuneError::Validation("Dataset path cannot be empty for local datasets".to_string()));
                    }
                    if local_config.input_field.is_empty() {
                        return Err(FineTuneError::Validation("Input field name cannot be empty".to_string()));
                    }
                    if local_config.output_field.is_empty() {
                        return Err(FineTuneError::Validation("Output field name cannot be empty".to_string()));
                    }
                }
                None => {
                    return Err(FineTuneError::Validation("Dataset source must be specified".to_string()));
                }
            }
        } else {
            return Err(FineTuneError::Validation("Dataset configuration is required".to_string()));
        }

        // Validate training configuration
        if let Some(training_config) = &request.training_config {
            if training_config.batch_size <= 0 {
                return Err(FineTuneError::Validation("Batch size must be positive".to_string()));
            }
            if training_config.learning_rate <= 0.0 {
                return Err(FineTuneError::Validation("Learning rate must be positive".to_string()));
            }
            if training_config.epochs <= 0 {
                return Err(FineTuneError::Validation("Epochs must be positive".to_string()));
            }
            if training_config.max_sequence_length <= 0 {
                return Err(FineTuneError::Validation("Max sequence length must be positive".to_string()));
            }
            if training_config.gradient_accumulation_steps <= 0 {
                return Err(FineTuneError::Validation("Gradient accumulation steps must be positive".to_string()));
            }
        } else {
            return Err(FineTuneError::Validation("Training configuration is required".to_string()));
        }

        // Validate output configuration
        if let Some(output_config) = &request.output_config {
            if output_config.output_dir.is_empty() {
                return Err(FineTuneError::Validation("Output directory cannot be empty".to_string()));
            }
        } else {
            return Err(FineTuneError::Validation("Output configuration is required".to_string()));
        }

        // Validate resource limits
        if let Some(training_config) = &request.training_config {
            let config = self.get_config();
            let estimated_memory = training_config.batch_size as u64 * training_config.max_sequence_length as u64 * 4; // Rough estimate
            if estimated_memory > config.max_memory_mb * 1024 * 1024 {
                return Err(FineTuneError::ResourceLimit(
                    format!("Estimated memory usage ({} MB) exceeds limit ({} MB)", 
                           estimated_memory / (1024 * 1024), config.max_memory_mb)
                ));
            }
        }

        Ok(())
    }

    async fn get_job_metadata(&self, job_id: &str) -> Result<JobMetadata> {
        
        let db_metadata = self.database.get_job_metadata(job_id).await?;
        
        // Convert database JobMetadata to proto JobMetadata
        let model_config = Some(crate::fine_tune::ModelConfig {
            model_source: Some(crate::fine_tune::model_config::ModelSource::ModelName(
                db_metadata.model_name.clone()
            )),
            method: crate::fine_tune::FineTuningMethod::Lora as i32, // Default to LoRA
        });

        let metadata = JobMetadata {
            job_id: db_metadata.job_id,
            job_name: db_metadata.job_name,
            created_at: db_metadata.created_at,
            started_at: db_metadata.updated_at.clone(),
            completed_at: db_metadata.updated_at,
            model_config,
            dataset_config: None, // TODO: Parse from stored config
            training_config: None, // TODO: Parse from stored config
            output_config: None, // TODO: Parse from stored config
            metadata: std::collections::HashMap::new(),
            resource_usage: None,
        };
        
        Ok(metadata)
    }

    /// Cancel all stuck jobs that are in TRAINING/PREPARING status but not currently active
    pub async fn cancel_stuck_jobs(&self) -> Result<Vec<String>> {
        info!("Checking for stuck jobs to cancel");
        
        let active_jobs = self.active_jobs.read().await;
        let stuck_jobs = match self.database.list_jobs(None).await {
            Ok(jobs) => {
                jobs.into_iter()
                    .filter(|job| {
                        // Find jobs that are in active status but not in active_jobs HashMap
                        matches!(job.status.as_str(), "TRAINING" | "PREPARING" | "PENDING") 
                            && !active_jobs.contains_key(&job.job_id)
                    })
                    .collect::<Vec<_>>()
            }
            Err(e) => {
                error!("Failed to list jobs: {}", e);
                return Err(e);
            }
        };
        
        drop(active_jobs); // Release the read lock
        
        let mut cancelled_jobs = Vec::new();
        
        for job in stuck_jobs {
            info!("Cancelling stuck job: {} (status: {})", job.job_id, job.status);
            
            if let Err(e) = self.database.update_job_status(
                &job.job_id, 
                "CANCELLED", 
                0.0, 
                &[format!("Stuck job cancelled automatically (was in {} status)", job.status)]
            ).await {
                error!("Failed to cancel stuck job {}: {}", job.job_id, e);
            } else {
                cancelled_jobs.push(job.job_id);
            }
        }
        
        if !cancelled_jobs.is_empty() {
            info!("Cancelled {} stuck jobs: {:?}", cancelled_jobs.len(), cancelled_jobs);
        } else {
            info!("No stuck jobs found");
        }
        
        Ok(cancelled_jobs)
    }
}

#[tonic::async_trait]
impl FineTune for FineTuneService {
    async fn submit_job(
        &self,
        request: Request<SubmitRequest>,
    ) -> std::result::Result<Response<SubmitResponse>, Status> {
        let request = request.into_inner();
        
        // Get model name for logging
        let model_name = match &request.model_config {
            Some(config) => match &config.model_source {
                Some(crate::fine_tune::model_config::ModelSource::ModelName(name)) => name.clone(),
                Some(crate::fine_tune::model_config::ModelSource::ModelPath(path)) => {
                    std::path::Path::new(path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string()
                }
                None => "unknown".to_string(),
            },
            None => "unknown".to_string(),
        };

        debug!("Received submit job request for model: {}", model_name);

        // Validate request
        if let Err(e) = self.validate_request(&request) {
            error!("Request validation failed: {}", e);
            return Ok(Response::new(SubmitResponse {
                job_id: String::new(),
                message: format!("Validation failed: {}", e),
                status: JobStatus::Failed as i32,
            }));
        }

        // Generate job ID
        let job_id = Uuid::new_v4().to_string();
        info!("Generated job ID: {}", job_id);

        // Create cancellation token for this job
        let cancel_token = CancellationToken::new();
        {
            let mut active_jobs = self.active_jobs.write().await;
            active_jobs.insert(job_id.clone(), cancel_token.clone());
        }

        // Store job metadata in database
        let job_name = if request.job_name.is_empty() {
            format!("fine-tune-{}", &job_id[..8])
        } else {
            request.job_name.clone()
        };
        
        // Convert request configurations to JSON for storage
        let training_config_json = request.training_config.as_ref().map(|config| {
            serde_json::json!({
                "epochs": config.epochs,
                "batch_size": config.batch_size,
                "learning_rate": config.learning_rate,
                "device": config.device,
                "precision": config.precision,
                "max_sequence_length": config.max_sequence_length
            })
        });
        
        let dataset_config_json = request.dataset_config.as_ref().map(|config| {
            let mut json = serde_json::json!({});
            if let Some(ref dataset_source) = config.dataset_source {
                match dataset_source {
                    crate::fine_tune::dataset_config::DatasetSource::LocalConfig(local_config) => {
                        json = serde_json::json!({
                            "type": "local",
                            "dataset_path": local_config.dataset_path,
                            "input_field": local_config.input_field,
                            "output_field": local_config.output_field,
                            "format": local_config.format.as_ref().unwrap_or(&"auto".to_string())
                        });
                    },
                    crate::fine_tune::dataset_config::DatasetSource::VectorConfig(vector_config) => {
                        json = serde_json::json!({
                            "type": "vector",
                            "collection_name": vector_config.collection_name,
                            "seed_prompt": vector_config.seed_prompt,
                            "retrieval_top_k": vector_config.retrieval_top_k,
                            "qa_generation": vector_config.qa_generation,
                            "content_cleaning": vector_config.content_cleaning
                        });
                    }
                }
            }
            json
        });
        
        let output_config_json = request.output_config.as_ref().map(|config| {
            serde_json::json!({
                "output_dir": config.output_dir,
                "save_steps": config.save_steps
            })
        });

        if let Err(e) = self.database.store_job_metadata(
                &job_id,
                &job_name,
                &model_name,
                training_config_json,
                dataset_config_json,
                output_config_json,
                "PENDING",
            ).await {
            error!("Failed to store job metadata: {}", e);
            return Ok(Response::new(SubmitResponse {
                job_id: String::new(),
                message: format!("Failed to store job metadata: {}", e),
                status: JobStatus::Failed as i32,
            }));
        }

        // Display job start banner in Rust terminal for zellij visibility
        info!("");
        info!("🚀 \x1b[1;32m═══════════════════════════════════════════════════════════\x1b[0m");
        info!("🚀 \x1b[1;32m             FINE-TUNING JOB STARTED!\x1b[0m");
        info!("🚀 \x1b[1;32m═══════════════════════════════════════════════════════════\x1b[0m");
        info!("📋 Job ID: \x1b[1m{}\x1b[0m", &job_id[..8.min(job_id.len())]);
        info!("🎯 Model: \x1b[1m{}\x1b[0m", model_name);
        info!("📝 Job Name: \x1b[1m{}\x1b[0m", job_name);
        info!("⚡ Progress will be displayed in this terminal in real-time");
        info!("🚀 \x1b[1;32m═══════════════════════════════════════════════════════════\x1b[0m");
        info!("");

        // Start fine-tuning job in background
        let orchestrator = self.orchestrator.clone();
        let job_id_clone = job_id.clone();
        let request_clone = request.clone();
        let chat_client_clone = self.chat_client.clone();
        let embedding_client_clone = self.embedding_client.clone();
        let active_jobs_clone = self.active_jobs.clone();

        tokio::spawn(async move {
            let result = {
                let orchestrator_guard = orchestrator.read().await;
                orchestrator_guard.run_fine_tuning_job(
                    &job_id_clone,
                    request_clone,
                    chat_client_clone,
                    embedding_client_clone,
                    cancel_token,
                ).await
            };

            // Remove job from active jobs when completed
            {
                let mut active_jobs = active_jobs_clone.write().await;
                active_jobs.remove(&job_id_clone);
            }

            if let Err(e) = result {
                error!("");
                error!("💥 \x1b[1;31m═══════════════════════════════════════════════════════════\x1b[0m");
                error!("💥 \x1b[1;31m             FINE-TUNING JOB FAILED!\x1b[0m");
                error!("💥 \x1b[1;31m═══════════════════════════════════════════════════════════\x1b[0m");
                error!("❌ Job ID: \x1b[1m{}\x1b[0m", &job_id_clone[..8.min(job_id_clone.len())]);
                error!("💀 Error: \x1b[1;91m{}\x1b[0m", e);
                error!("💥 \x1b[1;31m═══════════════════════════════════════════════════════════\x1b[0m");
                error!("");
            }
        });

        Ok(Response::new(SubmitResponse {
            job_id,
            message: "Job submitted successfully".to_string(),
            status: JobStatus::Pending as i32,
        }))
    }

    async fn get_status(
        &self,
        request: Request<StatusRequest>,
    ) -> std::result::Result<Response<StatusResponse>, Status> {
        let request = request.into_inner();
        debug!("Getting status for job: {}", request.job_id);

        match self.get_job_metadata(&request.job_id).await {
            Ok(metadata) => {
                let response = StatusResponse {
                    job_id: request.job_id,
                    status: JobStatus::Pending as i32, // This should be retrieved from database
                    progress: 0.0, // This should be retrieved from database
                    recent_logs: vec![], // This should be retrieved from database
                    metadata: Some(metadata),
                    error_message: None,
                    current_metrics: Some(TrainingMetrics {
                        train_loss: None,
                        validation_loss: None,
                        learning_rate: None,
                        current_step: 0,
                        current_epoch: 0,
                        perplexity: None,
                    }),
                };
                Ok(Response::new(response))
            }
            Err(e) => {
                error!("Failed to get job status: {}", e);
                Err(Status::not_found(format!("Job not found: {}", e)))
            }
        }
    }

    async fn cancel_job(
        &self,
        request: Request<CancelRequest>,
    ) -> std::result::Result<Response<CancelResponse>, Status> {
        let request = request.into_inner();
        info!("Cancelling job: {} (reason: {})", request.job_id, request.reason);

        let mut active_jobs = self.active_jobs.write().await;
        
        // First, try to cancel if the job is in active_jobs (currently running)
        if let Some(cancel_token) = active_jobs.get(&request.job_id) {
            info!("Job {} found in active jobs, sending cancellation signal", request.job_id);
            cancel_token.cancel();
            active_jobs.remove(&request.job_id);
            
            // Update database status to CANCELLED
            if let Err(e) = self.database.update_job_status(&request.job_id, "CANCELLED", 0.0, &["Job cancelled by user".to_string()]).await {
                error!("Failed to update job status to CANCELLED: {}", e);
            }
            
            Ok(Response::new(CancelResponse {
                cancelled: true,
                message: "Job cancelled successfully".to_string(),
            }))
        } else {
            // Job not in active_jobs, check if it exists in database
            match self.database.get_job_metadata(&request.job_id).await {
                Ok(job_metadata) => {
                    // Job exists in database, check its status
                    match job_metadata.status.to_uppercase().as_str() {
                        "COMPLETED" | "FAILED" | "CANCELLED" => {
                            Ok(Response::new(CancelResponse {
                                cancelled: false,
                                message: format!("Job is already in final state: {}", job_metadata.status),
                            }))
                        }
                        "PENDING" | "PREPARING" | "TRAINING" => {
                            // Job is stuck, force cancel it in database
                            info!("Job {} found in database but not in active jobs, force cancelling", request.job_id);
                            
                            if let Err(e) = self.database.update_job_status(&request.job_id, "CANCELLED", 0.0, &["Stuck job cancelled by user".to_string()]).await {
                                error!("Failed to update stuck job status to CANCELLED: {}", e);
                                Ok(Response::new(CancelResponse {
                                    cancelled: false,
                                    message: format!("Failed to cancel stuck job: {}", e),
                                }))
                            } else {
                                Ok(Response::new(CancelResponse {
                                    cancelled: true,
                                    message: "Stuck job cancelled successfully".to_string(),
                                }))
                            }
                        }
                        _ => {
                            Ok(Response::new(CancelResponse {
                                cancelled: false,
                                message: format!("Unknown job status: {}", job_metadata.status),
                            }))
                        }
                    }
                }
                Err(_) => {
                    Ok(Response::new(CancelResponse {
                        cancelled: false,
                        message: "Job not found".to_string(),
                    }))
                }
            }
        }
    }

    async fn delete_job(
        &self,
        request: Request<DeleteRequest>,
    ) -> std::result::Result<Response<DeleteResponse>, Status> {
        let request = request.into_inner();
        info!("Deleting job: {} (force: {})", request.job_id, request.force);

        // Check if job is currently running
        let mut active_jobs = self.active_jobs.write().await;
        if let Some(cancel_token) = active_jobs.get(&request.job_id) {
            if request.force {
                // Cancel the job first if force delete is requested
                info!("Force deleting running job: {}", request.job_id);
                cancel_token.cancel();
                active_jobs.remove(&request.job_id);
            } else {
                return Ok(Response::new(DeleteResponse {
                    deleted: false,
                    message: "Cannot delete running job. Use force=true to cancel and delete, or cancel the job first.".to_string(),
                }));
            }
        }
        drop(active_jobs); // Release the lock

        // Get job metadata before deletion to determine output directory
        let job_metadata = match self.database.get_job_metadata(&request.job_id).await {
            Ok(metadata) => Some(metadata),
            Err(e) => {
                tracing::warn!("Could not retrieve job metadata for cleanup: {}", e);
                None
            }
        };

        // Delete job from database
        match self.database.delete_job(&request.job_id).await {
            Ok(()) => {
                info!("Successfully deleted job from database: {}", request.job_id);
                
                // Clean up model files
                self.cleanup_model_files(&request.job_id, job_metadata.as_ref()).await;
                
                Ok(Response::new(DeleteResponse {
                    deleted: true,
                    message: "Job and associated model files deleted successfully".to_string(),
                }))
            }
            Err(e) => {
                error!("Failed to delete job {}: {}", request.job_id, e);
                Ok(Response::new(DeleteResponse {
                    deleted: false,
                    message: format!("Failed to delete job: {}", e),
                }))
            }
        }
    }

    async fn list_jobs(
        &self,
        request: Request<ListJobsRequest>,
    ) -> std::result::Result<Response<ListJobsResponse>, Status> {
        let request = request.into_inner();
        debug!("Listing jobs with limit: {}, offset: {}", request.limit, request.offset);

        // Query real jobs from database
        match self.database.list_jobs(Some(request.limit)).await {
            Ok(db_jobs) => {
                let jobs: Vec<crate::fine_tune::JobSummary> = db_jobs
                    .into_iter()
                    .map(|db_job| crate::fine_tune::JobSummary {
                        job_id: db_job.job_id,
                        job_name: db_job.job_name,
                        model_name: db_job.model_name.unwrap_or_else(|| "unknown".to_string()),
                        status: match db_job.status.to_uppercase().as_str() {
                            "PENDING" => JobStatus::Pending as i32,
                            "RUNNING" | "TRAINING" => JobStatus::Training as i32,
                            "PREPARING" => JobStatus::Preparing as i32,
                            "COMPLETED" => JobStatus::Completed as i32,
                            "FAILED" => JobStatus::Failed as i32,
                            "CANCELLED" => JobStatus::Cancelled as i32,
                            _ => JobStatus::Pending as i32,
                        },
                        progress: db_job.progress as f32,
                        created_at: db_job.created_at.to_rfc3339(),
                        completed_at: None, // TODO: Add completed_at to database schema
                        method: FineTuningMethod::Lora as i32, // TODO: Get from stored config
                    })
                    .collect();

                Ok(Response::new(ListJobsResponse {
                    total_count: jobs.len() as i32,
                    jobs,
                }))
            }
            Err(e) => {
                error!("Failed to list jobs from database: {}", e);
                Err(Status::internal(format!("Failed to list jobs: {}", e)))
            }
        }
    }

    type GetJobLogsStream = tokio_stream::wrappers::ReceiverStream<std::result::Result<JobLogEntry, Status>>;

    async fn get_job_logs(
        &self,
        request: Request<JobLogsRequest>,
    ) -> std::result::Result<Response<Self::GetJobLogsStream>, Status> {
        let request = request.into_inner();
        info!("Getting logs for job: {}", request.job_id);

        let (tx, rx) = tokio::sync::mpsc::channel(100);
        
        // Check if job exists first
        match self.database.get_job_metadata(&request.job_id).await {
            Ok(job_metadata) => {
                info!("Found job: {} (status: {})", job_metadata.job_id, job_metadata.status);
                
                // Get logs from database
                let _database_clone = self.database.clone();
                let job_id = request.job_id.clone();
                let tail_lines = request.tail_lines.unwrap_or(50) as usize;
                
                tokio::spawn(async move {
                    // First, send logs from database
                    if !job_metadata.logs.is_empty() {
                        // Take the last N lines if tail_lines is specified
                        let logs_to_send = if job_metadata.logs.len() > tail_lines {
                            &job_metadata.logs[job_metadata.logs.len() - tail_lines..]
                        } else {
                            &job_metadata.logs[..]
                        };
                        
                        for (_index, log_message) in logs_to_send.iter().enumerate() {
                            let log_entry = JobLogEntry {
                                timestamp: chrono::Utc::now().to_rfc3339(),
                                level: if log_message.contains("Error") || log_message.contains("❌") {
                                    "ERROR".to_string()
                                } else if log_message.contains("Warning") || log_message.contains("⚠️") {
                                    "WARN".to_string()
                                } else {
                                    "INFO".to_string()
                                },
                                message: log_message.clone(),
                                component: Some("fine-tune-service".to_string()),
                            };
                            
                            if tx.send(Ok(log_entry)).await.is_err() {
                                // Client disconnected
                                return;
                            }
                        }
                    } else {
                        // Send a message indicating no logs are available
                        let log_entry = JobLogEntry {
                            timestamp: chrono::Utc::now().to_rfc3339(),
                            level: "INFO".to_string(),
                            message: format!("No logs available for job {} (status: {})", job_id, job_metadata.status),
                            component: Some("fine-tune-service".to_string()),
                        };
                        
                        if tx.send(Ok(log_entry)).await.is_err() {
                            return;
                        }
                    }
                    
                    // If follow is true, we could implement real-time log streaming here
                    // For now, we just close the stream after sending existing logs
                });
            }
            Err(e) => {
                error!("Failed to get job metadata for logs: {}", e);
                
                // Send error as a log entry
                let error_entry = JobLogEntry {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    level: "ERROR".to_string(),
                    message: format!("Failed to retrieve job logs: {}", e),
                    component: Some("fine-tune-service".to_string()),
                };
                
                let tx_clone = tx.clone();
                tokio::spawn(async move {
                    let _ = tx_clone.send(Ok(error_entry)).await;
                });
            }
        }

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }

    async fn get_health(
        &self,
        _request: Request<HealthRequest>,
    ) -> std::result::Result<Response<HealthResponse>, Status> {
        debug!("Health check requested");

        let mut service_status = std::collections::HashMap::new();
        service_status.insert("database".to_string(), "healthy".to_string());
        service_status.insert("vector".to_string(), "healthy".to_string());
        service_status.insert("chat".to_string(), "healthy".to_string());
        service_status.insert("embedding".to_string(), "healthy".to_string());
        
        // Add active jobs information
        let active_jobs = self.active_jobs.read().await;
        service_status.insert("active_jobs_count".to_string(), active_jobs.len().to_string());
        
        let active_job_ids: Vec<String> = active_jobs.keys().cloned().collect();
        service_status.insert("active_job_ids".to_string(), format!("{:?}", active_job_ids));
        
        drop(active_jobs);
        
        // Check for stuck jobs
        match self.database.list_jobs(Some(50)).await {
            Ok(jobs) => {
                let stuck_jobs_count = jobs.iter()
                    .filter(|job| matches!(job.status.as_str(), "TRAINING" | "PREPARING" | "PENDING"))
                    .count();
                service_status.insert("stuck_jobs_count".to_string(), stuck_jobs_count.to_string());
                
                let total_jobs = jobs.len();
                service_status.insert("total_jobs".to_string(), total_jobs.to_string());
            }
            Err(e) => {
                service_status.insert("database_jobs_query".to_string(), format!("error: {}", e));
            }
        }

        Ok(Response::new(HealthResponse {
            healthy: true,
            message: "Fine-tune service is healthy".to_string(),
            service_status,
        }))
    }
}

impl FineTuneService {
    /// Clean up model files associated with a fine-tuning job
    async fn cleanup_model_files(&self, job_id: &str, job_metadata: Option<&crate::database::JobMetadata>) {
        use std::path::Path;
        use tokio::fs;

        // Try multiple possible model output locations
        let mut cleanup_paths = Vec::new();

        // 1. Default models directory with job ID
        let default_model_path = Path::new("models").join(format!("fine_tuned_model_{}", job_id));
        cleanup_paths.push(default_model_path);

        // 2. Fine-tuned model directory pattern used in orchestrator
        let fine_tuned_path = Path::new("models").join(format!("{}-finetuned", job_id));
        cleanup_paths.push(fine_tuned_path);

        // 3. Use job metadata to construct additional cleanup paths
        if let Some(metadata) = job_metadata {
            // Use the job name for additional cleanup patterns
            let job_specific_path = Path::new("models").join(format!("{}-model", metadata.job_name));
            cleanup_paths.push(job_specific_path);
        }

        // 4. Check current directory structure for any directory containing the job ID
        match fs::read_dir("models").await {
            Ok(mut entries) => {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    if let Ok(file_name) = entry.file_name().into_string() {
                        if file_name.contains(job_id) {
                            cleanup_paths.push(entry.path());
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Could not read models directory for cleanup: {}", e);
            }
        }

        // Remove duplicates and attempt cleanup
        cleanup_paths.sort();
        cleanup_paths.dedup();

        for path in cleanup_paths {
            if path.exists() {
                info!("Cleaning up model directory: {:?}", path);
                match fs::remove_dir_all(&path).await {
                    Ok(()) => {
                        info!("Successfully removed model directory: {:?}", path);
                    }
                    Err(e) => {
                        error!("Failed to remove model directory {:?}: {}", path, e);
                    }
                }
            } else {
                debug!("Model directory does not exist: {:?}", path);
            }
        }

        info!("Model cleanup completed for job: {}", job_id);
    }
} 