use super::{
    manager::WorldManager,
    serialization,
    transitions::{self, TerrainTransitionConfig},
};
use crate::tiles::{
    chunk::coords, Chunk, ChunkData, ChunkPos, DirtyChunk, CHUNK_LOAD_RADIUS, TILE_DISPLAY_SIZE,
};
use bevy::prelude::*;
use bevy::sprite_render::{TileData, TilemapChunk, TilemapChunkTileData};
use bevy::{
    image::{Image, ImageArrayLayout, ImageLoaderSettings},
    prelude::On,
};
#[cfg(feature = "debug_chunks")]
use std::collections::HashSet;

#[derive(Component, Deref, DerefMut)]
pub struct UpdateTimer(Timer);

/// Resource for tracking periodic chunk saves (every 30 seconds by default)
#[derive(Resource, Deref, DerefMut)]
pub struct ChunkSaveTimer(Timer);

impl Default for ChunkSaveTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(30.0, TimerMode::Repeating))
    }
}

// #[derive(Resource, Deref, DerefMut)]
// struct SeededRng(ChaCha8Rng);

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

/// Helper function to load chunk data into cache without spawning visual entities.
/// This is the data-layer operation that ensures ChunkData exists in cache.
///
/// Returns a reference to the cached ChunkData.
fn load_chunk_data_only(world: &mut WorldManager, chunk_pos: ChunkPos) -> ChunkData {
    world.ensure_chunk_cached(chunk_pos);
    world
        .get_cached_chunk(&chunk_pos)
        .expect("chunk should exist in cache after ensure_chunk_cached")
        .clone()
}

/// System to load chunks around the camera
pub fn load_chunks_around_camera(
    mut commands: Commands,
    mut world: ResMut<WorldManager>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
    mut transition_config: Option<ResMut<TerrainTransitionConfig>>,
    camera_query: Query<(&Transform, &Projection), With<Camera2d>>,
    window_query: Query<&Window>,
) {
    let Some(camera_chunk) = world.camera_chunk else {
        return;
    };

    // Calculate dynamic load radius based on zoom level
    let load_radius = calculate_load_radius(&camera_query, &window_query);

    // Get chunks that should be loaded
    let chunks_to_load = camera_chunk.chunks_in_radius(load_radius);
    #[cfg(feature = "debug_chunks")]
    let has_loaded_chunks = !chunks_to_load.is_empty();
    let tileset = if let Some(config) = transition_config.as_deref_mut() {
        if config.enabled {
            transitions::ensure_runtime_tileset(config, &mut images).unwrap_or_else(|error| {
                error!("Failed to build transition tileset: {}", error);
                asset_server.load_with_settings(
                    "tilesets/terrain_array.png",
                    |settings: &mut ImageLoaderSettings| {
                        settings.array_layout = Some(ImageArrayLayout::RowCount { rows: 2 });
                    },
                )
            })
        } else {
            asset_server.load_with_settings(
                "tilesets/terrain_array.png",
                |settings: &mut ImageLoaderSettings| {
                    settings.array_layout = Some(ImageArrayLayout::RowCount { rows: 2 });
                },
            )
        }
    } else {
        asset_server.load_with_settings(
            "tilesets/terrain_array.png",
            |settings: &mut ImageLoaderSettings| {
                settings.array_layout = Some(ImageArrayLayout::RowCount { rows: 2 });
            },
        )
    };

    for chunk_pos in chunks_to_load {
        // Skip if already loaded
        if world.is_loaded(&chunk_pos) {
            continue;
        }

        // Load chunk data into cache (without spawning visual entities)
        load_chunk_data_only(&mut world, chunk_pos);
        if let Some(config) = transition_config.as_deref_mut() {
            transitions::refresh_chunk_transitions(&mut world, config, &mut images, chunk_pos);
        }
        let chunk_data = world
            .get_cached_chunk(&chunk_pos)
            .cloned()
            .expect("chunk should be cached before visual spawn");

        // Get world position for chunk
        let chunk_origin = chunk_pos.to_world(crate::tiles::CHUNK_PIXEL_SIZE);
        let chunk_center = chunk_origin + Vec2::splat(crate::tiles::CHUNK_PIXEL_SIZE / 2.0);

        // Spawn visual entities (view layer) from cached data
        let mut layer_entities = [Entity::PLACEHOLDER; crate::tiles::NUM_LAYERS];
        for layer_idx in 0..crate::tiles::NUM_LAYERS {
            let tile_data = chunk_data.layer_to_tilemap_data(layer_idx);
            let z_pos = crate::tiles::layer_z_position(layer_idx);

            let entity = commands
                .spawn((
                    TilemapChunk {
                        chunk_size: UVec2::splat(crate::tiles::CHUNK_SIZE as u32),
                        tile_display_size: UVec2::splat(TILE_DISPLAY_SIZE),
                        tileset: tileset.clone(),
                        ..default()
                    },
                    TilemapChunkTileData(tile_data),
                    Transform::from_xyz(chunk_center.x, chunk_center.y, z_pos),
                    Chunk::with_layer(chunk_pos, layer_idx),
                    UpdateTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
                ))
                .id();

            layer_entities[layer_idx] = entity;
        }

        // Register visual entities in world manager
        world.register_chunk(chunk_pos, layer_entities);

        info!(
            "Loaded chunk {:?} with {} layers",
            chunk_pos,
            crate::tiles::NUM_LAYERS
        );
    }

    // Print chunk grid after loading
    #[cfg(feature = "debug_chunks")]
    if has_loaded_chunks {
        let visible_chunks = calculate_visible_chunks(&camera_query, &window_query);
        print_chunk_grid(&world, camera_chunk, visible_chunks, load_radius);
    }
}

