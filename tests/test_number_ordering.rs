use serde_yaml_bw::Number;
use std::cmp::Ordering;

#[test]
fn test_ordering_between_int_and_float() {
    assert_eq!(Number::from(2).partial_cmp(&Number::from(2.0)), Some(Ordering::Equal));
    assert!(Number::from(1) < Number::from(1.5));
    assert!(Number::from(3) > Number::from(2.5));
    assert!(Number::from(-3) < Number::from(-2.5));
}

#[test]
fn test_ordering_special_values() {
    assert!(Number::from(f64::INFINITY) > Number::from(1));
    assert!(Number::from(f64::NEG_INFINITY) < Number::from(0));
    assert!(Number::from(f64::NAN) > Number::from(0));
}
