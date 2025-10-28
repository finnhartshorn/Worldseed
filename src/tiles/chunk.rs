use super::{constants::*, types::*};
use bevy::prelude::*;
use bevy::sprite_render::{TileData, TilemapChunkTileData};

/// Component marking a chunk entity with its position
#[derive(Component, Debug, Clone, Copy)]
pub struct Chunk {
    pub position: ChunkPos,
}

impl Chunk {
    pub fn new(position: ChunkPos) -> Self {
        Self { position }
    }
}

/// Marker component for chunks that have been modified and need saving
#[derive(Component, Debug)]
pub struct DirtyChunk;

/// Chunk data storage (separate from the visual tilemap)
#[derive(Debug, Clone)]
pub struct ChunkData {
    pub position: ChunkPos,
    pub tiles: Box<[TileId; CHUNK_AREA]>,
}

impl ChunkData {
    /// Create a new chunk filled with a specific tile
    pub fn filled(position: ChunkPos, tile_id: TileId) -> Self {
        Self {
            position,
            tiles: Box::new([tile_id; CHUNK_AREA]),
        }
    }

    /// Create an empty chunk (all TILE_EMPTY)
    pub fn empty(position: ChunkPos) -> Self {
        Self::filled(position, TILE_EMPTY)
    }

    /// Get tile at local chunk coordinates (0-31, 0-31)
    pub fn get_tile(&self, local_x: usize, local_y: usize) -> Option<TileId> {
        if local_x >= CHUNK_SIZE || local_y >= CHUNK_SIZE {
            return None;
        }
        let index = local_y * CHUNK_SIZE + local_x;
        Some(self.tiles[index])
    }

    /// Set tile at local chunk coordinates (0-31, 0-31)
    pub fn set_tile(&mut self, local_x: usize, local_y: usize, tile_id: TileId) -> bool {
        if local_x >= CHUNK_SIZE || local_y >= CHUNK_SIZE {
            return false;
        }
        let index = local_y * CHUNK_SIZE + local_x;
        self.tiles[index] = tile_id;
        true
    }

    /// Convert ChunkData to Bevy's TilemapChunkTileData
    pub fn to_tilemap_data(&self) -> Vec<Option<TileData>> {
        self.tiles
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

    /// Create ChunkData from Bevy's TilemapChunkTileData
    pub fn from_tilemap_data(
        position: ChunkPos,
        tile_data: &[Option<TileData>],
    ) -> Option<Self> {
        if tile_data.len() != CHUNK_AREA {
            return None;
        }

        let mut tiles = Box::new([TILE_EMPTY; CHUNK_AREA]);
        for (i, data) in tile_data.iter().enumerate() {
            tiles[i] = match data {
                Some(tile) => {
                    // Add 1 because tileset indices start at 0, but our TILE_EMPTY is 0
                    (tile.tileset_index + 1) as TileId
                }
                None => TILE_EMPTY,
            };
        }

        Some(Self { position, tiles })
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

        // Set and get tile
        assert!(chunk.set_tile(5, 10, TILE_GRASS));
        assert_eq!(chunk.get_tile(5, 10), Some(TILE_GRASS));

        // Out of bounds
        assert!(!chunk.set_tile(32, 0, TILE_DIRT));
        assert_eq!(chunk.get_tile(32, 0), None);
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
