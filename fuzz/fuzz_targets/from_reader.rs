#![no_main]

use libfuzzer_sys::fuzz_target;
use serde_yaml_bw::{Deserializer, Value};
use std::io::Cursor;

// Our reader is now more than just wrapper around slice so needs special attention
fuzz_target!(|data: &[u8]| {
    if data.len() <= 10240 {
        let reader = Cursor::new(data);
        let _result: Result<Value, _> = serde_yaml_bw::from_reader(reader);
    }
});