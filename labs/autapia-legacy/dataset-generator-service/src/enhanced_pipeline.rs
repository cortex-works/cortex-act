use crate::api_config::{ApiConfiguration, EnhancedDataset, EnhancedDatasetExample, ConversationTurn, ToolDefinition, ParameterSchema, ParameterProperty, ExampleMetadata, ToolCall, FunctionCall, FunctionDefinition};
use autapia_shared_types::DatasetMetadata;
use crate::clients::ServiceClients;
use anyhow::{Result, anyhow};
use log::{info, debug};
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task::JoinSet;
use std::time::Instant;
use serde_json::json;

/// Enhanced pipeline for generating comprehensive API documentation datasets
pub struct EnhancedApiDatasetPipeline {
    clients: Arc<ServiceClients>,
    api_config: ApiConfiguration,
}

impl EnhancedApiDatasetPipeline {
    /// Create new enhanced pipeline
    pub fn new(clients: Arc<ServiceClients>) -> Self {
        Self {
            clients,
            api_config: ApiConfiguration::new(),
        }
    }
    
    /// Generate comprehensive API dataset with multiple variations (optimized with concurrency)
    pub async fn generate_comprehensive_dataset(
        &self,
        variations_per_use_case: usize,
        output_path: &str,
        format: crate::api_config::DatasetFormat,
    ) -> Result<EnhancedDataset> {
        let start_time = Instant::now();
        info!("Starting enhanced comprehensive API dataset generation");
        info!("Variations per use case: {}", variations_per_use_case);
        
        let mut examples = Vec::new();
        let mut total_endpoints = 0;
        
        // IMPORTANT: Add size limits for FiftyOne format (5-10k examples max)
        const MAX_EXAMPLES: usize = 8000; // Target 8k examples for optimal size
        const MAX_EXAMPLES_PER_ENDPOINT: usize = 5; // Limit examples per endpoint to prevent bloat
        
        // Pre-calculate total work for progress tracking
        let total_work: usize = self.api_config.services.values()
            .map(|service| service.endpoints.iter()
                .map(|endpoint| endpoint.use_cases.len().min(MAX_EXAMPLES_PER_ENDPOINT) * variations_per_use_case.min(2))
                .sum::<usize>())
            .sum();
        
        let estimated_examples = total_work.min(MAX_EXAMPLES);
        info!("Estimated total examples to generate: {} (capped at {})", estimated_examples, MAX_EXAMPLES);
        
        // Process services concurrently with controlled parallelism
        const MAX_CONCURRENT_SERVICES: usize = 3;
        let mut service_tasks = JoinSet::new();
        let mut completed_count = 0;
        let mut example_count = 0;
        
        for (service_name, service_def) in &self.api_config.services {
            // Stop if we've reached the maximum examples
            if example_count >= MAX_EXAMPLES {
                info!("Reached maximum examples limit ({}), stopping generation", MAX_EXAMPLES);
                break;
            }
            
            let service_name = service_name.clone();
            let service_def = service_def.clone();
            let remaining_examples = MAX_EXAMPLES.saturating_sub(example_count);
            let limited_variations = variations_per_use_case.min(2); // Limit to max 2 variations
            let self_ref = self.clone_for_async();
            
            // Limit concurrency to prevent resource exhaustion
            if service_tasks.len() >= MAX_CONCURRENT_SERVICES {
                if let Some(result) = service_tasks.join_next().await {
                    let (service_examples, endpoints_count): (Vec<EnhancedDatasetExample>, usize) = result??;
                    let examples_to_add = service_examples.len().min(remaining_examples);
                    examples.extend(service_examples.into_iter().take(examples_to_add));
                    example_count = examples.len();
                    total_endpoints += endpoints_count;
                    completed_count += 1;
                    
                    let progress = (completed_count as f64 / self.api_config.services.len() as f64) * 100.0;
                    info!("Service processing progress: {:.1}% (Examples: {})", progress, example_count);
                }
            }
            
            service_tasks.spawn(async move {
                self_ref.generate_service_examples_limited(service_name, service_def, limited_variations, remaining_examples).await
            });
        }
        
        // Collect remaining results
        while let Some(result) = service_tasks.join_next().await {
            if example_count >= MAX_EXAMPLES {
                break;
            }
            
            let (service_examples, endpoints_count): (Vec<EnhancedDatasetExample>, usize) = result??;
            let remaining_examples = MAX_EXAMPLES.saturating_sub(example_count);
            let examples_to_add = service_examples.len().min(remaining_examples);
            examples.extend(service_examples.into_iter().take(examples_to_add));
            example_count = examples.len();
            total_endpoints += endpoints_count;
            completed_count += 1;
            
            let progress = (completed_count as f64 / self.api_config.services.len() as f64) * 100.0;
            info!("Service processing progress: {:.1}% (Examples: {})", progress, example_count);
        }
        
        let generation_time = start_time.elapsed();
        info!("Example generation completed in {:.2}s", generation_time.as_secs_f64());
        
        // Create comprehensive dataset
        let total_examples = examples.len();
        let dataset = match format {
            crate::api_config::DatasetFormat::SingleTurn => {
                // Convert multi-turn examples to FiftyOne single-turn format
                let single_turn_examples = self.convert_to_fiftyone_format(&examples)?;
                EnhancedDataset {
                    format: crate::api_config::DatasetFormat::SingleTurn,
                    multi_turn_data: None,
                    single_turn_data: Some(single_turn_examples),
                    metadata: DatasetMetadata {
                        job_id: uuid::Uuid::new_v4().to_string(),
                        dataset_type: "single_turn_api".to_string(),
                        status: "completed".to_string(),
                        created_at: chrono::Utc::now().to_rfc3339(),
                        completed_at: Some(chrono::Utc::now().to_rfc3339()),
                        total_samples: Some(total_examples as u32),
                        output_path: None,
                    },
                }
            },
            crate::api_config::DatasetFormat::MultiTurn => {
                EnhancedDataset {
                    format: crate::api_config::DatasetFormat::MultiTurn,
                    multi_turn_data: Some(examples),
                    single_turn_data: None,
                    metadata: DatasetMetadata {
                        job_id: uuid::Uuid::new_v4().to_string(),
                        dataset_type: "multi_turn_api".to_string(),
                        status: "completed".to_string(),
                        created_at: chrono::Utc::now().to_rfc3339(),
                        completed_at: Some(chrono::Utc::now().to_rfc3339()),
                        total_samples: Some(total_examples as u32),
                        output_path: None,
                    },
                }
            }
        };
        
        // Save dataset
        let save_start = Instant::now();
        self.save_dataset(&dataset, output_path).await?;
        let save_time = save_start.elapsed();
        
        let total_time = start_time.elapsed();
        info!("Enhanced dataset generation completed in {:.2}s total", total_time.as_secs_f64());
        info!("  - Generation: {:.2}s", generation_time.as_secs_f64());
        info!("  - Serialization: {:.2}s", save_time.as_secs_f64());
        info!("Final examples: {} (target: 5k-10k)", total_examples);
        info!("Total endpoints: {}", total_endpoints);
        info!("Services covered: {}", self.api_config.services.keys().count());
        info!("Performance: {:.1} examples/second", total_examples as f64 / generation_time.as_secs_f64());
        
        Ok(dataset)
    }
    
