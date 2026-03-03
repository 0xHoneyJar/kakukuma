# SDD: Block Character Usability & CLI Polish

> **Cycle**: 020
> **Created**: 2026-03-02
> **Status**: Draft
> **PRD Reference**: `grimoires/loa/prd.md` (cycle-020)

---

## 1. Architecture Overview

This cycle adds character discoverability features to the existing CLI and TUI systems, integrates unmerged cycle 19 work, and fixes compiler warnings.

```
src/cell.rs (blocks module)           ← Single source of truth for chars
    ↓                                    Already has: ALL, CATEGORY_SIZES, constants
    ↓ NEW: CHAR_INFO array with name/category metadata
    ↓
src/cli/mod.rs                        ← NEW: Chars command variant
    ↓
src/cli/chars.rs (NEW)                ← Chars handler: JSON and plain output
    ↓
src/cli/draw.rs                       ← MODIFIED: --ch accepts String, resolves via lookup
    ↓
src/ui/mod.rs                         ← MODIFIED: picker shows selected char info line
```

**Key principle**: All character metadata lives in `src/cell.rs`. CLI and TUI consume it. No duplication.

---

## 2. Component Design

### 2.1 Character Metadata (src/cell.rs)

Add a `CharInfo` struct and `CHAR_INFO` const array alongside existing `blocks` module:

```rust
pub struct CharInfo {
    pub ch: char,
    pub name: &'static str,       // Primary alias: "full", "upper-half", etc.
    pub alt: &'static str,        // Alt alias: "block", "top", etc. Empty = none
    pub category: &'static str,   // "primary", "shade", "vertical-fill", "horizontal-fill"
    pub codepoint: &'static str,  // "U+2588"
}

pub const CHAR_INFO: [CharInfo; 20] = [
    CharInfo { ch: FULL,         name: "full",         alt: "block",  category: "primary",         codepoint: "U+2588" },
    CharInfo { ch: UPPER_HALF,   name: "upper-half",   alt: "top",    category: "primary",         codepoint: "U+2580" },
    CharInfo { ch: LOWER_HALF,   name: "lower-half",   alt: "bottom", category: "primary",         codepoint: "U+2584" },
    CharInfo { ch: LEFT_HALF,    name: "left-half",    alt: "left",   category: "primary",         codepoint: "U+258C" },
    CharInfo { ch: RIGHT_HALF,   name: "right-half",   alt: "right",  category: "primary",         codepoint: "U+2590" },
    CharInfo { ch: SHADE_LIGHT,  name: "shade-light",  alt: "light",  category: "shade",           codepoint: "U+2591" },
    CharInfo { ch: SHADE_MEDIUM, name: "shade-medium",  alt: "medium", category: "shade",           codepoint: "U+2592" },
    CharInfo { ch: SHADE_DARK,   name: "shade-dark",   alt: "dark",   category: "shade",           codepoint: "U+2593" },
    CharInfo { ch: LOWER_1_8,    name: "lower-1-8",    alt: "",       category: "vertical-fill",   codepoint: "U+2581" },
    CharInfo { ch: LOWER_1_4,    name: "lower-1-4",    alt: "",       category: "vertical-fill",   codepoint: "U+2582" },
    CharInfo { ch: LOWER_3_8,    name: "lower-3-8",    alt: "",       category: "vertical-fill",   codepoint: "U+2583" },
    CharInfo { ch: LOWER_5_8,    name: "lower-5-8",    alt: "",       category: "vertical-fill",   codepoint: "U+2585" },
    CharInfo { ch: LOWER_3_4,    name: "lower-3-4",    alt: "",       category: "vertical-fill",   codepoint: "U+2586" },
    CharInfo { ch: LOWER_7_8,    name: "lower-7-8",    alt: "",       category: "vertical-fill",   codepoint: "U+2587" },
    CharInfo { ch: LEFT_7_8,     name: "left-7-8",     alt: "",       category: "horizontal-fill", codepoint: "U+2589" },
    CharInfo { ch: LEFT_3_4,     name: "left-3-4",     alt: "",       category: "horizontal-fill", codepoint: "U+258A" },
    CharInfo { ch: LEFT_5_8,     name: "left-5-8",     alt: "",       category: "horizontal-fill", codepoint: "U+258B" },
    CharInfo { ch: LEFT_3_8,     name: "left-3-8",     alt: "",       category: "horizontal-fill", codepoint: "U+258D" },
    CharInfo { ch: LEFT_1_4,     name: "left-1-4",     alt: "",       category: "horizontal-fill", codepoint: "U+258E" },
    CharInfo { ch: LEFT_1_8,     name: "left-1-8",     alt: "",       category: "horizontal-fill", codepoint: "U+258F" },
];
```

