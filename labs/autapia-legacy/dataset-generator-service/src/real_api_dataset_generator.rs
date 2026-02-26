use crate::schema_extractor::{ServiceApiSchema, ExtractedEndpoint};
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use rand::Rng;
use std::collections::{HashMap, HashSet};
use log::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiftyOneApiExample {
    pub query: Vec<String>,
    pub tools: Vec<String>,
    pub answers: Vec<String>,
}

pub struct ApiDatasetGenerator {
    schemas: Vec<ServiceApiSchema>,
    rng: rand::rngs::ThreadRng,
}

impl ApiDatasetGenerator {
    pub fn new(schemas: Vec<ServiceApiSchema>) -> Self {
        Self {
            schemas,
            rng: rand::thread_rng(),
        }
    }

    /// Generate a complete single_turn_api dataset in FiftyOne format
    pub fn generate_dataset(&mut self, num_examples: usize) -> Vec<FiftyOneApiExample> {
        let mut examples = Vec::new();
        
        info!("Generating {} FiftyOne format examples from {} services", num_examples, self.schemas.len());

        // Collect all endpoints from all services and clone them to avoid borrowing issues
        let all_endpoints: Vec<ExtractedEndpoint> = self.schemas
            .iter()
            .flat_map(|schema| &schema.endpoints)
            .cloned()
            .collect();

        if all_endpoints.is_empty() {
            warn!("No endpoints found in schemas, cannot generate examples");
            return examples;
        }

        for i in 0..num_examples {
            // Randomly select an endpoint
            let endpoint_idx = self.rng.gen_range(0..all_endpoints.len());
            let endpoint = &all_endpoints[endpoint_idx];
            
            // Generate a realistic example for this endpoint
            if let Some(example) = self.generate_single_example(endpoint) {
                examples.push(example);
                
                if (i + 1) % 100 == 0 {
                    info!("Generated {} examples so far", i + 1);
                }
            }
        }

        info!("Successfully generated {} FiftyOne format examples", examples.len());
        examples
    }

    /// Generate a single realistic API example in FiftyOne format
    fn generate_single_example(&mut self, endpoint: &ExtractedEndpoint) -> Option<FiftyOneApiExample> {
        // Generate a realistic user query based on the endpoint
        let user_query = self.generate_user_query(endpoint);
        
        // Create the single tool definition for this endpoint only
        let tool_definition = self.create_tool_definition(endpoint);

        // Generate realistic function call arguments based on the endpoint schema
        let function_arguments = self.generate_function_arguments(endpoint);

        // Create the answer (function call)
        let answer = json!({
            "name": tool_definition["name"],
            "arguments": function_arguments
        });

        let example = FiftyOneApiExample {
            query: vec![user_query],
            tools: vec![serde_json::to_string(&tool_definition).unwrap_or_default()],
            answers: vec![serde_json::to_string(&answer).unwrap_or_default()],
        };

        Some(example)
    }

    /// Create a single tool definition for the endpoint
    fn create_tool_definition(&self, endpoint: &ExtractedEndpoint) -> Value {
        // Create clean function name
        let function_name = format!("{}_{}", 
            endpoint.service_name.replace("-", "_"),
            endpoint.method_name
        );

        // Extract parameters from input schema
        let parameters = if let Some(properties) = endpoint.input_schema.get("properties") {
            endpoint.input_schema.clone()
        } else {
            json!({
                "type": "object",
                "properties": {},
                "required": []
            })
        };

        json!({
            "name": function_name,
            "description": endpoint.description,
            "parameters": parameters
        })
    }

