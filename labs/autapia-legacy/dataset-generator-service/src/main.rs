#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use clap::{Parser, Subcommand};
use dotenv::dotenv;
use log::{info, error};
use tonic::transport::Server;
use tokio::net::TcpListener;
use axum::{
    routing::{get, post},
    Router,
    response::Json,
    http::StatusCode,
    extract::State,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

mod service;
mod clients;
mod pipeline;
mod types;
mod types_minimal;
mod api_config;
mod enhanced_pipeline;
mod utils;
mod database;
mod error;
mod function_tools;
mod validation;
mod orchestrator;
mod schema_extractor;
mod real_api_dataset_generator;
mod real_api_dataset_command;
mod dynamic_function_dataset_generator;
mod export_dynamic_function_dataset;

use service::DatasetGeneratorService;

#[derive(Parser, Debug, Clone)]
#[command(name = "dataset-generator-service")]
#[command(about = "Dataset Generator microservice for Autapia")]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// gRPC server address (for service mode)
    #[arg(long, env = "GRPC_ADDR", default_value = "0.0.0.0:20090")]
    pub grpc_addr: String,

    /// Database service address (legacy - now using direct database connections)
    #[arg(long, env = "DATABASE_SERVICE_ADDR", default_value = "http://127.0.0.1:20000")]
    pub database_service_addr: String,

    /// Vector service address
    #[arg(long, env = "VECTOR_SERVICE_ADDR", default_value = "http://127.0.0.1:20030")]
    pub vector_service_addr: String,

    /// Embedding service address
    #[arg(long, env = "EMBEDDING_SERVICE_ADDR", default_value = "http://127.0.0.1:20020")]
    pub embedding_service_addr: String,

    /// Chat service address
    #[arg(long, env = "CHAT_SERVICE_ADDR", default_value = "http://127.0.0.1:20010")]
    pub chat_service_addr: String,

    /// Output directory for generated datasets
    #[arg(long, env = "OUTPUT_DIR", default_value = "./datasets")]
    pub output_dir: String,

    /// Maximum concurrent jobs
    #[arg(long, env = "MAX_CONCURRENT_JOBS", default_value = "5")]
    pub max_concurrent_jobs: usize,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Run the dataset generator service (default mode)
    Serve,
    /// Export a dynamic function calling dataset that syncs with the canonical function tools registry
    ExportDynamicDataset {
        /// Number of examples to generate per function
        #[arg(long, default_value = "3")]
        examples_per_function: usize,
        
        /// Output file path
        #[arg(long, default_value = "datasets/single_turn_api.json")]
        output_file: String,
        
        /// Whether to force overwrite existing file
        #[arg(long)]
        force_overwrite: bool,
        
        /// Whether to validate the generated dataset
        #[arg(long, default_value = "true")]
        validate: bool,
    },
}

impl Default for Args {
    fn default() -> Self {
        Self {
            command: None,
            grpc_addr: "0.0.0.0:20090".to_string(),
            database_service_addr: "http://127.0.0.1:20000".to_string(),
            vector_service_addr: "http://127.0.0.1:20030".to_string(),
            embedding_service_addr: "http://127.0.0.1:20020".to_string(),
            chat_service_addr: "http://127.0.0.1:20010".to_string(),
            output_dir: "./datasets".to_string(),
            max_concurrent_jobs: 5,
        }
    }
}

impl Args {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        dotenv().ok();
        
