use crate::error;
use crate::mapping::{DuplicateKey, MappingVisitor};
use crate::value::tagged::{self, TagStringVisitor};
use crate::value::TaggedValue;
use crate::{number, spanned, Error, Mapping, Sequence, Span, Value};
use serde::de::value::{BorrowedStrDeserializer, StrDeserializer};
use serde::de::{
    self, Deserialize, DeserializeSeed, Deserializer, EnumAccess, Error as _, Expected, MapAccess,
    SeqAccess, Unexpected, VariantAccess, Visitor,
};
use serde::forward_to_deserialize_any;
use std::collections::HashSet;
use std::fmt;
use std::slice;
use std::vec;

impl Value {
    /// Deserialize a [Value] from a string of YAML text.
    pub fn from_str<F>(s: &str, duplicate_key_callback: F) -> Result<Self, Error>
    where
        F: FnMut(&Self) -> DuplicateKey,
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
        F: FnMut(&Self) -> DuplicateKey,
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
        F: FnMut(&Self) -> DuplicateKey,
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
        U: FnMut(Value, Value),
        F: FnMut(Value) -> Result<Value, Box<dyn std::error::Error + 'static + Send + Sync>>,
    {
        let de = ValueDeserializer::new_with(
            self,
            Some(&mut unused_key_callback),
            Some(&mut field_transformer),
        );

        T::deserialize(de)
    }
}

pub(crate) struct ValueVisitor<'a, F: FnMut(&Value) -> DuplicateKey>(pub &'a mut F);

impl<'de, F> serde::de::Visitor<'de> for ValueVisitor<'_, F>
where
    F: FnMut(&Value) -> DuplicateKey,
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
        let visitor = SequenceVisitor(&mut *self.0);
        let sequence = de.deserialize_seq(visitor)?;
        Ok(Value::sequence(sequence))
    }

    fn visit_map<A>(self, data: A) -> Result<Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let de = serde::de::value::MapAccessDeserializer::new(data);
        let visitor = MappingVisitor(&mut *self.0);
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

impl<'de, F> DeserializeSeed<'de> for ValueVisitor<'_, F>
where
    F: FnMut(&Value) -> DuplicateKey,
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

struct SequenceVisitor<'a, F>(pub &'a mut F);

impl<'de, F> serde::de::Visitor<'de> for SequenceVisitor<'_, F>
where
    F: FnMut(&Value) -> DuplicateKey,
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
        while let Some(value) = seq.next_element_seed(ValueVisitor(&mut *self.0))? {
            values.push(value);
        }
        Ok(values)
    }
}

fn deserialize<'de, D, F>(deserializer: D, mut duplicate_key_callback: F) -> Result<Value, D::Error>
where
    D: serde::Deserializer<'de>,
    F: FnMut(&Value) -> DuplicateKey,
{
    let start = spanned::get_marker();
    let val = deserializer.deserialize_any(ValueVisitor(&mut duplicate_key_callback))?;
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
        let start = spanned::get_marker();
        let val = deserializer.deserialize_any(ValueVisitor(&mut |_| DuplicateKey::Error))?;
        let span = Span::from(start..spanned::get_marker());

        #[cfg(feature = "filename")]
        let span = span.maybe_capture_filename();

        Ok(val.with_span(span))
    }
}

impl Value {
    fn deserialize_number<'de, V>(&self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let span = self.span();
        self.broadcast_end_mark();
        match self.untag_ref() {
            Value::Number(n, ..) => n.deserialize_any(visitor),
            other => Err(other.invalid_type(&visitor)),
        }
        .map_err(|e| error::set_span(e, span))
    }
}

fn visit_sequence<'de, 'a, V, U, F>(
    sequence: Sequence,
    visitor: V,
    unused_key_callback: Option<&'a mut U>,
    field_transformer: Option<&'a mut F>,
) -> Result<V::Value, Error>
where
    V: Visitor<'de>,
    U: FnMut(Value, Value),
    F: FnMut(Value) -> Result<Value, Box<dyn std::error::Error + 'static + Send + Sync>>,
{
    let len = sequence.len();
    let mut deserializer = SeqDeserializer::new(sequence, unused_key_callback, field_transformer);
    let seq = visitor.visit_seq(&mut deserializer)?;
    let remaining = deserializer.iter.len();
    if remaining == 0 {
        Ok(seq)
    } else {
        Err(Error::invalid_length(len, &"fewer elements in sequence"))
    }
}

