use crate::api_config::{ApiConfiguration, EnhancedDataset};
use crate::enhanced_pipeline::EnhancedApiDatasetPipeline;
use crate::validation::{DatasetValidator, ValidationConfig, ValidationResult};
use crate::error::{DatasetError, Result};
use crate::clients::ServiceClients;
use anyhow::anyhow;
use log::{info, warn, debug, error};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};
use tokio::time::{sleep, timeout};

/// Dataset generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetGenerationConfig {
    pub service_name: String,
    pub examples_per_endpoint: usize,
    pub variations_per_use_case: usize,
    pub include_edge_cases: bool,
    pub quality_level: String,
}

/// Orchestrator for coordinated dataset generation across multiple services
pub struct DatasetOrchestrator {
    orchestration_config: OrchestrationConfig,
    service_registry: ServiceRegistry,
    pipeline: Arc<EnhancedApiDatasetPipeline>,
    validator: Arc<RwLock<DatasetValidator>>,
    generation_state: Arc<RwLock<GenerationState>>,
    coordinator_semaphore: Arc<Semaphore>,
}

/// Configuration for dataset generation orchestration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationConfig {
    /// Maximum concurrent dataset generations
    pub max_concurrent_generations: usize,
    /// Services to coordinate with
    pub target_services: Vec<String>,
    /// Quality requirements for generated datasets
    pub quality_requirements: QualityRequirements,
    /// Retry and resilience settings
    pub resilience_config: ResilienceConfig,
    /// Coordination timeouts
    pub coordination_timeouts: CoordinationTimeouts,
    /// Output and storage settings
    pub output_config: OutputConfig,
}

/// Quality requirements for orchestrated generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityRequirements {
    /// Minimum validation score across all services
    pub min_validation_score: f64,
    /// Required endpoint coverage per service
    pub min_coverage_per_service: f64,
    /// Maximum acceptable error rate
    pub max_error_rate: f64,
    /// Quality consistency requirements
    pub consistency_requirements: ConsistencyRequirements,
}

/// Consistency requirements across services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyRequirements {
    /// Maximum variance in quality scores between services
    pub max_quality_variance: f64,
    /// Required overlap in data patterns
    pub min_pattern_overlap: f64,
    /// Consistency in response formats
    pub require_format_consistency: bool,
}

/// Resilience and retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResilienceConfig {
    /// Maximum retry attempts per service
    pub max_retry_attempts: usize,
    /// Base delay between retries (exponential backoff)
    pub base_retry_delay_ms: u64,
    /// Maximum delay between retries
    pub max_retry_delay_ms: u64,
    /// Circuit breaker settings
    pub circuit_breaker: CircuitBreakerConfig,
    /// Health check intervals
    pub health_check_interval_ms: u64,
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Failure threshold to open circuit
    pub failure_threshold: usize,
    /// Success threshold to close circuit
    pub success_threshold: usize,
    /// Timeout for half-open state
    pub timeout_ms: u64,
}

/// Coordination timeout settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationTimeouts {
    /// Overall orchestration timeout
    pub total_orchestration_timeout_seconds: u64,
    /// Per-service generation timeout
    pub per_service_timeout_seconds: u64,
    /// Validation timeout per dataset
    pub validation_timeout_seconds: u64,
    /// Service discovery timeout
    pub service_discovery_timeout_seconds: u64,
}

/// Output configuration for orchestrated generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Output directory for generated datasets
    pub output_directory: String,
    /// File naming pattern
    pub file_naming_pattern: String,
    /// Whether to generate consolidated output
    pub generate_consolidated_output: bool,
    /// Compression settings
    pub compression_config: CompressionConfig,
}

/// Compression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Enable compression
    pub enabled: bool,
    /// Compression algorithm
    pub algorithm: CompressionAlgorithm,
    /// Compression level (1-9)
    pub level: u8,
}

/// Supported compression algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    Gzip,
    Zstd,
    Lz4,
}

/// Service registry for coordination
#[derive(Debug, Clone)]
pub struct ServiceRegistry {
    services: HashMap<String, ServiceInfo>,
    health_status: HashMap<String, ServiceHealth>,
}

/// Information about a registered service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub base_url: String,
    pub api_endpoints: Vec<String>,
    pub capabilities: ServiceCapabilities,
    pub priority: ServicePriority,
    pub dependencies: Vec<String>,
}

