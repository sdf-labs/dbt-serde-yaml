use crate::mapping::{DuplicateKey, MappingVisitor};
use crate::path::Path;
use crate::value::de::borrowed::ValueRefDeserializer;
use crate::value::tagged::TagStringVisitor;
use crate::value::TaggedValue;
use crate::{error, number, spanned, Error, Sequence, Span, Value};
use serde::de::{
    self, Deserialize, DeserializeSeed, Deserializer, EnumAccess, Error as _, Expected, MapAccess,
    SeqAccess, Unexpected, VariantAccess, Visitor,
};
use std::fmt;

mod borrowed;
mod owned;

pub(crate) use borrowed::{MapRefDeserializer, SeqRefDeserializer};
pub use owned::ValueDeserializer;

/// A type alias for the result of transforming a [Value] into another [Value].
pub type TransformedResult =
    Result<Option<Value>, Box<dyn std::error::Error + 'static + Send + Sync>>;

impl Value {
    /// Deserialize a [Value] from a string of YAML text.
    pub fn from_str<F>(s: &str, duplicate_key_callback: F) -> Result<Self, Error>
    where
        F: FnMut(Path<'_>, &Self, &Self) -> DuplicateKey,
    {
        let de = crate::de::Deserializer::from_str(s);
        spanned::set_marker(spanned::Marker::start());
        let res = deserialize(de, duplicate_key_callback);
        spanned::reset_marker();
        res
    }

    /// Deserialize a [Value] from an IO stream of YAML text.
    pub fn from_reader<R, F>(rdr: R, duplicate_key_callback: F) -> Result<Self, Error>
    where
        R: std::io::Read,
        F: FnMut(Path<'_>, &Self, &Self) -> DuplicateKey,
    {
        let de = crate::de::Deserializer::from_reader(rdr);
        spanned::set_marker(spanned::Marker::start());
        let res = deserialize(de, duplicate_key_callback);
        spanned::reset_marker();
        res
    }

    /// Deserialize a [Value] from a byte slice of YAML text.
    pub fn from_slice<F>(s: &[u8], duplicate_key_callback: F) -> Result<Self, Error>
    where
        F: FnMut(Path<'_>, &Self, &Self) -> DuplicateKey,
    {
        let de = crate::de::Deserializer::from_slice(s);
        spanned::set_marker(spanned::Marker::start());
        let res = deserialize(de, duplicate_key_callback);
        spanned::reset_marker();
        res
    }

    /// Deserialize a [Value] into an instance of some [Deserialize] type `T`.
    pub fn into_typed<'de, T, U, F>(
        self,
        mut unused_key_callback: U,
        mut field_transformer: F,
    ) -> Result<T, Error>
    where
        T: Deserialize<'de>,
        U: FnMut(Path<'_>, &Value, &Value),
        F: for<'v> FnMut(&'v Value) -> TransformedResult,
    {
        let de = ValueDeserializer::new_with(
            self,
            Path::Root,
            Some(&mut unused_key_callback),
            Some(&mut field_transformer),
        );

        T::deserialize(de)
    }

    /// Deserialize a [Value] into an instance of some [Deserialize] type `T`,
    /// without consuming the [Value].
    pub fn to_typed<'de, T, U, F>(
        &'de self,
        mut unused_key_callback: U,
        mut field_transformer: F,
    ) -> Result<T, Error>
    where
        T: Deserialize<'de>,
        U: FnMut(Path<'_>, &Value, &Value),
        F: for<'v> FnMut(&'v Value) -> TransformedResult,
    {
        let de = ValueRefDeserializer::new_with(
            self,
            Path::Root,
            Some(&mut unused_key_callback),
            Some(&mut field_transformer),
        );
        T::deserialize(de)
    }
}

pub(crate) struct ValueVisitor<'a, 'b, F: FnMut(Path<'_>, &Value, &Value) -> DuplicateKey> {
    pub callback: &'a mut F,
    pub path: Path<'b>,
}

impl<'de, F> serde::de::Visitor<'de> for ValueVisitor<'_, '_, F>
where
    F: FnMut(Path<'_>, &Value, &Value) -> DuplicateKey,
{
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("any valid YAML value")
    }

    fn visit_bool<E>(self, b: bool) -> Result<Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::bool(b))
    }

    fn visit_i64<E>(self, i: i64) -> Result<Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::number(i.into()))
    }

    fn visit_u64<E>(self, u: u64) -> Result<Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::number(u.into()))
    }

    fn visit_f64<E>(self, f: f64) -> Result<Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::number(f.into()))
    }

    fn visit_str<E>(self, s: &str) -> Result<Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::string(s.to_owned()))
    }

    fn visit_string<E>(self, s: String) -> Result<Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::string(s))
    }

    fn visit_unit<E>(self) -> Result<Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::null())
    }

    fn visit_none<E>(self) -> Result<Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::null())
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer)
    }

    fn visit_seq<A>(self, data: A) -> Result<Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let de = serde::de::value::SeqAccessDeserializer::new(data);
        let visitor = SequenceVisitor {
            callback: &mut *self.callback,
            path: self.path,
        };
        let sequence = de.deserialize_seq(visitor)?;
        Ok(Value::sequence(sequence))
    }

    fn visit_map<A>(self, data: A) -> Result<Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let de = serde::de::value::MapAccessDeserializer::new(data);
        let visitor = MappingVisitor {
            callback: &mut *self.callback,
            path: self.path,
        };
        let mapping = de.deserialize_map(visitor)?;
        Ok(Value::mapping(mapping))
    }

    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: EnumAccess<'de>,
    {
        let (tag, contents) = data.variant_seed(TagStringVisitor)?;
        let value = contents.newtype_variant()?;
        Ok(Value::tagged(TaggedValue { tag, value }))
    }
}

