use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Root {
    foo: Vec<i64>,
    bar: Vec<i64>,
}

#[test]
fn y_rlu9() {
    let yaml = r#"foo:
- 42
bar:
  - 44
"#;

    let v: Root = serde_yaml_bw::from_str(yaml).expect("parse inner YAML");
    assert_eq!(v.foo, vec![42]);
    assert_eq!(v.bar, vec![44]);
}
