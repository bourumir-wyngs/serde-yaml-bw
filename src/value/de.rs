use crate::value::tagged::{self, TagStringVisitor};
use crate::value::TaggedValue;
use crate::{number, Error, Mapping, Sequence, Value};
use serde::de::value::{BorrowedStrDeserializer, StrDeserializer};
use serde::de::{
    self, Deserialize, DeserializeSeed, Deserializer, EnumAccess, Error as _, Expected, MapAccess,
    SeqAccess, Unexpected, VariantAccess, Visitor,
};
use serde::forward_to_deserialize_any;
use std::fmt;
use std::slice;
use std::vec;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ValueVisitor;

        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = Value;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("any YAML value")
            }

            fn visit_bool<E>(self, b: bool) -> Result<Value, E>
            where
                E: de::Error,
            {
                Ok(Value::Bool(b, None))
            }

            fn visit_i64<E>(self, i: i64) -> Result<Value, E>
            where
                E: de::Error,
            {
                Ok(Value::Number(i.into(), None))
            }

            fn visit_u64<E>(self, u: u64) -> Result<Value, E>
            where
                E: de::Error,
            {
                Ok(Value::Number(u.into(), None))
            }

            fn visit_f64<E>(self, f: f64) -> Result<Value, E>
            where
                E: de::Error,
            {
                Ok(Value::Number(f.into(), None))
            }

            fn visit_str<E>(self, s: &str) -> Result<Value, E>
            where
                E: de::Error,
            {
                Ok(Value::String(s.to_owned(), None))
            }

            fn visit_string<E>(self, s: String) -> Result<Value, E>
            where
                E: de::Error,
            {
                Ok(Value::String(s, None))
            }

            fn visit_unit<E>(self) -> Result<Value, E>
            where
                E: de::Error,
            {
                Ok(Value::Null(None))
            }

            fn visit_none<E>(self) -> Result<Value, E>
            where
                E: de::Error,
            {
                Ok(Value::Null(None))
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                Deserialize::deserialize(deserializer)
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Value, E>
            where
                E: de::Error,
            {
                let mut sequence = Sequence::with_capacity(v.len());
                sequence.elements = v
                    .iter()
                    .copied()
                    .map(|b| Value::Number(number::Number::from(b), None))
                    .collect();
                Ok(Value::Sequence(sequence))
            }

            fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Value, E>
            where
                E: de::Error,
            {
                let mut sequence = Sequence::with_capacity(v.len());
                sequence.elements = v
                    .into_iter()
                    .map(|b| Value::Number(number::Number::from(b), None))
                    .collect();
                Ok(Value::Sequence(sequence))
            }

            fn visit_seq<A>(self, data: A) -> Result<Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let de = serde::de::value::SeqAccessDeserializer::new(data);
                let sequence = Sequence::deserialize(de)?;
                Ok(Value::Sequence(sequence))
            }

            fn visit_map<A>(self, data: A) -> Result<Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let de = serde::de::value::MapAccessDeserializer::new(data);
                let mapping = Mapping::deserialize(de)?;
                Ok(Value::Mapping(mapping))
            }

            fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
            where
                A: EnumAccess<'de>,
            {
                let (tag, contents) = data.variant_seed(TagStringVisitor)?;
                let value = contents.newtype_variant()?;
                Ok(Value::Tagged(Box::new(TaggedValue { tag, value })))
            }
        }

        deserializer.deserialize_any(ValueVisitor)
    }
}

impl Value {
    fn deserialize_number<'de, V>(&self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.untag_ref() {
            Value::Number(n, _) => n.deserialize_any(visitor),
            other => Err(other.invalid_type(&visitor)),
        }
    }
}

fn visit_sequence<'de, V>(sequence: Sequence, visitor: V) -> Result<V::Value, Error>
where
    V: Visitor<'de>,
{
    let len = sequence.len();
    let mut deserializer = SeqDeserializer::new(sequence);
    let seq = visitor.visit_seq(&mut deserializer)?;
    let remaining = deserializer.iter.len();
    if remaining == 0 {
        Ok(seq)
    } else {
        Err(Error::invalid_length(len, &"fewer elements in sequence"))
    }
}

