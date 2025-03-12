use std::collections::HashSet;

use dbt_serde_yaml::{Spanned, Value};
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

    let yaml = "x: 1.0\ny: 2.0\n---\nx: 3.0\ny: 4.0\n";
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
        // TODO: exclude the document separator
        "---\nx: 3.0\ny: 4.0"
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
            let v = map.get(&Value::string("x".to_string())).unwrap();
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
