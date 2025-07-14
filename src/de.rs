use crate::error::{self, Error, ErrorImpl};
use crate::libyaml::error::Mark;
use crate::libyaml::parser::{Scalar, ScalarStyle};
use crate::libyaml::tag::Tag;
use crate::loader::{Document, Loader};
use crate::path::Path;
use std::str::FromStr;
use serde::de::value::StrDeserializer;
use serde::de::{
    self, Deserialize, DeserializeOwned, DeserializeSeed, Expected, IgnoredAny, Unexpected, Visitor,
};
use std::fmt;
use std::collections::HashSet;
use std::cell::RefCell;
use std::rc::Rc;
use crate::duplicate_key::DuplicateKeyError;
use crate::value::{Value, Sequence, Mapping};
use crate::number::Number;
use std::io;
use std::mem;
use std::num::ParseIntError;
use std::str;
use std::sync::Arc;

type Result<T, E = Error> = std::result::Result<T, E>;

/// A structure that deserializes YAML into Rust values.
///
/// # Examples
///
/// Deserializing a single document:
///
/// ```
/// use anyhow::Result;
/// use serde::Deserialize;
/// use serde_yaml_bw::Value;
///
/// fn main() -> Result<()> {
///     let input = "k: 107\n";
///     let de = serde_yaml_bw::Deserializer::from_str(input);
///     let value = Value::deserialize(de)?;
///     println!("{:?}", value);
///     Ok(())
/// }
/// ```
///
/// Deserializing multi-doc YAML:
///
/// ```
/// use anyhow::Result;
/// use serde::Deserialize;
/// use serde_yaml_bw::Value;
///
/// fn main() -> Result<()> {
///     let input = "---\nk: 107\n...\n---\nj: 106\n";
///
///     for document in serde_yaml_bw::Deserializer::from_str(input) {
///         let value = Value::deserialize(document)?;
///         println!("{:?}", value);
///     }
///
///     Ok(())
/// }
/// ```
pub struct Deserializer<'de> {
    progress: Progress<'de>,
}

pub(crate) enum Progress<'de> {
    Str(&'de str),
    Slice(&'de [u8]),
    Read(Box<dyn io::Read + 'de>),
    Iterable(Loader<'de>),
    Document(Document<'de>),
    Fail(Arc<ErrorImpl>),
}

impl<'de> Default for Progress<'de> {
    fn default() -> Self {
        Progress::Str("")
    }
}

impl<'de> Deserializer<'de> {
    /// Creates a YAML deserializer from a `&str`.
    pub fn from_str(s: &'de str) -> Self {
        let progress = Progress::Str(s);
        Deserializer { progress }
    }

    /// Creates a YAML deserializer from a `&[u8]`.
    pub fn from_slice(v: &'de [u8]) -> Self {
        let progress = Progress::Slice(v);
        Deserializer { progress }
    }

    /// Creates a YAML deserializer from an `io::Read`.
    ///
    /// Reader-based deserializers do not support deserializing borrowed types
    /// like `&str`, since the `std::io::Read` trait has no non-copying methods
    /// -- everything it does involves copying bytes out of the data source.
    pub fn from_reader<R>(rdr: R) -> Self
    where
        R: io::Read + 'de,
    {
        let progress = Progress::Read(Box::new(rdr));
        Deserializer { progress }
    }

    fn de<T>(
        self,
        f: impl for<'document> FnOnce(&mut DeserializerFromEvents<'de, 'document>) -> Result<T>,
    ) -> Result<T> {
        let mut pos = 0;
        let mut jumpcount = 0;
        let enum_depth = Rc::new(RefCell::new(0usize));

        match self.progress {
            Progress::Iterable(_) => return Err(error::new(ErrorImpl::MoreThanOneDocument)),
            Progress::Document(document) => {
                let t = f(&mut DeserializerFromEvents {
                    document: &document,
                    pos: &mut pos,
                    jumpcount: &mut jumpcount,
                    path: Path::Root,
                    remaining_depth: 128,
                    enum_depth: Rc::clone(&enum_depth),
                })?;
                if let Some(parse_error) = document.error {
                    return Err(error::shared(parse_error));
                }
                return Ok(t);
            }
            _ => {}
        }

        let mut loader = Loader::new(self.progress)?;
        let Some(document) = loader.next_document() else {
            return Err(error::new(ErrorImpl::EndOfStream));
        };
        let t = f(&mut DeserializerFromEvents {
            document: &document,
            pos: &mut pos,
            jumpcount: &mut jumpcount,
            path: Path::Root,
            remaining_depth: 128,
            enum_depth: Rc::clone(&enum_depth),
        })?;
        if let Some(parse_error) = document.error {
            return Err(error::shared(parse_error));
        }
        if loader.next_document().is_none() {
            Ok(t)
        } else {
            Err(error::new(ErrorImpl::MoreThanOneDocument))
        }
    }
}

impl Iterator for Deserializer<'_> {
    type Item = Self;

    fn next(&mut self) -> Option<Self> {
        match &mut self.progress {
            Progress::Iterable(loader) => {
                let document = loader.next_document()?;
                return Some(Deserializer {
                    progress: Progress::Document(document),
                });
            }
            Progress::Document(_) => return None,
            Progress::Fail(err) => {
                return Some(Deserializer {
                    progress: Progress::Fail(Arc::clone(err)),
                });
            }
            _ => {}
        }

        let input = mem::take(&mut self.progress);
        match Loader::new(input) {
            Ok(loader) => {
                self.progress = Progress::Iterable(loader);
                self.next()
            }
            Err(err) => {
                let fail = err.shared();
                self.progress = Progress::Fail(Arc::clone(&fail));
                Some(Deserializer {
                    progress: Progress::Fail(fail),
                })
            }
        }
    }
}

impl<'de> de::Deserializer<'de> for Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_any(visitor))
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_bool(visitor))
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_i8(visitor))
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_i16(visitor))
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_i32(visitor))
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_i64(visitor))
    }

    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_i128(visitor))
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_u8(visitor))
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_u16(visitor))
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_u32(visitor))
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_u64(visitor))
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_u128(visitor))
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_f32(visitor))
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_f64(visitor))
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_char(visitor))
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_str(visitor))
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_string(visitor))
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_bytes(visitor))
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_byte_buf(visitor))
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_option(visitor))
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_unit(visitor))
    }

    fn deserialize_unit_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_unit_struct(name, visitor))
    }

    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_newtype_struct(name, visitor))
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_seq(visitor))
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_tuple(len, visitor))
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_tuple_struct(name, len, visitor))
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_map(visitor))
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_struct(name, fields, visitor))
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_enum(name, variants, visitor))
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_identifier(visitor))
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de(|state| state.deserialize_ignored_any(visitor))
    }
}

#[derive(Debug)]
pub(crate) struct ScalarEvent<'de> {
    pub anchor: Option<String>,
    pub value: Scalar<'de>,
}

