# Rhythm Aim Trainer

A 3d aim trainer/game designed to allow playing osu! beatmaps in 3d.

## Important

Mouse sensitivity is in units of inches per 360 (in/360).

## Building

Build the binary using

```rust
cargo build --release
```

Omit the `--release` flag to build in debug mode.

This will create a binary in the `target/release` directory.
This needs to be run in the main directory at the same level as the assets file.

Alternatively to build and run.

```rust
cargo run --release
```

## Project Layout

`root`

- `assets`
  - Holds assets needed at runtime, audio, shaders, gltf scenes, skybox textures
- `src`
  - Game code
- `crates`
  - holds sub crates
  - allows for higher optimization level for the beatmap parser
