use super::viewport::Viewport;
use crate::app::{geometry, line_renderer::LineRenderer, shader_system::ShaderRegistry};
use crate::sim::{
    World,
    tictactoe::{Player, Tile},
};
use egui;
use egui_wgpu;
use wgpu;

/// Renderer for the simulation view
pub struct SimRenderer {
    viewport: Viewport,
    shader_registry: ShaderRegistry,
    width: u32,
    height: u32,
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

        // Initialize shader system
        let mut shader_registry = ShaderRegistry::new();
        shader_registry.register(Box::new(LineRenderer::new()));

        // Get surface config from viewport
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: viewport.texture.format(),
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        shader_registry.init_all(device, &config);

        Self {
            viewport,
            shader_registry,
            width,
            height,
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
        self.width = width;
        self.height = height;

        // Reinitialize shaders with new size
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.viewport.texture.format(),
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        self.shader_registry.init_all(device, &config);
    }

    /// Draws the simulation to the texture
    pub fn draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        world: &World,
    ) {
        // Get tic-tac-toe simulation
        let tictactoe = match world.tictactoe() {
            Some(ttt) => ttt,
            None => return, // No tic-tac-toe sim, nothing to render
        };

        // Create board layout
        let layout = geometry::BoardLayout::centered(self.width as f32, self.height as f32);

        // Get line renderer
        if let Some(line_renderer) = self.shader_registry.get_mut("line")
            && let Some(line_renderer) = line_renderer.as_any_mut().downcast_mut::<LineRenderer>()
        {
            // Generate board grid lines
            for line in geometry::generate_board_grid(&layout) {
                line_renderer.draw_line(line.from, line.to, line.thickness);
            }

            // Generate pieces
            let board = tictactoe.board();
            for (row, row_tiles) in board.iter().enumerate() {
                for (col, &tile) in row_tiles.iter().enumerate() {
                    match tile {
                        Tile::X => {
                            for line in geometry::generate_x(&layout, row, col) {
                                line_renderer.draw_line(line.from, line.to, line.thickness);
                            }
                        }
                        Tile::O => {
                            for line in geometry::generate_o(&layout, row, col) {
                                line_renderer.draw_line(line.from, line.to, line.thickness);
                            }
                        }
                        Tile::Empty => {}
                    }
                }
            }

            // Generate score numbers at top (with more padding)
            let score_y = layout.center_y - (layout.cell_size * 1.5) - 100.0;
            let x_score = tictactoe.wins(Player::X);
            let o_score = tictactoe.wins(Player::O);

            // X score on left
            for line in geometry::generate_number(
                x_score,
                layout.center_x - 80.0,
                score_y,
                30.0,
                50.0,
                10.0,
                layout.line_thickness,
            ) {
                line_renderer.draw_line(line.from, line.to, line.thickness);
            }

            // O score on right
            for line in geometry::generate_number(
                o_score,
                layout.center_x + 50.0,
                score_y,
                30.0,
                50.0,
                10.0,
                layout.line_thickness,
            ) {
                line_renderer.draw_line(line.from, line.to, line.thickness);
            }
        }

        // Prepare shaders
        self.shader_registry.begin_frame(device, queue);

        // Render pass - black background with tic-tac-toe
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Sim Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.viewport.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0, // Black background
                            g: 0.0,
                            b: 0.0,
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

            // Render all shaders (tic-tac-toe lines)
            self.shader_registry.render_all(&mut rpass);
        }

        // End frame for shaders
        self.shader_registry.end_frame();
    }
}