#[derive(Debug)]
pub(crate) struct SequenceStartEvent {
    pub anchor: Option<String>,
    pub tag: Option<Tag>,
}

#[derive(Debug)]
pub(crate) struct MappingStartEvent {
    pub anchor: Option<String>,
    pub tag: Option<Tag>,
}

#[derive(Debug)]
pub(crate) enum Event<'de> {
    Alias(usize),
    Scalar(ScalarEvent<'de>),
    SequenceStart(SequenceStartEvent),
    SequenceEnd,
    MappingStart(MappingStartEvent),
    MappingEnd,
    Void,
}

struct DeserializerFromEvents<'de, 'document> {
    document: &'document Document<'de>,
    pos: &'document mut usize,
    jumpcount: &'document mut usize,
    path: Path<'document>,
    remaining_depth: u8,
    enum_depth: Rc<RefCell<usize>>,
}

impl<'de, 'document> DeserializerFromEvents<'de, 'document> {
    fn peek_event(&self) -> Result<&'document Event<'de>> {
        self.peek_event_mark().map(|(event, _mark)| event)
    }

    fn peek_event_mark(&self) -> Result<(&'document Event<'de>, Mark)> {
        match self.document.events.get(*self.pos) {
            Some((event, mark)) => Ok((event, *mark)),
            None => Err(match &self.document.error {
                Some(parse_error) => error::shared(Arc::clone(parse_error)),
                None => error::new(ErrorImpl::EndOfStream),
            }),
        }
    }

    fn next_event(&mut self) -> Result<&'document Event<'de>> {
        self.next_event_mark().map(|(event, _mark)| event)
    }

    fn next_event_mark(&mut self) -> Result<(&'document Event<'de>, Mark)> {
        self.peek_event_mark().map(|(event, mark)| {
            *self.pos += 1;
            match event {
                Event::Scalar(_) | Event::SequenceEnd | Event::MappingEnd => {
                    let mut depth = self.enum_depth.borrow_mut();
                    if *depth > 0 {
                        *depth -= 1;
                    }
                }
                _ => {}
            }
            (event, mark)
        })
    }

    fn jump<'anchor>(
        &'anchor mut self,
        pos: &'anchor mut usize,
    ) -> Result<DeserializerFromEvents<'de, 'anchor>> {
        *self.jumpcount += 1;
        let limit = self
            .document
            .events
            .len()
            .checked_mul(100)
            .ok_or_else(|| error::new(ErrorImpl::RepetitionLimitExceeded))?;
        if *self.jumpcount > limit {
            return Err(error::new(ErrorImpl::RepetitionLimitExceeded));
        }
        match self.document.aliases.get(*pos) {
            Some(found) => {
                *pos = *found;
                Ok(DeserializerFromEvents {
                    document: self.document,
                    pos,
                    jumpcount: self.jumpcount,
                    path: Path::Alias { parent: &self.path },
                    remaining_depth: self.remaining_depth,
                    enum_depth: Rc::clone(&self.enum_depth),
                })
            }
            None => {
                Err(error::fix_mark(
                    error::new(ErrorImpl::UnresolvedAlias),
                    self.peek_event_mark()?.1,
                    self.path,
                ))
            }
        }
    }

    fn ignore_any(&mut self) -> Result<()> {
        enum Nest {
            Sequence,
            Mapping,
        }

        let mut stack = Vec::new();

        loop {
            match self.next_event()? {
                Event::Alias(_) | Event::Scalar(_) | Event::Void => {}
                Event::SequenceStart(_) => {
                    stack.push(Nest::Sequence);
                }
                Event::MappingStart(_) => {
                    stack.push(Nest::Mapping);
                }
                Event::SequenceEnd => match stack.pop() {
                    Some(Nest::Sequence) => {}
                    None | Some(Nest::Mapping) => {
                        {
                            return Err(error::fix_mark(
                                error::new(ErrorImpl::UnexpectedEndOfSequence),
                                self.peek_event_mark()?.1,
                                self.path,
                            ));
                        }
                    }
                },
                Event::MappingEnd => match stack.pop() {
                    Some(Nest::Mapping) => {}
                    None | Some(Nest::Sequence) => {
                        return Err(error::fix_mark(
                            error::new(ErrorImpl::UnexpectedEndOfMapping),
                            self.peek_event_mark()?.1,
                            self.path,
                        ));
                    }
                },
            }
            if stack.is_empty() {
                return Ok(());
            }
        }
    }

    fn visit_sequence<V>(&mut self, visitor: V, mark: Mark) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let (value, len) = self.recursion_check(mark, |de| {
            let mut seq = SeqAccess {
                empty: false,
                de,
                len: 0,
            };
            let value = visitor.visit_seq(&mut seq)?;
            Ok((value, seq.len))
        })?;
        self.end_sequence(len)?;
        Ok(value)
    }

    fn visit_mapping<V>(&mut self, visitor: V, mark: Mark) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let (value, len) = self.recursion_check(mark, |de| {
            let mut map = MapAccess {
                empty: false,
                de,
                len: 0,
                key: None,
                seen: HashSet::new(),
            };
            let value = visitor.visit_map(&mut map)?;
            Ok((value, map.len))
        })?;
        self.end_mapping(len)?;
        Ok(value)
    }

    fn end_sequence(&mut self, len: usize) -> Result<()> {
        let total = {
            let mut seq = SeqAccess {
                empty: false,
                de: self,
                len,
            };
            while de::SeqAccess::next_element::<IgnoredAny>(&mut seq)?.is_some() {}
            seq.len
        };
        match self.next_event()? {
            Event::SequenceEnd | Event::Void => {}
            _ => {
                return Err(error::fix_mark(
                    error::new(ErrorImpl::UnexpectedEndOfSequence),
                    self.peek_event_mark()?.1,
                    self.path,
                ));
            }
        }
        if total == len {
            Ok(())
        } else {
            struct ExpectedSeq(usize);
            impl Expected for ExpectedSeq {
                fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    if self.0 == 1 {
                        write!(formatter, "sequence of 1 element")
                    } else {
                        write!(formatter, "sequence of {} elements", self.0)
                    }
                }
            }
            Err(de::Error::invalid_length(total, &ExpectedSeq(len)))
        }
    }

    fn end_mapping(&mut self, len: usize) -> Result<()> {
        let total = {
            let mut map = MapAccess {
                empty: false,
                de: self,
                len,
                key: None,
                seen: HashSet::new(),
            };
            while de::MapAccess::next_entry::<IgnoredAny, IgnoredAny>(&mut map)?.is_some() {}
            map.len
        };
        match self.next_event()? {
            Event::MappingEnd | Event::Void => {}
            _ => {
                return Err(crate::error::fix_mark(
                    crate::error::new(ErrorImpl::UnexpectedEndOfMapping),
                    self.peek_event_mark()?.1,
                    self.path,
                ));
            }
        }
        if total == len {
            Ok(())
        } else {
            struct ExpectedMap(usize);
            impl Expected for ExpectedMap {
                fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    if self.0 == 1 {
                        write!(formatter, "map containing 1 entry")
                    } else {
                        write!(formatter, "map containing {} entries", self.0)
                    }
                }
            }
            Err(de::Error::invalid_length(total, &ExpectedMap(len)))
        }
    }

    fn recursion_check<F: FnOnce(&mut Self) -> Result<T>, T>(
        &mut self,
        mark: Mark,
        f: F,
    ) -> Result<T> {
        let previous_depth = self.remaining_depth;
        self.remaining_depth = match previous_depth.checked_sub(1) {
            Some(depth) => depth,
            None => return Err(error::new(ErrorImpl::RecursionLimitExceeded(mark))),
        };
        let result = f(self);
        self.remaining_depth = previous_depth;
        result
    }

    pub(crate) fn parse_value(&mut self) -> Result<Value> {
        use Event::{Alias, MappingEnd, MappingStart, Scalar, SequenceEnd, SequenceStart, Void};
        let (event, _mark) = self.next_event_mark()?;
        match event {
            &Alias(id) => {
                let alias_index = match self.document.aliases.get(id) {
                    Some(idx) => *idx,
                    None => return Err(error::new(ErrorImpl::UnresolvedAlias)),
                };
                let name = match &self.document.events[alias_index].0 {
                    Scalar(s) => s.anchor.clone(),
                    SequenceStart(s) => s.anchor.clone(),
                    MappingStart(m) => m.anchor.clone(),
                    _ => None,
                }
                .ok_or_else(|| error::new(ErrorImpl::UnresolvedAlias))?;
                Ok(Value::Alias(name))
            }
            Scalar(scalar) => Ok(parse_scalar_value(scalar)),
            SequenceStart(seq) => {
                let anchor = seq.anchor.clone();
                let mut elements = Vec::new();
                while !matches!(self.peek_event()?, SequenceEnd) {
                    elements.push(self.parse_value()?);
                }
                self.next_event()?; // consume SequenceEnd
                Ok(Value::Sequence(Sequence { anchor, elements }))
            }
            MappingStart(map) => {
                let anchor = map.anchor.clone();
                let mut mapping = Mapping::new();
                while !matches!(self.peek_event()?, MappingEnd) {
                    let key = self.parse_value()?;
                    if mapping.contains_key(&key) {
                        return Err(error::new(ErrorImpl::Message(
                            DuplicateKeyError::from_value(&key).to_string(),
                            None,
                        )));
                    }
                    let value = self.parse_value()?;
                    mapping.insert(key, value);
                }
                self.next_event()?; // consume MappingEnd
                mapping.anchor = anchor;
                Ok(Value::Mapping(mapping))
            }
            SequenceEnd => Err(error::new(ErrorImpl::UnexpectedEndOfSequence)),
            MappingEnd => Err(error::new(ErrorImpl::UnexpectedEndOfMapping)),
            Void => Ok(Value::Null(None)),
        }
    }
}

