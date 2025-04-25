//! The Value enum, a loosely typed way of representing any valid YAML value.

mod de;
mod debug;
mod from;
mod index;
mod partial_eq;
mod ser;
pub(crate) mod tagged;

use crate::error::{self, Error, ErrorImpl};
use crate::{spanned, Span};
use serde::de::{Deserialize, DeserializeOwned, IntoDeserializer};
use serde::Serialize;
use std::hash::{Hash, Hasher};
use std::mem;

pub use self::index::Index;
pub use self::ser::Serializer;
pub use self::tagged::{Tag, TaggedValue};
#[doc(inline)]
pub use crate::mapping::Mapping;
pub use crate::number::Number;
#[doc(inline)]
pub(crate) use de::ValueVisitor;

/// Represents any valid YAML value.
#[derive(Clone)]
pub enum Value {
    /// Represents a YAML null value.
    Null(Span),
    /// Represents a YAML boolean.
    Bool(bool, Span),
    /// Represents a YAML numerical value, whether integer or floating point.
    Number(Number, Span),
    /// Represents a YAML string.
    String(String, Span),
    /// Represents a YAML sequence in which the elements are
    /// `dbt_serde_yaml::Value`.
    Sequence(Sequence, Span),
    /// Represents a YAML mapping in which the keys and values are both
    /// `dbt_serde_yaml::Value`.
    Mapping(Mapping, Span),
    /// A representation of YAML's `!Tag` syntax, used for enums.
    Tagged(Box<TaggedValue>, Span),
}

impl PartialEq for Value {
    fn eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Null(..), Value::Null(..)) => true,
            (Value::Bool(a, ..), Value::Bool(b, ..)) => a == b,
            (Value::Number(a, ..), Value::Number(b, ..)) => a == b,
            (Value::String(a, ..), Value::String(b, ..)) => a == b,
            (Value::Sequence(a, ..), Value::Sequence(b, ..)) => a == b,
            (Value::Mapping(a, ..), Value::Mapping(b, ..)) => a == b,
            (Value::Tagged(a, ..), Value::Tagged(b, ..)) => a == b,
            _ => false,
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Value) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Value::Null(..), Value::Null(..)) => Some(std::cmp::Ordering::Equal),
            (Value::Bool(a, ..), Value::Bool(b, ..)) => a.partial_cmp(b),
            (Value::Number(a, ..), Value::Number(b, ..)) => a.partial_cmp(b),
            (Value::String(a, ..), Value::String(b, ..)) => a.partial_cmp(b),
            (Value::Sequence(a, ..), Value::Sequence(b, ..)) => a.partial_cmp(b),
            (Value::Mapping(a, ..), Value::Mapping(b, ..)) => a.partial_cmp(b),
            (Value::Tagged(a, ..), Value::Tagged(b, ..)) => a.partial_cmp(b),
            _ => None,
        }
    }
}

/// The default value is `Value::Null`.
///
/// This is useful for handling omitted `Value` fields when deserializing.
///
/// # Examples
///
/// ```
/// # use serde_derive::Deserialize;
/// use serde::Deserialize as _;
/// use dbt_serde_yaml::Value;
///
/// #[derive(Deserialize)]
/// struct Settings {
///     level: i32,
///     #[serde(default)]
///     extras: Value,
/// }
///
/// # fn try_main() -> Result<(), dbt_serde_yaml::Error> {
/// let data = r#" { "level": 42 } "#;
/// let s: Settings = dbt_serde_yaml::from_str(data)?;
///
/// assert_eq!(s.level, 42);
/// assert_eq!(s.extras, Value::null());
/// #
/// #     Ok(())
/// # }
/// #
/// # try_main().unwrap()
/// ```
impl Default for Value {
    fn default() -> Value {
        Value::Null(Span::default())
    }
}

/// A YAML sequence in which the elements are `dbt_serde_yaml::Value`.
pub type Sequence = Vec<Value>;