    /// Clone self for async operations (shallow clone with Arc)
    fn clone_for_async(&self) -> Self {
        Self {
            clients: self.clients.clone(),
            api_config: self.api_config.clone(),
        }
    }
    
    /// Generate examples for a single service with size limits
    async fn generate_service_examples_limited(
        &self,
        service_name: String,
        service_def: crate::api_config::ServiceDefinition,
        variations_per_use_case: usize,
        max_examples: usize,
    ) -> Result<(Vec<EnhancedDatasetExample>, usize)> {
        let mut examples = Vec::new();
        let endpoint_count = service_def.endpoints.len();
        let max_per_endpoint = max_examples / endpoint_count.max(1);
        
        for endpoint in &service_def.endpoints {
            if examples.len() >= max_examples {
                break;
            }
            
            // Limit use cases per endpoint to prevent bloat
            let limited_use_cases = endpoint.use_cases.iter().take(3).cloned().collect::<Vec<_>>();
            
            for (_use_case_idx, use_case) in limited_use_cases.iter().enumerate() {
                let examples_for_this_case = examples.len();
                if examples_for_this_case >= max_per_endpoint {
                    break;
                }
                
                // Generate limited variations
                for variation in 1..=variations_per_use_case {
                    if examples.len() >= max_examples || examples.len() >= max_per_endpoint {
                        break;
                }
                    
                    match self.generate_single_enhanced_example(
                        &service_name,
                        endpoint,
                        use_case,
                        variation
                    ).await {
                        Ok(example) => examples.push(example),
                        Err(e) => {
                            debug!("Failed to generate example for {}.{}: {}", service_name, endpoint.name, e);
        }
                    }
                }
            }
        }
        
        info!("Generated {} examples for service: {} (limit: {})", examples.len(), service_name, max_examples);
        Ok((examples, endpoint_count))
    }
    