**Lookup function**:

```rust
/// Resolve a character alias to a char. Returns None if not found.
/// Single-char input returns the char directly (backward compat).
pub fn resolve_char_alias(input: &str) -> Option<char> {
    if input.chars().count() == 1 {
        return Some(input.chars().next().unwrap());
    }
    let lower = input.to_lowercase();
    CHAR_INFO.iter().find(|info| {
        info.name == lower || (!info.alt.is_empty() && info.alt == lower)
    }).map(|info| info.ch)
}

/// Look up CharInfo by char. Returns None for non-block chars.
pub fn char_info(ch: char) -> Option<&'static CharInfo> {
    CHAR_INFO.iter().find(|info| info.ch == ch)
}
```

### 2.2 `kakukuma chars` Command (src/cli/chars.rs — NEW)

**Command definition** (added to `Command` enum in mod.rs):

```rust
/// List available block characters
Chars {
    /// Filter by category
    #[arg(long)]
    category: Option<String>,
    /// Human-readable table output instead of JSON
    #[arg(long)]
    plain: bool,
},
```

**Handler** (`src/cli/chars.rs`):

```rust
pub fn run_chars(category: Option<&str>, plain: bool) -> io::Result<()> {
    let chars: Vec<&CharInfo> = if let Some(cat) = category {
        blocks::CHAR_INFO.iter().filter(|c| c.category == cat).collect()
    } else {
        blocks::CHAR_INFO.iter().collect()
    };

    if plain {
        print_plain_table(&chars);
    } else {
        print_json(&chars);
    }
    Ok(())
}
```

**JSON output** follows existing pattern (see inspect, stats commands):
```json
{
  "characters": [...],
  "categories": ["primary", "shade", "vertical-fill", "horizontal-fill"],
  "total": 20
}
```

### 2.3 `--ch` Alias Resolution (src/cli/draw.rs)

**Change**: `DrawOpts.ch` type from `Option<char>` to `Option<String>`.

**Resolution** in each draw handler:

```rust
let ch = match &opts.ch {
    Some(s) => resolve_char_alias(s).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput,
            format!("Unknown character '{}'. Run 'kakukuma chars' for available characters.", s))
    })?,
    None => blocks::FULL,
};
```

**Updated help text**:
```rust
/// Block character: raw char (█) or name (full, shade-light, etc.). See 'kakukuma chars'.
#[arg(long)]
pub ch: Option<String>,
```

### 2.4 TUI Block Picker Enhancement (src/ui/mod.rs)

The picker already has category labels. Add a **selected char info line** at the bottom:

**Current layout** (width=38, height=10):
```
┌─ Block Picker ─────────────────────┐
│  Primary:    █ ▀ ▄ ▌ ▐            │
│  Shades:     ░ ▒ ▓                │
│  Vert Fill:  ▁ ▂ ▃ ▅ ▆ ▇          │
│  Horiz Fill: ▉ ▊ ▋ ▍ ▎ ▏          │
│                                    │
│  ← → ↑ ↓  Enter  Esc             │
└────────────────────────────────────┘
```

**After** (height +1 → 11):
```
┌─ Block Picker ─────────────────────┐
│  Primary:    █ ▀ ▄ ▌ ▐            │
│  Shades:     ░ ▒ ▓                │
│  Vert Fill:  ▁ ▂ ▃ ▅ ▆ ▇          │
│  Horiz Fill: ▉ ▊ ▋ ▍ ▎ ▏          │
│                                    │
│  █ full (U+2588)                   │  ← NEW: selected char info
│  ← → ↑ ↓  Enter  Esc             │
└────────────────────────────────────┘
```

