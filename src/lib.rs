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
//!     let values: Vec<Enum> = serde_yaml_bw::from_str(yaml).unwrap();
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
//!     let values: Vec<Enum> = serde_yaml_bw::from_str(yaml).unwrap();
//!     assert_eq!(values[0], Enum::Tuple(0, 0, 0));
//!     assert_eq!(values[1], Enum::Struct { x: 1.0, y: 2.0 });
//!
//!     // Variants with no data are written as just the string name.
//!     let yaml = "
//!         - Unit
//!     ";
//!     let values: Vec<Enum> = serde_yaml_bw::from_str(yaml).unwrap();
//!     assert_eq!(values[0], Enum::Unit);
//!
//!     Ok(())
//! }
//! ```

#![doc(html_root_url = "https://docs.rs/serde_yaml_bw/2.1.0")]
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
    from_reader, from_reader_multi, from_slice, from_slice_multi, from_str, from_str_multi,
    from_str_value, digits_but_not_number, parse_bool_casefold, parse_f64,
    Deserializer, StreamDeserializer, DeserializerOptions
};
pub use crate::error::{Error, Location, Result};
pub use crate::ser::{
    to_string, to_string_multi, to_writer, to_writer_multi, Serializer, SerializerBuilder,
};
#[doc(inline)]
pub use crate::value::{from_value, to_value, Number, Sequence, Value};

#[doc(inline)]
pub use crate::mapping::Mapping;

mod de;
mod error;
mod libyaml;
mod loader;
pub mod mapping;
mod duplicate_key;
mod number;
mod path;
mod ser;
pub mod value;

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

