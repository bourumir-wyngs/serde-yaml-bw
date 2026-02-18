use serde_yaml_gtc as serde_yaml;
// MYW6: Block Scalar Strip (|-)
#[test]
fn yaml_myw6_block_scalar_strip() {
    let y = r#"|-
 ab
 
 
...
"#;
    let s: String = serde_yaml::from_str(y).expect("failed to parse MYW6");
    assert_eq!(s.as_str(), "ab");
}
