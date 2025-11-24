use wgpu;
use egui;
use egui_wgpu;
use crate::sim::World;
use super::viewport::Viewport;

/// Renderer for the simulation view
pub struct SimRenderer {
    viewport: Viewport,
}

impl SimRenderer {
    /// Creates a new simulation renderer
    pub fn new(
        device: &wgpu::Device,
        egui_renderer: &mut egui_wgpu::Renderer,
        width: u32,
        height: u32,
    ) -> Self {
        let viewport = Viewport::new(device, egui_renderer, width, height, "Sim Texture");

        Self {
            viewport,
        }
    }

    /// Returns the texture ID for egui
    pub fn texture_id(&self) -> egui::TextureId {
        self.viewport.texture_id
    }

    /// Resizes the render texture
    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        egui_renderer: &mut egui_wgpu::Renderer,
        width: u32,
        height: u32,
    ) {
        self.viewport.resize(device, egui_renderer, width, height);
    }

    /// Draws the simulation to the texture
    pub fn draw(&self, encoder: &mut wgpu::CommandEncoder, _world: &World) {
        let mut _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Sim Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.viewport.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.1, // Dark blue-ish background
                        g: 0.15,
                        b: 0.2,
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

        // TODO: Add actual simulation rendering here
        // For now, it just clears the texture
    }
}
