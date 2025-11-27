⏺ I'll help you design a layered rendering API that supports interleaved shader calls with depth ordering. Let me start with a plan.

⏺ Analysis: Current System Limitations

  Key Issues:
  1. Fixed render order - Shaders execute in registration order (src/app/shader_system.rs:78-83)
  2. No interleaving - All LineRenderer calls batch together, then all EllipseRenderer calls
  3. No depth control - Can't insert a line "between" two ellipse layers
  4. No render state - No clip rects, blend modes, or per-draw-call state
  5. Batch-only model - Each shader gets one render() call per frame

  Root cause: The begin_frame() → render() → end_frame() model assumes one batch per shader per frame.

⏺ Design: Layered Command Buffer API

  Core Concept

  Replace the current "shader batching" model with a command buffer that:
  - Accumulates draw commands with explicit depth
  - Sorts commands by depth before rendering
  - Supports arbitrary interleaving of shader types
  - Optional depth buffer for true z-testing

  Proposed API

  // Immediate declarative style - builder pattern
  renderer.line([0, 0], [100, 100])
      .thickness(4.0)
      .color([1.0, 1.0, 1.0])
      .depth(0.0)  // Background
      .clip_rect(Some(rect));

  renderer.ellipse([50, 50], 20.0, 15.0)
      .rotation(0.5)
      .color([0.2, 0.6, 0.3])
      .depth(0.5)  // Mid-layer
      .alpha(0.8);

  renderer.line([50, 50], [150, 150])
      .thickness(2.0)
      .depth(1.0);  // Foreground

  // Batch convenience for common depths
  renderer.at_depth(0.0, |r| {
      for line in board_lines {
          r.line(line.from, line.to).thickness(4.0);
      }
  });

  renderer.at_depth(0.5, |r| {
      for leaf in leaves {
          r.ellipse(leaf.pos, leaf.radius_x, leaf.radius_y)
              .rotation(leaf.angle)
              .alpha(leaf.growth);
      }
  });

  Architecture

  // New command abstraction
  pub enum DrawCommand {
      Line { from: [f32; 2], to: [f32; 2], thickness: f32, color: [f32; 3] },
      Ellipse { center: [f32; 2], radius_x: f32, radius_y: f32, rotation: f32, color: [f32; 3], alpha: f32 },
      // Future: Sprite, Mesh, Text, etc.
  }

  pub struct RenderCommand {
      command: DrawCommand,
      depth: f32,
      clip_rect: Option<Rect>,
      blend_mode: BlendMode,
  }

  pub struct CommandBuffer {
      commands: Vec<RenderCommand>,
  }

  impl CommandBuffer {
      pub fn line(&mut self, from: [f32; 2], to: [f32; 2]) -> DrawBuilder<'_> {
          DrawBuilder::new(self, DrawCommand::Line {
              from, to,
              thickness: 1.0,
              color: [1.0, 1.0, 1.0]
          })
      }

      pub fn ellipse(&mut self, center: [f32; 2], radius_x: f32, radius_y: f32) -> DrawBuilder<'_> {
          DrawBuilder::new(self, DrawCommand::Ellipse {
              center, radius_x, radius_y,
              rotation: 0.0,
              color: [1.0, 1.0, 1.0],
              alpha: 1.0
          })
      }

      pub fn at_depth(&mut self, depth: f32, f: impl FnOnce(&mut DepthScope<'_>)) {
          let mut scope = DepthScope { buffer: self, depth };
          f(&mut scope);
      }

      fn submit(&mut self) {
          // Sort by depth (stable sort preserves order for equal depths)
          self.commands.sort_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap());

          // Group consecutive commands by shader type for batching
          // Render in depth order
      }
  }

  // Builder for fluent API
  pub struct DrawBuilder<'a> {
      buffer: &'a mut CommandBuffer,
      command: DrawCommand,
      depth: f32,
      clip_rect: Option<Rect>,
      blend_mode: BlendMode,
  }

  impl<'a> DrawBuilder<'a> {
      pub fn depth(mut self, depth: f32) -> Self {
          self.depth = depth;
          self
      }

      pub fn thickness(mut self, thickness: f32) -> Self {
          if let DrawCommand::Line { thickness: ref mut t, .. } = self.command {
              *t = thickness;
          }
          self
      }

      pub fn color(mut self, color: [f32; 3]) -> Self {
          match &mut self.command {
              DrawCommand::Line { color: c, .. } | DrawCommand::Ellipse { color: c, .. } => *c = color,
          }
          self
      }

      pub fn clip_rect(mut self, rect: Option<Rect>) -> Self {
          self.clip_rect = rect;
          self
      }
  }

  impl Drop for DrawBuilder<'_> {
      fn drop(&mut self) {
          // Auto-submit when builder is dropped
          self.buffer.commands.push(RenderCommand {
              command: self.command.clone(),
              depth: self.depth,
              clip_rect: self.clip_rect,
              blend_mode: self.blend_mode,
          });
      }
  }

⏺ Pitch: Why This Design?

  1. Declarative Immediate Mode

  Current code in src/app/renderer/sim.rs:164-217 is already immediate-mode (regenerate geometry each frame). New API makes it declarative too:

  // Before: Implicit ordering, opaque batching
  line_renderer.draw_line(from, to, thickness);  // Drawn at ???
  ellipse_renderer.draw_ellipse(ellipse);         // Drawn after all lines

  // After: Explicit depth, clear intent
  renderer.line(from, to).thickness(4.0).depth(0.0);  // Background
  renderer.ellipse(center, rx, ry).depth(0.5);         // Foreground

  2. Solves Your Interleaving Problem

  // Lines → Leaves → Lines → Leaves (impossible before)
  renderer.line(grid_line).depth(0.0);     // Board grid
  renderer.ellipse(leaf1).depth(0.1);      // Leaf under score
  renderer.line(score_digit).depth(0.2);   // Score number
  renderer.ellipse(leaf2).depth(0.3);      // Leaf on top

  3. Batching Still Happens

  Command buffer sorts by depth, then groups consecutive same-type commands for efficient batching:
  - Commands at depth 0.0-0.1: 50 lines → 1 LineRenderer batch
  - Commands at depth 0.1-0.2: 30 ellipses → 1 EllipseRenderer batch
  - Commands at depth 0.2-0.3: 20 lines → 1 LineRenderer batch

  4. Depth Buffer Integration

  Current code has depth_stencil: None (src/app/line_renderer.rs:302). With depth buffer:
  - GPU handles z-testing automatically
  - Overlapping geometry "just works"
  - Can skip sorting for pixel-perfect depth

  5. Extensibility

  // Future features slot in cleanly
  renderer.sprite(texture_id, pos).depth(0.5).flip_x(true);
  renderer.text("Score: 10", pos).depth(1.0).font_size(24);
  renderer.mesh(vertices).depth(0.0).blend_mode(BlendMode::Additive);

  6. No Breaking Changes

  Keep existing LineRenderer/EllipseRenderer implementations, just change how they're invoked:
  - Old: shader_registry.get_mut("line") → manual batch
  - New: Command buffer dispatches to same renderers

  Trade-offs

  - Overhead: Sorting commands each frame (mitigated: stable sort is ~O(n log n), n is small for 2D)
  - Complexity: More indirection (DrawBuilder → RenderCommand → Shader)
  - Memory: Storing command list vs direct rendering (mitigated: commands are small, cleared each frame)

  Verdict: Overhead is negligible for typical 2D workloads (<10k draw calls/frame). Flexibility gain is massive.

⏺ Integration Plan

  Phase 1: Core Infrastructure (No Breaking Changes)

  Goal: Add command buffer without touching existing code

  1. Create src/app/renderer/command.rs
    - DrawCommand enum
    - RenderCommand struct
    - CommandBuffer struct with sorting
    - DrawBuilder for fluent API
  2. Add CommandBuffer to SimRenderer (src/app/renderer/sim.rs:24-29)
  pub struct SimRenderer {
      viewport: Viewport,
      shader_registry: ShaderRegistry,
      command_buffer: CommandBuffer,  // NEW
      width: u32,
      height: u32,
  }
  3. Keep existing methods working
    - LineRenderer::draw_line() still exists
    - EllipseRenderer::draw_ellipse() still exists
    - No changes to shader_system.rs yet

  Phase 2: Parallel API (Both Old & New Work)

  Goal: Implement command dispatch while keeping old code paths

  4. Implement command dispatch in CommandBuffer::submit()
  fn submit(&mut self, shader_registry: &mut ShaderRegistry) {
      self.commands.sort_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap());

      for cmd in &self.commands {
          match &cmd.command {
              DrawCommand::Line { from, to, thickness, color } => {
                  if let Some(line_renderer) = shader_registry.get_mut("line")
                      .and_then(|r| r.as_any_mut().downcast_mut::<LineRenderer>())
                  {
                      line_renderer.draw_line(*from, *to, *thickness);
                  }
              }
              DrawCommand::Ellipse { center, radius_x, radius_y, rotation, color, alpha } => {
                  if let Some(ellipse_renderer) = shader_registry.get_mut("ellipse")
                      .and_then(|r| r.as_any_mut().downcast_mut::<EllipseRenderer>())
                  {
                      ellipse_renderer.draw_ellipse(crate::app::ellipse_renderer::Ellipse {
                          center: *center,
                          radius_x: *radius_x,
                          radius_y: *radius_y,
                          rotation: *rotation,
                          color: *color,
                          alpha: *alpha,
                      });
                  }
              }
          }
      }

      self.commands.clear();
  }
  5. Add command buffer methods to SimRenderer
  impl SimRenderer {
      pub fn commands(&mut self) -> &mut CommandBuffer {
          &mut self.command_buffer
      }
  }
  6. Update SimRenderer::draw() to dispatch commands (src/app/renderer/sim.rs:138-299)
  pub fn draw(&mut self, encoder: &mut wgpu::CommandEncoder, device: &wgpu::Device, queue: &wgpu::Queue, world: &World) {
      // ... existing board layout code ...

      // NEW: Dispatch command buffer BEFORE shader begin_frame
      self.command_buffer.submit(&mut self.shader_registry);

      // EXISTING: Prepare shaders
      self.shader_registry.begin_frame(device, queue);

      // EXISTING: Render pass
      // ...
  }

  Phase 3: Migrate Rendering Code

  Goal: Port src/app/renderer/sim.rs:138-299 to use new API

  7. Create helper in SimRenderer for world-space commands
  impl SimRenderer {
      /// Returns a command builder that automatically transforms world coords to screen coords
      pub fn world_commands(&mut self, layout: &BoardLayout) -> WorldCommandScope<'_> {
          WorldCommandScope {
              buffer: &mut self.command_buffer,
              layout,
          }
      }
  }

  pub struct WorldCommandScope<'a> {
      buffer: &'a mut CommandBuffer,
      layout: &'a BoardLayout,
  }

  impl WorldCommandScope<'_> {
      pub fn ellipse(&mut self, world_pos: [f32; 2], radius_x: f32, radius_y: f32) -> DrawBuilder<'_> {
          let screen_pos = self.layout.world_to_screen(world_pos);
          self.buffer.ellipse(screen_pos, radius_x, radius_y)
      }
  }
  8. Refactor board rendering (src/app/renderer/sim.rs:158-218)
  // BEFORE
  if let Some(line_renderer) = self.shader_registry.get_mut("line")
      .and_then(|r| r.as_any_mut().downcast_mut::<LineRenderer>())
  {
      for line in geometry::generate_board_grid(&layout) {
          line_renderer.draw_line(line.from, line.to, line.thickness);
      }
  }

  // AFTER
  let cmds = self.commands();
  for line in geometry::generate_board_grid(&layout) {
      cmds.line(line.from, line.to)
          .thickness(line.thickness)
          .depth(0.0);  // Explicit background layer
  }
  9. Refactor leaf rendering (src/app/renderer/sim.rs:220-265)
  // BEFORE
  if let Some(ellipse_renderer) = /* ... */ {
      for leaf in leaf_sim.leaves() {
          ellipse_renderer.draw_ellipse(/* ... */);
      }
  }

  // AFTER
  let mut world_cmds = self.world_commands(&layout);
  for leaf in leaf_sim.leaves() {
      world_cmds.ellipse(leaf.position, leaf.size * leaf.growth, /* ... */)
          .rotation(leaf.rotation)
          .color(LEAF_COLORS[leaf.color_variant as usize % 4])
          .alpha(leaf.growth)
          .depth(0.5);  // Explicit foreground layer
  }

  Phase 4: Depth Buffer (Optional Enhancement)

  Goal: Add true GPU depth testing

  10. Add depth texture to Viewport (src/app/renderer/viewport.rs:5-19)
  pub struct Viewport {
      pub texture_id: egui::TextureId,
      pub texture: wgpu::Texture,
      pub view: wgpu::TextureView,
      pub depth_texture: wgpu::Texture,      // NEW
      pub depth_view: wgpu::TextureView,     // NEW
      pub width: u32,
      pub height: u32,
      label: String,
  }
  11. Update shader pipelines to use depth (src/app/line_renderer.rs:302, src/app/ellipse_renderer.rs:309)
  depth_stencil: Some(wgpu::DepthStencilState {
      format: wgpu::TextureFormat::Depth32Float,
      depth_write_enabled: true,
      depth_compare: wgpu::CompareFunction::Less,
      stencil: wgpu::StencilState::default(),
      bias: wgpu::DepthBiasState::default(),
  }),
  12. Pass depth value to shaders via uniforms
    - Add depth: f32 to vertex shader uniforms
    - Output to @builtin(frag_depth) in fragment shader

  Phase 5: Cleanup (Remove Old Code)

  Goal: Delete deprecated methods

  13. Remove direct shader access from SimRenderer::draw()
  14. Make ShaderRegistry::get_mut() private
  15. Update documentation

⏺ ---
  Summary

  Recommended API:
  // Declarative immediate mode with explicit depth layering
  renderer.line(from, to).thickness(4.0).depth(0.0);
  renderer.ellipse(center, rx, ry).rotation(angle).depth(0.5);

  // Batch helper for common depths
  renderer.at_depth(0.0, |r| {
      for line in lines { r.line(line.from, line.to); }
  });

  Key Benefits:
  - ✅ Solves interleaving problem (lines → leaves → lines → leaves)
  - ✅ Declarative depth ordering
  - ✅ Backwards compatible (keep existing shader code)
  - ✅ Extensible (sprites, text, meshes slot in easily)
  - ✅ Optional depth buffer support
  - ✅ Efficient batching (sort then group by shader type)

  Integration Path:
  1. Phase 1: Add CommandBuffer infrastructure (non-breaking)
  2. Phase 2: Implement command dispatch (both APIs work)
  3. Phase 3: Port rendering code to new API
  4. Phase 4: Optional depth buffer
  5. Phase 5: Remove deprecated code

  Alternative considered: Layer-based API with named layers (renderer.layer("background").line(...)), but depth-based is more flexible and matches GPU
  mental model.

  ---
  Next Steps

  Would you like me to:
  1. Implement Phase 1 - Create the core command buffer infrastructure?
  2. Prototype a smaller proof-of-concept - Just the command types and builder API?
  3. Refine the design - Discuss alternative approaches or API ergonomics?
  4. Something else - Different direction entirely?

  What's your preference?

> // Immediate declarative style - builder pattern
  renderer.line([0, 0], [100, 100])
      .thickness(4.0)
      .color([1.0, 1.0, 1.0])
      .depth(0.0)  // Background
      .clip_rect(Some(rect));

  renderer.ellipse([50, 50], 20.0, 15.0)
      .rotation(0.5)
      .color([0.2, 0.6, 0.3])
      .depth(0.5)  // Mid-layer
      .alpha(0.8);

  renderer.line([50, 50], [150, 150])
      .thickness(2.0)
      .depth(1.0);  // Foreground

Good points for the patterns used. Remember shaders are built to process batches. Please make this API reflect that

  // Batch convenience for common depths
  renderer.at_depth(0.0, |r| {
      for line in board_lines {
          r.line(line.from, line.to).thickness(4.0);
      }
  });

You bring it up here. But I'm not convinced this captures the whole problem.

Goal: Implement command dispatch while keeping old code paths <-- You have to support the behavior of old code paths, but you can just not use the old
functions anymore (don't delete them)

  7. Create helper in SimRenderer for world-space commands
  impl SimRenderer {
      /// Returns a command builder that automatically transforms world coords to screen coords
      pub fn world_commands(&mut self, layout: &BoardLayout) -> WorldCommandScope<'_> {
          WorldCommandScope {
              buffer: &mut self.command_buffer,
              layout,
          }
      }
  }

  pub struct WorldCommandScope<'a> {
      buffer: &'a mut CommandBuffer,
      layout: &'a BoardLayout,
  }

  impl WorldCommandScope<'_> {
      pub fn ellipse(&mut self, world_pos: [f32; 2], radius_x: f32, radius_y: f32) -> DrawBuilder<'_> {
          let screen_pos = self.layout.world_to_screen(world_pos);
          self.buffer.ellipse(screen_pos, radius_x, radius_y)
      }
  }
  ⎿  Interrupted · What should Claude do instead?

 ▐▛███▜▌   Claude Code v2.0.55