/// Service capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceCapabilities {
    pub supports_streaming: bool,
    pub supports_batch_processing: bool,
    pub max_concurrent_requests: usize,
    pub estimated_throughput_per_second: f64,
}

/// Service priority for generation ordering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServicePriority {
    Critical,
    High,
    Medium,
    Low,
}

/// Health status of a service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    pub is_healthy: bool,
    pub last_check: chrono::DateTime<chrono::Utc>,
    pub response_time_ms: Option<u64>,
    pub error_count: usize,
    pub circuit_breaker_state: CircuitBreakerState,
}

/// Circuit breaker states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

/// Current generation state across all services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationState {
    pub active_generations: HashMap<String, GenerationStatus>,
    pub completed_generations: HashMap<String, GenerationResult>,
    pub failed_generations: HashMap<String, GenerationFailure>,
    pub overall_progress: OverallProgress,
}

/// Status of an individual service generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationStatus {
    pub service_name: String,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub progress_percentage: f64,
    pub current_phase: GenerationPhase,
    pub examples_generated: usize,
    pub estimated_completion: Option<chrono::DateTime<chrono::Utc>>,
}

/// Phases of dataset generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GenerationPhase {
    Initializing,
    GeneratingExamples,
    Validating,
    OutputGeneration,
    Completed,
    Failed,
}

/// Result of completed generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationResult {
    pub service_name: String,
    pub dataset_path: String,
    pub validation_result: ValidationResult,
    pub generation_duration: Duration,
    pub examples_count: usize,
    pub quality_score: f64,
}

/// Information about failed generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationFailure {
    pub service_name: String,
    pub error_message: String,
    pub failure_time: chrono::DateTime<chrono::Utc>,
    pub retry_count: usize,
    pub is_recoverable: bool,
}

/// Overall progress across all services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverallProgress {
    pub total_services: usize,
    pub completed_services: usize,
    pub failed_services: usize,
    pub overall_percentage: f64,
    pub estimated_completion: Option<chrono::DateTime<chrono::Utc>>,
}

/// Orchestrated generation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationRequest {
    pub services: Option<Vec<String>>, // If None, uses all registered services
    pub generation_config: DatasetGenerationConfig,
    pub validation_config: ValidationConfig,
    pub priority: OrchestrationPriority,
    pub correlation_id: String,
}

/// Priority for orchestration requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrchestrationPriority {
    Urgent,
    High,
    Normal,
    Low,
}

/// Orchestration result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationResult {
    pub correlation_id: String,
    pub overall_success: bool,
    pub service_results: HashMap<String, GenerationResult>,
    pub service_failures: HashMap<String, GenerationFailure>,
    pub orchestration_duration: Duration,
    pub consolidated_dataset_path: Option<String>,
    pub quality_summary: QualitySummary,
}

/// Quality summary across all generated datasets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualitySummary {
    pub overall_score: f64,
    pub service_scores: HashMap<String, f64>,
    pub consistency_metrics: ConsistencyMetrics,
    pub recommendations: Vec<String>,
}

/// Consistency metrics across services
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyMetrics {
    pub quality_variance: f64,
    pub pattern_overlap_score: f64,
    pub format_consistency_score: f64,
}

impl Default for OrchestrationConfig {
    fn default() -> Self {
        Self {
            max_concurrent_generations: 3,
            target_services: vec![
                "embedding-service".to_string(),
                "chat-service".to_string(),
                "mcp-service".to_string(),
                "fine-tune-service".to_string(),
                "a2a-service".to_string(),
                "storage-service".to_string(),
                "vector-service".to_string(),
                "settings-service".to_string(),
            ],
            quality_requirements: QualityRequirements::default(),
            resilience_config: ResilienceConfig::default(),
            coordination_timeouts: CoordinationTimeouts::default(),
            output_config: OutputConfig::default(),
        }
    }
}

impl Default for QualityRequirements {
    fn default() -> Self {
        Self {
            min_validation_score: 0.8,
            min_coverage_per_service: 0.95,
            max_error_rate: 0.05,
            consistency_requirements: ConsistencyRequirements::default(),
        }
    }
}

impl Default for ConsistencyRequirements {
    fn default() -> Self {
        Self {
            max_quality_variance: 0.1,
            min_pattern_overlap: 0.7,
            require_format_consistency: true,
        }
    }
}

