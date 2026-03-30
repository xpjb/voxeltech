//! Main application: SDL3 + wgpu loop, hotkeys, mouse input
//!
//! Uses a SyncWindow wrapper to satisfy wgpu's Send/Sync requirements for create_surface.

use glam::{IVec2, Mat4, Vec3, Vec4};
use sdl3::event::Event;
use sdl3::event::EventType;
use sdl3::EventSubsystem;
use sdl3::keyboard::Keycode;
use sdl3::mouse::MouseButton;
use sdl3::mouse::RelativeMouseState;
use std::time::Instant;
use wgpu::rwh::{HasDisplayHandle, HasWindowHandle};
use wgpu::util::DeviceExt;

use crate::game::state::GameState;
use crate::render::camera::Camera;
use crate::render::fly_camera::FlyCamera;
use crate::render::instance::InstanceBuffer;
use crate::render::mesh::Mesh;
use crate::render::palette_overlay::{PaletteDraw, PalettePipeline, pick_swatch};
use crate::render::pipeline::VoxelPipeline;
use crate::voxel::model::VoxelModel;
use crate::voxel::palette;
use crate::voxel::pick::raycast_voxels;
use crate::voxel::structures::{get_structure, STRUCTURE_COUNT};

const GRID_SIZE: i32 = 32;
const TILE_SIZE: f32 = 16.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum AppMode {
    World,
    VoxelEditor,
}

