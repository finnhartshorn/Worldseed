use super::{generator, savegame::WorldShape, serialization};
use crate::tiles::{
    chunk::coords, ChunkData, ChunkPos, TileId, CHUNK_PIXEL_SIZE, LAYER_GROUND, TILE_EMPTY,
};
use crate::world::savegame::WorldGenerationConfig;
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Represents a pending tile modification
#[derive(Debug, Clone)]
pub struct TileModification {
    pub world_x: f32,
    pub world_y: f32,
    pub tile_id: u16,
    pub layer: usize,
}

/// Event triggered when chunk data changes and visuals need to be updated
#[derive(Event, Debug, Clone)]
pub struct ChunkDataChanged {
    pub chunk_pos: ChunkPos,
    pub layer: usize,
}

/// World manager resource that tracks all loaded chunks and their state
#[derive(Resource)]
pub struct WorldManager {
    /// Map of chunk positions to their layer entities
    /// Key: ChunkPos, Value: Array of entity IDs (one per layer)
    pub active_chunks: HashMap<ChunkPos, [Entity; crate::tiles::NUM_LAYERS]>,

    /// Set of chunks that have been modified and need saving
    pub dirty_chunks: HashSet<ChunkPos>,

    /// In-memory cache of chunk data
    pub chunk_cache: HashMap<ChunkPos, ChunkData>,

    /// Directory where chunk files are saved
    pub save_directory: PathBuf,

    /// Current camera chunk position (for loading/unloading decisions)
    pub camera_chunk: Option<ChunkPos>,

    /// Active world-generation config for newly generated chunks
    pub generation: WorldGenerationConfig,

    /// Queue of pending tile modifications
    pub pending_tile_modifications: Vec<TileModification>,
}

impl WorldManager {
    pub fn new(save_directory: PathBuf) -> Self {
        Self {
            active_chunks: HashMap::new(),
            dirty_chunks: HashSet::new(),
            chunk_cache: HashMap::new(),
            save_directory,
            camera_chunk: None,
            generation: WorldGenerationConfig::default(),
            pending_tile_modifications: Vec::new(),
        }
    }

    /// Check if a chunk is currently loaded
    pub fn is_loaded(&self, pos: &ChunkPos) -> bool {
        self.active_chunks.contains_key(pos)
    }

    /// Get the entities for a loaded chunk (all layers)
    pub fn get_chunk_entities(
        &self,
        pos: &ChunkPos,
    ) -> Option<&[Entity; crate::tiles::NUM_LAYERS]> {
        self.active_chunks.get(pos)
    }

    /// Get a specific layer entity for a chunk
    pub fn get_chunk_layer_entity(&self, pos: &ChunkPos, layer: usize) -> Option<Entity> {
        self.active_chunks.get(pos).map(|entities| entities[layer])
    }

    /// Register chunk layer entities
    pub fn register_chunk(&mut self, pos: ChunkPos, entities: [Entity; crate::tiles::NUM_LAYERS]) {
        self.active_chunks.insert(pos, entities);
    }

    /// Unregister a chunk entity (when despawning)
    pub fn unregister_chunk(
        &mut self,
        pos: &ChunkPos,
    ) -> Option<[Entity; crate::tiles::NUM_LAYERS]> {
        self.active_chunks.remove(pos)
    }

    /// Mark a chunk as dirty (needs saving)
    pub fn mark_dirty(&mut self, pos: ChunkPos) {
        self.dirty_chunks.insert(pos);
    }

    /// Clear dirty flag for a chunk (after saving)
    pub fn clear_dirty(&mut self, pos: &ChunkPos) {
        self.dirty_chunks.remove(pos);
    }

    /// Check if a chunk is dirty
    pub fn is_dirty(&self, pos: &ChunkPos) -> bool {
        self.dirty_chunks.contains(pos)
    }

    /// Get all dirty chunk positions
    pub fn get_dirty_chunks(&self) -> Vec<ChunkPos> {
        self.dirty_chunks.iter().copied().collect()
    }

    /// Add chunk data to cache
    pub fn cache_chunk(&mut self, data: ChunkData) {
        self.chunk_cache.insert(data.position, data);
    }

    /// Get chunk data from cache
    pub fn get_cached_chunk(&self, pos: &ChunkPos) -> Option<&ChunkData> {
        self.chunk_cache.get(pos)
    }

    /// Remove chunk data from cache
    pub fn uncache_chunk(&mut self, pos: &ChunkPos) -> Option<ChunkData> {
        self.chunk_cache.remove(pos)
    }

    /// Get the path to a chunk save file
    pub fn get_chunk_path(&self, pos: &ChunkPos) -> PathBuf {
        self.save_directory
            .join("chunks")
            .join(format!("chunk_{}_{}.bin", pos.x, pos.y))
    }

