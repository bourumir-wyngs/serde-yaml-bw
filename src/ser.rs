//! YAML Serialization
//!
//! This module provides YAML serialization with the type `Serializer`.

use crate::error::{self, Error, ErrorImpl};
use crate::libyaml;
use crate::libyaml::emitter::{Emitter, Event, Mapping, Scalar, ScalarStyle, Sequence, SequenceStyle};
use crate::libyaml::tag::Tag;
use crate::value::tagged::{self, MaybeTag};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use serde::de::Visitor;
use serde::ser::{self, Serializer as _};
use std::collections::HashSet;
use std::fmt::{self, Display};
use std::io;
use std::mem;
use std::num;
use std::str;

pub(crate) const ALIAS_NEWTYPE: &str = "$serde_yaml::alias";
pub(crate) const ANCHOR_NEWTYPE: &str = "$serde_yaml::anchor";
pub(crate) const FLOW_SEQ_NEWTYPE: &str = "$serde_yaml::flow_seq";

type Result<T, E = Error> = std::result::Result<T, E>;

/// Wrapper type that serializes the contained sequence using YAML flow style.
///
/// # Example
///
/// ```
/// use serde::Serialize;
/// use serde_yaml_bw::{to_string, FlowSeq};
///
/// #[derive(Serialize)]
/// struct Data {
///     flow: FlowSeq<Vec<u32>>,
/// }
///
/// let yaml = to_string(&Data { flow: FlowSeq(vec![1, 2, 3]) }).unwrap();
/// assert_eq!(yaml, "flow: [1, 2, 3]\n");
/// ```
pub struct FlowSeq<T>(pub T);

impl<T> ser::Serialize for FlowSeq<T>
where
    T: ser::Serialize,
{
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_newtype_struct(FLOW_SEQ_NEWTYPE, &self.0)
    }
}

/// Builder to configure [`Serializer`].
/// ```
/// use serde::Serialize;
/// use serde_yaml_bw::SerializerBuilder;
///
/// #[derive(Serialize)]
/// struct Inner { value: u32 }
/// #[derive(Serialize)]
/// struct Outer { inner: Inner }
///
/// fn main() -> Result<(), serde_yaml_bw::Error> {
///     let mut buf = Vec::new();
///     let mut ser = SerializerBuilder::new()
///         .indent(4)
///         .width(80)
///         .build(&mut buf)?;
///     let data = Outer { inner: Inner { value: 1 } };
///     data.serialize(&mut ser)?;
///     drop(ser);
///     assert_eq!(buf, b"inner:\n    value: 1\n");
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct SerializerBuilder {
    /// Preferred line width. A value of `-1` disables line wrapping.
    width: i32,
    /// Number of spaces to indent nested structures.
    indent: i32,
    /// Scalar style to use for simple scalars when none is specified.
    scalar_style: ScalarStyle,
}

impl Default for SerializerBuilder {
    fn default() -> Self {
        Self {
            width: -1,
            indent: 2,
            scalar_style: ScalarStyle::Plain,
        }
    }
}

impl SerializerBuilder {
    /// Create a builder with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set preferred line width; `-1` means unlimited.
    pub fn width(mut self, width: i32) -> Self {
        self.width = width;
        self
    }

    /// Set indentation increment.
    pub fn indent(mut self, indent: i32) -> Self {
        self.indent = indent;
        self
    }

    /// Set default scalar style used for simple scalars.
    pub fn scalar_style(mut self, style: ScalarStyle) -> Self {
        self.scalar_style = style;
        self
    }

    /// Build a [`Serializer`] writing to the given writer.
    pub fn build<W: io::Write>(self, writer: W) -> Result<Serializer<W>> {
        let mut emitter = Emitter::new(writer, self.width, self.indent)?;
        emitter.emit(Event::StreamStart)?;
        Ok(Serializer {
            depth: 0,
            state: State::default(),
            tag_stack: Vec::new(),
            pending_anchor: None,
            anchors: HashSet::new(),
            emitter,
            default_scalar_style: self.scalar_style,
            next_sequence_style: None,
        })
    }
}

