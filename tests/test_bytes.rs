use serde_yaml_bw as yaml;

#[test]
fn test_serialize_bytes() {
    let bytes: &[u8] = &[1, 2, 3];
    let s = yaml::to_string(&bytes).unwrap();
    assert_eq!(s, "- 1\n- 2\n- 3\n");
}
