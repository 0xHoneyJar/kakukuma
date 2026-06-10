mod helpers;

use helpers::*;

fn create_canvas_with_art(prefix: &str) -> std::path::PathBuf {
    let f = temp_file(prefix);
    run_ok(kakukuma().args(["new", f.to_str().unwrap(), "--width", "16", "--height", "16"]));
    run_ok(kakukuma().args([
        "draw", "pencil", f.to_str().unwrap(), "5,5", "--color", "#FF0000",
    ]));
    f
}

#[test]
fn preview_ansi_non_empty() {
    let f = create_canvas_with_art("preview_ansi");
    let out = run_ok(kakukuma().args(["preview", f.to_str().unwrap()]));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.is_empty());
    // Should contain ANSI escape codes
    assert!(stdout.contains("\x1b["));
    cleanup(&f);
}

#[test]
fn preview_json_valid() {
    let f = create_canvas_with_art("preview_json");
    let out = run_ok(kakukuma().args(["preview", f.to_str().unwrap(), "--format", "json"]));
    let json = stdout_json(&out);
    assert_eq!(json["width"], 16);
    assert_eq!(json["height"], 16);
    assert_eq!(json["non_empty_count"], 1);
    assert!(json["cells"].is_array());
    cleanup(&f);
}

#[test]
fn preview_region_filtering() {
    let f = create_canvas_with_art("preview_region");
    let out = run_ok(kakukuma().args([
        "preview", f.to_str().unwrap(), "--format", "json", "--region", "4,4,6,6",
    ]));
    let json = stdout_json(&out);
    assert_eq!(json["non_empty_count"], 1);
    cleanup(&f);
}

#[test]
fn preview_plain_non_empty() {
    let f = create_canvas_with_art("preview_plain");
    let out = run_ok(kakukuma().args(["preview", f.to_str().unwrap(), "--format", "plain"]));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.is_empty());
    // Should NOT contain ANSI escapes
    assert!(!stdout.contains("\x1b["));
    cleanup(&f);
}
