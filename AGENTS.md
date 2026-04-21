# AGENTS.md

Guidance for coding agents working in this repository. Keep this file focused on architecture, invariants, and common edit paths. Prefer reading the code for exact signatures.

## Project

Worldseed is a 2D Bevy 0.18.1 game using Minifantasy-style pixel art assets. The game has app-state-driven menus, a draft setup flow, animated world entities, dynamic chunk loading, terrain editing, autosaves, and a sampled minimap.

## Commands

```bash
cargo run
cargo build
cargo test
cargo build --release

# Chunk loading/debug output
cargo run --features debug_chunks
cargo build --features debug_chunks
```

## Asset Rules

Minifantasy assets are licensed separately and are not redistributed in the repository. They must be placed under `assets/` locally.

When adding or changing sprites:
- Use the `minifantasy-assets` skill if it is available in the current agent environment.
- If the skill is unavailable, inspect existing asset paths and atlas helpers before guessing dimensions.
- Use `tools/sprite_bounds.py` for padded or uncertain sheets. Prefer measured `URect`/`Rect` values over hand-measuring.
- Keep pixel art sampling nearest-neighbor and preserve the 8x8 source, 32x32 display convention unless the surrounding code intentionally differs.

Common sprite-bounds commands:

```bash
python3 tools/sprite_bounds.py assets/path/to/sheet.png --frame-width 32 --frame-height 32
python3 tools/sprite_bounds.py assets/path/to/sheet.png --frame-width 32 --frame-height 32 --stable --per-row
python3 tools/sprite_bounds.py assets/path/to/sheet.png --frame-width 32 --frame-height 32 --stable --emit-bevy-helper build_creature_layout
```

## Architecture

### Modules

- `src/main.rs` - app states, menus, draft setup, in-game UI, camera controls, saving orchestration
- `src/entities/` - components, spawning helpers, animation, AI behavior, entity/world interactions
- `src/world/` - chunk loading/unloading, world generation, terrain transitions, serialization, save metadata, entity saves
- `src/tiles/` - chunk data structures, constants, tile IDs, coordinate helpers
- `src/map/` - map modal and sampled minimap rendering
- `src/draft/` - draft cards and setup-board metadata

### App Flow

`AppState` in `src/main.rs` drives the application:

`MainMenu` -> `LoadSlotSelect` -> `SlotHub` -> `LoadWorldSelect` or `DraftSetup` -> `InGame`

State-specific setup and cleanup use `OnEnter`, `OnExit`, and `run_if(in_state(...))`. `setup_world`, `setup_ui`, and save notification UI are only created when entering `InGame`; cleanup removes gameplay cameras/UI/entities when leaving.

### World And Chunks

The world uses an explicit model/view split:

- Model: `WorldManager.chunk_cache` stores authoritative `ChunkData`.
- View: Bevy `TilemapChunk` entities are spawned/despawned around the camera.
- Chunk data persists in cache after visual chunks unload.
- Tile edits must go through `WorldManager.queue_tile_modification(x, y, tile_id, layer)`.
- Tile modifications work even for unrendered chunks because the cache is updated first.
- Ground-layer edits refresh terrain-transition overlays when transitions are enabled.

Each chunk has three tile layers: `LAYER_GROUND`, `LAYER_DECORATION`, and `LAYER_OVERLAY`. Each visible chunk position spawns one `TilemapChunk` entity per layer. Terrain textures are stacked vertically in `assets/tilesets/terrain_array.png` and loaded as array textures.

Chunk loading is camera/zoom aware. Camera movement updates the current chunk first, then loader systems load nearby chunks and unload distant visual chunks with hysteresis.

### Entities

Core entity logic uses `Position` for game state and `Transform` for rendering. `sync_world_render_transform` copies `Position` to `Transform` and applies deterministic y-sorted depth through `WorldRenderDepth`.

Common components live in `src/entities/types.rs`:

- `Position`, `Velocity`, `Direction`, `EntityState`, `Health`, `EntityBundle`
- `WorldRenderDepth`, `WorldClickableEntity`
- `Human`, `ForestGuardian`, `Snail`, `RtsTree`, `TreeSpirit`, `VariantTree`
- `GrowingTree`, `GrowthStage`, `TreeVariant`, `TreeSpawner`
- behavior components such as `RoamingBehavior` and `WindingPath`

Spawning helpers live in `src/entities/spawning.rs`; behavior and animation systems live in `src/entities/systems.rs`.

Important ordering invariants:

- AI behavior sets velocity before velocity is applied.
- Velocity changes position before entity state/direction/animation update.
- Position/depth sync runs before entity-world interactions such as snail dirt trails.
- Entity state/direction updates happen before sprite animation frame cycling.

### Rendering

- `ImagePlugin::default_nearest()` is used for pixel art.
- Tile layers render at z `0.0`, `0.1`, and `0.2`.
- World sprites use `WorldRenderDepth`: `z = stratum_base - y * 0.00001 + bias`.
- Render strata bases are `Ground` 0.0, `Decoration` 10.0, `WorldObject` 20.0, and `Overlay` 30.0.
- Use `Anchor::BOTTOM_CENTER` for world sprites so y-sorting tracks the footpoint.
- Use deterministic depth biases for same-footpoint ties instead of relying on spawn order.

### UI And Input

In-game UI is mostly in `src/main.rs`.

