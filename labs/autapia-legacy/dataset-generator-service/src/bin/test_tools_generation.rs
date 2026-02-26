use dataset_generator_service::{
    api_config::ApiConfiguration, 
    enhanced_pipeline::EnhancedApiDatasetPipeline,
    clients::ServiceClients
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Testing Tools Generation...");
    
    // Load API configuration
    let api_config = ApiConfiguration::new();
    println!("📊 Loaded {} services with {} total endpoints", 
        api_config.services.len(),
        api_config.services.values().map(|s| s.endpoints.len()).sum::<usize>()
    );
    
    // Create pipeline with mock clients
    let clients = ServiceClients::mock_for_testing().await?;
    let pipeline = EnhancedApiDatasetPipeline::new(Arc::new(clients));
    
    // Test tools generation
    println!("\n🛠️ Generating comprehensive tools list...");
    let tools = pipeline.generate_comprehensive_tools_list();
    
    println!("✅ Generated {} tools", tools.len());
    
    // Verify we have tools for all services
    println!("\n📋 Tools by service:");
    for (service_name, _service_def) in &api_config.services {
        let service_tools: Vec<_> = tools.iter()
            .filter(|tool| tool.function.name.contains(service_name))
            .collect();
        println!("  {} → {} tools", service_name, service_tools.len());
        
        // List first few tools for verification
        for (i, tool) in service_tools.iter().take(3).enumerate() {
            println!("    {}: {}", i + 1, tool.function.name);
        }
        if service_tools.len() > 3 {
            println!("    ... and {} more", service_tools.len() - 3);
        }
    }
    
    // Check for function name uniqueness
    let mut function_names = std::collections::HashSet::new();
    let mut duplicates = Vec::new();
    
    for tool in &tools {
        if !function_names.insert(&tool.function.name) {
            duplicates.push(&tool.function.name);
        }
    }
    
    if duplicates.is_empty() {
        println!("\n✅ All function names are unique");
    } else {
        println!("\n⚠️ Found {} duplicate function names:", duplicates.len());
        for duplicate in duplicates {
            println!("  - {}", duplicate);
        }
    }
    
    println!("\n📊 Summary:");
    println!("  Total tools generated: {}", tools.len());
    println!("  Expected endpoints: {}", api_config.services.iter().map(|(_, s)| s.endpoints.len()).sum::<usize>());
    println!("  Tools coverage: {:.1}%", (tools.len() as f64 / api_config.services.iter().map(|(_, s)| s.endpoints.len()).sum::<usize>() as f64) * 100.0);
    
    Ok(())
}
