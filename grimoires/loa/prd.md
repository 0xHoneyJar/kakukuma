# PRD: CLI Polish & Image Export

> **Status**: Draft
> **Created**: 2026-03-01
> **Author**: AI Agent + @gumibera
> **Cycle**: 019

---

## 1. Problem Statement

During hands-on agent testing of the kakukuma CLI (cycle 018), three friction points were identified:

1. **Inconsistent CLI argument patterns**: Some commands use required named flags (`--output`, `--commands`) where positional args would be natural. The pattern varies between subcommands — `draw rect FILE FROM TO` uses positionals while `export FILE --output OUT` requires a flag for the output path. Agents must `--help` nearly every command before first use.

2. **No image export**: The export system supports plain text, ANSI, and JSON — all text formats. There's no way to produce a PNG image from canvas art. For sharing, embedding in web pages, or game asset pipelines, image export is the obvious missing piece. The `image` crate is already a dependency (used by import and reference layer).

3. **Linear undo loses data on overlapping operations**: When a `clear` operation overwrites cells from a previous `rect`, undoing the clear only restores the clear's snapshot — not the rect's state. This is standard linear undo behavior, but users (and agents) can lose work unexpectedly.

> Source: Agent CLI testing session, 2026-03-01. Friction observed during house-drawing, batch-draw, and undo/redo workflows.

---

## 2. Goals & Success Metrics

| Goal | Metric | Target |
|------|--------|--------|
| CLI consistency | Commands needing `--help` before first agent use | 0 (every command follows predictable pattern) |
| Image export | PNG export from CLI | `kakukuma export art.kaku out.png --format png` works |
| Agent efficiency | Commands with required named flags | 0 (all required args are positional) |
| Backward compat | Existing CLI scripts still work | 100% (old flag syntax accepted as aliases) |
| Test coverage | All tests pass | 356+ existing + new tests |

---

## 3. Users

### Primary: AI Agents (Claude, GPT, etc.)
- Issue CLI commands programmatically
- Parse JSON output to chain operations
- Need predictable, uniform argument patterns to minimize trial-and-error
- Use image export for visual validation and sharing results

### Secondary: Human Artists
- Use CLI for scripting and automation
- Want PNG export for sharing art on social media, embedding in projects
- Expect consistent UX across subcommands

---

## 4. Functional Requirements

### FR-1: CLI Argument Normalization

**Priority**: P0 (directly blocks agent ergonomics)

Normalize all subcommands to follow a uniform pattern:
```
kakukuma <verb> [sub-verb] <FILE> [required-positionals...] [--optional-flags...]
```

**Commands requiring changes** (required named flags → positional args):

| Command | Current | After |
|---------|---------|-------|
| `export` | `<FILE> --output <OUT> [--format F]` | `<FILE> <OUTPUT> [--format F]` |
| `import` | `<IMAGE> --output <FILE> [--width W]` | `<IMAGE> <OUTPUT> [--width W]` |
| `batch` | `<FILE> --commands <JSON> [--dry-run]` | `<FILE> <COMMANDS> [--dry-run]` |
| `palette export` | `<NAME> --output <FILE>` | `<NAME> <OUTPUT>` |

**Backward compatibility**: The old `--output` and `--commands` flags MUST still work as aliases. Clap supports this natively — mark the flag as hidden when a positional alternative exists, or use `alias`.

**Commands that are already fine** (no changes needed):
- `draw pencil/eraser/line/rect/fill/eyedropper` — FILE first, coords positional, options named
- `preview`, `inspect`, `diff`, `stats`, `undo`, `redo`, `history`, `clear`, `resize`, `new`, `reference`
- All palette subcommands except `export`

### FR-2: PNG Image Export

**Priority**: P0 (key missing capability)

Add `--format png` to the export command. Renders canvas cells as a pixel grid image.

