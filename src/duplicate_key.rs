use crate::number::Number as Num;
use crate::value::Value;
use std::fmt::{self, Display};
use std::str::FromStr;

#[derive(Clone)]
pub(crate) enum DuplicateKeyKind {
    Null,
    Bool(bool),
    Number(Num),
    String(String),
    Other,
}

pub(crate) struct DuplicateKeyError {
    pub(crate) kind: DuplicateKeyKind,
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
        DuplicateKeyError { kind }
    }

    pub(crate) fn from_scalar(bytes: &[u8]) -> Self {
        use DuplicateKeyKind::{Bool, Null, Number, Other, String};
        if is_null(bytes) {
            return DuplicateKeyError { kind: Null };
        }
        if let Ok(s) = std::str::from_utf8(bytes) {
            if let Some(b) = parse_bool(s) {
                return DuplicateKeyError { kind: Bool(b) };
            }
            if let Ok(n) = Num::from_str(s) {
                return DuplicateKeyError { kind: Number(n) };
            }
            return DuplicateKeyError { kind: String(s.to_string()) };
        }
        DuplicateKeyError { kind: Other }
    }
}

fn is_null(s: &[u8]) -> bool {
    matches!(s, b"null" | b"Null" | b"NULL" | b"~")
}

fn parse_bool(s: &str) -> Option<bool> {
    match s {
        "true" | "True" | "TRUE" => Some(true),
        "false" | "False" | "FALSE" => Some(false),
        _ => None,
    }
}

impl Display for DuplicateKeyError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        use DuplicateKeyKind::{Bool, Null, Number, Other, String};
        formatter.write_str("duplicate entry ")?;
        match &self.kind {
            Null => formatter.write_str("with null key"),
            Bool(b) => write!(formatter, "with key `{}`", b),
            Number(n) => write!(formatter, "with key {}", n),
            String(s) => write!(formatter, "with key {:?}", s),
            Other => formatter.write_str("in YAML map"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_bool, is_null, DuplicateKeyError, DuplicateKeyKind};
    use crate::number::Number;

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
        assert_eq!(parse_bool("true"), Some(true));
        assert_eq!(parse_bool("True"), Some(true));
        assert_eq!(parse_bool("TRUE"), Some(true));
        assert_eq!(parse_bool("false"), Some(false));
        assert_eq!(parse_bool("False"), Some(false));
        assert_eq!(parse_bool("FALSE"), Some(false));
        assert_eq!(parse_bool("other"), None);
    }

    #[test]
    fn test_from_scalar_parsing() {
        let err = DuplicateKeyError::from_scalar(b"null");
        matches!(err.kind, DuplicateKeyKind::Null);

        let err = DuplicateKeyError::from_scalar(b"true");
        matches!(err.kind, DuplicateKeyKind::Bool(true));

        let err = DuplicateKeyError::from_scalar(b"42");
        assert!(matches!(err.kind, DuplicateKeyKind::Number(n) if n == Number::from(42)));
    }

    #[test]
    fn test_display() {
        let err = DuplicateKeyError::from_scalar(b"dup");
        assert_eq!(format!("{}", err), "duplicate entry with key \"dup\"");
    }
}

