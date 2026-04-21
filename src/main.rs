use bevy::{
    app::AppExit,
    image::{ImageLoaderSettings, ImageSampler},
    input::mouse::MouseWheel,
    picking::pointer::PointerButton,
    prelude::*,
    sprite::{Anchor, SpritePickingMode, SpritePickingSettings},
    sprite_render::TilemapChunk,
    window::PrimaryWindow,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(States, Default, Clone, Eq, PartialEq, Debug, Hash)]
pub enum AppState {
    #[default]
    MainMenu,
    LoadSlotSelect,
    SlotHub,
    LoadWorldSelect,
    DraftSetup,
    InGame,
}

mod draft;
mod entities;
mod map;
mod tiles;
mod world;

use draft::cards::{
    label_plate_height, set_icon as set_draft_card_icon, DraftCard, DraftCardCategory,
    VISIBLE_DRAFT_CARDS,
};
use entities::{
    animate_sprite, apply_velocity, snail_dirt_trail, spawn_forest_guardian, spawn_human,
    spawn_saved_forest_guardian, spawn_saved_human, spawn_saved_snail, spawn_saved_tree_spirit,
    spawn_saved_variant_tree, spawn_snail, spawn_tree_spirit, sync_world_render_transform,
    update_animation_from_direction, update_direction_from_velocity,
    update_guardian_animation_from_state, update_roaming_behavior, update_state_from_velocity,
    update_tree_growth, update_tree_spawning, update_winding_path, Direction, ForestGuardian,
    GrowingTree, Health, Human, Position, RoamingBehavior, SavedActorState, Snail, TreeSpawner,
    TreeSpirit, TreeVariant, VariantTree, Velocity, WindingPath, WorldClickableEntity,
    WorldRenderDepth,
};
use map::{MapModal, MapPlugin, MapState};
use tiles::constants::{LAYER_GROUND, TILE_DIRT, TILE_GRASS, TILE_WORLD_SIZE};
use world::{
    entities_save::{
        self, SavedActor, SavedEntity, SavedForestGuardian, SavedGrowingTreeEntity, SavedSnail,
        SavedWorldEntities,
    },
    loader, pregenerate_world_chunks,
    savegame::{self, SaveSlotMetadata, WorldMetadata},
    TerrainTransitionConfig, WorldElement, WorldGenerationConfig, WorldManager, WorldShape,
};

// UI sprite vertical offsets for proper centering
const HUMAN_SPRITE_OFFSET: f32 = 1.0;
const FOREST_GUARDIAN_SPRITE_OFFSET: f32 = 10.0;
const SNAIL_SPRITE_OFFSET: f32 = 10.0;
const SNAIL_SPRITE_OFFSET_X: f32 = 10.0;
const STYLIZED_UI_TEXTURE_PATH: &str = "ui/Stylized_UI.png";
const ICONS_UI_TEXTURE_PATH: &str = "ui/Icons_Only.png";
const BLOOM_POOL_SIZE: f32 = 64.0;
const BLOOM_HALO_EXTRA_SIZE: f32 = 28.0;
const BLOOM_HALO_DURATION: f32 = 0.35;
const WORLD_CLICK_HALO_EXTRA_SIZE: f32 = 28.0;
const WORLD_CLICK_HALO_PADDING: f32 = 12.0;
const WORLD_CLICK_HALO_DEPTH_OFFSET: f32 = 0.001;
const WORLD_ENTITY_ALPHA_THRESHOLD: f32 = 0.1;
const TIME_CONTROL_BUTTON_SIZE: f32 = 28.0;
const TIME_CONTROL_BUTTON_GAP: f32 = 10.0;
const TIME_CONTROL_BUTTON_BOTTOM: f32 = 2.0;
const TIME_CONTROL_PLAY_SPEED: f32 = 1.0;
const TIME_CONTROL_FAST_SPEED: f32 = 2.0;

// Camera zoom configuration
const ZOOM_MIN: f32 = 0.5; // Max zoom in (smaller = more zoomed in)
const ZOOM_MAX: f32 = 3.0; // Max zoom out (larger = more zoomed out)
const ZOOM_SPEED: f32 = 0.1; // Zoom change per input

// Camera movement configuration
const BASE_PAN_SPEED: f32 = 200.0; // Base speed for panning when at minimum zoom

const MAIN_MENU_UI_TEXTURE_PATH: &str = "ui/Classic_UI_Only.png";
const DRAFT_CARD_FRAME_TEXTURE_PATH: &str = "ui/Card_Frame_40x48.png";
const DRAFT_NAME_PLATE_TEXTURE_PATH: &str = "ui/Plate_Tileset_8x8.png";
const DRAFT_CARD_FRAME_TILE_SIZE: UVec2 = UVec2::new(40, 48);
const DRAFT_CARD_FRAME_COLUMNS: u32 = 2;
const DRAFT_CARD_FRAME_ROWS: u32 = 4;
const DRAFT_MAX_COLUMNS: usize = 5;
const DRAFT_MAX_ROWS: usize = 5;
const DRAFT_GRID_SLOT_WIDTH: f32 = 132.0;
const DRAFT_GRID_SLOT_HEIGHT: f32 = 156.0;
const DRAFT_GRID_CARD_WIDTH: f32 = 120.0;
const DRAFT_GRID_CARD_HEIGHT: f32 = 144.0;
const DRAFT_TRAY_CARD_HEIGHT: f32 = 164.0;
const DRAFT_NAME_PLATE_WIDTH: f32 = 92.0;
const DRAFT_CARD_ICON_SIZE: f32 = 52.0;
const DRAFT_LAYOUT_WIDTH: f32 = 736.0;
const DRAFT_LAYOUT_HEIGHT: f32 = 900.0;
const DRAFT_LAYOUT_HORIZONTAL_PADDING: f32 = 32.0;
const DRAFT_LAYOUT_VERTICAL_PADDING: f32 = 24.0;
const MAIN_MENU_BUTTON_HEIGHT: f32 = 70.0;
const MAIN_MENU_BUTTON_WIDTH: f32 = 260.0;
const MAIN_MENU_BUTTON_SOURCE_HEIGHT: f32 = 14.0;
const MAIN_MENU_BUTTON_SOURCE_WIDTH: f32 = MAIN_MENU_BUTTON_WIDTH / 5.0;
const MAIN_MENU_BUTTON_SCALE: f32 = 5.0;

fn load_main_menu_ui_texture(assets: &AssetServer) -> Handle<Image> {
    assets.load_with_settings(
        MAIN_MENU_UI_TEXTURE_PATH,
        |settings: &mut ImageLoaderSettings| {
            settings.sampler = ImageSampler::nearest();
        },
    )
}

fn load_stylized_ui_texture(assets: &AssetServer) -> Handle<Image> {
    assets.load_with_settings(
        STYLIZED_UI_TEXTURE_PATH,
        |settings: &mut ImageLoaderSettings| {
            settings.sampler = ImageSampler::nearest();
        },
    )
}

fn load_icons_ui_texture(assets: &AssetServer) -> Handle<Image> {
    assets.load_with_settings(
        ICONS_UI_TEXTURE_PATH,
        |settings: &mut ImageLoaderSettings| {
            settings.sampler = ImageSampler::nearest();
        },
    )
}

fn main_menu_button_clicked_rect() -> Rect {
    Rect::new(65.0, 17.0, 111.0, 31.0)
}

fn main_menu_button_hovered_rect() -> Rect {
    Rect::new(65.0, 241.0, 111.0, 255.0)
}

fn main_menu_button_standard_rect() -> Rect {
    Rect::new(65.0, 129.0, 111.0, 143.0)
}

fn main_menu_button_image_mode() -> NodeImageMode {
    NodeImageMode::Sliced(TextureSlicer {
        // The button art is 46 px wide; keeping 19 px caps leaves an 8 px center slice to tile.
        border: BorderRect::from([19.0, 19.0, 0.0, 0.0]),
        center_scale_mode: SliceScaleMode::Tile { stretch_value: 1.0 },
        sides_scale_mode: SliceScaleMode::Stretch,
        max_corner_scale: 1.0,
    })
}

fn main_menu_label_rect() -> Rect {
    Rect::new(385.0, 129.0, 431.0, 143.0)
}

fn circular_pool_background_rect() -> Rect {
    Rect::new(808.0, 240.0, 840.0, 272.0)
}

fn circular_pool_fill_rects() -> [Rect; 14] {
    std::array::from_fn(|index| {
        let x = 200.0 + (index as f32 * 32.0);
        Rect::new(x, 1744.0, x + 32.0, 1776.0)
    })
}

fn time_control_fast_forward_rect() -> Rect {
    Rect::new(16.0, 304.0, 24.0, 312.0)
}

fn time_control_play_rect() -> Rect {
    Rect::new(32.0, 304.0, 40.0, 312.0)
}

fn time_control_pause_rect() -> Rect {
    Rect::new(64.0, 304.0, 72.0, 312.0)
}

// UI marker components
#[derive(Component)]
struct GuardianSubmenu;

#[derive(Component)]
struct GuardianButton;

#[derive(Component)]
struct TerrainSubmenu;

#[derive(Component)]
struct TerrainButton;

#[derive(Component)]
struct BloomPoolFill;

#[derive(Component)]
struct BloomPoolButton;

#[derive(Component)]
struct BloomPoolHalo;

#[derive(Component)]
struct BloomPoolSelectionHalo;

#[derive(Component)]
struct BloomHaloPulse {
    timer: Timer,
}

#[derive(Component)]
struct TimeControlButton {
    mode: SimulationControl,
}

#[derive(Component)]
struct WorldClickHalo;

#[derive(Component)]
struct WorldClickHaloPulse {
    timer: Timer,
    base_diameter: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SimulationControl {
    Pause,
    Play,
    FastForward,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum UIMode {
    None,
    EscapeMenu,
    Map,
}

impl Default for UIMode {
    fn default() -> Self { UIMode::None }
}

#[derive(Resource, Clone, Copy, PartialEq, Eq)]
struct GameUIState {
    mode: UIMode,
    simulation: SimulationControl,
    debug_bounds_enabled: bool,
    save_requested: bool,
    reset_requested: bool,
}

impl Default for GameUIState {
    fn default() -> Self {
        Self {
            mode: UIMode::None,
            simulation: SimulationControl::Play,
            debug_bounds_enabled: false,
            save_requested: false,
            reset_requested: false,
        }
    }
}

impl GameUIState {
    fn is_escape_menu_open(&self) -> bool { self.mode == UIMode::EscapeMenu }
    fn is_map_visible(&self) -> bool { self.mode == UIMode::Map }
    fn open_escape_menu(&mut self) { self.mode = UIMode::EscapeMenu; }
    fn close_escape_menu(&mut self) { self.mode = UIMode::None; }
    fn close_all_modals(&mut self) { self.mode = UIMode::None; }
    fn toggle_map(&mut self) {
        self.mode = match self.mode {
            UIMode::Map => UIMode::None,
            _ => UIMode::Map,
        };
    }
    fn request_save(&mut self) { self.save_requested = true; }
    fn request_reset(&mut self) { self.reset_requested = true; }
    fn take_save_request(&mut self) -> bool {
        std::mem::replace(&mut self.save_requested, false)
    }
    fn take_reset_request(&mut self) -> bool {
        std::mem::replace(&mut self.reset_requested, false)
    }
}

// Entity type identifier for buttons
#[derive(Component, Clone, Debug)]
enum EntityType {
    Human,
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
                (EntityType::Human, EntityType::Human) => true,
                (EntityType::Snail, EntityType::Snail) => true,
                (EntityType::ForestGuardian(a), EntityType::ForestGuardian(b)) => a == b,
                _ => false,
            }
        } else {
            false
        }
    }
}

impl EntityType {
    fn bloom_cost(&self) -> u16 {
        1
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

#[derive(Resource, Deref, DerefMut)]
struct EntityAutosaveTimer(Timer);

impl Default for EntityAutosaveTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(30.0, TimerMode::Repeating))
    }
}

#[derive(Component)]
struct InGameCamera;

#[derive(Component)]
struct InGameUiRoot;

#[derive(Component)]
struct EscapeMenuRoot;

#[derive(Component)]
struct MainMenuRoot;

#[derive(Component)]
struct ContinueGameButton;

#[derive(Component)]
struct LoadGameButton;

#[derive(Component)]
struct NewGameButton;

#[derive(Component)]
struct ContinueWorldButton;

#[derive(Component)]
struct LoadWorldButton;

#[derive(Component)]
struct NewWorldButton;

#[derive(Component)]
struct QuitButton;

#[derive(Component)]
struct BackToMainMenuButton;

#[derive(Component)]
struct ExitWorldButton;

#[derive(Component)]
struct ExitToMainMenuButton;

#[derive(Component)]
struct QuitGameButton;

#[derive(Component)]
struct BackToSlotHubButton;

#[derive(Component)]
struct DisabledMenuButton;

#[derive(Component)]
struct SelectSlotButton {
    slot_id: String,
}

#[derive(Component)]
struct SelectWorldButton {
    world_id: String,
}

#[derive(Component)]
struct MainMenuCamera;

#[derive(Component)]
struct LoadSlotSelectRoot;

#[derive(Component)]
struct LoadSlotSelectCamera;

#[derive(Component)]
struct SlotHubRoot;

#[derive(Component)]
struct SlotHubCamera;

#[derive(Component)]
struct LoadWorldSelectRoot;

#[derive(Component)]
struct LoadWorldSelectCamera;

#[derive(Component)]
struct DraftSetupRoot;

#[derive(Component)]
struct DraftSetupCamera;

#[derive(Component)]
struct DraftSetupScaler;

#[derive(Component)]
struct DraftTrayCard;

#[derive(Component)]
struct DraftGridColumn {
    index: usize,
}

#[derive(Component)]
struct DraftGridCell {
    column: usize,
    row: usize,
}

#[derive(Component)]
struct DraftPlacedCardVisual;

#[derive(Component)]
struct DraftPlacedCardIcon;

#[derive(Component)]
struct DraftPlacedCardInfinityIcon;

#[derive(Component)]
struct DraftCardFrame;

#[derive(Component)]
struct DraftCardIcon;

#[derive(Component)]
struct DraftCardInfinityIcon;

#[derive(Component)]
struct DraftCardGhost;

#[derive(Component)]
struct DraftConfirmButton;

#[derive(Component)]
struct MainMenuButtonImage;

#[derive(Component)]
struct DraftConfirmButtonLabel;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
struct DraftCardTypeComponent(DraftCard);

#[derive(Component, Default)]
struct DraftConfirmButtonPressState {
    armed: bool,
}

#[derive(Component, Clone, Copy)]
struct MainMenuButtonSprites {
    standard: Rect,
    hovered: Rect,
    pressed: Rect,
}

#[derive(Component, Default)]
struct MainMenuButtonPressState {
    armed: bool,
}

#[derive(Resource, Clone, Debug)]
struct Bloom {
    current: u16,
    max: u16,
}

impl Default for Bloom {
    fn default() -> Self {
        Self {
            current: 14,
            max: 14,
        }
    }
}

impl Bloom {
    fn percent(&self) -> f32 {
        if self.max == 0 {
            0.0
        } else {
            self.current as f32 / self.max as f32
        }
    }

    fn can_spend(&self, amount: u16) -> bool {
        self.current >= amount
    }

    fn spend(&mut self, amount: u16) -> bool {
        if !self.can_spend(amount) {
            return false;
        }

        self.current -= amount;
        true
    }
}

#[derive(Resource, Default, Clone, Debug)]
struct BloomSelection {
    selected: bool,
}

#[derive(Resource, Clone, Debug)]
struct DraftBoard {
    available_columns: u8,
    cells: [[Option<DraftCard>; DRAFT_MAX_ROWS]; DRAFT_MAX_COLUMNS],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DraftCellPosition {
    column: usize,
    row: usize,
}

impl Default for DraftBoard {
    fn default() -> Self {
        Self {
            available_columns: 1,
            cells: [[None; DRAFT_MAX_ROWS]; DRAFT_MAX_COLUMNS],
        }
    }
}

impl DraftBoard {
    fn reset(&mut self, available_columns: u8) {
        self.available_columns = available_columns.clamp(1, DRAFT_MAX_COLUMNS as u8);
        self.cells = [[None; DRAFT_MAX_ROWS]; DRAFT_MAX_COLUMNS];
    }

    fn available_columns(&self) -> usize {
        self.available_columns as usize
    }

    fn card_at(&self, column: usize, row: usize) -> Option<DraftCard> {
        self.cells
            .get(column)
            .and_then(|rows| rows.get(row))
            .copied()
            .flatten()
    }

    fn card_at_with_omission(
        &self,
        column: usize,
        row: usize,
        omitted: Option<DraftCellPosition>,
    ) -> Option<DraftCard> {
        if omitted == Some(DraftCellPosition { column, row }) {
            None
        } else {
            self.card_at(column, row)
        }
    }

    fn is_column_unlocked(&self, column: usize) -> bool {
        column < self.available_columns()
    }

    fn is_row_unlocked(&self, column: usize, row: usize) -> bool {
        self.is_row_unlocked_with_omission(column, row, None)
    }

    fn is_row_unlocked_with_omission(
        &self,
        column: usize,
        row: usize,
        omitted: Option<DraftCellPosition>,
    ) -> bool {
        if !self.is_column_unlocked(column) || row >= DRAFT_MAX_ROWS {
            return false;
        }

        if row == 0 {
            return true;
        }

        self.card_at_with_omission(column, row - 1, omitted)
            .map(DraftCard::unlocks_next_row)
            .unwrap_or(false)
            && self.is_row_unlocked_with_omission(column, row - 1, omitted)
    }

    fn can_place_with_omission(
        &self,
        column: usize,
        row: usize,
        omitted: Option<DraftCellPosition>,
    ) -> bool {
        self.is_row_unlocked_with_omission(column, row, omitted)
    }

    fn allowed_category(&self, row: usize) -> Option<DraftCardCategory> {
        match row {
            0 => Some(DraftCardCategory::Shape),
            1 => Some(DraftCardCategory::Element),
            _ => None,
        }
    }

    fn can_place_card(&self, column: usize, row: usize, card: DraftCard) -> bool {
        self.can_place_card_with_omission(column, row, card, None)
    }

    fn can_place_card_with_omission(
        &self,
        column: usize,
        row: usize,
        card: DraftCard,
        omitted: Option<DraftCellPosition>,
    ) -> bool {
        self.can_place_with_omission(column, row, omitted)
            && self
                .allowed_category(row)
                .map(|category| category == card.category())
                .unwrap_or(false)
    }

    fn place_card(&mut self, column: usize, row: usize, card: DraftCard) -> bool {
        if !self.can_place_card(column, row, card) {
            return false;
        }

        self.cells[column][row] = Some(card);
        self.clear_locked_rows_in_column(column);
        true
    }

    fn remove_card(&mut self, column: usize, row: usize) -> Option<DraftCard> {
        let card = self
            .cells
            .get_mut(column)
            .and_then(|rows| rows.get_mut(row))
            .and_then(Option::take);
        if card.is_some() {
            self.clear_locked_rows_in_column(column);
        }
        card
    }

    fn clear_locked_rows_in_column(&mut self, column: usize) {
        if column >= DRAFT_MAX_COLUMNS {
            return;
        }

        for row in 1..DRAFT_MAX_ROWS {
            if !self.is_row_unlocked(column, row) {
                self.cells[column][row] = None;
            }
        }
    }

