use crate::tiles::{ChunkData, ChunkPos, TILE_GRASS};

/// Generate a new chunk at the given position
/// For now, generates a simple grass world
pub fn generate_chunk(position: ChunkPos) -> ChunkData {
    // Start with all grass tiles
    let chunk = ChunkData::filled(position, TILE_GRASS);

    // Future: Add procedural generation
    // - Use noise functions for terrain variety
    // - Add dirt patches, water, trees, etc.
    // - Generate based on biomes
    // - Add structures/features

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
