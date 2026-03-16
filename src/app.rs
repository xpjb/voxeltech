//! Main application: SDL3 + wgpu loop, hotkeys, mouse input
//!
//! Uses a SyncWindow wrapper to satisfy wgpu's Send/Sync requirements for create_surface.

use glam::{IVec2, Mat4, Vec3};
use sdl3::event::Event;
use sdl3::keyboard::Keycode;
use wgpu::util::DeviceExt;
use wgpu::rwh::{HasDisplayHandle, HasWindowHandle};

use crate::game::state::GameState;
use crate::render::camera::Camera;
use crate::render::instance::InstanceBuffer;
use crate::render::mesh::Mesh;
use crate::render::pipeline::VoxelPipeline;
use crate::voxel::structures::{get_structure, STRUCTURE_COUNT};

const GRID_SIZE: i32 = 32;
const TILE_SIZE: f32 = 16.0;

pub fn run(single_structure: Option<u8>) {
    let sdl = sdl3::init().unwrap();
    let video = sdl.video().unwrap();
    let window = video
        .window("Voxel Builder", 1024, 768)
        .position_centered()
        .resizable()
        .build()
        .unwrap();

    let mut game_state = GameState::default();
    let single = single_structure.map(|n| n.min(9));

    if let Some(n) = single {
        // Single-structure debug mode: one instance at center tile
        let center_tile = IVec2::new(GRID_SIZE / 2, GRID_SIZE / 2);
        game_state.place(center_tile, n);
        game_state.selected_structure = Some(n);
    } else {
        game_state.selected_structure = Some(0);
    }

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let surface = instance
        .create_surface(SyncWindow(&window))
        .expect("create surface");

    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
    }))
    .expect("request adapter");

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("voxel device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: Default::default(),
        },
        None,
    ))
    .expect("request device");

    let mut surface_config = surface
        .get_default_config(&adapter, 1024, 768)
        .expect("default config");
    surface_config.format = wgpu::TextureFormat::Bgra8UnormSrgb;
    surface.configure(&device, &surface_config);

    let mut camera = match single {
        Some(_) => Camera::single_structure(),
        None => Camera::default(),
    };
    camera.resize(1024.0, 768.0);

    let view_proj = camera.view_projection_matrix();
    let camera_uniform = view_proj.to_cols_array();

    let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("camera uniform"),
        contents: bytemuck::cast_slice(&camera_uniform),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let pipeline = VoxelPipeline::new(&device, surface_config.format);
    let meshes: Vec<Mesh> = (0..STRUCTURE_COUNT)
        .map(|i| {
            let model = get_structure(i as u8);
            let (verts, indices) = model.to_mesh();
            Mesh::from_voxel_mesh(&device, &verts, &indices)
        })
        .collect();

    let instance_buffer = InstanceBuffer::new(&device);

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &pipeline.bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: instance_buffer.buffer.as_entire_binding(),
            },
        ],
        label: None,
    });

    let depth_format = wgpu::TextureFormat::Depth32Float;
    let mut depth_texture = create_depth_texture(&device, 1024, 768, depth_format);

    let mut event_pump = sdl.event_pump().unwrap();
    let mut running = true;
    let mut mouse_x = 0.0f32;
    let mut mouse_y = 0.0f32;

    while running {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => running = false,
                Event::KeyDown {
                    keycode: Some(k), ..
                } => {
                    match k {
                        Keycode::Escape => running = false,
                        Keycode::Q => camera.rotate_around_target(-std::f32::consts::FRAC_PI_2),
                        Keycode::E => camera.rotate_around_target(std::f32::consts::FRAC_PI_2),
                        Keycode::A if single.is_some() => {
                            let center_tile = IVec2::new(GRID_SIZE / 2, GRID_SIZE / 2);
                            let prev = game_state.selected_structure.map_or(9, |s| {
                                if s == 0 { 9 } else { s - 1 }
                            });
                            game_state.remove(center_tile);
                            game_state.place(center_tile, prev);
                            game_state.select_structure(Some(prev));
                        }
                        Keycode::D if single.is_some() => {
                            let center_tile = IVec2::new(GRID_SIZE / 2, GRID_SIZE / 2);
                            let next = game_state.selected_structure.map_or(0, |s| {
                                if s >= 9 { 0 } else { s + 1 }
                            });
                            game_state.remove(center_tile);
                            game_state.place(center_tile, next);
                            game_state.select_structure(Some(next));
                        }
                        Keycode::_1 => game_state.select_structure(Some(0)),
                        Keycode::_2 => game_state.select_structure(Some(1)),
                        Keycode::_3 => game_state.select_structure(Some(2)),
                        Keycode::_4 => game_state.select_structure(Some(3)),
                        Keycode::_5 => game_state.select_structure(Some(4)),
                        Keycode::_6 => game_state.select_structure(Some(5)),
                        Keycode::_7 => game_state.select_structure(Some(6)),
                        Keycode::_8 => game_state.select_structure(Some(7)),
                        Keycode::_9 => game_state.select_structure(Some(8)),
                        Keycode::_0 => game_state.select_structure(Some(9)),
                        Keycode::A => {
                            let prev = game_state.selected_structure.map_or(9, |s| if s == 0 { 9 } else { s - 1 });
                            game_state.select_structure(Some(prev));
                            if single.is_some() {
                                let center_tile = IVec2::new(GRID_SIZE / 2, GRID_SIZE / 2);
                                game_state.remove(center_tile);
                                game_state.place(center_tile, prev);
                            }
                        }
                        Keycode::D => {
                            let next = game_state.selected_structure.map_or(0, |s| if s >= 9 { 0 } else { s + 1 });
                            game_state.select_structure(Some(next));
                            if single.is_some() {
                                let center_tile = IVec2::new(GRID_SIZE / 2, GRID_SIZE / 2);
                                game_state.remove(center_tile);
                                game_state.place(center_tile, next);
                            }
                        }
                        _ => {}
                    }
                }
                Event::MouseMotion { x, y, .. } => {
                    mouse_x = x;
                    mouse_y = y;
                }
                Event::MouseButtonDown {
                    mouse_btn: sdl3::mouse::MouseButton::Left,
                    ..
                } => {
                    if let Some(id) = game_state.selected_structure {
                        let (w, h) = window.size();
                        let tile = screen_to_tile(mouse_x as i32, mouse_y as i32, w, h, &camera);
                        if tile.x >= 0 && tile.x < GRID_SIZE && tile.y >= 0 && tile.y < GRID_SIZE {
                            let key = IVec2::new(tile.x, tile.y);
                            if game_state.get(key).is_some() {
                                game_state.remove(key);
                            } else {
                                game_state.place(key, id);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        let size = window.size();
        let (width, height) = (size.0, size.1);
        if width > 0 && height > 0 {
            if depth_texture.width != width || depth_texture.height != height {
                depth_texture = create_depth_texture(&device, width, height, depth_format);
            }
            surface_config.width = width;
            surface_config.height = height;
            surface.configure(&device, &surface_config);
            camera.resize(width as f32, height as f32);
            queue.write_buffer(
                &camera_buffer,
                0,
                bytemuck::cast_slice(&camera.view_projection_matrix().to_cols_array()),
            );

            let mut instance_by_structure: Vec<Vec<Mat4>> = vec![Vec::new(); STRUCTURE_COUNT];
            for p in game_state.grid.values() {
                if (p.structure_id as usize) < STRUCTURE_COUNT {
                    instance_by_structure[p.structure_id as usize].push(Mat4::from_translation(
                        Vec3::new(
                            p.tile.x as f32 * TILE_SIZE,
                            0.0,
                            p.tile.y as f32 * TILE_SIZE,
                        ),
                    ));
                }
            }

            // Build flat instance buffer (structure 0, then 1, ...) so we can use base_instance.
            let mut all_instances: Vec<Mat4> = Vec::new();
            for insts in &instance_by_structure {
                all_instances.extend(insts);
            }
            if !all_instances.is_empty() {
                instance_buffer.write(&queue, &all_instances);
            }

            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
            let frame = surface
                .get_current_texture()
                .expect("get current texture");
            let view = frame
                .texture
                .create_view(&wgpu::TextureViewDescriptor::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.1,
                                g: 0.12,
                                b: 0.15,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &depth_texture.view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                pass.set_pipeline(&pipeline.pipeline);
                let mut base_instance = 0u32;
                for (i, mesh) in meshes.iter().enumerate() {
                    let insts = &instance_by_structure[i];
                    if insts.is_empty() {
                        continue;
                    }
                    let count = insts.len() as u32;
                    pass.set_bind_group(0, &bind_group, &[]);
                    pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                    pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..mesh.index_count, 0, base_instance..base_instance + count);
                    base_instance += count;
                }
            }
            queue.submit([encoder.finish()]);
            frame.present();
        }
    }
}

/// Wrapper to satisfy wgpu's Send/Sync requirements for create_surface.
/// SDL3's Window is not Send/Sync due to raw pointers; we only use it on the main thread.
struct SyncWindow<'a>(&'a sdl3::video::Window);

unsafe impl<'a> Send for SyncWindow<'a> {}
unsafe impl<'a> Sync for SyncWindow<'a> {}

impl<'a> HasWindowHandle for SyncWindow<'a> {
    fn window_handle(&self) -> Result<wgpu::rwh::WindowHandle<'_>, wgpu::rwh::HandleError> {
        self.0.window_handle()
    }
}

