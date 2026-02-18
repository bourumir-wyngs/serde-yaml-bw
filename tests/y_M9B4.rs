use serde_yaml_gtc as serde_yaml;
// M9B4: Literal Scalar — expect "literal\n\ttext\n"

#[test]
fn yaml_m9b4_literal_scalar() {
    let y = r#"|
 literal
 	text


"#;
    let s: String = serde_yaml::from_str(y).expect("failed to parse M9B4");
    assert_eq!(s, "literal\n\ttext\n");
}
