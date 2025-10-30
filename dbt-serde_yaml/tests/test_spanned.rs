use std::collections::HashSet;

use dbt_serde_yaml::{Span, Spanned, UntaggedEnumDeserialize};
use indoc::indoc;
use serde::Deserialize as _;
use serde_derive::{Deserialize, Serialize};

#[test]
fn test_spanned_basic() {
    #[derive(Deserialize, Serialize, PartialEq, Debug, Hash, Eq, Clone)]
    struct Point {
        x: u64,
        y: u64,
    }

    let v: Spanned<Point> = Point { x: 10, y: 20 }.into();
    assert_eq!(v.x, 10);

    #[derive(Deserialize, PartialEq, Debug, Hash, Eq, Clone)]
    struct Parent {
        child: Spanned<Point>,
    }
    let mut hashset: HashSet<Parent> = HashSet::new();
    let parent = Parent {
        child: Spanned::new(Point { x: 10, y: 20 }),
    };
    hashset.insert(parent.clone());
    assert!(hashset.contains(&parent));
}

#[test]
fn test_spanned_de_basic() {
    #[derive(Deserialize, Serialize, PartialEq, Debug)]
    struct Point {
        x: f64,
        y: f64,
    }

    let yaml = "x: 1.0\ny: 2.0\n";
    let spanned_point: Spanned<Point> = dbt_serde_yaml::from_str(yaml).unwrap();
    assert!(spanned_point.has_valid_span());
    assert_eq!(*spanned_point, Point { x: 1.0, y: 2.0 });
    assert_eq!(spanned_point.span().start.index, 0);
    assert_eq!(spanned_point.span().start.line, 1);
    assert_eq!(spanned_point.span().start.column, 1);
    assert_eq!(spanned_point.span().end.index, 14);
    assert_eq!(spanned_point.span().end.line, 3);
    assert_eq!(spanned_point.span().end.column, 1);

    #[derive(Deserialize)]
    struct Point2 {
        x: Spanned<f64>,
        y: Spanned<f64>,
    }

    let point2: Point2 = dbt_serde_yaml::from_str(yaml).unwrap();
    assert_eq!(*point2.x, 1.0);
    assert!(point2.x.has_valid_span());
    assert!(point2.y.has_valid_span());
    assert_eq!(point2.x.span().start.index, 3);
    assert_eq!(*point2.y, 2.0);
    assert_eq!(point2.y.span().start.line, 2);
    assert_eq!(point2.y.span().start.column, 4);
    assert_eq!(point2.y.span().end.line, 3);
    assert_eq!(point2.y.span().end.column, 1);
    assert_eq!(
        yaml[point2.x.span().start.index..point2.x.span().end.index].trim(),
        "1.0"
    );
    assert_eq!(
        yaml[point2.y.span().start.index..point2.y.span().end.index].trim(),
        "2.0"
    );
}

