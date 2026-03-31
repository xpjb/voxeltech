//! Crosshair lines and brush swatch in NDC (same shader/vertex layout as palette overlay).

use wgpu::util::DeviceExt;

use crate::render::palette_overlay::PaletteVertex;

pub struct EditorHud {
    pub line_pipeline: wgpu::RenderPipeline,
    pub tri_pipeline: wgpu::RenderPipeline,
}

impl EditorHud {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("editor hud"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/palette_overlay.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("editor hud layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let vertex_buffers = &[wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<PaletteVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }];

        let blend = Some(wgpu::BlendState::ALPHA_BLENDING);
        let line_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("editor hud lines"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: vertex_buffers,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        let tri_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("editor hud tris"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: vertex_buffers,
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        Self {
            line_pipeline,
            tri_pipeline,
        }
    }
}

pub struct EditorHudDraw {
    pub line_vertex_buffer: wgpu::Buffer,
    pub line_vertex_count: u32,
    pub swatch_vertex_buffer: wgpu::Buffer,
    pub swatch_index_buffer: wgpu::Buffer,
    pub swatch_index_count: u32,
}

impl EditorHudDraw {
    pub fn rebuild(device: &wgpu::Device, width: u32, height: u32, brush: glam::Vec4) -> Self {
        let w = width.max(1) as f32;
        let h = height.max(1) as f32;
        let cx = w * 0.5;
        let cy = h * 0.5;

        const ARM: f32 = 14.0;
        const GAP: f32 = 5.0;
        let cross = [1.0_f32, 1.0_f32, 1.0_f32, 0.92_f32];

        let v = |px: f32, py: f32, c: [f32; 4]| PaletteVertex {
            pos: pixel_to_ndc(px, py, w, h),
            _pad: [0.0, 0.0],
            color: c,
        };

        let line_verts = vec![
            v(cx - ARM, cy, cross),
            v(cx - GAP, cy, cross),
            v(cx + GAP, cy, cross),
            v(cx + ARM, cy, cross),
            v(cx, cy - ARM, cross),
            v(cx, cy - GAP, cross),
            v(cx, cy + GAP, cross),
            v(cx, cy + ARM, cross),
        ];

        const SWATCH: f32 = 15.0;
        const OFF: f32 = 20.0;
        let sx0 = cx + OFF;
        let sy0 = cy - SWATCH * 0.5;
        let sx1 = sx0 + SWATCH;
        let sy1 = cy + SWATCH * 0.5;
        let border = 1.5_f32;
        let outline = [0.08_f32, 0.09_f32, 0.12_f32, 1.0_f32];
        let fill = [brush.x, brush.y, brush.z, brush.w.max(0.001)];

        let (swatch_verts, swatch_indices) = bordered_quad_ndc(
            sx0, sy0, sx1, sy1, border, outline, fill, w, h,
        );

        let line_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("editor hud lines vb"),
            contents: bytemuck::cast_slice(&line_verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let swatch_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("editor hud swatch vb"),
            contents: bytemuck::cast_slice(&swatch_verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let swatch_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("editor hud swatch ib"),
            contents: bytemuck::cast_slice(&swatch_indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            line_vertex_buffer,
            line_vertex_count: line_verts.len() as u32,
            swatch_vertex_buffer,
            swatch_index_buffer,
            swatch_index_count: swatch_indices.len() as u32,
        }
    }
}

fn pixel_to_ndc(px: f32, py: f32, w: f32, h: f32) -> [f32; 2] {
    let nx = (px / w) * 2.0 - 1.0;
    let ny = 1.0 - (py / h) * 2.0;
    [nx, ny]
}

/// Outer quad (outline) and inset quad (fill), 8 verts, 12 indices (2 tris each).
fn bordered_quad_ndc(
    px0: f32,
    py0: f32,
    px1: f32,
    py1: f32,
    border: f32,
    outline: [f32; 4],
    fill: [f32; 4],
    w: f32,
    h: f32,
) -> (Vec<PaletteVertex>, Vec<u32>) {
    let v = |px: f32, py: f32, c: [f32; 4]| PaletteVertex {
        pos: pixel_to_ndc(px, py, w, h),
        _pad: [0.0, 0.0],
        color: c,
    };

    let ox0 = px0 - border;
    let oy0 = py0 - border;
    let ox1 = px1 + border;
    let oy1 = py1 + border;

    let ix0 = px0;
    let iy0 = py0;
    let ix1 = px1;
    let iy1 = py1;

    let verts = vec![
        v(ox0, oy0, outline),
        v(ox1, oy0, outline),
        v(ox1, oy1, outline),
        v(ox0, oy1, outline),
        v(ix0, iy0, fill),
        v(ix1, iy0, fill),
        v(ix1, iy1, fill),
        v(ix0, iy1, fill),
    ];

    let mut idx = Vec::new();
    for base in [0u32, 4] {
        idx.extend_from_slice(&[
            base, base + 1, base + 2, base, base + 2, base + 3,
        ]);
    }

    (verts, idx)
}
