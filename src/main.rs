use bevy::{
    input::mouse::MouseWheel,
    prelude::*,
    sprite_render::TilemapChunk,
    window::PrimaryWindow,
};

mod entities;
mod map;
mod tiles;
mod world;

use entities::{
    animate_sprite, apply_velocity, snail_dirt_trail, spawn_forest_guardian, spawn_player,
    spawn_snail, spawn_tree_spirit, sync_position_with_transform, update_animation_from_direction,
    update_direction_from_velocity, update_roaming_behavior, update_state_from_velocity,
    update_tree_growth, update_tree_spawning, update_winding_path, Position, TreeVariant,
};
use map::MapPlugin;
use world::{loader, WorldManager};

// UI sprite vertical offsets for proper centering
const HUMAN_SPRITE_OFFSET: f32 = 1.0;
const FOREST_GUARDIAN_SPRITE_OFFSET: f32 = 10.0;
const SNAIL_SPRITE_OFFSET: f32 = 10.0;
const SNAIL_SPRITE_OFFSET_X: f32 = 10.0;

// Camera zoom configuration
const ZOOM_MIN: f32 = 0.5;  // Max zoom in (smaller = more zoomed in)
const ZOOM_MAX: f32 = 3.0;  // Max zoom out (larger = more zoomed out)
const ZOOM_SPEED: f32 = 0.1; // Zoom change per input

// UI marker components
#[derive(Component)]
struct GuardianSubmenu;

#[derive(Component)]
struct GuardianButton;

// Entity type identifier for buttons
#[derive(Component, Clone, Debug)]
enum EntityType {
    Player,
    ForestGuardian(String), // Variant name: "oak", "birch", etc.
    Snail,
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

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(MapPlugin)
        .init_resource::<WorldManager>()
        .init_resource::<PlacementMode>()
        .add_systems(Startup, (setup_world, setup_ui))
        .add_systems(
            Update,
            (
                // Asset and rendering updates
                update_tileset_image,
                // AI behaviors (before velocity application)
                update_roaming_behavior,
                update_winding_path,
                // Entity state updates
                apply_velocity,
                update_state_from_velocity,
                update_direction_from_velocity,
                update_animation_from_direction,
                sync_position_with_transform.after(apply_velocity),
                // Entity interactions with world
                snail_dirt_trail.after(sync_position_with_transform),
                // Tree spawning and growth
                update_tree_spawning,
                update_tree_growth,
                // Animation
                animate_sprite,
                // Camera controls
                move_camera,
                zoom_camera,
                // Entity placement
                handle_entity_placement,
                update_button_selection,
                // World management
                loader::update_camera_chunk,
                loader::load_chunks_around_camera.after(loader::update_camera_chunk),
                loader::unload_distant_chunks.after(loader::load_chunks_around_camera),
                loader::apply_tile_modifications.after(snail_dirt_trail),
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

fn update_tileset_image(
    chunk_query: Query<&TilemapChunk>,
    mut events: MessageReader<AssetEvent<Image>>,
    mut images: ResMut<Assets<Image>>,
) {
    for event in events.read() {
        // Check if any chunk uses this texture
        for chunk in chunk_query.iter() {
            if event.is_loaded_with_dependencies(chunk.tileset.id()) {
                if let Some(image) = images.get_mut(&chunk.tileset) {
                    // Reinterpret the vertically-stacked texture as an array texture with 2 layers
                    // terrain_array.png is 8x16 (two 8x8 tiles stacked)
                    image.reinterpret_stacked_2d_as_array(2);
                    info!("Tileset reinterpreted as 2-layer array texture");
                }
                break; // Only need to reinterpret once per texture
            }
        }
    }
}


/// Camera movement system for testing chunk loading
fn move_camera(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
) {
    if let Ok(mut transform) = camera_query.single_mut() {
        let speed = 200.0; // pixels per second
        let delta = time.delta_secs();

        if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
            transform.translation.y += speed * delta;
        }
        if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
            transform.translation.y -= speed * delta;
        }
        if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
            transform.translation.x -= speed * delta;
        }
        if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
            transform.translation.x += speed * delta;
        }
    }
}

