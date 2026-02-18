use serde_yaml_gtc as serde_yaml;
// L383: Two scalar docs with trailing comments -> two documents each "foo".

#[test]
fn yaml_l383_two_scalar_docs_with_comments() {
    let y = r#"--- foo  # comment
--- foo  # comment
"#;
    let docs: Vec<String> = serde_yaml::from_multiple(y).expect("failed to parse L383");
    assert_eq!(docs, vec!["foo".to_string(), "foo".to_string()]);
}
