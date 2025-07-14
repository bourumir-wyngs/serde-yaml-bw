pub fn decode_base64(input: &str) -> Option<Vec<u8>> {
    let mut cleaned = Vec::with_capacity(input.len());
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'+' | b'/' | b'=' => cleaned.push(b),
            b' ' | b'\n' | b'\r' | b'\t' => {}
            _ => return None,
        }
    }
    if cleaned.len() % 4 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(cleaned.len() / 4 * 3);
    let mut i = 0;
    while i < cleaned.len() {
        let c0 = cleaned[i];
        let c1 = cleaned[i + 1];
        let c2 = cleaned[i + 2];
        let c3 = cleaned[i + 3];
        let v0 = decode_b64_char(c0)?;
        let v1 = decode_b64_char(c1)?;
        if c2 == b'=' {
            if c3 != b'=' || i + 4 != cleaned.len() {
                return None;
            }
            out.push((v0 << 2) | (v1 >> 4));
            break;
        }
        let v2 = decode_b64_char(c2)?;
        if c3 == b'=' {
            if i + 4 != cleaned.len() {
                return None;
            }
            out.push((v0 << 2) | (v1 >> 4));
            out.push(((v1 & 0xf) << 4) | (v2 >> 2));
            break;
        }
        let v3 = decode_b64_char(c3)?;
        out.push((v0 << 2) | (v1 >> 4));
        out.push(((v1 & 0xf) << 4) | (v2 >> 2));
        out.push(((v2 & 0x3) << 6) | v3);
        i += 4;
    }
    Some(out)
}

fn decode_b64_char(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}
