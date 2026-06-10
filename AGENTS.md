# Kakukuma for AI Agents

Kakukuma is an ANSI art editor designed for both humans (TUI) and AI agents
(CLI). This document is the agent contract: how to perceive, draw, and verify.

## The loop

You are a painter who must explicitly look at the canvas. Work in this cycle:

```
1. DRAW      kakukuma batch art.kaku ops.json     (or draw subcommands)
2. PERCEIVE  kakukuma snapshot art.kaku           (PNG for vision + grid for reasoning)
3. CRITIQUE  look at the PNG; read the grid
4. REFINE    more batch ops; undo if needed
```

Never chain many draw operations without a `snapshot` in between. Drawing
blind compounds errors.

## Perception

### `kakukuma snapshot <file> [out.png]`

One call, two channels:

- Writes a **PNG render** (default `<file>.png`) — view it with your vision
  capability to judge composition, color, and shape quality.
- Prints **describe JSON**: dimensions, content hash, fill stats, bounding
  box, legend, and the symbol grid.

Options: `--cell-size 8x16` (pixels per cell), `--scale N` (upscale).

### `kakukuma describe <file> [--plain]`

The structured channel alone. The key output is the **indexed grid**:

```json
{
  "width": 16, "height": 8,
  "hash": "a1b2c3d4e5f60789",
  "legend": [
    {"symbol": "a", "char": "█", "fg": "#FF0000", "bg": null, "count": 41},
    {"symbol": "b", "char": "▒", "fg": "#0000FF", "bg": "#000000", "count": 12}
  ],
  "grid": [
    "................",
    "....aaaaaaaa....",
    "...abbbbbbbba...",
    "....aaaaaaaa....",
    "................"
  ]
}
```

Each distinct cell appearance (char + fg + bg) gets one legend symbol,
assigned by frequency (`a` = most common). `.` = empty cell. One character
per cell: a 48x32 canvas costs ~1.6k characters to perceive precisely.
Row index = y, column index = x — `grid[y][x]` is the cell at (x, y).

### Other inspection commands

| Command | Use when |
|---------|----------|
| `inspect <file> X,Y` / `--region x1,y1,x2,y2` | exact cell values |
| `stats <file>` | color/char distributions, symmetry scores |
| `diff <a.kaku> <b.kaku>` / `diff <file> --before` | what changed |
| `history <file>` | operation log |

## Verification with hashes

Every mutating command prints a `hash` — a stable content hash of the canvas.
`describe`/`snapshot` print it too. Use it to confirm state without re-reading:
if your `batch` result hash matches your next `describe` hash, nothing changed
in between. Identical canvases always have identical hashes.

## Drawing

### Batch (preferred for multi-step work)

`kakukuma batch <file> <ops.json>` — one atomic save, one result:

```json
{
  "operations": [
    {"op": "clear"},
    {"op": "gradient", "x1": 0, "y1": 0, "x2": 47, "y2": 31,
     "start": "#1a0533", "end": "#ff6b35", "direction": "vertical"},
    {"op": "ellipse", "x1": 18, "y1": 4, "x2": 30, "y2": 12,
     "fg": "#FFE66D", "filled": true},
    {"op": "draw", "tool": "line", "x1": 0, "y1": 28, "x2": 47, "y2": 28,
     "fg": "#2EC4B6", "ch": "upper-half"},
    {"op": "set_cell", "x": 24, "y": 8, "ch": "░", "fg": "#FFFFFF"}
  ]
}
```

Available ops:

| op | required fields | optional |
|----|-----------------|----------|
| `draw` | `tool` (pencil/eraser/line/rect/fill), coords | `ch`, `fg`, `bg`, `filled` |
| `set_cell` | `x`, `y` | `ch`, `fg`, `bg` |
| `ellipse` | `x1,y1,x2,y2` (bounding box) | `ch`, `fg`, `bg`, `filled` |
| `gradient` | `x1,y1,x2,y2`, `start`, `end` | `direction` (vertical/horizontal), `dither` |
| `clear` | — | `region: [x1,y1,x2,y2]` |
| `resize` | `width`, `height` | — |

Colors are `#RRGGBB` hex (full truecolor — don't limit yourself to 16 colors).
Characters: raw (`█`) or names (`full`, `upper-half`, `shade-light`, ...);
list them with `kakukuma chars`.

### Direct draw subcommands

For single operations: `kakukuma draw pencil|line|rect|ellipse|fill|gradient|eraser|eyedropper ...`
(e.g. `kakukuma draw ellipse art.kaku 4,4 20,12 --filled --fg "#FF0000"`).
All support `--symmetry horizontal|vertical|quad` except gradient.

### Gradient `dither` mode

`"dither": true` uses the classic ANSI shade ramp (`█ ░ ▒ ▓`) blending the
two colors instead of smooth per-cell RGB. Use it for retro texture; use
smooth (default) for skies and lighting.

## Craft notes (what makes ANSI art look good)

- **Half-blocks double your vertical resolution**: `upper-half`/`lower-half`
  with distinct fg/bg paint two "pixels" per cell. The PNG render resolves
  these — trust the PNG, not your mental model.
- **Shade chars (`░▒▓`) are your dithering/texture tool** — mixing fg over bg
  at 25/50/75%.
- **Work background → foreground**: gradient or fill the backdrop first,
  then large shapes, then detail with `set_cell`.
- **Use the bounding box** from `describe` to keep compositions centered.
- **Undo is cheap**: `kakukuma undo <file>` reverts the last logged
  operation. Experiment freely.

## Safety and recovery

- All saves are atomic; the `.kaku` file is JSON if you ever need to read it raw.
- `batch` reports per-operation errors without aborting the rest; check
  `"errors"` in the result.
- Out-of-range coordinates clamp or no-op — they never panic.
- Exit codes: `0` ok, `1` user error (bad input), `2` internal error. Errors
  print JSON to stderr with `"code"`.
