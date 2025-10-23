#[test]
#[ignore]
// While standard calls to ignore unknown directives, libyaml treats them as errors
// and this cannot be turned off. Unsure if makes sense to doctor YAML input. May make
// more sense to require cleaner YAML in the input.
fn yaml_2lfx_reserved_directives_ignored_parse_scalar() {
    // Reserved directive should be ignored; the document value is the string "foo"
    let yaml = "%FOO  bar baz # Should be ignored\n# with a warning.\n---\n\"foo\"\n";

    let s: String = serde_yaml_bw::from_str(yaml).expect("failed to parse 2LFX scalar");
    assert_eq!(s, "foo");
}
