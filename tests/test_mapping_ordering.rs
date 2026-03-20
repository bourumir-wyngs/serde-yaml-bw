use serde_yaml_bw::{Mapping, Value};
use std::cmp::Ordering;

fn mapping_key(first: (&str, i64), second: (&str, i64)) -> Value {
    let mut map = Mapping::new();
    map.insert(first.0.into(), first.1.into());
    map.insert(second.0.into(), second.1.into());
    Value::Mapping(map)
}

#[test]
fn mapping_partial_ord_is_consistent_with_eq_for_mapping_keys() {
    let key_a_1 = mapping_key(("a", 1), ("z", 2));
    let key_b_1 = mapping_key(("m", 3), ("n", 4));

    let key_a_2 = mapping_key(("z", 2), ("a", 1));
    let key_b_2 = mapping_key(("n", 4), ("m", 3));

    let mut lhs = Mapping::new();
    lhs.insert(key_a_1, Value::from("left-a"));
    lhs.insert(key_b_1, Value::from("left-b"));

    let mut rhs = Mapping::new();
    rhs.insert(key_a_2, Value::from("left-a"));
    rhs.insert(key_b_2, Value::from("left-b"));

    assert_eq!(lhs, rhs);
    assert_eq!(lhs.partial_cmp(&rhs), Some(Ordering::Equal));
}
