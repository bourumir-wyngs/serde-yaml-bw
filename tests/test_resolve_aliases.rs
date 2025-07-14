use serde_yaml_bw::{from_str_value_preserve};

#[test]
fn test_unknown_anchor_in_from_str_value_preserve() {
    let yaml = "*some";
    let err = from_str_value_preserve(yaml).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("unknown anchor"), "unexpected error: {msg}");
}
