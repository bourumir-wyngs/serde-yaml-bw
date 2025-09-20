use crate::number::Number as Num;
use crate::value::Value;
use std::fmt::{self, Display};
use std::str::FromStr;

#[derive(Clone, Debug)]
pub(crate) enum DuplicateKeyKind {
    Null,
    Bool(bool),
    Number(Num),
    String(String),
    Other,
}

use crate::libyaml::error::Mark;

#[derive(Clone, Debug)]
pub(crate) struct DuplicateKeyError {
    pub(crate) kind: DuplicateKeyKind,
    /// Location of the first occurrence of the key, if known.
    pub(crate) first: Option<Mark>,
    /// Location of the duplicate key occurrence, if known. This is generally
    /// used as the primary error location by the caller.
    pub(crate) duplicate: Option<Mark>,
}

impl DuplicateKeyError {
    pub(crate) fn from_value(value: &Value) -> Self {
        use DuplicateKeyKind::{Bool, Null, Number, Other, String};
        let kind = match value {
            Value::Null(_) => Null,
            Value::Bool(b, _) => Bool(*b),
            Value::Number(n, _) => Number(n.clone()),
            Value::String(s, _) => String(s.clone()),
            _ => Other,
        };
        DuplicateKeyError { kind, first: None, duplicate: None }
    }

    pub(crate) fn from_value_with_marks(value: &Value, first: Mark, duplicate: Mark) -> Self {
        let mut err = Self::from_value(value);
        err.first = Some(first);
        err.duplicate = Some(duplicate);
        err
    }

    pub(crate) fn from_scalar(bytes: &[u8]) -> Self {
        use DuplicateKeyKind::{Bool, Null, Number, String};
        if is_null(bytes) {
            return DuplicateKeyError { kind: Null, first: None, duplicate: None };
        }
        if let Ok(s) = std::str::from_utf8(bytes) {
            if let Some(b) = parse_bool(s) {
                return DuplicateKeyError { kind: Bool(b), first: None, duplicate: None };
            }
            if let Ok(n) = Num::from_str(s) {
                return DuplicateKeyError { kind: Number(n), first: None, duplicate: None };
            }
            return DuplicateKeyError {
                kind: String(s.to_string()),
                first: None,
                duplicate: None,
            };
        }
        DuplicateKeyError { kind: DuplicateKeyKind::Other, first: None, duplicate: None }
    }

    pub(crate) fn from_scalar_with_marks(bytes: &[u8], first: Mark, duplicate: Mark) -> Self {
        let mut err = Self::from_scalar(bytes);
        err.first = Some(first);
        err.duplicate = Some(duplicate);
        err
    }
}

fn is_null(s: &[u8]) -> bool {
    matches!(s, b"null" | b"Null" | b"NULL" | b"~")
}

fn parse_bool(s: &str) -> Option<bool> {
    crate::de::parse_bool_casefold(s)
}

impl Display for DuplicateKeyError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        use DuplicateKeyKind::{Bool, Null, Number, Other, String};
        formatter.write_str("duplicate entry ")?;
        match &self.kind {
            Null => formatter.write_str("with null key")?,
            Bool(b) => write!(formatter, "with key `{}`", b)?,
            Number(n) => write!(formatter, "with key {}", n)?,
            String(s) => write!(formatter, "with key {:?}", s)?,
            Other => formatter.write_str("in YAML map")?,
        }
        if let Some(first) = self.first {
            // Note: The primary error location (duplicate) will be shown by the
            // enclosing error formatter. Here we include the first occurrence.
            let line = first.line() as usize + 1;
            let col = first.column() as usize + 1;
            write!(formatter, " (first defined at line {} column {})", line, col)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{is_null, DuplicateKeyError, DuplicateKeyKind};
    use crate::number::Number;
    use crate::parse_bool_casefold;

    #[test]
    fn test_is_null_variants() {
        assert!(is_null(b"null"));
        assert!(is_null(b"Null"));
        assert!(is_null(b"NULL"));
        assert!(is_null(b"~"));
        assert!(!is_null(b"nul"));
    }

    #[test]
    fn test_parse_bool_variants() {
        assert_eq!(parse_bool_casefold("true"), Some(true));
        assert_eq!(parse_bool_casefold("True"), Some(true));
        assert_eq!(parse_bool_casefold("TRUE"), Some(true));
        assert_eq!(parse_bool_casefold("false"), Some(false));
        assert_eq!(parse_bool_casefold("False"), Some(false));
        assert_eq!(parse_bool_casefold("FALSE"), Some(false));
        assert_eq!(parse_bool_casefold("other"), None);
    }

    #[test]
    fn test_from_scalar_parsing() {
        let err = DuplicateKeyError::from_scalar(b"null");
        assert!(matches!(err.kind, DuplicateKeyKind::Null));

        let err = DuplicateKeyError::from_scalar(b"true");
        assert!(matches!(err.kind, DuplicateKeyKind::Bool(true)));

        let err = DuplicateKeyError::from_scalar(b"42");
        assert!(matches!(err.kind, DuplicateKeyKind::Number(n) if n == Number::from(42)));
    }

    #[test]
    fn test_display_variants() {
        let err = DuplicateKeyError::from_scalar(b"null");
        assert_eq!(format!("{}", err), "duplicate entry with null key");

        let err = DuplicateKeyError::from_scalar(b"true");
        assert_eq!(format!("{}", err), "duplicate entry with key `true`");

        let err = DuplicateKeyError::from_scalar(b"false");
        assert_eq!(format!("{}", err), "duplicate entry with key `false`");

        let err = DuplicateKeyError::from_scalar(b"42");
        assert_eq!(format!("{}", err), "duplicate entry with key 42");

        let err = DuplicateKeyError::from_scalar(b"dup");
        assert_eq!(format!("{}", err), "duplicate entry with key \"dup\"");

        let err = DuplicateKeyError::from_scalar(b"\xFF");
        assert_eq!(format!("{}", err), "duplicate entry in YAML map");
    }
}