▝▜█████▛▘  Sonnet 4.5 · Claude Pro
  ▘▘ ▝▝    ~/Documents/repos/oil-pool

! git ls-files
  ⎿  .clippy.toml
     .githooks/pre-commit
     .github/workflows/rust.yml
     … +55 lines (ctrl+o to expand)

> Review this rust project from an engine standpoint. Focus on rendering and rendering sim-interaction points. Write a short review

⏺ I'll review the rendering architecture and sim-interaction points in this Rust project. Let me examine the key files.

⏺ Read(src/app/renderer/mod.rs)
  ⎿  Read 273 lines

⏺ Read(src/app/renderer/sim.rs)
  ⎿  Read 301 lines

⏺ Read(src/app/renderer/viewport.rs)
  ⎿  Read 109 lines

⏺ Read(src/app/line_renderer.rs)
  ⎿  Read 346 lines

⏺ Read(src/app/ellipse_renderer.rs)
  ⎿  Read 353 lines

⏺ Read(src/sim/mod.rs)
  ⎿  Read 260 lines

⏺ Read(src/app/shader_system.rs)
  ⎿  Read 114 lines

⏺ Read(src/app/geometry.rs)
  ⎿  Read 298 lines

⏺ Rendering Architecture Review

  Overall Structure

  Clean separation between simulation (sim/) and rendering (app/renderer/). The World manages simulations generically, while renderers read simulation
  state without coupling.

  Rendering Pipeline

  Main Flow (src/app/renderer/mod.rs:177-270):
  1. SimRenderer draws to offscreen texture
  2. Texture registered with egui as TextureId
  3. egui UI composites sim texture onto screen

  Layering (src/app/renderer/sim.rs:138-298):
  - Lines rendered first (background: board grid, X/O pieces, scores)
  - Ellipses rendered second (foreground: leaves with alpha blending)
  - Single render pass with explicit ordering via ShaderRegistry::render_order

  Sim-Renderer Interaction Points

  Good: Read-only access pattern
  - Renderers only read &World (src/app/renderer/sim.rs:143)
  - No state mutation during draw
  - Coordinate transforms isolated in BoardLayout::world_to_screen() (src/app/geometry.rs:66-71)

  Coupling point to watch:
  - init_vines() called from Renderer::init_leaf_vines() (src/app/renderer/mod.rs:35-37) mutates World
  - Initialization logic split between renderer and simulation layer - consider moving vine setup entirely to LeafSimulation::new()

  Strengths

  1. Pluggable shader system - Registration-based ordering allows easy extension
  2. Immediate mode geometry - Each frame regenerates vertices from sim state (simple, no stale state)
  3. World-to-screen transforms - Clean separation between simulation coordinates and pixel coordinates
  4. Renderer independence - LineRenderer and EllipseRenderer share no state, can be developed separately

  Potential Issues

  1. No depth buffer - Layering enforced by render order only. Works for 2 layers, but could cause issues at scale
  2. Per-frame vertex buffer recreation (src/app/line_renderer.rs:192-198) - Creating new buffers every frame. Fine for small geometry, but consider
  write_buffer() if vertex counts grow
  3. Hardcoded render order (src/app/renderer/sim.rs:43-44) - Lines always before ellipses. Consider z-ordering or explicit layer IDs if more complexity
   needed
  4. Initialization boundary blur - init_vines() crosses renderer→sim boundary for setup. Should be pure sim responsibility

  Verdict

  Solid foundation for a simple 2D engine. Architecture is extensible and separation of concerns is mostly clean. Main risk is scaling - current design
  optimized for clarity over performance, which is appropriate for the current scope.

> Problem: If I understand correctly, the rendering layer ordering currently depends on the call order of the shaders. What if I want to draw lines,
then leaves, then lines, then leaves? You are correct, I need a depth buffer, but I also require a renderer system api that cleanly exposes layer based
rendering, in a declarative & immediate fashion. I'm imagining something like addLayer(label: optional str, depth_offset: float, ...) I dunno though I'm
 looking for ideas and advice. Please recommend a rendering system API that cleanly exposes usages like "call this registered shader with these args in
this clip rect, etc". Please start with a plan, then a pitch, then an integration plan, then we can execute

⏺ I'll help you design a layered rendering API that supports interleaved shader calls with depth ordering. Let me start with a plan.

⏺ Analysis: Current System Limitations

  Key Issues:
  1. Fixed render order - Shaders execute in registration order (src/app/shader_system.rs:78-83)
  2. No interleaving - All LineRenderer calls batch together, then all EllipseRenderer calls
  3. No depth control - Can't insert a line "between" two ellipse layers
  4. No render state - No clip rects, blend modes, or per-draw-call state
  5. Batch-only model - Each shader gets one render() call per frame

  Root cause: The begin_frame() → render() → end_frame() model assumes one batch per shader per frame.

⏺ Design: Layered Command Buffer API

  Core Concept

  Replace the current "shader batching" model with a command buffer that:
  - Accumulates draw commands with explicit depth
  - Sorts commands by depth before rendering
  - Supports arbitrary interleaving of shader types
  - Optional depth buffer for true z-testing

  Proposed API

  // Immediate declarative style - builder pattern
  renderer.line([0, 0], [100, 100])
      .thickness(4.0)
      .color([1.0, 1.0, 1.0])
      .depth(0.0)  // Background
      .clip_rect(Some(rect));

  renderer.ellipse([50, 50], 20.0, 15.0)
      .rotation(0.5)
      .color([0.2, 0.6, 0.3])
      .depth(0.5)  // Mid-layer
      .alpha(0.8);

  renderer.line([50, 50], [150, 150])
      .thickness(2.0)
      .depth(1.0);  // Foreground

  // Batch convenience for common depths
  renderer.at_depth(0.0, |r| {
      for line in board_lines {
          r.line(line.from, line.to).thickness(4.0);
      }
  });

  renderer.at_depth(0.5, |r| {
      for leaf in leaves {
          r.ellipse(leaf.pos, leaf.radius_x, leaf.radius_y)
              .rotation(leaf.angle)
              .alpha(leaf.growth);
      }
  });

  Architecture

  // New command abstraction
  pub enum DrawCommand {
      Line { from: [f32; 2], to: [f32; 2], thickness: f32, color: [f32; 3] },
      Ellipse { center: [f32; 2], radius_x: f32, radius_y: f32, rotation: f32, color: [f32; 3], alpha: f32 },
      // Future: Sprite, Mesh, Text, etc.
  }

  pub struct RenderCommand {
      command: DrawCommand,
      depth: f32,
      clip_rect: Option<Rect>,
      blend_mode: BlendMode,
  }

  pub struct CommandBuffer {
      commands: Vec<RenderCommand>,
  }

  impl CommandBuffer {
      pub fn line(&mut self, from: [f32; 2], to: [f32; 2]) -> DrawBuilder<'_> {
          DrawBuilder::new(self, DrawCommand::Line {
              from, to,
              thickness: 1.0,
              color: [1.0, 1.0, 1.0]
          })
      }

      pub fn ellipse(&mut self, center: [f32; 2], radius_x: f32, radius_y: f32) -> DrawBuilder<'_> {
          DrawBuilder::new(self, DrawCommand::Ellipse {
              center, radius_x, radius_y,
              rotation: 0.0,
              color: [1.0, 1.0, 1.0],
              alpha: 1.0
          })
      }

      pub fn at_depth(&mut self, depth: f32, f: impl FnOnce(&mut DepthScope<'_>)) {
          let mut scope = DepthScope { buffer: self, depth };
          f(&mut scope);
      }

      fn submit(&mut self) {
          // Sort by depth (stable sort preserves order for equal depths)
          self.commands.sort_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap());

          // Group consecutive commands by shader type for batching
          // Render in depth order
      }
  }

  // Builder for fluent API
  pub struct DrawBuilder<'a> {
      buffer: &'a mut CommandBuffer,
      command: DrawCommand,
      depth: f32,
      clip_rect: Option<Rect>,
      blend_mode: BlendMode,
  }

  impl<'a> DrawBuilder<'a> {
      pub fn depth(mut self, depth: f32) -> Self {
          self.depth = depth;
          self
      }

      pub fn thickness(mut self, thickness: f32) -> Self {
          if let DrawCommand::Line { thickness: ref mut t, .. } = self.command {
              *t = thickness;
          }
          self
      }

      pub fn color(mut self, color: [f32; 3]) -> Self {
          match &mut self.command {
              DrawCommand::Line { color: c, .. } | DrawCommand::Ellipse { color: c, .. } => *c = color,
          }
          self
      }

      pub fn clip_rect(mut self, rect: Option<Rect>) -> Self {
          self.clip_rect = rect;
          self
      }
  }

  impl Drop for DrawBuilder<'_> {
      fn drop(&mut self) {
          // Auto-submit when builder is dropped
          self.buffer.commands.push(RenderCommand {
              command: self.command.clone(),
              depth: self.depth,
              clip_rect: self.clip_rect,
              blend_mode: self.blend_mode,
          });
      }
  }

⏺ Pitch: Why This Design?

  1. Declarative Immediate Mode

  Current code in src/app/renderer/sim.rs:164-217 is already immediate-mode (regenerate geometry each frame). New API makes it declarative too:

  // Before: Implicit ordering, opaque batching
  line_renderer.draw_line(from, to, thickness);  // Drawn at ???
  ellipse_renderer.draw_ellipse(ellipse);         // Drawn after all lines

  // After: Explicit depth, clear intent
  renderer.line(from, to).thickness(4.0).depth(0.0);  // Background
  renderer.ellipse(center, rx, ry).depth(0.5);         // Foreground

  2. Solves Your Interleaving Problem

  // Lines → Leaves → Lines → Leaves (impossible before)
  renderer.line(grid_line).depth(0.0);     // Board grid
  renderer.ellipse(leaf1).depth(0.1);      // Leaf under score
  renderer.line(score_digit).depth(0.2);   // Score number
  renderer.ellipse(leaf2).depth(0.3);      // Leaf on top

  3. Batching Still Happens

  Command buffer sorts by depth, then groups consecutive same-type commands for efficient batching:
  - Commands at depth 0.0-0.1: 50 lines → 1 LineRenderer batch
  - Commands at depth 0.1-0.2: 30 ellipses → 1 EllipseRenderer batch
  - Commands at depth 0.2-0.3: 20 lines → 1 LineRenderer batch

  4. Depth Buffer Integration

  Current code has depth_stencil: None (src/app/line_renderer.rs:302). With depth buffer:
  - GPU handles z-testing automatically
  - Overlapping geometry "just works"
  - Can skip sorting for pixel-perfect depth

  5. Extensibility

  // Future features slot in cleanly
  renderer.sprite(texture_id, pos).depth(0.5).flip_x(true);
  renderer.text("Score: 10", pos).depth(1.0).font_size(24);
  renderer.mesh(vertices).depth(0.0).blend_mode(BlendMode::Additive);

  6. No Breaking Changes

  Keep existing LineRenderer/EllipseRenderer implementations, just change how they're invoked:
  - Old: shader_registry.get_mut("line") → manual batch
  - New: Command buffer dispatches to same renderers

  Trade-offs

  - Overhead: Sorting commands each frame (mitigated: stable sort is ~O(n log n), n is small for 2D)
  - Complexity: More indirection (DrawBuilder → RenderCommand → Shader)
  - Memory: Storing command list vs direct rendering (mitigated: commands are small, cleared each frame)

  Verdict: Overhead is negligible for typical 2D workloads (<10k draw calls/frame). Flexibility gain is massive.

⏺ Integration Plan

  Phase 1: Core Infrastructure (No Breaking Changes)

  Goal: Add command buffer without touching existing code

  1. Create src/app/renderer/command.rs
    - DrawCommand enum
    - RenderCommand struct
    - CommandBuffer struct with sorting
    - DrawBuilder for fluent API
  2. Add CommandBuffer to SimRenderer (src/app/renderer/sim.rs:24-29)
  pub struct SimRenderer {
      viewport: Viewport,
      shader_registry: ShaderRegistry,
      command_buffer: CommandBuffer,  // NEW
      width: u32,
      height: u32,
  }
  3. Keep existing methods working
    - LineRenderer::draw_line() still exists
    - EllipseRenderer::draw_ellipse() still exists
    - No changes to shader_system.rs yet

  Phase 2: Parallel API (Both Old & New Work)

  Goal: Implement command dispatch while keeping old code paths

  4. Implement command dispatch in CommandBuffer::submit()
  fn submit(&mut self, shader_registry: &mut ShaderRegistry) {
      self.commands.sort_by(|a, b| a.depth.partial_cmp(&b.depth).unwrap());

      for cmd in &self.commands {
          match &cmd.command {
              DrawCommand::Line { from, to, thickness, color } => {
                  if let Some(line_renderer) = shader_registry.get_mut("line")
                      .and_then(|r| r.as_any_mut().downcast_mut::<LineRenderer>())
                  {
                      line_renderer.draw_line(*from, *to, *thickness);
                  }
              }
              DrawCommand::Ellipse { center, radius_x, radius_y, rotation, color, alpha } => {
                  if let Some(ellipse_renderer) = shader_registry.get_mut("ellipse")
                      .and_then(|r| r.as_any_mut().downcast_mut::<EllipseRenderer>())
                  {
                      ellipse_renderer.draw_ellipse(crate::app::ellipse_renderer::Ellipse {
                          center: *center,
                          radius_x: *radius_x,
                          radius_y: *radius_y,
                          rotation: *rotation,
                          color: *color,
                          alpha: *alpha,
                      });
                  }
              }
          }
      }

      self.commands.clear();
  }
  5. Add command buffer methods to SimRenderer
  impl SimRenderer {
      pub fn commands(&mut self) -> &mut CommandBuffer {
          &mut self.command_buffer
      }
  }
  6. Update SimRenderer::draw() to dispatch commands (src/app/renderer/sim.rs:138-299)
  pub fn draw(&mut self, encoder: &mut wgpu::CommandEncoder, device: &wgpu::Device, queue: &wgpu::Queue, world: &World) {
      // ... existing board layout code ...

      // NEW: Dispatch command buffer BEFORE shader begin_frame
      self.command_buffer.submit(&mut self.shader_registry);

      // EXISTING: Prepare shaders
      self.shader_registry.begin_frame(device, queue);

      // EXISTING: Render pass
      // ...
  }

  Phase 3: Migrate Rendering Code

  Goal: Port src/app/renderer/sim.rs:138-299 to use new API

  7. Create helper in SimRenderer for world-space commands
  impl SimRenderer {
      /// Returns a command builder that automatically transforms world coords to screen coords
      pub fn world_commands(&mut self, layout: &BoardLayout) -> WorldCommandScope<'_> {
          WorldCommandScope {
              buffer: &mut self.command_buffer,
              layout,
          }
      }
  }

  pub struct WorldCommandScope<'a> {
      buffer: &'a mut CommandBuffer,
      layout: &'a BoardLayout,
  }

  impl WorldCommandScope<'_> {
      pub fn ellipse(&mut self, world_pos: [f32; 2], radius_x: f32, radius_y: f32) -> DrawBuilder<'_> {
          let screen_pos = self.layout.world_to_screen(world_pos);
          self.buffer.ellipse(screen_pos, radius_x, radius_y)
      }
  }
  8. Refactor board rendering (src/app/renderer/sim.rs:158-218)
  // BEFORE
  if let Some(line_renderer) = self.shader_registry.get_mut("line")
      .and_then(|r| r.as_any_mut().downcast_mut::<LineRenderer>())
  {
      for line in geometry::generate_board_grid(&layout) {
          line_renderer.draw_line(line.from, line.to, line.thickness);
      }
  }

  // AFTER
  let cmds = self.commands();
  for line in geometry::generate_board_grid(&layout) {
      cmds.line(line.from, line.to)
          .thickness(line.thickness)
          .depth(0.0);  // Explicit background layer
  }
  9. Refactor leaf rendering (src/app/renderer/sim.rs:220-265)
  // BEFORE
  if let Some(ellipse_renderer) = /* ... */ {
      for leaf in leaf_sim.leaves() {
          ellipse_renderer.draw_ellipse(/* ... */);
      }
  }

  // AFTER
  let mut world_cmds = self.world_commands(&layout);
  for leaf in leaf_sim.leaves() {
      world_cmds.ellipse(leaf.position, leaf.size * leaf.growth, /* ... */)
          .rotation(leaf.rotation)
          .color(LEAF_COLORS[leaf.color_variant as usize % 4])
          .alpha(leaf.growth)
          .depth(0.5);  // Explicit foreground layer
  }

  Phase 4: Depth Buffer (Optional Enhancement)

  Goal: Add true GPU depth testing

  10. Add depth texture to Viewport (src/app/renderer/viewport.rs:5-19)
  pub struct Viewport {
      pub texture_id: egui::TextureId,
      pub texture: wgpu::Texture,
      pub view: wgpu::TextureView,
      pub depth_texture: wgpu::Texture,      // NEW
      pub depth_view: wgpu::TextureView,     // NEW
      pub width: u32,
      pub height: u32,
      label: String,
  }
  11. Update shader pipelines to use depth (src/app/line_renderer.rs:302, src/app/ellipse_renderer.rs:309)
  depth_stencil: Some(wgpu::DepthStencilState {
      format: wgpu::TextureFormat::Depth32Float,
      depth_write_enabled: true,
      depth_compare: wgpu::CompareFunction::Less,
      stencil: wgpu::StencilState::default(),
      bias: wgpu::DepthBiasState::default(),
  }),
  12. Pass depth value to shaders via uniforms
    - Add depth: f32 to vertex shader uniforms
    - Output to @builtin(frag_depth) in fragment shader

  Phase 5: Cleanup (Remove Old Code)

  Goal: Delete deprecated methods

  13. Remove direct shader access from SimRenderer::draw()
  14. Make ShaderRegistry::get_mut() private
  15. Update documentation

