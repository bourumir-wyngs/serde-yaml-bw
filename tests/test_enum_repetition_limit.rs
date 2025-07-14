use indoc::indoc;
use serde::de::Deserialize;
use serde_derive::Deserialize as Derive;
use serde_yaml_bw::Deserializer;
use std::collections::BTreeMap;
use std::fmt::Debug;

fn test_error<'de, T>(yaml: &'de str, expected: &str)
where
    T: Deserialize<'de> + Debug,
{
    let result = T::deserialize(Deserializer::from_str(yaml));
    assert_eq!(expected, result.unwrap_err().to_string());

    let mut deserializer = Deserializer::from_str(yaml);
    if let Some(first_document) = deserializer.next() {
        if deserializer.next().is_none() {
            let result = T::deserialize(first_document);
            assert_eq!(expected, result.unwrap_err().to_string());
        }
    }
}

#[derive(Derive, Debug)]
enum Node {
    Unit,
    List(Vec<Node>),
}

#[cfg(not(miri))]
#[test]
fn test_enum_billion_laughs() {
    let yaml = indoc! {
        "
        a: &a !Unit
        b: &b !List [*a,*a,*a,*a,*a,*a,*a,*a,*a]
        c: &c !List [*b,*b,*b,*b,*b,*b,*b,*b,*b]
        d: &d !List [*c,*c,*c,*c,*c,*c,*c,*c,*c]
        e: &e !List [*d,*d,*d,*d,*d,*d,*d,*d,*d]
        f: &f !List [*e,*e,*e,*e,*e,*e,*e,*e,*e]
        g: &g !List [*f,*f,*f,*f,*f,*f,*f,*f,*f]
        h: &h !List [*g,*g,*g,*g,*g,*g,*g,*g,*g]
        i: &i !List [*h,*h,*h,*h,*h,*h,*h,*h,*h]
        "
    };
    let expected = "repetition limit exceeded";
    test_error::<BTreeMap<String, Node>>(yaml, expected);
}

