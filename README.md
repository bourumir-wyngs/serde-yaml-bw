Serde YAML, maintained by Bourumir Wyngs
==========

Rust library for using the [Serde] serialization framework with data in [YAML]
file format.

[![GitHub](https://img.shields.io/badge/GitHub-777777)](https://github.com/bourumir-wyngs/serde-yaml-bw)
[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/bourumir-wyngs/serde-yaml-bw/rust.yml)](https://github.com/bourumir-wyngs/serde-yaml-bw/actions)

The excellent and extremely useful [serde_yaml](https://github.com/dtolnay/serde-yaml) is now set to read only mode
and marked as deprecated. This fork aims to provide some basic mainenance like moving dependency version ranges ahead.

It is important to have at least some outlet for bug reports, requests for enhancement and pull requests as well.
We would also work on bug fixes if would be any. We will review and merge the pull requests, if we would receive any.
All interested in further development of the library are encouraged to look into existing pull
requests and assist us on this.

This package inherits a huge testing suite that should never break.


[Serde]: https://github.com/serde-rs/serde

[YAML]: https://yaml.org/

## Dependency

You can use the "package rename" to try this branch without changing your code:

```toml
[dependencies]
serde = "1.0"
serde_yaml = { package = "serde_yaml_bw", version = "1.0.0" }
```

If you're not in favor of this renaming, you can specify the dependency in the usual manner (as in example below). 
However, in this case you will need to update the package references in your source code accordingly. We are still 
uncertain which approach is preferred. 

So far it may be little need for you to do this as all changes we have so far are increments of the minor versions in
dependent packages and some additional tests. But if the project would get some bug fixes, this may change.

Release notes are available under [GitHub releases].

[GitHub releases]: https://github.com/dtolnay/serde-yaml/releases

## Using Serde YAML

[API documentation is available in rustdoc form][docs.rs] but the general idea
is:

[docs.rs]: https://docs.rs/serde_yaml

Examples below use the new package name without renaming:

```toml
[dependencies]
serde = "1.0"
serde_yaml_bw ="1.0.0" 
```

```rust
use std::collections::BTreeMap;

fn main() -> Result<(), serde_yaml_bw::Error> {
    // You have some type.
    let mut map = BTreeMap::new();
    map.insert("x".to_string(), 1.0);
    map.insert("y".to_string(), 2.0);

    // Serialize it to a YAML string.
    let yaml = serde_yaml_bw::to_string(&map)?;
    assert_eq!(yaml, "x: 1.0\ny: 2.0\n");

    // Deserialize it back to a Rust type.
    let deserialized_map: BTreeMap<String, f64> = serde_yaml_bw::from_str(&yaml)?;
    assert_eq!(map, deserialized_map);
    Ok(())
}
```

It can also be used with Serde's derive macros to handle structs and enums
defined in your program.

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
```

Structs serialize in the obvious way:

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Point {
    x: f64,
    y: f64,
}

fn main() -> Result<(), serde_yaml_bw::Error> {
    let point = Point { x: 1.0, y: 2.0 };

    let yaml = serde_yaml_bw::to_string(&point)?;
    assert_eq!(yaml, "x: 1.0\ny: 2.0\n");

    let deserialized_point: Point = serde_yaml_bw::from_str(&yaml)?;
    assert_eq!(point, deserialized_point);
    Ok(())
}
```

Enums serialize using YAML's `!tag` syntax to identify the variant name.

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
enum Enum {
    Unit,
    Newtype(usize),
    Tuple(usize, usize, usize),
    Struct { x: f64, y: f64 },
}

fn main() -> Result<(), serde_yaml_bw::Error> {
    let yaml = "
        - !Newtype 1
        - !Tuple [0, 0, 0]
        - !Struct {x: 1.0, y: 2.0}
    ";
    let values: Vec<Enum> = serde_yaml_bw::from_str(yaml).unwrap();
    assert_eq!(values[0], Enum::Newtype(1));
    assert_eq!(values[1], Enum::Tuple(0, 0, 0));
    assert_eq!(values[2], Enum::Struct { x: 1.0, y: 2.0 });

    // The last two in YAML's block style instead:
    let yaml = "
        - !Tuple
          - 0
          - 0
          - 0
        - !Struct
          x: 1.0
          y: 2.0
    ";
    let values: Vec<Enum> = serde_yaml_bw::from_str(yaml).unwrap();
    assert_eq!(values[0], Enum::Tuple(0, 0, 0));
    assert_eq!(values[1], Enum::Struct { x: 1.0, y: 2.0 });

    // Variants with no data can be written using !Tag or just the string name.
    let yaml = "
        - Unit  # serialization produces this one
        - !Unit
    ";
    let values: Vec<Enum> = serde_yaml_bw::from_str(yaml).unwrap();
    assert_eq!(values[0], Enum::Unit);
    assert_eq!(values[1], Enum::Unit);

    Ok(())
}
```

<br>

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>
<br>
<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
<br>
<sub>
Bourumir Wyngs is using the rights granted by the licenses above to take over the project
and continue its development.
</sub>