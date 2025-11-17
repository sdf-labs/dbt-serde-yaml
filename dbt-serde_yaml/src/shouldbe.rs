//! This module defines the `ShouldBe` type, which can be used as an error
//! recovery mechanism during deserialization.
//!
//! See the [ShouldBe] documentation for more details.

use std::{
    fmt::Debug,
    sync::{
        atomic::{self, AtomicPtr},
        Arc,
    },
};

use serde::{
    de::{DeserializeOwned, Error as _},
    Deserialize, Deserializer, Serialize,
};

use crate::{Error, Value};

/// Represents a value that "should be" deserialized to type `T`, or provides
/// information about why it failed to.
///
/// This wrapper type can be used as an error recovery mechanism in
/// `#[derive(Deserialize)]` structs to "containerize" local failures, without
/// failing the deserialization process: deserializing into a `ShouldBe<T>` will
/// *always* succeed, producing a [ShouldBe] object that either wraps a valid
/// `T` value, or the error (and the corresponding pre-deserialized value, if
/// deserializing from a [Value]) that caused the failure.
///
/// You can think of [`ShouldBe<T>`] as a more versatile `Result<T, Error>` that
/// exposes the equality, ordering, hashing, cloning, and (de)serialization
/// semantics of `T` (and indeed, [`ShouldBe<T>`] is `Into<Result<T, Error>>`),
/// while also providing a more ergonomic API for inspecting the error case.
///
/// ## Example
///
/// ```
/// # use dbt_serde_yaml::{ShouldBe, Value};
/// # use serde_derive::{Serialize, Deserialize};
/// use serde::{Serialize as _, Deserialize as _};
///
/// #[derive(Serialize, Deserialize, PartialEq, Debug)]
/// struct Inner {
///     field: i32,
/// }
///
/// #[derive(Serialize, Deserialize, PartialEq, Debug)]
/// struct Outer {
///    items: Vec<ShouldBe<Inner>>,
/// }
///
/// fn main() -> Result<(), dbt_serde_yaml::Error> {
///    let yaml = r#"
///        items:
///          - field: 1
///          - field: "2"
///          - x: 3
///    "#;
///    let value: Value = dbt_serde_yaml::from_str(&yaml)?;
///
///    let outer: Outer = value.into_typed(|_, _, _| {}, |_| Ok(None))?;
///    assert_eq!(outer.items.len(), 3);
///    assert_eq!(outer.items[0].as_ref(), Some(&Inner { field: 1 }));
///    assert!(outer.items[1].isnt());
///    assert_eq!(outer.items[1].as_err_msg().unwrap(),
///               "invalid type: string \"2\", expected i32 at line 4 column 19");
///    assert!(outer.items[2].isnt());
///    assert_eq!(outer.items[2].as_err_msg().unwrap(),
///               "missing field `field` at line 5 column 12");
///
///    Ok(())
/// }
/// ```
///
/// # Clone semantics
///
/// [`ShouldBe<T>`] is cloneable as long as `T` is cloneable. Cloning a
/// [ShouldBe::AndIs] variant clones the inner `T` value. Cloning a
/// [ShouldBe::ButIsnt] variant, however, does *not* clone the inner [Error], as
/// [Error] is not cloneable. Instead, the cloned [ShouldBe::ButIsnt] will share
/// the same underlying [Error] instance as the original. The same is true for
/// the raw [Value] stored within a [ShouldBe::ButIsnt], if one exists.
///
/// # Inspecting errors
///
/// [ShouldBe] provides two methods to inspect the error that caused a failed
/// deserialization: [ShouldBe::as_err_msg] and [ShouldBe::take_err], which
/// return the error message and the original [Error] instance, respectively.
///
/// If you only need the error message, then [ShouldBe::as_err_msg] is always
/// available on a [ShouldBe::ButIsnt] variant and can be called at any time. If
/// you need the original [Error] instance, however, you must be aware of the
/// ownership semantics of [ShouldBe::take_err]: the captured [Error] instance
/// within a [ShouldBe::ButIsnt] is never directly observable from outside, and
/// the only way to access it is by extracting it via [ShouldBe::take_err]. This
/// method transfers ownership of the [Error] out of the [ShouldBe] instance to
/// the caller. This means that [ShouldBe::take_err] will return `Some(Error)`
/// *only* the first time it is called on *all [ShouldBe] instances cloned from
/// the same [ShouldBe::ButIsnt] instance* (see "Clone semantics"); subsequent
/// calls will return `None`. This is generally what you'd want when handling
/// errors, as it guarantees that each unique error is only handled once
/// regardless of how many times the [ShouldBe::ButIsnt] instance has been
/// cloned.
///
/// ## Inspecting the raw [Value]
///
/// If a [ShouldBe::ButIsnt] instance was deserialized from a [Value], it will
/// also capture the corresponding [Value] object that failed to deserialize.
/// You can access it via the [ShouldBe::as_ref_raw] method.
///
/// ## Example
/// ```
/// # use dbt_serde_yaml::{ShouldBe, Value};
///
/// fn main() -> Result<(), dbt_serde_yaml::Error> {
///    let yaml = "k: v\n";
///    let value: Value = dbt_serde_yaml::from_str(&yaml)?;
///    let should_be: ShouldBe<i32> = value.to_typed(|_, _, _| {}, |_| Ok(None))?;
///
///    assert!(should_be.isnt());
///    assert_eq!(should_be.as_err_msg().unwrap(),
///              "invalid type: map, expected i32 at line 1 column 1");
///   
///    let cloned = should_be.clone();
///    assert!(cloned.isnt());
///    // Take the error from the original instance
///    let err = should_be.take_err().unwrap();
///    assert_eq!(err.location().unwrap().index, 0);
///    // Subsequent calls to take_err() return None
///    assert!(should_be.take_err().is_none());
///    assert!(cloned.take_err().is_none());
///    // But the error message is still available
///    assert_eq!(cloned.as_err_msg().unwrap(),
///             "invalid type: map, expected i32 at line 1 column 1");
///    // The raw Value is also available
///    assert_eq!(should_be.as_ref_raw().unwrap(), &value);
///    assert_eq!(cloned.as_ref_raw().unwrap(), &value);
///
///    Ok(())
/// }
/// ```
///
/// # Serializing a [`ShouldBe<T>`]
///
/// You can serialize a [`ShouldBe<T>`] instance as long as `T` is serializable.
/// When serializing a [ShouldBe::AndIs] variant, the inner `T` value is
/// serialized as usual. When serializing a [ShouldBe::ButIsnt] variant, if it
/// contains a raw [Value] (i.e., it was deserialized from a [Value]), then the
/// raw [Value] is serialized; otherwise, an error is raised and serialization
/// fails.
///
/// ```
/// # use dbt_serde_yaml::{ShouldBe, Value};
///
/// fn main() -> Result<(), dbt_serde_yaml::Error> {
///   let yaml = "k: v\n";
///   let value: Value = dbt_serde_yaml::from_str(&yaml)?;
///   let should_be: ShouldBe<i32> = value.into_typed(|_, _, _| {}, |_| Ok(None))?;
///
///   assert!(should_be.isnt());
///   let serialized = dbt_serde_yaml::to_string(&should_be)?;
///   assert_eq!(serialized, yaml);
///
///   Ok(())
/// }
/// ```
///
#[derive(Clone)]
pub enum ShouldBe<T> {
    /// On successful deserialization, will contain the expected value of type
    /// `T`.
    AndIs(T),

