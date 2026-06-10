mod helpers;

use helpers::*;

#[test]
fn stats_empty_canvas() {
    let f = temp_file("stats_empty");
    run_ok(kakukuma().args(["new", f.to_str().unwrap(), "--width", "16", "--height", "16"]));

    let out = run_ok(kakukuma().args(["stats", f.to_str().unwrap()]));
    let json = stdout_json(&out);

    assert_eq!(json["canvas"]["width"], 16);
    assert_eq!(json["canvas"]["height"], 16);
    assert_eq!(json["canvas"]["total_cells"], 256);
    assert_eq!(json["fill"]["filled"], 0);
    assert_eq!(json["fill"]["empty"], 256);
    assert_eq!(json["fill"]["fill_percent"], 0.0);
    assert!(json["bounding_box"].is_null());
    assert_eq!(json["symmetry_score"]["horizontal"], 1.0);
    assert_eq!(json["symmetry_score"]["vertical"], 1.0);
    assert_eq!(json["characters"]["unique"], 0);

    cleanup(&f);
}

#[test]
fn stats_with_content() {
    let f = temp_file("stats_content");
    run_ok(kakukuma().args(["new", f.to_str().unwrap(), "--width", "16", "--height", "16"]));
    run_ok(kakukuma().args([
        "draw", "pencil", f.to_str().unwrap(), "5,5", "--color", "#FF0000",
    ]));
    run_ok(kakukuma().args([
        "draw", "pencil", f.to_str().unwrap(), "10,10", "--color", "#00FF00",
    ]));

    let out = run_ok(kakukuma().args(["stats", f.to_str().unwrap()]));
    let json = stdout_json(&out);

    assert_eq!(json["fill"]["filled"], 2);
    assert_eq!(json["colors"]["unique_fg"], 2);
    assert_eq!(json["bounding_box"]["min_x"], 5);
    assert_eq!(json["bounding_box"]["min_y"], 5);
    assert_eq!(json["bounding_box"]["max_x"], 10);
    assert_eq!(json["bounding_box"]["max_y"], 10);

    // Check color distribution has percent
    let fg_dist = json["colors"]["fg_distribution"].as_array().unwrap();
    assert_eq!(fg_dist.len(), 2);
    assert_eq!(fg_dist[0]["count"], 1);
    assert_eq!(fg_dist[0]["percent"], 50.0);

    cleanup(&f);
}

#[test]
fn stats_symmetry_scores() {
    let f = temp_file("stats_sym");
    run_ok(kakukuma().args(["new", f.to_str().unwrap(), "--width", "16", "--height", "16"]));

    // Draw symmetric pattern (horizontal): same cell on both sides
    run_ok(kakukuma().args([
        "draw", "pencil", f.to_str().unwrap(), "0,8", "--color", "#FF0000",
    ]));
    run_ok(kakukuma().args([
        "draw", "pencil", f.to_str().unwrap(), "15,8", "--color", "#FF0000",
    ]));

    let out = run_ok(kakukuma().args(["stats", f.to_str().unwrap()]));
    let json = stdout_json(&out);

    // Horizontal symmetry should be high (mirror pair matches)
    let h_sym = json["symmetry_score"]["horizontal"].as_f64().unwrap();
    assert!(h_sym > 0.99, "horizontal symmetry should be high for mirrored cells: {}", h_sym);

    cleanup(&f);
}
