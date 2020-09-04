use protoc_rust_grpc::Codegen;

fn main() {
    Codegen::new()
        .out_dir("src")
        .input("zemu.proto")
        .rust_protobuf(true)
        .run()
        .expect("protoc-rust-grpc");
}
