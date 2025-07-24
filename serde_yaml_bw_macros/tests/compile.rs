#[test]
fn invalid_yaml_fails() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/invalid.rs");
}
