// 6M2F: Aliases in Explicit Block Mapping
use std::collections::HashMap;

#[test]
#[ignore] // libyaml limitation: parsing explicit block mapping followed by an empty-key entry written as ': value' is rejected; would need preprocessor to normalize
fn yaml_6m2f_aliases_in_explicit_block_mapping() {
    let y = "? &a a\n: &b b\n: *a\n";
    let m: HashMap<Option<String>, String> = serde_yaml_bw::from_str(y).expect("failed to parse 6M2F");
    assert_eq!(m.get(&Some("a".to_string())).map(String::as_str), Some("b"));
    assert_eq!(m.get(&None).map(String::as_str), Some("a"));
}
