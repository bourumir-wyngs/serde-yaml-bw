use serde_yaml_gtc as serde_yaml;
use serde_yaml::Value;

#[test]
fn test_recursion_limit_exceeded() {
    let depth = 129;
    let yaml = "[".repeat(depth) + &"]".repeat(depth);
    let err = serde_yaml::from_str::<Value>(&yaml).unwrap_err();
    assert!(
        err.to_string().starts_with("recursion limit exceeded"),
        "unexpected error: {}",
        err
    );
}
