# Argento

A black-and-white photo editor for Linux (Rust, GTK4 + libadwaita), modelled
on Silver Efex. `docs/ARCHITECTURE.md` (in Italian) is the design document:
read it before adding a feature, follow its roadmap, and update it when a
decision changes.

## Commands

```sh
cargo build                     # debug build; argento-core is still optimised
cargo run                       # start the app
cargo test                      # all crates
cargo clippy --all-targets      # must be clean
cargo fmt
```

## Layout

- `crates/argento-core`: pure processing. No GTK, no file system, no I/O.
  - `image`: linear f32 buffers (`RgbImage`, `GrayImage`), sRGB conversions,
    proxy downscaling.
  - `ops/`: one module per pipeline stage (`filter`, `tone`, `structure`).
  - `pipeline`: runs a `Recipe` with a cache per stage.
  - `recipe`, `preset`: the parameters of an edit; presets are partial
    recipes.
- `crates/argento-io`: file loading (later export, ICC, metadata, sidecars).
- `crates/argento-app`: the GTK app, binary `argento`.
  - `window`: layout, `Photos` (loading) and `Editor` (recipe changes,
    history).
  - `worker`: the preview thread, latest-request-wins.
  - `canvas`, `adjustments`, `preset_browser`, `histogram`, `history`.

## Conventions

- The pipeline works in linear f32 RGB; conversion to sRGB happens only on
  output. Tone curves operate in perceptual (sRGB-encoded) space.
- Spatial parameters (blur radii, grain) are fractions of the image's longer
  side, never pixels, so the 2048 px preview proxy matches full-resolution
  export.
- Each stage is a pure function `(input, params) -> output`. Add new stages
  to `Pipeline` with their own cache slot keyed by the params of that stage
  and all earlier ones.
- Recipe parameters are fractions: sliders show percent, the recipe stores
  percent / 100.
- Heavy work never runs on the UI thread: previews go through `Worker`,
  loading and thumbnails through `gio::spawn_blocking`. Results are tagged
  with a `PhotoId` and discarded if the photo has changed.
- UI text is in English; the architecture document is in Italian.
- Doc comments explain why, not what. Keep tests next to the code
  (`#[cfg(test)] mod tests`).

## Versions

Rust edition 2024. `gtk4` 0.11 (renamed `gtk`, feature `v4_12`),
`libadwaita` 0.9 (renamed `adw`, feature `v1_6`).

## Git

Commit only when asked. Commit messages: a short imperative subject, then a
body explaining what and why.
