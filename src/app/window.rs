//! Window configuration and management

use crate::config::WindowConfig;
use winit::dpi::LogicalSize;
use winit::window::{Fullscreen, WindowAttributes};

/// Creates window attributes from configuration
pub fn window_attributes_from_config(config: &WindowConfig) -> WindowAttributes {
    let mut attrs = WindowAttributes::default()
        .with_title(config.title.clone())
        .with_inner_size(LogicalSize::new(config.width, config.height))
        .with_resizable(config.resizable)
        .with_decorations(config.decorated);

    // Set fullscreen mode if requested
    if config.fullscreen {
        attrs = attrs.with_fullscreen(Some(Fullscreen::Borderless(None)));
    }

    attrs
}
