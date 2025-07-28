#![allow(clippy::derive_partial_eq_without_eq)]

use indoc::indoc;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
struct Scalars {
    folded: String,
    folded_strip: String,
    folded_keep: String,
    literal: String,
    literal_strip: String,
    literal_keep: String,
}

#[test]
fn test_block_scalars() {
    let yaml = indoc! {
        "
        folded: >
          line1
          line2
        folded_strip: >-
          line1
          line2
        folded_keep: >+
          line1
          line2

        literal: |
          line1
          line2
        literal_strip: |-
          line1
          line2
        literal_keep: |+
          line1
          line2

        "
    };

    let expected = Scalars {
        folded: "line1 line2\n".to_owned(),
        folded_strip: "line1 line2".to_owned(),
        folded_keep: "line1 line2\n\n".to_owned(),
        literal: "line1\nline2\n".to_owned(),
        literal_strip: "line1\nline2".to_owned(),
        literal_keep: "line1\nline2\n\n".to_owned(),
    };

    let result: Scalars = serde_yaml_bw::from_str(yaml).unwrap();
    assert_eq!(expected, result);
}

#[test]
fn test_block_scalars_2() {
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
    #[derive(Debug, Deserialize)]
    struct Scalars2 {
        literal_clip: String,
        literal_strip: String,
        literal_keep: String,
        folded_clip: String,
        folded_strip: String,
        folded_keep: String,
    }
    let data: Scalars2 = serde_yaml_bw::from_str(yaml).unwrap();
    assert_eq!(data.literal_clip, "foo\nbar\n");
    assert_eq!(data.literal_strip, "foo\nbar");
    assert_eq!(data.literal_keep, "foo\nbar\n\n");
    assert_eq!(data.folded_clip, "foo bar\n");
    assert_eq!(data.folded_strip, "foo bar");
    assert_eq!(data.folded_keep, "foo bar\n");
}
