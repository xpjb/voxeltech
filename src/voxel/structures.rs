//! 10 pre-built voxel models for structures (16x16 footprint each)

use glam::{IVec3, Vec4};

use super::model::VoxelModel;

pub const STRUCTURE_COUNT: usize = 10;

/// Get voxel model for structure id 0-9
pub fn get_structure(id: u8) -> VoxelModel {
    match id {
        0 => color_test(),
        1 => tower(),
        2 => barn(),
        3 => factory(),
        4 => house(),
        5 => silo(),
        6 => warehouse(),
        7 => guard_post(),
        8 => cottage(),
        9 => fort(),
        _ => small_hut(),
    }
}

// Color palette
const WOOD: Vec4 = Vec4::from_array([0.55, 0.35, 0.2, 1.0]);
const STONE: Vec4 = Vec4::from_array([0.5, 0.5, 0.55, 1.0]);
const RED_ROOF: Vec4 = Vec4::from_array([0.7, 0.2, 0.15, 1.0]);
#[allow(dead_code)]
const DARK_ROOF: Vec4 = Vec4::from_array([0.2, 0.15, 0.1, 1.0]);
const BRICK: Vec4 = Vec4::from_array([0.65, 0.35, 0.3, 1.0]);
const CONCRETE: Vec4 = Vec4::from_array([0.6, 0.58, 0.55, 1.0]);
const METAL: Vec4 = Vec4::from_array([0.5, 0.52, 0.55, 1.0]);
const DOOR: Vec4 = Vec4::from_array([0.4, 0.25, 0.15, 1.0]);
const THATCH: Vec4 = Vec4::from_array([0.6, 0.45, 0.2, 1.0]);
const CREAM: Vec4 = Vec4::from_array([0.95, 0.9, 0.8, 1.0]);

/// 16x16 color test swatches: RGB, CMY, W, B first; then grayscale; then hue wheel variations.
fn color_test() -> VoxelModel {
    let dim = IVec3::new(16, 1, 16);
    let mut m = VoxelModel::new(dim);
    let colors = color_test_palette();
    for i in 0..256 {
        let x = (i % 16) as i32;
        let z = (i / 16) as i32;
        m.set(x, 0, z, colors[i]);
    }
    m
}

fn color_test_palette() -> [Vec4; 256] {
    let mut out = [Vec4::ZERO; 256];
    // 0-7: Black, White, R, G, B, C, M, Y
    out[0] = Vec4::from_array([0.0, 0.0, 0.0, 1.0]);
    out[1] = Vec4::from_array([1.0, 1.0, 1.0, 1.0]);
    out[2] = Vec4::from_array([1.0, 0.0, 0.0, 1.0]);
    out[3] = Vec4::from_array([0.0, 1.0, 0.0, 1.0]);
    out[4] = Vec4::from_array([0.0, 0.0, 1.0, 1.0]);
    out[5] = Vec4::from_array([0.0, 1.0, 1.0, 1.0]);
    out[6] = Vec4::from_array([1.0, 0.0, 1.0, 1.0]);
    out[7] = Vec4::from_array([1.0, 1.0, 0.0, 1.0]);
    // 8-15: grayscale steps
    for i in 1..=8 {
        let g = i as f32 / 9.0;
        out[7 + i] = Vec4::from_array([g, g, g, 1.0]);
    }
    // 16-31: full grayscale ramp (16 steps)
    for i in 0..16 {
        let g = i as f32 / 15.0;
        out[16 + i] = Vec4::from_array([g, g, g, 1.0]);
    }
    // 32-255: hue wheel (14 rows × 16 hues) at varying saturation, L=0.5 for vibrancy
    for row in 0..14 {
        let sat = 1.0 - (row as f32 * 0.06); // 100% down to 16%
        for col in 0..16 {
            let hue = col as f32 / 16.0;
            let (r, g, b) = hsl_to_rgb(hue, sat, 0.5);
            out[32 + row * 16 + col] = Vec4::from_array([r, g, b, 1.0]);
        }
    }
    out
}

/// Hue in [0,1), S and L in [0,1]. Returns linear RGB.
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s <= 0.0 {
        return (l, l, l);
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let hue_to_rgb = |t: f32| {
        let t = if t < 0.0 { t + 1.0 } else if t > 1.0 { t - 1.0 } else { t };
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };
    let r = hue_to_rgb(h + 1.0 / 3.0);
    let g = hue_to_rgb(h);
    let b = hue_to_rgb(h - 1.0 / 3.0);
    (r, g, b)
}

fn small_hut() -> VoxelModel {
    let dim = IVec3::new(16, 5, 16);
    let mut m = VoxelModel::new(dim);
    for x in 2..14 {
        for z in 2..14 {
            for y in 0..3 {
                m.set(x, y, z, WOOD);
            }
        }
    }
    for x in 3..13 {
        for z in 3..13 {
            let peak = 2 + (6i32 - (x - 6i32).abs().min((z - 6i32).abs())) / 2;
            for y in 3..peak.min(5) {
                m.set(x, y, z, RED_ROOF);
            }
        }
    }
    m.set(8, 0, 7, DOOR);
    m
}

fn tower() -> VoxelModel {
    let dim = IVec3::new(16, 14, 16);
    let mut m = VoxelModel::new(dim);
    let (cx, cz) = (7, 7);
    let r = 4;
    for x in (cx - r)..=(cx + r) {
        for z in (cz - r)..=(cz + r) {
            if (x - cx) * (x - cx) + (z - cz) * (z - cz) <= r * r + 2 {
                for y in 0..12 {
                    m.set(x, y, z, STONE);
                }
                for y in 12..14 {
                    if (x - cx) % 2 == 0 || (z - cz) % 2 == 0 {
                        m.set(x, y, z, STONE);
                    }
                }
            }
        }
    }
    m
}

