use assert_cmd::Command;

#[test]
fn root_cli_does_not_expose_transitional_workbench() {
    let assert = Command::cargo_bin("mitase")
        .expect("binary should build")
        .arg("--help")
        .assert()
        .success();

    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    assert!(
        !stdout
            .lines()
            .any(|line| line.trim_start().starts_with("workbench"))
    );
}
