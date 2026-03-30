//! Orthographic camera, view-projection matrix

use glam::{IVec3, Mat4, Vec3, Vec4};

/// Simple orthographic camera looking at the tile grid.
/// Higher `zoom` = more zoomed in (see fewer world units).
#[derive(Clone, Copy)]
pub struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub near: f32,
    pub far: f32,
    pub zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        let center = Vec3::new(256.0, 0.0, 256.0);
        let dist: f32 = 220.0;
        let angle_h: f32 = 0.785; // 45° diagonal
        let angle_v: f32 = 0.615; // ~35° elevation
        let eye = center
            + Vec3::new(
                angle_h.cos() * angle_v.cos() * dist,
                angle_v.sin() * dist,
                angle_h.sin() * angle_v.cos() * dist,
            );
        Self {
            eye,
            target: center,
            up: Vec3::Y,
            viewport_width: 1024.0,
            viewport_height: 768.0,
            near: 0.1,
            far: 1000.0,
            zoom: 8.0,
        }
    }
}

impl Camera {
    /// For --single mode: isometric view centered on one structure at grid center.
    /// Same ~35° elevation + 45° azimuth as default for classic isometric look.
    pub fn single_structure() -> Self {
        let center = Vec3::new(264.0, 8.0, 264.0); // Center of tile (16,16) structure
        let dist: f32 = 100.0; // Closer for single structure
        let angle_h: f32 = 0.785; // 45° diagonal (isometric)
        let angle_v: f32 = 0.615; // ~35° from horizontal
        let eye = center
            + Vec3::new(
                angle_h.cos() * angle_v.cos() * dist,
                angle_v.sin() * dist,
                angle_h.sin() * angle_v.cos() * dist,
            );
        Self {
            eye,
            target: center,
            up: Vec3::Y, // World up for proper isometric
            viewport_width: 1024.0,
            viewport_height: 768.0,
            near: 0.1,
            far: 1000.0,
            zoom: 24.0,
        }
    }

    /// Isometric-style view centered on a voxel model at the origin (local space 0..dim).
    pub fn for_voxel_editor(dim: IVec3, viewport_width: f32, viewport_height: f32) -> Self {
        let cx = dim.x as f32 * 0.5;
        let cy = dim.y as f32 * 0.5;
        let cz = dim.z as f32 * 0.5;
        let center = Vec3::new(cx, cy, cz);
        let dist: f32 = 80.0 + (dim.y as f32) * 4.0;
        let angle_h: f32 = 0.785;
        let angle_v: f32 = 0.615;
        let eye = center
            + Vec3::new(
                angle_h.cos() * angle_v.cos() * dist,
                angle_v.sin() * dist,
                angle_h.sin() * angle_v.cos() * dist,
            );
        let mut c = Self {
            eye,
            target: center,
            up: Vec3::Y,
            viewport_width,
            viewport_height,
            near: 0.1,
            far: 1000.0,
            zoom: 22.0,
        };
        c.resize(viewport_width, viewport_height);
        c
    }

    pub fn view_projection_matrix(&self) -> Mat4 {
        let view = Mat4::look_at_rh(self.eye, self.target, self.up);
        let w = self.viewport_width / self.zoom;
        let h = self.viewport_height / self.zoom;
        let proj = Mat4::orthographic_rh(-w / 2.0, w / 2.0, -h / 2.0, h / 2.0, self.near, self.far);
        proj * view
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.viewport_width = width;
        self.viewport_height = height;
    }

    /// Rotate the camera 90° around the target (Y axis). Positive = right, negative = left.
    pub fn rotate_around_target(&mut self, delta_angle: f32) {
        let offset = self.eye - self.target;
        let c = delta_angle.cos();
        let s = delta_angle.sin();
        let new_x = offset.x * c - offset.z * s;
        let new_z = offset.x * s + offset.z * c;
        self.eye = self.target + glam::Vec3::new(new_x, offset.y, new_z);
    }

    /// Orthographic: parallel view direction from eye toward target. Ray origin is the unprojected near-plane point for the pixel.
    pub fn ray_from_pixel(&self, screen_x: f32, screen_y: f32, width: u32, height: u32) -> (Vec3, Vec3) {
        let w = width.max(1) as f32;
        let h = height.max(1) as f32;
        let ndc_x = (screen_x / w) * 2.0 - 1.0;
        let ndc_y = 1.0 - (screen_y / h) * 2.0;
        let inv = self.view_projection_matrix().inverse();
        let clip_near = Vec4::new(ndc_x, ndc_y, -1.0, 1.0);
        let world_near = inv * clip_near;
        let world_near = world_near / world_near.w;
        let origin = Vec3::new(world_near.x, world_near.y, world_near.z);
        let dir = (self.target - self.eye).normalize();
        (origin, dir)
    }
}
