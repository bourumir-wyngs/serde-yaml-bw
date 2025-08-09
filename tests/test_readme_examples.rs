use serde::{Deserialize, Serialize};
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

#[derive(Deserialize, Serialize, Debug)]
#[allow(dead_code)]
struct Move {
    by: f32,
    constraints: Vec<Constraint>,
}

#[derive(Deserialize, Serialize, Debug)]
#[allow(dead_code)]
enum Constraint {
    StayWithin { x: f32, y: f32, r: f32 },
    MaxSpeed { v: f32 },
}

#[test]
fn deserialize_robot_moves() {
    let yaml = r#"
- by: 10.0
  constraints:
    - StayWithin:
        x: 0.0
        y: 0.0
        r: 5.0
    - StayWithin:
        x: 4.0
        y: 0.0
        r: 5.0
    - MaxSpeed:
        v: 3.5
"#;

    let robot_moves: Vec<Move> = serde_yaml_bw::from_str(yaml).unwrap();

    assert_eq!(robot_moves.len(), 1);
    assert_eq!(robot_moves[0].by, 10.0);
    assert_eq!(robot_moves[0].constraints.len(), 3);
}

#[test]
fn serialize_robot_moves() {
    let robot_moves: Vec<Move> = vec![
        Move {
            by: 1.0,
            constraints: vec![
                Constraint::StayWithin {
                    x: 0.0,
                    y: 0.0,
                    r: 5.0,
                },
                Constraint::MaxSpeed { v: 100.0 },
            ],
        },
        Move {
            by: 2.0,
            constraints: vec![Constraint::MaxSpeed { v: 10.0 }],
        },
    ];
    let yaml = "- by: 1.0\n  constraints:\n  - StayWithin:\n      x: 0.0\n      y: 0.0\n      r: 5.0\n  - MaxSpeed:\n      v: 100.0\n- by: 2.0\n  constraints:\n  - MaxSpeed:\n      v: 10.0\n";
    assert_eq!(serde_yaml_bw::to_string(&robot_moves).unwrap(), yaml);
}