fn visit_sequence_ref<'de, V>(sequence: &'de Sequence, visitor: V) -> Result<V::Value, Error>
where
    V: Visitor<'de>,
{
    let len = sequence.len();
    let mut deserializer = SeqRefDeserializer::new(sequence);
    let seq = visitor.visit_seq(&mut deserializer)?;
    let remaining = deserializer.iter.len();
    if remaining == 0 {
        Ok(seq)
    } else {
        Err(Error::invalid_length(len, &"fewer elements in sequence"))
    }
}

fn visit_mapping<'de, 'a, V, U, F>(
    mapping: Mapping,
    visitor: V,
    unused_key_callback: Option<&'a mut U>,
    field_transformer: Option<&'a mut F>,
) -> Result<V::Value, Error>
where
    V: Visitor<'de>,
    U: FnMut(Value, Value),
    F: FnMut(Value) -> Result<Value, Box<dyn std::error::Error + 'static + Send + Sync>>,
{
    let len = mapping.len();
    let mut deserializer = MapDeserializer::new(mapping, unused_key_callback, field_transformer);
    let map = visitor.visit_map(&mut deserializer)?;
    let remaining = deserializer.iter.len();
    if remaining == 0 {
        Ok(map)
    } else {
        Err(Error::invalid_length(len, &"fewer elements in map"))
    }
}

fn visit_struct<'de, 'a, V, U, F>(
    mapping: Mapping,
    visitor: V,
    known_keys: &'static [&'static str],
    unused_key_callback: Option<&'a mut U>,
    field_transformer: Option<&'a mut F>,
) -> Result<V::Value, Error>
where
    V: Visitor<'de>,
    U: FnMut(Value, Value),
    F: FnMut(Value) -> Result<Value, Box<dyn std::error::Error + 'static + Send + Sync>>,
{
    let len = mapping.len();
    let mut deserializer =
        StructDeserializer::new(mapping, known_keys, unused_key_callback, field_transformer);
    let map = visitor.visit_map(&mut deserializer)?;
    let remaining = deserializer.iter.len() + deserializer.rest.len();
    if remaining == 0 {
        Ok(map)
    } else {
        Err(Error::invalid_length(len, &"fewer elements in struct"))
    }
}

fn visit_mapping_ref<'de, V>(mapping: &'de Mapping, visitor: V) -> Result<V::Value, Error>
where
    V: Visitor<'de>,
{
    let len = mapping.len();
    let mut deserializer = MapRefDeserializer::new(mapping);
    let map = visitor.visit_map(&mut deserializer)?;
    let remaining = deserializer.iter.unwrap().len();
    if remaining == 0 {
        Ok(map)
    } else {
        Err(Error::invalid_length(len, &"fewer elements in map"))
    }
}

