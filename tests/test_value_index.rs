use serde_yaml_gtc as serde_yaml;
use serde_yaml::Value;

#[test]
fn test_value_index_returns_null() {
    let value: Value = serde_yaml::from_str("{a: {b: 1}}" ).unwrap();
    assert_eq!(value["a"]["c"], Value::Null(None));
    assert_eq!(value["x"][0], Value::Null(None));
}
