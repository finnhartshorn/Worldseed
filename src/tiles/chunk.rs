use super::{constants::*, types::*};
use bevy::prelude::*;
use bevy::sprite_render::{TileData, TilemapChunkTileData};

/// Component marking a chunk entity with its position
/// This component is attached to each layer entity
#[derive(Component, Debug, Clone, Copy)]
pub struct Chunk {
    pub position: ChunkPos,
    pub layer: usize,
}

impl Chunk {
    pub fn new(position: ChunkPos) -> Self {
        Self {
            position,
            layer: LAYER_GROUND,
        }
    }

    pub fn with_layer(position: ChunkPos, layer: usize) -> Self {
        Self { position, layer }
    }
}

/// Marker component for chunks that have been modified and need saving
#[derive(Component, Debug)]
pub struct DirtyChunk;

/// Chunk data storage (separate from the visual tilemap)
/// Now stores multiple layers of tiles
#[derive(Debug, Clone)]
pub struct ChunkData {
    pub position: ChunkPos,
    /// Array of tile layers [LAYER_GROUND, LAYER_DECORATION, LAYER_OVERLAY]
    /// Each layer is a CHUNK_AREA array of tile IDs
    pub layers: Box<[[TileId; CHUNK_AREA]; NUM_LAYERS]>,
}

impl ChunkData {
    /// Create a new chunk with all layers filled with a specific tile
    pub fn filled(position: ChunkPos, tile_id: TileId) -> Self {
        Self {
            position,
            layers: Box::new([[tile_id; CHUNK_AREA]; NUM_LAYERS]),
        }
    }

    /// Create a new chunk with specific tile for each layer
    pub fn filled_layers(position: ChunkPos, layer_tiles: [TileId; NUM_LAYERS]) -> Self {
        let mut layers = Box::new([[TILE_EMPTY; CHUNK_AREA]; NUM_LAYERS]);
        for (layer_idx, &tile_id) in layer_tiles.iter().enumerate() {
            layers[layer_idx] = [tile_id; CHUNK_AREA];
        }
        Self { position, layers }
    }

    /// Create an empty chunk (all layers TILE_EMPTY)
    pub fn empty(position: ChunkPos) -> Self {
        Self::filled(position, TILE_EMPTY)
    }

    /// Get tile at local chunk coordinates (0-31, 0-31) for a specific layer
    pub fn get_tile(&self, layer: usize, local_x: usize, local_y: usize) -> Option<TileId> {
        if layer >= NUM_LAYERS || local_x >= CHUNK_SIZE || local_y >= CHUNK_SIZE {
            return None;
        }
        let index = local_y * CHUNK_SIZE + local_x;
        Some(self.layers[layer][index])
    }

    /// Set tile at local chunk coordinates (0-31, 0-31) for a specific layer
    pub fn set_tile(&mut self, layer: usize, local_x: usize, local_y: usize, tile_id: TileId) -> bool {
        if layer >= NUM_LAYERS || local_x >= CHUNK_SIZE || local_y >= CHUNK_SIZE {
            return false;
        }
        let index = local_y * CHUNK_SIZE + local_x;
        self.layers[layer][index] = tile_id;
        true
    }

    /// Convert a specific layer of ChunkData to Bevy's TilemapChunkTileData
    pub fn layer_to_tilemap_data(&self, layer: usize) -> Vec<Option<TileData>> {
        if layer >= NUM_LAYERS {
            return vec![None; CHUNK_AREA];
        }

        self.layers[layer]
            .iter()
            .map(|&tile_id| {
                if tile_id == TILE_EMPTY {
                    None
                } else {
                    // Subtract 1 because TILE_EMPTY is 0, but tileset indices start at 0
                    Some(TileData::from_tileset_index((tile_id - 1) as u16))
                }
            })
            .collect()
    }

    /// Convert ChunkData to Bevy's TilemapChunkTileData (legacy - returns ground layer)
    #[deprecated(note = "Use layer_to_tilemap_data instead to specify which layer")]
    pub fn to_tilemap_data(&self) -> Vec<Option<TileData>> {
        self.layer_to_tilemap_data(LAYER_GROUND)
    }

