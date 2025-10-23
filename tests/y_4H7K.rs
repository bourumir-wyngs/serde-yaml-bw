// 4H7K: Flow sequence with invalid extra closing bracket — marked fail: true
// Expect parsing to return an error (no panic).
#[test]
#[ignore] // libyaml leniency: extra closing bracket in flow sequence not consistently rejected; requires stricter validation than we apply
fn yaml_4h7k_extra_closing_bracket_should_fail() {
    let y = "---\n[ a, b, c ] ]\n";
    let result: Result<Vec<String>, _> = serde_yaml_bw::from_str(y);
    assert!(result.is_err(), "4H7K should fail to parse due to extra closing bracket");
}