fn barn() -> VoxelModel {
    let dim = IVec3::new(16, 7, 16);
    let mut m = VoxelModel::new(dim);
    for x in 1..15 {
        for z in 1..15 {
            for y in 0..4 {
                m.set(x, y, z, BRICK);
            }
        }
    }
    for x in 2..14 {
        for z in 2..14 {
            for y in 4..7 {
                let slope = (x - 2).min(13 - x).min((z - 2).min(13 - z));
                if slope >= 0 && y < 4 + (slope / 2) + 1 {
                    m.set(x, y, z, RED_ROOF);
                }
            }
        }
    }
    m.set(7, 0, 5, DOOR);
    m.set(8, 0, 5, DOOR);
    m
}

fn factory() -> VoxelModel {
    let dim = IVec3::new(16, 9, 16);
    let mut m = VoxelModel::new(dim);
    for x in 1..15 {
        for z in 1..15 {
            for y in 0..6 {
                m.set(x, y, z, CONCRETE);
            }
        }
    }
    for x in 5..7 {
        for z in 5..11 {
            for y in 6..9 {
                m.set(x, y, z, METAL);
            }
        }
    }
    for x in 9..11 {
        for z in 5..11 {
            for y in 6..9 {
                m.set(x, y, z, METAL);
            }
        }
    }
    m
}

fn house() -> VoxelModel {
    let dim = IVec3::new(16, 7, 16);
    let mut m = VoxelModel::new(dim);
    for x in 2..14 {
        for z in 2..14 {
            for y in 0..4 {
                m.set(x, y, z, CREAM);
            }
        }
    }
    for x in 3..13 {
        for z in 3..13 {
            for y in 4..7 {
                let dist_from_peak = (x as i32 - 8).abs().min((x as i32 - 7).abs());
                if y <= 4 + (3 - dist_from_peak.min(3)).max(0) {
                    m.set(x, y, z, RED_ROOF);
                }
            }
        }
    }
    m.set(8, 0, 7, DOOR);
    m.set(7, 2, 5, CREAM);
    m.set(8, 2, 5, CREAM);
    m
}

fn silo() -> VoxelModel {
    let dim = IVec3::new(16, 11, 16);
    let mut m = VoxelModel::new(dim);
    let (cx, cz) = (7, 7);
    let r = 5;
    for x in (cx - r)..=(cx + r) {
        for z in (cz - r)..=(cz + r) {
            if (x - cx) * (x - cx) + (z - cz) * (z - cz) <= r * r + 1 {
                for y in 0..9 {
                    m.set(x, y, z, CONCRETE);
                }
                for y in 9..11 {
                    let d = (x - cx) * (x - cx) + (z - cz) * (z - cz);
                    if d <= (r - 1) * (r - 1) + 2 {
                        m.set(x, y, z, CONCRETE);
                    }
                }
            }
        }
    }
    m
}

fn warehouse() -> VoxelModel {
    let dim = IVec3::new(16, 7, 16);
    let mut m = VoxelModel::new(dim);
    for x in 0..16 {
        for z in 0..16 {
            for y in 0..6 {
                m.set(x, y, z, METAL);
            }
        }
    }
    for x in 1..15 {
        for z in 1..15 {
            m.set(x, 6, z, METAL);
        }
    }
    m.set(7, 0, 3, DOOR);
    m.set(8, 0, 3, DOOR);
    m
}

fn guard_post() -> VoxelModel {
    let dim = IVec3::new(16, 9, 16);
    let mut m = VoxelModel::new(dim);
    for x in 5..11 {
        for z in 5..11 {
            for y in 0..2 {
                m.set(x, y, z, STONE);
            }
        }
    }
    for x in 4..12 {
        for z in 4..12 {
            for y in 2..7 {
                m.set(x, y, z, WOOD);
            }
        }
    }
    for x in 5..11 {
        for z in 5..11 {
            m.set(x, 7, z, WOOD);
        }
    }
    for x in 6..10 {
        for z in 6..10 {
            for y in 8..9 {
                m.set(x, y, z, WOOD);
            }
        }
    }
    m
}

fn cottage() -> VoxelModel {
    let dim = IVec3::new(16, 5, 16);
    let mut m = VoxelModel::new(dim);
    for x in 3..13 {
        for z in 3..13 {
            for y in 0..3 {
                m.set(x, y, z, WOOD);
            }
        }
    }
    for x in 4..12 {
        for z in 4..12 {
            for y in 3..5 {
                m.set(x, y, z, THATCH);
            }
        }
    }
    m.set(8, 0, 6, DOOR);
    m
}

fn fort() -> VoxelModel {
    let dim = IVec3::new(16, 7, 16);
    let mut m = VoxelModel::new(dim);
    for x in 2..14 {
        for z in 2..14 {
            for y in 0..5 {
                m.set(x, y, z, STONE);
            }
        }
    }
    for x in 3..13 {
        for z in 3..13 {
            m.set(x, 5, z, STONE);
        }
    }
    for x in 2..14 {
        for z in 2..14 {
            if (x - 2) % 2 == 0 || (z - 2) % 2 == 0 {
                m.set(x, 6, z, STONE);
            }
        }
    }
    m.set(8, 0, 7, DOOR);
    m
}
