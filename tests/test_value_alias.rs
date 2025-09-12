use serde_yaml_bw::Value;

#[test]
fn test_alias_serialization() {
    let value = Value::Alias("anchor".to_string());
    let yaml = serde_yaml_bw::to_string(&value).unwrap();
    assert_eq!(yaml, "*anchor\n");
}

#[test]
fn test_alias_in_sequence_resolves() {
    use serde_yaml_bw::value::Sequence;

    let value = Value::Sequence(Sequence {
        anchor: None,
        elements: vec![
            Value::Number(1.into(), Some("id".to_string())),
            Value::Alias("id".to_string()),
        ],
    });

    let yaml = serde_yaml_bw::to_string(&value).unwrap();
    assert_eq!(yaml, "- &id 1\n- *id\n");
}

#[test]
fn test_alias_in_mapping_branch() {
    use serde_yaml_bw::Mapping;

    let mut mapping = Mapping::new();
    mapping.insert(
        Value::String("a".to_string(), None),
        Value::String("foo".to_string(), Some("id".to_string())),
    );
    mapping.insert(
        Value::String("b".to_string(), None),
        Value::Alias("id".to_string()),
    );

    let value = Value::Mapping(mapping);
    let yaml = serde_yaml_bw::to_string(&value).unwrap();
    assert_eq!(yaml, "a: &id foo\nb: *id\n");
}
