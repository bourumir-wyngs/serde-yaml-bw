use std::collections::HashMap;

#[test]
#[ignore] // libyaml issue: anchors with a colon in the name are not accepted; requires preprocessor which we don't want
fn yaml_2sxe_anchors_with_colon_in_name_parse_mapping() {
    let yaml = "&a: key: &a value\nfoo:\n  *a:\n";

    let m: HashMap<String, String> = serde_yaml_bw::from_str(yaml).expect("failed to parse 2SXE mapping");
    assert_eq!(m.get("key").map(String::as_str), Some("value"));
    assert_eq!(m.get("foo").map(String::as_str), Some("key"));
    assert_eq!(m.len(), 2);
}
