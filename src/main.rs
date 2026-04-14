use bevy::{
    input::mouse::MouseWheel, picking::pointer::PointerButton, prelude::*,
    sprite::Anchor, sprite_render::TilemapChunk, window::PrimaryWindow,
};
use std::fs;

mod entities;
mod map;
mod tiles;
mod world;

use entities::{
    animate_sprite, apply_velocity, snail_dirt_trail, spawn_forest_guardian, spawn_player,
    spawn_snail, spawn_tree_spirit, sync_world_render_transform, update_animation_from_direction,
    update_direction_from_velocity, update_guardian_animation_from_state, update_roaming_behavior,
    update_state_from_velocity, update_tree_growth, update_tree_spawning, update_winding_path,
    Direction, Position, TreeVariant, Velocity, WorldRenderDepth,
};
use map::MapPlugin;
use tiles::constants::{LAYER_GROUND, TILE_DIRT, TILE_GRASS, TILE_WORLD_SIZE};
use world::{loader, WorldManager};

// UI sprite vertical offsets for proper centering
const HUMAN_SPRITE_OFFSET: f32 = 1.0;
const FOREST_GUARDIAN_SPRITE_OFFSET: f32 = 10.0;
const SNAIL_SPRITE_OFFSET: f32 = 10.0;
const SNAIL_SPRITE_OFFSET_X: f32 = 10.0;

// Camera zoom configuration
const ZOOM_MIN: f32 = 0.5; // Max zoom in (smaller = more zoomed in)
const ZOOM_MAX: f32 = 3.0; // Max zoom out (larger = more zoomed out)
const ZOOM_SPEED: f32 = 0.1; // Zoom change per input

// Camera movement configuration
const BASE_PAN_SPEED: f32 = 200.0; // Base speed for panning when at minimum zoom

// UI marker components
#[derive(Component)]
struct GuardianSubmenu;

#[derive(Component)]
struct GuardianButton;

#[derive(Component)]
struct TerrainSubmenu;

#[derive(Component)]
struct TerrainButton;

// Entity type identifier for buttons
#[derive(Component, Clone, Debug)]
enum EntityType {
    Player,
    ForestGuardian(String), // Variant name: "oak", "birch", etc.
    Snail,
}

// Terrain type identifier for terrain painting
#[derive(Component, Clone, Debug, PartialEq)]
enum TerrainType {
    Grass,
    Dirt,
}

// Placement mode resource - tracks which entity type is selected for placement
#[derive(Resource, Default, Clone, Debug)]
struct PlacementMode {
    selected: Option<EntityType>,
}

impl PlacementMode {
    fn select(&mut self, entity_type: EntityType) {
        self.selected = Some(entity_type);
    }

    fn deselect(&mut self) {
        self.selected = None;
    }

    fn is_selected(&self, entity_type: &EntityType) -> bool {
        if let Some(ref selected) = self.selected {
            match (selected, entity_type) {
                (EntityType::Player, EntityType::Player) => true,
                (EntityType::Snail, EntityType::Snail) => true,
                (EntityType::ForestGuardian(a), EntityType::ForestGuardian(b)) => a == b,
                _ => false,
            }
        } else {
            false
        }
    }
}

// Paint mode resource - tracks which terrain type is selected for painting
#[derive(Resource, Default, Clone, Debug)]
struct PaintMode {
    selected: Option<TerrainType>,
}

impl PaintMode {
    fn select(&mut self, terrain_type: TerrainType) {
        self.selected = Some(terrain_type);
    }

    fn deselect(&mut self) {
        self.selected = None;
    }

    fn is_selected(&self, terrain_type: &TerrainType) -> bool {
        if let Some(ref selected) = self.selected {
            selected == terrain_type
        } else {
            false
        }
    }
}

#[derive(Resource, Default, Clone, Debug)]
struct PaintDragState {
    last_painted_tile: Option<IVec2>,
}

impl PaintDragState {
    fn reset(&mut self) {
        self.last_painted_tile = None;
    }
}

#[derive(Resource, Default, Clone, Debug)]
struct SpriteBoundsDebug {
    enabled: bool,
}

// Save notification resource - tracks save notification display state
#[derive(Resource)]
struct SaveNotification {
    timer: Timer,
    visible: bool,
}

impl Default for SaveNotification {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0.0, TimerMode::Once),
            visible: false,
        }
    }
}

impl SaveNotification {
    fn show(&mut self, duration: f32) {
        self.timer = Timer::from_seconds(duration, TimerMode::Once);
        self.visible = true;
    }

    fn tick(&mut self, delta: std::time::Duration) {
        if self.visible {
            self.timer.tick(delta);
            if self.timer.just_finished() {
                self.visible = false;
            }
        }
    }
}