⏺ ---
  Summary

  Recommended API:
  // Declarative immediate mode with explicit depth layering
  renderer.line(from, to).thickness(4.0).depth(0.0);
  renderer.ellipse(center, rx, ry).rotation(angle).depth(0.5);

  // Batch helper for common depths
  renderer.at_depth(0.0, |r| {
      for line in lines { r.line(line.from, line.to); }
  });

  Key Benefits:
  - ✅ Solves interleaving problem (lines → leaves → lines → leaves)
  - ✅ Declarative depth ordering
  - ✅ Backwards compatible (keep existing shader code)
  - ✅ Extensible (sprites, text, meshes slot in easily)
  - ✅ Optional depth buffer support
  - ✅ Efficient batching (sort then group by shader type)

  Integration Path:
  1. Phase 1: Add CommandBuffer infrastructure (non-breaking)
  2. Phase 2: Implement command dispatch (both APIs work)
  3. Phase 3: Port rendering code to new API
  4. Phase 4: Optional depth buffer
  5. Phase 5: Remove deprecated code

  Alternative considered: Layer-based API with named layers (renderer.layer("background").line(...)), but depth-based is more flexible and matches GPU
  mental model.

  ---
  Next Steps

  Would you like me to:
  1. Implement Phase 1 - Create the core command buffer infrastructure?
  2. Prototype a smaller proof-of-concept - Just the command types and builder API?
  3. Refine the design - Discuss alternative approaches or API ergonomics?
  4. Something else - Different direction entirely?

  What's your preference?

> // Immediate declarative style - builder pattern
  renderer.line([0, 0], [100, 100])
      .thickness(4.0)
      .color([1.0, 1.0, 1.0])
      .depth(0.0)  // Background
      .clip_rect(Some(rect));

  renderer.ellipse([50, 50], 20.0, 15.0)
      .rotation(0.5)
      .color([0.2, 0.6, 0.3])
      .depth(0.5)  // Mid-layer
      .alpha(0.8);

  renderer.line([50, 50], [150, 150])
      .thickness(2.0)
      .depth(1.0);  // Foreground

Good points for the patterns used. Remember shaders are built to process batches. Please make this API reflect that

  // Batch convenience for common depths
  renderer.at_depth(0.0, |r| {
      for line in board_lines {
          r.line(line.from, line.to).thickness(4.0);
      }
  });

You bring it up here. I'm curious if we can have an api that also allows for submitting batches such as r.lines(<slice of geometry data>).thickness...

Goal: Implement command dispatch while keeping old code paths <-- You have to support the behavior of old code paths, but you can just not use the old
functions anymore (don't delete them)

  7. Create helper in SimRenderer for world-space commands
  impl SimRenderer {
      /// Returns a command builder that automatically transforms world coords to screen coords
      pub fn world_commands(&mut self, layout: &BoardLayout) -> WorldCommandScope<'_> {
          WorldCommandScope {
              buffer: &mut self.command_buffer,
              layout,
          }
      }
  }

  pub struct WorldCommandScope<'a> {
      buffer: &'a mut CommandBuffer,
      layout: &'a BoardLayout,
  }

  impl WorldCommandScope<'_> {
      pub fn ellipse(&mut self, world_pos: [f32; 2], radius_x: f32, radius_y: f32) -> DrawBuilder<'_> {
          let screen_pos = self.layout.world_to_screen(world_pos);
          self.buffer.ellipse(screen_pos, radius_x, radius_y)
      }
  }

Please explain this in slightly more detail.

  3. Refine the design - Discuss alternative approaches or API ergonomics? <-- This one
  ⎿  Interrupted · What should Claude do instead?

> Good plan though. Love the majority of it. Just some tweaks and alignments

⏺ Excellent feedback! Let me refine the design to be batch-first and explain the coordinate transform pattern better.

⏺ Refined Design: Batch-First API

  Core Principle: Match Shader Batch Processing

  Key insight: Shaders are optimized for batches, so the API should make batching natural while supporting individual draws when properties vary per
  item.

  Proposed API (Refined)

  // ===== BATCH API: Shared properties across multiple items =====

  // Submit slice of line segments (from, to) - all share thickness/color/depth
  renderer.lines(&[
      ([0.0, 0.0], [100.0, 0.0]),
      ([0.0, 100.0], [100.0, 100.0]),
      ([200.0, 0.0], [200.0, 100.0]),
  ])
  .thickness(4.0)
  .color([1.0, 1.0, 1.0])
  .depth(0.0);

  // Submit slice of ellipses - all share color/alpha/depth
  renderer.ellipses(&[
      ([50.0, 50.0], 20.0, 15.0, 0.0),  // (center, rx, ry, rotation)
      ([100.0, 100.0], 25.0, 18.0, 0.5),
  ])
  .color([0.2, 0.6, 0.3])
  .alpha(0.8)
  .depth(0.5);

  // ===== INDIVIDUAL API: When properties vary per item =====

  // Board grid lines (uniform) - use batch API
  let grid_lines: Vec<([f32; 2], [f32; 2])> = generate_board_grid(&layout)
      .into_iter()
      .map(|line| (line.from, line.to))
      .collect();

  renderer.lines(&grid_lines)
      .thickness(4.0)
      .depth(0.0);

  // Leaves (non-uniform colors/alphas) - use individual API
  for leaf in leaves {
      renderer.ellipse(leaf.position, leaf.radius_x, leaf.radius_y)
          .rotation(leaf.rotation)
          .color(LEAF_COLORS[leaf.color_variant % 4])  // Each leaf different
          .alpha(leaf.growth)  // Each leaf different
          .depth(0.5);
  }

  // ===== CONVENIENCE: Batch builder with closure =====

  renderer.at_depth(0.0, |r| {
      // Collect batch data
      let mut line_segments = Vec::new();
      for row in 1..3 {
          let y = row as f32 * cell_size;
          line_segments.push(([0.0, y], [300.0, y]));
      }

      // Submit as batch
      r.lines(&line_segments).thickness(4.0);
  });

  Design Details

  1. Batch Commands Store Slices

  pub enum DrawCommand {
      // Batch variants - store owned Vecs
      Lines {
          segments: Vec<([f32; 2], [f32; 2])>,  // Vec of (from, to)
          thickness: f32,
          color: [f32; 3],
      },
      Ellipses {
          items: Vec<EllipseGeometry>,  // Vec of geometry
          color: [f32; 3],
          alpha: f32,
      },

      // Individual variants - single item
      Line {
          from: [f32; 2],
          to: [f32; 2],
          thickness: f32,
          color: [f32; 3],
      },
      Ellipse {
          center: [f32; 2],
          radius_x: f32,
          radius_y: f32,
          rotation: f32,
          color: [f32; 3],
          alpha: f32,
      },
  }

  pub struct EllipseGeometry {
      pub center: [f32; 2],
      pub radius_x: f32,
      pub radius_y: f32,
      pub rotation: f32,
  }

  2. Builder API for Both Patterns

  impl CommandBuffer {
      // Batch API - takes slices
      pub fn lines(&mut self, segments: &[([f32; 2], [f32; 2])]) -> BatchDrawBuilder<'_> {
          BatchDrawBuilder::new(self, DrawCommand::Lines {
              segments: segments.to_vec(),  // Clone the slice
              thickness: 1.0,
              color: [1.0, 1.0, 1.0],
          })
      }

      pub fn ellipses(&mut self, items: &[EllipseGeometry]) -> BatchDrawBuilder<'_> {
          BatchDrawBuilder::new(self, DrawCommand::Ellipses {
              items: items.to_vec(),
              color: [1.0, 1.0, 1.0],
              alpha: 1.0,
          })
      }

      // Individual API - single item
      pub fn line(&mut self, from: [f32; 2], to: [f32; 2]) -> DrawBuilder<'_> {
          DrawBuilder::new(self, DrawCommand::Line {
              from, to,
              thickness: 1.0,
              color: [1.0, 1.0, 1.0],
          })
      }

      pub fn ellipse(&mut self, center: [f32; 2], radius_x: f32, radius_y: f32) -> DrawBuilder<'_> {
          DrawBuilder::new(self, DrawCommand::Ellipse {
              center, radius_x, radius_y,
              rotation: 0.0,
              color: [1.0, 1.0, 1.0],
              alpha: 1.0,
          })
      }
  }

  // Builder for batch commands
  pub struct BatchDrawBuilder<'a> {
      buffer: &'a mut CommandBuffer,
      command: DrawCommand,
      depth: f32,
      clip_rect: Option<Rect>,
  }

  impl<'a> BatchDrawBuilder<'a> {
      pub fn thickness(mut self, thickness: f32) -> Self {
          if let DrawCommand::Lines { thickness: ref mut t, .. } = self.command {
              *t = thickness;
          }
          self
      }

      pub fn color(mut self, color: [f32; 3]) -> Self {
          match &mut self.command {
              DrawCommand::Lines { color: c, .. }
              | DrawCommand::Ellipses { color: c, .. } => *c = color,
              _ => {}
          }
          self
      }

      pub fn depth(mut self, depth: f32) -> Self {
          self.depth = depth;
          self
      }
  }

  impl Drop for BatchDrawBuilder<'_> {
      fn drop(&mut self) {
          // Auto-submit when builder drops
          self.buffer.commands.push(RenderCommand {
              command: self.command.clone(),
              depth: self.depth,
              clip_rect: self.clip_rect,
          });
      }
  }

  // Similar for DrawBuilder (individual items)

⏺ WorldCommandScope: Coordinate Transform Pattern Explained

  The Problem

  Your sim uses two coordinate systems:

  1. World coordinates (sim space) - src/sim/leaf.rs stores leaf positions
    - Origin at board center
    - Units in cell_size
    - Range: [-1.5, 1.5] for 3x3 board
    - Example: [0.0, 0.0] = center of board, [-0.5, 0.5] = top-left cell center
  2. Screen coordinates (pixel space) - LineRenderer/EllipseRenderer expect pixels
    - Origin at top-left corner
    - Units in pixels
    - Range: [0, width] x [0, height]
    - Example: [400, 300] = center of 800x600 window

  Current Approach (src/app/renderer/sim.rs:229)

  Each renderer call manually transforms:
  for leaf in leaf_sim.leaves() {
      // MANUAL TRANSFORM: world → screen
      let screen_position = layout.world_to_screen(leaf.position);

      // Then render at screen position
      ellipse_renderer.draw_ellipse(Ellipse {
          center: screen_position,  // Converted
          // ...
      });
  }

  Problem: Every draw call site must remember to transform coordinates. Easy to forget or mix up coordinate spaces.

  Proposed Solution: Scoped Transform API

  // Create a scoped command builder that auto-transforms
  let mut world_cmds = renderer.world_commands(&layout);

  // Now all positions are in WORLD SPACE automatically
  for leaf in leaf_sim.leaves() {
      world_cmds.ellipse(
          leaf.position,  // WORLD COORDS - no manual transform!
          leaf.radius_x,
          leaf.radius_y
      )
      .rotation(leaf.rotation)
      .color(LEAF_COLORS[leaf.color_variant % 4])
      .alpha(leaf.growth)
      .depth(0.5);
  }

  Implementation

  impl SimRenderer {
      /// Returns a command scope that works in world coordinates
      pub fn world_commands(&mut self, layout: &BoardLayout) -> WorldCommandScope<'_> {
          WorldCommandScope {
              buffer: &mut self.command_buffer,
              layout,
          }
      }
  }

  /// A command builder that auto-transforms world coords → screen coords
  pub struct WorldCommandScope<'a> {
      buffer: &'a mut CommandBuffer,
      layout: &'a BoardLayout,  // Holds transform parameters
  }

  impl WorldCommandScope<'_> {
      /// Draw ellipse at WORLD position (will be transformed to screen)
      pub fn ellipse(
          &mut self,
          world_pos: [f32; 2],  // Takes world coords
          radius_x: f32,
          radius_y: f32
      ) -> DrawBuilder<'_> {
          // TRANSFORM HAPPENS HERE
          let screen_pos = self.layout.world_to_screen(world_pos);

          // Delegate to regular command buffer with screen coords
          self.buffer.ellipse(screen_pos, radius_x, radius_y)
      }

      /// Draw line from WORLD positions
      pub fn line(
          &mut self,
          world_from: [f32; 2],
          world_to: [f32; 2],
      ) -> DrawBuilder<'_> {
          let screen_from = self.layout.world_to_screen(world_from);
          let screen_to = self.layout.world_to_screen(world_to);
          self.buffer.line(screen_from, screen_to)
      }

      /// Batch API for world-space ellipses
      pub fn ellipses(&mut self, world_items: &[EllipseGeometry]) -> BatchDrawBuilder<'_> {
          // Transform entire batch to screen space
          let screen_items: Vec<_> = world_items
              .iter()
              .map(|item| EllipseGeometry {
                  center: self.layout.world_to_screen(item.center),
                  radius_x: item.radius_x,
                  radius_y: item.radius_y,
                  rotation: item.rotation,
              })
              .collect();

          self.buffer.ellipses(&screen_items)
      }
  }

  Usage Patterns

  pub fn draw(&mut self, world: &World) {
      let layout = BoardLayout::centered(self.width as f32, self.height as f32);

      // Board grid uses SCREEN SPACE (already in pixels)
      self.commands().lines(&grid_lines)
          .thickness(4.0)
          .depth(0.0);

      // Leaves use WORLD SPACE (sim coordinates)
      {
          let mut world_cmds = self.world_commands(&layout);
          for leaf in world.leaf().unwrap().leaves() {
              world_cmds.ellipse(leaf.position, leaf.radius_x, leaf.radius_y)
                  .rotation(leaf.rotation)
                  .alpha(leaf.growth)
                  .depth(0.5);
          }
      }  // Scope ends, world_cmds dropped

      // Back to screen space if needed
      self.commands().line([0, 0], [100, 100])
          .thickness(2.0)
          .depth(1.0);
  }

  Why This Pattern?

  1. Type safety - WorldCommandScope API signals "this expects world coords"
  2. Less error-prone - Can't forget to transform
  3. Scoped - Clear boundary between coordinate systems
  4. Extensible - Could add CameraCommandScope for view transforms later
  5. Zero runtime cost - Transform happens once per call, same as before

  Alternative considered: Store coordinate space in command metadata and transform during dispatch. Rejected because transform math is cheap and doing
  it early is clearer.

