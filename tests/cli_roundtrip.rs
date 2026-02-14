mod helpers;

use helpers::*;

#[test]
fn roundtrip_new_draw_inspect() {
    let f = temp_file("roundtrip");
    run_ok(kakukuma().args(["new", f.to_str().unwrap(), "--width", "16", "--height", "16"]));

    // Draw at 3 positions
    run_ok(kakukuma().args([
        "draw", "pencil", f.to_str().unwrap(), "0,0", "--color", "#FF0000",
    ]));
    run_ok(kakukuma().args([
        "draw", "pencil", f.to_str().unwrap(), "5,5", "--color", "#00FF00",
    ]));
    run_ok(kakukuma().args([
        "draw", "pencil", f.to_str().unwrap(), "10,10", "--fg", "#0000FF", "--bg", "#FFFFFF",
    ]));

    // Verify each via inspect
    let c1 = stdout_json(&run_ok(kakukuma().args(["inspect", f.to_str().unwrap(), "0,0"])));
    assert_eq!(c1["fg"], "#FF0000");
    assert_eq!(c1["empty"], false);

    let c2 = stdout_json(&run_ok(kakukuma().args(["inspect", f.to_str().unwrap(), "5,5"])));
    assert_eq!(c2["fg"], "#00FF00");

    let c3 = stdout_json(&run_ok(kakukuma().args(["inspect", f.to_str().unwrap(), "10,10"])));
    assert_eq!(c3["fg"], "#0000FF");
    assert_eq!(c3["bg"], "#FFFFFF");

    // Preview JSON should show 3 non-empty
    let preview = stdout_json(&run_ok(kakukuma().args([
        "preview", f.to_str().unwrap(), "--format", "json",
    ])));
    assert_eq!(preview["non_empty_count"], 3);

    // Stats should show 3 filled
    let stats = stdout_json(&run_ok(kakukuma().args(["stats", f.to_str().unwrap()])));
    assert_eq!(stats["fill"]["filled"], 3);
    assert_eq!(stats["characters"]["unique"], 1);

    cleanup(&f);
}

#[test]
fn roundtrip_draw_export_verify() {
    let f = temp_file("roundtrip_export");
    run_ok(kakukuma().args(["new", f.to_str().unwrap()]));
    run_ok(kakukuma().args([
        "draw", "pencil", f.to_str().unwrap(), "0,0", "--color", "#FF0000",
    ]));

    let export_path = f.with_extension("ans");
    let out = run_ok(kakukuma().args([
        "export", f.to_str().unwrap(), "--output", export_path.to_str().unwrap(),
    ]));
    let json = stdout_json(&out);
    assert_eq!(json["format"], "ansi");
    assert!(export_path.exists());

    let content = std::fs::read_to_string(&export_path).unwrap();
    assert!(!content.is_empty());

    let _ = std::fs::remove_file(&export_path);
    cleanup(&f);
}