    /// On failed deserialization, will contain the error and raw value (if
    /// deserialized from a [Value]) that caused the failure.
    ButIsnt(WhyNot),
}

impl<T> ShouldBe<T> {
    /// Returns a reference to the inner `T` value if it exists
    pub fn as_ref(&self) -> Option<&T> {
        match self {
            ShouldBe::AndIs(value) => Some(value),
            ShouldBe::ButIsnt(_) => None,
        }
    }

    /// Returns a mutable reference to the inner `T` value if it exists
    pub fn as_ref_mut(&mut self) -> Option<&mut T> {
        match self {
            ShouldBe::AndIs(value) => Some(value),
            ShouldBe::ButIsnt(_) => None,
        }
    }

    /// Returns a reference to the raw [Value] if this object represents a
    /// failed deserialization.
    pub fn as_ref_raw(&self) -> Option<&crate::Value> {
        match self {
            ShouldBe::AndIs(_) => None,
            ShouldBe::ButIsnt(why_not) => why_not.as_ref_raw(),
        }
    }

    /// Returns the error message if this object represents a failed
    /// deserialization.
    pub fn as_err_msg(&self) -> Option<&str> {
        match self {
            ShouldBe::AndIs(_) => None,
            ShouldBe::ButIsnt(why_not) => Some(why_not.as_msg()),
        }
    }

    /// True if this object wraps a valid `T` value, false otherwise.
    pub fn is(&self) -> bool {
        matches!(self, ShouldBe::AndIs(_))
    }

    /// True if this object represents a failed deserialization, false
    /// otherwise.
    pub fn isnt(&self) -> bool {
        matches!(self, ShouldBe::ButIsnt(_))
    }

    /// Consumes self, returning the inner `T` value if it exists.
    pub fn into_inner(self) -> Option<T> {
        match self {
            ShouldBe::AndIs(value) => Some(value),
            ShouldBe::ButIsnt(_) => None,
        }
    }

