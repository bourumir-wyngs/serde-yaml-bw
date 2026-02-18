use serde_yaml_gtc as serde_yaml;
// 4V8U: Plain scalar with backslashes
#[test]
fn yaml_4v8u_plain_scalar_with_backslashes() {
    let y = "---\nplain\\value\\with\\backslashes\n";
    let s: String = serde_yaml::from_str(y).expect("failed to parse 4V8U");
    assert_eq!(s, "plain\\value\\with\\backslashes");
}
