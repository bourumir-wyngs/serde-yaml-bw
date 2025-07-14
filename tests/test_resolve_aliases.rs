use serde_yaml_bw::{from_str_value_preserve, Value};

#[test]
fn unresolved_alias_returns_err() {
    let yaml = "a: *missing";
    let mut value: Value = from_str_value_preserve(yaml).unwrap();
    let err = value.resolve_aliases().unwrap_err();
    assert_eq!(err.to_string(), "unresolved alias");
}
