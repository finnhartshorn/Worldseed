use crate::{
    tiles::{ChunkData, ChunkPos, CHUNK_SIZE, CHUNK_SIZE_I32, LAYER_GROUND, TILE_EMPTY},
    world::savegame::{WorldElement, WorldGenerationConfig, WorldShape},
};
use bevy::prelude::IVec2;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::{HashMap, HashSet};

fn ground_tile_for_element(element: WorldElement) -> u16 {
    match element {
        WorldElement::Grass => crate::tiles::TILE_GRASS,
        WorldElement::Dirt => crate::tiles::TILE_DIRT,
    }
}

/// Generate a new chunk from the active world-generation config.
pub fn generate_chunk(position: ChunkPos, config: WorldGenerationConfig) -> ChunkData {
    match config.shape {
        WorldShape::Infinity => generate_infinite_plane(position, config.element),
        // Island worlds are pregenerated once up front and then loaded from disk.
        WorldShape::Island => ChunkData::empty(position),
    }
}

pub fn pregenerate_world_chunks(config: WorldGenerationConfig) -> Vec<ChunkData> {
    match config.shape {
        WorldShape::Infinity => Vec::new(),
        WorldShape::Island => pregenerate_island_chunks(config),
    }
}

fn generate_infinite_plane(position: ChunkPos, element: WorldElement) -> ChunkData {
    let mut chunk = ChunkData::empty(position);
    let tile_id = ground_tile_for_element(element);

    for y in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            chunk.set_tile(LAYER_GROUND, x, y, tile_id);
        }
    }

    chunk
}

fn pregenerate_island_chunks(config: WorldGenerationConfig) -> Vec<ChunkData> {
    let mut land_tiles = HashSet::new();
    let target_tiles = target_island_tile_count(config.power);
    let max_radius_tiles = max_island_radius_tiles(target_tiles);
    let mut frontier = Vec::new();
    let mut frontier_set = HashSet::new();
    let mut rng = StdRng::seed_from_u64(config.seed);

    land_tiles.insert(IVec2::ZERO);
    frontier.push(IVec2::ZERO);
    frontier_set.insert(IVec2::ZERO);

    while land_tiles.len() < target_tiles && !frontier.is_empty() {
        let frontier_index = rng.random_range(0..frontier.len());
        let tile_pos = frontier.swap_remove(frontier_index);
        frontier_set.remove(&tile_pos);
        fill_empty_neighbors(
            tile_pos,
            target_tiles,
            max_radius_tiles,
            &mut land_tiles,
            &mut frontier,
            &mut frontier_set,
        );
    }

    let tile_id = ground_tile_for_element(config.element);
    let mut chunks = HashMap::new();
    for tile_pos in land_tiles {
        let chunk_pos = ChunkPos::from_tile(tile_pos, CHUNK_SIZE_I32);
        let local_x = tile_pos.x.rem_euclid(CHUNK_SIZE_I32) as usize;
        let local_y = tile_pos.y.rem_euclid(CHUNK_SIZE_I32) as usize;
        let chunk = chunks
            .entry(chunk_pos)
            .or_insert_with(|| ChunkData::empty(chunk_pos));
        chunk.set_tile(LAYER_GROUND, local_x, local_y, tile_id);
    }

    let mut chunks: Vec<_> = chunks.into_values().collect();
    chunks.sort_by_key(|chunk| (chunk.position.x, chunk.position.y));
    chunks
}

fn target_island_tile_count(power: u8) -> usize {
    usize::from(power.max(1)) * CHUNK_SIZE * CHUNK_SIZE
}

fn max_island_radius_tiles(target_tiles: usize) -> i32 {
    let approximate_land_radius = (target_tiles as f32 / std::f32::consts::PI).sqrt();
    (approximate_land_radius * 1.8).ceil().max(8.0) as i32
}

fn within_island_bounds(tile_pos: IVec2, max_radius_tiles: i32) -> bool {
    let dx = i64::from(tile_pos.x);
    let dy = i64::from(tile_pos.y);
    let radius_sq = i64::from(max_radius_tiles) * i64::from(max_radius_tiles);
    dx * dx + dy * dy <= radius_sq
}

fn fill_empty_neighbors(
    tile_pos: IVec2,
    target_tiles: usize,
    max_radius_tiles: i32,
    land_tiles: &mut HashSet<IVec2>,
    frontier: &mut Vec<IVec2>,
    frontier_set: &mut HashSet<IVec2>,
) {
    for neighbor in moore_neighbors(tile_pos) {
        if land_tiles.len() >= target_tiles {
            break;
        }

        if within_island_bounds(neighbor, max_radius_tiles) && land_tiles.insert(neighbor) {
            if frontier_set.insert(neighbor) {
                frontier.push(neighbor);
            }
        }
    }
}