struct SeqAccess<'de, 'document, 'seq> {
    empty: bool,
    de: &'seq mut DeserializerFromEvents<'de, 'document>,
    len: usize,
}

impl<'de> de::SeqAccess<'de> for SeqAccess<'de, '_, '_> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        if self.empty {
            return Ok(None);
        }
        match self.de.peek_event()? {
            Event::SequenceEnd | Event::Void => Ok(None),
            _ => {
                let mut element_de = DeserializerFromEvents {
                    document: self.de.document,
                    pos: self.de.pos,
                    jumpcount: self.de.jumpcount,
                    path: Path::Seq {
                        parent: &self.de.path,
                        index: self.len,
                    },
                    remaining_depth: self.de.remaining_depth,
                    enum_depth: Rc::clone(&self.de.enum_depth),
                };
                self.len += 1;
                seed.deserialize(&mut element_de).map(Some)
            }
        }
    }
}

struct MapAccess<'de, 'document, 'map> {
    empty: bool,
    de: &'map mut DeserializerFromEvents<'de, 'document>,
    len: usize,
    key: Option<&'document [u8]>,
    seen: std::collections::HashSet<Vec<u8>>,
}

impl<'de> de::MapAccess<'de> for MapAccess<'de, '_, '_> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        if self.empty {
            return Ok(None);
        }
        match self.de.peek_event()? {
            Event::MappingEnd | Event::Void => Ok(None),
            Event::Scalar(scalar) => {
                self.len += 1;
                if !self.seen.insert(scalar.value.value.to_vec()) {
                    return Err(de::Error::custom(DuplicateKeyError::from_scalar(&scalar.value.value)));
                }
                self.key = Some(&scalar.value.value);
                seed.deserialize(&mut *self.de).map(Some)
            }
            _ => {
                self.len += 1;
                self.key = None;
                seed.deserialize(&mut *self.de).map(Some)
            }
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        let mut value_de = DeserializerFromEvents {
            document: self.de.document,
            pos: self.de.pos,
            jumpcount: self.de.jumpcount,
            path: if let Some(key) = self.key.and_then(|key| str::from_utf8(key).ok()) {
                Path::Map {
                    parent: &self.de.path,
                    key,
                }
            } else {
                Path::Unknown {
                    parent: &self.de.path,
                }
            },
            remaining_depth: self.de.remaining_depth,
            enum_depth: Rc::clone(&self.de.enum_depth),
        };
        seed.deserialize(&mut value_de)
    }
}

struct EnumAccess<'de, 'document, 'variant> {
    de: &'variant mut DeserializerFromEvents<'de, 'document>,
    tag: &'document str,
}

impl<'de, 'variant> de::EnumAccess<'de> for EnumAccess<'de, '_, 'variant> {
    type Error = Error;
    type Variant = DeserializerFromEvents<'de, 'variant>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        let str_de = StrDeserializer::<Error>::new(self.tag);
        let variant = seed.deserialize(str_de)?;
        *self.de.enum_depth.borrow_mut() += 1;
        let visitor = DeserializerFromEvents {
            document: self.de.document,
            pos: self.de.pos,
            jumpcount: self.de.jumpcount,
            path: self.de.path,
            remaining_depth: self.de.remaining_depth,
            enum_depth: Rc::clone(&self.de.enum_depth),
        };
        Ok((variant, visitor))
    }
}

impl<'de> de::VariantAccess<'de> for DeserializerFromEvents<'de, '_> {
    type Error = Error;

    fn unit_variant(mut self) -> Result<()> {
        Deserialize::deserialize(&mut self)
    }

    fn newtype_variant_seed<T>(mut self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut self)
    }

    fn tuple_variant<V>(mut self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(&mut self, visitor)
    }

    fn struct_variant<V>(mut self, fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_struct(&mut self, "", fields, visitor)
    }
}

struct UnitVariantAccess<'de, 'document, 'variant> {
    de: &'variant mut DeserializerFromEvents<'de, 'document>,
}

