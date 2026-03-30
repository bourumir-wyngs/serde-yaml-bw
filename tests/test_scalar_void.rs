use serde::Deserialize;
use serde_yaml_bw::{budget::Budget, Deserializer, DeserializerOptions};
use std::io::Cursor;

#[test]
fn test_scalar_budget_breach_bool() {
    let input = "true\n";
    let options = DeserializerOptions {
        budget: Some(Budget {
            max_total_scalar_bytes: 0,
            ..Budget::default()
        }),
        ..DeserializerOptions::default()
    };
    
    let err = bool::deserialize(Deserializer::from_reader_with_options(
        Cursor::new(input.as_bytes()),
        &options,
    )).unwrap_err();
    
    assert!(err.to_string().contains("budget exceeded"), "Expected budget exceeded, got: {:?}", err);
}