/// Convert a `T` into `dbt_serde_yaml::Value` which is an enum that can represent
/// any valid YAML data.
///
/// This conversion can fail if `T`'s implementation of `Serialize` decides to
/// return an error.
///
/// ```
/// # use dbt_serde_yaml::Value;
/// let val = dbt_serde_yaml::to_value("s").unwrap();
/// assert_eq!(val, Value::string("s".to_owned()));
/// ```
pub fn to_value<T>(value: T) -> Result<Value, Error>
where
    T: Serialize,
{
    value.serialize(Serializer)
}

/// Interpret a `dbt_serde_yaml::Value` as an instance of type `T`.
///
/// This conversion can fail if the structure of the Value does not match the
/// structure expected by `T`, for example if `T` is a struct type but the Value
/// contains something other than a YAML map. It can also fail if the structure
/// is correct but `T`'s implementation of `Deserialize` decides that something
/// is wrong with the data, for example required struct fields are missing from
/// the YAML map or some number is too big to fit in the expected primitive
/// type.
///
/// ```
/// # use dbt_serde_yaml::Value;
/// let val = Value::string("foo".to_owned());
/// let s: String = dbt_serde_yaml::from_value(val).unwrap();
/// assert_eq!("foo", s);
/// ```
pub fn from_value<T>(value: Value) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    value.broadcast_start_mark();
    let res = Deserialize::deserialize(value.into_deserializer());
    spanned::reset_marker();
    res
}

impl Value {
    /// Index into a YAML sequence or map. A string index can be used to access
    /// a value in a map, and a usize index can be used to access an element of
    /// an sequence.
    ///
    /// Returns `None` if the type of `self` does not match the type of the
    /// index, for example if the index is a string and `self` is a sequence or
    /// a number. Also returns `None` if the given key does not exist in the map
    /// or the given index is not within the bounds of the sequence.
    ///
    /// ```
    /// # fn main() -> dbt_serde_yaml::Result<()> {
    /// use dbt_serde_yaml::Value;
    ///
    /// let object: Value = dbt_serde_yaml::from_str(r#"{ A: 65, B: 66, C: 67 }"#)?;
    /// let x = object.get("A").unwrap();
    /// assert_eq!(x, 65);
    ///
    /// let sequence: Value = dbt_serde_yaml::from_str(r#"[ "A", "B", "C" ]"#)?;
    /// let x = sequence.get(2).unwrap();
    /// assert_eq!(x, &Value::string("C".into()));
    ///
    /// assert_eq!(sequence.get("A"), None);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Square brackets can also be used to index into a value in a more concise
    /// way. This returns `Value::Null` in cases where `get` would have returned
    /// `None`.
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// #
    /// # fn main() -> dbt_serde_yaml::Result<()> {
    /// let object: Value = dbt_serde_yaml::from_str(r#"
    /// A: [a, á, à]
    /// B: [b, b́]
    /// C: [c, ć, ć̣, ḉ]
    /// 42: true
    /// "#)?;
    /// assert_eq!(object["B"][0], Value::string("b".into()));
    ///
    /// assert_eq!(object[Value::string("D".into())], Value::null());
    /// assert_eq!(object["D"], Value::null());
    /// assert_eq!(object[0]["x"]["y"]["z"], Value::null());
    ///
    /// assert_eq!(object[42], Value::bool(true));
    /// # Ok(())
    /// # }
    /// ```
    pub fn get<I: Index>(&self, index: I) -> Option<&Value> {
        index.index_into(self)
    }

    /// Index into a YAML sequence or map. A string index can be used to access
    /// a value in a map, and a usize index can be used to access an element of
    /// an sequence.
    ///
    /// Returns `None` if the type of `self` does not match the type of the
    /// index, for example if the index is a string and `self` is a sequence or
    /// a number. Also returns `None` if the given key does not exist in the map
    /// or the given index is not within the bounds of the sequence.
    pub fn get_mut<I: Index>(&mut self, index: I) -> Option<&mut Value> {
        index.index_into_mut(self)
    }

    /// Returns true if the `Value` is a Null. Returns false otherwise.
    ///
    /// For any Value on which `is_null` returns true, `as_null` is guaranteed
    /// to return `Some(())`.
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("null").unwrap();
    /// assert!(v.is_null());
    /// ```
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("false").unwrap();
    /// assert!(!v.is_null());
    /// ```
    pub fn is_null(&self) -> bool {
        if let Value::Null(..) = self.untag_ref() {
            true
        } else {
            false
        }
    }

