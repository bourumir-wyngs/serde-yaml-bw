#![no_main]

use libfuzzer_sys::fuzz_target;
use serde_yaml_bw::{Value};
use std::io::Cursor;

fuzz_target!(|data: &[u8]| {
    if data.len() <= 10240 {
        let _ = serde_yaml_bw::from_slice::<serde_yaml_bw::Value>(data);
        
        let reader = Cursor::new(data);
        let _: Result<Value, _> = serde_yaml_bw::from_reader(reader);
    }
});
