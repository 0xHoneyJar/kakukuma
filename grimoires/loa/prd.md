# PRD: Import Fidelity & Color Intelligence

> **Status**: Approved
> **Created**: 2026-03-06
> **Updated**: 2026-03-14 (expanded to cover clipboard import, perceptual quantization, UI improvements)
> **Author**: Loa (AI Agent) + @gumibera
> **Cycle**: 022

---

## 1. Problem Statement

Kakukuma can import images and convert them to ANSI art, but the results require expert-level flag tuning to look good — and even then, the output may display incorrectly depending on terminal color support. The TUI import workflow is also clunky: no clipboard support, slow file navigation, and no way to control preprocessing. This is unacceptable for a tool that serves both human artists and AI agents.

Five concrete problems:

### 1.1 Import Requires Expert Knowledge

Converting an image to readable ANSI art currently requires this incantation:

```bash
kakukuma import photo.png art.kaku --width 40 --height 14 --quantize truecolor --normalize --preserve-hue
kakukuma export art.kaku art.ans --format ansi --color-format 256-hue
```

A user (or agent) who just runs `kakukuma import photo.png art.kaku` gets 256-color quantization without normalize or hue-preservation — producing muddy, gray output for most photographs. The defaults are wrong for the common case.

> Source: Direct observation in cycle-020 session. Every test image required manual flag tuning.

### 1.2 Terminal Color Mismatch

The app has no way to detect terminal color capabilities. A user who exports with `--color-format truecolor` and views on macOS Terminal.app sees garbled white text because Terminal.app doesn't support `\e[38;2;r;g;bm`. There's no warning, no fallback, and the `256-hue` mode that would fix it is undiscoverable.

> Source: `grimoires/loa/context/image-to-ansi-art.md` — truecolor renders as white on non-truecolor terminals

### 1.3 No ANSI Preview Command

The two-step import→export pipeline is correct architecturally (separation of concerns), but for the most common use case — "show me this image as ANSI art" — it's too many steps. There's no single command to go from image to terminal output.

> Source: User workflow observation — every test required import + export + cat

### 1.4 Brown/Dark Skin Tones Render Too Dark

The xterm-256 color cube has a dead zone from 0→95 per channel. Dark chromatic colors (like brown skin tones ~RGB 180,120,70) lose their hue and snap to gray/black. The existing `nearest_256` uses plain Euclidean RGB distance which doesn't match human perception — a dark gray can be "closer" mathematically than the correct warm brown.

> Source: User comparison of kakukuma import vs original image showing muddy skin tones.

### 1.5 TUI Import Is Clunky

No clipboard paste support — users must navigate the file browser every time. The file browser requires scrolling through long directory listings with no filtering. Import options lack preprocessing controls (normalize, posterize, hue-preserve) that are available on CLI. Full-blocks import mode renders blank due to `is_empty()` check treating `ch=' '` cells as empty.

> Source: Direct user feedback during cycle-020 interactive sessions.

---

## 2. Goals & Success Metrics

| Goal | Metric | Target |
|------|--------|--------|
| Zero-config import produces readable results | Default import of photograph produces recognizable ANSI art | No manual flags needed for 80% of images |
| Terminal-correct color output | ANSI art displays correctly on user's terminal | Auto-detect via COLORTERM env var |
| Single-command image preview | Image → terminal output in one command | `kakukuma render image.png` |
| Accurate skin tone / warm color reproduction | Brown/warm tones stay chromatic in 256-color mode | Perceptual quantization + brightness lift |
| Clipboard import in TUI | Paste image from Finder via Ctrl+V | macOS clipboard integration |
| Fast file navigation | Type-to-filter in import browse dialog | Character filtering + path mode |
| Preserve existing power-user flags | All current flags remain functional | No regressions |

---

## 3. Requirements

### 3.1 Smart Import Defaults (P0)

**What**: Change `import` defaults so the common case (photograph → readable ANSI art) works without flags.

- Default `--quantize` should be `truecolor` (store full RGB, defer quantization to export)
- Default `--normalize` should be `true` (most photographs benefit from histogram stretching)
- Default `--preserve-hue` should be `true` (the dead-zone problem affects almost all images)
- Add `--no-normalize` and `--no-preserve-hue` flags for opt-out

**Why**: The current defaults optimize for the uncommon case (pre-quantized art). Photographs are the primary import use case for both humans and agents.

### 3.2 Terminal Color Auto-Detection (P0)

**What**: Detect terminal color capabilities and choose the best export color format automatically.

Detection logic:
1. Check `COLORTERM` env var: `truecolor` or `24bit` → use truecolor
2. Check `TERM` env var: contains `256color` → use 256-color with hue preservation
3. Fallback: use 256-color with hue preservation (safe default)

Apply this in:
- `preview` command (default `--color-format` becomes `auto` instead of `truecolor`)
- `export` command (same)
- New `render` command

**Why**: Users shouldn't need to know about COLORTERM to get correct output. The detection is standard across terminal emulators.

### 3.3 `render` Command — Single-Step Image-to-Terminal (P1)

**What**: New CLI command that takes an image path and outputs ANSI art directly to stdout.