impl Default for ResilienceConfig {
    fn default() -> Self {
        Self {
            max_retry_attempts: 3,
            base_retry_delay_ms: 1000,
            max_retry_delay_ms: 30000,
            circuit_breaker: CircuitBreakerConfig::default(),
            health_check_interval_ms: 30000,
        }
    }
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout_ms: 60000,
        }
    }
}

impl Default for CoordinationTimeouts {
    fn default() -> Self {
        Self {
            total_orchestration_timeout_seconds: 3600, // 1 hour
            per_service_timeout_seconds: 900, // 15 minutes
            validation_timeout_seconds: 300, // 5 minutes
            service_discovery_timeout_seconds: 30,
        }
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            output_directory: "./generated_datasets".to_string(),
            file_naming_pattern: "{service}_dataset_{timestamp}.json".to_string(),
            generate_consolidated_output: true,
            compression_config: CompressionConfig::default(),
        }
    }
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            algorithm: CompressionAlgorithm::Gzip,
            level: 6,
        }
    }
}

impl DatasetOrchestrator {
    /// Create new orchestrator with default configuration
    pub async fn new(
        service_clients: Arc<ServiceClients>,
    ) -> Result<Self> {
        let config = OrchestrationConfig::default();
        Self::with_config(config, service_clients).await
    }

    /// Create orchestrator with custom configuration
    pub async fn with_config(
        config: OrchestrationConfig,
        service_clients: Arc<ServiceClients>,
    ) -> Result<Self> {
        info!("Initializing Dataset Orchestrator");

        let pipeline = Arc::new(EnhancedApiDatasetPipeline::new(
            service_clients,
        ));

        let validator = Arc::new(RwLock::new(DatasetValidator::new()));
        let service_registry = ServiceRegistry::new();
        let generation_state = Arc::new(RwLock::new(GenerationState::new()));
        let coordinator_semaphore = Arc::new(Semaphore::new(config.max_concurrent_generations));

        let orchestrator = Self {
            orchestration_config: config,
            service_registry,
            pipeline,
            validator,
            generation_state,
            coordinator_semaphore,
        };

        // Initialize service registry
        orchestrator.initialize_service_registry().await?;

        info!("Dataset Orchestrator initialized successfully");
        Ok(orchestrator)
    }

    /// Execute orchestrated dataset generation across multiple services
    pub async fn orchestrate_generation(
        &self,
        request: OrchestrationRequest,
    ) -> Result<OrchestrationResult> {
        let orchestration_start = Instant::now();
        let total_timeout = Duration::from_secs(self.orchestration_config.coordination_timeouts.total_orchestration_timeout_seconds);

        info!("Starting orchestrated dataset generation for correlation_id: {}", request.correlation_id);

        // Determine target services
        let target_services = request.services.unwrap_or_else(|| {
            self.orchestration_config.target_services.clone()
        });

        info!("Target services: {:?}", target_services);

        // Validate service availability
        self.validate_service_availability(&target_services).await?;

        // Initialize generation state
        self.initialize_generation_state(&target_services, &request.correlation_id).await?;

        // Execute generation with timeout
        let orchestration_result = match timeout(total_timeout, self.execute_coordinated_generation(
            target_services,
            request.generation_config,
            request.validation_config,
            request.correlation_id.clone(),
        )).await {
            Ok(result) => result?,
            Err(_) => {
                error!("Orchestration timed out after {}s", total_timeout.as_secs());
                return Err(DatasetError::generation(
                    "orchestration", 
                    "Orchestration timeout exceeded"
                ));
            }
        };

        let orchestration_duration = orchestration_start.elapsed();
        info!("Orchestration completed in {:.2}s", orchestration_duration.as_secs_f64());

        Ok(OrchestrationResult {
            correlation_id: request.correlation_id,
            overall_success: orchestration_result.overall_success,
            service_results: orchestration_result.service_results,
            service_failures: orchestration_result.service_failures,
            orchestration_duration,
            consolidated_dataset_path: orchestration_result.consolidated_dataset_path,
            quality_summary: orchestration_result.quality_summary,
        })
    }

    /// Get current orchestration status
    pub async fn get_orchestration_status(&self) -> Result<GenerationState> {
        let state = self.generation_state.read().await;
        Ok(state.clone())
    }

