use serde_yaml_gtc as serde_yaml;
#[test]
#[ignore] // Libyaml limitation, would require preprocessor.
fn yaml_3hfz_invalid_content_after_document_end_marker_should_error() {
    let yaml = "---\nkey: value\n... invalid\n";

    let result = serde_yaml::from_str::<std::collections::HashMap<String, String>>(yaml);
    assert!(
        result.is_err(),
        "Expected error due to trailing content after document end marker, got: {:?}",
        result
    );
}