    /// If the `Value` is a Null, returns (). Returns None otherwise.
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("null").unwrap();
    /// assert_eq!(v.as_null(), Some(()));
    /// ```
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("false").unwrap();
    /// assert_eq!(v.as_null(), None);
    /// ```
    pub fn as_null(&self) -> Option<()> {
        match self.untag_ref() {
            Value::Null(..) => Some(()),
            _ => None,
        }
    }

    /// Returns true if the `Value` is a Boolean. Returns false otherwise.
    ///
    /// For any Value on which `is_boolean` returns true, `as_bool` is
    /// guaranteed to return the boolean value.
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("true").unwrap();
    /// assert!(v.is_bool());
    /// ```
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("42").unwrap();
    /// assert!(!v.is_bool());
    /// ```
    pub fn is_bool(&self) -> bool {
        self.as_bool().is_some()
    }

    /// If the `Value` is a Boolean, returns the associated bool. Returns None
    /// otherwise.
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("true").unwrap();
    /// assert_eq!(v.as_bool(), Some(true));
    /// ```
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("42").unwrap();
    /// assert_eq!(v.as_bool(), None);
    /// ```
    pub fn as_bool(&self) -> Option<bool> {
        match self.untag_ref() {
            Value::Bool(b, ..) => Some(*b),
            _ => None,
        }
    }

    /// Returns true if the `Value` is a Number. Returns false otherwise.
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("5").unwrap();
    /// assert!(v.is_number());
    /// ```
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("true").unwrap();
    /// assert!(!v.is_number());
    /// ```
    pub fn is_number(&self) -> bool {
        match self.untag_ref() {
            Value::Number(..) => true,
            _ => false,
        }
    }

    /// Returns true if the `Value` is an integer between `i64::MIN` and
    /// `i64::MAX`.
    ///
    /// For any Value on which `is_i64` returns true, `as_i64` is guaranteed to
    /// return the integer value.
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("1337").unwrap();
    /// assert!(v.is_i64());
    /// ```
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("null").unwrap();
    /// assert!(!v.is_i64());
    /// ```
    pub fn is_i64(&self) -> bool {
        self.as_i64().is_some()
    }

    /// If the `Value` is an integer, represent it as i64 if possible. Returns
    /// None otherwise.
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("1337").unwrap();
    /// assert_eq!(v.as_i64(), Some(1337));
    /// ```
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("false").unwrap();
    /// assert_eq!(v.as_i64(), None);
    /// ```
    pub fn as_i64(&self) -> Option<i64> {
        match self.untag_ref() {
            Value::Number(n, ..) => n.as_i64(),
            _ => None,
        }
    }

    /// Returns true if the `Value` is an integer between `u64::MIN` and
    /// `u64::MAX`.
    ///
    /// For any Value on which `is_u64` returns true, `as_u64` is guaranteed to
    /// return the integer value.
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("1337").unwrap();
    /// assert!(v.is_u64());
    /// ```
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("null").unwrap();
    /// assert!(!v.is_u64());
    /// ```
    pub fn is_u64(&self) -> bool {
        self.as_u64().is_some()
    }

    /// If the `Value` is an integer, represent it as u64 if possible. Returns
    /// None otherwise.
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("1337").unwrap();
    /// assert_eq!(v.as_u64(), Some(1337));
    /// ```
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("false").unwrap();
    /// assert_eq!(v.as_u64(), None);
    /// ```
    pub fn as_u64(&self) -> Option<u64> {
        match self.untag_ref() {
            Value::Number(n, ..) => n.as_u64(),
            _ => None,
        }
    }

    /// Returns true if the `Value` is a number that can be represented by f64.
    ///
    /// For any Value on which `is_f64` returns true, `as_f64` is guaranteed to
    /// return the floating point value.
    ///
    /// Currently this function returns true if and only if both `is_i64` and
    /// `is_u64` return false but this is not a guarantee in the future.
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("256.01").unwrap();
    /// assert!(v.is_f64());
    /// ```
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("true").unwrap();
    /// assert!(!v.is_f64());
    /// ```
    pub fn is_f64(&self) -> bool {
        match self.untag_ref() {
            Value::Number(n, ..) => n.is_f64(),
            _ => false,
        }
    }