    /// Generate examples for a single service (used in concurrent processing)
    async fn generate_service_examples(
        &self,
        service_name: String,
        service_def: crate::api_config::ServiceDefinition,
        variations_per_use_case: usize,
    ) -> Result<(Vec<EnhancedDatasetExample>, usize)> {
        // Use the limited version with a reasonable default
        self.generate_service_examples_limited(service_name, service_def, variations_per_use_case, 1000).await
    }
    
    /// Generate a single enhanced example for an API endpoint
    async fn generate_single_enhanced_example(
        &self,
        service_name: &str,
        _endpoint: &crate::api_config::ApiEndpoint,
        use_case: &str,
        variation: usize,
    ) -> Result<EnhancedDatasetExample> {
        // Generate user query based on use case and variation
        let user_query = self.generate_varied_user_query(use_case, variation);
        
        // Create a single tool definition for this specific endpoint
        let function_name = format!("{}_{}", 
            service_name.replace("-", "_"),
            _endpoint.name.chars()
                .enumerate()
                .map(|(i, c)| if i > 0 && c.is_uppercase() { format!("_{}", c.to_lowercase()) } else { c.to_lowercase().to_string() })
                .collect::<String>()
        );
        
        // Create tool definition (only ONE tool per example - FiftyOne format requirement)
        let tool_definition = ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: function_name.clone(),
                description: _endpoint.description.clone(),
                parameters: ParameterSchema {
                    r#type: "object".to_string(),
                    properties: _endpoint.parameters.iter().map(|(key, value)| {
                        (key.clone(), ParameterProperty {
                            r#type: "string".to_string(),
                            description: value.clone(),
                            items: None,
                        })
                    }).collect(),
                    required: _endpoint.parameters.keys().take(2).cloned().collect(), // Limit required params
                },
            },
        };
        
        // Generate realistic function call arguments
        let function_arguments = self.generate_realistic_arguments(&_endpoint.parameters);
        
        // Create assistant response with proper function call
        let assistant_response = format!(
            "I'll help you with that. Let me call the appropriate function to {}",
            _endpoint.description.to_lowercase()
        );
        
        // Create function call for the conversation
        let function_call = json!({
            "name": function_name,
            "arguments": function_arguments
        }).to_string();

        let conversations = vec![
            ConversationTurn {
                role: "user".to_string(),
                content: user_query,
                tool_calls: None,
                tool_call_id: None,
            },
            ConversationTurn {
                role: "assistant".to_string(),
                content: assistant_response,
                tool_calls: Some(vec![ToolCall {
                    id: "call_1".to_string(),
                    r#type: "function".to_string(),
                    function: FunctionCall {
                        name: function_name,
                        arguments: function_call,
                    },
                }]),
                tool_call_id: None,
            },
        ];
        
        // Create example metadata
        let metadata = ExampleMetadata {
            endpoint: _endpoint.name.clone(),
            service: service_name.to_string(),
            use_case: use_case.to_string(),
            variation,
            complexity: if _endpoint.parameters.is_empty() { "simple".to_string() } else { "moderate".to_string() },
            scenario_type: "function_calling".to_string(),
        };
        
        Ok(EnhancedDatasetExample {
            conversations,
            system: "You are a helpful AI assistant that can call functions to help users with their requests.".to_string(),
            tools: vec![tool_definition], // ONLY ONE TOOL per example
            metadata,
        })
    }
    
    /// Generate realistic function arguments
    fn generate_realistic_arguments(&self, parameters: &HashMap<String, String>) -> serde_json::Value {
        let mut args = serde_json::Map::new();
        let mut rng = rand::thread_rng();
        
        for (param_name, _description) in parameters.iter().take(3) { // Limit parameters
            let sample_value = match param_name.as_str() {
                name if name.contains("id") => json!(format!("sample_id_{}", rng.gen_range(1000..9999))),
                name if name.contains("name") => json!(format!("sample_name_{}", rng.gen_range(100..999))),
                name if name.contains("size") || name.contains("limit") => json!(rng.gen_range(10..1000)),
                name if name.contains("text") || name.contains("content") => json!("Sample text content"),
                name if name.contains("type") => json!("sample_type"),
                name if name.contains("format") => json!("json"),
                _ => json!(format!("sample_{}", param_name)),
            };
            args.insert(param_name.clone(), sample_value);
        }
        
        serde_json::Value::Object(args)
    }
    
    /// Generate varied user queries
    fn generate_varied_user_query(&self, use_case: &str, variation: usize) -> String {
        let _use_case_lower = use_case.to_lowercase();
        
        let query_templates = vec![
            format!("I need help with {}", use_case.to_lowercase()),
            format!("Can you assist me with {}?", use_case.to_lowercase()),
            format!("Please help me {}", use_case.to_lowercase()),
            format!("I want to {}", use_case.to_lowercase()),
            format!("How can I {}?", use_case.to_lowercase()),
        ];
        
        let template_idx = variation % query_templates.len();
        query_templates[template_idx].clone()
    }
    
    /// Generate comprehensive tools list for the service
    fn generate_comprehensive_tools(&self) -> Vec<ToolDefinition> {
        let mut tools = Vec::new();
        let mut tool_count = 0;
        const MAX_TOOLS: usize = 50; // Limit total tools to prevent bloat
        
        for (service_name, service_def) in &self.api_config.services {
            if tool_count >= MAX_TOOLS {
                break;
            }
            
            for endpoint in &service_def.endpoints {
                if tool_count >= MAX_TOOLS {
                    break;
                }
                
                // Create properties for parameters
                let mut properties = HashMap::new();
                let mut required = Vec::new();
                
                for (param_name, param_desc) in &endpoint.parameters {
                    properties.insert(param_name.clone(), ParameterProperty {
                        r#type: "string".to_string(),
                            description: param_desc.clone(),
                            items: None,
                    });
                    
                    // Only mark essential parameters as required
                    if !["limit", "offset", "filter", "metadata", "source", "include_completed", "force"].contains(&param_name.as_str()) {
                        required.push(param_name.clone());
                    }
                }
                
                // Create unique function name combining service and endpoint
                let function_name = format!("{}_{}", 
                    service_name.replace("-", "_"),
                    endpoint.name.chars()
                        .enumerate()
                        .map(|(i, c)| if i > 0 && c.is_uppercase() { format!("_{}", c.to_lowercase()) } else { c.to_lowercase().to_string() })
                        .collect::<String>()
                );
                
                let tool_description = format!("{} - {} {}", 
                    endpoint.description, 
                    endpoint.method, 
                    endpoint.endpoint
                );
                
                tools.push(ToolDefinition {
                    r#type: "function".to_string(),
                    function: FunctionDefinition {
                        name: function_name,
                        description: tool_description,
                        parameters: ParameterSchema {
                            r#type: "object".to_string(),
                            properties,
                            required,
                        },
                    },
                });
                
                tool_count += 1;
            }
        }
        
        info!("Generated comprehensive tools list with {} tools from {} services", 
              tool_count, self.api_config.services.len());
        
        tools
    }
    
    /// Public method to get comprehensive tools list for testing
    pub fn generate_comprehensive_tools_list(&self) -> Vec<ToolDefinition> {
        self.generate_comprehensive_tools()
    }
    
    /// Generate parameter description
    fn generate_parameter_description(&self, param_name: &str) -> String {
        if param_name.contains("id") {
            format!("Unique identifier for the {}", param_name.replace("_id", "").replace("_", " "))
        } else if param_name.contains("name") {
            format!("Name for the {}", param_name.replace("_name", "").replace("_", " "))
        } else if param_name.contains("config") {
            "Configuration object with settings and parameters".to_string()
        } else if param_name.contains("data") {
            "Data payload containing the required information".to_string()
        } else if param_name.contains("filter") {
            "Filter criteria to narrow down results".to_string()
        } else if param_name.contains("limit") {
            "Maximum number of items to return".to_string()
        } else if param_name.contains("offset") {
            "Number of items to skip for pagination".to_string()
        } else {
            format!("{} parameter", param_name.replace("_", " "))
        }
    }
    
    /// Save dataset to file in FiftyOne format
    async fn save_dataset(&self, dataset: &EnhancedDataset, output_path: &str) -> Result<()> {
        info!("Saving enhanced dataset to: {}", output_path);

        // Convert to FiftyOne format for single-turn, keep original for multi-turn
        let content_to_save = match dataset.format {
            crate::api_config::DatasetFormat::SingleTurn => {
                // For single-turn, convert to FiftyOne format array
                if let Some(single_turn_data) = &dataset.single_turn_data {
                    let fiftyone_format: Vec<serde_json::Value> = single_turn_data.iter().map(|example| {
                        serde_json::json!({
                            "query": example.query,
                            "tools": example.tools,
                            "answers": example.answers
                        })
                    }).collect();
                    
                    serde_json::to_string_pretty(&fiftyone_format)
                        .map_err(|e| anyhow!("Failed to serialize FiftyOne dataset: {}", e))?
                } else {
                    "[]".to_string()
                }
            },
            crate::api_config::DatasetFormat::MultiTurn => {
                // For multi-turn, save the conversations format
                let empty_vec = Vec::new();
                let multi_turn_data = dataset.multi_turn_data.as_ref().unwrap_or(&empty_vec);
                serde_json::to_string_pretty(multi_turn_data)
                    .map_err(|e| anyhow!("Failed to serialize multi-turn dataset: {}", e))?
            }
        };

        let content_len = content_to_save.len();
        tokio::fs::write(output_path, content_to_save).await
            .map_err(|e| anyhow!("Failed to write dataset file: {}", e))?;

        info!("Enhanced dataset saved successfully: {} bytes in FiftyOne format", content_len);
        Ok(())
    }

    /// Convert to FiftyOne format with proper structure
    fn convert_to_fiftyone_format(&self, examples: &[EnhancedDatasetExample]) -> Result<Vec<crate::api_config::SingleTurnDatasetExample>> {
        let mut fiftyone_examples = Vec::new();
        
        for example in examples {
            // Extract user query from conversation
            let user_query = example.conversations.iter()
                .find(|turn| turn.role == "user")
                .map(|turn| turn.content.clone())
                .unwrap_or_else(|| "Help me with this task".to_string());

            // Get the single tool (FiftyOne format requirement: exactly 1 tool per example)
            let tool_json = if !example.tools.is_empty() {
                serde_json::to_string(&example.tools[0]).unwrap_or_default()
            } else {
                serde_json::json!({
                    "name": "default_function",
                    "description": "Default function",
                    "parameters": {"type": "object", "properties": {}, "required": []}
                }).to_string()
            };

            // Generate proper answer with function call
            let function_name = example.tools.get(0)
                .map(|tool| tool.function.name.clone())
                .unwrap_or_else(|| "default_function".to_string());

            let answer_json = serde_json::json!({
                "name": function_name,
                "arguments": self.generate_realistic_arguments(&HashMap::new())
            });

            let fiftyone_example = crate::api_config::SingleTurnDatasetExample {
                query: vec![user_query],
                tools: vec![tool_json],
                answers: vec![serde_json::to_string(&answer_json).unwrap_or_default()],
                metadata: example.metadata.clone(),
            };
            
            fiftyone_examples.push(fiftyone_example);
        }
        
        info!("Converted {} examples to FiftyOne format", fiftyone_examples.len());
        Ok(fiftyone_examples)
    }

    /// Convert EnhancedDataset to APIGen format (legacy support)
    fn convert_to_apigen_format(&self, dataset: &EnhancedDataset) -> Result<Vec<serde_json::Value>> {
        let mut apigen_examples = Vec::new();
        
        // Get the data from the appropriate field based on format
        let _examples = match dataset.format {
            crate::api_config::DatasetFormat::MultiTurn => {
                // For multi-turn, we need to process the multi_turn_data field
                let empty_vec = Vec::new();
                let multi_turn_examples = dataset.multi_turn_data.as_ref().unwrap_or(&empty_vec);
                
                for example in multi_turn_examples {
                    let mut conversations = Vec::new();
                    
                    for turn in &example.conversations {
                        match turn.role.as_str() {
                            "user" => {
                                conversations.push(serde_json::json!({
                                    "role": "user",
                                    "content": turn.content
                                }));
                            },
                            "assistant" => {
                                        conversations.push(serde_json::json!({
                                    "role": "assistant", 
                                    "content": turn.content
                                    }));
                            },
                            _ => {} // Skip other roles
                        }
                    }
                    
                        let apigen_example = serde_json::json!({
                            "conversations": conversations,
                            "system": example.system,
                            "tools": example.tools
                        });
                        apigen_examples.push(apigen_example);
                }
                
                return Ok(apigen_examples);
            },
            crate::api_config::DatasetFormat::SingleTurn => {
                // For single-turn, return the single_turn_data field directly as JSON
                if let Some(single_turn_data) = &dataset.single_turn_data {
                    for example in single_turn_data {
                        let apigen_example = serde_json::json!({
                            "query": example.query,
                            "tools": example.tools,
                            "answers": example.answers
                        });
                        apigen_examples.push(apigen_example);
                    }
                }
                
                return Ok(apigen_examples);
            },
        };
    }

    /// Convert Vec<EnhancedDatasetExample> to APIGen single-turn format (legacy support)
    fn convert_examples_to_apigen_format(&self, examples: &[EnhancedDatasetExample]) -> Result<Vec<crate::api_config::SingleTurnDatasetExample>> {
        // Use the new FiftyOne format method instead
        self.convert_to_fiftyone_format(examples)
    }
}