    /// Cancel ongoing orchestration
    pub async fn cancel_orchestration(&self, correlation_id: &str) -> Result<()> {
        info!("Cancelling orchestration: {}", correlation_id);
        
        let mut state = self.generation_state.write().await;
        
        // Collect services to cancel
        let services_to_cancel: Vec<String> = state.active_generations.iter()
            .filter_map(|(service, status)| {
                if status.start_time.to_rfc3339().contains(correlation_id) {
                    Some(service.clone())
                } else {
                    None
                }
            })
            .collect();
        
        // Mark active generations as cancelled
        for service in services_to_cancel {
            state.failed_generations.insert(service.clone(), GenerationFailure {
                service_name: service.clone(),
                error_message: "Cancelled by user request".to_string(),
                failure_time: chrono::Utc::now(),
                retry_count: 0,
                is_recoverable: false,
            });
        }
        
        state.active_generations.clear();
        
        info!("Orchestration cancelled: {}", correlation_id);
        Ok(())
    }

    /// Execute coordinated generation across services
    async fn execute_coordinated_generation(
        &self,
        target_services: Vec<String>,
        generation_config: DatasetGenerationConfig,
        validation_config: ValidationConfig,
        correlation_id: String,
    ) -> Result<OrchestrationResult> {
        let mut service_results = HashMap::new();
        let mut service_failures = HashMap::new();

        // Generate datasets for each service
        let generation_tasks = target_services.into_iter().map(|service_name| {
            let generation_config = generation_config.clone();
            let validation_config = validation_config.clone();
            let correlation_id = correlation_id.clone();
            
            async move {
                self.generate_service_dataset(
                    service_name,
                    generation_config,
                    validation_config,
                    correlation_id,
                ).await
            }
        });

        // Execute with controlled concurrency
        let results = futures::future::join_all(generation_tasks).await;

        // Process results
        for result in results {
            match result {
                Ok(generation_result) => {
                    service_results.insert(generation_result.service_name.clone(), generation_result);
                }
                Err(DatasetError::Generation { operation, message }) if message.contains("Failed generation for") => {
                    if let Some(service_name) = message.split("Failed generation for ").nth(1) {
                        let service_name = service_name.split(':').next().unwrap_or("unknown").to_string();
                        service_failures.insert(service_name.clone(), GenerationFailure {
                            service_name,
                            error_message: message,
                            failure_time: chrono::Utc::now(),
                            retry_count: 0,
                            is_recoverable: true,
                        });
                    }
                }
                Err(err) => {
                    error!("Unexpected generation error: {}", err);
                    service_failures.insert("unknown".to_string(), GenerationFailure {
                        service_name: "unknown".to_string(),
                        error_message: err.to_string(),
                        failure_time: chrono::Utc::now(),
                        retry_count: 0,
                        is_recoverable: false,
                    });
                }
            }
        }

        // Generate consolidated dataset if requested
        let consolidated_dataset_path = if self.orchestration_config.output_config.generate_consolidated_output {
            Some(self.generate_consolidated_dataset(&service_results).await?)
        } else {
            None
        };

        // Calculate quality summary
        let quality_summary = self.calculate_quality_summary(&service_results);

        // Determine overall success
        let overall_success = service_failures.is_empty() && 
                             quality_summary.overall_score >= self.orchestration_config.quality_requirements.min_validation_score;

        Ok(OrchestrationResult {
            correlation_id,
            overall_success,
            service_results,
            service_failures,
            orchestration_duration: Duration::from_secs(0), // Will be set by caller
            consolidated_dataset_path,
            quality_summary,
        })
    }

