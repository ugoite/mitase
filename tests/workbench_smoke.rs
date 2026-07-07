use std::process::Command;

#[test]
fn workbench_projection_exposes_explicit_run_state_and_exact_anchors() {
    let output = Command::new(env!("CARGO_BIN_EXE_syu"))
        .args([
            "workbench",
            "project",
            "--workspace",
            "fixtures/v1/valid-web-app",
            "--format",
            "json",
        ])
        .output()
        .expect("run workbench projection");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let projection: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(projection["validation"]["state"], "not_run");
    assert!(projection["validation"]["phases"].is_array());
    assert!(
        projection["items"]
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item["anchors"]
                .as_array()
                .is_some_and(|anchors| !anchors.is_empty())))
    );
}
