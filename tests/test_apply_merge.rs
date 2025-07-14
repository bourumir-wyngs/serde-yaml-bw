use indoc::indoc;
use serde_yaml_bw::Value;

#[test]
fn test_apply_merge_example() {
    let config = indoc! {r#"
        tasks:
          build: &webpack_shared
            command: webpack
            args: build
            inputs:
              - 'src/**/*'
          start:
            <<: *webpack_shared
            args: start
    "#};

    let mut value: Value = serde_yaml_bw::from_str(config).unwrap();
    value.apply_merge().unwrap();
    value.resolve_aliases().unwrap();

    assert_eq!(value["tasks"]["start"]["command"], "webpack");
    assert_eq!(value["tasks"]["start"]["args"], "start");
}
