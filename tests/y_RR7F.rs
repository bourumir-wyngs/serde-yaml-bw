use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Root {
    a: f64,
    d: i64,
}

#[test]
fn y_rr7f() {
    let yaml = r#"a: 4.2
? d
: 23
"#;

    let v: Root = serde_yaml_bw::from_str(yaml).expect("parse inner YAML");
    assert!((v.a - 4.2).abs() < 1e-12);
    assert_eq!(v.d, 23);
}