    /// If the `Value` is a number, represent it as f64 if possible. Returns
    /// None otherwise.
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("13.37").unwrap();
    /// assert_eq!(v.as_f64(), Some(13.37));
    /// ```
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("false").unwrap();
    /// assert_eq!(v.as_f64(), None);
    /// ```
    pub fn as_f64(&self) -> Option<f64> {
        match self.untag_ref() {
            Value::Number(i, ..) => i.as_f64(),
            _ => None,
        }
    }

    /// Returns true if the `Value` is a String. Returns false otherwise.
    ///
    /// For any Value on which `is_string` returns true, `as_str` is guaranteed
    /// to return the string slice.
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("'lorem ipsum'").unwrap();
    /// assert!(v.is_string());
    /// ```
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("42").unwrap();
    /// assert!(!v.is_string());
    /// ```
    pub fn is_string(&self) -> bool {
        self.as_str().is_some()
    }

    /// If the `Value` is a String, returns the associated str. Returns None
    /// otherwise.
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("'lorem ipsum'").unwrap();
    /// assert_eq!(v.as_str(), Some("lorem ipsum"));
    /// ```
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("false").unwrap();
    /// assert_eq!(v.as_str(), None);
    /// ```
    pub fn as_str(&self) -> Option<&str> {
        match self.untag_ref() {
            Value::String(s, ..) => Some(s),
            _ => None,
        }
    }

    /// Returns true if the `Value` is a sequence. Returns false otherwise.
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("[1, 2, 3]").unwrap();
    /// assert!(v.is_sequence());
    /// ```
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("true").unwrap();
    /// assert!(!v.is_sequence());
    /// ```
    pub fn is_sequence(&self) -> bool {
        self.as_sequence().is_some()
    }

    /// If the `Value` is a sequence, return a reference to it if possible.
    /// Returns None otherwise.
    ///
    /// ```
    /// # use dbt_serde_yaml::{Value, Number};
    /// let v: Value = dbt_serde_yaml::from_str("[1, 2]").unwrap();
    /// assert_eq!(v.as_sequence(), Some(&vec![Value::number(Number::from(1)), Value::number(Number::from(2))]));
    /// ```
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("false").unwrap();
    /// assert_eq!(v.as_sequence(), None);
    /// ```
    pub fn as_sequence(&self) -> Option<&Sequence> {
        match self.untag_ref() {
            Value::Sequence(seq, ..) => Some(seq),
            _ => None,
        }
    }

    /// If the `Value` is a sequence, return a mutable reference to it if
    /// possible. Returns None otherwise.
    ///
    /// ```
    /// # use dbt_serde_yaml::{Value, Number};
    /// let mut v: Value = dbt_serde_yaml::from_str("[1]").unwrap();
    /// let s = v.as_sequence_mut().unwrap();
    /// s.push(Value::number(Number::from(2)));
    /// assert_eq!(s, &vec![Value::number(Number::from(1)), Value::number(Number::from(2))]);
    /// ```
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let mut v: Value = dbt_serde_yaml::from_str("false").unwrap();
    /// assert_eq!(v.as_sequence_mut(), None);
    /// ```
    pub fn as_sequence_mut(&mut self) -> Option<&mut Sequence> {
        match self.untag_mut() {
            Value::Sequence(seq, ..) => Some(seq),
            _ => None,
        }
    }

    /// Returns true if the `Value` is a mapping. Returns false otherwise.
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("a: 42").unwrap();
    /// assert!(v.is_mapping());
    /// ```
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("true").unwrap();
    /// assert!(!v.is_mapping());
    /// ```
    pub fn is_mapping(&self) -> bool {
        self.as_mapping().is_some()
    }

