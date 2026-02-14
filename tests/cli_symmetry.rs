mod helpers;

use helpers::*;

#[test]
fn symmetry_horizontal() {
    let f = temp_file("sym_h");
    run_ok(kakukuma().args(["new", f.to_str().unwrap(), "--width", "16", "--height", "16"]));

    // Draw with horizontal symmetry — should mirror across vertical center
    run_ok(kakukuma().args([
        "draw", "pencil", f.to_str().unwrap(), "2,5",
        "--color", "#FF0000", "--symmetry", "horizontal",
    ]));

    // Original position
    let c1 = stdout_json(&run_ok(kakukuma().args(["inspect", f.to_str().unwrap(), "2,5"])));
    assert_eq!(c1["empty"], false);
    assert_eq!(c1["fg"], "#FF0000");

    // Mirror position (width=16, mirror of x=2 is 16-1-2=13)
    let c2 = stdout_json(&run_ok(kakukuma().args(["inspect", f.to_str().unwrap(), "13,5"])));
    assert_eq!(c2["empty"], false);
    assert_eq!(c2["fg"], "#FF0000");

    cleanup(&f);
}

#[test]
fn symmetry_vertical() {
    let f = temp_file("sym_v");
    run_ok(kakukuma().args(["new", f.to_str().unwrap(), "--width", "16", "--height", "16"]));

    // Draw with vertical symmetry — should mirror across horizontal center
    run_ok(kakukuma().args([
        "draw", "pencil", f.to_str().unwrap(), "5,2",
        "--color", "#00FF00", "--symmetry", "vertical",
    ]));

    // Original position
    let c1 = stdout_json(&run_ok(kakukuma().args(["inspect", f.to_str().unwrap(), "5,2"])));
    assert_eq!(c1["empty"], false);

    // Mirror position (height=16, mirror of y=2 is 16-1-2=13)
    let c2 = stdout_json(&run_ok(kakukuma().args(["inspect", f.to_str().unwrap(), "5,13"])));
    assert_eq!(c2["empty"], false);
    assert_eq!(c2["fg"], "#00FF00");

    cleanup(&f);
}

#[test]
fn symmetry_quad() {
    let f = temp_file("sym_q");
    run_ok(kakukuma().args(["new", f.to_str().unwrap(), "--width", "16", "--height", "16"]));

    // Draw with quad symmetry — should create 4 copies
    run_ok(kakukuma().args([
        "draw", "pencil", f.to_str().unwrap(), "2,3",
        "--color", "#0000FF", "--symmetry", "quad",
    ]));

    // Original
    let c1 = stdout_json(&run_ok(kakukuma().args(["inspect", f.to_str().unwrap(), "2,3"])));
    assert_eq!(c1["empty"], false);

    // Horizontal mirror (13, 3)
    let c2 = stdout_json(&run_ok(kakukuma().args(["inspect", f.to_str().unwrap(), "13,3"])));
    assert_eq!(c2["empty"], false);

    // Vertical mirror (2, 12)
    let c3 = stdout_json(&run_ok(kakukuma().args(["inspect", f.to_str().unwrap(), "2,12"])));
    assert_eq!(c3["empty"], false);

    // Diagonal mirror (13, 12)
    let c4 = stdout_json(&run_ok(kakukuma().args(["inspect", f.to_str().unwrap(), "13,12"])));
    assert_eq!(c4["empty"], false);

    cleanup(&f);
}
