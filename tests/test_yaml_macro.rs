use serde_yaml_bw::{yaml, Value};

#[test]
fn yaml_macro_parses_value() {
    let v: Value = yaml!("a: 1");
    assert_eq!(v["a"], Value::from(1));
}

#[test]
#[should_panic]
fn yaml_macro_panics_on_invalid_yaml() {
    let _v: Value = yaml!(":");
}
