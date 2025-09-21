use crate::libyaml;
use crate::libyaml::util::Owned;
use std::ffi::{c_void, CString};
use std::io;
use std::mem::MaybeUninit;
use std::ptr::{self, addr_of_mut};
use std::slice;
use unsafe_libyaml_norway as sys;

#[derive(Debug)]
pub(crate) enum Error {
    Libyaml(libyaml::error::Error),
    Io(io::Error),
}

pub(crate) struct Emitter<W>
where
    W: io::Write,
{
    pin: Owned<EmitterPinned<W>>,
}

struct EmitterPinned<W>
where
    W: io::Write,
{
    sys: sys::yaml_emitter_t,
    write: Option<W>,
    write_error: Option<io::Error>,
}

#[derive(Debug)]
pub(crate) enum Event<'a> {
    StreamStart,
    StreamEnd,
    DocumentStart,
    DocumentEnd,
    Alias(String),
    Scalar(Scalar<'a>),
    SequenceStart(Sequence),
    SequenceEnd,
    MappingStart(Mapping),
    MappingEnd,
}

#[derive(Debug)]
pub(crate) struct Scalar<'a> {
    pub anchor: Option<String>,
    pub tag: Option<String>,
    pub value: &'a str,
    pub style: ScalarStyle,
}

#[derive(Debug, Clone, Copy)]
pub enum ScalarStyle {
    Any,
    Plain,
    SingleQuoted,
    Literal,
}

#[derive(Debug, Clone, Copy)]
/// Style of an emitted YAML sequence.
pub enum SequenceStyle {
    /// Let the emitter choose the most appropriate style.
    Any,
    /// Emit the sequence in block style.
    Block,
    /// Emit the sequence in flow style.
    Flow,
}

#[derive(Debug)]
pub(crate) struct Sequence {
    pub anchor: Option<String>,
    pub tag: Option<String>,
    pub style: SequenceStyle,
}