/// A structure for serializing Rust values into YAML.
///
/// # Example
///
/// ```
/// use anyhow::Result;
/// use serde::Serialize;
/// use std::collections::BTreeMap;
///
/// fn main() -> Result<()> {
///     let mut buffer = Vec::new();
///     let mut ser = serde_yaml_bw::Serializer::new(&mut buffer)?;
///
///     let mut object = BTreeMap::new();
///     object.insert("k", 107);
///     object.serialize(&mut ser)?;
///
///     object.insert("J", 74);
///     object.serialize(&mut ser)?;
///
///     drop(ser);
///     assert_eq!(buffer, b"k: 107\n---\nJ: 74\nk: 107\n");
///     Ok(())
/// }
/// ```
pub struct Serializer<W>
where
    W: io::Write,
{
    depth: usize,
    state: State,
    /// Stack of YAML tags currently in scope.
    tag_stack: Vec<String>,
    pending_anchor: Option<String>,
    anchors: HashSet<String>,
    emitter: Emitter<W>,
    default_scalar_style: ScalarStyle,
    next_sequence_style: Option<SequenceStyle>,
}

enum State {
    /// Serializer is idle and no special handling is in progress. This
    /// variant is returned by `Default`.
    NothingInParticular,
    CheckForTag,
    CheckForDuplicateTag,
    FoundTag(String),
    AlreadyTagged,
}

impl Default for State {
    fn default() -> Self {
        // New serializers start out with no special state.
        State::NothingInParticular
    }
}

impl<W> Serializer<W>
where
    W: io::Write,
{
    /// Creates a new YAML serializer.
    pub fn new(writer: W) -> Result<Self> {
        SerializerBuilder::new().build(writer)
    }

    /// Calls [`.flush()`](io::Write::flush) on the underlying `io::Write`
    /// object.
    pub fn flush(&mut self) -> Result<()> {
        self.emitter.flush()?;
        Ok(())
    }

    /// Return the underlying `io::Write` object from the `Serializer`.
    ///
    /// # Errors
    ///
    /// Returns an error if the writer has already been taken.
    pub fn into_inner(mut self) -> Result<W> {
        self.emitter.emit(Event::StreamEnd)?;
        self.emitter.flush()?;
        let writer = self.emitter.into_inner()?;
        Ok(writer)
    }

    fn emit_scalar(&mut self, mut scalar: Scalar) -> Result<()> {
        self.flush_mapping_start()?;
        if let Some(tag) = self.take_tag() {
            scalar.tag = Some(tag);
        }
        scalar.anchor = self.pending_anchor.take();
        if let Some(ref a) = scalar.anchor {
            self.anchors.insert(a.clone());
        }
        self.value_start()?;
        self.emitter.emit(Event::Scalar(scalar))?;
        self.value_end()
    }

    fn emit_sequence_start(&mut self, style: SequenceStyle) -> Result<()> {
        self.flush_mapping_start()?;
        self.value_start()?;
        let tag = self.take_tag();
        let anchor = self.pending_anchor.take();
        if let Some(ref a) = anchor {
            self.anchors.insert(a.clone());
        }
        let style = self.next_sequence_style.take().unwrap_or(style);
        self.emitter
            .emit(Event::SequenceStart(Sequence { anchor, tag, style }))?;
        Ok(())
    }

    fn emit_sequence_end(&mut self) -> Result<()> {
        self.emitter.emit(Event::SequenceEnd)?;
        self.value_end()
    }

    fn emit_mapping_start(&mut self) -> Result<()> {
        self.flush_mapping_start()?;
        self.value_start()?;
        let tag = self.take_tag();
        let anchor = self.pending_anchor.take();
        if let Some(ref a) = anchor {
            self.anchors.insert(a.clone());
        }
        self.emitter
            .emit(Event::MappingStart(Mapping { anchor, tag }))?;
        Ok(())
    }

    fn emit_mapping_end(&mut self) -> Result<()> {
        self.emitter.emit(Event::MappingEnd)?;
        self.value_end()
    }

    fn emit_alias(&mut self, anchor: &str) -> Result<()> {
        if !self.anchors.contains(anchor) {
            use crate::libyaml::error::Mark;
            use crate::libyaml::parser::Anchor as YamlAnchor;
            let mark = unsafe { mem::MaybeUninit::<Mark>::zeroed().assume_init() };
            let missing = YamlAnchor(anchor.as_bytes().to_vec().into_boxed_slice());
            return Err(error::new(ErrorImpl::UnknownAnchor(mark, missing)));
        }
        self.flush_mapping_start()?;
        self.value_start()?;
        self.emitter.emit(Event::Alias(anchor.to_owned()))?;
        self.value_end()
    }

    fn value_start(&mut self) -> Result<()> {
        if self.depth == 0 {
            self.emitter.emit(Event::DocumentStart)?;
        }
        self.depth += 1;
        Ok(())
    }

    fn value_end(&mut self) -> Result<()> {
        self.depth -= 1;
        if self.depth == 0 {
            self.emitter.emit(Event::DocumentEnd)?;
        }
        Ok(())
    }

    fn take_tag(&mut self) -> Option<String> {
        if let State::FoundTag(mut tag) = mem::take(&mut self.state) {
            if !tag.starts_with('!') {
                tag.insert(0, '!');
            }
            Some(tag)
        } else {
            None
        }
    }

    fn flush_mapping_start(&mut self) -> Result<()> {
        if let State::CheckForTag = self.state {
            self.state = State::NothingInParticular;
            self.emit_mapping_start()?;
        } else if let State::CheckForDuplicateTag = self.state {
            self.state = State::NothingInParticular;
        }
        Ok(())
    }
}