impl<'de> de::EnumAccess<'de> for UnitVariantAccess<'de, '_, '_> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        Ok((seed.deserialize(&mut *self.de)?, self))
    }
}

impl<'de> de::VariantAccess<'de>
    for UnitVariantAccess<'de, '_, '_>
{
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, _seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        Err(de::Error::invalid_type(
            Unexpected::UnitVariant,
            &"newtype variant",
        ))
    }

    fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(de::Error::invalid_type(
            Unexpected::UnitVariant,
            &"tuple variant",
        ))
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(de::Error::invalid_type(
            Unexpected::UnitVariant,
            &"struct variant",
        ))
    }
}

fn visit_scalar<'de, V>(visitor: V, scalar: &Scalar<'de>, tagged_already: bool) -> Result<V::Value>
where
    V: Visitor<'de>,
{
    let Ok(v) = str::from_utf8(&scalar.value) else {
        return Err(de::Error::invalid_type(
            Unexpected::Bytes(&scalar.value),
            &visitor,
        ));
    };
    if let (Some(tag), false) = (&scalar.tag, tagged_already) {
        if tag == Tag::BOOL {
            return match parse_bool(v) {
                Some(v) => visitor.visit_bool(v),
                None => Err(de::Error::invalid_value(Unexpected::Str(v), &"a boolean")),
            };
        } else if tag == Tag::INT {
            return match visit_int(visitor, v) {
                Ok(result) => result,
                Err(_) => Err(de::Error::invalid_value(Unexpected::Str(v), &"an integer")),
            };
        } else if tag == Tag::FLOAT {
            return match parse_f64(v) {
                Some(v) => visitor.visit_f64(v),
                None => Err(de::Error::invalid_value(Unexpected::Str(v), &"a float")),
            };
        } else if tag == Tag::BINARY {
            match decode_base64(v) {
                Some(bytes) => return visitor.visit_byte_buf(bytes),
                None => return Err(de::Error::invalid_value(Unexpected::Str(v), &"base64")),
            }
        } else if tag == Tag::NULL {
            return match parse_null(v.as_bytes()) {
                Some(()) => visitor.visit_unit(),
                None => Err(de::Error::invalid_value(Unexpected::Str(v), &"null")),
            };
        } else if tag.starts_with("!") && scalar.style == ScalarStyle::Plain {
            return visit_untagged_scalar(visitor, v, scalar.repr, scalar.style);
        }
    } else if scalar.style == ScalarStyle::Plain {
        return visit_untagged_scalar(visitor, v, scalar.repr, scalar.style);
    }
    if let Some(borrowed) = parse_borrowed_str(v, scalar.repr, scalar.style) {
        visitor.visit_borrowed_str(borrowed)
    } else {
        visitor.visit_str(v)
    }
}

fn parse_borrowed_str<'de>(
    utf8_value: &str,
    repr: Option<&'de [u8]>,
    style: ScalarStyle,
) -> Option<&'de str> {
    let borrowed_repr = repr?;
    let expected_offset = match style {
        ScalarStyle::Plain => 0,
        ScalarStyle::SingleQuoted | ScalarStyle::DoubleQuoted => 1,
        ScalarStyle::Literal | ScalarStyle::Folded => return None,
    };
    let expected_end = borrowed_repr.len().checked_sub(expected_offset)?;
    let expected_start = expected_end.checked_sub(utf8_value.len())?;
    let borrowed_bytes = borrowed_repr.get(expected_start..expected_end)?;
    if borrowed_bytes == utf8_value.as_bytes() {
        return str::from_utf8(borrowed_bytes).ok()
    }
    None
}

fn parse_null(scalar: &[u8]) -> Option<()> {
    match scalar {
        b"null" | b"Null" | b"NULL" | b"~" => Some(()),
        _ => None,
    }
}

fn parse_bool(scalar: &str) -> Option<bool> {
    match scalar {
        "true" | "True" | "TRUE" => Some(true),
        "false" | "False" | "FALSE" => Some(false),
        _ => None,
    }
}

fn decode_base64(input: &str) -> Option<Vec<u8>> {
    let mut cleaned = Vec::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'+' | b'/' | b'=' => cleaned.push(b),
            b' ' | b'\n' | b'\r' | b'\t' => {}
            _ => return None,
        }
    }
    if cleaned.len() % 4 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(cleaned.len() / 4 * 3);
    let mut i = 0;
    while i < cleaned.len() {
        let c0 = cleaned[i];
        let c1 = cleaned[i + 1];
        let c2 = cleaned[i + 2];
        let c3 = cleaned[i + 3];
        let v0 = decode_b64_char(c0)?;
        let v1 = decode_b64_char(c1)?;
        if c2 == b'=' {
            if c3 != b'=' || i + 4 != cleaned.len() {
                return None;
            }
            out.push((v0 << 2) | (v1 >> 4));
            break;
        }
        let v2 = decode_b64_char(c2)?;
        if c3 == b'=' {
            if i + 4 != cleaned.len() {
                return None;
            }
            out.push((v0 << 2) | (v1 >> 4));
            out.push(((v1 & 0xf) << 4) | (v2 >> 2));
            break;
        }
        let v3 = decode_b64_char(c3)?;
        out.push((v0 << 2) | (v1 >> 4));
        out.push(((v1 & 0xf) << 4) | (v2 >> 2));
        out.push(((v2 & 0x3) << 6) | v3);
        i += 4;
    }
    Some(out)
}

