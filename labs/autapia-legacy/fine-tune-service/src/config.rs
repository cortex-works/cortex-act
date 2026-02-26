use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    pub services: ServiceConfig,
    pub fine_tuning: FineTuningConfig,
    pub training: TrainingConfig,
    pub storage: StorageConfig,
    pub data_dir: String,
    pub max_memory_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub database_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub vector_service_url: String,
    pub chat_service_url: String,
    pub embedding_service_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FineTuningConfig {
    pub models_dir: String,
    pub datasets_dir: String,
    pub max_concurrent_jobs: usize,
    pub default_device: String,
    pub model_name: String, // Fixed to xLAM model
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfig {
    pub lora_rank: Option<i32>,
    pub lora_alpha: Option<i32>,
    pub use_qlora: Option<bool>,
    pub enable_qlora_comparison: Option<bool>,
    pub batch_size: Option<i32>,
    pub gradient_accumulation_steps: Option<i32>,
    pub learning_rate: Option<f64>,
    pub warmup_ratio: Option<f64>,
    pub max_seq_length: Option<i32>,
    pub save_steps: Option<i32>,
    pub logging_steps: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub artifacts_dir: String,
    pub temp_dir: String,
    pub max_dataset_size_mb: usize,
}

impl Config {
    pub fn load(config_path: Option<&str>) -> Result<Self> {
        // If a config file is provided, load from file
        if let Some(path) = config_path {
            let content = std::fs::read_to_string(path)?;
            let config: Config = serde_json::from_str(&content)?;
            return Ok(config);
        }

        // Otherwise, load from environment variables with defaults
        Ok(Config {
            database: DatabaseConfig {
                database_url: env::var("DATABASE_URL")
                    .unwrap_or_else(|_| "postgresql://postgres:root@localhost:5432/autapia".to_string()),
            },
            services: ServiceConfig {
                vector_service_url: env::var("VECTOR_SERVICE_URL")
                    .unwrap_or_else(|_| "http://127.0.0.1:20030".to_string()),
                chat_service_url: env::var("CHAT_SERVICE_URL")
                    .unwrap_or_else(|_| "http://127.0.0.1:20010".to_string()),
                embedding_service_url: env::var("EMBEDDING_SERVICE_URL")
                    .unwrap_or_else(|_| "http://127.0.0.1:20020".to_string()),
            },
            fine_tuning: FineTuningConfig {
                models_dir: env::var("MODELS_DIR")
                    .unwrap_or_else(|_| "./models".to_string()),
                datasets_dir: env::var("DATASETS_DIR")
                    .unwrap_or_else(|_| "./datasets".to_string()),
                max_concurrent_jobs: env::var("MAX_CONCURRENT_JOBS")
                    .unwrap_or_else(|_| "2".to_string())
                    .parse()
                    .unwrap_or(2),
                default_device: env::var("DEFAULT_DEVICE")
                    .unwrap_or_else(|_| {
                        // Auto-detect device based on platform
                        if cfg!(target_os = "macos") {
                            "mps".to_string()
                        } else if cfg!(feature = "cuda") {
                            "cuda".to_string()
                        } else {
                            "cpu".to_string()
                        }
                    }),
                model_name: "Salesforce/xLAM-2-1b-fc-r".to_string(), // Fixed model
            },
            training: TrainingConfig {
                lora_rank: env::var("LORA_RANK").ok().and_then(|v| v.parse().ok()),
                lora_alpha: env::var("LORA_ALPHA").ok().and_then(|v| v.parse().ok()),
                use_qlora: env::var("USE_QLORA").ok().and_then(|v| v.parse().ok()),
                enable_qlora_comparison: env::var("ENABLE_QLORA_COMPARISON").ok().and_then(|v| v.parse().ok()),
                batch_size: env::var("BATCH_SIZE").ok().and_then(|v| v.parse().ok()),
                gradient_accumulation_steps: env::var("GRADIENT_ACCUMULATION_STEPS").ok().and_then(|v| v.parse().ok()),
                learning_rate: env::var("LEARNING_RATE").ok().and_then(|v| v.parse().ok()),
                warmup_ratio: env::var("WARMUP_RATIO").ok().and_then(|v| v.parse().ok()),
                max_seq_length: env::var("MAX_SEQ_LENGTH").ok().and_then(|v| v.parse().ok()),
                save_steps: env::var("SAVE_STEPS").ok().and_then(|v| v.parse().ok()),
                logging_steps: env::var("LOGGING_STEPS").ok().and_then(|v| v.parse().ok()),
            },
            storage: StorageConfig {
                artifacts_dir: env::var("ARTIFACTS_DIR")
                    .unwrap_or_else(|_| "./artifacts".to_string()),
                temp_dir: env::var("TEMP_DIR")
                    .unwrap_or_else(|_| "./temp".to_string()),
                max_dataset_size_mb: env::var("MAX_DATASET_SIZE_MB")
                    .unwrap_or_else(|_| "1024".to_string())
                    .parse()
                    .unwrap_or(1024),
            },
            data_dir: env::var("DATA_DIR")
                .unwrap_or_else(|_| "./data".to_string()),
            max_memory_mb: env::var("MAX_MEMORY_MB")
                .unwrap_or_else(|_| "8192".to_string())
                .parse()
                .unwrap_or(8192),
        })
    }

    pub fn validate(&self) -> Result<()> {
        // Validate configuration values
        if self.fine_tuning.max_concurrent_jobs == 0 {
            return Err(anyhow::anyhow!("max_concurrent_jobs must be greater than 0"));
        }

        if self.storage.max_dataset_size_mb == 0 {
            return Err(anyhow::anyhow!("max_dataset_size_mb must be greater than 0"));
        }

        // Validate device
        match self.fine_tuning.default_device.as_str() {
            "cpu" | "cuda" | "mps" => {},
            _ => return Err(anyhow::anyhow!("Invalid device: {}", self.fine_tuning.default_device)),
        }

        Ok(())
    }

    pub fn ensure_directories(&self) -> Result<()> {
        // Create necessary directories if they don't exist
        std::fs::create_dir_all(&self.fine_tuning.models_dir)?;
        std::fs::create_dir_all(&self.fine_tuning.datasets_dir)?;
        std::fs::create_dir_all(&self.storage.artifacts_dir)?;
        std::fs::create_dir_all(&self.storage.temp_dir)?;
        
        Ok(())
    }
} 