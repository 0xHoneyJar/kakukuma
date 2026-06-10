mod helpers;

use helpers::*;

#[test]
fn diff_identical_files() {
    let f1 = temp_file("diff_id1");
    let f2 = temp_file("diff_id2");
    run_ok(kakukuma().args(["new", f1.to_str().unwrap(), "--width", "16", "--height", "16"]));
    run_ok(kakukuma().args(["new", f2.to_str().unwrap(), "--width", "16", "--height", "16"]));

    let out = run_ok(kakukuma().args(["diff", f1.to_str().unwrap(), f2.to_str().unwrap()]));
    let json = stdout_json(&out);
    assert_eq!(json["added"], 0);
    assert_eq!(json["removed"], 0);
    assert_eq!(json["modified"], 0);
    assert_eq!(json["unchanged"], 256);
    assert_eq!(json["changes"].as_array().unwrap().len(), 0);

    cleanup(&f1);
    cleanup(&f2);
}

#[test]
fn diff_with_changes() {
    let f1 = temp_file("diff_ch1");
    let f2 = temp_file("diff_ch2");
    run_ok(kakukuma().args(["new", f1.to_str().unwrap(), "--width", "16", "--height", "16"]));
    run_ok(kakukuma().args(["new", f2.to_str().unwrap(), "--width", "16", "--height", "16"]));
    run_ok(kakukuma().args([
        "draw", "pencil", f2.to_str().unwrap(), "5,5", "--color", "#FF0000",
    ]));

    let out = run_ok(kakukuma().args(["diff", f1.to_str().unwrap(), f2.to_str().unwrap()]));
    let json = stdout_json(&out);
    assert_eq!(json["added"], 1);
    assert_eq!(json["removed"], 0);
    assert_eq!(json["unchanged"], 255);

    let changes = json["changes"].as_array().unwrap();
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0]["x"], 5);
    assert_eq!(changes[0]["y"], 5);

    cleanup(&f1);
    cleanup(&f2);
}

#[test]
fn diff_before_mode() {
    let f = temp_file("diff_before");
    run_ok(kakukuma().args(["new", f.to_str().unwrap(), "--width", "16", "--height", "16"]));
    run_ok(kakukuma().args([
        "draw", "pencil", f.to_str().unwrap(), "3,3", "--color", "#00FF00",
    ]));

    let out = run_ok(kakukuma().args(["diff", f.to_str().unwrap(), "--before"]));
    let json = stdout_json(&out);
    assert_eq!(json["added"], 1);
    assert_eq!(json["changes"].as_array().unwrap().len(), 1);

    cleanup(&f);
}

#[test]
fn diff_before_empty_log_fails() {
    let f = temp_file("diff_before_empty");
    run_ok(kakukuma().args(["new", f.to_str().unwrap()]));
    let out = kakukuma()
        .args(["diff", f.to_str().unwrap(), "--before"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    cleanup(&f);
}