⏺ Command Buffer Design: Batch Processing

  Dispatch Strategy

  Goal: Minimize draw calls by grouping commands with same shader type and depth.

  impl CommandBuffer {
      pub fn submit(&mut self, shader_registry: &mut ShaderRegistry) {
          // 1. Sort by depth (stable sort preserves insertion order for equal depths)
          self.commands.sort_by(|a, b| {
              a.depth.partial_cmp(&b.depth).unwrap_or(std::cmp::Ordering::Equal)
          });

          // 2. Group consecutive commands by shader type
          let mut current_shader: Option<&str> = None;
          let mut pending_batch: Vec<&RenderCommand> = Vec::new();

          for cmd in &self.commands {
              let shader_name = cmd.command.shader_name();

              // If shader type changes, flush pending batch
              if current_shader != Some(shader_name) {
                  self.flush_batch(&pending_batch, shader_registry);
                  pending_batch.clear();
                  current_shader = Some(shader_name);
              }

              pending_batch.push(cmd);
          }

          // 3. Flush final batch
          self.flush_batch(&pending_batch, shader_registry);

          // 4. Clear for next frame
          self.commands.clear();
      }

      fn flush_batch(&self, commands: &[&RenderCommand], shader_registry: &mut ShaderRegistry) {
          if commands.is_empty() {
              return;
          }

          // Dispatch to appropriate shader based on command type
          match &commands[0].command {
              DrawCommand::Lines { .. } | DrawCommand::Line { .. } => {
                  if let Some(line_renderer) = shader_registry
                      .get_mut("line")
                      .and_then(|r| r.as_any_mut().downcast_mut::<LineRenderer>())
                  {
                      // Feed all line commands to LineRenderer
                      for cmd in commands {
                          match &cmd.command {
                              DrawCommand::Lines { segments, thickness, color } => {
                                  for (from, to) in segments {
                                      line_renderer.draw_line(*from, *to, *thickness);
                                  }
                              }
                              DrawCommand::Line { from, to, thickness, color } => {
                                  line_renderer.draw_line(*from, *to, *thickness);
                              }
                              _ => {}
                          }
                      }
                  }
              }

              DrawCommand::Ellipses { .. } | DrawCommand::Ellipse { .. } => {
                  if let Some(ellipse_renderer) = shader_registry
                      .get_mut("ellipse")
                      .and_then(|r| r.as_any_mut().downcast_mut::<EllipseRenderer>())
                  {
                      // Feed all ellipse commands to EllipseRenderer
                      for cmd in commands {
                          match &cmd.command {
                              DrawCommand::Ellipses { items, color, alpha } => {
                                  for item in items {
                                      ellipse_renderer.draw_ellipse(Ellipse {
                                          center: item.center,
                                          radius_x: item.radius_x,
                                          radius_y: item.radius_y,
                                          rotation: item.rotation,
                                          color: *color,
                                          alpha: *alpha,
                                      });
                                  }
                              }
                              DrawCommand::Ellipse { center, radius_x, radius_y, rotation, color, alpha } => {
                                  ellipse_renderer.draw_ellipse(Ellipse {
                                      center: *center,
                                      radius_x: *radius_x,
                                      radius_y: *radius_y,
                                      rotation: *rotation,
                                      color: *color,
                                      alpha: *alpha,
                                  });
                              }
                              _ => {}
                          }
                      }
                  }
              }
          }
      }
  }

  impl DrawCommand {
      fn shader_name(&self) -> &str {
          match self {
              DrawCommand::Line { .. } | DrawCommand::Lines { .. } => "line",
              DrawCommand::Ellipse { .. } | DrawCommand::Ellipses { .. } => "ellipse",
          }
      }
  }

  Batching Example

  Input commands:
  renderer.lines(&[line1, line2, line3]).depth(0.0);  // Command 1: 3 lines
  renderer.ellipse(e1).depth(0.1);                     // Command 2: 1 ellipse
  renderer.ellipse(e2).depth(0.1);                     // Command 3: 1 ellipse
  renderer.line(line4).depth(0.2);                     // Command 4: 1 line
  renderer.lines(&[line5, line6]).depth(0.2);         // Command 5: 2 lines

  After sorting by depth:
  [0.0] Command 1: Lines (3 items) → "line" shader
  [0.1] Command 2: Ellipse (1 item) → "ellipse" shader
  [0.1] Command 3: Ellipse (1 item) → "ellipse" shader
  [0.2] Command 4: Line (1 item) → "line" shader
  [0.2] Command 5: Lines (2 items) → "line" shader

  Batching groups:
  Batch 1 (depth 0.0, shader "line"):
    - Feed 3 lines to LineRenderer
    - LineRenderer.render() called ONCE with 3 lines

  Batch 2 (depth 0.1, shader "ellipse"):
    - Feed 2 ellipses to EllipseRenderer
    - EllipseRenderer.render() called ONCE with 2 ellipses

  Batch 3 (depth 0.2, shader "line"):
    - Feed 3 lines to LineRenderer (1 + 2)
    - LineRenderer.render() called ONCE with 3 lines

  Result: 3 shader bindings + 3 draw calls (vs 5 if we didn't batch)

  Optimization: Pre-group Batch Commands

  For even better performance with large batch commands:

  pub enum DrawCommand {
      // Batch variant already groups geometry
      Lines {
          segments: Vec<([f32; 2], [f32; 2])>,
          thickness: f32,
          color: [f32; 3],
      },
  }

  // When flushing, a Lines command with 100 segments becomes
  // 100 line_renderer.draw_line() calls, but only 1 vertex buffer upload
  // because LineRenderer batches internally (src/app/line_renderer.rs:178-199)

  Key insight: The command buffer sorts/groups at a coarse level (by shader type), then individual renderers batch geometry into GPU buffers. Two-level
  batching!

⏺ Revised Integration Plan

  Phase 1: Core Infrastructure (~30 min)

  Create src/app/renderer/command.rs:

  // 1.1 - Geometry types for batches
  pub struct EllipseGeometry {
      pub center: [f32; 2],
      pub radius_x: f32,
      pub radius_y: f32,
      pub rotation: f32,
  }

  // 1.2 - Draw commands (batch + individual variants)
  pub enum DrawCommand {
      // Batch commands - multiple items with shared properties
      Lines {
          segments: Vec<([f32; 2], [f32; 2])>,
          thickness: f32,
          color: [f32; 3],
      },
      Ellipses {
          items: Vec<EllipseGeometry>,
          color: [f32; 3],
          alpha: f32,
      },

      // Individual commands - single item with unique properties
      Line {
          from: [f32; 2],
          to: [f32; 2],
          thickness: f32,
          color: [f32; 3],
      },
      Ellipse {
          center: [f32; 2],
          radius_x: f32,
          radius_y: f32,
          rotation: f32,
          color: [f32; 3],
          alpha: f32,
      },
  }

  impl DrawCommand {
      pub fn shader_name(&self) -> &str {
          match self {
              DrawCommand::Line { .. } | DrawCommand::Lines { .. } => "line",
              DrawCommand::Ellipse { .. } | DrawCommand::Ellipses { .. } => "ellipse",
          }
      }
  }

  // 1.3 - Render command with depth/state
  pub struct RenderCommand {
      pub command: DrawCommand,
      pub depth: f32,
      pub clip_rect: Option<Rect>,  // For future use
  }

  // 1.4 - Command buffer with sorting/dispatch
  pub struct CommandBuffer {
      commands: Vec<RenderCommand>,
  }

  impl CommandBuffer {
      pub fn new() -> Self {
          Self { commands: Vec::new() }
      }

      // Batch API
      pub fn lines(&mut self, segments: &[([f32; 2], [f32; 2])]) -> BatchDrawBuilder<'_> {
          BatchDrawBuilder::new(self, DrawCommand::Lines {
              segments: segments.to_vec(),
              thickness: 1.0,
              color: [1.0, 1.0, 1.0],
          })
      }

      pub fn ellipses(&mut self, items: &[EllipseGeometry]) -> BatchDrawBuilder<'_> {
          BatchDrawBuilder::new(self, DrawCommand::Ellipses {
              items: items.to_vec(),
              color: [1.0, 1.0, 1.0],
              alpha: 1.0,
          })
      }

      // Individual API
      pub fn line(&mut self, from: [f32; 2], to: [f32; 2]) -> DrawBuilder<'_> {
          DrawBuilder::new(self, DrawCommand::Line {
              from, to,
              thickness: 1.0,
              color: [1.0, 1.0, 1.0],
          })
      }

      pub fn ellipse(&mut self, center: [f32; 2], radius_x: f32, radius_y: f32) -> DrawBuilder<'_> {
          DrawBuilder::new(self, DrawCommand::Ellipse {
              center, radius_x, radius_y,
              rotation: 0.0,
              color: [1.0, 1.0, 1.0],
              alpha: 1.0,
          })
      }

      // Convenience scope
      pub fn at_depth(&mut self, depth: f32, f: impl FnOnce(&mut DepthScope<'_>)) {
          let mut scope = DepthScope { buffer: self, depth };
          f(&mut scope);
      }

      // Internal: submit commands to shader registry
      pub(crate) fn submit(&mut self, shader_registry: &mut ShaderRegistry) {
          // Sort by depth (stable sort)
          self.commands.sort_by(|a, b| {
              a.depth.partial_cmp(&b.depth).unwrap_or(std::cmp::Ordering::Equal)
          });

          // Dispatch in batches (see design above)
          // ... (implementation from previous section)

          self.commands.clear();
      }
  }

  // 1.5 - Builder types (BatchDrawBuilder, DrawBuilder, DepthScope)
  // ... (implementations from design section)

  Update src/app/renderer/mod.rs:
  pub mod command;  // Add module declaration

  Phase 2: Integrate CommandBuffer (~20 min)

  Add to SimRenderer (src/app/renderer/sim.rs:24-29):
  use super::command::CommandBuffer;

  pub struct SimRenderer {
      viewport: Viewport,
      shader_registry: ShaderRegistry,
      command_buffer: CommandBuffer,  // NEW
      width: u32,
      height: u32,
  }

  impl SimRenderer {
      pub fn new(/* ... */) -> Self {
          // ...
          Self {
              viewport,
              shader_registry,
              command_buffer: CommandBuffer::new(),  // NEW
              width,
              height,
          }
      }

      /// Returns the command buffer for drawing
      pub fn commands(&mut self) -> &mut CommandBuffer {
          &mut self.command_buffer
      }

      // Keep init_vines(), texture_id(), resize() unchanged
  }

  Update SimRenderer::draw() (src/app/renderer/sim.rs:138-299):
  pub fn draw(&mut self, encoder: &mut wgpu::CommandEncoder, device: &wgpu::Device, queue: &wgpu::Queue, world: &World) {
      let layout = geometry::BoardLayout::centered(self.width as f32, self.height as f32);

      // ... existing tictactoe/leaf sim access code ...

      // NEW: Dispatch command buffer BEFORE shader begin_frame
      self.command_buffer.submit(&mut self.shader_registry);

      // EXISTING: Prepare shaders
      self.shader_registry.begin_frame(device, queue);

      // EXISTING: Render pass (unchanged)
      {
          let mut rpass = encoder.begin_render_pass(/* ... */);
          self.shader_registry.render_all(&mut rpass);
      }

      // EXISTING: End frame
      self.shader_registry.end_frame();
  }

  Note: Old shader accumulation still works! LineRenderer/EllipseRenderer's draw_line()/draw_ellipse() methods are called by command buffer dispatch, so
   existing behavior is preserved.

  Phase 3: Add WorldCommandScope (~15 min)

  Add to src/app/renderer/command.rs:
  use crate::app::geometry::BoardLayout;

  pub struct WorldCommandScope<'a> {
      buffer: &'a mut CommandBuffer,
      layout: &'a BoardLayout,
  }

  impl<'a> WorldCommandScope<'a> {
      pub fn new(buffer: &'a mut CommandBuffer, layout: &'a BoardLayout) -> Self {
          Self { buffer, layout }
      }

      pub fn ellipse(&mut self, world_pos: [f32; 2], radius_x: f32, radius_y: f32) -> DrawBuilder<'_> {
          let screen_pos = self.layout.world_to_screen(world_pos);
          self.buffer.ellipse(screen_pos, radius_x, radius_y)
      }

      pub fn line(&mut self, world_from: [f32; 2], world_to: [f32; 2]) -> DrawBuilder<'_> {
          let screen_from = self.layout.world_to_screen(world_from);
          let screen_to = self.layout.world_to_screen(world_to);
          self.buffer.line(screen_from, screen_to)
      }

      pub fn ellipses(&mut self, world_items: &[EllipseGeometry]) -> BatchDrawBuilder<'_> {
          let screen_items: Vec<_> = world_items
              .iter()
              .map(|item| EllipseGeometry {
                  center: self.layout.world_to_screen(item.center),
                  ..*item
              })
              .collect();
          self.buffer.ellipses(&screen_items)
      }
  }

  Add helper to SimRenderer (src/app/renderer/sim.rs):
  impl SimRenderer {
      pub fn world_commands(&mut self, layout: &BoardLayout) -> WorldCommandScope<'_> {
          WorldCommandScope::new(&mut self.command_buffer, layout)
      }
  }

  Phase 4: Migrate Rendering Code (~30 min)

  Board grid (src/app/renderer/sim.rs:158-218):
  // BEFORE (delete this block)
  if let Some(line_renderer) = self.shader_registry.get_mut("line")
      .and_then(|r| r.as_any_mut().downcast_mut::<LineRenderer>())
  {
      for line in geometry::generate_board_grid(&layout) {
          line_renderer.draw_line(line.from, line.to, line.thickness);
      }
      // ... X/O pieces, scores ...
  }

  // AFTER
  let board_lines: Vec<_> = geometry::generate_board_grid(&layout)
      .into_iter()
      .map(|line| (line.from, line.to))
      .collect();

  self.commands().lines(&board_lines)
      .thickness(layout.line_thickness)
      .depth(0.0);  // Background layer

  // X pieces
  for (row, row_tiles) in board.iter().enumerate() {
      for (col, &tile) in row_tiles.iter().enumerate() {
          if let Tile::X = tile {
              let x_lines: Vec<_> = geometry::generate_x(&layout, row, col)
                  .into_iter()
                  .map(|line| (line.from, line.to))
                  .collect();
              self.commands().lines(&x_lines)
                  .thickness(layout.line_thickness)
                  .depth(0.0);
          }
      }
  }

  // (Similar for O pieces and scores)

  Leaves (src/app/renderer/sim.rs:220-265):
  // BEFORE (delete this block)
  if let Some(ellipse_renderer) = self.shader_registry.get_mut("ellipse")
      .and_then(|r| r.as_any_mut().downcast_mut::<EllipseRenderer>())
      && let Some(leaf_sim) = leaf_sim
  {
      for leaf in leaf_sim.leaves() {
          let screen_position = layout.world_to_screen(leaf.position);
          ellipse_renderer.draw_ellipse(/* ... */);
      }
  }

  // AFTER
  if let Some(leaf_sim) = leaf_sim {
      let mut world_cmds = self.world_commands(&layout);
      for leaf in leaf_sim.leaves() {
          world_cmds.ellipse(leaf.position, leaf.size * leaf.growth, leaf.size * leaf.growth * leaf.aspect)
              .rotation(leaf.rotation)
              .color(LEAF_COLORS[leaf.color_variant as usize % 4])
              .alpha(leaf.growth)
              .depth(0.5);  // Foreground layer
      }
  }

  Phase 5: Verification (~10 min)

  1. Run cargo build - should compile
  2. Run the game - rendering should look identical
  3. Verify depth ordering works by temporarily changing depths
  4. Test that old LineRenderer/EllipseRenderer code paths still work (they're called by command dispatch)

  Phase 6: Future Enhancements (Optional)

  - Add depth buffer support (Phase 4 from original plan)
  - Add clip rect functionality
  - Add blend mode variants
  - Add sprite/text rendering commands

  ---

⏺ Summary of Refined Design

  Key Changes from Initial Proposal

  1. Batch-first API - Primary API is lines(&[...]) / ellipses(&[...]) with shared properties
  2. Individual fallback - Single line() / ellipse() for varying properties
  3. WorldCommandScope - Scoped coordinate transform API for world→screen conversions
  4. Two-level batching - Command buffer groups by shader, then shaders batch geometry to GPU
  5. Keep old behavior - LineRenderer/EllipseRenderer still work the same way internally

  API Comparison

  // OLD: Manual shader access, implicit ordering
  if let Some(line_renderer) = shader_registry.get_mut("line") {
      for line in lines {
          line_renderer.draw_line(line.from, line.to, line.thickness);
      }
  }

  // NEW: Declarative batches with explicit depth
  renderer.lines(&lines_slice)
      .thickness(4.0)
      .depth(0.0);

  // NEW: World-space convenience
  let mut world_cmds = renderer.world_commands(&layout);
  world_cmds.ellipse(leaf.position, radius_x, radius_y)
      .alpha(leaf.growth)
      .depth(0.5);

  Benefits

  - ✅ Solves interleaving (lines → leaves → lines → leaves)
  - ✅ Batch-optimized (matches GPU/shader model)
  - ✅ Declarative depth ordering
  - ✅ Type-safe coordinate transforms
  - ✅ Backwards compatible (old code paths still work)
  - ✅ Extensible (easy to add sprites, text, meshes)

  ---
  Ready to implement?

  The plan is broken into 6 phases (~105 min total). Would you like me to:

  A) Start implementing - Begin with Phase 1 (core infrastructure)?

  B) Clarify something - Any part of the design need more explanation?

  C) Adjust the design - Want to change API ergonomics or approach?

  Your call!

> B) Clarify something - Any part of the design need more explanation?

  C) Adjust the design - Want to change API ergonomics or approach?

