//! The Value enum, a loosely typed way of representing any valid YAML value.

mod de;
mod debug;
mod from;
mod index;
mod partial_eq;
mod resolve_aliases;
mod ser;
pub mod tagged;

use crate::error::{self, Error, ErrorImpl};
use serde::de::{Deserialize, DeserializeOwned, IntoDeserializer};
use serde::Serialize;
use std::hash::{Hash, Hasher};
use std::mem;
use std::sync::atomic::{AtomicUsize, Ordering};

pub use self::index::Index;
pub use self::ser::Serializer;
pub use self::tagged::{Tag, TaggedValue};
#[doc(inline)]
pub use crate::mapping::Mapping;
pub use crate::number::Number;

static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

pub(crate) fn next_id() -> usize {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

/// Represents any valid YAML value or in some cases error
#[derive(Clone, PartialEq, PartialOrd)]
pub enum Value {
    /// Represents a YAML null value.
    Null(Option<String>),
    /// Represents a YAML boolean.
    Bool(bool, Option<String>),
    /// Represents a YAML numerical value, whether integer or floating point.
    Number(Number, Option<String>),
    /// Represents a YAML string.
    String(String, Option<String>),
    /// Represents a YAML sequence in which the elements are
    /// `serde_yaml_bw::Value`.
    Sequence(Sequence),
    /// Represents a YAML mapping in which the keys and values are both
    /// `serde_yaml_bw::Value`.
    Mapping(Mapping),
    /// Represents an alias reference to an anchored node.
    Alias(String),
    /// A representation of YAML's `!Tag` syntax, used for enums.
    Tagged(Box<TaggedValue>),
}

/// The default value is `Value::Null(None)`.
///
/// This is useful for handling omitted `Value` fields when deserializing.
///
/// # Examples
///
/// ```
/// # use serde::Deserialize;
/// use serde_yaml_bw::Value;
///
/// #[derive(Deserialize)]
/// struct Settings {
///     level: i32,
///     #[serde(default)]
///     extras: Value,
/// }
///
/// # fn try_main() -> Result<(), serde_yaml_bw::Error> {
/// let data = r#" { "level": 42 } "#;
/// let s: Settings = serde_yaml_bw::from_str(data)?;
///
/// assert_eq!(s.level, 42);
/// assert_eq!(s.extras, Value::Null(None));
/// #
/// #     Ok(())
/// # }
/// #
/// # try_main().unwrap()
/// ```
impl Default for Value {
    fn default() -> Value {
        Value::Null(None)
    }
}

/// A YAML sequence in which the elements are `serde_yaml_bw::Value`.
#[derive(Clone, Debug, Default)]
pub struct Sequence {
    /// Optional anchor associated with this sequence.
    pub anchor: Option<String>,
    /// Elements of the YAML sequence.
    pub elements: Vec<Value>,
}

impl PartialEq for Sequence {
    fn eq(&self, other: &Self) -> bool {
        self.anchor == other.anchor && self.elements == other.elements
    }
}

impl Eq for Sequence {}

impl PartialOrd for Sequence {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.anchor.partial_cmp(&other.anchor) {
            Some(std::cmp::Ordering::Equal) => self.elements.partial_cmp(&other.elements),
            non_eq => non_eq,
        }
    }
}

impl std::hash::Hash for Sequence {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.anchor.hash(state);
        self.elements.hash(state);
    }
}

impl serde::Serialize for Sequence {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.elements.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for Sequence {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let elements = Vec::<Value>::deserialize(deserializer)?;
        Ok(Sequence {
            anchor: None,
            elements,
        })
    }
}

impl<'a> IntoIterator for &'a Sequence {
    type Item = &'a Value;
    type IntoIter = std::slice::Iter<'a, Value>;
    fn into_iter(self) -> Self::IntoIter {
        self.elements.iter()
    }
}

impl<'a> IntoIterator for &'a mut Sequence {
    type Item = &'a mut Value;
    type IntoIter = std::slice::IterMut<'a, Value>;
    fn into_iter(self) -> Self::IntoIter {
        self.elements.iter_mut()
    }
}

impl IntoIterator for Sequence {
    type Item = Value;
    type IntoIter = std::vec::IntoIter<Value>;
    fn into_iter(self) -> Self::IntoIter {
        self.elements.into_iter()
    }
}