impl<'de> Deserializer<'de> for Value {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer::new(self).deserialize_any(visitor)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer::new(self).deserialize_bool(visitor)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer::new(self).deserialize_i8(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer::new(self).deserialize_i16(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer::new(self).deserialize_i32(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer::new(self).deserialize_i64(visitor)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer::new(self).deserialize_u8(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer::new(self).deserialize_u16(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer::new(self).deserialize_u32(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer::new(self).deserialize_u64(visitor)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer::new(self).deserialize_f32(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer::new(self).deserialize_f64(visitor)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer::new(self).deserialize_char(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer::new(self).deserialize_str(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer::new(self).deserialize_string(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer::new(self).deserialize_bytes(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer::new(self).deserialize_byte_buf(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer::new(self).deserialize_option(visitor)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer::new(self).deserialize_unit(visitor)
    }

    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer::new(self).deserialize_unit_struct(name, visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer::new(self).deserialize_newtype_struct(name, visitor)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer::new(self).deserialize_seq(visitor)
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer::new(self).deserialize_tuple(len, visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer::new(self).deserialize_tuple_struct(name, len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer::new(self).deserialize_map(visitor)
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer::new(self).deserialize_struct(name, fields, visitor)
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer::new(self).deserialize_enum(name, variants, visitor)
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer::new(self).deserialize_identifier(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueDeserializer::new(self).deserialize_ignored_any(visitor)
    }
}

pub struct ValueDeserializer<'a, U, F> {
    value: Value,
    unused_key_callback: Option<&'a mut U>,
    field_transformer: Option<&'a mut F>,
    // Flag indicating whether the value has been already been transformed by
    // field_transformer:
    is_transformed: bool,
}

impl<'a>
    ValueDeserializer<
        'a,
        fn(Value, Value),
        fn(Value) -> Result<Value, Box<dyn std::error::Error + 'static + Send + Sync>>,
    >
{
    pub(crate) fn new(value: Value) -> Self {
        ValueDeserializer {
            value,
            unused_key_callback: None,
            field_transformer: None,
            is_transformed: false,
        }
    }
}

impl<'a, U, F> ValueDeserializer<'a, U, F> {
    fn new_with(
        value: Value,
        unused_key_callback: Option<&'a mut U>,
        field_transformer: Option<&'a mut F>,
    ) -> Self {
        ValueDeserializer {
            value,
            unused_key_callback,
            field_transformer,
            is_transformed: false,
        }
    }
}

impl<U, F> ValueDeserializer<'_, U, F>
where
    U: FnMut(Value, Value),
    F: FnMut(Value) -> Result<Value, Box<dyn std::error::Error + 'static + Send + Sync>>,
{
    fn maybe_apply_transformation(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error + 'static + Send + Sync>> {
        if let Some(transformer) = &mut self.field_transformer {
            if !self.is_transformed && crate::verbatim::should_transform_any() {
                self.value = transformer(std::mem::take(&mut self.value))?;
            }
        }
        Ok(())
    }
}

impl<'de, U, F> Deserializer<'de> for ValueDeserializer<'_, U, F>
where
    U: FnMut(Value, Value),
    F: FnMut(Value) -> Result<Value, Box<dyn std::error::Error + 'static + Send + Sync>>,
{
    type Error = Error;

    fn deserialize_any<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.maybe_apply_transformation()?;
        let span = self.value.span();
        self.value.broadcast_end_mark();
        match self.value {
            Value::Null(..) => visitor.visit_unit(),
            Value::Bool(v, ..) => visitor.visit_bool(v),
            Value::Number(n, ..) => n.deserialize_any(visitor),
            Value::String(v, ..) => visitor.visit_string(v),
            Value::Sequence(v, ..) => {
                visit_sequence(v, visitor, self.unused_key_callback, self.field_transformer)
            }
            Value::Mapping(v, ..) => {
                visit_mapping(v, visitor, self.unused_key_callback, self.field_transformer)
            }
            Value::Tagged(tagged, ..) => visitor.visit_enum(*tagged),
        }
        .map_err(|e| error::set_span(e, span))
    }

    fn deserialize_bool<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.maybe_apply_transformation()?;
        let span = self.value.span();
        self.value.broadcast_end_mark();
        match self.value.untag() {
            Value::Bool(v, ..) => visitor.visit_bool(v),
            other => Err(other.invalid_type(&visitor)),
        }
        .map_err(|e| error::set_span(e, span))
    }

    fn deserialize_i8<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.maybe_apply_transformation()?;
        self.value.deserialize_number(visitor)
    }

    fn deserialize_i16<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.maybe_apply_transformation()?;
        self.value.deserialize_number(visitor)
    }

    fn deserialize_i32<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.maybe_apply_transformation()?;
        self.value.deserialize_number(visitor)
    }

    fn deserialize_i64<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.maybe_apply_transformation()?;
        self.value.deserialize_number(visitor)
    }

    fn deserialize_i128<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.maybe_apply_transformation()?;
        self.value.deserialize_number(visitor)
    }

    fn deserialize_u8<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.maybe_apply_transformation()?;
        self.value.deserialize_number(visitor)
    }

    fn deserialize_u16<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.maybe_apply_transformation()?;
        self.value.deserialize_number(visitor)
    }

    fn deserialize_u32<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.maybe_apply_transformation()?;
        self.value.deserialize_number(visitor)
    }

    fn deserialize_u64<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.maybe_apply_transformation()?;
        self.value.deserialize_number(visitor)
    }

    fn deserialize_u128<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.maybe_apply_transformation()?;
        self.value.deserialize_number(visitor)
    }

    fn deserialize_f32<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.maybe_apply_transformation()?;
        self.value.deserialize_number(visitor)
    }

