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

    Ok(())
}
