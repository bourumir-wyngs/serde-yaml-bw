use crate::de::{Event, MappingStartEvent, Progress, ScalarEvent, SequenceStartEvent};
use crate::error::{self, ErrorImpl, Result};
use crate::libyaml::error::Mark;
use crate::libyaml::parser::{Anchor, Event as YamlEvent, Parser};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;
use sysinfo::System;

// Safety margin in percents (if we take all available memory, we may crash anyway).
const RAM_SAFETY_MARGIN: u64 = 5;

/// Returns the free RAM in bytes after applying the margin.
///
/// The margin is computed from currently available memory, reserving the given
/// percentage to remain unused.
fn safe_allocatable_ram(margin_percent: u64) -> u64 {
    let mut sys = System::new();
    sys.refresh_memory();
    let avail = sys.available_memory();
    let margin = avail * margin_percent / 100;
    avail.saturating_sub(margin)
}

fn anchor_to_string(anchor: &Anchor) -> String {
    String::from_utf8_lossy(&anchor.0).into_owned()
}

pub(crate) struct Loader<'input> {
    parser: Option<Parser<'input>>,
    document_count: usize,
}

pub(crate) struct Document<'input> {
    pub events: Vec<(Event<'input>, Mark)>,
    pub error: Option<Arc<ErrorImpl>>,
    /// Map from alias id to index in events.
    pub aliases: BTreeMap<usize, usize>,
    /// Location of the explicit document end marker if present.
    pub end_mark: Option<Mark>,
}

impl<'input> Loader<'input> {
    pub fn new(progress: Progress<'input>) -> Result<Self> {
        let parser = match progress {
            Progress::Str(s) => Parser::new(Cow::Borrowed(s.as_bytes()))?,
            Progress::Slice(bytes) => Parser::new(Cow::Borrowed(bytes))?,
            Progress::Read(rdr) => Parser::from_reader(rdr)?,
            Progress::Iterable(_) | Progress::Document(_) => {
                return Err(error::new(ErrorImpl::MoreThanOneDocument));
            }
            Progress::Fail(err) => return Err(error::shared(err)),
        };

        Ok(Loader {
            parser: Some(parser),
            document_count: 0,
        })
    }

    pub fn next_document(&mut self) -> Option<Document<'input>> {
        let Some(parser) = &mut self.parser else {
            return None;
        };

        let first = self.document_count == 0;
        self.document_count += 1;

        let mut anchors = BTreeMap::new();
        let mut document = Document {
            events: Vec::new(),
            error: None,
            aliases: BTreeMap::new(),
            end_mark: None,
        };

        let mut seen = false;
        loop {
            let (event, mark) = match parser.next() {
                Ok((event, mark)) => {
                    seen = true;
                    (event, mark)
                }
                Err(err) => {
                    if !seen {
                        self.parser = None;
                        return None;
                    }
                    document.error = Some(err.shared());
                    return Some(document);
                }
            };
            let event = match event {
                YamlEvent::StreamStart => continue,
                YamlEvent::StreamEnd => {
                    self.parser = None;
                    return if first {
                        if document.events.is_empty() {
                            document.events.push((Event::Void, mark));
                        }
                        Some(document)
                    } else {
                        None
                    };
                }
                YamlEvent::DocumentStart => continue,
                YamlEvent::DocumentEnd => {
                    document.end_mark = Some(mark);
                    return Some(document);
                }
                YamlEvent::Alias(alias) => match anchors.get(&alias) {
                    Some(id) => Event::Alias(*id),
                    None => {
                        document.error =
                            Some(error::new(ErrorImpl::UnknownAnchor(mark, alias)).shared());
                        return Some(document);
                    }
                },
                YamlEvent::Scalar(mut scalar) => {
                    let anchor_name = scalar.anchor.take().map(|a| {
                        let name = anchor_to_string(&a);
                        let id = anchors.len();
                        anchors.insert(a, id);
                        document.aliases.insert(id, document.events.len());
                        name
                    });
                    Event::Scalar(ScalarEvent {
                        anchor: anchor_name,
                        value: scalar,
                    })
                }
                YamlEvent::SequenceStart(mut sequence_start) => {
                    let anchor_name = sequence_start.anchor.take().map(|a| {
                        let name = anchor_to_string(&a);
                        let id = anchors.len();
                        anchors.insert(a, id);
                        document.aliases.insert(id, document.events.len());
                        name
                    });
                    Event::SequenceStart(SequenceStartEvent {
                        anchor: anchor_name,
                        tag: sequence_start.tag,
                    })
                }
                YamlEvent::SequenceEnd => Event::SequenceEnd,
                YamlEvent::MappingStart(mut mapping_start) => {
                    let anchor_name = mapping_start.anchor.take().map(|a| {
                        let name = anchor_to_string(&a);
                        let id = anchors.len();
                        anchors.insert(a, id);
                        document.aliases.insert(id, document.events.len());
                        name
                    });
                    Event::MappingStart(MappingStartEvent {
                        anchor: anchor_name,
                        tag: mapping_start.tag,
                    })
                }
                YamlEvent::MappingEnd => Event::MappingEnd,
                YamlEvent::Void => Event::Void,
            };
            document.events.push((event, mark));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Read};

    #[test]
    fn anchored_scalar_event_keeps_anchor() {
        let yaml = "a: &id 1\nb: *id\n";
        let mut loader = Loader::new(Progress::Str(yaml)).unwrap();
        let document = loader.next_document().unwrap();
        let mut found = false;
        for (event, _) in &document.events {
            if let Event::Scalar(scalar) = event {
                if let Some(name) = &scalar.anchor {
                    assert_eq!(name, "id");
                    found = true;
                }
            }
        }
        assert!(found, "anchored scalar not found");
    }

    struct EndlessReader;

    impl Read for EndlessReader {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            for byte in buf.iter_mut() {
                *byte = b'a';
            }
            Ok(buf.len())
        }
    }

    // Running memory to exhaustion may hit the CI server with many other jobs hard, so please 
    // run this test only on your workstation.
    #[test]
    #[ignore]
    fn test_loader_with_indefinite_input() {
        let endless_reader = EndlessReader;
        let loader_result = Loader::new(Progress::Read(Box::new(endless_reader)));

        assert!(loader_result.is_err(), "Expected memory limit error");
        if let Err(err) = loader_result {
            if !err.to_string().contains("exceeds permissible memory") {
                panic!("Unexpected error {}", err);
            }
            println!("{}", err);
        } else {
            panic!("Read infinite reader till the end");
        }
    }
}
