use serde_yaml_gtc as serde_yaml;
use std::collections::BTreeMap;

// P94K: Multi-Line Comments — mapping remains { key: value }
#[test]
fn yaml_p94k_multi_line_comments() {
    let y = r#"key:    # Comment
        # lines
  value



"#;
    let map: BTreeMap<String, String> = serde_yaml::from_str(&y).expect("failed to parse P94K");
    assert_eq!(map.get("key").map(String::as_str), Some("value"));
}
