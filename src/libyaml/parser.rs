use crate::libyaml::cstr::{self, CStr};
use crate::libyaml::error::{Error as LibyamlError, Mark};
use crate::error::{self, Error, ErrorImpl, Result};
use crate::libyaml::tag::Tag;
use crate::libyaml::util::Owned;
use std::borrow::Cow;
use std::fmt::{self, Debug};
use std::mem::MaybeUninit;
use std::ptr::{addr_of_mut, NonNull};
use std::slice;
use std::io::{self, Read};
use unsafe_libyaml as sys;

pub(crate) struct Parser<'input> {
    pin: Owned<ParserPinned<'input>>,
}

struct ParserPinned<'input> {
    sys: sys::yaml_parser_t,
    input: Option<Cow<'input, [u8]>>,
    reader: Option<Box<dyn Read + 'input>>,
    read_error: Option<io::Error>,
}

#[derive(Debug)]
pub(crate) enum Event<'input> {
    StreamStart,
    StreamEnd,
    DocumentStart,
    DocumentEnd,
    Alias(Anchor),
    Scalar(Scalar<'input>),
    SequenceStart(SequenceStart),
    SequenceEnd,
    MappingStart(MappingStart),
    MappingEnd,
    /// Placeholder event for unknown or empty libyaml events
    Void,
}

pub(crate) struct Scalar<'input> {
    pub anchor: Option<Anchor>,
    pub tag: Option<Tag>,
    pub value: Box<[u8]>,
    pub style: ScalarStyle,
    pub repr: Option<&'input [u8]>,
}

#[derive(Debug)]
pub(crate) struct SequenceStart {
    pub anchor: Option<Anchor>,
    pub tag: Option<Tag>,
}

#[derive(Debug)]
pub(crate) struct MappingStart {
    pub anchor: Option<Anchor>,
    pub tag: Option<Tag>,
}

#[derive(Ord, PartialOrd, Eq, PartialEq, Hash)]
pub(crate) struct Anchor(pub(crate) Box<[u8]>);

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum ScalarStyle {
    Plain,
    SingleQuoted,
    DoubleQuoted,
    Literal,
    Folded,
}

impl<'input> Parser<'input> {
    pub fn new(input: Cow<'input, [u8]>) -> Result<Parser<'input>> {
        let owned = Owned::<ParserPinned>::new_uninit();
        let pin = unsafe {
            let parser = addr_of_mut!((*owned.ptr).sys);
            if sys::yaml_parser_initialize(parser).fail {
                return Err(Error::from(LibyamlError::parse_error(parser)));
            }
            sys::yaml_parser_set_encoding(parser, sys::YAML_UTF8_ENCODING);
            sys::yaml_parser_set_input_string(parser, input.as_ptr(), input.len() as u64);
            addr_of_mut!((*owned.ptr).input).write(Some(input));
            addr_of_mut!((*owned.ptr).reader).write(None);
            addr_of_mut!((*owned.ptr).read_error).write(None);
            Owned::assume_init(owned)
        };
        Ok(Parser { pin })
    }

    pub fn from_reader<R>(reader: R) -> Result<Parser<'input>>
    where
        R: Read + 'input,
    {
        unsafe fn read_handler(
            data: *mut std::os::raw::c_void,
            buffer: *mut u8,
            size: u64,
            size_read: *mut u64,
        ) -> i32 {
            unsafe {
                let pinned = &mut *(data as *mut ParserPinned);
                let reader = pinned.reader.as_mut().unwrap();
                let slice = std::slice::from_raw_parts_mut(buffer, size as usize);
                match reader.read(slice) {
                    Ok(len) => {
                        *size_read = len as u64;
                        1
                    }
                    Err(err) => {
                        pinned.read_error = Some(err);
                        *size_read = 0;
                        0
                    }
                }
            }
        }

        let owned = Owned::<ParserPinned>::new_uninit();
        let pin = unsafe {
            let parser = addr_of_mut!((*owned.ptr).sys);
            if sys::yaml_parser_initialize(parser).fail {
                return Err(Error::from(LibyamlError::parse_error(parser)));
            }
            sys::yaml_parser_set_encoding(parser, sys::YAML_UTF8_ENCODING);
            addr_of_mut!((*owned.ptr).reader).write(Some(Box::new(reader)));
            addr_of_mut!((*owned.ptr).read_error).write(None);
            let data = owned.ptr;
            sys::yaml_parser_set_input(parser, read_handler as sys::yaml_read_handler_t, data.cast());
            addr_of_mut!((*owned.ptr).input).write(None);
            Owned::assume_init(owned)
        };
        Ok(Parser { pin })
    }

    pub fn next(&mut self) -> Result<(Event<'input>, Mark)> {
        let mut event = MaybeUninit::<sys::yaml_event_t>::uninit();
        unsafe {
            if let Some(err) = (*self.pin.ptr).read_error.take() {
                return Err(error::new(ErrorImpl::Io(err)));
            }
            let parser = addr_of_mut!((*self.pin.ptr).sys);
            if (&*parser).error != sys::YAML_NO_ERROR {
                return Err(Error::from(LibyamlError::parse_error(parser)));
            }
            let event = event.as_mut_ptr();
            if sys::yaml_parser_parse(parser, event).fail {
                sys::yaml_event_delete(event);
                if let Some(err) = (*self.pin.ptr).read_error.take() {
                    return Err(error::new(ErrorImpl::Io(err)));
                }
                return Err(Error::from(LibyamlError::parse_error(parser)));
            }
            let ret = convert_event(&*event, &(*self.pin.ptr).input).map_err(error::new)?;
            let mark = Mark {
                sys: (*event).start_mark,
            };
            sys::yaml_event_delete(event);
            Ok((ret, mark))
        }
    }
}

unsafe fn convert_event<'input>(
    sys: &sys::yaml_event_t,
    input: &Option<Cow<'input, [u8]>>,
) -> std::result::Result<Event<'input>, ErrorImpl> {
    match sys.type_ {
        sys::YAML_STREAM_START_EVENT => Ok(Event::StreamStart),
        sys::YAML_STREAM_END_EVENT => Ok(Event::StreamEnd),
        sys::YAML_DOCUMENT_START_EVENT => Ok(Event::DocumentStart),
        sys::YAML_DOCUMENT_END_EVENT => Ok(Event::DocumentEnd),
        sys::YAML_ALIAS_EVENT => match unsafe { optional_anchor(sys.data.alias.anchor)? } {
            Some(anchor) => Ok(Event::Alias(anchor)),
            None => Err(ErrorImpl::UnknownAnchor(
                Mark {
                    sys: sys.start_mark,
                },
                Anchor(Box::from(&b""[..])),
            )),
        },
        sys::YAML_SCALAR_EVENT => Ok(Event::Scalar(Scalar {
            anchor: unsafe { optional_anchor(sys.data.scalar.anchor) }?,
            tag: unsafe { optional_tag(sys.data.scalar.tag) }?,
            value: Box::from(unsafe {
                slice::from_raw_parts(sys.data.scalar.value, sys.data.scalar.length as usize)
            }),
            style: match unsafe { sys.data.scalar.style } {
                sys::YAML_PLAIN_SCALAR_STYLE => ScalarStyle::Plain,
                sys::YAML_SINGLE_QUOTED_SCALAR_STYLE => ScalarStyle::SingleQuoted,
                sys::YAML_DOUBLE_QUOTED_SCALAR_STYLE => ScalarStyle::DoubleQuoted,
                sys::YAML_LITERAL_SCALAR_STYLE => ScalarStyle::Literal,
                sys::YAML_FOLDED_SCALAR_STYLE => ScalarStyle::Folded,
                // Treat any unrecognized style as plain to avoid panicking
                sys::YAML_ANY_SCALAR_STYLE | _ => ScalarStyle::Plain,
            },
            repr: if let Some(Cow::Borrowed(input)) = input {
                Some(&input[sys.start_mark.index as usize..sys.end_mark.index as usize])
            } else {
                None
            },
        })),
        sys::YAML_SEQUENCE_START_EVENT => Ok(Event::SequenceStart(SequenceStart {
            anchor: unsafe { optional_anchor(sys.data.sequence_start.anchor) }?,
            tag: unsafe { optional_tag(sys.data.sequence_start.tag) }?,
        })),
        sys::YAML_SEQUENCE_END_EVENT => Ok(Event::SequenceEnd),
        sys::YAML_MAPPING_START_EVENT => Ok(Event::MappingStart(MappingStart {
            anchor: unsafe { optional_anchor(sys.data.mapping_start.anchor) }?,
            tag: unsafe { optional_tag(sys.data.mapping_start.tag) }?,
        })),
        sys::YAML_MAPPING_END_EVENT => Ok(Event::MappingEnd),
        // Unknown or empty events should not cause a panic
        sys::YAML_NO_EVENT => Ok(Event::Void),
        _ => Ok(Event::Void),
    }
}

