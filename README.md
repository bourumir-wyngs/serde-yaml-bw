![panic-free](https://img.shields.io/badge/panic--free-✔️-brightgreen)
[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/bourumir-wyngs/serde-yaml-bw/rust.yml)](https://github.com/bourumir-wyngs/serde-yaml-bw/actions)
[![crates.io](https://img.shields.io/crates/v/serde_yaml_bw.svg)](https://crates.io/crates/serde_yaml_bw)
[![crates.io](https://img.shields.io/crates/l/serde_yaml_bw.svg)](https://crates.io/crates/serde_yaml_bw)
[![crates.io](https://img.shields.io/crates/d/serde_yaml_bw.svg)](https://crates.io/crates/serde_yaml_bw)
[![docs.rs](https://docs.rs/serde_yaml_bw/badge.svg)](https://docs.rs/serde_yaml_bw)
[![Fuzz & Audit](https://github.com/bourumir-wyngs/serde-yaml-bw/actions/workflows/ci.yml/badge.svg)](https://github.com/bourumir-wyngs/serde-yaml-bw/actions/workflows/ci.yml)

This is a strongly typed YAML serialization and deserialization library, designed to provide (mostly) panic-free operation. Specifically, it should not panic when encountering malformed YAML syntax. This makes the library suitable for safely parsing user-supplied YAML content. JSON can be parsed as well. The library is hardened against the Billion Laughs attack, infinite recursion from merge keys and anchors (the limits are configurable) and duplicate keys. As the library only deserializes into explicitly defined types (no dynamic object instantiation), the usual YAML-based code execution [exploits](https://www.arp242.net/yaml-config.html) don’t apply.

Historically the project started as fork of **serde-yaml** but has seen notable development thereafter.

Our fork supports merge keys, which reduce redundancy and verbosity by specifying shared key-value pairs once and then reusing them across multiple mappings. It additionally supports nested enums for Rust-aligned parsing of polymorphic data, as well as the !!binary tag.

The library also uses Rust structure that is a parsing target as kind of schema. This schema allows to parse properly both "true" and "1.2" into String even without quotes. It can also handle standard YAML 1.1 boolean values when parsed into boolean (y, yes, on, n, no, off and the like).

These extensions come at the cost of some API restrictions: write access to indices and mappings has been removed. Read access remains possible, with `Value::Null` returned on invalid access. Also, duplicate keys are not longer permitted in YAML, returning proper error message instead.

We do not encourage using this crate beyond serialization with serde. If your use-case requires additional functionality, there are better-suited crates available, such as [yaml-rust2](https://crates.io/crates/yaml-rust2) and the newer, more experimental [saphyr](https://crates.io/crates/saphyr), both capable of handling valid YAML that is not directly representable with Rust structures.

Since the API has changed to a more restrictive version, the major version number has been incremented.

If a panic does occur under some short and clear input, please report it as a bug.

## Usage Example

Here's an example demonstrating how to parse YAML into a Rust structure using `serde_yaml_bw` with proper error
handling:

```rust
use serde::Deserialize;

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
use serde::Deserialize;

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

fn main() {
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

Merge keys are standard in YAML 1.1. Although YAML 1.2 no longer includes merge keys in its specification, it doesn't explicitly disallow them either, and many parsers implement this feature.

### Nested enums

Externally tagged enums naturally nest in YAML as maps keyed by the variant name. They enable the use of strict types (Rust enums with associated data) instead of falling back to generic maps.

```rust
#[derive(Deserialize)]
struct Move {
    by: f32, // Distance for a robot to move
    constraints: Vec<Constraint>, // Restrict how it is allowed to move
}

/// Restrict space, speed, force, whatever - with associated data.
/// Multiple constraints can be taken into consideration
#[derive(Deserialize)]
enum Constraint {
    StayWithin { x: f32, y: f32, r: f32 },
    MaxSpeed { v: f32 },
}

fn main() {
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
}
```

### Composite keys

YAML complex keys are useful for coordinate transformations, multi-field identifiers, test cases with compound conditions and the like. While Rust struct field names must be strings, Rust maps can also use complex keys — so such YAML structures can be parsed directly into maps.

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct Point { x: i32, y: i32 }

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Transform {
    // Transform between two coordinate systems
    map: HashMap<Point, Point>,
}

fn main() {
    let yaml = r#"
  map:
      {x: 1, y: 2}: {x: 3, y: 4}
      {x: 5, y: 6}: {x: 7, y: 8}
"#;
    let transform: Transform = serde_yaml_bw::from_str(yaml).unwrap();
}
```

### Binary scalars

YAML values tagged with `!!binary` are automatically base64-decoded when deserializing into `Vec<u8>`. To serialize in this form, annotate the field with `#[serde(with = "serde_bytes")]` from the [serde_bytes](https://docs.rs/serde_bytes/0.11.17/serde_bytes/) crate.

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
struct Blob {
    data: Vec<u8>,
}

fn parse_blob() {
    let blob: Blob = serde_yaml_bw::from_str("data: !!binary aGVsbG8=").unwrap();
    assert_eq!(blob.data, b"hello");
}
```

### Rc, Arc, Box and Cow

To serialize references (`Rc`, `Arc`), just add the [`"rc"` feature](https://serde.rs/feature-flags.html#-features-rc) to [Serde](https://serde.rs/). `Box` and `Cow` are supported [out of the box](https://serde.rs/data-model.html).

### Streaming

This library does not read the whole content of the Reader before even trying to parse. Hence it is possible to implement
streaming using the new [`StreamDeserializer`](https://docs.rs/serde_yaml_bw/latest/serde_yaml_bw/struct.StreamDeserializer.html).

```rust
use serde::Deserialize;
use std::fs::File;

#[derive(Debug, Deserialize)]
struct Record { id: i32 }

fn read_records() -> std::io::Result<()> {
    let file = File::open("records.yaml")?;
    for doc in serde_yaml_bw::Deserializer::from_reader(file).into_iter::<Record>() {
        println!("id = {}", doc?.id);
    }
    Ok(())
}
```

[`DeserializerOptions`](https://docs.rs/serde_yaml_bw/latest/serde_yaml_bw/struct.DeserializerOptions.html)
can be adjusted to control recursion or alias expansion limits. The formatting of emitted YAML can be configured using [`SerializerBuilder`](https://docs.rs/serde_yaml_bw/latest/serde_yaml_bw/struct.SerializerBuilder.html) that is useful for a human-intended output.

### Rust struct as schema

This reader uses the passed Rust struct as a YAML schema. Knowing that our parsing target is a String or a boolean field allows us to assign correctly values that would result in an error if parsed without this background knowledge:

- YAML sees values like 1.2 as numbers, but if it is something like a version, the parsing target is likely to be a string. If parsed into a structure field, it is easy to see if the parsing target is a string or a number; hence, 1.2 can also be parsed as "1.2" without generating an unnecessary error.
- Values like y, on, n, no, off can be used as boolean values in YAML 1.1. This causes the "Norway problem" if the parsing target is actually a string. However, if we know the parsing target, it is easy to parse "no" as false (for boolean) or as a String (for String).

### Anchors and references

While we initially assumed that [`Value`](https://docs.rs/serde_yaml_bw/latest/serde_yaml_bw/enum.Value.html) was an unnecessary relic in our package and even considered removing it, we recently encountered a use case involving very large structures with frequent repetition. In such cases, YAML anchors and references make the documents much more human-readable.

To support this, the package now allows constructing abstract `Value` nodes that can hold any data but also carry an optional *anchor* field. In addition, [`Value::Alias`](https://docs.rs/serde_yaml_bw/latest/serde_yaml_bw/enum.Value.html#variant.Alias) represents a reference (serialized as a YAML alias). This makes it possible to work directly with YAML containing anchors and references.

#### Example

```rust
use serde_yaml_bw::{Mapping, Value};

let mut mapping = Mapping::new();
mapping.insert(
    Value::String("a".to_string(), None),
    Value::String("foo".to_string(), Some("id".to_string())),
);
mapping.insert(
    Value::String("b".to_string(), None),
    Value::Alias("id".to_string()),
);

let value = Value::Mapping(mapping);
let yaml = serde_yaml_bw::to_string(&value).unwrap();
assert_eq!(yaml, "a: &id foo\nb: *id\n");
```

#### Preserving Anchors

In addition, the public function [`from_str_value_preserve`](https://docs.rs/serde_yaml_bw/latest/serde_yaml_bw/fn.from_str_value_preserve.html) can be used to parse a YAML string into a [`Value`](https://docs.rs/serde_yaml_bw/latest/serde_yaml_bw/enum.Value.html) **without resolving references or merge keys**. These can then be resolved later using [`Value::resolve_aliases`](https://docs.rs/serde_yaml_bw/latest/serde_yaml_bw/enum.Value.html#method.resolve_aliases) and [`Value::apply_merge`](https://docs.rs/serde_yaml_bw/latest/serde_yaml_bw/enum.Value.html#method.apply_merge) once you need to expand them.