// Marker component for save notification UI text
#[derive(Component)]
struct SaveNotificationText;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(MapPlugin)
        .init_resource::<WorldManager>()
        .init_resource::<loader::ChunkSaveTimer>()
        .init_resource::<PlacementMode>()
        .init_resource::<PaintMode>()
        .init_resource::<PaintDragState>()
        .init_resource::<SpriteBoundsDebug>()
        .init_resource::<SaveNotification>()
        // Register observer for ChunkDataChanged to sync visuals
        .add_observer(loader::sync_chunk_visuals_on_data_change)
        .add_systems(Startup, (setup_world, setup_ui, setup_save_notification_ui))
        .add_systems(
            Update,
            (
                // AI behaviors (before velocity application)
                update_roaming_behavior,
                update_winding_path,
                // Entity state updates
                apply_velocity,
                update_state_from_velocity,
                // Guardian animation switching (after state, before direction)
                update_guardian_animation_from_state.after(update_state_from_velocity),
                update_direction_from_velocity,
                update_animation_from_direction,
                sync_world_render_transform.after(apply_velocity),
                // Entity interactions with world
                snail_dirt_trail.after(sync_world_render_transform),
                // Tree spawning and growth
                update_tree_spawning,
                update_tree_growth,
                // Entity interactions with world
                // Animation
                animate_sprite,
                zoom_camera,
                move_camera,
                // loader::update_tilemap,
            ),
        )
        .add_systems(
            Update,
            (
                // Entity placement and terrain painting
                handle_entity_placement,
                handle_terrain_painting,
                update_button_selection,
                update_terrain_button_selection,
                // Save notification
                handle_manual_save,
                update_save_notification,
                toggle_sprite_bounds_debug,
                // World management
                loader::update_camera_chunk,
                loader::load_chunks_around_camera.after(loader::update_camera_chunk),
                loader::unload_distant_chunks.after(loader::load_chunks_around_camera),
                loader::apply_tile_modifications,
                loader::autosave_dirty_chunks,
                draw_snail_debug_bounds,
                // Map reset
                reset_world_map,
            ),
        )
        .run();
}

fn setup_world(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    // Spawn camera at origin
    commands.spawn((Camera2d, Transform::from_xyz(0.0, 0.0, 999.0)));

    // Spawn player character at world origin
    spawn_player(
        &mut commands,
        Position::new(0.0, 0.0),
        &assets,
        &mut texture_atlas_layouts,
    );

    // Spawn forest guardian to the left
    spawn_forest_guardian(
        &mut commands,
        Position::new(-100.0, 0.0),
        "oak",
        &assets,
        &mut texture_atlas_layouts,
    );

    // Spawn snail to the right
    spawn_snail(
        &mut commands,
        Position::new(100.0, 0.0),
        &assets,
        &mut texture_atlas_layouts,
    );

    // Spawn a test tree spirit above the player - grows every 3 seconds per stage
    spawn_tree_spirit(
        &mut commands,
        Position::new(0.0, 100.0),
        TreeVariant::Oak,
        3.0, // 3 seconds per growth stage
        &assets,
        &mut texture_atlas_layouts,
    );

    info!("World setup complete with entities using position and state components");
}

/// Orthographic zoom avoids camera transform scaling issues with tilemap rendering.
fn zoom_camera(
    mut scroll_events: MessageReader<MouseWheel>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut camera_query: Query<&mut Projection, With<Camera2d>>,
) {
    if let Ok(mut projection) = camera_query.single_mut() {
        let mut zoom_delta = 0.0;

        for event in scroll_events.read() {
            zoom_delta -= event.y * ZOOM_SPEED;
        }

        if keyboard.just_pressed(KeyCode::Minus) {
            zoom_delta += ZOOM_SPEED;
        }
        if keyboard.just_pressed(KeyCode::Equal) {
            zoom_delta -= ZOOM_SPEED;
        }

        if zoom_delta != 0.0 {
            if let Projection::Orthographic(ref mut ortho) = projection.as_mut() {
                ortho.scale = (ortho.scale + zoom_delta).clamp(ZOOM_MIN, ZOOM_MAX);
            }
        }
    }
}

/// Moves camera with arrow keys. Speed scales with zoom level.
fn move_camera(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut camera_query: Query<(&mut Transform, &Projection), With<Camera2d>>,
) {
    let Ok((mut transform, projection)) = camera_query.single_mut() else {
        return;
    };

    let zoom_scale = match projection {
        Projection::Orthographic(ortho) => ortho.scale,
        _ => 1.0,
    };

    // Scale speed relative to minimum zoom (most zoomed in = baseline speed)
    let effective_speed = BASE_PAN_SPEED * (zoom_scale / ZOOM_MIN);
    let mut direction = Vec2::ZERO;

    if keyboard.pressed(KeyCode::ArrowUp) {
        direction.y += 1.0;
    }
    if keyboard.pressed(KeyCode::ArrowDown) {
        direction.y -= 1.0;
    }
    if keyboard.pressed(KeyCode::ArrowLeft) {
        direction.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::ArrowRight) {
        direction.x += 1.0;
    }

    if direction != Vec2::ZERO {
        direction = direction.normalize();
        transform.translation.x += direction.x * effective_speed * time.delta_secs();
        transform.translation.y += direction.y * effective_speed * time.delta_secs();
    }
}

