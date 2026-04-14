use crate::tiles::{ChunkData, ChunkPos, CHUNK_SIZE, LAYER_GROUND, TILE_GRASS};

/// Generate a new chunk at the given position with all grass terrain
pub fn generate_chunk(position: ChunkPos) -> ChunkData {
    // Start with empty chunk
    let mut chunk = ChunkData::empty(position);

    // Fill with all grass terrain on ground layer
    for y in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            chunk.set_tile(LAYER_GROUND, x, y, TILE_GRASS);
        }
    }

    // Decoration and overlay layers remain empty

    chunk
}

// Future: Add more sophisticated generation
/*
use noise::{NoiseFn, Perlin};

pub struct WorldGenerator {
    terrain_noise: Perlin,
    seed: u32,
}

impl WorldGenerator {
    pub fn new(seed: u32) -> Self {
        Self {
            terrain_noise: Perlin::new(seed),
            seed,
        }
    }

    pub fn generate_chunk(&self, position: ChunkPos) -> ChunkData {
        let mut chunk = ChunkData::empty(position);

        for y in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let world_x = position.x * CHUNK_SIZE_I32 + x as i32;
                let world_y = position.y * CHUNK_SIZE_I32 + y as i32;

                // Sample noise
                let noise_value = self.terrain_noise.get([
                    world_x as f64 * 0.05,
                    world_y as f64 * 0.05,
                ]);

                // Choose tile based on noise value
                let tile = if noise_value > 0.3 {
                    TILE_GRASS
                } else if noise_value > 0.0 {
                    TILE_DIRT
                } else {
                    TILE_WATER
                };

                chunk.set_tile(x, y, tile);
            }
        }

        chunk
    }
}
*/
