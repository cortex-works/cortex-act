#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use clap::{Arg, Command};
use tracing::{info, warn, error};
use tonic::{transport::Server, Request};
use std::sync::Arc;
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};

mod config;
mod database;
mod error;
mod orchestrator;
mod service;
mod log_forwarder;
// mod function_tools;

// Client modules for communicating with other services
mod client;

// Training engine modules
mod engine;

// Generated gRPC code
pub mod fine_tune {
    tonic::include_proto!("fine_tune");
}

use config::Config;
use service::FineTuneService;
use fine_tune::{
    fine_tune_server::{FineTuneServer, FineTune},
    CancelRequest,
};

#[derive(Debug, Serialize, Deserialize)]
struct ReloadResponse {
    success: bool,
    message: String,
    timestamp: String,
}

// Admin HTTP endpoints
async fn reload_config_handler(
    State(service): State<Arc<FineTuneService>>,
) -> Result<Json<ReloadResponse>, (StatusCode, Json<ReloadResponse>)> {
    info!("📡 Admin reload request received for fine-tune service");
    
    match service.reload_config().await {
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

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "fine-tune-service",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

fn create_admin_router(service: Arc<FineTuneService>) -> Router {
    Router::new()
        .route("/admin/reload", post(reload_config_handler))
        .route("/admin/health", axum::routing::get(health_check))
        .with_state(service)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let matches = Command::new("fine-tune-service")
        .version("1.0")
        .about("Fine-tuning microservice")
        .subcommand(
            Command::new("serve")
                .about("Start the gRPC server")
        )
        .subcommand(
            Command::new("cancel-stuck")
                .about("Cancel all stuck jobs")
        )
        .subcommand(
            Command::new("cancel")
                .about("Cancel a specific job")
                .arg(
                    Arg::new("job_id")
                        .help("Job ID to cancel")
                        .required(true)
                        .index(1)
                )
                .arg(
                    Arg::new("reason")
                        .help("Reason for cancellation")
                        .short('r')
                        .long("reason")
                        .default_value("Manual cancellation")
                )
        )
        .get_matches();

    // Load configuration
    let config = Config::load(None)?;
    info!("Loaded configuration: {:?}", config);

    // Create service with reload capability
    let service = Arc::new(FineTuneService::new_with_reload(config.clone()).await?);

    match matches.subcommand() {
        Some(("serve", _)) => {
            start_servers(service).await?;
        }
        Some(("cancel-stuck", _)) => {
            // Cancel all stuck jobs
            info!("Cancelling all stuck jobs...");
            match service.cancel_stuck_jobs().await {
                Ok(cancelled_jobs) => {
                    if cancelled_jobs.is_empty() {
                        info!("✅ No stuck jobs found");
                    } else {
                        info!("✅ Cancelled {} stuck jobs: {:?}", cancelled_jobs.len(), cancelled_jobs);
                    }
                }
                Err(e) => {
                    error!("❌ Failed to cancel stuck jobs: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(("cancel", sub_matches)) => {
            // Cancel specific job
            let job_id = sub_matches.get_one::<String>("job_id").unwrap();
            let reason = sub_matches.get_one::<String>("reason").unwrap();
            
            info!("Cancelling job {} (reason: {})", job_id, reason);
            
            let request = Request::new(CancelRequest {
                job_id: job_id.clone(),
                reason: reason.clone(),
            });
            
            match service.cancel_job(request).await {
                Ok(response) => {
                    let response = response.into_inner();
                    if response.cancelled {
                        info!("✅ {}", response.message);
                    } else {
                        warn!("⚠️ {}", response.message);
                    }
                }
                Err(e) => {
                    error!("❌ Failed to cancel job: {}", e);
                    std::process::exit(1);
                }
            }
        }
        _ => {
            // Default to serve
            start_servers(service).await?;
        }
    }

    Ok(())
}

async fn start_servers(service: Arc<FineTuneService>) -> Result<(), Box<dyn std::error::Error>> {
    // gRPC server address
    let grpc_addr = "127.0.0.1:20080".parse()?;
    
    // Admin HTTP server (on port 20081 - 1 port higher than gRPC)
    let admin_addr = std::net::SocketAddr::from(([127, 0, 0, 1], 20081));
    let admin_router = create_admin_router(service.clone());
    
    // Display service startup banner
    info!("");
    info!("🚀 \x1b[1;32m═══════════════════════════════════════════════════════════\x1b[0m");
    info!("🚀 \x1b[1;32m          FINE-TUNE SERVICE WITH LLAMA FACTORY\x1b[0m");
    info!("🚀 \x1b[1;32m═══════════════════════════════════════════════════════════\x1b[0m");
    info!("🌐 gRPC Server: \x1b[1;36m{}\x1b[0m", grpc_addr);
    info!("🔧 Admin HTTP: \x1b[1;36m{}\x1b[0m", admin_addr);
    info!("⚡ Engine: \x1b[1;33mLLaMA Factory E5/Universal Engine\x1b[0m");
    info!("💻 Optimized for: \x1b[1;35mApple Silicon M4 Pro\x1b[0m");
    info!("📊 Progress: \x1b[1;32mReal-time tracking in this terminal\x1b[0m");
    info!("🔄 Ready to accept fine-tuning jobs...");
    info!("🚀 \x1b[1;32m═══════════════════════════════════════════════════════════\x1b[0m");
    info!("");

    // Register function tools with LLM Knowledge Service
    // match function_tools::register_function_tools().await {
    //     Ok(_) => info!("✅ Successfully registered function tools with LLM Knowledge Service"),
    //     Err(e) => error!("❌ Failed to register function tools: {}", e),
    // }

    // Start both servers concurrently
    let grpc_server = {
        let service_for_grpc = service.clone();
        async move {
            Server::builder()
                .add_service(FineTuneServer::new(service_for_grpc.as_ref().clone()))
                .serve(grpc_addr)
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
        }
    };
    
    let admin_server = async {
        axum::serve(
            tokio::net::TcpListener::bind(admin_addr).await?,
            admin_router
        )
        .await
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    };
    
    // Run both servers concurrently
    tokio::try_join!(grpc_server, admin_server)?;
    
    Ok(())
}