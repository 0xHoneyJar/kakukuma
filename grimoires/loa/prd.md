# PRD: Creative Power Tools — Command Palette, Reference Layer, Batch Draw

> **Cycle**: 018
> **Created**: 2026-02-28
> **Status**: Draft
> **Author**: AI Peer + @gumibera

---

## 1. Problem Statement

Kakukuma is a mature terminal ANSI art editor (285 tests, 17 cycles, library crate + full CLI) with strong fundamentals but three gaps that prevent it from reaching "best-in-class" status:

1. **Discoverability**: The editor has 25+ keybindings, 21 modes, and 6 tools — but no unified way to find or invoke them. Users must memorize shortcuts or read help. Alt-key shortcuts conflict with OS-level bindings on Linux/Mac.

2. **No reference tracing**: Artists can't place a source image behind the canvas for tracing. This is the #1 workflow gap vs. REXPaint, which supports glyph browsing and visual reference. It blocks the "two-pass shading workflow" (blockout → refine) that professional ANSI artists rely on.

3. **Agent batch operations are serial**: Despite cycle-017's CLI completeness, agents must issue one command per operation. A workflow like "draw 50 cells, fill 3 regions, set 10 colors" requires 63 separate process invocations. This is slow and defeats the spatial reasoning guidance that agents should use multi-pass decomposition (blockout → detail → shade → validate).

> Sources: `grimoires/loa/context/strategic-product-roadmap.md`, `grimoires/loa/context/spatial-reasoning-constraints.md`, codebase reality (`grimoires/loa/reality/`)

---

## 2. Vision & Mission

**Vision**: Kakukuma is as discoverable for beginners as it is powerful for agents — every capability is one Spacebar press away, every reference image is one toggle away, every agent workflow is one batch file away.

**Mission**: Add three features that each unlock a distinct user dimension: command palette (human discoverability), reference layer (artist productivity), batch draw (agent throughput).

**Why now**: Cycle-017 completed the agent-first foundation (lib.rs, CLI completeness, JSON errors). These three features are the natural next layer — they build on the existing CLI and TUI infrastructure without requiring architectural changes.

---

## 3. Goals & Success Metrics

| Goal | Metric | Target |
|------|--------|--------|
| Command palette | Spacebar opens fuzzy search, all commands accessible | Every keybinding reachable via palette |
| Reference layer | Image visible behind canvas in TUI | Toggle on/off, adjustable opacity |
| Batch draw | Multi-step JSON operations in single CLI call | 50+ operations in one invocation |
| No regressions | All existing tests pass | 285+ tests green |
| Performance | Batch draw throughput | 100 operations < 500ms |

---

## 4. User & Agent Context

### Human Persona: The Discovering Artist
- Opens Kakukuma for the first time, sees a canvas
- Presses Spacebar, types "sym" → finds symmetry toggle
- Loads a reference image to trace a character portrait
- Never needs to read the help screen

### Agent Persona: The Batch Operator
- Receives a design brief: "Create a 48x32 splash screen with a red border and centered text"
- Generates a batch JSON file with 200+ draw operations
- Executes in a single `kakukuma batch` call
- Inspects result with `kakukuma inspect` to validate

> Source: `grimoires/loa/context/spatial-reasoning-constraints.md` — agents should use multi-pass decomposition with explicit coordinates, not spatial imagination

---

## 5. Functional Requirements

### FR-1: Command Palette (Spacebar)

A fuzzy-search command palette triggered by Spacebar in Normal mode. Provides instant access to every editor action.

**Acceptance Criteria:**
- Spacebar in Normal mode opens command palette overlay (centered, top-third of screen)
- Text input field with instant fuzzy filtering as user types
- Arrow keys to navigate filtered results, Enter to execute, Esc to dismiss
- Commands include ALL existing keybindings mapped to descriptive names:
  - Tools: "Pencil", "Eraser", "Line", "Rectangle", "Fill", "Eyedropper"
  - Canvas: "New Canvas", "Resize Canvas", "Clear Canvas", "Import Image"
  - File: "Save", "Save As", "Open", "Export"
  - Edit: "Undo", "Redo"
  - View: "Zoom In", "Zoom Out", "Toggle Grid", "Cycle Theme"
  - Character: "Block Picker", "Shade Cycle", "Character Input"
  - Color: "Hex Color Input", "Color Sliders"
  - Symmetry: "Symmetry Off", "Symmetry Horizontal", "Symmetry Vertical", "Symmetry Quad"
  - Help: "Show Help", "Quit"
- Each command shows its keyboard shortcut as right-aligned hint text
- Fuzzy matching: "sav" matches "Save", "Save As"; "sym h" matches "Symmetry Horizontal"
- Theme-aware styling (uses `app.theme()`)
- Existing Spacebar behavior (draw at cursor in canvas-cursor mode) moves to Enter key or a palette command

**Note**: The Spacebar currently places a cell when `canvas_cursor_active` is true. When canvas cursor is active, Spacebar retains its draw behavior. The command palette opens only when canvas cursor is NOT active (the default state).

### FR-2: Reference Layer

An image overlay displayed behind the canvas in the TUI, with adjustable transparency, for tracing and reference.

**Acceptance Criteria:**
- `kakukuma reference <project.kaku> <image.png>` CLI command sets the reference image
- `kakukuma reference <project.kaku> --clear` removes the reference
- Reference image stored as path in the `.kaku` project file (project format v6)
- In TUI: reference image rendered behind canvas cells at reduced brightness
- Transparency level adjustable (3 levels: dim/medium/bright) via command palette or shortcut
- Toggle reference visibility on/off via command palette ("Toggle Reference") or shortcut
- Reference image scaled to fit canvas dimensions (reuses existing `import.rs` scaling logic)
- Reference rendering: convert image pixels to background colors at reduced saturation/brightness
- Transparent canvas cells show reference image; opaque cells occlude it
- Works at all zoom levels (1x, 2x, 4x)
- Backward compatible: v5 projects load without reference (field is optional)

