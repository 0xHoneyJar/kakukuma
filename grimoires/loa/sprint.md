# Sprint Plan: Block Character Usability & CLI Polish

> **Cycle**: 020
> **Created**: 2026-03-02
> **PRD**: grimoires/loa/prd.md
> **SDD**: grimoires/loa/sdd.md

---

## Sprint Overview

| Item | Value |
|------|-------|
| Total sprints | 2 |
| Sprint 1 | Cycle 19 merge + Character metadata + CLI chars command |
| Sprint 2 | --ch alias resolution + TUI picker enhancement + Warning fixes |
| Estimated new tests | 13 |
| Target total tests | 388+ |
| New files | 1 (`src/cli/chars.rs`) |
| Modified files | 6 (`src/cell.rs`, `src/cli/mod.rs`, `src/cli/draw.rs`, `src/ui/mod.rs`, `src/canvas.rs`, `src/ui/statusbar.rs`) |

---

## Sprint 1: Cycle 19 Merge + Character Metadata + CLI Chars Command

**Goal**: Merge the unmerged cycle 19 work (CLI normalization + PNG export), add character metadata to `cell.rs`, and implement the `kakukuma chars` CLI command for agent discoverability.

### Task 1.1: Merge Cycle 19 Branch

**Description**: Merge `feature/cycle-019-cli-polish-image-export` into the new cycle branch. This brings CLI normalization (positional args), auto-format detection, and PNG export engine onto main.

**Files**: All files modified in cycle 19 commits (primarily `src/cli/mod.rs`, `src/export.rs`, `src/cli/preview.rs`)

**Acceptance Criteria**:
- [ ] All 3 cycle 19 commits cherry-picked or merged
- [ ] Merge conflicts resolved (if any)
- [ ] All 375 existing tests pass
- [ ] `cargo build` succeeds with no new warnings

### Task 1.2: Character Metadata in cell.rs

**Description**: Add `CharInfo` struct, `CHAR_INFO` const array, `resolve_char_alias()`, and `char_info()` functions to the `blocks` module in `src/cell.rs`. This is the single source of truth for all character metadata.

**Files**: `src/cell.rs`

**Acceptance Criteria**:
- [ ] `CharInfo` struct with `ch`, `name`, `alt`, `category`, `codepoint` fields
- [ ] `CHAR_INFO: [CharInfo; 20]` const array covering all 20 block chars
- [ ] `resolve_char_alias("full")` returns `Some('█')`
- [ ] `resolve_char_alias("block")` returns `Some('█')` (alt alias)
- [ ] `resolve_char_alias("█")` returns `Some('█')` (single char passthrough)
- [ ] `resolve_char_alias("unknown")` returns `None`
- [ ] `char_info('█')` returns `Some(CharInfo { name: "full", ... })`
- [ ] `char_info('a')` returns `None` (non-block char)

### Task 1.3: `kakukuma chars` CLI Command

**Description**: Add a new `Chars` variant to the `Command` enum and create `src/cli/chars.rs` handler that outputs character catalog as JSON or plain text table.

**Files**: `src/cli/mod.rs`, `src/cli/chars.rs` (new)

**Acceptance Criteria**:
- [ ] `Chars` variant added to `Command` enum with `--category` and `--plain` options
- [ ] `kakukuma chars` outputs JSON with `characters`, `categories`, `total` fields
- [ ] `kakukuma chars --plain` outputs human-readable table grouped by category
- [ ] `kakukuma chars --category shade` filters to shade characters only
- [ ] `kakukuma chars --category primary --plain` combines filter + format
- [ ] Invalid `--category` value returns structured JSON error
- [ ] Module declared in `src/cli/mod.rs` and wired into `run()` dispatch

### Task 1.4: Sprint 1 Tests

**Description**: Unit tests for character metadata and chars command.

**Files**: `src/cell.rs` (test module), `src/cli/mod.rs` (test module)