#[test]
fn test_spanned_de_multidoc() -> Result<(), dbt_serde_yaml::Error> {
    #[derive(Deserialize, Serialize, PartialEq, Debug)]
    struct Point {
        x: Spanned<f64>,
        y: Spanned<f64>,
    }

    let yaml = indoc! {"
        ---
        x: 1.0
        y: 2.0
        ---
        x: 3.0
        y: 4.0
    "};
    let mut points = vec![];
    for document in dbt_serde_yaml::Deserializer::from_str(yaml) {
        let point = Spanned::<Point>::deserialize(document)?;
        assert!(point.has_valid_span());
        points.push(point);
    }
    assert_eq!(*points[0].x, 1.0);
    assert_eq!(*points[0].y, 2.0);
    assert_eq!(*points[1].x, 3.0);
    assert_eq!(*points[1].y, 4.0);

    assert_eq!(
        yaml[points[0].span().start.index..points[0].span().end.index].trim(),
        "x: 1.0\ny: 2.0"
    );
    assert_eq!(
        yaml[points[1].span().start.index..points[1].span().end.index].trim(),
        "x: 3.0\ny: 4.0"
    );

    Ok(())
}

#[test]
fn test_spanned_ser() {
    #[derive(Deserialize, Serialize, PartialEq, Debug)]
    struct Point {
        x: f64,
        y: f64,
    }

    let point = Point { x: 1.0, y: 2.0 };
    let spanned_point = Spanned::new(point);
    let yaml = dbt_serde_yaml::to_string(&spanned_point).unwrap();
    assert_eq!(yaml, "x: 1.0\ny: 2.0\n");

    #[derive(Serialize)]
    struct Point2 {
        x: Spanned<f64>,
        y: Spanned<f64>,
    }

    let point2 = Point2 {
        x: Spanned::new(1.0),
        y: Spanned::new(2.0),
    };
    let yaml = dbt_serde_yaml::to_string(&point2).unwrap();
    assert_eq!(yaml, "x: 1.0\ny: 2.0\n");
}

#[test]
fn test_spanned_de_from_value() {
    #![allow(dead_code)]

    #[derive(Deserialize, Debug, PartialEq, Eq, Serialize)]
    struct Thing;

    #[derive(UntaggedEnumDeserialize, Debug, Serialize)]
    #[serde(untagged)]
    enum Inner {
        S(Spanned<String>),
        I(Spanned<i32>),
        T(Spanned<Vec<Thing>>),
    }

    impl Inner {
        fn span(&self) -> &Span {
            match self {
                Inner::S(spanned) => spanned.span(),
                Inner::I(spanned) => spanned.span(),
                Inner::T(spanned) => spanned.span(),
            }
        }
    }

    #[derive(Deserialize, Debug, Serialize)]
    struct Point {
        x: Option<Spanned<f64>>,
        y: Spanned<dbt_serde_yaml::Value>,
        a: Spanned<Option<f64>>,
        t: Spanned<Thing>,
        v: Spanned<Vec<Spanned<String>>>,
        w: Spanned<Vec<Inner>>,
    }

    let yaml = indoc! {"
        x: 1.0
        y: 2.0
        z: 3.0
        t: null
        v:
          - aaa
          - bbb
        w:
          - ccc
          - 1
    "};

    let value: dbt_serde_yaml::Value = dbt_serde_yaml::from_str(yaml).unwrap();
    let point: Spanned<Point> = dbt_serde_yaml::from_value(value).unwrap();

    let expected = r#"{1:1[0]..11:1[65]} Point {
    x: Some(
        {1:4[3]..2:1[7]} 1.0,
    ),
    y: {2:4[10]..3:1[14]} Number(2.0) @{2:4[10]..3:1[14]},
    a: {11:1[65]..11:1[65]} None,
    t: {4:4[24]..5:1[29]} Thing,
    v: {6:3[34]..8:1[48]} [
        {6:5[36]..7:5[44]} "aaa",
        {7:5[44]..8:1[48]} "bbb",
    ],
    w: {9:3[53]..11:1[65]} [
        S(
            {9:5[55]..10:5[63]} "ccc",
        ),
        I(
            {10:5[63]..11:1[65]} 1,
        ),
    ],
}"#;

    let point_repr = format!("{:#?}", point);
    eprintln!("{point_repr}");
    assert_eq!(point_repr, expected);

    // Test roundtrip
    let value = dbt_serde_yaml::to_value(&point).unwrap();
    let point: Spanned<Point> = dbt_serde_yaml::from_value(value).unwrap();
    let point_repr = format!("{:#?}", point);
    eprintln!("{point_repr}");
    assert_eq!(point_repr, expected);
}

#[test]
fn test_value_to_value_span() {
    let yaml = indoc! {"
        key:
          - name: hello
            nested:
              key1:
                - item1
              key2: value2
        "};

    #[derive(Deserialize, Debug, Serialize)]
    struct Root {
        key: Vec<Spanned<dbt_serde_yaml::Value>>,
    }

    let value: dbt_serde_yaml::Value = dbt_serde_yaml::from_str(yaml).unwrap();
    let expected = r#"[
    {2:5[9]..7:1[80]} Mapping {
        String("name") @{2:5[9]..2:11[15]}: String("hello") @{2:11[15]..3:5[25]},
        String("nested") @{3:5[25]..4:7[39]}: Mapping {
            String("key1") @{4:7[39]..5:9[53]}: Sequence [
                String("item1") @{5:11[55]..6:7[67]},
            ] @{5:9[53]..6:7[67]},
            String("key2") @{6:7[67]..6:13[73]}: String("value2") @{6:13[73]..7:1[80]},
        } @{4:7[39]..7:1[80]},
    } @{2:5[9]..7:1[80]},
]"#;

    let root: Spanned<Root> = value.to_typed(|_, _, _| {}, |_| Ok(None)).unwrap();
    let root_repr = format!("{:#?}", root.key);
    eprintln!("{root_repr}");
    assert_eq!(root_repr, expected);

    let root: Spanned<Root> = value.into_typed(|_, _, _| {}, |_| Ok(None)).unwrap();
    let root_repr = format!("{:#?}", root.key);
    eprintln!("{root_repr}");
    assert_eq!(root_repr, expected);

    // Test roundtrip
    let value = dbt_serde_yaml::to_value(&root).unwrap();
    let root: Spanned<Root> = value.to_typed(|_, _, _| {}, |_| Ok(None)).unwrap();
    let root_repr = format!("{:#?}", root.key);
    eprintln!("{root_repr}");
    assert_eq!(root_repr, expected);
}

#[allow(dead_code)]
fn my_custom_deserialize<'de, D>(deserializer: D) -> Result<Spanned<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value: f64 = f64::deserialize(deserializer)?;
    Ok(Spanned::new(value))
}

