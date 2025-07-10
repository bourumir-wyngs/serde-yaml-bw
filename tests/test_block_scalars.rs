use indoc::indoc;
use serde_yaml_bw as yaml;

#[test]
fn test_literal_string() {
    let yaml_str = "|\n  line1\n  line2\n";
    let expected = "line1\nline2\n";
    let value = yaml::from_str::<String>(yaml_str).unwrap();
    assert_eq!(value, expected);
}

#[test]
fn test_literal_string_chomp() {
    let yaml_str = indoc!("|-\n  line1\n  line2\n");
    let expected = "line1\nline2";
    let value: String = yaml::from_str(yaml_str).unwrap();
    assert_eq!(value, expected);
}

#[test]
fn test_folded_string() {
    let yaml_str = indoc!(">\n  folded\n  lines\n");
    let expected = "folded lines\n";
    let value: String = yaml::from_str(yaml_str).unwrap();
    assert_eq!(value, expected);
}

#[test]
fn test_folded_string_chomp() {
    let yaml_str = indoc!(">-\n  folded\n  lines\n");
    let expected = "folded lines";
    let value: String = yaml::from_str(yaml_str).unwrap();
    assert_eq!(value, expected);
}

#[test]
fn test_literal_int() {
    let yaml_str = indoc!("!!int |-\n  42\n");
    let value: i32 = yaml::from_str(yaml_str).unwrap();
    assert_eq!(value, 42);
}

#[test]
fn test_folded_int() {
    let yaml_str = indoc!("!!int >-\n  42\n");
    let value: i32 = yaml::from_str(yaml_str).unwrap();
    assert_eq!(value, 42);
}

#[test]
fn test_literal_bool() {
    let yaml_str = indoc!("!!bool |-\n  true\n");
    let value: bool = yaml::from_str(yaml_str).unwrap();
    assert!(value);
}

#[test]
fn test_folded_bool() {
    let yaml_str = indoc!("!!bool >-\n  true\n");
    let value: bool = yaml::from_str(yaml_str).unwrap();
    assert!(value);
}

#[test]
fn test_literal_float() {
    let yaml_str = indoc!("!!float |-\n  3.14\n");
    let value: f64 = yaml::from_str(yaml_str).unwrap();
    assert_eq!(value, 3.14);
}

#[test]
fn test_folded_float() {
    let yaml_str = indoc!("!!float >-\n  3.14\n");
    let value: f64 = yaml::from_str(yaml_str).unwrap();
    assert_eq!(value, 3.14);
}

#[test]
fn test_literal_null() {
    let yaml_str = indoc!("!!null |-\n  null\n");
    let value: Option<()> = yaml::from_str(yaml_str).unwrap();
    assert_eq!(value, None);
}

#[test]
fn test_folded_null() {
    let yaml_str = indoc!("!!null >-\n  null\n");
    let value: Option<()> = yaml::from_str(yaml_str).unwrap();
    assert_eq!(value, None);
}
