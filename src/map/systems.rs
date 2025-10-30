use bevy::prelude::*;
use bevy::input::keyboard::KeyCode;
use super::{MapModal, MapState, MapConfig, MapContent, MAP_TILE_SIZE, MAP_TILESET_COLS, MAP_TILESET_ROWS, MAP_TILE_GRASS_PLAIN, MAP_TILE_DIRT, MAP_TILE_UNKNOWN};
use crate::world::WorldManager;
use crate::tiles::{ChunkPos, TILE_GRASS, TILE_DIRT, LAYER_GROUND, CHUNK_AREA};
use std::collections::HashMap;

/// Toggles map visibility when 'M' key is pressed
pub fn toggle_map_visibility(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut map_state: ResMut<MapState>,
    mut modal_query: Single<&mut Visibility, With<MapModal>>,
) {
    if keyboard.just_pressed(KeyCode::KeyM) {
        map_state.visible = !map_state.visible;

        **modal_query = if map_state.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

/// Marker component for dynamically spawned map tiles
#[derive(Component)]
pub struct MapTile;

/// Updates the map display based on loaded chunks
pub fn update_map_display(
    mut commands: Commands,
    map_state: Res<MapState>,
    map_config: Res<MapConfig>,
    world_manager: Res<WorldManager>,
    map_content_query: Single<Entity, With<MapContent>>,
    existing_tiles: Query<Entity, With<MapTile>>,
    assets: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    // Only update when map becomes visible
    if !map_state.is_changed() || !map_state.visible {
        return;
    }

    // Clear existing map tiles
    for tile_entity in existing_tiles.iter() {
        commands.entity(tile_entity).despawn();
    }

    // Get the map content container
    let map_content = *map_content_query;

    // Load the map tileset
    let texture = assets.load("maps/Minifantasy_MapsLandAndSea.png");
    let layout = TextureAtlasLayout::from_grid(
        UVec2::splat(MAP_TILE_SIZE as u32),
        MAP_TILESET_COLS as u32,
        MAP_TILESET_ROWS as u32,
        None,
        None,
    );
    let texture_atlas_layout = texture_atlas_layouts.add(layout);

    // Convert loaded chunks to map tiles
    let map_tiles = convert_chunks_to_map_tiles(&world_manager, map_config.chunks_per_map_tile);

    // Find the bounds of the map
    let (min_x, max_x, min_y, max_y) = find_map_bounds(&map_tiles);

    // Render map tiles using actual sprites
    commands.entity(map_content).with_children(|parent| {
        // Create a grid container for map tiles
        parent.spawn((
            Node {
                display: Display::Grid,
                grid_template_columns: vec![GridTrack::auto(); (max_x - min_x + 1) as usize],
                grid_template_rows: vec![GridTrack::auto(); (max_y - min_y + 1) as usize],
                column_gap: Val::Px(0.0),
                row_gap: Val::Px(0.0),
                ..default()
            },
        )).with_children(|grid| {
            // Render tiles from top to bottom, left to right
            for y in (min_y..=max_y).rev() {
                for x in min_x..=max_x {
                    let map_pos = MapTilePos { x, y };
                    let tile_index = if let Some(chunks) = map_tiles.get(&map_pos) {
                        // Loaded chunks - analyze terrain to determine map tile
                        determine_map_tile_from_chunks(chunks, &world_manager)
                    } else {
                        // Unloaded/unknown - use deep water for unexplored areas
                        MAP_TILE_UNKNOWN
                    };

                    grid.spawn((
                        MapTile,
                        ImageNode {
                            image: texture.clone(),
                            texture_atlas: Some(TextureAtlas {
                                layout: texture_atlas_layout.clone(),
                                index: tile_index,
                            }),
                            ..default()
                        },
                        Node {
                            width: Val::Px(MAP_TILE_SIZE),
                            height: Val::Px(MAP_TILE_SIZE),
                            ..default()
                        },
                    ));
                }
            }
        });

        // Add map legend/info
        parent.spawn((
            Text::new(format!(
                "Map Coverage: {} tiles | Chunks per tile: {} | Terrain-aware rendering (Grass/Dirt)",
                map_tiles.len(),
                map_config.chunks_per_map_tile
            )),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(Color::srgb(0.8, 0.8, 0.8)),
            Node {
                margin: UiRect::top(Val::Px(10.0)),
                ..default()
            },
        ));
    });
}

/// Represents a position in the map tile grid
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct MapTilePos {
    x: i32,
    y: i32,
}

/// Convert loaded world chunks to map tile positions
fn convert_chunks_to_map_tiles(
    world_manager: &WorldManager,
    chunks_per_tile: u32,
) -> HashMap<MapTilePos, Vec<ChunkPos>> {
    let mut map_tiles: HashMap<MapTilePos, Vec<ChunkPos>> = HashMap::new();
    let divisor = chunks_per_tile as i32;

    for chunk_pos in world_manager.active_chunks.keys() {
        // Convert chunk position to map tile position
        let map_x = chunk_pos.x.div_euclid(divisor);
        let map_y = chunk_pos.y.div_euclid(divisor);
        let map_pos = MapTilePos { x: map_x, y: map_y };

        map_tiles.entry(map_pos)
            .or_insert_with(Vec::new)
            .push(*chunk_pos);
    }

    map_tiles
}

/// Find the bounding box of map tiles
fn find_map_bounds(map_tiles: &HashMap<MapTilePos, Vec<ChunkPos>>) -> (i32, i32, i32, i32) {
    if map_tiles.is_empty() {
        return (0, 0, 0, 0);
    }

    let mut min_x = i32::MAX;
    let mut max_x = i32::MIN;
    let mut min_y = i32::MAX;
    let mut max_y = i32::MIN;

    for pos in map_tiles.keys() {
        min_x = min_x.min(pos.x);
        max_x = max_x.max(pos.x);
        min_y = min_y.min(pos.y);
        max_y = max_y.max(pos.y);
    }

    (min_x, max_x, min_y, max_y)
}

/// Analyze chunks to determine which map tile to display
/// Returns the appropriate map tile index based on terrain composition
fn determine_map_tile_from_chunks(chunks: &[ChunkPos], world_manager: &WorldManager) -> usize {
    let mut total_grass = 0;
    let mut total_dirt = 0;
    let mut total_tiles = 0;

    // Analyze all chunks that contribute to this map tile
    for chunk_pos in chunks {
        if let Some(chunk_data) = world_manager.chunk_cache.get(chunk_pos) {
            // Count terrain types on the ground layer
            for tile_id in chunk_data.layers[LAYER_GROUND].iter() {
                total_tiles += 1;
                match *tile_id {
                    TILE_GRASS => total_grass += 1,
                    TILE_DIRT => total_dirt += 1,
                    _ => {} // Ignore empty tiles
                }
            }
        }
    }

    // If no tiles were analyzed, default to grass
    if total_tiles == 0 {
        return MAP_TILE_GRASS_PLAIN;
    }

    // Calculate percentages
    let grass_percentage = (total_grass as f32 / total_tiles as f32) * 100.0;
    let dirt_percentage = (total_dirt as f32 / total_tiles as f32) * 100.0;

    // Determine map tile based on dominant terrain
    // If more than 50% dirt, show as dirt
    if dirt_percentage > 50.0 {
        MAP_TILE_DIRT
    } else if grass_percentage > 30.0 {
        // More than 30% grass shows as grass
        MAP_TILE_GRASS_PLAIN
    } else {
        // Mixed or sparse - default to grass
        MAP_TILE_GRASS_PLAIN
    }
}