fn decode_b64_char(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

fn parse_scalar_value(scalar: &ScalarEvent) -> Value {
    let anchor = scalar.anchor.clone();
    let Ok(repr) = std::str::from_utf8(&scalar.value.value) else {
        return Value::String(String::from_utf8_lossy(&scalar.value.value).into_owned(), anchor);
    };
    if scalar.value.style == ScalarStyle::Plain {
        if parse_null(&scalar.value.value).is_some() {
            return Value::Null(anchor);
        }
        if let Some(b) = parse_bool(repr) {
            return Value::Bool(b, anchor);
        }
        if let Ok(num) = Number::from_str(repr) {
            return Value::Number(num, anchor);
        }
    }
    Value::String(repr.to_owned(), anchor)
}

fn make_negative(rest: &str) -> String {
    let mut negative = String::with_capacity(rest.len() + 1);
    negative.push('-');
    negative.push_str(rest);
    negative
}

fn parse_unsigned_int<T>(
    scalar: &str,
    from_str_radix: fn(&str, radix: u32) -> Result<T, ParseIntError>,
) -> Option<T> {
    let unpositive = scalar.strip_prefix('+').unwrap_or(scalar);
    if unpositive.contains(['+', '-']) {
        return None;
    }
    if let Some(rest) = unpositive.strip_prefix("0x") {
        if rest.is_empty() || rest.starts_with(['+', '-']) {
            return None;
        }
        if let Ok(int) = from_str_radix(rest, 16) {
            return Some(int);
        }
    }
    if let Some(rest) = unpositive.strip_prefix("0o") {
        if rest.is_empty() || rest.starts_with(['+', '-']) {
            return None;
        }
        if let Ok(int) = from_str_radix(rest, 8) {
            return Some(int);
        }
    }
    if let Some(rest) = unpositive.strip_prefix("0b") {
        if rest.is_empty() || rest.starts_with(['+', '-']) {
            return None;
        }
        if let Ok(int) = from_str_radix(rest, 2) {
            return Some(int);
        }
    }
    if unpositive.starts_with(['+', '-']) {
        return None;
    }
    if digits_but_not_number(scalar) {
        return None;
    }
    from_str_radix(unpositive, 10).ok()
}

fn parse_signed_int<T>(
    scalar: &str,
    from_str_radix: fn(&str, radix: u32) -> Result<T, ParseIntError>,
) -> Option<T> {
    let unpositive = if let Some(unpositive) = scalar.strip_prefix('+') {
        if unpositive.starts_with(['+', '-']) {
            return None;
        }
        unpositive
    } else {
        scalar
    };
    if let Some(rest) = unpositive.strip_prefix("0x") {
        if rest.starts_with(['+', '-']) {
            return None;
        }
        if let Ok(int) = from_str_radix(rest, 16) {
            return Some(int);
        }
    }
    if let Some(rest) = scalar.strip_prefix("-0x") {
        let negative = make_negative(rest);
        if let Ok(int) = from_str_radix(&negative, 16) {
            return Some(int);
        }
    }
    if let Some(rest) = unpositive.strip_prefix("0o") {
        if rest.starts_with(['+', '-']) {
            return None;
        }
        if let Ok(int) = from_str_radix(rest, 8) {
            return Some(int);
        }
    }
    if let Some(rest) = scalar.strip_prefix("-0o") {
        let negative = make_negative(rest);
        if let Ok(int) = from_str_radix(&negative, 8) {
            return Some(int);
        }
    }
    if let Some(rest) = unpositive.strip_prefix("0b") {
        if rest.starts_with(['+', '-']) {
            return None;
        }
        if let Ok(int) = from_str_radix(rest, 2) {
            return Some(int);
        }
    }
    if let Some(rest) = scalar.strip_prefix("-0b") {
        let negative = make_negative(rest);
        if let Ok(int) = from_str_radix(&negative, 2) {
            return Some(int);
        }
    }
    if digits_but_not_number(scalar) {
        return None;
    }
    from_str_radix(unpositive, 10).ok()
}

fn parse_negative_int<T>(
    scalar: &str,
    from_str_radix: fn(&str, radix: u32) -> Result<T, ParseIntError>,
) -> Option<T> {
    if !scalar.starts_with('-') {
        return None;
    }
    let unsigned = &scalar[1..];
    if unsigned.starts_with(['+', '-']) || unsigned.contains('+') || unsigned.contains('-') {
        return None;
    }
    if let Some(rest) = scalar.strip_prefix("-0x") {
        if rest.is_empty() || rest.starts_with(['+', '-']) {
            return None;
        }
        let negative = make_negative(rest);
        if let Ok(int) = from_str_radix(&negative, 16) {
            return Some(int);
        }
    }
    if let Some(rest) = scalar.strip_prefix("-0o") {
        if rest.is_empty() || rest.starts_with(['+', '-']) {
            return None;
        }
        let negative = make_negative(rest);
        if let Ok(int) = from_str_radix(&negative, 8) {
            return Some(int);
        }
    }
    if let Some(rest) = scalar.strip_prefix("-0b") {
        if rest.is_empty() || rest.starts_with(['+', '-']) {
            return None;
        }
        let negative = make_negative(rest);
        if let Ok(int) = from_str_radix(&negative, 2) {
            return Some(int);
        }
    }
    if digits_but_not_number(scalar) {
        return None;
    }
    from_str_radix(scalar, 10).ok()
}

/// Parse a scalar as a floating point value.
///
/// Returns `Some(f64)` if the string is a valid finite float or special value
/// like `.inf`. Otherwise returns `None`.
pub fn parse_f64(scalar: &str) -> Option<f64> {
    let unpositive = if let Some(unpositive) = scalar.strip_prefix('+') {
        if unpositive.starts_with(['+', '-']) {
            return None;
        }
        unpositive
    } else {
        scalar
    };
    if let ".inf" | ".Inf" | ".INF" = unpositive {
        return Some(f64::INFINITY);
    }
    if let "-.inf" | "-.Inf" | "-.INF" = scalar {
        return Some(f64::NEG_INFINITY);
    }
    if let ".nan" | ".NaN" | ".NAN" = scalar {
        return Some(f64::NAN.copysign(1.0));
    }
    if let Ok(float) = unpositive.parse::<f64>() {
        if float.is_finite() {
            return Some(float);
        }
    }
    None
}

/// Check if a digit string should be treated as a YAML string rather than a number.
///
/// Leading zeros like `"00"` mean the scalar is not parsed as a numeric value.
pub fn digits_but_not_number(scalar: &str) -> bool {
    // Leading zero(s) followed by numeric characters is a string according to
    // the YAML 1.2 spec. https://yaml.org/spec/1.2/spec.html#id2761292
    let scalar = scalar.strip_prefix(['-', '+']).unwrap_or(scalar);
    scalar.len() > 1 && scalar.starts_with('0') && scalar[1..].bytes().all(|b| b.is_ascii_digit())
}

pub(crate) fn visit_int<'de, V>(visitor: V, v: &str) -> Result<Result<V::Value>, V>
where
    V: Visitor<'de>,
{
    if let Some(int) = parse_unsigned_int(v, u64::from_str_radix) {
        return Ok(visitor.visit_u64(int));
    }
    if let Some(int) = parse_negative_int(v, i64::from_str_radix) {
        return Ok(visitor.visit_i64(int));
    }
    if let Some(int) = parse_unsigned_int(v, u128::from_str_radix) {
        return Ok(visitor.visit_u128(int));
    }
    if let Some(int) = parse_negative_int(v, i128::from_str_radix) {
        return Ok(visitor.visit_i128(int));
    }
    Err(visitor)
}

pub(crate) fn visit_untagged_scalar<'de, V>(
    visitor: V,
    v: &str,
    repr: Option<&'de [u8]>,
    style: ScalarStyle,
) -> Result<V::Value>
where
    V: Visitor<'de>,
{
    if v.is_empty() || parse_null(v.as_bytes()) == Some(()) {
        return visitor.visit_unit();
    }
    if let Some(boolean) = parse_bool(v) {
        return visitor.visit_bool(boolean);
    }
    let visitor = match visit_int(visitor, v) {
        Ok(result) => return result,
        Err(visitor) => visitor,
    };
    if !digits_but_not_number(v) {
        if let Some(float) = parse_f64(v) {
            return visitor.visit_f64(float);
        }
    }
    if let Some(borrowed) = parse_borrowed_str(v, repr, style) {
        visitor.visit_borrowed_str(borrowed)
    } else {
        visitor.visit_str(v)
    }
}

