use dbt_serde_yaml::{Error, Number, ShouldBe, Value, WhyNot};
use serde::de::Error as _;
use serde_derive::Deserialize;

#[test]
fn test_shouldbe() {
    let valid: ShouldBe<i32> = ShouldBe::AndIs(42);
    let invalid: ShouldBe<i32> = ShouldBe::ButIsnt {
        raw: Some(Value::number(Number::from(0))),
        why_not: WhyNot::Original(Error::custom("Expected a number")),
    };

    assert!(valid.is());
    assert!(invalid.isnt());

    assert_eq!(valid.as_ref(), Some(&42));
    assert_eq!(invalid.as_ref(), None);
    assert_eq!(invalid.as_ref_raw(), Some(&Value::number(Number::from(0))));
    assert_eq!(
        invalid.as_ref_err().unwrap().to_string(),
        "Expected a number"
    );
}

#[test]
fn test_deserialize_str() {
    let yaml = r#"
        valid: 42
        invalid: 
          raw: 0
          why_not: "Expected a number"
    "#;

    let deserialized: std::result::Result<std::collections::HashMap<String, ShouldBe<i32>>, _> =
        dbt_serde_yaml::from_str(yaml);

    assert!(deserialized.is_ok());
    let map = deserialized.unwrap();

    assert_eq!(map["valid"].as_ref(), Some(&42));
    assert!(map["invalid"].isnt());
    assert_eq!(
        map["invalid"].as_ref_err().unwrap().to_string(),
        "invalid: invalid type: map, expected i32 at line 4 column 11"
    );
}

#[test]
fn test_deserialize_value() {
    let yaml = r#"
        valid: 
          x: 42
        invalid: 
          x: "Expected a number"
    "#;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Inner {
        x: i32,
    }

    #[derive(Debug, Deserialize)]
    struct Outer {
        valid: ShouldBe<Inner>,
        invalid: ShouldBe<Inner>,
    }

    let value: Value = dbt_serde_yaml::from_str(yaml).unwrap();

    let thing: ShouldBe<Outer> = value
        .into_typed(
            |_, _, _| panic!("Unused key in deserialization"),
            |_| Ok(None),
        )
        .unwrap();

    assert!(thing.is());
    let thing = thing.into_inner().unwrap();
    assert_eq!(thing.valid, ShouldBe::AndIs(Inner { x: 42 }));
    assert_eq!(thing.valid.as_ref().unwrap().x, 42);
    assert!(thing.invalid.isnt());
    assert_eq!(
        thing.invalid.as_ref_raw().unwrap(),
        &Value::mapping(
            [(
                Value::string("x".to_string()),
                Value::string("Expected a number".to_string())
            )]
            .into_iter()
            .collect()
        )
    );
    assert_eq!(
        thing.invalid.as_ref_err().unwrap().to_string(),
        "invalid type: string \"Expected a number\", expected i32 at line 5 column 14"
    );
}
