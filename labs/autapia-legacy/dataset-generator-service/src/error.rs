use thiserror::Error;

pub type Result<T> = std::result::Result<T, DatasetError>;

#[derive(Error, Debug, Clone)]
pub enum DatasetError {
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Pipeline error: {0}")]
    Pipeline(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    #[error("Job not found: {0}")]
    JobNotFound(String),
    
    #[error("Dataset not found: {0}")]
    DatasetNotFound(String),
    
    #[error("Service connection error: {service} - {message}")]
    ServiceConnection { service: String, message: String },
    
    #[error("Generation error: {operation} failed - {message}")]
    Generation { operation: String, message: String },
    
    #[error("Validation error: {field} - {message}")]
    Validation { field: String, message: String },
    
    #[error("Resource exhausted: {resource} - {message}")]
    ResourceExhausted { resource: String, message: String },
    
    #[error("Parsing error: {format} - {message}")]
    Parsing { format: String, message: String },
    
    #[error("File system error: {operation} - {path} - {message}")]
    FileSystem { operation: String, path: String, message: String },
    
    #[error("Network timeout: {service} after {timeout_ms}ms")]
    NetworkTimeout { service: String, timeout_ms: u64 },
    
    #[error("Rate limit exceeded: {service} - retry after {retry_after_ms}ms")]
    RateLimit { service: String, retry_after_ms: u64 },
    
    #[error("Authentication error: {service} - {message}")]
    Authentication { service: String, message: String },
    
    #[error("IO error: {0}")]
    Io(String),
    
    #[error("JSON error: {0}")]
    Json(String),
    
    #[error("UUID error: {0}")]
    Uuid(String),
    
    #[error("HTTP error: {0}")]
    Http(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

impl DatasetError {
    /// Create a database error with context
    pub fn database<S: Into<String>>(message: S) -> Self {
        DatasetError::Database(message.into())
    }
    
    /// Create a pipeline error with context
    pub fn pipeline<S: Into<String>>(message: S) -> Self {
        DatasetError::Pipeline(message.into())
    }
    
    /// Create a service connection error
    pub fn service_connection<S1: Into<String>, S2: Into<String>>(service: S1, message: S2) -> Self {
        DatasetError::ServiceConnection {
            service: service.into(),
            message: message.into(),
        }
    }
    
    /// Create a generation error
    pub fn generation<S1: Into<String>, S2: Into<String>>(operation: S1, message: S2) -> Self {
        DatasetError::Generation {
            operation: operation.into(),
            message: message.into(),
        }
    }
    
    /// Create a validation error
    pub fn validation<S1: Into<String>, S2: Into<String>>(field: S1, message: S2) -> Self {
        DatasetError::Validation {
            field: field.into(),
            message: message.into(),
        }
    }
    
    /// Create a resource exhausted error
    pub fn resource_exhausted<S1: Into<String>, S2: Into<String>>(resource: S1, message: S2) -> Self {
        DatasetError::ResourceExhausted {
            resource: resource.into(),
            message: message.into(),
        }
    }
    
    /// Create a parsing error
    pub fn parsing<S1: Into<String>, S2: Into<String>>(format: S1, message: S2) -> Self {
        DatasetError::Parsing {
            format: format.into(),
            message: message.into(),
        }
    }
    
    /// Create a file system error
    pub fn file_system<S1: Into<String>, S2: Into<String>, S3: Into<String>>(operation: S1, path: S2, message: S3) -> Self {
        DatasetError::FileSystem {
            operation: operation.into(),
            path: path.into(),
            message: message.into(),
        }
    }
    
    /// Create a network timeout error
    pub fn network_timeout<S: Into<String>>(service: S, timeout_ms: u64) -> Self {
        DatasetError::NetworkTimeout {
            service: service.into(),
            timeout_ms,
        }
    }
    
    /// Create a rate limit error
    pub fn rate_limit<S: Into<String>>(service: S, retry_after_ms: u64) -> Self {
        DatasetError::RateLimit {
            service: service.into(),
            retry_after_ms,
        }
    }
    
    /// Create an authentication error
    pub fn authentication<S1: Into<String>, S2: Into<String>>(service: S1, message: S2) -> Self {
        DatasetError::Authentication {
            service: service.into(),
            message: message.into(),
        }
    }
    
    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            DatasetError::NetworkTimeout { .. } => true,
            DatasetError::RateLimit { .. } => true,
            DatasetError::ServiceConnection { .. } => true,
            DatasetError::Http(err_msg) => {
                // Check if it's a temporary HTTP error based on error message
                err_msg.contains("timeout") || err_msg.contains("connect") || err_msg.contains("connection")
            }
            DatasetError::Database(msg) => {
                // Check for temporary database issues
                msg.contains("connection") || msg.contains("timeout")
            }
            _ => false,
        }
    }
    
    /// Get suggested retry delay in milliseconds
    pub fn retry_delay_ms(&self) -> Option<u64> {
        match self {
            DatasetError::RateLimit { retry_after_ms, .. } => Some(*retry_after_ms),
            DatasetError::NetworkTimeout { .. } => Some(1000), // 1 second
            DatasetError::ServiceConnection { .. } => Some(2000), // 2 seconds
            DatasetError::Http(_) => Some(1000), // 1 second
            DatasetError::Database(_) => Some(5000), // 5 seconds
            _ => None,
        }
    }
    
    /// Get the category of error for metrics/logging
    pub fn category(&self) -> &'static str {
        match self {
            DatasetError::Database(_) => "database",
            DatasetError::Pipeline(_) => "pipeline",
            DatasetError::Configuration(_) => "configuration",
            DatasetError::JobNotFound(_) => "not_found",
            DatasetError::DatasetNotFound(_) => "not_found", 
            DatasetError::ServiceConnection { .. } => "service_connection",
            DatasetError::Generation { .. } => "generation",
            DatasetError::Validation { .. } => "validation",
            DatasetError::ResourceExhausted { .. } => "resource_exhausted",
            DatasetError::Parsing { .. } => "parsing",
            DatasetError::FileSystem { .. } => "file_system",
            DatasetError::NetworkTimeout { .. } => "network_timeout",
            DatasetError::RateLimit { .. } => "rate_limit",
            DatasetError::Authentication { .. } => "authentication",
            DatasetError::Io(_) => "io",
            DatasetError::Json(_) => "json",
            DatasetError::Uuid(_) => "uuid",
            DatasetError::Http(_) => "http",
            DatasetError::Internal(_) => "internal",
        }
    }
}

