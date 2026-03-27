use serde::Deserialize;
use serde_yaml_gtc as serde_yaml;

// M5C3: Block Scalar Nodes — literal with indent |2 and folded >1 with local tag
#[derive(Debug, Deserialize)]
struct Root {
    literal: String,
    folded: String,
}

#[test]
fn yaml_m5c3_block_scalar_nodes() {
    let y = r#"literal: |2
  value
folded:
   !foo
  >1
 value
"#;
    let v: Root = serde_yaml::from_str(y).expect("failed to parse M5C3");
    assert_eq!(v.literal, "value\n");
    assert_eq!(v.folded, "value\n");
}
