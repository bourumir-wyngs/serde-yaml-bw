use serde_yaml_gtc as serde_yaml;
#[test]
fn yaml_3r3p_single_block_sequence_with_anchor() {
    let yaml = "&sequence\n- a\n";

    let v: Vec<String> = serde_yaml::from_str(yaml).expect("failed to parse 3R3P sequence");
    assert_eq!(v, vec!["a".to_string()]);
}
