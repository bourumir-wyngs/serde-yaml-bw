use indoc::indoc;
use serde_derive::Deserialize;
use serde_yaml_bw::{Deserializer};
use serde::Deserialize as _;
use std::fmt::Debug;

fn test_de<T>(yaml: &str, expected: &T)
where
    T: serde::de::DeserializeOwned + PartialEq + Debug,
{
    let deserialized: T = T::deserialize(Deserializer::from_str(yaml)).unwrap();
    assert_eq!(*expected, deserialized);
}


#[derive(Deserialize, PartialEq, Debug)]
enum Inner {
    A { val: String },
    B(u8, u8),
}

#[derive(Deserialize, PartialEq, Debug)]
struct Wrapper {
    first: Inner,
    second: Inner,
}

#[test]
#[ignore]
fn test_alias_in_struct_variant() {
    let yaml = indoc! {
        "
        first: !A
          val: &a shared
        second: !A
          val: *a
        "
    };
    let expected = Wrapper {
        first: Inner::A { val: "shared".to_owned() },
        second: Inner::A { val: "shared".to_owned() },
    };
    test_de(yaml, &expected);
}

#[derive(Deserialize, PartialEq, Debug)]
struct TupleEnumWrapper {
    item: TupleEnum,
    num: u8,
}

#[derive(Deserialize, PartialEq, Debug)]
enum TupleEnum {
    Variant(u8, u8),
}

#[test]
#[ignore]
fn test_alias_in_tuple_variant() {
    let yaml = indoc! {
        "
        item: !Variant
          - &first 1
          - 2
        num: *first
        "
    };
    let expected = TupleEnumWrapper { item: TupleEnum::Variant(1, 2), num: 1 };
    test_de(yaml, &expected);
}

#[derive(Deserialize, Debug)]
enum Outer {
    Inner(InnerEnum),
}

#[derive(Deserialize, Debug)]
enum InnerEnum {
    Item(Vec<u8>),
}

#[test]
#[ignore]
fn test_nested_enum_alias_error() {
    let yaml = indoc! {
        "
        outer: !Inner
          !Item
            - &v 0
        alias: *v
        "
    };
    let result = StructWithOuter::deserialize(Deserializer::from_str(yaml));
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("unknown variant"), "unexpected message: {}", msg);
}

#[derive(Deserialize, Debug)]
struct StructWithOuter {
    outer: Outer,
    alias: u8,
}


