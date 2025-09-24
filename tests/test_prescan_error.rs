use serde_yaml_bw::{self as yaml, Value, Error};

#[test]
fn prescan_reports_location_and_message() {
    // Saphyr's scanner should error before the main parser runs.
    let yaml_input = "key: value: another";

    let err: Error = yaml::from_str::<Value>(yaml_input).expect_err("expected pre-scan error");
    let msg = err.to_string();

    // The message should come from Saphyr's ScanError and include human-readable context
    println!("[{}]", msg);
    assert_eq!(msg, "mapping values are not allowed in this context at line 1 column 11");

    // location() should also be populated with precise coordinates.
    let loc = err.location().expect("expected location on pre-scan error");
    assert_eq!(loc.line(), 1);
    assert_eq!(loc.column(), 11);
}
