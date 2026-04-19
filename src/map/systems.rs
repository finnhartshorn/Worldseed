use super::{
    MapConfig, MapContent, MapDynamicContent, MapModal, MapState, MINIMAP_DIRT_COLOR,
    MINIMAP_EMPTY_COLOR, MINIMAP_GRASS_COLOR, MINIMAP_MAX_DISPLAY_HEIGHT_RATIO,
    MINIMAP_MAX_DISPLAY_WIDTH_RATIO, MINIMAP_MAX_PIXEL_SIZE, MINIMAP_UNKNOWN_COLOR,
};
use crate::tiles::{
    ChunkPos, TileId, CHUNK_SIZE_I32, LAYER_GROUND, TILE_DIRT, TILE_EMPTY, TILE_GRASS,
};
use crate::world::WorldManager;
use bevy::input::keyboard::KeyCode;
use bevy::prelude::*;

/// Toggles map visibility when 'M' key is pressed.
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

/// Cycles the minimap sample size while the modal is open.
pub fn cycle_map_resolution(
    keyboard: Res<ButtonInput<KeyCode>>,
    map_config: Res<MapConfig>,
    mut map_state: ResMut<MapState>,
) {
    if !map_state.visible || map_config.sample_sizes_in_tiles.is_empty() {
        return;
    }

    if keyboard.just_pressed(KeyCode::Equal) && map_state.active_sample_size_index > 0 {
        map_state.active_sample_size_index -= 1;
    }

    if keyboard.just_pressed(KeyCode::Minus)
        && map_state.active_sample_size_index + 1 < map_config.sample_sizes_in_tiles.len()
    {
        map_state.active_sample_size_index += 1;
    }
}

/// Marker component for dynamic minimap pixels.
#[derive(Component)]
pub struct MapTile;

/// Updates the minimap display from loaded chunk data.
pub fn update_map_display(
    mut commands: Commands,
    map_state: Res<MapState>,
    map_config: Res<MapConfig>,
    world_manager: Res<WorldManager>,
    map_content_query: Single<Entity, With<MapContent>>,
    existing_dynamic_content: Query<Entity, With<MapDynamicContent>>,
    primary_window: Single<&Window>,
) {
    if !map_state.visible {
        return;
    }

    let should_update = map_state.is_changed() || world_manager.is_changed();
    if !should_update {
        return;
    }

    for entity in &existing_dynamic_content {
        commands.entity(entity).despawn();
    }

    let map_content = *map_content_query;

    let Some(bounds) = loaded_tile_bounds(&world_manager) else {
        spawn_empty_state(&mut commands, map_content);
        return;
    };

    let sample_size = map_state.active_sample_size(&map_config).max(1);
    let sample_bounds = SampleBounds::from_tile_bounds(bounds, sample_size as i32);
    let samples = build_minimap_samples(&world_manager, sample_bounds, sample_size as i32);

    let columns = sample_bounds.width() as usize;
    let rows = sample_bounds.height() as usize;
    let pixel_size = minimap_pixel_size(*primary_window, columns, rows);
    let grid_width = pixel_size * columns as f32;
    let grid_height = pixel_size * rows as f32;

    commands.entity(map_content).with_children(|parent| {
        parent
            .spawn((
                MapDynamicContent,
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(12.0),
                    ..default()
                },
            ))
            .with_children(|root| {
                root.spawn((
                    Node {
                        width: Val::Px(grid_width),
                        height: Val::Px(grid_height),
                        display: Display::Grid,
                        grid_template_columns: vec![GridTrack::auto(); columns],
                        grid_template_rows: vec![GridTrack::auto(); rows],
                        justify_content: JustifyContent::Center,
                        align_content: AlignContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.04, 0.05, 0.06)),
                ))
                .with_children(|grid| {
                    for sample in samples {
                        grid.spawn((
                            MapTile,
                            Node {
                                width: Val::Px(pixel_size),
                                height: Val::Px(pixel_size),
                                ..default()
                            },
                            BackgroundColor(sample.to_color()),
                        ));
                    }
                });

                root.spawn((
                    Text::new(format!(
                        "Loaded area: {}x{} tiles | Minimap: {}x{} px | Sample: {}x{} tiles/pixel",
                        bounds.width(),
                        bounds.height(),
                        columns,
                        rows,
                        sample_size,
                        sample_size
                    )),
                    TextFont {
                        font_size: 14.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.8, 0.8, 0.8)),
                ));
            });
    });
}

fn spawn_empty_state(commands: &mut Commands, map_content: Entity) {
    commands.entity(map_content).with_children(|parent| {
        parent.spawn((
            MapDynamicContent,
            Text::new("No loaded chunks available for the minimap yet."),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::srgb(0.75, 0.75, 0.78)),
        ));
    });
}

fn minimap_pixel_size(window: &Window, columns: usize, rows: usize) -> f32 {
    let max_width = window.width() * MINIMAP_MAX_DISPLAY_WIDTH_RATIO;
    let max_height = window.height() * MINIMAP_MAX_DISPLAY_HEIGHT_RATIO;
    let width_limited = max_width / columns.max(1) as f32;
    let height_limited = max_height / rows.max(1) as f32;

    width_limited
        .min(height_limited)
        .floor()
        .clamp(1.0, MINIMAP_MAX_PIXEL_SIZE)
}

