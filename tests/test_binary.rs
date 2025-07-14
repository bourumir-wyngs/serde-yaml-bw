use serde::{Deserialize, Serialize};
use serde_yaml_bw as yaml;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Data {
    data: Vec<u8>,
}

#[test]
fn test_deserialize_binary_tag() {
    let yaml_str = "data:\n- 104\n- 101\n- 108\n- 108\n- 111\n";
    let parsed: Data = yaml::from_str(yaml_str).unwrap();
    assert_eq!(parsed.data, b"hello");
}

#[test]
fn test_deserialize_vec_u8_direct() {
    let bytes: Vec<u8> = yaml::from_str("- 1\n- 2\n- 3\n").unwrap();
    assert_eq!(bytes, vec![1, 2, 3]);
}

#[test]
fn test_serialize_vec_as_sequence() {
    let data = Data {
        data: b"hi".to_vec(),
    };
    let yaml_str = yaml::to_string(&data).unwrap();
    assert_eq!(yaml_str, "data:\n- 104\n- 105\n");
}
