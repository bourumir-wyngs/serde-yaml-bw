use crate::{mapping, private, Value};
use std::ops;

/// A type that can be used to index into a `serde_yaml_bw::Value`. 
/// See the `get` methods of `Value`.
///
/// This trait is sealed and cannot be implemented for types outside of
/// `serde_yaml_bw`.
pub trait Index: private::Sealed {
    /// Return None if the key is not already in the sequence or object.
    #[doc(hidden)]
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value>;
}

impl Index for usize {
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value> {
        match v.untag_ref() {
            Value::Sequence(vec) => vec.get(*self),
            Value::Mapping(vec) => vec.get(Value::Number((*self).into(), None)),
            _ => None,
        }
    }
}

fn index_into_mapping<'v, I>(index: &I, v: &'v Value) -> Option<&'v Value>
where
    I: ?Sized + mapping::Index,
{
    match v.untag_ref() {
        Value::Mapping(map) => map.get(index),
        _ => None,
    }
}

impl Index for Value {
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value> {
        index_into_mapping(self, v)
    }
}

impl Index for str {
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value> {
        index_into_mapping(self, v)
    }    
}

impl Index for String {
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value> {
        self.as_str().index_into(v)
    }
}

impl<T> Index for &T
where
    T: ?Sized + Index,
{
    fn index_into<'v>(&self, v: &'v Value) -> Option<&'v Value> {
        (**self).index_into(v)
    }
}

// The usual semantics of Index is to panic on invalid indexing.
//
// That said, the usual semantics are for things like `Vec` and `BTreeMap` which
// have different use cases than Value. If you are working with a Vec, you know
// that you are working with a Vec and you can get the len of the Vec and make
// sure your indices are within bounds. The Value use cases are more
// loosey-goosey. You got some YAML from an endpoint and you want to pull values
// out of it. Outside of this Index impl, you already have the option of using
// `value.as_sequence()` and working with the Vec directly, or matching on
// `Value::Sequence` and getting the Vec directly. The Index impl means you can
// skip that and index directly into the thing using a concise syntax. You don't
// have to check the type, you don't have to check the len, it is all about what
// you expect the Value to look like.
//
// Basically the use cases that would be well served by panicking here are
// better served by using one of the other approaches: `get` and `get_mut`,
// `as_sequence`, or match. The value of this impl is that it adds a way of
// working with Value that is not well served by the existing approaches:
// concise and careless and sometimes that is exactly what you want.
impl<I> ops::Index<I> for Value
where
    I: Index,
{
    type Output = Value;

    /// Index into a `serde_yaml_bw::Value` using the syntax `value[0]` or
    /// `value["k"]`.
    ///
    /// Returns `Value::Null` if the type of `self` does not match the type of
    /// the index, for example if the index is a string and `self` is a sequence
    /// or a number. Also returns `Value::Null` if the given key does not exist
    /// in the map or the given index is not within the bounds of the sequence.
    ///
    /// For retrieving deeply nested values, you should have a look at the
    /// `Value::pointer` method.
    ///
    /// # Examples
    ///
    /// ```
    /// # use serde_yaml_bw::Value;
    /// #
    /// # fn main() -> serde_yaml_bw::Result<()> {
    /// let data: serde_yaml_bw::Value = serde_yaml_bw::from_str(r#"{ x: { y: [z, zz] } }"#)?;
    ///
    /// assert_eq!(data["x"]["y"], serde_yaml_bw::from_str::<Value>(r#"["z", "zz"]"#).unwrap());
    /// assert_eq!(data["x"]["y"][0], serde_yaml_bw::from_str::<Value>(r#""z""#).unwrap());
    ///
    /// assert_eq!(data["a"], serde_yaml_bw::from_str::<Value>(r#"null"#).unwrap()); // returns null for undefined values
    /// assert_eq!(data["a"]["b"], serde_yaml_bw::from_str::<Value>(r#"null"#).unwrap()); // does not panic
    /// # Ok(())
    /// # }
    /// ```
    fn index(&self, index: I) -> &Value {
        static NULL: Value = Value::Null(None);
        index.index_into(self).unwrap_or(&NULL)
    }
}

