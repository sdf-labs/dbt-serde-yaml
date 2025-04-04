//! The

use serde::{ser::Serializer, Deserialize, Deserializer, Serialize};
use std::{
    fmt::{self, Debug},
    hash::{Hash, Hasher},
    ops::Deref,
};

mod span;

pub use span::Marker;
pub use span::Span;

/// A wrapper type that can be used to capture the source location of a
/// deserialized value.
///
/// NOTE:
/// - Only works with the dbt_serde_yaml deserializer.
/// - May contain leading and trailing whitespace.
pub struct Spanned<T> {
    span: Span,
    node: T,
}

impl<'de, T> Spanned<T>
where
    T: Deserialize<'de>,
{
    /// Create a new `Spanned` value with the given node.
    pub fn new(node: T) -> Self {
        Spanned {
            span: Default::default(),
            node,
        }
    }
}

impl<T> Spanned<T> {
    /// Transform the inner node by applying the given function.
    pub fn map<U, F>(self, f: F) -> Spanned<U>
    where
        F: FnOnce(T) -> U,
    {
        Spanned {
            span: self.span,
            node: f(self.node),
        }
    }

    /// Consumes the [Spanned] and returns the inner node.
    pub fn into_inner(self) -> T {
        self.node
    }

    /// Get the captured source span.
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// True if this [Spanned] actually contains a valid span.
    pub fn has_valid_span(&self) -> bool {
        self.span.is_valid()
    }
}

impl<T> Deref for Spanned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl<T> AsRef<T> for Spanned<T> {
    fn as_ref(&self) -> &T {
        &self.node
    }
}

impl<T> AsMut<T> for Spanned<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.node
    }
}

impl<T> Clone for Spanned<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Spanned {
            span: self.span.clone(),
            node: self.node.clone(),
        }
    }
}

impl<T> Debug for Spanned<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} {:?}", self.span, self.node)
    }
}

impl<T> PartialEq for Spanned<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node
    }
}

impl<T> Eq for Spanned<T> where T: Eq {}

impl<T> PartialOrd for Spanned<T>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.node.partial_cmp(&other.node)
    }
}

impl<T> Ord for Spanned<T>
where
    T: Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.node.cmp(&other.node)
    }
}

impl<T> Hash for Spanned<T>
where
    T: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.node.hash(state);
    }
}

impl<T> Serialize for Spanned<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        set_marker(self.span.start);
        let res = T::serialize(&self.node, serializer);
        set_marker(self.span.end);
        res
    }
}

impl<'de, T> Deserialize<'de> for Spanned<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let start_marker = get_marker();
        let node = T::deserialize(deserializer)?;
        let end_marker = get_marker();
        let span: Span = (start_marker..end_marker).into();

        #[cfg(feature = "filename")]
        let span = span.maybe_capture_filename();

        Ok(Spanned { span, node })
    }
}

#[cfg(feature = "schemars")]
impl<T> schemars::JsonSchema for Spanned<T>
where
    T: schemars::JsonSchema,
{
    fn schema_name() -> String {
        T::schema_name()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        T::json_schema(gen)
    }

    fn is_referenceable() -> bool {
        false
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        T::schema_id()
    }
}

#[cfg(feature = "filename")]
/// A scope guard that sets the current source filename.
pub struct WithFilenameScope {
    original: Option<std::sync::Arc<std::path::PathBuf>>,
}

#[cfg(feature = "filename")]
impl Drop for WithFilenameScope {
    fn drop(&mut self) {
        FILENAME.with(|f| *f.borrow_mut() = std::mem::take(&mut self.original));
    }
}

#[cfg(feature = "filename")]
/// Set the current source filename. Returns a scope guard that restores the
/// original filename when dropped.
pub fn with_filename(filename: impl Into<std::path::PathBuf>) -> WithFilenameScope {
    let filename = filename.into();
    let original = FILENAME.with(|f| f.borrow_mut().take());
    FILENAME.with(|f| *f.borrow_mut() = Some(std::sync::Arc::new(filename)));
    WithFilenameScope { original }
}

/// Set the current source location marker.
///
/// This is called by [Deserializer] implementations to inform the
/// [crate::Spanned] and [crate::Value] types about the current source location.
pub fn set_marker(marker: impl Into<Marker>) {
    MARKER.with(|m| *m.borrow_mut() = Some(marker.into()));
}

/// Reset the source location marker.
pub fn reset_marker() {
    MARKER.with(|m| *m.borrow_mut() = None);
}

/// Get the current source location marker.
pub(crate) fn get_marker() -> Option<Marker> {
    MARKER.with(|m| *m.borrow())
}

#[cfg(feature = "filename")]
/// Set the current source filename.
pub(crate) fn set_filename(filename: std::sync::Arc<std::path::PathBuf>) {
    FILENAME.with(|f| *f.borrow_mut() = Some(filename));
}

#[cfg(feature = "filename")]
/// Get the current source filename.
pub(crate) fn get_filename() -> Option<std::sync::Arc<std::path::PathBuf>> {
    FILENAME.with(|f| f.borrow().clone())
}

thread_local! {
    static MARKER: std::cell::RefCell<Option<Marker>> = const {
        std::cell::RefCell::new(None)
    };

    #[cfg(feature = "filename")]
    static FILENAME: std::cell::RefCell<Option<std::sync::Arc<std::path::PathBuf>>> = const {
        std::cell::RefCell::new(None)
    };
}
