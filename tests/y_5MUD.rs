use std::collections::HashMap;

// 5MUD: Colon and adjacent value on next line in flow mapping
#[test]
#[ignore]
fn yaml_5mud_colon_and_adjacent_value_on_next_line() {
    let y = "---\n{ \"foo\"\n  :bar }\n";
    let m: HashMap<String, String> = serde_yaml_bw::from_str(y).expect("failed to parse 5MUD");
    assert_eq!(m.get("foo").map(String::as_str), Some("bar"));
}
