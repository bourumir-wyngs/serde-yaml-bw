#[test]
fn y_rxy3() {
    let yaml = r#"---
'
...
'
"#;

    let res: Result<serde_json::Value, serde_yaml_bw::Error> = serde_yaml_bw::from_str(yaml);
    assert!(
        res.is_err(),
        "Expected parse error for invalid document-end marker inside single-quoted string"
    );
}
