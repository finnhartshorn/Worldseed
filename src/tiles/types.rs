use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// Type alias for tile IDs (u16 allows 0-65,535 unique tiles)
pub type TileId = u16;

/// Chunk position in chunk coordinates (not world/tile coordinates)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkPos {
    pub x: i32,
    pub y: i32,
}

impl ChunkPos {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Convert from world position (in pixels) to chunk position
    pub fn from_world(world_pos: Vec2, chunk_pixel_size: f32) -> Self {
        Self {
            x: (world_pos.x / chunk_pixel_size).floor() as i32,
            y: (world_pos.y / chunk_pixel_size).floor() as i32,
        }
    }

    /// Convert from tile position to chunk position
    pub fn from_tile(tile_pos: IVec2, chunk_size: i32) -> Self {
        Self {
            x: tile_pos.x.div_euclid(chunk_size),
            y: tile_pos.y.div_euclid(chunk_size),
        }
    }

    /// Get world position of chunk's bottom-left corner (in pixels)
    pub fn to_world(&self, chunk_pixel_size: f32) -> Vec2 {
        Vec2::new(
            self.x as f32 * chunk_pixel_size,
            self.y as f32 * chunk_pixel_size,
        )
    }

    /// Get all chunks in a square radius around this chunk
    pub fn chunks_in_radius(&self, radius: i32) -> Vec<ChunkPos> {
        let mut chunks = Vec::with_capacity(((radius * 2 + 1) * (radius * 2 + 1)) as usize);
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                chunks.push(ChunkPos::new(self.x + dx, self.y + dy));
            }
        }
        chunks
    }

    /// Manhattan distance between two chunk positions
    pub fn manhattan_distance(&self, other: &ChunkPos) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }

    /// Chebyshev distance (square/max distance) between two chunk positions
    /// This represents the minimum number of moves to reach another chunk
    /// when diagonal movement is allowed (matches square radius behavior)
    pub fn chebyshev_distance(&self, other: &ChunkPos) -> i32 {
        (self.x - other.x).abs().max((self.y - other.y).abs())
    }
}

impl From<(i32, i32)> for ChunkPos {
    fn from((x, y): (i32, i32)) -> Self {
        Self::new(x, y)
    }
}

impl From<IVec2> for ChunkPos {
    fn from(v: IVec2) -> Self {
        Self::new(v.x, v.y)
    }
}

impl From<ChunkPos> for IVec2 {
    fn from(pos: ChunkPos) -> Self {
        IVec2::new(pos.x, pos.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_world() {
        let chunk_pixel_size = 256.0; // 32 tiles * 8 pixels

        // Origin chunk
        let pos = ChunkPos::from_world(Vec2::new(0.0, 0.0), chunk_pixel_size);
        assert_eq!(pos, ChunkPos::new(0, 0));

        // Positive chunks
        let pos = ChunkPos::from_world(Vec2::new(256.0, 256.0), chunk_pixel_size);
        assert_eq!(pos, ChunkPos::new(1, 1));

        // Negative chunks
        let pos = ChunkPos::from_world(Vec2::new(-256.0, -256.0), chunk_pixel_size);
        assert_eq!(pos, ChunkPos::new(-1, -1));
    }

    #[test]
    fn test_chunks_in_radius() {
        let center = ChunkPos::new(0, 0);
        let chunks = center.chunks_in_radius(1);
        assert_eq!(chunks.len(), 9); // 3x3 grid

        let chunks = center.chunks_in_radius(3);
        assert_eq!(chunks.len(), 49); // 7x7 grid
    }

    #[test]
    fn test_manhattan_distance() {
        let a = ChunkPos::new(0, 0);
        let b = ChunkPos::new(3, 4);
        assert_eq!(a.manhattan_distance(&b), 7);
    }

    #[test]
    fn test_chebyshev_distance() {
        let a = ChunkPos::new(0, 0);

        // Diagonal - should be max of differences (4)
        let b = ChunkPos::new(3, 4);
        assert_eq!(a.chebyshev_distance(&b), 4);

        // Horizontal - should be just x difference
        let c = ChunkPos::new(5, 0);
        assert_eq!(a.chebyshev_distance(&c), 5);

        // Vertical - should be just y difference
        let d = ChunkPos::new(0, 3);
        assert_eq!(a.chebyshev_distance(&d), 3);

        // Perfect diagonal - both differences equal
        let e = ChunkPos::new(3, 3);
        assert_eq!(a.chebyshev_distance(&e), 3);
    }
}
