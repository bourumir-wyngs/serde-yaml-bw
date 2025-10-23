// BS4K: Comment between plain scalar lines — marked fail: true
// Expect parsing to return an error (no panic).
#[test]
#[ignore] // !ssfr
fn yaml_bs4k_comment_between_plain_scalar_lines_should_fail() {
    let y = "word1  # comment\nword2\n";
    let result: Result<String, _> = serde_yaml_bw::from_str(y);
    assert!(result.is_err(), "BS4K should fail due to comment breaking a multiline plain scalar");
}