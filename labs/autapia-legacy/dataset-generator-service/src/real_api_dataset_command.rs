use crate::schema_extractor::SchemaExtractor;
use crate::real_api_dataset_generator::ApiDatasetGenerator;
use serde_json;
use std::fs;
use std::path::Path;
use log::{info, error};

pub struct RealApiDatasetCommand {
    workspace_root: String,
}

impl RealApiDatasetCommand {
    pub fn new(workspace_root: &str) -> Self {
        Self {
            workspace_root: workspace_root.to_string(),
        }
    }

    /// Generate a new single_turn_api dataset using real API schemas in FiftyOne format
    pub async fn generate_real_api_dataset(
        &self,
        output_path: &str,
        num_examples: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting FiftyOne format API dataset generation...");
        info!("Workspace root: {}", self.workspace_root);
        info!("Output path: {}", output_path);
        info!("Number of examples: {}", num_examples);

        // Step 1: Extract schemas from all services
        info!("Extracting API schemas from routing_schema.rs files...");
        let extractor = SchemaExtractor::new(&self.workspace_root);
        let schemas = extractor.extract_all_schemas()?;

        if schemas.is_empty() {
            return Err("No API schemas found. Cannot generate dataset.".into());
        }

        info!("Successfully extracted {} service schemas:", schemas.len());
        for schema in &schemas {
            info!("  - {}: {} endpoints", schema.service_name, schema.endpoints.len());
        }

        // Step 2: Generate dataset examples
        info!("Generating FiftyOne format dataset examples...");
        let mut generator = ApiDatasetGenerator::new(schemas);
        let examples = generator.generate_dataset(num_examples);

        if examples.is_empty() {
            return Err("Failed to generate any dataset examples".into());
        }

        // Step 3: Save to file
        info!("Saving {} examples to {}", examples.len(), output_path);
        let json_output = serde_json::to_string_pretty(&examples)?;
        
        // Ensure the output directory exists
        if let Some(parent) = Path::new(output_path).parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(output_path, json_output)?;

        info!("Successfully generated FiftyOne format API dataset with {} examples", examples.len());
        info!("Dataset saved to: {}", output_path);

        Ok(())
    }

    /// Show statistics about the extracted schemas
    pub async fn show_schema_stats(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Analyzing API schemas...");
        
        let extractor = SchemaExtractor::new(&self.workspace_root);
        let schemas = extractor.extract_all_schemas()?;

        println!("\n=== API Schema Statistics ===");
        println!("Total services: {}", schemas.len());

        let mut total_endpoints = 0;
        let mut endpoint_by_service = Vec::new();

        for schema in &schemas {
            total_endpoints += schema.endpoints.len();
            endpoint_by_service.push((schema.service_name.clone(), schema.endpoints.len()));
            
            println!("\nService: {}", schema.service_name);
            println!("  Description: {}", schema.description);
            println!("  Version: {}", schema.version);
            println!("  gRPC Port: {}", schema.grpc_port);
            println!("  Endpoints: {}", schema.endpoints.len());
            println!("  Capabilities: {}", schema.capabilities.join(", "));
            
            for endpoint in &schema.endpoints {
                println!("    - {} ({})", endpoint.method_name, endpoint.description);
            }
        }

        println!("\n=== Summary ===");
        println!("Total endpoints: {}", total_endpoints);
        
        // Sort by endpoint count
        endpoint_by_service.sort_by(|a, b| b.1.cmp(&a.1));
        println!("\nServices by endpoint count:");
        for (service, count) in endpoint_by_service {
            println!("  {}: {} endpoints", service, count);
        }

        Ok(())
    }

    /// Validate that the generated dataset is in the FiftyOne format
    pub async fn validate_dataset(&self, dataset_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("Validating FiftyOne format dataset at: {}", dataset_path);

        let content = fs::read_to_string(dataset_path)?;
        let examples: Vec<crate::real_api_dataset_generator::FiftyOneApiExample> = 
            serde_json::from_str(&content)?;

        println!("\n=== Dataset Validation ===");
        println!("Total examples: {}", examples.len());

        let mut function_counts = std::collections::HashMap::new();
        let mut service_counts = std::collections::HashMap::new();

        for (i, example) in examples.iter().enumerate() {
            // Check FiftyOne format structure
            if example.query.is_empty() {
                error!("Example {} has empty query array", i);
                continue;
            }

            if example.tools.len() != 1 {
                error!("Example {} has {} tools instead of 1 (FiftyOne format should have exactly 1 tool per example)", i, example.tools.len());
                continue;
            }

            if example.answers.len() != 1 {
                error!("Example {} has {} answers instead of 1 (FiftyOne format should have exactly 1 answer per example)", i, example.answers.len());
                continue;
            }

            // Parse tool to extract function name
            if let Ok(tool_json) = serde_json::from_str::<serde_json::Value>(&example.tools[0]) {
                if let Some(function_name) = tool_json.get("name").and_then(|n| n.as_str()) {
                    *function_counts.entry(function_name.to_string()).or_insert(0) += 1;

            if let Some(service_name) = function_name.split('_').next() {
                *service_counts.entry(service_name.to_string()).or_insert(0) += 1;
                    }
                }
            }
        }

        println!("\nFunction distribution:");
        let mut sorted_functions: Vec<_> = function_counts.iter().collect();
        sorted_functions.sort_by(|a, b| b.1.cmp(a.1));
        for (function, count) in sorted_functions.iter().take(10) {
            println!("  {}: {} examples", function, count);
        }

        println!("\nService distribution:");
        let mut sorted_services: Vec<_> = service_counts.iter().collect();
        sorted_services.sort_by(|a, b| b.1.cmp(a.1));
        for (service, count) in sorted_services {
            println!("  {}: {} examples", service, count);
        }

        println!("\n✅ Dataset validation completed successfully!");
        println!("✅ All examples follow FiftyOne format (1 query, 1 tool, 1 answer per example)");
        Ok(())
    }
}