impl<W> ser::Serializer for &mut Serializer<W>
where
    W: io::Write,
{
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.emit_scalar(Scalar {
            anchor: None,
            tag: None,
            value: if v { "true" } else { "false" },
            style: self.default_scalar_style,
        })
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.emit_scalar(Scalar {
            anchor: None,
            tag: None,
            value: itoa::Buffer::new().format(v),
            style: self.default_scalar_style,
        })
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.emit_scalar(Scalar {
            anchor: None,
            tag: None,
            value: itoa::Buffer::new().format(v),
            style: self.default_scalar_style,
        })
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.emit_scalar(Scalar {
            anchor: None,
            tag: None,
            value: itoa::Buffer::new().format(v),
            style: self.default_scalar_style,
        })
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        self.emit_scalar(Scalar {
            anchor: None,
            tag: None,
            value: itoa::Buffer::new().format(v),
            style: self.default_scalar_style,
        })
    }

    fn serialize_i128(self, v: i128) -> Result<()> {
        self.emit_scalar(Scalar {
            anchor: None,
            tag: None,
            value: itoa::Buffer::new().format(v),
            style: self.default_scalar_style,
        })
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.emit_scalar(Scalar {
            anchor: None,
            tag: None,
            value: itoa::Buffer::new().format(v),
            style: self.default_scalar_style,
        })
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.emit_scalar(Scalar {
            anchor: None,
            tag: None,
            value: itoa::Buffer::new().format(v),
            style: self.default_scalar_style,
        })
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.emit_scalar(Scalar {
            anchor: None,
            tag: None,
            value: itoa::Buffer::new().format(v),
            style: self.default_scalar_style,
        })
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.emit_scalar(Scalar {
            anchor: None,
            tag: None,
            value: itoa::Buffer::new().format(v),
            style: self.default_scalar_style,
        })
    }

    fn serialize_u128(self, v: u128) -> Result<()> {
        self.emit_scalar(Scalar {
            anchor: None,
            tag: None,
            value: itoa::Buffer::new().format(v),
            style: self.default_scalar_style,
        })
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        let mut buffer = ryu::Buffer::new();
        self.emit_scalar(Scalar {
            anchor: None,
            tag: None,
            value: match v.classify() {
                num::FpCategory::Infinite if v.is_sign_positive() => ".inf",
                num::FpCategory::Infinite => "-.inf",
                num::FpCategory::Nan => ".nan",
                _ => buffer.format_finite(v),
            },
            style: self.default_scalar_style,
        })
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        let mut buffer = ryu::Buffer::new();
        self.emit_scalar(Scalar {
            anchor: None,
            tag: None,
            value: match v.classify() {
                num::FpCategory::Infinite if v.is_sign_positive() => ".inf",
                num::FpCategory::Infinite => "-.inf",
                num::FpCategory::Nan => ".nan",
                _ => buffer.format_finite(v),
            },
            style: self.default_scalar_style,
        })
    }

    fn serialize_char(self, value: char) -> Result<()> {
        self.emit_scalar(Scalar {
            anchor: None,
            tag: None,
            value: value.encode_utf8(&mut [0u8; 4]),
            style: ScalarStyle::SingleQuoted,
        })
    }

    fn serialize_str(self, value: &str) -> Result<()> {
        struct InferScalarStyle;

        impl Visitor<'_> for InferScalarStyle {
            type Value = ScalarStyle;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("I wonder")
            }

            fn visit_bool<E>(self, _v: bool) -> Result<Self::Value, E> {
                Ok(ScalarStyle::SingleQuoted)
            }

            fn visit_i64<E>(self, _v: i64) -> Result<Self::Value, E> {
                Ok(ScalarStyle::SingleQuoted)
            }

            fn visit_i128<E>(self, _v: i128) -> Result<Self::Value, E> {
                Ok(ScalarStyle::SingleQuoted)
            }

            fn visit_u64<E>(self, _v: u64) -> Result<Self::Value, E> {
                Ok(ScalarStyle::SingleQuoted)
            }

            fn visit_u128<E>(self, _v: u128) -> Result<Self::Value, E> {
                Ok(ScalarStyle::SingleQuoted)
            }

            fn visit_f64<E>(self, _v: f64) -> Result<Self::Value, E> {
                Ok(ScalarStyle::SingleQuoted)
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
                Ok(if crate::de::digits_but_not_number(v) {
                    ScalarStyle::SingleQuoted
                } else {
                    ScalarStyle::Any
                })
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(ScalarStyle::SingleQuoted)
            }
        }

        let style = if value.contains('\n') {
            ScalarStyle::Literal
        } else {
            let result = crate::de::visit_untagged_scalar(
                InferScalarStyle,
                value,
                None,
                libyaml::parser::ScalarStyle::Plain,
            );
            result.unwrap_or(ScalarStyle::Any)
        };

        self.emit_scalar(Scalar {
            anchor: None,
            tag: None,
            value,
            style,
        })
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<()> {
        let encoded = BASE64_STANDARD.encode(value);
        self.emit_scalar(Scalar {
            anchor: None,
            tag: Some(Tag::BINARY.into()),
            value: &encoded,
            style: self.default_scalar_style,
        })
    }

    fn serialize_unit(self) -> Result<()> {
        self.emit_scalar(Scalar {
            anchor: None,
            tag: None,
            value: "null",
            style: self.default_scalar_style,
        })
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T>(mut self, name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        if name == ALIAS_NEWTYPE {
            value.serialize(AliasHelper { ser: &mut self })
        } else if name == ANCHOR_NEWTYPE {
            value.serialize(AnchorHelper { ser: &mut self })
        } else if name == FLOW_SEQ_NEWTYPE {
            self.next_sequence_style = Some(SequenceStyle::Flow);
            let result = value.serialize(&mut *self);
            if self.next_sequence_style.is_some() {
                self.next_sequence_style = None;
                return Err(ser::Error::custom("flow sequence newtype must serialize a sequence"));
            }
            result
        } else {
            value.serialize(self)
        }
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.emit_mapping_start()?;
        self.serialize_str(variant)?;
        value.serialize(&mut *self)?;
        self.emit_mapping_end()
    }

    fn serialize_none(self) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_some<V>(self, value: &V) -> Result<()>
    where
        V: ?Sized + ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        self.emit_sequence_start(SequenceStyle::Any)?;
        Ok(self)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        self.emit_sequence_start(SequenceStyle::Any)?;
        Ok(self)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.emit_sequence_start(SequenceStyle::Any)?;
        Ok(self)
    }

    fn serialize_tuple_variant(
        self,
        _enm: &'static str,
        _idx: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.emit_mapping_start()?;
        self.serialize_str(variant)?;
        self.emit_sequence_start(SequenceStyle::Any)?;
        Ok(self)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        if len == Some(1) {
            self.state = if let State::FoundTag(_) = self.state {
                self.emit_mapping_start()?;
                State::CheckForDuplicateTag
            } else {
                State::CheckForTag
            };
        } else {
            self.emit_mapping_start()?;
        }
        Ok(self)
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        self.emit_mapping_start()?;
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _enm: &'static str,
        _idx: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.emit_mapping_start()?;
        self.serialize_str(variant)?;
        self.emit_mapping_start()?;
        Ok(self)
    }

    fn collect_str<T>(self, value: &T) -> Result<Self::Ok>
    where
        T: ?Sized + Display,
    {
        let string = if let State::CheckForTag | State::CheckForDuplicateTag = self.state {
            match tagged::check_for_tag(value) {
                MaybeTag::Error => return Err(error::new(ErrorImpl::TagError)),
                MaybeTag::NotTag(string) => string,
                MaybeTag::Tag(string) => {
                    self.state = State::FoundTag(string);
                    return Ok(());
                }
            }
        } else {
            value.to_string()
        };

        self.serialize_str(&string)
    }
}