fn setup_ui(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    // Root UI container on the left side
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(16.0),
                top: Val::Px(16.0),
                bottom: Val::Px(16.0),
                width: Val::Px(82.0),
                padding: UiRect::all(Val::Px(9.0)),
                border: UiRect::all(Val::Px(1.0)),
                border_radius: BorderRadius::all(Val::Px(12.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Start,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.1, 0.12, 0.88)),
            BorderColor::all(Color::srgba(0.72, 0.8, 0.76, 0.22)),
        ))
        .with_children(|parent| {
            // Load textures for UI buttons
            let guardian_texture = assets.load("creatures/forest_guardians/oak_guardian_idle.png");
            let guardian_layout = TextureAtlasLayout::from_grid(UVec2::splat(32), 8, 4, None, None);
            let guardian_atlas_layout = texture_atlas_layouts.add(guardian_layout);

            let human_texture = assets.load("characters/human_walk.png");
            let human_layout = TextureAtlasLayout::from_grid(UVec2::splat(32), 4, 4, None, None);
            let human_atlas_layout = texture_atlas_layouts.add(human_layout);

            let snail_texture = assets.load("creatures/snail/snail_crawl.png");
            let snail_layout = TextureAtlasLayout::from_grid(UVec2::splat(32), 4, 4, None, None);
            let snail_atlas_layout = texture_atlas_layouts.add(snail_layout);

            // Button 1 - With Human sprite
            parent
                .spawn((
                    Button,
                    EntityType::Player,
                    Node {
                        width: Val::Px(64.0),
                        height: Val::Px(64.0),
                        display: Display::Flex,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(0.0)),
                        border_radius: BorderRadius::all(Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.2, 0.3)),
                    BorderColor::all(Color::srgb(0.4, 0.4, 0.6)),
                ))
                .observe(button_interaction)
                .with_children(|button| {
                    button.spawn((
                        ImageNode {
                            image: human_texture.clone(),
                            image_mode: NodeImageMode::Stretch,
                            texture_atlas: Some(TextureAtlas {
                                layout: human_atlas_layout.clone(),
                                index: 0,
                            }),
                            ..default()
                        },
                        Node {
                            width: Val::Px(64.0),
                            height: Val::Px(64.0),
                            margin: UiRect {
                                top: Val::Px(HUMAN_SPRITE_OFFSET),
                                ..default()
                            },
                            ..default()
                        },
                    ));
                });

            // Button 2 - With Forest Guardian sprite (with submenu row)
            parent
                .spawn((Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(0.0),
                    align_items: AlignItems::Center,
                    ..default()
                },))
                .with_children(|row| {
                    // Main guardian button
                    row.spawn((
                        Button,
                        GuardianButton,
                        EntityType::ForestGuardian("oak".to_string()),
                        Node {
                            width: Val::Px(64.0),
                            height: Val::Px(64.0),
                            display: Display::Flex,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::Px(0.0)),
                            border_radius: BorderRadius::all(Val::Px(4.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.15, 0.3, 0.15)),
                        BorderColor::all(Color::srgb(0.3, 0.6, 0.3)),
                    ))
                    .observe(button_interaction)
                    .observe(guardian_button_right_click)
                    .with_children(|button| {
                        // Add guardian sprite directly
                        button.spawn((
                            ImageNode {
                                image: guardian_texture.clone(),
                                image_mode: NodeImageMode::Stretch,
                                texture_atlas: Some(TextureAtlas {
                                    layout: guardian_atlas_layout.clone(),
                                    index: 0, // First frame
                                }),
                                ..default()
                            },
                            Node {
                                width: Val::Px(64.0),
                                height: Val::Px(64.0),
                                margin: UiRect {
                                    top: Val::Px(FOREST_GUARDIAN_SPRITE_OFFSET),
                                    ..default()
                                },
                                ..default()
                            },
                        ));
                    });

                    // Submenu container (initially hidden)
                    let guardians = [
                        ("Oak", "oak_guardian_idle.png"),
                        ("Birch", "birch_guardian_idle.png"),
                        ("Hickory", "hickory_guardian_idle.png"),
                        ("Pine", "pine_guardian_idle.png"),
                        ("Willow", "willow_guardian_idle.png"),
                    ];

                    row.spawn((
                        GuardianSubmenu,
                        Node {
                            display: Display::None, // Hidden by default
                            margin: UiRect::left(Val::Px(-9.0)),
                            padding: UiRect::all(Val::Px(9.0)),
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(10.0),
                            border_radius: BorderRadius::all(Val::Px(12.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.08, 0.1, 0.12, 0.88)),
                    ))
                    .with_children(|submenu| {
                        for (name, filename) in guardians.iter() {
                            let texture =
                                assets.load(format!("creatures/forest_guardians/{}", filename));
                            let layout = texture_atlas_layouts.add(
                                TextureAtlasLayout::from_grid(UVec2::splat(32), 8, 4, None, None),
                            );
                            let variant = name.to_lowercase();

                            submenu
                                .spawn((
                                    Button,
                                    EntityType::ForestGuardian(variant),
                                    Node {
                                        width: Val::Px(64.0),
                                        height: Val::Px(64.0),
                                        display: Display::Flex,
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        padding: UiRect::all(Val::Px(0.0)),
                                        border_radius: BorderRadius::all(Val::Px(4.0)),
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(0.15, 0.3, 0.15)),
                                    BorderColor::all(Color::srgb(0.3, 0.6, 0.3)),
                                ))
                                .observe(button_interaction)
                                .with_children(|button| {
                                    button.spawn((
                                        ImageNode {
                                            image: texture.clone(),
                                            image_mode: NodeImageMode::Stretch,
                                            texture_atlas: Some(TextureAtlas {
                                                layout: layout.clone(),
                                                index: 0,
                                            }),
                                            ..default()
                                        },
                                        Node {
                                            width: Val::Px(64.0),
                                            height: Val::Px(64.0),
                                            margin: UiRect {
                                                top: Val::Px(FOREST_GUARDIAN_SPRITE_OFFSET),
                                                ..default()
                                            },
                                            ..default()
                                        },
                                    ));
                                });
                        }
                    });
                });

            // Button 3 - With Snail sprite
            parent
                .spawn((
                    Button,
                    EntityType::Snail,
                    Node {
                        width: Val::Px(64.0),
                        height: Val::Px(64.0),
                        display: Display::Flex,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        padding: UiRect::all(Val::Px(0.0)),
                        border_radius: BorderRadius::all(Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.25, 0.2, 0.25)),
                    BorderColor::all(Color::srgb(0.5, 0.4, 0.5)),
                ))
                .observe(button_interaction)
                .with_children(|button| {
                    button.spawn((
                        ImageNode {
                            image: snail_texture.clone(),
                            image_mode: NodeImageMode::Stretch,
                            texture_atlas: Some(TextureAtlas {
                                layout: snail_atlas_layout.clone(),
                                index: 0,
                            }),
                            ..default()
                        },
                        Node {
                            width: Val::Px(64.0),
                            height: Val::Px(64.0),
                            margin: UiRect {
                                top: Val::Px(SNAIL_SPRITE_OFFSET),
                                left: Val::Px(SNAIL_SPRITE_OFFSET_X),
                                ..default()
                            },
                            ..default()
                        },
                    ));
                });

            // Button 4 - Terrain painting (with submenu row)
            parent
                .spawn((Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(0.0),
                    align_items: AlignItems::Center,
                    ..default()
                },))
                .with_children(|row| {
                    // Load terrain tileset for UI (separate file - won't be reinterpreted as array texture)
                    // terrain_array_ui.png is 8x16 pixels = 2 tiles stacked vertically (8x8 each)
                    let terrain_ui_texture = assets.load("tilesets/terrain_array_ui.png");
                    let terrain_ui_layout =
                        TextureAtlasLayout::from_grid(UVec2::splat(8), 1, 2, None, None);
                    let terrain_ui_atlas_layout = texture_atlas_layouts.add(terrain_ui_layout);

                    // Main terrain button (starts with grass)
                    row.spawn((
                        Button,
                        TerrainButton,
                        TerrainType::Grass,
                        Node {
                            width: Val::Px(64.0),
                            height: Val::Px(64.0),
                            display: Display::Flex,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            padding: UiRect::all(Val::Px(0.0)),
                            border_radius: BorderRadius::all(Val::Px(4.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.2, 0.3, 0.2)),
                        BorderColor::all(Color::srgb(0.4, 0.6, 0.4)),
                    ))
                    .observe(terrain_button_interaction)
                    .observe(terrain_button_right_click)
                    .with_children(|button| {
                        // Add grass tile sprite (index 0 in atlas = TILE_GRASS in world)
                        button.spawn((
                            ImageNode {
                                image: terrain_ui_texture.clone(),
                                image_mode: NodeImageMode::Stretch,
                                texture_atlas: Some(TextureAtlas {
                                    layout: terrain_ui_atlas_layout.clone(),
                                    index: 0, // First tile in atlas = grass
                                }),
                                ..default()
                            },
                            Node {
                                width: Val::Px(64.0),
                                height: Val::Px(64.0),
                                ..default()
                            },
                        ));
                    });

                    // Submenu container (initially hidden)
                    row.spawn((
                        TerrainSubmenu,
                        Node {
                            display: Display::None, // Hidden by default
                            margin: UiRect::left(Val::Px(-9.0)),
                            padding: UiRect::all(Val::Px(9.0)),
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(10.0),
                            border_radius: BorderRadius::all(Val::Px(12.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.08, 0.1, 0.12, 0.88)),
                    ))
                    .with_children(|submenu| {
                        // Grass button
                        submenu
                            .spawn((
                                Button,
                                TerrainType::Grass,
                                Node {
                                    width: Val::Px(64.0),
                                    height: Val::Px(64.0),
                                    display: Display::Flex,
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    padding: UiRect::all(Val::Px(0.0)),
                                    border_radius: BorderRadius::all(Val::Px(4.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.2, 0.3, 0.2)),
                                BorderColor::all(Color::srgb(0.4, 0.6, 0.4)),
                            ))
                            .observe(terrain_button_interaction)
                            .with_children(|button| {
                                button.spawn((
                                    ImageNode {
                                        image: terrain_ui_texture.clone(),
                                        image_mode: NodeImageMode::Stretch,
                                        texture_atlas: Some(TextureAtlas {
                                            layout: terrain_ui_atlas_layout.clone(),
                                            index: 0, // First tile = grass
                                        }),
                                        ..default()
                                    },
                                    Node {
                                        width: Val::Px(64.0),
                                        height: Val::Px(64.0),
                                        ..default()
                                    },
                                ));
                            });

                        // Dirt button
                        submenu
                            .spawn((
                                Button,
                                TerrainType::Dirt,
                                Node {
                                    width: Val::Px(64.0),
                                    height: Val::Px(64.0),
                                    display: Display::Flex,
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    padding: UiRect::all(Val::Px(0.0)),
                                    border_radius: BorderRadius::all(Val::Px(4.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.2, 0.3, 0.2)),
                                BorderColor::all(Color::srgb(0.4, 0.6, 0.4)),
                            ))
                            .observe(terrain_button_interaction)
                            .with_children(|button| {
                                button.spawn((
                                    ImageNode {
                                        image: terrain_ui_texture.clone(),
                                        image_mode: NodeImageMode::Stretch,
                                        texture_atlas: Some(TextureAtlas {
                                            layout: terrain_ui_atlas_layout.clone(),
                                            index: 1, // Second tile = dirt
                                        }),
                                        ..default()
                                    },
                                    Node {
                                        width: Val::Px(64.0),
                                        height: Val::Px(64.0),
                                        ..default()
                                    },
                                ));
                            });
                    });
                });
        });
}

fn button_interaction(
    trigger: On<Pointer<Click>>,
    mut param_set: ParamSet<(
        Query<(&EntityType, Option<&GuardianButton>), With<Button>>,
        Query<(&mut EntityType, &Children), With<GuardianButton>>,
    )>,
    mut placement_mode: ResMut<PlacementMode>,
    mut paint_mode: ResMut<PaintMode>,
    mut submenu_query: Query<&mut Node, With<GuardianSubmenu>>,
    mut image_query: Query<&mut ImageNode>,
    assets: Res<AssetServer>,
) {
    // First, get the clicked button's info
    let button_info = param_set
        .p0()
        .get(trigger.entity)
        .ok()
        .map(|(et, gb)| (et.clone(), gb.is_none()));

    if let Some((entity_type, is_not_main_guardian)) = button_info {
        // Check if this is a guardian variant from the submenu (not the main guardian button)
        let is_submenu_guardian =
            matches!(entity_type, EntityType::ForestGuardian(_)) && is_not_main_guardian;

        if is_submenu_guardian {
            // Guardian variant selected from submenu - close menu and update main button
            if let Ok(mut submenu_node) = submenu_query.single_mut() {
                submenu_node.display = Display::None;
            }

            // Update the main guardian button's EntityType and icon
            if let Ok((mut guardian_entity_type, children)) = param_set.p1().single_mut() {
                *guardian_entity_type = entity_type.clone();

                // Update the icon texture
                if let EntityType::ForestGuardian(variant) = &entity_type {
                    let texture_path =
                        format!("creatures/forest_guardians/{}_guardian_idle.png", variant);
                    let new_texture = assets.load(&texture_path);

                    // Find and update the child ImageNode
                    for child in children {
                        if let Ok(mut image_node) = image_query.get_mut(*child) {
                            image_node.image = new_texture.clone();
                            info!("Updated guardian button icon to {} variant", variant);
                            break;
                        }
                    }
                }
            }
        }

        // Clear terrain paint mode when selecting entity
        paint_mode.deselect();

        // Toggle selection - if already selected, deselect; otherwise select
        if placement_mode.is_selected(&entity_type) {
            placement_mode.deselect();
            info!("Deselected entity placement");
        } else {
            placement_mode.select(entity_type.clone());
            info!("Selected entity type for placement: {:?}", entity_type);
        }
    }
}

fn guardian_button_right_click(
    trigger: On<Pointer<Click>>,
    mut submenu_query: Query<&mut Node, With<GuardianSubmenu>>,
) {
    // Only respond to right-click (Secondary button)
    if trigger.event().button != PointerButton::Secondary {
        return;
    }

    // Toggle submenu visibility
    if let Ok(mut node) = submenu_query.single_mut() {
        node.display = if node.display == Display::None {
            Display::Flex
        } else {
            Display::None
        };
    }
}

fn terrain_button_interaction(
    trigger: On<Pointer<Click>>,
    mut param_set: ParamSet<(
        Query<(&TerrainType, Option<&TerrainButton>), With<Button>>,
        Query<(&mut TerrainType, &Children), With<TerrainButton>>,
    )>,
    mut paint_mode: ResMut<PaintMode>,
    mut placement_mode: ResMut<PlacementMode>,
    mut submenu_query: Query<&mut Node, With<TerrainSubmenu>>,
    mut image_query: Query<&mut ImageNode>,
) {
    // First, get the clicked button's info
    let button_info = param_set
        .p0()
        .get(trigger.entity)
        .ok()
        .map(|(tt, tb)| (tt.clone(), tb.is_none()));

    if let Some((terrain_type, is_not_main_terrain)) = button_info {
        // Check if this is a terrain variant from the submenu (not the main terrain button)
        let is_submenu_terrain = is_not_main_terrain;

        if is_submenu_terrain {
            // Terrain variant selected from submenu - close menu and update main button
            if let Ok(mut submenu_node) = submenu_query.single_mut() {
                submenu_node.display = Display::None;
            }

            // Update the main terrain button's TerrainType and icon
            if let Ok((mut terrain_button_type, children)) = param_set.p1().single_mut() {
                *terrain_button_type = terrain_type.clone();

                // Update the icon texture atlas index (0-based, not tile IDs)
                let atlas_index = match terrain_type {
                    TerrainType::Grass => 0, // First tile in atlas
                    TerrainType::Dirt => 1,  // Second tile in atlas
                };

                // Find and update the child ImageNode's texture atlas index
                for child in children {
                    if let Ok(mut image_node) = image_query.get_mut(*child) {
                        if let Some(ref mut atlas) = image_node.texture_atlas {
                            atlas.index = atlas_index;
                            info!("Updated terrain button icon to {:?} terrain", terrain_type);
                        }
                        break;
                    }
                }
            }
        }

        // Clear entity placement mode when selecting terrain
        placement_mode.deselect();

        // Toggle selection - if already selected, deselect; otherwise select
        if paint_mode.is_selected(&terrain_type) {
            paint_mode.deselect();
            info!("Deselected terrain painting");
        } else {
            paint_mode.select(terrain_type.clone());
            info!("Selected terrain type for painting: {:?}", terrain_type);
        }
    }
}

fn terrain_button_right_click(
    trigger: On<Pointer<Click>>,
    mut submenu_query: Query<&mut Node, With<TerrainSubmenu>>,
) {
    // Only respond to right-click (Secondary button)
    if trigger.event().button != PointerButton::Secondary {
        return;
    }

    // Toggle submenu visibility
    if let Ok(mut node) = submenu_query.single_mut() {
        node.display = if node.display == Display::None {
            Display::Flex
        } else {
            Display::None
        };
    }
}

/// Updates button visual feedback based on placement mode selection
fn update_button_selection(
    placement_mode: Res<PlacementMode>,
    mut buttons: Query<(&EntityType, &mut BackgroundColor, &mut BorderColor), With<Button>>,
) {
    // Only update if placement mode changed
    if !placement_mode.is_changed() {
        return;
    }

    for (entity_type, mut bg_color, mut border_color) in buttons.iter_mut() {
        let is_selected = placement_mode.is_selected(entity_type);

        // Update colors based on entity type and selection state
        match entity_type {
            EntityType::Player => {
                if is_selected {
                    *bg_color = BackgroundColor(Color::srgb(0.3, 0.3, 0.5));
                    *border_color = BorderColor::all(Color::srgb(0.6, 0.6, 1.0));
                } else {
                    *bg_color = BackgroundColor(Color::srgb(0.2, 0.2, 0.3));
                    *border_color = BorderColor::all(Color::srgb(0.4, 0.4, 0.6));
                }
            }
            EntityType::ForestGuardian(_) => {
                if is_selected {
                    *bg_color = BackgroundColor(Color::srgb(0.25, 0.5, 0.25));
                    *border_color = BorderColor::all(Color::srgb(0.5, 1.0, 0.5));
                } else {
                    *bg_color = BackgroundColor(Color::srgb(0.15, 0.3, 0.15));
                    *border_color = BorderColor::all(Color::srgb(0.3, 0.6, 0.3));
                }
            }
            EntityType::Snail => {
                if is_selected {
                    *bg_color = BackgroundColor(Color::srgb(0.4, 0.3, 0.4));
                    *border_color = BorderColor::all(Color::srgb(0.8, 0.6, 0.8));
                } else {
                    *bg_color = BackgroundColor(Color::srgb(0.25, 0.2, 0.25));
                    *border_color = BorderColor::all(Color::srgb(0.5, 0.4, 0.5));
                }
            }
        }
    }
}

/// Updates terrain button visual feedback based on paint mode selection
fn update_terrain_button_selection(
    paint_mode: Res<PaintMode>,
    mut buttons: Query<(&TerrainType, &mut BackgroundColor, &mut BorderColor), With<Button>>,
) {
    // Only update if paint mode changed
    if !paint_mode.is_changed() {
        return;
    }

    for (terrain_type, mut bg_color, mut border_color) in buttons.iter_mut() {
        let is_selected = paint_mode.is_selected(terrain_type);

        // Update colors based on selection state
        if is_selected {
            *bg_color = BackgroundColor(Color::srgb(0.3, 0.5, 0.3)); // Brighter when selected
            *border_color = BorderColor::all(Color::srgb(0.6, 1.0, 0.6));
        } else {
            *bg_color = BackgroundColor(Color::srgb(0.2, 0.3, 0.2)); // Standard color
            *border_color = BorderColor::all(Color::srgb(0.4, 0.6, 0.4));
        }
    }
}

/// Handles mouse clicks to place entities in the world
fn handle_entity_placement(
    placement_mode: Res<PlacementMode>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform, &Projection), With<Camera2d>>,
    ui_query: Query<&Interaction, With<Button>>,
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    // Only handle left clicks when an entity type is selected
    if !mouse_button.just_pressed(MouseButton::Left) {
        return;
    }

    let Some(ref entity_type) = placement_mode.selected else {
        return;
    };

    // Don't spawn entities if cursor is over any UI element
    for interaction in ui_query.iter() {
        if *interaction == Interaction::Pressed || *interaction == Interaction::Hovered {
            return;
        }
    }

    // Get the primary window
    let Ok(window) = windows.single() else {
        return;
    };

    // Get cursor position in window
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    // Get camera components
    let Ok((camera, camera_transform, _projection)) = camera_query.single() else {
        return;
    };

    // Convert cursor position to world position
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };

    // Spawn the entity at the world position
    let position = Position::new(world_pos.x, world_pos.y);

    match entity_type {
        EntityType::Player => {
            spawn_player(&mut commands, position, &assets, &mut texture_atlas_layouts);
            info!("Spawned player at ({}, {})", world_pos.x, world_pos.y);
        }
        EntityType::ForestGuardian(variant) => {
            spawn_forest_guardian(
                &mut commands,
                position,
                variant,
                &assets,
                &mut texture_atlas_layouts,
            );
            info!(
                "Spawned {} forest guardian at ({}, {})",
                variant, world_pos.x, world_pos.y
            );
        }
        EntityType::Snail => {
            spawn_snail(&mut commands, position, &assets, &mut texture_atlas_layouts);
            info!("Spawned snail at ({}, {})", world_pos.x, world_pos.y);
        }
    }
}

