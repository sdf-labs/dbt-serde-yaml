use std::fmt::{self, Debug, Display};
use std::ops::Range;

use crate::libyaml::error::Mark;

/// A source span.
#[derive(Clone, Copy, PartialEq, PartialOrd)]
pub struct Span {
    /// The start of the span.
    pub start: Marker,

    /// The end of the span.
    pub end: Marker,
}

impl Span {
    /// Create a new span.
    pub fn new(start: Marker, end: Marker) -> Self {
        Span { start, end }
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
        Span { start, end }
    }
}

impl From<Range<Option<Marker>>> for Span {
    fn from(range: Range<Option<Marker>>) -> Self {
        let start = range.start.unwrap_or_default();
        let end = range.end.unwrap_or_default();
        Span { start, end }
    }
}

impl From<Span> for Range<Option<usize>> {
    fn from(span: Span) -> Self {
        Some(span.start.index)..Some(span.end.index)
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
        write!(f, "{}:{}", self.line, self.column)
    }
}