impl<W> ser::SerializeSeq for &mut Serializer<W>
where
    W: io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, elem: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        elem.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.emit_sequence_end()?;
        if let Some(tag) = self.tag_stack.pop() {
            self.state = State::FoundTag(tag);
        }
        Ok(())
    }
}

impl<W> ser::SerializeTuple for &mut Serializer<W>
where
    W: io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, elem: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        elem.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.emit_sequence_end()
    }
}

impl<W> ser::SerializeTupleStruct for &mut Serializer<W>
where
    W: io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<V>(&mut self, value: &V) -> Result<()>
    where
        V: ?Sized + ser::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.emit_sequence_end()
    }
}

impl<W> ser::SerializeTupleVariant for &mut Serializer<W>
where
    W: io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<V>(&mut self, v: &V) -> Result<()>
    where
        V: ?Sized + ser::Serialize,
    {
        v.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.emit_sequence_end()?;
        self.emit_mapping_end()
    }
}

impl<W> ser::SerializeMap for &mut Serializer<W>
where
    W: io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.flush_mapping_start()?;
        key.serialize(&mut **self)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(&mut **self)
    }

    fn serialize_entry<K, V>(&mut self, key: &K, value: &V) -> Result<(), Self::Error>
    where
        K: ?Sized + ser::Serialize,
        V: ?Sized + ser::Serialize,
    {
        key.serialize(&mut **self)?;
        let tagged = matches!(self.state, State::FoundTag(_));
        value.serialize(&mut **self)?;
        if tagged {
            self.state = State::AlreadyTagged;
        }
        Ok(())
    }

    fn end(self) -> Result<()> {
        if let State::CheckForTag = self.state {
            self.emit_mapping_start()?;
        }
        if !matches!(self.state, State::AlreadyTagged) {
            self.emit_mapping_end()?;
        }
        self.state = State::NothingInParticular;
        Ok(())
    }
}