This is looking great. I'm seeing a ton of good ideas and considerations. Want to make sure we consider: Can't we make use of scissoring, or
letterboxing, or cutting the viewport (I don't know the right term)? That feels like it could be very useful in solving the world_space to screen space
problem since normalized device coord would align with where the thing is being rendered at. Where does this fit into the api how can we use context
accumulation during the command construction to make this very powerful for rendering scenes?

⏺ Excellent insight! You're absolutely right - viewport transforms and scissor rects are the GPU-native way to solve coordinate transforms. This is much
   more powerful than manual world_to_screen() transforms.

  Let me redesign with viewport-based rendering and show how context accumulation makes this powerful.

⏺ Viewport-Based Coordinate Transform System

  The GPU-Native Approach

  You're right - instead of CPU-side world_to_screen() transforms, we should use:

  1. Scissor rects - Clip rendering to regions
  2. Viewport state - Define pixel regions
  3. Shader transforms - GPU handles coordinate mapping via uniforms

  This enables:
  - Multiple coordinate spaces
  - Nested viewports (minimap, split-screen, UI panels)
  - Automatic clipping
  - Render-to-texture with different coords

  Core Concept: Viewport Context

  // Define a viewport with its coordinate space
  renderer.viewport(ViewportConfig {
      // Pixel region (where to render)
      pixel_rect: Rect { x: 200, y: 100, width: 400, height: 400 },

      // Logical coordinate bounds (what coords map to this region)
      coord_bounds: Bounds {
          min: [-1.5, -1.5],
          max: [1.5, 1.5],
      },

      // Optional depth range
      depth_range: (0.0, 1.0),
  }, |vp| {
      // Inside this scope, positions are in coord_bounds space
      // GPU automatically transforms to pixel_rect

      vp.ellipse([0.0, 0.0], 0.5, 0.5)  // Center of viewport, in logical coords
          .color([0.2, 0.6, 0.3])
          .depth(0.5);

      vp.line([-1.5, 0.0], [1.5, 0.0])  // Horizontal line across viewport
          .thickness(2.0)
          .depth(0.0);
  });

  // Outside scope: back to default viewport (full screen, pixel coords)
  renderer.line([0, 0], [100, 100])
      .thickness(4.0)
      .depth(0.0);

  How It Works

  1. Viewport state is passed to shaders via uniforms:

  #[repr(C)]
  struct ViewportUniforms {
      screen_size: [f32; 2],        // Current viewport pixel size
      coord_min: [f32; 2],          // Logical coord min
      coord_max: [f32; 2],          // Logical coord max
  }

  2. Vertex shader transforms logical coords → NDC:

  struct ViewportUniforms {
      screen_size: vec2<f32>,
      coord_min: vec2<f32>,
      coord_max: vec2<f32>,
  }

  @group(0) @binding(0)
  var<uniform> viewport: ViewportUniforms;

  @vertex
  fn vs_main(in: VertexInput) -> VertexOutput {
      // Input position is in logical coordinate space
      let logical_x = in.position.x;
      let logical_y = in.position.y;

      // Map to NDC [-1, 1]
      let coord_range = viewport.coord_max - viewport.coord_min;
      let ndc_x = ((logical_x - viewport.coord_min.x) / coord_range.x) * 2.0 - 1.0;
      let ndc_y = ((logical_y - viewport.coord_min.y) / coord_range.y) * 2.0 - 1.0;

      out.clip_position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
      return out;
  }

  3. RenderPass gets scissor + viewport set:

  // When entering viewport scope
  rpass.set_scissor_rect(
      config.pixel_rect.x,
      config.pixel_rect.y,
      config.pixel_rect.width,
      config.pixel_rect.height,
  );

  rpass.set_viewport(
      config.pixel_rect.x as f32,
      config.pixel_rect.y as f32,
      config.pixel_rect.width as f32,
      config.pixel_rect.height as f32,
      config.depth_range.0,
      config.depth_range.1,
  );

  Context Accumulation: Nested Viewports

  The power move: Stack viewport contexts for composition!

  // Full screen background
  renderer.clear([0.1, 0.1, 0.1]);

  // Main game viewport (letterboxed 16:9)
  renderer.viewport(letterbox_rect, world_bounds, |game| {
      // Board grid in world space
      game.lines(&grid_lines).thickness(4.0).depth(0.0);

      // Leaves in world space
      for leaf in leaves {
          game.ellipse(leaf.position, leaf.radius_x, leaf.radius_y)
              .rotation(leaf.rotation)
              .alpha(leaf.growth)
              .depth(0.5);
      }

      // NESTED: Minimap in corner (viewport inside viewport!)
      game.viewport(ViewportConfig {
          pixel_rect: Rect { x: game.width() - 150, y: 10, width: 140, height: 140 },
          coord_bounds: world_bounds,  // Same coords as main view
      }, |minimap| {
          // Render same scene at smaller scale
          minimap.lines(&grid_lines).thickness(1.0).depth(0.0);
          for leaf in leaves {
              minimap.ellipse(leaf.position, leaf.radius_x * 0.5, leaf.radius_y * 0.5)
                  .depth(0.5);
          }
      });
  });

  // UI overlay (pixel coords, no viewport transform)
  renderer.text("Score: 10", [10, 10])
      .font_size(24)
      .depth(1.0);

  What happens:
  1. Main viewport sets scissor/viewport + world coord transform
  2. Nested minimap viewport:
    - Pushes new scissor/viewport state
    - Inherits parent's coord transform (or overrides)
    - Renders
    - Pops state back to parent
  3. UI renders in default viewport (full screen, pixel coords)

  API Design: Context Stack

  pub struct RenderContext {
      // Stack of viewport states
      viewport_stack: Vec<ViewportState>,

      // Current active state (top of stack)
      current: ViewportState,

      // Command buffer
      commands: Vec<RenderCommand>,
  }

  struct ViewportState {
      pixel_rect: Rect,
      coord_bounds: Bounds,
      depth_range: (f32, f32),
      scissor_enabled: bool,
  }

  impl RenderContext {
      /// Push a new viewport context (nested rendering)
      pub fn viewport(
          &mut self,
          config: ViewportConfig,
          f: impl FnOnce(&mut ViewportScope<'_>),
      ) {
          // Push current state onto stack
          self.viewport_stack.push(self.current.clone());

          // Update current state
          self.current = ViewportState {
              pixel_rect: config.pixel_rect,
              coord_bounds: config.coord_bounds,
              depth_range: config.depth_range,
              scissor_enabled: true,
          };

          // Execute drawing commands in new context
          let mut scope = ViewportScope { ctx: self };
          f(&mut scope);

          // Pop state
          self.current = self.viewport_stack.pop().unwrap();
      }

      /// Convert logical coords to pixel coords (for debugging/UI)
      pub fn logical_to_pixels(&self, logical_pos: [f32; 2]) -> [f32; 2] {
          let bounds = &self.current.coord_bounds;
          let rect = &self.current.pixel_rect;

          let norm_x = (logical_pos[0] - bounds.min[0]) / (bounds.max[0] - bounds.min[0]);
          let norm_y = (logical_pos[1] - bounds.min[1]) / (bounds.max[1] - bounds.min[1]);

          [
              rect.x as f32 + norm_x * rect.width as f32,
              rect.y as f32 + norm_y * rect.height as f32,
          ]
      }
  }

  /// Scoped viewport access (ensures context is maintained)
  pub struct ViewportScope<'a> {
      ctx: &'a mut RenderContext,
  }

  impl ViewportScope<'_> {
      /// Draw in current viewport's coordinate space
      pub fn ellipse(&mut self, pos: [f32; 2], rx: f32, ry: f32) -> DrawBuilder<'_> {
          // Position is in current viewport's logical coords
          // Will be transformed by shader uniforms
          self.ctx.commands.push(RenderCommand {
              command: DrawCommand::Ellipse {
                  center: pos,  // Logical coords!
                  radius_x: rx,
                  radius_y: ry,
                  rotation: 0.0,
                  color: [1.0, 1.0, 1.0],
                  alpha: 1.0,
              },
              depth: 0.0,
              viewport_id: self.ctx.viewport_stack.len(),  // Which viewport?
          });

          // Return builder for chaining
          // ...
      }

      /// Nested viewport
      pub fn viewport(
          &mut self,
          config: ViewportConfig,
          f: impl FnOnce(&mut ViewportScope<'_>),
      ) {
          self.ctx.viewport(config, f);
      }

      /// Get current viewport dimensions
      pub fn width(&self) -> u32 {
          self.ctx.current.pixel_rect.width
      }

      pub fn height(&self) -> u32 {
          self.ctx.current.pixel_rect.height
      }
  }

  Command Dispatch with Viewport State

  struct RenderCommand {
      command: DrawCommand,
      depth: f32,
      viewport_id: usize,  // Which viewport context?
  }

  impl RenderContext {
      fn submit(&mut self, shader_registry: &mut ShaderRegistry, rpass: &mut RenderPass) {
          // Sort by (viewport_id, depth) to minimize state changes
          self.commands.sort_by_key(|cmd| (cmd.viewport_id, OrderedFloat(cmd.depth)));

          let mut current_viewport = usize::MAX;

          for cmd in &self.commands {
              // Switch viewport state if needed
              if cmd.viewport_id != current_viewport {
                  let state = self.viewport_stack.get(cmd.viewport_id)
                      .unwrap_or(&self.default_viewport);

                  // Set scissor rect
                  rpass.set_scissor_rect(
                      state.pixel_rect.x,
                      state.pixel_rect.y,
                      state.pixel_rect.width,
                      state.pixel_rect.height,
                  );

                  // Set viewport transform
                  rpass.set_viewport(
                      state.pixel_rect.x as f32,
                      state.pixel_rect.y as f32,
                      state.pixel_rect.width as f32,
                      state.pixel_rect.height as f32,
                      state.depth_range.0,
                      state.depth_range.1,
                  );

                  // Update shader uniforms with coord transform
                  shader_registry.update_viewport_uniforms(state);

                  current_viewport = cmd.viewport_id;
              }

              // Render command
              // ...
          }
      }
  }

