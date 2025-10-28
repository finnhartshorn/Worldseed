# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Worldseed is a 2D game built with Bevy 0.17 using Minifantasy pixel art assets (8x8 resolution). The game features animated creatures, characters, and a tilemap-based world.

## Build & Run Commands

```bash
# Build and run the game
cargo run

# Build only (optimized for dependencies, faster dev builds)
cargo build

# Release build
cargo build --release
```

## Important: Working with Minifantasy Assets

**Asset Licensing Note:** The Minifantasy assets are NOT included in the GitHub repository due to licensing restrictions. We have a license to use them in the game, but not to redistribute them as source files. Assets must be obtained separately from https://krishna-palacio.itch.io/ and placed in the `assets/` directory structure.

**ALWAYS use the `minifantasy-assets` skill when adding new sprites or creatures.** This skill provides:
- Comprehensive documentation of all available Minifantasy asset packs
- Technical specifications (sprite dimensions, animation frame counts, grid layouts)
- Proper naming conventions and file paths
- Animation details for each creature type

To use the skill:
```
/skill minifantasy-assets
```

The skill will help you:
- Select appropriate creatures/characters from available packs
- Determine correct `TextureAtlasLayout` grid dimensions
- Understand animation frame sequences
- Locate asset files properly

## Architecture

### Single-File Structure
Currently all game logic resides in `src/main.rs`. As the project grows, consider extracting systems into separate modules.

### Core Systems

1. **Tilemap System** (`setup`, `update_tileset_image`)
   - Uses Bevy's `TilemapChunk` for grid-based terrain rendering
   - Terrain tiles are stacked vertically in source images and reinterpreted as array textures
   - Grid size: 10×10 tiles at 32×32 pixel display size
   - Tileset structure: terrain_array.png contains vertically stacked 8×8 tiles

2. **Animation System** (`animate_sprite`)
   - Component-based: `AnimationIndices` + `AnimationTimer`
   - Works with `TextureAtlas` for sprite sheet animation
   - Frame-based cycling through sprite indices
   - Timers control animation speed per entity

3. **UI System** (`setup_ui`, `button_interaction`, `guardian_button_right_click`)
   - Left-side vertical button panel using Bevy UI nodes
   - Buttons display creature sprites with custom offsets for proper centering
   - Guardian button has expandable submenu showing 5 guardian variants
   - UI sprites require vertical offset constants (see `*_SPRITE_OFFSET` constants)

4. **Chunk Loading System** (`world/loader.rs`)
   - Dynamically loads/unloads terrain chunks based on camera position
   - Load radius: 3 chunks (7×7 grid = 49 chunks) defined in `CHUNK_LOAD_RADIUS`
   - Unload radius: 5 chunks (11×11 grid = 121 chunks) defined in `CHUNK_UNLOAD_RADIUS`
   - **Hysteresis design**: Unload radius is +2 above load radius to prevent chunk thrashing
   - Prevents repeated load/unload cycles when camera moves back and forth near chunk boundaries
   - Chunks are serialized to disk when unloaded if marked dirty
   - Constants defined in `src/tiles/constants.rs`

### Entity Organization

**Marker Components**: Used to identify entity types
- `ForestGuardian` - Oak guardian creature
- `Snail` - Snail creature
- `GuardianSubmenu` - UI submenu container
- `GuardianButton` - Guardian selection button

### Asset Structure

```
assets/
├── characters/        # Player character sprites
│   ├── human_walk.png (32×32 frames, 4×4 grid)
│   └── mage_walk.png
├── creatures/
│   ├── forest_guardians/  # 5 variants: oak, birch, hickory, pine, willow
│   │   └── *_guardian_idle.png (32×32 frames, 8×4 grid)
│   ├── snail/
│   │   └── snail_crawl.png (32×32 frames, 4×4 grid)
│   └── tree_spirits/
└── tilesets/
    └── terrain_array.png (8×16 stacked tiles)
```

### Sprite Sheet Specifications

- **Source resolution**: 8×8 pixels (Minifantasy standard)
- **Display size**: 32×32 pixels (4× scale)
- **Animation layout**: Frames organized in grids (rows = directions, columns = frames)
- **Directions**: 4-directional (NW-NE-SW-SE) in rows

### Camera & Rendering

- Single `Camera2d` at world origin
- Nearest-neighbor filtering via `ImagePlugin::default_nearest()` for pixel art
- Z-ordering: tilemap at 0.0, sprites at 1.0+

## Development Patterns

### Adding New Creatures

1. **First, invoke the `minifantasy-assets` skill** to find appropriate sprites and get technical specs
2. Load texture and create `TextureAtlasLayout` with correct grid dimensions (from skill documentation)
3. Add marker component (e.g., `#[derive(Component)] struct NewCreature;`)
4. Spawn entity with `Sprite::from_atlas_image`, `AnimationIndices`, and `AnimationTimer`
5. Add to animation query in `animate_sprite` (already handles all sprites)

### UI Sprite Centering

Minifantasy sprites have varying vertical centering. Add offset constants when sprites appear off-center in UI:
```rust
const CREATURE_SPRITE_OFFSET: f32 = 10.0; // Adjust per sprite
```

### Tilemap Modification

- Tile data stored as `Vec<Option<TileData>>` in `TilemapChunkTileData`
- Access tiles by `(y * chunk_width) + x`
- Use `TileData::from_tileset_index(n)` to reference array texture layers

## Bevy 0.17 Specifics

- Uses new `sprite_render` module for tilemaps
- Observer pattern for UI interactions (`observe` method)
- `MessageReader` for asset events (replaces `EventReader` for some types)
- `Single<T>` query for single-entity queries

## Asset References

The `minifantasy-assets` skill includes a comprehensive master list of all available creatures (`references/minifantasy_creatures_master_list.md`) with animation specifications. All assets maintain 8×8 pixel base resolution with 4-directional animations.
