use endpoint_protection_agent::config::Config;

#[test]
fn test_parse_config() {
    let toml = r#"
        agent_id = "test-agent"
        check_interval_seconds = 10
        server_url = "http://example.com"
        paths_to_monitor = ["/tmp", "/home/test"]
    "#;

    let cfg: Config = toml::from_str(toml).unwrap();
    assert_eq!(cfg.agent_id, "test-agent");
    assert_eq!(cfg.check_interval_seconds, 10);
    assert_eq!(cfg.server_url.as_deref(), Some("http://example.com"));
    assert_eq!(cfg.paths_to_monitor.len(), 2);
}

#[test]
fn test_parse_config_missing_optional_url() {
    let toml = r#"
        agent_id = "test-agent"
        check_interval_seconds = 5
        paths_to_monitor = ["/tmp"]
    "#;

    let cfg: Config = toml::from_str(toml).unwrap();
    assert_eq!(cfg.agent_id, "test-agent");
    assert_eq!(cfg.server_url, None);
}
