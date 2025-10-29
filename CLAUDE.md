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

### Module Structure
The codebase is organized into modules:
- `src/main.rs` - Main entry point, UI systems, camera controls
- `src/entities/` - Entity system, components, spawning, and behavior systems
- `src/world/` - World management, chunk loading/unloading, generation, serialization
- `src/tiles/` - Tile system, chunk data structures, constants, registry

### Core Systems

1. **Tilemap System** (`setup_world`, `update_tileset_image`)
   - Uses Bevy's `TilemapChunk` for grid-based terrain rendering
   - Terrain tiles are stacked vertically in source images and reinterpreted as array textures
   - Chunk size: 32×32 tiles at 32×32 pixel display size (1,024 tiles per chunk)
   - Source tile resolution: 8×8 pixels (scaled 4× for display)
   - Tileset structure: terrain_array.png contains vertically stacked 8×8 tiles

2. **Entity System** (`entities/` module)
   - **Components** (`types.rs`):
     - `Position` - World position in pixels (separate from Transform for game logic)
     - `Velocity` - Movement speed in pixels per second
     - `Direction` - Four-directional facing (NW, NE, SW, SE) maps to sprite sheet rows
     - `EntityState` - State machine (Idle, Moving, Attacking, Dead)
     - `Health` - Health tracking with damage/heal methods
     - `EntityBundle` - Convenient bundle with Position, Velocity, Direction, EntityState, Health
     - Marker components: `Player`, `ForestGuardian`, `Snail`
   - **Spawning** (`spawning.rs`):
     - `spawn_player()`, `spawn_forest_guardian()`, `spawn_snail()` - Entity spawning functions
     - `AnimationIndices` - First and last frame indices for animation loops
     - `AnimationTimer` - Controls animation speed (supports FPS or duration)
   - **Systems** (`systems.rs`):
     - `apply_velocity` - Applies velocity to position each frame
     - `update_state_from_velocity` - Auto-transitions between Idle/Moving states
     - `update_direction_from_velocity` - Updates facing direction from movement
     - `update_animation_from_direction` - Selects correct sprite sheet row for direction
     - `sync_position_with_transform` - Syncs Position component to Transform for rendering
     - `animate_sprite` - Cycles through animation frames based on timers
     - `snail_dirt_trail` - Makes snails turn tiles into dirt with 20% chance as they move
     - `update_roaming_behavior` - Updates entities with RoamingBehavior component
     - `update_winding_path` - Updates entities with WindingPath component

3. **Animation System**
   - Component-based: `AnimationIndices` + `AnimationTimer` (defined in `entities/spawning.rs`)
   - Works with `TextureAtlas` for sprite sheet animation
   - Frame-based cycling through sprite indices
   - Direction-aware: automatically uses correct sprite sheet row based on entity facing
   - Timers control animation speed per entity
   - Generic system handles all animated sprites automatically

4. **UI System** (`setup_ui`, `button_interaction`, `guardian_button_right_click`)
   - Left-side vertical button panel using Bevy UI nodes
   - Buttons display creature sprites with custom offsets for proper centering
   - Guardian button has expandable submenu showing 5 guardian variants
   - UI sprites require vertical offset constants (see `*_SPRITE_OFFSET` constants)

5. **World Management System** (`world/` module)
   - `loader.rs` - Dynamic chunk loading/unloading based on camera position and zoom
   - `manager.rs` - WorldManager resource, tracks loaded chunks and statistics
   - `generator.rs` - Procedural terrain generation
   - `serialization.rs` - Chunk persistence to disk
   - **Zoom-aware loading**: Load/unload radii automatically adjust based on camera zoom level
   - Load radius: Calculated from visible viewport + 2 chunk buffer (minimum 3 chunks)
   - Unload radius: Load radius + 2 chunks (hysteresis buffer)
   - **Hysteresis design**: +2 chunk buffer prevents thrashing
   - Prevents repeated load/unload cycles when camera moves back and forth near chunk boundaries
   - When zoomed out, more chunks load to cover larger visible area
   - When zoomed in, fewer chunks load since less area is visible
   - Chunks serialize when unloaded if dirty
   - Base constants defined in `src/tiles/constants.rs` (used as minimums)
   - **Tile Modification System**: Entities can modify world tiles dynamically
     - `TileModification` - Queued tile change requests (world position + tile ID)
     - `queue_tile_modification()` - Queue a tile change for processing
     - `apply_tile_modifications` system - Applies queued changes to both cache and visual tilemap
     - Changes are marked dirty for automatic serialization

