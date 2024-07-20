// builds the src/cln.rs from the node.proto and primities.proto definitions
fn main() -> Result<(), std::io::Error> {
    tonic_build::configure()
        .build_client(true)
        .build_server(false)
        .out_dir("src")
        .compile(&["protos/node.proto", "protos/primitives.proto"], &["protos"])
}
