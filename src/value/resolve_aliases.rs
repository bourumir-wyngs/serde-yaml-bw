use crate::error::{self, Error, ErrorImpl};
use crate::Value;
use std::collections::{HashMap, HashSet};

impl Value {
    /// Recursively replace all [`Alias`](Value::Alias) nodes with copies of the
    /// values referenced by their anchors.
    ///
    /// ```
    /// use serde_yaml_bw::{Value, from_str_value};
    ///
    /// let yaml = "a: &anchor 1\nb: *anchor";
    /// let value: Value = from_str_value(yaml).unwrap();
    /// assert_eq!(value["b"], Value::Number(1.into(), None));
    /// ```
    pub fn resolve_aliases(&mut self) -> Result<(), Error> {
        fn count_nodes(value: &Value) -> usize {
            match value {
                Value::Sequence(seq) => 1 + seq.elements.iter().map(count_nodes).sum::<usize>(),
                Value::Mapping(map) => {
                    1 + map
                        .iter()
                        .map(|(k, v)| count_nodes(k) + count_nodes(v))
                        .sum::<usize>()
                }
                Value::Tagged(tagged) => 1 + count_nodes(&tagged.value),
                _ => 1,
            }
        }

        fn collect_anchors(value: &Value, anchors: &mut HashMap<String, Value>) {
            match value {
                Value::Null(anchor)
                | Value::Bool(_, anchor)
                | Value::Number(_, anchor)
                | Value::String(_, anchor) => {
                    if let Some(name) = anchor {
                        anchors.insert(name.clone(), value.clone());
                    }
                }
                Value::Sequence(seq) => {
                    if let Some(name) = &seq.anchor {
                        anchors.insert(name.clone(), value.clone());
                    }
                    for item in &seq.elements {
                        collect_anchors(item, anchors);
                    }
                }
                Value::Mapping(map) => {
                    if let Some(name) = &map.anchor {
                        anchors.insert(name.clone(), value.clone());
                    }
                    for (k, v) in map {
                        collect_anchors(k, anchors);
                        collect_anchors(v, anchors);
                    }
                }
                Value::Tagged(tagged) => collect_anchors(&tagged.value, anchors),
                Value::Alias(_) => {}
            }
        }

        fn resolve(
            value: &mut Value,
            anchors: &HashMap<String, Value>,
            visiting: &mut HashSet<String>,
            expansions: &mut usize,
            expansion_limit: usize,
        ) -> Result<(), Error> {
            match value {
                Value::Alias(name) => {
                    *expansions = expansions
                        .checked_add(1)
                        .ok_or_else(|| error::new(ErrorImpl::RepetitionLimitExceeded))?;
                    if *expansions > expansion_limit {
                        return Err(error::new(ErrorImpl::RepetitionLimitExceeded));
                    }
                    let alias = name.clone();
                    if !visiting.insert(alias.clone()) {
                        return Err(error::new(ErrorImpl::MergeRecursion));
                    }
                    let mut replacement = match anchors.get(&alias) {
                        Some(v) => v.clone(),
                        None => return Err(error::new(ErrorImpl::UnresolvedAlias)),
                    };
                    resolve(
                        &mut replacement,
                        anchors,
                        visiting,
                        expansions,
                        expansion_limit,
                    )?;
                    strip_anchors(&mut replacement);
                    *value = replacement;
                    visiting.remove(&alias);
                    Ok(())
                }
                Value::Sequence(seq) => {
                    for item in &mut seq.elements {
                        resolve(item, anchors, visiting, expansions, expansion_limit)?;
                    }
                    Ok(())
                }
                Value::Mapping(map) => {
                    for v in map.values_mut() {
                        resolve(v, anchors, visiting, expansions, expansion_limit)?;
                    }
                    Ok(())
                }
                Value::Tagged(tagged) => {
                    resolve(&mut tagged.value, anchors, visiting, expansions, expansion_limit)
                }
                _ => Ok(()),
            }
        }

        fn strip_anchors(value: &mut Value) {
            match value {
                Value::Null(anchor)
                | Value::Bool(_, anchor)
                | Value::Number(_, anchor)
                | Value::String(_, anchor) => {
                    *anchor = None;
                }
                Value::Sequence(seq) => {
                    seq.anchor = None;
                    for v in &mut seq.elements {
                        strip_anchors(v);
                    }
                }
                Value::Mapping(map) => {
                    map.anchor = None;
                    for v in map.values_mut() {
                        strip_anchors(v);
                    }
                }
                Value::Tagged(tagged) => strip_anchors(&mut tagged.value),
                Value::Alias(_) => {}
            }
        }

        let mut anchors = HashMap::new();
        collect_anchors(self, &mut anchors);
        let mut visiting = HashSet::new();
        let expansion_limit = count_nodes(self)
            .checked_mul(100)
            .ok_or_else(|| error::new(ErrorImpl::RepetitionLimitExceeded))?;
        let mut expansions = 0;
        resolve(
            self,
            &anchors,
            &mut visiting,
            &mut expansions,
            expansion_limit,
        )
    }
}