- `PlacementMode`, `PaintMode`, and `BloomSelection` are mutually exclusive.
- Entity placement spends `Bloom`; insufficient bloom prevents spawning.
- Terrain painting queues ground-layer tile modifications.
- World click placement/painting uses `camera.viewport_to_world_2d(...)`.
- The escape menu gates most simulation, placement, chunk loading, saving, and debug systems via `escape_menu_closed`.
- Time controls update `Time<Virtual>` through `SimulationControlState`.

Repo-specific Bevy UI notes:

- UI interaction uses observer handlers via `.observe(...)`.
- Use `bevy::picking::pointer::PointerButton` to distinguish primary/secondary clicks.
- `ImageNode.texture_atlas` holds UI texture atlases.
- `ParamSet` is used where a system needs conflicting immutable/mutable queries for the same component.

### Saves

Save data is organized as:

```text
saves/
  <slot>/meta.bin
  <slot>/worlds/<world>/meta.bin
  <slot>/worlds/<world>/world/chunks/chunk_X_Y.bin
  <slot>/worlds/<world>/entities.bin
```

Persisted world metadata includes camera state, bloom, and generation config. Slot metadata tracks draft progression through `available_columns`.

Persisted entities currently include humans, forest guardians, snails, tree spirits, and variant trees. If a new entity type must survive reloads, update `src/world/entities_save.rs` and the save/restore orchestration in `src/main.rs`.

Chunk saves use the v2 multi-layer binary format with checksum validation and v1 single-layer compatibility.

### Map

`MapPlugin` owns the map modal. Press `M` to show/hide it. The minimap renders colored UI pixels by sampling loaded chunk data from the ground layer. While open, `=` and `-` cycle the sample size.

## Common Tasks

### Adding An Entity Type

1. Use `minifantasy-assets` if available, or inspect existing assets and run `tools/sprite_bounds.py`.
2. Add marker/data components in `src/entities/types.rs`.
3. Add a spawn helper in `src/entities/spawning.rs`.
4. Register UI placement and bloom cost in `src/main.rs` if the entity is player-placeable.
5. Add persistence support if the entity should be saved.

Spawn helpers should follow the existing world-sprite pattern:

```rust
commands.spawn((
    NewCreature,
    WorldClickableEntity,
    EntityBundle::new(position.x, position.y, max_health),
    WorldRenderDepth::new(RenderStratum::WorldObject),
    Pickable::default(),
    Sprite::from_atlas_image(texture, TextureAtlas { layout, index: 0 }),
    Anchor::BOTTOM_CENTER,
    Transform::from_xyz(position.x, position.y, 0.0),
    AnimationIndices::new(first, last),
    AnimationTimer::from_fps(fps),
));
```

The shared entity pipeline handles position sync, state changes from velocity, direction changes, y-sorted depth, and direction-aware animation.

### Adding Or Changing Sprites

- Prefer existing atlas helper functions in `src/entities/spawning.rs`.
- Use crop layouts when a sheet has padding instead of assuming full 32x32 cells.
- Guardian idle and walk sheets have different frame counts; keep `GuardianAnimations` in sync.
- Variant-tree asset paths and frame sizes are defined by `TreeVariant` helpers in `src/entities/types.rs`.
- UI terrain uses `assets/tilesets/terrain_array_ui.png`; world tilemaps use `assets/tilesets/terrain_array.png`.

### Modifying Terrain

Use `WorldManager.queue_tile_modification`; do not mutate tilemap visuals directly.

```rust
world.queue_tile_modification(position.x, position.y, TILE_DIRT, LAYER_GROUND);
```

`apply_tile_modifications` converts world coordinates to chunk/local tile coordinates, updates cached `ChunkData`, syncs visible tilemap data if present, marks chunks dirty, and refreshes terrain transitions around ground edits.

### Updating Placement Or Painting UI

- Entity buttons use `EntityType`; terrain buttons use `TerrainType`.
- Selecting placement clears painting and bloom selection; selecting terrain clears placement and bloom selection.
- Placement spends bloom through `EntityType::bloom_cost()`.
- Right-click submenus update the main button icon by changing `ImageNode.image` and atlas state.
- Keep UI sprite centering offsets near the existing `*_SPRITE_OFFSET` constants.

### Adding Persisted State

Decide which persistence layer owns the data:

- Chunk/tile data: `src/world/serialization.rs`
- Slot/world metadata: `src/world/savegame.rs`
- Runtime world entities: `src/world/entities_save.rs`

When changing save formats, preserve legacy decode paths where existing saves might break.

### Working On Terrain Generation

`WorldGenerationConfig` controls shape, element, power, and seed. `WorldShape::Island` uses pregeneration and lazy missing chunks are empty; `WorldShape::Infinity` generates chunks on demand. Keep generation deterministic for a given seed/config.

### Working On The Map

Map rendering samples `WorldManager.chunk_cache`; it does not use a cartographic tileset. To add terrain colors, update map constants and the sample/color logic in `src/map/systems.rs`.

## Final Checks

Before finishing code changes, run the narrowest relevant check. Prefer `cargo test` for shared behavior, save/load changes, terrain generation, chunk management, and entity systems. Use `cargo build` for compile-only UI or asset-path changes when tests are not relevant.

`AGENTS.md` and `CLAUDE.md` are intended to be hard links. If you edit one with tooling that rewrites files, verify and restore the link:

```bash
ls -li AGENTS.md CLAUDE.md
ln -f AGENTS.md CLAUDE.md
```
