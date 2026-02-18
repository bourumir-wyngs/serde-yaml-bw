use serde_yaml_gtc as serde_yaml;
// V55R: Aliases in Block Sequence
// Expect: ["a", "b", "a", "b"]

#[test]
fn yaml_v55r_aliases_in_block_sequence() {
    let y = "- &a a\n- &b b\n- *a\n- *b\n";
    let v: Vec<String> = serde_yaml::from_str(y).expect("failed to parse V55R");
    assert_eq!(v, vec!["a", "b", "a", "b"]);
}
