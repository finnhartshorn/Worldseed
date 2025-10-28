use super::{generator, manager::WorldManager, serialization};
use crate::tiles::{
    chunk::coords, Chunk, ChunkData, ChunkPos, DirtyChunk, CHUNK_LOAD_RADIUS, CHUNK_UNLOAD_RADIUS,
    TILE_DISPLAY_SIZE,
};
use bevy::prelude::*;
use bevy::sprite_render::{TilemapChunk, TilemapChunkTileData};
use std::collections::HashSet;

/// System to track camera position and trigger chunk loading/unloading
pub fn update_camera_chunk(
    camera_query: Query<&Transform, With<Camera2d>>,
    mut world: ResMut<WorldManager>,
) {
    if let Ok(camera_transform) = camera_query.single() {
        let camera_pos = camera_transform.translation.truncate();
        let chunk_pos = coords::world_to_chunk(camera_pos);

        // Only trigger loading if camera moved to a new chunk
        if world.camera_chunk != Some(chunk_pos) {
            world.update_camera_position(chunk_pos);
            info!("Camera moved to chunk {:?}", chunk_pos);
        }
    }
}

/// System to load chunks around the camera
pub fn load_chunks_around_camera(
    mut commands: Commands,
    mut world: ResMut<WorldManager>,
    asset_server: Res<AssetServer>,
) {
    let Some(camera_chunk) = world.camera_chunk else {
        return;
    };

    // Get chunks that should be loaded
    let chunks_to_load = camera_chunk.chunks_in_radius(CHUNK_LOAD_RADIUS);
    let has_loaded_chunks = !chunks_to_load.is_empty();

    for chunk_pos in chunks_to_load {
        // Skip if already loaded
        if world.is_loaded(&chunk_pos) {
            continue;
        }

        // Try to load from cache first
        let chunk_data = if let Some(cached) = world.get_cached_chunk(&chunk_pos) {
            cached.clone()
        } else {
            // Try to load from disk
            let chunk_path = world.get_chunk_path(&chunk_pos);
            if serialization::chunk_exists(&chunk_path) {
                match serialization::load_chunk(&chunk_path) {
                    Ok(data) => {
                        info!("Loaded chunk {:?} from disk", chunk_pos);
                        data
                    }
                    Err(e) => {
                        warn!("Failed to load chunk {:?}: {}, generating new", chunk_pos, e);
                        generator::generate_chunk(chunk_pos)
                    }
                }
            } else {
                // Generate new chunk
                info!("Generating new chunk {:?}", chunk_pos);
                generator::generate_chunk(chunk_pos)
            }
        };

        // Convert to Bevy tilemap format
        let tile_data = chunk_data.to_tilemap_data();
        let world_pos = chunk_pos.to_world(crate::tiles::CHUNK_PIXEL_SIZE);

        // Spawn chunk entity
        let entity = commands
            .spawn((
                TilemapChunk {
                    chunk_size: UVec2::splat(crate::tiles::CHUNK_SIZE as u32),
                    tile_display_size: UVec2::splat(TILE_DISPLAY_SIZE),
                    tileset: asset_server.load("tilesets/terrain_array.png"),
                    ..default()
                },
                TilemapChunkTileData(tile_data),
                Transform::from_xyz(world_pos.x, world_pos.y, 0.0),
                Chunk::new(chunk_pos),
            ))
            .id();

        // Register in world manager
        world.register_chunk(chunk_pos, entity);
        world.cache_chunk(chunk_data);

        info!("Loaded chunk {:?} at entity {:?}", chunk_pos, entity);
    }

    // Print chunk grid after loading
    if has_loaded_chunks {
        print_chunk_grid(&world, camera_chunk);
    }
}

