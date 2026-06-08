use assert_cmd::Command;

#[test]
fn workbench_help_lists_browser_launch_options() {
    let assert = Command::cargo_bin("syu")
        .expect("binary should build")
        .args(["workbench", "--help"])
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    assert!(stdout.contains("Run the Workbench server"));
    assert!(stdout.contains("--bind"));
    assert!(stdout.contains("--port"));
    assert!(stdout.contains("--allow-remote-bind"));
    assert!(stdout.contains("--show-log"));
}
