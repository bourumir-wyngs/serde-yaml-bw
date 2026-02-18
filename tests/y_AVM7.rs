use serde_yaml_gtc as serde_yaml;
// AVM7: Empty Stream — expect no documents when using from_multiple
#[test]
#[ignore]
fn yaml_avm7_empty_stream() {
    let y = "";
    let docs: Vec<String> = serde_yaml::from_multiple(y).expect("failed to parse AVM7");
    assert!(docs.is_empty(), "Expected no documents for empty stream");
}
