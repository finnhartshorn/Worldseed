use super::types::TileId;

/// Tile registry for storing tile properties and metadata
/// Future: Add tile properties like walkability, durability, etc.
#[derive(Debug, Clone)]
pub struct TileRegistry {
    // Future fields:
    // pub tiles: HashMap<TileId, TileProperties>,
}

impl TileRegistry {
    pub fn new() -> Self {
        Self {
            // Initialize registry
        }
    }

    /// Check if a tile ID is valid
    pub fn is_valid_tile(&self, _tile_id: TileId) -> bool {
        // For now, all tile IDs are valid
        // Future: Check against registered tiles
        true
    }
}

impl Default for TileRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Future: Add TileProperties struct
// #[derive(Debug, Clone)]
// pub struct TileProperties {
//     pub name: String,
//     pub walkable: bool,
//     pub transparent: bool,
//     pub durability: u32,
// }
