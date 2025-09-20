use serde_yaml_bw::Value;

#[test]
fn test_value_number_cross_type_ordering_equality() {
    use std::cmp::Ordering;

    // Mixed int/float should compare equal by numeric value via PartialOrd.
    assert_eq!(
        Value::from(1u64).partial_cmp(&Value::from(1.0f64)),
        Some(Ordering::Equal)
    );
    assert_eq!(
        Value::from(-2i64).partial_cmp(&Value::from(-2.0f64)),
        Some(Ordering::Equal)
    );

    // Different numeric values should compare as expected.
    assert_eq!(
        Value::from(1u64).partial_cmp(&Value::from(2.0f64)),
        Some(Ordering::Less)
    );
    assert_eq!(
        Value::from(-1i64).partial_cmp(&Value::from(0.0f64)),
        Some(Ordering::Less)
    );
}

#[test]
fn test_value_number_ordering_int_float() {
    // Ensure mixed int/float comparisons obey numeric order.
    assert!(Value::from(1u64) < Value::from(2.0f64));
    assert!(Value::from(-2i64) < Value::from(-1.5f64));
    assert!(Value::from(10u64) > Value::from(9.999f64));
}

#[test]
fn test_special_floats_round_trip() {
    // NaN round-trip: cannot compare equality, but we can assert is_nan after deserialization.
    let nan_yaml = serde_yaml_bw::to_string(&f64::NAN).expect("serialize NaN");
    let nan_back: f64 = serde_yaml_bw::from_str(&nan_yaml).expect("deserialize NaN");
    assert!(nan_back.is_nan(), "Expected NaN after round-trip, got {nan_back:?} (yaml: {nan_yaml})");

    // +Inf round-trip
    let inf_yaml = serde_yaml_bw::to_string(&f64::INFINITY).expect("serialize +inf");
    let inf_back: f64 = serde_yaml_bw::from_str(&inf_yaml).expect("deserialize +inf");
    assert!(inf_back.is_infinite() && inf_back.is_sign_positive(), "Expected +inf after round-trip (yaml: {inf_yaml})");

    // -Inf round-trip
    let ninf_yaml = serde_yaml_bw::to_string(&f64::NEG_INFINITY).expect("serialize -inf");
    let ninf_back: f64 = serde_yaml_bw::from_str(&ninf_yaml).expect("deserialize -inf");
    assert!(ninf_back.is_infinite() && ninf_back.is_sign_negative(), "Expected -inf after round-trip (yaml: {ninf_yaml})");
}
