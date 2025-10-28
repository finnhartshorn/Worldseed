use crate::tiles::{ChunkData, ChunkPos};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// World manager resource that tracks all loaded chunks and their state
#[derive(Resource)]
pub struct WorldManager {
    /// Map of chunk positions to their entity IDs
    pub active_chunks: HashMap<ChunkPos, Entity>,

    /// Set of chunks that have been modified and need saving
    pub dirty_chunks: HashSet<ChunkPos>,

    /// In-memory cache of chunk data
    pub chunk_cache: HashMap<ChunkPos, ChunkData>,

    /// Directory where chunk files are saved
    pub save_directory: PathBuf,

    /// Current camera chunk position (for loading/unloading decisions)
    pub camera_chunk: Option<ChunkPos>,
}

impl WorldManager {
    pub fn new(save_directory: PathBuf) -> Self {
        Self {
            active_chunks: HashMap::new(),
            dirty_chunks: HashSet::new(),
            chunk_cache: HashMap::new(),
            save_directory,
            camera_chunk: None,
        }
    }

    /// Check if a chunk is currently loaded
    pub fn is_loaded(&self, pos: &ChunkPos) -> bool {
        self.active_chunks.contains_key(pos)
    }

    /// Get the entity for a loaded chunk
    pub fn get_chunk_entity(&self, pos: &ChunkPos) -> Option<Entity> {
        self.active_chunks.get(pos).copied()
    }

    /// Register a new chunk entity
    pub fn register_chunk(&mut self, pos: ChunkPos, entity: Entity) {
        self.active_chunks.insert(pos, entity);
    }

    /// Unregister a chunk entity (when despawning)
    pub fn unregister_chunk(&mut self, pos: &ChunkPos) -> Option<Entity> {
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

    /// Update the camera's chunk position
    pub fn update_camera_position(&mut self, chunk_pos: ChunkPos) {
        self.camera_chunk = Some(chunk_pos);
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
