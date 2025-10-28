# Worldseed

A 2D game built with Bevy 0.17 using Minifantasy pixel art assets.

## About

Worldseed is a tile-based game featuring animated creatures, characters, and a pixel art world. The game uses Bevy's ECS architecture and sprite rendering systems to bring the world to life.

## Asset Licensing

**Note:** The game assets (sprites, tilesets, etc.) are **not included in this repository** due to licensing restrictions. While we have a license to use these assets in the game, we cannot redistribute them as source files, which uploading to GitHub would constitute.

### Credits

All pixel art assets are from the **Minifantasy** collection by **Krishna Palacio**.

- **Artist:** Krishna Palacio
- **Collection:** Minifantasy
- **Website:** https://krishna-palacio.itch.io/
- **License:** Assets are licensed for use in games but not for redistribution

If you wish to run this project, you will need to purchase and download the Minifantasy asset packs separately and place them in the `assets/` directory according to the structure defined in `CLAUDE.md`.

## Build & Run

```bash
# Build and run the game
cargo run

# Build only
cargo build

# Release build
cargo build --release
```

## Requirements

- Rust (latest stable)
- Bevy 0.17
- Minifantasy asset packs (not included, see Asset Licensing above)

## Project Structure

See `CLAUDE.md` for detailed development documentation, architecture overview, and asset structure.

## License

Code: [Choose your license]
Assets: Minifantasy by Krishna Palacio (not included in repository)