impl std::ops::Deref for Sequence {
    type Target = Vec<Value>;
    fn deref(&self) -> &Self::Target {
        &self.elements
    }
}

impl std::ops::DerefMut for Sequence {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.elements
    }
}

impl Sequence {
    /// Creates an empty YAML sequence.
    #[inline]
    pub fn new() -> Self {
        Sequence {
            anchor: None,
            elements: Vec::new(),
        }
    }

    /// Creates an empty YAML sequence with the given initial capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Sequence {
            anchor: None,
            elements: Vec::with_capacity(capacity),
        }
    }

    /// Creates an empty YAML sequence with the given anchor.
    #[inline]
    pub fn with_anchor(anchor: impl Into<String>) -> Self {
        Sequence {
            anchor: Some(anchor.into()),
            elements: Vec::new(),
        }
    }

    /// Const constructor used for statics.
    pub const fn const_new() -> Self {
        Sequence {
            anchor: None,
            elements: Vec::new(),
        }
    }
}

/// Convert a `T` into `serde_yaml_bw::Value` which is an enum that can represent
/// any valid YAML data.
///
/// This conversion can fail if `T`'s implementation of `Serialize` decides to
/// return an error.
///
/// ```
/// # use serde_yaml_bw::Value;
/// let val = serde_yaml_bw::to_value("s").unwrap();
/// assert_eq!(val, Value::String("s".to_owned(), None));
/// ```
pub fn to_value<T>(value: T) -> Result<Value, Error>
where
    T: Serialize,
{
    value.serialize(Serializer)
}

/// Interpret a `serde_yaml_bw::Value` as an instance of type `T`.
///
/// This conversion can fail if the structure of the Value does not match the
/// structure expected by `T`, for example if `T` is a struct type but the Value
/// contains something other than a YAML map. It can also fail if the structure
/// is correct but `T`'s implementation of `Deserialize` decides that something
/// is wrong with the data, for example required struct fields are missing from
/// the YAML map or some number is too big to fit in the expected primitive
/// type.
///
/// ```
/// # use serde_yaml_bw::Value;
/// let val = Value::String("foo".to_owned(), None);
/// let s: String = serde_yaml_bw::from_value(val).unwrap();
/// assert_eq!("foo", s);
/// ```
pub fn from_value<T>(value: Value) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    Deserialize::deserialize(value)
}

impl Value {
    /// Index into a YAML sequence or map. A string index can be used to access
    /// a value in a map, and a usize index can be used to access an element of
    /// an sequence.
    ///
    /// Returns `None` if the type of `self` does not match the type of the
    /// index, for example if the index is a string and `self` is a sequence or
    /// a number. Also returns `None` if the given key does not exist in the map
    /// or the given index is not within the bounds of the sequence.
    ///
    /// ```
    /// # fn main() -> serde_yaml_bw::Result<()> {
    /// use serde_yaml_bw::Value;
    ///
    /// let object: Value = serde_yaml_bw::from_str(r#"{ A: 65, B: 66, C: 67 }"#)?;
    /// let x = object.get("A").unwrap();
    /// assert_eq!(x, 65);
    ///
    /// let sequence: Value = serde_yaml_bw::from_str(r#"[ "A", "B", "C" ]"#)?;
    /// let x = sequence.get(2).unwrap();
    /// assert_eq!(x, &Value::String("C".into(), None));
    ///
    /// assert_eq!(sequence.get("A"), None);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Square brackets can also be used to index into a value in a more concise
    /// way. This returns `Value::Null` in cases where `get` would have returned
    /// `None`.
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// #
    /// # fn main() -> serde_yaml_bw::Result<()> {
    /// let object: Value = serde_yaml_bw::from_str(r#"
    /// A: [a, á, à]
    /// B: [b, b́]
    /// C: [c, ć, ć̣, ḉ]
    /// 42: true
    /// "#)?;
    /// assert_eq!(object["B"][0], Value::String("b".into(), None));
    ///
    /// assert_eq!(object[Value::String("D".into(), None)], Value::Null(None));
    /// assert_eq!(object["D"], Value::Null(None));
    /// assert_eq!(object[0]["x"]["y"]["z"], Value::Null(None));
    ///
    /// assert_eq!(object[42], Value::Bool(true, None));
    /// # Ok(())
    /// # }
    /// ```
    pub fn get<I: Index>(&self, index: I) -> Option<&Value> {
        index.index_into(self)
    }

