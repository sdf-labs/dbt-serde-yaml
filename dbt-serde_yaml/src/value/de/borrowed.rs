use std::slice;

use serde::{
    de::{
        value::BorrowedStrDeserializer, DeserializeSeed, EnumAccess, Error as _, MapAccess,
        SeqAccess, Unexpected, VariantAccess, Visitor,
    },
    forward_to_deserialize_any, Deserializer,
};

use crate::{error, value::tagged, Error, Mapping, Sequence, Value};

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
            Value::Null(..) => visitor.visit_unit(),
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