    /// Generate dataset for individual service
    async fn generate_service_dataset(
        &self,
        service_name: String,
        generation_config: DatasetGenerationConfig,
        validation_config: ValidationConfig,
        correlation_id: String,
    ) -> Result<GenerationResult> {
        let _permit = self.coordinator_semaphore.acquire().await
            .map_err(|e| DatasetError::generation("semaphore", &format!("Failed to acquire semaphore: {}", e)))?;

        let generation_start = Instant::now();
        
        info!("Starting dataset generation for service: {}", service_name);

        // Update generation state
        self.update_generation_status(&service_name, GenerationStatus {
            service_name: service_name.clone(),
            start_time: chrono::Utc::now(),
            progress_percentage: 0.0,
            current_phase: GenerationPhase::Initializing,
            examples_generated: 0,
            estimated_completion: None,
        }).await?;

        // Load API configuration for service
        let api_config = self.load_service_api_config(&service_name).await?;

        // Update progress
        self.update_generation_progress(&service_name, 10.0, GenerationPhase::GeneratingExamples).await?;

        // Generate enhanced dataset
        let dataset = self.pipeline.generate_comprehensive_dataset(generation_config.variations_per_use_case, "/tmp/temp", crate::api_config::DatasetFormat::MultiTurn).await
            .map_err(|e| DatasetError::generation("pipeline", &format!("Failed generation for {}: {}", service_name, e)))?;

        // Update progress
        self.update_generation_progress(&service_name, 70.0, GenerationPhase::Validating).await?;

        // Validate dataset
        let mut validator = self.validator.write().await;
        let validation_result = validator.validate_dataset(&dataset).await
            .map_err(|e| DatasetError::validation("dataset", &format!("Validation failed for {}: {}", service_name, e)))?;

        // Update progress
        self.update_generation_progress(&service_name, 90.0, GenerationPhase::OutputGeneration).await?;

        // Save dataset
        let dataset_path = self.save_service_dataset(&service_name, &dataset).await?;

        // Update progress
        self.update_generation_progress(&service_name, 100.0, GenerationPhase::Completed).await?;

        let generation_duration = generation_start.elapsed();
        
        let result = GenerationResult {
            service_name: service_name.clone(),
            dataset_path,
            validation_result: validation_result.clone(),
            generation_duration,
            examples_count: dataset.multi_turn_data.as_ref().map(|d| d.len()).unwrap_or(0),
            quality_score: validation_result.validation_score,
        };

        // Update completed generations
        {
            let mut state = self.generation_state.write().await;
            state.completed_generations.insert(service_name.clone(), result.clone());
            state.active_generations.remove(&service_name);
        }

        info!("Completed dataset generation for {}: {} examples, score: {:.3}", 
              service_name, result.examples_count, result.quality_score);

        Ok(result)
    }

    // Helper methods
    async fn initialize_service_registry(&self) -> Result<()> {
        // This would typically discover services from a service registry
        // For now, using static configuration
        info!("Initializing service registry with {} target services", 
              self.orchestration_config.target_services.len());
        Ok(())
    }

    async fn validate_service_availability(&self, services: &[String]) -> Result<()> {
        debug!("Validating availability of {} services", services.len());
        
        for service in services {
            // Perform health check
            let is_healthy = self.check_service_health(service).await?;
            if !is_healthy {
                return Err(DatasetError::service_connection(
                    service, 
                    &format!("Service {} is not healthy", service)
                ));
            }
        }
        
        Ok(())
    }

    async fn check_service_health(&self, service_name: &str) -> Result<bool> {
        // This would implement actual health check logic
        debug!("Checking health for service: {}", service_name);
        Ok(true) // Simplified for now
    }

    async fn initialize_generation_state(&self, services: &[String], correlation_id: &str) -> Result<()> {
        let mut state = self.generation_state.write().await;
        state.overall_progress.total_services = services.len();
        state.overall_progress.completed_services = 0;
        state.overall_progress.failed_services = 0;
        state.overall_progress.overall_percentage = 0.0;
        
        info!("Initialized generation state for {} services (correlation_id: {})", 
              services.len(), correlation_id);
        Ok(())
    }

    async fn update_generation_status(&self, service_name: &str, status: GenerationStatus) -> Result<()> {
        let mut state = self.generation_state.write().await;
        state.active_generations.insert(service_name.to_string(), status);
        Ok(())
    }

    async fn update_generation_progress(&self, service_name: &str, percentage: f64, phase: GenerationPhase) -> Result<()> {
        let mut state = self.generation_state.write().await;
        if let Some(status) = state.active_generations.get_mut(service_name) {
            status.progress_percentage = percentage;
            status.current_phase = phase;
        }
        Ok(())
    }

    async fn load_service_api_config(&self, service_name: &str) -> Result<ApiConfiguration> {
        // This would load actual API configuration for the service
        debug!("Loading API configuration for service: {}", service_name);
        
        // For now, return a minimal configuration
        Ok(ApiConfiguration::new())
    }

