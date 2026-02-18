use serde_yaml_gtc as serde_yaml;
#[test]
fn yaml_3myt_plain_scalar_looking_like_key_comment_anchor_tag() {
    let yaml = "---\nk:#foo\n &a !t s\n";

    let s: String = serde_yaml::from_str(yaml).expect("failed to parse 3MYT scalar");
    assert_eq!(s, "k:#foo &a !t s");
}