fn loaded_tile_bounds(world_manager: &WorldManager) -> Option<TileBounds> {
    let mut chunks = world_manager.active_chunks.keys().copied();
    let first = chunks.next()?;

    let mut min_chunk_x = first.x;
    let mut max_chunk_x = first.x;
    let mut min_chunk_y = first.y;
    let mut max_chunk_y = first.y;

    for chunk in chunks {
        min_chunk_x = min_chunk_x.min(chunk.x);
        max_chunk_x = max_chunk_x.max(chunk.x);
        min_chunk_y = min_chunk_y.min(chunk.y);
        max_chunk_y = max_chunk_y.max(chunk.y);
    }

    Some(TileBounds {
        min_x: min_chunk_x * CHUNK_SIZE_I32,
        max_x: (max_chunk_x + 1) * CHUNK_SIZE_I32 - 1,
        min_y: min_chunk_y * CHUNK_SIZE_I32,
        max_y: (max_chunk_y + 1) * CHUNK_SIZE_I32 - 1,
    })
}

fn build_minimap_samples(
    world_manager: &WorldManager,
    sample_bounds: SampleBounds,
    sample_size: i32,
) -> Vec<MapSample> {
    let mut samples = Vec::with_capacity((sample_bounds.width() * sample_bounds.height()) as usize);

    for sample_y in (sample_bounds.min_y..=sample_bounds.max_y).rev() {
        for sample_x in sample_bounds.min_x..=sample_bounds.max_x {
            let world_tile_x = sample_x * sample_size;
            let world_tile_y = sample_y * sample_size;
            samples.push(average_sample_color(
                world_manager,
                world_tile_x,
                world_tile_y,
                sample_size,
            ));
        }
    }

    samples
}

fn average_sample_color(
    world_manager: &WorldManager,
    start_tile_x: i32,
    start_tile_y: i32,
    sample_size: i32,
) -> MapSample {
    let mut sum = Vec3::ZERO;
    let mut populated_tiles = 0;
    let mut known_tiles = 0;

    for local_y in 0..sample_size {
        for local_x in 0..sample_size {
            let tile_x = start_tile_x + local_x;
            let tile_y = start_tile_y + local_y;

            if let Some(tile_id) = get_ground_tile(world_manager, tile_x, tile_y) {
                known_tiles += 1;

                if tile_id != TILE_EMPTY {
                    let color = tile_color(tile_id);
                    sum += Vec3::new(color[0], color[1], color[2]);
                    populated_tiles += 1;
                }
            }
        }
    }

    if populated_tiles > 0 {
        let average = sum / populated_tiles as f32;
        MapSample::Known([average.x, average.y, average.z])
    } else if known_tiles > 0 {
        MapSample::Known(MINIMAP_EMPTY_COLOR)
    } else {
        MapSample::Unknown
    }
}

fn get_ground_tile(world_manager: &WorldManager, tile_x: i32, tile_y: i32) -> Option<TileId> {
    let chunk_pos = ChunkPos::from_tile(IVec2::new(tile_x, tile_y), CHUNK_SIZE_I32);
    let chunk_data = world_manager.chunk_cache.get(&chunk_pos)?;
    let local_x = tile_x.rem_euclid(CHUNK_SIZE_I32) as usize;
    let local_y = tile_y.rem_euclid(CHUNK_SIZE_I32) as usize;

    chunk_data.get_tile(LAYER_GROUND, local_x, local_y)
}

fn tile_color(tile_id: TileId) -> [f32; 3] {
    match tile_id {
        TILE_GRASS => MINIMAP_GRASS_COLOR,
        TILE_DIRT => MINIMAP_DIRT_COLOR,
        TILE_EMPTY => MINIMAP_EMPTY_COLOR,
        _ => MINIMAP_UNKNOWN_COLOR,
    }
}

#[derive(Debug, Clone, Copy)]
enum MapSample {
    Known([f32; 3]),
    Unknown,
}