unsafe fn optional_anchor(anchor: *const u8) -> std::result::Result<Option<Anchor>, ErrorImpl> {
    let ptr = match NonNull::new(anchor as *mut i8) {
        Some(p) => p,
        None => return Ok(None),
    };
    let cstr = unsafe { CStr::from_ptr(ptr) };
    cstr
        .to_bytes()
        .map(|bytes| Some(Anchor(Box::from(bytes))))
        .map_err(|_| ErrorImpl::TagError)
}

unsafe fn optional_tag(tag: *const u8) -> std::result::Result<Option<Tag>, ErrorImpl> {
    let ptr = match NonNull::new(tag as *mut i8) {
        Some(p) => p,
        None => return Ok(None),
    };
    let cstr = unsafe { CStr::from_ptr(ptr) };
    cstr
        .to_bytes()
        .map(|bytes| Some(Tag(Box::from(bytes))))
        .map_err(|_| ErrorImpl::TagError)
}

impl Debug for Scalar<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let Scalar {
            anchor,
            tag,
            value,
            style,
            repr: _,
        } = self;

        struct LossySlice<'a>(&'a [u8]);

        impl Debug for LossySlice<'_> {
            fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                cstr::debug_lossy(self.0, formatter)
            }
        }

        formatter
            .debug_struct("Scalar")
            .field("anchor", anchor)
            .field("tag", tag)
            .field("value", &LossySlice(value))
            .field("style", style)
            .finish()
    }
}

impl Debug for Anchor {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        cstr::debug_lossy(&self.0, formatter)
    }
}

impl Drop for ParserPinned<'_> {
    fn drop(&mut self) {
        unsafe { sys::yaml_parser_delete(&raw mut self.sys) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::borrow::Cow;
    use std::io::{self, Read};

    #[test]
    fn repeated_parse_errors_do_not_leak() {
        let yaml = ":";
        for _ in 0..100 {
            let mut parser = Parser::new(Cow::Borrowed(yaml.as_bytes())).unwrap();
            loop {
                match parser.next() {
                    Ok(_) => continue,
                    Err(_) => break,
                }
            }
        }
    }

    struct FailingReader;

    impl Read for FailingReader {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::Other, "fail"))
        }
    }

    #[test]
    fn read_error_is_propagated() {
        let mut parser = Parser::from_reader(FailingReader).unwrap();
        let err = parser.next().unwrap_err();
        assert_eq!(err.to_string(), "fail");
    }
}
