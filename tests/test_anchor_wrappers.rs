use serde::Serialize;
use std::rc::Rc;
use std::sync::{Arc, Weak as ArcWeak};

use serde_yaml_bw::{
    to_string, to_string_multi, ArcAnchor, ArcWeakAnchor, RcAnchor, RcWeakAnchor, SerializerBuilder,
    SerializerOptions,
};

#[test]
fn rc_anchor_emits_aliases() {
    #[derive(Serialize, Clone)]
    struct Node {
        name: String,
    }

    let n1 = Rc::new(Node {
        name: "node one".to_string(),
    });
    let n2 = Rc::new(Node {
        name: "node two".to_string(),
    });

    let wrapped1 = RcAnchor(n1.clone());
    let wrapped2 = RcAnchor(n2.clone());

    let data = vec![
        wrapped1.clone(),
        wrapped1.clone(),
        wrapped1,
        wrapped2.clone(),
        RcAnchor(n1),
        wrapped2,
    ];

    let yaml = to_string(&data).expect("serialization should succeed");
    let expected = "- &a1\n  name: node one\n- *a1\n- *a1\n- &a2\n  name: node two\n- *a1\n- *a2\n";
    assert_eq!(yaml, expected);
}

#[test]
fn rc_weak_anchor_turns_into_null_when_dropped() {
    #[derive(Serialize)]
    struct Payload {
        value: u32,
    }

    let orphan = {
        let strong = Rc::new(Payload { value: 7 });
        RcWeakAnchor(Rc::downgrade(&strong))
    };

    let yaml = to_string(&vec![orphan]).expect("serialization should succeed");
    assert_eq!(yaml, "- null\n");
}

#[test]
fn custom_anchor_name_generator_is_used() {
    #[derive(Serialize)]
    struct Node {
        id: u8,
    }

    let item = Arc::new(Node { id: 7 });

    let mut options = SerializerOptions::default();
    options.set_anchor_name_fn(|idx| format!("node_{idx}"));

    let mut buffer = Vec::new();
    {
        let mut serializer = SerializerBuilder::default()
            .options(options.clone())
            .build(&mut buffer)
            .expect("serializer should build");
        let payload = vec![ArcAnchor(item.clone()), ArcAnchor(item.clone())];
        payload
            .serialize(&mut serializer)
            .expect("serialization should succeed");
    }

    let yaml = String::from_utf8(buffer).expect("valid UTF-8");
    assert_eq!(yaml, "- &node_0\n  id: 7\n- *node_0\n");
}

#[test]
fn arc_weak_anchor_aliases_existing_anchor() {
    #[derive(Serialize)]
    struct Node {
        id: u8,
    }

    #[derive(Serialize)]
    struct Payload {
        strong: ArcAnchor<Node>,
        weak: ArcWeakAnchor<Node>,
    }

    let strong = Arc::new(Node { id: 1 });
    let weak: ArcWeak<Node> = Arc::downgrade(&strong);
    let yaml = to_string(&Payload {
        strong: ArcAnchor(strong.clone()),
        weak: ArcWeakAnchor(weak),
    })
    .expect("serialization should succeed");
    assert_eq!(yaml, "strong: &a1\n  id: 1\nweak: *a1\n");
}

#[test]
fn multi_document_helpers_reset_anchor_state() {
    #[derive(Serialize)]
    struct Node {
        label: &'static str,
    }

    let shared = Rc::new(Node { label: "shared" });
    let docs = vec![
        vec![RcAnchor(shared.clone()), RcAnchor(shared.clone())],
        vec![RcAnchor(shared.clone()), RcAnchor(shared.clone())],
    ];

    let yaml = to_string_multi(&docs).expect("serialization should succeed");
    let expected = "- &a1\n  label: shared\n- *a1\n---\n- &a1\n  label: shared\n- *a1\n";
    assert_eq!(yaml, expected);
}
