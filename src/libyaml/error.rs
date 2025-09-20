use crate::libyaml::cstr::{self, CStr};
use std::fmt::{self, Debug, Display, Write as _};
use std::ptr::NonNull;
use std::str;
use unsafe_libyaml_norway as sys;

pub(crate) struct Error {
    kind: sys::yaml_error_type_t,
    problem: Box<[u8]>,
    problem_offset: u64,
    problem_mark: Mark,
    context: Option<Box<[u8]>>,
    context_mark: Mark,
}

// `problem` must point to a valid NUL-terminated C string provided by
// libyaml. The caller ensures the pointer is non-null and the data remains
// alive for the duration of the call.
fn define_string(problem: NonNull<i8>) -> Box<[u8]> {
    const EMPTY_Z: [u8; 1] = [0];
    // SAFETY: `problem` satisfies the invariants described above, so creating a
    // `CStr` and reading its bytes is sound.
    let cstr = unsafe { CStr::from_ptr(problem) };
    Box::from(cstr.to_bytes().unwrap_or(&EMPTY_Z))
}

impl Error {
    // SAFETY: `parser` must be a valid pointer to an initialized libyaml parser
    // struct. The data it references must remain alive while we read from it.
    pub unsafe fn parse_error(parser: *const sys::yaml_parser_t) -> Self {
        // SAFETY: All reads from `parser` are valid because the caller ensures it
        // points to a properly initialized `yaml_parser_t`.
        unsafe {
            Error {
                kind: (&*parser).error,
                problem: match NonNull::new((&*parser).problem.cast_mut()) {
                    Some(problem) => define_string(problem),
                    None => Box::from(&b"libyaml parser failed but there is no error"[..]),
                },
                problem_offset: (&*parser).problem_offset,
                problem_mark: Mark {
                    sys: (&*parser).problem_mark,
                },
                context: match NonNull::new((&*parser).context.cast_mut()) {
                    Some(context) => Some(define_string(context)),
                    None => None,
                },
                context_mark: Mark {
                    sys: (&*parser).context_mark,
                },
            }
        }
    }

    // SAFETY: `emitter` must be a valid pointer to an initialized libyaml
    // emitter. The referenced data must outlive this function.
    pub unsafe fn emit_error(emitter: *const sys::yaml_emitter_t) -> Self {
        // SAFETY: All reads from `emitter` are valid because the caller ensures it
        // points to a properly initialized `yaml_emitter_t`.
        unsafe {
            Error {
                kind: (&*emitter).error,
                problem: match NonNull::new((&*emitter).problem.cast_mut()) {
                    Some(problem) => define_string(problem),
                    None => Box::from(&b"libyaml emitter failed but there is no error"[..]),
                },
                problem_offset: 0,
                problem_mark: Mark::default(),
                context: None,
                context_mark: Mark::default(),
            }
        }
    }

    pub fn mark(&self) -> Mark {
        self.problem_mark
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        display_lossy(&self.problem, formatter)?;
        if self.problem_mark.sys.line != 0 || self.problem_mark.sys.column != 0 {
            write!(formatter, " at {}", self.problem_mark)?;
        } else if self.problem_offset != 0 {
            write!(formatter, " at position {}", self.problem_offset)?;
        }
        if let Some(context) = &self.context {
            formatter.write_str(", ")?;
            display_lossy(context, formatter)?;
            if (self.context_mark.sys.line != 0 || self.context_mark.sys.column != 0)
                && (self.context_mark.sys.line != self.problem_mark.sys.line
                    || self.context_mark.sys.column != self.problem_mark.sys.column)
            {
                write!(formatter, " at {}", self.context_mark)?;
            }
        }
        Ok(())
    }
}