**Rendering rules:**
- Each canvas cell maps to a fixed-size pixel block (e.g., 8x16 pixels — character aspect ratio)
- Cell size configurable via `--cell-size WxH` (default: `8x16`)
- Foreground character rendered as colored pixels against background
- For block characters (`█`, `▀`, `▄`, `▌`, `▐`, `░`, `▓`, `▒`, shade fills): render as solid/half/quarter fills using fg/bg colors
- Transparent cells (no fg, no bg): render as transparent PNG pixels (alpha=0)
- Auto-crop to bounding box (same as text export), unless `--no-crop` flag
- Output format: RGBA PNG

**Character rendering approach:**
- Block characters (full, half, quarter, shade): geometric fill — no font rendering needed
- Printable ASCII/Unicode characters: render as filled rectangle with fg color (simplified — not glyph-accurate)
- This produces "pixel art style" output, not terminal-screenshot-style

**CLI interface:**
```
kakukuma export art.kaku out.png --format png [--cell-size 8x16] [--no-crop] [--scale 2]
```

- `--scale N`: integer upscale factor (nearest-neighbor) for crisp pixel art at larger sizes
- `--no-crop`: export full canvas including empty borders

**JSON output on success:**
```json
{"exported": "out.png", "format": "png", "width": 384, "height": 512, "cell_size": "8x16"}
```

**image crate usage:**
- Already in Cargo.toml: `image = { version = "0.25", features = ["png", "jpeg", "gif", "bmp"] }`
- Need to verify `png` feature includes encoding (not just decoding)
- Use `image::RgbaImage::new(w, h)` to create output buffer
- Use `img.put_pixel(x, y, Rgba([r, g, b, a]))` to fill
- Use `img.save(path)` to write PNG

### FR-3: Undo Documentation (No Code Change)

**Priority**: P2 (nice-to-have, documentation only)

After analysis, the linear undo model is **the correct choice** for kakukuma:
- Tree-based undo adds significant complexity (branch selection UI, state management)
- The linear model matches user expectations from every mainstream editor
- The "data loss" scenario only occurs with overlapping clear operations — a rare edge case
- CLI undo is file-based with 256-entry limit — adequate for CLI workflows

**Action**: Document the undo behavior in `--help` text for `undo` and `clear` commands. Add a note that `clear` is destructive and cannot be fully reversed if it overlaps with prior operations.

---

## 5. Technical Constraints

- **Color**: All rendered colors must go through `Rgb::to_ratatui()` for terminal. PNG export uses raw RGB values directly (no terminal color constraint for image output).
- **image crate**: Already a dependency. PNG encode support needs verification.
- **Backward compatibility**: Old CLI flag syntax must continue working. No breaking changes to JSON output schemas.
- **Performance**: PNG export of a 128x128 canvas at 8x16 cell size = 1024x2048 pixels. Must complete in <1 second.
- **File detection**: When output path ends in `.png`, auto-detect format as PNG (no `--format` required).

---

## 6. Scope

### In Scope
- CLI argument normalization (4 commands + 1 palette subcommand)
- Backward-compatible flag aliases
- PNG export with block character rendering
- Cell size configuration and integer scaling
- Auto-crop for PNG export
- Undo help text improvements

### Out of Scope
- JPEG/GIF/BMP export (future cycle if needed)
- Font/glyph-accurate character rendering in PNG
- Undo tree implementation
- TUI changes (this is a CLI-only cycle)
- New CLI commands (just fixing existing ones)

---

## 7. Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| `image` crate PNG encode not included in current features | Blocks FR-2 | Check features, add `"png"` encode if needed — likely already works since import uses it |
| Breaking existing agent scripts that use `--output` flag | P0 blocker | Keep old flags as hidden aliases with clap |
| Block character rendering complexity | Delays FR-2 | Start with the ~15 common block chars only; add more incrementally |
| Cell size choice affects art quality | User confusion | Good default (8x16) and document the option |

---

## 8. Dependencies

- `image` crate v0.25 (already in Cargo.toml)
- `clap` derive macros for argument refactoring
- Existing `export.rs` auto-crop logic (reuse bounding_box calculation)
- Existing `Rgb` type for color handling
