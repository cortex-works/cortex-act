use clap::{Parser, Subcommand};
use log::info;
use std::env;

// Import the modules from the main crate
use dataset_generator_service::real_api_dataset_command::RealApiDatasetCommand;

#[derive(Parser)]
#[command(name = "generate-real-api-dataset")]
#[command(about = "Generate real API datasets using extracted schemas from routing_schema.rs files")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Generate a new single_turn_api dataset using real API schemas
    Generate {
        /// Number of examples to generate
        #[arg(short, long, default_value = "1000")]
        num_examples: usize,
        
        /// Output file path
        #[arg(short, long, default_value = "./datasets/single_turn_api_real.json")]
        output: String,
        
        /// Workspace root directory
        #[arg(short, long)]
        workspace_root: Option<String>,
    },
    
    /// Show statistics about the extracted API schemas
    Stats {
        /// Workspace root directory
        #[arg(short, long)]
        workspace_root: Option<String>,
    },
    
    /// Validate an existing dataset
    Validate {
        /// Dataset file to validate
        #[arg(short, long)]
        dataset_path: String,
        
        /// Workspace root directory  
        #[arg(short, long)]
        workspace_root: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Generate { num_examples, output, workspace_root } => {
            let workspace_root = workspace_root.unwrap_or_else(|| {
                env::current_dir()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            });
            
            info!("Generating {} examples", num_examples);
            info!("Workspace root: {}", workspace_root);
            info!("Output file: {}", output);
            
            let command = RealApiDatasetCommand::new(&workspace_root);
            command.generate_real_api_dataset(&output, num_examples).await?;
            
            println!("✅ Successfully generated {} examples in {}", num_examples, output);
        },
        
        Commands::Stats { workspace_root } => {
            let workspace_root = workspace_root.unwrap_or_else(|| {
                env::current_dir()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            });
            
            let command = RealApiDatasetCommand::new(&workspace_root);
            command.show_schema_stats().await?;
        },
        
        Commands::Validate { dataset_path, workspace_root } => {
            let workspace_root = workspace_root.unwrap_or_else(|| {
                env::current_dir()
                    .unwrap()
                    .to_string_lossy()
                    .to_string()
            });
            
            let command = RealApiDatasetCommand::new(&workspace_root);
            command.validate_dataset(&dataset_path).await?;
            
            println!("✅ Dataset validation completed");
        },
    }
    
    Ok(())
}