    fn recipe_card(&self, row: usize) -> Option<DraftCard> {
        self.card_at(0, row)
    }
}

fn recipe_shape(card: DraftCard) -> Option<WorldShape> {
    match card {
        DraftCard::Island => Some(WorldShape::Island),
        DraftCard::Infinity => Some(WorldShape::Infinity),
        _ => None,
    }
}

fn recipe_element(card: DraftCard) -> Option<WorldElement> {
    match card {
        DraftCard::Grass => Some(WorldElement::Grass),
        DraftCard::Dirt => Some(WorldElement::Dirt),
        _ => None,
    }
}

fn draft_generation_config(board: &DraftBoard) -> Option<WorldGenerationConfig> {
    Some(WorldGenerationConfig {
        shape: recipe_shape(board.recipe_card(0)?)?,
        element: recipe_element(board.recipe_card(1)?)?,
        ..WorldGenerationConfig::default()
    })
}

fn draft_confirm_enabled(board: &DraftBoard) -> bool {
    draft_generation_config(board).is_some()
}

fn apply_draft_card_icon(
    image_node: &mut ImageNode,
    image_style: &mut Node,
    infinity_style: &mut Node,
    card: DraftCard,
    assets: Option<&AssetServer>,
    texture_atlas_layouts: Option<&mut Assets<TextureAtlasLayout>>,
) {
    if card == DraftCard::Infinity {
        image_node.image = Handle::default();
        image_node.rect = None;
        image_node.texture_atlas = None;
        image_style.display = Display::None;
        infinity_style.display = Display::Flex;
    } else {
        set_draft_card_icon(image_node, card, assets, texture_atlas_layouts);
        image_style.display = Display::Flex;
        infinity_style.display = Display::None;
    }
}

fn spawn_infinity_icon(parent: &mut ChildSpawnerCommands, marker: impl Bundle, visible: bool) {
    parent
        .spawn((
            marker,
            Node {
                width: Val::Px(DRAFT_CARD_ICON_SIZE),
                height: Val::Px(DRAFT_CARD_ICON_SIZE * 0.72),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(DRAFT_CARD_ICON_SIZE * 0.08),
                display: if visible {
                    Display::Flex
                } else {
                    Display::None
                },
                ..default()
            },
        ))
        .with_children(|icon| {
            icon.spawn(Node {
                width: Val::Px(DRAFT_CARD_ICON_SIZE * 0.34),
                height: Val::Px(DRAFT_CARD_ICON_SIZE * 0.34),
                position_type: PositionType::Absolute,
                left: Val::Px(DRAFT_CARD_ICON_SIZE * 0.14),
                top: Val::Px(DRAFT_CARD_ICON_SIZE * 0.17),
                border: UiRect::all(Val::Px(3.0)),
                border_radius: BorderRadius::all(Val::Px(999.0)),
                ..default()
            })
            .insert(BorderColor::all(Color::srgb(0.93, 0.88, 0.75)))
            .insert(BackgroundColor(Color::NONE));
            icon.spawn(Node {
                width: Val::Px(DRAFT_CARD_ICON_SIZE * 0.34),
                height: Val::Px(DRAFT_CARD_ICON_SIZE * 0.34),
                position_type: PositionType::Absolute,
                left: Val::Px(DRAFT_CARD_ICON_SIZE * 0.40),
                top: Val::Px(DRAFT_CARD_ICON_SIZE * 0.17),
                border: UiRect::all(Val::Px(3.0)),
                border_radius: BorderRadius::all(Val::Px(999.0)),
                ..default()
            })
            .insert(BorderColor::all(Color::srgb(0.93, 0.88, 0.75)))
            .insert(BackgroundColor(Color::NONE));
        });
}

#[derive(Resource, Default, Clone, Debug)]
struct DraftDragState {
    active_card: Option<DraftCard>,
    source_cell: Option<DraftCellPosition>,
    hovered_cell: Option<usize>,
    cursor_pos: Option<Vec2>,
}

#[derive(Resource, Clone, Debug)]
struct SaveGameState {
    root_dir: PathBuf,
    slots: Vec<SaveSlotMetadata>,
    worlds: Vec<WorldMetadata>,
    active_slot_id: Option<String>,
    active_world_id: Option<String>,
    draft_target_slot_id: Option<String>,
}

impl Default for SaveGameState {
    fn default() -> Self {
        Self {
            root_dir: PathBuf::from("saves"),
            slots: Vec::new(),
            worlds: Vec::new(),
            active_slot_id: None,
            active_world_id: None,
            draft_target_slot_id: None,
        }
    }
}

impl SaveGameState {
    fn refresh_slots(&mut self) {
        self.slots = savegame::list_slots(&self.root_dir).unwrap_or_else(|err| {
            error!("Failed to list save slots: {err}");
            Vec::new()
        });
    }

    fn refresh_worlds(&mut self) {
        self.worlds = self
            .active_slot_id
            .as_deref()
            .map(|slot_id| {
                savegame::list_worlds(&self.root_dir, slot_id).unwrap_or_else(|err| {
                    error!("Failed to list worlds for slot {slot_id}: {err}");
                    Vec::new()
                })
            })
            .unwrap_or_default();
    }

    fn activate_slot(&mut self, slot_id: String) {
        self.active_slot_id = Some(slot_id);
        self.active_world_id = None;
        self.refresh_worlds();
    }

    fn active_slot(&self) -> Option<&SaveSlotMetadata> {
        self.active_slot_id
            .as_deref()
            .and_then(|slot_id| self.slots.iter().find(|slot| slot.id == slot_id))
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .insert_resource(SpritePickingSettings {
            picking_mode: SpritePickingMode::AlphaThreshold(WORLD_ENTITY_ALPHA_THRESHOLD),
            ..default()
        })
        .init_state::<AppState>()
        .add_plugins(MapPlugin)
        .init_resource::<WorldManager>()
        .init_resource::<TerrainTransitionConfig>()
        .init_resource::<loader::ChunkSaveTimer>()
        .init_resource::<EntityAutosaveTimer>()
        .init_resource::<PlacementMode>()
        .init_resource::<PaintMode>()
        .init_resource::<PaintDragState>()
        .init_resource::<GameUIState>()
        .init_resource::<SaveNotification>()
        .init_resource::<Bloom>()
        .init_resource::<BloomSelection>()
        .init_resource::<DraftBoard>()
        .init_resource::<DraftDragState>()
        .init_resource::<SaveGameState>()
        // Register observer for ChunkDataChanged to sync visuals
        .add_observer(loader::sync_chunk_visuals_on_data_change)
        .add_systems(OnEnter(AppState::MainMenu), setup_main_menu)
        .add_systems(OnExit(AppState::MainMenu), cleanup_main_menu)
        .add_systems(OnEnter(AppState::LoadSlotSelect), setup_load_slot_select)
        .add_systems(OnExit(AppState::LoadSlotSelect), cleanup_load_slot_select)
        .add_systems(OnEnter(AppState::SlotHub), setup_slot_hub)
        .add_systems(OnExit(AppState::SlotHub), cleanup_slot_hub)
        .add_systems(OnEnter(AppState::LoadWorldSelect), setup_load_world_select)
        .add_systems(OnExit(AppState::LoadWorldSelect), cleanup_load_world_select)
        .add_systems(OnEnter(AppState::DraftSetup), setup_draft_setup)
        .add_systems(OnExit(AppState::DraftSetup), cleanup_draft_setup)
        .add_systems(
            OnEnter(AppState::InGame),
            (setup_world, setup_ui, setup_save_notification_ui),
        )
        .add_systems(OnExit(AppState::InGame), cleanup_in_game)
        .add_systems(
            Update,
            (
                update_main_menu_button_visuals,
                handle_continue_game_button_interaction,
                handle_load_game_button_interaction,
                handle_new_game_button_interaction,
                handle_quit_button_interaction,
            )
                .run_if(in_state(AppState::MainMenu)),
        )
        .add_systems(
            Update,
            (
                update_main_menu_button_visuals,
                handle_select_slot_button_interaction,
                handle_back_to_main_menu_button_interaction,
            )
                .run_if(in_state(AppState::LoadSlotSelect)),
        )
        .add_systems(
            Update,
            (
                update_main_menu_button_visuals,
                handle_continue_world_button_interaction,
                handle_load_world_button_interaction,
                handle_new_world_button_interaction,
                handle_back_to_main_menu_button_interaction,
            )
                .run_if(in_state(AppState::SlotHub)),
        )
        .add_systems(
            Update,
            (
                update_main_menu_button_visuals,
                handle_select_world_button_interaction,
                handle_back_to_slot_hub_button_interaction,
            )
                .run_if(in_state(AppState::LoadWorldSelect)),
        )
        .add_systems(
            Update,
            (
                start_draft_card_drag,
                update_main_menu_button_visuals,
                update_draft_layout_scale,
                update_draft_drag_cursor,
                update_draft_hovered_cell.after(update_draft_drag_cursor),
                handle_draft_card_drop.after(update_draft_hovered_cell),
                handle_draft_debug_unlock_columns,
                handle_draft_confirm_button_interaction,
                update_draft_visuals,
                update_draft_confirm_button_visuals,
            )
                .run_if(in_state(AppState::DraftSetup)),
        )
        .add_systems(
            Update,
            (
                update_main_menu_button_visuals,
                handle_keyboard_shortcuts,
                sync_escape_menu_visibility,
                update_bloom_pool,
                update_bloom_selection_halo,
                update_time_control_button_visuals,
                animate_bloom_halo,
                animate_world_click_halo,
                pulse_bloom_halo_on_world_click,
                handle_bloom_world_entity_click,
                handle_exit_world_button_interaction,
                handle_exit_to_main_menu_button_interaction,
                handle_quit_game_button_interaction,
            )
                .run_if(in_state(AppState::InGame)),
        )
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
            )
                .run_if(escape_menu_closed)
                .run_if(in_state(AppState::InGame)),
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
                handle_manual_save_requested,
                update_save_notification,
                // World management
                handle_reset_requested,
                loader::update_camera_chunk,
                loader::load_chunks_around_camera.after(loader::update_camera_chunk),
                loader::unload_distant_chunks.after(loader::load_chunks_around_camera),
                loader::apply_tile_modifications,
                world::transitions::log_transition_cache_pressure,
                loader::autosave_dirty_chunks,
                autosave_entities,
                draw_snail_debug_bounds,
            )
                .run_if(escape_menu_closed)
                .run_if(in_state(AppState::InGame)),
        )
        .run();
}

fn escape_menu_closed(ui_state: Res<GameUIState>) -> bool {
    !ui_state.is_escape_menu_open()
}

fn formatted_timestamp(timestamp: u64) -> String {
    format!("{timestamp}")
}

fn spawn_menu_button(
    panel: &mut ChildSpawnerCommands<'_>,
    ui_texture: &Handle<Image>,
    button_sprites: MainMenuButtonSprites,
    label: impl Into<String>,
    text_color: Color,
    extra_components: impl Bundle,
) {
    panel
        .spawn((
            Button,
            button_sprites,
            MainMenuButtonPressState::default(),
            Node {
                width: Val::Px(MAIN_MENU_BUTTON_WIDTH),
                height: Val::Px(MAIN_MENU_BUTTON_HEIGHT),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(Val::Px(24.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            extra_components,
        ))
        .with_children(|button| {
            button.spawn((
                MainMenuButtonImage,
                ImageNode {
                    image: ui_texture.clone(),
                    rect: Some(main_menu_button_standard_rect()),
                    image_mode: main_menu_button_image_mode(),
                    ..default()
                },
                Node {
                    width: Val::Px(MAIN_MENU_BUTTON_SOURCE_WIDTH),
                    height: Val::Px(MAIN_MENU_BUTTON_SOURCE_HEIGHT),
                    position_type: PositionType::Absolute,
                    ..default()
                },
                UiTransform::from_scale(Vec2::splat(MAIN_MENU_BUTTON_SCALE)),
            ));
            button.spawn((
                Text::new(label.into()),
                TextFont {
                    font_size: 28.0,
                    ..default()
                },
                TextColor(text_color),
            ));
        });
}

fn spawn_menu_panel(
    parent: &mut ChildSpawnerCommands<'_>,
    ui_texture: &Handle<Image>,
    title: &str,
    panel_width: f32,
    body: impl FnOnce(&mut ChildSpawnerCommands<'_>, MainMenuButtonSprites, &Handle<Image>),
) {
    let button_sprites = MainMenuButtonSprites {
        standard: main_menu_button_standard_rect(),
        hovered: main_menu_button_hovered_rect(),
        pressed: main_menu_button_clicked_rect(),
    };

    parent
        .spawn((
            Node {
                width: Val::Px(panel_width),
                max_width: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(28.0), Val::Px(30.0)),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.02, 0.03, 0.82)),
        ))
        .with_children(|panel| {
            panel
                .spawn((
                    Node {
                        width: Val::Px(480.0),
                        height: Val::Px(144.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        padding: UiRect::horizontal(Val::Px(20.0)),
                        ..default()
                    },
                    ImageNode {
                        image: ui_texture.clone(),
                        rect: Some(main_menu_label_rect()),
                        image_mode: NodeImageMode::Stretch,
                        ..default()
                    },
                ))
                .with_children(|label_node| {
                    label_node.spawn((
                        Text::new(title),
                        TextFont {
                            font_size: 38.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.16, 0.11, 0.05)),
                        Node {
                            margin: UiRect::top(Val::Px(20.0)),
                            ..default()
                        },
                    ));
                });

            body(panel, button_sprites, ui_texture);
        });
}

fn setup_main_menu(
    mut commands: Commands,
    assets: Option<Res<AssetServer>>,
    mut save_state: ResMut<SaveGameState>,
) {
    let ui_texture: Handle<Image> = assets
        .as_ref()
        .map_or_else(Handle::default, |assets| load_main_menu_ui_texture(assets));
    save_state.refresh_slots();
    let has_slots = !save_state.slots.is_empty();

    commands.spawn((MainMenuCamera, Camera2d));

    commands
        .spawn((
            MainMenuRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(32.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.04, 0.05, 0.07)),
        ))
        .with_children(|parent| {
            spawn_menu_panel(
                parent,
                &ui_texture,
                "Worldseed",
                520.0,
                |panel, button_sprites, ui_texture| {
                    let enabled_color = Color::srgb(0.19, 0.12, 0.06);
                    let disabled_color = Color::srgb(0.42, 0.39, 0.35);

                    if has_slots {
                        spawn_menu_button(
                            panel,
                            ui_texture,
                            button_sprites,
                            "Continue Game",
                            enabled_color,
                            ContinueGameButton,
                        );
                        spawn_menu_button(
                            panel,
                            ui_texture,
                            button_sprites,
                            "Load Game",
                            enabled_color,
                            LoadGameButton,
                        );
                    } else {
                        spawn_menu_button(
                            panel,
                            ui_texture,
                            button_sprites,
                            "Continue Game",
                            disabled_color,
                            (ContinueGameButton, DisabledMenuButton),
                        );
                        spawn_menu_button(
                            panel,
                            ui_texture,
                            button_sprites,
                            "Load Game",
                            disabled_color,
                            (LoadGameButton, DisabledMenuButton),
                        );
                    }

                    spawn_menu_button(
                        panel,
                        ui_texture,
                        button_sprites,
                        "New Game",
                        enabled_color,
                        NewGameButton,
                    );
                    spawn_menu_button(
                        panel,
                        ui_texture,
                        button_sprites,
                        "Quit",
                        enabled_color,
                        QuitButton,
                    );
                },
            );
        });
}

fn handle_continue_game_button_interaction(
    mut interaction_query: Query<
        (&Interaction, &mut MainMenuButtonPressState),
        (
            Changed<Interaction>,
            With<ContinueGameButton>,
            Without<DisabledMenuButton>,
        ),
    >,
    mut save_state: ResMut<SaveGameState>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (interaction, mut press_state) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                press_state.armed = true;
            }
            Interaction::Hovered if press_state.armed => {
                press_state.armed = false;
                save_state.refresh_slots();
                if let Some(slot) = savegame::most_recent_slot(&save_state.root_dir)
                    .ok()
                    .flatten()
                {
                    if let Err(err) = savegame::touch_slot(&save_state.root_dir, &slot.id) {
                        error!("Failed to touch slot {}: {err}", slot.id);
                    }
                    save_state.activate_slot(slot.id);
                    next_state.set(AppState::SlotHub);
                }
            }
            Interaction::None => {
                press_state.armed = false;
            }
            Interaction::Hovered => {}
        }
    }
}

fn handle_load_game_button_interaction(
    mut interaction_query: Query<
        (&Interaction, &mut MainMenuButtonPressState),
        (
            Changed<Interaction>,
            With<LoadGameButton>,
            Without<DisabledMenuButton>,
        ),
    >,
    mut save_state: ResMut<SaveGameState>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (interaction, mut press_state) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => press_state.armed = true,
            Interaction::Hovered if press_state.armed => {
                press_state.armed = false;
                save_state.refresh_slots();
                next_state.set(AppState::LoadSlotSelect);
            }
            Interaction::None => press_state.armed = false,
            Interaction::Hovered => {}
        }
    }
}

fn handle_new_game_button_interaction(
    mut interaction_query: Query<
        (&Interaction, &mut MainMenuButtonPressState),
        (Changed<Interaction>, With<NewGameButton>),
    >,
    mut save_state: ResMut<SaveGameState>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (interaction, mut press_state) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => press_state.armed = true,
            Interaction::Hovered if press_state.armed => {
                press_state.armed = false;
                match savegame::create_slot(&save_state.root_dir) {
                    Ok(slot) => {
                        save_state.refresh_slots();
                        save_state.activate_slot(slot.id.clone());
                        save_state.draft_target_slot_id = Some(slot.id);
                        next_state.set(AppState::DraftSetup);
                    }
                    Err(err) => error!("Failed to create save slot: {err}"),
                }
            }
            Interaction::None => press_state.armed = false,
            Interaction::Hovered => {}
        }
    }
}

fn update_main_menu_button_visuals(
    mut interaction_query: Query<
        (&Interaction, &MainMenuButtonSprites, &Children),
        (
            Changed<Interaction>,
            With<Button>,
            Without<DisabledMenuButton>,
        ),
    >,
    mut image_query: Query<&mut ImageNode, With<MainMenuButtonImage>>,
) {
    for (interaction, sprites, children) in &mut interaction_query {
        let rect = Some(match *interaction {
            Interaction::Pressed => sprites.pressed,
            Interaction::Hovered => sprites.hovered,
            Interaction::None => sprites.standard,
        });

        for child in children.iter() {
            if let Ok(mut image) = image_query.get_mut(child) {
                image.rect = rect;
            }
        }
    }
}

fn handle_quit_button_interaction(
    mut interaction_query: Query<
        (&Interaction, &mut MainMenuButtonPressState),
        (Changed<Interaction>, With<QuitButton>),
    >,
    mut app_exit: MessageWriter<AppExit>,
) {
    for (interaction, mut press_state) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                press_state.armed = true;
            }
            Interaction::Hovered if press_state.armed => {
                press_state.armed = false;
                app_exit.write(AppExit::Success);
            }
            Interaction::None => {
                press_state.armed = false;
            }
            Interaction::Hovered => {}
        }
    }
}