fn visit_sequence_ref<'de, V>(sequence: &'de Sequence, visitor: V) -> Result<V::Value, Error>
where
    V: Visitor<'de>,
{
    let len = sequence.len();
    let mut deserializer = SeqRefDeserializer::new(sequence);
    let seq = visitor.visit_seq(&mut deserializer)?;
    let remaining = deserializer.iter.len();
    if remaining == 0 {
        Ok(seq)
    } else {
        Err(Error::invalid_length(len, &"fewer elements in sequence"))
    }
}

fn visit_mapping<'de, V>(mapping: Mapping, visitor: V) -> Result<V::Value, Error>
where
    V: Visitor<'de>,
{
    let len = mapping.len();
    let mut deserializer = MapDeserializer::new(mapping);
    let map = visitor.visit_map(&mut deserializer)?;
    let remaining = deserializer.iter.len();
    if remaining == 0 {
        Ok(map)
    } else {
        Err(Error::invalid_length(len, &"fewer elements in map"))
    }
}

fn visit_mapping_ref<'de, V>(mapping: &'de Mapping, visitor: V) -> Result<V::Value, Error>
where
    V: Visitor<'de>,
{
    let len = mapping.len();
    let mut deserializer = MapRefDeserializer::new(mapping);
    let map = visitor.visit_map(&mut deserializer)?;
    if let Some(remaining) = deserializer.iter {
        if remaining.len() == 0 {
            Ok(map)
        } else {
            Err(Error::invalid_length(len, &"fewer elements in map"))
        }
    } else {
        Err(Error::invalid_length(len, &"failed to unwrap deserializer"))
    }
}

impl<'de> Deserializer<'de> for Value {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Null(_) => visitor.visit_unit(),
            Value::Bool(v, _) => visitor.visit_bool(v),
            Value::Number(n, _) => n.deserialize_any(visitor),
            Value::String(v, _) => visitor.visit_string(v),
            Value::Sequence(v) => visit_sequence(v, visitor),
            Value::Mapping(v) => visit_mapping(v, visitor),
            Value::Alias(name) => visitor.visit_str(&name),
            Value::Tagged(tagged) => visitor.visit_enum(*tagged),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.untag() {
            Value::Bool(v, _) => visitor.visit_bool(v),
            other => Err(other.invalid_type(&visitor)),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.untag() {
            Value::String(v, _) => visitor.visit_string(v),
            other => Err(other.invalid_type(&visitor)),
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_byte_buf(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Tagged(tagged) if tagged.tag == crate::libyaml::tag::Tag::BINARY => {
                match tagged.value {
                    Value::String(v, _) => match BASE64_STANDARD.decode(&v) {
                        Ok(bytes) => visitor.visit_byte_buf(bytes),
                        Err(_err) => Err(de::Error::invalid_value(Unexpected::Str(&v), &"base64")),
                    },
                    other => Err(other.invalid_type(&visitor)),
                }
            }
            other => match other.untag() {
                Value::String(v, _) => visitor.visit_string(v),
                Value::Sequence(v) => visit_sequence(v, visitor),
                other => Err(other.invalid_type(&visitor)),
            },
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Null(_) => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Null(_) => visitor.visit_unit(),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.untag() {
            Value::Sequence(v) => visit_sequence(v, visitor),
            Value::Null(_) => visit_sequence(Sequence::new(), visitor),
            other => Err(other.invalid_type(&visitor)),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Error>
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
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.untag() {
            Value::Mapping(v) => visit_mapping(v, visitor),
            Value::Null(_) => visit_mapping(Mapping::new(), visitor),
            other => Err(other.invalid_type(&visitor)),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let tag;
        visitor.visit_enum(match self {
            Value::Tagged(tagged) => EnumDeserializer {
                tag: {
                    tag = tagged.tag.string;
                    tagged::nobang(&tag)
                },
                value: Some(tagged.value),
            },
            Value::String(variant, _) => EnumDeserializer {
                tag: {
                    tag = variant;
                    &tag
                },
                value: None,
            },
            Value::Mapping(map) => {
                if map.len() == 1 {
                    let (key, value) = map.into_iter().next().unwrap();
                    match key {
                        Value::String(var, _) => {
                            tag = var;
                            EnumDeserializer { tag: &tag, value: Some(value) }
                        }
                        other => {
                            return Err(Error::invalid_type(
                                other.unexpected(),
                                &"string",
                            ));
                        }
                    }
                } else {
                    return Err(Error::invalid_type(
                        Value::Mapping(map).unexpected(),
                        &"a leaf for empty enum, otherwise map naming the fields for enum",
                    ));
                }
            }
            other => {
                return Err(Error::invalid_type(
                    other.unexpected(),
                    &"a leaf for empty enum, otherwise map naming the fields for enum",
                ));
            }
        })
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        drop(self);
        visitor.visit_unit()
    }
}

struct EnumDeserializer<'a> {
    tag: &'a str,
    value: Option<Value>,
}

impl<'de> EnumAccess<'de> for EnumDeserializer<'_> {
    type Error = Error;
    type Variant = VariantDeserializer;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Error>
    where
        V: DeserializeSeed<'de>,
    {
        let str_de = StrDeserializer::<Error>::new(self.tag);
        let variant = seed.deserialize(str_de)?;
        let visitor = VariantDeserializer { value: self.value };
        Ok((variant, visitor))
    }
}

struct VariantDeserializer {
    value: Option<Value>,
}

impl<'de> VariantAccess<'de> for VariantDeserializer {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        match self.value {
            Some(value) => value.unit_variant(),
            None => Ok(()),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.value {
            Some(value) => value.newtype_variant_seed(seed),
            None => Err(Error::invalid_type(
                Unexpected::UnitVariant,
                &"newtype variant",
            )),
        }
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Some(value) => value.tuple_variant(len, visitor),
            None => Err(Error::invalid_type(
                Unexpected::UnitVariant,
                &"tuple variant",
            )),
        }
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Some(value) => value.struct_variant(fields, visitor),
            None => Err(Error::invalid_type(
                Unexpected::UnitVariant,
                &"struct variant",
            )),
        }
    }
}

