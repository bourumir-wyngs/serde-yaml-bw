use serde_yaml_bw::Number;

#[test]
fn test_is_i64_and_as_i64() {
    let neg = Number::from(-5i64);
    assert!(neg.is_i64());
    assert_eq!(neg.as_i64(), Some(-5));

    let pos = Number::from(5u64);
    assert!(pos.is_i64());
    assert_eq!(pos.as_i64(), Some(5));

    let big = Number::from(u64::MAX);
    assert!(!big.is_i64());
    assert_eq!(big.as_i64(), None);
}

#[test]
fn test_is_infinite_and_finite() {
    let inf = Number::from(f64::INFINITY);
    assert!(inf.is_infinite());
    assert!(!inf.is_finite());

    let finite = Number::from(10);
    assert!(!finite.is_infinite());
    assert!(finite.is_finite());
}
