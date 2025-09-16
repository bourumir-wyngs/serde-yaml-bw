use serde::Serialize;
use serde_yaml_bw::{to_string, FlowSeq};

#[derive(Serialize)]
struct Data {
    flow: FlowSeq<Vec<u32>>,
    block: Vec<u32>,
}

#[test]
fn flow_sequence_renders_with_brackets() {
    let data = Data {
        flow: FlowSeq(vec![1, 2, 3]),
        block: vec![4, 5, 6],
    };
    let yaml = to_string(&data).unwrap();
    assert_eq!(yaml, "flow: [1, 2, 3]\nblock:\n- 4\n- 5\n- 6\n");
}

#[test]
fn test_flow_seq_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    // Arrange
    let f: FlowSeq<Vec<i32>> = FlowSeq(vec![1, 2, 3]);

    // Act: serialize to YAML, then deserialize back
    let yaml = serde_yaml_bw::to_string(&f)?;
    let from_yaml: FlowSeq<Vec<i32>> = serde_yaml_bw::from_str(&yaml)?;

    // Assert: round-trip equality
    assert_eq!(from_yaml, f, "Deserialized value should equal the original");

    // Optional: ensure we're actually getting FLOW style (e.g., "[1, 2, 3]\n")
    assert!(
        yaml.trim_start().starts_with('['),
        "Expected flow-style YAML sequence, got:\n{yaml}"
    );

    Ok(())
}