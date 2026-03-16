//! Orthographic camera, view-projection matrix

use glam::{Mat4, Vec3};

/// Simple orthographic camera looking at the tile grid.
/// Higher `zoom` = more zoomed in (see fewer world units).
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
}
