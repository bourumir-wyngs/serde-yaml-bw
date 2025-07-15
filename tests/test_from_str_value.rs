use indoc::indoc;
use serde_yaml_bw::{from_str_value, Number, Value};

#[test]
fn test_from_str_value_resolves_alias() {
    let yaml = "a: &id 1\nb: *id";
    let value: Value = from_str_value(yaml).unwrap();

    assert_eq!(value["b"], Value::Number(Number::from(1), None));
}

#[test]
fn test_from_str_value_applies_merge() {
    let yaml = indoc! {
        r#"
        defaults: &defaults
          a: 1
          b: 2

        actual:
          <<: *defaults
          c: 3
        "#
    };

    let value: Value = from_str_value(yaml).unwrap();

    assert_eq!(value["actual"]["a"], Value::Number(Number::from(1), None));
    assert_eq!(value["actual"]["b"], Value::Number(Number::from(2), None));
    assert_eq!(value["actual"]["c"], Value::Number(Number::from(3), None));
}
