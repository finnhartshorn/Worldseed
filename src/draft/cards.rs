use bevy::prelude::*;

const DRAFT_CARD_FRAME_BRONZE_BLUE_INDEX: usize = 0;
const DRAFT_CARD_FRAME_BRONZE_RED_INDEX: usize = 1;
const DRAFT_ISLAND_TEXTURE_PATH: &str = "ui/New_Hills_forgotten_plains.png";

pub const VISIBLE_DRAFT_CARDS: [DraftCard; 4] = [
    DraftCard::Grass,
    DraftCard::Dirt,
    DraftCard::Island,
    DraftCard::Infinity,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DraftCard {
    Human,
    Guardian,
    Snail,
    Grass,
    Dirt,
    Island,
    Infinity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DraftCardCategory {
    Entity,
    Element,
    Shape,
}

impl DraftCard {
    pub fn category(self) -> DraftCardCategory {
        match self {
            DraftCard::Human | DraftCard::Guardian | DraftCard::Snail => DraftCardCategory::Entity,
            DraftCard::Grass | DraftCard::Dirt => DraftCardCategory::Element,
            DraftCard::Island | DraftCard::Infinity => DraftCardCategory::Shape,
        }
    }

    pub fn unlocks_next_row(self) -> bool {
        matches!(self, Self::Island | Self::Infinity)
    }

    pub fn frame_index(self) -> usize {
        if self.unlocks_next_row() {
            DRAFT_CARD_FRAME_BRONZE_RED_INDEX
        } else {
            DRAFT_CARD_FRAME_BRONZE_BLUE_INDEX
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            DraftCard::Human => "Human",
            DraftCard::Guardian => "Guardian",
            DraftCard::Snail => "Snail",
            DraftCard::Grass => "Grass",
            DraftCard::Dirt => "Dirt",
            DraftCard::Island => "Island Card",
            DraftCard::Infinity => "Infinity Card",
        }
    }
}

pub fn label_plate_height(label: &str) -> f32 {
    match label.lines().count() {
        0 | 1 => 24.0,
        2 => 40.0,
        _ => 56.0,
    }
}

pub fn set_icon(
    image_node: &mut ImageNode,
    card: DraftCard,
    assets: Option<&AssetServer>,
    texture_atlas_layouts: Option<&mut Assets<TextureAtlasLayout>>,
) {
    let Some(assets) = assets else {
        image_node.image = Handle::default();
        image_node.texture_atlas = None;
        return;
    };

    match card {
        DraftCard::Human => {
            image_node.image = assets.load("characters/human_walk.png");
            image_node.rect = None;
            image_node.texture_atlas = texture_atlas_layouts.map(|layouts| TextureAtlas {
                layout: layouts.add(TextureAtlasLayout::from_grid(
                    UVec2::splat(32),
                    4,
                    4,
                    None,
                    None,
                )),
                index: 0,
            });
        }
        DraftCard::Guardian => {
            image_node.image = assets.load("creatures/forest_guardians/oak_guardian_idle.png");
            image_node.rect = None;
            image_node.texture_atlas = texture_atlas_layouts.map(|layouts| TextureAtlas {
                layout: layouts.add(TextureAtlasLayout::from_grid(
                    UVec2::splat(32),
                    8,
                    4,
                    None,
                    None,
                )),
                index: 0,
            });
        }
        DraftCard::Snail => {
            image_node.image = assets.load("creatures/snail/snail_crawl.png");
            image_node.rect = None;
            image_node.texture_atlas = texture_atlas_layouts.map(|layouts| TextureAtlas {
                layout: layouts.add(TextureAtlasLayout::from_grid(
                    UVec2::splat(32),
                    4,
                    4,
                    None,
                    None,
                )),
                index: 0,
            });
        }
        DraftCard::Grass => {
            image_node.image = assets.load("tilesets/terrain_array_ui.png");
            image_node.rect = None;
            image_node.texture_atlas = texture_atlas_layouts.map(|layouts| TextureAtlas {
                layout: layouts.add(TextureAtlasLayout::from_grid(
                    UVec2::splat(8),
                    1,
                    2,
                    None,
                    None,
                )),
                index: 0,
            });
        }
        DraftCard::Dirt => {
            image_node.image = assets.load("tilesets/terrain_array_ui.png");
            image_node.rect = None;
            image_node.texture_atlas = texture_atlas_layouts.map(|layouts| TextureAtlas {
                layout: layouts.add(TextureAtlasLayout::from_grid(
                    UVec2::splat(8),
                    1,
                    2,
                    None,
                    None,
                )),
                index: 1,
            });
        }
        DraftCard::Island => {
            image_node.image = assets.load(DRAFT_ISLAND_TEXTURE_PATH);
            image_node.rect = Some(Rect::new(16.0, 8.0, 40.0, 32.0));
            image_node.texture_atlas = None;
        }
        DraftCard::Infinity => {
            image_node.image = Handle::default();
            image_node.rect = None;
            image_node.texture_atlas = None;
        }
    }
}
