//! Game application module
//!
//! Handles windowing, rendering, and user input.

pub mod config;
pub mod debug_ui;
mod geometry;
pub mod input;
mod line_renderer;
mod renderer;
mod runner;
mod shader_system;
mod window;

pub use config::{AppConfig, WindowConfig};
pub use debug_ui::DebugUIState;
pub use runner::App;
pub use window::window_attributes_from_config;
