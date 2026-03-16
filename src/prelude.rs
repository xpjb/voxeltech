//! Blanket re-exports for convenient `use crate::prelude::*`
//!
//! Add commonly used types here so modules can import with a single line.

// Game
pub use crate::game::state::{GameState, Placement, StructureId};

// Voxel
pub use crate::voxel::model::{VoxelModel, VoxelVertex};
pub use crate::voxel::structures::{get_structure, STRUCTURE_COUNT};

// Glam (often needed alongside our types)
pub use glam::{IVec2, IVec3, Mat4, Vec3, Vec4};
