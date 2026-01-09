// 6LVF: Reserved Directives — parse single doc with a quoted string
#[test]
#[ignore]
fn yaml_6lvf_reserved_directives() {
    let y = "%FOO  bar baz # Should be ignored\n                  # with a warning.\n--- \"foo\"\n";
    let s: String = serde_yaml_bw::from_str(y).expect("failed to parse 6LVF");
    assert_eq!(s, "foo");
}
