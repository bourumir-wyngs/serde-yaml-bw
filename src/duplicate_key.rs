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
        use DuplicateKeyKind::*;
        let kind = match value {
            Value::Null => Null,
            Value::Bool(b) => Bool(*b),
            Value::Number(n) => Number(n.clone()),
            Value::String(s) => String(s.clone()),
            _ => Other,
        };
        DuplicateKeyError { kind }
    }

    pub(crate) fn from_scalar(bytes: &[u8]) -> Self {
        use DuplicateKeyKind::*;
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
        use DuplicateKeyKind::*;
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

