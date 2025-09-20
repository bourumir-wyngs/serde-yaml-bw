#![no_main]

use libfuzzer_sys::fuzz_target;
use serde_yaml_bw::Value;
use std::io::Cursor;

// This fuzzer stresses large scalar handling, both plain and block scalars.
// We cap constructed sizes to avoid pathological memory usage.
fuzz_target!(|data: &[u8]| {
    // Cap to 1 MiB generated content.
    let cap: usize = 1 << 20;

    // Repeat the fuzz input to build a long line.
    let mut plain = String::new();
    while plain.len() < cap {
        if plain.len() + data.len() > cap { break; }
        plain.push_str(&String::from_utf8_lossy(data));
    }

    // 1) Plain scalar
    let yaml_plain = format!("{plain}\n");

    // 2) Block literal scalar with folded lines
    let yaml_block = format!("|\n  {plain}\n  {plain}\n");

    for y in [&yaml_plain, &yaml_block] {
        let _v1: Result<Value, _> = serde_yaml_bw::from_str(y);
        let _v2: Result<Value, _> = serde_yaml_bw::from_reader(Cursor::new(y.as_bytes()));
        let _v3: Result<Value, _> = serde_yaml_bw::from_slice(y.as_bytes());

        // Also try deserializing directly into String in some cases.
        let _s: Result<String, _> = serde_yaml_bw::from_str(y);
    }
});
