#![allow(clippy::uninlined_format_args)]
use indoc::indoc;
use serde::{Deserialize, Serialize};
use serde_yaml_bw::{yaml, yaml_template, to_string};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Config {
    name: String,
    count: u32,
}

#[test]
fn test_yaml_macro() {
    let value = yaml!(indoc!("\
        name: John
        count: 1
    "));
    let cfg: Config = serde_yaml_bw::from_value(value).unwrap();
    assert_eq!(cfg, Config { name: "John".to_owned(), count: 1 });
}

#[test]
fn test_yaml_template_macro() {
    let user = "Jane";
    let value = yaml_template!(indoc!("\
        name: {user}
        count: {c}
    "), c = 2, user = user);
    let cfg: Config = serde_yaml_bw::from_value(value).unwrap();
    assert_eq!(cfg, Config { name: "Jane".to_owned(), count: 2 });
}

#[test]
fn test_yaml_macro_serialization() {
    let cfg = Config { name: "John".to_owned(), count: 1 };
    let out = to_string(&cfg).unwrap();
    assert_eq!(out, "name: John\ncount: 1\n");
}

