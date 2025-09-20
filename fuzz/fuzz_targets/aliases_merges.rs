#![no_main]

use libfuzzer_sys::fuzz_target;
use serde_yaml_bw::Value;
use std::io::Cursor;

// This fuzzer biases inputs toward anchors, aliases, and YAML merge keys (<<).
// It constructs a couple of YAML documents influenced by the input and runs
// both Value-preserving and normal deserialization paths.
fuzz_target!(|data: &[u8]| {
    if data.len() > 16 * 1024 { return; }

    // Basic bytes to string; replace invalid UTF-8 with replacement so we can embed it safely.
    let s = String::from_utf8_lossy(data);

    // 1) Anchors and aliases only.
    let yaml_alias = format!(
        "a: &A {s}\nb: *A\nseq: &S [1, 2, 3]\nseq_alias: *S\n"
    );

    // 2) Merge keys: build two maps that get merged into target.
    let yaml_merge = format!(
        "base1: &B1 {{k: 1, v: {s}}}\nbase2: &B2 {{k: 2, w: {s}}}\nmerged: {{<<: [*B1, *B2], extra: 3}}\n"
    );

    for y in [&yaml_alias, &yaml_merge] {
        // Preserve aliases first.
        let _preserve: Result<Value, _> = serde_yaml_bw::from_str_value_preserve(y);

        // Normal path resolves aliases then applies merges inside from_str_value.
        let _value_only: Result<Value, _> = serde_yaml_bw::from_str_value(y);

        // Also run generic from_str to a Value (covers post-processing apply_merge path).
        let _value: Result<Value, _> = serde_yaml_bw::from_str(y);

        // Exercise reader-based API as well.
        let _reader_value: Result<Value, _> = serde_yaml_bw::from_reader(Cursor::new(y.as_bytes()));
    }
});
