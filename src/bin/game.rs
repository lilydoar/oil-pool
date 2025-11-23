use oil_pool::app::App;
use oil_pool::build_info;
use sysinfo::System;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};
use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    // Initialize tracing subscriber
    // Set RUST_LOG environment variable to control log level (e.g., RUST_LOG=debug)
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    // Log build information
    tracing::info!(
        git.branch = %build_info::GIT_BRANCH,
        git.sha = %build_info::git_sha_short(),
        git.dirty = build_info::is_git_dirty(),
        "Starting Oil Pool Game: {}@{}{}",
        build_info::GIT_BRANCH,
        build_info::git_sha_short(),
        if build_info::is_git_dirty() { "*" } else { "" }
    );
    tracing::debug!(
        build.timestamp = %build_info::BUILD_TIMESTAMP,
        build.target = %build_info::CARGO_TARGET_TRIPLE,
        build.opt_level = %build_info::CARGO_OPT_LEVEL,
        rustc.version = %build_info::RUSTC_SEMVER,
        rustc.channel = %build_info::RUSTC_CHANNEL,
        git.commit_timestamp = %build_info::GIT_COMMIT_TIMESTAMP,
        "Build details: {} opt-{} | rustc {} ({})",
        build_info::CARGO_TARGET_TRIPLE,
        build_info::CARGO_OPT_LEVEL,
        build_info::RUSTC_SEMVER,
        build_info::RUSTC_CHANNEL
    );

    // Log runtime system information (anonymous)
    log_system_info();

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    // Load configuration from environment (defaults to "debug" profile)
    // Set APP_PROFILE=release to use release configuration
    let mut app = App::from_env();

    event_loop
        .run_app(&mut app)
        .expect("Failed to run event loop");
}

/// Logs anonymous runtime system information
fn log_system_info() {
    let mut sys = System::new_all();
    sys.refresh_all();

    let os_name = System::name().unwrap_or_else(|| "Unknown".to_string());
    let os_version = System::os_version().unwrap_or_else(|| "Unknown".to_string());
    let kernel_version = System::kernel_version().unwrap_or_else(|| "Unknown".to_string());
    let physical_cores = System::physical_core_count().unwrap_or(0);
    let logical_cores = sys.cpus().len();
    let total_memory_gb = sys.total_memory() as f64 / 1_073_741_824.0;

    tracing::info!(
        os = %format!("{} {}", os_name, os_version),
        cpu.cores = logical_cores,
        memory.gb = %format!("{:.1}", total_memory_gb),
        "Runtime: {} {} | {} cores | {:.1} GB RAM",
        os_name,
        os_version,
        logical_cores,
        total_memory_gb
    );
    tracing::debug!(
        os.kernel = %kernel_version,
        cpu.physical_cores = physical_cores,
        cpu.logical_cores = logical_cores,
        arch = %build_info::CARGO_TARGET_TRIPLE,
        "System details: kernel {} | {} physical cores | {}",
        kernel_version,
        physical_cores,
        build_info::CARGO_TARGET_TRIPLE
    );
}