pub(crate) struct SeqDeserializer {
    iter: vec::IntoIter<Value>,
}

impl SeqDeserializer {
    pub(crate) fn new(seq: Sequence) -> Self {
        SeqDeserializer {
            iter: seq.into_iter(),
        }
    }
}

impl<'de> Deserializer<'de> for SeqDeserializer {
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let len = self.iter.len();
        if len == 0 {
            visitor.visit_unit()
        } else {
            let ret = visitor.visit_seq(&mut self)?;
            let remaining = self.iter.len();
            if remaining == 0 {
                Ok(ret)
            } else {
                Err(Error::invalid_length(len, &"fewer elements in sequence"))
            }
        }
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        drop(self);
        visitor.visit_unit()
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct
        map struct enum identifier
    }
}

impl<'de> SeqAccess<'de> for SeqDeserializer {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => seed.deserialize(value).map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        match self.iter.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None,
        }
    }
}

pub(crate) struct MapDeserializer {
    iter: <Mapping as IntoIterator>::IntoIter,
    value: Option<Value>,
}

impl MapDeserializer {
    pub(crate) fn new(map: Mapping) -> Self {
        MapDeserializer {
            iter: map.into_iter(),
            value: None,
        }
    }
}

impl<'de> MapAccess<'de> for MapDeserializer {
    type Error = Error;

    fn next_key_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                seed.deserialize(key).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<T>(&mut self, seed: T) -> Result<T::Value, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => seed.deserialize(value),
            None => Err(de::Error::custom("visit_value called before visit_key"))
        }
    }

    fn size_hint(&self) -> Option<usize> {
        match self.iter.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None,
        }
    }
}

impl<'de> Deserializer<'de> for MapDeserializer {
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(self)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        drop(self);
        visitor.visit_unit()
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct
        map struct enum identifier
    }
}