impl<W> ser::SerializeStruct for &mut Serializer<W>
where
    W: io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<V>(&mut self, key: &'static str, value: &V) -> Result<()>
    where
        V: ?Sized + ser::Serialize,
    {
        self.serialize_str(key)?;
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.emit_mapping_end()?;
        if let Some(tag) = self.tag_stack.pop() {
            self.state = State::FoundTag(tag);
        }
        Ok(())
    }
}

impl<W> ser::SerializeStructVariant for &mut Serializer<W>
where
    W: io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<V>(&mut self, field: &'static str, v: &V) -> Result<()>
    where
        V: ?Sized + ser::Serialize,
    {
        self.serialize_str(field)?;
        v.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.emit_mapping_end()?;
        self.emit_mapping_end()
    }
}

struct AnchorHelper<'a, W>
where
    W: io::Write,
{
    ser: &'a mut Serializer<W>,
}

struct AnchorSeq<'a, W>
where
    W: io::Write,
{
    ser: &'a mut Serializer<W>,
    state: AnchorState,
}

enum AnchorState {
    Anchor,
    Value,
    Done,
}

struct AnchorName<'a, W>
where
    W: io::Write,
{
    ser: &'a mut Serializer<W>,
}

#[inline]
fn anchor_must_be_string<T>() -> Result<T> {
    Err(ser::Error::custom("anchor must be a string"))
}

