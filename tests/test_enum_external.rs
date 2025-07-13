#![allow(clippy::derive_partial_eq_without_eq)]

use indoc::indoc;
use serde_derive::{Serialize, Deserialize};
use serde_yaml_bw;
use std::fmt::Debug;

fn test_serde<T>(thing: &T, yaml: &str)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + Debug,
{
    let serialized = serde_yaml_bw::to_string(thing).unwrap();
    assert_eq!(yaml, serialized);
}

#[test]
fn test_simple_enum() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    enum Simple {
        A,
        B,
    }
    let thing = Simple::A;
    let yaml = "A\n";
    test_serde(&thing, yaml);
}

#[test]
fn test_enum_with_fields() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    enum Variant {
        Color { r: u8, g: u8, b: u8 },
    }
    let thing = Variant::Color { r: 32, g: 64, b: 96 };
    let yaml = indoc! {r#"
        Color:
          r: 32
          g: 64
          b: 96
    "#};
    test_serde(&thing, yaml);
}

#[test]
fn test_nested_enum() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    enum Outer {
        Inner(Inner),
    }
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    enum Inner {
        Newtype(u8),
    }
    let thing = Outer::Inner(Inner::Newtype(0));
    let yaml = indoc! {r#"
        Inner:
          Newtype: 0
    "#};
    test_serde(&thing, yaml);
}
