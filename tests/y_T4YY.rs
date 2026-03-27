use serde_json::Value;
use serde_yaml_gtc as serde_yaml;

// T4YY: Single quoted lines with a real trailing space and a blank line between lines.

#[test]
fn y_t4yy_single_quoted_lines_suite_glyphs() {
    let y = "---\n' 1st non-empty\n\n  2nd non-empty \n  3rd non-empty '\n";
    let r: Result<Value, _> = serde_yaml::from_str(y);
    assert!(
        r.is_ok(),
        "Parser failed to handle single-quoted multi-line scalar with trailing space and blank line: {:?}",
        r
    );
}
