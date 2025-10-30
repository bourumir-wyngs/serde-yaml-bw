use crate::error::{self, Error, ErrorImpl};
use crate::value::tagged::Tag;
use crate::Value;
use crate::Number;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// Shared reference-counted node representing a YAML value with anchors preserved.
pub type AnchorNode = Rc<AnchorValue>;

/// Tagged YAML value within an [`AnchorValue`].
#[derive(Clone, Debug, PartialEq)]
pub struct AnchorTagged {
    /// YAML tag associated with this value.
    pub tag: Tag,
    /// Tagged value contents.
    pub value: AnchorNode,
}

/// YAML value that preserves alias identity through shared [`Rc`] pointers.
#[derive(Clone, Debug, PartialEq)]
pub enum AnchorValue {
    /// YAML `null` value.
    Null,
    /// Boolean scalar value.
    Bool(bool),
    /// Numeric scalar value.
    Number(Number),
    /// String scalar value.
    String(String),
    /// YAML sequence preserving alias identity.
    Sequence(Vec<AnchorNode>),
    /// YAML mapping preserving alias identity.
    Mapping(Vec<(AnchorNode, AnchorNode)>),
    /// Tagged YAML node.
    Tagged(AnchorTagged),
}

impl Value {
    /// Build a reference-counted graph representation of this YAML value, preserving
    /// the identity of anchors and aliases.
    pub fn to_anchor_graph(&self) -> Result<AnchorNode, Error> {
        let mut anchors = HashMap::new();
        let mut visiting = HashSet::new();
        build_anchor_node(self, &mut anchors, &mut visiting)
    }
}

fn build_anchor_node(
    value: &Value,
    anchors: &mut HashMap<String, AnchorNode>,
    visiting: &mut HashSet<String>,
) -> Result<AnchorNode, Error> {
    match value {
        Value::Null(anchor) => {
            let node = Rc::new(AnchorValue::Null);
            if let Some(name) = anchor {
                anchors.insert(name.clone(), Rc::clone(&node));
            }
            Ok(node)
        }
        Value::Bool(boolean, anchor) => {
            let node = Rc::new(AnchorValue::Bool(*boolean));
            if let Some(name) = anchor {
                anchors.insert(name.clone(), Rc::clone(&node));
            }
            Ok(node)
        }
        Value::Number(number, anchor) => {
            let node = Rc::new(AnchorValue::Number(number.clone()));
            if let Some(name) = anchor {
                anchors.insert(name.clone(), Rc::clone(&node));
            }
            Ok(node)
        }
        Value::String(string, anchor) => {
            let node = Rc::new(AnchorValue::String(string.clone()));
            if let Some(name) = anchor {
                anchors.insert(name.clone(), Rc::clone(&node));
            }
            Ok(node)
        }
        Value::Sequence(sequence) => {
            if let Some(name) = &sequence.anchor {
                if !visiting.insert(name.clone()) {
                    return Err(error::new(ErrorImpl::MergeRecursion));
                }
            }
            let mut elements = Vec::with_capacity(sequence.elements.len());
            for item in &sequence.elements {
                elements.push(build_anchor_node(item, anchors, visiting)?);
            }
            let node = Rc::new(AnchorValue::Sequence(elements));
            if let Some(name) = &sequence.anchor {
                anchors.insert(name.clone(), Rc::clone(&node));
                visiting.remove(name);
            }
            Ok(node)
        }
        Value::Mapping(mapping) => {
            if let Some(name) = &mapping.anchor {
                if !visiting.insert(name.clone()) {
                    return Err(error::new(ErrorImpl::MergeRecursion));
                }
            }
            let mut entries = Vec::with_capacity(mapping.len());
            for (key, value) in mapping.iter() {
                let key_node = build_anchor_node(key, anchors, visiting)?;
                let value_node = build_anchor_node(value, anchors, visiting)?;
                entries.push((key_node, value_node));
            }
            let node = Rc::new(AnchorValue::Mapping(entries));
            if let Some(name) = &mapping.anchor {
                anchors.insert(name.clone(), Rc::clone(&node));
                visiting.remove(name);
            }
            Ok(node)
        }
        Value::Tagged(tagged) => {
            let value = build_anchor_node(&tagged.value, anchors, visiting)?;
            let node = Rc::new(AnchorValue::Tagged(AnchorTagged {
                tag: tagged.tag.clone(),
                value,
            }));
            Ok(node)
        }
        Value::Alias(name) => {
            if visiting.contains(name) {
                return Err(error::new(ErrorImpl::MergeRecursion));
            }
            anchors
                .get(name)
                .cloned()
                .ok_or_else(|| error::new(ErrorImpl::UnresolvedAlias))
        }
    }
}
