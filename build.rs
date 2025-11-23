use vergen::{BuildBuilder, CargoBuilder, Emitter, RustcBuilder};

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

    Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&cargo)?
        .add_instructions(&rustc)?
        .emit()?;

    Ok(())
}
