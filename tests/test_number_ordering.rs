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
    let int_zero = Number::from(0);
    let int_one = Number::from(1);
    let float_zero = Number::from(0.0);
    let float_one = Number::from(1.0);

    let inf = Number::from(f64::INFINITY);
    let neg_inf = Number::from(f64::NEG_INFINITY);
    let nan = Number::from(f64::NAN);

    // +∞ comparisons against integers and floats
    assert_eq!(inf.partial_cmp(&int_one), Some(Ordering::Greater));
    assert_eq!(inf.partial_cmp(&float_one), Some(Ordering::Greater));
    assert!(std::panic::catch_unwind(|| inf > int_one).unwrap());
    assert!(std::panic::catch_unwind(|| inf > float_one).unwrap());

    // −∞ comparisons against integers and floats
    assert_eq!(neg_inf.partial_cmp(&int_zero), Some(Ordering::Less));
    assert_eq!(neg_inf.partial_cmp(&float_zero), Some(Ordering::Less));
    assert!(std::panic::catch_unwind(|| neg_inf < int_zero).unwrap());
    assert!(std::panic::catch_unwind(|| neg_inf < float_zero).unwrap());

    // NaN comparisons
    assert_eq!(nan.partial_cmp(&int_zero), Some(Ordering::Greater));
    assert!(nan.partial_cmp(&float_zero).is_none());
    assert!(std::panic::catch_unwind(|| nan > int_zero).unwrap());
    // direct comparison with float returns false but does not panic
    assert!(!std::panic::catch_unwind(|| nan > float_zero).unwrap());
}