/// Handles mouse clicks to paint terrain in the world
fn handle_terrain_painting(
    paint_mode: Res<PaintMode>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    ui_query: Query<&Interaction, With<Button>>,
    mut paint_drag_state: ResMut<PaintDragState>,
    mut world_manager: ResMut<WorldManager>,
) {
    if !mouse_button.pressed(MouseButton::Left) {
        paint_drag_state.reset();
        return;
    }

    let Some(ref terrain_type) = paint_mode.selected else {
        paint_drag_state.reset();
        return;
    };

    // Don't paint terrain if cursor is over any UI element
    for interaction in ui_query.iter() {
        if *interaction == Interaction::Pressed || *interaction == Interaction::Hovered {
            paint_drag_state.reset();
            return;
        }
    }

    // Get the primary window
    let Ok(window) = windows.single() else {
        paint_drag_state.reset();
        return;
    };

    // Get cursor position in window
    let Some(cursor_pos) = window.cursor_position() else {
        paint_drag_state.reset();
        return;
    };

    // Get camera components
    let Ok((camera, camera_transform)) = camera_query.single() else {
        paint_drag_state.reset();
        return;
    };

    // Convert cursor position to world position
    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        paint_drag_state.reset();
        return;
    };

    let tile_pos = world_to_tile_pos(world_pos);
    if paint_drag_state.last_painted_tile == Some(tile_pos) {
        return;
    }

    // Determine which tile to paint based on terrain type
    let tile_id = match terrain_type {
        TerrainType::Grass => TILE_GRASS,
        TerrainType::Dirt => TILE_DIRT,
    };

    // Queue the tile modification on the ground layer
    world_manager.queue_tile_modification(world_pos.x, world_pos.y, tile_id, LAYER_GROUND);
    paint_drag_state.last_painted_tile = Some(tile_pos);
    info!(
        "Painted {:?} tile at ({}, {})",
        terrain_type, world_pos.x, world_pos.y
    );
}

