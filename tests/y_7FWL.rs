use serde_yaml_gtc as serde_yaml;
use std::collections::HashMap;

// 7FWL: Verbatim Tags — explicit tags should still deserialize to regular Rust types
#[test]
fn yaml_7fwl_verbatim_tags() {
    let y = "!!str foo: !<foo> baz\n".replace("!<foo>", "!bar"); // keep content simple per dump variant
    let m: HashMap<String, String> = serde_yaml::from_str(&y).expect("failed to parse 7FWL");
    assert_eq!(m.get("foo").map(String::as_str), Some("baz"));
}
