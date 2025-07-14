use serde_yaml_bw::value::Tag;

#[test]
fn tag_starts_with_prefix_variants() {
    let tag_with_bang = Tag::new("!Thing").unwrap();
    let tag_without_bang = Tag::new("Thing").unwrap();
    assert!(tag_with_bang.starts_with("!Th"));
    assert!(tag_with_bang.starts_with("Th"));
    assert!(tag_without_bang.starts_with("!Th"));
    assert!(tag_without_bang.starts_with("Th"));
}