// Convert from autapia_database_client errors
impl From<autapia_database_client::DatabaseError> for DatasetError {
    fn from(err: autapia_database_client::DatabaseError) -> Self {
        DatasetError::Database(err.to_string())
    }
}

// Convert from sqlx errors
impl From<sqlx::Error> for DatasetError {
    fn from(err: sqlx::Error) -> Self {
        DatasetError::Database(err.to_string())
    }
}

impl From<std::io::Error> for DatasetError {
    fn from(err: std::io::Error) -> Self {
        DatasetError::Io(err.to_string())
    }
}

impl From<serde_json::Error> for DatasetError {
    fn from(err: serde_json::Error) -> Self {
        DatasetError::Json(err.to_string())
    }
}

impl From<uuid::Error> for DatasetError {
    fn from(err: uuid::Error) -> Self {
        DatasetError::Uuid(err.to_string())
    }
}

impl From<reqwest::Error> for DatasetError {
    fn from(err: reqwest::Error) -> Self {
        DatasetError::Http(err.to_string())
    }
}

// Convert from tonic Status errors
impl From<tonic::Status> for DatasetError {
    fn from(status: tonic::Status) -> Self {
        let service = "grpc_service"; // Could be made more specific
        DatasetError::ServiceConnection {
            service: service.to_string(),
            message: status.message().to_string(),
        }
    }
}

