use serde_derive::Deserialize;
use serde_yaml_bw::from_str_with_merge;

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
    let parsed: Config = from_str_with_merge(yaml_input).expect("Failed to deserialize YAML");

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
