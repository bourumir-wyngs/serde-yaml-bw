![panic-free](https://img.shields.io/badge/panic--free-✔️-brightgreen)
[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/bourumir-wyngs/serde-yaml-bw/rust.yml)](https://github.com/bourumir-wyngs/serde-yaml-bw/actions)
[![crates.io](https://img.shields.io/crates/v/serde_yaml_bw.svg)](https://crates.io/crates/serde_yaml_bw)
[![crates.io](https://img.shields.io/crates/l/serde_yaml_bw.svg)](https://crates.io/crates/serde_yaml_bw)
[![crates.io](https://img.shields.io/crates/d/serde_yaml_bw.svg)](https://crates.io/crates/serde_yaml_bw)
[![docs.rs](https://docs.rs/serde_yaml_bw/badge.svg)](https://docs.rs/serde_yaml_bw)

This package is a fork of **serde-yaml**, designed to provide (mostly) panic-free operation. Specifically, it should not panic when encountering malformed YAML syntax. This makes the library suitable for safely parsing user-supplied YAML content.

This increased safety comes at the cost of some API restrictions: write access to indices and mappings has been removed. Read access remains possible, with `Value::Null` returned on invalid access. Also, from 2.0.1, duplicate keys are not longer permitted in YAML, returning proper error message instead.

We do not encourage using this crate beyond serialization with serde. If your use-case requires additional functionality, there are better-suited crates available, such as [yaml-rust2](https://crates.io/crates/yaml-rust2) and the newer, more experimental [saphyr](https://crates.io/crates/saphyr), both capable of handling valid YAML that is not directly representable with Rust structures.

Since the API has changed to a more restrictive version, the major version number has been incremented.

If a panic does occur under some short and clear input, please report it as a bug.


## Usage Example

Here's a concise example demonstrating how to parse YAML into a Rust structure using `serde_yaml_bw` with proper error
handling:

```rust
use serde::Deserialize;
use serde_derive::Deserialize;
use serde_yaml_bw::Deserializer;

// Define the structure representing your YAML data.
#[derive(Debug, Deserialize)]
struct Config {
    name: String,
    enabled: bool,
    retries: i32,
}

fn main() {
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
```
or, with multiple documents and question operator:
```rust
use serde::Deserialize;
use serde_derive::Deserialize;
use serde_yaml_bw::Deserializer;

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

    Ok(configs)
}
```
and then

```rust
fn example_multi() -> anyhow::Result<()> {
    let configs = parse()?;
    println!("Parsed successfully: {:?}", configs);
    Ok(())
}

```

## Limitations

Anchors and aliases are currently expanded during deserialization and their names are not preserved when serialized again. As a result, a `from_str` -> `to_string` round trip loses anchor information. Supporting anchors and aliases would require extending the `Value` representation and serializer to retain anchor names and emit alias events.