        Ok(Self {
            command: None,
            grpc_addr: std::env::var("GRPC_ADDR").unwrap_or_else(|_| "0.0.0.0:20090".to_string()),
            database_service_addr: std::env::var("DATABASE_SERVICE_ADDR").unwrap_or_else(|_| "http://127.0.0.1:20000".to_string()),
            vector_service_addr: std::env::var("VECTOR_SERVICE_ADDR").unwrap_or_else(|_| "http://127.0.0.1:20030".to_string()),
            embedding_service_addr: std::env::var("EMBEDDING_SERVICE_ADDR").unwrap_or_else(|_| "http://127.0.0.1:20020".to_string()),
            chat_service_addr: std::env::var("CHAT_SERVICE_ADDR").unwrap_or_else(|_| "http://127.0.0.1:20010".to_string()),
            output_dir: std::env::var("OUTPUT_DIR").unwrap_or_else(|_| "./datasets".to_string()),
            max_concurrent_jobs: std::env::var("MAX_CONCURRENT_JOBS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ReloadResponse {
    success: bool,
    message: String,
    timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GenerateEnhancedDatasetRequest {
    pub variations_per_use_case: Option<usize>,
    pub job_name: Option<String>,
    pub output_format: Option<String>,
    pub include_all_services: Option<bool>,
    pub target_samples: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GenerateEnhancedDatasetResponse {
    pub success: bool,
    pub message: String,
    pub job_id: Option<String>,
    pub total_examples: Option<usize>,
    pub total_endpoints: Option<usize>,
    pub services_covered: Option<Vec<String>>,
    pub error: Option<String>,
}

type SharedConfig = Arc<RwLock<Args>>;
type SharedService = Arc<DatasetGeneratorService>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Load environment variables
    dotenv().ok();
    
    // Initialize logging
    env_logger::init();
    
    // Parse command line arguments
    let args = Args::parse();
    
    // Handle CLI commands
    if let Some(command) = &args.command {
        match command {
            Commands::ExportDynamicDataset { 
                examples_per_function, 
                output_file, 
                force_overwrite, 
                validate 
            } => {
                let export_args = export_dynamic_function_dataset::ExportDynamicFunctionDatasetArgs {
                    examples_per_function: *examples_per_function,
                    output_file: output_file.clone(),
                    force_overwrite: *force_overwrite,
                    validate: *validate,
                };
                
                return export_dynamic_function_dataset::export_dynamic_function_dataset(export_args).await;
            }
            Commands::Serve => {
                // Continue to server mode
            }
        }
    }
    
    let shared_config = Arc::new(RwLock::new(args.clone()));
    
    info!("Starting Dataset Generator Service");
    info!("gRPC server listening on: {}", args.grpc_addr);
    info!("Using direct database connection (database-per-service pattern)");
    info!("Vector service: {}", args.vector_service_addr);
    info!("Embedding service: {}", args.embedding_service_addr);
    info!("Chat service: {}", args.chat_service_addr);
    info!("Output directory: {}", args.output_dir);
    
    // Create output directory if it doesn't exist
    tokio::fs::create_dir_all(&args.output_dir).await?;
    
    // Initialize database connection
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:root@localhost:5432/autapia".to_string());
    
    // Create the service
    let service = DatasetGeneratorService::new(
        database_url,
        args.vector_service_addr,
        args.embedding_service_addr,
        args.chat_service_addr,
        args.output_dir,
        args.max_concurrent_jobs,
        shared_config.clone(),
    ).await?;
    
    let shared_service = Arc::new(service);
    
    // Register function tools with LLM Knowledge Service
    info!("📋 Registering function tools with LLM Knowledge Service...");
    let function_tools = function_tools::get_function_tools();
    let service_name = "dataset-generator-service".to_string();
    match autapia_microservice_types::function_tools::register_function_tools(
        &service_name,
        &function_tools,
    ).await {
        Ok(_) => info!("✅ Successfully registered function tools"),
        Err(e) => info!("⚠️ Failed to register function tools: {}", e),
    }

    // Parse server addresses - use 20090 for gRPC and 20091 for admin
    let grpc_addr = args.grpc_addr.parse()?;
    let admin_addr: std::net::SocketAddr = "0.0.0.0:20091".parse()?;
    
    info!("🚀 Starting Dataset Generator Service");
    info!("📡 gRPC server listening on {}", grpc_addr);
    info!("🔧 Admin server listening on {}", admin_addr);

    // Admin HTTP server with enhanced dataset generation endpoint
    let admin_router = Router::new()
        .route("/admin/health", get(health_check))
        .route("/admin/reload", post(reload_config_handler))
        .route("/admin/generate-enhanced-dataset", post(generate_enhanced_dataset_handler))
        .with_state((shared_config.clone(), shared_service.clone()));

    // gRPC server
    let grpc_service_clone = shared_service.clone();
    let grpc_server = async move {
        Server::builder()
            .add_service(dataset_generator::dataset_generator_server::DatasetGeneratorServer::new((*grpc_service_clone).clone()))
            .serve(grpc_addr)
            .await
    };
    
    // Admin HTTP server  
    let admin_server = async move {
        axum::serve(
            TcpListener::bind(admin_addr).await?,
            admin_router
        ).await
    };

    // Run both servers concurrently
    let grpc_result = tokio::spawn(grpc_server);
    let admin_result = tokio::spawn(admin_server);
    
    let _ = tokio::try_join!(grpc_result, admin_result)?;
    Ok(())
}

// Admin endpoint handlers
async fn health_check() -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "status": "healthy",
        "service": "dataset-generator-service",
        "version": "0.1.0",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

async fn reload_config_handler(
    State((config, _service)): State<(SharedConfig, SharedService)>,
) -> Result<Json<ReloadResponse>, (StatusCode, Json<ReloadResponse>)> {
    info!("📡 Admin reload request received for dataset-generator-service");
    
    match reload_configuration(config).await {
        Ok(message) => {
            Ok(Json(ReloadResponse {
                success: true,
                message,
                timestamp: chrono::Utc::now().to_rfc3339(),
            }))
        }
        Err(e) => {
            let error_response = ReloadResponse {
                success: false,
                message: e.to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}

async fn generate_enhanced_dataset_handler(
    State((_config, service)): State<(SharedConfig, SharedService)>,
    Json(request): Json<GenerateEnhancedDatasetRequest>,
) -> Result<Json<GenerateEnhancedDatasetResponse>, (StatusCode, Json<GenerateEnhancedDatasetResponse>)> {
    info!("🚀 Enhanced dataset generation request received");
    info!("📊 Request: variations_per_use_case={:?}, target_samples={:?}, output_format={:?}", 
          request.variations_per_use_case, request.target_samples, request.output_format);
    
    let variations_per_use_case = request.variations_per_use_case.unwrap_or(2); // Reduced from 3 to 2 for smaller datasets
    
    // Map output format to DatasetFormat
    let format = match request.output_format.as_deref() {
        Some("jsonl_training") => crate::api_config::DatasetFormat::SingleTurn,
        Some("sharegpt") => crate::api_config::DatasetFormat::MultiTurn,
        Some("mixed_format") => crate::api_config::DatasetFormat::MultiTurn, // For now, mixed defaults to multi-turn
        _ => crate::api_config::DatasetFormat::MultiTurn, // Default to multi-turn
    };
    
    info!("🎯 Using dataset format: {:?}", format);
    
    match service.generate_enhanced_api_dataset_internal(variations_per_use_case, format).await {
        Ok(job_id) => {
            info!("✅ Enhanced dataset generation started with job ID: {}", job_id);
            
            // For now, return immediate success with expected values
            // In production, you'd track the job and return actual results
            let total_endpoints = 85; // Our comprehensive API configuration
            let estimated_examples = variations_per_use_case * total_endpoints;
            
            Ok(Json(GenerateEnhancedDatasetResponse {
                success: true,
                message: format!("Enhanced API dataset generation started with {} variations per use case", variations_per_use_case),
                job_id: Some(job_id),
                total_examples: Some(estimated_examples),
                total_endpoints: Some(total_endpoints),
                services_covered: Some(vec![
                    "a2a-service".to_string(),
                    "chat-service".to_string(),
                    "dataset-generator-service".to_string(),
                    "embedding-service".to_string(),
                    "fine-tune-service".to_string(),
                    "mcp-service".to_string(),
                    "storage-service".to_string(),
                    "vector-service".to_string(),
                    "settings-service".to_string(),
                    "monitoring-service".to_string(),
                    "autapia-core-api".to_string(),
                    "llm-knowledge-service".to_string(),
                    "workflow-service".to_string(),
                    "planner-service".to_string(),
                ]),
                error: None,
            }))
        }
        Err(e) => {
            error!("❌ Enhanced dataset generation failed: {}", e);
            
            let error_response = GenerateEnhancedDatasetResponse {
                success: false,
                message: "Enhanced dataset generation failed".to_string(),
                job_id: None,
                total_examples: None,
                total_endpoints: None,
                services_covered: None,
                error: Some(e.to_string()),
            };
            
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)))
        }
    }
}

async fn reload_configuration(config: SharedConfig) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    info!("🔄 Reloading dataset-generator-service configuration...");
    
    // Load fresh configuration from environment variables
    let new_config = Args::from_env()?;
    
    // Update the shared configuration
    {
        let mut config_guard = config.write().await;
        *config_guard = new_config.clone();
    }
    
    // Ensure output directory exists
    tokio::fs::create_dir_all(&new_config.output_dir).await?;
    
    info!("✅ Configuration reloaded successfully");
    info!("📁 Output directory: {}", new_config.output_dir);
    info!("🔗 Vector service: {}", new_config.vector_service_addr);
    info!("🔗 Embedding service: {}", new_config.embedding_service_addr);
    info!("🔗 Chat service: {}", new_config.chat_service_addr);
    info!("⚙️ Max concurrent jobs: {}", new_config.max_concurrent_jobs);
    
    Ok("Dataset generator configuration reloaded successfully".to_string())
}

// Include the generated protobuf code
pub mod dataset_generator {
    tonic::include_proto!("dataset_generator");
}