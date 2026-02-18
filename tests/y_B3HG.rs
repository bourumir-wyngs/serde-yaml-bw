use serde_yaml_gtc as serde_yaml;
// B3HG: Folded Scalar — expect "folded text\n"
#[test]
fn yaml_b3hg_folded_scalar() {
    let y = "--- >\n folded\n text\n\n\n";
    let s: String = serde_yaml::from_str(y).expect("failed to parse B3HG");
    assert_eq!(s, "folded text\n");
}
