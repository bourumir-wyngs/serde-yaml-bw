use serde::{Deserialize, Serialize};
use serde_yaml_bw::{to_string, yaml};

#[test]
fn yaml_macro_map() {
    let value = yaml!({
        "x": 1,
        "enabled": true,
        "list": [1, 2, 3],
    });

    assert_eq!(
        to_string(&value).unwrap(),
        "x: 1\nenabled: true\nlist:\n- 1\n- 2\n- 3\n"
    );
}

#[test]
fn yaml_macro_struct_roundtrip() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Config {
        name: String,
        count: u8,
    }

    let cfg = Config {
        name: "demo".into(),
        count: 2,
    };

    let value = yaml!({
        "name": cfg.name.clone(),
        "count": cfg.count,
    });

    let parsed: Config = serde_yaml_bw::from_value(value).unwrap();
    assert_eq!(parsed, cfg);
}
