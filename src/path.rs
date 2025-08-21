use std::fmt::{self, Display};

/// Path to the current value in the input, like `dependencies.serde.typo1`.
#[derive(Debug, Copy, Clone)]
pub enum Path<'a> {
    Root,
    Seq { parent: &'a Path<'a>, index: usize },
    Map { parent: &'a Path<'a>, key: &'a str },
    Alias { parent: &'a Path<'a> },
    Unknown { parent: &'a Path<'a> },
}

impl Display for Path<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        struct Parent<'a>(&'a Path<'a>);

        impl Display for Parent<'_> {
            fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
                match self.0 {
                    Path::Root => Ok(()),
                    path => write!(formatter, "{}.", path),
                }
            }
        }

        match self {
            Path::Root => formatter.write_str("."),
            Path::Seq { parent, index } => write!(formatter, "{}[{}]", parent, index),
            Path::Map { parent, key } => write!(formatter, "{}{}", Parent(parent), key),
            Path::Alias { parent } => write!(formatter, "{}", parent),
            Path::Unknown { parent } => write!(formatter, "{}?", Parent(parent)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Path;

    #[test]
    fn test_display_variants() {
        let root = Path::Root;
        assert_eq!(format!("{}", root), ".");

        let seq = Path::Seq {
            parent: &root,
            index: 1,
        };
        assert_eq!(format!("{}", seq), ".[1]");

        let map = Path::Map {
            parent: &seq,
            key: "name",
        };
        assert_eq!(format!("{}", map), ".[1].name");

        let alias = Path::Alias { parent: &map };
        assert_eq!(format!("{}", alias), ".[1].name");

        let unknown = Path::Unknown { parent: &map };
        assert_eq!(format!("{}", unknown), ".[1].name.?");
    }

    #[test]
    fn test_debug_variants() {
        let root = Path::Root;
        assert_eq!(format!("{:?}", root), "Root");

        let seq = Path::Seq {
            parent: &root,
            index: 1,
        };
        assert_eq!(format!("{:?}", seq), "Seq { parent: Root, index: 1 }");
    }
}
