fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile fine-tune service proto
    tonic_build::compile_protos("proto/fine_tune.proto")?;
    
    Ok(())
} 