/// System to unload chunks far from the camera
pub fn unload_distant_chunks(
    mut commands: Commands,
    mut world: ResMut<WorldManager>,
    chunk_query: Query<(Entity, &Chunk)>,
    camera_query: Query<(&Transform, &Projection), With<Camera2d>>,
    window_query: Query<&Window>,
) {
    let Some(camera_chunk) = world.camera_chunk else {
        return;
    };

    // Calculate dynamic radii based on zoom level
    let load_radius = calculate_load_radius(&camera_query, &window_query);
    let unload_radius = calculate_unload_radius(load_radius);

    let mut chunks_to_unload = Vec::new();

    // Find chunks outside the unload radius
    for (entity, chunk) in chunk_query.iter() {
        let distance = camera_chunk.chebyshev_distance(&chunk.position);
        if distance > unload_radius {
            chunks_to_unload.push((entity, chunk.position));
        }
    }

    // Unload chunks
    #[cfg(feature = "debug_chunks")]
    let has_unloaded_chunks = !chunks_to_unload.is_empty();

    for (_entity, chunk_pos) in chunks_to_unload {
        // Note: entity is just one layer entity, we need to despawn all layers
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

        // Despawn all layer entities (visual cleanup only)
        if let Some(layer_entities) = world.unregister_chunk(&chunk_pos) {
            for layer_entity in layer_entities {
                commands.entity(layer_entity).despawn();
            }
        }

        // NOTE: We no longer uncache chunk data here!
        // ChunkData persists in cache independent of rendering state.
        // This allows entities to modify unrendered chunks and enables faster re-loading.
        // Cache is unlimited as per architecture decision.
        // world.uncache_chunk(&chunk_pos);  // REMOVED

        info!(
            "Unloaded visual entities for chunk {:?} (data preserved in cache)",
            chunk_pos
        );
    }

    // Print chunk grid after unloading
    #[cfg(feature = "debug_chunks")]
    if has_unloaded_chunks {
        let visible_chunks = calculate_visible_chunks(&camera_query, &window_query);
        print_chunk_grid(&world, camera_chunk, visible_chunks, load_radius);
    }
}

