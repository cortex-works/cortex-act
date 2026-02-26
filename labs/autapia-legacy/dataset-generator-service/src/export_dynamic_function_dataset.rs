use clap::Parser;
use log::{info, error, warn};
use std::path::Path;
use std::fs;
use serde_json;

use crate::dynamic_function_dataset_generator::DynamicFunctionDatasetGenerator;

#[derive(Parser, Debug)]
#[command(name = "export-dynamic-function-dataset")]
#[command(about = "Export a function calling dataset that is always in sync with the canonical function tools registry")]
pub struct ExportDynamicFunctionDatasetArgs {
    /// Number of examples to generate per function (default: 3 for good coverage)
    #[arg(long, default_value = "3")]
    pub examples_per_function: usize,
    
    /// Output file path (default: datasets/single_turn_api.json)
    #[arg(long, default_value = "datasets/single_turn_api.json")]
    pub output_file: String,
    
    /// Whether to force overwrite existing file
    #[arg(long, default_value = "true")]
    pub force_overwrite: bool,
    
    /// Whether to validate the generated dataset
    #[arg(long, default_value = "true")]
    pub validate: bool,
}

pub async fn export_dynamic_function_dataset(args: ExportDynamicFunctionDatasetArgs) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("🚀 Starting dynamic function dataset export");
    info!("Configuration:");
    info!("  • Examples per function: {}", args.examples_per_function);
    info!("  • Output file: {}", args.output_file);
    info!("  • Force overwrite: {}", args.force_overwrite);
    info!("  • Validate: {}", args.validate);

    // Check if output file exists and handle overwrite
    if Path::new(&args.output_file).exists() && !args.force_overwrite {
        return Err(format!("Output file '{}' already exists. Use --force-overwrite to replace it.", args.output_file).into());
    }

    // Create output directory if it doesn't exist
    if let Some(parent_dir) = Path::new(&args.output_file).parent() {
        fs::create_dir_all(parent_dir)?;
        info!("✅ Created output directory: {}", parent_dir.display());
    }

    // Generate the dynamic dataset
    let mut generator = DynamicFunctionDatasetGenerator::new();
    info!("📊 Generating dataset from canonical function tools registry...");
    let examples = generator.generate_complete_dataset(args.examples_per_function);

    if examples.is_empty() {
        return Err("No examples were generated! Check the function tools registry.".into());
    }

    info!("✅ Generated {} examples", examples.len());

    // Validate the dataset if requested
    if args.validate {
        info!("🔍 Validating generated dataset...");
        validate_dataset(&examples)?;
        info!("✅ Dataset validation passed");
    }

    // Export to JSON file
    info!("💾 Exporting dataset to: {}", args.output_file);
    let json_output = serde_json::to_string_pretty(&examples)?;
    fs::write(&args.output_file, json_output)?;

    // Generate summary report
    generate_export_summary(&examples, &args.output_file)?;

    info!("🎉 Dynamic function dataset export completed successfully!");
    info!("📂 Dataset saved to: {}", args.output_file);
    
    Ok(())
}

/// Validate the generated dataset to ensure quality and completeness
fn validate_dataset(examples: &[crate::dynamic_function_dataset_generator::FiftyOneApiExample]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut function_names = std::collections::HashSet::new();
    let mut validation_errors = Vec::new();

    for (i, example) in examples.iter().enumerate() {
        // Check structure
        if example.query.len() != 1 {
            validation_errors.push(format!("Example {}: Expected exactly 1 query, got {}", i, example.query.len()));
        }
        if example.tools.len() != 1 {
            validation_errors.push(format!("Example {}: Expected exactly 1 tool, got {}", i, example.tools.len()));
        }
        if example.answers.len() != 1 {
            validation_errors.push(format!("Example {}: Expected exactly 1 answer, got {}", i, example.answers.len()));
        }

        // Validate tool format
        if !example.tools.is_empty() {
            match serde_json::from_str::<serde_json::Value>(&example.tools[0]) {
                Ok(tool) => {
                    if tool["type"] != "function" {
                        validation_errors.push(format!("Example {}: Tool type should be 'function'", i));
                    }
                    if let Some(function_name) = tool["function"]["name"].as_str() {
                        function_names.insert(function_name.to_string());
                    } else {
                        validation_errors.push(format!("Example {}: Missing function name", i));
                    }
                }
                Err(e) => {
                    validation_errors.push(format!("Example {}: Invalid tool JSON: {}", i, e));
                }
            }
        }

        // Validate answer format
        if !example.answers.is_empty() {
            match serde_json::from_str::<serde_json::Value>(&example.answers[0]) {
                Ok(answer) => {
                    if !answer["name"].is_string() {
                        validation_errors.push(format!("Example {}: Answer missing function name", i));
                    }
                    if !answer["arguments"].is_object() {
                        validation_errors.push(format!("Example {}: Answer arguments should be an object", i));
                    }
                }
                Err(e) => {
                    validation_errors.push(format!("Example {}: Invalid answer JSON: {}", i, e));
                }
            }
        }

        // Validate query is non-empty and reasonable
        if !example.query.is_empty() && example.query[0].trim().is_empty() {
            validation_errors.push(format!("Example {}: Query is empty", i));
        }
    }

    // Report validation results
    info!("📊 Validation Summary:");
    info!("  • Total examples: {}", examples.len());
    info!("  • Unique functions covered: {}", function_names.len());
    info!("  • Validation errors: {}", validation_errors.len());

    if !validation_errors.is_empty() {
        error!("❌ Dataset validation failed with {} errors:", validation_errors.len());
        for error in validation_errors.iter().take(10) { // Show first 10 errors
            error!("  • {}", error);
        }
        if validation_errors.len() > 10 {
            error!("  • ... and {} more errors", validation_errors.len() - 10);
        }
        return Err("Dataset validation failed".into());
    }

    // Check function coverage
    let expected_functions = get_expected_function_names();
    let missing_functions: Vec<_> = expected_functions.difference(&function_names).collect();
    if !missing_functions.is_empty() {
        warn!("⚠️  Missing coverage for functions: {:?}", missing_functions);
    } else {
        info!("✅ All expected functions are covered");
    }

    Ok(())
}

