use crate::tiles::{
    ChunkPos, TileId, CHUNK_SIZE, CHUNK_SIZE_I32, LAYER_GROUND, LAYER_OVERLAY, TILE_DIRT,
    TILE_EMPTY, TILE_GRASS,
};
use crate::world::manager::WorldManager;
use bevy::asset::{Assets, RenderAssetUsages};
use bevy::image::{Image, ImageSampler};
use bevy::log::error;
use bevy::prelude::{Handle, IVec2, Resource};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use image::RgbaImage;
use std::collections::{HashMap, HashSet};

const BIT_NW: u8 = 1 << 0;
const BIT_N: u8 = 1 << 1;
const BIT_NE: u8 = 1 << 2;
const BIT_W: u8 = 1 << 3;
const BIT_E: u8 = 1 << 4;
const BIT_SW: u8 = 1 << 5;
const BIT_S: u8 = 1 << 6;
const BIT_SE: u8 = 1 << 7;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StampKind {
    Nw,
    N,
    Ne,
    W,
    E,
    Sw,
    S,
    Se,
    Tl,
    Tr,
    Bl,
    Br,
    TlBr,
    BlTr,
}

impl StampKind {
    const fn bit(self) -> u16 {
        match self {
            Self::Nw => 1 << 0,
            Self::N => 1 << 1,
            Self::Ne => 1 << 2,
            Self::W => 1 << 3,
            Self::E => 1 << 4,
            Self::Sw => 1 << 5,
            Self::S => 1 << 6,
            Self::Se => 1 << 7,
            Self::Tl => 1 << 8,
            Self::Tr => 1 << 9,
            Self::Bl => 1 << 10,
            Self::Br => 1 << 11,
            Self::TlBr => 1 << 12,
            Self::BlTr => 1 << 13,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct StampSet(u16);

impl StampSet {
    fn insert(&mut self, stamp: StampKind) {
        self.0 |= stamp.bit();
    }

    fn remove(&mut self, stamp: StampKind) {
        self.0 &= !stamp.bit();
    }

    pub fn contains(&self, stamp: StampKind) -> bool {
        self.0 & stamp.bit() != 0
    }

    fn ordered(self) -> [Option<StampKind>; 14] {
        let order = [
            StampKind::N,
            StampKind::E,
            StampKind::S,
            StampKind::W,
            StampKind::Nw,
            StampKind::Ne,
            StampKind::Sw,
            StampKind::Se,
            StampKind::TlBr,
            StampKind::BlTr,
            StampKind::Tl,
            StampKind::Tr,
            StampKind::Bl,
            StampKind::Br,
        ];
        let mut out = [None; 14];
        for (idx, stamp) in order.into_iter().enumerate() {
            if self.contains(stamp) {
                out[idx] = Some(stamp);
            }
        }
        out
    }
}

#[derive(Debug, Clone)]
pub struct TerrainTransitionDescriptor {
    pub terrain_tile: TileId,
    pub terrain_stamps_path: String,
    pub terrain_stamps_origin: (u32, u32),
    pub void_stamps_path: String,
    pub void_stamps_origin: (u32, u32),
    /// Key: packed masks (`void_mask << 8 | terrain_mask`), value: overlay tile id.
    /// Kept for direct single-source lookups and focused tests.
    pub mask_to_overlay_tile: HashMap<u16, TileId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct TransitionContribution {
    source_tile: TileId,
    terrain_mask: u8,
    void_mask: u8,
}

#[derive(Resource, Debug, Clone)]
pub struct TerrainTransitionConfig {
    pub enabled: bool,
    pub overlay_layer: usize,
    pub terrain_precedence: Vec<TileId>,
    pub assets_root: String,
    pub base_tileset_path: String,
    pub base_tile_count: usize,
    pub tile_size: u32,
    pub generated_tileset: Option<Handle<Image>>,
    pub generated_tileset_rows: usize,
    pub descriptors: HashMap<TileId, TerrainTransitionDescriptor>,
    composite_to_overlay_tile: HashMap<Vec<TransitionContribution>, TileId>,
    stamp_sheets_by_terrain: HashMap<TileId, TerrainStampSheets>,
    unique_tile_index: HashMap<Vec<u8>, usize>,
}

impl Default for TerrainTransitionConfig {
    fn default() -> Self {
        let stamp_sheet = "tmp/Biome_Transitions/Biome_Transitions.png";
        let mut descriptors = HashMap::new();
        descriptors.insert(
            TILE_GRASS,
            TerrainTransitionDescriptor {
                terrain_tile: TILE_GRASS,
                terrain_stamps_path: stamp_sheet.to_string(),
                terrain_stamps_origin: (8, 8),
                void_stamps_path: stamp_sheet.to_string(),
                void_stamps_origin: (40, 8),
                mask_to_overlay_tile: HashMap::new(),
            },
        );
        descriptors.insert(
            TILE_DIRT,
            TerrainTransitionDescriptor {
                terrain_tile: TILE_DIRT,
                terrain_stamps_path: stamp_sheet.to_string(),
                terrain_stamps_origin: (72, 8),
                void_stamps_path: stamp_sheet.to_string(),
                void_stamps_origin: (104, 8),
                mask_to_overlay_tile: HashMap::new(),
            },
        );

        Self {
            enabled: true,
            overlay_layer: LAYER_OVERLAY,
            terrain_precedence: vec![TILE_GRASS, TILE_DIRT],
            assets_root: ".".to_string(),
            base_tileset_path: "assets/tilesets/terrain_array.png".to_string(),
            base_tile_count: 2,
            tile_size: 8,
            generated_tileset: None,
            generated_tileset_rows: 2,
            descriptors,
            composite_to_overlay_tile: HashMap::new(),
            stamp_sheets_by_terrain: HashMap::new(),
            unique_tile_index: HashMap::new(),
        }
    }
}

impl TerrainTransitionConfig {
    pub fn packed_mask(terrain_mask: u8, void_mask: u8) -> u16 {
        ((void_mask as u16) << 8) | terrain_mask as u16
    }

    pub fn resolve_overlay_tile(
        &self,
        center_tile: TileId,
        terrain_mask: u8,
        void_mask: u8,
    ) -> TileId {
        let Some(descriptor) = self.descriptors.get(&center_tile) else {
            return TILE_EMPTY;
        };
        let key = Self::packed_mask(terrain_mask, void_mask);
        descriptor
            .mask_to_overlay_tile
            .get(&key)
            .copied()
            .unwrap_or(TILE_EMPTY)
    }

    fn resolve_composite_overlay_tile(&self, contributions: &[TransitionContribution]) -> TileId {
        self.composite_to_overlay_tile
            .get(contributions)
            .copied()
            .unwrap_or(TILE_EMPTY)
    }

    fn terrain_rank(&self, tile_id: TileId) -> Option<usize> {
        self.terrain_precedence
            .iter()
            .position(|&candidate| candidate == tile_id)
    }

    fn has_precedence_over(&self, center_tile: TileId, neighbor_tile: TileId) -> bool {
        match (
            self.terrain_rank(center_tile),
            self.terrain_rank(neighbor_tile),
        ) {
            (Some(center), Some(neighbor)) => center < neighbor,
            (Some(_), None) => true,
            _ => false,
        }
    }
}

pub fn ensure_runtime_tileset(
    config: &mut TerrainTransitionConfig,
    images: &mut Assets<Image>,
) -> Result<Handle<Image>, String> {
    if let Some(existing) = config.generated_tileset.clone() {
        return Ok(existing);
    }

    let tile_size = config.tile_size;
    let assets_root = std::path::Path::new(&config.assets_root);
    let base_path = assets_root.join(&config.base_tileset_path);
    let base_image = load_rgba_image(&base_path)?;
    let base_tiles = extract_column_tiles(&base_image, tile_size, config.base_tile_count)?;

    let mut unique_tiles: Vec<Vec<u8>> = base_tiles;
    let mut unique_index: HashMap<Vec<u8>, usize> = unique_tiles
        .iter()
        .enumerate()
        .map(|(idx, tile)| (tile.clone(), idx))
        .collect();
    let mut stamp_cache: HashMap<String, HashMap<StampKind, Vec<u8>>> = HashMap::new();
    let mut stamps_by_terrain = HashMap::new();

    for descriptor in config.descriptors.values_mut() {
        descriptor.mask_to_overlay_tile.clear();
        let terrain_stamps = load_stamp_sheet(
            &assets_root.join(&descriptor.terrain_stamps_path),
            descriptor.terrain_stamps_origin,
            tile_size,
            &mut stamp_cache,
        )?;
        let void_stamps = load_stamp_sheet(
            &assets_root.join(&descriptor.void_stamps_path),
            descriptor.void_stamps_origin,
            tile_size,
            &mut stamp_cache,
        )?;

        for (terrain_mask, void_mask) in (1u16..=255)
            .map(|mask| (mask as u8, 0))
            .chain((1u16..=255).map(|mask| (0, mask as u8)))
        {
            let tile = compose_overlay_tile(
                tile_size,
                &terrain_stamps,
                &void_stamps,
                terrain_mask,
                void_mask,
            );
            if is_fully_transparent(&tile) {
                continue;
            }
            let index = if let Some(index) = unique_index.get(&tile) {
                *index
            } else {
                let next = unique_tiles.len();
                unique_index.insert(tile.clone(), next);
                unique_tiles.push(tile);
                next
            };
            descriptor.mask_to_overlay_tile.insert(
                TerrainTransitionConfig::packed_mask(terrain_mask, void_mask),
                (index + 1) as TileId,
            );
        }

        stamps_by_terrain.insert(
            descriptor.terrain_tile,
            TerrainStampSheets {
                terrain_stamps,
                void_stamps,
            },
        );
    }

    config.composite_to_overlay_tile.clear();
    config.stamp_sheets_by_terrain = stamps_by_terrain;
    config.unique_tile_index = unique_index;

    let layer_count = unique_tiles.len() as u32;
    let mut data = Vec::with_capacity((tile_size * tile_size * 4 * layer_count) as usize);
    for tile in &unique_tiles {
        data.extend_from_slice(tile);
    }
    let mut image = Image::new(
        Extent3d {
            width: tile_size,
            height: tile_size,
            depth_or_array_layers: layer_count.max(1),
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    let handle = images.add(image);
    config.generated_tileset_rows = layer_count as usize;
    config.generated_tileset = Some(handle.clone());
    Ok(handle)
}

pub fn log_transition_cache_pressure(config: bevy::prelude::Res<TerrainTransitionConfig>) {
    if config.generated_tileset_rows > 2000 {
        error!(
            "Transition texture cache is near the 2048 array-layer limit: {} layers allocated",
            config.generated_tileset_rows
        );
    }
}

#[derive(Debug, Clone)]
struct TerrainStampSheets {
    terrain_stamps: HashMap<StampKind, Vec<u8>>,
    void_stamps: HashMap<StampKind, Vec<u8>>,
}

fn compose_composite_overlay_tile(
    tile_size: u32,
    stamps_by_terrain: &HashMap<TileId, TerrainStampSheets>,
    contributions: &[TransitionContribution],
) -> Vec<u8> {
    let mut out = vec![0u8; (tile_size * tile_size * 4) as usize];
    for contribution in contributions {
        let Some(stamps) = stamps_by_terrain.get(&contribution.source_tile) else {
            continue;
        };
        let tile = compose_overlay_tile(
            tile_size,
            &stamps.terrain_stamps,
            &stamps.void_stamps,
            contribution.terrain_mask,
            contribution.void_mask,
        );
        alpha_non_overlap_blit(&mut out, &tile);
    }
    out
}

fn get_or_insert_composite_overlay_tile(
    config: &mut TerrainTransitionConfig,
    images: &mut Assets<Image>,
    contributions: Vec<TransitionContribution>,
) -> TileId {
    let existing = config.resolve_composite_overlay_tile(&contributions);
    if existing != TILE_EMPTY {
        return existing;
    }

    let tile = compose_composite_overlay_tile(
        config.tile_size,
        &config.stamp_sheets_by_terrain,
        &contributions,
    );
    if is_fully_transparent(&tile) {
        return TILE_EMPTY;
    }

    let index = if let Some(index) = config.unique_tile_index.get(&tile) {
        *index
    } else {
        let Some(handle) = config.generated_tileset.clone() else {
            error!("Cannot cache transition overlay before generated tileset exists");
            return TILE_EMPTY;
        };
        let Some(image) = images.get_mut(&handle) else {
            error!("Cannot cache transition overlay because generated tileset image is missing");
            return TILE_EMPTY;
        };
        let Some(data) = image.data.as_mut() else {
            error!(
                "Cannot cache transition overlay because generated tileset image has no CPU data"
            );
            return TILE_EMPTY;
        };

        let next = config.generated_tileset_rows;
        if next >= u16::MAX as usize {
            error!("Transition cache exhausted u16 tile ids");
            return TILE_EMPTY;
        }

        data.extend_from_slice(&tile);
        image.texture_descriptor.size.depth_or_array_layers = (next + 1) as u32;
        config.unique_tile_index.insert(tile, next);
        config.generated_tileset_rows = next + 1;
        next
    };

    let tile_id = (index + 1) as TileId;
    config
        .composite_to_overlay_tile
        .insert(contributions, tile_id);
    tile_id
}

fn sorted_terrain_tiles(config: &TerrainTransitionConfig) -> Vec<TileId> {
    let mut out = Vec::new();
    for &tile_id in &config.terrain_precedence {
        if config.descriptors.contains_key(&tile_id) && !out.contains(&tile_id) {
            out.push(tile_id);
        }
    }

    let mut remaining: Vec<_> = config
        .descriptors
        .keys()
        .copied()
        .filter(|tile_id| !out.contains(tile_id))
        .collect();
    remaining.sort_unstable();
    out.extend(remaining);
    out
}

fn build_contribution_key(
    config: &TerrainTransitionConfig,
    target_tile: TileId,
    neighbor_tiles: &[TileId; 8],
) -> Vec<TransitionContribution> {
    let mut contributions = Vec::new();
    for source_tile in sorted_terrain_tiles(config) {
        if !is_valid_source_for_target(config, source_tile, target_tile) {
            continue;
        }

        let mut mask = 0u8;
        for (index, (bit, _)) in neighbor_offsets().into_iter().enumerate() {
            if neighbor_tiles[index] == source_tile {
                mask |= bit;
            }
        }

        if mask == 0 {
            continue;
        }

        let contribution = if target_tile == TILE_EMPTY {
            TransitionContribution {
                source_tile,
                terrain_mask: 0,
                void_mask: mask,
            }
        } else {
            TransitionContribution {
                source_tile,
                terrain_mask: mask,
                void_mask: 0,
            }
        };
        contributions.push(contribution);
    }

    contributions
}

fn load_rgba_image(path: &std::path::Path) -> Result<RgbaImage, String> {
    let image = image::open(path)
        .map_err(|error| format!("Failed to open {}: {}", path.display(), error))?;
    Ok(image.to_rgba8())
}

fn extract_column_tiles(
    image: &RgbaImage,
    tile_size: u32,
    count: usize,
) -> Result<Vec<Vec<u8>>, String> {
    if image.width() < tile_size || image.height() < tile_size * count as u32 {
        return Err(format!(
            "Base tileset is too small: expected at least {}x{}, got {}x{}",
            tile_size,
            tile_size * count as u32,
            image.width(),
            image.height()
        ));
    }

    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let y0 = i as u32 * tile_size;
        out.push(extract_tile(image, 0, y0, tile_size));
    }
    Ok(out)
}

fn load_stamp_sheet(
    path: &std::path::Path,
    origin: (u32, u32),
    tile_size: u32,
    cache: &mut HashMap<String, HashMap<StampKind, Vec<u8>>>,
) -> Result<HashMap<StampKind, Vec<u8>>, String> {
    let key = format!("{}:{}:{}", path.display(), origin.0, origin.1);
    if let Some(existing) = cache.get(&key) {
        return Ok(existing.clone());
    }

    let image = load_rgba_image(path)?;
    let required_width = origin.0 + tile_size * 3;
    let required_height = origin.1 + tile_size * 5;
    if image.width() < required_width || image.height() < required_height {
        return Err(format!(
            "Stamp sheet {} at origin ({}, {}) must fit {}x{}, got {}x{}",
            path.display(),
            origin.0,
            origin.1,
            required_width,
            required_height,
            image.width(),
            image.height()
        ));
    }

    let mut map = HashMap::new();
    let ox = origin.0;
    let oy = origin.1;
    map.insert(StampKind::Nw, extract_tile(&image, ox, oy, tile_size));
    map.insert(
        StampKind::N,
        extract_tile(&image, ox + tile_size, oy, tile_size),
    );
    map.insert(
        StampKind::Ne,
        extract_tile(&image, ox + tile_size * 2, oy, tile_size),
    );
    map.insert(
        StampKind::W,
        extract_tile(&image, ox, oy + tile_size, tile_size),
    );
    map.insert(
        StampKind::E,
        extract_tile(&image, ox + tile_size * 2, oy + tile_size, tile_size),
    );
    map.insert(
        StampKind::Sw,
        extract_tile(&image, ox, oy + tile_size * 2, tile_size),
    );
    map.insert(
        StampKind::S,
        extract_tile(&image, ox + tile_size, oy + tile_size * 2, tile_size),
    );
    map.insert(
        StampKind::Se,
        extract_tile(&image, ox + tile_size * 2, oy + tile_size * 2, tile_size),
    );
    map.insert(
        StampKind::Tl,
        extract_tile(&image, ox, oy + tile_size * 3, tile_size),
    );
    map.insert(
        StampKind::Tr,
        extract_tile(&image, ox + tile_size, oy + tile_size * 3, tile_size),
    );
    map.insert(
        StampKind::TlBr,
        extract_tile(&image, ox + tile_size * 2, oy + tile_size * 3, tile_size),
    );
    map.insert(
        StampKind::Bl,
        extract_tile(&image, ox, oy + tile_size * 4, tile_size),
    );
    map.insert(
        StampKind::Br,
        extract_tile(&image, ox + tile_size, oy + tile_size * 4, tile_size),
    );
    map.insert(
        StampKind::BlTr,
        extract_tile(&image, ox + tile_size * 2, oy + tile_size * 4, tile_size),
    );

    cache.insert(key, map.clone());
    Ok(map)
}

fn extract_tile(image: &RgbaImage, x0: u32, y0: u32, tile_size: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity((tile_size * tile_size * 4) as usize);
    for y in y0..(y0 + tile_size) {
        for x in x0..(x0 + tile_size) {
            out.extend_from_slice(image.get_pixel(x, y).0.as_slice());
        }
    }
    out
}

fn compose_overlay_tile(
    tile_size: u32,
    terrain_stamps: &HashMap<StampKind, Vec<u8>>,
    void_stamps: &HashMap<StampKind, Vec<u8>>,
    terrain_mask: u8,
    void_mask: u8,
) -> Vec<u8> {
    let mut out = vec![0u8; (tile_size * tile_size * 4) as usize];
    let terrain = derive_stamp_set(terrain_mask).ordered();
    for stamp in terrain.into_iter().flatten() {
        if let Some(tile) = terrain_stamps.get(&stamp) {
            alpha_non_overlap_blit(&mut out, tile);
        }
    }
    let void = derive_stamp_set(void_mask).ordered();
    for stamp in void.into_iter().flatten() {
        if let Some(tile) = void_stamps.get(&stamp) {
            alpha_non_overlap_blit(&mut out, tile);
        }
    }
    out
}

fn alpha_non_overlap_blit(dst: &mut [u8], src: &[u8]) {
    for (dst_px, src_px) in dst.chunks_exact_mut(4).zip(src.chunks_exact(4)) {
        let src_a = src_px[3];
        let dst_a = dst_px[3];
        if src_a != 0 && dst_a == 0 {
            dst_px.copy_from_slice(src_px);
        }
    }
}

fn is_fully_transparent(tile: &[u8]) -> bool {
    tile.chunks_exact(4).all(|pixel| pixel[3] == 0)
}

/// Returns stamp semantics for a single 8-neighbor mask.
/// This captures the `NW` (connected) vs `TL` (isolated diagonal) distinction.
pub fn derive_stamp_set(mask: u8) -> StampSet {
    let mut out = StampSet::default();

    if mask & BIT_N != 0 {
        out.insert(StampKind::N);
    }
    if mask & BIT_E != 0 {
        out.insert(StampKind::E);
    }
    if mask & BIT_S != 0 {
        out.insert(StampKind::S);
    }
    if mask & BIT_W != 0 {
        out.insert(StampKind::W);
    }

    // Connected corners: if both adjacent edges differ, use corner stamp.
    if mask & BIT_N != 0 && mask & BIT_W != 0 {
        out.insert(StampKind::Nw);
    }
    if mask & BIT_N != 0 && mask & BIT_E != 0 {
        out.insert(StampKind::Ne);
    }
    if mask & BIT_S != 0 && mask & BIT_W != 0 {
        out.insert(StampKind::Sw);
    }
    if mask & BIT_S != 0 && mask & BIT_E != 0 {
        out.insert(StampKind::Se);
    }

    // Isolated diagonals: only when the diagonal differs and both adjacent edges do not.
    let tl = mask & BIT_NW != 0 && mask & BIT_N == 0 && mask & BIT_W == 0;
    let tr = mask & BIT_NE != 0 && mask & BIT_N == 0 && mask & BIT_E == 0;
    let bl = mask & BIT_SW != 0 && mask & BIT_S == 0 && mask & BIT_W == 0;
    let br = mask & BIT_SE != 0 && mask & BIT_S == 0 && mask & BIT_E == 0;

    if tl {
        out.insert(StampKind::Tl);
    }
    if tr {
        out.insert(StampKind::Tr);
    }
    if bl {
        out.insert(StampKind::Bl);
    }
    if br {
        out.insert(StampKind::Br);
    }

    // Prefer paired diagonal source stamps when both halves are active.
    if tl && br {
        out.remove(StampKind::Tl);
        out.remove(StampKind::Br);
        out.insert(StampKind::TlBr);
    }
    if bl && tr {
        out.remove(StampKind::Bl);
        out.remove(StampKind::Tr);
        out.insert(StampKind::BlTr);
    }

    out
}

pub fn refresh_chunk_transitions(
    world: &mut WorldManager,
    config: &mut TerrainTransitionConfig,
    images: &mut Assets<Image>,
    chunk_pos: ChunkPos,
) -> HashSet<ChunkPos> {
    let mut changed_chunks = HashSet::new();
    if !config.enabled {
        return changed_chunks;
    }
    if let Err(error) = ensure_runtime_tileset(config, images) {
        error!("Failed to build transition tileset: {}", error);
        return changed_chunks;
    }

    let base_x = chunk_pos.x * CHUNK_SIZE_I32;
    let base_y = chunk_pos.y * CHUNK_SIZE_I32;

    for y in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let tile_pos = IVec2::new(base_x + x as i32, base_y + y as i32);
            if let Some(changed) = recompute_overlay_for_tile(world, config, images, tile_pos) {
                changed_chunks.insert(changed);
            }
        }
    }

    changed_chunks
}

pub fn refresh_transitions_around_ground_edit(
    world: &mut WorldManager,
    config: &mut TerrainTransitionConfig,
    images: &mut Assets<Image>,
    edited_tile: IVec2,
) -> HashSet<ChunkPos> {
    let mut changed_chunks = HashSet::new();
    if !config.enabled {
        return changed_chunks;
    }
    if let Err(error) = ensure_runtime_tileset(config, images) {
        error!("Failed to build transition tileset: {}", error);
        return changed_chunks;
    }

    for dy in -1..=1 {
        for dx in -1..=1 {
            let tile_pos = IVec2::new(edited_tile.x + dx, edited_tile.y + dy);
            if let Some(changed) = recompute_overlay_for_tile(world, config, images, tile_pos) {
                changed_chunks.insert(changed);
            }
        }
    }

    changed_chunks
}

fn recompute_overlay_for_tile(
    world: &mut WorldManager,
    config: &mut TerrainTransitionConfig,
    images: &mut Assets<Image>,
    tile_pos: IVec2,
) -> Option<ChunkPos> {
    if config.overlay_layer == LAYER_GROUND {
        return None;
    }

    let overlay_tile = resolve_overlay_for_tile(world, config, images, tile_pos);

    set_tile_at_tile(world, config.overlay_layer, tile_pos, overlay_tile)
}

fn resolve_overlay_for_tile(
    world: &mut WorldManager,
    config: &mut TerrainTransitionConfig,
    images: &mut Assets<Image>,
    tile_pos: IVec2,
) -> TileId {
    let center_tile = ground_tile_at_tile(world, tile_pos);
    let neighbor_tiles = neighbor_tiles_at(world, tile_pos);
    let contributions = build_contribution_key(config, center_tile, &neighbor_tiles);
    if contributions.is_empty() {
        return TILE_EMPTY;
    }

    get_or_insert_composite_overlay_tile(config, images, contributions)
}

#[cfg(test)]
fn select_source_terrain_for_target(
    world: &mut WorldManager,
    config: &TerrainTransitionConfig,
    tile_pos: IVec2,
    center_tile: TileId,
) -> Option<TileId> {
    let mut source = None;
    for (_, offset) in neighbor_offsets() {
        let neighbor_tile = ground_tile_at_tile(world, tile_pos + offset);
        if !is_valid_source_for_target(config, neighbor_tile, center_tile) {
            continue;
        }

        source = match source {
            Some(current) if config.has_precedence_over(current, neighbor_tile) => Some(current),
            _ => Some(neighbor_tile),
        };
    }
    source
}

fn neighbor_tiles_at(world: &mut WorldManager, tile_pos: IVec2) -> [TileId; 8] {
    let mut out = [TILE_EMPTY; 8];
    for (index, (_, offset)) in neighbor_offsets().into_iter().enumerate() {
        out[index] = ground_tile_at_tile(world, tile_pos + offset);
    }
    out
}

fn is_valid_source_for_target(
    config: &TerrainTransitionConfig,
    source_tile: TileId,
    target_tile: TileId,
) -> bool {
    if source_tile == TILE_EMPTY || source_tile == target_tile {
        return false;
    }

    if target_tile == TILE_EMPTY {
        config.descriptors.contains_key(&source_tile)
    } else {
        config.has_precedence_over(source_tile, target_tile)
    }
}

#[cfg(test)]
fn build_source_neighbor_mask(
    world: &mut WorldManager,
    tile_pos: IVec2,
    source_tile: TileId,
) -> u8 {
    let mut mask = 0u8;
    for (bit, offset) in neighbor_offsets() {
        let neighbor_tile = ground_tile_at_tile(world, tile_pos + offset);
        if neighbor_tile == source_tile {
            mask |= bit;
        }
    }
    mask
}

fn neighbor_offsets() -> [(u8, IVec2); 8] {
    [
        (BIT_NW, IVec2::new(-1, 1)),
        (BIT_N, IVec2::new(0, 1)),
        (BIT_NE, IVec2::new(1, 1)),
        (BIT_W, IVec2::new(-1, 0)),
        (BIT_E, IVec2::new(1, 0)),
        (BIT_SW, IVec2::new(-1, -1)),
        (BIT_S, IVec2::new(0, -1)),
        (BIT_SE, IVec2::new(1, -1)),
    ]
}

fn ground_tile_at_tile(world: &mut WorldManager, tile_pos: IVec2) -> TileId {
    let chunk_pos = ChunkPos::from_tile(tile_pos, CHUNK_SIZE_I32);
    world.ensure_chunk_cached(chunk_pos);
    let local_x = tile_pos.x.rem_euclid(CHUNK_SIZE_I32) as usize;
    let local_y = tile_pos.y.rem_euclid(CHUNK_SIZE_I32) as usize;
    world
        .chunk_cache
        .get(&chunk_pos)
        .and_then(|chunk| chunk.get_tile(LAYER_GROUND, local_x, local_y))
        .unwrap_or(TILE_EMPTY)
}

fn set_tile_at_tile(
    world: &mut WorldManager,
    layer: usize,
    tile_pos: IVec2,
    tile_id: TileId,
) -> Option<ChunkPos> {
    let chunk_pos = ChunkPos::from_tile(tile_pos, CHUNK_SIZE_I32);
    world.ensure_chunk_cached(chunk_pos);

    let local_x = tile_pos.x.rem_euclid(CHUNK_SIZE_I32) as usize;
    let local_y = tile_pos.y.rem_euclid(CHUNK_SIZE_I32) as usize;
    let chunk = world.chunk_cache.get_mut(&chunk_pos)?;
    let previous = chunk
        .get_tile(layer, local_x, local_y)
        .unwrap_or(TILE_EMPTY);
    if previous == tile_id {
        return None;
    }
    if chunk.set_tile(layer, local_x, local_y, tile_id) {
        Some(chunk_pos)
    } else {
        None
    }
}

pub fn world_to_tile(world_x: f32, world_y: f32) -> IVec2 {
    use crate::tiles::TILE_WORLD_SIZE;
    IVec2::new(
        (world_x / TILE_WORLD_SIZE).floor() as i32,
        (world_y / TILE_WORLD_SIZE).floor() as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tiles::{ChunkData, ChunkPos, LAYER_GROUND, TILE_DIRT, TILE_GRASS};

    #[test]
    fn connected_corner_uses_nw_not_tl() {
        let mask = BIT_N | BIT_W | BIT_NW;
        let stamps = derive_stamp_set(mask);
        assert!(stamps.contains(StampKind::Nw));
        assert!(!stamps.contains(StampKind::Tl));
    }

    #[test]
    fn isolated_diagonal_uses_tl() {
        let mask = BIT_NW;
        let stamps = derive_stamp_set(mask);
        assert!(stamps.contains(StampKind::Tl));
        assert!(!stamps.contains(StampKind::Nw));
    }

    #[test]
    fn paired_isolated_diagonals_use_compound_stamp() {
        let mask = BIT_NW | BIT_SE;
        let stamps = derive_stamp_set(mask);
        assert!(stamps.contains(StampKind::TlBr));
        assert!(!stamps.contains(StampKind::Tl));
        assert!(!stamps.contains(StampKind::Br));
    }

    #[test]
    fn refresh_around_ground_edit_updates_overlay_layer() {
        let mut world = WorldManager::default();
        let chunk_pos = ChunkPos::new(0, 0);
        let mut chunk = ChunkData::filled(chunk_pos, TILE_DIRT);
        chunk.set_tile(LAYER_GROUND, 5, 5, TILE_GRASS);
        chunk.set_tile(LAYER_GROUND, 6, 5, TILE_DIRT);
        world.cache_chunk(chunk);

        let mut map = HashMap::new();
        map.insert(TerrainTransitionConfig::packed_mask(BIT_W, 0), 42);
        let mut descriptors = HashMap::new();
        descriptors.insert(
            TILE_GRASS,
            TerrainTransitionDescriptor {
                terrain_tile: TILE_GRASS,
                terrain_stamps_path: String::new(),
                terrain_stamps_origin: (0, 0),
                void_stamps_path: String::new(),
                void_stamps_origin: (0, 0),
                mask_to_overlay_tile: map,
            },
        );
        let mut config = TerrainTransitionConfig {
            enabled: true,
            overlay_layer: LAYER_OVERLAY,
            terrain_precedence: vec![TILE_GRASS, TILE_DIRT],
            assets_root: "assets".to_string(),
            base_tileset_path: "tilesets/terrain_array.png".to_string(),
            base_tile_count: 2,
            tile_size: 8,
            generated_tileset: Some(Handle::default()),
            generated_tileset_rows: 2,
            descriptors,
            composite_to_overlay_tile: HashMap::from([(
                vec![TransitionContribution {
                    source_tile: TILE_GRASS,
                    terrain_mask: BIT_W,
                    void_mask: 0,
                }],
                42,
            )]),
            stamp_sheets_by_terrain: HashMap::new(),
            unique_tile_index: HashMap::new(),
        };

        let mut images = Assets::<Image>::default();
        let changed = refresh_transitions_around_ground_edit(
            &mut world,
            &mut config,
            &mut images,
            IVec2::new(6, 5),
        );
        assert!(changed.contains(&chunk_pos));
        let overlay = world
            .get_cached_chunk(&chunk_pos)
            .and_then(|c| c.get_tile(LAYER_OVERLAY, 6, 5))
            .unwrap_or(TILE_EMPTY);
        assert_eq!(overlay, 42);
    }

    #[test]
    fn higher_precedence_tile_does_not_receive_lower_precedence_overlay() {
        let mut world = WorldManager::default();
        let chunk_pos = ChunkPos::new(0, 0);
        let mut chunk = ChunkData::filled(chunk_pos, TILE_GRASS);
        chunk.set_tile(LAYER_GROUND, 6, 5, TILE_DIRT);
        world.cache_chunk(chunk);

        let config = TerrainTransitionConfig::default();
        let source =
            select_source_terrain_for_target(&mut world, &config, IVec2::new(5, 5), TILE_GRASS);

        assert_eq!(source, None);
    }

    #[test]
    fn lower_precedence_tile_receives_higher_precedence_overlay() {
        let mut world = WorldManager::default();
        let chunk_pos = ChunkPos::new(0, 0);
        let mut chunk = ChunkData::filled(chunk_pos, TILE_DIRT);
        chunk.set_tile(LAYER_GROUND, 5, 5, TILE_GRASS);
        chunk.set_tile(LAYER_GROUND, 6, 5, TILE_DIRT);
        world.cache_chunk(chunk);

        let config = TerrainTransitionConfig::default();
        let source =
            select_source_terrain_for_target(&mut world, &config, IVec2::new(6, 5), TILE_DIRT);
        let mask = build_source_neighbor_mask(&mut world, IVec2::new(6, 5), TILE_GRASS);

        assert_eq!(source, Some(TILE_GRASS));
        assert_ne!(mask & BIT_W, 0);
    }

    #[test]
    fn void_tile_receives_adjacent_terrain_void_overlay() {
        let mut world = WorldManager::default();
        let chunk_pos = ChunkPos::new(0, 0);
        let mut chunk = ChunkData::empty(chunk_pos);
        chunk.set_tile(LAYER_GROUND, 5, 5, TILE_GRASS);
        world.cache_chunk(chunk);

        let mut map = HashMap::new();
        map.insert(TerrainTransitionConfig::packed_mask(0, BIT_W), 84);
        let mut descriptors = HashMap::new();
        descriptors.insert(
            TILE_GRASS,
            TerrainTransitionDescriptor {
                terrain_tile: TILE_GRASS,
                terrain_stamps_path: String::new(),
                terrain_stamps_origin: (0, 0),
                void_stamps_path: String::new(),
                void_stamps_origin: (0, 0),
                mask_to_overlay_tile: map,
            },
        );
        let mut config = TerrainTransitionConfig {
            enabled: true,
            overlay_layer: LAYER_OVERLAY,
            terrain_precedence: vec![TILE_GRASS, TILE_DIRT],
            assets_root: "assets".to_string(),
            base_tileset_path: "tilesets/terrain_array.png".to_string(),
            base_tile_count: 2,
            tile_size: 8,
            generated_tileset: Some(Handle::default()),
            generated_tileset_rows: 2,
            descriptors,
            composite_to_overlay_tile: HashMap::from([(
                vec![TransitionContribution {
                    source_tile: TILE_GRASS,
                    terrain_mask: 0,
                    void_mask: BIT_W,
                }],
                84,
            )]),
            stamp_sheets_by_terrain: HashMap::new(),
            unique_tile_index: HashMap::new(),
        };

        let mut images = Assets::<Image>::default();
        let overlay =
            resolve_overlay_for_tile(&mut world, &mut config, &mut images, IVec2::new(6, 5));

        assert_eq!(overlay, 84);
    }

    #[test]
    fn void_tile_can_receive_multiple_terrain_void_contributions() {
        let config = TerrainTransitionConfig::default();
        let neighbors = [
            TILE_EMPTY, TILE_EMPTY, TILE_EMPTY, TILE_GRASS, TILE_DIRT, TILE_EMPTY, TILE_EMPTY,
            TILE_EMPTY,
        ];

        let contributions = build_contribution_key(&config, TILE_EMPTY, &neighbors);

        assert_eq!(
            contributions,
            vec![
                TransitionContribution {
                    source_tile: TILE_GRASS,
                    terrain_mask: 0,
                    void_mask: BIT_W,
                },
                TransitionContribution {
                    source_tile: TILE_DIRT,
                    terrain_mask: 0,
                    void_mask: BIT_E,
                },
            ]
        );
    }

    #[test]
    fn terrain_tile_can_receive_multiple_higher_precedence_contributions() {
        let mut config = TerrainTransitionConfig::default();
        config.terrain_precedence = vec![TILE_GRASS, 3, TILE_DIRT];
        config.descriptors.insert(
            3,
            TerrainTransitionDescriptor {
                terrain_tile: 3,
                terrain_stamps_path: String::new(),
                terrain_stamps_origin: (0, 0),
                void_stamps_path: String::new(),
                void_stamps_origin: (0, 0),
                mask_to_overlay_tile: HashMap::new(),
            },
        );
        let neighbors = [
            TILE_EMPTY, TILE_EMPTY, TILE_EMPTY, TILE_GRASS, 3, TILE_EMPTY, TILE_EMPTY, TILE_EMPTY,
        ];

        let contributions = build_contribution_key(&config, TILE_DIRT, &neighbors);

        assert_eq!(
            contributions,
            vec![
                TransitionContribution {
                    source_tile: TILE_GRASS,
                    terrain_mask: BIT_W,
                    void_mask: 0,
                },
                TransitionContribution {
                    source_tile: 3,
                    terrain_mask: BIT_E,
                    void_mask: 0,
                },
            ]
        );
    }

    #[test]
    fn default_config_builds_runtime_tileset_from_combined_stamp_sheet() {
        if !std::path::Path::new("tmp/Biome_Transitions/Biome_Transitions.png").exists() {
            return;
        }

        let mut config = TerrainTransitionConfig::default();
        let mut images = Assets::<Image>::default();
        let handle = ensure_runtime_tileset(&mut config, &mut images).unwrap();

        assert!(images.get(&handle).is_some());
        assert!(config.generated_tileset_rows > config.base_tile_count);
        assert!(
            config.generated_tileset_rows <= 2048,
            "generated {} transition tiles",
            config.generated_tileset_rows
        );
        assert!(!config
            .descriptors
            .get(&TILE_GRASS)
            .unwrap()
            .mask_to_overlay_tile
            .is_empty());
        assert!(!config
            .descriptors
            .get(&TILE_DIRT)
            .unwrap()
            .mask_to_overlay_tile
            .is_empty());
    }
}
