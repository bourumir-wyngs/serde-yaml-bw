// 6CA3: Tab indented top flow — an empty flow sequence with tab indentation
#[test]
#[ignore]
fn yaml_6ca3_tab_indented_top_flow() {
    // Use actual tabs to indent the flow collection.
    let y = "\t\t\t\t[\n\t\t\t\t]\n";
    let v: Vec<u8> = serde_yaml_bw::from_str(y).expect("failed to parse 6CA3");
    assert!(v.is_empty());
}
