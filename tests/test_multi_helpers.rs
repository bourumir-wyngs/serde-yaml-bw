use indoc::indoc;
use serde::{Deserialize, Serialize};
use serde_yaml_gtc as serde_yaml;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Point {
    x: i32,
}

#[test]
fn test_from_str_multi() {
    let yaml = indoc!("---\nx: 1\n---\nx: 2\n");
    let points: Vec<Point> = serde_yaml::from_str_multi(yaml).unwrap();
    assert_eq!(points, vec![Point { x: 1 }, Point { x: 2 }]);
}

#[test]
fn test_to_string_multi() {
    let points = vec![Point { x: 1 }, Point { x: 2 }];
    let out = serde_yaml::to_string_multi(&points).unwrap();
    assert_eq!(out, "x: 1\n---\nx: 2\n");
}