impl<'de, F> DeserializeSeed<'de> for ValueVisitor<'_, '_, F>
where
    F: FnMut(Path<'_>, &Value, &Value) -> DuplicateKey,
{
    type Value = Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let start = spanned::get_marker();
        let val = deserializer.deserialize_any(self)?;
        let span = Span::from(start..spanned::get_marker());

        #[cfg(feature = "filename")]
        let span = span.maybe_capture_filename();

        Ok(val.with_span(span))
    }
}

struct SequenceVisitor<'a, 'b, F> {
    pub callback: &'a mut F,
    pub path: Path<'b>,
}

impl<'de, F> serde::de::Visitor<'de> for SequenceVisitor<'_, '_, F>
where
    F: FnMut(Path<'_>, &Value, &Value) -> DuplicateKey,
{
    type Value = Sequence;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a sequence")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        let mut idx = 0;
        while let Some(value) = seq.next_element_seed(ValueVisitor {
            callback: &mut *self.callback,
            path: Path::Seq {
                parent: &self.path,
                index: idx,
            },
        })? {
            idx += 1;
            values.push(value);
        }

        Ok(values)
    }
}

fn deserialize<'de, D, F>(deserializer: D, mut duplicate_key_callback: F) -> Result<Value, D::Error>
where
    D: serde::Deserializer<'de>,
    F: FnMut(Path<'_>, &Value, &Value) -> DuplicateKey,
{
    let start = spanned::get_marker();
    set_is_deserializing_value();
    let res = deserializer.deserialize_any(ValueVisitor {
        callback: &mut duplicate_key_callback,
        path: Path::Root,
    });
    reset_is_deserializing_value();
    // Fast path: if the deserializer has returned a value through the side
    // channel, then we use it and ignore the result of the deserializer.
    if let Some(value) = THE_VALUE.with(|cell| cell.take()) {
        return Ok(value);
    }

    let val = res?;
    let span = Span::from(start..spanned::get_marker());

    #[cfg(feature = "filename")]
    let span = span.maybe_capture_filename();

    Ok(val.with_span(span))
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize(deserializer, |_, _, _| DuplicateKey::Error)
    }
}