// Convert DatasetError to tonic Status for gRPC responses
impl From<DatasetError> for tonic::Status {
    fn from(err: DatasetError) -> Self {
        match err {
            DatasetError::JobNotFound(msg) | DatasetError::DatasetNotFound(msg) => {
                tonic::Status::not_found(msg)
            }
            DatasetError::Validation { field, message } => {
                tonic::Status::invalid_argument(format!("Invalid {}: {}", field, message))
            }
            DatasetError::ResourceExhausted { resource, message } => {
                tonic::Status::resource_exhausted(format!("{}: {}", resource, message))
            }
            DatasetError::Authentication { message, .. } => {
                tonic::Status::unauthenticated(message)
            }
            DatasetError::Configuration(msg) => {
                tonic::Status::failed_precondition(format!("Configuration error: {}", msg))
            }
            _ => tonic::Status::internal(err.to_string()),
        }
    }
}

/// Retry configuration for operations
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub exponential_backoff: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 1000,
            max_delay_ms: 30000,
            exponential_backoff: true,
        }
    }
}

/// Retry utility for operations that may fail temporarily
pub struct RetryOperation {
    config: RetryConfig,
}

impl RetryOperation {
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }
    
    pub fn with_defaults() -> Self {
        Self {
            config: RetryConfig::default(),
        }
    }
    
    /// Execute an async operation with retry logic
    pub async fn execute<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut last_error = None;
        
        for attempt in 1..=self.config.max_attempts {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(err) => {
                    last_error = Some(err.clone());
                    
                    // Check if error is retryable
                    if !err.is_retryable() || attempt == self.config.max_attempts {
                        return Err(err);
                    }
                    
                    // Calculate delay
                    let delay_ms = if self.config.exponential_backoff {
                        std::cmp::min(
                            self.config.base_delay_ms * (2_u64.pow(attempt - 1)),
                            self.config.max_delay_ms,
                        )
                    } else {
                        err.retry_delay_ms().unwrap_or(self.config.base_delay_ms)
                    };
                    
                    log::warn!(
                        "Operation failed (attempt {}/{}): {}. Retrying in {}ms...",
                        attempt,
                        self.config.max_attempts,
                        err,
                        delay_ms
                    );
                    
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                }
            }
        }
        
        // This should never be reached, but just in case
        Err(last_error.unwrap_or_else(|| DatasetError::Internal("Retry loop failed".to_string())))
    }
}

/// Circuit breaker for service calls
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub recovery_timeout_ms: u64,
    pub half_open_max_calls: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout_ms: 30000, // 30 seconds
            half_open_max_calls: 3,
        }
    }
}

/// Error context for better debugging and monitoring
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub operation: String,
    pub service: Option<String>,
    pub job_id: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: std::collections::HashMap<String, String>,
}

impl ErrorContext {
    pub fn new<S: Into<String>>(operation: S) -> Self {
        Self {
            operation: operation.into(),
            service: None,
            job_id: None,
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        }
    }
    
    pub fn with_service<S: Into<String>>(mut self, service: S) -> Self {
        self.service = Some(service.into());
        self
    }
    
    pub fn with_job_id<S: Into<String>>(mut self, job_id: S) -> Self {
        self.job_id = Some(job_id.into());
        self
    }
    
    pub fn with_metadata<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Result extension trait for adding context
pub trait ResultExt<T> {
    fn with_context(self, context: ErrorContext) -> Result<T>;
    fn with_operation<S: Into<String>>(self, operation: S) -> Result<T>;
}

impl<T, E> ResultExt<T> for std::result::Result<T, E>
where
    E: Into<DatasetError>,
{
    fn with_context(self, context: ErrorContext) -> Result<T> {
        self.map_err(|e| {
            let err: DatasetError = e.into();
            log::error!("Error in {}: {} {:?}", context.operation, err, context);
            err
        })
    }
    
    fn with_operation<S: Into<String>>(self, operation: S) -> Result<T> {
        self.with_context(ErrorContext::new(operation))
    }
}
