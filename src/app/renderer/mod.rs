//! Rendering module for egui UI with wgpu backend
//!
//! ## Architecture
//!
//! - `command`: Trait-based command buffer system for declarative rendering
//! - `context`: Rendering context with viewport transforms and builder APIs
//! - `sim`: Simulation renderer that draws game state to offscreen texture
//! - `viewport`: Viewport texture management for rendering to egui

use std::sync::Arc;

use crate::sim::World;
use egui::Context;
use tracing::info;
use wgpu::{Device, Queue, Surface, SurfaceConfiguration};
use winit::event::WindowEvent;
use winit::window::Window;

pub mod command;
pub mod context;
pub mod sim;
pub mod viewport;
use sim::SimRenderer;

/// Renderer handles wgpu setup and egui rendering
pub struct Renderer {
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    egui_ctx: Context,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
    sim_renderer: SimRenderer,
}

impl Renderer {
    /// Returns a reference to the surface configuration
    pub fn config(&self) -> &SurfaceConfiguration {
        &self.config
    }

    /// Returns a reference to the simulation renderer
    /// Useful for coordinate conversion and other simulation-specific rendering queries
    pub fn sim_renderer(&self) -> &SimRenderer {
        &self.sim_renderer
    }

    /// Initialize vines in the leaf simulation (call once after world is created)
    pub fn init_leaf_vines(&mut self, world: &mut World) {
        self.sim_renderer.init_vines(world);
    }

    /// Creates a new renderer for the given window
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        info!("Initializing wgpu renderer");

        // Create wgpu instance
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // Create surface
        let surface = instance.create_surface(window.clone())?;

        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        info!(
            adapter.name = adapter.get_info().name,
            adapter.backend = ?adapter.get_info().backend,
            "Found GPU adapter"
        );

        // Request device and queue
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Main Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
                experimental_features: Default::default(),
            })
            .await?;

        // Configure surface
        let size = window.inner_size();
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        info!(
            surface.width = config.width,
            surface.height = config.height,
            surface.format = ?config.format,
            "Surface configured"
        );

        // Initialize egui
        let egui_ctx = Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            None,
            None,
            None,
        );

        let mut egui_renderer = egui_wgpu::Renderer::new(
            &device,
            config.format,
            egui_wgpu::RendererOptions {
                depth_stencil_format: None,
                msaa_samples: 1,
                ..Default::default()
            },
        );

        // Initialize sim renderer
        let sim_renderer =
            SimRenderer::new(&device, &mut egui_renderer, config.width, config.height);

        info!("egui initialized successfully");

        Ok(Self {
            surface,
            device,
            queue,
            config,
            egui_ctx,
            egui_state,
            egui_renderer,
            sim_renderer,
        })
    }

    /// Handles window events for egui
    pub fn handle_event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        let response = self.egui_state.on_window_event(window, event);
        response.consumed
    }

    /// Resizes the surface
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            // Resize sim renderer (currently matches window size)
            self.sim_renderer.resize(
                &self.device,
                &mut self.egui_renderer,
                new_size.width,
                new_size.height,
            );

            info!(
                width = new_size.width,
                height = new_size.height,
                "Surface resized"
            );
        }
    }

    /// Renders a frame with egui UI
    pub fn draw(
        &mut self,
        window: &Window,
        world: &World,
        mut render_ui: impl FnMut(&Context, egui::TextureId),
    ) -> Result<(), wgpu::SurfaceError> {
        // Get the surface texture
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Prepare rendering encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Draw simulation first (to offscreen texture)
        self.sim_renderer
            .draw(&mut encoder, &self.device, &self.queue, world);

        // Prepare egui
        let raw_input = self.egui_state.take_egui_input(window);
        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            render_ui(ctx, self.sim_renderer.texture_id());
        });

        // Handle platform output
        self.egui_state
            .handle_platform_output(window, full_output.platform_output);

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: window.scale_factor() as f32,
        };

        // Upload egui primitives
        let tris = self
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, *id, image_delta);
        }

        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &tris,
            &screen_descriptor,
        );

        // Render pass - use forget_lifetime() for egui_wgpu compatibility
        {
            let rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Egui Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.1,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.egui_renderer
                .render(&mut rpass.forget_lifetime(), &tris, &screen_descriptor);
        }

        // Cleanup textures
        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        // Submit commands
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
