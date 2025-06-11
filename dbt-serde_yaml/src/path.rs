//! Path to the current value in the input.

use std::fmt::{self, Display};

/// A structured representation of a path to the current value in the input,
/// like `dependencies.serde.typo1`.
#[derive(Copy, Clone)]
pub enum Path<'a> {
    /// The root of the input.
    Root,
    /// A sequence index.
    Seq {
        /// The path to the parent value.
        parent: &'a Path<'a>,
        /// The index of the current value.
        index: usize,
    },
    /// A map key.
    Map {
        /// The path to the parent value.
        parent: &'a Path<'a>,
        /// The key of the current value.
        key: &'a str,
    },
    /// An alias.
    Alias {
        /// The path to the parent value.
        parent: &'a Path<'a>,
    },
    /// An unknown path.
    Unknown {
        /// The path to the parent value.
        parent: &'a Path<'a>,
    },
}

impl Display for Path<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        struct Parent<'a>(&'a Path<'a>);

        impl Display for Parent<'_> {
            fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
                match self.0 {
                    Path::Root => Ok(()),
                    path => write!(formatter, "{}.", path),
                }
            }
        }

        match self {
            Path::Root => formatter.write_str("."),
            Path::Seq { parent, index } => write!(formatter, "{}[{}]", parent, index),
            Path::Map { parent, key } => write!(formatter, "{}{}", Parent(parent), key),
            Path::Alias { parent } => write!(formatter, "{}", parent),
            Path::Unknown { parent } => write!(formatter, "{}?", Parent(parent)),
        }
    }
}