    /// Create ChunkData from Bevy's TilemapChunkTileData for a single layer
    pub fn from_tilemap_data_single_layer(
        position: ChunkPos,
        layer: usize,
        tile_data: &[Option<TileData>],
    ) -> Option<Self> {
        if tile_data.len() != CHUNK_AREA || layer >= NUM_LAYERS {
            return None;
        }

        let mut chunk = Self::empty(position);
        for (i, data) in tile_data.iter().enumerate() {
            chunk.layers[layer][i] = match data {
                Some(tile) => {
                    // Add 1 because tileset indices start at 0, but our TILE_EMPTY is 0
                    (tile.tileset_index + 1) as TileId
                }
                None => TILE_EMPTY,
            };
        }

        Some(chunk)
    }

    /// Create ChunkData from Bevy's TilemapChunkTileData (legacy - sets ground layer only)
    #[deprecated(note = "Use from_tilemap_data_single_layer instead to specify which layer")]
    pub fn from_tilemap_data(
        position: ChunkPos,
        tile_data: &[Option<TileData>],
    ) -> Option<Self> {
        Self::from_tilemap_data_single_layer(position, LAYER_GROUND, tile_data)
    }
}

/// Helper functions for chunk coordinate conversions
pub mod coords {
    use super::*;

    /// Convert world position to chunk position
    pub fn world_to_chunk(world_pos: Vec2) -> ChunkPos {
        ChunkPos::from_world(world_pos, CHUNK_PIXEL_SIZE)
    }

    /// Convert tile position to chunk position
    pub fn tile_to_chunk(tile_pos: IVec2) -> ChunkPos {
        ChunkPos::from_tile(tile_pos, CHUNK_SIZE_I32)
    }

    /// Convert world position to local tile position within chunk (0-31, 0-31)
    pub fn world_to_local_tile(world_pos: Vec2) -> (usize, usize) {
        let tile_x = (world_pos.x / TILE_SIZE).floor() as i32;
        let tile_y = (world_pos.y / TILE_SIZE).floor() as i32;
        let local_x = tile_x.rem_euclid(CHUNK_SIZE_I32) as usize;
        let local_y = tile_y.rem_euclid(CHUNK_SIZE_I32) as usize;
        (local_x, local_y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_data_get_set() {
        let mut chunk = ChunkData::empty(ChunkPos::new(0, 0));

        // Set and get tile on ground layer
        assert!(chunk.set_tile(LAYER_GROUND, 5, 10, TILE_GRASS));
        assert_eq!(chunk.get_tile(LAYER_GROUND, 5, 10), Some(TILE_GRASS));

        // Set tile on decoration layer
        assert!(chunk.set_tile(LAYER_DECORATION, 5, 10, TILE_DIRT));
        assert_eq!(chunk.get_tile(LAYER_DECORATION, 5, 10), Some(TILE_DIRT));

        // Ground layer should still be grass
        assert_eq!(chunk.get_tile(LAYER_GROUND, 5, 10), Some(TILE_GRASS));

        // Out of bounds - invalid coordinates
        assert!(!chunk.set_tile(LAYER_GROUND, 32, 0, TILE_DIRT));
        assert_eq!(chunk.get_tile(LAYER_GROUND, 32, 0), None);

        // Out of bounds - invalid layer
        assert!(!chunk.set_tile(NUM_LAYERS, 5, 10, TILE_DIRT));
        assert_eq!(chunk.get_tile(NUM_LAYERS, 5, 10), None);
    }

    #[test]
    fn test_world_to_local_tile() {
        // Position in first chunk, middle tile
        let (x, y) = coords::world_to_local_tile(Vec2::new(128.0, 128.0));
        assert_eq!((x, y), (16, 16)); // 128 / 8 = 16

        // Position in negative chunk
        let (x, y) = coords::world_to_local_tile(Vec2::new(-8.0, -8.0));
        assert_eq!((x, y), (31, 31)); // Wraps to last tile
    }
}