⏺ Context Accumulation: Full Rendering State Stack

  Beyond viewports, we can stack any rendering state:

  Stackable Context Types

  pub enum RenderState {
      Viewport(ViewportConfig),
      BlendMode(BlendMode),
      DepthRange(f32, f32),      // Min/max depth for this context
      Transform(Mat3x3),          // 2D transform matrix
      ColorTint([f32; 4]),        // Multiply all colors by this
      AlphaMultiplier(f32),       // Multiply all alphas
  }

  impl RenderContext {
      /// Generic state push/pop
      pub fn with_state<T>(
          &mut self,
          state: RenderState,
          f: impl FnOnce(&mut ViewportScope<'_>) -> T,
      ) -> T {
          self.push_state(state);
          let result = {
              let mut scope = ViewportScope { ctx: self };
              f(&mut scope)
          };
          self.pop_state();
          result
      }

      /// Convenience methods
      pub fn viewport(&mut self, config: ViewportConfig, f: impl FnOnce(&mut ViewportScope<'_>)) {
          self.with_state(RenderState::Viewport(config), f);
      }

      pub fn blend_mode(&mut self, mode: BlendMode, f: impl FnOnce(&mut ViewportScope<'_>)) {
          self.with_state(RenderState::BlendMode(mode), f);
      }

      pub fn transform(&mut self, matrix: Mat3x3, f: impl FnOnce(&mut ViewportScope<'_>)) {
          self.with_state(RenderState::Transform(matrix), f);
      }

      pub fn tinted(&mut self, color: [f32; 4], f: impl FnOnce(&mut ViewportScope<'_>)) {
          self.with_state(RenderState::ColorTint(color), f);
      }
  }

  Composition Examples

  1. Fading panel with custom viewport:
  renderer.viewport(panel_rect, panel_bounds, |panel| {
      panel.tinted([1.0, 1.0, 1.0, 0.5], |faded| {
          // Everything in here is 50% transparent
          faded.rect([0, 0], [100, 100], [0.2, 0.2, 0.2]);
          faded.text("Settings", [10, 10]);
      });
  });

  2. Rotated minimap:
  renderer.viewport(minimap_rect, world_bounds, |minimap| {
      minimap.transform(Mat3x3::rotation(player_angle), |rotated| {
          // World rendered rotated around player
          rotated.lines(&world_grid).depth(0.0);
          rotated.ellipse(player.pos, 5.0, 5.0).color([1.0, 0.0, 0.0]).depth(1.0);
      });
  });

  3. Damage flash effect:
  if damage_taken {
      renderer.tinted([1.0, 0.3, 0.3, 0.8], |flashed| {
          // Entire scene rendered with red tint
          flashed.viewport(game_rect, world_bounds, |game| {
              render_scene(game);
          });
      });
  } else {
      renderer.viewport(game_rect, world_bounds, |game| {
          render_scene(game);
      });
  }

  4. Additive blending for glow effects:
  renderer.viewport(game_rect, world_bounds, |game| {
      // Normal rendering
      game.lines(&grid).depth(0.0);

      // Glow layer with additive blending
      game.blend_mode(BlendMode::Additive, |glow| {
          for particle in particles {
              glow.ellipse(particle.pos, particle.size, particle.size)
                  .color([1.0, 0.8, 0.2])
                  .alpha(particle.intensity)
                  .depth(0.5);
          }
      });
  });

  State Inheritance and Composition

  struct ContextState {
      viewport: ViewportState,
      blend_mode: BlendMode,
      depth_range: (f32, f32),
      transform: Mat3x3,
      color_tint: [f32; 4],
      alpha_multiplier: f32,
  }

  impl RenderContext {
      fn push_state(&mut self, state: RenderState) {
          // Clone current state
          let mut new_state = self.current.clone();

          // Apply delta
          match state {
              RenderState::Viewport(config) => {
                  new_state.viewport = ViewportState::from(config);
              }
              RenderState::BlendMode(mode) => {
                  new_state.blend_mode = mode;
              }
              RenderState::Transform(matrix) => {
                  // Multiply transforms (compose)
                  new_state.transform = new_state.transform * matrix;
              }
              RenderState::ColorTint(tint) => {
                  // Multiply tints (compose)
                  new_state.color_tint = [
                      new_state.color_tint[0] * tint[0],
                      new_state.color_tint[1] * tint[1],
                      new_state.color_tint[2] * tint[2],
                      new_state.color_tint[3] * tint[3],
                  ];
              }
              RenderState::AlphaMultiplier(alpha) => {
                  new_state.alpha_multiplier *= alpha;
              }
              RenderState::DepthRange(min, max) => {
                  // Nested depth ranges compress into parent range
                  let parent_range = new_state.depth_range.1 - new_state.depth_range.0;
                  new_state.depth_range = (
                      new_state.depth_range.0 + min * parent_range,
                      new_state.depth_range.0 + max * parent_range,
                  );
              }
          }

          self.state_stack.push(self.current.clone());
          self.current = new_state;
      }

      fn pop_state(&mut self) {
          self.current = self.state_stack.pop().expect("State stack underflow");
      }
  }

⏺ Integration: Viewport Context + Batch Commands

  Unified API

  // Commands capture current context state
  renderer.viewport(game_rect, world_bounds, |game| {
      // Batch API - coordinates in world space
      let grid_lines: Vec<([f32; 2], [f32; 2])> = generate_grid();
      game.lines(&grid_lines)
          .thickness(4.0)
          .depth(0.0);

      // Individual API - when properties vary
      for leaf in leaves {
          game.ellipse(leaf.position, leaf.radius_x, leaf.radius_y)
              .rotation(leaf.rotation)
              .color(LEAF_COLORS[leaf.variant])
              .alpha(leaf.growth)
              .depth(0.5);
      }

      // Nested context
      game.tinted([1.0, 0.5, 0.5, 1.0], |tinted| {
          tinted.line([-1.0, 0.0], [1.0, 0.0])
              .thickness(2.0)
              .depth(0.6);
      });
  });

  Command Structure (Revised)

  pub struct RenderCommand {
      command: DrawCommand,          // Geometry data
      depth: f32,                    // Depth within context
      context_id: usize,             // Which context state?
  }

  pub struct DrawCommand {
      // Same as before - batch + individual variants
      Lines { segments: Vec<([f32; 2], [f32; 2])>, thickness: f32, color: [f32; 3] },
      Ellipses { items: Vec<EllipseGeometry>, color: [f32; 3], alpha: f32 },
      Line { from: [f32; 2], to: [f32; 2], thickness: f32, color: [f32; 3] },
      Ellipse { /* ... */ },
  }

  pub struct RenderContext {
      // Context state stack
      state_stack: Vec<ContextState>,
      current: ContextState,

      // Snapshot of all context states used this frame
      context_snapshots: Vec<ContextState>,

      // Commands reference context snapshots by ID
      commands: Vec<RenderCommand>,
  }

  impl RenderContext {
      fn capture_context_snapshot(&mut self) -> usize {
          // Check if this exact state already exists (dedup)
          if let Some(id) = self.context_snapshots
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
  }

  Builder Integration

  impl ViewportScope<'_> {
      pub fn lines(&mut self, segments: &[([f32; 2], [f32; 2])]) -> BatchDrawBuilder<'_> {
          // Capture current context state
          let context_id = self.ctx.capture_context_snapshot();

          BatchDrawBuilder::new(
              self.ctx,
              DrawCommand::Lines {
                  segments: segments.to_vec(),
                  thickness: 1.0,
                  color: [1.0, 1.0, 1.0],
              },
              context_id,  // Baked into builder
          )
      }

      pub fn ellipse(&mut self, pos: [f32; 2], rx: f32, ry: f32) -> DrawBuilder<'_> {
          let context_id = self.ctx.capture_context_snapshot();

          DrawBuilder::new(
              self.ctx,
              DrawCommand::Ellipse {
                  center: pos,
                  radius_x: rx,
                  radius_y: ry,
                  rotation: 0.0,
                  color: [1.0, 1.0, 1.0],
                  alpha: 1.0,
              },
              context_id,
          )
      }
  }

  // Builder auto-submits on drop
  impl Drop for DrawBuilder<'_> {
      fn drop(&mut self) {
          self.ctx.commands.push(RenderCommand {
              command: self.command.clone(),
              depth: self.depth,
              context_id: self.context_id,  // References snapshot
          });
      }
  }

  Dispatch: Sort by Context + Depth

  impl RenderContext {
      pub fn submit(&mut self, shader_registry: &mut ShaderRegistry, rpass: &mut RenderPass) {
          // Sort by (context_id, depth, shader_type)
          // This minimizes GPU state changes
          self.commands.sort_by(|a, b| {
              (a.context_id, OrderedFloat(a.depth), a.command.shader_name())
                  .cmp(&(b.context_id, OrderedFloat(b.depth), b.command.shader_name()))
          });

          let mut current_context = usize::MAX;
          let mut current_shader = "";

          for cmd in &self.commands {
              // Switch context if needed
              if cmd.context_id != current_context {
                  let state = &self.context_snapshots[cmd.context_id];
                  self.apply_context_state(rpass, shader_registry, state);
                  current_context = cmd.context_id;
                  current_shader = "";  // Force shader rebind after context change
              }

              // Switch shader if needed
              let shader_name = cmd.command.shader_name();
              if shader_name != current_shader {
                  current_shader = shader_name;
              }

              // Dispatch to shader
              self.dispatch_command(shader_registry, &cmd.command);
          }

          // Clear for next frame
          self.commands.clear();
          self.context_snapshots.clear();
      }

      fn apply_context_state(
          &self,
          rpass: &mut RenderPass,
          shader_registry: &mut ShaderRegistry,
          state: &ContextState,
      ) {
          // Set scissor rect
          rpass.set_scissor_rect(
              state.viewport.pixel_rect.x,
              state.viewport.pixel_rect.y,
              state.viewport.pixel_rect.width,
              state.viewport.pixel_rect.height,
          );

          // Set viewport transform
          rpass.set_viewport(
              state.viewport.pixel_rect.x as f32,
              state.viewport.pixel_rect.y as f32,
              state.viewport.pixel_rect.width as f32,
              state.viewport.pixel_rect.height as f32,
              state.depth_range.0,
              state.depth_range.1,
          );

          // Update shader uniforms with context state
          shader_registry.update_context_uniforms(ContextUniforms {
              screen_size: [
                  state.viewport.pixel_rect.width as f32,
                  state.viewport.pixel_rect.height as f32,
              ],
              coord_min: state.viewport.coord_bounds.min,
              coord_max: state.viewport.coord_bounds.max,
              transform: state.transform,
              color_tint: state.color_tint,
              alpha_multiplier: state.alpha_multiplier,
          });
      }
  }

  Example: Your Current Use Case

  pub fn draw(&mut self, world: &World) {
      // Define board viewport
      let board_rect = Rect {
          x: (self.width as i32 - 400) / 2,
          y: (self.height as i32 - 400) / 2,
          width: 400,
          height: 400,
      };

      let world_bounds = Bounds {
          min: [-1.5, -1.5],
          max: [1.5, 1.5],
      };

      // Render in world coordinates
      self.commands().viewport(
          ViewportConfig { pixel_rect: board_rect, coord_bounds: world_bounds },
          |board| {
              // Grid lines in world space
              let grid_lines = vec![
                  ([-1.5, -0.5], [1.5, -0.5]),  // Horizontal 1
                  ([-1.5, 0.5], [1.5, 0.5]),    // Horizontal 2
                  ([-0.5, -1.5], [-0.5, 1.5]),  // Vertical 1
                  ([0.5, -1.5], [0.5, 1.5]),    // Vertical 2
              ];

              board.lines(&grid_lines)
                  .thickness(4.0)
                  .color([1.0, 1.0, 1.0])
                  .depth(0.0);

              // X and O pieces (world coords)
              for (row, row_tiles) in world.tictactoe().unwrap().board().iter().enumerate() {
                  for (col, &tile) in row_tiles.iter().enumerate() {
                      let world_pos = [
                          -1.0 + col as f32,  // World coords: -1, 0, 1
                          -1.0 + row as f32,
                      ];

                      match tile {
                          Tile::X => {
                              let x_lines = vec![
                                  ([world_pos[0] - 0.3, world_pos[1] - 0.3],
                                   [world_pos[0] + 0.3, world_pos[1] + 0.3]),
                                  ([world_pos[0] + 0.3, world_pos[1] - 0.3],
                                   [world_pos[0] - 0.3, world_pos[1] + 0.3]),
                              ];
                              board.lines(&x_lines).thickness(4.0).depth(0.1);
                          }
                          Tile::O => {
                              board.ellipse(world_pos, 0.3, 0.3)
                                  .depth(0.1);
                          }
                          Tile::Empty => {}
                      }
                  }
              }

              // Leaves (world coords, varying properties)
              if let Some(leaf_sim) = world.leaf() {
                  for leaf in leaf_sim.leaves() {
                      board.ellipse(leaf.position, leaf.radius_x * leaf.growth, leaf.radius_y * leaf.growth)
                          .rotation(leaf.rotation)
                          .color(LEAF_COLORS[leaf.color_variant as usize % 4])
                          .alpha(leaf.growth)
                          .depth(0.5);
                  }
              }
          },
      );
  }

  Benefits:
  - ✅ No manual world_to_screen() transforms
  - ✅ GPU handles coordinate mapping via shaders
  - ✅ Scissor rect clips geometry automatically
  - ✅ Nested viewports for minimap, UI panels, etc.
  - ✅ Context state composes (tints, transforms, blend modes)
  - ✅ Efficient: Sort by context to minimize state changes

⏺ WorldCommandScope → Viewport Context Migration

  The Old Approach (What We Designed Initially)

  // Manual coordinate transform wrapper
  let mut world_cmds = renderer.world_commands(&layout);
  world_cmds.ellipse(leaf.position, rx, ry)  // Transforms world→screen in CPU
      .depth(0.5);

  The New Approach (Viewport-Based)

  // GPU-accelerated coordinate transform via viewport
  renderer.viewport(board_rect, world_bounds, |board| {
      board.ellipse(leaf.position, rx, ry)  // GPU transforms via shader
          .depth(0.5);
  });

  Why Viewport is Superior

  WorldCommandScope (CPU transform):
  impl WorldCommandScope<'_> {
      pub fn ellipse(&mut self, world_pos: [f32; 2], rx: f32, ry: f32) -> DrawBuilder<'_> {
          // CPU does the math
          let screen_pos = self.layout.world_to_screen(world_pos);

          // Pass screen coords to shader
          self.buffer.ellipse(screen_pos, rx, ry)
      }
  }

  Viewport (GPU transform):
  // Shader does the math
  @vertex
  fn vs_main(in: VertexInput) -> VertexOutput {
      // Input is in world coords
      let world_x = in.position.x;
      let world_y = in.position.y;

      // Transform to NDC (GPU handles this)
      let coord_range = viewport.coord_max - viewport.coord_min;
      let ndc_x = ((world_x - viewport.coord_min.x) / coord_range.x) * 2.0 - 1.0;
      let ndc_y = ((world_y - viewport.coord_min.y) / coord_range.y) * 2.0 - 1.0;

      out.clip_position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
      return out;
  }

  Advantages:
  1. GPU parallelism - Transforms 1000s of vertices simultaneously
  2. Less CPU work - No per-vertex transform on CPU
  3. Scissor clipping - Free clipping from GPU
  4. Composable - Nested viewports, transforms stack naturally
  5. Flexible - Can add rotation, scale, etc. via transform matrix

  When to Use Each

  | Pattern          | Use When                                | Example                               |
  |------------------|-----------------------------------------|---------------------------------------|
  | Viewport context | Rendering scenes in logical coordinates | Game board, minimap, UI panels        |
  | Screen space     | Direct pixel manipulation               | Debug overlays, exact pixel placement |
  | Manual transform | One-off coordinate conversions          | Hit testing, mouse picking            |

  Complete Example: Migration Path

  Before (with WorldCommandScope):
  pub fn draw(&mut self, world: &World) {
      let layout = BoardLayout::centered(self.width as f32, self.height as f32);

      // Screen-space commands
      let grid_lines: Vec<_> = geometry::generate_board_grid(&layout)
          .into_iter()
          .map(|line| (line.from, line.to))  // Already in screen coords
          .collect();

      self.commands().lines(&grid_lines)
          .thickness(4.0)
          .depth(0.0);

      // World-space commands (CPU transform)
      let mut world_cmds = self.world_commands(&layout);
      for leaf in world.leaf().unwrap().leaves() {
          world_cmds.ellipse(leaf.position, leaf.rx, leaf.ry)  // Transforms each call
              .depth(0.5);
      }
  }

  After (with viewport context):
  pub fn draw(&mut self, world: &World) {
      // Calculate viewport region
      let board_size = self.width.min(self.height) as f32 * 0.6;
      let board_rect = Rect {
          x: ((self.width as f32 - board_size) / 2.0) as i32,
          y: ((self.height as f32 - board_size) / 2.0) as i32,
          width: board_size as u32,
          height: board_size as u32,
      };

      // Define world coordinate bounds
      let world_bounds = Bounds {
          min: [-1.5, -1.5],  // 3x3 board spans -1.5 to +1.5
          max: [1.5, 1.5],
      };

      // Everything in world coordinates now
      self.commands().viewport(
          ViewportConfig { pixel_rect: board_rect, coord_bounds: world_bounds },
          |board| {
              // Grid lines in world coords (much simpler!)
              board.lines(&[
                  ([-1.5, -0.5], [1.5, -0.5]),
                  ([-1.5, 0.5], [1.5, 0.5]),
                  ([-0.5, -1.5], [-0.5, 1.5]),
                  ([0.5, -1.5], [0.5, 1.5]),
              ])
              .thickness(4.0)
              .depth(0.0);

              // Leaves in world coords (GPU transforms)
              for leaf in world.leaf().unwrap().leaves() {
                  board.ellipse(leaf.position, leaf.rx, leaf.ry)
                      .rotation(leaf.rotation)
                      .alpha(leaf.growth)
                      .depth(0.5);
              }
          },
      );
  }

  Wins:
  - Simpler geometry generation (no screen coords needed)
  - GPU handles transforms
  - Free clipping to board region
  - Easy to add camera/zoom/pan later

> Don't forget to do one more consideration pass for other use cases. When we want to expand this to:
  // Future features slot in cleanly
  renderer.sprite(texture_id, pos).depth(0.5).flip_x(true);
  renderer.text("Score: 10", pos).depth(1.0).font_size(24);
  renderer.mesh(vertices).depth(0.0).blend_mode(BlendMode::Additive);
what does the api look like as it grows? How does implementation scale? Can we have a trait that various command types can implement to make it easy to
add new rendering command kinds (different shaders, effects, etc). We don't need to go overboard, but where do we hit complexity limits as the current
plan scales past 2-3 kinds of rendering commands

