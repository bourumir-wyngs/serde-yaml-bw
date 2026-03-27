use indoc::indoc;
use serde::Deserialize;
use serde_yaml_gtc as serde_yaml;

#[derive(Debug, PartialEq, Deserialize)]
struct Point {
    x: i32,
}

#[test]
fn test_stream_deserializer() {
    let yaml = indoc!("---\nx: 1\n---\nx: 2\n");
    let mut stream = serde_yaml::Deserializer::from_str(yaml).into_iter::<Point>();
    assert_eq!(stream.next().unwrap().unwrap(), Point { x: 1 });
    assert_eq!(stream.next().unwrap().unwrap(), Point { x: 2 });
    assert!(stream.next().is_none());
}
