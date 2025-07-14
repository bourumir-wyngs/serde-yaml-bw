use serde_yaml_bw::Value;

#[test]
fn sequence_indexing() {
    let yaml = "- a\n- b";
    let value: Value = serde_yaml_bw::from_str(yaml).unwrap();

    assert_eq!(value[0], Value::from("a"));
    assert_eq!(value[2], Value::Null(None));
}

#[test]
fn mapping_indexing() {
    let yaml = "k: v";
    let value: Value = serde_yaml_bw::from_str(yaml).unwrap();

    assert_eq!(value["k"], Value::from("v"));
    assert_eq!(value["missing"], Value::Null(None));
    assert_eq!(value[0], Value::Null(None));
}
