// BEC7: “YAML” directive (1.3) followed by a document with a quoted string
#[test]
#[ignore] // !ssfr
fn yaml_bec7_yaml_directive_then_string() {
    let y = "%YAML 1.3 # Attempt parsing\n              # with a warning\n---\n\"foo\"\n";
    let s: String = serde_yaml_bw::from_str(y).expect("failed to parse BEC7");
    assert_eq!(s, "foo");
}