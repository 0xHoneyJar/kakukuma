# Kakukuma ʕ•ᴥ•ʔ

Terminal-native ANSI art editor using Unicode half-block characters.

![Rust](https://img.shields.io/badge/Rust-2021-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

## Features

- **Dynamic canvas** — resizable from 8x8 to 128x128 with half-block rendering
- **6 drawing tools**: Pencil, Eraser, Line, Rectangle, Fill, Eyedropper
- **256-color support** with curated 24-color palette and full xterm-256 browser
- **3 built-in themes** — Warm, Neon, Dark — cycle with `Ctrl+T`
- **HSL color sliders** for precise color picking
- **Custom palettes** — create, save, load, and share `.palette` files
- **Symmetry modes** — horizontal, vertical, or both for mirrored drawing
- **Undo/redo** with full stroke-level history
- **Project files** — save/load `.kaku` files with auto-save recovery
- **Export** — ANSI art to clipboard or file, with optional plain Unicode export
- **Mouse support** — click and drag to draw, right-click to eyedrop

## Installation

Requires [Rust](https://rustup.rs/) (2021 edition).

```bash
git clone https://github.com/0xhoneyjar/kakukuma.git
cd kakukuma
cargo build --release
```

The binary will be at `target/release/kakukuma`.

## Usage

```bash
# Start with a blank canvas
cargo run

# Open an existing project
cargo run -- myart.kaku
```

## Keybindings

### Tools

| Key | Tool |
|-----|------|
| `P` | Pencil — draw single cells |
| `E` | Eraser — clear cells |
| `L` | Line — click start, click end |
| `R` | Rectangle — click corner, click opposite corner |
| `F` | Fill — flood fill from click point |
| `I` | Eyedropper — pick color from canvas |
| `B` | Cycle block character (full, upper half, lower half, left half, right half) |
| `T` | Toggle rectangle filled/outline |

### Colors

| Key | Action |
|-----|--------|
| `1`-`0` | Quick select from curated palette |
| `Arrow keys` | Browse full 256-color palette |
| `S` | Open HSL color sliders |
| `C` | Open custom palette dialog |
| `A` | Add current color to active palette |
| `Right-click` | Quick eyedropper |

### Canvas

| Key | Action |
|-----|--------|
| `H` | Toggle horizontal symmetry |
| `V` | Toggle vertical symmetry |
| `Tab` | Toggle preview mode |
| `Ctrl+T` | Cycle theme (Warm / Neon / Dark) |

### File Operations

| Key | Action |
|-----|--------|
| `Ctrl+S` | Save project |
| `Ctrl+O` | Open project |
| `Ctrl+N` | New canvas (choose dimensions) |
| `Ctrl+E` | Export dialog |
| `Ctrl+Z` | Undo |
| `Ctrl+Y` | Redo |
| `Q` | Quit |
| `?` | Help |

## File Formats

| Extension | Description |
|-----------|-------------|
| `.kaku` | Project file (JSON, preserves all state) |
| `.palette` | Custom color palette (JSON, shareable) |
| `.txt` | Plain Unicode export (blocks without color) |
| `.ans` | ANSI art export (256-color escape codes) |

## Architecture

```
src/
├── main.rs        Entry point, terminal setup
├── app.rs         Application state and logic
├── canvas.rs      Dynamic-size cell grid (8-128)
├── cell.rs        Color256 type, BlockChar, Cell
├── theme.rs       3 built-in color themes
├── tools.rs       Drawing tool implementations
├── input.rs       Keyboard and mouse handlers
├── history.rs     Undo/redo (command pattern)
├── symmetry.rs    Mirror transformations
├── palette.rs     Curated colors, hue groups, HSL, custom palettes
├── project.rs     .kaku file save/load (v1-v3)
├── export.rs      Plain Unicode and ANSI art export
└── ui/
    ├── mod.rs       Layout, dialogs, header
    ├── editor.rs    Canvas rendering widget (half-block)
    ├── toolbar.rs   Tool list panel
    ├── palette.rs   Color palette panel
    └── statusbar.rs Bottom status bar
```

Built with [ratatui](https://github.com/ratatui/ratatui) and [crossterm](https://github.com/crossterm-rs/crossterm).

## License

[MIT](LICENSE.md)

---

Ridden with [Loa](https://github.com/0xHoneyJar/loa)
