use thiserror::Error;
use tonic::{Code, Status};

#[derive(Error, Debug)]
pub enum FineTuneError {
    #[error("Configuration error: {0}")]
    Config(#[from] anyhow::Error),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Vector service error: {0}")]
    VectorService(String),

    #[error("Chat service error: {0}")]
    ChatService(String),

    #[error("Embedding service error: {0}")]
    EmbeddingService(String),

    #[error("Job not found: {job_id}")]
    JobNotFound { job_id: String },

    #[error("Job already exists: {job_id}")]
    JobAlreadyExists { job_id: String },

    #[error("Job in invalid state: {job_id}, current state: {current_state}")]
    InvalidJobState {
        job_id: String,
        current_state: String,
    },

    #[error("Training error: {0}")]
    Training(String),

    #[error("Engine error: {0}")]
    Engine(String),

    #[error("Dataset error: {0}")]
    Dataset(String),

    #[error("Dataset error: {0}")]
    DatasetError(String),

    #[error("Config error: {0}")]
    ConfigError(String),

    #[error("Engine error: {0}")]
    EngineError(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Model error: {0}")]
    Model(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("gRPC transport error: {0}")]
    Transport(#[from] tonic::transport::Error),

    #[error("gRPC status error: {0}")]
    Status(#[from] tonic::Status),

    #[error("Resource limit exceeded: {0}")]
    ResourceLimit(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Cancellation error: {0}")]
    Cancellation(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Candle framework error: {0}")]
    Candle(#[from] candle_core::Error),
}

impl From<FineTuneError> for Status {
    fn from(error: FineTuneError) -> Self {
        match error {
            FineTuneError::JobNotFound { job_id } => {
                Status::new(Code::NotFound, format!("Job not found: {}", job_id))
            }
            FineTuneError::JobAlreadyExists { job_id } => {
                Status::new(Code::AlreadyExists, format!("Job already exists: {}", job_id))
            }
            FineTuneError::InvalidJobState { job_id, current_state } => {
                Status::new(
                    Code::FailedPrecondition,
                    format!("Job {} is in invalid state: {}", job_id, current_state),
                )
            }
            FineTuneError::Validation(msg) => {
                Status::new(Code::InvalidArgument, format!("Validation error: {}", msg))
            }
            FineTuneError::ResourceLimit(msg) => {
                Status::new(Code::ResourceExhausted, format!("Resource limit exceeded: {}", msg))
            }
            FineTuneError::Timeout(msg) => {
                Status::new(Code::DeadlineExceeded, format!("Timeout: {}", msg))
            }
            FineTuneError::Cancellation(msg) => {
                Status::new(Code::Cancelled, format!("Cancelled: {}", msg))
            }
            FineTuneError::Database(msg) => {
                Status::new(Code::Internal, format!("Database error: {}", msg))
            }
            FineTuneError::VectorService(msg) => {
                Status::new(Code::Internal, format!("Vector service error: {}", msg))
            }
            FineTuneError::ChatService(msg) => {
                Status::new(Code::Internal, format!("Chat service error: {}", msg))
            }
            FineTuneError::EmbeddingService(msg) => {
                Status::new(Code::Internal, format!("Embedding service error: {}", msg))
            }
            FineTuneError::Training(msg) => {
                Status::new(Code::Internal, format!("Training error: {}", msg))
            }
            FineTuneError::Engine(msg) => {
                Status::new(Code::Internal, format!("Engine error: {}", msg))
            }
            FineTuneError::Dataset(msg) => {
                Status::new(Code::Internal, format!("Dataset error: {}", msg))
            }
            FineTuneError::Model(msg) => {
                Status::new(Code::Internal, format!("Model error: {}", msg))
            }
            FineTuneError::Storage(msg) => {
                Status::new(Code::Internal, format!("Storage error: {}", msg))
            }
            FineTuneError::Serialization(err) => {
                Status::new(Code::Internal, format!("Serialization error: {}", err))
            }
            FineTuneError::Transport(err) => {
                Status::new(Code::Internal, format!("Transport error: {}", err))
            }
            FineTuneError::Status(status) => status,
            FineTuneError::Config(err) => {
                Status::new(Code::Internal, format!("Configuration error: {}", err))
            }
            FineTuneError::Internal(msg) => {
                Status::new(Code::Internal, format!("Internal error: {}", msg))
            }
            FineTuneError::Candle(err) => {
                Status::new(Code::Internal, format!("Candle framework error: {}", err))
            }
            FineTuneError::DatasetError(msg) => {
                Status::new(Code::Internal, format!("Dataset error: {}", msg))
            }
            FineTuneError::ConfigError(msg) => {
                Status::new(Code::Internal, format!("Config error: {}", msg))
            }
            FineTuneError::EngineError(msg) => {
                Status::new(Code::Internal, format!("Engine error: {}", msg))
            }
            FineTuneError::IoError(msg) => {
                Status::new(Code::Internal, format!("IO error: {}", msg))
            }
        }
    }
}

impl From<std::io::Error> for FineTuneError {
    fn from(err: std::io::Error) -> Self {
        FineTuneError::IoError(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, FineTuneError>; 