```bash
# Minimal — smart defaults handle everything
kakukuma render photo.png

# With size control
kakukuma render photo.png --width 60 --height 20

# With explicit color format
kakukuma render photo.png --color-format 256
```

Internally this is import_image → to_ansi, no intermediate .kaku file. Uses all the smart defaults from 3.1 and auto-detection from 3.2.

**Why**: The most common use case should be the simplest command. Agents and users shouldn't need to understand the import→export pipeline for quick previews.

### 3.4 Improve TUI Import Dialog (P1)

**What**: Expose the new import options in the TUI import flow:
- Show normalize toggle (N key, default on)
- Show preserve-hue toggle (H key, default on)
- Show posterize selector (5 presets: Off, 8, 12, 16, 24 colors)
- Source label shows "Clipboard image" when pasting

**Why**: TUI users currently get the old defaults with no way to access the new flags without the CLI.

### 3.5 Clipboard Image Import (P1)

**What**: Allow users to paste images from macOS Finder into the TUI via Ctrl+V.

Detection logic:
1. Try `osascript` to read file URL from macOS clipboard (Finder Cmd+C)
2. Fall back to `arboard::get_image()` for raw pixel data (screenshots)
3. Open import options dialog with source set to clipboard

**Why**: Navigating the file browser for every import is slow. Copy in Finder → Ctrl+V in kakukuma is the natural workflow.

### 3.6 Import Browse Type-to-Filter (P1)

**What**: Allow typing in the import file browser to filter the file list.
- Regular characters filter the displayed list in real-time
- `/` or `~` enters path mode for absolute/home-relative paths
- Tab completes matching files
- Backspace clears filter

**Why**: Scrolling through large directories is painful. Type-to-filter is standard in file pickers.

### 3.7 Perceptual Color Quantization (P0)

**What**: Replace Euclidean RGB distance with perceptually-weighted distance in `nearest_256_hue()`.

- Use "redmean" weighted formula (accounts for human color perception)
- Add luminance preservation term with asymmetric dark penalty (darkening penalized 1.5× more)
- Increase gray penalty from 8000 to 20000 for chromatic source pixels
- Add brightness lift for dark chromatic pixels in `boost_saturation()` to escape dead zone
- Auto-apply color_boost 1.2× for 256-color, 1.4× for 16-color in TUI

**Why**: Brown skin tones, warm colors, and dark chromatic tones all lose their hue with plain Euclidean distance. Perceptual weighting keeps them warm and recognizable.

### 3.8 Full Blocks Import Fix (P0)

**What**: Fix `rasterize_full_blocks()` to use `ch: '█'` with `fg` color instead of `ch: ' '` with `bg` color.

**Why**: The renderer's `is_empty()` checks `ch == ' '` — cells with space character are treated as empty regardless of bg color, making full-blocks imports appear blank.

---

## 4. Non-Goals

- **Batch import of 10,000+ images** — Important future capability but out of scope for this cycle. Current architecture handles one image at a time. Batch support needs streaming/parallel architecture.
- **Dithering** — Ordered dithering could help the 256-color dead zone but adds significant complexity. Deferred.
- **Custom color palettes for import** — Interesting idea (e.g., "import this photo using only my project's palette") but scope creep for this cycle.
- **Animated GIF support** — Currently imports first frame only. Multi-frame is a separate feature.
- **Drag-and-drop** — Terminal.app's mouse capture intercepts drag events. Not possible without dropping mouse support. Clipboard paste is the alternative.

---

## 5. User Stories

### 5.1 Human Artist
> "I found a cool reference image and want to see what it looks like as ANSI art in my terminal. I run `kakukuma render ref.png` and it just works — correct colors, recognizable image."

### 5.2 AI Agent
> "I need to convert a product screenshot to ANSI art for display in a terminal dashboard. I run `kakukuma render screenshot.png --width 80 --height 24` and get correctly-colored output I can embed."

### 5.3 Power User
> "I'm creating a splash screen and need precise control. I import with truecolor, manually adjust in the TUI, then export with specific color settings. All the old flags still work."

---

## 6. Technical Context

### Existing Architecture
- `src/import.rs`: `import_image()` (Lanczos3 downscale) and `import_mosaic()` (grid averaging)
- `src/export.rs`: `to_ansi()` with `ColorFormat` enum (TrueColor, Color256, Color256Hue, Color16)
- `src/cell.rs`: `nearest_256()`, `nearest_256_hue()`, `nearest_16()` quantizers
- `src/cli/mod.rs`: `Import`, `Preview`, `Export` commands

### Key Constraint
- `Color::Indexed(n)` only in ratatui — truecolor escapes fail on many terminals
- xterm-256 color cube: 0→95 dead zone makes dark chromatic colors quantize to gray
- `nearest_256_hue()` mitigates this with saturation-aware penalty

### Reference
- `grimoires/loa/context/image-to-ansi-art.md` — Full technical analysis from cycle-020 exploration

---

## 7. Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Changing defaults breaks existing workflows | Medium | Medium | Add `--no-normalize`, `--no-preserve-hue` escape hatches |
| COLORTERM detection unreliable on some terminals | Low | Low | Always allow explicit `--color-format` override |
| `render` command seen as redundant with `preview` | Low | Low | `render` takes image paths, `preview` takes .kaku files — distinct entry points |
