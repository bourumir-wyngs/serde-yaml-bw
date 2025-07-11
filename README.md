![panic-free](https://img.shields.io/badge/panic--free-✔️-brightgreen)
[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/bourumir-wyngs/serde-yaml-bw/rust.yml)](https://github.com/bourumir-wyngs/serde-yaml-bw/actions)
[![crates.io](https://img.shields.io/crates/v/serde_yaml_bw.svg)](https://crates.io/crates/serde_yaml_bw)
[![crates.io](https://img.shields.io/crates/l/serde_yaml_bw.svg)](https://crates.io/crates/serde_yaml_bw)
[![crates.io](https://img.shields.io/crates/d/serde_yaml_bw.svg)](https://crates.io/crates/serde_yaml_bw)
[![docs.rs](https://docs.rs/serde_yaml_bw/badge.svg)](https://docs.rs/serde_yaml_bw)

This package is a fork of **serde-yaml**, designed to provide (mostly) panic-free operation. Specifically, it should not panic when encountering malformed YAML syntax. This makes the library suitable for safely parsing user-supplied YAML content. Our fork also supports merge keys, which reduce redundancy and verbosity by enabling the reuse of common key-value pairs across multiple mappings.

These extensions come at the cost of some API restrictions: write access to indices and mappings has been removed. Read access remains possible, with `Value::Null` returned on invalid access. Also, duplicate keys are not longer permitted in YAML, returning proper error message instead.

We do not encourage using this crate beyond serialization with serde. If your use-case requires additional functionality, there are better-suited crates available, such as [yaml-rust2](https://crates.io/crates/yaml-rust2) and the newer, more experimental [saphyr](https://crates.io/crates/saphyr), both capable of handling valid YAML that is not directly representable with Rust structures.

Since the API has changed to a more restrictive version, the major version number has been incremented.

If a panic does occur under some short and clear input, please report it as a bug.

### Thread Safety

Internally the library uses a `CStr` wrapper for libyaml strings. This type is
`Send` and `Sync` only when referencing data that lives for the `'static`
lifetime, so short-lived pointers returned by the parser must not be shared
across threads.


## Usage Example

Here's an example demonstrating how to parse YAML into a Rust structure using `serde_yaml_bw` with proper error
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
        ...
        Three dots optionally mark the end of the document. You can write anything after this marker.
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
Here is example with merge keys (inherited properties):
```rust
use serde_derive::Deserialize;

/// Configuration to parse into. Does not include "defaults"
#[derive(Debug, Deserialize, PartialEq)]
struct Config {
    development: Connection,
    production: Connection,
}

#[derive(Debug, Deserialize, PartialEq)]
struct Connection {
    adapter: String,
    host: String,
    database: String,
}

#[test]
fn test_anchor_alias_deserialization() {
    let yaml_input = r#"
# Here we define "default configuration"    
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
        development: Connection {
            adapter: "postgres".into(),
            host: "localhost".into(),
            database: "dev_db".into(),
        },
        production: Connection {
            adapter: "postgres".into(),
            host: "localhost".into(),
            database: "prod_db".into(),
        },
    };

    // Assert parsed config matches expected
    assert_eq!(parsed, expected);
}
```
It is possible to construct infinite recursion with merge keys in YAML (RecursionLimitExceeded error would be returned)