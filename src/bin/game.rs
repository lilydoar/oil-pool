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
    tracing::info!("Starting Oil Pool Game");
    tracing::info!("Build version: {}", build_info::version_string());
    tracing::debug!("Build details:\n{}", build_info::detailed_info());

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

    // OS information
    tracing::info!(
        "Runtime OS: {} {} (kernel: {})",
        System::name().unwrap_or_else(|| "Unknown".to_string()),
        System::os_version().unwrap_or_else(|| "Unknown".to_string()),
        System::kernel_version().unwrap_or_else(|| "Unknown".to_string())
    );

    // CPU information
    tracing::info!(
        "Runtime CPU: {} physical cores, {} logical cores",
        System::physical_core_count().unwrap_or(0),
        sys.cpus().len()
    );

    // Memory information (convert from bytes to GB)
    let total_memory_gb = sys.total_memory() as f64 / 1_073_741_824.0;
    tracing::info!("Runtime Memory: {:.2} GB total", total_memory_gb);

    // Architecture (from build target, which matches runtime on native builds)
    tracing::info!("Runtime Architecture: {}", build_info::CARGO_TARGET_TRIPLE);
}