fn handle_exit_world_button_interaction(
    mut interaction_query: Query<
        (&Interaction, &mut MainMenuButtonPressState),
        (Changed<Interaction>, With<ExitWorldButton>),
    >,
    mut save_state: ResMut<SaveGameState>,
    mut world_manager: ResMut<WorldManager>,
    mut ui_state: ResMut<GameUIState>,
    camera_query: Query<(&Transform, &Projection), With<InGameCamera>>,
    bloom: Option<Res<Bloom>>,
    human_query: Query<
        (
            &Position,
            &Velocity,
            &Direction,
            &entities::EntityState,
            &Health,
        ),
        With<Human>,
    >,
    guardian_query: Query<(
        &Position,
        &Velocity,
        &Direction,
        &entities::EntityState,
        &Health,
        &ForestGuardian,
        &RoamingBehavior,
        &TreeSpawner,
    )>,
    snail_query: Query<
        (
            &Position,
            &Velocity,
            &Direction,
            &entities::EntityState,
            &Health,
            &WindingPath,
        ),
        With<Snail>,
    >,
    tree_spirit_query: Query<(&Position, &GrowingTree), With<TreeSpirit>>,
    variant_tree_query: Query<(&Position, &GrowingTree), With<VariantTree>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (interaction, mut press_state) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => press_state.armed = true,
            Interaction::Hovered if press_state.armed => {
                press_state.armed = false;
                save_world_snapshot(
                    &mut world_manager,
                    &camera_query,
                    bloom.as_deref(),
                    &human_query,
                    &guardian_query,
                    &snail_query,
                    &tree_spirit_query,
                    &variant_tree_query,
                    None,
                );
                save_state.active_world_id = None;
                save_state.refresh_worlds();
                ui_state.close_escape_menu();
                next_state.set(AppState::SlotHub);
            }
            Interaction::None => press_state.armed = false,
            Interaction::Hovered => {}
        }
    }
}

fn handle_exit_to_main_menu_button_interaction(
    mut interaction_query: Query<
        (&Interaction, &mut MainMenuButtonPressState),
        (Changed<Interaction>, With<ExitToMainMenuButton>),
    >,
    mut save_state: ResMut<SaveGameState>,
    mut world_manager: ResMut<WorldManager>,
    mut ui_state: ResMut<GameUIState>,
    camera_query: Query<(&Transform, &Projection), With<InGameCamera>>,
    bloom: Option<Res<Bloom>>,
    human_query: Query<
        (
            &Position,
            &Velocity,
            &Direction,
            &entities::EntityState,
            &Health,
        ),
        With<Human>,
    >,
    guardian_query: Query<(
        &Position,
        &Velocity,
        &Direction,
        &entities::EntityState,
        &Health,
        &ForestGuardian,
        &RoamingBehavior,
        &TreeSpawner,
    )>,
    snail_query: Query<
        (
            &Position,
            &Velocity,
            &Direction,
            &entities::EntityState,
            &Health,
            &WindingPath,
        ),
        With<Snail>,
    >,
    tree_spirit_query: Query<(&Position, &GrowingTree), With<TreeSpirit>>,
    variant_tree_query: Query<(&Position, &GrowingTree), With<VariantTree>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (interaction, mut press_state) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => press_state.armed = true,
            Interaction::Hovered if press_state.armed => {
                press_state.armed = false;
                save_world_snapshot(
                    &mut world_manager,
                    &camera_query,
                    bloom.as_deref(),
                    &human_query,
                    &guardian_query,
                    &snail_query,
                    &tree_spirit_query,
                    &variant_tree_query,
                    None,
                );
                save_state.active_slot_id = None;
                save_state.active_world_id = None;
                save_state.refresh_slots();
                save_state.refresh_worlds();
                ui_state.close_escape_menu();
                next_state.set(AppState::MainMenu);
            }
            Interaction::None => press_state.armed = false,
            Interaction::Hovered => {}
        }
    }
}

fn handle_quit_game_button_interaction(
    mut interaction_query: Query<
        (&Interaction, &mut MainMenuButtonPressState),
        (Changed<Interaction>, With<QuitGameButton>),
    >,
    mut world_manager: ResMut<WorldManager>,
    camera_query: Query<(&Transform, &Projection), With<InGameCamera>>,
    bloom: Option<Res<Bloom>>,
    human_query: Query<
        (
            &Position,
            &Velocity,
            &Direction,
            &entities::EntityState,
            &Health,
        ),
        With<Human>,
    >,
    guardian_query: Query<(
        &Position,
        &Velocity,
        &Direction,
        &entities::EntityState,
        &Health,
        &ForestGuardian,
        &RoamingBehavior,
        &TreeSpawner,
    )>,
    snail_query: Query<
        (
            &Position,
            &Velocity,
            &Direction,
            &entities::EntityState,
            &Health,
            &WindingPath,
        ),
        With<Snail>,
    >,
    tree_spirit_query: Query<(&Position, &GrowingTree), With<TreeSpirit>>,
    variant_tree_query: Query<(&Position, &GrowingTree), With<VariantTree>>,
    mut app_exit: MessageWriter<AppExit>,
) {
    for (interaction, mut press_state) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => press_state.armed = true,
            Interaction::Hovered if press_state.armed => {
                press_state.armed = false;
                save_world_snapshot(
                    &mut world_manager,
                    &camera_query,
                    bloom.as_deref(),
                    &human_query,
                    &guardian_query,
                    &snail_query,
                    &tree_spirit_query,
                    &variant_tree_query,
                    None,
                );
                app_exit.write(AppExit::Success);
            }
            Interaction::None => press_state.armed = false,
            Interaction::Hovered => {}
        }
    }
}

fn cleanup_main_menu(
    mut commands: Commands,
    menu_query: Query<Entity, With<MainMenuRoot>>,
    camera_query: Query<Entity, With<MainMenuCamera>>,
) {
    for entity in &menu_query {
        commands.entity(entity).despawn();
    }

    for entity in &camera_query {
        commands.entity(entity).despawn();
    }
}

fn setup_load_slot_select(
    mut commands: Commands,
    assets: Option<Res<AssetServer>>,
    mut save_state: ResMut<SaveGameState>,
) {
    let ui_texture: Handle<Image> = assets
        .as_ref()
        .map_or_else(Handle::default, |assets| load_main_menu_ui_texture(assets));
    save_state.refresh_slots();

    commands.spawn((LoadSlotSelectCamera, Camera2d));
    commands
        .spawn((
            LoadSlotSelectRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(32.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.04, 0.05, 0.07)),
        ))
        .with_children(|parent| {
            spawn_menu_panel(
                parent,
                &ui_texture,
                "Load Slot",
                560.0,
                |panel, button_sprites, ui_texture| {
                    let enabled_color = Color::srgb(0.19, 0.12, 0.06);

                    if save_state.slots.is_empty() {
                        panel.spawn((
                            Text::new("No save slots exist yet."),
                            TextFont {
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.83, 0.8, 0.74)),
                        ));
                    } else {
                        for slot in &save_state.slots {
                            spawn_menu_button(
                                panel,
                                ui_texture,
                                button_sprites,
                                format!(
                                    "{}  |  created {}  |  last played {}",
                                    slot.display_name,
                                    formatted_timestamp(slot.created_at),
                                    formatted_timestamp(slot.last_played_at)
                                ),
                                enabled_color,
                                SelectSlotButton {
                                    slot_id: slot.id.clone(),
                                },
                            );
                        }
                    }

                    spawn_menu_button(
                        panel,
                        ui_texture,
                        button_sprites,
                        "Back",
                        enabled_color,
                        BackToMainMenuButton,
                    );
                },
            );
        });
}

fn cleanup_load_slot_select(
    mut commands: Commands,
    roots: Query<Entity, With<LoadSlotSelectRoot>>,
    cameras: Query<Entity, With<LoadSlotSelectCamera>>,
) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    for entity in &cameras {
        commands.entity(entity).despawn();
    }
}

fn setup_slot_hub(
    mut commands: Commands,
    assets: Option<Res<AssetServer>>,
    mut save_state: ResMut<SaveGameState>,
) {
    let ui_texture: Handle<Image> = assets
        .as_ref()
        .map_or_else(Handle::default, |assets| load_main_menu_ui_texture(assets));
    save_state.refresh_slots();
    save_state.refresh_worlds();
    let slot_label = save_state
        .active_slot()
        .map(|slot| {
            format!(
                "{}  |  created {}  |  last played {}",
                slot.display_name,
                formatted_timestamp(slot.created_at),
                formatted_timestamp(slot.last_played_at)
            )
        })
        .unwrap_or_else(|| "No active slot selected.".to_string());
    let has_worlds = !save_state.worlds.is_empty();

    commands.spawn((SlotHubCamera, Camera2d));
    commands
        .spawn((
            SlotHubRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(32.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.04, 0.05, 0.07)),
        ))
        .with_children(|parent| {
            spawn_menu_panel(
                parent,
                &ui_texture,
                "Slot Hub",
                560.0,
                |panel, button_sprites, ui_texture| {
                    panel.spawn((
                        Text::new(slot_label.clone()),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.78, 0.75, 0.69)),
                    ));

                    let enabled_color = Color::srgb(0.19, 0.12, 0.06);
                    let disabled_color = Color::srgb(0.42, 0.39, 0.35);

                    if has_worlds {
                        spawn_menu_button(
                            panel,
                            ui_texture,
                            button_sprites,
                            "Continue World",
                            enabled_color,
                            ContinueWorldButton,
                        );
                        spawn_menu_button(
                            panel,
                            ui_texture,
                            button_sprites,
                            "Load World",
                            enabled_color,
                            LoadWorldButton,
                        );
                    } else {
                        spawn_menu_button(
                            panel,
                            ui_texture,
                            button_sprites,
                            "Continue World",
                            disabled_color,
                            (ContinueWorldButton, DisabledMenuButton),
                        );
                        spawn_menu_button(
                            panel,
                            ui_texture,
                            button_sprites,
                            "Load World",
                            disabled_color,
                            (LoadWorldButton, DisabledMenuButton),
                        );
                    }

                    spawn_menu_button(
                        panel,
                        ui_texture,
                        button_sprites,
                        "New World",
                        enabled_color,
                        NewWorldButton,
                    );
                    spawn_menu_button(
                        panel,
                        ui_texture,
                        button_sprites,
                        "Back",
                        enabled_color,
                        BackToMainMenuButton,
                    );
                },
            );
        });
}

fn cleanup_slot_hub(
    mut commands: Commands,
    roots: Query<Entity, With<SlotHubRoot>>,
    cameras: Query<Entity, With<SlotHubCamera>>,
) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    for entity in &cameras {
        commands.entity(entity).despawn();
    }
}

fn setup_load_world_select(
    mut commands: Commands,
    assets: Option<Res<AssetServer>>,
    mut save_state: ResMut<SaveGameState>,
) {
    let ui_texture: Handle<Image> = assets
        .as_ref()
        .map_or_else(Handle::default, |assets| load_main_menu_ui_texture(assets));
    save_state.refresh_worlds();

    commands.spawn((LoadWorldSelectCamera, Camera2d));
    commands
        .spawn((
            LoadWorldSelectRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(32.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.04, 0.05, 0.07)),
        ))
        .with_children(|parent| {
            spawn_menu_panel(
                parent,
                &ui_texture,
                "Load World",
                560.0,
                |panel, button_sprites, ui_texture| {
                    let enabled_color = Color::srgb(0.19, 0.12, 0.06);

                    if save_state.worlds.is_empty() {
                        panel.spawn((
                            Text::new("This slot has no worlds yet."),
                            TextFont {
                                font_size: 18.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.83, 0.8, 0.74)),
                        ));
                    } else {
                        for world in &save_state.worlds {
                            spawn_menu_button(
                                panel,
                                ui_texture,
                                button_sprites,
                                format!(
                                    "{}  |  created {}  |  last played {}",
                                    world.display_name,
                                    formatted_timestamp(world.created_at),
                                    formatted_timestamp(world.last_played_at)
                                ),
                                enabled_color,
                                SelectWorldButton {
                                    world_id: world.id.clone(),
                                },
                            );
                        }
                    }

                    spawn_menu_button(
                        panel,
                        ui_texture,
                        button_sprites,
                        "Back",
                        enabled_color,
                        BackToSlotHubButton,
                    );
                },
            );
        });
}

fn cleanup_load_world_select(
    mut commands: Commands,
    roots: Query<Entity, With<LoadWorldSelectRoot>>,
    cameras: Query<Entity, With<LoadWorldSelectCamera>>,
) {
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    for entity in &cameras {
        commands.entity(entity).despawn();
    }
}

fn handle_select_slot_button_interaction(
    mut interaction_query: Query<
        (
            &Interaction,
            &mut MainMenuButtonPressState,
            &SelectSlotButton,
        ),
        (Changed<Interaction>, Without<DisabledMenuButton>),
    >,
    mut save_state: ResMut<SaveGameState>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (interaction, mut press_state, button) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => press_state.armed = true,
            Interaction::Hovered if press_state.armed => {
                press_state.armed = false;
                if let Err(err) = savegame::touch_slot(&save_state.root_dir, &button.slot_id) {
                    error!("Failed to touch slot {}: {err}", button.slot_id);
                }
                save_state.activate_slot(button.slot_id.clone());
                next_state.set(AppState::SlotHub);
            }
            Interaction::None => press_state.armed = false,
            Interaction::Hovered => {}
        }
    }
}

fn handle_continue_world_button_interaction(
    mut interaction_query: Query<
        (&Interaction, &mut MainMenuButtonPressState),
        (
            Changed<Interaction>,
            With<ContinueWorldButton>,
            Without<DisabledMenuButton>,
        ),
    >,
    mut save_state: ResMut<SaveGameState>,
    mut world_manager: ResMut<WorldManager>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (interaction, mut press_state) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => press_state.armed = true,
            Interaction::Hovered if press_state.armed => {
                press_state.armed = false;
                let Some(slot_id) = save_state.active_slot_id.clone() else {
                    continue;
                };
                if let Ok(Some(world)) = savegame::most_recent_world(&save_state.root_dir, &slot_id)
                {
                    if activate_world(&mut save_state, &mut world_manager, &slot_id, &world.id)
                        .is_ok()
                    {
                        next_state.set(AppState::InGame);
                    }
                }
            }
            Interaction::None => press_state.armed = false,
            Interaction::Hovered => {}
        }
    }
}

fn handle_load_world_button_interaction(
    mut interaction_query: Query<
        (&Interaction, &mut MainMenuButtonPressState),
        (
            Changed<Interaction>,
            With<LoadWorldButton>,
            Without<DisabledMenuButton>,
        ),
    >,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (interaction, mut press_state) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => press_state.armed = true,
            Interaction::Hovered if press_state.armed => {
                press_state.armed = false;
                next_state.set(AppState::LoadWorldSelect);
            }
            Interaction::None => press_state.armed = false,
            Interaction::Hovered => {}
        }
    }
}

fn handle_new_world_button_interaction(
    mut interaction_query: Query<
        (&Interaction, &mut MainMenuButtonPressState),
        (Changed<Interaction>, With<NewWorldButton>),
    >,
    mut save_state: ResMut<SaveGameState>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (interaction, mut press_state) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => press_state.armed = true,
            Interaction::Hovered if press_state.armed => {
                press_state.armed = false;
                save_state.draft_target_slot_id = save_state.active_slot_id.clone();
                next_state.set(AppState::DraftSetup);
            }
            Interaction::None => press_state.armed = false,
            Interaction::Hovered => {}
        }
    }
}

fn handle_select_world_button_interaction(
    mut interaction_query: Query<
        (
            &Interaction,
            &mut MainMenuButtonPressState,
            &SelectWorldButton,
        ),
        (Changed<Interaction>, Without<DisabledMenuButton>),
    >,
    mut save_state: ResMut<SaveGameState>,
    mut world_manager: ResMut<WorldManager>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (interaction, mut press_state, button) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => press_state.armed = true,
            Interaction::Hovered if press_state.armed => {
                press_state.armed = false;
                let Some(slot_id) = save_state.active_slot_id.clone() else {
                    continue;
                };
                if activate_world(
                    &mut save_state,
                    &mut world_manager,
                    &slot_id,
                    &button.world_id,
                )
                .is_ok()
                {
                    next_state.set(AppState::InGame);
                }
            }
            Interaction::None => press_state.armed = false,
            Interaction::Hovered => {}
        }
    }
}

fn handle_back_to_main_menu_button_interaction(
    mut interaction_query: Query<
        (&Interaction, &mut MainMenuButtonPressState),
        (Changed<Interaction>, With<BackToMainMenuButton>),
    >,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (interaction, mut press_state) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => press_state.armed = true,
            Interaction::Hovered if press_state.armed => {
                press_state.armed = false;
                next_state.set(AppState::MainMenu);
            }
            Interaction::None => press_state.armed = false,
            Interaction::Hovered => {}
        }
    }
}

fn handle_back_to_slot_hub_button_interaction(
    mut interaction_query: Query<
        (&Interaction, &mut MainMenuButtonPressState),
        (Changed<Interaction>, With<BackToSlotHubButton>),
    >,
    mut next_state: ResMut<NextState<AppState>>,
) {
    for (interaction, mut press_state) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => press_state.armed = true,
            Interaction::Hovered if press_state.armed => {
                press_state.armed = false;
                next_state.set(AppState::SlotHub);
            }
            Interaction::None => press_state.armed = false,
            Interaction::Hovered => {}
        }
    }
}

fn activate_world(
    save_state: &mut SaveGameState,
    world_manager: &mut WorldManager,
    slot_id: &str,
    world_id: &str,
) -> Result<(), savegame::SaveGameError> {
    savegame::touch_world(&save_state.root_dir, slot_id, world_id)?;
    save_state.active_slot_id = Some(slot_id.to_string());
    save_state.active_world_id = Some(world_id.to_string());
    save_state.refresh_slots();
    save_state.refresh_worlds();
    world_manager.save_directory =
        savegame::world_save_path(&save_state.root_dir, slot_id, world_id);
    Ok(())
}

fn pregenerate_world_if_needed(
    root_dir: &Path,
    slot_id: &str,
    world: &WorldMetadata,
) -> Result<(), String> {
    let generation = world.generation.unwrap_or_default();
    let world_dir = savegame::world_save_path(root_dir, slot_id, &world.id);

    for chunk in pregenerate_world_chunks(generation) {
        let chunk_path = world_dir.join("chunks").join(format!(
            "chunk_{}_{}.bin",
            chunk.position.x, chunk.position.y
        ));
        world::serialization::save_chunk(&chunk, &chunk_path).map_err(|err| {
            format!(
                "failed to save pregenerated chunk {:?}: {err}",
                chunk.position
            )
        })?;
    }

    Ok(())
}

fn saved_actor(
    position: &Position,
    velocity: &Velocity,
    direction: &Direction,
    state: &entities::EntityState,
    health: &Health,
) -> SavedActor {
    SavedActor {
        position: *position,
        velocity: *velocity,
        direction: *direction,
        state: *state,
        health: *health,
    }
}