    /// Returns true if the `Value` is a Null. Returns false otherwise.
    ///
    /// For any Value on which `is_null` returns true, `as_null` is guaranteed
    /// to return `Some(())`.
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("null").unwrap();
    /// assert!(v.is_null());
    /// ```
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("false").unwrap();
    /// assert!(!v.is_null());
    /// ```
    pub fn is_null(&self) -> bool {
        matches!(self.untag_ref(), Value::Null(_))
    }

    /// If the `Value` is a Null, returns (). Returns None otherwise.
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("null").unwrap();
    /// assert_eq!(v.as_null(), Some(()));
    /// ```
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("false").unwrap();
    /// assert_eq!(v.as_null(), None);
    /// ```
    pub fn as_null(&self) -> Option<()> {
        match self.untag_ref() {
            Value::Null(_) => Some(()),
            _ => None,
        }
    }

    /// Returns true if the `Value` is a Boolean. Returns false otherwise.
    ///
    /// For any Value on which `is_boolean` returns true, `as_bool` is
    /// guaranteed to return the boolean value.
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("true").unwrap();
    /// assert!(v.is_bool());
    /// ```
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("42").unwrap();
    /// assert!(!v.is_bool());
    /// ```
    pub fn is_bool(&self) -> bool {
        matches!(self.untag_ref(), Value::Bool(_, _))
    }

    /// If the `Value` is a Boolean, returns the associated bool. Returns None
    /// otherwise.
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("true").unwrap();
    /// assert_eq!(v.as_bool(), Some(true));
    /// ```
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("42").unwrap();
    /// assert_eq!(v.as_bool(), None);
    /// ```
    pub fn as_bool(&self) -> Option<bool> {
        match self.untag_ref() {
            Value::Bool(b, _) => Some(*b),
            _ => None,
        }
    }

    /// Returns true if the `Value` is a Number. Returns false otherwise.
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("5").unwrap();
    /// assert!(v.is_number());
    /// ```
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("true").unwrap();
    /// assert!(!v.is_number());
    /// ```
    pub fn is_number(&self) -> bool {
        matches!(self.untag_ref(), Value::Number(_, _))
    }

    /// Returns true if the `Value` is an integer between `i64::MIN` and
    /// `i64::MAX`.
    ///
    /// For any Value on which `is_i64` returns true, `as_i64` is guaranteed to
    /// return the integer value.
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("1337").unwrap();
    /// assert!(v.is_i64());
    /// ```
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("null").unwrap();
    /// assert!(!v.is_i64());
    /// ```
    pub fn is_i64(&self) -> bool {
        matches!(self.untag_ref(), Value::Number(n, _) if n.is_i64())
    }

    /// If the `Value` is an integer, represent it as i64 if possible. Returns
    /// None otherwise.
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("1337").unwrap();
    /// assert_eq!(v.as_i64(), Some(1337));
    /// ```
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("false").unwrap();
    /// assert_eq!(v.as_i64(), None);
    /// ```
    pub fn as_i64(&self) -> Option<i64> {
        match self.untag_ref() {
            Value::Number(n, _) => n.as_i64(),
            _ => None,
        }
    }

    /// Returns true if the `Value` is an integer between `u64::MIN` and
    /// `u64::MAX`.
    ///
    /// For any Value on which `is_u64` returns true, `as_u64` is guaranteed to
    /// return the integer value.
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("1337").unwrap();
    /// assert!(v.is_u64());
    /// ```
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("null").unwrap();
    /// assert!(!v.is_u64());
    /// ```
    pub fn is_u64(&self) -> bool {
        matches!(self.untag_ref(), Value::Number(n, _) if n.is_u64())
    }

    /// If the `Value` is an integer, represent it as u64 if possible. Returns
    /// None otherwise.
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("1337").unwrap();
    /// assert_eq!(v.as_u64(), Some(1337));
    /// ```
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("false").unwrap();
    /// assert_eq!(v.as_u64(), None);
    /// ```
    pub fn as_u64(&self) -> Option<u64> {
        match self.untag_ref() {
            Value::Number(n, _) => n.as_u64(),
            _ => None,
        }
    }