impl Debug for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let mut formatter = formatter.debug_struct("Error");
        if let Some(kind) = match self.kind {
            sys::YAML_MEMORY_ERROR => Some("MEMORY"),
            sys::YAML_READER_ERROR => Some("READER"),
            sys::YAML_SCANNER_ERROR => Some("SCANNER"),
            sys::YAML_PARSER_ERROR => Some("PARSER"),
            sys::YAML_COMPOSER_ERROR => Some("COMPOSER"),
            sys::YAML_WRITER_ERROR => Some("WRITER"),
            sys::YAML_EMITTER_ERROR => Some("EMITTER"),
            _ => None,
        } {
            formatter.field("kind", &format_args!("{}", kind));
        }
        struct DebugLossy<'a>(&'a [u8]);
        impl Debug for DebugLossy<'_> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                cstr::debug_lossy(self.0, f)
            }
        }
        formatter.field("problem", &DebugLossy(&self.problem));
        if self.problem_mark.sys.line != 0 || self.problem_mark.sys.column != 0 {
            formatter.field("problem_mark", &self.problem_mark);
        } else if self.problem_offset != 0 {
            formatter.field("problem_offset", &self.problem_offset);
        }
        if let Some(context) = &self.context {
            formatter.field("context", &DebugLossy(context));
            if self.context_mark.sys.line != 0 || self.context_mark.sys.column != 0 {
                formatter.field("context_mark", &self.context_mark);
            }
        }
        formatter.finish()
    }
}

#[derive(Copy, Clone)]
pub(crate) struct Mark {
    pub(super) sys: sys::yaml_mark_t,
}

impl Default for Mark {
    fn default() -> Self {
        // SAFETY: yaml_mark_t is a plain old data struct from libyaml. A zeroed value
        // represents an empty/unknown location (index=0, line=0, column=0) and is
        // used by libyaml to denote absence of a mark. This matches how this type is
        // consumed throughout the codebase (checking for zeros).
        let sys_mark = unsafe { std::mem::MaybeUninit::<sys::yaml_mark_t>::zeroed().assume_init() };
        Self { sys: sys_mark }
    }
}

impl Mark {
    pub fn index(&self) -> u64 {
        self.sys.index
    }

    pub fn line(&self) -> u64 {
        self.sys.line
    }

    pub fn column(&self) -> u64 {
        self.sys.column
    }
}

impl Display for Mark {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        if self.sys.line != 0 || self.sys.column != 0 {
            write!(
                formatter,
                "line {} column {}",
                self.sys.line + 1,
                self.sys.column + 1,
            )
        } else {
            write!(formatter, "position {}", self.sys.index)
        }
    }
}

impl Debug for Mark {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let mut formatter = formatter.debug_struct("Mark");
        if self.sys.line != 0 || self.sys.column != 0 {
            formatter.field("line", &(self.sys.line + 1));
            formatter.field("column", &(self.sys.column + 1));
        } else {
            formatter.field("index", &self.sys.index);
        }
        formatter.finish()
    }
}

#[cfg(test)]
mod tests {
    use crate::libyaml::parser::Parser;
    use crate::libyaml::emitter::{Emitter, Event, Error as EmitterError};
    use std::borrow::Cow;

    #[test]
    fn parse_error_after_drop() {
        let err = {
            let mut parser = Parser::new(Cow::Borrowed(b"@" as &[u8])).unwrap();
            parser.next().unwrap();
            parser.next().unwrap_err()
        };
        let _ = format!("{}", err);
        let _ = format!("{:?}", err);
    }

    #[test]
    fn emit_error_after_drop() {
        let err = {
            let mut emitter = Emitter::new(Vec::<u8>::new(), -1, 2).unwrap();
            emitter.emit(Event::MappingEnd).unwrap_err()
        };
        if let EmitterError::Libyaml(inner) = err {
            let _ = format!("{}", inner);
            let _ = format!("{:?}", inner);
        } else {
            panic!("expected libyaml error");
        }
    }
}

fn display_lossy(mut bytes: &[u8], formatter: &mut fmt::Formatter) -> fmt::Result {
    fn write_filtered(s: &str, f: &mut fmt::Formatter) -> fmt::Result {
        for ch in s.chars() {
            if ch.is_control() && ch != '\n' && ch != '\r' && ch != '\t' {
                f.write_char(char::REPLACEMENT_CHARACTER)?;
            } else {
                f.write_char(ch)?;
            }
        }
        Ok(())
    }

    loop {
        match str::from_utf8(bytes) {
            Ok(valid) => return write_filtered(valid, formatter),
            Err(utf8_error) => {
                let valid_up_to = utf8_error.valid_up_to();
                write_filtered(str::from_utf8(&bytes[..valid_up_to]).unwrap(), formatter)?;
                formatter.write_char(char::REPLACEMENT_CHARACTER)?;
                if let Some(error_len) = utf8_error.error_len() {
                    bytes = &bytes[valid_up_to + error_len..];
                } else {
                    return Ok(());
                }
            }
        }
    }
}
