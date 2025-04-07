use std::fmt::{self, Debug, Display};
use std::ops::Range;
#[cfg(feature = "filename")]
use std::path::PathBuf;
#[cfg(feature = "filename")]
use std::sync::Arc;

use crate::libyaml::error::Mark;

/// A source span.
#[derive(Clone, PartialEq, PartialOrd)]
pub struct Span {
    /// The start of the span.
    pub start: Marker,

    /// The end of the span.
    pub end: Marker,

    #[cfg(feature = "filename")]
    /// An optional filename.
    pub filename: Option<Arc<PathBuf>>,
}

impl Span {
    /// Create a new span.
    pub fn new(start: Marker, end: Marker) -> Self {
        Span {
            start,
            end,
            #[cfg(feature = "filename")]
            filename: None,
        }
    }

    /// True if this span is valid.
    pub fn is_valid(&self) -> bool {
        self.start.index <= self.end.index
            && self.start.line > 0
            && self.start.column > 0
            && self.end.line > 0
            && self.end.column > 0
    }

    /// Construct an empty (invalid) span.
    pub const fn zero() -> Self {
        Span {
            start: Marker::zero(),
            end: Marker::zero(),
            #[cfg(feature = "filename")]
            filename: None,
        }
    }
}

#[cfg(feature = "filename")]
impl Span {
    /// Create a new span with the specified filename.
    pub fn new_with_filename(
        start: impl Into<Marker>,
        end: impl Into<Marker>,
        filename: impl Into<Arc<PathBuf>>,
    ) -> Self {
        Span {
            start: start.into(),
            end: end.into(),
            filename: Some(filename.into()),
        }
    }

    /// Replace the filename in this span with the given filename.
    pub fn with_filename(self, filename: impl Into<Arc<PathBuf>>) -> Self {
        Span {
            filename: Some(filename.into()),
            ..self
        }
    }

    /// Get the filename in this span.
    pub fn get_filename(&self) -> Option<&std::path::Path> {
        self.filename.as_deref().map(|f| f.as_ref())
    }

    pub(crate) fn maybe_capture_filename(self) -> Self {
        if let Some(filename) = crate::spanned::get_filename() {
            Self {
                filename: Some(filename),
                ..self
            }
        } else {
            self
        }
    }
}

impl Default for Span {
    fn default() -> Self {
        Span::zero()
    }
}

impl Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}..{:?}", self.start, self.end)
    }
}

impl Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

impl From<(Marker, Marker)> for Span {
    fn from((start, end): (Marker, Marker)) -> Self {
        Span::new(start, end)
    }
}

impl From<Range<Option<Marker>>> for Span {
    fn from(range: Range<Option<Marker>>) -> Self {
        let start = range.start.unwrap_or_default();
        let end = range.end.unwrap_or_default();
        Span::new(start, end)
    }
}

impl From<Marker> for Span {
    fn from(marker: Marker) -> Self {
        Span::new(marker, marker)
    }
}

/// A location in the source string.
#[derive(Copy, Clone, PartialEq, PartialOrd)]
pub struct Marker {
    /// Offset in bytes from the start of the source string.
    pub index: usize,

    /// Line number in the source string.
    pub line: usize,

    /// Column number in the source string.
    pub column: usize,
}

impl Marker {
    /// Create a new location.
    pub fn new(index: usize, line: usize, column: usize) -> Self {
        Marker {
            index,
            line,
            column,
        }
    }

    /// Create a location pointing to the start of the source string.
    pub const fn start() -> Self {
        Marker {
            index: 0,
            line: 1,
            column: 1,
        }
    }

    /// Create an empty location.
    pub const fn zero() -> Self {
        Marker {
            index: 0,
            line: 0,
            column: 0,
        }
    }

    /// Return the line number of this location.
    pub fn line(&self) -> usize {
        self.line
    }

    /// Return the column number of this location.
    pub fn column(&self) -> usize {
        self.column
    }

    /// Return the index of this location.
    pub fn index(&self) -> usize {
        self.index
    }
}

impl Default for Marker {
    fn default() -> Self {
        Marker::zero()
    }
}

impl From<Mark> for Marker {
    fn from(mark: Mark) -> Self {
        Marker {
            index: mark.index() as usize,
            // `line` and `column` returned from libyaml are 0-indexed
            line: mark.line() as usize + 1,
            column: mark.column() as usize + 1,
        }
    }
}

impl Debug for Marker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}[{}]", self.line, self.column, self.index)
    }
}

impl Display for Marker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {} column {}", self.line, self.column)
    }
}
