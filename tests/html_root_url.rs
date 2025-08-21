#[test]
fn html_root_url_matches_package_version() {
    let lib_rs = include_str!("../src/lib.rs");
    let actual = lib_rs
        .lines()
        .find(|line| line.contains("html_root_url"))
        .expect("html_root_url attribute missing");
    let expected = format!(
        "#![doc(html_root_url = \"https://docs.rs/serde_yaml_bw/{}\")]",
        env!("CARGO_PKG_VERSION"),
    );
    assert_eq!(
        actual,
        expected,
        "html_root_url attribute is out of sync with package version",
    );
}
