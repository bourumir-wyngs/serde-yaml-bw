![panic-free](https://img.shields.io/badge/panic--free-✔️-brightgreen)
[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/bourumir-wyngs/serde-yaml-bw/rust.yml)](https://github.com/bourumir-wyngs/serde-yaml-bw/actions)
[![crates.io](https://img.shields.io/crates/v/serde_yaml_bw.svg)](https://crates.io/crates/serde_yaml_bw)
[![crates.io](https://img.shields.io/crates/l/serde_yaml_bw.svg)](https://crates.io/crates/serde_yaml_bw)
[![crates.io](https://img.shields.io/crates/d/serde_yaml_bw.svg)](https://crates.io/crates/serde_yaml_bw)
[![docs.rs](https://docs.rs/serde_yaml_bw/badge.svg)](https://docs.rs/serde_yaml_bw)

This package is a fork of serde-yaml designed to provide panic-free operation. Although it may still fail under certain specific circumstances, it should never panic—even when encountering malformed YAML documents — as long as they fit into memory. Any occurrences of panics under these conditions are considered bugs, and we welcome bug reports and contributions to resolve them.

Our version has no `panic!()` and `.unwrap()` constructs, opting instead to return proper error messages. 
This makes the library suitable for parsing user-supplied YAML content. There is currently a new test suite 
with numerous malformed YAML cases they should yield to errors, not panic. 

Unfortunately, this safety came at the cost of removing write access to indices and mappings. Read access is still 
possible, Value::Null is returned on invalid access. We do not encourage using Value, the indented usage of this
crate is to serialize and deserialize with serde. If your code is going beyond that, there are more suitable
crates like [yaml-rust2]([https://crates.io/crates/yaml-rust2) that can parse YAML not directly representable
with Rust structures. As the API now changed into a more restrictive one, the major version number was incremented.

If panic does occur under some conditions, please report this as bug. 

## Usage Example

Here's a concise example demonstrating how to parse YAML into a Rust structure using `serde_yaml_bw` with proper error handling:

```rust
use serde::{Serialize, Deserialize};

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




