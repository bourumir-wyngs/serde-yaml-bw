// AVM7: Empty Stream — expect no documents when using from_multiple
#[test]
#[ignore] // !ssfr
fn yaml_avm7_empty_stream() {
    let y = "";
    let docs: Vec<String> = serde_yaml_bw::from_str_multi(y).expect("failed to parse AVM7");
    assert!(docs.is_empty(), "Expected no documents for empty stream");
}
