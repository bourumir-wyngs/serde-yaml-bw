use serde_derive::{Deserialize, Serialize};
use indoc::indoc;

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
fn test_to_string_multi() {
    let points = vec![Point { x: 1 }, Point { x: 2 }];
    let out = serde_yaml_bw::to_string_multi(&points).unwrap();
    assert_eq!(out, "x: 1\n---\nx: 2\n");
}
