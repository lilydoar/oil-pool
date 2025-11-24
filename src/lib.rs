//! Oil Pool Game
//!
//! A game built with Rust, winit, and wgpu.

/// Game application - windowing, rendering, and input handling
pub mod app;

/// Build-time information (git SHA, branch, timestamp, etc.)
pub mod build_info;

/// Health check system for validating application startup and status
pub mod health;

/// Game simulation - world state, entities, and physics
pub mod sim;