impl<'a, W> ser::Serializer for AnchorHelper<'a, W>
where
    W: io::Write,
{
    type Ok = ();
    type Error = Error;

    type SerializeSeq = AnchorSeq<'a, W>;
    type SerializeTuple = AnchorSeq<'a, W>;
    type SerializeTupleStruct = AnchorSeq<'a, W>;
    type SerializeTupleVariant = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeMap = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeStructVariant = ser::Impossible<Self::Ok, Self::Error>;

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        if len != Some(2) {
            return Err(ser::Error::custom("anchor requires tuple of (name, value)"));
        }
        Ok(AnchorSeq {
            ser: self.ser,
            state: AnchorState::Anchor,
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        if len != 2 {
            return Err(ser::Error::custom("anchor requires tuple of (name, value)"));
        }
        Ok(AnchorSeq {
            ser: self.ser,
            state: AnchorState::Anchor,
        })
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        if len != 2 {
            return Err(ser::Error::custom("anchor requires tuple of (name, value)"));
        }
        Ok(AnchorSeq {
            ser: self.ser,
            state: AnchorState::Anchor,
        })
    }

    fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("anchor requires tuple of (name, value)"))
    }
    fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("anchor requires tuple of (name, value)"))
    }
    fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("anchor requires tuple of (name, value)"))
    }
    fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("anchor requires tuple of (name, value)"))
    }
    fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("anchor requires tuple of (name, value)"))
    }
    fn serialize_i128(self, _v: i128) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("anchor requires tuple of (name, value)"))
    }
    fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("anchor requires tuple of (name, value)"))
    }
    fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("anchor requires tuple of (name, value)"))
    }
    fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("anchor requires tuple of (name, value)"))
    }
    fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("anchor requires tuple of (name, value)"))
    }
    fn serialize_u128(self, _v: u128) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("anchor requires tuple of (name, value)"))
    }
    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("anchor requires tuple of (name, value)"))
    }
    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("anchor requires tuple of (name, value)"))
    }
    fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("anchor requires tuple of (name, value)"))
    }
    fn serialize_str(self, _v: &str) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("anchor requires tuple of (name, value)"))
    }
    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("anchor requires tuple of (name, value)"))
    }
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("anchor requires tuple of (name, value)"))
    }
    fn serialize_some<T>(self, _value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        Err(ser::Error::custom("anchor requires tuple of (name, value)"))
    }
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("anchor requires tuple of (name, value)"))
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("anchor requires tuple of (name, value)"))
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Err(ser::Error::custom("anchor requires tuple of (name, value)"))
    }
    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        Err(ser::Error::custom("anchor requires tuple of (name, value)"))
    }
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        Err(ser::Error::custom("anchor requires tuple of (name, value)"))
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(ser::Error::custom("anchor requires tuple of (name, value)"))
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(ser::Error::custom("anchor requires tuple of (name, value)"))
    }
    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(ser::Error::custom("anchor requires tuple of (name, value)"))
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(ser::Error::custom("anchor requires tuple of (name, value)"))
    }
}