    fn deserialize_f64<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.maybe_apply_transformation()?;
        self.value.deserialize_number(visitor)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_string<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.maybe_apply_transformation()?;
        let span = self.value.span();
        self.value.broadcast_end_mark();
        match self.value.untag() {
            Value::String(v, ..) => visitor.visit_string(v),
            other => Err(other.invalid_type(&visitor)),
        }
        .map_err(|e| error::set_span(e, span))
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_byte_buf(visitor)
    }

    fn deserialize_byte_buf<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.maybe_apply_transformation()?;
        let span = self.value.span();
        self.value.broadcast_end_mark();
        match self.value.untag() {
            Value::String(v, ..) => visitor.visit_string(v),
            Value::Sequence(v, ..) => {
                visit_sequence(v, visitor, self.unused_key_callback, self.field_transformer)
            }
            other => Err(other.invalid_type(&visitor)),
        }
        .map_err(|e| error::set_span(e, span))
    }

    fn deserialize_option<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.maybe_apply_transformation()?;
        let span = self.value.span();
        self.value.broadcast_end_mark();
        match self.value {
            Value::Null(..) => visitor.visit_none(),
            _ => visitor.visit_some(ValueDeserializer::<U, F> {
                value: self.value,
                unused_key_callback: self.unused_key_callback,
                field_transformer: self.field_transformer,
                is_transformed: true,
            }),
        }
        .map_err(|e| error::set_span(e, span))
    }

    fn deserialize_unit<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.maybe_apply_transformation()?;
        let span = self.value.span();
        self.value.broadcast_end_mark();
        match self.value {
            Value::Null(..) => visitor.visit_unit(),
            _ => Err(self.value.invalid_type(&visitor)),
        }
        .map_err(|e| error::set_span(e, span))
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let span = self.value.span();
        self.value.broadcast_end_mark();
        visitor
            .visit_newtype_struct(self)
            .map_err(|e| error::set_span(e, span))
    }

    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.maybe_apply_transformation()?;
        let span = self.value.span();
        self.value.broadcast_end_mark();
        match self.value.untag() {
            Value::Sequence(v, ..) => {
                visit_sequence(v, visitor, self.unused_key_callback, self.field_transformer)
            }
            Value::Null(..) => visit_sequence(
                Sequence::new(),
                visitor,
                self.unused_key_callback,
                self.field_transformer,
            ),
            other => Err(other.invalid_type(&visitor)),
        }
        .map_err(|e| error::set_span(e, span))
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.maybe_apply_transformation()?;
        let span = self.value.span();
        self.value.broadcast_end_mark();
        match self.value.untag() {
            Value::Mapping(v, ..) => {
                visit_mapping(v, visitor, self.unused_key_callback, self.field_transformer)
            }
            Value::Null(..) => visit_mapping(
                Mapping::new(),
                visitor,
                self.unused_key_callback,
                self.field_transformer,
            ),
            other => Err(other.invalid_type(&visitor)),
        }
        .map_err(|e| error::set_span(e, span))
    }

    fn deserialize_struct<V>(
        mut self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.maybe_apply_transformation()?;
        let span = self.value.span();
        self.value.broadcast_end_mark();
        match self.value.untag() {
            Value::Mapping(v, ..) => visit_struct(
                v,
                visitor,
                fields,
                self.unused_key_callback,
                self.field_transformer,
            ),
            Value::Null(..) => visit_struct(
                Mapping::new(),
                visitor,
                fields,
                self.unused_key_callback,
                self.field_transformer,
            ),
            other => Err(other.invalid_type(&visitor)),
        }
        .map_err(|e| error::set_span(e, span))
    }

    fn deserialize_enum<V>(
        mut self,
        _name: &str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.maybe_apply_transformation()?;
        let span = self.value.span();
        self.value.broadcast_end_mark();

        let tag;
        visitor
            .visit_enum(match self.value {
                Value::Tagged(tagged, ..) => EnumDeserializer {
                    tag: {
                        tag = tagged.tag.string;
                        tagged::nobang(&tag)
                    },
                    value: Some(tagged.value),
                    unused_key_callback: self.unused_key_callback,
                    field_transformer: self.field_transformer,
                },
                Value::String(variant, ..) => EnumDeserializer {
                    tag: {
                        tag = variant;
                        &tag
                    },
                    value: None,
                    unused_key_callback: self.unused_key_callback,
                    field_transformer: self.field_transformer,
                },
                other => {
                    return Err(Error::invalid_type(
                        other.unexpected(),
                        &"a Value::Tagged enum",
                    ));
                }
            })
            .map_err(|e| error::set_span(e, span))
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.value.broadcast_end_mark();
        let span = self.value.span();
        drop(self);
        visitor.visit_unit().map_err(|e| error::set_span(e, span))
    }
}

