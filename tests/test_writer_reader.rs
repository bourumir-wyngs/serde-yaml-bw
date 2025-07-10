use serde_derive::{Deserialize, Serialize};
use serde::Deserialize as _;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Point {
    x: i32,
    y: i32,
}

#[test]
fn test_write_then_read_struct() {
    use serde::Serialize as _;
    let point = Point { x: 1, y: 2 };
    let mut buf = Vec::new();
    {
        let mut ser = serde_yaml_bw::Serializer::new(&mut buf).unwrap();
        point.serialize(&mut ser).unwrap();
    }
    let de: Point = serde_yaml_bw::from_slice(&buf).unwrap();
    assert_eq!(de, point);
}

#[test]
fn test_reader_deserialize() {
    let yaml = "x: 3\ny: 4\n";
    let reader = std::io::Cursor::new(yaml.as_bytes());
    let de = serde_yaml_bw::Deserializer::from_reader(reader);
    let p = Point::deserialize(de).unwrap();
    assert_eq!(p, Point { x: 3, y: 4 });
}
