//! Editor color swatches (same distribution as `color_test` structure palette).

use glam::Vec4;
use std::sync::OnceLock;

/// Number of swatches in the palette grid (16×16).
pub const SWATCH_COUNT: usize = 256;

static SWATCHES: OnceLock<[Vec4; SWATCH_COUNT]> = OnceLock::new();

fn build_swatches() -> [Vec4; SWATCH_COUNT] {
    let mut out = [Vec4::ZERO; SWATCH_COUNT];
    out[0] = Vec4::from_array([0.0, 0.0, 0.0, 1.0]);
    out[1] = Vec4::from_array([1.0, 1.0, 1.0, 1.0]);
    out[2] = Vec4::from_array([1.0, 0.0, 0.0, 1.0]);
    out[3] = Vec4::from_array([0.0, 1.0, 0.0, 1.0]);
    out[4] = Vec4::from_array([0.0, 0.0, 1.0, 1.0]);
    out[5] = Vec4::from_array([0.0, 1.0, 1.0, 1.0]);
    out[6] = Vec4::from_array([1.0, 0.0, 1.0, 1.0]);
    out[7] = Vec4::from_array([1.0, 1.0, 0.0, 1.0]);
    for i in 1..=8 {
        let g = i as f32 / 9.0;
        out[7 + i] = Vec4::from_array([g, g, g, 1.0]);
    }
    for i in 0..16 {
        let g = i as f32 / 15.0;
        out[16 + i] = Vec4::from_array([g, g, g, 1.0]);
    }
    for row in 0..14 {
        let sat = 1.0 - (row as f32 * 0.06);
        for col in 0..16 {
            let hue = col as f32 / 16.0;
            let (r, g, b) = hsl_to_rgb(hue, sat, 0.5);
            out[32 + row * 16 + col] = Vec4::from_array([r, g, b, 1.0]);
        }
    }
    out
}

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
        let t = if t < 0.0 {
            t + 1.0
        } else if t > 1.0 {
            t - 1.0
        } else {
            t
        };
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

/// All editor palette colors (256 entries), built once.
pub fn swatches() -> &'static [Vec4; SWATCH_COUNT] {
    SWATCHES.get_or_init(build_swatches)
}
