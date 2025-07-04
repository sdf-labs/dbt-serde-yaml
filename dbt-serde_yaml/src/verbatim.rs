//! This module defines the `Verbatim` type, which is a wrapper type that can be
//! used to in `#[derive(Deserialize)]` structs to protect fields from the
//! `field_transfomer` when deserialized by the `Value::into_typed` method.

use std::{
    fmt::{self, Debug},
    hash::Hash,
    hash::Hasher,
    ops::{Deref, DerefMut},
};

use serde::{de::Error as _, de::Visitor, Deserialize, Deserializer, Serialize, Serializer};

use crate::{value::TransformedResult, Path, Value};

/// A wrapper type that protects the inner value from being transformed by the
/// `field_transformer` when deserialized by the `Value::into_typed` method
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Hash, Default)]
pub struct Verbatim<T> {
    inner: Option<Value>,
    phantom: std::marker::PhantomData<T>,
}

impl<T> Verbatim<T> {
    /// Creates a new `Verbatim` instance from a `Value`.
    pub fn new(value: Value) -> Self {
        Verbatim {
            inner: Some(value),
            phantom: std::marker::PhantomData,
        }
    }

    /// Creates a new `Verbatim` instance that represents a missing value.
    pub fn new_missing() -> Self {
        Verbatim {
            inner: None,
            phantom: std::marker::PhantomData,
        }
    }

    /// Returns a reference to the inner `Value`, if it exists.
    pub fn as_ref(&self) -> Option<&Value> {
        self.inner.as_ref()
    }

    /// Unwraps the [Verbatim], returning the inner `Value` if present.
    pub fn into_inner(self) -> Option<Value> {
        self.inner
    }

    /// Returns true if this [Verbatim] instance represents a missing value.
    pub fn is_missing(&self) -> bool {
        self.inner.is_none()
    }

    /// Returns true if this [Verbatim] instance contains a value.
    pub fn is_present(&self) -> bool {
        self.inner.is_some()
    }
}

impl<'de, T> Verbatim<T>
where
    T: Deserialize<'de>,
{
    /// Deserialize this [Verbatim] instance into the target type `T`.
    pub fn into_typed<U, F>(
        self,
        unused_key_callback: U,
        field_transformer: F,
    ) -> Result<T, crate::Error>
    where
        U: FnMut(Path<'_>, &Value, &Value),
        F: for<'v> FnMut(&'v Value) -> TransformedResult,
    {
        if let Some(value) = self.inner {
            value.into_typed(unused_key_callback, field_transformer)
        } else {
            T::deserialize(MissingFieldDeserializer)
        }
    }

    /// Deserialize this [Verbatim] instance into the target type `T`
    pub fn into_typed_default(self) -> Result<T, crate::Error> {
        self.into_typed(|_, _, _| {}, |_| Ok(None))
    }

    /// Deserialize this [Verbatim] instance to the target type `T`, without
    /// consuming it.
    pub fn to_typed<U, F>(
        &'de self,
        unused_key_callback: U,
        field_transformer: F,
    ) -> Result<T, crate::Error>
    where
        U: FnMut(Path<'_>, &Value, &Value),
        F: for<'v> FnMut(&'v Value) -> TransformedResult,
    {
        if let Some(value) = &self.inner {
            value.to_typed(unused_key_callback, field_transformer)
        } else {
            T::deserialize(MissingFieldDeserializer)
        }
    }

    /// Deserialize this [Verbatim] instance to the target type `T`, without consuming it,
    pub fn to_typed_default(&'de self) -> Result<T, crate::Error> {
        self.to_typed(|_, _, _| {}, |_| Ok(None))
    }
}

impl<T> Deref for Verbatim<T> {
    type Target = Option<Value>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for Verbatim<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T> From<Value> for Verbatim<T> {
    fn from(value: Value) -> Self {
        Verbatim::new(value)
    }
}

impl<T> Serialize for Verbatim<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.inner.serialize(serializer)
    }
}

struct MissingFieldDeserializer;

impl<'de> Deserializer<'de> for MissingFieldDeserializer {
    type Error = crate::Error;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Self::Error::custom("missing field"))
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_none()
    }

    // Other methods are not needed for this deserializer.
    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes byte_buf
        unit unit_struct newtype_struct seq tuple tuple_struct map struct
        enum identifier ignored_any
    }
}

impl<'de, T> Deserialize<'de> for Verbatim<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let _g = with_should_not_transform_any();

        let maybe_value = Value::deserialize(deserializer);

        match maybe_value {
            Ok(value) => Ok(Verbatim::new(value)),
            Err(err) => {
                let msg = err.to_string();
                // missing field errors must be handled specially, as dictated by T:
                if msg.starts_with("missing field ")
                    && T::deserialize(MissingFieldDeserializer).is_ok()
                {
                    // If T can be deserialized from a missing field, then we
                    // retain the missing field in the Verbatim value:
                    Ok(Verbatim::new_missing())
                } else {
                    // Otherwise, we propagate the error.
                    Err(err)
                }
            }
        }
    }
}

#[cfg(feature = "schemars")]
impl<T> schemars::JsonSchema for Verbatim<T>
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

/// A wrapper type that protects the inner value from being transformed by the
/// `field_transformer` when deserialized by the `Value::into_typed` method.
///
/// @Deprecated This type is deprecated and will be removed in a future version.
/// Use [Verbatim] instead.
#[repr(transparent)]
pub struct VerbatimLegacy<T>(pub T);

impl<T> Deref for VerbatimLegacy<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for VerbatimLegacy<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> Clone for VerbatimLegacy<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        VerbatimLegacy(self.0.clone())
    }
}

impl<T> Copy for VerbatimLegacy<T> where T: Copy {}

impl<T> Debug for VerbatimLegacy<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T> PartialEq for VerbatimLegacy<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T> Eq for VerbatimLegacy<T> where T: Eq {}

impl<T> PartialOrd for VerbatimLegacy<T>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<T> Ord for VerbatimLegacy<T>
where
    T: Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T> Hash for VerbatimLegacy<T>
where
    T: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl<T> Default for VerbatimLegacy<T>
where
    T: Default,
{
    fn default() -> Self {
        VerbatimLegacy(T::default())
    }
}

impl<T> From<T> for VerbatimLegacy<T> {
    fn from(value: T) -> Self {
        VerbatimLegacy(value)
    }
}

impl<T> Serialize for VerbatimLegacy<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for VerbatimLegacy<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let _g = with_should_not_transform_any();
        T::deserialize(deserializer).map(VerbatimLegacy)
    }
}

#[cfg(feature = "schemars")]
impl<T> schemars::JsonSchema for VerbatimLegacy<T>
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

pub(crate) fn should_transform_any() -> bool {
    SHOULD_TRANSFORM_ANY.with(|flag| flag.get())
}

struct ShouldTransformAnyGuard(bool);

impl Drop for ShouldTransformAnyGuard {
    fn drop(&mut self) {
        SHOULD_TRANSFORM_ANY.with(|flag| flag.set(self.0));
    }
}

fn with_should_not_transform_any() -> ShouldTransformAnyGuard {
    let current = SHOULD_TRANSFORM_ANY.with(|flag| flag.get());
    SHOULD_TRANSFORM_ANY.with(|flag| flag.set(false));
    ShouldTransformAnyGuard(current)
}

thread_local! {
    static SHOULD_TRANSFORM_ANY: std::cell::Cell<bool>  = const {
        std::cell::Cell::new(true)
    };
}
