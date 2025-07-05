use serde_yaml_bw;
use std::collections::HashMap;
use serde_derive::Deserialize;

#[test]
fn test_recursive_yaml_references_fail() {
    let yaml = "a: &anchor\n  b: *anchor";
    let res: Result<serde_yaml_bw::Value, _> = serde_yaml_bw::from_str(yaml);
    assert!(res.is_err(), "Recursive references should fail");
}

#[test]
fn test_non_string_keys_fail() {
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Data {
        map: HashMap<String, String>,
    }

    let yaml = "map:\n  ? [1, 2, 3]\n  : \"value\"";
    let res: Result<Data, _> = serde_yaml_bw::from_str(yaml);
    assert!(res.is_err(), "Non-string keys should fail");
}

#[test]
fn test_custom_yaml_tags() {
    #[derive(Debug, Deserialize)]
    struct Data {
        data: serde_yaml_bw::Value,
    }

    let yaml = "data: !CustomTag\n  key: value";
    let res: Data = serde_yaml_bw::from_str(yaml).expect("Should parse");

    match res.data {
        serde_yaml_bw::Value::Tagged(tagged) => {
            assert_eq!(tagged.tag, "!CustomTag");
            match tagged.value {
                serde_yaml_bw::Value::Mapping(map) => {
                    assert!(map.contains_key(&serde_yaml_bw::Value::String("key".into())));
                }
                other => panic!("Expected mapping inside tag, got: {:?}", other),
            }
        }
        other => panic!("Expected TaggedValue, got: {:?}", other),
    }
}

#[test]
fn test_large_integer_overflow_fail() {
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Data {
        big: u64,
    }

    let yaml = "big: 123456789012345678901234567890";
    let res: Result<Data, _> = serde_yaml_bw::from_str(yaml);
    assert!(res.is_err(), "Large integer overflow should fail");
}

#[test]
fn test_circular_references_fail() {
    let yaml = "a: &anchor\n  b: &anchor2\n    c: *anchor";
    let res: Result<serde_yaml_bw::Value, _> = serde_yaml_bw::from_str(yaml);
    assert!(res.is_err(), "Circular references should fail");
}

#[test]
fn test_unexpected_type_fail() {
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Config {
        name: String,
        age: u32,
    }

    let yaml = "config: John";
    let res: Result<HashMap<String, Config>, _> = serde_yaml_bw::from_str(yaml);
    assert!(res.is_err(), "Unexpected scalar instead of struct should fail");
}

#[test]
fn test_invalid_base64_fail() {
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Data {
        data: Vec<u8>,
    }

    let yaml = "data: !!binary invalid-base64-data";
    let res: Result<Data, _> = serde_yaml_bw::from_str(yaml);
    assert!(res.is_err(), "Invalid base64 should fail");
}

