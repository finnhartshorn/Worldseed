use crate::tiles::{ChunkData, ChunkPos, TILE_GRASS, TILE_DIRT, CHUNK_SIZE, LAYER_GROUND};

/// Generate a new chunk at the given position
/// Generates a checkerboard pattern of grass and dirt tiles
pub fn generate_chunk(position: ChunkPos) -> ChunkData {
    // Start with empty chunk
    let mut chunk = ChunkData::empty(position);

    // Create checkerboard pattern on ground layer
    for y in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            // Alternate between grass and dirt based on tile coordinates
            let tile = if (x + y) % 2 == 0 {
                TILE_GRASS
            } else {
                TILE_DIRT
            };
            chunk.set_tile(LAYER_GROUND, x, y, tile);
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
