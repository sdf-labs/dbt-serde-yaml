#![allow(
    clippy::derive_partial_eq_without_eq,
    clippy::eq_op,
    clippy::uninlined_format_args
)]

use dbt_serde_yaml::{Number, Value};
use indoc::indoc;
use serde::de::IntoDeserializer;
use serde::Deserialize;
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

#[test]
fn test_merge() {
    // From https://yaml.org/type/merge.html.
    let yaml = indoc! {"
        ---
        - &CENTER { x: 1, y: 2 }
        - &LEFT { x: 0, y: 2 }
        - &BIG { r: 10 }
        - &SMALL { r: 1 }

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
    "};

    let mut value: Value = dbt_serde_yaml::from_str(yaml).unwrap();
    assert!(value.span().is_valid());
    value.apply_merge().unwrap();
    for i in 5..=7 {
        assert_eq!(value[4], value[i]);
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
