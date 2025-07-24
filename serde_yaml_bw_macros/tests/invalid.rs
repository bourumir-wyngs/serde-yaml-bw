use serde_yaml_bw_macros::yaml;
fn main() {
    let _val = yaml!("invalid: [1, 2");
}