impl<'a, W> ser::SerializeSeq for AnchorSeq<'a, W>
where
    W: io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, elem: &T) -> Result<(), Error>
    where
        T: ?Sized + ser::Serialize,
    {
        match self.state {
            AnchorState::Anchor => {
                elem.serialize(AnchorName { ser: self.ser })?;
                self.state = AnchorState::Value;
                Ok(())
            }
            AnchorState::Value => {
                elem.serialize(&mut *self.ser)?;
                self.state = AnchorState::Done;
                Ok(())
            }
            AnchorState::Done => Err(ser::Error::custom("anchor accepts only two elements")),
        }
    }

    fn end(self) -> Result<(), Error> {
        if matches!(self.state, AnchorState::Value | AnchorState::Anchor) {
            Err(ser::Error::custom("anchor requires tuple of (name, value)"))
        } else {
            Ok(())
        }
    }
}

impl<'a, W> ser::SerializeTuple for AnchorSeq<'a, W>
where
    W: io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, elem: &T) -> Result<(), Error>
    where
        T: ?Sized + ser::Serialize,
    {
        ser::SerializeSeq::serialize_element(self, elem)
    }

    fn end(self) -> Result<(), Error> {
        ser::SerializeSeq::end(self)
    }
}

impl<'a, W> ser::SerializeTupleStruct for AnchorSeq<'a, W>
where
    W: io::Write,
{
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, v: &T) -> Result<(), Error>
    where
        T: ?Sized + ser::Serialize,
    {
        ser::SerializeSeq::serialize_element(self, v)
    }

    fn end(self) -> Result<(), Error> {
        ser::SerializeSeq::end(self)
    }
}

impl<'a, W> ser::Serializer for AnchorName<'a, W>
where
    W: io::Write,
{
    type Ok = ();
    type Error = Error;

    type SerializeSeq = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTuple = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeMap = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeStructVariant = ser::Impossible<Self::Ok, Self::Error>;

    fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
        self.ser.pending_anchor = Some(value.to_owned());
        Ok(())
    }

    fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
        anchor_must_be_string()
    }
    fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
        anchor_must_be_string()
    }
    fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
        anchor_must_be_string()
    }
    fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> {
        anchor_must_be_string()
    }
    fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> {
        anchor_must_be_string()
    }
    fn serialize_i128(self, _v: i128) -> Result<Self::Ok, Self::Error> {
        anchor_must_be_string()
    }
    fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
        anchor_must_be_string()
    }
    fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
        anchor_must_be_string()
    }
    fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> {
        anchor_must_be_string()
    }
    fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> {
        anchor_must_be_string()
    }
    fn serialize_u128(self, _v: u128) -> Result<Self::Ok, Self::Error> {
        anchor_must_be_string()
    }
    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
        anchor_must_be_string()
    }
    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
        anchor_must_be_string()
    }
    fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
        anchor_must_be_string()
    }
    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
        anchor_must_be_string()
    }
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        anchor_must_be_string()
    }
    fn serialize_some<T>(self, _value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        anchor_must_be_string()
    }
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        anchor_must_be_string()
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        anchor_must_be_string()
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        anchor_must_be_string()
    }
    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        anchor_must_be_string()
    }
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        anchor_must_be_string()
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        anchor_must_be_string()
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        anchor_must_be_string()
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        anchor_must_be_string()
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        anchor_must_be_string()
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        anchor_must_be_string()
    }
    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        anchor_must_be_string()
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        anchor_must_be_string()
    }
}

struct AliasHelper<'a, W>
where
    W: io::Write,
{
    ser: &'a mut Serializer<W>,
}

#[inline]
fn alias_must_be_string<T>() -> Result<T> {
    Err(ser::Error::custom("alias must be a string"))
}