fn is_plain_or_tagged_literal_scalar(
    expected: &str,
    scalar: &Scalar,
    tagged_already: bool,
) -> bool {
    match (scalar.style, &scalar.tag, tagged_already) {
        (ScalarStyle::Plain, _, _) => true,
        (ScalarStyle::Literal | ScalarStyle::Folded, Some(tag), false) => tag == expected,
        _ => false,
    }
}

fn invalid_type(event: &Event, exp: &dyn Expected) -> Error {
    enum Void {}

    struct InvalidType<'a> {
        exp: &'a dyn Expected,
    }

    impl Visitor<'_> for InvalidType<'_> {
        type Value = Void;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            self.exp.fmt(formatter)
        }
    }

    match event {
        Event::Alias(_) => error::new(ErrorImpl::UnresolvedAlias),
        Event::Scalar(scalar) => {
            let get_type = InvalidType { exp };
            match visit_scalar(get_type, &scalar.value, false) {
                Ok(void) => match void {},
                Err(invalid_type) => invalid_type,
            }
        }
        Event::SequenceStart(_) => de::Error::invalid_type(Unexpected::Seq, exp),
        Event::MappingStart(_) => de::Error::invalid_type(Unexpected::Map, exp),
        Event::SequenceEnd => error::new(ErrorImpl::UnexpectedEndOfSequence),
        Event::MappingEnd => error::new(ErrorImpl::UnexpectedEndOfMapping),
        Event::Void => error::new(ErrorImpl::EndOfStream),
    }
}

fn parse_tag(libyaml_tag: Option<&Tag>) -> Option<&str> {
    let mut bytes: &[u8] = libyaml_tag?;
    if let (b'!', rest) = bytes.split_first()? {
        if !rest.is_empty() {
            bytes = rest;
        }
        str::from_utf8(bytes).ok()
    } else {
        None
    }
}