fn world_to_tile_pos(world_pos: Vec2) -> IVec2 {
    IVec2::new(
        (world_pos.x / TILE_WORLD_SIZE).floor() as i32,
        (world_pos.y / TILE_WORLD_SIZE).floor() as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_to_tile_pos_handles_positive_and_negative_coordinates() {
        assert_eq!(world_to_tile_pos(Vec2::new(0.0, 0.0)), IVec2::new(0, 0));
        assert_eq!(world_to_tile_pos(Vec2::new(31.9, 31.9)), IVec2::new(0, 0));
        assert_eq!(world_to_tile_pos(Vec2::new(32.0, 32.0)), IVec2::new(1, 1));
        assert_eq!(world_to_tile_pos(Vec2::new(-0.1, -0.1)), IVec2::new(-1, -1));
        assert_eq!(world_to_tile_pos(Vec2::new(-32.0, -32.0)), IVec2::new(-1, -1));
    }
}

/// Setup UI for save notification in top right corner
fn setup_save_notification_ui(mut commands: Commands) {
    // Create text container in top right corner
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            right: Val::Px(20.0),
            top: Val::Px(20.0),
            padding: UiRect::all(Val::Px(10.0)),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                SaveNotificationText,
                Text::new("Saving..."),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
                Node {
                    padding: UiRect::all(Val::Px(8.0)),
                    ..default()
                },
                Visibility::Hidden, // Start hidden
            ));
        });
}

