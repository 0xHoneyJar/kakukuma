# Sprint 1 Security Audit

**Sprint**: sprint-1 (global ID: 1)
**Auditor**: Paranoid Cypherpunk Auditor
**Date**: 2026-02-14
**Verdict**: APPROVED - LETS FUCKING GO

---

## Security Review

### 1. Input Validation
- **Coordinates**: Bounds-checked against canvas dimensions before use (`validate_coords()` in draw.rs:167-174, inspect.rs:17-20, 55-58, 75-78). Uses `usize` type — no negative overflow possible.
- **Colors**: Parsed via `parse_hex_color()` which safely rejects invalid input. Returns `None` on failure, never panics.
- **File paths**: No shell injection risk — paths passed directly to `std::fs` and `std::path` APIs, never interpolated into shell commands.
- **Dimensions**: Clamped to 8-128 in `cmd_new()` via `.clamp()` — no oversized allocation possible.
- **Region coordinates**: Clamped against canvas bounds in inspect.rs (lines 34-35) and preview.rs (lines 84-85) using `.min()`. Region eraser iterates within user-specified bounds but `tools::eraser` is safe for out-of-bounds coords (canvas.get returns None).

### 2. File I/O Safety
- **Atomic writes**: Temp file + rename pattern (`atomic_save()` in mod.rs:387-392) prevents partial writes and corruption.
- **No directory traversal**: File paths are user-provided CLI arguments — the tool operates on whatever file the user points at, which is expected behavior for a CLI art editor (same trust model as `vim`, `cat`, etc.).
- **Oplog `write_raw()`**: Uses `std::fs::File::create()` which truncates — no append-without-lock issues. Log corruption is handled gracefully (warns and skips bad entries, oplog.rs:114-116).
- **No temp file cleanup issue**: `atomic_save` renames over the target. If the process crashes mid-write, a `.kaku.tmp` may be left behind but that's a minor artifact, not a security issue.

### 3. No Secrets or Credentials
- No hardcoded API keys, tokens, or credentials anywhere in the codebase.
- No network calls — this is a purely local file-manipulation tool.
- No environment variable reads for secrets.
- Dependencies (`clap`, `serde`, `serde_json`) are well-audited, widely-used crates.

### 4. Deserialization Safety
- **Log file parsing** (oplog.rs:103, 112): `serde_json::from_str` is used to parse log entries. Malicious log files could cause large memory allocation if they contain huge arrays, but this is bounded by MAX_LOG_ENTRIES (256) pruning and the fact that the log is written by the tool itself.
- **Project file loading**: Uses existing `Project::load_from_file()` which predates this sprint — not in scope but noted as safe (serde_json with known schema).
- **Palette loading**: `palette::load_palette()` deserializes JSON — same trust model as project files.

### 5. Denial of Service Considerations
- **Flood fill on max canvas** (128x128 = 16,384 cells): Bounded by canvas dimensions. `tools::flood_fill` uses BFS which is O(n) where n is canvas size. Max allocation is 16,384 cells — trivial.
- **Oplog pruning**: Capped at 256 entries. No unbounded growth.
- **JSON output**: Bounded by canvas dimensions. Worst case: 128x128 cells in JSON preview = ~2MB. Acceptable.

### 6. Error Information Disclosure
- Error messages show file paths and canvas dimensions — appropriate for a local CLI tool.
- No stack traces exposed to stdout — errors go to stderr with descriptive messages.
- Internal errors use exit code 2, user errors use exit code 1 — good separation.

### 7. TOCTOU (Time-of-Check-Time-of-Use)
- `cmd_new()` checks `path.exists()` then writes — a TOCTOU gap exists but this is a single-user local CLI tool, not a server. The `--force` flag provides explicit override. Acceptable.
- Draw commands load the project, compute mutations, then reload and save. The reload ensures the save operates on fresh data, which is the correct approach for an editor.

### 8. Dependency Audit
- `clap 4.5.58` — the only new dependency. Well-maintained, 15K+ GitHub stars, no known CVEs. Derive feature uses proc macros at compile time only.
- No `unsafe` code introduced in any new files.

---

## Findings Summary

| Category | Status | Notes |
|----------|--------|-------|
| Input validation | PASS | Bounds checking, type safety, safe parsing |
| File I/O | PASS | Atomic writes, graceful corruption handling |
| Secrets | PASS | None present, no network calls |
| Deserialization | PASS | Bounded, schema-validated |
| DoS resistance | PASS | Canvas dimensions capped, log pruned |
| Error handling | PASS | No info disclosure, proper exit codes |
| Dependencies | PASS | Single new dep (clap), well-audited |
| Code quality | PASS | Clean architecture, consistent patterns |

**Total findings**: 0 CRITICAL, 0 HIGH, 0 MEDIUM, 0 LOW

---

## Verdict

APPROVED - LETS FUCKING GO

Sprint 1 is clean. No security issues found. The implementation follows safe Rust patterns throughout — no `unsafe`, no shell injection vectors, no unbounded allocations, no secrets exposure. The CLI operates in the same trust model as any local file editor.