    /// Generate realistic function arguments based on the endpoint schema
    fn generate_function_arguments(&mut self, endpoint: &ExtractedEndpoint) -> Value {
        let mut arguments = json!({});

        if let Some(properties) = endpoint.input_schema.get("properties").and_then(|p| p.as_object()) {
            let required_fields = endpoint.input_schema.get("required")
                .and_then(|r| r.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                .unwrap_or_default();

            for (param_name, param_schema) in properties {
                // Generate appropriate sample values based on parameter type and name
                if let Some(sample_value) = self.generate_sample_parameter_value(param_name, param_schema, &required_fields) {
                    arguments[param_name] = sample_value;
                }
            }
        }

        arguments
    }

    /// Generate sample parameter values based on type and name
    fn generate_sample_parameter_value(&mut self, param_name: &str, param_schema: &Value, required_fields: &[&str]) -> Option<Value> {
        let param_type = param_schema.get("type")?.as_str()?;
        let is_required = required_fields.contains(&param_name);

        // Skip optional parameters sometimes to add variety
        if !is_required && self.rng.gen_bool(0.3) {
            return None;
        }

        match param_type {
            "string" => {
                let sample_value = match param_name {
                    name if name.contains("id") => format!("sample_id_{}", self.rng.gen_range(1000..9999)),
                    name if name.contains("name") => format!("sample_name_{}", self.rng.gen_range(100..999)),
                    name if name.contains("path") || name.contains("file") => "/path/to/sample/file.txt".to_string(),
                    name if name.contains("url") => "https://example.com/api/endpoint".to_string(),
                    name if name.contains("type") => "sample_type".to_string(),
                    name if name.contains("format") => "json".to_string(),
                    name if name.contains("status") => "active".to_string(),
                    name if name.contains("dataset") => "sample_dataset".to_string(),
                    _ => format!("sample_{}", param_name),
                };
                Some(json!(sample_value))
            },
            "integer" => {
                let sample_value = match param_name {
                    name if name.contains("size") || name.contains("count") || name.contains("limit") => self.rng.gen_range(10..1000),
                    name if name.contains("port") => self.rng.gen_range(8000..9999),
                    name if name.contains("id") => self.rng.gen_range(1..10000),
                    name if name.contains("timeout") => self.rng.gen_range(1000..30000),
                    _ => self.rng.gen_range(1..100),
                };
                Some(json!(sample_value))
            },
            "number" | "float" => {
                let sample_value = match param_name {
                    name if name.contains("ratio") || name.contains("threshold") => self.rng.gen_range(0.1..1.0),
                    name if name.contains("score") => self.rng.gen_range(0.0..1.0),
                    _ => self.rng.gen_range(0.1..100.0),
                };
                Some(json!(sample_value))
            },
            "boolean" => Some(json!(self.rng.gen_bool(0.5))),
            "array" => {
                // Generate simple array with 1-3 elements
                let array_size = self.rng.gen_range(1..=3);
                let mut array = Vec::new();
                
                for i in 0..array_size {
                    array.push(json!(format!("item_{}", i + 1)));
                }
                Some(json!(array))
            },
            "object" => Some(json!({})), // Empty object for simplicity
            _ => Some(json!("sample_value")),
        }
    }

    /// Generate varied user queries based on endpoint functionality
    fn generate_user_query(&mut self, endpoint: &ExtractedEndpoint) -> String {
        let query_templates = self.get_query_templates_for_endpoint(endpoint);
        let template_idx = self.rng.gen_range(0..query_templates.len());
        query_templates[template_idx].clone()
    }

    /// Get query templates based on endpoint type and functionality
    fn get_query_templates_for_endpoint(&self, endpoint: &ExtractedEndpoint) -> Vec<String> {
        let method_lower = endpoint.method_name.to_lowercase();
        let service_name = endpoint.service_name.replace("-", " ");
        
        match method_lower.as_str() {
            name if name.contains("generate") => vec![
                format!("I need to generate data using {}", service_name),
                format!("Can you help me create content with {}?", service_name),
                format!("How do I use {} to generate new data?", service_name),
                "I want to create a new dataset with specific parameters".to_string(),
                "Generate synthetic data for my machine learning project".to_string(),
            ],
            name if name.contains("create") => vec![
                format!("I need to create something in {}", service_name),
                format!("How can I create a new resource using {}?", service_name),
                format!("Please help me set up a new configuration in {}", service_name),
                "I want to create a new resource with specific settings".to_string(),
            ],
            name if name.contains("list") || name.contains("get") => vec![
                format!("I need to retrieve information from {}", service_name),
                format!("Can you help me list available resources in {}?", service_name),
                format!("How do I get data from {}?", service_name),
                "Show me the available items in the system".to_string(),
                "I need to check what's currently available".to_string(),
            ],
            name if name.contains("delete") || name.contains("remove") => vec![
                format!("I need to delete something from {}", service_name),
                format!("How can I remove data using {}?", service_name),
                format!("Please help me clean up resources in {}", service_name),
                "I want to delete this resource safely".to_string(),
                "Remove the specified item from the system".to_string(),
            ],
            name if name.contains("update") || name.contains("modify") => vec![
                format!("I need to update configuration in {}", service_name),
                format!("How can I modify existing data using {}?", service_name),
                format!("Please help me change settings in {}", service_name),
                "I want to update the current configuration".to_string(),
                "Modify the existing resource with new parameters".to_string(),
            ],
            name if name.contains("analyze") || name.contains("check") => vec![
                format!("I need to analyze data using {}", service_name),
                format!("Can you help me check the status with {}?", service_name),
                format!("How do I examine information in {}?", service_name),
                "Analyze the current data for insights".to_string(),
                "Check the quality and status of the system".to_string(),
            ],
            name if name.contains("convert") || name.contains("transform") => vec![
                format!("I need to convert data using {}", service_name),
                format!("How can I transform format with {}?", service_name),
                format!("Please help me change the data format in {}", service_name),
                "Convert the data to a different format".to_string(),
                "Transform the input to match required specifications".to_string(),
            ],
            name if name.contains("search") || name.contains("find") => vec![
                format!("I need to search for information in {}", service_name),
                format!("Can you help me find data using {}?", service_name),
                format!("How do I locate specific items in {}?", service_name),
                "Search for relevant information in the system".to_string(),
                "Find the data that matches my criteria".to_string(),
            ],
            _ => vec![
                format!("I need help with {} functionality", endpoint.method_name),
                format!("How can I use {} in {}?", endpoint.method_name, service_name),
                endpoint.description.clone(),
                format!("Please help me with {}", endpoint.description.to_lowercase()),
            ],
        }
    }
}
