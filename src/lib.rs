//!
//! Rust library for using the [Serde] serialization framework with data in
//! [YAML] file format.
//!
//! [Serde]: https://github.com/serde-rs/serde
//! [YAML]: https://yaml.org/
//!
//! # Examples
//!
//! ```
//! use std::collections::BTreeMap;
//!
//! fn main() -> Result<(), serde_yaml_bw::Error> {
//!     // You have some type.
//!     let mut map = BTreeMap::new();
//!     map.insert("x".to_string(), 1.0);
//!     map.insert("y".to_string(), 2.0);
//!
//!     // Serialize it to a YAML string.
//!     let yaml = serde_yaml_bw::to_string(&map)?;
//!     assert_eq!(yaml, "x: 1.0\ny: 2.0\n");
//!
//!     // Deserialize it back to a Rust type.
//!     let deserialized_map: BTreeMap<String, f64> = serde_yaml_bw::from_str(&yaml)?;
//!     assert_eq!(map, deserialized_map);
//!     Ok(())
//! }
//! ```

//! ## Errors
//!
//! Attempting to serialize a value with an invalid YAML tag will
//! result in an [`Error`] whose cause is internally represented by the private
//! `TagError` variant.
//!
//! ## Using Serde derive
//!
//! It can also be used with Serde's derive macros to handle structs and enums
//! defined in your program.
//!
//! Structs serialize in the obvious way:
//!
//! ```
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize, PartialEq, Debug)]
//! struct Point {
//!     x: f64,
//!     y: f64,
//! }
//!
//! fn main() -> Result<(), serde_yaml_bw::Error> {
//!     let point = Point { x: 1.0, y: 2.0 };
//!
//!     let yaml = serde_yaml_bw::to_string(&point)?;
//!     assert_eq!(yaml, "x: 1.0\ny: 2.0\n");
//!
//!     let deserialized_point: Point = serde_yaml_bw::from_str(&yaml)?;
//!     assert_eq!(point, deserialized_point);
//!     Ok(())
//! }
//! ```
//!
//! Enums serialize using a YAML map whose key is the variant name.
//!
//! ```
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize, PartialEq, Debug)]
//! enum Enum {
//!     Unit,
//!     Newtype(usize),
//!     Tuple(usize, usize, usize),
//!     Struct { x: f64, y: f64 },
//! }
//!
//! fn main() -> Result<(), serde_yaml_bw::Error> {
//!     let yaml = "
//!         - Newtype: 1
//!         - Tuple:
//!           - 0
//!           - 0
//!           - 0
//!         - Struct:
//!             x: 1.0
//!             y: 2.0
//!     ";
//!     let values: Vec<Enum> = serde_yaml_bw::from_str(yaml)?;
//!     assert_eq!(values[0], Enum::Newtype(1));
//!     assert_eq!(values[1], Enum::Tuple(0, 0, 0));
//!     assert_eq!(values[2], Enum::Struct { x: 1.0, y: 2.0 });
//!
//!     // The last two in YAML's block style instead:
//!     let yaml = "
//!         - Tuple:
//!           - 0
//!           - 0
//!           - 0
//!         - Struct:
//!             x: 1.0
//!             y: 2.0
//!     ";
//!     let values: Vec<Enum> = serde_yaml_bw::from_str(yaml)?;
//!     assert_eq!(values[0], Enum::Tuple(0, 0, 0));
//!     assert_eq!(values[1], Enum::Struct { x: 1.0, y: 2.0 });
//!
//!     // Variants with no data are written as just the string name.
//!     let yaml = "
//!         - Unit
//!     ";
//!     let values: Vec<Enum> = serde_yaml_bw::from_str(yaml)?;
//!     assert_eq!(values[0], Enum::Unit);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Using `Value`
//!
//! The `Value` enum represents any YAML node at runtime. It is useful when you
//! don't have a fixed Rust type or when you need to preserve YAML features such
//! as anchors and aliases.
//!
//! Basic construction examples:
//!
//! ```
//! use serde_yaml_bw::{Mapping, Value};
//!
//! // Scalars
//! let n = Value::from(42);                // number
//! let b = Value::from(true);              // bool
//! let s = Value::from("hello");           // string
//!
//! // Sequence from Vec and from iterator
//! let seq = Value::from(vec![1, 2, 3]);
//! let seq2: Value = ["a", "b", "c"].into_iter().collect();
//!
//! // Mapping (a key to value map)
//! let mut m = Mapping::new();
//! m.set("name", Value::from("app"));
//! m.set("version", Value::from(1));
//! let doc = Value::from(m);
//!
//! // Serialize to YAML
//! let yaml = serde_yaml_bw::to_string(&doc).unwrap();
//! assert!(yaml.contains("name: app"));
//! ```
//!
//! ### Anchors and aliases
//!
//! When building `Value`s manually you can attach an anchor to scalars,
//! sequences, and mappings, and you can create `Value::Alias` nodes that refer
//! to them by name. During serialization, aliases to missing anchors are
//! rejected by default.
//!
//! ```
//! use serde_yaml_bw::{Mapping, Value};
//!
//! // Create a scalar with an anchor "greet"
//! let anchored = Value::String("Hello".to_string(), Some("greet".to_string()));
//!
//! // Build a small document that reuses the anchored value via an alias
//! let mut root = Mapping::new();
//! root.set("first", anchored.clone());
//! root.set("second", Value::Alias("greet".to_string()));
//! let yaml = serde_yaml_bw::to_string(&Value::from(root)).unwrap();
//!
//! // The exact formatting may vary, but both an anchor and an alias are emitted
//! assert!(yaml.contains("&greet"), "expected an anchor in: {yaml}");
//! assert!(yaml.contains("*greet"), "expected an alias in: {yaml}");
//! ```
//!
//! Anchors can be attached to complex nodes too:
//!
//! ```
//! use serde_yaml_bw::{Mapping, Value};
//!
//! // An anchored mapping
//! let mut base = Mapping::with_anchor("base");
//! base.set("x", 1.into());
//! base.set("y", 2.into());
//!
//! // Refer to it elsewhere using an alias node
//! let mut root = Mapping::new();
//! root.set("anchor", Value::from(base));
//! root.set("alias", Value::Alias("base".into()));
//!
//! let out = serde_yaml_bw::to_string(&Value::from(root)).unwrap();
//! assert!(out.contains("&base"));
//! assert!(out.contains("*base"));
//! ```
//!
//! If you serialize an alias whose anchor has not appeared earlier in the
//! document, serialization fails by default. You can change this behavior using
//! `SerializerBuilder::check_unresolved_anchors(false)` if you prefer deferred
//! checking on the consumer side.
//!
//! ```
//! use serde::Serialize;
//! use serde_yaml_bw::{SerializerBuilder, Value};
//!
//! // Try to serialize a dangling alias and observe the error.
//! let mut buf = Vec::new();
//! let mut ser = SerializerBuilder::default()
//!     .check_unresolved_anchors(true)
//!     .build(&mut buf)
//!     .unwrap();
//!
//! let err = Value::Alias("missing".into()).serialize(&mut ser).unwrap_err();
//! assert!(err.to_string().starts_with("reference to non existing anchor"));
//! ```
//!
//! For in-memory processing of YAML Values that contain anchors/aliases, you
//! can expand aliases after parsing:
//!
//! ```
//! use serde_yaml_bw::Value;
//!
//! let yaml = "a: &id 1\nb: *id\n";
//! let mut v: Value = serde_yaml_bw::from_str_value_preserve(yaml).unwrap();
//! v.resolve_aliases().unwrap();
//! assert_eq!(v["a"].as_i64(), v["b"].as_i64());
//! ```

