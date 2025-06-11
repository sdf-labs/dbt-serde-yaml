use std::collections::HashSet;

use dbt_serde_yaml::Spanned;
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

    let v = Spanned::new(Point { x: 10, y: 20 });
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
    #[derive(Deserialize, Debug, PartialEq, Eq)]
    struct Thing;

    #[derive(Deserialize)]
    struct Point {
        x: Spanned<f64>,
        y: Spanned<dbt_serde_yaml::Value>,
        a: Spanned<Option<f64>>,
        t: Spanned<Thing>,
    }

    let yaml = indoc! {"
        x: 1.0
        y: 2.0
        z: 3.0
        t: null
    "};

    let value: dbt_serde_yaml::Value = dbt_serde_yaml::from_str(yaml).unwrap();
    let point: Spanned<Point> = dbt_serde_yaml::from_value(value).unwrap();

    assert!(point.has_valid_span());
    assert_eq!(point.span().start.line, 1);
    assert_eq!(point.span().start.column, 1);
    assert_eq!(point.span().end.line, 5);
    assert_eq!(point.span().end.column, 1);

    assert_eq!(*point.x, 1.0);
    assert!(point.a.is_none());
    assert_eq!(*point.t, Thing {});
    assert!(point.x.has_valid_span());
    assert!(point.y.has_valid_span());
    assert_eq!(point.x.span().start.index, 3);
    assert_eq!(*point.y, 2.0);
    assert_eq!(point.y.span().start.line, 2);
    assert_eq!(point.y.span().start.column, 4);
    assert_eq!(point.y.span().end.line, 3);
    assert_eq!(point.y.span().end.column, 1);
    assert_eq!(
        yaml[point.x.span().start.index..point.x.span().end.index].trim(),
        "1.0"
    );
    assert_eq!(
        yaml[point.y.span().start.index..point.y.span().end.index].trim(),
        "2.0"
    );
}

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
