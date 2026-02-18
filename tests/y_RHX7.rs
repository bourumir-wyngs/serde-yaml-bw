use serde_yaml_gtc as serde_yaml;
#[test]
fn y_rhx7() {
    let yaml = r#"---
key: value
%YAML 1.2
---
"#;

    let res: Result<serde_json::Value, serde_yaml::Error> = serde_yaml::from_str(yaml);
    assert!(
        res.is_err(),
        "Expected parse error for invalid YAML directive placement"
    );
}
