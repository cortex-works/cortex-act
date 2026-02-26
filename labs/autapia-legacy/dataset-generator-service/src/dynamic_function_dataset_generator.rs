use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use rand::Rng;
use log::{info, warn, debug};
use std::collections::HashMap;

// Import the shared function tools module
use autapia_microservice_types::function_tools::{
    ServiceDefinition, FunctionTool, ServiceFunctionRegistry, StructuredOutputsConfig,
    chat_service, rag_service, vector_service, embedding_service, mcp_service
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiftyOneApiExample {
    pub query: Vec<String>,
    pub tools: Vec<String>,
    pub answers: Vec<String>,
}

/// Dynamic dataset generator that always stays in sync with the canonical function tools registry
pub struct DynamicFunctionDatasetGenerator {
    rng: rand::rngs::ThreadRng,
}

impl DynamicFunctionDatasetGenerator {
    pub fn new() -> Self {
        Self {
            rng: rand::thread_rng(),
        }
    }

    /// Generate a complete dataset that covers ALL function tools from the registry
    pub fn generate_complete_dataset(&mut self, examples_per_function: usize) -> Vec<FiftyOneApiExample> {
        let mut examples = Vec::new();
        
        info!("Generating dynamic function dataset from canonical registry");

        // Get all service registries from the shared function tools
        let service_registries = self.get_all_service_registries();
        
        let mut total_functions = 0;
        for registry in &service_registries {
            total_functions += registry.function_tools.len();
        }
        
        info!("Found {} services with {} total functions", service_registries.len(), total_functions);

        // Generate examples for each function in each service
        for registry in &service_registries {
            info!("Processing service: {} with {} functions", 
                  registry.service_name, registry.function_tools.len());
            
            for function_tool in &registry.function_tools {
                debug!("Generating examples for function: {}.{}", 
                      registry.service_name, function_tool.name);
                
                // Generate multiple examples for each function to ensure coverage
                for i in 0..examples_per_function {
                    if let Some(example) = self.generate_function_example(&registry, function_tool, i) {
                        examples.push(example);
                    }
                }
            }
        }

        info!("Successfully generated {} examples covering {} functions from {} services", 
              examples.len(), total_functions, service_registries.len());
        
        examples
    }

    /// Get all service registries from the shared function tools module
    pub fn get_all_service_registries(&self) -> Vec<ServiceFunctionRegistry> {
        let mut registries = vec![
            // Core microservices with defined registries in shared module
            chat_service::create_registry(),
            rag_service::create_registry(),
            vector_service::create_registry(),
            embedding_service::create_registry(),
            mcp_service::create_registry(),
        ];

        // Add additional services that may not have shared registries yet
        registries.extend(self.get_additional_service_registries());
        
        registries
    }

    /// Get additional service registries for services not yet in shared module
    fn get_additional_service_registries(&self) -> Vec<ServiceFunctionRegistry> {
        vec![
            self.create_config_service_registry(),
            self.create_monitor_service_registry(),
            self.create_settings_service_registry(),
        ]
    }

    /// Create config service registry
    fn create_config_service_registry(&self) -> ServiceFunctionRegistry {
        let mut registry = ServiceFunctionRegistry::new(
            "config-service".to_string(),
            "Configuration management service for centralized settings".to_string(),
            "0.1.0".to_string(),
            20070,
            20071,
            vec!["config".to_string(), "settings".to_string()],
            vec![],
        );

        registry.add_tool(FunctionTool::new("get_config", "Get configuration settings for a service")
            .with_parameters(json!({
                "type": "object",
                "properties": {
                    "service_name": {
                        "type": "string",
                        "description": "Name of the service to get config for"
                    },
                    "config_key": {
                        "type": "string", 
                        "description": "Specific configuration key to retrieve"
                    }
                },
                "required": ["service_name"]
            }))
            .with_structured_outputs(StructuredOutputsConfig { enabled: false, response_schema: None, strict_mode: false })
            .with_tags(vec!["config".to_string(), "settings".to_string()]));

        registry
    }

    /// Create monitor service registry
    fn create_monitor_service_registry(&self) -> ServiceFunctionRegistry {
        let mut registry = ServiceFunctionRegistry::new(
            "monitor-service".to_string(),
            "System monitoring and health checking service".to_string(),
            "0.1.0".to_string(),
            20110,
            20111,
            vec!["monitoring".to_string(), "health".to_string()],
            vec![],
        );

        registry.add_tool(FunctionTool::new("get_system_metrics", "Get current system performance metrics")
            .with_parameters(json!({
                "type": "object",
                "properties": {
                    "service_name": {
                        "type": "string",
                        "description": "Specific service to monitor"
                    },
                    "metric_type": {
                        "type": "string",
                        "description": "Type of metrics (cpu, memory, disk, network)"
                    }
                }
            }))
            .with_structured_outputs(StructuredOutputsConfig { enabled: false, response_schema: None, strict_mode: false })
            .with_tags(vec!["monitoring".to_string(), "metrics".to_string()]));

        registry
    }

    /// Create settings service registry
    fn create_settings_service_registry(&self) -> ServiceFunctionRegistry {
        let mut registry = ServiceFunctionRegistry::new(
            "settings-service".to_string(),
            "Centralized settings and configuration management".to_string(),
            "0.1.0".to_string(),
            20120,
            20121,
            vec!["settings".to_string(), "configuration".to_string()],
            vec![],
        );

        registry.add_tool(FunctionTool::new("get_settings", "Get system settings and preferences")
            .with_parameters(json!({
                "type": "object",
                "properties": {
                    "category": {
                        "type": "string",
                        "description": "Settings category (user, system, service)"
                    },
                    "setting_key": {
                        "type": "string",
                        "description": "Specific setting key to retrieve"
                    }
                }
            }))
            .with_structured_outputs(StructuredOutputsConfig { enabled: false, response_schema: None, strict_mode: false })
            .with_tags(vec!["settings".to_string(), "preferences".to_string()]));

        registry
    }

    /// Generate a realistic example for a specific function
    fn generate_function_example(
        &mut self, 
        registry: &ServiceFunctionRegistry, 
        function_tool: &FunctionTool,
        variant: usize
    ) -> Option<FiftyOneApiExample> {
        // Generate a realistic user query based on the function
        let user_query = self.generate_user_query_for_function(function_tool, variant);
        
        // Create the tool definition in the expected format
        let tool_definition = self.create_hf_tool_definition(function_tool);

        // Generate realistic function call arguments based on the schema
        let function_arguments = self.generate_function_arguments(function_tool, variant);

        // Create the answer (function call)
        // Note: FunctionCall.arguments should be a JSON string, not a JSON object
        let answer = json!({
            "name": function_tool.name,
            "arguments": serde_json::to_string(&function_arguments).unwrap_or_default()
        });

        let example = FiftyOneApiExample {
            query: vec![user_query],
            tools: vec![serde_json::to_string(&tool_definition).unwrap_or_default()],
            answers: vec![serde_json::to_string(&answer).unwrap_or_default()],
        };

        Some(example)
    }

    /// Create a Hugging Face compatible tool definition
    fn create_hf_tool_definition(&self, function_tool: &FunctionTool) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": function_tool.name,
                "description": function_tool.description,
                "parameters": function_tool.parameters
            }
        })
    }

    /// Generate realistic function arguments based on the function schema
    fn generate_function_arguments(&mut self, function_tool: &FunctionTool, variant: usize) -> Value {
        let mut arguments = json!({});

        if let Some(properties) = function_tool.parameters.get("properties").and_then(|p| p.as_object()) {
            for (param_name, param_schema) in properties {
                // Always include required parameters
                let is_required = function_tool.required.contains(&param_name.to_string());
                
                // For optional parameters, include them based on variant to create diversity
                let should_include = is_required || (variant % 3 != 0); // Include ~66% of optional params
                
                if should_include {
                    if let Some(sample_value) = self.generate_sample_parameter_value(param_name, param_schema, variant) {
                        arguments[param_name] = sample_value;
                    }
                }
            }
        }

        arguments
    }

    /// Generate sample parameter values based on type, name, and variant
    fn generate_sample_parameter_value(&mut self, param_name: &str, param_schema: &Value, variant: usize) -> Option<Value> {
        let param_type = param_schema.get("type")?.as_str().unwrap_or("string");
        
        match param_type {
            "string" => {
                let sample_value = match param_name {
                    // Context-aware string generation based on parameter names
                    name if name.contains("id") => format!("id_{}", 1000 + (variant * 123) % 8999),
                    name if name.contains("name") => format!("resource_name_{}", variant + 1),
                    name if name.contains("collection") => match variant % 4 {
                        0 => "documents".to_string(),
                        1 => "embeddings".to_string(),
                        2 => "chat_history".to_string(),
                        _ => "user_data".to_string(),
                    },
                    name if name.contains("prompt") || name.contains("query") => match variant % 5 {
                        0 => "Explain machine learning concepts".to_string(),
                        1 => "What is the meaning of life?".to_string(),
                        2 => "Write a short story about AI".to_string(),
                        3 => "Help me debug this code".to_string(),
                        _ => "Create a data analysis report".to_string(),
                    },
                    name if name.contains("model") => match variant % 3 {
                        0 => "gpt-4".to_string(),
                        1 => "claude-3".to_string(),
                        _ => "mixtral-8x7b".to_string(),
                    },
                    name if name.contains("path") || name.contains("file") => match variant % 3 {
                        0 => "/data/input.json".to_string(),
                        1 => "/projects/ml_model.pkl".to_string(),
                        _ => "/workspace/dataset.csv".to_string(),
                    },
                    name if name.contains("url") => "https://api.example.com/endpoint".to_string(),
                    name if name.contains("format") => match variant % 3 {
                        0 => "json".to_string(),
                        1 => "csv".to_string(),
                        _ => "parquet".to_string(),
                    },
                    name if name.contains("status") => match variant % 2 {
                        0 => "active".to_string(),
                        _ => "pending".to_string(),
                    },
                    name if name.contains("tool_name") => match variant % 4 {
                        0 => "file_search".to_string(),
                        1 => "web_search".to_string(),
                        2 => "code_analyzer".to_string(),
                        _ => "data_processor".to_string(),
                    },
                    name if name.contains("distance") => match variant % 3 {
                        0 => "cosine".to_string(),
                        1 => "euclidean".to_string(),
                        _ => "dot".to_string(),
                    },
                    _ => format!("sample_{}", param_name),
                };
                Some(json!(sample_value))
            },
            "integer" => {
                let sample_value = match param_name {
                    name if name.contains("dimension") => match variant % 3 {
                        0 => 768,
                        1 => 1536,
                        _ => 384,
                    },
                    name if name.contains("limit") || name.contains("count") => (variant % 20 + 1) * 5,
                    name if name.contains("port") => 8000 + (variant % 1000),
                    name if name.contains("id") => 1000 + (variant * 43) % 9000,
                    name if name.contains("max_tokens") => match variant % 4 {
                        0 => 100,
                        1 => 500,
                        2 => 1000,
                        _ => 2048,
                    },
                    name if name.contains("timeout") => (variant % 10 + 1) * 5000,
                    name if name.contains("offset") => variant * 10,
                    _ => (variant % 50) + 1,
                };
                Some(json!(sample_value))
            },
            "number" | "float" => {
                let sample_value = match param_name {
                    name if name.contains("temperature") => (variant as f64 % 20.0) / 20.0, // 0.0 to 1.0
                    name if name.contains("threshold") || name.contains("score") => ((variant % 80 + 10) as f64) / 100.0, // 0.1 to 0.9
                    name if name.contains("ratio") => ((variant % 70 + 10) as f64) / 100.0, // 0.1 to 0.8
                    _ => (variant as f64 % 10.0) + 0.1,
                };
                Some(json!(sample_value))
            },
            "boolean" => {
                // Vary boolean values based on parameter name and variant
                let value = match param_name {
                    name if name.contains("with_payload") => variant % 3 != 0, // Usually true
                    name if name.contains("with_vector") => variant % 4 != 0, // Usually true
                    name if name.contains("use_raw") => variant % 5 == 0, // Usually false
                    name if name.contains("streaming") => variant % 3 == 0, // Sometimes true
                    _ => variant % 2 == 0,
                };
                Some(json!(value))
            },
            "array" => {
                // Generate arrays based on context
                let array_value = match param_name {
                    name if name.contains("vector") || name.contains("embedding") => {
                        // Generate sample vector
                        let dimension = match variant % 3 {
                            0 => 3,   // Small for testing
                            1 => 768, // Common embedding size
                            _ => 384, // Alternative size
                        };
                        let vector: Vec<f64> = (0..dimension)
                            .map(|i| ((variant + i) as f64 * 0.1) % 1.0 - 0.5)
                            .collect();
                        json!(vector)
                    },
                    name if name.contains("point_ids") || name.contains("ids") => {
                        json!([
                            format!("id_{}", variant),
                            format!("id_{}", variant + 1),
                            format!("id_{}", variant + 2)
                        ])
                    },
                    name if name.contains("points") => {
                        json!([{
                            "id": format!("point_{}", variant),
                            "vector": [0.1, 0.2, 0.3],
                            "payload": {"category": "sample"}
                        }])
                    },
                    name if name.contains("tags") => {
                        json!(["tag1", "tag2", "important"])
                    },
                    _ => {
                        // Generic array
                        json!([format!("item_{}", variant), format!("item_{}", variant + 1)])
                    }
                };
                Some(array_value)
            },
            "object" => {
                // Generate sample objects based on context
                let object_value = match param_name {
                    name if name.contains("payload") || name.contains("metadata") => {
                        json!({
                            "category": format!("category_{}", variant % 5),
                            "timestamp": "2024-01-15T10:30:00Z",
                            "version": variant + 1
                        })
                    },
                    name if name.contains("config") || name.contains("settings") => {
                        json!({
                            "enable_feature": variant % 2 == 0,
                            "threshold": 0.5,
                            "max_retries": 3
                        })
                    },
                    _ => json!({"sample": "data"}),
                };
                Some(object_value)
            },
            _ => Some(json!("unknown_type_value")),
        }
    }

    /// Generate realistic user queries for specific functions with variety
    fn generate_user_query_for_function(&mut self, function_tool: &FunctionTool, variant: usize) -> String {
        let function_name = &function_tool.name;
        let description = &function_tool.description;
        
        // Create varied queries based on function type and variant
        match function_name.as_str() {
            "complete" => match variant % 4 {
                0 => "Can you help me write a response to this message?".to_string(),
                1 => "I need text completion for my prompt".to_string(),
                2 => "Generate a continuation of this text".to_string(),
                _ => "Complete this sentence for me".to_string(),
            },
            "complete_stream" => match variant % 3 {
                0 => "I want streaming text generation".to_string(),
                1 => "Can you generate text in real-time?".to_string(),
                _ => "Stream the completion as it's generated".to_string(),
            },
            "search" => match variant % 4 {
                0 => "Find documents similar to my query".to_string(),
                1 => "Search for relevant information".to_string(),
                2 => "I need to find related content".to_string(),
                _ => "Look up documents about this topic".to_string(),
            },
            "create_collection" => match variant % 3 {
                0 => "Set up a new vector database collection".to_string(),
                1 => "I need to create a collection for embeddings".to_string(),
                _ => "How do I create a new vector store?".to_string(),
            },
            "upsert_points" => match variant % 3 {
                0 => "Add these vectors to the collection".to_string(),
                1 => "Store new embeddings in the database".to_string(),
                _ => "Update vector data in the collection".to_string(),
            },
            "search_points" => match variant % 4 {
                0 => "Find the most similar vectors".to_string(),
                1 => "Search for nearest neighbors".to_string(),
                2 => "Find vectors similar to this one".to_string(),
                _ => "Locate the closest embeddings".to_string(),
            },
            "get_points" => match variant % 3 {
                0 => "Retrieve specific vectors by ID".to_string(),
                1 => "Get these particular data points".to_string(),
                _ => "Fetch vectors with specific identifiers".to_string(),
            },
            "generate_embeddings" => match variant % 4 {
                0 => "Convert this text to embeddings".to_string(),
                1 => "Create vector representations".to_string(),
                2 => "Generate embeddings for my data".to_string(),
                _ => "Transform text into vectors".to_string(),
            },
            "generate_embeddings_batch" => match variant % 3 {
                0 => "Process multiple texts into embeddings".to_string(),
                1 => "Batch convert these texts to vectors".to_string(),
                _ => "Generate embeddings for all these documents".to_string(),
            },
            "execute_mcp_tool" => match variant % 4 {
                0 => "Run this specific tool for me".to_string(),
                1 => "Execute the MCP tool with these parameters".to_string(),
                2 => "Use this tool to process my request".to_string(),
                _ => "Apply this tool to my data".to_string(),
            },
            "list_mcp_tools" => match variant % 3 {
                0 => "Show me all available tools".to_string(),
                1 => "What tools can I use?".to_string(),
                _ => "List the MCP tools I have access to".to_string(),
            },
            _ => {
                // Fallback to generating based on description
                match variant % 3 {
                    0 => format!("I need help with {}", description.to_lowercase()),
                    1 => format!("How do I use {}?", function_name),
                    _ => description.clone(),
                }
            }
        }
    }
}

impl Default for DynamicFunctionDatasetGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_complete_dataset() {
        let mut generator = DynamicFunctionDatasetGenerator::new();
        let examples = generator.generate_complete_dataset(2);
        
        // Should have examples for all functions
        assert!(!examples.is_empty());
        println!("Generated {} examples", examples.len());
        
        // Verify structure
        for example in examples.iter().take(3) {
            assert_eq!(example.query.len(), 1);
            assert_eq!(example.tools.len(), 1);
            assert_eq!(example.answers.len(), 1);
            
            // Verify tool format
            let tool: Value = serde_json::from_str(&example.tools[0]).unwrap();
            assert_eq!(tool["type"], "function");
            assert!(tool["function"]["name"].is_string());
            assert!(tool["function"]["description"].is_string());
            
            // Verify answer format
            let answer: Value = serde_json::from_str(&example.answers[0]).unwrap();
            assert!(answer["name"].is_string());
            assert!(answer["arguments"].is_string()); // arguments should be a JSON string
        }
    }

    #[test]
    fn test_all_services_covered() {
        let generator = DynamicFunctionDatasetGenerator::new();
        let registries = generator.get_all_service_registries();
        
        // Verify we have the expected services
        let service_names: Vec<String> = registries.iter()
            .map(|r| r.service_name.clone())
            .collect();
        
        assert!(service_names.contains(&"chat-service".to_string()));
        assert!(service_names.contains(&"rag-service".to_string()));
        assert!(service_names.contains(&"vector-service".to_string()));
        assert!(service_names.contains(&"embedding-service".to_string()));
        assert!(service_names.contains(&"mcp-service".to_string()));
    }
}
