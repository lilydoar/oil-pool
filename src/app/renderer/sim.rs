use super::context::RenderContext;
use super::viewport::Viewport;
use crate::app::{
    ellipse_renderer::EllipseRenderer, geometry, line_renderer::LineRenderer,
    shader_system::ShaderRegistry,
};
use crate::sim::{
    World,
    leaf::Vine,
    tictactoe::{Player, Tile},
};
use egui;
use egui_wgpu;
use wgpu;

/// Green color palette for leaves
const LEAF_COLORS: [[f32; 3]; 4] = [
    [0.2, 0.6, 0.3],   // Medium green
    [0.15, 0.7, 0.35], // Bright green
    [0.25, 0.5, 0.25], // Dark green
    [0.3, 0.65, 0.4],  // Light green
];

/// Renderer for the simulation view
pub struct SimRenderer {
    viewport: Viewport,
    shader_registry: ShaderRegistry,
    render_context: RenderContext,
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
        shader_registry.register(Box::new(LineRenderer::new())); // Board (background)
        shader_registry.register(Box::new(EllipseRenderer::new())); // Leaves (foreground)

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

        // Initialize rendering context
        let render_context = RenderContext::new(width, height);

        Self {
            viewport,
            shader_registry,
            render_context,
            width,
            height,
        }
    }

    /// Returns the texture ID for egui
    pub fn texture_id(&self) -> egui::TextureId {
        self.viewport.texture_id
    }

    /// Returns a mutable reference to the rendering context for command building
    pub fn commands(&mut self) -> &mut RenderContext {
        &mut self.render_context
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

        // Recreate render context with new size
        self.render_context = RenderContext::new(width, height);

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

    /// Initialize vines in leaf simulation (call once before drawing)
    /// Vines are stored in world coordinates (board-relative, centered at origin)
    pub fn init_vines(&mut self, world: &mut World) {
        use tracing::info;

        if let Some(leaf_sim) = world.leaf_mut()
            && leaf_sim.vines().is_empty()
        {
            info!("Initializing vines in world space (cell_size units, origin at board center)");

            // Add horizontal vines (2 interior grid lines only, matching board grid)
            // In world coords: -1.5 to +1.5 range represents the 3x3 board
            for i in 1..3 {
                let y = -1.5 + (i as f32); // i=1: -0.5, i=2: 0.5 (world coords)
                let start = [-1.5, y];
                let end = [1.5, y];
                info!(
                    "Adding horizontal vine {} at world y={}: {:?} → {:?}",
                    i, y, start, end
                );
                leaf_sim.add_vine(Vine::new(start, end));
            }

            // Add vertical vines (2 interior grid lines only, matching board grid)
            for i in 1..3 {
                let x = -1.5 + (i as f32); // i=1: -0.5, i=2: 0.5 (world coords)
                let start = [x, -1.5];
                let end = [x, 1.5];
                info!(
                    "Adding vertical vine {} at world x={}: {:?} → {:?}",
                    i, x, start, end
                );
                leaf_sim.add_vine(Vine::new(start, end));
            }

            info!("Vines initialized: {} total", leaf_sim.vines().len());
        }
    }

    /// Draws the simulation to the texture
    pub fn draw(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        world: &World,
    ) {
        // Clear command buffer for new frame
        self.render_context.clear();

        // Create board layout
        let layout = geometry::BoardLayout::centered(self.width as f32, self.height as f32);

        // Get tic-tac-toe simulation
        let tictactoe = match world.tictactoe() {
            Some(ttt) => ttt,
            None => return, // No tic-tac-toe sim, nothing to render
        };

        // Get leaf simulation
        let leaf_sim = world.leaf();

        // RENDER TICTACTOE BOARD (background layer)
        {
            // Board grid lines
            let grid_lines: Vec<([f32; 2], [f32; 2])> = geometry::generate_board_grid(&layout)
                .into_iter()
                .map(|line| (line.from, line.to))
                .collect();

            self.commands()
                .lines(&grid_lines)
                .thickness(layout.line_thickness)
                .depth(0.0);

            // Generate pieces
            let board = tictactoe.board();
            for (row, row_tiles) in board.iter().enumerate() {
                for (col, &tile) in row_tiles.iter().enumerate() {
                    match tile {
                        Tile::X => {
                            let x_lines: Vec<([f32; 2], [f32; 2])> =
                                geometry::generate_x(&layout, row, col)
                                    .into_iter()
                                    .map(|line| (line.from, line.to))
                                    .collect();

                            self.commands()
                                .lines(&x_lines)
                                .thickness(layout.line_thickness)
                                .depth(0.1);
                        }
                        Tile::O => {
                            let o_lines: Vec<([f32; 2], [f32; 2])> =
                                geometry::generate_o(&layout, row, col)
                                    .into_iter()
                                    .map(|line| (line.from, line.to))
                                    .collect();

                            self.commands()
                                .lines(&o_lines)
                                .thickness(layout.line_thickness)
                                .depth(0.1);
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
            let x_score_lines: Vec<([f32; 2], [f32; 2])> = geometry::generate_number(
                x_score,
                layout.center_x - 80.0,
                score_y,
                30.0,
                50.0,
                10.0,
                layout.line_thickness,
            )
            .into_iter()
            .map(|line| (line.from, line.to))
            .collect();

            self.commands()
                .lines(&x_score_lines)
                .thickness(layout.line_thickness)
                .depth(0.2);

            // O score on right
            let o_score_lines: Vec<([f32; 2], [f32; 2])> = geometry::generate_number(
                o_score,
                layout.center_x + 50.0,
                score_y,
                30.0,
                50.0,
                10.0,
                layout.line_thickness,
            )
            .into_iter()
            .map(|line| (line.from, line.to))
            .collect();

            self.commands()
                .lines(&o_score_lines)
                .thickness(layout.line_thickness)
                .depth(0.2);
        }

        // RENDER LEAVES (foreground layer - rendered on top of board)
        if let Some(leaf_sim) = leaf_sim {
            for leaf in leaf_sim.leaves() {
                // Transform leaf position from world space to screen space
                let screen_position = layout.world_to_screen(leaf.position);

                // Scale size and alpha by growth (0.0 = invisible, 1.0 = full)
                let current_size_x = leaf.size * leaf.growth;
                let current_size_y = current_size_x * leaf.aspect;
                let current_alpha = leaf.growth; // Fully opaque when grown

                // Calculate focus point for rotation (makes leaves appear to hang from a point)
                // For an ellipse, focus is at distance c = sqrt(a² - b²) from center
                let a_sq = current_size_x * current_size_x;
                let b_sq = current_size_y * current_size_y;
                let focus_distance = if a_sq > b_sq {
                    (a_sq - b_sq).sqrt()
                } else {
                    0.0 // Degenerate case (circle)
                };

                // Focus offset rotated by leaf rotation angle
                let focus_offset_x = focus_distance * leaf.rotation.cos();
                let focus_offset_y = focus_distance * leaf.rotation.sin();

                // Adjust center so rotation happens around focus point instead of center
                let adjusted_center = [
                    screen_position[0] - focus_offset_x,
                    screen_position[1] - focus_offset_y,
                ];

                self.commands()
                    .ellipse(adjusted_center, current_size_x, current_size_y)
                    .rotation(leaf.rotation)
                    .color(LEAF_COLORS[leaf.color_variant as usize % 4])
                    .alpha(current_alpha)
                    .depth(0.5);
            }
        }

        // Submit command buffer to shader registry (dispatches commands to shaders)
        self.render_context.submit(&mut self.shader_registry);

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
