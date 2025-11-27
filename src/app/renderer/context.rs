//! Rendering context with viewport transforms and state management

use super::super::shader_system::ShaderRegistry;
use super::command::{EllipseCommand, EllipseGeometry, LineCommand, RenderCommand};

// ============================================================================
// GEOMETRY TYPES
// ============================================================================

/// Rectangle in pixel coordinates
#[derive(Clone, Debug)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Coordinate bounds in logical space
#[derive(Clone, Debug)]
pub struct Bounds {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

// ============================================================================
// VIEWPORT CONFIGURATION
// ============================================================================

/// Viewport configuration
#[derive(Clone, Debug)]
pub struct ViewportConfig {
    /// Pixel region to render to
    pub pixel_rect: Rect,

    /// Logical coordinate bounds that map to this region
    pub coord_bounds: Bounds,

    /// Depth range (default: 0.0 to 1.0)
    pub depth_range: (f32, f32),
}

impl ViewportConfig {
    /// Create a viewport with default depth range
    pub fn new(pixel_rect: Rect, coord_bounds: Bounds) -> Self {
        Self {
            pixel_rect,
            coord_bounds,
            depth_range: (0.0, 1.0),
        }
    }
}

/// Viewport state
#[derive(Clone, Debug)]
pub struct ViewportState {
    pub pixel_rect: Rect,
    pub coord_bounds: Bounds,
    pub depth_range: (f32, f32),
}

impl From<ViewportConfig> for ViewportState {
    fn from(config: ViewportConfig) -> Self {
        Self {
            pixel_rect: config.pixel_rect,
            coord_bounds: config.coord_bounds,
            depth_range: config.depth_range,
        }
    }
}

// ============================================================================
// CONTEXT STATE
// ============================================================================

/// Complete rendering context state
#[derive(Clone, Debug)]
pub struct ContextState {
    pub viewport: ViewportState,
    pub color_tint: [f32; 4],
    pub alpha_multiplier: f32,
}

impl ContextState {
    /// Create default context state for full screen
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            viewport: ViewportState {
                pixel_rect: Rect {
                    x: 0,
                    y: 0,
                    width,
                    height,
                },
                coord_bounds: Bounds {
                    min: [0.0, 0.0],
                    max: [width as f32, height as f32],
                },
                depth_range: (0.0, 1.0),
            },
            color_tint: [1.0, 1.0, 1.0, 1.0],
            alpha_multiplier: 1.0,
        }
    }
}

impl PartialEq for ContextState {
    fn eq(&self, other: &Self) -> bool {
        // Simple comparison for deduplication
        self.viewport.pixel_rect.x == other.viewport.pixel_rect.x
            && self.viewport.pixel_rect.y == other.viewport.pixel_rect.y
            && self.viewport.pixel_rect.width == other.viewport.pixel_rect.width
            && self.viewport.pixel_rect.height == other.viewport.pixel_rect.height
    }
}

// ============================================================================
// RENDER CONTEXT
// ============================================================================

/// Main rendering context with command buffer
pub struct RenderContext {
    /// Stack of context states
    state_stack: Vec<ContextState>,

    /// Current active state
    current: ContextState,

    /// Snapshots of all context states used this frame
    context_snapshots: Vec<ContextState>,

    /// Commands reference context snapshots by ID
    commands: Vec<RenderCommand>,
}

impl RenderContext {
    /// Create a new rendering context
    pub fn new(width: u32, height: u32) -> Self {
        let default_state = ContextState::new(width, height);

        Self {
            state_stack: Vec::new(),
            current: default_state.clone(),
            context_snapshots: vec![default_state], // Pre-populate default context
            commands: Vec::with_capacity(256),      // Pre-allocate
        }
    }

    /// Capture current context state and return its ID
    pub(crate) fn capture_context_snapshot(&mut self) -> usize {
        // Check if this exact state already exists (dedup)
        if let Some(id) = self
            .context_snapshots
            .iter()
            .position(|s| s == &self.current)
        {
            return id;
        }

        // New state - snapshot it
        let id = self.context_snapshots.len();
        self.context_snapshots.push(self.current.clone());
        id
    }

