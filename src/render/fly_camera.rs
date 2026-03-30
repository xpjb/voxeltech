//! First-person fly camera (perspective) for voxel editor — Minecraft-style look + WASD/Space/Shift.

use glam::{IVec3, Mat4, Vec3, Vec4};
use sdl3::keyboard::KeyboardState;
use sdl3::keyboard::Scancode;

#[derive(Clone, Copy)]
pub struct FlyCamera {
    pub position: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub fov_y: f32,
    pub near: f32,
    pub far: f32,
}

impl FlyCamera {
    /// Start just outside the model AABB (local 0..dim), isometric-style direction toward center.
    pub fn for_voxel_model(dim: IVec3, viewport_width: f32, viewport_height: f32) -> Self {
        let min_b = Vec3::ZERO;
        let max_b = Vec3::new(dim.x as f32, dim.y as f32, dim.z as f32);
        let center = (min_b + max_b) * 0.5;
        // Same diagonal as the old ortho editor: ~45° yaw, moderate pitch.
        let angle_h: f32 = 0.785;
        let angle_v: f32 = 0.5;
        let dir = Vec3::new(
            angle_h.cos() * angle_v.cos(),
            angle_v.sin(),
            angle_h.sin() * angle_v.cos(),
        )
        .normalize();
        let t_exit = distance_to_exit_aabb(center, dir, min_b, max_b);
        let margin = 3.5_f32;
        let eye = center + dir * (t_exit + margin);
        Self::from_eye_target(eye, center, viewport_width, viewport_height)
    }

    fn from_eye_target(eye: Vec3, target: Vec3, vw: f32, vh: f32) -> Self {
        let forward = (target - eye).normalize();
        let pitch = forward.y.clamp(-0.999, 0.999).asin();
        let yaw = forward.x.atan2(-forward.z);
        Self {
            position: eye,
            yaw,
            pitch,
            viewport_width: vw,
            viewport_height: vh,
            fov_y: 60_f32.to_radians(),
            near: 0.05,
            far: 500.0,
        }
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.viewport_width = width;
        self.viewport_height = height;
    }

    pub fn forward(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        Vec3::new(sy * cp, sp, -cy * cp)
    }

    pub fn add_look(&mut self, dx: f32, dy: f32) {
        const SENS: f32 = 0.002;
        // +dx moves view right (standard FPS; negated relative to previous).
        self.yaw += dx * SENS;
        self.pitch -= dy * SENS;
        self.pitch = self.pitch.clamp(-1.55_f32, 1.55_f32);
    }

    pub fn fly_tick(&mut self, kb: &KeyboardState<'_>, dt: f32) {
        let f = self.forward();
        let forward_flat = Vec3::new(f.x, 0.0, f.z).normalize_or_zero();
        let right = f.cross(Vec3::Y).normalize_or_zero();
        let speed = 22.0_f32 * dt;
        let mut v = Vec3::ZERO;
        if kb.is_scancode_pressed(Scancode::W) {
            v += forward_flat;
        }
        if kb.is_scancode_pressed(Scancode::S) {
            v -= forward_flat;
        }
        if kb.is_scancode_pressed(Scancode::D) {
            v += right;
        }
        if kb.is_scancode_pressed(Scancode::A) {
            v -= right;
        }
        if kb.is_scancode_pressed(Scancode::Space) {
            v += Vec3::Y;
        }
        if kb.is_scancode_pressed(Scancode::LShift) || kb.is_scancode_pressed(Scancode::RShift) {
            v -= Vec3::Y;
        }
        if v.length_squared() > 1e-8 {
            self.position += v.normalize() * speed;
        }
    }

    pub fn view_projection_matrix(&self) -> Mat4 {
        let f = self.forward();
        let view = Mat4::look_at_rh(self.position, self.position + f, Vec3::Y);
        let aspect = (self.viewport_width / self.viewport_height.max(1.0)).max(0.001);
        let proj = Mat4::perspective_rh(self.fov_y, aspect, self.near, self.far);
        proj * view
    }

    /// Pinhole ray matching [`Mat4::perspective_rh`] (WebGPU NDC z ∈ [0,1]), not OpenGL z ∈ [−1,1].
    /// Inverting `view * proj` with z = −1 / +1 skews off-center rays; that felt like mirrored X picking.
    pub fn ray_from_pixel(&self, screen_x: f32, screen_y: f32, width: u32, height: u32) -> (Vec3, Vec3) {
        let w = width.max(1) as f32;
        let h = height.max(1) as f32;
        let ndc_x = (screen_x / w) * 2.0 - 1.0;
        let ndc_y = 1.0 - (screen_y / h) * 2.0;
        // Use the same width/height as NDC (caller's window size), not cached viewport:
        // `fly_cam.resize` runs after input each frame, so aspect would lag on resize.
        let aspect = (w / h.max(1.0)).max(0.001);
        let tan_half = (self.fov_y * 0.5).tan();
        // View space: −Z forward (same as glam look_at_rh + perspective_rh).
        let dir_view = Vec3::new(
            ndc_x * tan_half * aspect,
            ndc_y * tan_half,
            -1.0,
        )
        .normalize();
        let f = self.forward().normalize();
        let view = Mat4::look_at_rh(self.position, self.position + f, Vec3::Y);
        let inv_view = view.inverse();
        let dw = inv_view * Vec4::new(dir_view.x, dir_view.y, dir_view.z, 0.0);
        let dir_world = Vec3::new(dw.x, dw.y, dw.z).normalize_or_zero();
        (self.position, dir_world)
    }
}

/// Shortest `t >= 0` with `origin + t * dir` on an AABB face (`dir` unit not required).
fn distance_to_exit_aabb(origin: Vec3, dir: Vec3, min_b: Vec3, max_b: Vec3) -> f32 {
    let mut t_min = f32::MAX;
    for i in 0..3 {
        let o = origin[i];
        let d = dir[i];
        if d > 1e-6 {
            t_min = t_min.min((max_b[i] - o) / d);
        } else if d < -1e-6 {
            t_min = t_min.min((min_b[i] - o) / d);
        }
    }
    if !t_min.is_finite() || t_min < 0.0 {
        max_b.x.max(max_b.y).max(max_b.z)
    } else {
        t_min
    }
}