/// Camera zoom system - supports scroll wheel and keyboard (- and = keys)
fn zoom_camera(
    mut scroll_events: MessageReader<MouseWheel>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut camera_query: Query<&mut Projection, With<Camera2d>>,
) {
    if let Ok(mut projection) = camera_query.single_mut() {
        let mut zoom_delta = 0.0;

        // Handle scroll wheel input
        for event in scroll_events.read() {
            zoom_delta -= event.y * ZOOM_SPEED;
        }

        // Handle keyboard input (- to zoom out, = to zoom in)
        if keyboard.just_pressed(KeyCode::Minus) {
            zoom_delta += ZOOM_SPEED;
        }
        if keyboard.just_pressed(KeyCode::Equal) {
            zoom_delta -= ZOOM_SPEED;
        }

        // Apply zoom delta and clamp to bounds
        if zoom_delta != 0.0 {
            if let Projection::Orthographic(ref mut ortho) = projection.as_mut() {
                ortho.scale = (ortho.scale + zoom_delta).clamp(ZOOM_MIN, ZOOM_MAX);
            }
        }
    }
}

fn setup_ui(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    // Root UI container on the left side
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            left: Val::Px(0.0),
            top: Val::Px(0.0),
            bottom: Val::Px(0.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Start,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(10.0),
            ..default()
        })
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
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.2, 0.3)),
                    BorderColor::all(Color::srgb(0.4, 0.4, 0.6)),
                    BorderRadius::all(Val::Px(4.0)),
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
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(10.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
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
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.15, 0.3, 0.15)),
                        BorderColor::all(Color::srgb(0.3, 0.6, 0.3)),
                        BorderRadius::all(Val::Px(4.0)),
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
                    let guardian_layout_submenu =
                        TextureAtlasLayout::from_grid(UVec2::splat(32), 8, 4, None, None);
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
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(10.0),
                            ..default()
                        },
                    ))
                    .with_children(|submenu| {
                        for (name, filename) in guardians.iter() {
                            let texture =
                                assets.load(format!("creatures/forest_guardians/{}", filename));
                            let layout = texture_atlas_layouts.add(guardian_layout_submenu.clone());
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
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(0.15, 0.3, 0.15)),
                                    BorderColor::all(Color::srgb(0.3, 0.6, 0.3)),
                                    BorderRadius::all(Val::Px(4.0)),
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
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.25, 0.2, 0.25)),
                    BorderColor::all(Color::srgb(0.5, 0.4, 0.5)),
                    BorderRadius::all(Val::Px(4.0)),
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

            // Button 4 - Empty
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(64.0),
                        height: Val::Px(64.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
                    BorderColor::all(Color::srgb(0.4, 0.4, 0.4)),
                    BorderRadius::all(Val::Px(4.0)),
                ))
                .observe(button_interaction);
        });
}

fn button_interaction(
    trigger: On<Pointer<Click>>,
    buttons: Query<&EntityType, With<Button>>,
    mut placement_mode: ResMut<PlacementMode>,
) {
    if let Ok(entity_type) = buttons.get(trigger.entity) {
        // Toggle selection - if already selected, deselect; otherwise select
        if placement_mode.is_selected(entity_type) {
            placement_mode.deselect();
            info!("Deselected entity placement");
        } else {
            placement_mode.select(entity_type.clone());
            info!("Selected entity type for placement: {:?}", entity_type);
        }
    }
}

fn guardian_button_right_click(
    _trigger: On<Pointer<Click>>,
    mut submenu_query: Query<&mut Node, With<GuardianSubmenu>>,
) {
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
    let Ok((camera, camera_transform, projection)) = camera_query.single() else {
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
            info!("Spawned {} forest guardian at ({}, {})", variant, world_pos.x, world_pos.y);
        }
        EntityType::Snail => {
            spawn_snail(&mut commands, position, &assets, &mut texture_atlas_layouts);
            info!("Spawned snail at ({}, {})", world_pos.x, world_pos.y);
        }
    }
}
