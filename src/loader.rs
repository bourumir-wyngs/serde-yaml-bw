use crate::de::{Event, Progress, ScalarEvent, SequenceStartEvent, MappingStartEvent};
use crate::error::{self, ErrorImpl, Result};
use crate::libyaml::error::Mark;
use crate::libyaml::parser::{Event as YamlEvent, Parser, Anchor};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::fs;
use std::io::Read;

const RAM_SAFETY_MARGIN: f64 = 25.0;

/// Returns recommended allocatable RAM in bytes after applying the margin.
/// On non-Linux platforms this falls back to `None`.
fn safe_allocatable_ram(margin_percent: f64) -> Option<u64> {
    let meminfo = fs::read_to_string("/proc/meminfo").ok()?;
    for line in meminfo.lines() {
        if let Some(rest) = line.strip_prefix("MemAvailable:") {
            if let Some(kb) = rest.split_whitespace().next() {
                if let Ok(kb) = kb.parse::<u64>() {
                    let bytes = kb * 1024;
                    let margin = (bytes as f64 * margin_percent / 100.0) as u64;
                    return Some(bytes.saturating_sub(margin));
                }
            }
        }
    }
    None
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
        let input = match progress {
            Progress::Str(s) => Cow::Borrowed(s.as_bytes()),
            Progress::Slice(bytes) => Cow::Borrowed(bytes),
            Progress::Read(mut rdr) => {
                let limit = safe_allocatable_ram(RAM_SAFETY_MARGIN).unwrap_or(u64::MAX) as usize;
                let mut buffer = Vec::new();
                loop {
                    let mut chunk = [0u8; 8192];
                    match rdr.read(&mut chunk) {
                        Ok(0) => break,
                        Ok(n) => {
                            if buffer.len() + n > limit {
                                return Err(error::new(ErrorImpl::Message(
                                    "input exceeds available memory".into(),
                                    None,
                                )));
                            }
                            buffer.extend_from_slice(&chunk[..n]);
                        }
                        Err(io_error) => return Err(error::new(ErrorImpl::Io(io_error))),
                    }
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