struct EnumDeserializer<'a, U, F> {
    tag: &'a str,
    value: Option<Value>,
    unused_key_callback: Option<&'a mut U>,
    field_transformer: Option<&'a mut F>,
}

impl<'de, 'a, U, F> EnumAccess<'de> for EnumDeserializer<'a, U, F>
where
    U: FnMut(Value, Value),
    F: FnMut(Value) -> Result<Value, Box<dyn std::error::Error + 'static + Send + Sync>>,
{
    type Error = Error;
    type Variant = VariantDeserializer<'a, U, F>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Error>
    where
        V: DeserializeSeed<'de>,
    {
        let str_de = StrDeserializer::<Error>::new(self.tag);
        let variant = seed.deserialize(str_de)?;
        let visitor = VariantDeserializer {
            value: self.value,
            unused_key_callback: self.unused_key_callback,
            field_transformer: self.field_transformer,
        };
        Ok((variant, visitor))
    }
}

struct VariantDeserializer<'a, U, F> {
    value: Option<Value>,
    unused_key_callback: Option<&'a mut U>,
    field_transformer: Option<&'a mut F>,
}

impl<'de, U, F> VariantAccess<'de> for VariantDeserializer<'_, U, F>
where
    U: FnMut(Value, Value),
    F: FnMut(Value) -> Result<Value, Box<dyn std::error::Error + 'static + Send + Sync>>,
{
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        match self.value {
            Some(value) => value.unit_variant(),
            None => Ok(()),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.value {
            Some(value) => seed.deserialize(ValueDeserializer::new_with(
                value,
                self.unused_key_callback,
                self.field_transformer,
            )),
            None => Err(Error::invalid_type(
                Unexpected::UnitVariant,
                &"newtype variant",
            )),
        }
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Some(Value::Sequence(v, ..)) => Deserializer::deserialize_any(
                SeqDeserializer::new(v, self.unused_key_callback, self.field_transformer),
                visitor,
            ),
            _ => Err(Error::invalid_type(
                Unexpected::UnitVariant,
                &"tuple variant",
            )),
        }
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Some(Value::Mapping(v, ..)) => Deserializer::deserialize_any(
                StructDeserializer::new(
                    v,
                    fields,
                    self.unused_key_callback,
                    self.field_transformer,
                ),
                visitor,
            ),
            _ => Err(Error::invalid_type(
                Unexpected::UnitVariant,
                &"struct variant",
            )),
        }
    }
}

impl<'de, U, F> VariantAccess<'de> for ValueDeserializer<'_, U, F>
where
    U: FnMut(Value, Value),
    F: FnMut(Value) -> Result<Value, Box<dyn std::error::Error + 'static + Send + Sync>>,
{
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        Deserialize::deserialize(self)
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Error>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(self)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        if let Value::Sequence(v, ..) = self.value {
            Deserializer::deserialize_any(
                SeqDeserializer::new(v, self.unused_key_callback, self.field_transformer),
                visitor,
            )
        } else {
            Err(Error::invalid_type(
                self.value.unexpected(),
                &"tuple variant",
            ))
        }
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        if let Value::Mapping(v, ..) = self.value {
            Deserializer::deserialize_any(
                StructDeserializer::new(
                    v,
                    fields,
                    self.unused_key_callback,
                    self.field_transformer,
                ),
                visitor,
            )
        } else {
            Err(Error::invalid_type(
                self.value.unexpected(),
                &"struct variant",
            ))
        }
    }
}

pub(crate) struct SeqDeserializer<'a, U, F> {
    iter: vec::IntoIter<Value>,
    unused_key_callback: Option<&'a mut U>,
    field_transformer: Option<&'a mut F>,
}

impl<'a, U, F> SeqDeserializer<'a, U, F>
where
    U: FnMut(Value, Value),
    F: FnMut(Value) -> Result<Value, Box<dyn std::error::Error + 'static + Send + Sync>>,
{
    pub(crate) fn new(
        vec: Vec<Value>,
        unused_key_callback: Option<&'a mut U>,
        field_transformer: Option<&'a mut F>,
    ) -> Self {
        SeqDeserializer {
            iter: vec.into_iter(),
            unused_key_callback,
            field_transformer,
        }
    }
}

