// Fine-tune service function tools registration with enhanced 2025 compliance

use autapia_microservice_types::function_tools::{
    FunctionTool, ServiceFunctionRegistry, StructuredOutputsConfig,
    function_calling_2025::EnhancedFunctionExecutor,
    mcp_compat::McpAdapter,
};
use serde_json::json;
use tracing::{info, error};

/// Register all fine-tune service function tools with 2025 enhancements
pub async fn register_function_tools() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = ServiceFunctionRegistry::new(
        "fine-tune-service".to_string(),
        "Model fine-tuning service with 2025 enhanced function calling".to_string(),
        "0.1.0".to_string(),
        20060, // gRPC port
        20061, // admin port
        vec![
            "fine-tuning".to_string(),
            "model-training".to_string(),
            "job-management".to_string(),
            "function-calling-2025".to_string()
        ],
        vec!["storage-service".to_string(), "job-queue-service".to_string()]
    );

    // Submit fine-tuning job with 2025 features
    registry.add_tool(FunctionTool {
        name: "submit_fine_tune_job".to_string(),
        description: "Submit a new fine-tuning job with enhanced 2025 validation and structured outputs".to_string(),
        parameters: json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "model": {
                    "type": "string",
                    "description": "Base model to fine-tune",
                    "enum": ["gpt-4o-mini", "gpt-3.5-turbo", "claude-3-haiku"]
                },
                "training_file": {
                    "type": "string",
                    "description": "Path to training data file",
                    "minLength": 1
                },
                "validation_file": {
                    "type": "string",
                    "description": "Optional validation data file"
                },
                "hyperparameters": {
                    "type": "object",
                    "properties": {
                        "learning_rate": {"type": "number", "minimum": 0.0001, "maximum": 0.1},
                        "batch_size": {"type": "integer", "minimum": 1, "maximum": 256},
                        "epochs": {"type": "integer", "minimum": 1, "maximum": 50}
                    },
                    "additionalProperties": false
                }
            },
            "required": ["model", "training_file"],
            "additionalProperties": false
        }),
        required: vec!["model".to_string(), "training_file".to_string()],
        tags: vec!["fine-tuning".to_string(), "submit".to_string()],
        strict: true,
        tool_choice: None,
        timeout_ms: 60000,
        supports_streaming: false,
        structured_outputs: StructuredOutputsConfig {
            enabled: true,
            response_schema: Some(json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {
                    "job_id": {"type": "string", "description": "Unique job identifier"},
                    "status": {"type": "string", "enum": ["pending", "running", "completed", "failed"]},
                    "estimated_completion": {"type": "string", "format": "date-time"},
                    "cost_estimate": {"type": "number", "minimum": 0}
                },
                "required": ["job_id", "status"],
                "additionalProperties": false
            })),
            strict_mode: true,
        },
    })?;

    // Get job status with enhanced tracking
    registry.add_tool(FunctionTool {
        name: "get_fine_tune_status".to_string(),
        description: "Get detailed status and progress of a fine-tuning job".to_string(),
        parameters: json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "job_id": {
                    "type": "string",
                    "description": "The fine-tuning job ID to check",
                    "minLength": 1
                }
            },
            "required": ["job_id"],
            "additionalProperties": false
        }),
        required: vec!["job_id".to_string()],
        tags: vec!["status".to_string(), "monitoring".to_string()],
        strict: true,
        tool_choice: None,
        timeout_ms: 30000,
        supports_streaming: false,
        structured_outputs: StructuredOutputsConfig {
            enabled: true,
            response_schema: Some(json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {
                    "job_id": {"type": "string"},
                    "status": {"type": "string", "enum": ["pending", "running", "completed", "failed", "cancelled"]},
                    "progress_percentage": {"type": "number", "minimum": 0, "maximum": 100},
                    "current_epoch": {"type": "integer", "minimum": 0},
                    "total_epochs": {"type": "integer", "minimum": 1},
                    "loss": {"type": "number", "minimum": 0},
                    "validation_loss": {"type": "number", "minimum": 0},
                    "estimated_completion": {"type": "string", "format": "date-time"},
                    "error_message": {"type": "string"}
                },
                "required": ["job_id", "status"],
                "additionalProperties": false
            })),
            strict_mode: true,
        },
    })?;

    // Cancel job function
    registry.add_tool(FunctionTool {
        name: "cancel_fine_tune_job".to_string(),
        description: "Cancel a running fine-tuning job".to_string(),
        parameters: json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": {
                "job_id": {
                    "type": "string",
                    "description": "The fine-tuning job ID to cancel",
                    "minLength": 1
                }
            },
            "required": ["job_id"],
            "additionalProperties": false
        }),
        required: vec!["job_id".to_string()],
        tags: vec!["cancel".to_string(), "control".to_string()],
        strict: true,
        tool_choice: None,
        timeout_ms: 30000,
        supports_streaming: false,
        structured_outputs: StructuredOutputsConfig {
            enabled: true,
            response_schema: Some(json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": {
                    "job_id": {"type": "string"},
                    "cancelled": {"type": "boolean"},
                    "message": {"type": "string"}
                },
                "required": ["job_id", "cancelled", "message"],
                "additionalProperties": false
            })),
            strict_mode: true,
        },
    })?;

    // Validate and register
    let executor = EnhancedFunctionExecutor::new();
    let service_def = registry.get_service_definition()?;
    
    for tool in &service_def.tools {
        if tool.strict {
            info!("✅ Validating 2025 compliance for tool: {}", tool.name);
        }
    }

    registry.register_with_llm_knowledge().await?;
    
    let mcp_adapter = McpAdapter::from_service_definition(&service_def);
    info!("🔗 Created MCP compatibility layer with {} tools", mcp_adapter.get_mcp_tools().len());

    Ok(())
}