pub fn run(single_structure: Option<u8>) {
    let sdl = sdl3::init().unwrap();
    // Ensure the event subsystem is initialized (needed for EventPump and global event filters).
    let _event_subsystem = sdl.event().unwrap();
    let sdl_mouse = sdl.mouse();
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
        let center_tile = IVec2::new(GRID_SIZE / 2, GRID_SIZE / 2);
        game_state.place(center_tile, n);
        game_state.selected_structure = Some(n);
    } else {
        game_state.selected_structure = Some(0);
    }

    let mut models: Vec<VoxelModel> = (0..STRUCTURE_COUNT)
        .map(|i| get_structure(i as u8))
        .collect();

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
    let mut saved_world_camera = camera;

    let sid0 = game_state.selected_structure.unwrap_or(0) as usize;
    let mut fly_cam = FlyCamera::for_voxel_model(models[sid0].dim, 1024.0, 768.0);

    let view_proj = camera.view_projection_matrix();
    let camera_uniform = view_proj.to_cols_array();

    let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("camera uniform"),
        contents: bytemuck::cast_slice(&camera_uniform),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let pipeline = VoxelPipeline::new(&device, surface_config.format);
    let palette_pipeline = PalettePipeline::new(&device, surface_config.format);

    let mut meshes: Vec<Mesh> = models
        .iter()
        .map(|m| {
            let (verts, indices) = m.to_mesh();
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

    let mut app_mode = AppMode::World;
    let mut brush_color = Vec4::new(1.0, 1.0, 1.0, 1.0);
    let mut palette_open = false;
    let mut palette_cache: Option<(u32, u32, PaletteDraw)> = None;
    let mut last_frame = Instant::now();

    while running {
        let dt = last_frame.elapsed().as_secs_f32().min(0.05);
        last_frame = Instant::now();

        // Queued mouse-motion storms (relative mode on Windows) can process 10k+ events/frame and
        // tank the CPU; SDL still accumulates deltas for SDL_GetRelativeMouseState when disabled.
        let editor_fly = app_mode == AppMode::VoxelEditor && !palette_open;
        EventSubsystem::set_event_enabled(EventType::MouseMotion, !editor_fly);

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => running = false,
                Event::KeyDown {
                    keycode: Some(k),
                    repeat,
                    ..
                } if !repeat => {
                    if k == Keycode::Escape && palette_open {
                        palette_open = false;
                        palette_cache = None;
                        if app_mode == AppMode::VoxelEditor {
                            sdl_mouse.set_relative_mouse_mode(&window, true);
                            sdl_mouse.show_cursor(false);
                        }
                        continue;
                    }
                    if k == Keycode::Escape {
                        continue;
                    }
                    match k {
                        Keycode::E => {
                            match app_mode {
                                AppMode::World => {
                                    saved_world_camera = camera;
                                    if let Some(sid) = game_state.selected_structure {
                                        fly_cam = FlyCamera::for_voxel_model(
                                            models[sid as usize].dim,
                                            camera.viewport_width,
                                            camera.viewport_height,
                                        );
                                    }
                                    app_mode = AppMode::VoxelEditor;
                                    if !palette_open {
                                        sdl_mouse.set_relative_mouse_mode(&window, true);
                                        sdl_mouse.show_cursor(false);
                                    }
                                }
                                AppMode::VoxelEditor => {
                                    camera = saved_world_camera;
                                    camera.resize(
                                        camera.viewport_width,
                                        camera.viewport_height,
                                    );
                                    app_mode = AppMode::World;
                                    sdl_mouse.set_relative_mouse_mode(&window, false);
                                    sdl_mouse.show_cursor(true);
                                }
                            }
                            palette_open = false;
                            palette_cache = None;
                        }
                        Keycode::C if app_mode == AppMode::VoxelEditor => {
                            palette_open = !palette_open;
                            if !palette_open {
                                palette_cache = None;
                                sdl_mouse.set_relative_mouse_mode(&window, true);
                                sdl_mouse.show_cursor(false);
                            } else {
                                sdl_mouse.set_relative_mouse_mode(&window, false);
                                sdl_mouse.show_cursor(true);
                            }
                        }
                        Keycode::Q if app_mode == AppMode::World => {
                            camera.rotate_around_target(-std::f32::consts::FRAC_PI_2);
                        }
                        Keycode::R if app_mode == AppMode::World => {
                            camera.rotate_around_target(std::f32::consts::FRAC_PI_2);
                        }
                        Keycode::A if single.is_some() && app_mode == AppMode::World => {
                            let center_tile = IVec2::new(GRID_SIZE / 2, GRID_SIZE / 2);
                            let prev = game_state.selected_structure.map_or(9, |s| {
                                if s == 0 { 9 } else { s - 1 }
                            });
                            game_state.remove(center_tile);
                            game_state.place(center_tile, prev);
                            game_state.select_structure(Some(prev));
                        }
                        Keycode::D if single.is_some() && app_mode == AppMode::World => {
                            let center_tile = IVec2::new(GRID_SIZE / 2, GRID_SIZE / 2);
                            let next = game_state.selected_structure.map_or(0, |s| {
                                if s >= 9 { 0 } else { s + 1 }
                            });
                            game_state.remove(center_tile);
                            game_state.place(center_tile, next);
                            game_state.select_structure(Some(next));
                        }
                        Keycode::_1 => {
                            select_with_editor_camera(
                                &mut game_state, &models, 0, &mut camera, &mut fly_cam, app_mode,
                            );
                        }
                        Keycode::_2 => {
                            select_with_editor_camera(
                                &mut game_state, &models, 1, &mut camera, &mut fly_cam, app_mode,
                            );
                        }
                        Keycode::_3 => {
                            select_with_editor_camera(
                                &mut game_state, &models, 2, &mut camera, &mut fly_cam, app_mode,
                            );
                        }
                        Keycode::_4 => {
                            select_with_editor_camera(
                                &mut game_state, &models, 3, &mut camera, &mut fly_cam, app_mode,
                            );
                        }
                        Keycode::_5 => {
                            select_with_editor_camera(
                                &mut game_state, &models, 4, &mut camera, &mut fly_cam, app_mode,
                            );
                        }
                        Keycode::_6 => {
                            select_with_editor_camera(
                                &mut game_state, &models, 5, &mut camera, &mut fly_cam, app_mode,
                            );
                        }
                        Keycode::_7 => {
                            select_with_editor_camera(
                                &mut game_state, &models, 6, &mut camera, &mut fly_cam, app_mode,
                            );
                        }
                        Keycode::_8 => {
                            select_with_editor_camera(
                                &mut game_state, &models, 7, &mut camera, &mut fly_cam, app_mode,
                            );
                        }
                        Keycode::_9 => {
                            select_with_editor_camera(
                                &mut game_state, &models, 8, &mut camera, &mut fly_cam, app_mode,
                            );
                        }
                        Keycode::_0 => {
                            select_with_editor_camera(
                                &mut game_state, &models, 9, &mut camera, &mut fly_cam, app_mode,
                            );
                        }
                        Keycode::A if app_mode == AppMode::World => {
                            let prev = game_state
                                .selected_structure
                                .map_or(9, |s| if s == 0 { 9 } else { s - 1 });
                            game_state.select_structure(Some(prev));
                            if single.is_some() {
                                let center_tile = IVec2::new(GRID_SIZE / 2, GRID_SIZE / 2);
                                game_state.remove(center_tile);
                                game_state.place(center_tile, prev);
                            }
                        }
                        Keycode::D if app_mode == AppMode::World => {
                            let next = game_state
                                .selected_structure
                                .map_or(0, |s| if s >= 9 { 0 } else { s + 1 });
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
                Event::MouseButtonDown { mouse_btn, x, y, .. } => {
                    let (w, h) = window.size();
                    let mx = x;
                    let my = y;
                    if palette_open {
                        if mouse_btn == MouseButton::Left {
                            if let Some(idx) = pick_swatch(mx, my, w, h) {
                                brush_color = palette::swatches()[idx];
                                palette_open = false;
                                palette_cache = None;
                                if app_mode == AppMode::VoxelEditor {
                                    sdl_mouse.set_relative_mouse_mode(&window, true);
                                    sdl_mouse.show_cursor(false);
                                }
                            }
                        }
                        continue;
                    }
                    if app_mode == AppMode::VoxelEditor {
                        if let Some(sid) = game_state.selected_structure {
                            let model = &models[sid as usize];
                            // Relative mouselook: cursor is hidden/warped; SDL click coords are unreliable.
                            // Match FPS-style block targeting through the viewport center.
                            let (pick_x, pick_y) = (w as f32 * 0.5, h as f32 * 0.5);
                            let (origin, dir) = fly_cam.ray_from_pixel(pick_x, pick_y, w, h);
                            if mouse_btn == MouseButton::Middle {
                                if let Some(hit) = raycast_voxels(model, origin, dir) {
                                    if let Some(c) = model.get(hit.solid.x, hit.solid.y, hit.solid.z) {
                                        brush_color = c;
                                    }
                                }
                            } else if let Some(hit) = raycast_voxels(model, origin, dir) {
                                match mouse_btn {
                                    MouseButton::Left => {
                                        models[sid as usize].set(
                                            hit.solid.x,
                                            hit.solid.y,
                                            hit.solid.z,
                                            Vec4::ZERO,
                                        );
                                        rebuild_mesh(&device, &mut meshes, &models, sid as usize);
                                    }
                                    MouseButton::Right => {
                                        let p = hit.air_before;
                                        if p.x >= 0
                                            && p.x < model.dim.x
                                            && p.y >= 0
                                            && p.y < model.dim.y
                                            && p.z >= 0
                                            && p.z < model.dim.z
                                            && !models[sid as usize].is_solid(p.x, p.y, p.z)
                                        {
                                            models[sid as usize].set(p.x, p.y, p.z, brush_color);
                                            rebuild_mesh(&device, &mut meshes, &models, sid as usize);
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        continue;
                    }
                    if mouse_btn == MouseButton::Left {
                        if let Some(id) = game_state.selected_structure {
                            let tile = screen_to_tile(mx as i32, my as i32, w, h, &camera);
                            if tile.x >= 0 && tile.x < GRID_SIZE && tile.y >= 0 && tile.y < GRID_SIZE
                            {
                                let key = IVec2::new(tile.x, tile.y);
                                if game_state.get(key).is_some() {
                                    game_state.remove(key);
                                } else {
                                    game_state.place(key, id);
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        if app_mode == AppMode::VoxelEditor && !palette_open {
            let rel = RelativeMouseState::new(&event_pump);
            fly_cam.add_look(rel.x(), rel.y());
            let kb = event_pump.keyboard_state();
            fly_cam.fly_tick(&kb, dt);
        }

        let size = window.size();
        let (width, height) = (size.0, size.1);
        if width > 0 && height > 0 {
            if depth_texture.width != width || depth_texture.height != height {
                depth_texture = create_depth_texture(&device, width, height, depth_format);
            }
            if width != surface_config.width || height != surface_config.height {
                surface_config.width = width;
                surface_config.height = height;
                surface.configure(&device, &surface_config);
            }
            camera.resize(width as f32, height as f32);
            fly_cam.resize(width as f32, height as f32);
            let view_proj = match app_mode {
                AppMode::World => camera.view_projection_matrix(),
                AppMode::VoxelEditor => fly_cam.view_projection_matrix(),
            };
            queue.write_buffer(
                &camera_buffer,
                0,
                bytemuck::cast_slice(&view_proj.to_cols_array()),
            );

            if app_mode == AppMode::VoxelEditor {
                let identity = [Mat4::IDENTITY];
                instance_buffer.write(&queue, &identity);
            } else {
                let mut instance_by_structure: Vec<Vec<Mat4>> =
                    vec![Vec::new(); STRUCTURE_COUNT];
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
                let mut all_instances: Vec<Mat4> = Vec::new();
                for insts in &instance_by_structure {
                    all_instances.extend(insts);
                }
                if !all_instances.is_empty() {
                    instance_buffer.write(&queue, &all_instances);
                }
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
                if app_mode == AppMode::VoxelEditor {
                    if let Some(sid) = game_state.selected_structure {
                        let i = sid as usize;
                        let mesh = &meshes[i];
                        if mesh.index_count > 0 {
                            pass.set_bind_group(0, &bind_group, &[]);
                            pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                            pass.set_index_buffer(
                                mesh.index_buffer.slice(..),
                                wgpu::IndexFormat::Uint32,
                            );
                            pass.draw_indexed(0..mesh.index_count, 0, 0..1);
                        }
                    }
                } else {
                    let mut instance_by_structure: Vec<Vec<Mat4>> =
                        vec![Vec::new(); STRUCTURE_COUNT];
                    for p in game_state.grid.values() {
                        if (p.structure_id as usize) < STRUCTURE_COUNT {
                            instance_by_structure[p.structure_id as usize].push(
                                Mat4::from_translation(Vec3::new(
                                    p.tile.x as f32 * TILE_SIZE,
                                    0.0,
                                    p.tile.y as f32 * TILE_SIZE,
                                )),
                            );
                        }
                    }
                    let mut base_instance = 0u32;
                    for (i, mesh) in meshes.iter().enumerate() {
                        let insts = &instance_by_structure[i];
                        if insts.is_empty() || mesh.index_count == 0 {
                            continue;
                        }
                        let count = insts.len() as u32;
                        pass.set_bind_group(0, &bind_group, &[]);
                        pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                        pass.set_index_buffer(
                            mesh.index_buffer.slice(..),
                            wgpu::IndexFormat::Uint32,
                        );
                        pass.draw_indexed(
                            0..mesh.index_count,
                            0,
                            base_instance..base_instance + count,
                        );
                        base_instance += count;
                    }
                }
            }

            if palette_open {
                let needs_rebuild = palette_cache
                    .as_ref()
                    .map_or(true, |(pw, ph, _)| *pw != width || *ph != height);
                if needs_rebuild {
                    palette_cache = Some((
                        width,
                        height,
                        PaletteDraw::rebuild(&device, width, height, palette::swatches()),
                    ));
                }
                if let Some((_, _, ref pd)) = palette_cache {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    pass.set_pipeline(&palette_pipeline.pipeline);
                    pass.set_vertex_buffer(0, pd.vertex_buffer.slice(..));
                    pass.set_index_buffer(pd.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..pd.index_count, 0, 0..1);
                }
            }

            queue.submit([encoder.finish()]);
            frame.present();
        }
    }
}

fn select_with_editor_camera(
    game_state: &mut GameState,
    models: &[VoxelModel],
    id: u8,
    _camera: &mut Camera,
    fly_cam: &mut FlyCamera,
    app_mode: AppMode,
) {
    game_state.select_structure(Some(id));
    if app_mode == AppMode::VoxelEditor {
        *fly_cam = FlyCamera::for_voxel_model(
            models[id as usize].dim,
            fly_cam.viewport_width,
            fly_cam.viewport_height,
        );
    }
}

fn rebuild_mesh(device: &wgpu::Device, meshes: &mut [Mesh], models: &[VoxelModel], index: usize) {
    let (verts, indices) = models[index].to_mesh();
    meshes[index] = Mesh::from_voxel_mesh(device, &verts, &indices);
}

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
    let clip_near = glam::Vec4::new(ndc_x, ndc_y, -1.0, 1.0);
    let world_near = view_proj_inv * clip_near;
    let world_near = world_near / world_near.w;
    let ray_origin = glam::Vec3::new(world_near.x, world_near.y, world_near.z);
    let view_dir = (camera.target - camera.eye).normalize();
    let t = -ray_origin.y / view_dir.y;
    let ground = ray_origin + t * view_dir;
    let tx = (ground.x / TILE_SIZE).floor() as i32;
    let tz = (ground.z / TILE_SIZE).floor() as i32;
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