impl<'de> Deserializer<'de> for &'de Value {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Null(_) => visitor.visit_unit(),
            Value::Bool(v, _) => visitor.visit_bool(*v),
            Value::Number(n, _) => n.deserialize_any(visitor),
            Value::String(v, _) => visitor.visit_borrowed_str(v),
            Value::Sequence(v) => visit_sequence_ref(v, visitor),
            Value::Mapping(v) => visit_mapping_ref(v, visitor),
            Value::Alias(name) => visitor.visit_borrowed_str(name),
            Value::Tagged(tagged) => visitor.visit_enum(&**tagged),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.untag_ref() {
            Value::Bool(v, _) => visitor.visit_bool(*v),
            other => Err(other.invalid_type(&visitor)),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.untag_ref() {
            Value::String(v, _) => visitor.visit_borrowed_str(v),
            other => Err(other.invalid_type(&visitor)),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Tagged(tagged) if tagged.tag == crate::libyaml::tag::Tag::BINARY => {
                match &tagged.value {
                    Value::String(v, _) => match BASE64_STANDARD.decode(v) {
                        Ok(bytes) => visitor.visit_byte_buf(bytes),
                        Err(_err) => Err(de::Error::invalid_value(Unexpected::Str(v), &"base64")),
                    },
                    other => Err(other.invalid_type(&visitor)),
                }
            }
            _ => match self.untag_ref() {
                Value::String(v, _) => visitor.visit_borrowed_str(v),
                Value::Sequence(v) => visit_sequence_ref(v, visitor),
                other => Err(other.invalid_type(&visitor)),
            },
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Tagged(tagged) if tagged.tag == crate::libyaml::tag::Tag::BINARY => {
                match &tagged.value {
                    Value::String(v, _) => match BASE64_STANDARD.decode(v) {
                        Ok(bytes) => visitor.visit_byte_buf(bytes),
                        Err(_err) => Err(de::Error::invalid_value(Unexpected::Str(v), &"base64")),
                    },
                    other => Err(other.invalid_type(&visitor)),
                }
            }
            _ => match self.untag_ref() {
                Value::String(v, _) => visitor.visit_borrowed_str(v),
                Value::Sequence(v) => visit_sequence_ref(v, visitor),
                other => Err(other.invalid_type(&visitor)),
            },
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Null(_) => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Null(_) => visitor.visit_unit(),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        static EMPTY: Sequence = Sequence::const_new();
        match self.untag_ref() {
            Value::Sequence(v) => visit_sequence_ref(v, visitor),
            Value::Null(_) => visit_sequence_ref(&EMPTY, visitor),
            other => Err(other.invalid_type(&visitor)),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Error>
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
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.untag_ref() {
            Value::Mapping(v) => visit_mapping_ref(v, visitor),
            Value::Null(_) => visitor.visit_map(&mut MapRefDeserializer {
                iter: None,
                value: None,
            }),
            other => Err(other.invalid_type(&visitor)),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_enum(match self {
            Value::Tagged(tagged) => EnumRefDeserializer {
                tag: tagged::nobang(&tagged.tag.string),
                value: Some(&tagged.value),
            },
            Value::String(variant, _) => EnumRefDeserializer {
                tag: variant,
                value: None,
            },
            Value::Mapping(map) => {
                if map.len() == 1 {
                    let (key, value) = map.iter().next().unwrap();
                    match key {
                        Value::String(var, _) => EnumRefDeserializer {
                            tag: var,
                            value: Some(value),
                        },
                        other => {
                            return Err(Error::invalid_type(
                                other.unexpected(),
                                &"string",
                            ));
                        }
                    }
                } else {
                    return Err(Error::invalid_type(
                        Value::Mapping(map.clone()).unexpected(),
                        &"a leaf for empty enum, otherwise map naming the fields for enum",
                    ));
                }
            }
            other => {
                return Err(Error::invalid_type(
                    other.unexpected(),
                    &"a leaf for empty enum, otherwise map naming the fields for enum",
                ));
            }
        })
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }
}

struct EnumRefDeserializer<'de> {
    tag: &'de str,
    value: Option<&'de Value>,
}

impl<'de> EnumAccess<'de> for EnumRefDeserializer<'de> {
    type Error = Error;
    type Variant = VariantRefDeserializer<'de>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Error>
    where
        V: DeserializeSeed<'de>,
    {
        let str_de = BorrowedStrDeserializer::<Error>::new(self.tag);
        let variant = seed.deserialize(str_de)?;
        let visitor = VariantRefDeserializer { value: self.value };
        Ok((variant, visitor))
    }
}