fn collect_saved_world_entities(
    human_query: &Query<
        (
            &Position,
            &Velocity,
            &Direction,
            &entities::EntityState,
            &Health,
        ),
        With<Human>,
    >,
    guardian_query: &Query<(
        &Position,
        &Velocity,
        &Direction,
        &entities::EntityState,
        &Health,
        &ForestGuardian,
        &RoamingBehavior,
        &TreeSpawner,
    )>,
    snail_query: &Query<
        (
            &Position,
            &Velocity,
            &Direction,
            &entities::EntityState,
            &Health,
            &WindingPath,
        ),
        With<Snail>,
    >,
    tree_spirit_query: &Query<(&Position, &GrowingTree), With<TreeSpirit>>,
    variant_tree_query: &Query<(&Position, &GrowingTree), With<VariantTree>>,
) -> SavedWorldEntities {
    let mut entities = Vec::new();

    for (position, velocity, direction, state, health) in human_query.iter() {
        entities.push(SavedEntity::Human(saved_actor(
            position, velocity, direction, state, health,
        )));
    }

    for (position, velocity, direction, state, health, guardian, roaming, tree_spawner) in
        guardian_query.iter()
    {
        entities.push(SavedEntity::ForestGuardian(SavedForestGuardian {
            actor: saved_actor(position, velocity, direction, state, health),
            guardian: *guardian,
            roaming: *roaming,
            tree_spawner: *tree_spawner,
        }));
    }

    for (position, velocity, direction, state, health, winding_path) in snail_query.iter() {
        entities.push(SavedEntity::Snail(SavedSnail {
            actor: saved_actor(position, velocity, direction, state, health),
            winding_path: winding_path.clone(),
        }));
    }

    for (position, growing_tree) in tree_spirit_query.iter() {
        entities.push(SavedEntity::TreeSpirit(SavedGrowingTreeEntity {
            position: *position,
            growing_tree: *growing_tree,
        }));
    }

    for (position, growing_tree) in variant_tree_query.iter() {
        entities.push(SavedEntity::VariantTree(SavedGrowingTreeEntity {
            position: *position,
            growing_tree: *growing_tree,
        }));
    }

    SavedWorldEntities::new(entities)
}

fn spawn_saved_world_entities(
    commands: &mut Commands,
    assets: &Res<AssetServer>,
    texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
    saved_world: SavedWorldEntities,
) {
    for saved_entity in saved_world.entities {
        match saved_entity {
            SavedEntity::Human(actor) => {
                spawn_saved_human(
                    commands,
                    SavedActorState {
                        position: actor.position,
                        velocity: actor.velocity,
                        direction: actor.direction,
                        state: actor.state,
                        health: actor.health,
                    },
                    assets,
                    texture_atlas_layouts,
                );
            }
            SavedEntity::ForestGuardian(saved) => {
                spawn_saved_forest_guardian(
                    commands,
                    SavedActorState {
                        position: saved.actor.position,
                        velocity: saved.actor.velocity,
                        direction: saved.actor.direction,
                        state: saved.actor.state,
                        health: saved.actor.health,
                    },
                    saved.guardian,
                    saved.roaming,
                    saved.tree_spawner,
                    assets,
                    texture_atlas_layouts,
                );
            }
            SavedEntity::Snail(saved) => {
                spawn_saved_snail(
                    commands,
                    SavedActorState {
                        position: saved.actor.position,
                        velocity: saved.actor.velocity,
                        direction: saved.actor.direction,
                        state: saved.actor.state,
                        health: saved.actor.health,
                    },
                    saved.winding_path,
                    assets,
                    texture_atlas_layouts,
                );
            }
            SavedEntity::TreeSpirit(saved) => {
                spawn_saved_tree_spirit(
                    commands,
                    saved.position,
                    saved.growing_tree,
                    assets,
                    texture_atlas_layouts,
                );
            }
            SavedEntity::VariantTree(saved) => {
                spawn_saved_variant_tree(
                    commands,
                    saved.position,
                    saved.growing_tree,
                    assets,
                    texture_atlas_layouts,
                );
            }
        }
    }
}

fn point_in_ui_node(
    point: Vec2,
    computed_node: &ComputedNode,
    transform: &UiGlobalTransform,
) -> bool {
    let Some(inverse) = transform.try_inverse() else {
        return false;
    };

    let local_point = inverse.transform_point2(point) + 0.5 * computed_node.size();
    (0.0..=computed_node.size.x).contains(&local_point.x)
        && (0.0..=computed_node.size.y).contains(&local_point.y)
}

fn cursor_over_blocking_ui(
    cursor: Vec2,
    ui_nodes: &Query<
        (
            &ComputedNode,
            &UiGlobalTransform,
            &Node,
            Option<&Visibility>,
            Option<&InheritedVisibility>,
        ),
        With<Node>,
    >,
) -> bool {
    for (computed_node, transform, node, visibility, inherited_visibility) in ui_nodes.iter() {
        if node.display == Display::None {
            continue;
        }

        if let Some(visibility) = visibility {
            if *visibility == Visibility::Hidden {
                continue;
            }
        }

        if let Some(inherited_visibility) = inherited_visibility {
            if !inherited_visibility.get() {
                continue;
            }
        }

        if computed_node.size().x <= 0.0 || computed_node.size().y <= 0.0 {
            continue;
        }

        if point_in_ui_node(cursor, computed_node, transform) {
            return true;
        }
    }

    false
}

fn setup_draft_setup(
    mut commands: Commands,
    assets: Option<Res<AssetServer>>,
    mut texture_atlas_layouts: Option<ResMut<Assets<TextureAtlasLayout>>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut draft_board: ResMut<DraftBoard>,
    mut draft_drag_state: ResMut<DraftDragState>,
    save_state: Res<SaveGameState>,
) {
    let available_columns = save_state
        .draft_target_slot_id
        .as_deref()
        .or(save_state.active_slot_id.as_deref())
        .and_then(|slot_id| save_state.slots.iter().find(|slot| slot.id == slot_id))
        .map(|slot| slot.available_columns)
        .unwrap_or(1);
    draft_board.reset(available_columns);
    draft_drag_state.active_card = None;
    draft_drag_state.source_cell = None;
    draft_drag_state.hovered_cell = None;
    draft_drag_state.cursor_pos = None;

    let frame_texture = assets
        .as_ref()
        .map(|assets| assets.load(DRAFT_CARD_FRAME_TEXTURE_PATH))
        .unwrap_or_default();
    let name_plate_texture = assets
        .as_ref()
        .map(|assets| assets.load(DRAFT_NAME_PLATE_TEXTURE_PATH))
        .unwrap_or_default();
    let ui_texture: Handle<Image> = assets
        .as_ref()
        .map_or_else(Handle::default, |assets| load_main_menu_ui_texture(assets));
    let frame_layout = texture_atlas_layouts
        .as_deref_mut()
        .map(|layouts| {
            layouts.add(TextureAtlasLayout::from_grid(
                DRAFT_CARD_FRAME_TILE_SIZE,
                DRAFT_CARD_FRAME_COLUMNS,
                DRAFT_CARD_FRAME_ROWS,
                None,
                None,
            ))
        })
        .unwrap_or_default();
    let button_sprites = MainMenuButtonSprites {
        standard: main_menu_button_standard_rect(),
        hovered: main_menu_button_hovered_rect(),
        pressed: main_menu_button_clicked_rect(),
    };
    let draft_layout_scale = windows
        .single()
        .ok()
        .map(draft_layout_scale_for_window)
        .unwrap_or(1.0);

    commands.spawn((DraftSetupCamera, Camera2d));

    commands
        .spawn((
            DraftSetupRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.05, 0.07, 0.09)),
        ))
        .with_children(|root| {
            root.spawn((
                DraftSetupScaler,
                Node {
                    width: Val::Px(DRAFT_LAYOUT_WIDTH),
                    height: Val::Px(DRAFT_LAYOUT_HEIGHT),
                    justify_content: JustifyContent::Start,
                    align_items: AlignItems::Center,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(12.0),
                    padding: UiRect::axes(
                        Val::Px(DRAFT_LAYOUT_HORIZONTAL_PADDING),
                        Val::Px(DRAFT_LAYOUT_VERTICAL_PADDING),
                    ),
                    ..default()
                },
                UiTransform::from_scale(Vec2::splat(draft_layout_scale)),
            ))
            .with_children(|content| {
                content.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    Text::new("Build your starting spread"),
                    TextFont {
                        font_size: 32.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.92, 0.88, 0.78)),
                ));

                content.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::Center,
                        margin: UiRect::bottom(Val::Px(8.0)),
                        ..default()
                    },
                    Text::new(
                        "First column is the world recipe: row 1 shape, row 2 element. Drag cards into unlocked columns. Island and Infinity cards unlock the row below.",
                    ),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.71, 0.76, 0.78)),
                ));

                content
                    .spawn((Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(10.0),
                        ..default()
                    },))
                    .with_children(|grid_container| {
                        for column in 0..DRAFT_MAX_COLUMNS {
                            grid_container
                                .spawn((
                                    DraftGridColumn { index: column },
                                    Node {
                                        flex_direction: FlexDirection::Column,
                                        row_gap: Val::Px(10.0),
                                        ..default()
                                    },
                                ))
                                .with_children(|grid_column| {
                                    for row in 0..DRAFT_MAX_ROWS {
                                        grid_column
                                            .spawn((
                                                Button,
                                                DraftGridCell { column, row },
                                                Node {
                                                    width: Val::Px(DRAFT_GRID_SLOT_WIDTH),
                                                    height: Val::Px(DRAFT_GRID_SLOT_HEIGHT),
                                                    justify_content: JustifyContent::Center,
                                                    align_items: AlignItems::Center,
                                                    border: UiRect::all(Val::Px(1.0)),
                                                    ..default()
                                                },
                                                BackgroundColor(Color::srgb(0.11, 0.12, 0.14)),
                                                BorderColor::all(Color::srgb(0.22, 0.25, 0.29)),
                                            ))
                                            .with_children(|cell| {
                                                let mut icon = ImageNode::default();
                                                let mut icon_node = Node {
                                                    width: Val::Px(DRAFT_CARD_ICON_SIZE),
                                                    height: Val::Px(DRAFT_CARD_ICON_SIZE),
                                                    ..default()
                                                };
                                                let mut infinity_node = Node::default();
                                                if let Some(assets) = assets.as_deref() {
                                                    apply_draft_card_icon(
                                                        &mut icon,
                                                        &mut icon_node,
                                                        &mut infinity_node,
                                                        DraftCard::Human,
                                                        Some(assets),
                                                        texture_atlas_layouts.as_deref_mut(),
                                                    );
                                                }

                                                cell.spawn((
                                                    DraftPlacedCardVisual,
                                                    Node {
                                                        width: Val::Px(DRAFT_GRID_CARD_WIDTH),
                                                        height: Val::Px(DRAFT_GRID_CARD_HEIGHT),
                                                        justify_content: JustifyContent::Center,
                                                        align_items: AlignItems::Center,
                                                        ..default()
                                                    },
                                                    Visibility::Hidden,
                                                ))
                                                .with_children(|frame| {
                                                    frame.spawn((
                                                        DraftCardFrame,
                                                        ImageNode {
                                                            image: frame_texture.clone(),
                                                            image_mode: NodeImageMode::Stretch,
                                                            texture_atlas: Some(TextureAtlas {
                                                                layout: frame_layout.clone(),
                                                                index: DraftCard::Human
                                                                    .frame_index(),
                                                            }),
                                                            ..default()
                                                        },
                                                        Node {
                                                            width: Val::Px(DRAFT_GRID_CARD_WIDTH),
                                                            height: Val::Px(DRAFT_GRID_CARD_HEIGHT),
                                                            position_type: PositionType::Absolute,
                                                            ..default()
                                                        },
                                                    ));

                                                    frame.spawn((
                                                        DraftPlacedCardIcon,
                                                        icon,
                                                        icon_node,
                                                    ));

                                                    spawn_infinity_icon(
                                                        frame,
                                                        DraftPlacedCardInfinityIcon,
                                                        false,
                                                    );

                                                    frame.spawn((
                                                        DraftCardFrame,
                                                        ImageNode {
                                                            image: frame_texture.clone(),
                                                            image_mode: NodeImageMode::Stretch,
                                                            texture_atlas: Some(TextureAtlas {
                                                                layout: frame_layout.clone(),
                                                                index: DraftCard::Human
                                                                    .frame_index(),
                                                            }),
                                                            ..default()
                                                        },
                                                        Node {
                                                            width: Val::Px(DRAFT_GRID_CARD_WIDTH),
                                                            height: Val::Px(DRAFT_GRID_CARD_HEIGHT),
                                                            position_type: PositionType::Absolute,
                                                            ..default()
                                                        },
                                                    ));
                                                });
                                            });
                                    }
                                });
                        }
                    });

                content
                    .spawn((Node {
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(18.0),
                        flex_wrap: FlexWrap::Wrap,
                        ..default()
                    },))
                    .with_children(|tray| {
                        for card in VISIBLE_DRAFT_CARDS {
                            let label = card.label();
                            let label_plate_height = label_plate_height(label);
                            tray.spawn((
                                Button,
                                DraftTrayCard,
                                DraftCardTypeComponent(card),
                                Node {
                                    width: Val::Px(DRAFT_GRID_CARD_WIDTH),
                                    height: Val::Px(DRAFT_TRAY_CARD_HEIGHT),
                                    justify_content: JustifyContent::Start,
                                    align_items: AlignItems::Center,
                                    flex_direction: FlexDirection::Column,
                                    row_gap: Val::Px(6.0),
                                    border: UiRect::all(Val::Px(1.0)),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.0)),
                                BorderColor::all(Color::srgba(0.95, 0.84, 0.58, 0.0)),
                            ))
                            .with_children(|card_button| {
                                card_button
                                    .spawn((Node {
                                        width: Val::Px(DRAFT_GRID_CARD_WIDTH),
                                        height: Val::Px(DRAFT_GRID_CARD_HEIGHT),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },))
                                    .with_children(|frame| {
                                        frame.spawn((
                                            DraftCardFrame,
                                            ImageNode {
                                                image: frame_texture.clone(),
                                                image_mode: NodeImageMode::Stretch,
                                                texture_atlas: Some(TextureAtlas {
                                                    layout: frame_layout.clone(),
                                                    index: card.frame_index(),
                                                }),
                                                ..default()
                                            },
                                            Node {
                                                width: Val::Px(DRAFT_GRID_CARD_WIDTH),
                                                height: Val::Px(DRAFT_GRID_CARD_HEIGHT),
                                                position_type: PositionType::Absolute,
                                                ..default()
                                            },
                                        ));

                                        let mut icon = ImageNode::default();
                                        let mut icon_node = Node {
                                            width: Val::Px(DRAFT_CARD_ICON_SIZE),
                                            height: Val::Px(DRAFT_CARD_ICON_SIZE),
                                            ..default()
                                        };
                                        let mut infinity_node = Node::default();
                                        if let Some(assets) = assets.as_deref() {
                                            apply_draft_card_icon(
                                                &mut icon,
                                                &mut icon_node,
                                                &mut infinity_node,
                                                card,
                                                Some(assets),
                                                texture_atlas_layouts.as_deref_mut(),
                                            );
                                        }

                                        frame.spawn((
                                            DraftCardIcon,
                                            icon,
                                            icon_node,
                                        ));

                                        spawn_infinity_icon(
                                            frame,
                                            DraftCardInfinityIcon,
                                            card == DraftCard::Infinity,
                                        );

                                        frame.spawn((
                                            DraftCardFrame,
                                            ImageNode {
                                                image: frame_texture.clone(),
                                                image_mode: NodeImageMode::Stretch,
                                                texture_atlas: Some(TextureAtlas {
                                                    layout: frame_layout.clone(),
                                                    index: card.frame_index(),
                                                }),
                                                ..default()
                                            },
                                            Node {
                                                width: Val::Px(DRAFT_GRID_CARD_WIDTH),
                                                height: Val::Px(DRAFT_GRID_CARD_HEIGHT),
                                                position_type: PositionType::Absolute,
                                                ..default()
                                            },
                                        ));
                                    });

                                card_button
                                    .spawn((
                                        ImageNode {
                                            image: name_plate_texture.clone(),
                                            rect: Some(Rect::new(8.0, 8.0, 32.0, 32.0)),
                                            image_mode: NodeImageMode::Sliced(TextureSlicer {
                                                border: BorderRect::all(6.0),
                                                center_scale_mode: SliceScaleMode::Tile {
                                                    stretch_value: 1.0,
                                                },
                                                sides_scale_mode: SliceScaleMode::Tile {
                                                    stretch_value: 1.0,
                                                },
                                                max_corner_scale: 1.0,
                                            }),
                                            ..default()
                                        },
                                        Node {
                                            width: Val::Px(DRAFT_NAME_PLATE_WIDTH),
                                            height: Val::Px(label_plate_height),
                                            justify_content: JustifyContent::Center,
                                            align_items: AlignItems::Center,
                                            margin: UiRect::top(Val::Px(-4.0)),
                                            ..default()
                                        },
                                    ))
                                    .with_children(|plate| {
                                        plate.spawn((
                                            Text::new(label),
                                            TextFont {
                                                font_size: 16.0,
                                                ..default()
                                            },
                                            TextColor(Color::srgb(0.17, 0.1, 0.04)),
                                        ));
                                    });
                            });
                        }
                    });

                content
                    .spawn((
                        Button,
                        DraftConfirmButton,
                        button_sprites,
                        DraftConfirmButtonPressState::default(),
                        Node {
                            width: Val::Px(MAIN_MENU_BUTTON_WIDTH),
                            height: Val::Px(MAIN_MENU_BUTTON_HEIGHT),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            padding: UiRect::horizontal(Val::Px(24.0)),
                            margin: UiRect::top(Val::Px(8.0)),
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                    ))
                    .with_children(|button| {
                        button.spawn((
                            MainMenuButtonImage,
                            ImageNode {
                                image: ui_texture,
                                rect: Some(main_menu_button_standard_rect()),
                                image_mode: main_menu_button_image_mode(),
                                ..default()
                            },
                            Node {
                                width: Val::Px(MAIN_MENU_BUTTON_SOURCE_WIDTH),
                                height: Val::Px(MAIN_MENU_BUTTON_SOURCE_HEIGHT),
                                position_type: PositionType::Absolute,
                                ..default()
                            },
                            UiTransform::from_scale(Vec2::splat(MAIN_MENU_BUTTON_SCALE)),
                        ));
                        button.spawn((
                            Text::new("Confirm"),
                            DraftConfirmButtonLabel,
                            TextFont {
                                font_size: 28.0,
                                ..default()
                            },
                            TextColor(Color::srgb(0.19, 0.12, 0.06)),
                        ));
                    });
            });
        });

    commands
        .spawn((
            DraftCardGhost,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(DRAFT_GRID_CARD_WIDTH),
                height: Val::Px(DRAFT_GRID_CARD_HEIGHT),
                left: Val::Px(-500.0),
                top: Val::Px(-500.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                display: Display::None,
                ..default()
            },
            GlobalZIndex(100),
            UiTransform::from_scale(Vec2::splat(draft_layout_scale)),
        ))
        .with_children(|ghost| {
            ghost.spawn((
                DraftCardFrame,
                ImageNode {
                    image: frame_texture.clone(),
                    image_mode: NodeImageMode::Stretch,
                    texture_atlas: Some(TextureAtlas {
                        layout: frame_layout.clone(),
                        index: DraftCard::Human.frame_index(),
                    }),
                    ..default()
                },
                Node {
                    width: Val::Px(DRAFT_GRID_CARD_WIDTH),
                    height: Val::Px(DRAFT_GRID_CARD_HEIGHT),
                    position_type: PositionType::Absolute,
                    ..default()
                },
            ));

            let mut icon = ImageNode::default();
            let mut icon_node = Node {
                width: Val::Px(DRAFT_CARD_ICON_SIZE),
                height: Val::Px(DRAFT_CARD_ICON_SIZE),
                ..default()
            };
            let mut infinity_node = Node::default();
            if let Some(assets) = assets.as_deref() {
                apply_draft_card_icon(
                    &mut icon,
                    &mut icon_node,
                    &mut infinity_node,
                    DraftCard::Human,
                    Some(assets),
                    texture_atlas_layouts.as_deref_mut(),
                );
            }
            ghost.spawn((DraftCardIcon, icon, icon_node));

            spawn_infinity_icon(ghost, DraftCardInfinityIcon, false);

            ghost.spawn((
                DraftCardFrame,
                ImageNode {
                    image: frame_texture,
                    image_mode: NodeImageMode::Stretch,
                    texture_atlas: Some(TextureAtlas {
                        layout: frame_layout,
                        index: DraftCard::Human.frame_index(),
                    }),
                    ..default()
                },
                Node {
                    width: Val::Px(DRAFT_GRID_CARD_WIDTH),
                    height: Val::Px(DRAFT_GRID_CARD_HEIGHT),
                    position_type: PositionType::Absolute,
                    ..default()
                },
            ));
        });
}

