use crate::de::{Event, Progress, ScalarEvent, SequenceStartEvent, MappingStartEvent};
use crate::error::{self, Error, ErrorImpl, Result};
use crate::libyaml::error::Mark;
use crate::libyaml::parser::{Event as YamlEvent, Parser, Anchor};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;

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
}

impl<'input> Loader<'input> {
    pub fn new(progress: Progress<'input>) -> Result<Self> {
        let input = match progress {
            Progress::Str(s) => Cow::Borrowed(s.as_bytes()),
            Progress::Slice(bytes) => Cow::Borrowed(bytes),
            Progress::Read(mut rdr) => {
                let mut buffer = Vec::new();
                if let Err(io_error) = rdr.read_to_end(&mut buffer) {
                    return Err(error::new(ErrorImpl::Io(io_error)));
                }
                Cow::Owned(buffer)
            }
            Progress::Iterable(_) | Progress::Document(_) => {
                return Err(error::new(ErrorImpl::MoreThanOneDocument));
            }
            Progress::Fail(err) => return Err(error::shared(err)),
        };

        Ok(Loader {
            parser: Some(Parser::new(input)?),
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
        };

        loop {
            let (event, mark) = match parser.next() {
                Ok((event, mark)) => (event, mark),
                Err(err) => {
                    document.error = Some(Error::from(err).shared());
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
                YamlEvent::DocumentEnd => return Some(document),
                YamlEvent::Alias(alias) => match anchors.get(&alias) {
                    Some(id) => Event::Alias(*id),
                    None => {
                        document.error = Some(error::new(
                            ErrorImpl::UnknownAnchor(mark, alias)).shared());
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
                    Event::Scalar(ScalarEvent { anchor: anchor_name, value: scalar })
                }
                YamlEvent::SequenceStart(mut sequence_start) => {
                    let anchor_name = sequence_start.anchor.take().map(|a| {
                        let name = anchor_to_string(&a);
                        let id = anchors.len();
                        anchors.insert(a, id);
                        document.aliases.insert(id, document.events.len());
                        name
                    });
                    Event::SequenceStart(SequenceStartEvent { anchor: anchor_name, tag: sequence_start.tag })
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
                    Event::MappingStart(MappingStartEvent { anchor: anchor_name, tag: mapping_start.tag })
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
}