impl<'de> de::Deserializer<'de> for &mut DeserializerFromEvents<'de, '_> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let tagged_already = *self.enum_depth.borrow() > 0;
        let (next, mark) = self.next_event_mark()?;

        fn enum_tag(tag: Option<&Tag>, tagged_already: bool) -> Option<&str> {
            if tagged_already {
                return None;
            }
            parse_tag(tag)
        }

        loop {
            match next {
                &Event::Alias(mut pos) => break self.jump(&mut pos)?.deserialize_any(visitor),
                Event::Scalar(scalar) => {
                    if let Some(tag) = enum_tag(scalar.value.tag.as_ref(), tagged_already) {
                        *self.pos -= 1;
                        break visitor.visit_enum(EnumAccess {
                            de: self,
                            tag,
                        });
                    }
                    break visit_scalar(visitor, &scalar.value, tagged_already);
                }
                Event::SequenceStart(sequence) => {
                    if let Some(tag) = enum_tag(sequence.tag.as_ref(), tagged_already) {
                        *self.pos -= 1;
                        break visitor.visit_enum(EnumAccess {
                            de: self,
                            tag,
                        });
                    }
                    break self.visit_sequence(visitor, mark);
                }
                Event::MappingStart(mapping) => {
                    if let Some(tag) = enum_tag(mapping.tag.as_ref(), tagged_already) {
                        *self.pos -= 1;
                        break visitor.visit_enum(EnumAccess {
                            de: self,
                            tag,
                        });
                    }
                    break self.visit_mapping(visitor, mark);
                }
                Event::SequenceEnd => {
                    break Err(error::fix_mark(
                        error::new(ErrorImpl::UnexpectedEndOfSequence),
                        mark,
                        self.path,
                    ));
                }
                Event::MappingEnd => {
                    break Err(error::fix_mark(
                        error::new(ErrorImpl::UnexpectedEndOfMapping),
                        mark,
                        self.path,
                    ));
                }
                Event::Void => break visitor.visit_none(),
            }
        }
            .map_err(|err| error::fix_mark(err, mark, self.path))
    }


    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let tagged_already = *self.enum_depth.borrow() > 0;
        let (next, mark) = self.next_event_mark()?;
        loop {
            match next {
                &Event::Alias(mut pos) => break self.jump(&mut pos)?.deserialize_bool(visitor),
                Event::Scalar(scalar)
                    if is_plain_or_tagged_literal_scalar(Tag::BOOL, &scalar.value, tagged_already) =>
                {
                    if let Ok(value) = str::from_utf8(&scalar.value.value) {
                        if let Some(boolean) = parse_bool(value) {
                            break visitor.visit_bool(boolean);
                        }
                    }
                }
                _ => {}
            }
            break Err(invalid_type(next, &visitor));
        }
        .map_err(|err| error::fix_mark(err, mark, self.path))
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let tagged_already = *self.enum_depth.borrow() > 0;
        let (next, mark) = self.next_event_mark()?;
        loop {
            match next {
                &Event::Alias(mut pos) => break self.jump(&mut pos)?.deserialize_i64(visitor),
                Event::Scalar(scalar)
                    if is_plain_or_tagged_literal_scalar(Tag::INT, &scalar.value, tagged_already) =>
                {
                    if let Ok(value) = str::from_utf8(&scalar.value.value) {
                        if let Some(int) = parse_signed_int(value, i64::from_str_radix) {
                            break visitor.visit_i64(int);
                        }
                    }
                }
                _ => {}
            }
            break Err(invalid_type(next, &visitor));
        }
        .map_err(|err| error::fix_mark(err, mark, self.path))
    }

    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let tagged_already = *self.enum_depth.borrow() > 0;
        let (next, mark) = self.next_event_mark()?;
        loop {
            match next {
                &Event::Alias(mut pos) => break self.jump(&mut pos)?.deserialize_i128(visitor),
                Event::Scalar(scalar)
                    if is_plain_or_tagged_literal_scalar(Tag::INT, &scalar.value, tagged_already) =>
                {
                    if let Ok(value) = str::from_utf8(&scalar.value.value) {
                        if let Some(int) = parse_signed_int(value, i128::from_str_radix) {
                            break visitor.visit_i128(int);
                        }
                    }
                }
                _ => {}
            }
            break Err(invalid_type(next, &visitor));
        }
        .map_err(|err| error::fix_mark(err, mark, self.path))
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let tagged_already = *self.enum_depth.borrow() > 0;
        let (next, mark) = self.next_event_mark()?;
        loop {
            match next {
                &Event::Alias(mut pos) => break self.jump(&mut pos)?.deserialize_u64(visitor),
                Event::Scalar(scalar)
                    if is_plain_or_tagged_literal_scalar(Tag::INT, &scalar.value, tagged_already) =>
                {
                    if let Ok(value) = str::from_utf8(&scalar.value.value) {
                        if let Some(int) = parse_unsigned_int(value, u64::from_str_radix) {
                            break visitor.visit_u64(int);
                        }
                    }
                }
                _ => {}
            }
            break Err(invalid_type(next, &visitor));
        }
        .map_err(|err| error::fix_mark(err, mark, self.path))
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let tagged_already = *self.enum_depth.borrow() > 0;
        let (next, mark) = self.next_event_mark()?;
        loop {
            match next {
                &Event::Alias(mut pos) => break self.jump(&mut pos)?.deserialize_u128(visitor),
                Event::Scalar(scalar)
                    if is_plain_or_tagged_literal_scalar(Tag::INT, &scalar.value, tagged_already) =>
                {
                    if let Ok(value) = str::from_utf8(&scalar.value.value) {
                        if let Some(int) = parse_unsigned_int(value, u128::from_str_radix) {
                            break visitor.visit_u128(int);
                        }
                    }
                }
                _ => {}
            }
            break Err(invalid_type(next, &visitor));
        }
        .map_err(|err| error::fix_mark(err, mark, self.path))
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_f64(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let tagged_already = *self.enum_depth.borrow() > 0;
        let (next, mark) = self.next_event_mark()?;
        loop {
            match next {
                &Event::Alias(mut pos) => break self.jump(&mut pos)?.deserialize_f64(visitor),
                Event::Scalar(scalar)
                    if is_plain_or_tagged_literal_scalar(Tag::FLOAT, &scalar.value, tagged_already) =>
                {
                    if let Ok(value) = str::from_utf8(&scalar.value.value) {
                        if let Some(float) = parse_f64(value) {
                            break visitor.visit_f64(float);
                        }
                    }
                }
                _ => {}
            }
            break Err(invalid_type(next, &visitor));
        }
        .map_err(|err| error::fix_mark(err, mark, self.path))
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let (next, mark) = self.next_event_mark()?;
        match next {
            Event::Scalar(scalar) => {
                if let Ok(v) = str::from_utf8(&scalar.value.value) {
                    if let Some(borrowed) = parse_borrowed_str(v, scalar.value.repr, scalar.value.style) {
                        visitor.visit_borrowed_str(borrowed)
                    } else {
                        visitor.visit_str(v)
                    }
                } else {
                    Err(invalid_type(next, &visitor))
                }
            }
            &Event::Alias(mut pos) => self.jump(&mut pos)?.deserialize_str(visitor),
            other => Err(invalid_type(other, &visitor)),
        }
        .map_err(|err: Error| error::fix_mark(err, mark, self.path))
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(error::new(ErrorImpl::BytesUnsupported))
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(error::new(ErrorImpl::BytesUnsupported))
    }

    /// Parses `null` as None and any other values as `Some(...)`.
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let is_some = match self.peek_event()? {
            &Event::Alias(mut pos) => {
                *self.pos += 1;
                return self.jump(&mut pos)?.deserialize_option(visitor);
            }
            Event::Scalar(scalar) => {
                let tagged_already = *self.enum_depth.borrow() > 0;
                if scalar.value.style != ScalarStyle::Plain {
                    true
                } else if let (Some(tag), false) = (&scalar.value.tag, tagged_already) {
                    if tag == Tag::NULL {
                        if let Some(()) = parse_null(&scalar.value.value) {
                            false
                        } else if let Ok(v) = str::from_utf8(&scalar.value.value) {
                            return Err(de::Error::invalid_value(Unexpected::Str(v), &"null"));
                        } else {
                            return Err(de::Error::invalid_value(
                                Unexpected::Bytes(&scalar.value.value),
                                &"null",
                            ));
                        }
                    } else {
                        true
                    }
                } else {
                    !scalar.value.value.is_empty() && parse_null(&scalar.value.value).is_none()
                }
            }
            Event::SequenceStart(_) | Event::MappingStart(_) => true,
            Event::SequenceEnd => return Err(error::new(ErrorImpl::UnexpectedEndOfSequence)),
            Event::MappingEnd => return Err(error::new(ErrorImpl::UnexpectedEndOfMapping)),
            Event::Void => false,
        };
        if is_some {
            visitor.visit_some(self)
        } else {
            *self.pos += 1;
            let mut depth = self.enum_depth.borrow_mut();
            if *depth > 0 {
                *depth -= 1;
            }
            visitor.visit_none()
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let tagged_already = *self.enum_depth.borrow() > 0;
        let (next, mark) = self.next_event_mark()?;
        match next {
            Event::Scalar(scalar) => {
                let is_null = if scalar.value.style != ScalarStyle::Plain {
                    false
                } else if let (Some(tag), false) = (&scalar.value.tag, tagged_already) {
                    tag == Tag::NULL && parse_null(&scalar.value.value).is_some()
                } else {
                    scalar.value.value.is_empty() || parse_null(&scalar.value.value).is_some()
                };
                if is_null {
                    visitor.visit_unit()
                } else if let Ok(v) = str::from_utf8(&scalar.value.value) {
                    Err(de::Error::invalid_value(Unexpected::Str(v), &"null"))
                } else {
                    Err(de::Error::invalid_value(
                        Unexpected::Bytes(&scalar.value.value),
                        &"null",
                    ))
                }
            }
            &Event::Alias(mut pos) => self.jump(&mut pos)?.deserialize_unit(visitor),
            Event::Void => visitor.visit_unit(),
            other => Err(invalid_type(other, &visitor)),
        }
        .map_err(|err| error::fix_mark(err, mark, self.path))
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    /// Parses a newtype struct as the underlying value.
    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let (_event, mark) = self.peek_event_mark()?;
        self.recursion_check(mark, |de| visitor.visit_newtype_struct(de))
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let (next, mark) = self.next_event_mark()?;
        match next {
            &Event::Alias(mut pos) => self.jump(&mut pos)?.deserialize_seq(visitor),
            Event::SequenceStart(_) => self.visit_sequence(visitor, mark),
            other => {
                if match other {
                    Event::Void => true,
                    Event::Scalar(scalar) => {
                        scalar.value.value.is_empty() && scalar.value.style == ScalarStyle::Plain
                    }
                    _ => false,
                } {
                    visitor.visit_seq(SeqAccess {
                        empty: true,
                        de: self,
                        len: 0,
                    })
                } else {
                    Err(invalid_type(other, &visitor))
                }
            }
        }
        .map_err(|err| error::fix_mark(err, mark, self.path))
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let (next, mark) = self.next_event_mark()?;
        match next {
            &Event::Alias(mut pos) => self.jump(&mut pos)?.deserialize_map(visitor),
            Event::MappingStart(_) => self.visit_mapping(visitor, mark),
            other => {
                if match other {
                    Event::Void => true,
                    Event::Scalar(scalar) => {
                        scalar.value.value.is_empty() && scalar.value.style == ScalarStyle::Plain
                    }
                    _ => false,
                } {
                    visitor.visit_map(MapAccess {
                        empty: true,
                        de: self,
                        len: 0,
                        key: None,
                        seen: HashSet::new(),
                    })
                } else {
                    Err(invalid_type(other, &visitor))
                }
            }
        }
        .map_err(|err| error::fix_mark(err, mark, self.path))
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    /// Parses an enum as a single key:value pair where the key identifies the
    /// variant and the value gives the content. A String will also parse correctly
    /// to a unit enum value.
    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let (next, mark) = self.peek_event_mark()?;
        let result = match next {
            &Event::Alias(mut pos) => {
                *self.pos += 1;
                self.jump(&mut pos)?
                    .deserialize_enum(name, variants, visitor)
            }
            Event::Scalar(scalar) => {
                if let Some(tag) = parse_tag(scalar.value.tag.as_ref()) {
                    return visitor.visit_enum(EnumAccess {
                        de: self,
                        tag,
                    });
                }
                visitor.visit_enum(UnitVariantAccess { de: self })
            }
            Event::MappingStart(mapping) => {
                if let Some(tag) = parse_tag(mapping.tag.as_ref()) {
                    return visitor.visit_enum(EnumAccess {
                        de: self,
                        tag,
                    });
                }
                self.next_event_mark()?; // consume MappingStart
                let (key_event, key_mark) = self.next_event_mark()?;
                let tag = match key_event {
                    Event::Scalar(scalar) => {
                        match std::str::from_utf8(&scalar.value.value) {
                            Ok(s) => s,
                            Err(_) => {
                                return Err(error::fix_mark(
                                    de::Error::invalid_type(
                                        Unexpected::Bytes(&scalar.value.value),
                                        &"string",
                                    ),
                                    key_mark,
                                    self.path,
                                ))
                            }
                        }
                    }
                    _ => {
                        struct ExpectedString;
                        impl Expected for ExpectedString {
                            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                                f.write_str("string")
                            }
                        }
                        let err = invalid_type(key_event, &ExpectedString);
                        return Err(error::fix_mark(err, key_mark, self.path));
                    }
                };
                let result = visitor.visit_enum(EnumAccess {
                    de: self,
                    tag,
                });
                let result = result.and_then(|v| {
                    self.end_mapping(1)?;
                    Ok(v)
                });
                return result;
            }
                Event::SequenceStart(sequence) => {
                    if let Some(tag) = parse_tag(sequence.tag.as_ref()) {
                        return visitor.visit_enum(EnumAccess {
                            de: self,
                            tag,
                        });
                    }
                    let err =
                        de::Error::invalid_type(Unexpected::Seq, &"a YAML tag starting with '!'");
                    Err(error::fix_mark(err, mark, self.path))
                }
                Event::SequenceEnd => Err(error::new(ErrorImpl::UnexpectedEndOfSequence)),
                Event::MappingEnd => Err(error::new(ErrorImpl::UnexpectedEndOfMapping)),
                Event::Void => Err(error::new(ErrorImpl::EndOfStream)),
        };
        result.map_err(|err| error::fix_mark(err, mark, self.path))
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.ignore_any()?;
        visitor.visit_unit()
    }
}

