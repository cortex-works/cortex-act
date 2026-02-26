//! Fine-tuning engine abstractions and implementations
//!
//! This module provides the core abstractions for fine-tuning engines and their implementations.
//! Currently supports LlamaFactory and MLX engines.

use crate::error::FineTuneError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

pub mod mlx_engine;

pub use mlx_engine::MLXEngine;

/// Engine factory for creating appropriate fine-tuning engines
pub struct EngineFactory;

impl EngineFactory {
    /// Create MLX engine (the only supported engine)
    pub async fn create_engine(_model_name: &str, _prefer_mlx: bool) -> Box<dyn FineTuningEngine + Send + Sync> {
        let mlx_engine = MLXEngine::new();
        
        if !mlx_engine.is_available().await {
            tracing::warn!("⚠️  MLX engine not available, but it's the only supported engine");
        }
        
        tracing::info!("🍎 Using MLX engine (Rust-native fine-tuning)");
        Box::new(mlx_engine)
    }
    
    /// Get available engines on this system
    pub async fn get_available_engines() -> Vec<String> {
        let mut engines = Vec::new();
        
        // Check MLX availability
        let mlx_engine = MLXEngine::new();
        if mlx_engine.is_available().await {
            engines.push("MLX".to_string());
        }
        
        engines
    }
    
    /// Check if a model is supported by the Rust engine
    pub fn is_supported_model(model_name: &str) -> bool {
        // Most transformer models are supported through Candle
        let supported_patterns = [
            "llama", "mistral", "mixtral", "phi", "gemma", "qwen",
            "codellama", "vicuna", "alpaca", "orca", "bert", "gpt"
        ];
        
        let model_lower = model_name.to_lowercase();
        supported_patterns.iter().any(|pattern| model_lower.contains(pattern))
    }
}

/// Simple fine-tune request for our local engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FineTuneRequest {
    pub job_id: String,
    pub model_name: String,
    pub dataset_path: String,
    pub training_config: Option<TrainingConfig>,
    pub job_name: String,
    pub description: Option<String>,
}

/// Simple training configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfig {
    pub epochs: u32,
    pub batch_size: u32,
    pub learning_rate: f64,
}

/// Simple fine-tune result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FineTuneResult {
    pub job_id: String,
    pub model_path: String,
    pub success: bool,
    pub message: String,
    pub metrics: Option<serde_json::Value>,
}

/// Progress update for fine-tuning jobs
#[derive(Debug, Clone)]
pub struct ProgressUpdate {
    pub job_id: String,
    pub progress: f32,
    pub stage: String,
    pub message: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Trait for fine-tuning engines
#[async_trait]
pub trait FineTuningEngine: Send + Sync {
    /// Start fine-tuning with progress updates
    async fn fine_tune(
        &self,
        request: FineTuneRequest,
        progress_tx: mpsc::UnboundedSender<ProgressUpdate>,
    ) -> Result<FineTuneResult, FineTuneError>;

    /// Get engine name
    fn name(&self) -> &'static str;

    /// Check if engine is available
    async fn is_available(&self) -> bool;
}