impl<'a> HasDisplayHandle for SyncWindow<'a> {
    fn display_handle(&self) -> Result<wgpu::rwh::DisplayHandle<'_>, wgpu::rwh::HandleError> {
        self.0.display_handle()
    }
}

fn screen_to_tile(screen_x: i32, screen_y: i32, width: u32, height: u32, camera: &Camera) -> IVec2 {
    let width = width.max(1) as f32;
    let height = height.max(1) as f32;
    let ndc_x = (screen_x as f32 / width) * 2.0 - 1.0;
    let ndc_y = 1.0 - (screen_y as f32 / height) * 2.0;
    let view_proj_inv = camera.view_projection_matrix().inverse();
    // Unproject using near plane (z=-1 in NDC) for orthographic
    let clip_near = glam::Vec4::new(ndc_x, ndc_y, -1.0, 1.0);
    let world = view_proj_inv * clip_near;
    let world = world / world.w;
    let tx = (world.x / TILE_SIZE).floor() as i32;
    let tz = (world.z / TILE_SIZE).floor() as i32;
    IVec2::new(tx, tz)
}

fn create_depth_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
) -> DepthTexture {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    DepthTexture {
        texture,
        view,
        width,
        height,
    }
}

struct DepthTexture {
    #[allow(dead_code)]
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
}
