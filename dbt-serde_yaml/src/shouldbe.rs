//! This module defines the `ShouldBe` type, which can be used as an error
//! recovery mechanism during deserialization.
//!
//! See the [ShouldBe] documentation for more details.

use std::fmt::Debug;

use serde::{
    de::{DeserializeOwned, Error as _},
    Deserialize, Deserializer, Serialize,
};

use crate::{Error, Value};

/// Represents a value that should be of type `T`, or provides information about
/// why it is not.
///
/// Use this type in `#[derive(Deserialize)]` structs to "containerize" local
/// failures, without failing the entire deserialization process.
///
/// # Example
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
///    assert_eq!(outer.items[1].as_ref_err().unwrap().to_string(),
///               "invalid type: string \"2\", expected i32 at line 4 column 19");
///    assert!(outer.items[2].isnt());
///    assert_eq!(outer.items[2].as_ref_err().unwrap().to_string(),
///               "missing field `field` at line 5 column 12");
///
///    Ok(())
/// }
/// ```
#[derive(Clone)]
pub enum ShouldBe<T> {
    /// On successful deserialization, will contain the expected value of type
    /// `T`.
    AndIs(T),

    /// Failed to deserialize the value into type `T`.
    ButIsnt {
        /// The raw value that was attempted to be deserialized.
        ///
        /// This field will *only* be populated when deserializing from a
        /// [Value]. When deserializing from other deserializers, this field
        /// will be `None`.
        raw: Option<crate::Value>,

        /// Contains the error or custom message corresponding to why the source
        /// value failed to deserialize into type `T`.
        why_not: WhyNot,
    },
}

impl<T> ShouldBe<T> {
    /// Returns a reference to the inner value if it exists
    pub fn as_ref(&self) -> Option<&T> {
        match self {
            ShouldBe::AndIs(value) => Some(value),
            ShouldBe::ButIsnt { raw: _, why_not: _ } => None,
        }
    }

    /// Returns a mutable reference to the inner value if it exists
    pub fn as_ref_mut(&mut self) -> Option<&mut T> {
        match self {
            ShouldBe::AndIs(value) => Some(value),
            ShouldBe::ButIsnt { raw: _, why_not: _ } => None,
        }
    }

    /// Returns a reference to the error if the value is not of type `T`.
    pub fn as_ref_err(&self) -> Option<&Error> {
        match self {
            ShouldBe::AndIs(_) => None,
            ShouldBe::ButIsnt { raw: _, why_not } => match why_not {
                WhyNot::Original(err) => Some(err),
                WhyNot::Custom(_) => None,
            },
        }
    }

    /// Returns a reference to the raw value if it exists.
    pub fn as_ref_raw(&self) -> Option<&crate::Value> {
        match self {
            ShouldBe::AndIs(_) => None,
            ShouldBe::ButIsnt { raw, why_not: _ } => raw.as_ref(),
        }
    }

    /// True if the value is of type `T`, false otherwise.
    pub fn is(&self) -> bool {
        matches!(self, ShouldBe::AndIs(_))
    }

    /// True if the value is not of type `T`, false otherwise.
    pub fn isnt(&self) -> bool {
        matches!(self, ShouldBe::ButIsnt { .. })
    }

    /// Consumes self, returning the inner value if it exists.
    pub fn into_inner(self) -> Option<T> {
        match self {
            ShouldBe::AndIs(value) => Some(value),
            ShouldBe::ButIsnt { raw: _, why_not: _ } => None,
        }
    }

    /// Consumes self, returning the raw value if it exists.
    pub fn into_raw(self) -> Option<crate::Value> {
        match self {
            ShouldBe::AndIs(_) => None,
            ShouldBe::ButIsnt { raw, why_not: _ } => raw,
        }
    }

    /// Extracts the raw value if it exists
    pub fn take_raw(&mut self) -> Option<crate::Value> {
        match self {
            ShouldBe::AndIs(_) => None,
            ShouldBe::ButIsnt { raw, why_not: _ } => raw.take(),
        }
    }

    /// Consumes self, returning the contained [Error].
    ///
    /// Panics if the value is valid (i.e., it is of type `T`).
    pub fn unwrap_err(self) -> Error {
        match self {
            ShouldBe::AndIs(_) => panic!("Called unwrap_err on a value that is valid"),
            ShouldBe::ButIsnt { raw: _, why_not } => why_not.into(),
        }
    }
}

/// Represents the reason why a value does not match the expected type or value.
pub enum WhyNot {
    /// The original error that occurred during deserialization.
    Original(Error),

    /// A custom message explaining why the value does not match the expected type or value.
    Custom(String),
}

impl Clone for WhyNot {
    fn clone(&self) -> Self {
        match self {
            WhyNot::Original(err) => WhyNot::Custom(err.to_string()),
            WhyNot::Custom(msg) => WhyNot::Custom(msg.clone()),
        }
    }
}

impl From<WhyNot> for Error {
    fn from(why_not: WhyNot) -> Self {
        match why_not {
            WhyNot::Original(err) => err,
            WhyNot::Custom(msg) => Error::custom(msg),
        }
    }
}

impl Debug for WhyNot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WhyNot::Original(err) => write!(f, "WhyNot::Original({})", err),
            WhyNot::Custom(msg) => write!(f, "WhyNot::Custom({})", msg),
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
            ShouldBe::ButIsnt { raw, why_not } => {
                write!(
                    f,
                    "ShouldBe::ButIsnt {{ raw: {:?}, why_not: {:?} }}",
                    raw, why_not
                )
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
            ShouldBe::ButIsnt { raw: _, why_not } => Err(why_not.into()),
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
            (ShouldBe::ButIsnt { raw: a, .. }, ShouldBe::ButIsnt { raw: b, .. }) => a == b,
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
            (ShouldBe::ButIsnt { raw: a, .. }, ShouldBe::ButIsnt { raw: b, .. }) => {
                a.partial_cmp(b)
            }
            _ => None,
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
            _ => std::cmp::Ordering::Equal,
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
            ShouldBe::ButIsnt { raw, .. } => raw.hash(state),
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
            ShouldBe::ButIsnt { raw, .. } => {
                if let Some(raw_value) = raw {
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
                    Ok(ShouldBe::ButIsnt {
                        raw: Some(raw),
                        why_not: WhyNot::Original(err),
                    })
                } else {
                    let err = Error::custom(err);
                    Ok(ShouldBe::ButIsnt {
                        raw: None,
                        why_not: WhyNot::Original(err),
                    })
                }
            }
        }
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