6. **Camera System** (`move_camera`, `zoom_camera`)
   - Keyboard movement (WASD/Arrow keys) at 200 pixels/second
   - Zoom via mouse wheel or keyboard (-/= keys)
   - Zoom range: 0.5× (max zoom in) to 3.0× (max zoom out)
   - Camera position and zoom level both drive chunk loading/unloading
   - Zoom level dynamically adjusts how many chunks are loaded (more when zoomed out, fewer when zoomed in)

### System Ordering

Update systems run in this order:
1. `update_tileset_image` - Process texture assets
2. **AI behaviors** (before velocity application):
   - `update_roaming_behavior` - Updates roaming entities
   - `update_winding_path` - Updates winding path entities
3. **Entity state pipeline:**
   - `apply_velocity` - Update positions from velocity
   - `update_state_from_velocity` - Update entity states (Idle/Moving)
   - `update_direction_from_velocity` - Update facing direction
   - `update_animation_from_direction` - Update sprite row for direction
   - `sync_position_with_transform` - Sync Position to Transform (after velocity)
4. **Entity-world interactions:**
   - `snail_dirt_trail` - Snails modify tiles as they move (after position sync)
5. `animate_sprite` - Cycle through animation frames
6. `move_camera` - Handle camera movement input
7. `zoom_camera` - Handle zoom input
8. `update_camera_chunk` - Track which chunk camera is in
9. `load_chunks_around_camera` - Load chunks in radius (after camera update)
10. `unload_distant_chunks` - Unload far chunks (after loading)
11. `apply_tile_modifications` - Apply queued tile changes to cache and visuals

**Critical orderings:**
- AI behaviors run before velocity application to set movement intent
- Entity state pipeline must run before animation to ensure correct sprite rows
- `sync_position_with_transform` must run after `apply_velocity` to reflect position changes
- Entity-world interactions run after position sync to use updated positions
- `apply_tile_modifications` runs after all tile changes are queued
- Camera updates before chunk loading to ensure correct chunks are loaded

### Resources

**WorldManager** (`world/manager.rs`)
- Tracks all loaded chunks by ChunkPos
- Maintains WorldStats (total chunks, loaded count, etc.)
- Initialized at startup with `init_resource::<WorldManager>()`
- Used by loader systems to coordinate chunk lifecycle
- Manages tile modification queue via `queue_tile_modification()` and `take_tile_modifications()`
- Tile changes update both cached `ChunkData` and visual `TilemapChunkTileData`

### Entity Organization

**Core Entity Components** (in `entities/types.rs`):
- `Position` - World position (separate from Transform for clean game logic)
- `Velocity` - Movement speed
- `Direction` - Facing direction (NW, NE, SW, SE)
- `EntityState` - State machine (Idle, Moving, Attacking, Dead)
- `Health` - Health tracking

**Marker Components**:
- Entity types (in `entities/types.rs`): `Player`, `ForestGuardian`, `Snail`
- UI components (in `main.rs`): `GuardianSubmenu`, `GuardianButton`

**Key Design Principles:**
- `Position` is separate from `Transform` - Position is for game logic, Transform is for rendering
- Entities automatically transition states based on velocity (via `update_state_from_velocity`)
- Direction automatically updates from velocity and controls sprite sheet row selection
- Animation system is direction-aware and handles all sprite types generically

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

- Single `Camera2d` spawned at world origin (0, 0, 999)
- Nearest-neighbor filtering via `ImagePlugin::default_nearest()` for pixel art
- Z-ordering: tilemap at 0.0, sprites at 1.0+

**Camera Controls** (for testing and navigation):
- **Movement**: WASD or Arrow Keys (200 pixels/second)
- **Zoom In**: Mouse scroll up or Equals (=) key
- **Zoom Out**: Mouse scroll down or Minus (-) key
- **Zoom Range**: 0.5× (max zoom in) to 3.0× (max zoom out)
- Camera position and zoom level determine which chunks load/unload
- Zooming out increases visible area and automatically loads more chunks
- Zooming in decreases visible area and allows distant chunks to unload

## Development Patterns

### Adding New Entity Types

1. **First, invoke the `minifantasy-assets` skill** to find appropriate sprites and get technical specs
2. **Add marker component** in `src/entities/types.rs`:
   ```rust
   #[derive(Component)]
   pub struct NewCreature;
   ```
