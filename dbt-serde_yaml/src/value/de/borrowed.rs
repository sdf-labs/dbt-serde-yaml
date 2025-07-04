use std::{collections::HashSet, slice};

use serde::{
    de::{
        value::BorrowedStrDeserializer, DeserializeSeed, EnumAccess, Error as _, MapAccess,
        SeqAccess, Unexpected, VariantAccess, Visitor,
    },
    forward_to_deserialize_any, Deserialize, Deserializer,
};

use crate::{
    error,
    value::{
        de::{
            is_deserializing_value_then_reset, reset_is_deserializing_value,
            store_deserializer_state, ValueDeserializer,
        },
        tagged,
    },
    Error, Mapping, Path, Sequence, Value,
};

use super::TransformedResult;

fn visit_sequence_ref<'p, 'u, 'de, V, U, F>(
    sequence: &'de Sequence,
    current_path: Path<'p>,
    visitor: V,
    unused_key_callback: Option<&'u mut U>,
    field_transformer: Option<&'u mut F>,
) -> Result<V::Value, Error>
where
    V: Visitor<'de>,
    U: for<'a, 'v> FnMut(Path<'a>, &'v Value, &'v Value),
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
{
    let len = sequence.len();
    let mut deserializer = SeqRefDeserializer::new_with(
        sequence,
        current_path,
        unused_key_callback,
        field_transformer,
    );
    let seq = visitor.visit_seq(&mut deserializer)?;
    let remaining = deserializer.iter.len();
    if remaining == 0 {
        Ok(seq)
    } else {
        Err(Error::invalid_length(len, &"fewer elements in sequence"))
    }
}

fn visit_mapping_ref<'p, 'u, 'de, V, U, F>(
    mapping: &'de Mapping,
    current_path: Path<'p>,
    visitor: V,
    unused_key_callback: Option<&'u mut U>,
    field_transformer: Option<&'u mut F>,
) -> Result<V::Value, Error>
where
    V: Visitor<'de>,
    U: for<'a, 'v> FnMut(Path<'a>, &'v Value, &'v Value),
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
{
    let len = mapping.len();
    let mut deserializer = MapRefDeserializer::new_with(
        mapping,
        current_path,
        unused_key_callback,
        field_transformer,
    );
    let map = visitor.visit_map(&mut deserializer)?;
    let has_remaining = deserializer.iter.unwrap().next().is_some();
    if !has_remaining {
        Ok(map)
    } else {
        Err(Error::invalid_length(len, &"fewer elements in map"))
    }
}

fn visit_struct_ref<'p, 'u, 'de, V, U, F>(
    mapping: &'de Mapping,
    current_path: Path<'p>,
    visitor: V,
    known_keys: &'static [&'static str],
    unused_key_callback: Option<&'u mut U>,
    field_transformer: Option<&'u mut F>,
) -> Result<V::Value, Error>
where
    V: Visitor<'de>,
    U: for<'a, 'v> FnMut(Path<'a>, &'v Value, &'v Value),
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
{
    let len = mapping.len();
    let mut deserializer = StructRefDeserializer::new_with(
        mapping,
        current_path,
        known_keys,
        unused_key_callback,
        field_transformer,
    );
    let map = visitor.visit_map(&mut deserializer)?;
    let has_remaining =
        deserializer.iter.unwrap().next().is_some() || !deserializer.rest.is_empty();
    if !has_remaining {
        Ok(map)
    } else {
        Err(Error::invalid_length(len, &"fewer elements in struct"))
    }
}

impl<'de> Deserializer<'de> for &'de Value {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_any(visitor)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_bool(visitor)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_i8(visitor)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_i16(visitor)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_i32(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_i64(visitor)
    }

    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_i128(visitor)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_u8(visitor)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_u16(visitor)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_u32(visitor)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_u64(visitor)
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_u128(visitor)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_f32(visitor)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_f64(visitor)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_char(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_str(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_string(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_bytes(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_byte_buf(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_option(visitor)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_unit(visitor)
    }

    fn deserialize_unit_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_unit_struct(name, visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_newtype_struct(name, visitor)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_seq(visitor)
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_tuple(_len, visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_tuple_struct(name, len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_map(visitor)
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_struct(name, fields, visitor)
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_enum(name, variants, visitor)
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_identifier(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        ValueRefDeserializer::new(self).deserialize_ignored_any(visitor)
    }
}

pub struct ValueRefDeserializer<'p, 'f, 'de, U, F> {
    value: &'de Value,
    path: Path<'p>,
    unused_key_callback: Option<&'f mut U>,
    field_transformer: Option<&'f mut F>,
    // Flag indicating whether the value has been already been transformed by
    // field_transformer:
    is_transformed: bool,
}

impl<'p, 'de>
    ValueRefDeserializer<'p, '_, 'de, fn(Path<'_>, &Value, &Value), fn(&Value) -> TransformedResult>
{
    pub(crate) fn new(value: &'de Value) -> Self {
        ValueRefDeserializer {
            value,
            path: Path::Root,
            unused_key_callback: None,
            field_transformer: None,
            is_transformed: false,
        }
    }
}

impl<'p, 'u, 'de, U, F> ValueRefDeserializer<'p, 'u, 'de, U, F>
where
    U: for<'a, 'v> FnMut(Path<'a>, &'v Value, &'v Value),
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
{
    pub(crate) fn new_with(
        value: &'de Value,
        path: Path<'p>,
        unused_key_callback: Option<&'u mut U>,
        field_transformer: Option<&'u mut F>,
    ) -> Self {
        ValueRefDeserializer {
            value,
            path,
            unused_key_callback,
            field_transformer,
            is_transformed: false,
        }
    }

    pub(crate) fn new_with_transformed(
        value: &'de Value,
        path: Path<'p>,
        unused_key_callback: Option<&'u mut U>,
        field_transformer: Option<&'u mut F>,
    ) -> Self {
        ValueRefDeserializer {
            value,
            path,
            unused_key_callback,
            field_transformer,
            is_transformed: true,
        }
    }
}

macro_rules! maybe_transform_and_forward_to_value_deserializer {
    ($self:expr, $method:ident, $($args:expr),*) => {
        if let Some(transformer) = &mut $self.field_transformer {
            if !$self.is_transformed && crate::verbatim::should_transform_any() {
                if let Some(v) = transformer(&$self.value)? {
                    return ValueDeserializer::new_with_transformed(
                        v,
                        $self.path,
                        $self.unused_key_callback,
                        $self.field_transformer,
                    )
                    .$method($($args),*);
                }
            }
        }
    };
}

use super::maybe_why_not;

impl<'de, U, F> Deserializer<'de> for ValueRefDeserializer<'_, '_, 'de, U, F>
where
    U: for<'a, 'v> FnMut(Path<'a>, &'v Value, &'v Value),
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
{
    type Error = Error;

    fn deserialize_any<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        maybe_transform_and_forward_to_value_deserializer!(self, deserialize_any, visitor);

        let span = self.value.span();
        self.value.broadcast_end_mark();
        if is_deserializing_value_then_reset() {
            store_deserializer_state(
                Some(self.value.clone()),
                self.path,
                self.unused_key_callback,
                self.field_transformer,
            );
            return Err(Error::custom("Value deserialized via fast path"));
        }

        maybe_why_not!(
            self.value,
            match self.value {
                Value::Null(..) => visitor.visit_unit(),
                Value::Bool(v, ..) => visitor.visit_bool(*v),
                Value::Number(n, ..) => n.deserialize_any(visitor),
                Value::String(v, ..) => visitor.visit_borrowed_str(v),
                Value::Sequence(v, ..) => visit_sequence_ref(
                    v,
                    self.path,
                    visitor,
                    self.unused_key_callback,
                    self.field_transformer,
                ),
                Value::Mapping(v, ..) => visit_mapping_ref(
                    v,
                    self.path,
                    visitor,
                    self.unused_key_callback,
                    self.field_transformer,
                ),
                Value::Tagged(tagged, ..) => visitor.visit_enum(&**tagged),
            }
            .map_err(|e| error::set_span(e, span))
        )
    }

    fn deserialize_bool<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        maybe_transform_and_forward_to_value_deserializer!(self, deserialize_bool, visitor);

        let span = self.value.span();
        self.value.broadcast_end_mark();
        maybe_why_not!(
            self.value,
            match self.value.untag_ref() {
                Value::Bool(v, ..) => visitor.visit_bool(*v),
                other => Err(other.invalid_type(&visitor)),
            }
            .map_err(|e| error::set_span(e, span))
        )
    }

    fn deserialize_i8<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        maybe_transform_and_forward_to_value_deserializer!(self, deserialize_i8, visitor);

        self.value.deserialize_number(visitor)
    }

    fn deserialize_i16<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        maybe_transform_and_forward_to_value_deserializer!(self, deserialize_i16, visitor);

        self.value.deserialize_number(visitor)
    }

    fn deserialize_i32<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        maybe_transform_and_forward_to_value_deserializer!(self, deserialize_i32, visitor);

        self.value.deserialize_number(visitor)
    }

    fn deserialize_i64<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        maybe_transform_and_forward_to_value_deserializer!(self, deserialize_i64, visitor);

        self.value.deserialize_number(visitor)
    }

    fn deserialize_i128<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        maybe_transform_and_forward_to_value_deserializer!(self, deserialize_i128, visitor);

        self.value.deserialize_number(visitor)
    }

    fn deserialize_u8<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        maybe_transform_and_forward_to_value_deserializer!(self, deserialize_u8, visitor);

        self.value.deserialize_number(visitor)
    }

    fn deserialize_u16<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        maybe_transform_and_forward_to_value_deserializer!(self, deserialize_u16, visitor);

        self.value.deserialize_number(visitor)
    }

    fn deserialize_u32<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        maybe_transform_and_forward_to_value_deserializer!(self, deserialize_u32, visitor);

        self.value.deserialize_number(visitor)
    }

    fn deserialize_u64<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        maybe_transform_and_forward_to_value_deserializer!(self, deserialize_u64, visitor);

        self.value.deserialize_number(visitor)
    }

    fn deserialize_u128<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        maybe_transform_and_forward_to_value_deserializer!(self, deserialize_u128, visitor);

        self.value.deserialize_number(visitor)
    }

    fn deserialize_f32<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        maybe_transform_and_forward_to_value_deserializer!(self, deserialize_f32, visitor);

        self.value.deserialize_number(visitor)
    }

    fn deserialize_f64<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        maybe_transform_and_forward_to_value_deserializer!(self, deserialize_f64, visitor);

        self.value.deserialize_number(visitor)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_str<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        maybe_transform_and_forward_to_value_deserializer!(self, deserialize_str, visitor);

        let span = self.value.span();
        self.value.broadcast_end_mark();
        maybe_why_not!(
            self.value,
            match self.value.untag_ref() {
                Value::String(v, ..) => visitor.visit_borrowed_str(v),
                other => Err(other.invalid_type(&visitor)),
            }
            .map_err(|e| error::set_span(e, span))
        )
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        maybe_transform_and_forward_to_value_deserializer!(self, deserialize_bytes, visitor);

        let span = self.value.span();
        self.value.broadcast_end_mark();
        maybe_why_not!(
            self.value,
            match self.value.untag_ref() {
                Value::String(v, ..) => visitor.visit_borrowed_str(v),
                Value::Sequence(v, ..) => visit_sequence_ref(
                    v,
                    self.path,
                    visitor,
                    self.unused_key_callback,
                    self.field_transformer,
                ),
                other => Err(other.invalid_type(&visitor)),
            }
            .map_err(|e| error::set_span(e, span))
        )
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        maybe_transform_and_forward_to_value_deserializer!(self, deserialize_option, visitor);

        let span = self.value.span();
        self.value.broadcast_end_mark();
        maybe_why_not!(
            self.value,
            match self.value {
                Value::Null(..) => visitor.visit_unit(),
                _ => visitor.visit_some(ValueRefDeserializer::new_with_transformed(
                    self.value,
                    self.path,
                    self.unused_key_callback,
                    self.field_transformer,
                )),
            }
            .map_err(|e| error::set_span(e, span))
        )
    }

    fn deserialize_unit<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        maybe_transform_and_forward_to_value_deserializer!(self, deserialize_unit, visitor);

        let span = self.value.span();
        self.value.broadcast_end_mark();
        maybe_why_not!(
            self.value,
            match self.value {
                Value::Null(..) => visitor.visit_unit(),
                _ => Err(self.value.invalid_type(&visitor)),
            }
            .map_err(|e| error::set_span(e, span))
        )
    }

    fn deserialize_unit_struct<V>(
        mut self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        maybe_transform_and_forward_to_value_deserializer!(
            self,
            deserialize_unit_struct,
            name,
            visitor
        );

        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        mut self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        maybe_transform_and_forward_to_value_deserializer!(
            self,
            deserialize_newtype_struct,
            name,
            visitor
        );

        let span = self.value.span();
        self.value.broadcast_end_mark();
        maybe_why_not!(
            self.value,
            visitor
                .visit_newtype_struct(ValueRefDeserializer::new_with_transformed(
                    self.value,
                    self.path,
                    self.unused_key_callback,
                    self.field_transformer
                ))
                .map_err(|e| error::set_span(e, span))
        )
    }

    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        static EMPTY: Sequence = Sequence::new();

        maybe_transform_and_forward_to_value_deserializer!(self, deserialize_seq, visitor);

        let span = self.value.span();
        self.value.broadcast_end_mark();
        maybe_why_not!(
            self.value,
            match self.value.untag_ref() {
                Value::Sequence(v, ..) => visit_sequence_ref(
                    v,
                    self.path,
                    visitor,
                    self.unused_key_callback,
                    self.field_transformer,
                ),
                Value::Null(..) => visit_sequence_ref(
                    &EMPTY,
                    self.path,
                    visitor,
                    self.unused_key_callback,
                    self.field_transformer,
                ),
                other => Err(other.invalid_type(&visitor)),
            }
            .map_err(|e| error::set_span(e, span))
        )
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
        maybe_transform_and_forward_to_value_deserializer!(self, deserialize_map, visitor);

        let span = self.value.span();
        self.value.broadcast_end_mark();
        maybe_why_not!(
            self.value,
            match self.value.untag_ref() {
                Value::Mapping(v, ..) => visit_mapping_ref(
                    v,
                    self.path,
                    visitor,
                    self.unused_key_callback,
                    self.field_transformer,
                ),
                Value::Null(..) => visitor.visit_map(&mut MapRefDeserializer::new_empty(self.path)),
                other => Err(other.invalid_type(&visitor)),
            }
            .map_err(|e| error::set_span(e, span))
        )
    }

    fn deserialize_struct<V>(
        mut self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        maybe_transform_and_forward_to_value_deserializer!(
            self,
            deserialize_struct,
            name,
            fields,
            visitor
        );

        let span = self.value.span();
        self.value.broadcast_end_mark();
        maybe_why_not!(
            self.value,
            match self.value.untag_ref() {
                Value::Mapping(v, ..) => visit_struct_ref(
                    v,
                    self.path,
                    visitor,
                    fields,
                    self.unused_key_callback,
                    self.field_transformer,
                ),
                Value::Null(..) => visitor.visit_map(&mut MapRefDeserializer::new_empty(self.path)),
                other => Err(other.invalid_type(&visitor)),
            }
            .map_err(|e| error::set_span(e, span))
        )
    }

    fn deserialize_enum<V>(
        mut self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        maybe_transform_and_forward_to_value_deserializer!(
            self,
            deserialize_enum,
            name,
            variants,
            visitor
        );

        let span = self.value.span();
        self.value.broadcast_end_mark();
        maybe_why_not!(
            self.value,
            visitor
                .visit_enum(match self.value {
                    Value::Tagged(tagged, ..) => EnumRefDeserializer {
                        tag: tagged::nobang(&tagged.tag.string),
                        path: self.path,
                        value: Some(&tagged.value),
                        unused_key_callback: self.unused_key_callback,
                        field_transformer: self.field_transformer,
                    },
                    Value::String(variant, ..) => EnumRefDeserializer {
                        tag: variant,
                        path: self.path,
                        value: None,
                        unused_key_callback: self.unused_key_callback,
                        field_transformer: self.field_transformer,
                    },
                    other => {
                        return Err(error::set_span(
                            Error::invalid_type(other.unexpected(), &"a Value::Tagged enum"),
                            span,
                        ));
                    }
                })
                .map_err(|e| error::set_span(e, span))
        )
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
        let span = self.value.span();
        self.value.broadcast_end_mark();
        maybe_why_not!(
            self.value,
            visitor.visit_unit().map_err(|e| error::set_span(e, span))
        )
    }
}

