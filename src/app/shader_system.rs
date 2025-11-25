//! Pluggable shader system for declarative rendering
//!
//! This module provides a shader registry that allows registering and using
//! different shader types in a declarative manner.

use std::collections::HashMap;
use wgpu::{Device, Queue, RenderPass, SurfaceConfiguration};

/// Trait that all shaders must implement
pub trait Shader: Send + Sync {
    /// Returns the shader's unique name
    fn name(&self) -> &str;

    /// Initializes the shader with the given device and configuration
    fn init(&mut self, device: &Device, config: &SurfaceConfiguration);

    /// Begins a new frame, allowing the shader to prepare for rendering
    fn begin_frame(&mut self, device: &Device, queue: &Queue);

    /// Renders the shader's contents to the given render pass
    fn render<'rpass>(&'rpass self, rpass: &mut RenderPass<'rpass>);

    /// Ends the frame, allowing cleanup
    fn end_frame(&mut self);

    /// Allows downcasting to concrete types
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// Registry for managing shaders
pub struct ShaderRegistry {
    shaders: HashMap<String, Box<dyn Shader>>,
    render_order: Vec<String>,
}

impl ShaderRegistry {
    /// Creates a new empty shader registry
    pub fn new() -> Self {
        Self {
            shaders: HashMap::new(),
            render_order: Vec::new(),
        }
    }

    /// Registers a shader with the registry
    ///
    /// Shaders are rendered in the order they are registered.
    pub fn register(&mut self, shader: Box<dyn Shader>) {
        let name = shader.name().to_string();
        self.render_order.push(name.clone());
        self.shaders.insert(name, shader);
    }

    /// Initializes all registered shaders
    pub fn init_all(&mut self, device: &Device, config: &SurfaceConfiguration) {
        for shader in self.shaders.values_mut() {
            shader.init(device, config);
        }
    }

    /// Gets a mutable reference to a shader by name
    pub fn get_mut(&mut self, name: &str) -> Option<&mut (dyn Shader + '_)> {
        if let Some(shader) = self.shaders.get_mut(name) {
            Some(shader.as_mut())
        } else {
            None
        }
    }

    /// Begins a new frame for all shaders
    pub fn begin_frame(&mut self, device: &Device, queue: &Queue) {
        for shader in self.shaders.values_mut() {
            shader.begin_frame(device, queue);
        }
    }

    /// Renders all shaders in registration order
    pub fn render_all<'rpass>(&'rpass self, rpass: &mut RenderPass<'rpass>) {
        for name in &self.render_order {
            if let Some(shader) = self.shaders.get(name) {
                shader.render(rpass);
            }
        }
    }

    /// Ends the frame for all shaders
    pub fn end_frame(&mut self) {
        for shader in self.shaders.values_mut() {
            shader.end_frame();
        }
    }
}

impl Default for ShaderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper macro for calling shaders with parameters
///
/// This provides the declarative API: `call_shader!(registry, "line", params)`
#[macro_export]
macro_rules! call_shader {
    ($registry:expr, $name:expr, $method:ident $(, $($args:expr),*)?) => {
        if let Some(shader) = $registry.get_mut($name) {
            // Downcast to specific shader type and call method
            if let Some(shader) = shader.downcast_mut::<dyn std::any::Any>() {
                shader.$method($($($args),*)?);
            }
        }
    };
}
