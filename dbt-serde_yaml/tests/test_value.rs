#![allow(
    clippy::derive_partial_eq_without_eq,
    clippy::eq_op,
    clippy::uninlined_format_args
)]

use std::collections::HashMap;

use dbt_serde_yaml::Spanned;
use dbt_serde_yaml::{value::TransformedResult, Number, Value, Verbatim};
use dbt_serde_yaml_derive::UntaggedEnumDeserialize;
use indoc::indoc;
use serde::de::{DeserializeOwned, IntoDeserializer};
use serde::Deserialize as _;
use serde_derive::{Deserialize, Serialize};

#[test]
fn test_nan() {
    let pos_nan = dbt_serde_yaml::from_str::<Value>(".nan").unwrap();
    assert!(pos_nan.is_f64());
    assert_eq!(pos_nan, pos_nan);

    let neg_fake_nan = dbt_serde_yaml::from_str::<Value>("-.nan").unwrap();
    assert!(neg_fake_nan.is_string());

    let significand_mask = 0xF_FFFF_FFFF_FFFF;
    let bits = (f64::NAN.copysign(1.0).to_bits() ^ significand_mask) | 1;
    let different_pos_nan = Value::number(Number::from(f64::from_bits(bits)));
    assert_eq!(pos_nan, different_pos_nan);
}

#[test]
fn test_digits() {
    let num_string = dbt_serde_yaml::from_str::<Value>("01").unwrap();
    assert!(num_string.is_string());
}

#[test]
fn test_into_deserializer() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Test {
        first: String,
        second: u32,
    }

    let value = dbt_serde_yaml::from_str::<Value>("xyz").unwrap();
    let s = String::deserialize(value.into_deserializer()).unwrap();
    assert_eq!(s, "xyz");

    let value = dbt_serde_yaml::from_str::<Value>("- first\n- second\n- third").unwrap();
    let arr = Vec::<String>::deserialize(value.into_deserializer()).unwrap();
    assert_eq!(arr, &["first", "second", "third"]);

    let value = dbt_serde_yaml::from_str::<Value>("first: abc\nsecond: 99").unwrap();
    let test = Test::deserialize(value.into_deserializer()).unwrap();
    assert_eq!(
        test,
        Test {
            first: "abc".to_string(),
            second: 99
        }
    );
}

type UnusedKeys = Vec<(String, Value, Value)>;

fn deserialize_value_inner<T: DeserializeOwned + PartialEq + std::fmt::Debug>(
    value: Value,
    mut transformer: impl Fn(&Value) -> TransformedResult,
) -> (Result<T, dbt_serde_yaml::Error>, UnusedKeys) {
    let mut unused_keys1 = vec![];
    let mut unused_keys2 = vec![];

    let borrowed_path: Result<T, dbt_serde_yaml::Error> = value.to_typed(
        |path, key: &Value, value: &Value| {
            unused_keys2.push((path.to_string(), key.clone(), value.clone()));
        },
        &mut transformer,
    );
    let owned_path: Result<T, dbt_serde_yaml::Error> = value.into_typed(
        |path, key: &Value, value: &Value| {
            unused_keys1.push((path.to_string(), key.clone(), value.clone()));
        },
        &mut transformer,
    );

    match (&borrowed_path, &owned_path) {
        (Ok(borrowed), Ok(owned)) => {
            assert_eq!(borrowed, owned);
        }
        (Err(e1), Err(e2)) => {
            assert_eq!(e1.to_string(), e2.to_string());
        }
        _ => {
            panic!(
                "Mismatched results: {:?} vs {:?}",
                borrowed_path, owned_path
            );
        }
    }

    assert!(unused_keys1 == unused_keys2);

    (owned_path, unused_keys1)
}

fn deserialize_value<T: DeserializeOwned + PartialEq + std::fmt::Debug>(
    value: Value,
    transformer: impl Fn(&Value) -> TransformedResult,
) -> (T, UnusedKeys) {
    let (result, unused_keys) = deserialize_value_inner(value, transformer);
    (result.unwrap(), unused_keys)
}

