fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: Compile common.proto
    tonic_build::configure()
        .build_server(false)
        .build_client(false)
        .compile_protos(
            &["../../proto/eaasp/common/v1/common.proto"],
            &["../../proto"],
        )?;

    // Step 2: Compile runtime.proto (client only) with extern_path
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .extern_path(".eaasp.common.v1", "crate::common_proto")
        .compile_protos(
            &["../../proto/eaasp/runtime/v1/runtime.proto"],
            &["../../proto"],
        )?;

    Ok(())
}
