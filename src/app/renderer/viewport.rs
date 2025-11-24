use wgpu;
use egui;
use egui_wgpu;

/// A viewport represents a render target texture that can be displayed in egui
pub struct Viewport {
    /// Texture ID for egui to display
    pub texture_id: egui::TextureId,
    /// The underlying wgpu texture
    pub texture: wgpu::Texture,
    /// The view of the texture for rendering
    pub view: wgpu::TextureView,
    /// Current width
    pub width: u32,
    /// Current height
    pub height: u32,
    /// Debug label
    label: String,
}

impl Viewport {
    /// Creates a new viewport
    pub fn new(
        device: &wgpu::Device,
        egui_renderer: &mut egui_wgpu::Renderer,
        width: u32,
        height: u32,
        label: impl Into<String>,
    ) -> Self {
        let label = label.into();
        let (texture, view) = Self::create_texture(device, width, height, &label);
        
        // Register texture with egui
        let texture_id = egui_renderer.register_native_texture(
            device,
            &view,
            wgpu::FilterMode::Linear,
        );

        Self {
            texture_id,
            texture,
            view,
            width,
            height,
            label,
        }
    }

    /// Resizes the viewport texture
    pub fn resize(
        &mut self,
        device: &wgpu::Device,
        egui_renderer: &mut egui_wgpu::Renderer,
        width: u32,
        height: u32,
    ) {
        if self.width == width && self.height == height {
            return;
        }
        
        // Ensure non-zero dimensions
        let width = width.max(1);
        let height = height.max(1);

        self.width = width;
        self.height = height;

        // Free the old texture from egui
        egui_renderer.free_texture(&self.texture_id);

        // Create new texture
        let (texture, view) = Self::create_texture(device, width, height, &self.label);
        self.texture = texture;
        self.view = view;

        // Register new texture with egui
        self.texture_id = egui_renderer.register_native_texture(
            device,
            &self.view,
            wgpu::FilterMode::Linear,
        );
    }

    /// Creates the wgpu texture and view
    fn create_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        label: &str,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // Use Rgba8UnormSrgb for general compatibility
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        (texture, view)
    }
}
