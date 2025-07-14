use serde_yaml_bw::{Number, unexpected};
use serde::de::Unexpected;

#[test]
fn test_nan_display() {
    let n = Number::from(f64::NAN);
    assert_eq!(format!("{}", n), ".nan");
}

#[test]
fn test_infinity_display() {
    assert_eq!(format!("{}", Number::from(f64::INFINITY)), ".inf");
    assert_eq!(format!("{}", Number::from(f64::NEG_INFINITY)), "-.inf");
}

#[test]
fn test_unexpected_variants() {
    let pos = Number::from(42u64);
    assert_eq!(unexpected(&pos), Unexpected::Unsigned(42));

    let neg = Number::from(-7i64);
    assert_eq!(unexpected(&neg), Unexpected::Signed(-7));

    let float = Number::from(3.5f64);
    assert_eq!(unexpected(&float), Unexpected::Float(3.5));
}
