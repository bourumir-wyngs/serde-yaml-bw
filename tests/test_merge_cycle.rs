use serde_yaml_bw::{Value, from_str_value_preserve};

#[test]
fn test_self_referential_merge_alias_error() {
    let yaml = "a: &a\n  b: 1\n  <<: *a";
    let mut value = from_str_value_preserve(yaml).unwrap();
    assert!(value.apply_merge().is_err());
}
