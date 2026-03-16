use glam::IVec2;
use std::collections::HashMap;

/// Structure ID 0-9 for the 10 available structures
pub type StructureId = u8;

/// A placed structure on the grid
#[derive(Clone, Copy, Debug)]
pub struct Placement {
    pub tile: IVec2,
    pub structure_id: StructureId,
}

/// Game state: grid of structures, selected structure for building
#[derive(Clone, Default)]
pub struct GameState {
    /// Map from tile position to placed structure
    pub grid: HashMap<IVec2, Placement>,
    /// Currently selected structure (0-9), None = no selection
    pub selected_structure: Option<StructureId>,
}

impl GameState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn place(&mut self, tile: IVec2, structure_id: StructureId) {
        self.grid.insert(
            tile,
            Placement {
                tile,
                structure_id,
            },
        );
    }

    pub fn remove(&mut self, tile: IVec2) -> Option<Placement> {
        self.grid.remove(&tile)
    }

    pub fn get(&self, tile: IVec2) -> Option<&Placement> {
        self.grid.get(&tile)
    }

    pub fn select_structure(&mut self, id: Option<StructureId>) {
        self.selected_structure = id;
    }
}
