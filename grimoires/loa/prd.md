# PRD: Block Character Usability & CLI Polish

> **Status**: Draft
> **Created**: 2026-03-02
> **Author**: AI Agent + @gumibera
> **Cycle**: 020

---

## 1. Problem Statement

Kakukuma supports 20 Unicode block elements for drawing, but discoverability is poor for both agents and humans:

1. **CLI `--ch` flag is opaque**: The draw commands accept `--ch <char>` but don't document which characters are available. Agents must guess or hard-code Unicode codepoints. There's no way to query the available character set programmatically.

2. **TUI block picker lacks labels**: The Shift+B picker shows a grid of 20 characters organized in 4 rows, but there are no category labels or character names. Users must recognize glyphs visually.

3. **No character name aliases**: CLI users must type raw Unicode (`--ch █`) which is error-prone and terminal-dependent. Named aliases like `--ch full` or `--ch shade-light` would be more ergonomic.

4. **Unmerged cycle 19 work**: CLI normalization (positional args), auto-format detection, and PNG export engine were completed but never merged to main. This work needs to ship.

5. **Compiler warnings**: Two dead-code warnings on main (`Canvas::clear()`, `build_spans()`) indicate housekeeping debt.

> Source: Block character audit and agent CLI testing, 2026-03-02.

---

## 2. Goals & Success Metrics

| Goal | Metric | Target |
|------|--------|--------|
| Agent discoverability | Agent can query all available characters programmatically | `kakukuma chars` returns JSON catalog |
| Character name aliases | Named `--ch` values accepted | All 20 blocks have aliases |
| TUI picker clarity | User can identify characters without guessing | Category labels + char names in picker |
| Cycle 19 inclusion | CLI normalization + PNG export on main | Merged and tested |
| Zero warnings | Compiler warnings | 0 |
| Test coverage | All tests pass | 375+ existing + new tests |

---

## 3. Users

### Primary: AI Agents (Claude, GPT, etc.)
- Need to query available characters before drawing
- Prefer named aliases over raw Unicode in commands
- Parse JSON output for character metadata (name, category, codepoint)

### Secondary: Human Artists
- Need visual identification in the TUI picker
- Want keyboard shortcuts for common character categories
- Expect clear --help documentation

---

## 4. Functional Requirements

### FR-1: `kakukuma chars` CLI Command

**Priority**: P0

Add a new `chars` subcommand that lists all available block characters with metadata.

**Output (default JSON)**:
```json
{
  "characters": [
    {"char": "█", "name": "full", "category": "primary", "codepoint": "U+2588"},
    {"char": "▀", "name": "upper-half", "category": "primary", "codepoint": "U+2580"},
    {"char": "░", "name": "shade-light", "category": "shade", "codepoint": "U+2591"}
  ],
  "categories": ["primary", "shade", "vertical-fill", "horizontal-fill"],
  "total": 20
}
```

**Options**:
- `--category <CAT>`: Filter by category (primary, shade, vertical-fill, horizontal-fill)
- `--plain`: Output as human-readable table instead of JSON

**Human-readable output (`--plain`)**:
```
PRIMARY (5):
  █  full          U+2588
  ▀  upper-half    U+2580
  ▄  lower-half    U+2584
  ▌  left-half     U+258C
  ▐  right-half    U+2590

SHADE (3):
  ░  shade-light   U+2591
  ▒  shade-medium  U+2592
  ▓  shade-dark    U+2593

VERTICAL-FILL (6):
  ▁  lower-1-8     U+2581
  ▂  lower-1-4     U+2582
  ▃  lower-3-8     U+2583
  ▅  lower-5-8     U+2585
  ▆  lower-3-4     U+2586
  ▇  lower-7-8     U+2587

HORIZONTAL-FILL (6):
  ▉  left-7-8      U+2589
  ▊  left-3-4      U+258A
  ▋  left-5-8      U+258B
  ▍  left-3-8      U+258D
  ▎  left-1-4      U+258E
  ▏  left-1-8      U+258F
```

### FR-2: Named Character Aliases for `--ch`

