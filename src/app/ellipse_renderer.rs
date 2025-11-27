//! Ellipse renderer for drawing filled elliptical shapes
//!
//! Similar to LineRenderer but generates ellipse geometry as triangle fans

use wgpu::{
    BindGroup, BindGroupLayout, Buffer, Device, Queue, RenderPass, RenderPipeline,
    SurfaceConfiguration, util::DeviceExt,
};

use super::shader_system::Shader;

/// WGSL shader for ellipse rendering with viewport support
const ELLIPSE_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec3<f32>,
    @location(2) alpha: f32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) alpha: f32,
}

struct Uniforms {
    screen_size: vec2<f32>,
    coord_min: vec2<f32>,
    coord_max: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // Input position is in logical coordinate space
    // Map from logical coords (coord_min..coord_max) to NDC (-1..1)
    let coord_range = uniforms.coord_max - uniforms.coord_min;
    let ndc_x = ((in.position.x - uniforms.coord_min.x) / coord_range.x) * 2.0 - 1.0;

    // Y-axis: flip for screen coordinates (top = -1, bottom = 1 in NDC)
    let ndc_y = 1.0 - ((in.position.y - uniforms.coord_min.y) / coord_range.y) * 2.0;

    out.clip_position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.color = in.color;
    out.alpha = in.alpha;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, in.alpha);
}
"#;

/// Vertex with alpha channel
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 3],
    alpha: f32,
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x3, 2 => Float32];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Uniform buffer for viewport transforms
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    screen_size: [f32; 2],
    coord_min: [f32; 2],
    coord_max: [f32; 2],
}

/// Ellipse definition (rendering command)
#[derive(Clone, Debug)]
pub struct Ellipse {
    pub center: [f32; 2],
    pub radius_x: f32,
    pub radius_y: f32,
    pub rotation: f32,
    pub color: [f32; 3],
    pub alpha: f32,
}

impl Ellipse {
    /// Generate vertices as triangle fan (center + perimeter)
    fn to_vertices(&self, segments: usize) -> Vec<Vertex> {
        let mut vertices = Vec::with_capacity(segments * 3);

        let center_vertex = Vertex {
            position: self.center,
            color: self.color,
            alpha: self.alpha,
        };

        // Generate triangles: center → edge[i] → edge[i+1]
        for i in 0..segments {
            let angle1 = (i as f32 / segments as f32) * 2.0 * std::f32::consts::PI;
            let angle2 = ((i + 1) as f32 / segments as f32) * 2.0 * std::f32::consts::PI;

            let p1 = self.point_at_angle(angle1);
            let p2 = self.point_at_angle(angle2);

            // Triangle: center, edge1, edge2
            vertices.push(center_vertex);
            vertices.push(Vertex {
                position: p1,
                color: self.color,
                alpha: self.alpha,
            });
            vertices.push(Vertex {
                position: p2,
                color: self.color,
                alpha: self.alpha,
            });
        }

        vertices
    }

    /// Calculate point on rotated ellipse at given angle
    fn point_at_angle(&self, angle: f32) -> [f32; 2] {
        // Point on unrotated ellipse
        let x = self.radius_x * angle.cos();
        let y = self.radius_y * angle.sin();

        // Rotate by ellipse rotation
        let cos_r = self.rotation.cos();
        let sin_r = self.rotation.sin();
        let rx = x * cos_r - y * sin_r;
        let ry = x * sin_r + y * cos_r;

        // Translate to center
        [self.center[0] + rx, self.center[1] + ry]
    }
}

/// Ellipse renderer shader
pub struct EllipseRenderer {
    pipeline: Option<RenderPipeline>,
    bind_group_layout: Option<BindGroupLayout>,
    bind_group: Option<BindGroup>,
    uniform_buffer: Option<Buffer>,
    vertex_buffer: Option<Buffer>,
    ellipses: Vec<Ellipse>,
    vertex_count: u32,
    screen_size: [f32; 2],
    segments_per_ellipse: usize, // Quality knob
}

impl EllipseRenderer {
    pub fn new() -> Self {
        Self {
            pipeline: None,
            bind_group_layout: None,
            bind_group: None,
            uniform_buffer: None,
            vertex_buffer: None,
            ellipses: Vec::new(),
            vertex_count: 0,
            screen_size: [800.0, 600.0],
            segments_per_ellipse: 16, // Low-poly for minimal aesthetic
        }
    }

    /// Add an ellipse to be rendered
    pub fn draw_ellipse(&mut self, ellipse: Ellipse) {
        self.ellipses.push(ellipse);
    }

    /// Clear all ellipses
    pub fn clear(&mut self) {
        self.ellipses.clear();
        self.vertex_count = 0;
    }

    /// Update vertex buffer with current ellipses
    fn update_vertex_buffer(&mut self, device: &Device, _queue: &Queue) {
        let mut vertices = Vec::new();
        for ellipse in &self.ellipses {
            vertices.extend(ellipse.to_vertices(self.segments_per_ellipse));
        }

        self.vertex_count = vertices.len() as u32;

        if vertices.is_empty() {
            return;
        }

        self.vertex_buffer = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Ellipse Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }),
        );
    }

    /// Update uniform buffer with screen size and viewport coords
    fn update_uniform_buffer(&mut self, queue: &Queue) {
        if let Some(buffer) = &self.uniform_buffer {
            let uniforms = Uniforms {
                screen_size: self.screen_size,
                coord_min: [0.0, 0.0],
                coord_max: self.screen_size,
            };
            queue.write_buffer(buffer, 0, bytemuck::cast_slice(&[uniforms]));
        }
    }
}

impl Default for EllipseRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Shader for EllipseRenderer {
    fn name(&self) -> &str {
        "ellipse"
    }

    fn init(&mut self, device: &Device, config: &SurfaceConfiguration) {
        self.screen_size = [config.width as f32, config.height as f32];

        // Create uniform buffer with viewport support
        // Default: coord_min = [0,0], coord_max = screen_size (backward compatible)
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Ellipse Uniform Buffer"),
            contents: bytemuck::cast_slice(&[Uniforms {
                screen_size: self.screen_size,
                coord_min: [0.0, 0.0],
                coord_max: self.screen_size,
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout (same structure as LineRenderer)
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Ellipse Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Ellipse Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Ellipse Shader"),
            source: wgpu::ShaderSource::Wgsl(ELLIPSE_SHADER.into()),
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Ellipse Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Ellipse Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING), // Enable alpha blending
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        self.pipeline = Some(pipeline);
        self.bind_group_layout = Some(bind_group_layout);
        self.bind_group = Some(bind_group);
        self.uniform_buffer = Some(uniform_buffer);
    }

    fn begin_frame(&mut self, device: &Device, queue: &Queue) {
        self.update_uniform_buffer(queue);
        self.update_vertex_buffer(device, queue);
    }

    fn render<'rpass>(&'rpass self, rpass: &mut RenderPass<'rpass>) {
        if self.vertex_count == 0 {
            return;
        }

        if let (Some(pipeline), Some(bind_group), Some(vertex_buffer)) =
            (&self.pipeline, &self.bind_group, &self.vertex_buffer)
        {
            rpass.set_pipeline(pipeline);
            rpass.set_bind_group(0, bind_group, &[]);
            rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
            rpass.draw(0..self.vertex_count, 0..1);
        }
    }

    fn end_frame(&mut self) {
        self.clear();
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
