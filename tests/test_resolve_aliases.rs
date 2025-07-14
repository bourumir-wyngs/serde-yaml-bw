use serde_yaml_bw::{from_str_value_preserve, Value};

#[test]
fn unresolved_alias_error() {
    let yaml = "anchor: &id 1\nalias: *id";
    let mut value: Value = from_str_value_preserve(yaml).unwrap();

    // Remove the anchor so `alias` refers to a missing anchor when resolving.
    if let Some(Value::Number(_, anchor)) = value
        .as_mapping_mut()
        .unwrap()
        .get_mut("anchor")
    {
        *anchor = None;
    }

    let err = value.resolve_aliases().unwrap_err();
    assert_eq!(err.to_string(), "unresolved alias");
}

#[test]
fn cyclic_aliases_error() {
    // Alias refers to itself through the anchor creating a cycle.
    let yaml = "a: &a\n  ref: *a\n";
    let mut value: Value = from_str_value_preserve(yaml).unwrap();

    let err = value.resolve_aliases().unwrap_err();
    assert_eq!(err.to_string(), "encountered recursive merge alias");
}