impl<'a, W> ser::Serializer for AliasHelper<'a, W>
where
    W: io::Write,
{
    type Ok = ();
    type Error = Error;

    type SerializeSeq = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTuple = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeMap = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeStructVariant = ser::Impossible<Self::Ok, Self::Error>;

    fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
        self.ser.emit_alias(value)
    }

    fn serialize_bool(self, _v: bool) -> Result<Self::Ok, Self::Error> {
        alias_must_be_string()
    }
    fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
        alias_must_be_string()
    }
    fn serialize_i16(self, _v: i16) -> Result<Self::Ok, Self::Error> {
        alias_must_be_string()
    }
    fn serialize_i32(self, _v: i32) -> Result<Self::Ok, Self::Error> {
        alias_must_be_string()
    }
    fn serialize_i64(self, _v: i64) -> Result<Self::Ok, Self::Error> {
        alias_must_be_string()
    }
    fn serialize_i128(self, _v: i128) -> Result<Self::Ok, Self::Error> {
        alias_must_be_string()
    }
    fn serialize_u8(self, _v: u8) -> Result<Self::Ok, Self::Error> {
        alias_must_be_string()
    }
    fn serialize_u16(self, _v: u16) -> Result<Self::Ok, Self::Error> {
        alias_must_be_string()
    }
    fn serialize_u32(self, _v: u32) -> Result<Self::Ok, Self::Error> {
        alias_must_be_string()
    }
    fn serialize_u64(self, _v: u64) -> Result<Self::Ok, Self::Error> {
        alias_must_be_string()
    }
    fn serialize_u128(self, _v: u128) -> Result<Self::Ok, Self::Error> {
        alias_must_be_string()
    }
    fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
        alias_must_be_string()
    }
    fn serialize_f64(self, _v: f64) -> Result<Self::Ok, Self::Error> {
        alias_must_be_string()
    }
    fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
        alias_must_be_string()
    }
    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
        alias_must_be_string()
    }
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        alias_must_be_string()
    }
    fn serialize_some<T>(self, _value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        alias_must_be_string()
    }
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        alias_must_be_string()
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        alias_must_be_string()
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        alias_must_be_string()
    }
    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        alias_must_be_string()
    }
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        alias_must_be_string()
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        alias_must_be_string()
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        alias_must_be_string()
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        alias_must_be_string()
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        alias_must_be_string()
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        alias_must_be_string()
    }
    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        alias_must_be_string()
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        alias_must_be_string()
    }
}

/// Serialize the given data structure as YAML into the IO stream.
///
/// Serialization can fail if `T`'s implementation of `Serialize` decides to
/// return an error.
pub fn to_writer<W, T>(writer: W, value: &T) -> Result<()>
where
    W: io::Write,
    T: ?Sized + ser::Serialize,
{
    let mut serializer = Serializer::new(writer)?;
    value.serialize(&mut serializer)
}

/// Serialize the given data structure as a String of YAML.
///
/// Serialization can fail if `T`'s implementation of `Serialize` decides to
/// return an error.
pub fn to_string<T>(value: &T) -> Result<String>
where
    T: ?Sized + ser::Serialize,
{
    let mut vec = Vec::with_capacity(128);
    to_writer(&mut vec, value)?;
    String::from_utf8(vec).map_err(|error| error::new(ErrorImpl::FromUtf8(error)))
}

/// Serialize the given array of data structures as multiple YAML documents into the IO stream.
pub fn to_writer_multi<W, T>(writer: W, values: &[T]) -> Result<()>
where
    W: io::Write,
    T: ser::Serialize,
{
    let mut serializer = Serializer::new(writer)?;
    for value in values {
        value.serialize(&mut serializer)?;
    }
    Ok(())
}

/// Serialize the given array of data structures as a YAML multi-document string.
pub fn to_string_multi<T>(values: &[T]) -> Result<String>
where
    T: ser::Serialize,
{
    let mut vec = Vec::with_capacity(128);
    to_writer_multi(&mut vec, values)?;
    String::from_utf8(vec).map_err(|error| error::new(ErrorImpl::FromUtf8(error)))
}
