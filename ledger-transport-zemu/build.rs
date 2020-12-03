use std::{env, fs, path::PathBuf};

use protoc_rust_grpc::Codegen;

fn out_dir() -> PathBuf {
    PathBuf::from(env::var_os("OUT_DIR").unwrap())
}

fn generate_mod_rs() {
    let out_dir = out_dir();

    let mods = glob::glob(&out_dir.join("*.rs").to_string_lossy())
        .expect("glob")
        .filter_map(|p| {
            p.ok()
                .map(|p| format!("pub mod {};", p.file_stem().unwrap().to_string_lossy()))
        })
        .collect::<Vec<_>>()
        .join("\n");

    let mod_rs = out_dir.join("proto_mod.rs");
    fs::write(&mod_rs, format!("// @generated\n{}\n", mods)).expect("write");

    println!("cargo:rustc-env=PROTO_MOD_RS={}", mod_rs.to_string_lossy());
}

fn main() {
    let out = out_dir();
    Codegen::new()
        .out_dir(out)
        .input("zemu.proto")
        .rust_protobuf(true)
        .run()
        .expect("protoc-rust-grpc");
    generate_mod_rs();
}
