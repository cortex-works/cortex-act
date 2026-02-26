// Generated gRPC code
pub mod fine_tune {
    tonic::include_proto!("fine_tune");
}

// Modules
pub mod config;
pub mod error;
pub mod orchestrator;
pub mod service;
pub mod database;
pub mod log_forwarder;

// Client modules for communicating with other services
pub mod client;

// Training engine modules
pub mod engine;

// Re-export commonly used types
pub use fine_tune::*; 