impl Sequence {
    /// Create a sequence event with the specified style.
    pub fn with_style(style: SequenceStyle) -> Self {
        Sequence {
            anchor: None,
            tag: None,
            style,
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum MappingStyle {
    Any,
    Block,
    Flow,
}

#[derive(Debug)]
pub(crate) struct Mapping {
    pub anchor: Option<String>,
    pub tag: Option<String>,
    pub style: MappingStyle,
}

impl<W> Emitter<W>
where
    W: io::Write,
{
    pub fn new(write: W, width: i32, indent: i32) -> Result<Emitter<W>, Error> {
        let owned = Owned::<EmitterPinned<W>>::new_uninit();
        // SAFETY: `owned.ptr` points to uninitialized memory for a libyaml
        // emitter. We initialize it using the libyaml API before any use.
        let pin = unsafe {
            let emitter = addr_of_mut!((*owned.ptr).sys);
            if sys::yaml_emitter_initialize(emitter).fail {
                return Err(Error::Libyaml(libyaml::error::Error::emit_error(emitter)));
            }
            sys::yaml_emitter_set_unicode(emitter, true);
            sys::yaml_emitter_set_width(emitter, width);
            sys::yaml_emitter_set_indent(emitter, indent);
            addr_of_mut!((*owned.ptr).write).write(Some(write));
            addr_of_mut!((*owned.ptr).write_error).write(None);
            sys::yaml_emitter_set_output(emitter, write_handler::<W>, owned.ptr.cast());
            Owned::assume_init(owned)
        };
        Ok(Emitter { pin })
    }

    pub fn emit(&mut self, event: Event) -> Result<(), Error> {
        let mut sys_event = MaybeUninit::<sys::yaml_event_t>::uninit();
        let sys_event = sys_event.as_mut_ptr();
        // SAFETY: `self.pin` holds a valid libyaml emitter and `sys_event` points
        // to uninitialized memory which we hand to libyaml to populate.
        unsafe {
            let emitter = addr_of_mut!((*self.pin.ptr).sys);
            let initialize_status = match event {
                Event::StreamStart => {
                    sys::yaml_stream_start_event_initialize(sys_event, sys::YAML_UTF8_ENCODING)
                }
                Event::StreamEnd => sys::yaml_stream_end_event_initialize(sys_event),
                Event::DocumentStart => {
                    let version_directive = ptr::null_mut();
                    let tag_directives_start = ptr::null_mut();
                    let tag_directives_end = ptr::null_mut();
                    let implicit = true;
                    sys::yaml_document_start_event_initialize(
                        sys_event,
                        version_directive,
                        tag_directives_start,
                        tag_directives_end,
                        implicit,
                    )
                }
                Event::DocumentEnd => {
                    let implicit = true;
                    sys::yaml_document_end_event_initialize(sys_event, implicit)
                }
                Event::Alias(anchor) => {
                    let anchor_c = CString::new(anchor).ok();
                    let anchor = anchor_c
                        .as_ref()
                        .map_or(ptr::null(), |cstr| cstr.as_ptr() as *const u8);
                    sys::yaml_alias_event_initialize(sys_event, anchor)
                }
                Event::Scalar(scalar) => {
                    let anchor_c = scalar
                        .anchor
                        .as_ref()
                        .and_then(|a| CString::new(a.as_str()).ok());
                    let anchor = anchor_c
                        .as_ref()
                        .map_or(ptr::null(), |cstr| cstr.as_ptr() as *const u8);
                    let tag_c = scalar
                        .tag
                        .as_ref()
                        .and_then(|tag| CString::new(tag.as_str()).ok());
                    let tag = tag_c
                        .as_ref()
                        .map_or(ptr::null(), |cstr| cstr.as_ptr() as *const u8);
                    let value = scalar.value.as_ptr();
                    let length = scalar.value.len() as i32;
                    let plain_implicit = tag.is_null();
                    let quoted_implicit = tag.is_null();
                    let style = match scalar.style {
                        ScalarStyle::Any => sys::YAML_ANY_SCALAR_STYLE,
                        ScalarStyle::Plain => sys::YAML_PLAIN_SCALAR_STYLE,
                        ScalarStyle::SingleQuoted => sys::YAML_SINGLE_QUOTED_SCALAR_STYLE,
                        ScalarStyle::Literal => sys::YAML_LITERAL_SCALAR_STYLE,
                    };
                    sys::yaml_scalar_event_initialize(
                        sys_event,
                        anchor,
                        tag,
                        value,
                        length,
                        plain_implicit,
                        quoted_implicit,
                        style,
                    )
                }
                Event::SequenceStart(sequence) => {
                    let anchor_c = sequence
                        .anchor
                        .as_ref()
                        .and_then(|a| CString::new(a.as_str()).ok());
                    let anchor = anchor_c
                        .as_ref()
                        .map_or(ptr::null(), |cstr| cstr.as_ptr() as *const u8);
                    let tag_c = sequence
                        .tag
                        .as_ref()
                        .and_then(|tag| CString::new(tag.as_str()).ok());
                    let tag = tag_c
                        .as_ref()
                        .map_or(ptr::null(), |cstr| cstr.as_ptr() as *const u8);
                    let implicit = tag.is_null();
                    let style = match sequence.style {
                        SequenceStyle::Any => sys::YAML_ANY_SEQUENCE_STYLE,
                        SequenceStyle::Block => sys::YAML_BLOCK_SEQUENCE_STYLE,
                        SequenceStyle::Flow => sys::YAML_FLOW_SEQUENCE_STYLE,
                    };
                    sys::yaml_sequence_start_event_initialize(
                        sys_event, anchor, tag, implicit, style,
                    )
                }
                Event::SequenceEnd => sys::yaml_sequence_end_event_initialize(sys_event),
                Event::MappingStart(mapping) => {
                    let anchor_c = mapping
                        .anchor
                        .as_ref()
                        .and_then(|a| CString::new(a.as_str()).ok());
                    let anchor = anchor_c
                        .as_ref()
                        .map_or(ptr::null(), |cstr| cstr.as_ptr() as *const u8);
                    let tag_c = mapping
                        .tag
                        .as_ref()
                        .and_then(|tag| CString::new(tag.as_str()).ok());
                    let tag = tag_c
                        .as_ref()
                        .map_or(ptr::null(), |cstr| cstr.as_ptr() as *const u8);
                    let implicit = tag.is_null();
                    let style = match mapping.style {
                        MappingStyle::Any => sys::YAML_ANY_MAPPING_STYLE,
                        MappingStyle::Block => sys::YAML_BLOCK_MAPPING_STYLE,
                        MappingStyle::Flow => sys::YAML_FLOW_MAPPING_STYLE,
                    };
                    sys::yaml_mapping_start_event_initialize(
                        sys_event, anchor, tag, implicit, style,
                    )
                }
                Event::MappingEnd => sys::yaml_mapping_end_event_initialize(sys_event),
            };
            if initialize_status.fail {
                return Err(Error::Libyaml(libyaml::error::Error::emit_error(emitter)));
            }
            if sys::yaml_emitter_emit(emitter, sys_event).fail {
                return Err(self.error());
            }
        }
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), Error> {
        // SAFETY: `self.pin` contains a valid emitter and libyaml requires
        // flushing through its C API.
        unsafe {
            let emitter = addr_of_mut!((*self.pin.ptr).sys);
            if sys::yaml_emitter_flush(emitter).fail {
                return Err(self.error());
            }
        }
        Ok(())
    }

    pub fn into_inner(self) -> Result<W, Error> {
        // SAFETY: `self.pin.ptr` uniquely owns the writer; taking it out and
        // returning it is safe and leaves the emitter in a dropped state.
        unsafe {
            match (*self.pin.ptr).write.take() {
                Some(writer) => Ok(writer),
                None => Err(Error::Io(io::Error::new(
                    io::ErrorKind::Other,
                    "emitter writer missing",
                ))),
            }
        }
    }

    fn error(&mut self) -> Error {
        // SAFETY: `self.pin.ptr` is valid while `self` is alive. We only read the
        // emitter fields to construct an error value.
        let emitter = unsafe { &mut *self.pin.ptr };
        if let Some(write_error) = emitter.write_error.take() {
            Error::Io(write_error)
        } else {
            // SAFETY: The emitter pointer is valid and libyaml expects a pointer to
            // an initialized emitter when extracting error information.
            Error::Libyaml(unsafe { libyaml::error::Error::emit_error(&raw const emitter.sys) })
        }
    }
}

// SAFETY: Called by libyaml with the pointer previously provided in
// `yaml_emitter_set_output`. Pointers and sizes are guaranteed valid by libyaml.
unsafe fn write_handler<W>(data: *mut c_void, buffer: *mut u8, size: u64) -> i32
where
    W: io::Write,
{
    let data = data.cast::<EmitterPinned<W>>();
    // SAFETY: `data` points to our `EmitterPinned` provided in `new`.
    let ptr = unsafe { &mut *data };
    match ptr.write.as_mut() {
        Some(writer) => match io::Write::write_all(writer, unsafe {
            // SAFETY: `buffer` is valid for `size` bytes as promised by libyaml.
            slice::from_raw_parts(buffer, size as usize)
        }) {
            Ok(()) => 1,
            Err(err) => {
                ptr.write_error = Some(err);
                0
            }
        },
        None => {
            ptr.write_error = Some(io::Error::new(
                io::ErrorKind::Other,
                "emitter writer missing",
            ));
            0
        }
    }
}

impl<W> Drop for EmitterPinned<W>
where
    W: io::Write,
{
    fn drop(&mut self) {
        // SAFETY: The emitter was initialized by libyaml and must be cleaned up
        // with `yaml_emitter_delete`.
        unsafe { sys::yaml_emitter_delete(&raw mut self.sys) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequence_with_style_flow() {
        let mut out = Vec::new();
        let mut emitter = Emitter::new(&mut out, -1, 2).unwrap();

        emitter.emit(Event::StreamStart).unwrap();
        emitter.emit(Event::DocumentStart).unwrap();
        emitter
            .emit(Event::SequenceStart(Sequence::with_style(SequenceStyle::Flow)))
            .unwrap();
        for value in ["1", "2", "3"] {
            emitter
                .emit(Event::Scalar(Scalar {
                    anchor: None,
                    tag: None,
                    value,
                    style: ScalarStyle::Any,
                }))
                .unwrap();
        }
        emitter.emit(Event::SequenceEnd).unwrap();
        emitter.emit(Event::DocumentEnd).unwrap();
        emitter.emit(Event::StreamEnd).unwrap();
        drop(emitter);

        assert_eq!(out, b"[1, 2, 3]\n");
    }
}