/// System to handle 's' key press and trigger manual save
fn handle_manual_save(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut save_notification: ResMut<SaveNotification>,
    mut world: ResMut<WorldManager>,
) {
    use world::serialization;

    // Only trigger on 's' key press
    if !keyboard.just_pressed(KeyCode::KeyS) {
        return;
    }

    let dirty_chunks = world.get_dirty_chunks();
    if dirty_chunks.is_empty() {
        info!("Manual save triggered but no dirty chunks to save");
        return;
    }

    info!("Manual save: saving {} dirty chunks", dirty_chunks.len());

    let mut saved_count = 0;
    let mut failed_count = 0;

    for chunk_pos in dirty_chunks {
        if let Some(chunk_data) = world.get_cached_chunk(&chunk_pos) {
            let chunk_path = world.get_chunk_path(&chunk_pos);
            match serialization::save_chunk(chunk_data, &chunk_path) {
                Ok(_) => {
                    debug!("Manually saved chunk {:?}", chunk_pos);
                    world.clear_dirty(&chunk_pos);
                    saved_count += 1;
                }
                Err(e) => {
                    error!("Failed to manually save chunk {:?}: {}", chunk_pos, e);
                    failed_count += 1;
                }
            }
        }
    }

    info!(
        "Manual save complete: {} saved, {} failed",
        saved_count, failed_count
    );

    // Show notification for duration of save + 2 seconds
    // We'll estimate save duration based on chunk count (very fast for our case)
    // So we'll just show for 2 seconds total
    save_notification.show(2.0);
}