/// System to unload chunks far from the camera
pub fn unload_distant_chunks(
    mut commands: Commands,
    mut world: ResMut<WorldManager>,
    chunk_query: Query<(Entity, &Chunk)>,
) {
    let Some(camera_chunk) = world.camera_chunk else {
        return;
    };

    let mut chunks_to_unload = Vec::new();

    // Find chunks outside the unload radius
    for (entity, chunk) in chunk_query.iter() {
        let distance = camera_chunk.chebyshev_distance(&chunk.position);
        if distance > CHUNK_UNLOAD_RADIUS {
            chunks_to_unload.push((entity, chunk.position));
        }
    }

    // Unload chunks
    let has_unloaded_chunks = !chunks_to_unload.is_empty();

    for (entity, chunk_pos) in chunks_to_unload {
        // Save if dirty
        if world.is_dirty(&chunk_pos) {
            if let Some(chunk_data) = world.get_cached_chunk(&chunk_pos) {
                let chunk_path = world.get_chunk_path(&chunk_pos);
                match serialization::save_chunk(chunk_data, &chunk_path) {
                    Ok(_) => {
                        info!("Saved chunk {:?} to disk", chunk_pos);
                        world.clear_dirty(&chunk_pos);
                    }
                    Err(e) => {
                        error!("Failed to save chunk {:?}: {}", chunk_pos, e);
                    }
                }
            }
        }

        // Despawn entity
        commands.entity(entity).despawn();
        world.unregister_chunk(&chunk_pos);
        world.uncache_chunk(&chunk_pos);

        info!("Unloaded chunk {:?}", chunk_pos);
    }

    // Print chunk grid after unloading
    if has_unloaded_chunks {
        print_chunk_grid(&world, camera_chunk);
    }
}

/// System to periodically save dirty chunks (autosave)
pub fn autosave_dirty_chunks(world: Res<WorldManager>) {
    for chunk_pos in world.get_dirty_chunks() {
        if let Some(chunk_data) = world.get_cached_chunk(&chunk_pos) {
            let chunk_path = world.get_chunk_path(&chunk_pos);
            match serialization::save_chunk(chunk_data, &chunk_path) {
                Ok(_) => {
                    debug!("Autosaved chunk {:?}", chunk_pos);
                }
                Err(e) => {
                    error!("Failed to autosave chunk {:?}: {}", chunk_pos, e);
                }
            }
        }
    }
}

/// System to mark chunks as dirty when tiles are modified
/// This will be triggered by tile editing systems (future implementation)
pub fn mark_modified_chunks(
    mut world: ResMut<WorldManager>,
    modified_chunks: Query<&Chunk, With<DirtyChunk>>,
) {
    for chunk in modified_chunks.iter() {
        world.mark_dirty(chunk.position);
    }
}

/// System to log world statistics for debugging
pub fn log_world_stats(world: Res<WorldManager>) {
    let stats = world.stats();
    debug!("World stats: {}", stats);
}

/// Print a visual representation of loaded chunks
fn print_chunk_grid(world: &WorldManager, camera_chunk: ChunkPos) {
    // Determine the range to display (show area around camera)
    let view_radius = 6; // Show 13x13 grid centered on camera
    let min_x = camera_chunk.x - view_radius;
    let max_x = camera_chunk.x + view_radius;
    let min_y = camera_chunk.y - view_radius;
    let max_y = camera_chunk.y + view_radius;

    // Collect loaded chunks into a set for fast lookup
    let loaded_chunks: HashSet<ChunkPos> = world.active_chunks.keys().copied().collect();

    // Build the grid string
    let mut grid = String::new();
    grid.push_str("\n╔═══════════════ Chunk Grid ═══════════════╗\n");

    // Print column numbers
    grid.push_str("  ");
    for x in min_x..=max_x {
        if x == camera_chunk.x {
            grid.push_str(&format!("{:>3}", x));
        } else {
            grid.push_str(&format!("{:3}", x));
        }
    }
    grid.push('\n');

    // Print each row
    for y in (min_y..=max_y).rev() {
        // Row number
        if y == camera_chunk.y {
            grid.push_str(&format!("{:>2}", y));
        } else {
            grid.push_str(&format!("{:2}", y));
        }

        // Chunks in this row
        for x in min_x..=max_x {
            let pos = ChunkPos::new(x, y);
            let is_loaded = loaded_chunks.contains(&pos);
            let is_camera = pos == camera_chunk;
            let is_in_load_radius = camera_chunk.chebyshev_distance(&pos) <= CHUNK_LOAD_RADIUS;

            let symbol = if is_camera {
                " @ "  // Camera position
            } else if is_loaded && is_in_load_radius {
                " █ "  // Loaded chunk in load radius
            } else if is_loaded {
                " ▓ "  // Loaded chunk outside load radius (about to unload)
            } else if is_in_load_radius {
                " ░ "  // Should be loaded but isn't (transitioning)
            } else {
                " · "  // Not loaded
            };

            grid.push_str(symbol);
        }
        grid.push('\n');
    }

    grid.push_str("╚══════════════════════════════════════════╝\n");
    grid.push_str("Legend: @ = Camera  █ = Loaded  ░ = Loading  · = Unloaded\n");
    grid.push_str(&format!("Loaded chunks: {} | Camera: {:?}\n", loaded_chunks.len(), camera_chunk));

    info!("{}", grid);
}
