use serde_yaml_gtc as serde_yaml;
#[test]
fn y_rxy3() {
    let yaml = r#"---
'
...
'
"#;

    let res: Result<serde_json::Value, serde_yaml::Error> = serde_yaml::from_str(yaml);
    assert!(
        res.is_err(),
        "Expected parse error for invalid document-end marker inside single-quoted string"
    );
}