/// Deserialize an instance of type `T` from a string of YAML text.
///
/// YAML merge keys are resolved automatically before the deserialization into
/// `T` happens.
///
/// This conversion can fail if the structure of the Value does not match the
/// structure expected by `T`, for example if `T` is a struct type but the Value
/// contains something other than a YAML map. It can also fail if the structure
/// is correct but `T`'s implementation of `Deserialize` decides that something
/// is wrong with the data, for example required struct fields are missing from
/// the YAML map or some number is too big to fit in the expected primitive
/// type.
pub fn from_str<T>(s: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    let mut value: Value = Value::deserialize(Deserializer::from_str(s))?;
    value.apply_merge()?;
    crate::value::from_value(value)
}

/// Deserialize an instance of type `T` from an IO stream of YAML.
///
/// YAML merge keys are resolved automatically before the deserialization into
/// `T` happens.
///
/// This conversion can fail if the structure of the Value does not match the
/// structure expected by `T`, for example if `T` is a struct type but the Value
/// contains something other than a YAML map. It can also fail if the structure
/// is correct but `T`'s implementation of `Deserialize` decides that something
/// is wrong with the data, for example required struct fields are missing from
/// the YAML map or some number is too big to fit in the expected primitive
/// type.
pub fn from_reader<R, T>(rdr: R) -> Result<T>
where
    R: io::Read,
    T: DeserializeOwned,
{
    let mut value: Value = Value::deserialize(Deserializer::from_reader(rdr))?;
    value.apply_merge()?;
    crate::value::from_value(value)
}

/// Deserialize an instance of type `T` from bytes of YAML text.
///
/// YAML merge keys are resolved automatically before the deserialization into
/// `T` happens.
///
/// This conversion can fail if the structure of the Value does not match the
/// structure expected by `T`, for example if `T` is a struct type but the Value
/// contains something other than a YAML map. It can also fail if the structure
/// is correct but `T`'s implementation of `Deserialize` decides that something
/// is wrong with the data, for example required struct fields are missing from
/// the YAML map or some number is too big to fit in the expected primitive
/// type.
pub fn from_slice<T>(v: &[u8]) -> Result<T>
where
    T: DeserializeOwned,
{
    let mut value: Value = Value::deserialize(Deserializer::from_slice(v))?;
    value.apply_merge()?;
    crate::value::from_value(value)
}

/// Deserialize a list of `T` from multiple YAML documents provided in a string.
pub fn from_str_multi<T>(s: &str) -> Result<Vec<T>>
where
    T: DeserializeOwned,
{
    Deserializer::from_str(s)
        .map(|doc| {
            let mut value: Value = Value::deserialize(doc)?;
            value.apply_merge()?;
            crate::value::from_value(value)
        })
        .collect()
}

/// Deserialize a list of `T` from multiple YAML documents provided in an IO stream.
pub fn from_reader_multi<R, T>(rdr: R) -> Result<Vec<T>>
where
    R: io::Read,
    T: DeserializeOwned,
{
    Deserializer::from_reader(rdr)
        .map(|doc| {
            let mut value: Value = Value::deserialize(doc)?;
            value.apply_merge()?;
            crate::value::from_value(value)
        })
        .collect()
}

/// Deserialize a list of `T` from multiple YAML documents provided in bytes.
pub fn from_slice_multi<T>(v: &[u8]) -> Result<Vec<T>>
where
    T: DeserializeOwned,
{
    Deserializer::from_slice(v)
        .map(|doc| {
            let mut value: Value = Value::deserialize(doc)?;
            value.apply_merge()?;
            crate::value::from_value(value)
        })
        .collect()
}

/// Deserialize a YAML `Value` while preserving anchors and aliases.
#[allow(clippy::redundant_closure_for_method_calls)]
pub fn from_str_value_preserve(s: &str) -> Result<Value> {
    Deserializer::from_str(s).de(|state| state.parse_value())
}