    /// If the `Value` is a mapping, return a reference to it if possible.
    /// Returns None otherwise.
    ///
    /// ```
    /// # use dbt_serde_yaml::{Value, Mapping, Number};
    /// let v: Value = dbt_serde_yaml::from_str("a: 42").unwrap();
    ///
    /// let mut expected = Mapping::new();
    /// expected.insert(Value::string("a".into()),Value::number(Number::from(42)));
    ///
    /// assert_eq!(v.as_mapping(), Some(&expected));
    /// ```
    ///
    /// ```
    /// # use dbt_serde_yaml::Value;
    /// let v: Value = dbt_serde_yaml::from_str("false").unwrap();
    /// assert_eq!(v.as_mapping(), None);
    /// ```
    pub fn as_mapping(&self) -> Option<&Mapping> {
        match self.untag_ref() {
            Value::Mapping(map, ..) => Some(map),
            _ => None,
        }
    }

    /// If the `Value` is a mapping, return a reference to it if possible.
    /// Returns None otherwise.
    ///
    /// ```
    /// # use dbt_serde_yaml::{Value, Mapping, Number};
    /// let mut v: Value = dbt_serde_yaml::from_str("a: 42").unwrap();
    /// let m = v.as_mapping_mut().unwrap();
    /// m.insert(Value::string("b".into()), Value::number(Number::from(21)));
    ///
    /// let mut expected = Mapping::new();
    /// expected.insert(Value::string("a".into()), Value::number(Number::from(42)));
    /// expected.insert(Value::string("b".into()), Value::number(Number::from(21)));
    ///
    /// assert_eq!(m, &expected);
    /// ```
    ///
    /// ```
    /// # use dbt_serde_yaml::{Value, Mapping};
    /// let mut v: Value = dbt_serde_yaml::from_str("false").unwrap();
    /// assert_eq!(v.as_mapping_mut(), None);
    /// ```
    pub fn as_mapping_mut(&mut self) -> Option<&mut Mapping> {
        match self.untag_mut() {
            Value::Mapping(map, ..) => Some(map),
            _ => None,
        }
    }

    /// Performs merging of `<<` keys into the surrounding mapping.
    ///
    /// The intended use of this in YAML is described in
    /// <https://yaml.org/type/merge.html>.
    ///
    /// ```
    /// use dbt_serde_yaml::Value;
    ///
    /// let config = "\
    /// tasks:
    ///   build: &webpack_shared
    ///     command: webpack
    ///     args: build
    ///     inputs:
    ///       - 'src/**/*'
    ///   start: &start
    ///     <<: *webpack_shared
    ///     args: start
    ///     outputs:
    ///       - 'dist/**/*'
    ///   progress:
    ///     <<: *start
    ///     args: progress
    /// ";
    ///
    /// let mut value: Value = dbt_serde_yaml::from_str(config).unwrap();
    /// value.apply_merge().unwrap();
    ///
    /// assert_eq!(value["tasks"]["start"]["command"], "webpack");
    /// assert_eq!(value["tasks"]["start"]["args"], "start");
    /// assert_eq!(value["tasks"]["progress"]["command"], "webpack");
    /// assert_eq!(value["tasks"]["progress"]["args"], "progress");
    /// assert_eq!(value["tasks"]["progress"]["outputs"][0], "dist/**/*");
    /// ```
    pub fn apply_merge(&mut self) -> Result<(), Error> {
        let mut stack = Vec::new();
        stack.push(self);
        while let Some(node) = stack.pop() {
            match node {
                Value::Mapping(mapping, ..) => {
                    loop {
                        match mapping.remove("<<") {
                            Some(Value::Mapping(merge, ..)) => {
                                for (k, v) in merge {
                                    mapping.entry(k).or_insert(v);
                                }
                            }
                            Some(Value::Sequence(sequence, ..)) => {
                                for value in sequence {
                                    match value {
                                        Value::Mapping(merge, ..) => {
                                            for (k, v) in merge {
                                                mapping.entry(k).or_insert(v);
                                            }
                                        }
                                        Value::Sequence(..) => {
                                            return Err(error::new(
                                                ErrorImpl::SequenceInMergeElement,
                                            ));
                                        }
                                        Value::Tagged(..) => {
                                            return Err(error::new(ErrorImpl::TaggedInMerge));
                                        }
                                        _unexpected => {
                                            return Err(error::new(
                                                ErrorImpl::ScalarInMergeElement,
                                            ));
                                        }
                                    }
                                }
                            }
                            None => {
                                break;
                            }
                            Some(Value::Tagged(..)) => {
                                return Err(error::new(ErrorImpl::TaggedInMerge))
                            }
                            Some(_unexpected) => return Err(error::new(ErrorImpl::ScalarInMerge)),
                        }
                    }
                    stack.extend(mapping.values_mut());
                }
                Value::Sequence(sequence, ..) => stack.extend(sequence),
                Value::Tagged(tagged, ..) => stack.push(&mut tagged.value),
                _ => {}
            }
        }
        Ok(())
    }