/// System to periodically save dirty chunks (autosave with timer)
pub fn autosave_dirty_chunks(
    time: Res<Time>,
    mut timer: ResMut<ChunkSaveTimer>,
    mut world: ResMut<WorldManager>,
) {
    // Tick the timer
    timer.tick(time.delta());

    // Only save when timer finishes
    if !timer.just_finished() {
        return;
    }

    let dirty_chunks = world.get_dirty_chunks();
    if dirty_chunks.is_empty() {
        return;
    }

    info!(
        "Autosave: saving {} dirty chunks (periodic save every {}s)",
        dirty_chunks.len(),
        timer.0.duration().as_secs()
    );

    for chunk_pos in dirty_chunks {
        if let Some(chunk_data) = world.get_cached_chunk(&chunk_pos) {
            let chunk_path = world.get_chunk_path(&chunk_pos);
            match serialization::save_chunk(chunk_data, &chunk_path) {
                Ok(_) => {
                    debug!("Autosaved chunk {:?}", chunk_pos);
                    // Clear dirty flag after successful save
                    world.clear_dirty(&chunk_pos);
                }
                Err(e) => {
                    error!("Failed to autosave chunk {:?}: {}", chunk_pos, e);
                    // Keep dirty flag on failure so we retry next time
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

/// System to apply pending tile modifications to cache and trigger observers for visual updates
pub fn apply_tile_modifications(
    mut world: ResMut<WorldManager>,
    mut commands: Commands,
    mut transition_config: Option<ResMut<TerrainTransitionConfig>>,
    mut images: ResMut<Assets<Image>>,
) {
    use crate::tiles::chunk::coords;
    use crate::tiles::LAYER_GROUND;
    use std::collections::HashSet;

    let modifications = world.take_tile_modifications();
    if modifications.is_empty() {
        return;
    }

    let mut changed_layers: HashSet<(ChunkPos, usize)> = HashSet::new();

    for modification in modifications {
        let world_pos = Vec2::new(modification.world_x, modification.world_y);
        if !world.has_land_at_world(world_pos) {
            debug!(
                "Ignoring tile modification on void at ({}, {})",
                modification.world_x, modification.world_y
            );
            continue;
        }

        // Convert world position to chunk position
        let chunk_pos = coords::world_to_chunk(world_pos);

        // Ensure chunk data is in cache (load from disk/generate if needed)
        // Modifications work on any chunk, regardless of render state
        if !world.chunk_cache.contains_key(&chunk_pos) {
            load_chunk_data_only(&mut world, chunk_pos);
            debug!(
                "Loaded chunk {:?} data to apply modification at ({}, {})",
                chunk_pos, modification.world_x, modification.world_y
            );
        }

        // Update the cache (this always succeeds)
        let chunk_data = world
            .chunk_cache
            .get_mut(&chunk_pos)
            .expect("Chunk data should be in cache after load_chunk_data_only");

        let (local_x, local_y) = coords::world_to_local_tile(world_pos);

        if chunk_data.set_tile(modification.layer, local_x, local_y, modification.tile_id) {
            // Mark chunk as dirty for later persistence
            world.mark_dirty(chunk_pos);
            changed_layers.insert((chunk_pos, modification.layer));

            if modification.layer == LAYER_GROUND {
                if let Some(config) = transition_config.as_deref_mut() {
                    if config.enabled {
                        let edited_tile =
                            transitions::world_to_tile(modification.world_x, modification.world_y);
                        let changed_transition_chunks =
                            transitions::refresh_transitions_around_ground_edit(
                                &mut world,
                                config,
                                &mut images,
                                edited_tile,
                            );
                        for changed_chunk in changed_transition_chunks {
                            world.mark_dirty(changed_chunk);
                            changed_layers.insert((changed_chunk, config.overlay_layer));
                        }
                    }
                }
            }
        }
    }

    for (chunk_pos, layer) in changed_layers {
        commands.trigger(super::manager::ChunkDataChanged { chunk_pos, layer });
        debug!(
            "Triggered ChunkDataChanged observer for chunk {:?}, layer {}",
            chunk_pos, layer
        );
    }
}

/// Observer system that syncs visual tilemap entities when chunk data changes
/// This creates a clean separation: data layer (ChunkData) -> event -> visual layer (TilemapChunk)
pub fn sync_chunk_visuals_on_data_change(
    trigger: On<super::manager::ChunkDataChanged>,
    world: Res<WorldManager>,
    mut chunk_query: Query<(&Chunk, &mut TilemapChunkTileData)>,
) {
    use crate::tiles::{CHUNK_SIZE, TILE_EMPTY};
    use bevy::sprite_render::TileData;

    let event = trigger.event();
    let chunk_pos = event.chunk_pos;
    let layer = event.layer;

    // Get the chunk data from cache
    let Some(chunk_data) = world.get_cached_chunk(&chunk_pos) else {
        warn!(
            "ChunkDataChanged event for chunk {:?} but no data in cache",
            chunk_pos
        );
        return;
    };

    // Find the visual entity for this chunk and layer
    for (chunk, mut tile_data) in chunk_query.iter_mut() {
        if chunk.position == chunk_pos && chunk.layer == layer {
            // Sync the entire layer from ChunkData to visual TilemapChunkTileData
            for y in 0..CHUNK_SIZE {
                for x in 0..CHUNK_SIZE {
                    let index = y * CHUNK_SIZE + x;
                    let tile_id_opt = chunk_data.get_tile(layer, x, y);

                    tile_data.0[index] = match tile_id_opt {
                        Some(tile_id) if tile_id != TILE_EMPTY => {
                            Some(TileData::from_tileset_index((tile_id - 1) as u16))
                        }
                        _ => None,
                    };
                }
            }

            debug!(
                "Synced visual tilemap for chunk {:?}, layer {} from cache",
                chunk_pos, layer
            );
            break;
        }
    }

    // If no visual entity exists, that's ok - chunk might not be rendered yet
    // The visual will be synced from cache when the chunk is loaded later
}

/// Calculate which chunks are visible in the camera viewport
#[cfg(feature = "debug_chunks")]
fn calculate_visible_chunks(
    camera_query: &Query<(&Transform, &Projection), With<Camera2d>>,
    window_query: &Query<&Window>,
) -> HashSet<ChunkPos> {
    let mut visible_chunks = HashSet::new();

    // Get camera data
    let Ok((camera_transform, projection)) = camera_query.single() else {
        return visible_chunks;
    };

    // Get window size
    let Ok(window) = window_query.single() else {
        return visible_chunks;
    };

    // Get zoom scale from projection
    let scale = if let Projection::Orthographic(ref ortho) = projection {
        ortho.scale
    } else {
        return visible_chunks;
    };

    // Calculate visible area in world coordinates
    let camera_pos = camera_transform.translation.truncate();
    let half_width = (window.width() / 2.0) * scale;
    let half_height = (window.height() / 2.0) * scale;

    // Calculate visible bounds
    let min_x = camera_pos.x - half_width;
    let max_x = camera_pos.x + half_width;
    let min_y = camera_pos.y - half_height;
    let max_y = camera_pos.y + half_height;

    // Convert to chunk coordinates
    let min_chunk = coords::world_to_chunk(Vec2::new(min_x, min_y));
    let max_chunk = coords::world_to_chunk(Vec2::new(max_x, max_y));

    // Collect all chunks that intersect with the visible area
    for x in min_chunk.x..=max_chunk.x {
        for y in min_chunk.y..=max_chunk.y {
            visible_chunks.insert(ChunkPos::new(x, y));
        }
    }

    visible_chunks
}

/// Calculate the appropriate chunk load radius based on camera zoom level
/// Returns a radius that covers the visible area plus a buffer for smooth loading
fn calculate_load_radius(
    camera_query: &Query<(&Transform, &Projection), With<Camera2d>>,
    window_query: &Query<&Window>,
) -> i32 {
    // Get camera data
    let Ok((_, projection)) = camera_query.single() else {
        return CHUNK_LOAD_RADIUS;
    };

    // Get window size
    let Ok(window) = window_query.single() else {
        return CHUNK_LOAD_RADIUS;
    };

    // Get zoom scale from projection
    let scale = if let Projection::Orthographic(ref ortho) = projection {
        ortho.scale
    } else {
        return CHUNK_LOAD_RADIUS;
    };

    // Calculate visible area in world coordinates
    let half_width = (window.width() / 2.0) * scale;
    let half_height = (window.height() / 2.0) * scale;

    // Calculate how many chunks are visible in each direction
    let chunks_horizontal = (half_width / crate::tiles::CHUNK_PIXEL_SIZE).ceil() as i32;
    let chunks_vertical = (half_height / crate::tiles::CHUNK_PIXEL_SIZE).ceil() as i32;

    // Use the larger dimension and add buffer of 2 chunks for smooth loading
    let visible_radius = chunks_horizontal.max(chunks_vertical);
    let load_radius = visible_radius + 2;

    // Ensure minimum radius of CHUNK_LOAD_RADIUS for close zoom
    load_radius.max(CHUNK_LOAD_RADIUS)
}

/// Calculate the unload radius based on load radius with hysteresis buffer
/// Always maintains +2 chunk buffer above load radius to prevent thrashing
fn calculate_unload_radius(load_radius: i32) -> i32 {
    load_radius + 2
}

/// Print a visual representation of loaded chunks
#[cfg(feature = "debug_chunks")]
fn print_chunk_grid(
    world: &WorldManager,
    camera_chunk: ChunkPos,
    visible_chunks: HashSet<ChunkPos>,
    load_radius: i32,
) {
    // Determine the range to display (show area around camera)
    // Use load_radius + 1 to show chunks just outside the load area
    let view_radius = (load_radius + 1).max(6); // Show at least 13x13 grid centered on camera
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
            let is_visible = visible_chunks.contains(&pos);
            let is_in_load_radius = camera_chunk.chebyshev_distance(&pos) <= load_radius;

            let symbol = if is_camera {
                " @ " // Camera position
            } else if is_visible && is_loaded {
                " ■ " // Visible and loaded chunk
            } else if is_visible {
                " □ " // Visible but not loaded
            } else if is_loaded && is_in_load_radius {
                " █ " // Loaded chunk in load radius
            } else if is_loaded {
                " ▓ " // Loaded chunk outside load radius (about to unload)
            } else if is_in_load_radius {
                " ░ " // Should be loaded but isn't (transitioning)
            } else {
                " · " // Not loaded
            };

            grid.push_str(symbol);
        }
        grid.push('\n');
    }

    grid.push_str("╚══════════════════════════════════════════╝\n");
    grid.push_str("Legend: @ = Camera  ■ = Visible+Loaded  □ = Visible  █ = Loaded  ░ = Loading  · = Unloaded\n");
    grid.push_str(&format!(
        "Loaded: {} | Visible: {} | Camera: {:?} | Load Radius: {} | Unload Radius: {}\n",
        loaded_chunks.len(),
        visible_chunks.len(),
        camera_chunk,
        load_radius,
        calculate_unload_radius(load_radius)
    ));

    info!("{}", grid);
}

pub fn update_tilemap(
    time: Res<Time>,
    mut query: Query<(&mut TilemapChunkTileData, &mut UpdateTimer)>,
) {
    use rand::prelude::*;

    let mut rng = rand::rng();
    for (mut tile_data, mut timer) in query.iter_mut() {
        timer.tick(time.delta());

        info!("Randomizing the chunk!");

        if timer.just_finished() {
            for _ in 0..50 {
                let index = rng.random_range(0..tile_data.len());
                tile_data[index] = Some(TileData::from_tileset_index(rng.random_range(0..2)));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tiles::{LAYER_GROUND, TILE_EMPTY, TILE_GRASS};
    use crate::world::savegame::{WorldElement, WorldGenerationConfig, WorldShape};
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_world_dir() -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("worldseed-loader-{unique}"));
        let _ = fs::remove_dir_all(&path);
        path
    }

    #[test]
    fn missing_island_chunks_load_as_empty() {
        let root = temp_world_dir();
        let mut world = WorldManager::new(root.clone());
        world.generation = WorldGenerationConfig {
            shape: WorldShape::Island,
            element: WorldElement::Grass,
            power: 1,
            seed: 77,
        };

        let chunk = load_chunk_data_only(&mut world, ChunkPos::new(0, 0));
        assert_eq!(chunk.get_tile(LAYER_GROUND, 0, 0), Some(TILE_EMPTY));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn missing_infinite_chunks_still_generate() {
        let root = temp_world_dir();
        let mut world = WorldManager::new(root.clone());
        world.generation = WorldGenerationConfig {
            shape: WorldShape::Infinity,
            element: WorldElement::Grass,
            ..WorldGenerationConfig::default()
        };

        let chunk = load_chunk_data_only(&mut world, ChunkPos::new(0, 0));
        assert_eq!(chunk.get_tile(LAYER_GROUND, 0, 0), Some(TILE_GRASS));

        let _ = fs::remove_dir_all(root);
    }
}
