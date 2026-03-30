use serde::Deserialize;
use serde_yaml_gtc as serde_yaml;

// 9SHH: Quoted Scalar Indicators — mapping with 'single' and 'double'
#[derive(Debug, Deserialize, PartialEq)]
struct Doc {
    single: String,
    double: String,
}

#[test]
fn yaml_9shh_quoted_scalar_indicators() {
    let y = "single: 'text'\ndouble: \"text\"\n";
    let d: Doc = serde_yaml::from_str(y).expect("failed to parse 9SHH");
    assert_eq!(d.single, "text");
    assert_eq!(d.double, "text");
}
