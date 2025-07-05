//! Path to the current value in the input.

use std::{
    cell::OnceCell,
    fmt::{self, Display},
};

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

impl<'a> Path<'a> {
    /// Returns an owned version of this path.
    pub fn to_owned_path(&self) -> OwnedPath {
        match self {
            Path::Root => OwnedPath::Root,
            Path::Seq { parent, index } => OwnedPath::Seq {
                parent: Box::new(parent.to_owned_path()),
                index: *index,
                borrowed: OnceCell::new(),
            },
            Path::Map { parent, key } => OwnedPath::Map {
                parent: Box::new(parent.to_owned_path()),
                key: key.to_string(),
                borrowed: OnceCell::new(),
            },
            Path::Alias { parent } => OwnedPath::Alias {
                parent: Box::new(parent.to_owned_path()),
                borrowed: OnceCell::new(),
            },
            Path::Unknown { parent } => OwnedPath::Unknown {
                parent: Box::new(parent.to_owned_path()),
                borrowed: OnceCell::new(),
            },
        }
    }
}

/// An owned version of a [Path].
pub enum OwnedPath {
    /// The root of the input.
    Root,
    /// A sequence index.
    Seq {
        /// The path to the parent value.
        parent: Box<OwnedPath>,
        /// The index of the current value.
        index: usize,
        /// A cell to hold the borrowed path.
        borrowed: OnceCell<Path<'static>>,
    },
    /// A map key.
    Map {
        /// The path to the parent value.
        parent: Box<OwnedPath>,
        /// The key of the current value.
        key: String,
        /// A cell to hold the borrowed path.
        borrowed: OnceCell<Path<'static>>,
    },
    /// An alias.
    Alias {
        /// The path to the parent value.
        parent: Box<OwnedPath>,
        /// A cell to hold the borrowed path.
        borrowed: OnceCell<Path<'static>>,
    },
    /// An unknown path.
    Unknown {
        /// The path to the parent value.
        parent: Box<OwnedPath>,
        /// A cell to hold the borrowed path.
        borrowed: OnceCell<Path<'static>>,
    },
}

impl<'a> OwnedPath {
    /// Returns a borrowed version of this path.
    pub fn as_path(&'a self) -> &'a Path<'a> {
        static ROOT: Path<'static> = Path::Root;

        match self {
            OwnedPath::Root => &ROOT,
            OwnedPath::Seq {
                parent,
                index,
                borrowed,
            } => {
                let borrowed = borrowed.get_or_init(|| {
                    let parent = parent.as_path();
                    // SAFETY: self-reference
                    unsafe {
                        std::mem::transmute(Path::Seq {
                            parent,
                            index: *index,
                        })
                    }
                });
                borrowed
            }
            OwnedPath::Map {
                parent,
                key,
                borrowed,
            } => {
                let borrowed = borrowed.get_or_init(|| {
                    let parent = parent.as_path();
                    // SAFETY: self-reference
                    unsafe {
                        std::mem::transmute(Path::Map {
                            parent,
                            key: key.as_ref(),
                        })
                    }
                });
                borrowed
            }
            OwnedPath::Alias { parent, borrowed } => {
                let borrowed = borrowed.get_or_init(|| {
                    let parent = parent.as_path();
                    // SAFETY: self-reference
                    unsafe { std::mem::transmute(Path::Alias { parent }) }
                });
                borrowed
            }
            OwnedPath::Unknown { parent, borrowed } => {
                let borrowed = borrowed.get_or_init(|| {
                    let parent = parent.as_path();
                    // SAFETY: self-reference
                    unsafe { std::mem::transmute(Path::Unknown { parent }) }
                });
                borrowed
            }
        }
    }
}