macro_rules! maybe_why_not {
    ($value_ref:expr, $res:expr) => {{
        let is_expecting_should_be = $crate::shouldbe::is_expecting_should_be_then_reset();
        let res = $res;
        match res {
            Err(err) if is_expecting_should_be => {
                let msg = err.to_string();
                $crate::shouldbe::set_why_not($value_ref.clone(), err);
                // This error will be ignored by ShouldBe, but we still have to
                // return an error here nonetheless.
                Err(Error::custom(msg))
            }
            _ => res,
        }
    }};
}
pub(crate) use maybe_why_not;

impl Value {
    fn deserialize_number<'de, V>(&self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        reset_is_deserializing_value();
        let span = self.span();
        self.broadcast_end_mark();
        maybe_why_not!(
            self,
            match self.untag_ref() {
                Value::Number(n, ..) => n.deserialize_any(visitor),
                other => Err(other.invalid_type(&visitor)),
            }
            .map_err(|e| error::set_span(e, span))
        )
    }

    #[cold]
    fn invalid_type(&self, exp: &dyn Expected) -> Error {
        error::set_span(de::Error::invalid_type(self.unexpected(), exp), self.span())
    }

    #[cold]
    pub(crate) fn unexpected(&self) -> Unexpected<'_> {
        match self {
            Value::Null(..) => Unexpected::Unit,
            Value::Bool(b, ..) => Unexpected::Bool(*b),
            Value::Number(n, ..) => number::unexpected(n),
            Value::String(s, ..) => Unexpected::Str(s),
            Value::Sequence(..) => Unexpected::Seq,
            Value::Mapping(..) => Unexpected::Map,
            Value::Tagged(..) => Unexpected::Enum,
        }
    }
}

fn is_deserializing_value_then_reset() -> bool {
    IS_DESERIALIZING_VALUE.with(|cell| cell.replace(false))
}

fn set_is_deserializing_value() {
    IS_DESERIALIZING_VALUE.with(|cell| cell.set(true));
}
fn reset_is_deserializing_value() {
    IS_DESERIALIZING_VALUE.with(|cell| cell.set(false));
}

fn store_deserializer_state<U, F>(
    value: Option<Value>,
    _path: Path<'_>,
    unused_key_callback: Option<&mut U>,
    field_transformer: Option<&mut F>,
) where
    U: for<'p, 'v> FnMut(Path<'p>, &'v Value, &'v Value),
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
{
    THE_VALUE.with(|cell| cell.set(value));
    UNUSED_KEY_CALLBACK.with(|cell| {
        cell.set(unused_key_callback.map(|cb| unsafe {
            std::mem::transmute(
                Box::new(cb) as Box<dyn for<'p, 'v> FnMut(Path<'p>, &'v Value, &'v Value)>
            )
        }))
    });
    FIELD_TRANSFORMER.with(|cell| {
        cell.set(field_transformer.map(|cb| unsafe {
            std::mem::transmute(
                Box::new(cb) as Box<dyn for<'v> FnMut(&'v Value) -> TransformedResult>
            )
        }))
    });
}

type UnusedKeyCallback = Box<dyn for<'p, 'v> FnMut(Path<'p>, &'v Value, &'v Value)>;
type FieldTransformer = Box<dyn for<'v> FnMut(&'v Value) -> TransformedResult>;

thread_local! {
    static IS_DESERIALIZING_VALUE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };

    static THE_VALUE: std::cell::Cell<Option<Value>> = const { std::cell::Cell::new(None) };
    static THE_PATH: std::cell::Cell<Path<'static>> = const { std::cell::Cell::new(Path::Root) };
    static UNUSED_KEY_CALLBACK: std::cell::Cell<Option<UnusedKeyCallback>> = std::cell::Cell::new(
        None
    );
    static FIELD_TRANSFORMER: std::cell::Cell<Option<FieldTransformer>> = std::cell::Cell::new(
        None
    );
}
