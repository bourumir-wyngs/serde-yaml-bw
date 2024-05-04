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
It is currently not clear how this should be deployed to _crates.io_ as the package name (that is taken) is tightly
bound to the references of this name in the code. Ideally we would like to provide drop-in replacement with the only
change in Cargo.toml, but we still need to figure out how this could possibly work. So far we simply do not deploy to
crates.io, you need to add the dependency as Git repository:

```toml
[dependencies]
serde = "1.0"
serde_yaml = { git = "https://github.com/bourumir-wyngs/serde-yaml-bw", branch = "1.0.0" }
```

We will keep releases in separate branches, to make sure you do not have unexpected changes as we are working
on the master branch.

So far it may be little need for you to do this as all changes we have so far are increments of the minor versions in
dependent packages and some additional tests. But if the project would get some bug fixes, this may change. 

Release notes are available under [GitHub releases].

[GitHub releases]: https://github.com/dtolnay/serde-yaml/releases

## Using Serde YAML

[API documentation is available in rustdoc form][docs.rs] but the general idea
is:

[docs.rs]: https://docs.rs/serde_yaml

```rust
use std::collections::BTreeMap;

fn main() -> Result<(), serde_yaml::Error> {
    // You have some type.
    let mut map = BTreeMap::new();
    map.insert("x".to_string(), 1.0);
    map.insert("y".to_string(), 2.0);

    // Serialize it to a YAML string.
    let yaml = serde_yaml::to_string(&map)?;
    assert_eq!(yaml, "x: 1.0\ny: 2.0\n");

    // Deserialize it back to a Rust type.
    let deserialized_map: BTreeMap<String, f64> = serde_yaml::from_str(&yaml)?;
    assert_eq!(map, deserialized_map);
    Ok(())
}
```

It can also be used with Serde's derive macros to handle structs and enums
defined in your program.

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_yaml = "0.9"
```

Structs serialize in the obvious way:

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Point {
    x: f64,
    y: f64,
}

fn main() -> Result<(), serde_yaml::Error> {
    let point = Point { x: 1.0, y: 2.0 };

    let yaml = serde_yaml::to_string(&point)?;
    assert_eq!(yaml, "x: 1.0\ny: 2.0\n");

    let deserialized_point: Point = serde_yaml::from_str(&yaml)?;
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

fn main() -> Result<(), serde_yaml::Error> {
    let yaml = "
        - !Newtype 1
        - !Tuple [0, 0, 0]
        - !Struct {x: 1.0, y: 2.0}
    ";
    let values: Vec<Enum> = serde_yaml::from_str(yaml).unwrap();
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
    let values: Vec<Enum> = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(values[0], Enum::Tuple(0, 0, 0));
    assert_eq!(values[1], Enum::Struct { x: 1.0, y: 2.0 });

    // Variants with no data can be written using !Tag or just the string name.
    let yaml = "
        - Unit  # serialization produces this one
        - !Unit
    ";
    let values: Vec<Enum> = serde_yaml::from_str(yaml).unwrap();
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