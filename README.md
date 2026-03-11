# Kakukuma

Terminal-native ANSI art editor using Unicode half-block characters for 2x vertical resolution.

![Rust](https://img.shields.io/badge/Rust-2021-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

## Features

- **Half-block rendering** — Unicode block characters give 2x vertical density for detailed pixel art
- **Dynamic canvas** — 8x8 to 128x128, default 48x32
- **6 drawing tools** — Pencil, Eraser, Line, Rectangle, Fill, Eyedropper
- **Full RGB color** — 256-color palette, HSL sliders, hex input (`X`), quick pick (`1`-`0`)
- **Block character system** — full/half/quarter blocks, shades, with picker dialog (`Shift+B`) and shade cycle (`G`)
- **Command palette** — fuzzy-searchable command list via `Spacebar` or `Ctrl+P`
- **3 themes** — Warm, Neon, Dark — cycle with `Ctrl+T`
- **Symmetry** — horizontal, vertical, or both for mirrored drawing
- **Reference layer** — trace over imported images with adjustable brightness
- **Undo/redo** — full stroke-level history
- **Project files** — `.kaku` format with auto-save recovery
- **Multi-format export** — ANSI art, plain text, JSON, and PNG with configurable color depth
- **Image import & render** — load PNG/JPEG onto canvas, or render directly to ANSI art with terminal-aware color
- **WASD navigation** — keyboard-driven canvas cursor with viewport scrolling
- **CLI toolchain** — scriptable commands for batch operations, export, import, preview, and more
- **Library crate** — `use kakukuma::{Canvas, Cell, Rgb, ...}` for external consumers

## Installation

Requires [Rust](https://rustup.rs/) (2021 edition).

```bash
git clone https://github.com/0xHoneyJar/kakukuma.git
cd kakukuma
cargo build --release
```

The binary will be at `target/release/kakukuma`.

## Usage

```bash
# Start with a blank canvas
kakukuma

# Open an existing project
kakukuma myart.kaku

# Preview in terminal
kakukuma preview myart.kaku

# Export to PNG
kakukuma export myart.kaku out.png

# Export to ANSI art
kakukuma export myart.kaku out.ans

# Render an image as ANSI art (one step, auto-detects terminal colors)
kakukuma render photo.png --width 60 --height 20

# Import an image into a project file for editing
kakukuma import photo.png art.kaku

# List available block characters
kakukuma chars --plain

# Batch operations
kakukuma batch operations.json myart.kaku
```

## Keybindings

### Tools

| Key | Tool |
|-----|------|
| `P` | Pencil |
| `E` | Eraser |
| `L` | Line |
| `R` | Rectangle |
| `F` | Fill |
| `I` | Eyedropper |
| `T` | Toggle rectangle filled/outline |

### Drawing

| Key | Action |
|-----|--------|
| `B` | Cycle block character |
| `Shift+B` | Open block picker dialog |
| `G` | Cycle shade character |
| `Space` | Draw at cursor (when WASD active) / open command palette |

### Colors

| Key | Action |
|-----|--------|
| `1`-`0` | Quick select from curated palette |
| `Arrow keys` | Browse 256-color palette |
| `S` | HSL color sliders |
| `X` | Hex color input |
| `C` | Palette manager |
| `A` | Add current color to palette |
| `Right-click` | Quick eyedropper |

### Canvas

| Key | Action |
|-----|--------|
| `W/A/S/D` | Move canvas cursor |
| `H` | Toggle horizontal symmetry |
| `V` | Toggle vertical symmetry |
| `Z` | Cycle zoom (1x / 2x / 4x) |
| `Ctrl+T` | Cycle theme |
| `Ctrl+R` | Resize canvas |

### File Operations

| Key | Action |
|-----|--------|
| `Ctrl+S` | Save project |
| `Ctrl+O` | Open project |
| `Ctrl+N` | New canvas |
| `Ctrl+E` | Export dialog |
| `Ctrl+I` | Import image |
| `Ctrl+P` | Command palette |
| `Ctrl+Z` | Undo |
| `Ctrl+Y` | Redo |
| `?` | Help |
| `Q` | Quit |

## CLI Commands

| Command | Description |
|---------|-------------|
| `new` | Create a new `.kaku` project file |
| `draw` | Draw on canvas (supports `--ch` aliases like `half-top`) |
| `preview` | Render canvas to stdout (ANSI, plain, JSON) |
| `export` | Export to file (ANSI, plain, JSON, PNG) |
| `import` | Import image file onto canvas |
| `render` | Convert image to ANSI art on stdout (no intermediate file) |
| `inspect` | Query cell data at coordinates |
| `resize` | Resize canvas dimensions |
| `clear` | Reset all cells to default |
| `batch` | Execute batch operations from JSON |
| `chars` | List available block characters with metadata |
| `reference` | Set or clear reference image |
| `diff` | Compare two canvas files |
| `stats` | Canvas statistics |
| `undo` / `redo` | CLI undo/redo with operation log |
| `history` | Show operation log |
| `palette` | Palette management |

## Image to ANSI Art

Kakukuma converts images (PNG, JPEG, etc.) into terminal-displayable ANSI art using Unicode half-block characters for 2x vertical resolution.

### Quick render (one command)

```bash
# Auto-detects your terminal's color support
kakukuma render photo.png

# Control output size
kakukuma render photo.png --width 60 --height 20
```

### Import for editing

```bash
# Import into a .kaku file to edit in the TUI
kakukuma import photo.png art.kaku

# Then export when done
kakukuma export art.kaku art.ans
```

### How color works

Color format is **auto-detected** from your terminal:

| Terminal | `COLORTERM` env var | Format used |
|----------|-------------------|-------------|
| iTerm2, Kitty, Alacritty, WezTerm | `truecolor` or `24bit` | 24-bit RGB (`\e[38;2;r;g;bm`) |
| macOS Terminal.app, most others | unset or other | 256-color with hue preservation (`\e[38;5;Nm`) |

Override with `--color-format truecolor`, `--color-format 256`, `--color-format 16`, etc.

### Smart defaults

Import and render apply **brightness normalization** and **hue-preserving quantization** by default — this makes photographs look good without manual tuning. Disable with `--no-normalize` or `--no-preserve-hue` if you're working with pre-processed pixel art.

## File Formats

| Extension | Description |
|-----------|-------------|
| `.kaku` | Project file — preserves all canvas state (JSON, v1-v5 compatible) |
| `.palette` | Custom color palette (JSON, shareable) |
| `.ans` | ANSI art export (256-color or 16-color escape codes) |
| `.txt` | Plain Unicode export (blocks without color) |
| `.png` | PNG image export (configurable cell size and scale) |
| `.json` | Structured JSON export (cell-level data) |

## Architecture

```
src/
├── lib.rs          Library crate (public API)
├── main.rs         Entry point, terminal setup, CLI dispatch
├── app.rs          Application state, command palette, modes
├── canvas.rs       Dynamic-size cell grid (8-128)
├── cell.rs         Rgb color, BlockChar, Cell, CharInfo metadata
├── theme.rs        3 built-in color themes
├── tools.rs        Drawing tool implementations
├── input.rs        Keyboard and mouse event handlers
├── history.rs      Undo/redo (command pattern)
├── oplog.rs        CLI operation log
├── symmetry.rs     Mirror transformations
├── palette.rs      Curated colors, hue groups, HSL, custom palettes
├── project.rs      .kaku file save/load (v1-v5)
├── export.rs       ANSI, plain, JSON, PNG export engine
├── import.rs       Image import with quantization
├── cli/
│   ├── mod.rs          CLI argument parsing (clap)
│   ├── batch.rs        Batch operation executor
│   ├── chars.rs        Character listing command
│   ├── diff.rs         Canvas diff
│   ├── draw.rs         CLI draw with --ch alias resolution
│   ├── history_cmd.rs  History/undo/redo commands
│   ├── inspect.rs      Cell inspection
│   ├── palette_cmd.rs  Palette management
│   ├── preview.rs      Terminal preview renderer
│   └── stats.rs        Canvas statistics
└── ui/
    ├── mod.rs        Layout, dialogs, overlays
    ├── editor.rs     Canvas rendering widget (half-block + zoom)
    ├── toolbar.rs    Tool and block character panel
    ├── palette.rs    Color palette panel
    └── statusbar.rs  Bottom status bar
```

Built with [ratatui](https://github.com/ratatui/ratatui) and [crossterm](https://github.com/crossterm-rs/crossterm).

## License

[MIT](LICENSE.md)

---

Ridden with [Loa](https://github.com/0xHoneyJar/loa)
