use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Dummy;

#[test]
fn test_recursion_limit_exceeded() {
    let depth = 129;
    let yaml = "[".repeat(depth) + &"]".repeat(depth);
    let err = serde_yaml_bw::from_str::<Dummy>(&yaml).unwrap_err();
    assert!(
        err.to_string().starts_with("recursion limit exceeded"),
        "unexpected error: {}",
        err
    );
}
