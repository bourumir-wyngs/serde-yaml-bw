#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use serde_yaml_bw::{from_str_value_preserve, Value};

    /// A simple struct we can deserialize into, to verify that alias resolution
    /// produces independent (cloned) values with identical content.
    #[allow(dead_code)]
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct Item {
        id: u32,
        name: String,
        tags: Vec<String>,
    }

    /// Container holding two fields that both alias the same anchored value in YAML.
    #[allow(dead_code)]
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct Container {
        first: Item,
        second: Item,
    }

    #[test]
    fn anchor_is_resolved_by_cloning_value() {
        // The same anchored mapping is referenced twice via aliases.
        // After resolution, `first` and `second` must be *equal in content* but
        // fully independent (i.e., modifying one in the Value tree does not
        // change the other).
        //
        // YAML structure:
        // anchor: &A { id: 7, name: "gizmo", tags: ["x", "y"] }
        // first: *A
        // second: *A
        let yaml = r#"
anchor: &A
  id: 7
  name: gizmo
  tags: [x, y]
first: *A
second: *A
"#;

        // Parse into a Value and resolve aliases explicitly.
        let mut doc: Value = from_str_value_preserve(yaml).expect("parse YAML");

        match &doc["first"] {
            Value::Alias(m) => assert_eq!(m, "A", "from_str_value_preserve must retain anchor values"),
            other => panic!("expected Mapping for `first`, got {other:?}"),
        }

        doc.resolve_aliases().expect("resolve aliases");

        // --- Check 1: the two aliased values are equal after resolution ---
        assert_eq!(doc["first"], doc["second"]);

        // --- Check 2: the replacement copies have no anchors left ---
        // (the original anchored node keeps its anchor; the copies must not)
        match &doc["first"] {
            Value::Mapping(m) => assert!(m.anchor.is_none(), "resolved copy must be anchor-free"),
            other => panic!("expected Mapping for `first`, got {other:?}"),
        }
        match &doc["second"] {
            Value::Mapping(m) => assert!(m.anchor.is_none(), "resolved copy must be anchor-free"),
            other => panic!("expected Mapping for `second`, got {other:?}"),
        }
        match &doc["anchor"] {
            Value::Mapping(m) => assert_eq!(m.anchor.as_deref(), Some("A")),
            other => panic!("expected Mapping with anchor at `anchor`, got {other:?}"),
        }

        // --- Check 3: mutating one does not affect the other (must be cloned) ---
        // Replace `first` with a different mapping; `second` should remain unchanged.
        let key_first = Value::String("first".into(), None);
        if let Value::Mapping(root) = &mut doc {
            // Build a replacement mapping value: { id: 100, name: "different", tags: ["z"] }
            let mut repl_map = serde_yaml_bw::Mapping::default();
            repl_map.insert(
                Value::String("id".into(), None),
                Value::Number(100u64.into(), None),
            );
            repl_map.insert(
                Value::String("name".into(), None),
                Value::String("different".into(), None),
            );
            let tags_seq = serde_yaml_bw::Sequence {
                elements: vec![Value::String("z".into(), None)],
                anchor: None,
            };
            repl_map.insert(
                Value::String("tags".into(), None),
                Value::Sequence(tags_seq),
            );
            // Overwrite `first`
            root.insert(key_first.clone(), Value::Mapping(repl_map));
        } else {
            panic!("root must be a Mapping");
        }

        // After mutation, `first` != `second`
        assert_ne!(doc["first"], doc["second"], "mutating one alias copy must not affect the other");

        // --- Optional: also ensure we can deserialize into typed structs and both are equal ---
        // Note: `serde_yaml_bw::from_value` is commonly provided. If your crate uses
        // a different name, adjust accordingly.
        let typed: Container = serde_yaml_bw::from_value(doc.clone()).expect("typed deserialize");
        assert_eq!(
            typed.first,
            Item { id: 100, name: "different".to_string(), tags: vec!["z".into()] }
        );
        assert_eq!(
            typed.second,
            Item { id: 7, name: "gizmo".to_string(), tags: vec!["x".into(), "y".into()] }
        );
    }
}