3. **Create spawning function** in `src/entities/spawning.rs`:
   ```rust
   pub fn spawn_new_creature(
       commands: &mut Commands,
       position: Position,
       assets: &Res<AssetServer>,
       texture_atlas_layouts: &mut ResMut<Assets<TextureAtlasLayout>>,
   ) -> Entity {
       let texture = assets.load("path/to/creature.png");
       let layout = TextureAtlasLayout::from_grid(UVec2::splat(32), cols, rows, None, None);
       let texture_atlas_layout = texture_atlas_layouts.add(layout);

       commands.spawn((
           NewCreature,
           EntityBundle::new(position.x, position.y, max_health),
           Sprite::from_atlas_image(texture, TextureAtlas { layout: texture_atlas_layout, index: 0 }),
           Transform::from_xyz(position.x, position.y, 1.0),
           AnimationIndices::new(first, last),
           AnimationTimer::from_fps(fps),
       )).id()
   }
   ```
4. **The entity system handles everything automatically:**
   - Position syncing to Transform
   - State management from velocity
   - Direction updates from movement
   - Direction-aware animation
   - No additional systems needed!

### Spawning Entities

Use the spawning functions from `entities/spawning.rs`:
```rust
// In setup or spawn systems:
spawn_player(&mut commands, Position::new(0.0, 0.0), &assets, &mut texture_atlas_layouts);
spawn_forest_guardian(&mut commands, Position::new(-100.0, 0.0), "oak", &assets, &mut texture_atlas_layouts);
spawn_snail(&mut commands, Position::new(100.0, 0.0), &assets, &mut texture_atlas_layouts);
```

### Controlling Entities

Modify entity components to affect behavior:
```rust
// Make entity move
fn control_entity(mut query: Query<&mut Velocity, With<Player>>) {
    for mut velocity in &mut query {
        velocity.x = 50.0; // Move right at 50 pixels/second
        velocity.y = 0.0;
        // Direction, state, and animation update automatically!
    }
}

// Check entity state
fn check_entity(query: Query<(&Position, &EntityState, &Health)>) {
    for (pos, state, health) in &query {
        if health.current < 50.0 && *state != EntityState::Dead {
            // Handle low health
        }
    }
}
```

### UI Sprite Centering

Minifantasy sprites have varying vertical centering. Add offset constants when sprites appear off-center in UI:
```rust
const CREATURE_SPRITE_OFFSET: f32 = 10.0; // Adjust per sprite
```

### Tilemap Modification

Entities can modify world tiles dynamically through the WorldManager:

```rust
// In a system that modifies tiles:
fn my_tile_modifier(
    mut world: ResMut<WorldManager>,
    query: Query<&Position, With<MyEntity>>,
) {
    for position in query.iter() {
        // Queue a tile modification at the entity's position
        world.queue_tile_modification(position.x, position.y, TILE_DIRT);
    }
}
```

**How it works:**
1. Call `world.queue_tile_modification(x, y, tile_id)` to queue changes
2. The `apply_tile_modifications` system processes all queued changes:
   - Converts world position to chunk coordinates
   - Updates cached `ChunkData` (for persistence)
   - Updates visual `TilemapChunkTileData` (for rendering)
   - Marks chunks as dirty for automatic saving
3. Tile constants are defined in `src/tiles/constants.rs`:
   - `TILE_EMPTY` (0) - Air/no tile
   - `TILE_GRASS` (1) - Grass terrain
   - `TILE_DIRT` (2) - Dirt terrain

**Example: Snail Dirt Trail**
The snail leaves dirt trails with a 20% chance as it moves:
- Uses `Changed<Position>` to detect movement
- Generates pseudo-random value from position hash
- Queues `TILE_DIRT` modification at current position
- Changes persist through chunk unload/reload cycles

## Bevy 0.17 Specifics

- Uses new `sprite_render` module for tilemaps (`TilemapChunk`, `TilemapChunkTileData`, `TileData`)
- Observer pattern for UI interactions (`observe` method on entities)
- `MessageReader` for asset events and input (replaces `EventReader` for `AssetEvent<T>` and `MouseWheel`)
- `Single<T>` query for single-entity queries (cleaner than `Query<T, With<...>>.single_mut()`)
- `ImageNode` component for UI sprites (replaces old `UiImage`)
- `Node` component for UI layout (replaces old `Style`)

## Asset References

The `minifantasy-assets` skill includes a comprehensive master list of all available creatures (`references/minifantasy_creatures_master_list.md`) with animation specifications. All assets maintain 8×8 pixel base resolution with 4-directional animations.
