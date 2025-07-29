use indoc::indoc;
use serde::Deserialize;
use serde_yaml_bw::{Deserializer, DeserializerOptions};


#[derive(Debug, Deserialize)]
struct Nested(Vec<Nested>);

#[test]

fn custom_recursion_limit_exceeded() {
    let depth = 3;
    let yaml = "[".repeat(depth) + &"]".repeat(depth);
    let mut opts = DeserializerOptions::default();
    opts.recursion_limit = 2;
    let err = Nested::deserialize(Deserializer::from_str_with_options(&yaml, &opts)).unwrap_err();
    assert!(
        err.to_string().starts_with("recursion limit exceeded"),
        "unexpected error: {}",
        err
    );
}

#[test]
fn custom_alias_limit_exceeded() {
    let yaml = indoc! {
        "
        first: &a 1
        second: [*a, *a, *a]
        "
    };
    #[derive(Debug, Deserialize)]
    struct Data {
        first: i32,
        second: Vec<i32>,
    }
    let mut opts = DeserializerOptions::default();
    opts.alias_limit = 2;
    let result = Data::deserialize(Deserializer::from_str_with_options(yaml, &opts));
    assert_eq!("repetition limit exceeded", result.unwrap_err().to_string());
}