impl<'de, U, F> Deserializer<'de> for SeqDeserializer<'_, U, F>
where
    U: FnMut(Value, Value),
    F: FnMut(Value) -> Result<Value, Box<dyn std::error::Error + 'static + Send + Sync>>,
{
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let len = self.iter.len();
        if len == 0 {
            visitor.visit_unit()
        } else {
            let ret = visitor.visit_seq(&mut self)?;
            let remaining = self.iter.len();
            if remaining == 0 {
                Ok(ret)
            } else {
                Err(Error::invalid_length(len, &"fewer elements in sequence"))
            }
        }
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        drop(self);
        visitor.visit_unit()
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct
        map struct enum identifier
    }
}

impl<'de, U, F> SeqAccess<'de> for SeqDeserializer<'_, U, F>
where
    U: FnMut(Value, Value),
    F: FnMut(Value) -> Result<Value, Box<dyn std::error::Error + 'static + Send + Sync>>,
{
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => {
                let deserializer = ValueDeserializer::new_with(
                    value,
                    self.unused_key_callback.as_deref_mut(),
                    self.field_transformer.as_deref_mut(),
                );
                seed.deserialize(deserializer).map(Some)
            }
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        match self.iter.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None,
        }
    }
}

pub(crate) struct MapDeserializer<'a, U, F> {
    iter: <Mapping as IntoIterator>::IntoIter,
    value: Option<Value>,
    unused_key_callback: Option<&'a mut U>,
    field_transformer: Option<&'a mut F>,
}

impl<'a, U, F> MapDeserializer<'a, U, F>
where
    U: FnMut(Value, Value),
    F: FnMut(Value) -> Result<Value, Box<dyn std::error::Error + 'static + Send + Sync>>,
{
    pub(crate) fn new(
        map: Mapping,
        unused_key_callback: Option<&'a mut U>,
        field_transformer: Option<&'a mut F>,
    ) -> Self {
        MapDeserializer {
            iter: map.into_iter(),
            value: None,
            unused_key_callback,
            field_transformer,
        }
    }
}

impl<'de, U, F> MapAccess<'de> for MapDeserializer<'_, U, F>
where
    U: FnMut(Value, Value),
    F: FnMut(Value) -> Result<Value, Box<dyn std::error::Error + 'static + Send + Sync>>,
{
    type Error = Error;

    fn next_key_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                seed.deserialize(key).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<T>(&mut self, seed: T) -> Result<T::Value, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => seed.deserialize(ValueDeserializer::new_with(
                value,
                self.unused_key_callback.as_deref_mut(),
                self.field_transformer.as_deref_mut(),
            )),
            None => panic!("visit_value called before visit_key"),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        match self.iter.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None,
        }
    }
}

impl<'de, U, F> Deserializer<'de> for MapDeserializer<'_, U, F>
where
    U: FnMut(Value, Value),
    F: FnMut(Value) -> Result<Value, Box<dyn std::error::Error + 'static + Send + Sync>>,
{
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(self)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        drop(self);
        visitor.visit_unit()
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct
        map struct enum identifier
    }
}

pub(crate) struct StructDeserializer<'a, U, F> {
    iter: <Mapping as IntoIterator>::IntoIter,
    value: Option<Value>,
    normal_keys: HashSet<&'static str>,
    flatten_keys: Vec<&'static str>,
    unused_key_callback: Option<&'a mut U>,
    field_transformer: Option<&'a mut F>,
    rest: Vec<(Value, Value)>,
    flatten_keys_done: usize,
}

impl<'a, U, F> StructDeserializer<'a, U, F>
where
    U: FnMut(Value, Value),
    F: FnMut(Value) -> Result<Value, Box<dyn std::error::Error + 'static + Send + Sync>>,
{
    pub(crate) fn new(
        map: Mapping,
        known_keys: &'static [&'static str],
        unused_key_callback: Option<&'a mut U>,
        field_transformer: Option<&'a mut F>,
    ) -> Self {
        let (normal_keys, flatten_keys): (Vec<_>, Vec<_>) = known_keys
            .iter()
            .copied()
            .partition(|key| !crate::is_flatten_key(key.as_bytes()));
        StructDeserializer {
            iter: map.into_iter(),
            value: None,
            normal_keys: normal_keys.into_iter().collect(),
            flatten_keys,
            unused_key_callback,
            field_transformer,
            rest: Vec::new(),
            flatten_keys_done: 0,
        }
    }

    pub(crate) fn has_flatten(&self) -> bool {
        !self.flatten_keys.is_empty()
    }

    fn has_unprocessed_flatten_keys(&self) -> bool {
        self.flatten_keys_done < self.flatten_keys.len()
    }
}

