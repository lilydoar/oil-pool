//! Line renderer for drawing thick white lines
//!
//! This shader renders lines as thick white segments on a transparent background.
//! Lines are represented as quads (two triangles) to support variable thickness.

use wgpu::{
    BindGroup, BindGroupLayout, Buffer, Device, Queue, RenderPass, RenderPipeline,
    SurfaceConfiguration, util::DeviceExt,
};

use super::shader_system::Shader;

/// WGSL shader code for line rendering
const LINE_SHADER: &str = r#"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

struct Uniforms {
    screen_size: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // Convert from screen coordinates to clip space
    // Screen coordinates: (0, 0) at top-left, (width, height) at bottom-right
    // Clip space: (-1, -1) at bottom-left, (1, 1) at top-right
    let clip_x = (in.position.x / uniforms.screen_size.x) * 2.0 - 1.0;
    let clip_y = 1.0 - (in.position.y / uniforms.screen_size.y) * 2.0;

    out.clip_position = vec4<f32>(clip_x, clip_y, 0.0, 1.0);
    out.color = in.color;

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
"#;

/// Vertex data for line rendering
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 3],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x3];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// Uniform buffer for screen size
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    screen_size: [f32; 2],
}

/// Line segment definition
#[derive(Clone, Debug)]
pub struct Line {
    pub from: [f32; 2],
    pub to: [f32; 2],
    pub thickness: f32,
    pub color: [f32; 3],
}

impl Line {
    /// Creates a new white line
    pub fn new(from: [f32; 2], to: [f32; 2], thickness: f32) -> Self {
        Self {
            from,
            to,
            thickness,
            color: [1.0, 1.0, 1.0], // White
        }
    }

    /// Generates vertices for this line as a quad
    fn to_vertices(&self) -> Vec<Vertex> {
        let dx = self.to[0] - self.from[0];
        let dy = self.to[1] - self.from[1];
        let len = (dx * dx + dy * dy).sqrt();

        if len == 0.0 {
            return vec![];
        }

        // Perpendicular direction for thickness
        let px = -dy / len * self.thickness * 0.5;
        let py = dx / len * self.thickness * 0.5;

        // Four corners of the quad
        let v1 = Vertex {
            position: [self.from[0] + px, self.from[1] + py],
            color: self.color,
        };
        let v2 = Vertex {
            position: [self.from[0] - px, self.from[1] - py],
            color: self.color,
        };
        let v3 = Vertex {
            position: [self.to[0] - px, self.to[1] - py],
            color: self.color,
        };
        let v4 = Vertex {
            position: [self.to[0] + px, self.to[1] + py],
            color: self.color,
        };

        // Two triangles: (v1, v2, v3) and (v1, v3, v4)
        vec![v1, v2, v3, v1, v3, v4]
    }
}

/// Line renderer shader
pub struct LineRenderer {
    pipeline: Option<RenderPipeline>,
    bind_group_layout: Option<BindGroupLayout>,
    bind_group: Option<BindGroup>,
    uniform_buffer: Option<Buffer>,
    vertex_buffer: Option<Buffer>,
    lines: Vec<Line>,
    vertex_count: u32,
    screen_size: [f32; 2],
}

impl LineRenderer {
    /// Creates a new line renderer
    pub fn new() -> Self {
        Self {
            pipeline: None,
            bind_group_layout: None,
            bind_group: None,
            uniform_buffer: None,
            vertex_buffer: None,
            lines: Vec::new(),
            vertex_count: 0,
            screen_size: [800.0, 600.0],
        }
    }

    /// Adds a line to be rendered
    pub fn draw_line(&mut self, from: [f32; 2], to: [f32; 2], thickness: f32) {
        self.lines.push(Line::new(from, to, thickness));
    }

    /// Clears all lines
    pub fn clear(&mut self) {
        self.lines.clear();
        self.vertex_count = 0;
    }

    /// Updates the vertex buffer with current lines
    fn update_vertex_buffer(&mut self, device: &Device, _queue: &Queue) {
        // Generate vertices from all lines
        let mut vertices = Vec::new();
        for line in &self.lines {
            vertices.extend(line.to_vertices());
        }

        self.vertex_count = vertices.len() as u32;

        if vertices.is_empty() {
            return;
        }

        // Create or update vertex buffer
        self.vertex_buffer = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Line Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }),
        );
    }

    /// Updates the uniform buffer with current screen size
    fn update_uniform_buffer(&mut self, queue: &Queue) {
        if let Some(buffer) = &self.uniform_buffer {
            let uniforms = Uniforms {
                screen_size: self.screen_size,
            };
            queue.write_buffer(buffer, 0, bytemuck::cast_slice(&[uniforms]));
        }
    }
}

impl Default for LineRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Shader for LineRenderer {
    fn name(&self) -> &str {
        "line"
    }

    fn init(&mut self, device: &Device, config: &SurfaceConfiguration) {
        self.screen_size = [config.width as f32, config.height as f32];

        // Create uniform buffer
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Line Uniform Buffer"),
            contents: bytemuck::cast_slice(&[Uniforms {
                screen_size: self.screen_size,
            }]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Line Bind Group Layout"),
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
            label: Some("Line Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Create shader module
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Line Shader"),
            source: wgpu::ShaderSource::Wgsl(LINE_SHADER.into()),
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Line Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Line Render Pipeline"),
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
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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
