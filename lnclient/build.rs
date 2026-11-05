fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Pure-Rust protobuf compile. tonic's include_proto! reads $OUT_DIR, and
    // prost-build only fills that directory after protoc succeeds.
    let cln = protox::compile(
        ["protos/cln/node.proto", "protos/cln/primitives.proto"],
        ["protos/cln"],
    )?;
    tonic_prost_build::configure()
        .build_client(true)
        .build_server(false)
        .compile_fds(cln)?;

    let lnd = protox::compile(
        ["protos/lnd/lightning.proto", "protos/lnd/router.proto"],
        ["protos/lnd"],
    )?;
    tonic_prost_build::configure()
        .build_client(true)
        .build_server(false)
        .compile_fds(lnd)?;

    println!("cargo:rerun-if-changed=protos");
    Ok(())
}