impl<'de, U, F> MapAccess<'de> for StructDeserializer<'_, U, F>
where
    U: FnMut(Value, Value),
    F: FnMut(Value) -> Result<Value, Box<dyn std::error::Error + 'static + Send + Sync>>,
{
    type Error = Error;

    fn next_key_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        loop {
            match self.iter.next() {
                Some((key, value)) => {
                    match key.as_str() {
                        Some(key_str) if crate::is_flatten_key(key_str.as_bytes()) => {
                            self.rest.push((key, value));
                            continue;
                        }
                        Some(key_str) if !self.normal_keys.contains(key_str) => {
                            if self.has_flatten() {
                                self.rest.push((key, value));
                                continue;
                            } else if let Some(callback) = &mut self.unused_key_callback {
                                value.broadcast_end_mark();
                                callback(key, value);
                                continue;
                            }
                        }
                        _ => {}
                    };

                    self.value = Some(value);
                    break seed.deserialize(ValueDeserializer::new(key)).map(Some);
                }
                None if self.has_unprocessed_flatten_keys() => {
                    let key = self.flatten_keys[self.flatten_keys_done];
                    break seed
                        .deserialize(ValueDeserializer::new(key.into()))
                        .map(Some);
                }
                None => break Ok(None),
            }
        }
    }

    fn next_value_seed<T>(&mut self, seed: T) -> Result<T::Value, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => seed.deserialize(ValueDeserializer::new_with(
                value,
                self.unused_key_callback.as_deref_mut(),
                self.field_transformer.as_deref_mut(),
            )),
            None if self.has_unprocessed_flatten_keys() => {
                self.flatten_keys_done += 1;

                let flattened = Value::mapping(self.rest.drain(..).collect());
                let mut collect_unused = |key, value| {
                    self.rest.push((key, value));
                };

                let deserializer = ValueDeserializer::new_with(
                    flattened,
                    Some(&mut collect_unused),
                    self.field_transformer.as_deref_mut(),
                );

                seed.deserialize(deserializer)
            }
            None => panic!("visit_value called before visit_key"),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        match self.iter.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None,
        }
    }
}

impl<'de, U, F> Deserializer<'de> for StructDeserializer<'_, U, F>
where
    U: FnMut(Value, Value),
    F: FnMut(Value) -> Result<Value, Box<dyn std::error::Error + 'static + Send + Sync>>,
{
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(self)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        drop(self);
        visitor.visit_unit()
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct
        map struct enum identifier
    }
}

impl<'de> Deserializer<'de> for &'de Value {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Null(..) => visitor.visit_unit(),
            Value::Bool(v, ..) => visitor.visit_bool(*v),
            Value::Number(n, ..) => n.deserialize_any(visitor),
            Value::String(v, ..) => visitor.visit_borrowed_str(v),
            Value::Sequence(v, ..) => visit_sequence_ref(v, visitor),
            Value::Mapping(v, ..) => visit_mapping_ref(v, visitor),
            Value::Tagged(tagged, ..) => visitor.visit_enum(&**tagged),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.untag_ref() {
            Value::Bool(v, ..) => visitor.visit_bool(*v),
            other => Err(other.invalid_type(&visitor)),
        }
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_number(visitor)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.untag_ref() {
            Value::String(v, ..) => visitor.visit_borrowed_str(v),
            other => Err(other.invalid_type(&visitor)),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.untag_ref() {
            Value::String(v, ..) => visitor.visit_borrowed_str(v),
            Value::Sequence(v, ..) => visit_sequence_ref(v, visitor),
            other => Err(other.invalid_type(&visitor)),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Null(..) => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            Value::Null(..) => visitor.visit_unit(),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        static EMPTY: Sequence = Sequence::new();
        match self.untag_ref() {
            Value::Sequence(v, ..) => visit_sequence_ref(v, visitor),
            Value::Null(..) => visit_sequence_ref(&EMPTY, visitor),
            other => Err(other.invalid_type(&visitor)),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.untag_ref() {
            Value::Mapping(v, ..) => visit_mapping_ref(v, visitor),
            Value::Null(..) => visitor.visit_map(&mut MapRefDeserializer {
                iter: None,
                value: None,
            }),
            other => Err(other.invalid_type(&visitor)),
        }
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_enum(match self {
            Value::Tagged(tagged, ..) => EnumRefDeserializer {
                tag: tagged::nobang(&tagged.tag.string),
                value: Some(&tagged.value),
            },
            Value::String(variant, ..) => EnumRefDeserializer {
                tag: variant,
                value: None,
            },
            other => {
                return Err(error::set_span(
                    Error::invalid_type(other.unexpected(), &"a Value::Tagged enum"),
                    self.span(),
                ));
            }
        })
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_string(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }
}

struct EnumRefDeserializer<'de> {
    tag: &'de str,
    value: Option<&'de Value>,
}

impl<'de> EnumAccess<'de> for EnumRefDeserializer<'de> {
    type Error = Error;
    type Variant = VariantRefDeserializer<'de>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Error>
    where
        V: DeserializeSeed<'de>,
    {
        let str_de = BorrowedStrDeserializer::<Error>::new(self.tag);
        let variant = seed.deserialize(str_de)?;
        let visitor = VariantRefDeserializer { value: self.value };
        Ok((variant, visitor))
    }
}

