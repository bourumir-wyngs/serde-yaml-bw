use base64::{engine::general_purpose, Engine as _};

pub fn decode_base64(input: &str) -> Option<Vec<u8>> {
    let bytes: Vec<u8> = input
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .collect();

    general_purpose::STANDARD.decode(bytes).ok()
}
