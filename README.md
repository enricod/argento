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
- Non-destructive: edits are saved automatically in a small text file next
  to the photo (`photo.jpg.argento`), and the photo itself is never
  modified. Opening the photo again brings the edit back.
- Export at full resolution as a 16-bit TIFF next to the original
  (`photo.jpg` → `photo-bw.tif`).

Not there yet: colour management (files are assumed to be sRGB), copying
EXIF metadata on export.

## Building

You need Rust 1.88 or newer, and the development files of GTK 4.12 or newer
and libadwaita 1.8 or newer. On Debian and Ubuntu:

```sh
sudo apt install libgtk-4-dev libadwaita-1-dev
```

Then:

```sh
cargo run --release
```

### Flatpak

The manifest is `build-aux/flatpak/io.github.enricod.Argento.yml` (GNOME 51
runtime). You need `flatpak-builder` and the Flathub remote:

```sh
make flatpak                 # build and install for the current user
flatpak run io.github.enricod.Argento
make flatpak-bundle          # argento.flatpak, to install on another computer
```

Crates are vendored from `build-aux/flatpak/cargo-sources.json`, since the build
has no network: run `make flatpak-sources` whenever `Cargo.lock` changes.

The Flatpak has access to the home folder and removable media, because
Argento writes its sidecar and the exported TIFF next to each photo.

## Keyboard shortcuts

| Shortcut | Action |
|---|---|
| Ctrl+O | Open a photo |
| Ctrl+E | Export |
| Ctrl+Z | Undo |
| Ctrl+Shift+Z, Ctrl+Y | Redo |
| `\` | Show the original |
| Ctrl+? | Keyboard shortcuts |
| Ctrl+Q | Quit |

## Architecture

The code is split into three crates:

- `argento-core`: the image processing, with no dependency on GTK or the
  file system.
- `argento-io`: loading and exporting files, sidecars (and later colour
  management and metadata).
- `argento-app`: the GTK interface, the `argento` binary.

[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) (in Italian) describes the
pipeline, the concurrency model and the decisions behind them.

## License

Argento is free software, released under the
[GNU General Public License](LICENSE), version 3 or any later version.