**Priority**: P0

All draw commands that accept `--ch` should accept named aliases in addition to raw characters:

| Character | Alias | Alt Aliases |
|-----------|-------|-------------|
| `█` | `full` | `block` |
| `▀` | `upper-half` | `top` |
| `▄` | `lower-half` | `bottom` |
| `▌` | `left-half` | `left` |
| `▐` | `right-half` | `right` |
| `░` | `shade-light` | `light` |
| `▒` | `shade-medium` | `medium` |
| `▓` | `shade-dark` | `dark` |
| `▁` | `lower-1-8` | - |
| `▂` | `lower-1-4` | - |
| `▃` | `lower-3-8` | - |
| `▅` | `lower-5-8` | - |
| `▆` | `lower-3-4` | - |
| `▇` | `lower-7-8` | - |
| `▉` | `left-7-8` | - |
| `▊` | `left-3-4` | - |
| `▋` | `left-5-8` | - |
| `▍` | `left-3-8` | - |
| `▎` | `left-1-4` | - |
| `▏` | `left-1-8` | - |

**Resolution order**:
1. If `--ch` is a single character, use it directly (backward compatible)
2. If `--ch` is a multi-character string, look up as alias name
3. If alias not found, return structured JSON error

**Updated `--ch` help text**:
```
--ch <CHAR>  Block character: raw char (█) or name (full, upper-half, shade-light, etc.)
             Run 'kakukuma chars' for full list.
```

### FR-3: TUI Block Picker Category Labels

**Priority**: P1

Enhance the block picker dialog (Shift+B) with:

1. **Category headers**: "Primary", "Shades", "Vertical Fill", "Horizontal Fill" above each row
2. **Selected char info**: Show the name and codepoint of the currently highlighted character at the bottom of the picker

### FR-4: Cycle 19 Integration

**Priority**: P0

Merge the completed cycle 19 work from `feature/cycle-019-cli-polish-image-export`:
- CLI argument normalization (positional args for export, import, batch, palette export)
- Auto-format detection (file extension → format)
- PNG export engine with block character rendering
- All associated tests

### FR-5: Compiler Warning Cleanup

**Priority**: P2

Fix the 2 dead-code warnings on main:
- `Canvas::clear()` — either use it or remove the `pub` and add `#[allow(dead_code)]`
- `build_spans()` in statusbar.rs — either use it or remove

---

## 5. Technical Constraints

- **Character alias lookup**: Must be compile-time data (const array or lazy_static), not runtime-loaded
- **JSON output**: Must match existing structured JSON patterns (see export, inspect commands)
- **Backward compatibility**: Raw `--ch █` must still work — aliases are additive
- **TUI picker**: Must not increase picker dialog size beyond terminal minimum (80x24)
- **Color**: `Color::Indexed(n)` only — no `Color::Rgb()` in terminal rendering

---

## 6. Scope

### In Scope
- `kakukuma chars` command with JSON and plain output
- Named `--ch` aliases for all 20 block characters
- TUI picker category labels and selected-char info
- Cycle 19 merge (CLI normalization + PNG export)
- Compiler warning fixes
- Tests for all new functionality

### Out of Scope
- Box drawing characters (future cycle)
- New block characters (20 is sufficient)
- TUI character search/filter
- Changes to block cycling (B/G keys)

---

## 7. Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Cycle 19 merge conflicts | Blocks integration | Cherry-pick commit-by-commit, resolve incrementally |
| Alias name collisions with Unicode chars | Confusing behavior | Single-char input always treated as raw char, multi-char as alias |
| Picker labels make dialog too wide | Breaks on small terminals | Use abbreviated labels, test at 80-col minimum |
| `--ch` help text too long | Clutters help output | Keep brief, point to `kakukuma chars` for full list |

---

## 8. Dependencies

- Cycle 19 branch `feature/cycle-019-cli-polish-image-export` (existing work)
- `clap` derive macros for new `chars` subcommand
- `serde_json` for JSON output (already a dependency)
- Existing `blocks` module in `src/cell.rs`
