use serde::Deserialize;
use serde_derive::{Deserialize, Serialize};
use serde_yaml_bw::Deserializer;

/// Test example 1 given in README
#[test]
fn example_main() {
    #[derive(Debug, Deserialize)]
    #[allow(dead_code)]
    struct Config {
        name: String,
        enabled: bool,
        retries: i32,
    }


    let yaml_input = r#"
        name: "My Application"
        enabled: true
        retries: 5
    "#;

    let config: Result<Config, _> = serde_yaml_bw::from_str(yaml_input);

    match config {
        Ok(parsed_config) => {
            println!("Parsed successfully: {:?}", parsed_config);
        }
        Err(e) => {
            eprintln!("Failed to parse YAML: {}", e);
        }
    }
}

/// Test example 2 given in README
#[test]
fn example_multi() -> anyhow::Result<()> {
    let configs = parse()?;
    println!("Parsed successfully: {:?}", configs);
    Ok(())
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Config {
    name: String,
    enabled: bool,
    retries: i32,
}

fn parse() -> anyhow::Result<Vec<Config>> {
    let yaml_input = r#"
# Configure the application    
name: "My Application"
enabled: true
retries: 5
---
# Configure the debugger
name: "My Debugger"
enabled: false
retries: 4
"#;

    let configs = Deserializer::from_str(yaml_input)
        .map(|doc| Config::deserialize(doc))
        .collect::<Result<Vec<_>, _>>()?; // <- question operator

    Ok(configs) // Ok on successful parsing or would be error on failure
}
/// Test nested enum example given in README
#[test]
fn example_nested() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    enum Outer {
        Inner(Inner),
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    enum Inner {
        Newtype(u8),
    }

    let yaml = indoc::indoc! {r#"
        Inner:
          Newtype: 0
    "#};

    let value: Outer = Outer::deserialize(serde_yaml_bw::Deserializer::from_str(yaml)).unwrap();
    assert_eq!(value, Outer::Inner(Inner::Newtype(0)));
}
