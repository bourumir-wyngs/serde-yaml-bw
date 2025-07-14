// Tests for Mapping::entry APIs
use serde_yaml_bw::{Mapping, Value};
use serde_yaml_bw::mapping::Entry;

#[test]
fn test_or_insert() {
    let mut mapping = Mapping::new();
    let ptr_first: *const Value = {
        let first = mapping.entry("k".into()).or_insert(1.into());
        assert_eq!(first.as_i64(), Some(1));
        first as *const _
    };
    assert_eq!(mapping.len(), 1);

    let ptr_second: *const Value = {
        let second = mapping.entry("k".into()).or_insert(2.into());
        assert_eq!(second.as_i64(), Some(1));
        second as *const _
    };
    assert_eq!(mapping.len(), 1);
    assert_eq!(ptr_first, ptr_second);
}

#[test]
fn test_or_insert_with() {
    let mut mapping = Mapping::new();
    let ptr_first: *const Value = {
        let first = mapping.entry("k".into()).or_insert_with(|| 1.into());
        assert_eq!(first.as_i64(), Some(1));
        first as *const _
    };
    assert_eq!(mapping.len(), 1);

    let ptr_second: *const Value = {
        let second = mapping.entry("k".into()).or_insert_with(|| 2.into());
        assert_eq!(second.as_i64(), Some(1));
        second as *const _
    };
    assert_eq!(mapping.len(), 1);
    assert_eq!(ptr_first, ptr_second);
}

#[test]
fn test_and_modify() {
    let mut mapping = Mapping::new();
    mapping.entry("k".into()).or_insert(1.into());

    let ptr_result: *const Value = {
        let result = mapping
            .entry("k".into())
            .and_modify(|v| *v = 2.into())
            .or_insert(0.into());
        assert_eq!(result.as_i64(), Some(2));
        result as *const _
    };
    assert_eq!(mapping.len(), 1);
    assert_eq!(ptr_result, mapping.get("k").unwrap() as *const _);
}

#[test]
fn test_entry_and_modify() {
    let mut map = Mapping::new();
    map.insert("a".into(), 1.into());

    match map.entry("a".into()).and_modify(|v| *v = 10.into()) {
        Entry::Occupied(o) => assert_eq!(o.get(), &Value::from(10)),
        Entry::Vacant(_) => panic!("expected occupied"),
    }

    match map.entry("b".into()) {
        Entry::Vacant(e) => {
            Entry::Vacant(e).or_insert(20.into());
        }
        Entry::Occupied(_) => panic!("expected vacant"),
    }

    match map.entry("c".into()) {
        Entry::Vacant(e) => {
            Entry::Vacant(e).or_insert_with(|| 30.into());
        }
        Entry::Occupied(_) => panic!("expected vacant"),
    }

    assert_eq!(map.get("a"), Some(&Value::from(10)));
    assert_eq!(map.get("b"), Some(&Value::from(20)));
    assert_eq!(map.get("c"), Some(&Value::from(30)));
}

#[test]
fn test_retain() {
    let mut map = Mapping::new();
    map.insert("a".into(), 1.into());
    map.insert("b".into(), 2.into());
    map.insert("c".into(), 3.into());

    map.retain(|_, v| v.as_i64().unwrap() % 2 == 1);

    assert!(map.contains_key("a"));
    assert!(!map.contains_key("b"));
    assert!(map.contains_key("c"));
    assert_eq!(map.len(), 2);
}

#[test]
fn test_into_keys_values_order() {
    let mut map = Mapping::new();
    map.insert("x".into(), 1.into());
    map.insert("y".into(), 2.into());

    let keys: Vec<_> = map.clone().into_keys().collect();
    assert_eq!(keys, vec![Value::from("x"), Value::from("y")]);

    let values: Vec<_> = map.into_values().collect();
    assert_eq!(values, vec![Value::from(1), Value::from(2)]);
}
