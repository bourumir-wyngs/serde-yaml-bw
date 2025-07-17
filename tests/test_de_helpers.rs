use serde_yaml_bw::{digits_but_not_number, parse_bool_casefold, parse_f64};

#[test]
fn test_digits_but_not_number() {
    assert!(digits_but_not_number("00"));
    assert!(digits_but_not_number("01"));
    assert!(!digits_but_not_number("0"));
}

#[test]
fn test_parse_f64_inf_and_invalid() {
    assert_eq!(parse_f64(".inf"), Some(f64::INFINITY));
    assert_eq!(parse_f64("inf"), None);
    assert_eq!(parse_f64("not a number"), None);
}

#[test]
fn test_parse_bool_casefold_variants() {
    assert_eq!(parse_bool_casefold("true"), Some(true));
    assert_eq!(parse_bool_casefold("True"), Some(true));
    assert_eq!(parse_bool_casefold("TRUE"), Some(true));
    assert_eq!(parse_bool_casefold("false"), Some(false));
    assert_eq!(parse_bool_casefold("False"), Some(false));
    assert_eq!(parse_bool_casefold("FALSE"), Some(false));
    assert_eq!(parse_bool_casefold("other"), None);
}
