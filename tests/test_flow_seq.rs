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
