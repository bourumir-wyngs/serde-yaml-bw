use serde_yaml::value::Tag;
use serde_yaml_gtc as serde_yaml;

#[test]
fn tag_new_empty_returns_err() {
    let err = Tag::new("").unwrap_err();
    assert_eq!(err.to_string(), "empty YAML tag is not allowed");
}
