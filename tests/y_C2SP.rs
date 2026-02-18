use serde_yaml_gtc as serde_yaml;
// C2SP: Flow Mapping Key on two lines — marked fail: true
// Expect parsing to return an error (no panic).
#[test]
fn yaml_c2sp_flow_mapping_key_on_two_lines_should_fail() {
    let y = "[23\n]: 42\n";
    let result: Result<std::collections::HashMap<String, i32>, _> = serde_yaml::from_str(y);
    assert!(
        result.is_err(),
        "C2SP should fail to parse due to split flow key across lines"
    );
}
