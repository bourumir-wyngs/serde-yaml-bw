use crate::libyaml::parser::Anchor;
use crate::libyaml::{emitter, error as libyaml};
use crate::path::Path;
use serde::{de, ser};
use std::error::Error as StdError;
use std::fmt::{self, Debug, Display};
use std::io;
use std::result;
use std::string;
use std::sync::Arc;
use crate::budget::BudgetBreach;


/// An error that happened serializing or deserializing YAML data.
pub struct Error(Box<ErrorImpl>);

/// Alias for a `Result` with the error type `serde_yaml_bw::Error`.
pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub(crate) enum ErrorImpl {
    Message(String, Option<Pos>),

    Libyaml(libyaml::Error),
    Io(io::Error),
    FromUtf8(string::FromUtf8Error),

    EndOfStream,
    MoreThanOneDocument,
    RecursionLimitExceeded(libyaml::Mark),
    RepetitionLimitExceeded,
    /// Input rejected due to exceeding YAML budget.
    BudgetExceeded(BudgetBreach),
    /// Error reported by the Saphyr pre-scanner (budget pre-check) with location.
    PreScan(ScanError),
    UnknownAnchor(libyaml::Mark, Anchor),
    ScalarInMerge,
    TaggedInMerge,
    ScalarInMergeElement,
    SequenceInMergeElement,
    MergeRecursion,
    EmptyTag,
    FailedToParseNumber,
    UnexpectedEndOfSequence,
    UnexpectedEndOfMapping,
    UnresolvedAlias,

    Shared(Arc<ErrorImpl>),
    SerializedValueBeforeSerializeKey,
    /// Indicates that an invalid YAML tag was encountered during serialization.
    TagError,
}

#[derive(Debug)]
pub struct ScanError {
    /// Human-readable error message from the pre-scanner.
    pub(crate) msg: String,
    /// Byte index in the source where the error occurred.
    pub(crate) index: usize,
    /// 1-based line number.
    pub(crate) line: usize,
    /// 1-based column number.
    pub(crate) column: usize,
}

impl ScanError {
    pub(crate) fn new(msg: String, index: usize, line: usize, column: usize) -> Self {
        Self { msg, index, line, column }
    }
}

impl From<&saphyr_parser::ScanError> for ScanError {
    fn from(e: &saphyr_parser::ScanError) -> Self {
        let m = e.marker();
        // saphyr's Display prints col + 1, so we do the same in stored column
        ScanError::new(e.info().to_owned(), m.index(), m.line(), m.col() + 1)
    }
}

#[derive(Debug)]
pub(crate) struct Pos {
    mark: libyaml::Mark,
    path: String,
}

/// The input location that an error occured.
#[derive(Debug)]
pub struct Location {
    index: usize,
    line: usize,
    column: usize,
}

impl Location {
    /// The byte index of the error
    pub fn index(&self) -> usize {
        self.index
    }

    /// The line of the error
    pub fn line(&self) -> usize {
        self.line
    }

    /// The column of the error
    pub fn column(&self) -> usize {
        self.column
    }

    // This is to keep decoupled with the yaml crate
    #[doc(hidden)]
    fn from_mark(mark: libyaml::Mark) -> Self {
        Location {
            index: mark.index() as usize,
            // `line` and `column` returned from libyaml are 0-indexed but all error messages add +1 to this value
            line: mark.line() as usize + 1,
            column: mark.column() as usize + 1,
        }
    }
}

impl Error {
    /// Returns the Location from the error if one exists.
    ///
    /// Not all types of errors have a location so this can return `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use serde_yaml_bw::{Value, Error};
    /// #
    /// // The `@` character as the first character makes this invalid yaml
    /// let invalid_yaml: Result<Value, Error> = serde_yaml_bw::from_str("@invalid_yaml");
    ///
    /// let location = invalid_yaml.unwrap_err().location().unwrap();
    ///
    /// assert_eq!(location.line(), 1);
    /// assert_eq!(location.column(), 1);
    /// ```
    pub fn location(&self) -> Option<Location> {
        self.0.location()
    }
}

pub(crate) fn new(inner: ErrorImpl) -> Error {
    Error(Box::new(inner))
}

pub(crate) fn shared(shared: Arc<ErrorImpl>) -> Error {
    Error(Box::new(ErrorImpl::Shared(shared)))
}

pub(crate) fn fix_mark(mut error: Error, mark: libyaml::Mark, path: Path) -> Error {
    if let ErrorImpl::Message(_, none @ None) = error.0.as_mut() {
        *none = Some(Pos {
            mark,
            path: path.to_string(),
        });
    }
    error
}

impl Error {
    pub(crate) fn shared(self) -> Arc<ErrorImpl> {
        if let ErrorImpl::Shared(err) = *self.0 {
            err
        } else {
            Arc::from(self.0)
        }
    }

    /// Returns true if this error is a simple message error.
    pub fn is_message(&self) -> bool {
        matches!(self.0.as_ref(), ErrorImpl::Message(_, _))
    }
}

impl From<libyaml::Error> for Error {
    fn from(err: libyaml::Error) -> Self {
        Error(Box::new(ErrorImpl::Libyaml(err)))
    }
}

impl From<emitter::Error> for Error {
    fn from(err: emitter::Error) -> Self {
        match err {
            emitter::Error::Libyaml(err) => Self::from(err),
            emitter::Error::Io(err) => new(ErrorImpl::Io(err)),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.0.source()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.display(f)
    }
}

// Remove two layers of verbosity from the debug representation. Humans often
// end up seeing this representation because it is what unwrap() shows.
impl Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.debug(f)
    }
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error(Box::new(ErrorImpl::Message(msg.to_string(), None)))
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error(Box::new(ErrorImpl::Message(msg.to_string(), None)))
    }
}

