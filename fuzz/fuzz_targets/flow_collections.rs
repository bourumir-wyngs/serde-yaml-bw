#![no_main]

use libfuzzer_sys::fuzz_target;
use serde_yaml_bw::Value;
use std::io::Cursor;

// This fuzzer focuses on flow-style collections: sequences [..] and mappings {..}.
// It wraps the fuzzer input into flow collections in several ways and exercises
// deserialization through different entry points.
fuzz_target!(|data: &[u8]| {
    if data.len() > 16 * 1024 { return; }

    let s = String::from_utf8_lossy(data);

    // 1) Flow sequence
    let yaml_seq = format!("[{s}]");
    // 2) Flow mapping
    let yaml_map = format!("{{{s}}}");
    // 3) A struct-like document using flow mapping at top level
    let yaml_doc = format!("root: {{{s}}}\narray: [{s}]\n");

    for y in [&yaml_seq, &yaml_map, &yaml_doc] {
        let _v1: Result<Value, _> = serde_yaml_bw::from_str(y);
        let _v2: Result<Value, _> = serde_yaml_bw::from_reader(Cursor::new(y.as_bytes()));
        let _v3: Result<Value, _> = serde_yaml_bw::from_slice(y.as_bytes());
    }
});
