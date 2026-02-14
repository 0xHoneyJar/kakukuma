mod helpers;

use helpers::*;

fn create_canvas(prefix: &str) -> std::path::PathBuf {
    let f = temp_file(prefix);
    run_ok(kakukuma().args(["new", f.to_str().unwrap(), "--width", "16", "--height", "16"]));
    f
}

#[test]
fn draw_pencil() {
    let f = create_canvas("draw_pencil");
    let out = run_ok(kakukuma().args([
        "draw", "pencil", f.to_str().unwrap(), "5,5", "--color", "#FF0000",
    ]));
    let json = stdout_json(&out);
    assert_eq!(json["ok"], true);
    assert_eq!(json["tool"], "pencil");
    assert_eq!(json["cells_modified"], 1);

    // Verify via inspect
    let out2 = run_ok(kakukuma().args(["inspect", f.to_str().unwrap(), "5,5"]));
    let cell = stdout_json(&out2);
    assert_eq!(cell["fg"], "#FF0000");
    assert_eq!(cell["empty"], false);

    cleanup(&f);
}

#[test]
fn draw_eraser() {
    let f = create_canvas("draw_eraser");
    run_ok(kakukuma().args([
        "draw", "pencil", f.to_str().unwrap(), "3,3", "--color", "#FF0000",
    ]));
    let out = run_ok(kakukuma().args([
        "draw", "eraser", f.to_str().unwrap(), "3,3",
    ]));
    let json = stdout_json(&out);
    assert_eq!(json["ok"], true);
    assert_eq!(json["tool"], "eraser");

    // Verify cell is cleared
    let out2 = run_ok(kakukuma().args(["inspect", f.to_str().unwrap(), "3,3"]));
    let cell = stdout_json(&out2);
    assert_eq!(cell["empty"], true);

    cleanup(&f);
}

#[test]
fn draw_line() {
    let f = create_canvas("draw_line");
    let out = run_ok(kakukuma().args([
        "draw", "line", f.to_str().unwrap(), "0,0", "15,15", "--color", "#00FF00",
    ]));
    let json = stdout_json(&out);
    assert_eq!(json["ok"], true);
    assert_eq!(json["tool"], "line");
    assert!(json["cells_modified"].as_u64().unwrap() > 0);

    // Verify endpoints
    let out_start = run_ok(kakukuma().args(["inspect", f.to_str().unwrap(), "0,0"]));
    assert_eq!(stdout_json(&out_start)["fg"], "#00FF00");

    let out_end = run_ok(kakukuma().args(["inspect", f.to_str().unwrap(), "15,15"]));
    assert_eq!(stdout_json(&out_end)["fg"], "#00FF00");

    cleanup(&f);
}

#[test]
fn draw_rect_outline() {
    let f = create_canvas("draw_rect");
    let out = run_ok(kakukuma().args([
        "draw", "rect", f.to_str().unwrap(), "2,2", "5,5", "--color", "#0000FF",
    ]));
    let json = stdout_json(&out);
    assert_eq!(json["ok"], true);
    assert_eq!(json["tool"], "rect");

    // Corner should be filled
    let corner = run_ok(kakukuma().args(["inspect", f.to_str().unwrap(), "2,2"]));
    assert_eq!(stdout_json(&corner)["fg"], "#0000FF");

    // Interior should be empty (outline only)
    let interior = run_ok(kakukuma().args(["inspect", f.to_str().unwrap(), "3,3"]));
    assert_eq!(stdout_json(&interior)["empty"], true);

    cleanup(&f);
}

#[test]
fn draw_rect_filled() {
    let f = create_canvas("draw_rect_filled");
    run_ok(kakukuma().args([
        "draw", "rect", f.to_str().unwrap(), "2,2", "5,5", "--color", "#0000FF", "--filled",
    ]));

    // Interior should be filled
    let interior = run_ok(kakukuma().args(["inspect", f.to_str().unwrap(), "3,3"]));
    assert_eq!(stdout_json(&interior)["empty"], false);
    assert_eq!(stdout_json(&interior)["fg"], "#0000FF");

    cleanup(&f);
}

#[test]
fn draw_fill() {
    let f = create_canvas("draw_fill");
    let out = run_ok(kakukuma().args([
        "draw", "fill", f.to_str().unwrap(), "0,0", "--color", "#FFFF00",
    ]));
    let json = stdout_json(&out);
    assert_eq!(json["ok"], true);
    assert_eq!(json["tool"], "fill");
    // Flood fill on empty 16x16 = 256 cells
    assert_eq!(json["cells_modified"], 256);

    cleanup(&f);
}

#[test]
fn draw_eyedropper() {
    let f = create_canvas("draw_eye");
    run_ok(kakukuma().args([
        "draw", "pencil", f.to_str().unwrap(), "7,7", "--color", "#ABCDEF",
    ]));

    let out = run_ok(kakukuma().args([
        "draw", "eyedropper", f.to_str().unwrap(), "7,7",
    ]));
    let json = stdout_json(&out);
    assert_eq!(json["fg"], "#ABCDEF");

    cleanup(&f);
}

#[test]
fn draw_invalid_color_fails() {
    let f = create_canvas("draw_bad_color");
    let out = kakukuma()
        .args(["draw", "pencil", f.to_str().unwrap(), "5,5", "--color", "not-a-color"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Invalid hex color"));
    assert!(stderr.contains("#RRGGBB"));
    cleanup(&f);
}

#[test]
fn draw_invalid_coords_fails() {
    let f = create_canvas("draw_invalid");
    let out = kakukuma()
        .args(["draw", "pencil", f.to_str().unwrap(), "100,100", "--color", "#FF0000"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("exceeds"));
    cleanup(&f);
}