struct EnumRefDeserializer<'p, 'u, 'de, U, F> {
    tag: &'de str,
    path: Path<'p>,
    value: Option<&'de Value>,
    unused_key_callback: Option<&'u mut U>,
    field_transformer: Option<&'u mut F>,
}

impl<'p, 'u, 'de, U, F> EnumAccess<'de> for EnumRefDeserializer<'p, 'u, 'de, U, F>
where
    U: for<'a, 'v> FnMut(Path<'a>, &'v Value, &'v Value),
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
{
    type Error = Error;
    type Variant = VariantRefDeserializer<'p, 'u, 'de, U, F>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Error>
    where
        V: DeserializeSeed<'de>,
    {
        let str_de = BorrowedStrDeserializer::<Error>::new(self.tag);
        let variant = seed.deserialize(str_de)?;
        let visitor = VariantRefDeserializer {
            value: self.value,
            path: self.path,
            unused_key_callback: self.unused_key_callback,
            field_transformer: self.field_transformer,
        };
        Ok((variant, visitor))
    }
}

struct VariantRefDeserializer<'p, 'u, 'de, U, F> {
    value: Option<&'de Value>,
    path: Path<'p>,
    unused_key_callback: Option<&'u mut U>,
    field_transformer: Option<&'u mut F>,
}

