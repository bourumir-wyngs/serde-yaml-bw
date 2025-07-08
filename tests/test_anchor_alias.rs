use serde_yaml_bw::{to_string, Value, from_str_value_preserve};

#[test]
fn test_anchor_alias_roundtrip() {
    let yaml_input = r#"
defaults: &defaults
  adapter: postgres
  host: localhost

development:
  <<: *defaults
  database: dev_db

production:
  <<: *defaults
  database: prod_db
"#;

    // Deserialize YAML with anchors and aliases
    let parsed: Value = from_str_value_preserve(yaml_input).expect("Failed to parse YAML");

    // Serialize back to YAML
    let serialized = to_string(&parsed).expect("Failed to serialize YAML");

    println!("Serialized output:\n{}", serialized);

    // Verify anchor/alias roundtrip (anchors preserved, aliases not expanded)
    assert!(
        serialized.matches("adapter: postgres").count() == 1,
        "Anchors and aliases were not correctly preserved; duplication detected"
    );

    assert!(
        serialized.contains("&defaults"),
        "Serialized output missing expected anchor"
    );

    assert!(
        serialized.matches("*defaults").count() >= 1,
        "Serialized output missing expected alias"
    );
}