#![doc(html_root_url = "https://docs.rs/serde_yaml_bw/2.4.0")]
#![deny(missing_docs, unsafe_op_in_unsafe_fn)]
// Suppressed clippy_pedantic lints
#![allow(
    // buggy
    clippy::iter_not_returning_iterator, // https://github.com/rust-lang/rust-clippy/issues/8285
    clippy::ptr_arg, // https://github.com/rust-lang/rust-clippy/issues/9218
    clippy::question_mark, // https://github.com/rust-lang/rust-clippy/issues/7859
    // private Deserializer::next
    clippy::should_implement_trait,
    // things are often more readable this way
    clippy::cast_lossless,
    clippy::checked_conversions,
    clippy::if_not_else,
    clippy::manual_assert,
    clippy::match_like_matches_macro,
    clippy::match_same_arms,
    clippy::module_name_repetitions,
    clippy::needless_pass_by_value,
    clippy::redundant_else,
    clippy::single_match_else,
    // code is acceptable
    clippy::blocks_in_conditions,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::derive_partial_eq_without_eq,
    clippy::derived_hash_with_manual_eq,
    clippy::doc_markdown,
    clippy::items_after_statements,
    clippy::let_underscore_untyped,
    clippy::manual_map,
    clippy::missing_panics_doc,
    clippy::never_loop,
    clippy::return_self_not_must_use,
    clippy::too_many_lines,
    clippy::uninlined_format_args,
    clippy::unsafe_removed_from_name,
    clippy::wildcard_in_or_patterns,
    // noisy
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
)]

pub use crate::de::{
    Deserializer, DeserializerOptions, StreamDeserializer, digits_but_not_number, from_reader,
    from_reader_multi, from_slice, from_slice_multi, from_str, from_str_multi, from_str_value,
    from_str_value_preserve, parse_bool_casefold, parse_f64, from_multiple
};
pub use crate::error::{Error, Location, Result};
pub use crate::libyaml::emitter::SequenceStyle;
pub use crate::ser::{
    ArcAnchor, ArcWeakAnchor, FlowMap, FlowSeq, RcAnchor, RcWeakAnchor, Serializer,
    SerializerBuilder, SerializerOptions, to_string, to_string_multi, to_string_multi_with_options,
    to_string_with_options, to_writer, to_writer_multi, to_writer_multi_with_options,
    to_writer_with_options,
};
#[doc(inline)]
pub use crate::value::{Number, Sequence, Value, from_value, to_value};

#[doc(inline)]
pub use crate::mapping::Mapping;

pub mod budget;
mod de;
mod duplicate_key;
mod error;
mod libyaml;
mod loader;
pub mod mapping;
mod number;
mod path;
mod ser;
pub mod value;
mod zmij_format;

pub use crate::number::unexpected;

// Prevent downstream code from implementing the Index trait.
mod private {
    pub trait Sealed {}
    impl Sealed for usize {}
    impl Sealed for str {}
    impl Sealed for String {}
    impl Sealed for crate::Value {}
    impl<T> Sealed for &T where T: ?Sized + Sealed {}
}
