use serde_derive::Deserialize;
use serde_yaml_bw::{from_str, Value};

#[derive(Debug, Deserialize, PartialEq)]
struct Config {
    defaults: Connection,
    development: Connection,
    production: Connection,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Connection {
    adapter: String,
    host: String,
    database: Option<String>,
}

#[test]
fn test_anchor_alias_deserialization() {
    let yaml_input = r#"
defaults: &defaults
  adapter: postgres
  host: localhost

development:
  <<: *defaults
  database: dev_db

production:
  <<: *defaults
  database: prod_db
"#;

    // Deserialize YAML with anchors, aliases and merge keys into the Config struct
    let parsed: Config = serde_yaml_bw::from_str(yaml_input).expect("Failed to deserialize YAML");

    // Define expected Config structure explicitly
    let expected = Config {
        defaults: Connection {
            adapter: "postgres".into(),
            host: "localhost".into(),
            database: None,
        },
        development: Connection {
            adapter: "postgres".into(),
            host: "localhost".into(),
            database: Some("dev_db".into()),
        },
        production: Connection {
            adapter: "postgres".into(),
            host: "localhost".into(),
            database: Some("prod_db".into()),
        },
    };

    // Assert parsed config matches expected
    assert_eq!(parsed, expected);
}

#[derive(Debug, Deserialize, PartialEq)]
struct Entry {
    x: i32,
    y: i32,
    r: i32,
    label: String,
}

#[test]
fn test_merge_fake_anchors() {
    let yaml = r#"
# Anchors define values not sufficient to build complete Rust structure.
# Complet structure can only be build by merging multiple
_anchors:
  CENTER: &CENTER { x: 1, y: 2 }
  LEFT: &LEFT { x: 0, y: 2 }
  BIG: &BIG { r: 10 }
  SMALL: &SMALL { r: 1 }

entries:
  - x: 1
    y: 2
    r: 10
    label: center/big

  - <<: *CENTER
    r: 10
    label: center/big

  - <<: [ *CENTER, *BIG ]
    label: center/big

  - <<: [ *BIG, *LEFT, { x: 1, y: 2 } ]
    label: center/big
"#;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Entry {
        x: i32,
        y: i32,
        r: i32,
        label: String,
    }

    #[derive(Debug, Deserialize)]
    struct Root {
        // Define the structure we actually do not serialize
        #[serde(skip)]
        _anchors: serde_yaml_bw::Value,

        entries: Vec<Entry>,
    }

    let root: Root = serde_yaml_bw::from_str(yaml).unwrap();

    // Check total entries (anchors are explicitly skipped)
    assert_eq!(root.entries.len(), 4);

    let base = Entry {
        x: 1,
        y: 2,
        r: 10,
        label: "center/big".to_string(),
    };

    assert_eq!(root.entries[0], base);
    assert_eq!(root.entries[1], base);
    assert_eq!(root.entries[2], base);
    assert_eq!(root.entries[3], base);

    assert!(root._anchors.is_null()); // fake anchors get null.
}


#[test]
fn test_merge_full_ancestor() {
    let yaml = r#"
# Ancestors contain all fields of the final structures, even if these values can be overridden.
- &CENTER { x: 0, y: 0, r: 5, label: from_CENTER }
- &LEFT { x: -1000, y: -4, r: 12, label: from_LEFT }
- &BIG { r: 1000, x: -1, y: -1, label: from_BIG }
- &SMALL { r: 1, x: -2, y: -2, label: from_SMALL }

- x: 1
  y: 2
  r: 10
  label: own_override

- <<: *CENTER
  r: 10

- <<: [ *CENTER, *BIG ]
  label: my_center_big

- <<: [ *BIG, *LEFT, { y: 2 } ]
"#;

    // Deserialize YAML directly into a Vec<Entry> using Serde
    let entries: Vec<Entry> = serde_yaml_bw::from_str(yaml).unwrap();

    // Check total entries (the first 4 YAML anchors are skipped as they do not match the Entry struct)
    assert_eq!(entries.len(), 8);

    let e1 = Entry {
        x: 1,
        y: 2,
        r: 10,
        label: "own_override".to_string(),
    };

    let e2 = Entry {
        x: 0,
        y: 0,
        r: 10,
        label: "from_CENTER".to_string(),
    };

    let e3 = Entry {
        x: -1,
        y: -1,
        r: 1000,
        label: "my_center_big".to_string(),
    };

    let e4 = Entry {
        x: -1000,
        y: 2,
        r: 12,
        label: "from_LEFT".to_string(),
    };


    assert_eq!(entries[4], e1);
    assert_eq!(entries[5], e2);
    assert_eq!(entries[6], e3);
    assert_eq!(entries[7], e4);
}