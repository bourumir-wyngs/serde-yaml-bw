use serde::{Deserialize, Serialize};
use std::io::{self, Cursor, Read};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Point {
    x: i32,
}

#[test]
fn test_from_slice_and_multi() {
    let bytes = b"x: 1\n";
    let point: Point = serde_yaml_bw::from_slice(bytes).unwrap();
    assert_eq!(point, Point { x: 1 });

    let multi = b"---\nx: 1\n---\nx: 2\n";
    let points: Vec<Point> = serde_yaml_bw::from_slice_multi(multi).unwrap();
    assert_eq!(points, vec![Point { x: 1 }, Point { x: 2 }]);
}

#[test]
fn test_from_reader_multi() {
    let multi = b"---\nx: 1\n---\nx: 2\n".to_vec();
    let cursor = Cursor::new(multi);
    let points: Vec<Point> = serde_yaml_bw::from_reader_multi(cursor).unwrap();
    assert_eq!(points, vec![Point { x: 1 }, Point { x: 2 }]);
}

#[test]
fn test_to_writer_and_multi() {
    let mut buf = Vec::new();
    let point = Point { x: 1 };
    serde_yaml_bw::to_writer(&mut buf, &point).unwrap();
    assert_eq!(String::from_utf8(buf.clone()).unwrap(), "x: 1\n");

    buf.clear();
    let points = vec![Point { x: 1 }, Point { x: 2 }];
    serde_yaml_bw::to_writer_multi(&mut buf, &points).unwrap();
    assert_eq!(String::from_utf8(buf).unwrap(), "x: 1\n---\nx: 2\n");
}

#[test]
fn test_error_location() {
    let result: Result<Point, _> = serde_yaml_bw::from_str("@");
    let err = result.unwrap_err();
    let loc = err.location().expect("location missing");
    assert_eq!(loc.line(), 1);
    assert_eq!(loc.column(), 1);
}

#[test]
fn test_from_reader_does_not_read_past_early_parse_error() {
    struct InvalidPrefixWithReadLimit {
        emitted: usize,
        limit: usize,
    }

    impl Read for InvalidPrefixWithReadLimit {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.emitted >= self.limit {
                return Err(io::Error::other("reader limit exceeded"));
            }

            let remaining = self.limit - self.emitted;
            let n = remaining.min(buf.len()).max(1);
            if self.emitted == 0 {
                buf[0] = b'@';
                for b in &mut buf[1..n] {
                    *b = b'a';
                }
            } else {
                for b in &mut buf[..n] {
                    *b = b'a';
                }
            }
            self.emitted += n;
            Ok(n)
        }
    }

    let result: Result<Point, _> = serde_yaml_bw::from_reader(InvalidPrefixWithReadLimit {
        emitted: 0,
        limit: 64 * 1024,
    });
    let err = result.unwrap_err();
    assert!(!err.to_string().contains("reader limit exceeded"));
}

#[test]
fn test_from_reader_ignored_field_does_not_require_alias_expansion() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Data {
        used: i32,
    }

    let yaml = b"used: 1\nignored: &loop\n  - *loop\n";
    let parsed: Data = serde_yaml_bw::from_reader(Cursor::new(yaml)).unwrap();
    assert_eq!(parsed, Data { used: 1 });
}
