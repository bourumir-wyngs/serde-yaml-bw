use indoc::indoc;
use serde::{Deserialize, Serialize};
use std::io::Cursor;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Point {
    x: i32,
}

#[test]
fn test_from_str_multi() {
    let yaml = indoc!("---\nx: 1\n---\nx: 2\n");
    let points: Vec<Point> = serde_yaml_bw::from_str_multi(yaml).unwrap();
    assert_eq!(points, vec![Point { x: 1 }, Point { x: 2 }]);
}

#[test]
fn test_from_multiple_str() {
    let yaml = indoc!("---\nx: 1\n---\nx: 2\n");
    let points: Vec<Point> = serde_yaml_bw::from_multiple(yaml).unwrap();
    assert_eq!(points, vec![Point { x: 1 }, Point { x: 2 }]);
}

#[test]
fn test_from_multiple_reader() {
    let yaml = b"---\nx: 3\n---\nx: 4\n";
    let cursor = Cursor::new(&yaml[..]);
    let de = serde_yaml_bw::Deserializer::from_reader(cursor);
    let points: Vec<Point> = serde_yaml_bw::from_multiple(de).unwrap();
    assert_eq!(points, vec![Point { x: 3 }, Point { x: 4 }]);
}

#[test]
fn test_from_multiple_with_options_recursion_limit() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Nested {
        values: Vec<Vec<i32>>,
    }

    let yaml = indoc!("---\nvalues:\n  - - 1\n");

    let nested: Vec<Nested> = serde_yaml_bw::from_multiple(yaml).unwrap();
    assert_eq!(
        nested,
        vec![Nested {
            values: vec![vec![1]]
        }]
    );

    let mut options = serde_yaml_bw::DeserializerOptions::default();
    options.recursion_limit = 1;

    let err = serde_yaml_bw::from_multiple_with_options::<_, Nested>(yaml, &options).unwrap_err();
    assert!(
        err.to_string().contains("recursion"),
        "unexpected error message: {err}"
    );
}

#[test]
fn test_to_string_multi() {
    let points = vec![Point { x: 1 }, Point { x: 2 }];
    let out = serde_yaml_bw::to_string_multi(&points).unwrap();
    assert_eq!(out, "x: 1\n---\nx: 2\n");
}
