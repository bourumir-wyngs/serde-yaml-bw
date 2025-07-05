use std::collections::HashMap;
use serde_derive::Deserialize;

#[test] 
fn test_duplicate_keys() {
    #[derive(Debug, Deserialize)]
    struct Data {
        data: HashMap<String, i32>,
    }

    let yaml_no_dups = "data:\n  key1: 1\n  key2: 2";
    match serde_yaml_bw::from_str::<Data>(yaml_no_dups) {
        Ok(data) => {
            assert_eq!(2, data.data.len());
            assert_eq!(1, *data.data.get("key1").unwrap());
            assert_eq!(2, *data.data.get("key2").unwrap());
        }
        Err(err) => assert_eq!(format!("{}", err), r#"Failes to parse valid YAML"#),
    }
    

    let yaml_dups = "data:\n  key: 1\n  key: 2";
    match serde_yaml_bw::from_str::<Data>(yaml_dups) {
        Ok(data) => { panic!("Takes duplicate keys and returns {data:?}") }
        Err(err) => assert_eq!(format!("{}", err), r#"duplicate key: "1""#),
    }
}