    async fn save_service_dataset(&self, service_name: &str, dataset: &EnhancedDataset) -> Result<String> {
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let filename = self.orchestration_config.output_config.file_naming_pattern
            .replace("{service}", service_name)
            .replace("{timestamp}", &timestamp.to_string());
        
        let file_path = format!("{}/{}", 
                               self.orchestration_config.output_config.output_directory, 
                               filename);

        // Create output directory if it doesn't exist
        if let Some(parent) = std::path::Path::new(&file_path).parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| DatasetError::file_system("create_dir_all", parent.to_string_lossy().as_ref(), &e.to_string()))?;
        }

        // Serialize and save dataset
        let empty_vec = Vec::new();
        let data_to_save = dataset.multi_turn_data.as_ref().unwrap_or(&empty_vec);
        let json_content = serde_json::to_string_pretty(data_to_save)
            .map_err(|e| DatasetError::parsing("json", &format!("Failed to serialize dataset: {}", e)))?;

        tokio::fs::write(&file_path, json_content).await
            .map_err(|e| DatasetError::file_system("write", &file_path, &e.to_string()))?;

        info!("Saved dataset for {} to: {}", service_name, file_path);
        Ok(file_path)
    }

    async fn generate_consolidated_dataset(&self, service_results: &HashMap<String, GenerationResult>) -> Result<String> {
        info!("Generating consolidated dataset from {} service results", service_results.len());

        // This would implement logic to combine datasets from multiple services
        // For now, just create a summary file
        let summary = serde_json::json!({
            "consolidated_dataset_summary": {
                "total_services": service_results.len(),
                "total_examples": service_results.values().map(|r| r.examples_count).sum::<usize>(),
                "average_quality_score": service_results.values().map(|r| r.quality_score).sum::<f64>() / service_results.len() as f64,
                "service_results": service_results
            }
        });

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let file_path = format!("{}/consolidated_dataset_{}.json", 
                               self.orchestration_config.output_config.output_directory, 
                               timestamp);

        let json_content = serde_json::to_string_pretty(&summary)
            .map_err(|e| DatasetError::parsing("json", &format!("Failed to serialize consolidated dataset: {}", e)))?;

        tokio::fs::write(&file_path, json_content).await
            .map_err(|e| DatasetError::file_system("write", &file_path, &e.to_string()))?;

        info!("Generated consolidated dataset: {}", file_path);
        Ok(file_path)
    }

    fn calculate_quality_summary(&self, service_results: &HashMap<String, GenerationResult>) -> QualitySummary {
        let service_scores: HashMap<String, f64> = service_results.iter()
            .map(|(service, result)| (service.clone(), result.quality_score))
            .collect();

        let overall_score = if service_scores.is_empty() {
            0.0
        } else {
            service_scores.values().sum::<f64>() / service_scores.len() as f64
        };

        let quality_variance = if service_scores.len() > 1 {
            let mean = overall_score;
            let variance = service_scores.values()
                .map(|score| (score - mean).powi(2))
                .sum::<f64>() / service_scores.len() as f64;
            variance.sqrt()
        } else {
            0.0
        };

        let consistency_metrics = ConsistencyMetrics {
            quality_variance,
            pattern_overlap_score: 0.8, // Placeholder
            format_consistency_score: 0.95, // Placeholder
        };

        let mut recommendations = Vec::new();
        if overall_score < self.orchestration_config.quality_requirements.min_validation_score {
            recommendations.push("Overall quality score below threshold - review generation parameters".to_string());
        }
        if quality_variance > self.orchestration_config.quality_requirements.consistency_requirements.max_quality_variance {
            recommendations.push("High variance in quality scores between services - standardize generation approach".to_string());
        }

        QualitySummary {
            overall_score,
            service_scores,
            consistency_metrics,
            recommendations,
        }
    }
}

impl ServiceRegistry {
    fn new() -> Self {
        Self {
            services: HashMap::new(),
            health_status: HashMap::new(),
        }
    }
}

impl GenerationState {
    fn new() -> Self {
        Self {
            active_generations: HashMap::new(),
            completed_generations: HashMap::new(),
            failed_generations: HashMap::new(),
            overall_progress: OverallProgress {
                total_services: 0,
                completed_services: 0,
                failed_services: 0,
                overall_percentage: 0.0,
                estimated_completion: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_orchestration_config_defaults() {
        let config = OrchestrationConfig::default();
        assert_eq!(config.max_concurrent_generations, 3);
        assert!(config.target_services.contains(&"embedding-service".to_string()));
        assert_eq!(config.quality_requirements.min_validation_score, 0.8);
    }

    #[tokio::test] 
    async fn test_generation_state_initialization() {
        let state = GenerationState::new();
        assert_eq!(state.overall_progress.total_services, 0);
        assert_eq!(state.overall_progress.completed_services, 0);
        assert!(state.active_generations.is_empty());
    }
}