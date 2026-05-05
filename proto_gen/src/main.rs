//! Generates micropb Rust types from `proto/` into `firmware/src/proto/`.
//!
//! Run via `mise run proto:gen`. The generated `.rs` is committed; cargo
//! builds of the firmware do not need protoc on PATH.

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .ok_or("could not derive workspace root from CARGO_MANIFEST_DIR")?;

    let proto_root = workspace_root.join("proto");
    let out_path = workspace_root.join("firmware/src/proto/valve.rs");

    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut generator = micropb_gen::Generator::new();
    generator.add_protoc_arg(format!("-I{}", proto_root.display()));

    generator.compile_protos(
        &[proto_root.join("flow_controller/v1/valve.proto")],
        &out_path,
    )?;

    println!("generated {}", out_path.display());
    Ok(())
}