impl<'de, U, F> VariantAccess<'de> for VariantRefDeserializer<'_, '_, 'de, U, F>
where
    U: for<'a, 'v> FnMut(Path<'a>, &'v Value, &'v Value),
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
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
            Some(value) => seed.deserialize(ValueRefDeserializer::new_with(
                value,
                self.path,
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
                SeqRefDeserializer::new_with(
                    v,
                    self.path,
                    self.unused_key_callback,
                    self.field_transformer,
                ),
                visitor,
            ),
            Some(value) => Err(Error::invalid_type(value.unexpected(), &"tuple variant")),
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
                StructRefDeserializer::new_with(
                    v,
                    self.path,
                    fields,
                    self.unused_key_callback,
                    self.field_transformer,
                ),
                visitor,
            ),
            Some(value) => Err(Error::invalid_type(value.unexpected(), &"struct variant")),
            None => Err(Error::invalid_type(
                Unexpected::UnitVariant,
                &"struct variant",
            )),
        }
    }
}

impl<'de, U, F> VariantAccess<'de> for ValueRefDeserializer<'_, '_, 'de, U, F>
where
    U: for<'a, 'v> FnMut(Path<'a>, &'v Value, &'v Value),
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
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
                SeqRefDeserializer::new_with(
                    v,
                    self.path,
                    self.unused_key_callback,
                    self.field_transformer,
                ),
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
                StructRefDeserializer::new_with(
                    v,
                    self.path,
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

pub(crate) struct SeqRefDeserializer<'p, 'u, 'de, U, F> {
    iter: slice::Iter<'de, Value>,
    path: Path<'p>,
    current_idx: usize,
    unused_key_callback: Option<&'u mut U>,
    field_transformer: Option<&'u mut F>,
}

impl<'p, 'u, 'de>
    SeqRefDeserializer<'p, 'u, 'de, fn(Path<'_>, &Value, &Value), fn(&Value) -> TransformedResult>
{
    pub(crate) fn new(slice: &'de [Value]) -> Self {
        SeqRefDeserializer {
            iter: slice.iter(),
            path: Path::Root,
            current_idx: 0,
            unused_key_callback: None,
            field_transformer: None,
        }
    }
}

