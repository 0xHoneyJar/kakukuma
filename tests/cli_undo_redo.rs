mod helpers;

use helpers::*;

#[test]
fn undo_reverses_draw() {
    let f = temp_file("undo_basic");
    run_ok(kakukuma().args(["new", f.to_str().unwrap(), "--width", "16", "--height", "16"]));
    run_ok(kakukuma().args([
        "draw", "pencil", f.to_str().unwrap(), "5,5", "--color", "#FF0000",
    ]));

    // Verify drawn
    let before = stdout_json(&run_ok(kakukuma().args(["inspect", f.to_str().unwrap(), "5,5"])));
    assert_eq!(before["empty"], false);

    // Undo
    let out = run_ok(kakukuma().args(["undo", f.to_str().unwrap()]));
    let json = stdout_json(&out);
    assert_eq!(json["ok"], true);
    assert_eq!(json["undone"], 1);

    // Verify undone
    let after = stdout_json(&run_ok(kakukuma().args(["inspect", f.to_str().unwrap(), "5,5"])));
    assert_eq!(after["empty"], true);

    cleanup(&f);
}

#[test]
fn redo_restores_after_undo() {
    let f = temp_file("redo_basic");
    run_ok(kakukuma().args(["new", f.to_str().unwrap(), "--width", "16", "--height", "16"]));
    run_ok(kakukuma().args([
        "draw", "pencil", f.to_str().unwrap(), "5,5", "--color", "#FF0000",
    ]));
    run_ok(kakukuma().args(["undo", f.to_str().unwrap()]));

    // Redo
    let out = run_ok(kakukuma().args(["redo", f.to_str().unwrap()]));
    let json = stdout_json(&out);
    assert_eq!(json["ok"], true);
    assert_eq!(json["redone"], 1);

    // Verify restored
    let after = stdout_json(&run_ok(kakukuma().args(["inspect", f.to_str().unwrap(), "5,5"])));
    assert_eq!(after["empty"], false);
    assert_eq!(after["fg"], "#FF0000");

    cleanup(&f);
}

#[test]
fn multi_undo() {
    let f = temp_file("multi_undo");
    run_ok(kakukuma().args(["new", f.to_str().unwrap(), "--width", "16", "--height", "16"]));
    run_ok(kakukuma().args([
        "draw", "pencil", f.to_str().unwrap(), "1,1", "--color", "#FF0000",
    ]));
    run_ok(kakukuma().args([
        "draw", "pencil", f.to_str().unwrap(), "2,2", "--color", "#00FF00",
    ]));
    run_ok(kakukuma().args([
        "draw", "pencil", f.to_str().unwrap(), "3,3", "--color", "#0000FF",
    ]));

    // Undo all 3
    let out = run_ok(kakukuma().args(["undo", f.to_str().unwrap(), "--count", "3"]));
    let json = stdout_json(&out);
    assert_eq!(json["undone"], 3);

    // All should be empty
    assert_eq!(stdout_json(&run_ok(kakukuma().args(["inspect", f.to_str().unwrap(), "1,1"])))["empty"], true);
    assert_eq!(stdout_json(&run_ok(kakukuma().args(["inspect", f.to_str().unwrap(), "2,2"])))["empty"], true);
    assert_eq!(stdout_json(&run_ok(kakukuma().args(["inspect", f.to_str().unwrap(), "3,3"])))["empty"], true);

    cleanup(&f);
}

#[test]
fn new_draw_clears_redo_stack() {
    let f = temp_file("redo_cleared");
    run_ok(kakukuma().args(["new", f.to_str().unwrap(), "--width", "16", "--height", "16"]));
    run_ok(kakukuma().args([
        "draw", "pencil", f.to_str().unwrap(), "1,1", "--color", "#FF0000",
    ]));
    run_ok(kakukuma().args(["undo", f.to_str().unwrap()]));

    // Draw something new — should clear redo stack
    run_ok(kakukuma().args([
        "draw", "pencil", f.to_str().unwrap(), "2,2", "--color", "#00FF00",
    ]));

    // Redo should fail
    let out = kakukuma()
        .args(["redo", f.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!out.status.success());

    cleanup(&f);
}

#[test]
fn undo_on_empty_fails() {
    let f = temp_file("undo_empty");
    run_ok(kakukuma().args(["new", f.to_str().unwrap()]));
    let out = kakukuma()
        .args(["undo", f.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!out.status.success());
    cleanup(&f);
}