#[test]
fn test_into_typed() {
    fn transformer(v: &Value) -> Result<Option<Value>, Box<dyn std::error::Error + Send + Sync>> {
        match v {
            Value::String(s, span) => Ok(Some(Value::String(format!("{} name", s), span.clone()))),
            _ => Ok(None),
        }
    }

    let value = dbt_serde_yaml::from_str::<Value>("xyz").unwrap();
    let (s, unused_keys): (String, _) = deserialize_value(value, |_| Ok(None));
    assert!(unused_keys.is_empty());
    assert_eq!(s, "xyz");

    let value = dbt_serde_yaml::from_str::<Value>("- first\n- second\n- third").unwrap();
    let (arr, unused_keys): (Vec<String>, _) = deserialize_value(value, transformer);
    assert!(unused_keys.is_empty());
    assert_eq!(arr, &["first name", "second name", "third name"]);

    #[derive(Debug, Deserialize, PartialEq)]
    struct Test {
        first: String,
        second: u32,
    }
    #[derive(Debug, Deserialize, PartialEq)]
    struct Test2 {
        first: Verbatim<Value>,
        third: u32,
        fourth: Value,
    }

    let value = dbt_serde_yaml::from_str::<Value>(indoc! {"
        first: abc
        second: 99
        third: 100
        fourth: my
        "})
    .unwrap();

    let (test, unused_keys): (Test, _) = deserialize_value(value.clone(), transformer);
    assert_eq!(
        unused_keys,
        vec![
            (
                "third".to_string(),
                Value::string("third".to_string()),
                Value::number(Number::from(100))
            ),
            (
                "fourth".to_string(),
                Value::string("fourth".to_string()),
                Value::string("my".to_string())
            )
        ]
    );
    assert_eq!(
        test,
        Test {
            first: "abc name".to_string(),
            second: 99
        }
    );

    let (test2, unused_keys): (Test2, _) = deserialize_value(value, transformer);
    assert_eq!(
        unused_keys,
        vec![(
            "second".to_string(),
            Value::string("second".to_string()),
            Value::number(Number::from(99))
        )]
    );
    assert_eq!(
        test2,
        Test2 {
            // field_transform is not applied to `Verbatim`-typed fields:
            first: Value::string("abc".to_string()).into(),
            third: 100,
            // Non-verbatim fields should be transformed:
            fourth: Value::string("my name".to_string()),
        }
    );

    #[derive(Debug, Deserialize, PartialEq)]
    struct Test3 {
        first: Test,
        seconds: Vec<Verbatim<Value>>,
        third: Option<Value>,
        fourth: Option<Option<String>>,
        #[serde(flatten)]
        rest: Option<HashMap<String, Value>>,
    }

    let value = dbt_serde_yaml::from_str::<Value>(indoc! {"
        first:
          first: abc
          second: 99
          third: 100
        seconds:
          -   first: A
              second: 1
              third: 1
              fourth: D
          -   first: B
              third: 2
              fourth: X
        fourth: xyz
        third: xyz
        fifth:
          sixth: cde
        "});
    let (test3, unused_keys): (Test3, _) = deserialize_value(value.unwrap(), transformer);

    assert_eq!(
        unused_keys,
        vec![(
            "first.third".to_string(),
            Value::string("third".to_string()),
            Value::number(Number::from(100))
        )]
    );

    assert_eq!(
        test3.first,
        Test {
            first: "abc name".to_string(),
            second: 99
        }
    );
    assert_eq!(test3.third, Some(Value::string("xyz name".to_string())));
    assert_eq!(test3.fourth, Some(Some("xyz name".to_string())));
    assert_eq!(
        test3.rest,
        Some(HashMap::from([(
            "fifth".to_string(),
            Value::mapping(
                [(
                    Value::string("sixth".to_string()),
                    Value::string("cde name".to_string()),
                )]
                .into_iter()
                .collect()
            )
        )]))
    );
    assert_eq!(test3.seconds.len(), 2);
    let (test2_1, unused_keys): (Test2, _) =
        deserialize_value(test3.seconds[0].as_ref().clone(), |v| {
            if let Some(n) = v.as_u64() {
                Ok(Some(Value::number(Number::from(n + 2))))
            } else {
                Ok(None)
            }
        });
    assert_eq!(
        unused_keys,
        vec![(
            "second".to_string(),
            Value::string("second".to_string()),
            Value::number(Number::from(1))
        )],
    );
    assert_eq!(
        test2_1,
        Test2 {
            first: Value::string("A".to_string()).into(),
            third: 3,
            fourth: Value::string("D".to_string()),
        }
    );
}

#[test]
fn test_into_typed_external_err() {
    #[derive(Debug, PartialEq)]
    struct Error {
        msg: String,
    }
    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Error: {}", self.msg)
        }
    }
    impl std::error::Error for Error {}

    let value = dbt_serde_yaml::from_str::<Value>("xyz").unwrap();
    let (err, unused_keys) = deserialize_value_inner::<String>(value, |v| {
        Err(Error {
            msg: format!("error {}", v.as_str().unwrap()),
        }
        .into())
    });
    if !unused_keys.is_empty() {
        panic!("unexpected unused keys: {:?}", unused_keys);
    }
    assert_eq!(
        err.unwrap_err()
            .into_external()
            .unwrap()
            .downcast::<Error>()
            .unwrap(),
        Box::new(Error {
            msg: "error xyz".to_string()
        })
    );
}

#[test]
fn test_merge() {
    // From https://yaml.org/type/merge.html.
    let yaml = indoc! {"
        ---
        - &CENTER { x: 1, y: 2 }
        - &LEFT { x: 0, y: 2 }
        - &BIG { r: 10 }
        - &SMALL { r: 1 }
        - &RECURSE { <<: *CENTER }

        # All the following maps are equal:

        - # Explicit keys
          x: 1
          y: 2
          r: 10
          label: center/big

        - # Merge one map
          << : *CENTER
          r: 10
          label: center/big

        - # Merge multiple maps
          << : [ *CENTER, *BIG ]
          label: center/big

        - # Override
          << : [ *BIG, *LEFT, *SMALL ]
          x: 1
          label: center/big

        - # Merge with reference that also contains a merge
          << : *RECURSE
          r: 10
          label: center/big
    "};

    let mut value: Value = dbt_serde_yaml::from_str(yaml).unwrap();
    value.apply_merge().unwrap();
    assert!(value.span().is_valid());
    for i in 6..=9 {
        assert_eq!(value[5], value[i]);
    }
}

#[test]
fn test_debug() {
    let yaml = indoc! {"
        'Null': ~
        Bool: true
        Number: 1
        String: ...
        Sequence:
          - true
        EmptySequence: []
        EmptyMapping: {}
        Tagged: !tag true
    "};

    let value: Value = dbt_serde_yaml::from_str(yaml).unwrap();
    assert!(value.span().is_valid());
    let debug = format!("{:#?}", value);

    let expected = indoc! {r#"
        Mapping {
            "Null": Null,
            "Bool": Bool(true),
            "Number": Number(1),
            "String": String("..."),
            "Sequence": Sequence [
                Bool(true),
            ],
            "EmptySequence": Sequence [],
            "EmptyMapping": Mapping {},
            "Tagged": TaggedValue {
                tag: !tag,
                value: Bool(true),
            },
        }"#
    };

    assert_eq!(debug, expected);
}

#[test]
fn test_tagged() {
    #[derive(Serialize)]
    enum Enum {
        Variant(usize),
    }

    let value = dbt_serde_yaml::to_value(Enum::Variant(0)).unwrap();

    let deserialized: dbt_serde_yaml::Value = dbt_serde_yaml::from_value(value.clone()).unwrap();
    assert_eq!(value, deserialized);

    let serialized = dbt_serde_yaml::to_value(&value).unwrap();
    assert_eq!(value, serialized);
}

#[test]
fn test_value_span() {
    let yaml = "x: 1.0\ny: 2.0\n";
    let value: Value = dbt_serde_yaml::from_str(yaml).unwrap();
    assert!(value.span().is_valid());
    assert_eq!(value.span().start.index, 0);
    assert_eq!(value.span().start.line, 1);
    assert_eq!(value.span().start.column, 1);
    assert_eq!(value.span().end.index, 14);
    assert_eq!(value.span().end.line, 3);
    assert_eq!(value.span().end.column, 1);

    match value {
        Value::Mapping(map, ..) => {
            let v = map.get(Value::string("x".to_string())).unwrap();
            assert!(v.span().is_valid());
            assert_eq!(v.span().start.line, 1);
            assert_eq!(v.span().start.column, 4);
            assert_eq!(v.span().end.line, 2);
            assert_eq!(v.span().end.column, 1);
            assert_eq!(yaml[v.span().start.index..v.span().end.index].trim(), "1.0");

            let keys = map.keys().collect::<Vec<_>>();
            assert_eq!(keys.len(), 2);
            let x = keys[0];
            assert!(x.span().is_valid());
            assert_eq!(x.span().start.line, 1);
            assert_eq!(x.span().start.column, 1);
            assert_eq!(x.span().end.line, 1);
            assert_eq!(yaml[x.span().start.index..x.span().end.index].trim(), "x:");

            let y = keys[1];
            assert!(y.span().is_valid());
            assert_eq!(y.span().start.line, 2);
            assert_eq!(y.span().start.column, 1);
            assert_eq!(y.span().end.line, 2);
            assert_eq!(yaml[y.span().start.index..y.span().end.index].trim(), "y:");
        }
        _ => panic!("expected mapping"),
    }
}

#[test]
fn test_value_span_multidoc() {
    let yaml = indoc! {"
        ---
        x: 1.0
        y: 2.0
        ---
        struc: !wat
          x: 0
        tuple: !wat
          - 0
          - 0
        newtype: !wat 0
        map: !wat
          x: 0
        vec: !wat
          - 0
        ---
    "};
    let mut values = vec![];
    for document in dbt_serde_yaml::Deserializer::from_str(yaml) {
        let value = Value::deserialize(document).unwrap();
        values.push(value);
    }
    assert_eq!(values.len(), 3);
    assert!(values[0].span().is_valid());
    assert!(values[1].span().is_valid());
    assert_eq!(
        yaml[values[0].span().start.index..values[0].span().end.index].trim(),
        "x: 1.0\ny: 2.0"
    );

    assert_eq!(values[1].span().start.line, 5);
    assert_eq!(values[1].span().start.column, 1);

    let struc_key_span = values[1]
        .as_mapping()
        .unwrap()
        .keys()
        .next()
        .unwrap()
        .span();
    assert_eq!(struc_key_span.start.line, 5);
    assert_eq!(struc_key_span.start.column, 1);
    assert_eq!(struc_key_span.end.line, 5);
    assert_eq!(struc_key_span.end.column, 8);

    let tuple_span = values[1].get("tuple").unwrap().span();
    assert_eq!(
        yaml[tuple_span.start.index..tuple_span.end.index].trim(),
        "!wat\n  - 0\n  - 0"
    );
}

#[test]
fn test_verbatim() {
    let yaml = indoc! {"
        x: 1
        y: 2
        z: 3
    "};

    #[derive(Deserialize, PartialEq, Eq, Debug, Hash)]
    struct Thing {
        x: i32,
        y: Verbatim<i32>,
        z: Verbatim<Option<i32>>,
        v: Verbatim<Option<String>>,
    }

    let value = dbt_serde_yaml::from_str::<Value>(yaml).unwrap();
    let thing: Thing = value
        .into_typed(
            |path, key: &Value, _| {
                panic!("unexpected key {:?} at path {:?}", key, path.to_string());
            },
            |v| {
                if let Some(v) = v.as_i64() {
                    Ok(Some(Value::from(v + 100)))
                } else {
                    Ok(None)
                }
            },
        )
        .unwrap();

    assert_eq!(thing.x, 101);
    assert_eq!(*thing.y, 2);
    assert_eq!(*thing.z, Some(3));
    assert!(thing.v.is_none());

    let thing2: Thing = dbt_serde_yaml::from_str(indoc! {"
        x: 101
        y: 2
        z: 3
    "})
    .unwrap();
    assert_eq!(thing, thing2);
}

#[test]
fn test_verbatim_flatten() {
    #[derive(Deserialize, Serialize, PartialEq, Eq, Debug)]
    struct Thing2 {
        x: Option<i32>,
        y: Verbatim<i32>,
        #[serde(flatten)]
        rest: Verbatim<HashMap<String, Option<i32>>>,
    }

    let value = dbt_serde_yaml::from_str::<Value>(indoc! {"
        x: 1
        y: 2
        z: 3
    "})
    .unwrap();
    let thing2: Thing2 = value
        .into_typed(
            |path, key: &Value, _| {
                panic!("unexpected key {:?} at path {:?}", key, path.to_string());
            },
            |v| {
                if v.is_i64() {
                    Ok(Some(Value::null()))
                } else {
                    Ok(None)
                }
            },
        )
        .unwrap();
    assert_eq!(thing2.x, None);
    assert_eq!(*thing2.y, 2);
    // Note: unfortunately `Verbatim` does not work in `flatten` fields:
    assert_eq!(*thing2.rest, HashMap::from_iter([("z".to_string(), None)]));

    let value = dbt_serde_yaml::to_value(thing2).unwrap();
    assert_eq!(
        value,
        dbt_serde_yaml::from_str::<Value>(indoc! {"
            x: null
            y: 2
            z: null
        "})
        .unwrap()
    );
}

#[cfg(feature = "flatten_dunder")]
#[test]
fn test_flatten() {
    #[derive(Deserialize, Serialize, PartialEq, Eq, Debug)]
    struct Thing3 {
        x: Option<i32>,
        y: Verbatim<i32>,
        __flatten__: HashMap<String, Verbatim<Option<i32>>>,
    }

    let value = dbt_serde_yaml::from_str::<Value>(indoc! {"
        x: 1
        y: 2
        z: 3
    "})
    .unwrap();
    let (thing3, unused_keys) = deserialize_value::<Thing3>(value, |_| Ok(None));
    if !unused_keys.is_empty() {
        panic!("unexpected unused keys: {:?}", unused_keys);
    }
    assert_eq!(thing3.x, Some(1));
    assert_eq!(*thing3.y, 2);
    assert_eq!(*thing3.__flatten__["z"], Some(3));

    let value = dbt_serde_yaml::to_value(thing3).unwrap();
    assert_eq!(
        value,
        dbt_serde_yaml::from_str::<Value>(indoc! {"
            x: 1
            y: 2
            z: 3
        "})
        .unwrap()
    );

    let value = dbt_serde_yaml::from_str::<Value>(indoc! {"
        y: 2
    "})
    .unwrap();
    let thing3 = Thing3::deserialize(value.into_deserializer()).unwrap();
    assert_eq!(
        thing3,
        Thing3 {
            x: None,
            y: 2.into(),
            __flatten__: HashMap::new()
        }
    );
}

#[cfg(feature = "flatten_dunder")]
#[test]
fn test_verbatim_flatten_nested() {
    #[derive(Deserialize, PartialEq, Eq, Debug)]
    struct Thing4 {
        x: Verbatim<Option<i32>>,
        __thing5__: Thing5,
    }

    #[derive(Deserialize, PartialEq, Eq, Debug)]
    struct Thing5 {
        a: Verbatim<Option<i32>>,
        __rest__: HashMap<String, Thing4>,
    }

    let value = dbt_serde_yaml::from_str::<Value>(indoc! {"
        a: 3
        x: 1
        b:
          x: 2
    "})
    .unwrap();
    let (thing4, unused_keys) = deserialize_value::<Thing4>(value, |v| {
        if v.is_i64() {
            panic!("transformer should not be called for i64 values: {:?}", v);
        } else {
            Ok(None)
        }
    });
    if !unused_keys.is_empty() {
        panic!("unexpected unused keys: {:?}", unused_keys);
    }
    assert_eq!(*thing4.x, Some(1));
    assert_eq!(*thing4.__thing5__.a, Some(3));
}

#[cfg(feature = "flatten_dunder")]
#[test]
fn test_multi_flatten_fields() {
    #[derive(Deserialize, PartialEq, Eq, Debug)]
    struct Thing6 {
        x: Option<i32>,
        __thing7__: Thing7,
        __rest__: HashMap<String, Option<i32>>,
        y: i32,
    }

    #[derive(Deserialize, PartialEq, Eq, Debug)]
    struct Thing7 {
        a: Option<i32>,
        __thing8__: Thing8,
    }

    #[derive(Deserialize, PartialEq, Eq, Debug)]
    struct Thing8 {
        b: Option<i32>,
    }

    let yaml = indoc! {"
        a: 3
        y: 5
        b: 4
        c: 1
    "};
    let (thing6, unused_keys) =
        deserialize_value::<Thing6>(dbt_serde_yaml::from_str::<Value>(yaml).unwrap(), |_| {
            Ok(None)
        });
    if !unused_keys.is_empty() {
        panic!("unexpected unused keys: {:?}", unused_keys);
    }
    assert_eq!(thing6.x, None);
    assert_eq!(thing6.__thing7__.a, Some(3));
    assert_eq!(thing6.__thing7__.__thing8__.b, Some(4));
    assert_eq!(thing6.__rest__, HashMap::from([("c".to_string(), Some(1))]));

    let yaml = indoc! {"
        a: 3
        x: 1
        b: 4
    "};
    let expected_err =
        dbt_serde_yaml::from_value::<Thing6>(dbt_serde_yaml::from_str::<Value>(yaml).unwrap())
            .unwrap_err()
            .to_string();
    assert_eq!(expected_err, "missing field `y` at line 1 column 1");
}

#[cfg(feature = "flatten_dunder")]
#[test]
fn test_multi_flatten_shouldbe() {
    use dbt_serde_yaml::ShouldBe;

    #[derive(Deserialize, PartialEq, Eq, Debug)]
    struct Thing6 {
        x: Option<i32>,
        __thing7__: Thing7,
        __rest__: HashMap<String, ShouldBe<Thing6>>,
        y: Verbatim<String>,
    }

    #[derive(Deserialize, PartialEq, Eq, Debug)]
    struct Thing7 {
        a: Option<i32>,
        __thing8__: Thing8,
    }

    #[derive(Deserialize, PartialEq, Eq, Debug)]
    struct Thing8 {
        c: Option<i32>,
    }

    let yaml = indoc! {"
        a: 3
        y: '5'
        b:
          x: 4
          y: 'a string'
        bb: 1
    "};
    let (thing6, unused_keys) =
        deserialize_value::<Thing6>(dbt_serde_yaml::from_str::<Value>(yaml).unwrap(), |v| {
            if v.is_string() {
                panic!(
                    "transformer should not be called for string values: {:?}",
                    v
                );
            } else {
                Ok(None)
            }
        });
    if !unused_keys.is_empty() {
        panic!("unexpected unused keys: {:?}", unused_keys);
    }
    assert_eq!(thing6.x, None);
    assert_eq!(thing6.__thing7__.a, Some(3));
    assert_eq!(thing6.__thing7__.__thing8__.c, None);
    assert_eq!(thing6.__rest__.len(), 2);
    assert!(thing6.__rest__["bb"].isnt());
    assert!(thing6.__rest__["b"].is());
    let inner = thing6.__rest__["b"].as_ref().unwrap();
    assert_eq!(inner.x, Some(4));
    assert_eq!(*inner.y, "a string");

    let yaml = indoc! {"
        a: 3
        x: 1
        b: 4
    "};
    let expected_err =
        dbt_serde_yaml::from_value::<Thing6>(dbt_serde_yaml::from_str::<Value>(yaml).unwrap())
            .unwrap_err()
            .to_string();
    assert_eq!(expected_err, "missing field `y` at line 1 column 1");
}

#[cfg(feature = "schemars")]
#[test]
fn test_schemars() {
    #![allow(dead_code)]

    use dbt_serde_yaml::JsonSchema;
    use schemars::schema_for;

    #[derive(JsonSchema)]
    struct Thing {
        x: Option<Option<i32>>,
        y: Verbatim<Value, String>,
        z: Verbatim<Option<Value>>,
        v: Verbatim<Option<Option<String>>>,
    }

    let schema = schema_for!(Thing);
    let schema_string = dbt_serde_yaml::to_string(&schema).unwrap();
    println!("{}", schema_string);
    assert_eq!(
        schema_string,
        indoc! {"
$schema: http://json-schema.org/draft-07/schema#
title: Thing
type: object
required:
- y
properties:
  v:
    type:
    - string
    - 'null'
  x:
    type:
    - integer
    - 'null'
    format: int32
  y:
    type: string
  z:
    anyOf:
    - $ref: '#/definitions/AnyValue'
    - type: 'null'
definitions:
  AnyValue: true
"}
    );
}

#[cfg(all(feature = "schemars", feature = "flatten_dunder"))]
#[test]
fn test_schemars_flatten() {
    #![allow(dead_code)]

    use dbt_serde_yaml::JsonSchema;
    use schemars::schema_for;

    #[derive(Deserialize, JsonSchema)]
    struct Thing {
        a: i32,
        #[serde(flatten)]
        rest: Verbatim<HashMap<String, Option<i32>>>,
    }

    #[derive(Deserialize, JsonSchema)]
    struct Thing2 {
        x: Option<i32>,
        y: Verbatim<Value, i32>,
        //        #[serde(flatten)]
        __thing__: Option<Thing>,
    }

    let schema = schema_for!(Thing2);
    let schema_string = dbt_serde_yaml::to_string(&schema).unwrap();
    println!("{}", schema_string);
    assert_eq!(
        schema_string,
        indoc! {"
$schema: http://json-schema.org/draft-07/schema#
title: Thing2
type: object
required:
- y
properties:
  a:
    type: integer
    format: int32
  x:
    type:
    - integer
    - 'null'
    format: int32
  y:
    type: integer
    format: int32
"}
    );
}

#[test]
fn test_untagged_enum() {
    #[derive(Deserialize, PartialEq, Eq, Debug)]
    struct Thing {
        a: Verbatim<i32>,
        b: Verbatim<Option<i32>>,
        c: bool,
    }

    #[allow(clippy::large_enum_variant)]
    #[derive(UntaggedEnumDeserialize, PartialEq, Eq, Debug)]
    #[serde(untagged)]
    enum Untagged<T> {
        String(String),
        Number(i32, i32),
        Thing(T),
        Unit,
    }

    let value = dbt_serde_yaml::from_str::<Value>("[42, 32]").unwrap();
    let untagged = deserialize_value::<Untagged<String>>(value, |_| Ok(None)).0;
    assert_eq!(untagged, Untagged::Number(42, 32));

    let value = dbt_serde_yaml::from_str::<Value>("'hello'").unwrap();
    let untagged = deserialize_value::<Untagged<i64>>(value, |_| Ok(None)).0;
    assert_eq!(untagged, Untagged::String("hello".to_string()));

    let value = dbt_serde_yaml::from_str::<Value>("").unwrap();
    let untagged = deserialize_value::<Untagged<Thing>>(value, |_| Ok(None)).0;
    assert_eq!(untagged, Untagged::Unit);

    let yaml = indoc! {"
        a: 3
        y: '5'
        c: false
    "};
    let value = dbt_serde_yaml::from_str::<Value>(yaml).unwrap();
    let (untagged, unused_keys) = deserialize_value::<Untagged<Thing>>(value, |v| {
        if v.is_u64() {
            Ok(Some(Value::from(v.as_u64().unwrap() + 100)))
        } else if v.is_bool() {
            Ok(Some(Value::from(true)))
        } else {
            Ok(None)
        }
    });
    assert_eq!(
        unused_keys,
        vec![(
            "y".to_string(),
            Value::string("y".to_string()),
            Value::string("5".to_string())
        )]
    );
    assert_eq!(
        untagged,
        Untagged::Thing(Thing {
            a: 3.into(),
            b: None.into(),
            c: true,
        })
    );

    let yaml = indoc! {"
        a: 3
        b: 4
    "};
    let value = dbt_serde_yaml::from_str::<Value>(yaml).unwrap();
    let expected_err = Untagged::<Thing>::deserialize(value.into_deserializer())
        .unwrap_err()
        .to_string();
    assert_eq!(
        expected_err,
        "data did not match any variant of untagged enum Untagged"
    );

    #[derive(Deserialize)]
    struct ThingWithSpanned {
        v: Spanned<Vec<Spanned<i32>>>,
    }

    let yaml = indoc! {"
        v:
          - 111
          - 222
    "};

    let value: dbt_serde_yaml::Value = dbt_serde_yaml::from_str(yaml).unwrap();
    let thing = match Untagged::<ThingWithSpanned>::deserialize(value.into_deserializer()).unwrap()
    {
        Untagged::Thing(thing) => thing,
        _ => panic!("expected Thing variant"),
    };
    assert_eq!(thing.v.len(), 2);
    assert_eq!(thing.v.span().start.line, 2);
    assert_eq!(thing.v.span().end.line, 4);
    assert_eq!(thing.v[0].span().start.index, 7);
    assert_eq!(thing.v[0].span().end.index, 15);
    assert_eq!(thing.v[0].span().start.line, 2);
    assert_eq!(thing.v[0].span().end.line, 3);
    assert_eq!(thing.v[1].span().start.line, 3);
    assert_eq!(thing.v[1].span().end.line, 4);
}

#[cfg(feature = "flatten_dunder")]
#[test]
fn test_untagged_enum_flatten_dunder() {
    #[derive(Deserialize, PartialEq, Eq, Debug)]
    struct Thing {
        a: Verbatim<i32>,
        b: Verbatim<Option<i32>>,
        c: bool,
    }

    #[allow(clippy::large_enum_variant)]
    #[derive(UntaggedEnumDeserialize, PartialEq, Eq, Debug)]
    #[serde(untagged)]
    enum Untagged<T> {
        String(String),
        Number(i32, i32),
        Thing(T),
        Unit,
    }

    #[derive(Deserialize, PartialEq, Eq, Debug)]
    struct Thing2 {
        __untagged__: Untagged<Thing>,
        __rest__: HashMap<String, String>,
    }

    let yaml = indoc! {"
        a: 3
        c: false
        y: '5'
    "};
    let value = dbt_serde_yaml::from_str::<Value>(yaml).unwrap();
    let (untagged, unused_keys) = deserialize_value::<Untagged<Thing2>>(value, |v| {
        if v.is_u64() {
            Ok(Some(Value::from(v.as_u64().unwrap() + 100)))
        } else if v.is_bool() {
            Ok(Some(Value::from(true)))
        } else {
            Ok(None)
        }
    });
    assert!(unused_keys.is_empty());
    assert_eq!(
        untagged,
        Untagged::Thing(Thing2 {
            __untagged__: Untagged::Thing(Thing {
                a: 3.into(),
                b: None.into(),
                c: true,
            }),
            // FIXME: flatten keys after a flattened untagged enum doesn't work yet:
            // __rest__: HashMap::from([("y".to_string(), "5".to_string())]),
            __rest__: HashMap::new(),
        })
    );

    let yaml = indoc! {"
        - a: 3
          c: false
        -
        - [1, 2]
        - 'hello'
    "};
    let value = dbt_serde_yaml::from_str::<Value>(yaml).unwrap();
    let (list, unused_keys) = deserialize_value::<Vec<Untagged<Thing2>>>(value, |v| {
        if v.is_u64() {
            Ok(Some(Value::from(v.as_u64().unwrap() + 100)))
        } else if v.is_bool() {
            Ok(Some(Value::from(true)))
        } else {
            Ok(None)
        }
    });
    assert!(unused_keys.is_empty());
    assert_eq!(list.len(), 4);
    assert_eq!(
        list[0],
        Untagged::Thing(Thing2 {
            __untagged__: Untagged::Thing(Thing {
                a: 3.into(),
                b: None.into(),
                c: true,
            }),
            __rest__: HashMap::new(),
        })
    );
    assert_eq!(list[1], Untagged::Unit);
    assert_eq!(list[2], Untagged::Number(101, 102));
    assert_eq!(list[3], Untagged::String("hello".to_string()));
}
