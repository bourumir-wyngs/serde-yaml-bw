use indoc::indoc;
use serde::Deserialize;
use serde_yaml_bw::{Deserializer, Value};
mod utils;

#[test]
fn custom_recursion_limit_exceeded() {
    let depth = 3;
    let yaml = "[".repeat(depth) + &"]".repeat(depth);
    let mut opts = utils::opts_no_pathology();
    opts.recursion_limit = 2;
    let err = Value::deserialize(Deserializer::from_str_with_options(&yaml, &opts)).unwrap_err();
    assert!(
        err.to_string().starts_with("recursion limit exceeded"),
        "unexpected error: {}",
        err
    );
}

#[test]
fn custom_recursion_limit_disabled() -> anyhow::Result<()>{
    let depth = 3;
    let yaml = "[".repeat(depth) + &"]".repeat(depth);
    let mut opts = utils::opts_no_pathology();
    opts.recursion_limit = 0; // disabled
    Value::deserialize(Deserializer::from_str_with_options(&yaml, &opts))?;
    Ok(())
}


#[test]
fn custom_alias_limit_exceeded() {
    let yaml = indoc! {
        "
        first: &a 1
        second: [*a, *a, *a]
        "
    };
    let mut opts = utils::opts_no_pathology();
    opts.alias_limit = 2;
    let result = Value::deserialize(Deserializer::from_str_with_options(yaml, &opts));
    assert_eq!("repetition limit exceeded", result.unwrap_err().to_string());
}