fn draft_layout_scale_for_window(window: &Window) -> f32 {
    let available_width = (window.width() - (DRAFT_LAYOUT_HORIZONTAL_PADDING * 2.0)).max(1.0);
    let available_height = (window.height() - (DRAFT_LAYOUT_VERTICAL_PADDING * 2.0)).max(1.0);

    (available_width / DRAFT_LAYOUT_WIDTH)
        .min(available_height / DRAFT_LAYOUT_HEIGHT)
        .min(1.0)
}

fn update_draft_layout_scale(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut layout_roots: Query<&mut UiTransform, With<DraftSetupScaler>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Ok(mut transform) = layout_roots.single_mut() else {
        return;
    };

    let scale = draft_layout_scale_for_window(window);
    transform.scale = Vec2::splat(scale);
}

fn cleanup_draft_setup(
    mut commands: Commands,
    draft_roots: Query<Entity, With<DraftSetupRoot>>,
    draft_cameras: Query<Entity, With<DraftSetupCamera>>,
    draft_ghosts: Query<Entity, With<DraftCardGhost>>,
    mut draft_drag_state: ResMut<DraftDragState>,
) {
    draft_drag_state.active_card = None;
    draft_drag_state.source_cell = None;
    draft_drag_state.hovered_cell = None;
    draft_drag_state.cursor_pos = None;

    for entity in &draft_roots {
        commands.entity(entity).despawn();
    }

    for entity in &draft_cameras {
        commands.entity(entity).despawn();
    }

    for entity in &draft_ghosts {
        commands.entity(entity).despawn();
    }
}

fn start_draft_card_drag(
    mut tray_cards: Query<
        (&Interaction, &DraftCardTypeComponent),
        (Changed<Interaction>, With<DraftTrayCard>),
    >,
    mut grid_cells: Query<(&Interaction, &DraftGridCell), Changed<Interaction>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    draft_board: Res<DraftBoard>,
    mut draft_drag_state: ResMut<DraftDragState>,
) {
    if !mouse_button.just_pressed(MouseButton::Left) {
        return;
    }

    let cursor_pos = windows
        .single()
        .ok()
        .and_then(|window| window.physical_cursor_position());

    for (interaction, card) in &mut tray_cards {
        if *interaction == Interaction::Pressed {
            draft_drag_state.active_card = Some(card.0);
            draft_drag_state.source_cell = None;
            draft_drag_state.hovered_cell = None;
            draft_drag_state.cursor_pos = cursor_pos;
            break;
        }
    }

    if draft_drag_state.active_card.is_some() {
        return;
    }

    for (interaction, cell) in &mut grid_cells {
        if *interaction != Interaction::Pressed {
            continue;
        }

        let source_cell = DraftCellPosition {
            column: cell.column,
            row: cell.row,
        };
        let Some(card) = draft_board.card_at(source_cell.column, source_cell.row) else {
            continue;
        };

        draft_drag_state.active_card = Some(card);
        draft_drag_state.source_cell = Some(source_cell);
        draft_drag_state.hovered_cell = None;
        draft_drag_state.cursor_pos = cursor_pos;
        break;
    }
}

fn update_draft_drag_cursor(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut draft_drag_state: ResMut<DraftDragState>,
) {
    if draft_drag_state.active_card.is_none() {
        draft_drag_state.cursor_pos = None;
        return;
    }

    draft_drag_state.cursor_pos = windows
        .single()
        .ok()
        .and_then(|window| window.physical_cursor_position());
}

fn update_draft_hovered_cell(
    mut next_hovered_cell: Local<Option<Option<usize>>>,
    cells: Query<(&DraftGridCell, &ComputedNode, &UiGlobalTransform, &Node)>,
    mut draft_drag_state: ResMut<DraftDragState>,
    draft_board: Res<DraftBoard>,
) {
    if draft_drag_state.active_card.is_none() {
        if *next_hovered_cell != Some(None) {
            draft_drag_state.hovered_cell = None;
            *next_hovered_cell = Some(None);
        }
        return;
    }

    let hovered = draft_drag_state.cursor_pos.and_then(|cursor| {
        cells
            .iter()
            .find_map(|(cell, computed_node, transform, node)| {
                if node.display == Display::None
                    || !draft_board.can_place_card_with_omission(
                        cell.column,
                        cell.row,
                        draft_drag_state.active_card?,
                        draft_drag_state.source_cell,
                    )
                {
                    return None;
                }

                point_in_ui_node(cursor, computed_node, transform)
                    .then_some(cell.column * DRAFT_MAX_ROWS + cell.row)
            })
    });

    if *next_hovered_cell != Some(hovered) {
        draft_drag_state.hovered_cell = hovered;
        *next_hovered_cell = Some(hovered);
    }
}

fn handle_draft_card_drop(
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut draft_board: ResMut<DraftBoard>,
    mut draft_drag_state: ResMut<DraftDragState>,
) {
    if !mouse_button.just_released(MouseButton::Left) {
        return;
    }

    if let Some(card) = draft_drag_state.active_card {
        match (draft_drag_state.source_cell, draft_drag_state.hovered_cell) {
            (Some(source), Some(index)) => {
                let target = DraftCellPosition {
                    column: index / DRAFT_MAX_ROWS,
                    row: index % DRAFT_MAX_ROWS,
                };

                if source != target
                    && draft_board.can_place_card_with_omission(
                        target.column,
                        target.row,
                        card,
                        Some(source),
                    )
                {
                    draft_board.remove_card(source.column, source.row);
                    draft_board.place_card(target.column, target.row, card);
                }
            }
            (Some(source), None) => {
                draft_board.remove_card(source.column, source.row);
            }
            (None, Some(index)) => {
                let column = index / DRAFT_MAX_ROWS;
                let row = index % DRAFT_MAX_ROWS;
                if draft_board.can_place_card(column, row, card) {
                    draft_board.place_card(column, row, card);
                }
            }
            (None, None) => {}
        }
    }

    draft_drag_state.active_card = None;
    draft_drag_state.source_cell = None;
    draft_drag_state.hovered_cell = None;
    draft_drag_state.cursor_pos = None;
}

fn handle_draft_debug_unlock_columns(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut draft_board: ResMut<DraftBoard>,
    mut save_state: ResMut<SaveGameState>,
) {
    if !keyboard.just_pressed(KeyCode::Equal) {
        return;
    }

    let Some(slot_id) = save_state
        .draft_target_slot_id
        .clone()
        .or_else(|| save_state.active_slot_id.clone())
    else {
        return;
    };

    let next_columns = (draft_board.available_columns + 1).min(DRAFT_MAX_COLUMNS as u8);
    if next_columns == draft_board.available_columns {
        return;
    }

    if let Err(err) =
        savegame::set_slot_available_columns(&save_state.root_dir, &slot_id, next_columns)
    {
        error!("Failed to update draft columns for slot {slot_id}: {err}");
        return;
    }

    draft_board.available_columns = next_columns;
    save_state.refresh_slots();
}

fn handle_draft_confirm_button_interaction(
    mut buttons: Query<
        (&Interaction, &mut DraftConfirmButtonPressState),
        (
            Changed<Interaction>,
            With<DraftConfirmButton>,
            Without<DisabledMenuButton>,
        ),
    >,
    draft_board: Res<DraftBoard>,
    mut save_state: ResMut<SaveGameState>,
    mut world_manager: ResMut<WorldManager>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if !draft_confirm_enabled(&draft_board) {
        for (_, mut press_state) in &mut buttons {
            press_state.armed = false;
        }
        return;
    }

    for (interaction, mut press_state) in &mut buttons {
        match *interaction {
            Interaction::Pressed => {
                press_state.armed = true;
            }
            Interaction::Hovered if press_state.armed => {
                press_state.armed = false;
                let Some(slot_id) = save_state.draft_target_slot_id.clone() else {
                    error!("Draft confirm missing target slot");
                    continue;
                };
                let Some(generation) = draft_generation_config(&draft_board) else {
                    warn!(
                        "Draft confirm requires a valid recipe in the first column: shape on row 1 and element on row 2"
                    );
                    continue;
                };

                match savegame::create_world(&save_state.root_dir, &slot_id, generation) {
                    Ok(world) => {
                        if let Err(err) =
                            pregenerate_world_if_needed(&save_state.root_dir, &slot_id, &world)
                        {
                            error!(
                                "Failed to pregenerate world {} for slot {}: {}",
                                world.id, slot_id, err
                            );
                            let world_root =
                                savegame::world_path(&save_state.root_dir, &slot_id, &world.id);
                            if let Err(remove_err) = fs::remove_dir_all(&world_root) {
                                error!(
                                    "Failed to clean up partially created world {}: {}",
                                    world.id, remove_err
                                );
                            }
                            continue;
                        }

                        if activate_world(&mut save_state, &mut world_manager, &slot_id, &world.id)
                            .is_ok()
                        {
                            save_state.draft_target_slot_id = None;
                            next_state.set(AppState::InGame);
                        }
                    }
                    Err(err) => error!("Failed to create world for slot {slot_id}: {err}"),
                }
            }
            Interaction::None => {
                press_state.armed = false;
            }
            Interaction::Hovered => {}
        }
    }
}

