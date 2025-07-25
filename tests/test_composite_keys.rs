use indoc::indoc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Transform {
    map: HashMap<Point, Point>,
}

#[test]
fn test_deserialize_transform() {
    let yaml = indoc! {
        "
        map:
          ? x: 1
            y: 2
          : x: 3
            y: 4
          ? x: 5
            y: 6
          : x: 7
            y: 8
        "
    };

    let mut map = HashMap::new();
    map.insert(Point { x: 1, y: 2 }, Point { x: 3, y: 4 });
    map.insert(Point { x: 5, y: 6 }, Point { x: 7, y: 8 });
    let expected = Transform { map };
    let actual: Transform = serde_yaml_bw::from_str(yaml).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_serialize_transform() {
    let mut map = HashMap::new();
    map.insert(Point { x: 1, y: 2 }, Point { x: 3, y: 4 });
    map.insert(Point { x: 5, y: 6 }, Point { x: 7, y: 8 });
    let transform = Transform { map };
    let yaml = serde_yaml_bw::to_string(&transform).unwrap();
    let expected_a = indoc! {
        "
        map:
          ? x: 1
            y: 2
          : x: 3
            y: 4
          ? x: 5
            y: 6
          : x: 7
            y: 8
        "
    };
    let expected_b = indoc! {
        "
        map:
          ? x: 5
            y: 6
          : x: 7
            y: 8
          ? x: 1
            y: 2
          : x: 3
            y: 4
        "
    };
    assert!(yaml == expected_a || yaml == expected_b);
}
