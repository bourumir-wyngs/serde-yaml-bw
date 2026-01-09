#[test]
fn y_rhx7() {
    let yaml = r#"---
key: value
%YAML 1.2
---
"#;

    let res: Result<serde_json::Value, serde_yaml_bw::Error> = serde_yaml_bw::from_str(yaml);
    assert!(
        res.is_err(),
        "Expected parse error for invalid YAML directive placement"
    );
}
