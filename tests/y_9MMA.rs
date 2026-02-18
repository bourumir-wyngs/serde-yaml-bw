use serde_yaml_gtc as serde_yaml;
// 9MMA: Directive by itself with no document — marked fail: true
// Expect parsing to return an error (no panic).
#[test]
fn yaml_9mma_directive_by_itself_should_fail() {
    let y = "%YAML 1.2\n";
    let result: Result<String, _> = serde_yaml::from_str(y);
    assert!(
        result.is_err(),
        "9MMA should fail because a directive without a document is invalid"
    );
}
