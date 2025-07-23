use serde::Serialize;

#[derive(Serialize)]
struct Inner { value: u32 }
#[derive(Serialize)]
struct Outer { inner: Inner }

#[test]
fn custom_indent() {
    let mut buf = Vec::new();
    let mut ser = serde_yaml_bw::SerializerBuilder::new()
        .indent(4)
        .build(&mut buf)
        .unwrap();
    let outer = Outer { inner: Inner { value: 1 } };
    outer.serialize(&mut ser).unwrap();
    drop(ser);
    assert_eq!(String::from_utf8(buf).unwrap(), "inner:\n    value: 1\n");
}

#[test]
fn custom_width() {
    #[derive(Serialize)]
    struct Data { text: String }
    let mut buf = Vec::new();
    let mut ser = serde_yaml_bw::SerializerBuilder::new()
        .width(10)
        .build(&mut buf)
        .unwrap();
    let data = Data { text: "a b c d e f g h i j k l m n o p".to_string() };
    data.serialize(&mut ser).unwrap();
    drop(ser);
    let output = String::from_utf8(buf).unwrap();
    assert!(output.contains("\n  d e f"));
}
