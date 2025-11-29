//! Build script for proto compilation.
//!
//! This is used during development to regenerate the proto types.
//! The generated code is committed to the repository, so this only needs
//! to run when the proto file changes.
//!
//! To regenerate: `cargo build --features regenerate-proto`
//!
//! The generated file will be placed in `src/generated.rs`.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Only regenerate if the feature is enabled
    #[cfg(feature = "regenerate-proto")]
    {
        let out_dir = std::path::PathBuf::from("src");
        tonic_build::configure()
            .out_dir(&out_dir)
            .compile_protos(&["proto/provider.proto"], &["proto"])?;

        // Rename the generated file
        let generated = out_dir.join("hemmer.provider.v1.rs");
        let target = out_dir.join("generated.rs");
        if generated.exists() {
            std::fs::rename(generated, target)?;
        }
    }

    // Always rerun if proto changes
    println!("cargo:rerun-if-changed=proto/provider.proto");

    Ok(())
}