/// Get the expected function names from the registry for coverage validation
fn get_expected_function_names() -> std::collections::HashSet<String> {
    let generator = DynamicFunctionDatasetGenerator::new();
    let registries = generator.get_all_service_registries();
    
    registries.iter()
        .flat_map(|registry| &registry.function_tools)
        .map(|tool| tool.name.clone())
        .collect()
}

/// Generate a summary report of the exported dataset
fn generate_export_summary(examples: &[crate::dynamic_function_dataset_generator::FiftyOneApiExample], output_file: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut function_counts = std::collections::HashMap::new();
    let mut service_counts = std::collections::HashMap::new();

    // Analyze the dataset
    for example in examples {
        if !example.tools.is_empty() {
            if let Ok(tool) = serde_json::from_str::<serde_json::Value>(&example.tools[0]) {
                if let Some(function_name) = tool["function"]["name"].as_str() {
                    *function_counts.entry(function_name.to_string()).or_insert(0) += 1;
                    
                    // Extract service name from function name (assuming service_function format)
                    if let Some(service_name) = function_name.split('_').next() {
                        *service_counts.entry(service_name.to_string()).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    // Generate summary file
    let summary_file = format!("{}.summary.txt", output_file);
    let mut summary = String::new();
    
    summary.push_str("=== DYNAMIC FUNCTION DATASET EXPORT SUMMARY ===\n\n");
    summary.push_str(&format!("Export timestamp: {}\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
    summary.push_str(&format!("Dataset file: {}\n", output_file));
    summary.push_str(&format!("Total examples: {}\n", examples.len()));
    summary.push_str(&format!("Unique functions: {}\n", function_counts.len()));
    summary.push_str(&format!("Services covered: {}\n\n", service_counts.len()));

    summary.push_str("=== FUNCTION COVERAGE ===\n");
    let mut sorted_functions: Vec<_> = function_counts.iter().collect();
    sorted_functions.sort_by_key(|(name, _)| name.as_str());
    
    for (function_name, count) in sorted_functions {
        summary.push_str(&format!("  {} : {} examples\n", function_name, count));
    }

    summary.push_str("\n=== SERVICE DISTRIBUTION ===\n");
    let mut sorted_services: Vec<_> = service_counts.iter().collect();
    sorted_services.sort_by(|(_, a), (_, b)| b.cmp(a)); // Sort by count descending
    
    for (service_name, count) in sorted_services {
        summary.push_str(&format!("  {} : {} examples\n", service_name, count));
    }

    summary.push_str("\n=== EXPORT CONFIGURATION ===\n");
    summary.push_str(&format!("Examples per function: Generated dynamically\n"));
    summary.push_str(&format!("Dataset format: FiftyOne compatible\n"));
    summary.push_str(&format!("Registry sync: Automatic from shared/autapia_microservice_types\n"));

    fs::write(&summary_file, summary)?;
    info!("📋 Export summary saved to: {}", summary_file);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_export_dynamic_function_dataset() {
        let temp_dir = tempdir().unwrap();
        let output_file = temp_dir.path().join("test_dataset.json");
        
        let args = ExportDynamicFunctionDatasetArgs {
            examples_per_function: 1,
            output_file: output_file.to_string_lossy().to_string(),
            force_overwrite: true,
            validate: true,
        };

        let result = export_dynamic_function_dataset(args).await;
        assert!(result.is_ok(), "Export should succeed: {:?}", result);
        
        // Verify file was created
        assert!(output_file.exists());
        
        // Verify content is valid JSON
        let content = fs::read_to_string(&output_file).unwrap();
        let examples: Vec<crate::dynamic_function_dataset_generator::FiftyOneApiExample> = 
            serde_json::from_str(&content).unwrap();
        
        assert!(!examples.is_empty());
    }
}
