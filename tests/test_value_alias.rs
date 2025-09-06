use serde_yaml_bw::{Mapping, Sequence, Value};

#[test]
fn test_alias_serialization() {
    let value = Value::Alias("anchor".to_string());
    let yaml = serde_yaml_bw::to_string(&value).unwrap();
    assert_eq!(yaml, "*anchor\n");
}

#[test]
fn test_alias_in_sequence_resolves() {
    let referenced = Value::String(
        "referenced".to_string(),
        Some("ref_value_anchor".to_string()),
    );

    let mut map = Mapping::new();
    map.insert(
        Value::String("a".to_string(), None),
        Value::String("b".to_string(), None),
    );
    map.insert(
        Value::String("c".to_string(), None),
        Value::Alias("ref_value_anchor".to_string()),
    );

    let seq = Value::Sequence(Sequence {
        anchor: None,
        elements: vec![referenced.clone(), Value::Mapping(map)],
    });

    let yaml = serde_yaml_bw::to_string(&seq).unwrap();
    assert_eq!(
        yaml,
        "- &ref_value_anchor referenced\n- a: b\n  c: *ref_value_anchor\n",
    );
    println!("{}", yaml);

    let parsed: Value = serde_yaml_bw::from_str_value(&yaml).unwrap();
    let parsed_seq = parsed.as_sequence().unwrap();
    assert_eq!(
        parsed_seq.elements[0],
        Value::String("referenced".to_string(), Some("ref_value_anchor".to_string())),
    );
    let parsed_map = parsed_seq.elements[1].as_mapping().unwrap();
    assert_eq!(
        parsed_map.get("c"),
        Some(&Value::String("referenced".to_string(), None)),
    );
}
