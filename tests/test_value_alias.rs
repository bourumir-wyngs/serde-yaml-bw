use serde_yaml_bw::Value;

#[test]
fn test_alias_serialization() {
    let value = Value::Alias("anchor".to_string());
    let yaml = serde_yaml_bw::to_string(&value).unwrap();
    assert_eq!(yaml, "*anchor\n");
}
