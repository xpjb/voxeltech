//! Ray vs voxel grid: first solid hit and previous air cell (for placement).

use glam::{IVec3, Vec3};

use crate::voxel::model::VoxelModel;

/// Solid voxel hit, and the air voxel stepped from when entering it (place block here for RMB).
#[derive(Clone, Copy, Debug)]
pub struct VoxelRayHit {
    pub solid: IVec3,
    /// Empty cell adjacent to the hit face (valid placement cell).
    pub air_before: IVec3,
}

/// Returns first solid along the ray inside `[0, dim)` unit cubes, or `None`.
pub fn raycast_voxels(model: &VoxelModel, origin: Vec3, dir: Vec3) -> Option<VoxelRayHit> {
    let dir = dir.normalize_or_zero();
    if dir.length_squared() < 1e-12 {
        return None;
    }

    let dim = model.dim;
    let max_b = Vec3::new(dim.x as f32, dim.y as f32, dim.z as f32);
    let t = intersect_aabb(origin, dir, Vec3::ZERO, max_b)?;
    let t_start = t.0.max(0.0);

    let p = origin + dir * (t_start + 1e-4);
    let mut x = p.x.floor() as i32;
    let mut y = p.y.floor() as i32;
    let mut z = p.z.floor() as i32;

    if x < 0 || x >= dim.x || y < 0 || y >= dim.y || z < 0 || z >= dim.z {
        return None;
    }

    let step_x = if dir.x >= 0.0 { 1 } else { -1 };
    let step_y = if dir.y >= 0.0 { 1 } else { -1 };
    let step_z = if dir.z >= 0.0 { 1 } else { -1 };

    let t_delta_x = if dir.x.abs() > 1e-8 {
        (1.0 / dir.x).abs()
    } else {
        f32::INFINITY
    };
    let t_delta_y = if dir.y.abs() > 1e-8 {
        (1.0 / dir.y).abs()
    } else {
        f32::INFINITY
    };
    let t_delta_z = if dir.z.abs() > 1e-8 {
        (1.0 / dir.z).abs()
    } else {
        f32::INFINITY
    };

    let boundary_x = if dir.x >= 0.0 {
        (x + 1) as f32
    } else {
        x as f32
    };
    let boundary_y = if dir.y >= 0.0 {
        (y + 1) as f32
    } else {
        y as f32
    };
    let boundary_z = if dir.z >= 0.0 {
        (z + 1) as f32
    } else {
        z as f32
    };

    let mut t_max_x = if dir.x.abs() > 1e-8 {
        (boundary_x - origin.x) / dir.x
    } else {
        f32::INFINITY
    };
    let mut t_max_y = if dir.y.abs() > 1e-8 {
        (boundary_y - origin.y) / dir.y
    } else {
        f32::INFINITY
    };
    let mut t_max_z = if dir.z.abs() > 1e-8 {
        (boundary_z - origin.z) / dir.z
    } else {
        f32::INFINITY
    };

    if t_max_x < t_start {
        t_max_x += t_delta_x;
    }
    if t_max_y < t_start {
        t_max_y += t_delta_y;
    }
    if t_max_z < t_start {
        t_max_z += t_delta_z;
    }

    let max_steps = (dim.x + dim.y + dim.z + 32) as u32;
    let mut prev_air: Option<IVec3> = None;

    for _ in 0..max_steps {
        if x < 0 || x >= dim.x || y < 0 || y >= dim.y || z < 0 || z >= dim.z {
            return None;
        }

        if model.is_solid(x, y, z) {
            let solid = IVec3::new(x, y, z);
            let air_before = prev_air.unwrap_or_else(|| air_cell_toward_camera(solid, dir, dim));
            return Some(VoxelRayHit { solid, air_before });
        }

        prev_air = Some(IVec3::new(x, y, z));

        if t_max_x < t_max_y {
            if t_max_x < t_max_z {
                x += step_x;
                t_max_x += t_delta_x;
            } else {
                z += step_z;
                t_max_z += t_delta_z;
            }
        } else if t_max_y < t_max_z {
            y += step_y;
            t_max_y += t_delta_y;
        } else {
            z += step_z;
            t_max_z += t_delta_z;
        }
    }

    None
}

/// When the ray starts inside the first solid with no recorded air step, pick adjacent air toward `-dir` (camera side).
fn air_cell_toward_camera(solid: IVec3, dir: Vec3, dim: IVec3) -> IVec3 {
    let ax = dir.x.abs();
    let ay = dir.y.abs();
    let az = dir.z.abs();
    let d = if ax >= ay && ax >= az {
        IVec3::new(-dir.x.signum() as i32, 0, 0)
    } else if ay >= az {
        IVec3::new(0, -dir.y.signum() as i32, 0)
    } else {
        IVec3::new(0, 0, -dir.z.signum() as i32)
    };
    let c = solid + d;
    if c.x >= 0 && c.x < dim.x && c.y >= 0 && c.y < dim.y && c.z >= 0 && c.z < dim.z {
        c
    } else {
        solid
    }
}

fn intersect_aabb(origin: Vec3, dir: Vec3, bmin: Vec3, bmax: Vec3) -> Option<(f32, f32)> {
    let mut t_min = f32::NEG_INFINITY;
    let mut t_max = f32::INFINITY;

    for i in 0..3 {
        let o = origin[i];
        let d = dir[i];
        let bn = bmin[i];
        let bx = bmax[i];
        if d.abs() < 1e-8 {
            if o < bn || o >= bx {
                return None;
            }
        } else {
            let mut t0 = (bn - o) / d;
            let mut t1 = (bx - o) / d;
            if t0 > t1 {
                std::mem::swap(&mut t0, &mut t1);
            }
            t_min = t_min.max(t0);
            t_max = t_max.min(t1);
        }
    }

    if t_min > t_max || t_max < 0.0 {
        None
    } else {
        Some((t_min, t_max))
    }
}
