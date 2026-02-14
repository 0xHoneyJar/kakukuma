mod helpers;

use helpers::*;

#[test]
fn new_creates_file_with_default_dimensions() {
    let f = temp_file("new_default");
    let out = run_ok(kakukuma().args(["new", f.to_str().unwrap()]));
    let json = stdout_json(&out);
    assert_eq!(json["width"], 48);
    assert_eq!(json["height"], 32);
    assert!(f.exists());
    cleanup(&f);
}

#[test]
fn new_custom_dimensions() {
    let f = temp_file("new_custom");
    let out = run_ok(kakukuma().args(["new", f.to_str().unwrap(), "--width", "24", "--height", "16"]));
    let json = stdout_json(&out);
    assert_eq!(json["width"], 24);
    assert_eq!(json["height"], 16);
    cleanup(&f);
}

#[test]
fn new_clamps_dimensions() {
    let f = temp_file("new_clamp");
    let out = run_ok(kakukuma().args(["new", f.to_str().unwrap(), "--width", "4", "--height", "200"]));
    let json = stdout_json(&out);
    assert_eq!(json["width"], 8);
    assert_eq!(json["height"], 128);
    cleanup(&f);
}

#[test]
fn new_fails_if_exists() {
    let f = temp_file("new_exists");
    run_ok(kakukuma().args(["new", f.to_str().unwrap()]));
    let out = kakukuma().args(["new", f.to_str().unwrap()]).output().unwrap();
    assert!(!out.status.success());
    cleanup(&f);
}

#[test]
fn new_force_overwrites() {
    let f = temp_file("new_force");
    run_ok(kakukuma().args(["new", f.to_str().unwrap()]));
    let out = run_ok(kakukuma().args(["new", f.to_str().unwrap(), "--force", "--width", "16", "--height", "16"]));
    let json = stdout_json(&out);
    assert_eq!(json["width"], 16);
    assert_eq!(json["height"], 16);
    cleanup(&f);
}

#[test]
fn new_creates_log_file() {
    let f = temp_file("new_log");
    run_ok(kakukuma().args(["new", f.to_str().unwrap()]));
    let log = f.with_extension("kaku.log");
    assert!(log.exists());
    cleanup(&f);
}
