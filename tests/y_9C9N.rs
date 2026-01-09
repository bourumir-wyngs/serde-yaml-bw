// 9C9N: Wrong indented flow sequence — marked fail: true
// Expect parsing to return an error (no panic).
#[test]
#[ignore]
fn yaml_9c9n_wrong_indented_flow_sequence_should_fail() {
    let y = "---\nflow: [a,\nb,\nc]\n";
    let result: Result<std::collections::HashMap<String, Vec<String>>, _> =
        serde_yaml_bw::from_str(y);
    assert!(
        result.is_err(),
        "9C9N should fail to parse due to wrong indentation in flow sequence"
    );
}
