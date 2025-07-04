use std::{collections::HashSet, vec};

use serde::{
    de::{
        value::StrDeserializer, DeserializeSeed, EnumAccess, Error as _, MapAccess, SeqAccess,
        Unexpected, VariantAccess, Visitor,
    },
    forward_to_deserialize_any, Deserialize, Deserializer,
};

use crate::{
    error,
    value::{
        de::{
            borrowed::ValueRefDeserializer, is_deserializing_value_then_reset,
            reset_is_deserializing_value, store_deserializer_state,
        },
        tagged,
    },
    Error, Mapping, Path, Sequence, Value,
};

use super::TransformedResult;

fn visit_sequence<'de, 'a, 'f, V, U, F>(
    sequence: Sequence,
    current_path: Path<'a>,
    visitor: V,
    unused_key_callback: Option<&'f mut U>,
    field_transformer: Option<&'f mut F>,
) -> Result<V::Value, Error>
where
    V: Visitor<'de>,
    U: for<'p, 'v> FnMut(Path<'p>, &'v Value, &'v Value),
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
{
    let len = sequence.len();
    let mut deserializer = SeqDeserializer::new(
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

fn visit_mapping<'de, 'a, 'f, V, U, F>(
    mapping: Mapping,
    current_path: Path<'a>,
    visitor: V,
    unused_key_callback: Option<&'f mut U>,
    field_transformer: Option<&'f mut F>,
) -> Result<V::Value, Error>
where
    V: Visitor<'de>,
    U: for<'p, 'v> FnMut(Path<'p>, &'v Value, &'v Value),
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
{
    let len = mapping.len();
    let mut deserializer = MapDeserializer::new(
        mapping,
        current_path,
        unused_key_callback,
        field_transformer,
    );
    let map = visitor.visit_map(&mut deserializer)?;
    let remaining = deserializer.iter.len();
    if remaining == 0 {
        Ok(map)
    } else {
        Err(Error::invalid_length(len, &"fewer elements in map"))
    }
}

fn visit_struct<'de, 'a, 'f, V, U, F>(
    mapping: Mapping,
    current_path: Path<'a>,
    visitor: V,
    known_keys: &'static [&'static str],
    unused_key_callback: Option<&'f mut U>,
    field_transformer: Option<&'f mut F>,
) -> Result<V::Value, Error>
where
    V: Visitor<'de>,
    U: for<'p, 'v> FnMut(Path<'p>, &'v Value, &'v Value),
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
{
    let len = mapping.len();
    let mut deserializer = StructDeserializer::new(
        mapping,
        current_path,
        known_keys,
        unused_key_callback,
        field_transformer,
    );
    let map = visitor.visit_map(&mut deserializer)?;
    let remaining = deserializer.iter.len() + deserializer.rest.len();
    if remaining == 0 {
        Ok(map)
    } else {
        Err(Error::invalid_length(len, &"fewer elements in struct"))
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

pub struct ValueDeserializer<'a, 'f, U, F> {
    value: Value,
    path: Path<'a>,
    unused_key_callback: Option<&'f mut U>,
    field_transformer: Option<&'f mut F>,
    // Flag indicating whether the value has been already been transformed by
    // field_transformer:
    is_transformed: bool,
}

impl ValueDeserializer<'_, '_, fn(Path<'_>, &Value, &Value), fn(&Value) -> TransformedResult> {
    pub(crate) fn new(value: Value) -> Self {
        ValueDeserializer {
            value,
            path: Path::Root,
            unused_key_callback: None,
            field_transformer: None,
            is_transformed: false,
        }
    }
}

impl<'a, 'f, U, F> ValueDeserializer<'a, 'f, U, F>
where
    U: for<'p, 'v> FnMut(Path<'p>, &'v Value, &'v Value),
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
{
    pub(crate) fn new_with(
        value: Value,
        path: Path<'a>,
        unused_key_callback: Option<&'f mut U>,
        field_transformer: Option<&'f mut F>,
    ) -> Self {
        ValueDeserializer {
            value,
            path,
            unused_key_callback,
            field_transformer,
            is_transformed: false,
        }
    }

    pub(crate) fn new_with_transformed(
        value: Value,
        path: Path<'a>,
        unused_key_callback: Option<&'f mut U>,
        field_transformer: Option<&'f mut F>,
    ) -> Self {
        ValueDeserializer {
            value,
            path,
            unused_key_callback,
            field_transformer,
            is_transformed: true,
        }
    }

    fn maybe_apply_transformation(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error + 'static + Send + Sync>> {
        if let Some(transformer) = &mut self.field_transformer {
            if !self.is_transformed && crate::verbatim::should_transform_any() {
                if let Some(v) = transformer(&self.value)? {
                    self.value = v;
                }
            }
        }
        Ok(())
    }
}

macro_rules! maybe_expecting_should_be {
    ($self:expr, $method:ident, $($args:expr),*) => {{
        if $crate::shouldbe::is_expecting_should_be_then_reset() {
            let res = ValueRefDeserializer::new_with_transformed(
                // SAFETY: ShouldBe<T>:Deserialize is only implemented for T:DeserializeOwned,
                // so we know that `res` can not contain references to `self.value`.
                unsafe { std::mem::transmute::<&Value, &'de Value>(&$self.value) },
                $self.path,
                $self.unused_key_callback,
                $self.field_transformer,
            )
            .$method($($args),*);
            return match res {
                Ok(value) => Ok(value),
                Err(e) => {
                    let msg = e.to_string();
                    crate::shouldbe::set_why_not($self.value, e);
                    // ShouldBe will ignore this and use the error in `why_not`,
                    // but we still need to return an error here nonetheless.
                    Err(Error::custom(msg))
                }
            };
        }
    }};
}

impl<'de, U, F> Deserializer<'de> for ValueDeserializer<'_, '_, U, F>
where
    U: for<'p, 'v> FnMut(Path<'p>, &'v Value, &'v Value),
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
{
    type Error = Error;

    fn deserialize_any<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.maybe_apply_transformation()?;
        maybe_expecting_should_be!(self, deserialize_any, visitor);

        let span = self.value.span();
        self.value.broadcast_end_mark();
        if is_deserializing_value_then_reset() {
            store_deserializer_state(
                Some(self.value),
                self.path,
                self.unused_key_callback,
                self.field_transformer,
            );
            return Err(Error::custom("Value deserialized via fast path"));
        }

        match self.value {
            Value::Null(..) => visitor.visit_unit(),
            Value::Bool(v, ..) => visitor.visit_bool(v),
            Value::Number(n, ..) => n.deserialize_any(visitor),
            Value::String(v, ..) => visitor.visit_string(v),
            Value::Sequence(v, ..) => visit_sequence(
                v,
                self.path,
                visitor,
                self.unused_key_callback,
                self.field_transformer,
            ),
            Value::Mapping(v, ..) => visit_mapping(
                v,
                self.path,
                visitor,
                self.unused_key_callback,
                self.field_transformer,
            ),
            Value::Tagged(tagged, ..) => visitor.visit_enum(*tagged),
        }
        .map_err(|e| error::set_span(e, span))
    }

    fn deserialize_bool<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.maybe_apply_transformation()?;
        maybe_expecting_should_be!(self, deserialize_bool, visitor);

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
        maybe_expecting_should_be!(self, deserialize_string, visitor);

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
        maybe_expecting_should_be!(self, deserialize_byte_buf, visitor);

        let span = self.value.span();
        self.value.broadcast_end_mark();
        match self.value.untag() {
            Value::String(v, ..) => visitor.visit_string(v),
            Value::Sequence(v, ..) => visit_sequence(
                v,
                self.path,
                visitor,
                self.unused_key_callback,
                self.field_transformer,
            ),
            other => Err(other.invalid_type(&visitor)),
        }
        .map_err(|e| error::set_span(e, span))
    }

    fn deserialize_option<V>(mut self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.maybe_apply_transformation()?;
        maybe_expecting_should_be!(self, deserialize_option, visitor);

        let span = self.value.span();
        self.value.broadcast_end_mark();
        match self.value {
            Value::Null(..) => visitor.visit_unit(),
            _ => visitor.visit_some(ValueDeserializer::<U, F> {
                value: self.value,
                path: self.path,
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
        maybe_expecting_should_be!(self, deserialize_unit, visitor);

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
        mut self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.maybe_apply_transformation()?;
        maybe_expecting_should_be!(self, deserialize_newtype_struct, name, visitor);

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
        maybe_expecting_should_be!(self, deserialize_seq, visitor);

        let span = self.value.span();
        self.value.broadcast_end_mark();
        match self.value.untag() {
            Value::Sequence(v, ..) => visit_sequence(
                v,
                self.path,
                visitor,
                self.unused_key_callback,
                self.field_transformer,
            ),
            Value::Null(..) => visit_sequence(
                Sequence::new(),
                self.path,
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
        maybe_expecting_should_be!(self, deserialize_map, visitor);

        let span = self.value.span();
        self.value.broadcast_end_mark();
        match self.value.untag() {
            Value::Mapping(v, ..) => visit_mapping(
                v,
                self.path,
                visitor,
                self.unused_key_callback,
                self.field_transformer,
            ),
            Value::Null(..) => visit_mapping(
                Mapping::new(),
                self.path,
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
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.maybe_apply_transformation()?;
        maybe_expecting_should_be!(self, deserialize_struct, name, fields, visitor);

        let span = self.value.span();
        self.value.broadcast_end_mark();
        match self.value.untag() {
            Value::Mapping(v, ..) => visit_struct(
                v,
                self.path,
                visitor,
                fields,
                self.unused_key_callback,
                self.field_transformer,
            ),
            Value::Null(..) => visit_struct(
                Mapping::new(),
                self.path,
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
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.maybe_apply_transformation()?;
        maybe_expecting_should_be!(self, deserialize_enum, name, variants, visitor);

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
                    path: self.path,
                    value: Some(tagged.value),
                    unused_key_callback: self.unused_key_callback,
                    field_transformer: self.field_transformer,
                },
                Value::String(variant, ..) => EnumDeserializer {
                    tag: {
                        tag = variant;
                        &tag
                    },
                    path: self.path,
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
        maybe_expecting_should_be!(self, deserialize_ignored_any, visitor);

        let span = self.value.span();
        self.value.broadcast_end_mark();
        drop(self);
        visitor.visit_unit().map_err(|e| error::set_span(e, span))
    }
}

struct EnumDeserializer<'a, 'f, U, F> {
    tag: &'a str,
    path: Path<'a>,
    value: Option<Value>,
    unused_key_callback: Option<&'f mut U>,
    field_transformer: Option<&'f mut F>,
}

impl<'de, 'a, 'f, U, F> EnumAccess<'de> for EnumDeserializer<'a, 'f, U, F>
where
    U: for<'p, 'v> FnMut(Path<'p>, &'v Value, &'v Value),
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
{
    type Error = Error;
    type Variant = VariantDeserializer<'a, 'f, U, F>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Error>
    where
        V: DeserializeSeed<'de>,
    {
        let str_de = StrDeserializer::<Error>::new(self.tag);
        let variant = seed.deserialize(str_de)?;
        let visitor = VariantDeserializer {
            value: self.value,
            path: self.path,
            unused_key_callback: self.unused_key_callback,
            field_transformer: self.field_transformer,
        };
        Ok((variant, visitor))
    }
}

struct VariantDeserializer<'a, 'f, U, F> {
    value: Option<Value>,
    path: Path<'a>,
    unused_key_callback: Option<&'f mut U>,
    field_transformer: Option<&'f mut F>,
}

impl<'de, U, F> VariantAccess<'de> for VariantDeserializer<'_, '_, U, F>
where
    U: for<'p, 'v> FnMut(Path<'p>, &'v Value, &'v Value),
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
            Some(value) => seed.deserialize(ValueDeserializer::new_with(
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
                SeqDeserializer::new(
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
                StructDeserializer::new(
                    v,
                    self.path,
                    fields,
                    self.unused_key_callback,
                    self.field_transformer,
                ),
                visitor,
            ),
            Some(value) => Err(Error::invalid_type(value.unexpected(), &"struct variant")),
            _ => Err(Error::invalid_type(
                Unexpected::UnitVariant,
                &"struct variant",
            )),
        }
    }
}

impl<'de, U, F> VariantAccess<'de> for ValueDeserializer<'_, '_, U, F>
where
    U: for<'p, 'v> FnMut(Path<'p>, &'v Value, &'v Value),
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
                SeqDeserializer::new(
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
                StructDeserializer::new(
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

pub(crate) struct SeqDeserializer<'a, 'f, U, F> {
    iter: vec::IntoIter<Value>,
    current_idx: usize,
    path: Path<'a>,
    unused_key_callback: Option<&'f mut U>,
    field_transformer: Option<&'f mut F>,
}

impl<'a, 'f, U, F> SeqDeserializer<'a, 'f, U, F>
where
    U: for<'p, 'v> FnMut(Path<'p>, &'v Value, &'v Value),
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
{
    pub(crate) fn new(
        vec: Vec<Value>,
        current_path: Path<'a>,
        unused_key_callback: Option<&'f mut U>,
        field_transformer: Option<&'f mut F>,
    ) -> Self {
        SeqDeserializer {
            iter: vec.into_iter(),
            current_idx: 0,
            path: current_path,
            unused_key_callback,
            field_transformer,
        }
    }
}

impl<'de, U, F> Deserializer<'de> for SeqDeserializer<'_, '_, U, F>
where
    U: for<'p, 'v> FnMut(Path<'p>, &'v Value, &'v Value),
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
        drop(self);
        visitor.visit_unit()
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct
        map struct enum identifier
    }
}

impl<'de, U, F> SeqAccess<'de> for SeqDeserializer<'_, '_, U, F>
where
    U: for<'p, 'v> FnMut(Path<'p>, &'v Value, &'v Value),
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
                let deserializer = ValueDeserializer::new_with(
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

pub(crate) struct MapDeserializer<'a, 'f, U, F> {
    iter: <Mapping as IntoIterator>::IntoIter,
    current_key: Option<String>,
    path: Path<'a>,
    value: Option<Value>,
    unused_key_callback: Option<&'f mut U>,
    field_transformer: Option<&'f mut F>,
}

impl<'a, 'f, U, F> MapDeserializer<'a, 'f, U, F>
where
    U: for<'p, 'v> FnMut(Path<'p>, &'v Value, &'v Value),
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
{
    pub(crate) fn new(
        map: Mapping,
        current_path: Path<'a>,
        unused_key_callback: Option<&'f mut U>,
        field_transformer: Option<&'f mut F>,
    ) -> Self {
        MapDeserializer {
            iter: map.into_iter(),
            current_key: None,
            path: current_path,
            value: None,
            unused_key_callback,
            field_transformer,
        }
    }
}

impl<'de, U, F> MapAccess<'de> for MapDeserializer<'_, '_, U, F>
where
    U: for<'p, 'v> FnMut(Path<'p>, &'v Value, &'v Value),
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
{
    type Error = Error;

    fn next_key_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        self.current_key = None;
        match self.iter.next() {
            Some((key, value)) => {
                self.value = Some(value);
                self.current_key = key.as_str().map(|s| s.to_string());
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
        match self.iter.size_hint() {
            (lower, Some(upper)) if lower == upper => Some(upper),
            _ => None,
        }
    }
}

impl<'de, U, F> Deserializer<'de> for MapDeserializer<'_, '_, U, F>
where
    U: for<'p, 'v> FnMut(Path<'p>, &'v Value, &'v Value),
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

pub(crate) struct StructDeserializer<'a, 'f, U, F> {
    iter: <Mapping as IntoIterator>::IntoIter,
    current_key: Option<String>,
    path: Path<'a>,
    value: Option<Value>,
    normal_keys: HashSet<&'static str>,
    flatten_keys: Vec<&'static str>,
    unused_key_callback: Option<&'f mut U>,
    field_transformer: Option<&'f mut F>,
    rest: Vec<(Value, Value)>,
    flatten_keys_done: usize,
}

impl<'a, 'f, U, F> StructDeserializer<'a, 'f, U, F>
where
    U: for<'p, 'v> FnMut(Path<'p>, &'v Value, &'v Value),
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
{
    pub(crate) fn new(
        map: Mapping,
        current_path: Path<'a>,
        known_keys: &'static [&'static str],
        unused_key_callback: Option<&'f mut U>,
        field_transformer: Option<&'f mut F>,
    ) -> Self {
        let (normal_keys, flatten_keys): (Vec<_>, Vec<_>) = known_keys
            .iter()
            .copied()
            .partition(|key| !crate::is_flatten_key(key.as_bytes()));
        StructDeserializer {
            iter: map.into_iter(),
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

impl<'de, U, F> MapAccess<'de> for StructDeserializer<'_, '_, U, F>
where
    U: for<'p, 'v> FnMut(Path<'p>, &'v Value, &'v Value),
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
{
    type Error = Error;

    fn next_key_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        self.current_key = None;
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
                                let key_string = key_str.to_string();
                                let path = Path::Map {
                                    parent: &self.path,
                                    key: &key_string,
                                };
                                callback(path, &key, &value);
                                continue;
                            }
                        }
                        _ => {}
                    };

                    self.current_key = key.as_str().map(|s| s.to_string());
                    self.value = Some(value);
                    break seed.deserialize(ValueDeserializer::new(key)).map(Some);
                }
                None if self.has_unprocessed_flatten_keys() => {
                    let key = self.flatten_keys[self.flatten_keys_done];
                    self.current_key = Some(key.to_string());
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

                let path = match self.current_key {
                    Some(ref key) => Path::Map {
                        parent: &self.path,
                        key,
                    },
                    None => Path::Unknown { parent: &self.path },
                };

                if self.has_unprocessed_flatten_keys() {
                    let rest = self.rest.drain(..).collect::<Mapping>();
                    let deserializer = FlattenDeserializer::new(
                        rest.into_iter(),
                        path,
                        &mut self.rest,
                        self.field_transformer.as_deref_mut(),
                    );
                    seed.deserialize(deserializer)
                } else {
                    let deserializer = ValueDeserializer::new_with(
                        Value::mapping(self.rest.drain(..).collect()),
                        path,
                        self.unused_key_callback.as_deref_mut(),
                        self.field_transformer.as_deref_mut(),
                    );
                    seed.deserialize(deserializer)
                }
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

impl<'de, U, F> Deserializer<'de> for StructDeserializer<'_, '_, U, F>
where
    U: for<'p, 'v> FnMut(Path<'p>, &'v Value, &'v Value),
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

struct FlattenDeserializer<'p, 'f, 'r, F> {
    iter: <Mapping as IntoIterator>::IntoIter,
    path: Path<'p>,
    remaining: &'r mut Vec<(Value, Value)>,
    field_transformer: Option<&'f mut F>,
}

impl<'p, 'f, 'r, F> FlattenDeserializer<'p, 'f, 'r, F>
where
    F: for<'v> FnMut(&'v Value) -> TransformedResult,
{
    pub(crate) fn new(
        iter: <Mapping as IntoIterator>::IntoIter,
        current_path: Path<'p>,
        remaining: &'r mut Vec<(Value, Value)>,
        field_transformer: Option<&'f mut F>,
    ) -> Self {
        FlattenDeserializer {
            iter,
            path: current_path,
            remaining,
            field_transformer,
        }
    }
}

impl<'p, 'f, 'r, 'de, F> Deserializer<'de> for FlattenDeserializer<'p, 'f, 'r, F>
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
            self.remaining.push((key.clone(), value.clone()));
        };
        let deserializer = MapDeserializer {
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
            self.remaining.push((key.clone(), value.clone()));
        };

        let (normal_keys, flatten_keys): (Vec<_>, Vec<_>) = fields
            .iter()
            .copied()
            .partition(|key| !crate::is_flatten_key(key.as_bytes()));
        let deserializer = StructDeserializer {
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