**Implementation**: Look up `blocks::char_info(selected_char)` and render a line showing `{char} {name} ({codepoint})`.

### 2.5 Cycle 19 Integration

**Strategy**: Merge the `feature/cycle-019-cli-polish-image-export` branch into the new cycle branch.

**Commits to include**:
- `4093b666` — feat(sprint-1): CLI normalization, auto-format detection, and PNG export engine
- `bd9465be` — feat(sprint-2): comprehensive PNG export test suite and sprint-1 review/audit artifacts
- `4a10acc8` — chore(sprint-6): review and audit approval for PNG export test suite

**Conflict risk**: Low — cycle 19 modifies `src/cli/mod.rs` and `src/export.rs`, and this cycle also modifies `src/cli/mod.rs`. Conflicts will be in the `Command` enum (add `Chars` variant) and `DrawOpts` (change `ch` type).

**Resolution strategy**: Apply cycle 19 first, then layer cycle 20 changes on top.

### 2.6 Compiler Warning Fixes

1. **`Canvas::clear()` (src/canvas.rs:51)**: Add `#[allow(dead_code)]` — it's a useful API method for lib consumers even if not called internally.

2. **`build_spans()` (src/ui/statusbar.rs:114)**: Investigate usage. If truly unused, remove it. If it was meant to replace something, integrate or remove.

---

## 3. Data Model

No persistent data changes. All character metadata is compile-time constants.

### 3.1 JSON Schema: `kakukuma chars`

```json
{
  "characters": [
    {
      "char": "string (single Unicode char)",
      "name": "string (primary alias)",
      "alt": "string (alt alias, may be empty)",
      "category": "string (primary|shade|vertical-fill|horizontal-fill)",
      "codepoint": "string (U+XXXX)"
    }
  ],
  "categories": ["string"],
  "total": "integer"
}
```

---

## 4. Security Considerations

- **No new attack surface**: `chars` command is read-only, outputs static data
- **Alias resolution**: `resolve_char_alias()` does string comparison only, no eval/exec
- **CLI input**: `--ch` value is validated against known aliases or accepted as single char — no injection risk
- **Color rules**: All terminal rendering still uses `Color::Indexed()` — no `Color::Rgb()` introduced

---

## 5. Test Strategy

### 5.1 New Tests

| Location | Tests | Description |
|----------|-------|-------------|
| `src/cell.rs` | +5 | `resolve_char_alias` for primary name, alt name, single char, unknown, case-insensitive |
| `src/cli/mod.rs` | +4 | `chars` command parsing, `--category` filter, `--plain` flag, `--ch` with alias |
| `src/cli/chars.rs` | +3 | JSON output structure, plain output format, category filter |
| `src/ui/mod.rs` | +1 | Picker info line renders correct name for selected char |

### 5.2 Existing Tests

All 375 tests from cycle 19 must pass after merge + modifications.

**Target**: 375 existing + 13 new = 388+ tests

---

## 6. File Change Summary

| File | Change | Scope |
|------|--------|-------|
| `src/cell.rs` | Add `CharInfo`, `CHAR_INFO`, `resolve_char_alias()`, `char_info()` | ~50 lines |
| `src/cli/mod.rs` | Add `Chars` variant to `Command`, wire dispatch | ~10 lines |
| `src/cli/chars.rs` | **NEW**: chars command handler | ~80 lines |
| `src/cli/draw.rs` | Change `ch: Option<char>` → `Option<String>`, add alias resolution | ~15 lines |
| `src/ui/mod.rs` | Add selected char info line to picker, increase height by 1 | ~10 lines |
| `src/canvas.rs` | Add `#[allow(dead_code)]` to `clear()` | 1 line |
| `src/ui/statusbar.rs` | Remove or annotate `build_spans()` | ~1-5 lines |

**Estimated total new/modified lines**: ~170 (excluding cycle 19 merge)
