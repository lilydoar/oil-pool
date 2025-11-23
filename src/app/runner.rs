//! Main application handler for the game

use std::time::Instant;

use tracing::{error, info, warn};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use super::window::window_attributes_from_config;
use crate::config::AppConfig;
use crate::sim::World;

/// Main game application
pub struct App {
    config: AppConfig,
    window: Option<Window>,
    world: World,
    last_update: Option<Instant>,
}

impl App {
    /// Creates a new game application with the provided configuration
    pub fn new(config: AppConfig) -> Self {
        info!(profile = %config.profile, "Starting game");
        info!(?config.window, "Window configuration");

        Self {
            config,
            window: None,
            world: World::new(),
            last_update: None,
        }
    }

    /// Creates a new game application with configuration loaded from environment
    pub fn from_env() -> Self {
        let config = AppConfig::load_from_env().unwrap_or_else(|e| {
            warn!(error = %e, "Failed to load config, using default configuration");
            AppConfig::default()
        });
        Self::new(config)
    }
}

impl Default for App {
    fn default() -> Self {
        Self::from_env()
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = window_attributes_from_config(&self.config.window);

            match event_loop.create_window(window_attributes) {
                Ok(window) => {
                    let size = window.inner_size();
                    info!(
                        window.width = size.width,
                        window.height = size.height,
                        "Window created successfully"
                    );
                    self.window = Some(window);
                    self.last_update = Some(Instant::now());
                }
                Err(e) => {
                    error!(error = %e, "Failed to create window");
                }
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Update simulation
        if let Some(last_update) = self.last_update {
            let now = Instant::now();
            let delta_time = (now - last_update).as_secs_f32();
            self.last_update = Some(now);

            self.world.tick(delta_time);

            // Request redraw after simulation update
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                info!("Close requested, exiting");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                // TODO: Render the world state
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}