/// System to update save notification visibility based on timer
fn update_save_notification(
    time: Res<Time>,
    mut save_notification: ResMut<SaveNotification>,
    mut query: Query<&mut Visibility, With<SaveNotificationText>>,
) {
    // Tick the notification timer
    save_notification.tick(time.delta());

    // Update visibility based on notification state
    if let Ok(mut visibility) = query.single_mut() {
        *visibility = if save_notification.visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

fn toggle_sprite_bounds_debug(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut sprite_bounds_debug: ResMut<SpriteBoundsDebug>,
) {
    if keyboard.just_pressed(KeyCode::KeyB) {
        sprite_bounds_debug.enabled = !sprite_bounds_debug.enabled;
        info!(
            "Sprite bounds debug {}",
            if sprite_bounds_debug.enabled {
                "enabled"
            } else {
                "disabled"
            }
        );
    }
}

fn draw_snail_debug_bounds(
    mut gizmos: Gizmos,
    sprite_bounds_debug: Res<SpriteBoundsDebug>,
    images: Res<Assets<Image>>,
    texture_atlas_layouts: Res<Assets<TextureAtlasLayout>>,
    sprite_query: Query<
        (
            &Sprite,
            Option<&Anchor>,
            &Transform,
            Option<&Position>,
            Option<&Velocity>,
            Option<&Direction>,
        ),
        With<WorldRenderDepth>,
    >,
) {
    if !sprite_bounds_debug.enabled {
        return;
    }

    for (sprite, anchor, transform, position, velocity, direction) in sprite_query.iter() {
        let Some(base_size) = sprite_debug_size(sprite, &images, &texture_atlas_layouts) else {
            continue;
        };

        let world_size = base_size * transform.scale.truncate().abs();
        let anchor = anchor.copied().unwrap_or_default();
        let sprite_center = transform.translation.truncate() - anchor.as_vec() * world_size;

        gizmos.rect_2d(
            Isometry2d::from_translation(sprite_center),
            world_size,
            Color::srgb(1.0, 0.2, 0.2),
        );

        gizmos.circle_2d(transform.translation.truncate(), 4.0, Color::srgb(0.2, 1.0, 0.2));

        if let Some(position) = position {
            gizmos.circle_2d(
                Vec2::new(position.x, position.y),
                3.0,
                Color::srgb(0.2, 0.6, 1.0),
            );
        }

        if let (Some(velocity), Some(_direction)) = (velocity, direction) {
            if velocity.magnitude() > 0.1 {
                draw_direction_gizmo(
                    &mut gizmos,
                    transform.translation.truncate(),
                    Vec2::new(velocity.x, velocity.y).normalize(),
                );
            }
        }
    }
}

fn draw_direction_gizmo(gizmos: &mut Gizmos, origin: Vec2, direction: Vec2) {
    const DIRECTION_GIZMO_LENGTH: f32 = 18.0;
    const DIRECTION_GIZMO_HEAD_LENGTH: f32 = 6.0;
    const DIRECTION_GIZMO_HEAD_ANGLE: f32 = 0.45;

    let tip = origin + direction * DIRECTION_GIZMO_LENGTH;
    let head_back = direction * DIRECTION_GIZMO_HEAD_LENGTH;
    let left_head = head_back.rotate(Vec2::from_angle(DIRECTION_GIZMO_HEAD_ANGLE));
    let right_head = head_back.rotate(Vec2::from_angle(-DIRECTION_GIZMO_HEAD_ANGLE));
    let color = Color::srgb(1.0, 0.85, 0.2);

    gizmos.line_2d(origin, tip, color);
    gizmos.line_2d(tip, tip - left_head, color);
    gizmos.line_2d(tip, tip - right_head, color);
}

fn sprite_debug_size(
    sprite: &Sprite,
    images: &Assets<Image>,
    texture_atlas_layouts: &Assets<TextureAtlasLayout>,
) -> Option<Vec2> {
    if let Some(custom_size) = sprite.custom_size {
        return Some(custom_size);
    }

    if let Some(texture_atlas) = &sprite.texture_atlas {
        if let Some(layout) = texture_atlas_layouts.get(&texture_atlas.layout) {
            if let Some(rect) = layout.textures.get(texture_atlas.index) {
                let size = rect.max - rect.min;
                return Some(size.as_vec2());
            }
        }
    }

    if let Some(rect) = sprite.rect {
        return Some(rect.size());
    }

    images.get(&sprite.image).map(|image| image.size().as_vec2())
}

/// System to reset the world map when 'R' key is pressed
fn reset_world_map(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut world_manager: ResMut<WorldManager>,
    mut camera_query: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
    chunk_entities: Query<Entity, With<TilemapChunk>>,
    mut commands: Commands,
) {
    // Only trigger on R key press
    if !keyboard.just_pressed(KeyCode::KeyR) {
        return;
    }

    info!("Resetting world map...");

    // 1. Despawn all active chunk entities (all 3 layers)
    for entity in chunk_entities.iter() {
        commands.entity(entity).despawn();
    }

    // 2. Clear WorldManager state
    world_manager.active_chunks.clear();
    world_manager.dirty_chunks.clear();
    world_manager.chunk_cache.clear();
    world_manager.pending_tile_modifications.clear();
    world_manager.camera_chunk = None;

    // 3. Delete save directory from disk
    let save_dir = &world_manager.save_directory;
    if save_dir.exists() {
        if let Err(e) = fs::remove_dir_all(save_dir) {
            warn!("Failed to delete save directory: {}", e);
        } else {
            info!("Deleted save directory: {:?}", save_dir);
        }
    }

    // 4. Reset camera to origin and zoom to default (1.0)
    if let Ok((mut transform, mut projection)) = camera_query.single_mut() {
        transform.translation.x = 0.0;
        transform.translation.y = 0.0;
        if let Projection::Orthographic(ref mut ortho) = projection.as_mut() {
            ortho.scale = 1.0;
        }
    }

    info!("World map reset complete");
}
