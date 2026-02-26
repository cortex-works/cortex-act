use crate::dataset_generator::{DatasetConfig, RawSource};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobStatus {
    Pending,
    Running,
    Processing,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
pub struct DatasetJob {
    pub id: String,
    pub status: JobStatus,
    pub config: DatasetConfig,
    pub sources: Vec<RawSource>,
    pub progress: f32,
    pub message: String,
    pub dataset_info: Option<DatasetMetadata>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub output_path: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetMetadata {
    pub dataset_id: String,
    pub name: String,
    pub description: String,
    pub total_samples: u64,
    pub train_samples: u64,
    pub validation_samples: u64,
    pub test_samples: u64,
    pub created_at: String,
    pub output_path: String,
    pub file_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessedChunk {
    pub id: String,
    pub text: String,
    pub embedding: Option<Vec<f32>>,
    pub metadata: std::collections::HashMap<String, String>,
    pub quality_score: f32,
    pub source_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QAExample {
    pub question: String,
    pub answer: String,
    pub context: String,
    pub quality_score: f32,
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetSample {
    pub id: String,
    pub input: String,
    pub output: String,
    pub instruction: Option<String>,
    pub context: Option<String>,
    pub metadata: std::collections::HashMap<String, String>,
    pub quality_score: f32,
    pub split: DatasetSplit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatasetSplit {
    Train,
    Validation,
    Test,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingStats {
    pub total_files_processed: usize,
    pub total_chunks_created: usize,
    pub total_embeddings_generated: usize,
    pub total_qa_pairs_extracted: usize,
    pub total_samples_after_filtering: usize,
    pub total_samples_final: usize,
    pub processing_time_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    pub text: String,
    pub model: String,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    pub embedding: Vec<f32>,
    pub model: String,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub prompt: String,
    pub model: String,
    pub provider: String,
    pub temperature: f32,
    pub max_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub response: String,
    pub model: String,
    pub provider: String,
} 