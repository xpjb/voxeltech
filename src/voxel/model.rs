//! VoxelModel struct and mesh generation from voxel data to vertex/index buffers

use glam::{IVec3, Vec3, Vec4};

/// RGBA colors per voxel. Layout: linearized [z][y][x], length = dim.x * dim.y * dim.z
/// Alpha < 0.5 is considered air (no mesh generated)
pub struct VoxelModel {
    pub data: Vec<Vec4>,
    pub dim: IVec3,
}

impl VoxelModel {
    pub fn new(dim: IVec3) -> Self {
        let len = (dim.x * dim.y * dim.z) as usize;
        let mut data = Vec::with_capacity(len);
        data.resize(len, Vec4::ZERO);
        Self { data, dim }
    }

    /// Linear index from x,y,z. Layout: [z][y][x]
    fn index(&self, x: i32, y: i32, z: i32) -> usize {
        (z * self.dim.x * self.dim.y + y * self.dim.x + x) as usize
    }

    pub fn get(&self, x: i32, y: i32, z: i32) -> Option<Vec4> {
        if x < 0 || x >= self.dim.x || y < 0 || y >= self.dim.y || z < 0 || z >= self.dim.z {
            return None;
        }
        let c = self.data[self.index(x, y, z)];
        if c.w >= 0.5 {
            Some(c)
        } else {
            None
        }
    }

    pub fn is_solid(&self, x: i32, y: i32, z: i32) -> bool {
        self.get(x, y, z).is_some()
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, color: Vec4) {
        if x >= 0 && x < self.dim.x && y >= 0 && y < self.dim.y && z >= 0 && z < self.dim.z {
            let idx = (z * self.dim.x * self.dim.y + y * self.dim.x + x) as usize;
            self.data[idx] = color;
        }
    }

    /// Convert voxel model to triangle mesh.
    /// Only emits faces that have no adjacent solid voxel (face culling).
    /// Vertex format: position (Vec3), normal (Vec3), color (Vec4)
    pub fn to_mesh(&self) -> (Vec<VoxelVertex>, Vec<u32>) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut vertex_offset = 0u32;

        for z in 0..self.dim.z {
            for y in 0..self.dim.y {
                for x in 0..self.dim.x {
                    if let Some(color) = self.get(x, y, z) {
                        let pos = Vec3::new(x as f32, y as f32, z as f32);

                        // -X face
                        if !self.is_solid(x - 1, y, z) {
                            let n = Vec3::new(-1.0, 0.0, 0.0);
                            add_quad(&mut vertices, &mut indices, &mut vertex_offset, pos, n, color, 0);
                        }
                        // +X face
                        if !self.is_solid(x + 1, y, z) {
                            let n = Vec3::new(1.0, 0.0, 0.0);
                            add_quad(&mut vertices, &mut indices, &mut vertex_offset, pos, n, color, 1);
                        }
                        // -Y face
                        if !self.is_solid(x, y - 1, z) {
                            let n = Vec3::new(0.0, -1.0, 0.0);
                            add_quad(&mut vertices, &mut indices, &mut vertex_offset, pos, n, color, 2);
                        }
                        // +Y face
                        if !self.is_solid(x, y + 1, z) {
                            let n = Vec3::new(0.0, 1.0, 0.0);
                            add_quad(&mut vertices, &mut indices, &mut vertex_offset, pos, n, color, 3);
                        }
                        // -Z face
                        if !self.is_solid(x, y, z - 1) {
                            let n = Vec3::new(0.0, 0.0, -1.0);
                            add_quad(&mut vertices, &mut indices, &mut vertex_offset, pos, n, color, 4);
                        }
                        // +Z face
                        if !self.is_solid(x, y, z + 1) {
                            let n = Vec3::new(0.0, 0.0, 1.0);
                            add_quad(&mut vertices, &mut indices, &mut vertex_offset, pos, n, color, 5);
                        }
                    }
                }
            }
        }

        (vertices, indices)
    }
}

/// Vertex format for voxel mesh: color, position, normal.
/// Color first ensures Vec4 is 16-byte aligned (avoids GPU misreads).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct VoxelVertex {
    pub color: Vec4,
    pub position: Vec3,
    pub normal: Vec3,
}
unsafe impl bytemuck::Pod for VoxelVertex {}
unsafe impl bytemuck::Zeroable for VoxelVertex {}

/// Face axis: 0=-X, 1=+X, 2=-Y, 3=+Y, 4=-Z, 5=+Z
fn add_quad(
    vertices: &mut Vec<VoxelVertex>,
    indices: &mut Vec<u32>,
    vertex_offset: &mut u32,
    pos: Vec3,
    normal: Vec3,
    color: Vec4,
    face: u8,
) {
    let (v0, v1, v2, v3) = quad_verts(pos, face);
    let base = *vertex_offset;
    vertices.push(VoxelVertex { color, position: v0, normal });
    vertices.push(VoxelVertex { color, position: v1, normal });
    vertices.push(VoxelVertex { color, position: v2, normal });
    vertices.push(VoxelVertex { color, position: v3, normal });
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    *vertex_offset += 4;
}

fn quad_verts(pos: Vec3, face: u8) -> (Vec3, Vec3, Vec3, Vec3) {
    // Winding: CCW when viewed from outside (front face). All faces reversed for consistent visibility.
    let (a, b, c, d) = match face {
        0 => ((0, 0, 1), (0, 1, 1), (0, 1, 0), (0, 0, 0)), // -X (reversed for SW view)
        1 => ((1, 0, 0), (1, 1, 0), (1, 1, 1), (1, 0, 1)), // +X (reversed for SE view)
        2 => ((0, 0, 0), (1, 0, 0), (1, 0, 1), (0, 0, 1)), // -Y
        3 => ((0, 1, 1), (1, 1, 1), (1, 1, 0), (0, 1, 0)), // +Y
        4 => ((1, 1, 0), (0, 1, 0), (0, 0, 0), (1, 0, 0)), // -Z (reversed for SW view)
        _ => ((0, 1, 1), (1, 1, 1), (1, 0, 1), (0, 0, 1)), // +Z (reversed for SE view)
    };
    let to_v = |(x, y, z): (i32, i32, i32)| pos + Vec3::new(x as f32, y as f32, z as f32);
    (to_v(a), to_v(b), to_v(c), to_v(d))
}
