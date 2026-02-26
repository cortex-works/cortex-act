
use dataset_generator_service::api_config::ApiConfiguration;
use dataset_generator_service::enhanced_pipeline::EnhancedApiDatasetPipeline;
use dataset_generator_service::clients::ServiceClients;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing API Configuration Coverage");
    
    // Initialize the API configuration
    let api_config = ApiConfiguration::new();
    
    println!("📊 API Configuration Statistics:");
    println!("  - Total services: {}", api_config.services.len());
    
    let mut total_endpoints = 0;
    let mut total_use_cases = 0;
    
    for (service_name, service_def) in &api_config.services {
        let endpoint_count = service_def.endpoints.len();
        total_endpoints += endpoint_count;
        
        let service_use_cases: usize = service_def.endpoints.iter()
            .map(|e| e.use_cases.len())
            .sum();
        total_use_cases += service_use_cases;
        
        println!("  - {}: {} endpoints, {} use cases", 
                 service_name, endpoint_count, service_use_cases);
    }
    
    println!("📈 Totals:");
    println!("  - Total endpoints: {}", total_endpoints);
    println!("  - Total use cases: {}", total_use_cases);
    
    // Test the enhanced pipeline tools generation
    println!("\n🛠️  Testing Enhanced Pipeline Tools Generation:");
    
    // Create mock clients for testing
    let mock_clients = Arc::new(ServiceClients::mock_for_testing().await?);
    let pipeline = EnhancedApiDatasetPipeline::new(mock_clients);
    
    // Generate comprehensive tools list
    let tools = pipeline.generate_comprehensive_tools_list();
    println!("  - Generated tools: {}", tools.len());
    
    if tools.len() == total_endpoints {
        println!("  ✅ Perfect! Tools count matches endpoint count");
    } else {
        println!("  ❌ Mismatch: {} tools vs {} endpoints", tools.len(), total_endpoints);
    }
    
    // Show first 10 tools
    println!("\n🔧 First 10 tools:");
    for (i, tool) in tools.iter().take(10).enumerate() {
        println!("  {}. {}: {}", i+1, tool.function.name, tool.function.description);
    }
    
    if tools.len() > 10 {
        println!("  ... and {} more tools", tools.len() - 10);
    }
    
    Ok(())
}