#[test]
fn test_custom_deserialize_with() {
    #[derive(Deserialize, Serialize, PartialEq, Debug)]
    struct Thing {
        #[serde(deserialize_with = "my_custom_deserialize")]
        x: Spanned<f64>,
        #[serde(deserialize_with = "my_custom_deserialize")]
        y: Spanned<f64>,
    }

    let yaml = indoc! {"
        x: 1.0
        y: 2.0
    "};

    let value: dbt_serde_yaml::Value = dbt_serde_yaml::from_str(yaml).unwrap();
    let thing: Spanned<Thing> = dbt_serde_yaml::from_value(value).unwrap();

    assert!(thing.has_valid_span());
    assert_eq!(thing.span().start.line, 1);
    assert_eq!(thing.span().start.column, 1);
    assert_eq!(thing.span().end.line, 3);
    assert_eq!(thing.span().end.column, 1);
}

#[cfg(feature = "filename")]
#[test]
fn test_with_filename() {
    use std::path::PathBuf;

    use serde::de::IntoDeserializer as _;

    let yaml = indoc! {"
        x: 1.0
        y: 2.0
    "};

    let value = {
        let _f = dbt_serde_yaml::with_filename(Some(std::path::PathBuf::from("filename.yml")));
        let value: dbt_serde_yaml::Value = dbt_serde_yaml::from_str(yaml).unwrap();
        assert_eq!(
            value.span().filename.as_deref(),
            Some(PathBuf::from("filename.yml")).as_ref()
        );

        {
            let _f = dbt_serde_yaml::with_filename(None);
            let value2: dbt_serde_yaml::Value = dbt_serde_yaml::from_str(yaml).unwrap();
            assert!(value2.span().filename.is_none());
        }

        dbt_serde_yaml::Value::deserialize(value.into_deserializer()).unwrap()
    };

    assert_eq!(
        value.span().filename.as_deref(),
        Some(PathBuf::from("filename.yml")).as_ref()
    );
}

#[cfg(feature = "schemars")]
#[test]
fn test_schemars() {
    use dbt_serde_yaml::JsonSchema;
    use dbt_serde_yaml::Verbatim;
    use schemars::schema_for;

    #[derive(Deserialize, Serialize, PartialEq, Debug, JsonSchema)]
    struct Point {
        x: Spanned<f64>,
        y: Verbatim<Spanned<String>>,
        z: Spanned<Option<f64>>,
    }

    let schema = schema_for!(Point);
    let yaml = dbt_serde_yaml::to_string(&schema).unwrap();
    println!("{yaml}");
    assert_eq!(
        yaml,
        indoc! {"
$schema: http://json-schema.org/draft-07/schema#
title: Point
type: object
required:
- x
- y
properties:
  x:
    type: number
    format: double
  y:
    type: string
  z:
    type:
    - number
    - 'null'
    format: double
"}
    );
}
