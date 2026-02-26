use autapia_microservice_types::function_tools::{FunctionTool, StructuredOutputsConfig};
use serde_json::json;

pub fn get_function_tools() -> Vec<FunctionTool> {
    vec![
        // Admin endpoints
        FunctionTool::new("health_check", "Check service health and return operational status")
            .with_parameters(json!({}))
            .with_structured_outputs(StructuredOutputsConfig { enabled: false, response_schema: None, strict_mode: false })
            .with_tags(vec!["admin".to_string()]),
        
        FunctionTool::new("reload_config", "Reload service configuration from file")
            .with_parameters(json!({}))
            .with_structured_outputs(StructuredOutputsConfig { enabled: false, response_schema: None, strict_mode: false })
            .with_tags(vec!["admin".to_string()]),

        // Dataset generator service gRPC endpoints
        FunctionTool::new("generate_completion_dataset", "Generate a completion dataset for fine-tuning language models")
            .with_parameters(json!({
                "type": "object",
                "properties": {
                    "model_name": {
                        "type": "string",
                        "description": "Name of the model to generate dataset for"
                    },
                    "num_examples": {
                        "type": "integer",
                        "description": "Number of examples to generate"
                    },
                    "domain": {
                        "type": "string",
                        "description": "Domain/topic for the dataset"
                    },
                    "output_format": {
                        "type": "string",
                        "description": "Output format for the dataset",
                        "enum": ["jsonl", "json", "csv"]
                    }
                }
            }))
            .with_structured_outputs(StructuredOutputsConfig { enabled: false, response_schema: None, strict_mode: false })
            .with_tags(vec!["dataset".to_string(), "generation".to_string()]),
        
        FunctionTool::new("generate_chat_dataset", "Generate a chat/conversation dataset for training")
            .with_parameters(json!({
                "type": "object",
                "properties": {
                    "model_name": {
                        "type": "string",
                        "description": "Name of the model to generate dataset for"
                    },
                    "num_conversations": {
                        "type": "integer",
                        "description": "Number of conversations to generate"
                    },
                    "conversation_length": {
                        "type": "integer",
                        "description": "Average length of conversations"
                    },
                    "persona": {
                        "type": "string",
                        "description": "AI persona for the conversations"
                    }
                }
            }))
            .with_structured_outputs(StructuredOutputsConfig { enabled: false, response_schema: None, strict_mode: false })
            .with_tags(vec!["dataset".to_string(), "chat".to_string()]),
        
        FunctionTool::new("generate_real_api_dataset", "Generate a dataset based on real API schemas and endpoints")
            .with_parameters(json!({
                "type": "object",
                "properties": {
                    "api_schema_path": {
                        "type": "string",
                        "description": "Path to the API schema file"
                    },
                    "service_names": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "List of service names to include"
                    },
                    "num_examples_per_endpoint": {
                        "type": "integer",
                        "description": "Number of examples to generate per endpoint"
                    }
                }
            }))
            .with_structured_outputs(StructuredOutputsConfig { enabled: false, response_schema: None, strict_mode: false })
            .with_tags(vec!["dataset".to_string(), "api".to_string()]),
        
        FunctionTool::new("validate_dataset", "Validate a generated dataset for quality and completeness")
            .with_parameters(json!({
                "type": "object", 
                "properties": {
                    "dataset_path": {
                        "type": "string",
                        "description": "Path to the dataset file to validate"
                    },
                    "format": {
                        "type": "string",
                        "description": "Expected format of the dataset",
                        "enum": ["jsonl", "json", "csv"]
                    }
                }
            }))
            .with_structured_outputs(StructuredOutputsConfig { enabled: false, response_schema: None, strict_mode: false })
            .with_tags(vec!["dataset".to_string(), "validation".to_string()]),
        
        FunctionTool::new("convert_dataset_format", "Convert dataset from one format to another")
            .with_parameters(json!({
                "type": "object",
                "properties": {
                    "input_path": {
                        "type": "string",
                        "description": "Path to the input dataset file"
                    },
                    "output_path": {
                        "type": "string", 
                        "description": "Path for the output dataset file"
                    },
                    "input_format": {
                        "type": "string",
                        "description": "Format of the input dataset",
                        "enum": ["jsonl", "json", "csv"]
                    },
                    "output_format": {
                        "type": "string",
                        "description": "Desired format for the output dataset",
                        "enum": ["jsonl", "json", "csv"]
                    }
                }
            }))
            .with_structured_outputs(StructuredOutputsConfig { enabled: false, response_schema: None, strict_mode: false })
            .with_tags(vec!["dataset".to_string(), "conversion".to_string()]),
        
        FunctionTool::new("analyze_dataset", "Analyze dataset statistics and quality metrics")
            .with_parameters(json!({
                "type": "object",
                "properties": {
                    "dataset_path": {
                        "type": "string",
                        "description": "Path to the dataset file to analyze"
                    },
                    "metrics": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "List of metrics to compute",
                        "default": ["size", "diversity", "quality"]
                    }
                }
            }))
            .with_structured_outputs(StructuredOutputsConfig { enabled: false, response_schema: None, strict_mode: false })
            .with_tags(vec!["dataset".to_string(), "analysis".to_string()]),
    ]
}
