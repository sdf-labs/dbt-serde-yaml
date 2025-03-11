use crate::libyaml::error::Mark;

/// A source span.
#[derive(Clone, Copy, Default, Debug)]
pub struct Span {
    /// The start of the span.
    pub start: Marker,

    /// The end of the span.
    pub end: Marker,
}

/// A location in the source string.
#[derive(Copy, Clone, Default, Debug)]
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
    pub fn zero() -> Self {
        Marker {
            index: 0,
            line: 1,
            column: 1,
        }
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
