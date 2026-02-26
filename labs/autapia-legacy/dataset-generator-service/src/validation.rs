use crate::api_config::{EnhancedDataset, EnhancedDatasetExample, ExampleMetadata};
use crate::error::{DatasetError, Result};
use anyhow::anyhow;
use log::{info, warn, debug, error};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

/// Comprehensive validation system for large-scale dataset generation
pub struct DatasetValidator {
    validation_config: ValidationConfig,
    metrics: ValidationMetrics,
}

/// Configuration for dataset validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Minimum number of examples per endpoint
    pub min_examples_per_endpoint: usize,
    /// Maximum number of examples per endpoint  
    pub max_examples_per_endpoint: usize,
    /// Minimum quality score threshold
    pub min_quality_score: f64,
    /// Required coverage percentage of API endpoints
    pub min_endpoint_coverage: f64,
    /// Maximum allowed duplicate content percentage
    pub max_duplicate_percentage: f64,
    /// Minimum conversation turn count
    pub min_conversation_turns: usize,
    /// Required metadata fields
    pub required_metadata_fields: Vec<String>,
    /// Content validation rules
    pub content_rules: ContentValidationRules,
    /// Performance thresholds
    pub performance_thresholds: PerformanceThresholds,
}

/// Content validation rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentValidationRules {
    /// Minimum user query length
    pub min_query_length: usize,
    /// Maximum user query length
    pub max_query_length: usize,
    /// Minimum assistant response length
    pub min_response_length: usize,
    /// Required patterns in system prompts
    pub required_system_prompt_patterns: Vec<String>,
    /// Forbidden content patterns
    pub forbidden_patterns: Vec<String>,
    /// Required tool definition fields
    pub required_tool_fields: Vec<String>,
}

/// Performance validation thresholds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceThresholds {
    /// Maximum validation time in seconds
    pub max_validation_time_seconds: f64,
    /// Maximum memory usage in MB
    pub max_memory_usage_mb: usize,
    /// Minimum examples per second during generation
    pub min_generation_rate: f64,
}

/// Validation metrics and statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationMetrics {
    pub total_examples: usize,
    pub valid_examples: usize,
    pub invalid_examples: usize,
    pub duplicate_examples: usize,
    pub endpoint_coverage: f64,
    pub quality_score_distribution: QualityDistribution,
    pub content_length_stats: ContentLengthStats,
    pub validation_errors: Vec<ValidationError>,
    pub performance_metrics: PerformanceMetrics,
}

/// Quality score distribution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityDistribution {
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub median: f64,
    pub std_dev: f64,
    pub percentiles: HashMap<u8, f64>, // 25th, 50th, 75th, 90th, 95th, 99th
}

/// Content length statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentLengthStats {
    pub query_lengths: LengthStats,
    pub response_lengths: LengthStats,
    pub conversation_turn_counts: LengthStats,
}

/// Length statistics for content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LengthStats {
    pub min: usize,
    pub max: usize,
    pub mean: f64,
    pub median: usize,
}

/// Performance metrics during validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub validation_duration_seconds: f64,
    pub memory_usage_mb: usize,
    pub examples_per_second: f64,
    pub throughput_mbps: f64,
}

/// Individual validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub error_type: ValidationErrorType,
    pub example_index: Option<usize>,
    pub field: Option<String>,
    pub message: String,
    pub severity: ErrorSeverity,
}

/// Types of validation errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationErrorType {
    MissingMetadata,
    InvalidContentLength,
    DuplicateContent,
    LowQualityScore,
    MalformedToolDefinition,
    InvalidConversationFlow,
    ForbiddenContent,
    PerformanceViolation,
    SchemaValidation,
    EndpointCoverage,
}

/// Error severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

/// Validation result summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub validation_score: f64,
    pub metrics: ValidationMetrics,
    pub recommendations: Vec<ValidationRecommendation>,
    pub validation_timestamp: String,
}

/// Validation recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRecommendation {
    pub category: RecommendationCategory,
    pub priority: RecommendationPriority,
    pub message: String,
    pub action_items: Vec<String>,
}