    /// Extracts the contained [Error] instance, if any.
    ///
    /// This method transfers ownership of the [Error] out of the [ShouldBe]
    /// instance to the caller. See the [ShouldBe] documentation for more
    /// details.
    pub fn take_err(&self) -> Option<Error> {
        match self {
            ShouldBe::AndIs(_) => None,
            ShouldBe::ButIsnt(why_not) => why_not.take_err(),
        }
    }
}

impl<T> Debug for ShouldBe<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShouldBe::AndIs(value) => value.fmt(f),
            ShouldBe::ButIsnt(why_not) => {
                write!(f, "ShouldBe::ButIsnt({:?})", why_not)
            }
        }
    }
}

impl<T> Default for ShouldBe<T>
where
    T: Default,
{
    fn default() -> Self {
        ShouldBe::AndIs(T::default())
    }
}

impl<T> From<T> for ShouldBe<T> {
    fn from(value: T) -> Self {
        ShouldBe::AndIs(value)
    }
}

impl<T> From<ShouldBe<T>> for Option<T> {
    fn from(should_be: ShouldBe<T>) -> Self {
        should_be.into_inner()
    }
}

impl<T> From<ShouldBe<T>> for Result<T, Error> {
    fn from(should_be: ShouldBe<T>) -> Self {
        match should_be {
            ShouldBe::AndIs(value) => Ok(value),
            ShouldBe::ButIsnt(why_not) => Err(why_not.into()),
        }
    }
}

impl<T> PartialEq for ShouldBe<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ShouldBe::AndIs(a), ShouldBe::AndIs(b)) => a == b,
            (ShouldBe::ButIsnt(a), ShouldBe::ButIsnt(b)) => a == b,
            _ => false,
        }
    }
}

impl<T> Eq for ShouldBe<T> where T: Eq {}

impl<T> PartialOrd for ShouldBe<T>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (ShouldBe::AndIs(a), ShouldBe::AndIs(b)) => a.partial_cmp(b),
            (ShouldBe::ButIsnt(a), ShouldBe::ButIsnt(b)) => a.partial_cmp(b),
            (ShouldBe::AndIs(_), ShouldBe::ButIsnt(_)) => Some(std::cmp::Ordering::Greater),
            (ShouldBe::ButIsnt(_), ShouldBe::AndIs(_)) => Some(std::cmp::Ordering::Less),
        }
    }
}

impl<T> Ord for ShouldBe<T>
where
    T: Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (ShouldBe::AndIs(a), ShouldBe::AndIs(b)) => a.cmp(b),
            (ShouldBe::ButIsnt(a), ShouldBe::ButIsnt(b)) => a.cmp(b),
            (ShouldBe::AndIs(_), ShouldBe::ButIsnt(_)) => std::cmp::Ordering::Greater,
            (ShouldBe::ButIsnt(_), ShouldBe::AndIs(_)) => std::cmp::Ordering::Less,
        }
    }
}

impl<T> std::hash::Hash for ShouldBe<T>
where
    T: std::hash::Hash,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            ShouldBe::AndIs(value) => value.hash(state),
            ShouldBe::ButIsnt(why_not) => why_not.hash(state),
        }
    }
}

impl<T> Serialize for ShouldBe<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            ShouldBe::AndIs(value) => value.serialize(serializer),
            ShouldBe::ButIsnt(why_not) => {
                if let Some(raw_value) = why_not.as_ref_raw() {
                    // If we have a raw value, we can serialize it.
                    raw_value.serialize(serializer)
                } else {
                    // Otherwise, we have to raise an error.
                    Err(serde::ser::Error::custom(
                        "Cannot serialize `ShouldBe::ButIsnt` without a raw value",
                    ))
                }
            }
        }
    }
}

impl<'de, T> Deserialize<'de> for ShouldBe<T>
where
    T: DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Communicate to the ValueDeserializers that we are expecting a
        // `ShouldBe` value.
        EXPECTING_SHOULD_BE.with(|cell| *cell.borrow_mut() = true);

        match T::deserialize(deserializer) {
            Ok(value) => Ok(ShouldBe::AndIs(value)),
            Err(err) => {
                if let Some((raw, err)) = take_why_not() {
                    Ok(ShouldBe::ButIsnt(WhyNot::new(Some(raw), err)))
                } else {
                    let err = Error::custom(err);
                    Ok(ShouldBe::ButIsnt(WhyNot::new(None, err)))
                }
            }
        }
    }
}

/// An opaque type that captures the reason why a deserialization to a
/// [`ShouldBe<T>`] failed.
///
/// This type is only meant to be used within the [ShouldBe] type.
#[derive(Clone)]
pub struct WhyNot(Arc<WhyNotImpl>);

