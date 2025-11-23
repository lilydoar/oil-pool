//! Game application module
//!
//! Handles windowing, rendering, and user input.

mod runner;
mod window;

pub use runner::App;
pub use window::window_attributes_from_config;