fn update_draft_visuals(
    draft_board: Res<DraftBoard>,
    draft_drag_state: Res<DraftDragState>,
    assets: Option<Res<AssetServer>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut texture_atlas_layouts: Option<ResMut<Assets<TextureAtlasLayout>>>,
    mut tray_cards: Query<
        (
            &DraftCardTypeComponent,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        (With<DraftTrayCard>, Without<DraftGridCell>),
    >,
    mut cells: Query<
        (
            &DraftGridCell,
            &mut Node,
            &Children,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        (
            With<DraftGridCell>,
            Without<DraftTrayCard>,
            Without<DraftCardGhost>,
        ),
    >,
    mut columns: Query<
        (&DraftGridColumn, &mut Node),
        (Without<DraftGridCell>, Without<DraftCardGhost>),
    >,
    mut placed_cards: Query<(&Children, &mut Visibility), With<DraftPlacedCardVisual>>,
    mut placed_card_icons: Query<
        (&mut ImageNode, &mut Node),
        (
            With<DraftPlacedCardIcon>,
            Without<DraftCardIcon>,
            Without<DraftGridCell>,
            Without<DraftGridColumn>,
            Without<DraftPlacedCardInfinityIcon>,
            Without<DraftCardFrame>,
            Without<DraftCardGhost>,
        ),
    >,
    mut placed_card_infinity_icons: Query<
        &mut Node,
        (
            With<DraftPlacedCardInfinityIcon>,
            Without<DraftPlacedCardIcon>,
            Without<DraftGridCell>,
            Without<DraftGridColumn>,
            Without<DraftCardFrame>,
            Without<DraftCardGhost>,
        ),
    >,
    mut draft_card_frames: Query<
        &mut ImageNode,
        (
            With<DraftCardFrame>,
            Without<DraftPlacedCardIcon>,
            Without<DraftPlacedCardInfinityIcon>,
            Without<DraftCardIcon>,
            Without<DraftCardInfinityIcon>,
            Without<DraftGridCell>,
            Without<DraftGridColumn>,
            Without<DraftCardGhost>,
        ),
    >,
    mut ghost_query: Query<(&Children, &mut Node, &mut UiTransform), With<DraftCardGhost>>,
    mut ghost_icons: Query<
        (&mut ImageNode, &mut Node),
        (
            With<DraftCardIcon>,
            Without<DraftPlacedCardIcon>,
            Without<DraftPlacedCardInfinityIcon>,
            Without<DraftCardInfinityIcon>,
            Without<DraftCardFrame>,
            Without<DraftGridCell>,
            Without<DraftGridColumn>,
            Without<DraftCardGhost>,
        ),
    >,
    mut ghost_infinity_icons: Query<
        &mut Node,
        (
            With<DraftCardInfinityIcon>,
            Without<DraftPlacedCardIcon>,
            Without<DraftPlacedCardInfinityIcon>,
            Without<DraftCardIcon>,
            Without<DraftCardFrame>,
            Without<DraftGridCell>,
            Without<DraftGridColumn>,
            Without<DraftCardGhost>,
        ),
    >,
) {
    fn update_card_icon_children(
        children: &Children,
        card: DraftCard,
        assets_ref: Option<&AssetServer>,
        texture_atlas_layouts: Option<&mut Assets<TextureAtlasLayout>>,
        image_query: &mut Query<
            (&mut ImageNode, &mut Node),
            (
                With<DraftPlacedCardIcon>,
                Without<DraftCardIcon>,
                Without<DraftGridCell>,
                Without<DraftGridColumn>,
                Without<DraftPlacedCardInfinityIcon>,
                Without<DraftCardFrame>,
                Without<DraftCardGhost>,
            ),
        >,
        infinity_query: &mut Query<
            &mut Node,
            (
                With<DraftPlacedCardInfinityIcon>,
                Without<DraftPlacedCardIcon>,
                Without<DraftGridCell>,
                Without<DraftGridColumn>,
                Without<DraftCardFrame>,
                Without<DraftCardGhost>,
            ),
        >,
    ) {
        let mut image_child = None;
        let mut infinity_child = None;

        for child in children.iter() {
            if image_query.get_mut(child).is_ok() {
                image_child = Some(child);
            }
            if infinity_query.get_mut(child).is_ok() {
                infinity_child = Some(child);
            }
        }

        let (Some(image_child), Some(infinity_child)) = (image_child, infinity_child) else {
            return;
        };
        let Ok((mut image, mut image_node)) = image_query.get_mut(image_child) else {
            return;
        };
        let Ok(mut infinity_node) = infinity_query.get_mut(infinity_child) else {
            return;
        };

        apply_draft_card_icon(
            &mut image,
            &mut image_node,
            &mut infinity_node,
            card,
            assets_ref,
            texture_atlas_layouts,
        );
    }

    fn update_ghost_icon_children(
        children: &Children,
        card: DraftCard,
        assets_ref: Option<&AssetServer>,
        texture_atlas_layouts: Option<&mut Assets<TextureAtlasLayout>>,
        image_query: &mut Query<
            (&mut ImageNode, &mut Node),
            (
                With<DraftCardIcon>,
                Without<DraftPlacedCardIcon>,
                Without<DraftPlacedCardInfinityIcon>,
                Without<DraftCardInfinityIcon>,
                Without<DraftCardFrame>,
                Without<DraftGridCell>,
                Without<DraftGridColumn>,
                Without<DraftCardGhost>,
            ),
        >,
        infinity_query: &mut Query<
            &mut Node,
            (
                With<DraftCardInfinityIcon>,
                Without<DraftPlacedCardIcon>,
                Without<DraftPlacedCardInfinityIcon>,
                Without<DraftCardIcon>,
                Without<DraftCardFrame>,
                Without<DraftGridCell>,
                Without<DraftGridColumn>,
                Without<DraftCardGhost>,
            ),
        >,
    ) {
        let mut image_child = None;
        let mut infinity_child = None;

        for child in children.iter() {
            if image_query.get_mut(child).is_ok() {
                image_child = Some(child);
            }
            if infinity_query.get_mut(child).is_ok() {
                infinity_child = Some(child);
            }
        }

        let (Some(image_child), Some(infinity_child)) = (image_child, infinity_child) else {
            return;
        };
        let Ok((mut image, mut image_node)) = image_query.get_mut(image_child) else {
            return;
        };
        let Ok(mut infinity_node) = infinity_query.get_mut(infinity_child) else {
            return;
        };

        apply_draft_card_icon(
            &mut image,
            &mut image_node,
            &mut infinity_node,
            card,
            assets_ref,
            texture_atlas_layouts,
        );
    }

    let assets_ref = assets.as_deref();
    let draft_layout_scale = windows
        .single()
        .ok()
        .map(draft_layout_scale_for_window)
        .unwrap_or(1.0);

    for (card_type, mut bg_color, mut border_color) in &mut tray_cards {
        let is_active = draft_drag_state.active_card == Some(card_type.0);
        *bg_color = if is_active {
            BackgroundColor(Color::srgba(0.86, 0.74, 0.36, 0.18))
        } else {
            BackgroundColor(Color::NONE)
        };
        *border_color = if is_active {
            BorderColor::all(Color::srgb(0.93, 0.82, 0.46))
        } else {
            BorderColor::all(Color::srgba(0.95, 0.84, 0.58, 0.0))
        };
    }

    for (column, mut node) in &mut columns {
        node.display = if draft_board.is_column_unlocked(column.index) {
            Display::Flex
        } else {
            Display::None
        };
    }

    for (cell, mut node, children, mut bg_color, mut border_color) in &mut cells {
        let occupied =
            draft_board.card_at_with_omission(cell.column, cell.row, draft_drag_state.source_cell);
        let is_visible = draft_board.is_row_unlocked_with_omission(
            cell.column,
            cell.row,
            draft_drag_state.source_cell,
        );
        let cell_index = cell.column * DRAFT_MAX_ROWS + cell.row;
        let is_hovered = draft_drag_state.hovered_cell == Some(cell_index);

        node.display = if is_visible {
            Display::Flex
        } else {
            Display::None
        };

        *bg_color = match occupied {
            Some(_) => BackgroundColor(Color::srgb(0.09, 0.1, 0.12)),
            None => BackgroundColor(Color::srgb(0.11, 0.12, 0.14)),
        };

        *border_color = if is_hovered {
            BorderColor::all(Color::srgb(0.94, 0.84, 0.54))
        } else {
            BorderColor::all(Color::srgb(0.22, 0.25, 0.29))
        };

        for child in children {
            if let Ok((placed_card_children, mut visibility)) = placed_cards.get_mut(*child) {
                if let Some(card) = occupied {
                    update_card_icon_children(
                        placed_card_children,
                        card,
                        assets_ref,
                        texture_atlas_layouts.as_deref_mut(),
                        &mut placed_card_icons,
                        &mut placed_card_infinity_icons,
                    );
                    for placed_card_child in placed_card_children {
                        if let Ok(mut image) = draft_card_frames.get_mut(*placed_card_child) {
                            if let Some(texture_atlas) = image.texture_atlas.as_mut() {
                                texture_atlas.index = card.frame_index();
                            }
                        }
                    }
                    *visibility = Visibility::Inherited;
                } else {
                    *visibility = Visibility::Hidden;
                }
            }
        }
    }

    if let Ok((children, mut ghost_node, mut ghost_transform)) = ghost_query.single_mut() {
        ghost_transform.scale = Vec2::splat(draft_layout_scale);
        if let (Some(card), Some(cursor)) =
            (draft_drag_state.active_card, draft_drag_state.cursor_pos)
        {
            let scale_factor = windows
                .single()
                .map(|window| window.scale_factor())
                .unwrap_or(1.0);
            let logical_cursor = cursor / scale_factor;
            ghost_node.display = Display::Flex;
            ghost_node.left = Val::Px(logical_cursor.x - (DRAFT_GRID_CARD_WIDTH * 0.5));
            ghost_node.top = Val::Px(logical_cursor.y - (DRAFT_GRID_CARD_HEIGHT * 0.5));

            for child in children {
                if let Ok(mut image) = draft_card_frames.get_mut(*child) {
                    if let Some(texture_atlas) = image.texture_atlas.as_mut() {
                        texture_atlas.index = card.frame_index();
                    }
                }
            }
            update_ghost_icon_children(
                children,
                card,
                assets_ref,
                texture_atlas_layouts.as_deref_mut(),
                &mut ghost_icons,
                &mut ghost_infinity_icons,
            );
        } else {
            ghost_node.display = Display::None;
            ghost_node.left = Val::Px(-500.0);
            ghost_node.top = Val::Px(-500.0);
        }
    }
}

fn update_draft_confirm_button_visuals(
    draft_board: Res<DraftBoard>,
    mut confirm_buttons: Query<
        (
            Entity,
            &Interaction,
            &Children,
            &mut DraftConfirmButtonPressState,
            Option<&DisabledMenuButton>,
        ),
        With<DraftConfirmButton>,
    >,
    mut confirm_button_images: Query<&mut ImageNode, With<MainMenuButtonImage>>,
    mut confirm_text: Query<&mut TextColor, With<DraftConfirmButtonLabel>>,
    mut commands: Commands,
) {
    let confirm_enabled = draft_confirm_enabled(&draft_board);

    if let Ok((entity, interaction, children, mut press_state, disabled_marker)) =
        confirm_buttons.single_mut()
    {
        if confirm_enabled {
            if disabled_marker.is_some() {
                commands.entity(entity).remove::<DisabledMenuButton>();
            }
        } else {
            if disabled_marker.is_none() {
                commands.entity(entity).insert(DisabledMenuButton);
            }
            press_state.armed = false;
        }

        let button_rect = if confirm_enabled {
            Some(match *interaction {
                Interaction::Pressed => main_menu_button_clicked_rect(),
                Interaction::Hovered => main_menu_button_hovered_rect(),
                Interaction::None => main_menu_button_standard_rect(),
            })
        } else {
            Some(main_menu_button_standard_rect())
        };

        for child in children.iter() {
            if let Ok(mut image) = confirm_button_images.get_mut(child) {
                image.rect = button_rect;
            }
            if let Ok(mut text_color) = confirm_text.get_mut(child) {
                text_color.0 = if confirm_enabled {
                    Color::srgb(0.19, 0.12, 0.06)
                } else {
                    Color::srgb(0.42, 0.39, 0.35)
                };
            }
        }
    }
}

fn setup_world(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    save_state: Res<SaveGameState>,
    mut world_manager: ResMut<WorldManager>,
    mut ui_state: ResMut<GameUIState>,
    mut bloom: ResMut<Bloom>,
) {
    ui_state.close_escape_menu();

    if let (Some(slot_id), Some(world_id)) = (
        save_state.active_slot_id.as_deref(),
        save_state.active_world_id.as_deref(),
    ) {
        world_manager.save_directory =
            savegame::world_save_path(&save_state.root_dir, slot_id, world_id);
    }

    let (saved_camera, saved_bloom) =
        match savegame::load_world_metadata_for_world_dir(&world_manager.save_directory) {
            Ok(Some(metadata)) => {
                world_manager.generation = metadata.generation.unwrap_or_default();
                (metadata.camera, metadata.bloom)
            }
            Ok(None) => {
                world_manager.generation = WorldGenerationConfig::default();
                (None, None)
            }
            Err(err) => {
                warn!("Failed to load world metadata: {err}");
                world_manager.generation = WorldGenerationConfig::default();
                (None, None)
            }
        };
    let camera_translation = saved_camera
        .map(|camera| Vec3::new(camera.x, camera.y, 999.0))
        .unwrap_or(Vec3::new(0.0, 0.0, 999.0));
    let camera_zoom = saved_camera
        .map(|camera| camera.zoom.clamp(ZOOM_MIN, ZOOM_MAX))
        .unwrap_or(1.0);
    bloom.current = saved_bloom.unwrap_or(bloom.max).min(bloom.max);

    commands.spawn((
        InGameCamera,
        Camera2d,
        Transform::from_translation(camera_translation),
        Projection::Orthographic(OrthographicProjection {
            scale: camera_zoom,
            ..OrthographicProjection::default_2d()
        }),
    ));

    let mut world_halo_timer = Timer::from_seconds(BLOOM_HALO_DURATION, TimerMode::Once);
    world_halo_timer.pause();

    commands.spawn((
        WorldClickHalo,
        WorldClickHaloPulse {
            timer: world_halo_timer,
            base_diameter: 0.0,
        },
        Mesh2d(meshes.add(Circle::new(0.5).to_ring(0.18))),
        MeshMaterial2d(color_materials.add(Color::srgba(1.0, 0.92, 0.45, 0.0))),
        Transform::from_xyz(0.0, 0.0, -1000.0),
        Visibility::Hidden,
    ));

    match entities_save::load_world_entities(&world_manager.save_directory) {
        Ok(Some(saved_world)) => {
            let entity_count = saved_world.entities.len();
            spawn_saved_world_entities(
                &mut commands,
                &assets,
                &mut texture_atlas_layouts,
                saved_world,
            );
            info!("World setup complete with {entity_count} restored entities");
        }
        Ok(None) => {
            spawn_forest_guardian(
                &mut commands,
                Position::new(-100.0, 0.0),
                "oak",
                &assets,
                &mut texture_atlas_layouts,
            );
            spawn_snail(
                &mut commands,
                Position::new(100.0, 0.0),
                &assets,
                &mut texture_atlas_layouts,
            );
            spawn_tree_spirit(
                &mut commands,
                Position::new(0.0, 100.0),
                TreeVariant::Oak,
                3.0,
                &assets,
                &mut texture_atlas_layouts,
            );
            info!("World setup complete with default seeded entities");
        }
        Err(err) => {
            error!("Failed to load saved world entities: {err}");
            spawn_forest_guardian(
                &mut commands,
                Position::new(-100.0, 0.0),
                "oak",
                &assets,
                &mut texture_atlas_layouts,
            );
            spawn_snail(
                &mut commands,
                Position::new(100.0, 0.0),
                &assets,
                &mut texture_atlas_layouts,
            );
            spawn_tree_spirit(
                &mut commands,
                Position::new(0.0, 100.0),
                TreeVariant::Oak,
                3.0,
                &assets,
                &mut texture_atlas_layouts,
            );
        }
    }
}

/// Orthographic zoom avoids camera transform scaling issues with tilemap rendering.
fn zoom_camera(
    mut scroll_events: MessageReader<MouseWheel>,
    keyboard: Res<ButtonInput<KeyCode>>,
    ui_state: Res<GameUIState>,
    mut camera_query: Query<&mut Projection, With<Camera2d>>,
) {
    if ui_state.is_map_visible() {
        return;
    }

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
    time: Res<Time<Real>>,
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
    mut ui_state: ResMut<GameUIState>,
    mut virtual_time: ResMut<Time<Virtual>>,
) {
    let stylized_ui_texture = load_stylized_ui_texture(&assets);
    let icons_ui_texture = load_icons_ui_texture(&assets);
    apply_simulation_control(
        &mut virtual_time,
        &mut ui_state,
        SimulationControl::Play,
    );

    // Root UI container on the left side
    commands
        .spawn((
            InGameUiRoot,
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
                    EntityType::Human,
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
                            let layout = texture_atlas_layouts.add(TextureAtlasLayout::from_grid(
                                UVec2::splat(32),
                                8,
                                4,
                                None,
                                None,
                            ));
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

    commands
        .spawn((
            InGameUiRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(32.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            GlobalZIndex(10),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Button,
                    BloomPoolButton,
                    Node {
                        width: Val::Px(BLOOM_POOL_SIZE),
                        height: Val::Px(BLOOM_POOL_SIZE),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    BorderColor::all(Color::NONE),
                ))
                .observe(handle_bloom_pool_click)
                .with_children(|pool| {
                    pool.spawn((
                        BloomPoolSelectionHalo,
                        Node {
                            width: Val::Px(BLOOM_POOL_SIZE + 10.0),
                            height: Val::Px(BLOOM_POOL_SIZE + 10.0),
                            position_type: PositionType::Absolute,
                            border_radius: BorderRadius::all(Val::Px(999.0)),
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                    ));

                    let mut halo_timer = Timer::from_seconds(BLOOM_HALO_DURATION, TimerMode::Once);
                    halo_timer.pause();

                    pool.spawn((
                        BloomPoolHalo,
                        BloomHaloPulse { timer: halo_timer },
                        Node {
                            width: Val::Px(BLOOM_POOL_SIZE),
                            height: Val::Px(BLOOM_POOL_SIZE),
                            position_type: PositionType::Absolute,
                            border_radius: BorderRadius::all(Val::Px(999.0)),
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                    ));

                    pool.spawn((
                        ImageNode {
                            image: stylized_ui_texture.clone(),
                            rect: Some(circular_pool_background_rect()),
                            image_mode: NodeImageMode::Stretch,
                            ..default()
                        },
                        Node {
                            width: Val::Px(BLOOM_POOL_SIZE),
                            height: Val::Px(BLOOM_POOL_SIZE),
                            position_type: PositionType::Absolute,
                            ..default()
                        },
                    ));

                    pool.spawn((
                        BloomPoolFill,
                        ImageNode {
                            image: stylized_ui_texture,
                            rect: Some(circular_pool_fill_rects()[0]),
                            image_mode: NodeImageMode::Stretch,
                            ..default()
                        },
                        Node {
                            width: Val::Px(BLOOM_POOL_SIZE),
                            height: Val::Px(BLOOM_POOL_SIZE),
                            position_type: PositionType::Absolute,
                            ..default()
                        },
                    ));
                });
        });

    commands
        .spawn((
            InGameUiRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(TIME_CONTROL_BUTTON_BOTTOM),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                column_gap: Val::Px(TIME_CONTROL_BUTTON_GAP),
                ..default()
            },
            GlobalZIndex(10),
        ))
        .with_children(|parent| {
            for (mode, icon_rect) in [
                (SimulationControl::Pause, time_control_pause_rect()),
                (SimulationControl::Play, time_control_play_rect()),
                (
                    SimulationControl::FastForward,
                    time_control_fast_forward_rect(),
                ),
            ] {
                parent
                    .spawn((
                        Button,
                        TimeControlButton { mode },
                        Node {
                            width: Val::Px(TIME_CONTROL_BUTTON_SIZE),
                            height: Val::Px(TIME_CONTROL_BUTTON_SIZE),
                            padding: UiRect::all(Val::Px(2.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border: UiRect::all(Val::Px(1.0)),
                            border_radius: BorderRadius::all(Val::Px(6.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.08, 0.1, 0.12, 0.9)),
                        BorderColor::all(Color::srgba(0.72, 0.8, 0.76, 0.22)),
                    ))
                    .observe(handle_time_control_button_click)
                    .with_children(|button| {
                        button.spawn((
                            ImageNode {
                                image: icons_ui_texture.clone(),
                                rect: Some(icon_rect),
                                image_mode: NodeImageMode::Stretch,
                                ..default()
                            },
                            Node {
                                width: Val::Px(TIME_CONTROL_BUTTON_SIZE - 8.0),
                                height: Val::Px(TIME_CONTROL_BUTTON_SIZE - 8.0),
                                ..default()
                            },
                        ));
                    });
            }
        });

    let ui_texture = load_main_menu_ui_texture(&assets);
    let button_sprites = MainMenuButtonSprites {
        standard: main_menu_button_standard_rect(),
        hovered: main_menu_button_hovered_rect(),
        pressed: main_menu_button_clicked_rect(),
    };
    let button_color = Color::srgb(0.19, 0.12, 0.06);

    commands
        .spawn((
            InGameUiRoot,
            EscapeMenuRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(32.0)),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.01, 0.01, 0.02, 0.72)),
            GlobalZIndex(100),
        ))
        .with_children(|parent| {
            spawn_menu_panel(
                parent,
                &ui_texture,
                "Paused",
                520.0,
                |panel, _, ui_texture| {
                    spawn_menu_button(
                        panel,
                        ui_texture,
                        button_sprites,
                        "Exit World",
                        button_color,
                        ExitWorldButton,
                    );
                    spawn_menu_button(
                        panel,
                        ui_texture,
                        button_sprites,
                        "Exit To Main Menu",
                        button_color,
                        ExitToMainMenuButton,
                    );
                    spawn_menu_button(
                        panel,
                        ui_texture,
                        button_sprites,
                        "Quit Game",
                        button_color,
                        QuitGameButton,
                    );
                },
            );
        });
}

fn handle_keyboard_shortcuts(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut ui_state: ResMut<GameUIState>,
    mut virtual_time: ResMut<Time<Virtual>>,
    mut map_modal_visibility: Single<&mut Visibility, With<MapModal>>,
) {
    // Escape - Toggle escape menu (but close map first if open)
    if keyboard.just_pressed(KeyCode::Escape) {
        if ui_state.is_map_visible() {
            ui_state.close_all_modals();
            **map_modal_visibility = Visibility::Hidden;
            return;
        }

        if ui_state.is_escape_menu_open() {
            ui_state.close_escape_menu();
        } else {
            ui_state.open_escape_menu();
        }
        return;
    }

    // S - Request manual save
    if keyboard.just_pressed(KeyCode::KeyS) {
        ui_state.request_save();
        return;
    }

    // B - Toggle sprite bounds debug
    if keyboard.just_pressed(KeyCode::KeyB) {
        ui_state.debug_bounds_enabled = !ui_state.debug_bounds_enabled;
        info!(
            "Sprite bounds debug {}",
            if ui_state.debug_bounds_enabled {
                "enabled"
            } else {
                "disabled"
            }
        );
        return;
    }

    // R - Request reset world map
    if keyboard.just_pressed(KeyCode::KeyR) {
        ui_state.request_reset();
        return;
    }

    // M - Toggle minimap
    if keyboard.just_pressed(KeyCode::KeyM) {
        ui_state.toggle_map();
        **map_modal_visibility = if ui_state.is_map_visible() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        return;
    }

    // Space - Toggle pause/play
    if keyboard.just_pressed(KeyCode::Space) {
        let new_mode = match ui_state.simulation {
            SimulationControl::Pause => SimulationControl::Play,
            SimulationControl::Play | SimulationControl::FastForward => SimulationControl::Pause,
        };
        apply_simulation_control(&mut virtual_time, &mut ui_state, new_mode);
        return;
    }
}

fn handle_manual_save_requested(
    mut ui_state: ResMut<GameUIState>,
    mut save_notification: ResMut<SaveNotification>,
    mut world: ResMut<WorldManager>,
    camera_query: Query<(&Transform, &Projection), With<InGameCamera>>,
    bloom: Res<Bloom>,
    human_query: Query<
        (
            &Position,
            &Velocity,
            &Direction,
            &entities::EntityState,
            &Health,
        ),
        With<Human>,
    >,
    guardian_query: Query<(
        &Position,
        &Velocity,
        &Direction,
        &entities::EntityState,
        &Health,
        &ForestGuardian,
        &RoamingBehavior,
        &TreeSpawner,
    )>,
    snail_query: Query<
        (
            &Position,
            &Velocity,
            &Direction,
            &entities::EntityState,
            &Health,
            &WindingPath,
        ),
        With<Snail>,
    >,
    tree_spirit_query: Query<(&Position, &GrowingTree), With<TreeSpirit>>,
    variant_tree_query: Query<(&Position, &GrowingTree), With<VariantTree>>,
) {
    if !ui_state.take_save_request() {
        return;
    }

    info!("Manual save requested");
    save_world_snapshot(
        &mut world,
        &camera_query,
        Some(&bloom),
        &human_query,
        &guardian_query,
        &snail_query,
        &tree_spirit_query,
        &variant_tree_query,
        Some(&mut save_notification),
    );
}

fn handle_reset_requested(
    mut ui_state: ResMut<GameUIState>,
    mut world_manager: ResMut<WorldManager>,
    mut save_state: ResMut<SaveGameState>,
    mut camera_query: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
    chunk_entities: Query<Entity, With<TilemapChunk>>,
    mut commands: Commands,
) {
    if !ui_state.take_reset_request() {
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
    world_manager.generation = WorldGenerationConfig::default();

    // 3. Delete the active world directory from disk
    let save_dir = &world_manager.save_directory;
    if save_dir.exists() {
        if let Err(e) = fs::remove_dir_all(save_dir) {
            warn!("Failed to delete save directory: {}", e);
        } else {
            info!("Deleted save directory: {:?}", save_dir);
        }
    }

    save_state.active_world_id = None;
    save_state.refresh_worlds();

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

fn sync_escape_menu_visibility(
    ui_state: Res<GameUIState>,
    mut menu_query: Query<&mut Node, With<EscapeMenuRoot>>,
) {
    if !ui_state.is_changed() {
        return;
    }

    let Ok(mut node) = menu_query.single_mut() else {
        return;
    };

    node.display = if ui_state.is_escape_menu_open() {
        Display::Flex
    } else {
        Display::None
    };
}

fn update_bloom_pool(
    bloom: Res<Bloom>,
    mut pool_query: Query<&mut ImageNode, With<BloomPoolFill>>,
) {
    if !bloom.is_changed() {
        return;
    }

    let Ok(mut image_node) = pool_query.single_mut() else {
        return;
    };

    let fill_rects = circular_pool_fill_rects();
    let bloom_percent = bloom.percent().clamp(0.0, 1.0);
    let last_index = fill_rects.len().saturating_sub(1);
    let fill_index = ((1.0 - bloom_percent) * last_index as f32).round() as usize;
    image_node.rect = Some(fill_rects[fill_index.min(last_index)]);
}

fn handle_bloom_pool_click(
    trigger: On<Pointer<Click>>,
    button_query: Query<(), With<BloomPoolButton>>,
    mut bloom_selection: ResMut<BloomSelection>,
    mut placement_mode: ResMut<PlacementMode>,
    mut paint_mode: ResMut<PaintMode>,
    mut halo_query: Query<&mut BloomHaloPulse, With<BloomPoolHalo>>,
    children_query: Query<&Children>,
) {
    if trigger.event().button != PointerButton::Primary {
        return;
    }

    if button_query.get(trigger.entity).is_err() {
        return;
    }

    let next_selected = !bloom_selection.selected;
    bloom_selection.selected = next_selected;

    if next_selected {
        placement_mode.deselect();
        paint_mode.deselect();
    }

    let Ok(children) = children_query.get(trigger.entity) else {
        return;
    };

    for child in children.iter() {
        if let Ok(mut pulse) = halo_query.get_mut(child) {
            pulse.timer.reset();
            pulse.timer.unpause();
            break;
        }
    }
}

fn apply_simulation_control(
    virtual_time: &mut Time<Virtual>,
    ui_state: &mut GameUIState,
    mode: SimulationControl,
) {
    match mode {
        SimulationControl::Pause => {
            virtual_time.set_relative_speed(TIME_CONTROL_PLAY_SPEED);
            virtual_time.pause();
        }
        SimulationControl::Play => {
            virtual_time.set_relative_speed(TIME_CONTROL_PLAY_SPEED);
            virtual_time.unpause();
        }
        SimulationControl::FastForward => {
            virtual_time.set_relative_speed(TIME_CONTROL_FAST_SPEED);
            virtual_time.unpause();
        }
    }

    ui_state.simulation = mode;
}

fn handle_time_control_button_click(
    trigger: On<Pointer<Click>>,
    button_query: Query<&TimeControlButton>,
    mut ui_state: ResMut<GameUIState>,
    mut virtual_time: ResMut<Time<Virtual>>,
) {
    if trigger.event().button != PointerButton::Primary {
        return;
    }

    let Ok(button) = button_query.get(trigger.entity) else {
        return;
    };

    apply_simulation_control(&mut virtual_time, &mut ui_state, button.mode);
}

fn update_time_control_button_visuals(
    ui_state: Res<GameUIState>,
    mut buttons: Query<
        (&TimeControlButton, &mut BackgroundColor, &mut BorderColor),
        With<TimeControlButton>,
    >,
) {
    if !ui_state.is_changed() {
        return;
    }

    for (button, mut background, mut border) in &mut buttons {
        if button.mode == ui_state.simulation {
            *background = BackgroundColor(Color::srgba(0.22, 0.17, 0.06, 0.94));
            *border = BorderColor::all(Color::srgb(0.94, 0.82, 0.36));
        } else {
            *background = BackgroundColor(Color::srgba(0.08, 0.1, 0.12, 0.9));
            *border = BorderColor::all(Color::srgba(0.72, 0.8, 0.76, 0.22));
        }
    }
}

fn update_bloom_selection_halo(
    bloom_selection: Res<BloomSelection>,
    mut halo_query: Query<&mut BackgroundColor, With<BloomPoolSelectionHalo>>,
) {
    if !bloom_selection.is_changed() {
        return;
    }

    let Ok(mut background_color) = halo_query.single_mut() else {
        return;
    };

    *background_color = if bloom_selection.selected {
        BackgroundColor(Color::srgba(1.0, 0.92, 0.45, 0.16))
    } else {
        BackgroundColor(Color::NONE)
    };
}

fn pulse_bloom_halo_on_world_click(
    bloom_selection: Res<BloomSelection>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut halo_query: Query<&mut BloomHaloPulse, With<BloomPoolHalo>>,
    ui_nodes: Query<
        (
            &ComputedNode,
            &UiGlobalTransform,
            &Node,
            Option<&Visibility>,
            Option<&InheritedVisibility>,
        ),
        With<Node>,
    >,
) {
    if !bloom_selection.selected || !mouse_button.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };

    let Some(ui_cursor) = window.physical_cursor_position() else {
        return;
    };

    if cursor_over_blocking_ui(ui_cursor, &ui_nodes) {
        return;
    }

    let Ok(mut pulse) = halo_query.single_mut() else {
        return;
    };

    pulse.timer.reset();
    pulse.timer.unpause();
}

fn handle_bloom_world_entity_click(
    bloom_selection: Res<BloomSelection>,
    mut pointer_clicks: MessageReader<Pointer<Click>>,
    images: Res<Assets<Image>>,
    texture_atlas_layouts: Res<Assets<TextureAtlasLayout>>,
    world_entities: Query<
        (
            &Sprite,
            Option<&Anchor>,
            &GlobalTransform,
            &Position,
            &WorldRenderDepth,
            &WorldClickableEntity,
        ),
        Without<WorldClickHalo>,
    >,
    mut world_halo_query: Query<
        (&mut WorldClickHaloPulse, &mut Transform, &mut Visibility),
        With<WorldClickHalo>,
    >,
) {
    if !bloom_selection.selected {
        return;
    }

    for click in pointer_clicks.read() {
        if click.button != PointerButton::Primary {
            continue;
        }

        let Ok((sprite, anchor, transform, position, render_depth, _)) =
            world_entities.get(click.entity)
        else {
            continue;
        };

        let Some((world_center, size)) = sprite_world_bounds(
            sprite,
            anchor.copied(),
            &transform.compute_transform(),
            &images,
            &texture_atlas_layouts,
        ) else {
            continue;
        };

        let render_z = render_depth.z_for_position(position);
        let halo_diameter = size.max_element() + WORLD_CLICK_HALO_PADDING;

        let Ok((mut world_pulse, mut world_halo_transform, mut visibility)) =
            world_halo_query.single_mut()
        else {
            return;
        };

        world_halo_transform.translation = Vec3::new(
            world_center.x,
            world_center.y,
            render_z - WORLD_CLICK_HALO_DEPTH_OFFSET,
        );
        world_halo_transform.scale = Vec3::splat(halo_diameter);
        *visibility = Visibility::Visible;
        world_pulse.base_diameter = halo_diameter;
        world_pulse.timer.reset();
        world_pulse.timer.unpause();
    }
}

fn animate_bloom_halo(
    time: Res<Time>,
    mut halo_query: Query<
        (&mut BloomHaloPulse, &mut Node, &mut BackgroundColor),
        With<BloomPoolHalo>,
    >,
) {
    for (mut pulse, mut node, mut background_color) in &mut halo_query {
        if pulse.timer.is_paused() {
            node.width = Val::Px(BLOOM_POOL_SIZE);
            node.height = Val::Px(BLOOM_POOL_SIZE);
            *background_color = BackgroundColor(Color::NONE);
            continue;
        }

        pulse.timer.tick(time.delta());

        if pulse.timer.is_finished() {
            pulse.timer.pause();
            node.width = Val::Px(BLOOM_POOL_SIZE);
            node.height = Val::Px(BLOOM_POOL_SIZE);
            *background_color = BackgroundColor(Color::NONE);
            continue;
        }

        let progress = pulse.timer.elapsed_secs() / BLOOM_HALO_DURATION;
        let size = BLOOM_POOL_SIZE + BLOOM_HALO_EXTRA_SIZE * progress;
        let alpha = (1.0 - progress) * 0.2;

        node.width = Val::Px(size);
        node.height = Val::Px(size);
        *background_color = BackgroundColor(Color::srgba(1.0, 0.92, 0.45, alpha));
    }
}

fn animate_world_click_halo(
    time: Res<Time>,
    mut color_materials: ResMut<Assets<ColorMaterial>>,
    mut halo_query: Query<
        (
            &mut WorldClickHaloPulse,
            &mut Transform,
            &MeshMaterial2d<ColorMaterial>,
            &mut Visibility,
        ),
        With<WorldClickHalo>,
    >,
) {
    for (mut pulse, mut transform, material, mut visibility) in &mut halo_query {
        let Some(color_material) = color_materials.get_mut(material) else {
            continue;
        };

        if pulse.timer.is_paused() {
            *visibility = Visibility::Hidden;
            color_material.color = Color::srgba(1.0, 0.92, 0.45, 0.0);
            continue;
        }

        pulse.timer.tick(time.delta());

        if pulse.timer.is_finished() {
            pulse.timer.pause();
            *visibility = Visibility::Hidden;
            color_material.color = Color::srgba(1.0, 0.92, 0.45, 0.0);
            continue;
        }

        let progress = pulse.timer.elapsed_secs() / BLOOM_HALO_DURATION;
        let size = pulse.base_diameter + WORLD_CLICK_HALO_EXTRA_SIZE * progress;
        let alpha = (1.0 - progress) * 0.2;

        *visibility = Visibility::Visible;
        transform.scale = Vec3::splat(size);
        color_material.color = Color::srgba(1.0, 0.92, 0.45, alpha);
    }
}

fn button_interaction(
    trigger: On<Pointer<Click>>,
    mut param_set: ParamSet<(
        Query<(&EntityType, Option<&GuardianButton>), With<Button>>,
        Query<(&mut EntityType, &Children), With<GuardianButton>>,
    )>,
    mut placement_mode: ResMut<PlacementMode>,
    mut paint_mode: ResMut<PaintMode>,
    mut bloom_selection: ResMut<BloomSelection>,
    mut submenu_query: Query<&mut Node, With<GuardianSubmenu>>,
    mut image_query: Query<&mut ImageNode>,
    assets: Res<AssetServer>,
) {
    if trigger.event().button != PointerButton::Primary {
        return;
    }

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
        bloom_selection.selected = false;

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
    mut bloom_selection: ResMut<BloomSelection>,
    mut submenu_query: Query<&mut Node, With<TerrainSubmenu>>,
    mut image_query: Query<&mut ImageNode>,
) {
    if trigger.event().button != PointerButton::Primary {
        return;
    }

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
        bloom_selection.selected = false;

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
            EntityType::Human => {
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
    ui_nodes: Query<
        (
            &ComputedNode,
            &UiGlobalTransform,
            &Node,
            Option<&Visibility>,
            Option<&InheritedVisibility>,
        ),
        With<Node>,
    >,
    mut bloom: ResMut<Bloom>,
    mut world_manager: ResMut<WorldManager>,
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

    let bloom_cost = entity_type.bloom_cost();
    if !bloom.can_spend(bloom_cost) {
        info!("Not enough bloom to create {:?}", entity_type);
        return;
    }

    // Get the primary window
    let Ok(window) = windows.single() else {
        return;
    };

    // Get cursor position in window
    let Some(ui_cursor_pos) = window.physical_cursor_position() else {
        return;
    };

    if cursor_over_blocking_ui(ui_cursor_pos, &ui_nodes) {
        return;
    }

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

    if !world_manager.has_land_at_world(world_pos) {
        info!(
            "Blocked entity placement on void at ({}, {})",
            world_pos.x, world_pos.y
        );
        return;
    }

    // Spawn the entity at the world position
    let position = Position::new(world_pos.x, world_pos.y);

    match entity_type {
        EntityType::Human => {
            bloom.spend(bloom_cost);
            spawn_human(&mut commands, position, &assets, &mut texture_atlas_layouts);
            info!("Spawned human at ({}, {})", world_pos.x, world_pos.y);
        }
        EntityType::ForestGuardian(variant) => {
            bloom.spend(bloom_cost);
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
            bloom.spend(bloom_cost);
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
    ui_nodes: Query<
        (
            &ComputedNode,
            &UiGlobalTransform,
            &Node,
            Option<&Visibility>,
            Option<&InheritedVisibility>,
        ),
        With<Node>,
    >,
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

    // Get the primary window
    let Ok(window) = windows.single() else {
        paint_drag_state.reset();
        return;
    };

    // Get cursor position in window
    let Some(ui_cursor_pos) = window.physical_cursor_position() else {
        paint_drag_state.reset();
        return;
    };

    if cursor_over_blocking_ui(ui_cursor_pos, &ui_nodes) {
        paint_drag_state.reset();
        return;
    }

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

    if !world_manager.has_land_at_world(world_pos) {
        paint_drag_state.reset();
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
    use crate::entities::RenderStratum;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("worldseed-{prefix}-{unique}"));
        let _ = fs::remove_dir_all(&root);
        root
    }

    #[test]
    fn world_to_tile_pos_handles_positive_and_negative_coordinates() {
        assert_eq!(world_to_tile_pos(Vec2::new(0.0, 0.0)), IVec2::new(0, 0));
        assert_eq!(world_to_tile_pos(Vec2::new(31.9, 31.9)), IVec2::new(0, 0));
        assert_eq!(world_to_tile_pos(Vec2::new(32.0, 32.0)), IVec2::new(1, 1));
        assert_eq!(world_to_tile_pos(Vec2::new(-0.1, -0.1)), IVec2::new(-1, -1));
        assert_eq!(
            world_to_tile_pos(Vec2::new(-32.0, -32.0)),
            IVec2::new(-1, -1)
        );
    }

    #[test]
    fn app_starts_in_main_menu_without_gameplay_entities() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::state::app::StatesPlugin)
            .init_state::<AppState>()
            .init_resource::<SaveGameState>()
            .add_systems(OnEnter(AppState::MainMenu), setup_main_menu);

        app.update();

        assert_eq!(
            *app.world().resource::<State<AppState>>().get(),
            AppState::MainMenu
        );
        let menu_count = {
            let world = app.world_mut();
            world.query::<&MainMenuRoot>().iter(world).count()
        };
        let menu_camera_count = {
            let world = app.world_mut();
            world.query::<&MainMenuCamera>().iter(world).count()
        };
        assert_eq!(menu_count, 1);
        assert_eq!(menu_camera_count, 1);
    }

    #[test]
    fn pressing_new_game_transitions_to_draft_setup_and_removes_menu() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::state::app::StatesPlugin)
            .init_state::<AppState>()
            .init_resource::<DraftBoard>()
            .init_resource::<DraftDragState>()
            .init_resource::<SaveGameState>()
            .add_systems(OnEnter(AppState::MainMenu), setup_main_menu)
            .add_systems(OnExit(AppState::MainMenu), cleanup_main_menu)
            .add_systems(OnEnter(AppState::DraftSetup), setup_draft_setup)
            .add_systems(OnExit(AppState::DraftSetup), cleanup_draft_setup)
            .add_systems(OnEnter(AppState::InGame), |mut commands: Commands| {
                commands.spawn((Camera2d, Transform::default()));
            })
            .add_systems(Update, handle_new_game_button_interaction);

        let temp_root = temp_root("test-new-game");
        app.world_mut().resource_mut::<SaveGameState>().root_dir = temp_root.clone();

        app.update();

        let new_game_button = {
            let world = app.world_mut();
            world
                .query_filtered::<Entity, With<NewGameButton>>()
                .single(world)
                .expect("expected new game button to exist")
        };
        app.world_mut()
            .entity_mut(new_game_button)
            .insert(Interaction::Pressed);

        app.update();

        app.world_mut()
            .entity_mut(new_game_button)
            .insert(Interaction::Hovered);

        app.update();
        app.update();

        assert_eq!(
            *app.world().resource::<State<AppState>>().get(),
            AppState::DraftSetup
        );
        let menu_count = {
            let world = app.world_mut();
            world.query::<&MainMenuRoot>().iter(world).count()
        };
        let draft_root_count = {
            let world = app.world_mut();
            world.query::<&DraftSetupRoot>().iter(world).count()
        };
        assert_eq!(menu_count, 0);
        assert_eq!(draft_root_count, 1);

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn confirm_from_draft_setup_transitions_to_gameplay() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::state::app::StatesPlugin)
            .init_state::<AppState>()
            .init_resource::<DraftBoard>()
            .init_resource::<DraftDragState>()
            .init_resource::<SaveGameState>()
            .init_resource::<WorldManager>()
            .add_systems(OnEnter(AppState::DraftSetup), setup_draft_setup)
            .add_systems(OnExit(AppState::DraftSetup), cleanup_draft_setup)
            .add_systems(OnEnter(AppState::InGame), |mut commands: Commands| {
                commands.spawn((Camera2d, Transform::default()));
            })
            .add_systems(Update, handle_draft_confirm_button_interaction);
        let temp_root = std::env::temp_dir().join("worldseed-test-confirm-draft");
        let _ = fs::remove_dir_all(&temp_root);
        let slot = savegame::create_slot(&temp_root).expect("slot should exist for draft confirm");
        {
            let mut save_state = app.world_mut().resource_mut::<SaveGameState>();
            save_state.root_dir = temp_root.clone();
            save_state.draft_target_slot_id = Some(slot.id);
        }

        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::DraftSetup);
        app.update();
        app.update();
        {
            let mut draft_board = app.world_mut().resource_mut::<DraftBoard>();
            assert!(draft_board.place_card(0, 0, DraftCard::Island));
            assert!(draft_board.place_card(0, 1, DraftCard::Grass));
        }

        let confirm_button = {
            let world = app.world_mut();
            world
                .query_filtered::<Entity, With<DraftConfirmButton>>()
                .single(world)
                .expect("expected confirm button to exist")
        };

        app.world_mut()
            .entity_mut(confirm_button)
            .insert(Interaction::Pressed);
        app.update();

        app.world_mut()
            .entity_mut(confirm_button)
            .insert(Interaction::Hovered);
        app.update();
        app.update();

        assert_eq!(
            *app.world().resource::<State<AppState>>().get(),
            AppState::InGame
        );

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn draft_setup_starts_with_one_visible_column() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::state::app::StatesPlugin)
            .init_state::<AppState>()
            .init_resource::<DraftBoard>()
            .init_resource::<DraftDragState>()
            .init_resource::<SaveGameState>()
            .add_systems(OnEnter(AppState::DraftSetup), setup_draft_setup)
            .add_systems(
                Update,
                (update_draft_visuals, update_draft_confirm_button_visuals)
                    .run_if(in_state(AppState::DraftSetup)),
            );

        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::DraftSetup);
        app.update();
        app.update();

        let grid_cell_count = {
            let world = app.world_mut();
            world.query::<&DraftGridCell>().iter(world).count()
        };

        assert_eq!(grid_cell_count, DRAFT_MAX_COLUMNS * DRAFT_MAX_ROWS);

        let visible_column_count = {
            let world = app.world_mut();
            world
                .query::<(&DraftGridColumn, &Node)>()
                .iter(world)
                .filter(|(_, node)| node.display != Display::None)
                .count()
        };

        assert_eq!(visible_column_count, 1);
    }

    #[test]
    fn draft_tray_only_shows_shape_and_element_cards() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::state::app::StatesPlugin)
            .init_state::<AppState>()
            .init_resource::<DraftBoard>()
            .init_resource::<DraftDragState>()
            .init_resource::<SaveGameState>()
            .add_systems(OnEnter(AppState::DraftSetup), setup_draft_setup);

        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::DraftSetup);
        app.update();
        app.update();

        let tray_cards = {
            let world = app.world_mut();
            world
                .query::<&DraftCardTypeComponent>()
                .iter(world)
                .map(|card| card.0)
                .collect::<Vec<_>>()
        };

        assert_eq!(tray_cards, VISIBLE_DRAFT_CARDS);
    }

    #[test]
    fn island_card_unlocks_next_row_only_in_same_column() {
        let mut board = DraftBoard::default();

        assert!(board.is_row_unlocked(0, 0));
        assert!(!board.is_row_unlocked(0, 1));

        assert!(board.place_card(0, 0, DraftCard::Island));
        assert!(board.is_row_unlocked(0, 1));
    }

    #[test]
    fn replacing_island_card_clears_cards_in_rows_it_no_longer_unlocks() {
        let mut board = DraftBoard::default();

        assert!(board.place_card(0, 0, DraftCard::Island));
        assert!(board.place_card(0, 1, DraftCard::Grass));
        assert_eq!(board.card_at(0, 1), Some(DraftCard::Grass));

        assert!(board.place_card(0, 0, DraftCard::Infinity));
        assert_eq!(board.card_at(0, 1), Some(DraftCard::Grass));

        assert!(board.remove_card(0, 0).is_some());
        assert!(!board.is_row_unlocked(0, 1));
        assert_eq!(board.card_at(0, 1), None);

        assert!(board.place_card(0, 0, DraftCard::Island));
        assert!(board.is_row_unlocked(0, 1));
        assert_eq!(board.card_at(0, 1), None);
    }

    #[test]
    fn removing_island_card_clears_cards_in_rows_it_no_longer_unlocks() {
        let mut board = DraftBoard::default();

        assert!(board.place_card(0, 0, DraftCard::Island));
        assert!(board.place_card(0, 1, DraftCard::Grass));

        assert_eq!(board.remove_card(0, 0), Some(DraftCard::Island));
        assert_eq!(board.card_at(0, 0), None);
        assert!(!board.is_row_unlocked(0, 1));
        assert_eq!(board.card_at(0, 1), None);
    }

    #[test]
    fn dragged_source_cell_is_treated_as_empty_for_drop_validation() {
        let mut board = DraftBoard::default();
        board.reset(2);

        assert!(board.place_card(0, 0, DraftCard::Island));
        assert!(board.place_card(0, 1, DraftCard::Grass));

        let source = Some(DraftCellPosition { column: 0, row: 0 });
        assert!(!board.can_place_card_with_omission(0, 1, DraftCard::Grass, source));
        assert!(board.can_place_card_with_omission(1, 0, DraftCard::Infinity, source));
    }

    #[test]
    fn draft_generation_config_uses_first_column_shape_and_element() {
        let mut board = DraftBoard::default();

        assert!(board.place_card(0, 0, DraftCard::Infinity));
        assert!(board.place_card(0, 1, DraftCard::Dirt));

        assert_eq!(
            draft_generation_config(&board),
            Some(WorldGenerationConfig {
                shape: WorldShape::Infinity,
                element: WorldElement::Dirt,
                ..WorldGenerationConfig::default()
            })
        );
    }

    #[test]
    fn draft_generation_config_rejects_invalid_recipe_order() {
        let mut board = DraftBoard::default();

        board.cells[0][0] = Some(DraftCard::Grass);

        assert_eq!(draft_generation_config(&board), None);
    }

    #[test]
    fn draft_board_rejects_wrong_card_category_for_recipe_rows() {
        let mut board = DraftBoard::default();

        assert!(!board.place_card(0, 0, DraftCard::Grass));
        assert!(board.place_card(0, 0, DraftCard::Island));
        assert!(!board.place_card(0, 1, DraftCard::Infinity));
        assert!(board.place_card(0, 1, DraftCard::Grass));
    }

    #[test]
    fn invalid_draft_confirm_does_not_transition_to_gameplay() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::state::app::StatesPlugin)
            .init_state::<AppState>()
            .init_resource::<DraftBoard>()
            .init_resource::<DraftDragState>()
            .init_resource::<SaveGameState>()
            .init_resource::<WorldManager>()
            .add_systems(OnEnter(AppState::DraftSetup), setup_draft_setup)
            .add_systems(OnExit(AppState::DraftSetup), cleanup_draft_setup)
            .add_systems(Update, handle_draft_confirm_button_interaction);
        let temp_root = std::env::temp_dir().join("worldseed-test-invalid-confirm-draft");
        let _ = fs::remove_dir_all(&temp_root);
        let slot = savegame::create_slot(&temp_root).expect("slot should exist for draft confirm");
        {
            let mut save_state = app.world_mut().resource_mut::<SaveGameState>();
            save_state.root_dir = temp_root.clone();
            save_state.draft_target_slot_id = Some(slot.id);
        }

        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::DraftSetup);
        app.update();
        app.update();

        let confirm_button = {
            let world = app.world_mut();
            world
                .query_filtered::<Entity, With<DraftConfirmButton>>()
                .single(world)
                .expect("expected confirm button to exist")
        };

        app.world_mut()
            .entity_mut(confirm_button)
            .insert(Interaction::Pressed);
        app.update();

        app.world_mut()
            .entity_mut(confirm_button)
            .insert(Interaction::Hovered);
        app.update();
        app.update();

        assert_eq!(
            *app.world().resource::<State<AppState>>().get(),
            AppState::DraftSetup
        );

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn draft_plus_key_unlocks_columns_and_persists_slot_progress() {
        use bevy::ecs::system::SystemState;

        let mut world = World::new();
        world.insert_resource(ButtonInput::<KeyCode>::default());
        world.insert_resource(DraftBoard::default());
        world.insert_resource(SaveGameState::default());

        let temp_root = std::env::temp_dir().join("worldseed-test-draft-columns");
        let _ = fs::remove_dir_all(&temp_root);
        let slot = savegame::create_slot(&temp_root).expect("slot should exist for draft columns");

        {
            let mut save_state = world.resource_mut::<SaveGameState>();
            save_state.root_dir = temp_root.clone();
            save_state.refresh_slots();
            save_state.draft_target_slot_id = Some(slot.id.clone());
        }

        world
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Equal);

        let mut system_state: SystemState<(
            Res<ButtonInput<KeyCode>>,
            ResMut<DraftBoard>,
            ResMut<SaveGameState>,
        )> = SystemState::new(&mut world);
        let (keyboard, draft_board, save_state) = system_state.get_mut(&mut world);
        handle_draft_debug_unlock_columns(keyboard, draft_board, save_state);
        system_state.apply(&mut world);

        assert_eq!(world.resource::<DraftBoard>().available_columns, 2);

        let updated_slot = savegame::list_slots(&temp_root)
            .expect("slots should list")
            .into_iter()
            .find(|candidate| candidate.id == slot.id)
            .expect("updated slot should exist");
        assert_eq!(updated_slot.available_columns, 2);

        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn escape_key_toggles_escape_menu_state() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<GameUIState>()
            .init_resource::<MapState>()
            .add_systems(Update, handle_keyboard_shortcuts);
        app.world_mut().spawn((MapModal, Visibility::Hidden));

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();

        assert!(app.world().resource::<GameUIState>().is_escape_menu_open());
    }

    #[test]
    fn escape_key_closes_minimap_without_opening_escape_menu() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<GameUIState>()
            .init_resource::<MapState>()
            .add_systems(Update, handle_keyboard_shortcuts);

        app.world_mut().spawn((MapModal, Visibility::Visible));
        let mut ui_state = app.world_mut().resource_mut::<GameUIState>();
        ui_state.toggle_map();
        drop(ui_state);

        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();

        assert!(!app.world().resource::<GameUIState>().is_map_visible());
        assert!(!app.world().resource::<GameUIState>().is_escape_menu_open());

        let visibility = {
            let world = app.world_mut();
            world
                .query_filtered::<&Visibility, With<MapModal>>()
                .single(world)
                .expect("expected map modal to exist")
                .clone()
        };
        assert_eq!(visibility, Visibility::Hidden);
    }

    #[test]
    fn exit_world_button_returns_to_slot_hub_and_cleans_up_in_game_entities() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::state::app::StatesPlugin)
            .init_state::<AppState>()
            .init_resource::<SaveGameState>()
            .init_resource::<WorldManager>()
            .init_resource::<SaveNotification>()
            .init_resource::<PlacementMode>()
            .init_resource::<PaintMode>()
            .init_resource::<PaintDragState>()
            .init_resource::<GameUIState>()
            .add_systems(OnExit(AppState::InGame), cleanup_in_game)
            .add_systems(Update, handle_exit_world_button_interaction);

        let temp_root = temp_root("test-exit-world");
        {
            let mut save_state = app.world_mut().resource_mut::<SaveGameState>();
            save_state.root_dir = temp_root.clone();
            save_state.active_slot_id = Some("slot-1".to_string());
            save_state.active_world_id = Some("world-1".to_string());
        }
        app.world_mut()
            .resource_mut::<WorldManager>()
            .save_directory = temp_root.join("world");
        app.world_mut().resource_mut::<GameUIState>().open_escape_menu();
        app.world_mut().spawn((InGameCamera, Camera2d));
        app.world_mut().spawn(InGameUiRoot);
        app.world_mut()
            .spawn(WorldRenderDepth::new(RenderStratum::WorldObject));
        app.world_mut().spawn((
            ExitWorldButton,
            MainMenuButtonPressState::default(),
            Interaction::Pressed,
        ));

        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::InGame);
        app.update();

        let exit_world_button = {
            let world = app.world_mut();
            world
                .query_filtered::<Entity, With<ExitWorldButton>>()
                .single(world)
                .expect("expected exit world button to exist")
        };

        app.world_mut()
            .entity_mut(exit_world_button)
            .insert(Interaction::Hovered);

        app.update();
        app.update();

        assert_eq!(
            *app.world().resource::<State<AppState>>().get(),
            AppState::SlotHub
        );
        assert_eq!(
            app.world()
                .resource::<SaveGameState>()
                .active_slot_id
                .as_deref(),
            Some("slot-1")
        );
        assert_eq!(
            app.world().resource::<SaveGameState>().active_world_id,
            None
        );
        assert!(!app.world().resource::<GameUIState>().is_escape_menu_open());

        let camera_count = {
            let world = app.world_mut();
            world.query::<&InGameCamera>().iter(world).count()
        };
        let ui_count = {
            let world = app.world_mut();
            world.query::<&InGameUiRoot>().iter(world).count()
        };
        let world_entity_count = {
            let world = app.world_mut();
            world.query::<&WorldRenderDepth>().iter(world).count()
        };

        assert_eq!(camera_count, 0);
        assert_eq!(ui_count, 0);
        assert_eq!(world_entity_count, 0);

        let _ = fs::remove_dir_all(temp_root);
    }
}

