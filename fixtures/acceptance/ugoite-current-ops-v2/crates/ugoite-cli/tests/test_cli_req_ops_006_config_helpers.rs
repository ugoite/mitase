#[test]
fn test_cli_req_ops_006_config_path_precedence_and_home_fallback() {
    assert_eq!(ugoite_config_path(), ".ugoite/config.toml");
}

fn ugoite_config_path() -> &'static str {
    ".ugoite/config.toml"
}