⏺ Scalability: Trait-Based Command System

  Excellent question - let's think about scaling to 10+ command types (sprites, text, meshes, particles, etc.).

  Current Problem: Enum Explosion

  // This doesn't scale well
  pub enum DrawCommand {
      Line { ... },
      Lines { ... },
      Ellipse { ... },
      Ellipses { ... },
      Sprite { ... },
      Sprites { ... },
      Text { ... },
      Mesh { ... },
      ParticleSystem { ... },
      NinePatch { ... },
      SDF { ... },
      // ... 50 variants later ...
  }

  // Dispatch becomes a massive match statement
  fn dispatch(&self, cmd: &DrawCommand, registry: &mut ShaderRegistry) {
      match cmd {
          DrawCommand::Line { ... } => { /* ... */ },
          DrawCommand::Lines { ... } => { /* ... */ },
          // ... 50 arms ...
      }
  }

  Solution: Trait-Based Commands

  /// Trait that all drawable commands implement
  pub trait DrawCommand: Send + Sync {
      /// Which shader renders this command?
      fn shader_name(&self) -> &str;

      /// Dispatch this command to the appropriate shader
      fn dispatch(&self, shader_registry: &mut ShaderRegistry);

      /// Sort key for batching (shader type + material)
      fn batch_key(&self) -> u64;

      /// Optional: Clone for command buffer
      fn clone_box(&self) -> Box<dyn DrawCommand>;
  }

  // Commands are now boxed trait objects
  pub struct RenderCommand {
      command: Box<dyn DrawCommand>,
      depth: f32,
      context_id: usize,
  }

  Implementing Commands

  Line command:
  #[derive(Clone)]
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
          // Hash shader name for batching
          // Lines with same thickness can batch together
          let mut hasher = DefaultHasher::new();
          "line".hash(&mut hasher);
          self.thickness.to_bits().hash(&mut hasher);
          hasher.finish()
      }

      fn clone_box(&self) -> Box<dyn DrawCommand> {
          Box::new(self.clone())
      }
  }

  Sprite command (future):
  #[derive(Clone)]
  pub struct SpriteCommand {
      pub texture_id: TextureId,
      pub position: [f32; 2],
      pub size: [f32; 2],
      pub flip_x: bool,
      pub flip_y: bool,
      pub tint: [f32; 4],
  }

  impl DrawCommand for SpriteCommand {
      fn shader_name(&self) -> &str {
          "sprite"
      }

      fn dispatch(&self, shader_registry: &mut ShaderRegistry) {
          if let Some(sprite_renderer) = shader_registry
              .get_mut("sprite")
              .and_then(|r| r.as_any_mut().downcast_mut::<SpriteRenderer>())
          {
              sprite_renderer.draw_sprite(self);
          }
      }

      fn batch_key(&self) -> u64 {
          // Batch by texture + shader
          let mut hasher = DefaultHasher::new();
          "sprite".hash(&mut hasher);
          self.texture_id.hash(&mut hasher);  // Same texture = batch together
          hasher.finish()
      }

      fn clone_box(&self) -> Box<dyn DrawCommand> {
          Box::new(self.clone())
      }
  }

  Text command (future):
  #[derive(Clone)]
  pub struct TextCommand {
      pub text: String,
      pub position: [f32; 2],
      pub font_size: f32,
      pub font_id: FontId,
      pub color: [f32; 4],
  }

  impl DrawCommand for TextCommand {
      fn shader_name(&self) -> &str {
          "text"
      }

      fn dispatch(&self, shader_registry: &mut ShaderRegistry) {
          if let Some(text_renderer) = shader_registry
              .get_mut("text")
              .and_then(|r| r.as_any_mut().downcast_mut::<TextRenderer>())
          {
              text_renderer.draw_text(&self.text, self.position, self.font_size, self.color);
          }
      }

      fn batch_key(&self) -> u64 {
          let mut hasher = DefaultHasher::new();
          "text".hash(&mut hasher);
          self.font_id.hash(&mut hasher);  // Same font = batch together
          hasher.finish()
      }

      fn clone_box(&self) -> Box<dyn DrawCommand> {
          Box::new(self.clone())
      }
  }

  Builder System Scales

  impl<'a> ViewportScope<'a> {
      /// Line command
      pub fn line(&mut self, from: [f32; 2], to: [f32; 2]) -> LineBuilder<'a> {
          LineBuilder::new(self.ctx, from, to)
      }

      /// Batch lines
      pub fn lines(&mut self, segments: &[([f32; 2], [f32; 2])]) -> LineBuilder<'a> {
          LineBuilder::with_batch(self.ctx, segments.to_vec())
      }

      /// Sprite command (future)
      pub fn sprite(&mut self, texture_id: TextureId, pos: [f32; 2]) -> SpriteBuilder<'a> {
          SpriteBuilder::new(self.ctx, texture_id, pos)
      }

      /// Batch sprites (future)
      pub fn sprites(&mut self, items: &[SpriteData]) -> SpriteBuilder<'a> {
          SpriteBuilder::with_batch(self.ctx, items.to_vec())
      }

      /// Text command (future)
      pub fn text(&mut self, text: impl Into<String>, pos: [f32; 2]) -> TextBuilder<'a> {
          TextBuilder::new(self.ctx, text.into(), pos)
      }
  }

  // Each builder type
  pub struct LineBuilder<'a> {
      ctx: &'a mut RenderContext,
      command: LineCommand,
      context_id: usize,
      depth: f32,
  }

  impl<'a> LineBuilder<'a> {
      pub fn thickness(mut self, thickness: f32) -> Self {
          self.command.thickness = thickness;
          self
      }

      pub fn color(mut self, color: [f32; 3]) -> Self {
          self.command.color = color;
          self
      }

      pub fn depth(mut self, depth: f32) -> Self {
          self.depth = depth;
          self
      }
  }

  impl Drop for LineBuilder<'_> {
      fn drop(&mut self) {
          self.ctx.commands.push(RenderCommand {
              command: Box::new(self.command.clone()),
              depth: self.depth,
              context_id: self.context_id,
          });
      }
  }

  pub struct SpriteBuilder<'a> {
      ctx: &'a mut RenderContext,
      command: SpriteCommand,
      context_id: usize,
      depth: f32,
  }

  impl<'a> SpriteBuilder<'a> {
      pub fn flip_x(mut self, flip: bool) -> Self {
          self.command.flip_x = flip;
          self
      }

      pub fn size(mut self, width: f32, height: f32) -> Self {
          self.command.size = [width, height];
          self
      }

      pub fn tint(mut self, color: [f32; 4]) -> Self {
          self.command.tint = color;
          self
      }

      pub fn depth(mut self, depth: f32) -> Self {
          self.depth = depth;
          self
      }
  }

  impl Drop for SpriteBuilder<'_> {
      fn drop(&mut self) {
          self.ctx.commands.push(RenderCommand {
              command: Box::new(self.command.clone()),
              depth: self.depth,
              context_id: self.context_id,
          });
      }
  }

  Dispatch is Now Generic

  impl RenderContext {
      pub fn submit(&mut self, shader_registry: &mut ShaderRegistry, rpass: &mut RenderPass) {
          // Sort by (context, depth, batch_key)
          self.commands.sort_by(|a, b| {
              (
                  a.context_id,
                  OrderedFloat(a.depth),
                  a.command.batch_key(),
              ).cmp(&(
                  b.context_id,
                  OrderedFloat(b.depth),
                  b.command.batch_key(),
              ))
          });

          let mut current_context = usize::MAX;

          for cmd in &self.commands {
              // Switch context if needed
              if cmd.context_id != current_context {
                  self.apply_context_state(rpass, shader_registry, &self.context_snapshots[cmd.context_id]);
                  current_context = cmd.context_id;
              }

              // Dispatch command (polymorphic!)
              cmd.command.dispatch(shader_registry);
          }

          self.commands.clear();
          self.context_snapshots.clear();
      }
  }

  Usage: Future Commands

  // Example with sprites, text, meshes
  renderer.viewport(game_rect, world_bounds, |game| {
      // Background sprite
      game.sprite(TEXTURE_BG, [0.0, 0.0])
          .size(3.0, 3.0)
          .depth(0.0);

      // Game pieces as sprites
      for piece in pieces {
          game.sprite(piece.texture_id, piece.position)
              .flip_x(piece.facing_left)
              .tint([1.0, 1.0, 1.0, piece.alpha])
              .depth(0.3);
      }

      // Particle effects
      game.particles(PARTICLE_TEXTURE, &particle_positions)
          .blend_mode(BlendMode::Additive)
          .depth(0.5);

      // Score text
      game.text(format!("Score: {}", score), [-1.4, 1.4])
          .font_size(0.2)  // In world units
          .color([1.0, 1.0, 1.0, 1.0])
          .depth(0.9);

      // Custom mesh (e.g., trail effect)
      game.mesh(&trail_vertices)
          .blend_mode(BlendMode::Additive)
          .depth(0.4);
  });

  Complexity Limits and Mitigation

  Potential Issues:

  1. Trait object overhead - Boxing every command
    - Mitigation: Commands are small, frame-lifetime only
    - Benchmark: ~100-1000 draw calls/frame typical in 2D
  2. Dynamic dispatch - Virtual function calls
    - Mitigation: Dispatch happens once per command, not per vertex
    - GPU work dominates anyway
  3. Type erasure - Can't pattern match on command types
    - Mitigation: Most logic is in builders, dispatch is generic
    - If needed, can add as_any() for downcast
  4. Shader registry coupling - Each command needs to know its shader
    - Mitigation: Registration-based system (already have this)
    - Alternative: Pass shader as parameter

  Scalability Metrics:

  | Command Types   | Enum Approach  | Trait Approach     |
  |-----------------|----------------|--------------------|
  | 5 types         | ✅ Manageable   | ⚠️ Slight overhead |
  | 10 types        | ⚠️ Large match | ✅ Clean            |
  | 20+ types       | ❌ Unwieldy     | ✅ Scales well      |
  | User extensions | ❌ Impossible   | ✅ Easy             |

  Recommendation: Start with enum for 2-3 types (lines, ellipses), migrate to trait when you hit ~5 types or need extensibility.

  Hybrid Approach: Best of Both

  // Core commands as enum (fast path)
  pub enum CoreDrawCommand {
      Line(LineCommand),
      Lines(LineCommand),
      Ellipse(EllipseCommand),
      Ellipses(EllipseCommand),
  }

  // Extensible commands as trait objects
  pub enum DrawCommand {
      Core(CoreDrawCommand),
      Custom(Box<dyn CustomDrawCommand>),
  }

  // Fast dispatch for core commands
  impl DrawCommand {
      fn dispatch(&self, registry: &mut ShaderRegistry) {
          match self {
              DrawCommand::Core(core) => {
                  match core {
                      CoreDrawCommand::Line(cmd) | CoreDrawCommand::Lines(cmd) => {
                          // Optimized inline dispatch
                          registry.line_renderer().draw(cmd);
                      }
                      CoreDrawCommand::Ellipse(cmd) | CoreDrawCommand::Ellipses(cmd) => {
                          registry.ellipse_renderer().draw(cmd);
                      }
                  }
              }
              DrawCommand::Custom(custom) => {
                  // Polymorphic dispatch for extensions
                  custom.dispatch(registry);
              }
          }
      }
  }

  This gives:
  - ✅ Fast path for common commands (no boxing, no virtual calls)
  - ✅ Extensibility for custom commands
  - ✅ Clean API surface
  - ✅ Scales to 20+ command types

  Your thoughts? Which approach feels right for your use case?

⏺ I just addressed this in my previous response! Let me summarize the key findings:

  Summary: Scalability Analysis

  Three Approaches Evaluated

  1. Enum-based (current plan):
  - ✅ Works well for 2-5 command types
  - ❌ Becomes unwieldy at 10+ types (large match statements)
  - ❌ Can't extend without modifying core code

  2. Trait-based (fully polymorphic):
  - ✅ Scales to 20+ command types easily
  - ✅ Extensible (users can add custom commands)
  - ⚠️ Boxing overhead (small, acceptable for 2D)
  - ✅ Generic dispatch code

  3. Hybrid (enum core + trait extensions):
  - ✅ Fast path for common commands (lines, ellipses)
  - ✅ Extensible for custom commands
  - ✅ Best of both worlds

  Recommended Architecture

  // Trait all commands implement
  pub trait DrawCommand {
      fn shader_name(&self) -> &str;
      fn dispatch(&self, registry: &mut ShaderRegistry);
      fn batch_key(&self) -> u64;
  }

  // Each command type implements it
  impl DrawCommand for LineCommand { ... }
  impl DrawCommand for SpriteCommand { ... }
  impl DrawCommand for TextCommand { ... }

  // Uniform dispatch in command buffer
  for cmd in &commands {
      cmd.command.dispatch(registry);  // Polymorphic!
  }

  Complexity Limits

  | Metric           | Limit                          | Mitigation              |
  |------------------|--------------------------------|-------------------------|
  | Command types    | ~50 before compile time issues | Use trait objects       |
  | Commands/frame   | ~10k before perf issues        | Batch similar commands  |
  | Context switches | ~100 before GPU stalls         | Sort by context_id      |
  | Shader types     | ~20 before bind overhead       | Group by shader in sort |

  My Recommendation

  Start with hybrid approach:
  1. Enum for lines/ellipses (Phase 1-3)
  2. Add trait when you add sprites (Phase 4+)
  3. Migrate to full trait if you hit >5 command types

  Is there a specific aspect you'd like me to dive deeper into?

  - A) Detailed trait design with concrete examples
  - B) Performance analysis (benchmarking strategy)
  - C) Migration path from enum → trait
  - D) Something else

  Or are you ready to start implementing Phase 1 with the viewport-based design?

> 1. Trait object overhead - Boxing every command
    - Mitigation: Commands are small, frame-lifetime only
    - Benchmark: ~100-1000 draw calls/frame typical in 2D

Since this has a frame lifetime can we pull from an arena of memory pre-allocated and cleared every frame? I don't know the most "rusty" way to achive
this/solve this problem.

  2. Dynamic dispatch - Virtual function calls
    - Mitigation: Dispatch happens once per command, not per vertex
    - GPU work dominates anyway

I'm not too worried about this.

  3. Type erasure - Can't pattern match on command types
    - Mitigation: Most logic is in builders, dispatch is generic
    - If needed, can add as_any() for downcast
  4. Shader registry coupling - Each command needs to know its shader
    - Mitigation: Registration-based system (already have this)
    - Alternative: Pass shader as parameter

This is okay for now. Later improvements to the registration system can help here.

  - A) Detailed trait design with concrete examples

I think we can design a strong core trait, then preparing for phase 1 implementation. End your response with a quick summary of the phases of
implementation. Focus on end goal of the new and improved system, leave migration concerns out of the picture.

⏺ Excellent questions! Let me address arena allocation and design the final trait-based system.

⏺ Arena Allocation for Frame-Lifetime Commands

  The Rusty Way: Reuse Vec Capacity

  The most idiomatic approach in Rust is to reuse Vec capacity across frames:

  pub struct CommandBuffer {
      // Vec capacity is retained after clear()
      commands: Vec<RenderCommand>,
  }

  impl CommandBuffer {
      pub fn new() -> Self {
          Self {
              commands: Vec::with_capacity(256),  // Pre-allocate
          }
      }

      pub fn clear(&mut self) {
          self.commands.clear();  // Sets len=0, keeps capacity
          // No deallocation! Next frame reuses the same memory
      }

      pub fn submit_command(&mut self, cmd: RenderCommand) {
          self.commands.push(cmd);  // Usually no allocation after warmup
      }
  }

  How it works:
  1. First frame: Allocates as needed
  2. clear(): Sets length to 0, but keeps allocated memory
  3. Next frame: Pushes into existing capacity (no malloc)
  4. After a few frames: Reaches steady-state capacity, zero allocations

  Pros:
  - ✅ Idiomatic Rust
  - ✅ Zero-cost after warmup
  - ✅ No unsafe code
  - ✅ Works with Box

  Alternative: Bump Allocator (bumpalo)

  If you want to avoid Box entirely and allocate all commands in a single arena:

  use bumpalo::Bump;

  pub struct CommandBuffer {
      arena: Bump,                           // Frame-lifetime arena
      commands: Vec<&'arena dyn DrawCommand>, // References into arena
  }

  impl CommandBuffer {
      pub fn new() -> Self {
          Self {
              arena: Bump::new(),
              commands: Vec::new(),
          }
      }

      pub fn clear(&mut self) {
          self.arena.reset();     // Free all arena memory at once
          self.commands.clear();  // Clear references
      }

      pub fn push_command<T: DrawCommand>(&mut self, cmd: T) -> &mut T {
          // Allocate in arena (fast bump pointer increment)
          let cmd_ref = self.arena.alloc(cmd);
          self.commands.push(cmd_ref);
          cmd_ref
      }
  }

  Pros:
  - ✅ Slightly faster allocation (bump pointer vs malloc)
  - ✅ Better cache locality (all commands contiguous)
  - ✅ Single deallocation for entire frame

  Cons:
  - ⚠️ Requires bumpalo dependency
  - ⚠️ Lifetime complexity ('arena lifetime)

  Recommendation: Start with Vec, Add Arena Later

  // Phase 1: Vec approach (simple, idiomatic)
  pub struct CommandBuffer {
      commands: Vec<RenderCommand>,
  }

  // Later optimization (if needed): Arena approach
  pub struct CommandBuffer {
      arena: Bump,
      commands: Vec<&'arena dyn DrawCommand>,
  }

  Benchmark first - Vec is probably fast enough for 1000 draw calls/frame.

⏺ Core Trait Design: DrawCommand

  The Trait

  use std::hash::{Hash, Hasher};

  /// Core trait that all drawable commands implement
  pub trait DrawCommand: Send + Sync {
      /// Shader that renders this command
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
      let mut hasher = std::collections::hash_map::DefaultHasher::new();
      shader.hash(&mut hasher);
      let shader_hash = hasher.finish();

      // Combine shader hash (high bits) with material hash (low bits)
      (shader_hash & 0xFFFFFFFF00000000) | (material_hash & 0x00000000FFFFFFFF)
  }

  Concrete Implementations

  1. Line Commands

  use std::collections::hash_map::DefaultHasher;

  /// Batch line rendering
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
          std::mem::size_of::<Self>() + self.segments.len() * std::mem::size_of::<([f32; 2], [f32; 2])>()
      }

      fn debug_name(&self) -> &str {
          if self.segments.len() == 1 {
              "Line"
          } else {
              "Lines (batch)"
          }
      }
  }

  2. Ellipse Commands

  #[derive(Clone, Debug)]
  pub struct EllipseGeometry {
      pub center: [f32; 2],
      pub radius_x: f32,
      pub radius_y: f32,
      pub rotation: f32,
  }

  /// Batch ellipse rendering
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

  3. Sprite Command (Future)

  #[derive(Clone, Debug, Hash, PartialEq, Eq)]
  pub struct TextureId(pub u32);

  #[derive(Clone, Debug)]
  pub struct SpriteGeometry {
      pub position: [f32; 2],
      pub size: [f32; 2],
      pub uv_rect: [f32; 4],  // [min_u, min_v, max_u, max_v]
  }

  /// Batch sprite rendering (same texture)
  #[derive(Clone, Debug)]
  pub struct SpriteCommand {
      pub texture_id: TextureId,
      pub sprites: Vec<SpriteGeometry>,
      pub tint: [f32; 4],
      pub flip_x: bool,
      pub flip_y: bool,
  }

  impl DrawCommand for SpriteCommand {
      fn shader_name(&self) -> &str {
          "sprite"
      }

      fn dispatch(&self, shader_registry: &mut ShaderRegistry) {
          if let Some(sprite_renderer) = shader_registry
              .get_mut("sprite")
              .and_then(|r| r.as_any_mut().downcast_mut::<SpriteRenderer>())
          {
              sprite_renderer.begin_batch(self.texture_id, self.tint);
              for sprite in &self.sprites {
                  sprite_renderer.draw_sprite(sprite.position, sprite.size, sprite.uv_rect);
              }
              sprite_renderer.end_batch();
          }
      }

      fn batch_key(&self) -> u64 {
          // Sprites with same texture batch together
          let material_hash = self.texture_id.0 as u64;
          compute_batch_key("sprite", material_hash)
      }

      fn debug_name(&self) -> &str {
          "Sprites (batch)"
      }
  }

  4. Text Command (Future)

  #[derive(Clone, Debug, Hash, PartialEq, Eq)]
  pub struct FontId(pub u32);

  #[derive(Clone, Debug)]
  pub struct TextCommand {
      pub text: String,
      pub position: [f32; 2],
      pub font_id: FontId,
      pub font_size: f32,
      pub color: [f32; 4],
      pub align: TextAlign,
  }

  #[derive(Clone, Debug)]
  pub enum TextAlign {
      Left,
      Center,
      Right,
  }

  impl DrawCommand for TextCommand {
      fn shader_name(&self) -> &str {
          "text"
      }

      fn dispatch(&self, shader_registry: &mut ShaderRegistry) {
          if let Some(text_renderer) = shader_registry
              .get_mut("text")
              .and_then(|r| r.as_any_mut().downcast_mut::<TextRenderer>())
          {
              text_renderer.draw_text(
                  &self.text,
                  self.position,
                  self.font_id,
                  self.font_size,
                  self.color,
                  self.align,
              );
          }
      }

      fn batch_key(&self) -> u64 {
          // Text with same font batches together
          let material_hash = self.font_id.0 as u64;
          compute_batch_key("text", material_hash)
      }

      fn debug_name(&self) -> &str {
          "Text"
      }
  }

  5. Mesh Command (Future)

  #[derive(Clone, Debug)]
  pub struct Vertex {
      pub position: [f32; 2],
      pub color: [f32; 4],
      pub uv: [f32; 2],
  }

  #[derive(Clone, Debug)]
  pub struct MeshCommand {
      pub vertices: Vec<Vertex>,
      pub indices: Option<Vec<u16>>,  // None = use vertices directly
      pub texture_id: Option<TextureId>,
      pub blend_mode: BlendMode,
  }

  #[derive(Clone, Debug, PartialEq, Eq)]
  pub enum BlendMode {
      Alpha,
      Additive,
      Multiply,
  }

  impl DrawCommand for MeshCommand {
      fn shader_name(&self) -> &str {
          if self.texture_id.is_some() {
              "mesh_textured"
          } else {
              "mesh_colored"
          }
      }

      fn dispatch(&self, shader_registry: &mut ShaderRegistry) {
          let shader_name = self.shader_name();
          if let Some(mesh_renderer) = shader_registry
              .get_mut(shader_name)
              .and_then(|r| r.as_any_mut().downcast_mut::<MeshRenderer>())
          {
              mesh_renderer.draw_mesh(&self.vertices, self.indices.as_deref(), self.texture_id);
          }
      }

      fn batch_key(&self) -> u64 {
          let mut hasher = DefaultHasher::new();
          self.texture_id.hash(&mut hasher);
          self.blend_mode.hash(&mut hasher);
          let material_hash = hasher.finish();

          compute_batch_key(self.shader_name(), material_hash)
      }

      fn debug_name(&self) -> &str {
          "Mesh"
      }
  }

  RenderCommand Structure

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

