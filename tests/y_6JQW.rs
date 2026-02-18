use serde_yaml_gtc as serde_yaml;
// 6JQW: In literals, newlines are preserved
#[test]
#[ignore]
fn yaml_6jqw_literals_preserve_newlines() {
    let y = "--- |\n\\//||\\/||\n// ||  ||__\n";
    let s: String = serde_yaml::from_str(y).expect("failed to parse 6JQW");
    assert_eq!(s, "\\//||\\/||\n// ||  ||__\n");
}
