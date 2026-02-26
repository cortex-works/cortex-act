fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_client(false) // We only need server code
        .compile_protos(
            &["../../shared/autapia_microservice_types/proto/dataset_generator.proto"],
            &["../../shared/autapia_microservice_types/proto"],
        )?;
    Ok(())
} 