use crate::libyaml::{emitter, error as libyaml};
use crate::path::Path;
use crate::{Marker, Span};
use serde::{de, ser};
use std::error::Error as StdError;
use std::fmt::{self, Debug, Display};
use std::io;
use std::result;
use std::string;
use std::sync::Arc;

/// An error that happened serializing or deserializing YAML data.
pub struct Error(Box<ErrorImpl>);

/// Alias for a `Result` with the error type `dbt_serde_yaml::Error`.
pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub(crate) enum ErrorImpl {
    Message(String, Option<Pos>),

    Libyaml(libyaml::Error),
    Io(io::Error),
    FromUtf8(string::FromUtf8Error),

    EndOfStream,
    MoreThanOneDocument,
    RecursionLimitExceeded(Marker),
    RepetitionLimitExceeded,
    BytesUnsupported,
    UnknownAnchor(Marker),
    SerializeNestedEnum,
    ScalarInMerge,
    TaggedInMerge,
    ScalarInMergeElement,
    SequenceInMergeElement,
    EmptyTag,
    FailedToParseNumber,
    FlattenNotMapping,

    External(Box<dyn StdError + 'static + Send + Sync>),

    Shared(Arc<ErrorImpl>),
}

#[derive(Debug)]
pub(crate) struct Pos {
    span: Span,
    path: String,
}

impl Error {
    /// Returns the Location from the error if one exists.
    ///
    /// Not all types of errors have a location so this can return `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use dbt_serde_yaml::{Value, Error};
    /// #
    /// // The `@` character as the first character makes this invalid yaml
    /// let invalid_yaml: Result<Value, Error> = dbt_serde_yaml::from_str("@invalid_yaml");
    ///
    /// let location = invalid_yaml.unwrap_err().location().unwrap();
    ///
    /// assert_eq!(location.line(), 1);
    /// assert_eq!(location.column(), 1);
    /// ```
    pub fn location(&self) -> Option<Marker> {
        self.0.location()
    }

    /// Returns the Span from the error if one exists.    
    ///
    /// Not all types of errors have a span so this can return `None`.
    pub fn span(&self) -> Option<Span> {
        self.0.span()
    }

    /// Unwraps the error and returns the underlying error if it is an external
    /// error; otherwise returns `None`.
    pub fn into_external(self) -> Option<Box<dyn StdError + 'static + Send + Sync>> {
        if let ErrorImpl::External(err) = *self.0 {
            Some(err)
        } else {
            None
        }
    }

    /// Returns the error message without the location information.
    pub fn display_no_mark(&self) -> impl Display + use<'_> {
        struct MessageNoMark<'a>(&'a ErrorImpl);
        impl Display for MessageNoMark<'_> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                match &self.0 {
                    ErrorImpl::Libyaml(err) => Display::fmt(err, f),
                    ErrorImpl::Shared(err) => err.display(f),
                    _ => self.0.message_no_mark(f),
                }
            }
        }
        MessageNoMark(&self.0)
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
        let span = Span::from(Marker::from(mark));

        #[cfg(feature = "filename")]
        let span = span.maybe_capture_filename();

        *none = Some(Pos {
            span,
            path: path.to_string(),
        });
    }
    error
}

pub(crate) fn set_span(mut error: Error, span: Span) -> Error {
    if let ErrorImpl::Message(_, pos) = error.0.as_mut() {
        if let Some(pos) = pos {
            if !pos.span.is_valid() {
                pos.span = span;
            }
        } else {
            *pos = Some(Pos {
                span,
                path: ".".to_string(),
            })
        }
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

impl From<Box<dyn StdError + 'static + Send + Sync>> for Error {
    fn from(err: Box<dyn StdError + 'static + Send + Sync>) -> Self {
        Error(Box::new(ErrorImpl::External(err)))
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

impl ErrorImpl {
    fn location(&self) -> Option<Marker> {
        self.span().map(|span| span.start)
    }

    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            ErrorImpl::Io(err) => err.source(),
            ErrorImpl::FromUtf8(err) => err.source(),
            ErrorImpl::Shared(err) => err.source(),
            ErrorImpl::External(err) => err.source(),
            _ => None,
        }
    }

    fn span(&self) -> Option<Span> {
        match self {
            ErrorImpl::Message(_, Some(Pos { span, path: _ })) => Some(span.clone()),
            ErrorImpl::RecursionLimitExceeded(mark) | ErrorImpl::UnknownAnchor(mark) => {
                Some(Span::from(*mark))
            }
            ErrorImpl::Libyaml(err) => Some(Marker::from(err.mark()).into()),
            ErrorImpl::Shared(err) => err.span(),
            _ => None,
        }
    }

    fn message_no_mark(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorImpl::Message(msg, None) => f.write_str(msg),
            ErrorImpl::Message(msg, Some(Pos { span: _, path })) => {
                if path != "." {
                    write!(f, "{}: ", path)?;
                }
                f.write_str(msg)
            }
            ErrorImpl::Libyaml(_) => unreachable!(),
            ErrorImpl::Io(err) => Display::fmt(err, f),
            ErrorImpl::FromUtf8(err) => Display::fmt(err, f),
            ErrorImpl::EndOfStream => f.write_str("EOF while parsing a value"),
            ErrorImpl::MoreThanOneDocument => f.write_str(
                "deserializing from YAML containing more than one document is not supported",
            ),
            ErrorImpl::RecursionLimitExceeded(_mark) => f.write_str("recursion limit exceeded"),
            ErrorImpl::RepetitionLimitExceeded => f.write_str("repetition limit exceeded"),
            ErrorImpl::BytesUnsupported => {
                f.write_str("serialization and deserialization of bytes in YAML is not implemented")
            }
            ErrorImpl::UnknownAnchor(_mark) => f.write_str("unknown anchor"),
            ErrorImpl::SerializeNestedEnum => {
                f.write_str("serializing nested enums in YAML is not supported yet")
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
            ErrorImpl::EmptyTag => f.write_str("empty YAML tag is not allowed"),
            ErrorImpl::FailedToParseNumber => f.write_str("failed to parse YAML number"),
            ErrorImpl::External(err) => Display::fmt(err.as_ref(), f),
            ErrorImpl::Shared(_) => unreachable!(),
            ErrorImpl::FlattenNotMapping => write!(f, "expected the flatten field to be a mapping"),
        }
    }

    fn display(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ErrorImpl::Libyaml(err) => Display::fmt(err, f),
            ErrorImpl::Shared(err) => err.display(f),
            _ => {
                self.message_no_mark(f)?;
                if let Some(mark) = self.location() {
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
                if let Some(mark) = self.location() {
                    write!(f, ", line: {}, column: {}", mark.line(), mark.column(),)?;
                }
                f.write_str(")")
            }
        }
    }
}
