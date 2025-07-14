use serde_yaml_bw::{Mapping, Value};

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
