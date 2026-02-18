use serde_yaml_gtc as serde_yaml;
// NP9H: Double Quoted Line Breaks — verify folding and escapes
// Expected: "folded to a space,\nto a line feed, or \t \tnon-content"
#[test]
fn yaml_np9h_double_quoted_line_breaks() {
    // Notes:
    // - In double-quoted scalars, a single line break folds to a SPACE.
    // - An EMPTY line becomes a REAL newline '\n'.
    // - A backslash '\' at end of a line ESCAPES the following newline (no space/newline inserted),
    //   and leading indentation on the next line is ignored.
    // - "\t" produces a TAB character.
    let y = r#""folded
 to a space,

 to a line feed, or \
 \t \tnon-content"
"#;

    let s: String = serde_yaml::from_str(y).expect("failed to parse NP9H");
    assert_eq!(s, "folded to a space,\nto a line feed, or \t \tnon-content");
}