fn moore_neighbors(tile_pos: IVec2) -> [IVec2; 8] {
    [
        tile_pos + IVec2::new(1, 1),
        tile_pos + IVec2::new(1, 0),
        tile_pos + IVec2::new(1, -1),
        tile_pos + IVec2::new(0, 1),
        tile_pos + IVec2::new(0, -1),
        tile_pos + IVec2::new(-1, -1),
        tile_pos + IVec2::new(-1, 0),
        tile_pos + IVec2::new(-1, 1),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tiles::{ChunkPos, TILE_DIRT, TILE_GRASS};
    use std::collections::{HashSet, VecDeque};

    fn count_land_tiles(chunks: &[ChunkData]) -> usize {
        chunks
            .iter()
            .flat_map(|chunk| chunk.layers[LAYER_GROUND].iter())
            .filter(|&&tile| tile != TILE_EMPTY)
            .count()
    }

    fn ground_tiles(chunks: &[ChunkData]) -> HashSet<IVec2> {
        let mut tiles = HashSet::new();

        for chunk in chunks {
            let base_x = chunk.position.x * CHUNK_SIZE_I32;
            let base_y = chunk.position.y * CHUNK_SIZE_I32;
            for y in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    if chunk.get_tile(LAYER_GROUND, x, y) != Some(TILE_EMPTY) {
                        tiles.insert(IVec2::new(base_x + x as i32, base_y + y as i32));
                    }
                }
            }
        }

        tiles
    }

    #[test]
    fn infinite_grass_matches_previous_default() {
        let chunk = generate_chunk(ChunkPos::new(0, 0), WorldGenerationConfig::default());

        for y in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                assert_eq!(chunk.get_tile(LAYER_GROUND, x, y), Some(TILE_GRASS));
            }
        }
    }

    #[test]
    fn infinite_dirt_fills_chunk_with_dirt() {
        let chunk = generate_chunk(
            ChunkPos::new(0, 0),
            WorldGenerationConfig {
                shape: WorldShape::Infinity,
                element: WorldElement::Dirt,
                ..WorldGenerationConfig::default()
            },
        );

        for y in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                assert_eq!(chunk.get_tile(LAYER_GROUND, x, y), Some(TILE_DIRT));
            }
        }
    }

    #[test]
    fn island_pregeneration_is_deterministic_for_seed_and_power() {
        let config = WorldGenerationConfig {
            shape: WorldShape::Island,
            element: WorldElement::Grass,
            power: 1,
            seed: 42,
        };

        let first = pregenerate_world_chunks(config);
        let second = pregenerate_world_chunks(config);

        assert_eq!(first.len(), second.len());
        assert_eq!(ground_tiles(&first), ground_tiles(&second));
    }

    #[test]
    fn higher_power_generates_more_land_tiles() {
        let small = pregenerate_world_chunks(WorldGenerationConfig {
            shape: WorldShape::Island,
            element: WorldElement::Grass,
            power: 1,
            seed: 99,
        });
        let large = pregenerate_world_chunks(WorldGenerationConfig {
            shape: WorldShape::Island,
            element: WorldElement::Grass,
            power: 2,
            seed: 99,
        });

        assert!(count_land_tiles(&large) > count_land_tiles(&small));
    }

    #[test]
    fn island_contains_origin_and_is_connected() {
        let chunks = pregenerate_world_chunks(WorldGenerationConfig {
            shape: WorldShape::Island,
            element: WorldElement::Grass,
            power: 1,
            seed: 12345,
        });

        let land_tiles = ground_tiles(&chunks);
        assert!(land_tiles.contains(&IVec2::ZERO));

        let mut visited = HashSet::new();
        let mut queue = VecDeque::from([IVec2::ZERO]);

        while let Some(tile) = queue.pop_front() {
            if !visited.insert(tile) {
                continue;
            }

            for neighbor in [
                tile + IVec2::X,
                tile - IVec2::X,
                tile + IVec2::Y,
                tile - IVec2::Y,
            ] {
                if land_tiles.contains(&neighbor) && !visited.contains(&neighbor) {
                    queue.push_back(neighbor);
                }
            }
        }

        assert_eq!(visited.len(), land_tiles.len());
    }

    #[test]
    fn island_lazy_chunk_generation_is_empty() {
        let chunk = generate_chunk(
            ChunkPos::new(0, 0),
            WorldGenerationConfig {
                shape: WorldShape::Island,
                element: WorldElement::Grass,
                power: 1,
                seed: 7,
            },
        );

        for y in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                assert_eq!(chunk.get_tile(LAYER_GROUND, x, y), Some(TILE_EMPTY));
            }
        }
    }
}