    /// Returns true if the `Value` is a number that can be represented by f64.
    ///
    /// For any Value on which `is_f64` returns true, `as_f64` is guaranteed to
    /// return the floating point value.
    ///
    /// Currently this function returns true if and only if both `is_i64` and
    /// `is_u64` return false but this is not a guarantee in the future.
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("256.01").unwrap();
    /// assert!(v.is_f64());
    /// ```
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("true").unwrap();
    /// assert!(!v.is_f64());
    /// ```
    pub fn is_f64(&self) -> bool {
        matches!(self.untag_ref(), Value::Number(n, _) if n.is_f64())
    }

    /// If the `Value` is a number, represent it as f64 if possible. Returns
    /// None otherwise.
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("13.37").unwrap();
    /// assert_eq!(v.as_f64(), Some(13.37));
    /// ```
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("false").unwrap();
    /// assert_eq!(v.as_f64(), None);
    /// ```
    pub fn as_f64(&self) -> Option<f64> {
        match self.untag_ref() {
            Value::Number(i, _) => i.as_f64(),
            _ => None,
        }
    }

    /// Returns true if the `Value` is a String. Returns false otherwise.
    ///
    /// For any Value on which `is_string` returns true, `as_str` is guaranteed
    /// to return the string slice.
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("'lorem ipsum'").unwrap();
    /// assert!(v.is_string());
    /// ```
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("42").unwrap();
    /// assert!(!v.is_string());
    /// ```
    pub fn is_string(&self) -> bool {
        matches!(self.untag_ref(), Value::String(_, _))
    }

    /// If the `Value` is a String, returns the associated str. Returns None
    /// otherwise.
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("'lorem ipsum'").unwrap();
    /// assert_eq!(v.as_str(), Some("lorem ipsum"));
    /// ```
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("false").unwrap();
    /// assert_eq!(v.as_str(), None);
    /// ```
    pub fn as_str(&self) -> Option<&str> {
        match self.untag_ref() {
            Value::String(s, _) => Some(s),
            _ => None,
        }
    }

    /// Returns true if the `Value` is a sequence. Returns false otherwise.
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("[1, 2, 3]").unwrap();
    /// assert!(v.is_sequence());
    /// ```
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("true").unwrap();
    /// assert!(!v.is_sequence());
    /// ```
    pub fn is_sequence(&self) -> bool {
        matches!(self.untag_ref(), Value::Sequence(_))
    }

    /// If the `Value` is a sequence, return a reference to it if possible.
    /// Returns None otherwise.
    ///
    /// ```
    /// # use serde_yaml_bw::{Value, Number, Sequence};
    /// let v: Value = serde_yaml_bw::from_str("[1, 2]").unwrap();
    /// let mut expected = Sequence::new();
    /// expected.elements = vec![
    ///     Value::Number(Number::from(1), None),
    ///     Value::Number(Number::from(2), None),
    /// ];
    /// let expected = expected;
    /// assert_eq!(v.as_sequence(), Some(&expected));
    /// ```
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("false").unwrap();
    /// assert_eq!(v.as_sequence(), None);
    /// ```
    pub fn as_sequence(&self) -> Option<&Sequence> {
        match self.untag_ref() {
            Value::Sequence(seq) => Some(seq),
            _ => None,
        }
    }

    /// If the `Value` is a sequence, return a mutable reference to it if
    /// possible. Returns None otherwise.
    ///
    /// ```
    /// # use serde_yaml_bw::{Value, Number, Sequence};
    /// let mut v: Value = serde_yaml_bw::from_str("[1]").unwrap();
    /// let s = v.as_sequence_mut().unwrap();
    /// s.push(Value::Number(Number::from(2), None));
    /// let mut expected = Sequence::new();
    /// expected.elements = vec![
    ///     Value::Number(Number::from(1), None),
    ///     Value::Number(Number::from(2), None),
    /// ];
    /// let expected = expected;
    /// assert_eq!(s, &expected);
    /// ```
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let mut v: Value = serde_yaml_bw::from_str("false").unwrap();
    /// assert_eq!(v.as_sequence_mut(), None);
    /// ```
    pub fn as_sequence_mut(&mut self) -> Option<&mut Sequence> {
        match self.untag_mut() {
            Value::Sequence(seq) => Some(seq),
            _ => None,
        }
    }

    /// Returns true if the `Value` is a mapping. Returns false otherwise.
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("a: 42").unwrap();
    /// assert!(v.is_mapping());
    /// ```
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("true").unwrap();
    /// assert!(!v.is_mapping());
    /// ```
    pub fn is_mapping(&self) -> bool {
        matches!(self.untag_ref(), Value::Mapping(_))
    }