/// Recommendation categories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationCategory {
    QualityImprovement,
    CoverageExpansion,
    PerformanceOptimization,
    ContentDiversity,
    SchemaCompliance,
}

/// Recommendation priorities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationPriority {
    Critical,
    High,
    Medium,
    Low,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            min_examples_per_endpoint: 3,
            max_examples_per_endpoint: 100,
            min_quality_score: 0.7,
            min_endpoint_coverage: 0.95,
            max_duplicate_percentage: 0.05,
            min_conversation_turns: 4,
            required_metadata_fields: vec![
                "endpoint".to_string(),
                "service".to_string(),
                "use_case".to_string(),
                "variation".to_string(),
            ],
            content_rules: ContentValidationRules::default(),
            performance_thresholds: PerformanceThresholds::default(),
        }
    }
}

impl Default for ContentValidationRules {
    fn default() -> Self {
        Self {
            min_query_length: 10,
            max_query_length: 500,
            min_response_length: 20,
            required_system_prompt_patterns: vec![
                "API documentation".to_string(),
                "function calling".to_string(),
            ],
            forbidden_patterns: vec![
                "TODO".to_string(),
                "FIXME".to_string(),
                "placeholder".to_string(),
            ],
            required_tool_fields: vec![
                "type".to_string(),
                "function".to_string(),
            ],
        }
    }
}

impl Default for PerformanceThresholds {
    fn default() -> Self {
        Self {
            max_validation_time_seconds: 300.0, // 5 minutes
            max_memory_usage_mb: 1024, // 1GB
            min_generation_rate: 5.0, // 5 examples per second
        }
    }
}

impl DatasetValidator {
    /// Create new validator with default configuration
    pub fn new() -> Self {
        Self {
            validation_config: ValidationConfig::default(),
            metrics: ValidationMetrics::default(),
        }
    }

    /// Create validator with custom configuration
    pub fn with_config(config: ValidationConfig) -> Self {
        Self {
            validation_config: config,
            metrics: ValidationMetrics::default(),
        }
    }

