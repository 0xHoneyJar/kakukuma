use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::cell::Rgb;
use crate::history::CellMutation;

const MAX_LOG_ENTRIES: usize = 256;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogHeader {
    pub pointer: usize,
    pub total: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogEntry {
    pub timestamp: String,
    pub command: String,
    pub mutations: Vec<LogMutation>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogMutation {
    pub x: usize,
    pub y: usize,
    pub old: LogCell,
    pub new: LogCell,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LogCell {
    pub ch: char,
    pub fg: Option<String>,
    pub bg: Option<String>,
}

impl LogCell {
    pub fn from_cell(cell: &crate::cell::Cell) -> Self {
        LogCell {
            ch: cell.ch,
            fg: cell.fg.map(|c| c.name()),
            bg: cell.bg.map(|c| c.name()),
        }
    }
}

impl LogMutation {
    pub fn from_cell_mutation(m: &CellMutation) -> Self {
        LogMutation {
            x: m.x,
            y: m.y,
            old: LogCell::from_cell(&m.old),
            new: LogCell::from_cell(&m.new),
        }
    }
}

fn rgb_from_hex(s: &str) -> Option<Rgb> {
    crate::cell::parse_hex_color(s)
}

impl LogCell {
    pub fn to_cell(&self) -> crate::cell::Cell {
        crate::cell::Cell {
            ch: self.ch,
            fg: self.fg.as_deref().and_then(rgb_from_hex),
            bg: self.bg.as_deref().and_then(rgb_from_hex),
        }
    }
}

/// Derive log path from .kaku path: "art.kaku" -> "art.kaku.log"
pub fn log_path(kaku_path: &Path) -> PathBuf {
    let mut p = kaku_path.as_os_str().to_os_string();
    p.push(".log");
    PathBuf::from(p)
}

/// Initialize an empty log file with a header line.
pub fn init_log(path: &Path) -> io::Result<()> {
    let header = LogHeader { pointer: 0, total: 0 };
    let line = serde_json::to_string(&header)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    std::fs::write(path, format!("{}\n", line))
}

/// Read the header and all entries from the log file.
fn read_raw(path: &Path) -> io::Result<(LogHeader, Vec<LogEntry>)> {
    if !path.exists() {
        return Ok((LogHeader { pointer: 0, total: 0 }, Vec::new()));
    }
    let file = std::fs::File::open(path)?;
    let reader = io::BufReader::new(file);
    let mut lines = reader.lines();

    let header_line = match lines.next() {
        Some(Ok(line)) => line,
        Some(Err(e)) => return Err(e),
        None => return Ok((LogHeader { pointer: 0, total: 0 }, Vec::new())),
    };
    let header: LogHeader = serde_json::from_str(&header_line)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Log header corrupt: {}", e)))?;

    let mut entries = Vec::new();
    for line_result in lines {
        let line = line_result?;
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<LogEntry>(&line) {
            Ok(entry) => entries.push(entry),
            Err(e) => {
                eprintln!("Warning: Skipping corrupt log entry: {}", e);
            }
        }
    }

    Ok((header, entries))
}

/// Write header and entries back to the log file.
fn write_raw(path: &Path, header: &LogHeader, entries: &[LogEntry]) -> io::Result<()> {
    let mut file = std::fs::File::create(path)?;
    let header_json = serde_json::to_string(header)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    writeln!(file, "{}", header_json)?;
    for entry in entries {
        let entry_json = serde_json::to_string(entry)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        writeln!(file, "{}", entry_json)?;
    }
    Ok(())
}

/// Append an entry to the operation log.
/// Truncates redo entries (entries after undo pointer) and prunes to MAX_LOG_ENTRIES.
pub fn append(path: &Path, entry: LogEntry) -> io::Result<()> {
    let (header, mut entries) = read_raw(path)?;

    // Truncate undone entries (everything after pointer)
    entries.truncate(header.pointer);

    // Append new entry
    entries.push(entry);

    // Prune to max
    if entries.len() > MAX_LOG_ENTRIES {
        let excess = entries.len() - MAX_LOG_ENTRIES;
        entries.drain(0..excess);
    }

    let new_header = LogHeader {
        pointer: entries.len(),
        total: entries.len(),
    };

    write_raw(path, &new_header, &entries)
}

/// Read all active entries (up to pointer).
pub fn active_entries(path: &Path) -> io::Result<Vec<LogEntry>> {
    let (header, entries) = read_raw(path)?;
    Ok(entries.into_iter().take(header.pointer).collect())
}

/// Read all entries (active + undone) with header.
pub fn read_log(path: &Path) -> io::Result<(LogHeader, Vec<LogEntry>)> {
    read_raw(path)
}

/// Pop the last N entries for undo. Returns the popped entries (in reverse order).
/// Moves the undo pointer rather than deleting (enables redo).
pub fn pop_for_undo(path: &Path, count: usize) -> io::Result<Vec<LogEntry>> {
    let (header, entries) = read_raw(path)?;

    if header.pointer == 0 {
        return Err(io::Error::new(io::ErrorKind::Other, "Nothing to undo"));
    }

    let actual_count = count.min(header.pointer);
    let new_pointer = header.pointer - actual_count;

    // Collect the undone entries
    let undone: Vec<LogEntry> = entries[new_pointer..header.pointer].to_vec();

    let new_header = LogHeader {
        pointer: new_pointer,
        total: entries.len(),
    };

    write_raw(path, &new_header, &entries)?;
    Ok(undone)
}

/// Restore the last N undone entries for redo.
pub fn push_for_redo(path: &Path, count: usize) -> io::Result<Vec<LogEntry>> {
    let (header, entries) = read_raw(path)?;

    let undone_count = entries.len() - header.pointer;
    if undone_count == 0 {
        return Err(io::Error::new(io::ErrorKind::Other, "Nothing to redo"));
    }

    let actual_count = count.min(undone_count);
    let new_pointer = header.pointer + actual_count;

    // Collect the redone entries
    let redone: Vec<LogEntry> = entries[header.pointer..new_pointer].to_vec();

    let new_header = LogHeader {
        pointer: new_pointer,
        total: entries.len(),
    };

    write_raw(path, &new_header, &entries)?;
    Ok(redone)
}

/// Create a LogEntry from CellMutations.
pub fn make_entry(command: &str, mutations: &[CellMutation]) -> LogEntry {
    LogEntry {
        timestamp: crate::project::now_iso8601(),
        command: command.to_string(),
        mutations: mutations.iter().map(LogMutation::from_cell_mutation).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::{blocks, Cell, Rgb};
    use crate::history::CellMutation;

    use std::sync::atomic::{AtomicUsize, Ordering};
    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn test_log_path() -> PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir();
        dir.join(format!("kaku_oplog_test_{}_{}.kaku.log", std::process::id(), id))
    }

    fn make_mutation(x: usize, y: usize) -> CellMutation {
        CellMutation {
            x,
            y,
            old: Cell::default(),
            new: Cell {
                ch: blocks::FULL,
                fg: Some(Rgb::new(255, 0, 0)),
                bg: None,
            },
        }
    }

    fn make_entry_helper(cmd: &str, x: usize, y: usize) -> LogEntry {
        make_entry(cmd, &[make_mutation(x, y)])
    }

    #[test]
    fn test_log_path_derivation() {
        let p = log_path(Path::new("art.kaku"));
        assert_eq!(p, PathBuf::from("art.kaku.log"));

        let p = log_path(Path::new("/tmp/my art.kaku"));
        assert_eq!(p, PathBuf::from("/tmp/my art.kaku.log"));
    }

    #[test]
    fn test_init_and_read_empty() {
        let path = test_log_path();
        init_log(&path).unwrap();

        let (header, entries) = read_log(&path).unwrap();
        assert_eq!(header.pointer, 0);
        assert_eq!(header.total, 0);
        assert!(entries.is_empty());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_append_and_read() {
        let path = test_log_path();
        init_log(&path).unwrap();

        let entry = make_entry_helper("draw pencil 5,5", 5, 5);
        append(&path, entry).unwrap();

        let (header, entries) = read_log(&path).unwrap();
        assert_eq!(header.pointer, 1);
        assert_eq!(header.total, 1);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].command, "draw pencil 5,5");
        assert_eq!(entries[0].mutations.len(), 1);
        assert_eq!(entries[0].mutations[0].x, 5);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_undo_pointer_movement() {
        let path = test_log_path();
        init_log(&path).unwrap();

        append(&path, make_entry_helper("draw pencil 0,0", 0, 0)).unwrap();
        append(&path, make_entry_helper("draw pencil 1,1", 1, 1)).unwrap();
        append(&path, make_entry_helper("draw pencil 2,2", 2, 2)).unwrap();

        // Undo last one
        let undone = pop_for_undo(&path, 1).unwrap();
        assert_eq!(undone.len(), 1);
        assert_eq!(undone[0].command, "draw pencil 2,2");

        let (header, _) = read_log(&path).unwrap();
        assert_eq!(header.pointer, 2);
        assert_eq!(header.total, 3);

        // Active entries should be 2
        let active = active_entries(&path).unwrap();
        assert_eq!(active.len(), 2);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_redo() {
        let path = test_log_path();
        init_log(&path).unwrap();

        append(&path, make_entry_helper("draw pencil 0,0", 0, 0)).unwrap();
        append(&path, make_entry_helper("draw pencil 1,1", 1, 1)).unwrap();

        pop_for_undo(&path, 1).unwrap();

        let redone = push_for_redo(&path, 1).unwrap();
        assert_eq!(redone.len(), 1);
        assert_eq!(redone[0].command, "draw pencil 1,1");

        let (header, _) = read_log(&path).unwrap();
        assert_eq!(header.pointer, 2);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_new_append_clears_redo() {
        let path = test_log_path();
        init_log(&path).unwrap();

        append(&path, make_entry_helper("cmd 1", 0, 0)).unwrap();
        append(&path, make_entry_helper("cmd 2", 1, 1)).unwrap();
        append(&path, make_entry_helper("cmd 3", 2, 2)).unwrap();

        // Undo 2 entries
        pop_for_undo(&path, 2).unwrap();

        // New append should clear the 2 undone entries
        append(&path, make_entry_helper("cmd 4", 3, 3)).unwrap();

        let (header, entries) = read_log(&path).unwrap();
        assert_eq!(header.pointer, 2);
        assert_eq!(header.total, 2);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].command, "cmd 1");
        assert_eq!(entries[1].command, "cmd 4");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_pruning_at_256() {
        let path = test_log_path();
        init_log(&path).unwrap();

        for i in 0..300 {
            append(&path, make_entry_helper(&format!("cmd {}", i), i % 48, 0)).unwrap();
        }

        let (header, entries) = read_log(&path).unwrap();
        assert_eq!(entries.len(), MAX_LOG_ENTRIES);
        assert_eq!(header.pointer, MAX_LOG_ENTRIES);
        // Should have the last 256 entries
        assert_eq!(entries[0].command, "cmd 44");
        assert_eq!(entries[entries.len() - 1].command, "cmd 299");

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_undo_on_empty_log() {
        let path = test_log_path();
        init_log(&path).unwrap();

        let result = pop_for_undo(&path, 1);
        assert!(result.is_err());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_redo_with_nothing_undone() {
        let path = test_log_path();
        init_log(&path).unwrap();

        append(&path, make_entry_helper("cmd 1", 0, 0)).unwrap();

        let result = push_for_redo(&path, 1);
        assert!(result.is_err());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_multi_undo() {
        let path = test_log_path();
        init_log(&path).unwrap();

        for i in 0..5 {
            append(&path, make_entry_helper(&format!("cmd {}", i), i, 0)).unwrap();
        }

        let undone = pop_for_undo(&path, 3).unwrap();
        assert_eq!(undone.len(), 3);

        let (header, _) = read_log(&path).unwrap();
        assert_eq!(header.pointer, 2);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_log_cell_roundtrip() {
        let cell = Cell {
            ch: blocks::FULL,
            fg: Some(Rgb::new(255, 128, 0)),
            bg: Some(Rgb::new(0, 0, 255)),
        };
        let log_cell = LogCell::from_cell(&cell);
        let restored = log_cell.to_cell();
        assert_eq!(restored.ch, cell.ch);
        assert_eq!(restored.fg, cell.fg);
        assert_eq!(restored.bg, cell.bg);
    }

    #[test]
    fn test_nonexistent_log_reads_empty() {
        let path = PathBuf::from("/tmp/nonexistent_oplog_test.kaku.log");
        let _ = std::fs::remove_file(&path);
        let (header, entries) = read_raw(&path).unwrap();
        assert_eq!(header.pointer, 0);
        assert_eq!(entries.len(), 0);
    }
}