    /// Returns the contained [Span].
    pub fn span(&self) -> Span {
        match self {
            Value::Null(span)
            | Value::Bool(_, span)
            | Value::Number(_, span)
            | Value::Sequence(_, span)
            | Value::Mapping(_, span)
            | Value::Tagged(_, span)
            | Value::String(_, span) => span.clone(),
        }
    }

    /// Replace the span of this value with the given span.
    pub fn with_span(self, span: impl Into<Span>) -> Self {
        let mut this = self;
        let span = span.into();
        this.set_span(span);
        this
    }

    /// Set the span of the value.
    fn set_span(&mut self, span: Span) {
        match self {
            Value::Null(ref mut s)
            | Value::Bool(_, ref mut s)
            | Value::Number(_, ref mut s)
            | Value::Sequence(_, ref mut s)
            | Value::Mapping(_, ref mut s)
            | Value::Tagged(_, ref mut s)
            | Value::String(_, ref mut s) => *s = span,
        }
    }

    fn broadcast_start_mark(&self) {
        spanned::set_marker(self.span().start);
        #[cfg(feature = "filename")]
        if let Some(filename) = self.span().filename {
            spanned::set_filename(filename);
        }
    }

    fn broadcast_end_mark(&self) {
        spanned::set_marker(self.span().end);
        #[cfg(feature = "filename")]
        if let Some(filename) = self.span().filename {
            spanned::set_filename(filename);
        }
    }
}

// Default constructors
impl Value {
    /// Construct a Null Value with no location information.
    pub const fn null() -> Value {
        Value::Null(Span::zero())
    }

    /// Construct a Bool Value with no location information.
    pub const fn bool(b: bool) -> Value {
        Value::Bool(b, Span::zero())
    }

    /// Construct a Number Value with no location information.
    pub const fn number(n: Number) -> Value {
        Value::Number(n, Span::zero())
    }

    /// Construct a String Value with no location information.
    pub const fn string(s: String) -> Value {
        Value::String(s, Span::zero())
    }

    /// Construct a Sequence Value with no location information.
    pub fn sequence(seq: Sequence) -> Value {
        Value::Sequence(seq, Span::zero())
    }

    /// Construct a Mapping Value with no location information.
    pub fn mapping(map: Mapping) -> Value {
        Value::Mapping(map, Span::zero())
    }

    /// Construct a Tagged Value with no location information.
    pub fn tagged(tagged: impl Into<Box<TaggedValue>>) -> Value {
        Value::Tagged(tagged.into(), Span::zero())
    }
}

impl Eq for Value {}

// NOTE: This impl must be kept consistent with HashLikeValue's Hash impl in
// mapping.rs in order for value[str] indexing to work.
impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        mem::discriminant(self).hash(state);
        match self {
            Value::Null(..) => {}
            Value::Bool(v, ..) => v.hash(state),
            Value::Number(v, ..) => v.hash(state),
            Value::String(v, ..) => v.hash(state),
            Value::Sequence(v, ..) => v.hash(state),
            Value::Mapping(v, ..) => v.hash(state),
            Value::Tagged(v, ..) => v.hash(state),
        }
    }
}

impl IntoDeserializer<'_, Error> for Value {
    type Deserializer = de::ValueDeserializer<
        'static,
        fn(Value, Value),
        fn(Value) -> Result<Value, Box<dyn std::error::Error + 'static + Send + Sync>>,
    >;

    fn into_deserializer(self) -> Self::Deserializer {
        de::ValueDeserializer::new(self)
    }
}

#[cfg(feature = "schemars")]
impl schemars::JsonSchema for Value {
    fn schema_name() -> String {
        "AnyValue".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::schema::Schema {
        true.into()
    }
}
