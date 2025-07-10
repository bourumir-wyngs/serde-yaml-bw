use serde_yaml_bw::{from_str, from_str_value_preserve, Value};
use serde_derive::{Deserialize, Serialize};

#[test]
fn test_self_referential_merge() {
    let yaml = "a: &a\n  b: 1\n  <<: *a";
    let mut value: Value = from_str_value_preserve(yaml).unwrap();
    assert!(value.apply_merge().is_err());
}
#[test]
fn test_self_referential_merge_serde() {
    #[derive(Debug, Serialize, Deserialize)]
    struct Node {
        b: i32,
        next: Option<Box<Node>>, // Potential for infinite recursion
    }

    // Invalid YAML: self-referential alias with merge causing recursion
    let yaml_invalid = r#"
a: &a
  b: 1
  <<: *a
"#;

    let result_invalid: Result<Node, _> = from_str(yaml_invalid);
    assert!(
        result_invalid.is_err(),
        "Self-referential alias should cause an error"
    );
    println!("Self-referential alias: {:?}", result_invalid.err().unwrap());

    // Valid YAML: alias merge without recursion
    let yaml_valid = r#"
default: &default
  b: 10

node:
  <<: *default
  next:
    b: 20
"#;    
    

    #[derive(Debug, Deserialize)]
    struct Wrapper {
        node: Node,
    }

    let result_valid: Result<Wrapper, _> = from_str(yaml_valid);
    assert!(result_valid.is_ok(), "Alias merge should parse successfully but {:?}", result_valid);

    let wrapper = result_valid.unwrap();

    // Assert parsed values
    assert_eq!(wrapper.node.b, 10);
    assert!(wrapper.node.next.is_some());
    assert_eq!(wrapper.node.next.as_ref().unwrap().b, 20);
    assert!(wrapper.node.next.as_ref().unwrap().next.is_none());


    // Invalid YAML: three-level recursion with a loop introduced
    let yaml_three_level_loop = r#"
level1: &level1
  b: 10
  next:
    <<: *level2

level2: &level2
  b: 20
  next:
    <<: *level3

level3: &level3
  b: 30
  next:
    <<: *level1  # Introduces a loop back to level1

node:
  <<: *level1
"#;

    let result_three_level_loop: Result<Wrapper, _> = from_str(yaml_three_level_loop);
    assert!(
        result_three_level_loop.is_err(),
        "Three-level recursive loop should cause an error"
    );
}    