impl MapSample {
    fn to_color(self) -> Color {
        match self {
            Self::Known([r, g, b]) => Color::srgb(r, g, b),
            Self::Unknown => Color::srgb(
                MINIMAP_UNKNOWN_COLOR[0],
                MINIMAP_UNKNOWN_COLOR[1],
                MINIMAP_UNKNOWN_COLOR[2],
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TileBounds {
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
}

impl TileBounds {
    fn width(self) -> i32 {
        self.max_x - self.min_x + 1
    }

    fn height(self) -> i32 {
        self.max_y - self.min_y + 1
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SampleBounds {
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
}

impl SampleBounds {
    fn from_tile_bounds(bounds: TileBounds, sample_size: i32) -> Self {
        Self {
            min_x: bounds.min_x.div_euclid(sample_size),
            max_x: bounds.max_x.div_euclid(sample_size),
            min_y: bounds.min_y.div_euclid(sample_size),
            max_y: bounds.max_y.div_euclid(sample_size),
        }
    }

    fn width(self) -> i32 {
        self.max_x - self.min_x + 1
    }

    fn height(self) -> i32 {
        self.max_y - self.min_y + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tiles::{ChunkData, ChunkPos, CHUNK_AREA};
    use std::collections::{HashMap, HashSet};
    use std::path::PathBuf;

    fn world_manager_with_chunk_data(chunks: Vec<ChunkData>) -> WorldManager {
        let mut active_chunks = HashMap::new();
        let mut chunk_cache = HashMap::new();

        for chunk in chunks {
            let pos = chunk.position;
            active_chunks.insert(pos, [Entity::PLACEHOLDER; crate::tiles::NUM_LAYERS]);
            chunk_cache.insert(pos, chunk);
        }

        WorldManager {
            active_chunks,
            dirty_chunks: HashSet::new(),
            chunk_cache,
            save_directory: PathBuf::from("test"),
            camera_chunk: None,
            generation: crate::world::WorldGenerationConfig::default(),
            pending_tile_modifications: Vec::new(),
        }
    }

    #[test]
    fn loaded_bounds_cover_negative_and_positive_chunks() {
        let world_manager = world_manager_with_chunk_data(vec![
            ChunkData::filled(ChunkPos::new(-1, 0), TILE_GRASS),
            ChunkData::filled(ChunkPos::new(1, 2), TILE_GRASS),
        ]);

        let bounds = loaded_tile_bounds(&world_manager).unwrap();

        assert_eq!(
            bounds,
            TileBounds {
                min_x: -32,
                max_x: 63,
                min_y: 0,
                max_y: 95,
            }
        );
    }

    #[test]
    fn sample_bounds_round_outward_for_negative_tiles() {
        let bounds = TileBounds {
            min_x: -1,
            max_x: 3,
            min_y: -4,
            max_y: 1,
        };

        let sample_bounds = SampleBounds::from_tile_bounds(bounds, 2);

        assert_eq!(
            sample_bounds,
            SampleBounds {
                min_x: -1,
                max_x: 1,
                min_y: -2,
                max_y: 0,
            }
        );
    }

    #[test]
    fn average_sample_color_blends_tiles() {
        let mut chunk = ChunkData::filled(ChunkPos::new(0, 0), TILE_EMPTY);
        assert!(chunk.set_tile(LAYER_GROUND, 0, 0, TILE_GRASS));
        assert!(chunk.set_tile(LAYER_GROUND, 1, 0, TILE_GRASS));
        assert!(chunk.set_tile(LAYER_GROUND, 0, 1, TILE_DIRT));
        assert!(chunk.set_tile(LAYER_GROUND, 1, 1, TILE_DIRT));

        let world_manager = world_manager_with_chunk_data(vec![chunk]);
        let sample = average_sample_color(&world_manager, 0, 0, 2);

        match sample {
            MapSample::Known([r, g, b]) => {
                assert!(
                    (r - ((MINIMAP_GRASS_COLOR[0] + MINIMAP_DIRT_COLOR[0]) / 2.0)).abs() < 0.001
                );
                assert!(
                    (g - ((MINIMAP_GRASS_COLOR[1] + MINIMAP_DIRT_COLOR[1]) / 2.0)).abs() < 0.001
                );
                assert!(
                    (b - ((MINIMAP_GRASS_COLOR[2] + MINIMAP_DIRT_COLOR[2]) / 2.0)).abs() < 0.001
                );
            }
            MapSample::Unknown => panic!("expected known sample color"),
        }
    }

    #[test]
    fn average_sample_color_uses_empty_color_for_loaded_empty_tiles() {
        let chunk = ChunkData::filled(ChunkPos::new(0, 0), TILE_EMPTY);
        let world_manager = world_manager_with_chunk_data(vec![chunk]);
        let sample = average_sample_color(&world_manager, 0, 0, 2);

        assert!(matches!(sample, MapSample::Known(color) if color == MINIMAP_EMPTY_COLOR));
    }

    #[test]
    fn average_sample_color_uses_unknown_color_for_unloaded_tiles() {
        let world_manager = world_manager_with_chunk_data(Vec::new());
        let sample = average_sample_color(&world_manager, 0, 0, 2);

        assert!(matches!(sample, MapSample::Unknown));
    }

    #[test]
    fn build_minimap_samples_creates_expected_grid_order() {
        let mut chunk = ChunkData::filled(ChunkPos::new(0, 0), TILE_EMPTY);
        chunk.layers[LAYER_GROUND] = [TILE_EMPTY; CHUNK_AREA];
        assert!(chunk.set_tile(LAYER_GROUND, 0, 31, TILE_GRASS));
        assert!(chunk.set_tile(LAYER_GROUND, 31, 0, TILE_DIRT));
        let world_manager = world_manager_with_chunk_data(vec![chunk]);

        let bounds = SampleBounds::from_tile_bounds(
            TileBounds {
                min_x: 0,
                max_x: 31,
                min_y: 0,
                max_y: 31,
            },
            32,
        );

        let samples = build_minimap_samples(&world_manager, bounds, 32);
        assert_eq!(samples.len(), 1);
    }
}
