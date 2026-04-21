use super::{MapContent, MapModal, MapState};
use bevy::prelude::*;

pub fn reset_map_state(mut map_state: ResMut<MapState>) {
    map_state.active_sample_size_index = MapState::default().active_sample_size_index;
}

pub fn cleanup_map_ui(mut commands: Commands, modal_query: Query<Entity, With<MapModal>>) {
    for entity in &modal_query {
        commands.entity(entity).despawn();
    }
}

/// Sets up the map modal UI (hidden by default)
pub fn setup_map_ui(mut commands: Commands) {
    // Root modal container (full screen overlay with semi-transparent background)
    commands
        .spawn((
            MapModal,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
            Visibility::Hidden, // Hidden by default
            ZIndex(1000),       // Ensure it's on top
        ))
        .with_children(|parent| {
            // Map content container (almost full screen, slightly inset)
            parent
                .spawn((
                    Node {
                        width: Val::Percent(95.0),
                        height: Val::Percent(95.0),
                        padding: UiRect::all(Val::Px(20.0)),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.15, 0.15, 0.2)),
                ))
                .with_children(|parent| {
                    // Title bar
                    parent.spawn((
                        Text::new("Loaded World Minimap (M closes, -/= changes resolution)"),
                        TextFont {
                            font_size: 24.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        Node {
                            margin: UiRect::bottom(Val::Px(20.0)),
                            ..default()
                        },
                    ));

                    // Map display area (will be populated dynamically by update_map_display)
                    parent.spawn((
                        MapContent,
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.1, 0.1, 0.15)),
                    ));
                });
        });
}
