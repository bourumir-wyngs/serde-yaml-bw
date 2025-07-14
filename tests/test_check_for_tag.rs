use serde_yaml_bw::value::tagged::{check_for_tag, MaybeTag};
use std::fmt::{self, Display};

#[test]
fn tag_detected() {
    let out = check_for_tag(&"!Thing");
    assert!(matches!(out, MaybeTag::Tag(ref s) if s == "Thing"));
}

#[test]
fn not_a_tag_normal_string() {
    let out = check_for_tag(&"normal");
    assert!(matches!(out, MaybeTag::NotTag(ref s) if s == "normal"));
}

#[test]
fn not_a_tag_bang_only() {
    let out = check_for_tag(&"!");
    assert!(matches!(out, MaybeTag::NotTag(ref s) if s == "!"));
}

struct FailsDisplay;

impl Display for FailsDisplay {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        Err(fmt::Error)
    }
}

#[test]
fn display_error_returns_error() {
    let out = check_for_tag(&FailsDisplay);
    assert!(matches!(out, MaybeTag::Error));
}