impl<'p, 'u, 'de, U, F> SeqRefDeserializer<'p, 'u, 'de, U, F> {
    pub(crate) fn new_with(
        slice: &'de [Value],
        current_path: Path<'p>,
        unused_key_callback: Option<&'u mut U>,
        field_transformer: Option<&'u mut F>,
    ) -> Self {
        SeqRefDeserializer {
            iter: slice.iter(),
            path: current_path,
            current_idx: 0,
            unused_key_callback,
            field_transformer,
        }
    }
}

impl<'de, U, F> Deserializer<'de> for SeqRefDeserializer<'_, '_, 'de, U, F>
where
    U: for<'a, 'v> FnMut(Path<'a>, &'v Value, &'v Value),
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
{
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        reset_is_deserializing_value();

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

impl<'de, U, F> SeqAccess<'de> for SeqRefDeserializer<'_, '_, 'de, U, F>
where
    U: for<'a, 'v> FnMut(Path<'a>, &'v Value, &'v Value),
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
{
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        self.current_idx += 1;
        match self.iter.next() {
            Some(value) => {
                let deserializer = ValueRefDeserializer::new_with(
                    value,
                    Path::Seq {
                        parent: &self.path,
                        index: self.current_idx - 1,
                    },
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

pub(crate) struct MapRefDeserializer<'p, 'u, 'de, U, F> {
    iter: Option<Box<dyn Iterator<Item = (&'de Value, &'de Value)> + 'de>>,
    current_key: Option<String>,
    path: Path<'p>,
    value: Option<&'de Value>,
    unused_key_callback: Option<&'u mut U>,
    field_transformer: Option<&'u mut F>,
}

impl<'p, 'u, 'de>
    MapRefDeserializer<'p, 'u, 'de, fn(Path<'_>, &Value, &Value), fn(&Value) -> TransformedResult>
{
    pub(crate) fn new(map: &'de Mapping) -> Self {
        MapRefDeserializer {
            iter: Some(Box::new(map.iter())),
            current_key: None,
            path: Path::Root,
            value: None,
            unused_key_callback: None,
            field_transformer: None,
        }
    }

    pub(crate) fn new_empty(path: Path<'p>) -> Self {
        MapRefDeserializer {
            iter: None,
            current_key: None,
            path,
            value: None,
            unused_key_callback: None,
            field_transformer: None,
        }
    }
}

impl<'p, 'u, 'de, U, F> MapRefDeserializer<'p, 'u, 'de, U, F>
where
    U: for<'a, 'v> FnMut(Path<'a>, &'v Value, &'v Value),
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
{
    pub(crate) fn new_with(
        map: &'de Mapping,
        path: Path<'p>,
        unused_key_callback: Option<&'u mut U>,
        field_transformer: Option<&'u mut F>,
    ) -> Self {
        MapRefDeserializer {
            iter: Some(Box::new(map.iter())),
            current_key: None,
            path,
            value: None,
            unused_key_callback,
            field_transformer,
        }
    }
}

impl<'de, U, F> MapAccess<'de> for MapRefDeserializer<'_, '_, 'de, U, F>
where
    U: for<'a, 'v> FnMut(Path<'a>, &'v Value, &'v Value),
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
{
    type Error = Error;

    fn next_key_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        self.current_key = None;
        match self.iter.as_mut().and_then(Iterator::next) {
            Some((key, value)) => {
                self.value = Some(value);
                self.current_key = key.as_str().map(String::from);
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
            Some(value) => seed.deserialize(ValueRefDeserializer::new_with(
                value,
                match self.current_key {
                    Some(ref key) => Path::Map {
                        parent: &self.path,
                        key,
                    },
                    None => Path::Unknown { parent: &self.path },
                },
                self.unused_key_callback.as_deref_mut(),
                self.field_transformer.as_deref_mut(),
            )),
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

impl<'p, 'de, U, F> Deserializer<'de> for MapRefDeserializer<'p, '_, 'de, U, F>
where
    U: for<'a, 'v> FnMut(Path<'a>, &'v Value, &'v Value),
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
{
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        reset_is_deserializing_value();
        visitor.visit_map(self)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let (normal_keys, flatten_keys): (Vec<_>, Vec<_>) = fields
            .iter()
            .copied()
            .partition(|key| !crate::is_flatten_key(key.as_bytes()));
        visitor.visit_map(StructRefDeserializer {
            iter: self.iter,
            current_key: None,
            path: self.path,
            value: None,
            normal_keys: normal_keys.into_iter().collect(),
            flatten_keys,
            unused_key_callback: self.unused_key_callback,
            field_transformer: self.field_transformer,
            rest: Vec::new(),
            flatten_keys_done: 0,
        })
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
        map enum identifier
    }
}

pub(crate) struct StructRefDeserializer<'p, 'u, 'de, U, F> {
    iter: Option<Box<dyn Iterator<Item = (&'de Value, &'de Value)> + 'de>>,
    current_key: Option<String>,
    path: Path<'p>,
    value: Option<&'de Value>,
    normal_keys: HashSet<&'static str>,
    flatten_keys: Vec<&'static str>,
    unused_key_callback: Option<&'u mut U>,
    field_transformer: Option<&'u mut F>,
    rest: Vec<(&'de Value, &'de Value)>,
    flatten_keys_done: usize,
}

impl<'p, 'u, 'de, U, F> StructRefDeserializer<'p, 'u, 'de, U, F>
where
    U: for<'a, 'v> FnMut(Path<'a>, &'v Value, &'v Value),
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
{
    pub(crate) fn new_with(
        map: &'de Mapping,
        current_path: Path<'p>,
        known_keys: &'static [&'static str],
        unused_key_callback: Option<&'u mut U>,
        field_transformer: Option<&'u mut F>,
    ) -> Self {
        let (normal_keys, flatten_keys): (Vec<_>, Vec<_>) = known_keys
            .iter()
            .copied()
            .partition(|key| !crate::is_flatten_key(key.as_bytes()));
        StructRefDeserializer {
            iter: Some(Box::new(map.iter())),
            current_key: None,
            path: current_path,
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

impl<'p, 'de, U, F> MapAccess<'de> for StructRefDeserializer<'p, '_, 'de, U, F>
where
    U: for<'a, 'v> FnMut(Path<'a>, &'v Value, &'v Value),
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
{
    type Error = Error;

    fn next_key_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        self.current_key = None;
        loop {
            match self.iter.as_mut().and_then(Iterator::next) {
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
                                let key_string = key_str.to_string();
                                let path = Path::Map {
                                    parent: &self.path,
                                    key: &key_string,
                                };
                                callback(path, key, value);
                                continue;
                            }
                        }
                        _ => {}
                    };

                    self.current_key = key.as_str().map(|s| s.to_string());
                    self.value = Some(value);
                    break seed.deserialize(ValueRefDeserializer::new(key)).map(Some);
                }
                None if self.has_unprocessed_flatten_keys() => {
                    let key = self.flatten_keys[self.flatten_keys_done];
                    self.current_key = Some(key.to_string());
                    break seed
                        .deserialize(super::ValueDeserializer::new(key.into()))
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
            Some(value) => seed.deserialize(ValueRefDeserializer::new_with(
                value,
                match self.current_key {
                    Some(ref key) => Path::Map {
                        parent: &self.path,
                        key,
                    },
                    None => Path::Unknown { parent: &self.path },
                },
                self.unused_key_callback.as_deref_mut(),
                self.field_transformer.as_deref_mut(),
            )),
            None if self.has_unprocessed_flatten_keys() => {
                self.flatten_keys_done += 1;

                let flattened = self.rest.drain(..).collect::<Vec<_>>();
                let path = match self.current_key {
                    Some(ref key) => Path::Map {
                        parent: &self.path,
                        key,
                    },
                    None => Path::Unknown { parent: &self.path },
                };

                if self.has_unprocessed_flatten_keys() {
                    let deserializer = FlattenRefDeserializer::new(
                        Some(Box::new(flattened.into_iter())),
                        path,
                        &mut self.rest,
                        self.field_transformer.as_deref_mut(),
                    );

                    seed.deserialize(deserializer)
                } else {
                    let deserializer = MapRefDeserializer {
                        iter: Some(Box::new(flattened.into_iter())),
                        current_key: None,
                        path,
                        value: None,
                        unused_key_callback: self.unused_key_callback.as_deref_mut(),
                        field_transformer: self.field_transformer.as_deref_mut(),
                    };
                    seed.deserialize(deserializer)
                }
            }
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

impl<'p, 'de, U, F> Deserializer<'de> for StructRefDeserializer<'p, '_, 'de, U, F>
where
    U: for<'a, 'v> FnMut(Path<'a>, &'v Value, &'v Value),
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
{
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        reset_is_deserializing_value();
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

struct FlattenRefDeserializer<'p, 'u, 'r, 'de, F> {
    iter: Option<Box<dyn Iterator<Item = (&'de Value, &'de Value)> + 'de>>,
    path: Path<'p>,
    remaining: &'r mut Vec<(&'de Value, &'de Value)>,
    field_transformer: Option<&'u mut F>,
}

impl<'p, 'u, 'r, 'de, F> FlattenRefDeserializer<'p, 'u, 'r, 'de, F>
where
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
{
    pub(crate) fn new(
        iter: Option<Box<dyn Iterator<Item = (&'de Value, &'de Value)> + 'de>>,
        current_path: Path<'p>,
        remaining: &'r mut Vec<(&'de Value, &'de Value)>,
        field_transformer: Option<&'u mut F>,
    ) -> Self {
        FlattenRefDeserializer {
            iter,
            path: current_path,
            remaining,
            field_transformer,
        }
    }
}

impl<'p, 'u, 'r, 'de, F> Deserializer<'de> for FlattenRefDeserializer<'p, 'u, 'r, 'de, F>
where
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
{
    type Error = Error;

    fn deserialize_any<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        reset_is_deserializing_value();
        let mut collect_unused = |_: Path<'_>, key: &Value, value: &Value| {
            // SAFETY: the references passed to this closure are
            // guaranteed to be borrowed for 'de
            let key: &'de Value = unsafe { std::mem::transmute(key) };
            let value: &'de Value = unsafe { std::mem::transmute(value) };
            self.remaining.push((key, value));
        };
        let deserializer = MapRefDeserializer {
            iter: self.iter,
            current_key: None,
            path: self.path,
            value: None,
            unused_key_callback: Some(&mut collect_unused),
            field_transformer: self.field_transformer.as_deref_mut(),
        };
        visitor.visit_map(deserializer)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        let mut collect_unused = |_: Path<'_>, key: &Value, value: &Value| {
            // SAFETY: the references passed to this closure are
            // guaranteed to be borrowed for 'de
            let key: &'de Value = unsafe { std::mem::transmute(key) };
            let value: &'de Value = unsafe { std::mem::transmute(value) };
            self.remaining.push((key, value));
        };

        let (normal_keys, flatten_keys): (Vec<_>, Vec<_>) = fields
            .iter()
            .copied()
            .partition(|key| !crate::is_flatten_key(key.as_bytes()));
        let deserializer = StructRefDeserializer {
            iter: self.iter,
            current_key: None,
            path: self.path,
            value: None,
            normal_keys: normal_keys.into_iter().collect(),
            flatten_keys,
            unused_key_callback: Some(&mut collect_unused),
            field_transformer: self.field_transformer,
            rest: Vec::new(),
            flatten_keys_done: 0,
        };
        visitor.visit_map(deserializer)
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
        map enum identifier
    }
}
