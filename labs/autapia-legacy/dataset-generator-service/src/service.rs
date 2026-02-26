use crate::dataset_generator::{
    dataset_generator_server::DatasetGenerator,
    *,
};
use crate::clients::ServiceClients;
use crate::pipeline::DatasetPipeline;
use crate::types::{JobStatus, DatasetJob};
use crate::enhanced_pipeline::EnhancedApiDatasetPipeline;
use crate::api_config::EnhancedDataset;
use anyhow::Result;
use log::{error, info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tonic::{Request, Response, Status};
use uuid::Uuid;

type SharedConfig = Arc<RwLock<crate::Args>>;

#[derive(Clone)]
pub struct DatasetGeneratorService {
    clients: Arc<ServiceClients>,
    pipeline: Arc<DatasetPipeline>,
    jobs: Arc<RwLock<HashMap<String, DatasetJob>>>,
    output_dir: String,
    max_concurrent_jobs: usize,
    active_jobs: Arc<Mutex<usize>>,
    config: SharedConfig,
    database: Arc<crate::database::DatasetDatabase>,
}

impl DatasetGeneratorService {
    pub async fn new(
        database_url: String,
        vector_service_addr: String,
        embedding_service_addr: String,
        chat_service_addr: String,
        output_dir: String,
        max_concurrent_jobs: usize,
        config: SharedConfig,
    ) -> Result<Self> {
        // Initialize database connection
        let database = Arc::new(crate::database::DatasetDatabase::new(&database_url).await
            .map_err(|e| anyhow::anyhow!("Failed to initialize database: {}", e))?);

        let clients = Arc::new(ServiceClients::new(
            vector_service_addr,
            embedding_service_addr,
            chat_service_addr,
        ).await?);

        let pipeline = Arc::new(DatasetPipeline::new(clients.clone()));

        Ok(Self {
            clients,
            pipeline,
            jobs: Arc::new(RwLock::new(HashMap::new())),
            output_dir,
            max_concurrent_jobs,
            active_jobs: Arc::new(Mutex::new(0)),
            config,
            database,
        })
    }

    /// Reload service configuration
    pub async fn reload_config(&self) -> Result<String> {
        info!("🔄 Reloading dataset-generator-service configuration...");
        
        // Load fresh configuration from environment
        let new_config = crate::Args::from_env()
            .map_err(|e| anyhow::anyhow!("Failed to load configuration: {}", e))?;
        
        // Update shared configuration
        {
            let mut config_guard = self.config.write().await;
            *config_guard = new_config.clone();
        }
        
        // Ensure output directory exists
        tokio::fs::create_dir_all(&new_config.output_dir).await
            .map_err(|e| anyhow::anyhow!("Failed to create output directory: {}", e))?;
        
        info!("✅ Configuration reloaded successfully");
        info!("📁 Output directory: {}", new_config.output_dir);
        info!("🔗 Vector service: {}", new_config.vector_service_addr);
        info!("🔗 Embedding service: {}", new_config.embedding_service_addr);
        info!("🔗 Chat service: {}", new_config.chat_service_addr);
        info!("⚙️ Max concurrent jobs: {}", new_config.max_concurrent_jobs);
        
        Ok("Dataset generator configuration reloaded successfully".to_string())
    }

    async fn can_start_job(&self) -> bool {
        let active = self.active_jobs.lock().await;
        *active < self.max_concurrent_jobs
    }

    async fn increment_active_jobs(&self) {
        let mut active = self.active_jobs.lock().await;
        *active += 1;
    }

    async fn decrement_active_jobs(&self) {
        let mut active = self.active_jobs.lock().await;
        if *active > 0 {
            *active -= 1;
        }
    }

    /// Generate enhanced comprehensive API documentation dataset
    pub async fn generate_enhanced_api_dataset_internal(
        &self,
        variations_per_use_case: usize,
        format: crate::api_config::DatasetFormat,
    ) -> Result<String, Status> {
        // Check if we have capacity for a new job
        let active_count = *self.active_jobs.lock().await;
        if active_count >= self.max_concurrent_jobs {
            return Err(Status::resource_exhausted("Maximum concurrent jobs reached"));
        }

        // Create database job entry
        let job_name = format!("enhanced_api_dataset_{}", chrono::Utc::now().format("%Y%m%d_%H%M_%S"));
        let create_job = crate::database::CreateEnhancedDatasetJob {
            job_name: job_name.clone(),
            variations_per_use_case: variations_per_use_case as i32,
        };

        let job_id = self.database.create_enhanced_dataset_job(&create_job).await
            .map_err(|e| Status::internal(format!("Failed to create job in database: {}", e)))?;

        info!("Created enhanced API dataset generation job: {} with ID: {}", job_name, job_id);

        // Increment active jobs counter
        self.increment_active_jobs().await;

        // Spawn background task for dataset generation
        let database_clone = self.database.clone();
        let clients_clone = self.clients.clone();
        let output_dir = self.output_dir.clone();
        let active_jobs_clone = self.active_jobs.clone();
        let job_id_for_task = job_id;

        tokio::spawn(async move {
            let start_time = std::time::Instant::now();
            
            let result = Self::generate_enhanced_dataset_background(
                job_id_for_task,
                variations_per_use_case,
                output_dir,
                clients_clone,
                database_clone.clone(),
                format,
            ).await;

            let execution_time = start_time.elapsed().as_secs_f32();

            // Update final job status in database
            match result {
                Ok((output_path, total_examples, total_endpoints, services_covered)) => {
                    if let Err(e) = database_clone.complete_enhanced_job(
                        &job_id_for_task,
                        total_examples,
                        total_endpoints,
                        &services_covered,
                        &output_path,
                    ).await {
                        error!("Failed to complete job in database: {}", e);
                    }

                    // Record job statistics
                    let stats = crate::database::JobStatistics {
                        id: Uuid::new_v4(),
                        job_id: job_id_for_task,
                        job_type: "enhanced".to_string(),
                        execution_time_seconds: Some(execution_time),
                        memory_usage_mb: None,
                        cpu_usage_percent: None,
                        records_processed: Some(total_examples),
                        success_rate: Some(1.0),
                        created_at: chrono::Utc::now(),
                    };
                    
                    if let Err(e) = database_clone.record_job_statistics(&job_id_for_task, "enhanced", &stats).await {
                        warn!("Failed to record job statistics: {}", e);
                    }
                }
                Err(e) => {
                    if let Err(db_err) = database_clone.fail_enhanced_job(&job_id_for_task, &e.to_string()).await {
                        error!("Failed to update job failure in database: {}", db_err);
                    }

                    // Record failure statistics
                    let stats = crate::database::JobStatistics {
                        id: Uuid::new_v4(),
                        job_id: job_id_for_task,
                        job_type: "enhanced".to_string(),
                        execution_time_seconds: Some(execution_time),
                        memory_usage_mb: None,
                        cpu_usage_percent: None,
                        records_processed: Some(0),
                        success_rate: Some(0.0),
                        created_at: chrono::Utc::now(),
                    };
                    
                    if let Err(e) = database_clone.record_job_statistics(&job_id_for_task, "enhanced", &stats).await {
                        warn!("Failed to record failure statistics: {}", e);
                    }
                }
            }

            // Decrement active jobs counter
            {
                let mut active = active_jobs_clone.lock().await;
                *active = active.saturating_sub(1);
            }
        });

        Ok(job_id.to_string())
    }

    /// Background task for enhanced dataset generation
    async fn generate_enhanced_dataset_background(
        job_id: Uuid,
        variations_per_use_case: usize,
        output_dir: String,
        clients: Arc<ServiceClients>,
        database: Arc<crate::database::DatasetDatabase>,
        format: crate::api_config::DatasetFormat,
    ) -> Result<(String, i32, i32, Vec<String>)> {
        info!("Starting enhanced dataset generation background task for job: {}", job_id);

        // Update progress in database
        if let Err(e) = database.update_enhanced_job_status(&job_id, "processing", 0.1, Some("Initializing enhanced pipeline...")).await {
            warn!("Failed to update job progress: {}", e);
        }

        // Create enhanced pipeline
        let pipeline = EnhancedApiDatasetPipeline::new(clients);

        // Update progress
        if let Err(e) = database.update_enhanced_job_status(&job_id, "processing", 0.3, Some("Generating enhanced API examples...")).await {
            warn!("Failed to update job progress: {}", e);
        }
        
        let output_filename = match format {
            crate::api_config::DatasetFormat::SingleTurn => "single_turn_api.json",
            crate::api_config::DatasetFormat::MultiTurn => "multi_turn_api.json",
        };
        let output_path = std::path::Path::new(&output_dir).join(&output_filename);
        
        let dataset = pipeline.generate_comprehensive_dataset(
            variations_per_use_case,
            output_path.to_str().unwrap(),
            format,
        ).await
        .map_err(|e| anyhow::anyhow!("Enhanced dataset generation failed: {}", e))?;

        // Update progress
        if let Err(e) = database.update_enhanced_job_status(&job_id, "processing", 0.9, Some("Finalizing enhanced dataset...")).await {
            warn!("Failed to update job progress: {}", e);
        }

        info!("Enhanced dataset generation completed for job: {}", job_id);
        info!("Generated {} examples covering {} endpoints", 
            dataset.metadata.total_samples.unwrap_or(0), 
            0); // endpoints info not available in new metadata

        Ok((
            output_path.to_string_lossy().to_string(),
            dataset.metadata.total_samples.unwrap_or(0) as i32,
            0, // endpoints info not available in new metadata
            Vec::new(), // services_covered info not available in new metadata
        ))
    }

    /// Helper method to update job progress
    async fn update_job_progress(
        jobs: &Arc<RwLock<HashMap<String, DatasetJob>>>,
        job_id: &str,
        progress: f32,
        message: &str,
    ) {
        let mut jobs_write = jobs.write().await;
        if let Some(job) = jobs_write.get_mut(job_id) {
            job.progress = progress;
            job.message = message.to_string();
            job.updated_at = chrono::Utc::now();
        }
    }
}

#[tonic::async_trait]
impl DatasetGenerator for DatasetGeneratorService {
    async fn generate_dataset(
        &self,
        request: Request<GenerateDatasetRequest>,
    ) -> Result<Response<GenerateDatasetResponse>, Status> {
        let req = request.into_inner();
        let job_id = if req.job_id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            req.job_id
        };

        info!("Received dataset generation request for job: {}", job_id);

        // Check if we can start a new job
        if !self.can_start_job().await {
            warn!("Maximum concurrent jobs reached, rejecting job: {}", job_id);
            return Ok(Response::new(GenerateDatasetResponse {
                job_id: job_id.clone(),
                status: GenerationStatus::Failed as i32,
                message: "Maximum concurrent jobs reached. Please try again later.".to_string(),
                dataset_info: None,
            }));
        }

        // Validate the request
        if req.config.is_none() {
            return Err(Status::invalid_argument("Dataset configuration is required"));
        }

        if req.sources.is_empty() {
            return Err(Status::invalid_argument("At least one data source is required"));
        }

        let config = req.config.unwrap();
        
        // Create job entry
        let job = DatasetJob {
            id: job_id.clone(),
            status: JobStatus::Pending,
            config: config.clone(),
            sources: req.sources.clone(),
            progress: 0.0,
            message: "Job queued for processing".to_string(),
            dataset_info: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            completed_at: None,
            output_path: None,
            error: None,
        };

        // Store the job
        {
            let mut jobs = self.jobs.write().await;
            jobs.insert(job_id.clone(), job);
        }

        // Start processing in background
        let pipeline = self.pipeline.clone();
        let jobs = self.jobs.clone();
        let output_dir = self.output_dir.clone();
        let active_jobs = self.active_jobs.clone();
        let job_id_clone = job_id.clone();
        
        tokio::spawn(async move {
            // Increment active job count
            {
                let mut active = active_jobs.lock().await;
                *active += 1;
            }

            // Update job status to processing
            {
                let mut jobs_map = jobs.write().await;
                if let Some(job) = jobs_map.get_mut(&job_id_clone) {
                    job.status = JobStatus::Processing;
                    job.message = "Starting dataset generation pipeline".to_string();
                    job.updated_at = chrono::Utc::now();
                }
            }

            // Run the pipeline
            let result = pipeline.process_dataset(
                job_id_clone.clone(),
                config,
                req.sources,
                output_dir,
                jobs.clone(),
            ).await;

            // Update final job status
            {
                let mut jobs_map = jobs.write().await;
                if let Some(job) = jobs_map.get_mut(&job_id_clone) {
                    match result {
                        Ok(dataset_info) => {
                            job.status = JobStatus::Completed;
                            job.progress = 1.0;
                            job.message = "Dataset generation completed successfully".to_string();
                            job.dataset_info = Some(dataset_info);
                        }
                        Err(e) => {
                            job.status = JobStatus::Failed;
                            job.message = format!("Dataset generation failed: {}", e);
                            log::error!("Job {} failed: {}", job_id_clone, e);
                        }
                    }
                    job.updated_at = chrono::Utc::now();
                }
            }

            // Decrement active job count
            {
                let mut active = active_jobs.lock().await;
                if *active > 0 {
                    *active -= 1;
                }
            }
        });

        Ok(Response::new(GenerateDatasetResponse {
            job_id: job_id.clone(),
            status: GenerationStatus::Pending as i32,
            message: "Dataset generation job started".to_string(),
            dataset_info: None,
        }))
    }

    async fn get_generation_status(
        &self,
        request: Request<GetGenerationStatusRequest>,
    ) -> Result<Response<GetGenerationStatusResponse>, Status> {
        let req = request.into_inner();
        let job_id = req.job_id;

        let jobs = self.jobs.read().await;
        match jobs.get(&job_id) {
            Some(job) => {
                let status = match job.status {
                    JobStatus::Pending => GenerationStatus::Pending,
                    JobStatus::Running => GenerationStatus::Processing,
                    JobStatus::Processing => GenerationStatus::Processing,
                    JobStatus::Completed => GenerationStatus::Completed,
                    JobStatus::Failed => GenerationStatus::Failed,
                };

                let dataset_info = job.dataset_info.as_ref().map(|info| DatasetInfo {
                    dataset_id: info.dataset_id.clone(),
                    name: info.name.clone(),
                    description: info.description.clone(),
                    total_samples: info.total_samples,
                    train_samples: info.train_samples,
                    validation_samples: info.validation_samples,
                    test_samples: info.test_samples,
                    created_at: info.created_at.clone(),
                    output_path: info.output_path.clone(),
                    file_size_bytes: info.file_size_bytes,
                });

                Ok(Response::new(GetGenerationStatusResponse {
                    job_id: job_id.clone(),
                    status: status as i32,
                    message: job.message.clone(),
                    progress: job.progress,
                    dataset_info,
                }))
            }
            None => Err(Status::not_found(format!("Job not found: {}", job_id))),
        }
    }

    async fn list_datasets(
        &self,
        request: Request<ListDatasetsRequest>,
    ) -> Result<Response<ListDatasetsResponse>, Status> {
        let req = request.into_inner();
        let page = req.page.max(1);
        let page_size = req.page_size.min(100).max(1);

        let jobs = self.jobs.read().await;
        let completed_jobs: Vec<_> = jobs
            .values()
            .filter(|job| matches!(job.status, JobStatus::Completed))
            .collect();

        let total_count = completed_jobs.len() as u32;
        let start_idx = ((page - 1) * page_size) as usize;
        let end_idx = (start_idx + page_size as usize).min(completed_jobs.len());

        let datasets: Vec<DatasetInfo> = completed_jobs
            .get(start_idx..end_idx)
            .unwrap_or(&[])
            .iter()
            .filter_map(|job| {
                job.dataset_info.as_ref().map(|info| DatasetInfo {
                    dataset_id: info.dataset_id.clone(),
                    name: info.name.clone(),
                    description: info.description.clone(),
                    total_samples: info.total_samples,
                    train_samples: info.train_samples,
                    validation_samples: info.validation_samples,
                    test_samples: info.test_samples,
                    created_at: info.created_at.clone(),
                    output_path: info.output_path.clone(),
                    file_size_bytes: info.file_size_bytes,
                })
            })
            .collect();

        Ok(Response::new(ListDatasetsResponse {
            datasets,
            total_count,
            page,
            page_size,
        }))
    }

    async fn delete_dataset(
        &self,
        request: Request<DeleteDatasetRequest>,
    ) -> Result<Response<DeleteDatasetResponse>, Status> {
        let req = request.into_inner();
        let dataset_id = req.dataset_id;

        // Find the job with this dataset
        let mut jobs = self.jobs.write().await;
        let job_to_remove = jobs
            .iter()
            .find(|(_, job)| {
                job.dataset_info
                    .as_ref()
                    .map(|info| info.dataset_id == dataset_id)
                    .unwrap_or(false)
            })
            .map(|(job_id, _)| job_id.clone());

        match job_to_remove {
            Some(job_id) => {
                if let Some(job) = jobs.get(&job_id) {
                    if let Some(dataset_info) = &job.dataset_info {
                        // Try to delete the dataset files
                        match tokio::fs::remove_file(&dataset_info.output_path).await {
                            Ok(_) => {
                                jobs.remove(&job_id);
                                info!("Deleted dataset: {}", dataset_id);
                                Ok(Response::new(DeleteDatasetResponse {
                                    success: true,
                                    message: "Dataset deleted successfully".to_string(),
                                }))
                            }
                            Err(e) => {
                                error!("Failed to delete dataset file: {}", e);
                                Ok(Response::new(DeleteDatasetResponse {
                                    success: false,
                                    message: format!("Failed to delete dataset file: {}", e),
                                }))
                            }
                        }
                    } else {
                        Ok(Response::new(DeleteDatasetResponse {
                            success: false,
                            message: "Dataset info not available".to_string(),
                        }))
                    }
                } else {
                    Err(Status::not_found(format!("Dataset not found: {}", dataset_id)))
                }
            }
            None => Err(Status::not_found(format!("Dataset not found: {}", dataset_id))),
        }
    }

    async fn generate_enhanced_api_dataset(
        &self,
        request: Request<GenerateEnhancedApiDatasetRequest>,
    ) -> Result<Response<GenerateEnhancedApiDatasetResponse>, Status> {
        let req = request.into_inner();
        let variations_per_use_case = req.variations_per_use_case as usize;
        let format = match req.format.as_str() {
            "single_turn" => crate::api_config::DatasetFormat::SingleTurn,
            "multi_turn" => crate::api_config::DatasetFormat::MultiTurn,
            _ => crate::api_config::DatasetFormat::MultiTurn, // Default to multi-turn for backward compatibility
        };
        
        info!("Received enhanced API dataset generation request with {} variations per use case and format: {:?}", 
              variations_per_use_case, format);

        // Validate input
        if variations_per_use_case == 0 {
            return Err(Status::invalid_argument("variations_per_use_case must be greater than 0"));
        }

        if variations_per_use_case > 10 {
            return Err(Status::invalid_argument("variations_per_use_case must be 10 or less"));
        }

        // Start the enhanced dataset generation with the specified format
        match self.generate_enhanced_api_dataset_internal(variations_per_use_case, format).await {
            Ok(job_id) => {
                info!("Enhanced API dataset generation job started: {}", job_id);
                Ok(Response::new(GenerateEnhancedApiDatasetResponse {
                    job_id,
                    success: true,
                    message: "Enhanced API dataset generation job started successfully".to_string(),
                }))
            }
            Err(e) => {
                error!("Failed to start enhanced API dataset generation: {}", e);
                Ok(Response::new(GenerateEnhancedApiDatasetResponse {
                    job_id: String::new(),
                    success: false,
                    message: format!("Failed to start enhanced API dataset generation: {}", e),
                }))
            }
        }
    }
}