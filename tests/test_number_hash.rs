use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde_yaml_bw::Number;

#[test]
fn hash_distinguishes_floats() {
    let a = Number::from(1.0f64);
    let b = Number::from(2.0f64);
    let mut hasher_a = DefaultHasher::new();
    a.hash(&mut hasher_a);
    let mut hasher_b = DefaultHasher::new();
    b.hash(&mut hasher_b);
    assert_ne!(hasher_a.finish(), hasher_b.finish());
}

#[test]
fn hash_zero_eq_neg_zero() {
    let a = Number::from(0.0f64);
    let b = Number::from(-0.0f64);
    let mut hasher_a = DefaultHasher::new();
    a.hash(&mut hasher_a);
    let mut hasher_b = DefaultHasher::new();
    b.hash(&mut hasher_b);
    assert_eq!(hasher_a.finish(), hasher_b.finish());
}