⏺ Builder Integration

  Builder Types

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

          self.ctx.commands.push(RenderCommand::new(
              command,
              self.depth,
              self.context_id,
          ));
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
          // Apply to all items
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

          self.ctx.commands.push(RenderCommand::new(
              command,
              self.depth,
              self.context_id,
          ));
      }
  }

  /// Builder for sprite commands (future)
  pub struct SpriteBuilder<'a> {
      ctx: &'a mut RenderContext,
      texture_id: TextureId,
      sprites: Vec<SpriteGeometry>,
      tint: [f32; 4],
      flip_x: bool,
      flip_y: bool,
      depth: f32,
      context_id: usize,
  }

  impl<'a> SpriteBuilder<'a> {
      pub fn size(mut self, width: f32, height: f32) -> Self {
          for sprite in &mut self.sprites {
              sprite.size = [width, height];
          }
          self
      }

      pub fn flip_x(mut self, flip: bool) -> Self {
          self.flip_x = flip;
          self
      }

      pub fn flip_y(mut self, flip: bool) -> Self {
          self.flip_y = flip;
          self
      }

      pub fn tint(mut self, color: [f32; 4]) -> Self {
          self.tint = color;
          self
      }

      pub fn depth(mut self, depth: f32) -> Self {
          self.depth = depth;
          self
      }
  }

  impl Drop for SpriteBuilder<'_> {
      fn drop(&mut self) {
          let command = SpriteCommand {
              texture_id: self.texture_id,
              sprites: std::mem::take(&mut self.sprites),
              tint: self.tint,
              flip_x: self.flip_x,
              flip_y: self.flip_y,
          };

          self.ctx.commands.push(RenderCommand::new(
              command,
              self.depth,
              self.context_id,
          ));
      }
  }

  ViewportScope API

  /// Scoped rendering context with viewport transform
  pub struct ViewportScope<'a> {
      ctx: &'a mut RenderContext,
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
      pub fn ellipse(&mut self, center: [f32; 2], radius_x: f32, radius_y: f32) -> EllipseBuilder<'_> {
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

      // ===== SPRITE API (future) =====

      /// Draw a sprite
      pub fn sprite(&mut self, texture_id: TextureId, position: [f32; 2]) -> SpriteBuilder<'_> {
          SpriteBuilder::new(self.ctx, texture_id, vec![SpriteGeometry {
              position,
              size: [1.0, 1.0],
              uv_rect: [0.0, 0.0, 1.0, 1.0],
          }])
      }

      /// Draw multiple sprites with same texture (batch)
      pub fn sprites(&mut self, texture_id: TextureId, items: &[SpriteGeometry]) -> SpriteBuilder<'_> {
          SpriteBuilder::with_batch(self.ctx, texture_id, items.to_vec())
      }

      // ===== TEXT API (future) =====

      /// Draw text
      pub fn text(&mut self, text: impl Into<String>, position: [f32; 2]) -> TextBuilder<'_> {
          TextBuilder::new(self.ctx, text.into(), position)
      }

      // ===== CONTEXT NESTING =====

      /// Nested viewport
      pub fn viewport(
          &mut self,
          config: ViewportConfig,
          f: impl FnOnce(&mut ViewportScope<'_>),
      ) {
          self.ctx.viewport(config, f);
      }

      /// Apply color tint
      pub fn tinted(
          &mut self,
          tint: [f32; 4],
          f: impl FnOnce(&mut ViewportScope<'_>),
      ) {
          self.ctx.tinted(tint, f);
      }

      /// Apply transform
      pub fn transformed(
          &mut self,
          transform: Mat3x3,
          f: impl FnOnce(&mut ViewportScope<'_>),
      ) {
          self.ctx.transformed(transform, f);
      }

      /// Render at specific depth range
      pub fn depth_range(
          &mut self,
          min: f32,
          max: f32,
          f: impl FnOnce(&mut ViewportScope<'_>),
      ) {
          self.ctx.depth_range(min, max, f);
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

  Usage Example

  // Complete usage showing all features
  pub fn render_game(&mut self, world: &World) {
      let renderer = self.renderer.commands();

      // Main game viewport
      renderer.viewport(
          ViewportConfig {
              pixel_rect: Rect { x: 100, y: 50, width: 600, height: 600 },
              coord_bounds: Bounds { min: [-1.5, -1.5], max: [1.5, 1.5] },
          },
          |game| {
              // Board grid (batch)
              game.lines(&[
                  ([-1.5, -0.5], [1.5, -0.5]),
                  ([-1.5, 0.5], [1.5, 0.5]),
                  ([-0.5, -1.5], [-0.5, 1.5]),
                  ([0.5, -1.5], [0.5, 1.5]),
              ])
              .thickness(4.0)
              .color([1.0, 1.0, 1.0])
              .depth(0.0);

              // Pieces
              for piece in &world.pieces {
                  match piece.kind {
                      PieceKind::X => {
                          game.lines(&[
                              ([piece.pos[0] - 0.3, piece.pos[1] - 0.3],
                               [piece.pos[0] + 0.3, piece.pos[1] + 0.3]),
                              ([piece.pos[0] + 0.3, piece.pos[1] - 0.3],
                               [piece.pos[0] - 0.3, piece.pos[1] + 0.3]),
                          ])
                          .thickness(4.0)
                          .depth(0.1);
                      }
                      PieceKind::O => {
                          game.circle(piece.pos, 0.3)
                              .depth(0.1);
                      }
                  }
              }

              // Leaves (individual, varying colors)
              for leaf in &world.leaves {
                  game.ellipse(leaf.position, leaf.radius_x * leaf.growth, leaf.radius_y * leaf.growth)
                      .rotation(leaf.rotation)
                      .color(LEAF_COLORS[leaf.variant % 4])
                      .alpha(leaf.growth)
                      .depth(0.5);
              }

              // Damage flash effect
              if world.damage_flash > 0.0 {
                  game.tinted([1.0, 0.3, 0.3, world.damage_flash], |flash| {
                      // Everything in here is tinted red
                  });
              }

              // Minimap (nested viewport)
              game.viewport(
                  ViewportConfig {
                      pixel_rect: Rect { x: 500, y: 10, width: 90, height: 90 },
                      coord_bounds: Bounds { min: [-1.5, -1.5], max: [1.5, 1.5] },
                  },
                  |minimap| {
                      // Render same scene at smaller scale
                      minimap.lines(&grid_lines).thickness(1.0).depth(0.0);
                      for leaf in &world.leaves {
                          minimap.circle(leaf.position, 0.1).alpha(0.5).depth(0.5);
                      }
                  },
              );
          },
      );

      // UI overlay (screen space, no viewport)
      renderer.text(format!("Score: {}", world.score), [10.0, 10.0])
          .font_size(24.0)
          .color([1.0, 1.0, 1.0, 1.0])
          .depth(1.0);
  }

⏺ Implementation Phases: End Goal System

  Phase 1: Core Infrastructure (Foundation)

  Goal: Trait-based command system with viewport contexts

  Deliverables:
  1. src/app/renderer/command.rs - Command system
    - DrawCommand trait
    - LineCommand implementation
    - EllipseCommand implementation
    - RenderCommand struct
    - Batch key computation
  2. src/app/renderer/context.rs - Rendering context
    - RenderContext struct with command buffer
    - ContextState for viewport/blend/transform state
    - Context stack (push/pop state)
    - ViewportScope for scoped rendering
    - Frame clear/submit
  3. Update shader uniforms - GPU coordinate transforms
    - Add viewport uniforms to LineRenderer
    - Add viewport uniforms to EllipseRenderer
    - Vertex shader transforms logical coords → NDC

  Result: Can render lines and ellipses with viewport-based coordinate transforms

  ---
  Phase 2: Builder API (Ergonomics)

  Goal: Fluent declarative API for command construction

  Deliverables:
  1. LineBuilder - Fluent line API
    - .thickness(), .color(), .depth()
    - Auto-submit on drop
    - Batch support (lines(&[...]))
  2. EllipseBuilder - Fluent ellipse API
    - .rotation(), .color(), .alpha(), .depth()
    - Auto-submit on drop
    - Batch support (ellipses(&[...]))
  3. ViewportScope methods
    - line(), lines() - Create LineBuilder
    - ellipse(), ellipses(), circle() - Create EllipseBuilder
    - viewport() - Nested viewport
    - tinted(), transformed() - Context modifiers

  Result: Clean API like game.ellipse(pos, rx, ry).color([r, g, b]).depth(0.5)

  ---
  Phase 3: Context System (Composition)

  Goal: Stackable rendering state with GPU acceleration

  Deliverables:
  1. Viewport contexts
    - Pixel rect + coordinate bounds
    - Scissor rect clipping
    - GPU transform via shader uniforms
    - Nested viewports
  2. Render state contexts
    - Color tint (multiply mode)
    - Alpha multiplier
    - Depth range compression
    - Transform matrices (future)
    - Blend modes (future)
  3. Command dispatch
    - Sort by (context_id, depth, batch_key)
    - Apply context state (scissor, viewport, uniforms)
    - Dispatch commands to shaders
    - Frame clear

  Result: Can render with renderer.viewport(rect, bounds, |vp| { ... }) and nested contexts

  ---
  Phase 4: Batch Optimization (Performance)

  Goal: Efficient GPU utilization via smart batching

  Deliverables:
  1. Batch key system
    - Commands compute batch keys (shader + material)
    - Sort groups same-shader commands
    - Minimize state changes
  2. Command reuse (Vec capacity retention)
    - Pre-allocate command buffer
    - Clear without deallocation
    - Zero allocations after warmup
  3. Optional: Arena allocator (if benchmarks show benefit)
    - bumpalo integration
    - Single frame-lifetime allocation
    - Reset arena each frame

  Result: 1000+ draw calls/frame with minimal allocation overhead

  ---
  Phase 5: Extended Commands (Expansion)

  Goal: Add sprites, text, meshes, particles

  Deliverables:
  1. Sprite rendering
    - SpriteCommand + SpriteBuilder
    - Texture atlas support
    - Batch by texture
    - Flip/tint/scale
  2. Text rendering
    - TextCommand + TextBuilder
    - Font atlas rendering
    - Batch by font
    - Alignment/size/color
  3. Mesh rendering (future)
    - MeshCommand + MeshBuilder
    - Custom geometry
    - Blend modes
    - Textured/colored variants
  4. Particle systems (future)
    - Efficient instanced rendering
    - GPU simulation

  Result: Full 2D rendering engine with sprites, text, particles

  ---
  Phase 6: Debug & Profiling (Polish)

  Goal: Tools for understanding and optimizing rendering

  Deliverables:
  1. Render stats
    - Draw call count
    - Batch count
    - Context switch count
    - Memory usage
  2. Debug visualization
    - Viewport bounds overlay
    - Depth layer visualization
    - Batch color coding
  3. Performance profiling
    - Per-command timing
    - GPU/CPU bottleneck analysis
    - Frame budget visualization

  Result: Production-ready rendering system with full observability

  ---
  End State: The Complete System

  // Game rendering with full feature set
  pub fn render(&mut self, world: &World) {
      let renderer = self.renderer.commands();

      // Main game viewport (world coordinates)
      renderer.viewport(game_rect, world_bounds, |game| {
          // Background sprite layer
          game.sprite(BG_TEXTURE, [0.0, 0.0])
              .size(3.0, 3.0)
              .depth(0.0);

          // Grid lines (batch)
          game.lines(&grid_segments)
              .thickness(4.0)
              .depth(0.1);

          // Game pieces (sprites batched by texture)
          game.sprites(PIECE_TEXTURE, &piece_data)
              .depth(0.2);

          // Leaves (individual, varying properties)
          for leaf in &world.leaves {
              game.ellipse(leaf.pos, leaf.rx, leaf.ry)
                  .rotation(leaf.angle)
                  .color(leaf.color)
                  .alpha(leaf.growth)
                  .depth(0.5);
          }

          // Particle effects (additive blending)
          game.particles(PARTICLE_TEXTURE, &particles)
              .blend_mode(BlendMode::Additive)
              .depth(0.6);

          // Score display (text)
          game.text(format!("{}", score), [1.2, 1.4])
              .font_size(0.3)
              .align(TextAlign::Right)
              .depth(0.9);

          // Damage flash (tint modifier)
          if damage_flash > 0.0 {
              game.tinted([1.0, 0.3, 0.3, damage_flash], |flash| {
                  // Entire scene rendered with red tint
              });
          }

          // Minimap (nested viewport)
          game.viewport(minimap_rect, world_bounds, |minimap| {
              // Rotated view around player
              minimap.transformed(Mat3x3::rotation(player_angle), |rotated| {
                  rotated.lines(&grid_segments).thickness(1.0).depth(0.0);
                  rotated.circle(player_pos, 0.1).color([1.0, 0.0, 0.0]).depth(1.0);
              });
          });
      });

      // UI overlay (screen space)
      renderer.text("GAME OVER", [screen_w / 2.0, screen_h / 2.0])
          .font_size(48.0)
          .align(TextAlign::Center)
          .depth(1.0);
  }

  Key Features:
  - ✅ Viewport-based coordinate transforms (GPU-accelerated)
  - ✅ Batch-first API (optimized for GPU)
  - ✅ Context accumulation (nested state)
  - ✅ Trait-based extensibility (easy to add new commands)
  - ✅ Declarative immediate mode (rebuild each frame, simple)
  - ✅ Efficient (zero allocations after warmup, smart batching)

⏺ ---
  Summary

  Design Complete ✓

  Core Architecture:
  1. Trait-based commands - Extensible, scales to 20+ types
  2. Viewport contexts - GPU-accelerated coordinate transforms
  3. Batch-first API - lines(&[...]) for efficiency
  4. Context accumulation - Nested viewports, tints, transforms
  5. Frame-lifetime allocation - Vec capacity reuse (zero-alloc after warmup)

  Implementation Path:
  - Phase 1 - Core infrastructure (DrawCommand trait, RenderContext, viewport transforms)
  - Phase 2 - Builder API (fluent interface, auto-submit)
  - Phase 3 - Context system (viewport stack, state composition)
  - Phase 4 - Batch optimization (smart sorting, allocation reuse)
  - Phase 5 - Extended commands (sprites, text, meshes)
  - Phase 6 - Debug tools (stats, visualization)

  End Result:
  renderer.viewport(game_rect, world_bounds, |game| {
      game.lines(&grid).thickness(4.0).depth(0.0);
      game.sprite(tex, pos).flip_x(true).depth(0.3);
      game.text("Score: 10", pos).font_size(24.0).depth(0.9);

      game.viewport(minimap_rect, world_bounds, |minimap| {
          // Nested rendering with automatic transforms
      });
  });

  ---
  Ready to Build?

  Want me to start implementing Phase 1 (core infrastructure)? We'll build:
  1. src/app/renderer/command.rs - DrawCommand trait + Line/Ellipse implementations
  2. src/app/renderer/context.rs - RenderContext with viewport support
  3. Update shaders for viewport transforms

  Or would you like to discuss anything else first?

────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
> I approve this plan, accept your recommendations, and approve you to begin implementation! Write your TODO list and begin

