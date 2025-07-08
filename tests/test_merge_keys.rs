use serde_yaml_bw::Value;

#[test]
fn test_merge_key_example() {
    let yaml = r#"
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

- <<: [ *BIG, *LEFT, { x: 1, y: 2 } ]
  label: center/big
"#;

    let mut value: Value = serde_yaml_bw::from_str(yaml).unwrap();
    value.apply_merge().unwrap();

    let seq = value.as_sequence().expect("root should be a sequence");
    assert_eq!(seq.len(), 8);

    let base = &seq[4];
    assert_eq!(base, &seq[5]);
    assert_eq!(base, &seq[6]);
    assert_eq!(base, &seq[7]);
}
