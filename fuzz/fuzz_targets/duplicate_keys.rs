#![no_main]

use libfuzzer_sys::fuzz_target;
use serde::Deserialize;
use serde_yaml_bw::Value;
use std::collections::HashMap;
use std::io::Cursor;

// This fuzzer constructs YAML mappings with intentional duplicate keys to
// exercise duplicate-key strategies and diagnostics. It targets both Value
// and typed maps/struct-like shapes.
fuzz_target!(|data: &[u8]| {
    if data.len() > 16 * 1024 { return; }
    let s = String::from_utf8_lossy(data);

    // Simple top-level map with duplicates.
    let yaml_top = format!("a: 1\na: 2\nkey: {s}\nkey: {s}\n");

    // Nested duplicates within flow and block styles.
    let yaml_nested = format!(
        "outer:\n  inner: {{x: 1, x: 2}}\n  arr: [{{k: {s}}}, {{k: {s}}}]\n"
    );

    for y in [&yaml_top, &yaml_nested] {
        let _v: Result<Value, _> = serde_yaml_bw::from_str(y);
        let _v_reader: Result<Value, _> = serde_yaml_bw::from_reader(Cursor::new(y.as_bytes()));
        let _v_slice: Result<Value, _> = serde_yaml_bw::from_slice(y.as_bytes());

        // Also try deserializing into a typed map to execute MapAccess path.
        let _typed: Result<HashMap<String, Value>, _> = serde_yaml_bw::from_str(y);
    }

    // Additionally, build YAML via flow mapping using the bytes as a key.
    let yaml_flow = format!("{{'{s}': 1, '{s}': 2}}\n");
    let _v_flow: Result<Value, _> = serde_yaml_bw::from_str(&yaml_flow);
});