struct VariantRefDeserializer<'de> {
    value: Option<&'de Value>,
}

impl<'de> VariantAccess<'de> for VariantRefDeserializer<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        match self.value {
            Some(value) => value.unit_variant(),
            None => Ok(()),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.value {
            Some(value) => value.newtype_variant_seed(seed),
            None => Err(Error::invalid_type(
                Unexpected::UnitVariant,
                &"newtype variant",
            )),
        }
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Some(value) => value.tuple_variant(len, visitor),
            None => Err(Error::invalid_type(
                Unexpected::UnitVariant,
                &"tuple variant",
            )),
        }
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Some(value) => value.struct_variant(fields, visitor),
            None => Err(Error::invalid_type(
                Unexpected::UnitVariant,
                &"struct variant",
            )),
        }
    }
}

pub(crate) struct SeqRefDeserializer<'de> {
    iter: slice::Iter<'de, Value>,
}

impl<'de> SeqRefDeserializer<'de> {
    pub(crate) fn new(slice: &'de [Value]) -> Self {
        SeqRefDeserializer { iter: slice.iter() }
    }
}

impl<'de> Deserializer<'de> for SeqRefDeserializer<'de> {
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let len = self.iter.len();
        if len == 0 {
            visitor.visit_unit()
        } else {
            let ret = visitor.visit_seq(&mut self)?;
            let remaining = self.iter.len();
            if remaining == 0 {
                Ok(ret)
            } else {
                Err(Error::invalid_length(len, &"fewer elements in sequence"))
            }
        }
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct
        map struct enum identifier
    }
}

impl<'de> SeqAccess<'de> for SeqRefDeserializer<'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => seed.deserialize(value).map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        match self.iter.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None,
        }
    }
}

pub(crate) struct MapRefDeserializer<'de> {
    iter: Option<<&'de Mapping as IntoIterator>::IntoIter>,
    value: Option<&'de Value>,
}

impl<'de> MapRefDeserializer<'de> {
    pub(crate) fn new(map: &'de Mapping) -> Self {
        MapRefDeserializer {
            iter: Some(map.iter()),
            value: None,
        }
    }
}

impl<'de> MapAccess<'de> for MapRefDeserializer<'de> {
    type Error = Error;

    fn next_key_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.as_mut().and_then(Iterator::next) {
            Some((key, value)) => {
                self.value = Some(value);
                seed.deserialize(key).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<T>(&mut self, seed: T) -> Result<T::Value, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => seed.deserialize(value),
            None => Err(de::Error::custom("visit_value called before visit_key"))
        }
    }

    fn size_hint(&self) -> Option<usize> {
        match self.iter.as_ref()?.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None,
        }
    }
}

impl<'de> Deserializer<'de> for MapRefDeserializer<'de> {
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(self)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct
        map struct enum identifier
    }
}

impl Value {
    #[cold]
    fn invalid_type<E>(&self, exp: &dyn Expected) -> E
    where
        E: de::Error,
    {
        de::Error::invalid_type(self.unexpected(), exp)
    }

    #[cold]
    pub(crate) fn unexpected(&self) -> Unexpected {
        match self {
            Value::Null(_) => Unexpected::Unit,
            Value::Bool(b, _) => Unexpected::Bool(*b),
            Value::Number(n, _) => number::unexpected(n),
            Value::String(s, _) => Unexpected::Str(s),
            Value::Sequence(_) => Unexpected::Seq,
            Value::Mapping(_) => Unexpected::Map,
            Value::Alias(_) => Unexpected::Other("alias"),
            Value::Tagged(_) => Unexpected::Enum,
        }
    }
}
