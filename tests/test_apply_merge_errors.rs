use indoc::indoc;
use serde_yaml_bw::{from_str_value_preserve, Value};
use serde_yaml_bw::value::{Tag, TaggedValue};

#[test]
fn test_scalar_in_merge() {
    let yaml = indoc!(r#"
        <<: 1
        a: 2
    "#);
    let mut value: Value = from_str_value_preserve(yaml).unwrap();
    let err = value.apply_merge().unwrap_err();
    assert_eq!(
        err.to_string(),
        "expected a mapping or list of mappings for merging, but found scalar"
    );
}

#[test]
fn test_tagged_in_merge() {
    let yaml = indoc!(r#"
        <<: {}
        a: 2
    "#);
    let mut value: Value = from_str_value_preserve(yaml).unwrap();
    if let Value::Mapping(ref mut map) = value {
        let merge = map.get_mut("<<").unwrap();
        let inner = std::mem::take(merge);
        let tag = Tag::new("foo").unwrap();
        *merge = Value::Tagged(Box::new(TaggedValue { tag, value: inner }));
    } else {
        panic!("expected mapping");
    }
    let err = value.apply_merge().unwrap_err();
    assert_eq!(err.to_string(), "unexpected tagged value in merge");
}

#[test]
fn test_scalar_in_merge_element() {
    let yaml = indoc!(r#"
        <<: [1]
        a: 2
    "#);
    let mut value: Value = from_str_value_preserve(yaml).unwrap();
    let err = value.apply_merge().unwrap_err();
    assert_eq!(
        err.to_string(),
        "expected a mapping for merging, but found scalar"
    );
}

#[test]
fn test_sequence_in_merge_element() {
    let yaml = indoc!(r#"
        <<:
          - [1, 2]
        a: 2
    "#);
    let mut value: Value = from_str_value_preserve(yaml).unwrap();
    let err = value.apply_merge().unwrap_err();
    assert_eq!(
        err.to_string(),
        "expected a mapping for merging, but found sequence"
    );
}

#[test]
fn test_merge_recursion() {
    let yaml = indoc!(r#"
        a: &a
          b: 1
    "#);
    let mut value: Value = from_str_value_preserve(yaml).unwrap();
    if let Value::Mapping(map) = &mut value {
        if let Some(Value::Mapping(a_map)) = map.get_mut("a") {
            let clone = a_map.clone();
            a_map.insert("self".into(), Value::Mapping(clone));
        }
    }
    let err = value.apply_merge().unwrap_err();
    assert_eq!(err.to_string(), "encountered recursive merge alias");
}