    /// Perform comprehensive validation of enhanced dataset
    pub async fn validate_dataset(&mut self, dataset: &EnhancedDataset) -> Result<ValidationResult> {
        let validation_start = Instant::now();
        info!("Starting comprehensive dataset validation");
        info!("Total examples to validate: {}", dataset.multi_turn_data.as_ref().map(|d| d.len()).unwrap_or(0));

        // Reset metrics
        self.metrics = ValidationMetrics::default();
        self.metrics.total_examples = dataset.multi_turn_data.as_ref().map(|d| d.len()).unwrap_or(0);

        // Perform validation phases
        self.validate_schema(dataset).await?;
        self.validate_content_quality(dataset).await?;
        self.validate_endpoint_coverage(dataset).await?;
        self.validate_diversity(dataset).await?;
        self.validate_performance(dataset).await?;

        // Calculate validation metrics
        let validation_duration = validation_start.elapsed();
        self.metrics.performance_metrics.validation_duration_seconds = validation_duration.as_secs_f64();
        self.metrics.performance_metrics.examples_per_second = 
            self.metrics.total_examples as f64 / validation_duration.as_secs_f64();

        // Generate validation result
        let validation_score = self.calculate_validation_score();
        let is_valid = validation_score >= 0.8 && 
                      self.metrics.validation_errors.iter()
                          .filter(|e| matches!(e.severity, ErrorSeverity::Critical))
                          .count() == 0;

        let recommendations = self.generate_recommendations();

        info!("Dataset validation completed in {:.2}s", validation_duration.as_secs_f64());
        info!("Validation score: {:.3}", validation_score);
        info!("Valid examples: {}/{}", self.metrics.valid_examples, self.metrics.total_examples);

        Ok(ValidationResult {
            is_valid,
            validation_score,
            metrics: self.metrics.clone(),
            recommendations,
            validation_timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Validate dataset schema and structure
    async fn validate_schema(&mut self, dataset: &EnhancedDataset) -> Result<()> {
        debug!("Validating dataset schema and structure");

        // Get actual data length
        let actual_data_len = dataset.multi_turn_data.as_ref().map(|d| d.len()).unwrap_or(0);

        // Validate metadata presence
        if dataset.metadata.total_samples.unwrap_or(0) as usize != actual_data_len {
            self.add_error(ValidationError {
                error_type: ValidationErrorType::SchemaValidation,
                example_index: None,
                field: Some("metadata.total_samples".to_string()),
                message: format!(
                    "Metadata mismatch: expected {} examples, found {}",
                    dataset.metadata.total_samples.unwrap_or(0),
                    actual_data_len
                ),
                severity: ErrorSeverity::Critical,
            });
        }

        // Validate each example
        if let Some(examples) = &dataset.multi_turn_data {
            for (index, example) in examples.iter().enumerate() {
                self.validate_example_schema(example, index).await?;
            }
        }

        Ok(())
    }

    /// Validate individual example schema
    async fn validate_example_schema(&mut self, example: &EnhancedDatasetExample, index: usize) -> Result<()> {
        // Validate conversation structure
        if example.conversations.len() < self.validation_config.min_conversation_turns {
            self.add_error(ValidationError {
                error_type: ValidationErrorType::InvalidConversationFlow,
                example_index: Some(index),
                field: Some("conversations".to_string()),
                message: format!(
                    "Too few conversation turns: {} (minimum: {})",
                    example.conversations.len(),
                    self.validation_config.min_conversation_turns
                ),
                severity: ErrorSeverity::High,
            });
        }

        // Validate metadata fields
        let required_fields = self.validation_config.required_metadata_fields.clone();
        for required_field in &required_fields {
            if !self.has_metadata_field(&example.metadata, required_field) {
                self.add_error(ValidationError {
                    error_type: ValidationErrorType::MissingMetadata,
                    example_index: Some(index),
                    field: Some(required_field.clone()),
                    message: format!("Missing required metadata field: {}", required_field),
                    severity: ErrorSeverity::Medium,
                });
            }
        }

        // Validate tool definitions
        let required_tool_fields = self.validation_config.content_rules.required_tool_fields.clone();
        for (tool_index, tool) in example.tools.iter().enumerate() {
            for required_field in &required_tool_fields {
                if !self.has_tool_field(tool, required_field) {
                    self.add_error(ValidationError {
                        error_type: ValidationErrorType::MalformedToolDefinition,
                        example_index: Some(index),
                        field: Some(format!("tools[{}].{}", tool_index, required_field)),
                        message: format!("Missing tool field: {}", required_field),
                        severity: ErrorSeverity::High,
                    });
                }
            }
        }

        Ok(())
    }

    /// Validate content quality and patterns
    async fn validate_content_quality(&mut self, dataset: &EnhancedDataset) -> Result<()> {
        debug!("Validating content quality and patterns");

        let mut quality_scores = Vec::new();
        let mut query_lengths = Vec::new();
        let mut response_lengths = Vec::new();
        let mut turn_counts = Vec::new();

        if let Some(examples) = &dataset.multi_turn_data {
            for (index, example) in examples.iter().enumerate() {
            // Extract user queries and assistant responses
            let user_queries: Vec<_> = example.conversations.iter()
                .filter(|turn| turn.role == "user")
                .collect();
            
            let assistant_responses: Vec<_> = example.conversations.iter()
                .filter(|turn| turn.role == "assistant" && !turn.content.is_empty())
                .collect();

            // Validate query lengths
            for query in &user_queries {
                let length = query.content.len();
                query_lengths.push(length);
                
                if length < self.validation_config.content_rules.min_query_length {
                    self.add_error(ValidationError {
                        error_type: ValidationErrorType::InvalidContentLength,
                        example_index: Some(index),
                        field: Some("user_query".to_string()),
                        message: format!("Query too short: {} chars (minimum: {})", 
                                       length, self.validation_config.content_rules.min_query_length),
                        severity: ErrorSeverity::Medium,
                    });
                } else if length > self.validation_config.content_rules.max_query_length {
                    self.add_error(ValidationError {
                        error_type: ValidationErrorType::InvalidContentLength,
                        example_index: Some(index),
                        field: Some("user_query".to_string()),
                        message: format!("Query too long: {} chars (maximum: {})", 
                                       length, self.validation_config.content_rules.max_query_length),
                        severity: ErrorSeverity::Low,
                    });
                }
            }

            // Validate response lengths
            for response in &assistant_responses {
                let length = response.content.len();
                response_lengths.push(length);
                
                if length < self.validation_config.content_rules.min_response_length {
                    self.add_error(ValidationError {
                        error_type: ValidationErrorType::InvalidContentLength,
                        example_index: Some(index),
                        field: Some("assistant_response".to_string()),
                        message: format!("Response too short: {} chars (minimum: {})", 
                                       length, self.validation_config.content_rules.min_response_length),
                        severity: ErrorSeverity::Medium,
                    });
                }
            }

            // Check for forbidden patterns
            let conversation_content: Vec<String> = example.conversations.iter()
                .map(|t| t.content.clone())
                .collect();
            let all_content = format!("{} {}", 
                                    conversation_content.join(" "),
                                    example.system);
            
            let forbidden_patterns = self.validation_config.content_rules.forbidden_patterns.clone();
            for pattern in &forbidden_patterns {
                if all_content.to_lowercase().contains(&pattern.to_lowercase()) {
                    self.add_error(ValidationError {
                        error_type: ValidationErrorType::ForbiddenContent,
                        example_index: Some(index),
                        field: None,
                        message: format!("Contains forbidden pattern: {}", pattern),
                        severity: ErrorSeverity::High,
                    });
                }
            }

            // Calculate quality score for this example
            let quality_score = self.calculate_example_quality_score(example);
            quality_scores.push(quality_score);
            
            if quality_score < self.validation_config.min_quality_score {
                self.add_error(ValidationError {
                    error_type: ValidationErrorType::LowQualityScore,
                    example_index: Some(index),
                    field: None,
                    message: format!("Low quality score: {:.3} (minimum: {:.3})", 
                                   quality_score, self.validation_config.min_quality_score),
                    severity: ErrorSeverity::Medium,
                });
            }

            turn_counts.push(example.conversations.len());
        }

        }

        // Update content statistics
        self.metrics.content_length_stats = ContentLengthStats {
            query_lengths: self.calculate_length_stats(&query_lengths),
            response_lengths: self.calculate_length_stats(&response_lengths),
            conversation_turn_counts: self.calculate_length_stats(&turn_counts),
        };

        self.metrics.quality_score_distribution = self.calculate_quality_distribution(&quality_scores);

        Ok(())
    }

    /// Validate endpoint coverage across services
    async fn validate_endpoint_coverage(&mut self, dataset: &EnhancedDataset) -> Result<()> {
        debug!("Validating endpoint coverage");

        let mut endpoint_counts: HashMap<String, usize> = HashMap::new();
        let mut service_counts: HashMap<String, usize> = HashMap::new();

        // Count examples per endpoint and service
        if let Some(examples) = &dataset.multi_turn_data {
            for example in examples {
                let endpoint = &example.metadata.endpoint;
                let service = &example.metadata.service;
                
                *endpoint_counts.entry(endpoint.clone()).or_insert(0) += 1;
                *service_counts.entry(service.clone()).or_insert(0) += 1;
            }
        }

        // Calculate coverage metrics
        let total_expected_endpoints = 1; // We don't have this info in new metadata
        let covered_endpoints = endpoint_counts.len();
        let coverage_percentage = covered_endpoints as f64 / total_expected_endpoints as f64;
        
        self.metrics.endpoint_coverage = coverage_percentage;

        if coverage_percentage < self.validation_config.min_endpoint_coverage {
            self.add_error(ValidationError {
                error_type: ValidationErrorType::EndpointCoverage,
                example_index: None,
                field: None,
                message: format!(
                    "Insufficient endpoint coverage: {:.1}% (minimum: {:.1}%)",
                    coverage_percentage * 100.0,
                    self.validation_config.min_endpoint_coverage * 100.0
                ),
                severity: ErrorSeverity::Critical,
            });
        }

        // Check for under-represented endpoints
        for (endpoint, count) in &endpoint_counts {
            if *count < self.validation_config.min_examples_per_endpoint {
                self.add_error(ValidationError {
                    error_type: ValidationErrorType::EndpointCoverage,
                    example_index: None,
                    field: Some(endpoint.clone()),
                    message: format!(
                        "Endpoint {} has only {} examples (minimum: {})",
                        endpoint, count, self.validation_config.min_examples_per_endpoint
                    ),
                    severity: ErrorSeverity::Medium,
                });
            }
        }

        info!("Endpoint coverage: {:.1}% ({}/{})", 
              coverage_percentage * 100.0, covered_endpoints, total_expected_endpoints);

        Ok(())
    }

    /// Validate content diversity and detect duplicates
    async fn validate_diversity(&mut self, dataset: &EnhancedDataset) -> Result<()> {
        debug!("Validating content diversity and detecting duplicates");

        let mut content_hashes: HashMap<String, Vec<usize>> = HashMap::new();
        
        // Generate content hashes for duplicate detection
        if let Some(examples) = &dataset.multi_turn_data {
            for (index, example) in examples.iter().enumerate() {
                let content_hash = self.generate_content_hash(example);
                content_hashes.entry(content_hash).or_insert_with(Vec::new).push(index);
            }
        }

        // Count duplicates
        let mut duplicate_count = 0;
        for (hash, indices) in &content_hashes {
            if indices.len() > 1 {
                duplicate_count += indices.len() - 1; // First occurrence is not a duplicate
                
                self.add_error(ValidationError {
                    error_type: ValidationErrorType::DuplicateContent,
                    example_index: Some(indices[1]), // Point to first duplicate
                    field: None,
                    message: format!("Duplicate content found in {} examples", indices.len()),
                    severity: ErrorSeverity::Medium,
                });
            }
        }

        self.metrics.duplicate_examples = duplicate_count;
        let total_examples = dataset.multi_turn_data.as_ref().map(|d| d.len()).unwrap_or(0);
        let duplicate_percentage = if total_examples > 0 { duplicate_count as f64 / total_examples as f64 } else { 0.0 };

        if duplicate_percentage > self.validation_config.max_duplicate_percentage {
            self.add_error(ValidationError {
                error_type: ValidationErrorType::DuplicateContent,
                example_index: None,
                field: None,
                message: format!(
                    "Too many duplicates: {:.1}% (maximum: {:.1}%)",
                    duplicate_percentage * 100.0,
                    self.validation_config.max_duplicate_percentage * 100.0
                ),
                severity: ErrorSeverity::High,
            });
        }

        info!("Duplicate content: {:.1}% ({}/{})", 
              duplicate_percentage * 100.0, duplicate_count, total_examples);

        Ok(())
    }

    /// Validate performance characteristics
    async fn validate_performance(&mut self, dataset: &EnhancedDataset) -> Result<()> {
        debug!("Validating performance characteristics");

        // Check memory usage (estimated)
        let estimated_memory_mb = self.estimate_memory_usage(dataset);
        self.metrics.performance_metrics.memory_usage_mb = estimated_memory_mb;

        if estimated_memory_mb > self.validation_config.performance_thresholds.max_memory_usage_mb {
            self.add_error(ValidationError {
                error_type: ValidationErrorType::PerformanceViolation,
                example_index: None,
                field: Some("memory_usage".to_string()),
                message: format!(
                    "High memory usage: {}MB (maximum: {}MB)",
                    estimated_memory_mb,
                    self.validation_config.performance_thresholds.max_memory_usage_mb
                ),
                severity: ErrorSeverity::Medium,
            });
        }

        Ok(())
    }

    /// Calculate overall validation score
    fn calculate_validation_score(&self) -> f64 {
        let mut score = 1.0_f64;

        // Penalty for validation errors
        for error in &self.metrics.validation_errors {
            let penalty = match error.severity {
                ErrorSeverity::Critical => 0.3,
                ErrorSeverity::High => 0.15,
                ErrorSeverity::Medium => 0.05,
                ErrorSeverity::Low => 0.01,
                ErrorSeverity::Info => 0.0,
            };
            score -= penalty;
        }

        // Bonus for good coverage
        if self.metrics.endpoint_coverage >= self.validation_config.min_endpoint_coverage {
            score += 0.1;
        }

        // Bonus for low duplicate rate
        let duplicate_rate = self.metrics.duplicate_examples as f64 / self.metrics.total_examples as f64;
        if duplicate_rate <= self.validation_config.max_duplicate_percentage / 2.0 {
            score += 0.05;
        }

        score.max(0.0_f64).min(1.0_f64)
    }

    /// Generate validation recommendations
    fn generate_recommendations(&self) -> Vec<ValidationRecommendation> {
        let mut recommendations = Vec::new();

        // Coverage recommendations
        if self.metrics.endpoint_coverage < self.validation_config.min_endpoint_coverage {
            recommendations.push(ValidationRecommendation {
                category: RecommendationCategory::CoverageExpansion,
                priority: RecommendationPriority::Critical,
                message: "Increase endpoint coverage to meet minimum requirements".to_string(),
                action_items: vec![
                    "Generate more examples for under-represented endpoints".to_string(),
                    "Review API configuration for missing endpoints".to_string(),
                    "Adjust variations_per_use_case parameter".to_string(),
                ],
            });
        }

        // Quality recommendations
        let low_quality_errors = self.metrics.validation_errors.iter()
            .filter(|e| matches!(e.error_type, ValidationErrorType::LowQualityScore))
            .count();
            
        if low_quality_errors > 0 {
            recommendations.push(ValidationRecommendation {
                category: RecommendationCategory::QualityImprovement,
                priority: RecommendationPriority::High,
                message: format!("Improve quality of {} examples with low scores", low_quality_errors),
                action_items: vec![
                    "Review and enhance content generation templates".to_string(),
                    "Add more realistic conversation variations".to_string(),
                    "Improve system prompt specificity".to_string(),
                ],
            });
        }

        // Performance recommendations
        if self.metrics.performance_metrics.memory_usage_mb > 
           self.validation_config.performance_thresholds.max_memory_usage_mb {
            recommendations.push(ValidationRecommendation {
                category: RecommendationCategory::PerformanceOptimization,
                priority: RecommendationPriority::Medium,
                message: "Optimize memory usage during dataset generation".to_string(),
                action_items: vec![
                    "Implement batch processing for large datasets".to_string(),
                    "Add memory-efficient serialization".to_string(),
                    "Consider streaming validation approach".to_string(),
                ],
            });
        }

        recommendations
    }

    // Helper methods
    fn add_error(&mut self, error: ValidationError) {
        self.metrics.validation_errors.push(error);
    }

    fn has_metadata_field(&self, metadata: &ExampleMetadata, field: &str) -> bool {
        match field {
            "endpoint" => !metadata.endpoint.is_empty(),
            "service" => !metadata.service.is_empty(),
            "use_case" => !metadata.use_case.is_empty(),
            "variation" => true, // variation is always present as usize
            _ => false,
        }
    }

    fn has_tool_field(&self, tool: &crate::api_config::ToolDefinition, field: &str) -> bool {
        match field {
            "type" => !tool.r#type.is_empty(),
            "function" => !tool.function.name.is_empty(),
            _ => false,
        }
    }

    fn calculate_example_quality_score(&self, example: &EnhancedDatasetExample) -> f64 {
        let mut score = 1.0_f64;

        // Check conversation completeness
        if example.conversations.len() < 4 {
            score -= 0.2;
        }

        // Check tool definition quality
        if example.tools.is_empty() {
            score -= 0.3;
        }

        // Check system prompt quality
        if example.system.len() < 50 {
            score -= 0.1;
        }

        // Check content variety
        let unique_roles: HashSet<_> = example.conversations.iter()
            .map(|t| &t.role)
            .collect();
        if unique_roles.len() < 3 {
            score -= 0.1;
        }

        score.max(0.0_f64).min(1.0_f64)
    }

    fn calculate_length_stats(&self, lengths: &[usize]) -> LengthStats {
        if lengths.is_empty() {
            return LengthStats { min: 0, max: 0, mean: 0.0, median: 0 };
        }

        let mut sorted = lengths.to_vec();
        sorted.sort_unstable();

        LengthStats {
            min: sorted[0],
            max: sorted[sorted.len() - 1],
            mean: lengths.iter().sum::<usize>() as f64 / lengths.len() as f64,
            median: sorted[sorted.len() / 2],
        }
    }

    fn calculate_quality_distribution(&self, scores: &[f64]) -> QualityDistribution {
        if scores.is_empty() {
            return QualityDistribution {
                min: 0.0, max: 0.0, mean: 0.0, median: 0.0, std_dev: 0.0,
                percentiles: HashMap::new(),
            };
        }

        let mut sorted = scores.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mean = scores.iter().sum::<f64>() / scores.len() as f64;
        let variance = scores.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / scores.len() as f64;
        let std_dev = variance.sqrt();

        let mut percentiles = HashMap::new();
        for &p in &[25, 50, 75, 90, 95, 99] {
            let index = ((p as f64 / 100.0) * (sorted.len() - 1) as f64) as usize;
            percentiles.insert(p, sorted[index]);
        }

        QualityDistribution {
            min: sorted[0],
            max: sorted[sorted.len() - 1],
            mean,
            median: sorted[sorted.len() / 2],
            std_dev,
            percentiles,
        }
    }

    fn generate_content_hash(&self, example: &EnhancedDatasetExample) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let conversation_content: Vec<String> = example.conversations.iter()
            .map(|t| t.content.clone())
            .collect();
        let content = format!("{}{}", 
            conversation_content.join(""),
            example.system
        );

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    fn estimate_memory_usage(&self, dataset: &EnhancedDataset) -> usize {
        // Rough estimation: JSON serialized size / 4 (compression factor)
        let json_str = serde_json::to_string(dataset).unwrap_or_default();
        json_str.len() / 4 / 1024 / 1024 // Convert to MB
    }
}

impl Default for ValidationMetrics {
    fn default() -> Self {
        Self {
            total_examples: 0,
            valid_examples: 0,
            invalid_examples: 0,
            duplicate_examples: 0,
            endpoint_coverage: 0.0,
            quality_score_distribution: QualityDistribution {
                min: 0.0, max: 0.0, mean: 0.0, median: 0.0, std_dev: 0.0,
                percentiles: HashMap::new(),
            },
            content_length_stats: ContentLengthStats {
                query_lengths: LengthStats { min: 0, max: 0, mean: 0.0, median: 0 },
                response_lengths: LengthStats { min: 0, max: 0, mean: 0.0, median: 0 },
                conversation_turn_counts: LengthStats { min: 0, max: 0, mean: 0.0, median: 0 },
            },
            validation_errors: Vec::new(),
            performance_metrics: PerformanceMetrics {
                validation_duration_seconds: 0.0,
                memory_usage_mb: 0,
                examples_per_second: 0.0,
                throughput_mbps: 0.0,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_validator_creation() {
        let validator = DatasetValidator::new();
        assert_eq!(validator.validation_config.min_examples_per_endpoint, 3);
        assert_eq!(validator.validation_config.min_quality_score, 0.7);
    }

    #[tokio::test]
    async fn test_validation_config_defaults() {
        let config = ValidationConfig::default();
        assert_eq!(config.min_endpoint_coverage, 0.95);
        assert_eq!(config.max_duplicate_percentage, 0.05);
        assert!(config.required_metadata_fields.contains(&"endpoint".to_string()));
    }

    #[tokio::test]
    async fn test_quality_score_calculation() {
        let validator = DatasetValidator::new();
        // Add test for quality score calculation logic
        let scores = vec![0.8, 0.9, 0.7, 0.95, 0.85];
        let distribution = validator.calculate_quality_distribution(&scores);
        assert_eq!(distribution.min, 0.7);
        assert_eq!(distribution.max, 0.95);
        assert!((distribution.mean - 0.85).abs() < 0.01);
    }
} 