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
fn test_is_u64_and_as_u64() {
    let pos = Number::from(5u64);
    assert!(pos.is_u64());
    assert_eq!(pos.as_u64(), Some(5));

    let neg = Number::from(-5i64);
    assert!(!neg.is_u64());
    assert_eq!(neg.as_u64(), None);
}

#[test]
fn test_is_f64_and_as_f64() {
    let float = Number::from(3.14);
    assert!(float.is_f64());
    assert_eq!(float.as_f64(), Some(3.14));

    let int = Number::from(10);
    assert!(!int.is_f64());
    assert_eq!(int.as_f64(), Some(10.0));
}

#[test]
fn test_is_nan() {
    let nan = Number::from(f64::NAN);
    assert!(nan.is_nan());

    let float = Number::from(3.14);
    assert!(!float.is_nan());

    let int = Number::from(5);
    assert!(!int.is_nan());
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

#[test]
fn test_parse_positive_integer() {
    let n = "42".parse::<Number>().unwrap();
    assert_eq!(n, Number::from(42));
}

#[test]
fn test_parse_negative_integer() {
    let n = "-42".parse::<Number>().unwrap();
    assert_eq!(n, Number::from(-42));
}

#[test]
fn test_parse_float() {
    let n = "3.14".parse::<Number>().unwrap();
    assert_eq!(n, Number::from(3.14));
}

#[test]
fn test_parse_invalid() {
    let err = "not_a_number".parse::<Number>().unwrap_err();
    assert_eq!(err.to_string(), "failed to parse YAML number");
}