    /// Push a new viewport context
    pub fn viewport(&mut self, config: ViewportConfig, f: impl FnOnce(&mut ViewportScope<'_>)) {
        // Push current state onto stack
        self.state_stack.push(self.current.clone());

        // Update current state
        self.current.viewport = config.into();

        // Execute drawing commands in new context
        let mut scope = ViewportScope { ctx: self };
        f(&mut scope);

        // Pop state
        self.current = self.state_stack.pop().expect("State stack underflow");
    }

    /// Apply color tint
    pub fn tinted(&mut self, tint: [f32; 4], f: impl FnOnce(&mut ViewportScope<'_>)) {
        self.state_stack.push(self.current.clone());

        // Multiply tints (compose)
        self.current.color_tint = [
            self.current.color_tint[0] * tint[0],
            self.current.color_tint[1] * tint[1],
            self.current.color_tint[2] * tint[2],
            self.current.color_tint[3] * tint[3],
        ];

        let mut scope = ViewportScope { ctx: self };
        f(&mut scope);

        self.current = self.state_stack.pop().expect("State stack underflow");
    }

    /// Convert logical coords to pixel coords (for debugging/UI)
    pub fn logical_to_pixels(&self, logical_pos: [f32; 2]) -> [f32; 2] {
        let bounds = &self.current.viewport.coord_bounds;
        let rect = &self.current.viewport.pixel_rect;

        let norm_x = (logical_pos[0] - bounds.min[0]) / (bounds.max[0] - bounds.min[0]);
        let norm_y = (logical_pos[1] - bounds.min[1]) / (bounds.max[1] - bounds.min[1]);

        [
            rect.x as f32 + norm_x * rect.width as f32,
            rect.y as f32 + norm_y * rect.height as f32,
        ]
    }

    /// Clear command buffer for new frame
    pub fn clear(&mut self) {
        self.commands.clear(); // Retains capacity
        self.context_snapshots.clear();
        // Re-add default context
        self.context_snapshots.push(self.current.clone());
    }

    /// Submit all commands to shader registry
    pub fn submit(&mut self, shader_registry: &mut ShaderRegistry) {
        // Sort by (context_id, depth, batch_key)
        // This minimizes GPU state changes
        self.commands.sort_by(|a, b| {
            (
                a.context_id,
                ordered_float::OrderedFloat(a.depth),
                a.command.batch_key(),
            )
                .cmp(&(
                    b.context_id,
                    ordered_float::OrderedFloat(b.depth),
                    b.command.batch_key(),
                ))
        });

        // Dispatch all commands
        // Note: Context state application (scissor/viewport) will be added
        // when we integrate with the render pass
        for cmd in &self.commands {
            cmd.command.dispatch(shader_registry);
        }

        // Don't clear here - will be done at frame start
    }

    /// Get reference to context snapshots (for render pass state application)
    pub fn context_snapshots(&self) -> &[ContextState] {
        &self.context_snapshots
    }

    /// Get reference to sorted commands (for render pass)
    pub fn commands(&self) -> &[RenderCommand] {
        &self.commands
    }

    // ===== CONVENIENCE DRAW METHODS (delegate to default viewport) =====

    /// Draw a single line (convenience method)
    pub fn line(&mut self, from: [f32; 2], to: [f32; 2]) -> LineBuilder<'_> {
        LineBuilder::new(self, from, to)
    }

    /// Draw multiple lines with shared properties (batch)
    pub fn lines(&mut self, segments: &[([f32; 2], [f32; 2])]) -> LineBuilder<'_> {
        LineBuilder::with_batch(self, segments.to_vec())
    }

    /// Draw a single ellipse (convenience method)
    pub fn ellipse(
        &mut self,
        center: [f32; 2],
        radius_x: f32,
        radius_y: f32,
    ) -> EllipseBuilder<'_> {
        EllipseBuilder::new(self, center, radius_x, radius_y)
    }

    /// Draw multiple ellipses with shared properties (batch)
    pub fn ellipses(&mut self, items: &[EllipseGeometry]) -> EllipseBuilder<'_> {
        EllipseBuilder::with_batch(self, items.to_vec())
    }

    /// Draw a circle (convenience)
    pub fn circle(&mut self, center: [f32; 2], radius: f32) -> EllipseBuilder<'_> {
        self.ellipse(center, radius, radius)
    }
}

// ============================================================================
// VIEWPORT SCOPE
// ============================================================================

/// Scoped rendering context with viewport transform
pub struct ViewportScope<'a> {
    pub(crate) ctx: &'a mut RenderContext,
}

impl<'a> ViewportScope<'a> {
    // ===== LINE API =====