pub(crate) fn sanitize_anchor(anchor: &Anchor) -> String {
    use std::fmt::Write as _;
    let mut sanitized = String::new();
    for &b in &anchor.0 {
        match b {
            b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b'-' | b'_' => {
                sanitized.push(b as char);
            }
            _ => {
                // Represent any other byte as an escaped hexadecimal sequence.
                write!(&mut sanitized, "\\x{:02X}", b).unwrap();
            }
        }
    }
    sanitized
}

impl ErrorImpl {
    fn location(&self) -> Option<Location> {
        match self {
            ErrorImpl::PreScan(se) => Some(Location { index: se.index, line: se.line, column: se.column }),
            _ => self.mark().map(Location::from_mark),
        }
    }

    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            ErrorImpl::Io(err) => err.source(),
            ErrorImpl::FromUtf8(err) => err.source(),
            ErrorImpl::Shared(err) => err.source(),
            _ => None,
        }
    }

    fn mark(&self) -> Option<libyaml::Mark> {
        match self {
            ErrorImpl::Message(_, Some(Pos { mark, path: _ }))
            | ErrorImpl::RecursionLimitExceeded(mark) => Some(*mark),
            ErrorImpl::Libyaml(err) => Some(err.mark()),
            ErrorImpl::Shared(err) => err.mark(),
            ErrorImpl::UnknownAnchor(mark, _alias) => Some(*mark),
            _ => None,
        }
    }

    fn message_no_mark(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorImpl::Message(msg, None) => f.write_str(msg),
            ErrorImpl::Message(msg, Some(Pos { mark: _, path })) => {
                if path != "." {
                    write!(f, "{}: ", path)?;
                }
                f.write_str(msg)
            }
            ErrorImpl::Libyaml(err) => Display::fmt(err, f),
            ErrorImpl::Io(err) => Display::fmt(err, f),
            ErrorImpl::FromUtf8(err) => Display::fmt(err, f),
            ErrorImpl::EndOfStream => f.write_str("EOF while parsing a value"),
            ErrorImpl::MoreThanOneDocument => f.write_str(
                "deserializing from YAML containing more than one document is not supported",
            ),
            ErrorImpl::RecursionLimitExceeded(_mark) => f.write_str("recursion limit exceeded"),
            ErrorImpl::RepetitionLimitExceeded => f.write_str("repetition limit exceeded"),
            ErrorImpl::UnknownAnchor(_mark, alias) => {
                f.write_str(&format!(
                    "reference to non existing anchor [{}]",
                    &crate::error::sanitize_anchor(alias)
                ))
            }
            ErrorImpl::ScalarInMerge => {
                f.write_str("expected a mapping or list of mappings for merging, but found scalar")
            }
            ErrorImpl::TaggedInMerge => f.write_str("unexpected tagged value in merge"),
            ErrorImpl::ScalarInMergeElement => {
                f.write_str("expected a mapping for merging, but found scalar")
            }
            ErrorImpl::SequenceInMergeElement => {
                f.write_str("expected a mapping for merging, but found sequence")
            }
            ErrorImpl::MergeRecursion => f.write_str("encountered recursive merge alias"),
            ErrorImpl::EmptyTag => f.write_str("empty YAML tag is not allowed"),
            ErrorImpl::FailedToParseNumber => f.write_str("failed to parse YAML number"),
            ErrorImpl::Shared(err) => err.display(f),
            ErrorImpl::UnexpectedEndOfSequence => f.write_str("unexpected end of sequence"),
            ErrorImpl::UnexpectedEndOfMapping => f.write_str("unexpected end of mapping"),
            ErrorImpl::UnresolvedAlias => f.write_str("unresolved alias"),
            ErrorImpl::SerializedValueBeforeSerializeKey => {
                f.write_str("serialize_value called before serialize_key")
            }
            ErrorImpl::TagError => f.write_str("unexpected tag error"),
            ErrorImpl::BudgetExceeded(b) => write!(f, "YAML budget exceeded: {:?}", b),
            ErrorImpl::PreScan(se) => write!(
                f,
                "{} at byte {} line {} column {}",
                se.msg, se.index, se.line, se.column
            ),
        }
    }

    fn display(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorImpl::Libyaml(err) => Display::fmt(err, f),
            ErrorImpl::Shared(err) => err.display(f),
            _ => {
                self.message_no_mark(f)?;
                if let Some(mark) = self.mark() {
                    if mark.line() != 0 || mark.column() != 0 {
                        write!(f, " at {}", mark)?;
                    }
                }
                Ok(())
            }
        }
    }

    fn debug(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorImpl::Libyaml(err) => Debug::fmt(err, f),
            ErrorImpl::Shared(err) => err.debug(f),
            _ => {
                f.write_str("Error(")?;
                struct MessageNoMark<'a>(&'a ErrorImpl);
                impl Display for MessageNoMark<'_> {
                    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                        self.0.message_no_mark(f)
                    }
                }
                let msg = MessageNoMark(self).to_string();
                Debug::fmt(&msg, f)?;
                if let Some(mark) = self.mark() {
                    write!(
                        f,
                        ", line: {}, column: {}",
                        mark.line() + 1,
                        mark.column() + 1,
                    )?;
                }
                f.write_str(")")
            }
        }
    }
}
