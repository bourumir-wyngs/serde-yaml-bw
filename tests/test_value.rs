#![allow(
    clippy::derive_partial_eq_without_eq,
    clippy::eq_op,
    clippy::uninlined_format_args
)]

use indoc::indoc;
use serde::de::IntoDeserializer;
use serde::Deserialize;
use serde_yaml_bw::{Number, Value};

#[test]
fn test_nan() {
    let pos_nan = serde_yaml_bw::from_str::<Value>(".nan").unwrap();
    assert!(pos_nan.is_f64());
    assert_eq!(pos_nan, pos_nan);

    let neg_fake_nan = serde_yaml_bw::from_str::<Value>("-.nan").unwrap();
    assert!(neg_fake_nan.is_string());

    let significand_mask = 0xF_FFFF_FFFF_FFFF;
    let bits = (f64::NAN.copysign(1.0).to_bits() ^ significand_mask) | 1;
    let different_pos_nan = Value::Number(Number::from(f64::from_bits(bits)), None);
    assert_eq!(pos_nan, different_pos_nan);
}

#[test]
fn test_digits() {
    let num_string = serde_yaml_bw::from_str::<Value>("01").unwrap();
    assert!(num_string.is_string());
}

#[test]
fn test_into_deserializer() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Test {
        first: String,
        second: u32,
    }

    let value = serde_yaml_bw::from_str::<Value>("xyz").unwrap();
    let s = String::deserialize(value.into_deserializer()).unwrap();
    assert_eq!(s, "xyz");

    let value = serde_yaml_bw::from_str::<Value>("- first\n- second\n- third").unwrap();
    let arr = Vec::<String>::deserialize(value.into_deserializer()).unwrap();
    assert_eq!(arr, &["first", "second", "third"]);

    let value = serde_yaml_bw::from_str::<Value>("first: abc\nsecond: 99").unwrap();
    let test = Test::deserialize(value.into_deserializer()).unwrap();
    assert_eq!(
        test,
        Test {
            first: "abc".to_string(),
            second: 99
        }
    );
}

#[test]
fn test_debug() {
    let yaml = indoc! {"
        'Null': ~
        Bool: true
        Number: 1
        String: ...
        Sequence:
          - true
        EmptySequence: []
        EmptyMapping: {}
        Tagged: !tag true
    "};

    let value: Value = serde_yaml_bw::from_str(yaml).unwrap();
    let debug = format!("{:#?}", value);

    let expected = indoc! {r#"
        Mapping {
            "Null": Null,
            "Bool": Bool(true),
            "Number": Number(1),
            "String": String("..."),
            "Sequence": Sequence [
                Bool(true),
            ],
            "EmptySequence": Sequence [],
            "EmptyMapping": Mapping {},
            "Tagged": TaggedValue {
                tag: !tag,
                value: Bool(true),
            },
        }"#
    };

    assert_eq!(debug, expected);
}