struct WhyNotImpl {
    /// The raw value that was attempted to be deserialized.
    ///
    /// This field will *only* be populated when deserializing from a
    /// [Value]. When deserializing from other deserializers, this field
    /// will be `None`.
    raw: Option<crate::Value>,

    /// The original error that occurred during deserialization.
    err: AtomicPtr<Error>,

    /// The string form of `err`
    err_msg: String,
}

impl WhyNot {
    /// Creates a new [WhyNot] from the given raw value and error.
    pub fn new(raw: Option<crate::Value>, err: Error) -> Self {
        let err_msg = err.to_string();
        Self(Arc::new(WhyNotImpl {
            raw,
            err: AtomicPtr::new(Box::into_raw(Box::new(err))),
            err_msg,
        }))
    }

    fn take_err(&self) -> Option<Error> {
        let ptr = self
            .0
            .err
            .swap(std::ptr::null_mut(), atomic::Ordering::SeqCst);
        if ptr.is_null() {
            None
        } else {
            Some(
                // SAFETY:
                // - `ptr` was constructed by [Box::into_raw] and never mutated;
                // - we are the sole owner of the pointer now
                // so it's safe to reconstruct the Box
                unsafe { *Box::from_raw(ptr) },
            )
        }
    }

    fn as_ref_raw(&self) -> Option<&crate::Value> {
        self.0.raw.as_ref()
    }

    fn as_msg(&self) -> &str {
        &self.0.err_msg
    }
}

// ----- Value semantics for WhyNot -----
//
// `WhyNot` instances are treated as the pair `(raw_value: Option<Value>,
// err_msg: String)` for the purposes of equality, ordering, and hashing.
// The `Error` instance is ignored.

impl PartialEq for WhyNot {
    fn eq(&self, other: &Self) -> bool {
        self.as_ref_raw() == other.as_ref_raw() && self.as_msg() == other.as_msg()
    }
}

impl Eq for WhyNot {}

impl PartialOrd for WhyNot {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for WhyNot {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.as_ref_raw().partial_cmp(&other.as_ref_raw()) {
            Some(std::cmp::Ordering::Equal) | None => self.as_msg().cmp(other.as_msg()),
            Some(ord) => ord,
        }
    }
}

impl std::hash::Hash for WhyNot {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_ref_raw().hash(state);
        self.as_msg().hash(state);
    }
}

// --------------------------------------

impl From<WhyNot> for Error {
    fn from(why_not: WhyNot) -> Self {
        if let Some(err) = why_not.take_err() {
            err
        } else {
            Error::custom(why_not.as_msg())
        }
    }
}

impl Drop for WhyNotImpl {
    fn drop(&mut self) {
        let ptr = self
            .err
            .swap(std::ptr::null_mut(), atomic::Ordering::SeqCst);
        if !ptr.is_null() {
            drop(
                // SAFETY:
                // - `ptr` was constructed by [Box::into_raw] and never mutated;
                // - we are the sole owner of the pointer now
                // so it's safe to reconstruct the Box
                unsafe { Box::from_raw(ptr) },
            );
        }
    }
}

impl Debug for WhyNot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WhyNot")
            .field("raw", &self.as_ref_raw())
            .field("err_msg", &self.as_msg())
            .finish()
    }
}

#[cfg(feature = "schemars")]
impl<T> schemars::JsonSchema for ShouldBe<T>
where
    T: schemars::JsonSchema,
{
    fn schema_name() -> String {
        T::schema_name()
    }

    fn json_schema(generator: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        T::json_schema(generator)
    }

    fn is_referenceable() -> bool {
        T::is_referenceable()
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        T::schema_id()
    }

    #[doc(hidden)]
    fn _schemars_private_non_optional_json_schema(
        generator: &mut schemars::gen::SchemaGenerator,
    ) -> schemars::schema::Schema {
        T::_schemars_private_non_optional_json_schema(generator)
    }

    #[doc(hidden)]
    fn _schemars_private_is_option() -> bool {
        T::_schemars_private_is_option()
    }
}

pub(crate) fn is_expecting_should_be_then_reset() -> bool {
    EXPECTING_SHOULD_BE.with(|cell| cell.replace(false))
}

fn take_why_not() -> Option<(Value, Error)> {
    WHY_NOT.with(|cell| cell.borrow_mut().take())
}

pub(crate) fn set_why_not(raw: Value, err: Error) {
    WHY_NOT.with(|cell| *cell.borrow_mut() = Some((raw, err)));
}

thread_local! {
    static EXPECTING_SHOULD_BE: std::cell::RefCell<bool> = const {std::cell::RefCell::new(false)};

    static WHY_NOT: std::cell::RefCell<Option<(Value, Error)>> = const {std::cell::RefCell::new(None)};
}
