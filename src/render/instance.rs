//! Instance buffer for placed structures - reusable buffer for mat4 per instance

use glam::Mat4;

const MAX_INSTANCES: usize = 1024;

pub struct InstanceBuffer {
    pub buffer: wgpu::Buffer,
}

impl InstanceBuffer {
    pub fn new(device: &wgpu::Device) -> Self {
        let size = (MAX_INSTANCES * 64) as u64; // mat4 = 64 bytes
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instance buffer"),
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self { buffer }
    }

    pub fn write(&self, queue: &wgpu::Queue, instances: &[Mat4]) {
        if instances.is_empty() {
            return;
        }
        let data: Vec<[f32; 16]> = instances.iter().map(|m| m.to_cols_array()).collect();
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&data));
    }

    /// Write instances at byte offset (for batched upload). Each Mat4 = 64 bytes.
    pub fn write_at(&self, queue: &wgpu::Queue, offset_bytes: u64, instances: &[Mat4]) {
        if instances.is_empty() {
            return;
        }
        let data: Vec<[f32; 16]> = instances.iter().map(|m| m.to_cols_array()).collect();
        queue.write_buffer(&self.buffer, offset_bytes, bytemuck::cast_slice(&data));
    }
}