**Acceptance Criteria**:
- [ ] Test: `resolve_char_alias("full")` returns FULL block
- [ ] Test: `resolve_char_alias("shade-light")` returns SHADE_LIGHT
- [ ] Test: `resolve_char_alias("block")` returns FULL (alt alias)
- [ ] Test: `resolve_char_alias("█")` returns FULL (single char)
- [ ] Test: `resolve_char_alias("FULL")` returns FULL (case insensitive)
- [ ] Test: `resolve_char_alias("nope")` returns None
- [ ] Test: `char_info` returns correct info for known block
- [ ] Test: chars command parse `["chars"]` succeeds
- [ ] Test: chars command parse `["chars", "--category", "shade"]` succeeds
- [ ] All 375+ tests pass

---

## Sprint 2: --ch Alias Resolution + TUI Picker Enhancement + Warning Fixes

**Goal**: Wire character aliases into the draw commands' `--ch` flag, enhance the TUI block picker with selected char info, and fix compiler warnings.

### Task 2.1: --ch Alias Resolution in DrawOpts

**Description**: Change `DrawOpts.ch` from `Option<char>` to `Option<String>` and add alias resolution using `resolve_char_alias()`. All draw commands benefit automatically.

**Files**: `src/cli/draw.rs`

**Acceptance Criteria**:
- [ ] `DrawOpts.ch` type changed to `Option<String>`
- [ ] Each draw handler resolves alias: `resolve_char_alias(s).ok_or_else(|| error)`
- [ ] `--ch █` still works (single char passthrough)
- [ ] `--ch full` works (primary alias)
- [ ] `--ch block` works (alt alias)
- [ ] `--ch shade-dark` works
- [ ] `--ch nope` returns structured JSON error with helpful message
- [ ] Help text updated: "Block character: raw char (█) or name (full, shade-light, etc.). See 'kakukuma chars'."

### Task 2.2: TUI Block Picker Selected Char Info

**Description**: Add a line to the block picker dialog showing the name and codepoint of the currently highlighted character. Increase dialog height by 1.

**Files**: `src/ui/mod.rs`

**Acceptance Criteria**:
- [ ] Picker height increased from 10 to 11
- [ ] New line between char grid and help line shows: `{char} {name} ({codepoint})`
- [ ] Info line updates as user navigates with arrow keys
- [ ] Info line uses `blocks::char_info()` for lookup
- [ ] Dialog still fits in 80x24 terminal minimum

### Task 2.3: Compiler Warning Fixes

**Description**: Fix the 2 dead-code warnings on the codebase.

**Files**: `src/canvas.rs`, `src/ui/statusbar.rs`

**Acceptance Criteria**:
- [ ] `Canvas::clear()` annotated with `#[allow(dead_code)]` (useful API for lib consumers)
- [ ] `build_spans()` in statusbar.rs: either removed (if truly unused) or annotated
- [ ] `cargo build` produces 0 warnings

### Task 2.4: Sprint 2 Tests

**Description**: Tests for alias resolution and picker enhancement.

**Files**: `src/cli/mod.rs` (test module), `src/ui/mod.rs` (test module)

**Acceptance Criteria**:
- [ ] Test: draw pencil with `--ch full` parses correctly
- [ ] Test: draw pencil with `--ch shade-dark` parses correctly
- [ ] Test: draw pencil with `--ch █` still works (backward compat)
- [ ] All 385+ tests pass (375 base + 10 sprint 1 + new)
- [ ] `cargo clippy` clean
- [ ] 0 compiler warnings

---

## Risk Assessment

| Risk | Sprint | Mitigation |
|------|--------|------------|
| Cycle 19 merge conflicts in cli/mod.rs | Sprint 1 | Apply cycle 19 first, then add Chars variant on top |
| `Option<String>` for --ch breaks existing CLI test assertions | Sprint 2 | Update any tests that construct DrawOpts directly |
| Picker height increase breaks small terminal rendering | Sprint 2 | Test at 80x24 minimum; 11 rows is well within bounds |
