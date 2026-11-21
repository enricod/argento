# Argento

Black-and-white photo editing for Linux, in the spirit of Silver Efex:
ready-made presets to start from, and fine adjustments when you need them.

Argento is written in Rust with GTK4 and libadwaita. It is at an early
stage: see the [roadmap](docs/ARCHITECTURE.md#7-roadmap).

## Features

- Opens JPEG, PNG and TIFF (8 and 16 bit), with Ctrl+O or by dropping a file
  on the window.
- Colour filters (red, orange, yellow, green, blue) with adjustable strength.
- Brightness, contrast and structure (multi-scale local contrast).
- 15 built-in presets, with thumbnails rendered on the current photo.
- Undo and redo, before/after comparison, luminance histogram.
- A live preview, rendered in the background on a reduced copy of the photo.

Not there yet: saving edits next to the photo, export, colour management
(files are assumed to be sRGB).

## Building

You need Rust 1.88 or newer, and the development files of GTK 4.12 or newer
and libadwaita 1.6 or newer. On Debian and Ubuntu:

```sh
sudo apt install libgtk-4-dev libadwaita-1-dev
```

Then:

```sh
cargo run --release
```

## Keyboard shortcuts

| Shortcut | Action |
|---|---|
| Ctrl+O | Open a photo |
| Ctrl+Z | Undo |
| Ctrl+Shift+Z, Ctrl+Y | Redo |
| `\` | Show the original |

## Architecture

The code is split into three crates:

- `argento-core`: the image processing, with no dependency on GTK or the
  file system.
- `argento-io`: loading files (and later exporting, colour management,
  sidecars).
- `argento-app`: the GTK interface, the `argento` binary.

[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) (in Italian) describes the
pipeline, the concurrency model and the decisions behind them.

## License

Argento is free software, released under the
[GNU General Public License](LICENSE), version 3 or any later version.
