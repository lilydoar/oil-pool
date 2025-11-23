use std::env;
use std::fs;
use std::path::Path;
use vergen::{BuildBuilder, CargoBuilder, Emitter, RustcBuilder};
use vergen_gitcl::{Emitter as GitEmitter, GitclBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Emit build metadata at build time
    let build = BuildBuilder::default()
        .build_timestamp(true) // Build timestamp
        .build()?;

    let cargo = CargoBuilder::default()
        .opt_level(true) // Optimization level
        .target_triple(true) // Target triple (e.g., x86_64-unknown-linux-gnu)
        .build()?;

    let rustc = RustcBuilder::default()
        .semver(true) // Rust compiler version
        .channel(true) // Rust channel (stable, beta, nightly)
        .build()?;

    // Emit git metadata using git command-line
    let gitcl = GitclBuilder::default()
        .sha(true) // Git commit SHA
        .branch(true) // Git branch name
        .commit_timestamp(true) // Git commit timestamp
        .dirty(true) // Whether working tree is dirty
        .build()?;

    Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&cargo)?
        .add_instructions(&rustc)?
        .emit()?;

    GitEmitter::default().add_instructions(&gitcl)?.emit()?;

    // Copy config files to output directory
    copy_configs()?;

    Ok(())
}

fn copy_configs() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = env::var("OUT_DIR")?;
    let profile = env::var("PROFILE")?;

    // Get the target directory (OUT_DIR is deep in build artifacts)
    // OUT_DIR is like: target/debug/build/oil-pool-xxx/out
    // We want: target/debug/config
    let target_dir = Path::new(&out_dir)
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .ok_or("Could not determine target directory")?;

    let config_out_dir = target_dir.join("config");
    fs::create_dir_all(&config_out_dir)?;

    // For release builds, only copy release.toml
    // For debug builds, copy both debug.toml and release.toml
    if profile == "release" {
        let release_config = Path::new("config/release.toml");
        if release_config.exists() {
            fs::copy(release_config, config_out_dir.join("release.toml"))?;
            println!("cargo:rerun-if-changed=config/release.toml");
        }
    } else {
        // Debug build - copy both profiles
        let debug_config = Path::new("config/debug.toml");
        if debug_config.exists() {
            fs::copy(debug_config, config_out_dir.join("debug.toml"))?;
            println!("cargo:rerun-if-changed=config/debug.toml");
        }

        let release_config = Path::new("config/release.toml");
        if release_config.exists() {
            fs::copy(release_config, config_out_dir.join("release.toml"))?;
            println!("cargo:rerun-if-changed=config/release.toml");
        }
    }

    Ok(())
}
