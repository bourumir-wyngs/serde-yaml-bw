use std::io::Read;

struct InfiniteZero;
impl Read for InfiniteZero {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        for b in buf.iter_mut() {
            *b = 0;
        }
        Ok(buf.len())
    }
}

#[derive(serde::Deserialize, Debug)]
struct Dummy {
    _a: i32,
}

#[test]
fn test_infinite_zero() {
    let rdr = InfiniteZero;
    // Should fail quickly, not OOM
    let err: Result<Dummy, _> = serde_yaml_bw::from_reader(rdr);
    assert!(err.is_err());
}