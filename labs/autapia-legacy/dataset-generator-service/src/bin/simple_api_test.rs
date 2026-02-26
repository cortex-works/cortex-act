use dataset_generator_service::api_config::ApiConfiguration;

fn main() {
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
    
    // Show some sample endpoints
    println!("\n🔧 Sample endpoints from each service:");
    for (service_name, service_def) in api_config.services.iter().take(5) {
        if let Some(first_endpoint) = service_def.endpoints.first() {
            println!("  - {}: {} ({})", 
                     service_name, 
                     first_endpoint.name, 
                     first_endpoint.endpoint);
        }
    }
    
    println!("\n✅ API Configuration loaded successfully!");
    println!("   Ready to generate comprehensive dataset with {} endpoints", total_endpoints);
}