    /// If the `Value` is a mapping, return a reference to it if possible.
    /// Returns None otherwise.
    ///
    /// ```
    /// # use serde_yaml_bw::{Value, Mapping, Number};
    /// let v: Value = serde_yaml_bw::from_str("a: 42").unwrap();
    ///
    /// let mut expected = Mapping::new();
    /// expected.insert(Value::String("a".into(), None), Value::Number(Number::from(42), None));
    ///
    /// assert_eq!(v.as_mapping(), Some(&expected));
    /// ```
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// let v: Value = serde_yaml_bw::from_str("false").unwrap();
    /// assert_eq!(v.as_mapping(), None);
    /// ```
    pub fn as_mapping(&self) -> Option<&Mapping> {
        match self.untag_ref() {
            Value::Mapping(map) => Some(map),
            _ => None,
        }
    }

    /// If the `Value` is a mapping, return a reference to it if possible.
    /// Returns None otherwise.
    ///
    /// ```
    /// # use serde_yaml_bw::{Value, Mapping, Number};
    /// let mut v: Value = serde_yaml_bw::from_str("a: 42").unwrap();
    /// let m = v.as_mapping_mut().unwrap();
    /// m.insert(Value::String("b".into(), None), Value::Number(Number::from(21), None));
    ///
    /// let mut expected = Mapping::new();
    /// expected.insert(Value::String("a".into(), None), Value::Number(Number::from(42), None));
    /// expected.insert(Value::String("b".into(), None), Value::Number(Number::from(21), None));
    ///
    /// assert_eq!(m, &expected);
    /// ```
    ///
    /// ```
    /// # use serde_yaml_bw::{Value, Mapping};
    /// let mut v: Value = serde_yaml_bw::from_str("false").unwrap();
    /// assert_eq!(v.as_mapping_mut(), None);
    /// ```
    pub fn as_mapping_mut(&mut self) -> Option<&mut Mapping> {
        match self.untag_mut() {
            Value::Mapping(map) => Some(map),
            _ => None,
        }
    }

    /// Performs merging of `<<` keys into the surrounding mapping.
    ///
    /// The intended use of this in YAML is described in
    /// <https://yaml.org/type/merge.html>.
    ///
    /// ```
    /// use serde_yaml_bw::Value;
    ///
    /// let config = "\
    /// tasks:
    ///   build: &webpack_shared
    ///     command: webpack
    ///     args: build
    ///     inputs:
    ///       - 'src/**/*'
    ///   start:
    ///     <<: *webpack_shared
    ///     args: start
    /// ";
    ///
    /// let value: Value = serde_yaml_bw::from_str_value(config).unwrap();
    ///
    /// assert_eq!(value["tasks"]["start"]["command"], "webpack");
    /// assert_eq!(value["tasks"]["start"]["args"], "start");
    /// ```
    pub fn apply_merge(&mut self) -> Result<(), Error> {
        use std::collections::HashSet;
        let mut stack = Vec::new();
        let mut visited = HashSet::new();
        stack.push(self);
        while let Some(node) = stack.pop() {
            match node {
                Value::Mapping(mapping) => {
                    let id = mapping.id;
                    if id != 0 && !visited.insert(id) {
                        return Err(error::new(ErrorImpl::MergeRecursion));
                    }
                    match mapping.remove("<<") {
                        Some(Value::Mapping(merge)) => {
                            for (k, v) in merge {
                                mapping.entry(k).or_insert(v);
                            }
                        }
                        Some(Value::Sequence(sequence)) => {
                            for value in sequence.into_iter().rev() {
                                match value {
                                    Value::Mapping(merge) => {
                                        for (k, v) in merge {
                                            mapping.entry(k).or_insert(v);
                                        }
                                    }
                                    Value::Sequence(_) => {
                                        return Err(error::new(ErrorImpl::SequenceInMergeElement));
                                    }
                                    Value::Tagged(_) => {
                                        return Err(error::new(ErrorImpl::TaggedInMerge));
                                    }
                                    _unexpected => {
                                        return Err(error::new(ErrorImpl::ScalarInMergeElement));
                                    }
                                }
                            }
                        }
                        None => {}
                        Some(Value::Tagged(_)) => return Err(error::new(ErrorImpl::TaggedInMerge)),
                        Some(_unexpected) => return Err(error::new(ErrorImpl::ScalarInMerge)),
                    }
                    stack.extend(mapping.values_mut());
                }
                Value::Sequence(sequence) => stack.extend(sequence),
                Value::Tagged(tagged) => stack.push(&mut tagged.value),
                _ => {}
            }
        }
        Ok(())
    }
}

