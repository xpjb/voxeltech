//! Screen-space palette swatches (NDC quads).

use wgpu::util::DeviceExt;

use crate::voxel::palette::SWATCH_COUNT;

const SWATCH_PX: f32 = 18.0;
const SWATCH_GAP: f32 = 2.0;
const PAD: f32 = 12.0;
const COLS: i32 = 16;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PaletteVertex {
    pub pos: [f32; 2],
    pub _pad: [f32; 2],
    pub color: [f32; 4],
}

unsafe impl bytemuck::Pod for PaletteVertex {}
unsafe impl bytemuck::Zeroable for PaletteVertex {}

pub struct PalettePipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl PalettePipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("palette overlay"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/palette_overlay.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("palette layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("palette pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
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
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        Self { pipeline }
    }
}

pub struct PaletteDraw {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
}

impl PaletteDraw {
    pub fn rebuild(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        swatches: &[glam::Vec4; SWATCH_COUNT],
    ) -> Self {
        let (verts, indices) = build_geometry(width, height, swatches);
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("palette vb"),
            contents: bytemuck::cast_slice(&verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("palette ib"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        Self {
            vertex_buffer,
            index_buffer,
            index_count: indices.len() as u32,
        }
    }
}

/// If `(mx, my)` hits a swatch, return its index (0..256).
pub fn pick_swatch(mx: f32, my: f32, width: u32, height: u32) -> Option<usize> {
    let w = width.max(1) as f32;
    let h = height.max(1) as f32;
    let grid_w = COLS as f32 * SWATCH_PX + (COLS - 1) as f32 * SWATCH_GAP;
    let rows = (SWATCH_COUNT / COLS as usize) as i32;
    let grid_h = rows as f32 * SWATCH_PX + (rows - 1) as f32 * SWATCH_GAP;
    let left = w - PAD - grid_w;
    let top = h - PAD - grid_h;
    if mx < left || my < top || mx >= left + grid_w || my >= top + grid_h {
        return None;
    }
    let lx = mx - left;
    let ly = my - top;
    let cell = SWATCH_PX + SWATCH_GAP;
    let col = (lx / cell).floor() as i32;
    let row = (ly / cell).floor() as i32;
    if col < 0 || col >= COLS || row < 0 || row >= rows {
        return None;
    }
    let ix = col + row * COLS;
    if lx - col as f32 * cell >= SWATCH_PX || ly - row as f32 * cell >= SWATCH_PX {
        return None;
    }
    let idx = ix as usize;
    if idx < SWATCH_COUNT {
        Some(idx)
    } else {
        None
    }
}

fn pixel_to_ndc(px: f32, py: f32, w: f32, h: f32) -> [f32; 2] {
    let nx = (px / w) * 2.0 - 1.0;
    let ny = 1.0 - (py / h) * 2.0;
    [nx, ny]
}

fn build_geometry(
    width: u32,
    height: u32,
    swatches: &[glam::Vec4; SWATCH_COUNT],
) -> (Vec<PaletteVertex>, Vec<u32>) {
    let w = width.max(1) as f32;
    let h = height.max(1) as f32;
    let grid_w = COLS as f32 * SWATCH_PX + (COLS - 1) as f32 * SWATCH_GAP;
    let rows = (SWATCH_COUNT / COLS as usize) as i32;
    let grid_h = rows as f32 * SWATCH_PX + (rows - 1) as f32 * SWATCH_GAP;
    let left = w - PAD - grid_w;
    let top = h - PAD - grid_h;

    let mut verts = Vec::new();
    let mut indices = Vec::new();

    let (panel_v, panel_i) = panel_background_ndc(left - 4.0, top - 4.0, grid_w + 8.0, grid_h + 8.0, w, h);
    verts.extend(panel_v);
    indices.extend(panel_i);

    let cell = SWATCH_PX + SWATCH_GAP;

    for i in 0..SWATCH_COUNT {
        let col = (i % COLS as usize) as f32;
        let row = (i / COLS as usize) as f32;
        let x0 = left + col * cell;
        let y0 = top + row * cell;
        let x1 = x0 + SWATCH_PX;
        let y1 = y0 + SWATCH_PX;
        let c = swatches[i];
        let c_arr = [c.x, c.y, c.z, c.w];
        let p00 = pixel_to_ndc(x0, y0, w, h);
        let p10 = pixel_to_ndc(x1, y0, w, h);
        let p11 = pixel_to_ndc(x1, y1, w, h);
        let p01 = pixel_to_ndc(x0, y1, w, h);
        let base = verts.len() as u32;
        verts.push(PaletteVertex {
            pos: p00,
            _pad: [0.0, 0.0],
            color: c_arr,
        });
        verts.push(PaletteVertex {
            pos: p10,
            _pad: [0.0, 0.0],
            color: c_arr,
        });
        verts.push(PaletteVertex {
            pos: p11,
            _pad: [0.0, 0.0],
            color: c_arr,
        });
        verts.push(PaletteVertex {
            pos: p01,
            _pad: [0.0, 0.0],
            color: c_arr,
        });
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    (verts, indices)
}

/// Dark translucent backdrop behind swatches.
fn panel_background_ndc(
    px: f32,
    py: f32,
    gw: f32,
    gh: f32,
    w: f32,
    h: f32,
) -> (Vec<PaletteVertex>, Vec<u32>) {
    let x1 = px + gw;
    let y1 = py + gh;
    let p00 = pixel_to_ndc(px, py, w, h);
    let p10 = pixel_to_ndc(x1, py, w, h);
    let p11 = pixel_to_ndc(x1, y1, w, h);
    let p01 = pixel_to_ndc(px, y1, w, h);
    let bg = [0.06, 0.07, 0.09, 0.85_f32];
    let v = |p: [f32; 2]| PaletteVertex {
        pos: p,
        _pad: [0.0, 0.0],
        color: bg,
    };
    let verts = vec![v(p00), v(p10), v(p11), v(p01)];
    let idx = vec![0, 1, 2, 0, 2, 3];
    (verts, idx)
}
