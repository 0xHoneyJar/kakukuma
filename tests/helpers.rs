use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_ID: AtomicUsize = AtomicUsize::new(0);

pub fn kakukuma() -> Command {
    Command::new(env!("CARGO_BIN_EXE_kakukuma"))
}

pub fn temp_file(prefix: &str) -> PathBuf {
    let id = TEST_ID.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir();
    dir.join(format!("kaku_integ_{}_{}_{}.kaku", prefix, std::process::id(), id))
}

pub fn run_ok(cmd: &mut Command) -> Output {
    let out = cmd.output().expect("failed to execute");
    assert!(
        out.status.success(),
        "command failed: {}\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

pub fn stdout_json(out: &Output) -> serde_json::Value {
    let s = String::from_utf8_lossy(&out.stdout);
    serde_json::from_str(&s).unwrap_or_else(|e| {
        panic!("invalid JSON in stdout: {}\nraw: {}", e, s);
    })
}

pub fn cleanup(path: &PathBuf) {
    let _ = std::fs::remove_file(path);
    let log = path.with_extension("kaku.log");
    let _ = std::fs::remove_file(&log);
}