    /// Ensure chunk data exists in cache, loading from disk or generating it when needed.
    pub fn ensure_chunk_cached(&mut self, chunk_pos: ChunkPos) {
        if self.chunk_cache.contains_key(&chunk_pos) {
            return;
        }

        let chunk_path = self.get_chunk_path(&chunk_pos);
        let chunk_data = if serialization::chunk_exists(&chunk_path) {
            match serialization::load_chunk(&chunk_path) {
                Ok(data) => data,
                Err(error) => {
                    warn!(
                        "Failed to load chunk {:?}: {}, falling back to generated data",
                        chunk_pos, error
                    );
                    match self.generation.shape {
                        WorldShape::Island => ChunkData::empty(chunk_pos),
                        WorldShape::Infinity => generator::generate_chunk(chunk_pos, self.generation),
                    }
                }
            }
        } else {
            match self.generation.shape {
                WorldShape::Island => ChunkData::empty(chunk_pos),
                WorldShape::Infinity => generator::generate_chunk(chunk_pos, self.generation),
            }
        };

        self.cache_chunk(chunk_data);
    }

    /// Return the ground tile at a world-space position, loading the chunk into cache if needed.
    pub fn ground_tile_at_world(&mut self, world_pos: Vec2) -> TileId {
        let chunk_pos = ChunkPos::from_world(world_pos, CHUNK_PIXEL_SIZE);
        self.ensure_chunk_cached(chunk_pos);

        let Some(chunk) = self.chunk_cache.get(&chunk_pos) else {
            return TILE_EMPTY;
        };

        let (local_x, local_y) = coords::world_to_local_tile(world_pos);
        chunk.get_tile(LAYER_GROUND, local_x, local_y).unwrap_or(TILE_EMPTY)
    }

    /// Void is represented by an empty ground tile. Land checks should go through this helper.
    pub fn has_land_at_world(&mut self, world_pos: Vec2) -> bool {
        self.ground_tile_at_world(world_pos) != TILE_EMPTY
    }

    /// Update the camera's chunk position
    pub fn update_camera_position(&mut self, chunk_pos: ChunkPos) {
        self.camera_chunk = Some(chunk_pos);
    }

    /// Queue a tile modification at a world position (in pixels)
    /// The modification will be applied by the apply_tile_modifications system
    pub fn queue_tile_modification(
        &mut self,
        world_x: f32,
        world_y: f32,
        tile_id: u16,
        layer: usize,
    ) {
        self.pending_tile_modifications.push(TileModification {
            world_x,
            world_y,
            tile_id,
            layer,
        });
    }

    /// Get all pending tile modifications and clear the queue
    pub fn take_tile_modifications(&mut self) -> Vec<TileModification> {
        std::mem::take(&mut self.pending_tile_modifications)
    }

    /// Get statistics about the world state
    pub fn stats(&self) -> WorldStats {
        WorldStats {
            loaded_chunks: self.active_chunks.len(),
            dirty_chunks: self.dirty_chunks.len(),
            cached_chunks: self.chunk_cache.len(),
            camera_chunk: self.camera_chunk,
        }
    }
}

impl Default for WorldManager {
    fn default() -> Self {
        Self::new(PathBuf::from("saves/world"))
    }
}

/// Statistics about the current world state
#[derive(Debug, Clone)]
pub struct WorldStats {
    pub loaded_chunks: usize,
    pub dirty_chunks: usize,
    pub cached_chunks: usize,
    pub camera_chunk: Option<ChunkPos>,
}

impl std::fmt::Display for WorldStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Loaded: {}, Dirty: {}, Cached: {}, Camera: {:?}",
            self.loaded_chunks, self.dirty_chunks, self.cached_chunks, self.camera_chunk
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tiles::{CHUNK_SIZE, TILE_GRASS, TILE_WORLD_SIZE};

    #[test]
    fn has_land_at_world_reads_ground_tiles_from_cache() {
        let mut world = WorldManager::default();
        let chunk_pos = ChunkPos::new(0, 0);
        let mut chunk = ChunkData::empty(chunk_pos);
        chunk.set_tile(LAYER_GROUND, 3, 4, TILE_GRASS);
        world.cache_chunk(chunk);

        let land_pos = Vec2::new(3.0 * TILE_WORLD_SIZE + 1.0, 4.0 * TILE_WORLD_SIZE + 1.0);
        let void_pos = Vec2::new(
            (CHUNK_SIZE as f32 - 1.0) * TILE_WORLD_SIZE,
            (CHUNK_SIZE as f32 - 1.0) * TILE_WORLD_SIZE,
        );

        assert!(world.has_land_at_world(land_pos));
        assert!(!world.has_land_at_world(void_pos));
    }
}
