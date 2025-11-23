//! Game application module
//!
//! Handles windowing, rendering, and user input.

pub mod config;
mod runner;
mod window;

pub use config::{AppConfig, WindowConfig};
pub use runner::App;
pub use window::window_attributes_from_config;
