fn main() -> Result<(), std::io::Error> {
    let _ = std::fs::create_dir("src/lnrpc");

    tonic_build::configure()
        .build_server(false)
        .out_dir("src/lnrpc")
        .format(false)
        .compile(&["protos/lightning.proto", "protos/router.proto"], &["protos"])
}