impl Eq for Value {}

// NOTE: This impl must be kept consistent with HashLikeValue's Hash impl in
// mapping.rs in order for value[str] indexing to work.
impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        mem::discriminant(self).hash(state);
        match self {
            Value::Null(_) => {}
            Value::Bool(v, _) => v.hash(state),
            Value::Number(v, _) => v.hash(state),
            Value::String(v, _) => v.hash(state),
            Value::Sequence(v) => v.hash(state),
            Value::Mapping(v) => v.hash(state),
            Value::Alias(v) => v.hash(state),
            Value::Tagged(v) => v.hash(state),
        }
    }
}

impl IntoDeserializer<'_, Error> for Value {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

impl Value {
    /// Wraps an array into a sequence (with no anchor).
    ///
    /// This method is handy when converting an array of objects that can be
    /// individually converted into [`Value`]s:
    ///
    /// ```
    /// use serde_yaml_bw::Value;
    ///
    /// fn to_value(x: &str) -> Value {
    ///     match x {
    ///         "one" => Value::from(1_usize),
    ///         "two_half" => Value::from(2.5_f32),
    ///         _ => Value::Null(None),
    ///     }
    /// }
    ///
    /// let annotation_objects = vec!["one", "two_half"];
    /// let annotations: Value = Value::from_vector(
    ///     annotation_objects
    ///         .iter()
    ///         .map(|annotation_object| to_value(annotation_object))
    ///         .collect(),
    /// );
    ///
    /// assert_eq!(annotations.as_sequence().unwrap().len(), 2);
    /// ```
    pub fn from_vector(values: Vec<Value>) -> Self {
        Value::Sequence(Sequence {
            anchor: None,
            elements: values
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::de::from_str_value_preserve;
    use crate::value::{Tag, TaggedValue};
    use indoc::indoc;

    #[test]
    fn test_apply_merge_example() {
        let config = indoc! {r#"
            tasks:
              build: &webpack_shared
                command: webpack
                args: build
                inputs:
                  - 'src/**/*'
              start:
                <<: *webpack_shared
                args: start
        "#};

        let mut value: Value = crate::from_str(config).unwrap();
        value.apply_merge().unwrap();
        value.resolve_aliases().unwrap();

        assert_eq!(value["tasks"]["start"]["command"], "webpack");
        assert_eq!(value["tasks"]["start"]["args"], "start");
    }

    #[test]
    fn test_scalar_in_merge() {
        let yaml = indoc!(
            r#"
            <<: 1
            a: 2
        "#
        );
        let mut value: Value = from_str_value_preserve(yaml).unwrap();
        let err = value.apply_merge().unwrap_err();
        assert_eq!(
            err.to_string(),
            "expected a mapping or list of mappings for merging, but found scalar"
        );
    }

    #[test]
    fn test_tagged_in_merge() {
        let yaml = indoc!(
            r#"
            <<: {}
            a: 2
        "#
        );
        let mut value: Value = from_str_value_preserve(yaml).unwrap();
        if let Value::Mapping(ref mut map) = value {
            let merge = map.get_mut("<<").unwrap();
            let inner = std::mem::take(merge);
            let tag = Tag::new("foo").unwrap();
            *merge = Value::Tagged(Box::new(TaggedValue { tag, value: inner }));
        } else {
            panic!("expected mapping");
        }
        let err = value.apply_merge().unwrap_err();
        assert_eq!(err.to_string(), "unexpected tagged value in merge");
    }

    #[test]
    fn test_scalar_in_merge_element() {
        let yaml = indoc!(
            r#"
            <<: [1]
            a: 2
        "#
        );
        let mut value: Value = from_str_value_preserve(yaml).unwrap();
        let err = value.apply_merge().unwrap_err();
        assert_eq!(
            err.to_string(),
            "expected a mapping for merging, but found scalar"
        );
    }

    #[test]
    fn test_sequence_in_merge_element() {
        let yaml = indoc!(
            r#"
            <<:
              - [1, 2]
            a: 2
        "#
        );
        let mut value: Value = from_str_value_preserve(yaml).unwrap();
        let err = value.apply_merge().unwrap_err();
        assert_eq!(
            err.to_string(),
            "expected a mapping for merging, but found sequence"
        );
    }

    #[test]
    fn test_merge_recursion() {
        let yaml = indoc!(
            r#"
            a: &a
              b: 1
        "#
        );
        let mut value: Value = from_str_value_preserve(yaml).unwrap();
        if let Value::Mapping(map) = &mut value {
            if let Some(Value::Mapping(a_map)) = map.get_mut("a") {
                let mut clone = a_map.clone();
                clone.id = a_map.id;
                a_map.insert("self".into(), Value::Mapping(clone));
            }
        }
        let err = value.apply_merge().unwrap_err();
        assert_eq!(err.to_string(), "encountered recursive merge alias");
    }

    #[test]
    fn unresolved_alias_error() {
        let yaml = "anchor: &id 1\nalias: *id";
        let mut value: Value = from_str_value_preserve(yaml).unwrap();

        if let Some(Value::Number(_, anchor)) = value.as_mapping_mut().unwrap().get_mut("anchor") {
            *anchor = None;
        }

        let err = value.resolve_aliases().unwrap_err();
        assert_eq!(err.to_string(), "unresolved alias");
    }

    #[test]
    fn cyclic_aliases_error() {
        let yaml = "a: &a\n  ref: *a\n";
        let mut value: Value = from_str_value_preserve(yaml).unwrap();

        let err = value.resolve_aliases().unwrap_err();
        assert_eq!(err.to_string(), "encountered recursive merge alias");
    }

    #[test]
    fn test_field_inheritance() {
        let yaml_input = r#"
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

        let parsed: Value = from_str_value_preserve(yaml_input).unwrap();

        let serialized = crate::to_string(&parsed).unwrap();

        if serialized.matches("adapter: postgres").count() != 1 {
            panic!("Anchors and aliases were not correctly preserved; duplication detected. Serialized output {serialized}");
        }
    }

    fn assert_same_entries(a: &Value, b: &Value) {
        let a = a.as_mapping().expect("a: expected a mapping");
        let b = b.as_mapping().expect("b: expected a mapping");

        assert_eq!(a.len(), b.len());
        for a_key in a.keys() {
            assert!(b.contains_key(a_key));
            let a_value = a
                .get(a_key)
                .unwrap_or_else(|| panic!("key not present in a: {a_key:?}"))
                .as_str();
            let b_value = b
                .get(a_key)
                .unwrap_or_else(|| panic!("key not present in b: {a_key:?}"))
                .as_str();

            assert_eq!(
                a_value,
                b_value,
                "key {:?} has different values: a {:?}, b {:?}",
                a_key.as_str(),
                a_value,
                b_value
            );
        }
    }

    #[test]
    fn test_merge_key_example() {
        let yaml = r#"
- &CENTER { x: 1, y: 2 }
- &LEFT { x: 0, y: 2 }
- &BIG { r: 10 }
- &SMALL { r: 1 }

- x: 1
  y: 2
  r: 10
  label: center/big

- <<: *CENTER
  r: 10
  label: center/big

- <<: [ *CENTER, *BIG ]
  label: center/big

# And here we have it all:
- <<: [ *BIG, *LEFT, { x: 1, y: 2 } ]
  label: center/big
"#;

        let value: Value = crate::from_str(yaml).unwrap();

        let seq = value.as_sequence().expect("root should be a sequence");
        assert_eq!(seq.len(), 8);

        let base = &seq[4];
        assert_same_entries(base, &seq[5]);
        assert_same_entries(base, &seq[6]);
        assert_same_entries(base, &seq[7]);
    }

    #[test]
    fn test_self_referential_merge() {
        let yaml = "a: &a\n  b: 1\n  <<: *a";
        let mut value: Value = from_str_value_preserve(yaml).unwrap();
        assert!(value.apply_merge().is_err());
    }

    #[test]
    fn test_self_referential_after_reallocation() {
        let yaml = "a: &a\n  b: 1\n  <<: *a";
        let value: Value = from_str_value_preserve(yaml).unwrap();
        let mut vec = Vec::new();
        vec.push(Value::Null(None));
        vec.push(value);
        for _ in 0..100 {
            vec.push(Value::Null(None));
        }
        let mut moved = vec.remove(1);
        assert!(moved.apply_merge().is_err());
    }
}
