use crate::de::{Event, MappingStartEvent, Progress, ScalarEvent, SequenceStartEvent};
use crate::error::{self, ErrorImpl, Result, ScanError as PreScanError};
use crate::libyaml::error::Mark;
use crate::libyaml::parser::{Anchor, Event as YamlEvent, Parser};
use crate::budget::check_yaml_budget;
use std::borrow::Cow;
use std::collections::HashMap;
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
    pub aliases: Vec<usize>,
    /// Location of the explicit document end marker if present.
    pub end_mark: Option<Mark>,
}

impl<'input> Loader<'input> {
    pub fn new(
        progress: Progress<'input>,
        options: &crate::de::DeserializerOptions,
    ) -> Result<Self> {
        let parser = match progress {
            Progress::Str(s) => {
                let bytes = s.as_bytes();
                if let Some(b) = &options.budget {
                    match check_yaml_budget(s, b) {
                        Ok(rep) => {
                            if let Some(breach) = rep.breached {
                                return Err(error::new(ErrorImpl::BudgetExceeded(breach)));
                            }
                        }
                        Err(se) => {
                            let pse: PreScanError = (&se).into();
                            return Err(error::new(ErrorImpl::PreScan(pse)));
                        }
                    }
                }
                Parser::new(Cow::Borrowed(bytes))?
            }
            Progress::Slice(bytes) => {
                if let Some(b) = &options.budget {
                    if let Ok(s) = std::str::from_utf8(bytes) {
                        match check_yaml_budget(s, b) {
                            Ok(rep) => {
                                if let Some(breach) = rep.breached {
                                    return Err(error::new(ErrorImpl::BudgetExceeded(breach)));
                                }
                            }
                            Err(se) => {
                                let pse: PreScanError = (&se).into();
                                return Err(error::new(ErrorImpl::PreScan(pse)));
                            }
                        }
                    }
                }
                Parser::new(Cow::Borrowed(bytes))?
            }
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

        let mut anchors = HashMap::new();
        let mut document = Document {
            events: Vec::new(),
            error: None,
            aliases: Vec::new(),
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
                        document.aliases.push(document.events.len());
                        name
                    });
                    let raw = String::from_utf8_lossy(&scalar.value).into_owned();
                    Event::Scalar(ScalarEvent {
                        anchor: anchor_name,
                        value: scalar,
                        raw,
                    })
                }
                YamlEvent::SequenceStart(mut sequence_start) => {
                    let anchor_name = sequence_start.anchor.take().map(|a| {
                        let name = anchor_to_string(&a);
                        let id = anchors.len();
                        anchors.insert(a, id);
                        document.aliases.push(document.events.len());
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
                        document.aliases.push(document.events.len());
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

    #[test]
    fn anchored_scalar_event_keeps_anchor() {
        let yaml = "a: &id 1\nb: *id\n";
        let mut loader = Loader::new(
            Progress::Str(yaml),
            &crate::de::DeserializerOptions::default(),
        )
        .unwrap();
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

    #[test]
    fn anchored_sequence_event_keeps_anchor() {
        let yaml = "a: &id [1, 2]\nb: *id\n";
        let mut loader = Loader::new(
            Progress::Str(yaml),
            &crate::de::DeserializerOptions::default(),
        )
        .unwrap();
        let document = loader.next_document().unwrap();
        let mut found = false;
        for (event, _) in &document.events {
            if let Event::SequenceStart(sequence) = event {
                if let Some(name) = &sequence.anchor {
                    assert_eq!(name, "id");
                    found = true;
                }
            }
        }
        assert!(found, "anchored sequence not found");
    }

    #[test]
    fn anchored_mapping_event_keeps_anchor() {
        let yaml = "a: &id {b: 1}\nc: *id\n";
        let mut loader = Loader::new(
            Progress::Str(yaml),
            &crate::de::DeserializerOptions::default(),
        )
        .unwrap();
        let document = loader.next_document().unwrap();
        let mut found = false;
        for (event, _) in &document.events {
            if let Event::MappingStart(mapping) = event {
                if let Some(name) = &mapping.anchor {
                    assert_eq!(name, "id");
                    found = true;
                }
            }
        }
        assert!(found, "anchored mapping not found");
    }
}
