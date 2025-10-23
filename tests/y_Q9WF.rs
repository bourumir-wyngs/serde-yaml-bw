use std::collections::BTreeMap;
use serde::Deserialize;

// Q9WF: Separation Spaces with complex key (flow mapping used as a mapping key)
// Write a full test that deserializes into a concrete Rust structure with a non-string key.
// The main code may need to be fixed to pass this, but this test is the expected behavior.

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
struct PlayerKey {
    first: String,
    last: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct Stats {
    hr: i64,
    avg: f64,
}

#[test]
fn yaml_q9wf_complex_key_flow_mapping_as_key() {
    let y = r#"{ first: Sammy, last: Sosa }:
# Statistics:
  hr:  # Home runs
     65
  avg: # Average
   0.278
"#;

    let map: BTreeMap<PlayerKey, Stats> = serde_yaml_bw::from_str(&y)
        .expect("Q9WF should parse: flow mapping used as map key with nested mapping value");

    assert_eq!(map.len(), 1);

    let expected_key = PlayerKey { first: "Sammy".into(), last: "Sosa".into() };
    let stats = map.get(&expected_key)
        .expect("expected key { first: Sammy, last: Sosa } to exist");

    assert_eq!(stats.hr, 65);
    // Floating point comparison allowing minor representation differences.
    assert!((stats.avg - 0.278).abs() < 1e-12, "expected avg ~= 0.278, got {}", stats.avg);
}