### FR-3: Batch Draw (JSON File Input)

Execute multiple CLI operations from a JSON file with atomic save. Designed for agent multi-pass workflows.

**Acceptance Criteria:**
- `kakukuma batch <project.kaku> --commands <ops.json>` executes all operations
- JSON schema for operations file:
  ```json
  {
    "operations": [
      {"op": "draw", "tool": "pencil", "x": 5, "y": 10, "ch": "█", "fg": "#FF0000", "bg": "#000080"},
      {"op": "draw", "tool": "rect", "x1": 0, "y1": 0, "x2": 47, "y2": 0, "ch": "▀", "fg": "#0000FF"},
      {"op": "draw", "tool": "fill", "x": 24, "y": 16, "fg": "#00FF00", "bg": "#001100"},
      {"op": "draw", "tool": "line", "x1": 0, "y1": 0, "x2": 47, "y2": 31, "fg": "#FFFFFF"},
      {"op": "clear", "region": [0, 0, 10, 10]},
      {"op": "resize", "width": 64, "height": 48},
      {"op": "set_cell", "x": 0, "y": 0, "ch": "█", "fg": "#FF0000", "bg": "#0000FF"}
    ]
  }
  ```
- All operations run against a single project load (no repeated file I/O)
- Best-effort execution: per-operation errors are skipped and reported, not fatal
- Atomic save: final write uses write-to-tmp + rename pattern (all-or-nothing I/O)
- JSON output: summary of operations executed, cells modified, errors
  ```json
  {"operations": 7, "cells_modified": 1536, "errors": 0, "file": "art.kaku"}
  ```
- Supports all existing draw tools: pencil, eraser, line, rect, fill
- Supports `set_cell` for direct cell manipulation (character + fg + bg)
- All draw operations accept optional `ch` (character, default '█'), `fg`, and `bg` fields
- Supports `clear` (full or region) and `resize`
- Operations execute in order (important for layered agent workflows)
- `--dry-run` flag to validate JSON without executing
- Performance: 100 operations < 500ms on standard hardware

---

## 6. Technical & Non-Functional Requirements

### NFR-1: No Breaking Changes
- TUI behavior identical for all existing workflows
- CLI output format for existing commands unchanged
- Project file format v5 remains loadable (v6 adds optional reference field)

### NFR-2: Test Coverage
- Command palette: tests for fuzzy matching, command registry completeness
- Reference layer: tests for project v6 roundtrip, reference scaling
- Batch draw: tests for JSON parsing, operation execution, error handling, atomic save
- All 285+ existing tests pass

### NFR-3: Spacebar Conflict Resolution
- When `canvas_cursor_active == true`: Spacebar draws (existing behavior)
- When `canvas_cursor_active == false` (default): Spacebar opens command palette
- This is a context-sensitive binding, not a breaking change

### NFR-4: Project File Versioning
- Version bumps to 6 only if reference layer data is present
- v5 projects with no reference save as v5 (no unnecessary version bump)
- v6 projects load in older versions by ignoring unknown fields

---

## 7. Scope & Prioritization

### In Scope (This Cycle)
1. Command palette (Spacebar, fuzzy search)
2. Reference layer (TUI + CLI, project v6)
3. Batch draw (JSON file input)

### Out of Scope (Future Cycles)
- Command palette: history, favorites, custom commands
- Reference layer: multiple reference images, per-layer opacity
- Batch draw: stdin pipe input, WebSocket streaming
- Layers system
- CRDT collaboration
- MCP server (full Model Context Protocol)
- Gradient maps / blending modes

---

## 8. Risks & Dependencies

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Spacebar conflict with canvas cursor | Low | Medium | Context-sensitive: only opens palette when cursor inactive |
| Reference image rendering performance | Medium | Medium | Scale once on load, cache rendered overlay; don't re-render per frame |
| Batch JSON schema complexity | Low | Low | Start minimal (7 op types), extend later |
| Project v6 backward compatibility | Low | High | Optional field, conditional version bump |

---

## 9. Success Definition

After this cycle, a human can:
```
# Press Spacebar → type "ref" → select "Set Reference Image"
# → file browser opens → select reference.png
# → reference appears dimly behind canvas
# → trace over it with drawing tools
# Press Spacebar → type "sym q" → select "Symmetry Quad"
# → symmetry activates, no shortcut memorization needed
```

And an agent can:
```bash
# Create a batch operations file
cat > ops.json << 'EOF'
{
  "operations": [
    {"op": "draw", "tool": "rect", "x1": 0, "y1": 0, "x2": 47, "y2": 31, "fg": "#FF0000"},
    {"op": "draw", "tool": "fill", "x": 24, "y": 16, "fg": "#000080"},
    {"op": "set_cell", "x": 20, "y": 15, "ch": "K", "fg": "#FFFFFF"},
    {"op": "set_cell", "x": 21, "y": 15, "ch": "A", "fg": "#FFFFFF"},
    {"op": "set_cell", "x": 22, "y": 15, "ch": "K", "fg": "#FFFFFF"},
    {"op": "set_cell", "x": 23, "y": 15, "ch": "U", "fg": "#FFFFFF"}
  ]
}
EOF

# Execute all operations atomically
kakukuma batch art.kaku --commands ops.json
# → {"operations": 6, "cells_modified": 162, "errors": 0}
```
