use serde_yaml_bw::{Number, Value, Mapping};
use serde_yaml_bw::value::Sequence;

#[test]
fn serialize_scalar_with_anchor() {
    let value = Value::Number(Number::from(1), Some("id".to_string()));
    let yaml = serde_yaml_bw::to_string(&value).unwrap();
    assert_eq!(yaml, "&id 1\n");
}

#[test]
fn serialize_sequence_with_anchor() {
    let seq = Sequence { anchor: Some("s".to_string()), elements: vec![Value::Bool(true, None)] };
    let yaml = serde_yaml_bw::to_string(&Value::Sequence(seq)).unwrap();
    assert_eq!(yaml, "&s\n- true\n");
}

#[test]
fn serialize_mapping_with_anchor() {
    let mut map = Mapping::new();
    map.anchor = Some("m".to_string());
    map.insert(Value::String("k".into(), None), Value::Bool(false, None));
    let yaml = serde_yaml_bw::to_string(&Value::Mapping(map)).unwrap();
    assert_eq!(yaml, "&m\nk: false\n");
}
