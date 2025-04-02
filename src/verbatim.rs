//! This module defines the `Verbatim` type, which is a wrapper type that can be
//! used to in `#[derive(Deserialize)]` structs to protect fields from the
//! `field_transfomer` when deserialized by the `Value::into_typed` method.

use std::{
    fmt::{self, Debug},
    hash::Hash,
    hash::Hasher,
    ops::{Deref, DerefMut},
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A wrapper type that protects the inner value from being transformed by the
/// `field_transformer` when deserialized by the `Value::into_typed` method.
pub struct Verbatim<T>(pub T);

impl<T> Deref for Verbatim<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Verbatim<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> Clone for Verbatim<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Verbatim(self.0.clone())
    }
}

impl<T> Copy for Verbatim<T> where T: Copy {}

impl<T> Debug for Verbatim<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T> PartialEq for Verbatim<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T> Eq for Verbatim<T> where T: Eq {}

impl<T> PartialOrd for Verbatim<T>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<T> Ord for Verbatim<T>
where
    T: Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T> Hash for Verbatim<T>
where
    T: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

impl<T> Default for Verbatim<T>
where
    T: Default,
{
    fn default() -> Self {
        Verbatim(T::default())
    }
}

impl<T> From<T> for Verbatim<T> {
    fn from(value: T) -> Self {
        Verbatim(value)
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
        self.0.serialize(serializer)
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
        T::deserialize(deserializer).map(Verbatim)
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
