//! Built-in health checks for core systems

pub mod build_info;
pub mod config;
pub mod debug_ui;
pub mod egui_context;
pub mod graphics_backend;
pub mod system_info;
pub mod world;

pub use build_info::BuildInfoCheck;
pub use config::ConfigCheck;
pub use debug_ui::DebugUICheck;
pub use egui_context::EguiContextCheck;
pub use graphics_backend::GraphicsBackendCheck;
pub use system_info::SystemInfoCheck;
pub use world::WorldCheck;