struct VariantRefDeserializer<'de> {
    value: Option<&'de Value>,
}

impl<'de> VariantAccess<'de> for VariantRefDeserializer<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        match self.value {
            Some(value) => value.unit_variant(),
            None => Ok(()),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.value {
            Some(value) => value.newtype_variant_seed(seed),
            None => Err(Error::invalid_type(
                Unexpected::UnitVariant,
                &"newtype variant",
            )),
        }
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Some(value) => value.tuple_variant(len, visitor),
            None => Err(Error::invalid_type(
                Unexpected::UnitVariant,
                &"tuple variant",
            )),
        }
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Some(value) => value.struct_variant(fields, visitor),
            None => Err(Error::invalid_type(
                Unexpected::UnitVariant,
                &"struct variant",
            )),
        }
    }
}

pub(crate) struct SeqRefDeserializer<'de> {
    iter: slice::Iter<'de, Value>,
}

impl<'de> SeqRefDeserializer<'de> {
    pub(crate) fn new(slice: &'de [Value]) -> Self {
        SeqRefDeserializer { iter: slice.iter() }
    }
}

impl<'de> Deserializer<'de> for SeqRefDeserializer<'de> {
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let len = self.iter.len();
        if len == 0 {
            visitor.visit_unit()
        } else {
            let ret = visitor.visit_seq(&mut self)?;
            let remaining = self.iter.len();
            if remaining == 0 {
                Ok(ret)
            } else {
                Err(Error::invalid_length(len, &"fewer elements in sequence"))
            }
        }
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct
        map struct enum identifier
    }
}

impl<'de> SeqAccess<'de> for SeqRefDeserializer<'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some(value) => seed.deserialize(value).map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        match self.iter.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None,
        }
    }
}

pub(crate) struct MapRefDeserializer<'de> {
    iter: Option<<&'de Mapping as IntoIterator>::IntoIter>,
    value: Option<&'de Value>,
}

impl<'de> MapRefDeserializer<'de> {
    pub(crate) fn new(map: &'de Mapping) -> Self {
        MapRefDeserializer {
            iter: Some(map.iter()),
            value: None,
        }
    }
}

impl<'de> MapAccess<'de> for MapRefDeserializer<'de> {
    type Error = Error;

    fn next_key_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.as_mut().and_then(Iterator::next) {
            Some((key, value)) => {
                self.value = Some(value);
                seed.deserialize(key).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<T>(&mut self, seed: T) -> Result<T::Value, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.value.take() {
            Some(value) => seed.deserialize(value),
            None => panic!("visit_value called before visit_key"),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        match self.iter.as_ref()?.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None,
        }
    }
}

impl<'de> Deserializer<'de> for MapRefDeserializer<'de> {
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(self)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct
        map struct enum identifier
    }
}

impl Value {
    #[cold]
    fn invalid_type(&self, exp: &dyn Expected) -> Error {
        error::set_span(de::Error::invalid_type(self.unexpected(), exp), self.span())
    }

    #[cold]
    pub(crate) fn unexpected(&self) -> Unexpected {
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
