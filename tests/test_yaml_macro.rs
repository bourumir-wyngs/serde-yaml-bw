use serde::{Deserialize, Serialize};
use serde_yaml_bw::{yaml, to_string, from_value};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Example {
    alpha: u32,
    beta: bool,
}

#[test]
fn test_yaml_macro_basic() {
    let value = yaml!({
        "alpha": 1,
        "beta": true,
    });

    let out = to_string(&value).unwrap();
    assert_eq!(out, "alpha: 1\nbeta: true\n");

    let s: Example = from_value(value).unwrap();
    assert_eq!(s, Example { alpha: 1, beta: true });
}
