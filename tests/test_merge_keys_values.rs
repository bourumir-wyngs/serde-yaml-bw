use serde_yaml_bw::{to_string, Value, from_str_value_preserve};

#[test]
fn test_field_inheritance() {
    let yaml_input = r"
defaults: &defaults
  adapter: postgres
  host: localhost

development:
  <<: *defaults
  database: dev_db

production:
  <<: *defaults
  database: prod_db
";

    // Deserialize YAML with anchors and aliases
    let parsed: Value = from_str_value_preserve(yaml_input).expect("Failed to parse YAML");

    // Serialize back to YAML
    let serialized = to_string(&parsed).expect("Failed to serialize YAML");

    // Verify anchor/alias roundtrip (anchors preserved, aliases not expanded)
    if serialized.matches("adapter: postgres").count() != 1 {
        panic!("Anchors and aliases were not correctly preserved; duplication detected. \
        Serialized output {serialized}"
        );
    }
}


fn assert_same_entries(a: &Value, b: &Value) {
    let a = a.as_mapping().expect("a: expected a mapping");
    let b = b.as_mapping().expect("b: expected a mapping");

    assert_eq!(a.len(), b.len());
    for a_key in a.keys() {
        assert!(b.contains_key(a_key));
        let key = a_key.as_str();
        let a_value = a
            .get(a_key)
            .unwrap_or_else(|| panic!("key not present in a: {key:?}"))
            .as_str();
        let b_value = b
            .get(a_key)
            .unwrap_or_else(|| panic!("key not present in b: {key:?}"))
            .as_str();

        assert_eq!(
            a_value, b_value,
            "key {:?} has different values: a {:?}, b {:?}",
            a_key.as_str(), a_value, b_value
        );
    }
}

#[test]
fn test_merge_key_example() {
    let yaml = r"
- &CENTER { x: 1, y: 2 }
- &LEFT { x: 0, y: 2 }
- &BIG { r: 10 }
- &SMALL { r: 1 }

- x: 1
  y: 2
  r: 10
  label: center/big

- <<: *CENTER
  r: 10
  label: center/big

- <<: [ *CENTER, *BIG ]
  label: center/big

# And here we have it all:
- <<: [ *BIG, *LEFT, { x: 1, y: 2 } ]
  label: center/big
";

    let value: Value = serde_yaml_bw::from_str(yaml).unwrap();

    let seq = value.as_sequence().expect("root should be a sequence");
    assert_eq!(seq.len(), 8);

    let base = &seq[4];
    assert_same_entries(base, &seq[5]);
    assert_same_entries(base, &seq[6]);
    assert_same_entries(base, &seq[7]);
}

