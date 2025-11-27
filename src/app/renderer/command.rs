//! Command-based rendering system
//!
//! Provides a trait-based command buffer for declarative rendering with
//! viewport-based coordinate transforms and efficient batching.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use super::super::shader_system::ShaderRegistry;
use super::super::{ellipse_renderer::EllipseRenderer, line_renderer::LineRenderer};

/// Core trait that all drawable commands implement
pub trait DrawCommand: Send + Sync {
    /// Which shader renders this command?
    fn shader_name(&self) -> &str;

    /// Dispatch this command to the shader registry
    /// Called once per command during frame submission
    fn dispatch(&self, shader_registry: &mut ShaderRegistry);

    /// Batching key - commands with same key can be batched together
    /// Format: high 32 bits = shader hash, low 32 bits = material/state hash
    fn batch_key(&self) -> u64;

    /// Approximate memory size for profiling (optional)
    fn size_hint(&self) -> usize {
        std::mem::size_of_val(self)
    }

    /// Debug name for render debugging
    fn debug_name(&self) -> &str {
        self.shader_name()
    }
}

/// Helper for computing batch keys
pub fn compute_batch_key(shader: &str, material_hash: u64) -> u64 {
    let mut hasher = DefaultHasher::new();
    shader.hash(&mut hasher);
    let shader_hash = hasher.finish();

    // Combine shader hash (high bits) with material hash (low bits)
    (shader_hash & 0xFFFFFFFF00000000) | (material_hash & 0x00000000FFFFFFFF)
}

// ============================================================================
// LINE COMMANDS
// ============================================================================

/// Batch line rendering command
#[derive(Clone, Debug)]
pub struct LineCommand {
    pub segments: Vec<([f32; 2], [f32; 2])>,
    pub thickness: f32,
    pub color: [f32; 3],
}

impl DrawCommand for LineCommand {
    fn shader_name(&self) -> &str {
        "line"
    }

    fn dispatch(&self, shader_registry: &mut ShaderRegistry) {
        if let Some(line_renderer) = shader_registry
            .get_mut("line")
            .and_then(|r| r.as_any_mut().downcast_mut::<LineRenderer>())
        {
            for (from, to) in &self.segments {
                line_renderer.draw_line(*from, *to, self.thickness);
            }
        }
    }

    fn batch_key(&self) -> u64 {
        // Lines with same thickness can batch together
        let material_hash = self.thickness.to_bits() as u64;
        compute_batch_key("line", material_hash)
    }

    fn size_hint(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.segments.len() * std::mem::size_of::<([f32; 2], [f32; 2])>()
    }

    fn debug_name(&self) -> &str {
        if self.segments.len() == 1 {
            "Line"
        } else {
            "Lines (batch)"
        }
    }
}

// ============================================================================
// ELLIPSE COMMANDS
// ============================================================================

/// Ellipse geometry data
#[derive(Clone, Debug)]
pub struct EllipseGeometry {
    pub center: [f32; 2],
    pub radius_x: f32,
    pub radius_y: f32,
    pub rotation: f32,
}

/// Batch ellipse rendering command
#[derive(Clone, Debug)]
pub struct EllipseCommand {
    pub items: Vec<EllipseGeometry>,
    pub color: [f32; 3],
    pub alpha: f32,
}

impl DrawCommand for EllipseCommand {
    fn shader_name(&self) -> &str {
        "ellipse"
    }

    fn dispatch(&self, shader_registry: &mut ShaderRegistry) {
        if let Some(ellipse_renderer) = shader_registry
            .get_mut("ellipse")
            .and_then(|r| r.as_any_mut().downcast_mut::<EllipseRenderer>())
        {
            for item in &self.items {
                ellipse_renderer.draw_ellipse(crate::app::ellipse_renderer::Ellipse {
                    center: item.center,
                    radius_x: item.radius_x,
                    radius_y: item.radius_y,
                    rotation: item.rotation,
                    color: self.color,
                    alpha: self.alpha,
                });
            }
        }
    }

    fn batch_key(&self) -> u64 {
        // Ellipses with same color/alpha can batch together
        let mut hasher = DefaultHasher::new();
        self.color[0].to_bits().hash(&mut hasher);
        self.color[1].to_bits().hash(&mut hasher);
        self.color[2].to_bits().hash(&mut hasher);
        self.alpha.to_bits().hash(&mut hasher);
        let material_hash = hasher.finish();

        compute_batch_key("ellipse", material_hash)
    }

    fn size_hint(&self) -> usize {
        std::mem::size_of::<Self>() + self.items.len() * std::mem::size_of::<EllipseGeometry>()
    }

    fn debug_name(&self) -> &str {
        if self.items.len() == 1 {
            "Ellipse"
        } else {
            "Ellipses (batch)"
        }
    }
}

// ============================================================================
// RENDER COMMAND
// ============================================================================

/// A command with rendering context
pub struct RenderCommand {
    /// The drawable command (trait object)
    pub command: Box<dyn DrawCommand>,

    /// Depth within viewport context
    pub depth: f32,

    /// Which viewport/context state to use
    pub context_id: usize,
}

impl RenderCommand {
    pub fn new(command: impl DrawCommand + 'static, depth: f32, context_id: usize) -> Self {
        Self {
            command: Box::new(command),
            depth,
            context_id,
        }
    }
}