/// Setup UI for save notification in top right corner
fn setup_save_notification_ui(mut commands: Commands) {
    // Create text container in top right corner
    commands
        .spawn((
            InGameUiRoot,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(20.0),
                top: Val::Px(20.0),
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
        ))
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

fn cleanup_in_game(
    mut commands: Commands,
    in_game_cameras: Query<(Entity, &Transform, &Projection), With<InGameCamera>>,
    in_game_ui_roots: Query<Entity, With<InGameUiRoot>>,
    world_entities: Query<Entity, With<WorldRenderDepth>>,
    chunk_entities: Query<Entity, With<TilemapChunk>>,
    mut world_manager: ResMut<WorldManager>,
    bloom: Option<Res<Bloom>>,
    human_query: Query<
        (
            &Position,
            &Velocity,
            &Direction,
            &entities::EntityState,
            &Health,
        ),
        With<Human>,
    >,
    guardian_query: Query<(
        &Position,
        &Velocity,
        &Direction,
        &entities::EntityState,
        &Health,
        &ForestGuardian,
        &RoamingBehavior,
        &TreeSpawner,
    )>,
    snail_query: Query<
        (
            &Position,
            &Velocity,
            &Direction,
            &entities::EntityState,
            &Health,
            &WindingPath,
        ),
        With<Snail>,
    >,
    tree_spirit_query: Query<(&Position, &GrowingTree), With<TreeSpirit>>,
    variant_tree_query: Query<(&Position, &GrowingTree), With<VariantTree>>,
    ui_state: (
        ResMut<PlacementMode>,
        ResMut<PaintMode>,
        ResMut<PaintDragState>,
        ResMut<SaveNotification>,
        ResMut<GameUIState>,
    ),
) {
    let (
        mut placement_mode,
        mut paint_mode,
        mut paint_drag_state,
        mut save_notification,
        mut game_ui_state,
    ) = ui_state;

    if let Some((_, transform, projection)) = in_game_cameras.iter().next() {
        save_world_camera(
            &world_manager.save_directory,
            savegame::CameraState {
                x: transform.translation.x,
                y: transform.translation.y,
                zoom: match projection {
                    Projection::Orthographic(ortho) => ortho.scale,
                    _ => 1.0,
                },
            },
        );
    }
    if let Some(bloom) = bloom.as_deref() {
        save_world_bloom(&world_manager.save_directory, bloom.current);
    }
    save_dirty_chunks(&mut world_manager);
    save_world_entities_only(
        &world_manager.save_directory,
        &human_query,
        &guardian_query,
        &snail_query,
        &tree_spirit_query,
        &variant_tree_query,
        None,
    );

    for (entity, _, _) in &in_game_cameras {
        commands.entity(entity).despawn();
    }

    for entity in &in_game_ui_roots {
        commands.entity(entity).despawn();
    }

    for entity in &world_entities {
        commands.entity(entity).despawn();
    }

    for entity in &chunk_entities {
        commands.entity(entity).despawn();
    }

    world_manager.active_chunks.clear();
    world_manager.dirty_chunks.clear();
    world_manager.chunk_cache.clear();
    world_manager.pending_tile_modifications.clear();
    world_manager.camera_chunk = None;
    world_manager.generation = WorldGenerationConfig::default();

    placement_mode.deselect();
    paint_mode.deselect();
    paint_drag_state.reset();
    save_notification.visible = false;
    game_ui_state.close_escape_menu();
}

fn save_dirty_chunks(world: &mut WorldManager) {
    use world::serialization;

    let dirty_chunks = world.get_dirty_chunks();
    if dirty_chunks.is_empty() {
        return;
    }

    let mut saved_count = 0;
    let mut failed_count = 0;

    for chunk_pos in dirty_chunks {
        if let Some(chunk_data) = world.get_cached_chunk(&chunk_pos) {
            let chunk_path = world.get_chunk_path(&chunk_pos);
            match serialization::save_chunk(chunk_data, &chunk_path) {
                Ok(_) => {
                    debug!("Saved chunk {:?}", chunk_pos);
                    world.clear_dirty(&chunk_pos);
                    saved_count += 1;
                }
                Err(err) => {
                    error!("Failed to save chunk {:?}: {}", chunk_pos, err);
                    failed_count += 1;
                }
            }
        }
    }

    info!(
        "Saved dirty chunks: {} saved, {} failed",
        saved_count, failed_count
    );
}

fn save_world_snapshot(
    world: &mut WorldManager,
    camera_query: &Query<(&Transform, &Projection), With<InGameCamera>>,
    bloom: Option<&Bloom>,
    human_query: &Query<
        (
            &Position,
            &Velocity,
            &Direction,
            &entities::EntityState,
            &Health,
        ),
        With<Human>,
    >,
    guardian_query: &Query<(
        &Position,
        &Velocity,
        &Direction,
        &entities::EntityState,
        &Health,
        &ForestGuardian,
        &RoamingBehavior,
        &TreeSpawner,
    )>,
    snail_query: &Query<
        (
            &Position,
            &Velocity,
            &Direction,
            &entities::EntityState,
            &Health,
            &WindingPath,
        ),
        With<Snail>,
    >,
    tree_spirit_query: &Query<(&Position, &GrowingTree), With<TreeSpirit>>,
    variant_tree_query: &Query<(&Position, &GrowingTree), With<VariantTree>>,
    save_notification: Option<&mut SaveNotification>,
) {
    if let Some(camera) = current_world_camera_state(camera_query) {
        save_world_camera(&world.save_directory, camera);
    }
    if let Some(bloom) = bloom {
        save_world_bloom(&world.save_directory, bloom.current);
    }
    save_dirty_chunks(world);
    save_world_entities_only(
        &world.save_directory,
        human_query,
        guardian_query,
        snail_query,
        tree_spirit_query,
        variant_tree_query,
        save_notification,
    );
}

fn current_world_camera_state(
    camera_query: &Query<(&Transform, &Projection), With<InGameCamera>>,
) -> Option<savegame::CameraState> {
    let Ok((transform, projection)) = camera_query.single() else {
        return None;
    };

    Some(savegame::CameraState {
        x: transform.translation.x,
        y: transform.translation.y,
        zoom: match projection {
            Projection::Orthographic(ortho) => ortho.scale,
            _ => 1.0,
        },
    })
}

fn save_world_camera(save_directory: &Path, camera: savegame::CameraState) {
    if let Err(err) = savegame::set_world_camera_for_world_dir(save_directory, camera) {
        error!("Failed to save world camera state: {err}");
    }
}

fn save_world_bloom(save_directory: &Path, bloom: u16) {
    if let Err(err) = savegame::set_world_bloom_for_world_dir(save_directory, bloom) {
        error!("Failed to save world bloom state: {err}");
    }
}

fn save_world_entities_only(
    save_directory: &PathBuf,
    human_query: &Query<
        (
            &Position,
            &Velocity,
            &Direction,
            &entities::EntityState,
            &Health,
        ),
        With<Human>,
    >,
    guardian_query: &Query<(
        &Position,
        &Velocity,
        &Direction,
        &entities::EntityState,
        &Health,
        &ForestGuardian,
        &RoamingBehavior,
        &TreeSpawner,
    )>,
    snail_query: &Query<
        (
            &Position,
            &Velocity,
            &Direction,
            &entities::EntityState,
            &Health,
            &WindingPath,
        ),
        With<Snail>,
    >,
    tree_spirit_query: &Query<(&Position, &GrowingTree), With<TreeSpirit>>,
    variant_tree_query: &Query<(&Position, &GrowingTree), With<VariantTree>>,
    save_notification: Option<&mut SaveNotification>,
) {
    let snapshot = collect_saved_world_entities(
        human_query,
        guardian_query,
        snail_query,
        tree_spirit_query,
        variant_tree_query,
    );

    match entities_save::save_world_entities(save_directory, &snapshot) {
        Ok(()) => {
            info!("Saved {} world entities", snapshot.entities.len());
            if let Some(save_notification) = save_notification {
                save_notification.show(2.0);
            }
        }
        Err(err) => {
            error!("Failed to save world entities: {err}");
        }
    }
}

fn autosave_entities(
    time: Res<Time>,
    mut timer: ResMut<EntityAutosaveTimer>,
    world: Res<WorldManager>,
    camera_query: Query<(&Transform, &Projection), With<InGameCamera>>,
    bloom: Res<Bloom>,
    human_query: Query<
        (
            &Position,
            &Velocity,
            &Direction,
            &entities::EntityState,
            &Health,
        ),
        With<Human>,
    >,
    guardian_query: Query<(
        &Position,
        &Velocity,
        &Direction,
        &entities::EntityState,
        &Health,
        &ForestGuardian,
        &RoamingBehavior,
        &TreeSpawner,
    )>,
    snail_query: Query<
        (
            &Position,
            &Velocity,
            &Direction,
            &entities::EntityState,
            &Health,
            &WindingPath,
        ),
        With<Snail>,
    >,
    tree_spirit_query: Query<(&Position, &GrowingTree), With<TreeSpirit>>,
    variant_tree_query: Query<(&Position, &GrowingTree), With<VariantTree>>,
) {
    timer.tick(time.delta());
    if !timer.just_finished() {
        return;
    }

    if let Some(camera) = current_world_camera_state(&camera_query) {
        save_world_camera(&world.save_directory, camera);
    }
    save_world_bloom(&world.save_directory, bloom.current);
    save_world_entities_only(
        &world.save_directory,
        &human_query,
        &guardian_query,
        &snail_query,
        &tree_spirit_query,
        &variant_tree_query,
        None,
    );
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

fn draw_snail_debug_bounds(
    mut gizmos: Gizmos,
    ui_state: Res<GameUIState>,
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
    if !ui_state.debug_bounds_enabled {
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

        gizmos.circle_2d(
            transform.translation.truncate(),
            4.0,
            Color::srgb(0.2, 1.0, 0.2),
        );

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

    images
        .get(&sprite.image)
        .map(|image| image.size().as_vec2())
}

fn sprite_world_bounds(
    sprite: &Sprite,
    anchor: Option<Anchor>,
    transform: &Transform,
    images: &Assets<Image>,
    texture_atlas_layouts: &Assets<TextureAtlasLayout>,
) -> Option<(Vec2, Vec2)> {
    let base_size = sprite_debug_size(sprite, images, texture_atlas_layouts)?;
    let world_size = base_size * transform.scale.truncate().abs();
    let anchor = anchor.unwrap_or_default();
    let center = transform.translation.truncate() - anchor.as_vec() * world_size;
    Some((center, world_size))
}
