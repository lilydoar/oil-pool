use oil_pool::app::App;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};
use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    // Initialize tracing subscriber
    // Set RUST_LOG environment variable to control log level (e.g., RUST_LOG=debug)
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    // Load configuration from environment (defaults to "debug" profile)
    // Set APP_PROFILE=release to use release configuration
    let mut app = App::from_env();

    event_loop
        .run_app(&mut app)
        .expect("Failed to run event loop");
}
