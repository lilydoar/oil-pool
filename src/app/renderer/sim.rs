use super::context::RenderContext;
use super::viewport::Viewport;
use crate::app::{
    ellipse_renderer::EllipseRenderer, line_renderer::LineRenderer, shader_system::ShaderRegistry,
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
    /// Last viewport config used for rendering (for coordinate conversion)
    last_viewport_config: Option<super::context::ViewportConfig>,
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
            last_viewport_config: None,
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

    /// Get the last viewport config used for rendering
    /// This is useful for converting screen coordinates to world coordinates for input handling
    pub fn viewport_config(&self) -> Option<&super::context::ViewportConfig> {
        self.last_viewport_config.as_ref()
    }

    /// Convert screen pixel coordinates to world coordinates
    /// Returns None if no viewport config is available yet (before first frame)
    pub fn screen_to_world(&self, screen_pos: [f32; 2]) -> Option<[f32; 2]> {
        self.last_viewport_config
            .as_ref()
            .map(|config| config.screen_to_world(screen_pos))
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

    /// Calculate viewport pixel rectangle that maintains aspect ratio
    fn calculate_viewport_rect(&self, world_bounds: &crate::sim::Bounds) -> super::context::Rect {
        use super::context::Rect;

        // Fit world bounds into available screen space while maintaining aspect ratio
        let world_aspect = world_bounds.aspect_ratio();
        let screen_aspect = self.width as f32 / self.height as f32;

        let (width, height) = if world_aspect > screen_aspect {
            // World is wider - fit to width
            let width = self.width as f32 * 0.9; // 90% of screen
            let height = width / world_aspect;
            (width, height)
        } else {
            // World is taller - fit to height
            let height = self.height as f32 * 0.9;
            let width = height * world_aspect;
            (width, height)
        };

        Rect {
            x: ((self.width as f32 - width) / 2.0) as i32,
            y: ((self.height as f32 - height) / 2.0) as i32,
            width: width as u32,
            height: height as u32,
        }
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
        use super::context::ViewportConfig;

        // Clear command buffer for new frame
        self.render_context.clear();

        // Get tic-tac-toe simulation
        let tictactoe = match world.tictactoe() {
            Some(ttt) => ttt,
            None => return, // No tic-tac-toe sim, nothing to render
        };

        // Get leaf simulation
        let leaf_sim = world.leaf();

        // Get camera from world
        let camera = world.camera();
        let world_bounds = *camera.view_bounds();

        // Calculate pixel rect (maintain aspect ratio of camera)
        let pixel_rect = self.calculate_viewport_rect(&world_bounds);

        // Convert pixel measurements to world units
        let pixels_to_world = world_bounds.width() / pixel_rect.width as f32;
        let line_thickness = 4.0 * pixels_to_world; // 4 pixels → world units

        // Convert sim::Bounds to renderer::context::Bounds
        let coord_bounds = super::context::Bounds {
            min: world_bounds.min,
            max: world_bounds.max,
        };

        let viewport_config = ViewportConfig::new(pixel_rect, coord_bounds);

        // Store for coordinate conversion in input handling
        self.last_viewport_config = Some(viewport_config.clone());

        // Render everything inside the world-coordinate viewport
        // All coordinates are now in world space (-1.5 to +1.5)
        self.render_context.viewport(viewport_config, |vp| {
            // RENDER TICTACTOE BOARD (background layer)
            // Board grid lines in world coordinates (2 horizontal + 2 vertical)
            let grid_lines: Vec<([f32; 2], [f32; 2])> = vec![
                // Horizontal lines
                ([-1.5, -0.5], [1.5, -0.5]),
                ([-1.5, 0.5], [1.5, 0.5]),
                // Vertical lines
                ([-0.5, -1.5], [-0.5, 1.5]),
                ([0.5, -1.5], [0.5, 1.5]),
            ];

            vp.lines(&grid_lines).thickness(line_thickness).depth(0.0);

            // Generate pieces in world coordinates
            let board = tictactoe.board();
            for (row, row_tiles) in board.iter().enumerate() {
                for (col, &tile) in row_tiles.iter().enumerate() {
                    // Cell center in world coords
                    let cx = -1.0 + col as f32;
                    let cy = -1.0 + row as f32;
                    let padding = 0.2; // Padding from cell edges

                    match tile {
                        Tile::X => {
                            // X is two diagonal lines
                            let x_lines = vec![
                                (
                                    [cx - 0.5 + padding, cy - 0.5 + padding],
                                    [cx + 0.5 - padding, cy + 0.5 - padding],
                                ),
                                (
                                    [cx - 0.5 + padding, cy + 0.5 - padding],
                                    [cx + 0.5 - padding, cy - 0.5 + padding],
                                ),
                            ];
                            vp.lines(&x_lines).thickness(line_thickness).depth(0.1);
                        }
                        Tile::O => {
                            // O is a circle
                            let radius = 0.5 - padding;
                            vp.circle([cx, cy], radius).depth(0.1);
                        }
                        Tile::Empty => {}
                    }
                }
            }

            // RENDER SCORES (using line-based digits in world coordinates)
            let x_score = tictactoe.wins(Player::X);
            let o_score = tictactoe.wins(Player::O);

            // Score positions in world coordinates (above the board)
            let score_y = 1.8; // Above the board (board goes from -1.5 to 1.5)
            let digit_width = 0.3;
            let digit_height = 0.5;
            let digit_thickness = line_thickness;

            // X score on left side
            let x_score_pos = [-1.0, score_y];
            render_digit(
                vp,
                x_score,
                x_score_pos,
                digit_width,
                digit_height,
                digit_thickness,
            );

            // O score on right side
            let o_score_pos = [0.7, score_y];
            render_digit(
                vp,
                o_score,
                o_score_pos,
                digit_width,
                digit_height,
                digit_thickness,
            );

            // RENDER LEAVES (foreground layer - rendered on top of board)
            // Leaves are already in world coordinates
            if let Some(leaf_sim) = leaf_sim {
                for leaf in leaf_sim.leaves() {
                    // Leaf position is already in world coordinates
                    // But leaf.size is in PIXELS, needs conversion to world units
                    let size_world = leaf.size * pixels_to_world;

                    // Scale size and alpha by growth (0.0 = invisible, 1.0 = full)
                    let current_size_x = size_world * leaf.growth;
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
                        leaf.position[0] - focus_offset_x,
                        leaf.position[1] - focus_offset_y,
                    ];

                    vp.ellipse(adjusted_center, current_size_x, current_size_y)
                        .rotation(leaf.rotation)
                        .color(LEAF_COLORS[leaf.color_variant as usize % 4])
                        .alpha(current_alpha)
                        .depth(0.5);
                }
            }
        }); // End viewport

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

/// Renders a single digit (0-9) using 7-segment display style
fn render_digit(
    vp: &mut super::context::ViewportScope<'_>,
    digit: u32,
    pos: [f32; 2],
    width: f32,
    height: f32,
    thickness: f32,
) {
    // 7-segment display segments (positioned relative to top-left corner)
    //   a
    //  f b
    //   g
    //  e c
    //   d

    let half_h = height / 2.0;

    // Segment positions (in world coordinates, relative to pos)
    let segments = [
        // a: top horizontal
        ([pos[0], pos[1]], [pos[0] + width, pos[1]]),
        // b: top-right vertical
        ([pos[0] + width, pos[1]], [pos[0] + width, pos[1] + half_h]),
        // c: bottom-right vertical
        (
            [pos[0] + width, pos[1] + half_h],
            [pos[0] + width, pos[1] + height],
        ),
        // d: bottom horizontal
        ([pos[0], pos[1] + height], [pos[0] + width, pos[1] + height]),
        // e: bottom-left vertical
        ([pos[0], pos[1] + half_h], [pos[0], pos[1] + height]),
        // f: top-left vertical
        ([pos[0], pos[1]], [pos[0], pos[1] + half_h]),
        // g: middle horizontal
        ([pos[0], pos[1] + half_h], [pos[0] + width, pos[1] + half_h]),
    ];

    // Which segments to light up for each digit (a, b, c, d, e, f, g)
    let segment_patterns = [
        [true, true, true, true, true, true, false],     // 0
        [false, true, true, false, false, false, false], // 1
        [true, true, false, true, true, false, true],    // 2
        [true, true, true, true, false, false, true],    // 3
        [false, true, true, false, false, true, true],   // 4
        [true, false, true, true, false, true, true],    // 5
        [true, false, true, true, true, true, true],     // 6
        [true, true, true, false, false, false, false],  // 7
        [true, true, true, true, true, true, true],      // 8
        [true, true, true, true, false, true, true],     // 9
    ];

    if digit >= 10 {
        return; // Only support single digits
    }

    let pattern = segment_patterns[digit as usize];
    let mut active_segments = Vec::new();

    for (i, &active) in pattern.iter().enumerate() {
        if active {
            active_segments.push(segments[i]);
        }
    }

    if !active_segments.is_empty() {
        vp.lines(&active_segments).thickness(thickness).depth(0.2);
    }
}
