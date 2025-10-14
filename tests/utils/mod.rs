use std::fmt::Debug;
use serde::Deserialize;
use serde_yaml_bw::{Deserializer, DeserializerOptions};

// Common test utilities shared across integration tests.
// All helpers in this module disable pathology detection by default, so tests
// exercise the second layer of defense in the parser unless a test explicitly
// wants pathology on (e.g., in test_repro_fuzz_targets.rs).

/// Returns DeserializerOptions with pathology detection disabled.
#[allow(dead_code)]
#[cfg(test)]
pub(crate) fn opts_no_pathology() -> DeserializerOptions {
    DeserializerOptions {
        budget: None,
        ..DeserializerOptions::default()
    }
}

/// Builds a Deserializer from &str with pathology detection disabled.
/// This function is used.
#[allow(dead_code)]
#[cfg(test)]
pub(crate) fn deserializer_no_pathology<'de>(yaml: &'de str) -> Deserializer<'de> {
    Deserializer::from_str_with_options(yaml, &opts_no_pathology())
}

// Run test with pathological YAML detector disabled as we must check if the second line
// of defense is still present. After all, pathology detector only activates for large
// input (to focus on DOS)
#[cfg(test)]
#[allow(dead_code)]
pub(crate) fn test_error<'de, T>(yaml: &'de str, expected: &str)
where
    T: Deserialize<'de> + Debug,
{
    // Run this test with pathology detection turned off to verify the second layer of defense.
    let opts = opts_no_pathology();

    let result = T::deserialize(Deserializer::from_str_with_options(yaml, &opts));
    assert_eq!(expected, result.unwrap_err().to_string());

    let mut deserializer = Deserializer::from_str_with_options(yaml, &opts);
    if let Some(first_document) = deserializer.next() {
        if deserializer.next().is_none() {
            let result = T::deserialize(first_document);
            assert_eq!(expected, result.unwrap_err().to_string());
        }
    }
}
