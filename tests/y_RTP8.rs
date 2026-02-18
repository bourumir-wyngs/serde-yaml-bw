use serde_yaml_gtc as serde_yaml;
#[test]
fn y_rtp8() {
    let yaml = r#"%YAML 1.2
---
Document
... # Suffix
"#;

    let v: String =
        serde_yaml::from_str(yaml).expect("parse inner YAML with directive and markers");
    assert_eq!(v, "Document");
}
