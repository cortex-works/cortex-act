use std::collections::HashMap;
use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use log::{info, warn, error};
use regex::Regex;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtractedEndpoint {
    pub service_name: String,
    pub method_name: String,
    pub description: String,
    pub input_schema: Value,
    pub output_schema: Value,
    pub required_permissions: Vec<String>,
    pub supports_streaming: bool,
    pub tags: Vec<String>,
    pub example_usage: String,
    pub estimated_latency_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceApiSchema {
    pub service_name: String,
    pub description: String,
    pub version: String,
    pub grpc_port: u16,
    pub capabilities: Vec<String>,
    pub use_cases: Vec<String>,
    pub dependencies: Vec<String>,
    pub endpoints: Vec<ExtractedEndpoint>,
}

pub struct SchemaExtractor {
    workspace_root: String,
}

impl SchemaExtractor {
    pub fn new(workspace_root: &str) -> Self {
        Self {
            workspace_root: workspace_root.to_string(),
        }
    }

    /// Extract all API schemas from routing_schema.rs files across all services
    pub fn extract_all_schemas(&self) -> Result<Vec<ServiceApiSchema>, Box<dyn std::error::Error>> {
        let mut schemas = Vec::new();
        
        // Find all routing_schema.rs files
        let services_dir = Path::new(&self.workspace_root).join("services");
        if !services_dir.exists() {
            return Err("Services directory not found".into());
        }

        for entry in fs::read_dir(&services_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let service_name = entry.file_name().to_string_lossy().to_string();
                let routing_schema_path = entry.path().join("src").join("routing_schema.rs");
                
                if routing_schema_path.exists() {
                    match self.extract_schema_from_file(&routing_schema_path, &service_name) {
                        Ok(Some(schema)) => {
                            info!("Extracted schema for service: {}", service_name);
                            schemas.push(schema);
                        },
                        Ok(None) => {
                            warn!("No schema found in file: {:?}", routing_schema_path);
                        },
                        Err(e) => {
                            error!("Failed to extract schema from {:?}: {}", routing_schema_path, e);
                        }
                    }
                }
            }
        }

        info!("Successfully extracted {} service schemas", schemas.len());
        Ok(schemas)
    }

    /// Extract schema from a single routing_schema.rs file
    fn extract_schema_from_file(
        &self, 
        file_path: &Path, 
        service_name: &str
    ) -> Result<Option<ServiceApiSchema>, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(file_path)?;
        
        // Parse the generate_routing_schema! macro call
        if let Some(schema) = self.parse_routing_schema_macro(&content, service_name)? {
            Ok(Some(schema))
        } else {
            Ok(None)
        }
    }

    /// Parse the generate_routing_schema! macro call using regex patterns
    fn parse_routing_schema_macro(
        &self,
        content: &str,
        default_service_name: &str
    ) -> Result<Option<ServiceApiSchema>, Box<dyn std::error::Error>> {
        // Find the macro call - simplified pattern
        let macro_start = content.find("generate_routing_schema!")
            .ok_or("No generate_routing_schema! macro found")?;
        
        // Find the opening brace after the macro name
        let start_brace = content[macro_start..]
            .find('{')
            .ok_or("No opening brace found for macro")?;
        
        let absolute_start = macro_start + start_brace;
        
        // Find matching closing brace
        let mut brace_count = 0;
        let mut end_pos = absolute_start;
        
        for (i, c) in content[absolute_start..].chars().enumerate() {
            match c {
                '{' => brace_count += 1,
                '}' => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        end_pos = absolute_start + i;
                        break;
                    }
                },
                _ => {}
            }
        }
        
        if brace_count != 0 {
            return Err("Unmatched braces in macro".into());
        }
        
        let macro_body = &content[absolute_start + 1..end_pos];
        
        // Extract basic service information
        let service_name = self.extract_string_field(macro_body, "service_name")
            .unwrap_or_else(|| default_service_name.to_string());
        let description = self.extract_string_field(macro_body, "description")
            .unwrap_or_else(|| format!("API service: {}", service_name));
        let version = self.extract_string_field(macro_body, "version")
            .unwrap_or_else(|| "0.1.0".to_string());
        let grpc_port = self.extract_number_field(macro_body, "grpc_port")
            .unwrap_or(20000) as u16;

        // Extract arrays
        let capabilities = self.extract_string_array(macro_body, "capabilities");
        let use_cases = self.extract_string_array(macro_body, "use_cases");
        let dependencies = self.extract_string_array(macro_body, "dependencies");

        // Extract endpoints
        let endpoints = self.extract_endpoints(macro_body, &service_name)?;

        Ok(Some(ServiceApiSchema {
            service_name,
            description,
            version,
            grpc_port,
            capabilities,
            use_cases,
            dependencies,
            endpoints,
        }))
    }

    /// Extract string field from macro body
    fn extract_string_field(&self, content: &str, field_name: &str) -> Option<String> {
        let pattern = format!(r#"{}\s*:\s*"([^"]+)""#, field_name);
        let regex = Regex::new(&pattern).ok()?;
        
        regex.captures(content)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().to_string())
    }

    /// Extract number field from macro body
    fn extract_number_field(&self, content: &str, field_name: &str) -> Option<i64> {
        let pattern = format!(r#"{}\s*:\s*(\d+)"#, field_name);
        let regex = Regex::new(&pattern).ok()?;
        
        regex.captures(content)
            .and_then(|cap| cap.get(1))
            .and_then(|m| m.as_str().parse().ok())
    }

    /// Extract string array from macro body
    fn extract_string_array(&self, content: &str, field_name: &str) -> Vec<String> {
        let pattern = format!(r#"{}\s*:\s*\[([^\]]+)\]"#, field_name);
        let regex = match Regex::new(&pattern) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };

        if let Some(captures) = regex.captures(content) {
            let array_content = captures.get(1).unwrap().as_str();
            
            // Extract quoted strings from array
            let item_regex = Regex::new(r#""([^"]+)""#).unwrap();
            return item_regex
                .captures_iter(array_content)
                .map(|cap| cap.get(1).unwrap().as_str().to_string())
                .collect();
        }

        Vec::new()
    }

    /// Extract endpoints from the macro body
    fn extract_endpoints(
        &self,
        content: &str,
        service_name: &str
    ) -> Result<Vec<ExtractedEndpoint>, Box<dyn std::error::Error>> {
        let mut endpoints = Vec::new();

        // Simple approach: just look for "method:" patterns and extract basic info
        let mut current_pos = 0;
        while let Some(method_pos) = content[current_pos..].find("method:") {
            let absolute_method_pos = current_pos + method_pos;
            
            // Extract the method name
            if let Some(quote_start) = content[absolute_method_pos..].find('"') {
                let method_name_start = absolute_method_pos + quote_start + 1;
                if let Some(quote_end) = content[method_name_start..].find('"') {
                    let method_name = &content[method_name_start..method_name_start + quote_end];
                    
                    // Extract description (next string after method)
                    let description = self.extract_next_string_field(&content[absolute_method_pos..], "description")
                        .unwrap_or_else(|| format!("API method: {}", method_name));
                    
                    // Extract example
                    let example_usage = self.extract_next_string_field(&content[absolute_method_pos..], "example")
                        .unwrap_or_else(|| format!("{}({{ /* parameters */ }})", method_name));
                    
                    // Extract latency
                    let estimated_latency_ms = self.extract_next_number_field(&content[absolute_method_pos..], "latency_ms")
                        .unwrap_or(1000) as u32;
                    
                    // Extract streaming flag
                    let supports_streaming = self.extract_next_boolean_field(&content[absolute_method_pos..], "streaming");
                    
                    // Create basic endpoint with minimal info
                    let endpoint = ExtractedEndpoint {
                        service_name: service_name.to_string(),
                        method_name: method_name.to_string(),
                        description,
                        input_schema: json!({"type": "object", "properties": {}}),
                        output_schema: json!({"type": "object", "properties": {}}),
                        required_permissions: Vec::new(),
                        supports_streaming,
                        tags: Vec::new(),
                        example_usage,
                        estimated_latency_ms,
                    };
                    
                    endpoints.push(endpoint);
                    current_pos = method_name_start + quote_end + 1;
                } else {
                    current_pos = absolute_method_pos + 1;
                }
            } else {
                current_pos = absolute_method_pos + 1;
            }
        }

        Ok(endpoints)
    }

    /// Extract the next string field after a given position
    fn extract_next_string_field(&self, content: &str, field_name: &str) -> Option<String> {
        if let Some(field_pos) = content.find(&format!("{}:", field_name)) {
            let after_field = &content[field_pos..];
            if let Some(quote_start) = after_field.find('"') {
                let quote_start_abs = quote_start + 1;
                if let Some(quote_end) = after_field[quote_start_abs..].find('"') {
                    return Some(after_field[quote_start_abs..quote_start_abs + quote_end].to_string());
                }
            }
        }
        None
    }

    /// Extract the next number field after a given position
    fn extract_next_number_field(&self, content: &str, field_name: &str) -> Option<i64> {
        if let Some(field_pos) = content.find(&format!("{}:", field_name)) {
            let after_field = &content[field_pos..];
            let number_regex = Regex::new(r":\s*(\d+)").ok()?;
            if let Some(captures) = number_regex.captures(after_field) {
                return captures.get(1)?.as_str().parse().ok();
            }
        }
        None
    }

    /// Extract the next boolean field after a given position
    fn extract_next_boolean_field(&self, content: &str, field_name: &str) -> bool {
        if let Some(field_pos) = content.find(&format!("{}:", field_name)) {
            let after_field = &content[field_pos..];
            if let Some(true_pos) = after_field.find("true") {
                if let Some(false_pos) = after_field.find("false") {
                    return true_pos < false_pos;
                }
                return true;
            }
        }
        false
    }

    /// Parse a single endpoint definition
    fn parse_single_endpoint(
        &self,
        endpoint_body: &str,
        method_name: &str,
        service_name: &str
    ) -> Result<Option<ExtractedEndpoint>, Box<dyn std::error::Error>> {
        let description = self.extract_string_field(endpoint_body, "description")
            .unwrap_or_else(|| format!("API method: {}", method_name));

        let example_usage = self.extract_string_field(endpoint_body, "example")
            .unwrap_or_else(|| format!("{}({{ /* parameters */ }})", method_name));

        let estimated_latency_ms = self.extract_number_field(endpoint_body, "latency_ms")
            .unwrap_or(1000) as u32;

        let supports_streaming = self.extract_boolean_field(endpoint_body, "streaming");

        // Extract tags array
        let tags = self.extract_string_array(endpoint_body, "tags");

        // Extract permissions array  
        let required_permissions = self.extract_string_array(endpoint_body, "permissions");

        // Extract JSON schemas for input and output
        let input_schema = self.extract_json_schema(endpoint_body, "input")?;
        let output_schema = self.extract_json_schema(endpoint_body, "output")?;

        Ok(Some(ExtractedEndpoint {
            service_name: service_name.to_string(),
            method_name: method_name.to_string(),
            description,
            input_schema,
            output_schema,
            required_permissions,
            supports_streaming,
            tags,
            example_usage,
            estimated_latency_ms,
        }))
    }

    /// Extract boolean field from content
    fn extract_boolean_field(&self, content: &str, field_name: &str) -> bool {
        let pattern = format!(r#"{}\s*:\s*(true|false)"#, field_name);
        let regex = match Regex::new(&pattern) {
            Ok(r) => r,
            Err(_) => return false,
        };

        regex.captures(content)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str() == "true")
            .unwrap_or(false)
    }

    /// Extract JSON schema from macro body
    fn extract_json_schema(
        &self,
        content: &str,
        field_name: &str
    ) -> Result<Value, Box<dyn std::error::Error>> {
        // Look for "field_name: json!(" pattern
        if let Some(field_start) = content.find(&format!("{}:", field_name)) {
            let after_field = &content[field_start..];
            
            if let Some(json_start) = after_field.find("json!(") {
                let absolute_json_start = field_start + json_start + 6; // 6 = len("json!(")
                
                // Find matching closing parenthesis
                let mut paren_count = 1;
                let mut end_pos = absolute_json_start;
                
                for (i, c) in content[absolute_json_start..].chars().enumerate() {
                    match c {
                        '(' => paren_count += 1,
                        ')' => {
                            paren_count -= 1;
                            if paren_count == 0 {
                                end_pos = absolute_json_start + i;
                                break;
                            }
                        },
                        _ => {}
                    }
                }
                
                let json_content = &content[absolute_json_start..end_pos];
                
                // Try to parse as JSON, with some cleanup for Rust-specific syntax
                let cleaned_json = self.clean_json_content(json_content);
                
                match serde_json::from_str::<Value>(&cleaned_json) {
                    Ok(value) => return Ok(value),
                    Err(e) => {
                        warn!("Failed to parse JSON schema for {}: {}", field_name, e);
                        warn!("Content was: {}", cleaned_json);
                        return Ok(json!({})); // Return empty object as fallback
                    }
                }
            }
        }
        
        Ok(json!({})) // Return empty object if not found
    }

    /// Clean JSON content to make it parseable
    fn clean_json_content(&self, content: &str) -> String {
        // Remove extra whitespace and handle some Rust-specific patterns
        content
            .trim()
            .replace("\"type\":", "\"type\":")
            .replace("\"properties\":", "\"properties\":")
            .replace("\"required\":", "\"required\":")
            .replace("\"description\":", "\"description\":")
            .replace("\"items\":", "\"items\":")
            .replace("\"enum\":", "\"enum\":")
    }
}
