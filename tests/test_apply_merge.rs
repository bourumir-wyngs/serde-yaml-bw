use std::collections::HashMap;
use indoc::indoc;
use serde_yaml_bw::{from_str_value_preserve, Value};

fn resolve_aliases(value: &mut Value) {
    fn collect_anchors(value: &Value, map: &mut HashMap<String, Value>) {
        match value {
            Value::Mapping(m) => {
                if let Some(name) = &m.anchor {
                    map.insert(name.clone(), value.clone());
                }
                for v in m.values() {
                    collect_anchors(v, map);
                }
            }
            Value::Sequence(s) => {
                if let Some(name) = &s.anchor {
                    map.insert(name.clone(), value.clone());
                }
                for v in &s.elements {
                    collect_anchors(v, map);
                }
            }
            Value::Tagged(t) => collect_anchors(&t.value, map),
            _ => {}
        }
    }

    fn substitute(value: &mut Value, map: &HashMap<String, Value>) {
        match value {
            Value::Alias(name) => {
                if let Some(v) = map.get(name).cloned() {
                    *value = v;
                }
            }
            Value::Mapping(m) => {
                for v in m.values_mut() {
                    substitute(v, map);
                }
            }
            Value::Sequence(s) => {
                for v in &mut s.elements {
                    substitute(v, map);
                }
            }
            Value::Tagged(t) => substitute(&mut t.value, map),
            _ => {}
        }
    }

    let mut anchors = HashMap::new();
    collect_anchors(value, &mut anchors);
    substitute(value, &anchors);
}

#[test]
fn test_apply_merge_preserves_and_merges() {
    let yaml = r#"defaults: &defaults
  adapter: postgres
  host: localhost

development:
  <<: *defaults
  database: dev_db

production:
  <<: *defaults
  database: prod_db"#;

    let mut value: Value = from_str_value_preserve(yaml).expect("failed to parse YAML");
    resolve_aliases(&mut value);
    value.apply_merge().expect("failed to apply merge");

    assert_eq!(value["development"]["host"].as_str().unwrap(), "localhost");
    assert_eq!(value["development"]["adapter"].as_str().unwrap(), "postgres");
    assert_eq!(value["production"]["host"].as_str().unwrap(), "localhost");
    assert_eq!(value["production"]["database"].as_str().unwrap(), "prod_db");
}

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

    assert_eq!(value["tasks"]["start"]["command"], "webpack");
    assert_eq!(value["tasks"]["start"]["args"], "start");
}
