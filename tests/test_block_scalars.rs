#![allow(
    clippy::derive_partial_eq_without_eq,
    clippy::uninlined_format_args,
)]

use indoc::indoc;
use serde_yaml_bw::Value;
use std::collections::BTreeMap;

#[test]
fn test_block_scalars() {
    let yaml = indoc! {
        "
        literal_clip: |
          foo
          bar
        literal_strip: |-
          foo
          bar
        literal_keep: |+
          foo
          bar

        folded_clip: >
          foo
          bar
        folded_strip: >-
          foo
          bar
        folded_keep: >+
          foo
          bar
        "
    };
    let data: BTreeMap<String, Value> = serde_yaml_bw::from_str(yaml).unwrap();
    assert_eq!(data.get("literal_clip").unwrap(), "foo\nbar\n");
    assert_eq!(data.get("literal_strip").unwrap(), "foo\nbar");
    assert_eq!(data.get("literal_keep").unwrap(), "foo\nbar\n\n");
    assert_eq!(data.get("folded_clip").unwrap(), "foo bar\n");
    assert_eq!(data.get("folded_strip").unwrap(), "foo bar");
    assert_eq!(data.get("folded_keep").unwrap(), "foo bar\n");
}