    /// Draw a single line
    pub fn line(&mut self, from: [f32; 2], to: [f32; 2]) -> LineBuilder<'_> {
        LineBuilder::new(self.ctx, from, to)
    }

    /// Draw multiple lines with shared properties (batch)
    pub fn lines(&mut self, segments: &[([f32; 2], [f32; 2])]) -> LineBuilder<'_> {
        LineBuilder::with_batch(self.ctx, segments.to_vec())
    }

    // ===== ELLIPSE API =====

    /// Draw a single ellipse
    pub fn ellipse(
        &mut self,
        center: [f32; 2],
        radius_x: f32,
        radius_y: f32,
    ) -> EllipseBuilder<'_> {
        EllipseBuilder::new(self.ctx, center, radius_x, radius_y)
    }

    /// Draw multiple ellipses with shared properties (batch)
    pub fn ellipses(&mut self, items: &[EllipseGeometry]) -> EllipseBuilder<'_> {
        EllipseBuilder::with_batch(self.ctx, items.to_vec())
    }

    /// Draw a circle (convenience)
    pub fn circle(&mut self, center: [f32; 2], radius: f32) -> EllipseBuilder<'_> {
        self.ellipse(center, radius, radius)
    }

    // ===== CONTEXT NESTING =====

    /// Nested viewport
    pub fn viewport(&mut self, config: ViewportConfig, f: impl FnOnce(&mut ViewportScope<'_>)) {
        self.ctx.viewport(config, f);
    }

    /// Apply color tint
    pub fn tinted(&mut self, tint: [f32; 4], f: impl FnOnce(&mut ViewportScope<'_>)) {
        self.ctx.tinted(tint, f);
    }

    // ===== CONTEXT QUERIES =====

    /// Get viewport dimensions
    pub fn width(&self) -> u32 {
        self.ctx.current.viewport.pixel_rect.width
    }

    pub fn height(&self) -> u32 {
        self.ctx.current.viewport.pixel_rect.height
    }

    /// Convert logical coords to pixels (for hit testing, etc.)
    pub fn logical_to_pixels(&self, logical_pos: [f32; 2]) -> [f32; 2] {
        self.ctx.logical_to_pixels(logical_pos)
    }
}

// ============================================================================
// BUILDER TYPES
// ============================================================================

/// Builder for line commands
pub struct LineBuilder<'a> {
    ctx: &'a mut RenderContext,
    segments: Vec<([f32; 2], [f32; 2])>,
    thickness: f32,
    color: [f32; 3],
    depth: f32,
    context_id: usize,
}

impl<'a> LineBuilder<'a> {
    fn new(ctx: &'a mut RenderContext, from: [f32; 2], to: [f32; 2]) -> Self {
        let context_id = ctx.capture_context_snapshot();
        Self {
            ctx,
            segments: vec![(from, to)],
            thickness: 1.0,
            color: [1.0, 1.0, 1.0],
            depth: 0.0,
            context_id,
        }
    }

    fn with_batch(ctx: &'a mut RenderContext, segments: Vec<([f32; 2], [f32; 2])>) -> Self {
        let context_id = ctx.capture_context_snapshot();
        Self {
            ctx,
            segments,
            thickness: 1.0,
            color: [1.0, 1.0, 1.0],
            depth: 0.0,
            context_id,
        }
    }

    pub fn thickness(mut self, thickness: f32) -> Self {
        self.thickness = thickness;
        self
    }

    pub fn color(mut self, color: [f32; 3]) -> Self {
        self.color = color;
        self
    }

    pub fn depth(mut self, depth: f32) -> Self {
        self.depth = depth;
        self
    }
}

impl Drop for LineBuilder<'_> {
    fn drop(&mut self) {
        // Create command and submit
        let command = LineCommand {
            segments: std::mem::take(&mut self.segments),
            thickness: self.thickness,
            color: self.color,
        };

        self.ctx
            .commands
            .push(RenderCommand::new(command, self.depth, self.context_id));
    }
}

/// Builder for ellipse commands
pub struct EllipseBuilder<'a> {
    ctx: &'a mut RenderContext,
    items: Vec<EllipseGeometry>,
    color: [f32; 3],
    alpha: f32,
    depth: f32,
    context_id: usize,
}

impl<'a> EllipseBuilder<'a> {
    fn new(ctx: &'a mut RenderContext, center: [f32; 2], radius_x: f32, radius_y: f32) -> Self {
        let context_id = ctx.capture_context_snapshot();
        Self {
            ctx,
            items: vec![EllipseGeometry {
                center,
                radius_x,
                radius_y,
                rotation: 0.0,
            }],
            color: [1.0, 1.0, 1.0],
            alpha: 1.0,
            depth: 0.0,
            context_id,
        }
    }

    fn with_batch(ctx: &'a mut RenderContext, items: Vec<EllipseGeometry>) -> Self {
        let context_id = ctx.capture_context_snapshot();
        Self {
            ctx,
            items,
            color: [1.0, 1.0, 1.0],
            alpha: 1.0,
            depth: 0.0,
            context_id,
        }
    }

    pub fn rotation(mut self, rotation: f32) -> Self {
        // Apply to all items in batch
        for item in &mut self.items {
            item.rotation = rotation;
        }
        self
    }

    pub fn color(mut self, color: [f32; 3]) -> Self {
        self.color = color;
        self
    }

    pub fn alpha(mut self, alpha: f32) -> Self {
        self.alpha = alpha;
        self
    }

    pub fn depth(mut self, depth: f32) -> Self {
        self.depth = depth;
        self
    }
}

impl Drop for EllipseBuilder<'_> {
    fn drop(&mut self) {
        let command = EllipseCommand {
            items: std::mem::take(&mut self.items),
            color: self.color,
            alpha: self.alpha,
        };

        self.ctx
            .commands
            .push(RenderCommand::new(command, self.depth, self.context_id));
    }
}
