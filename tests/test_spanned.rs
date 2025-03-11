use std::collections::HashSet;

use dbt_serde_yaml::Spanned;
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
    assert_eq!(point2.x.span().start.index, 3);
    assert_eq!(*point2.y, 2.0);
    assert_eq!(point2.y.span().start.line, 2);
    assert_eq!(point2.y.span().start.column, 4);
    assert_eq!(point2.y.span().end.line, 3);
    assert_eq!(point2.y.span().end.column, 1);
    assert_eq!(
        &yaml[point2.x.span().start.index..point2.x.span().end.index],
        "1.0\n"
    );
    assert_eq!(
        &yaml[point2.y.span().start.index..point2.y.span().end.index],
        "2.0\n"
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
        points.push(point);
    }
    assert_eq!(*points[0].x, 1.0);
    assert_eq!(*points[0].y, 2.0);
    assert_eq!(*points[1].x, 3.0);
    assert_eq!(*points[1].y, 4.0);

    assert_eq!(
        &yaml[points[0].span().start.index..points[0].span().end.index],
        "x: 1.0\ny: 2.0\n"
    );
    assert_eq!(
        &yaml[points[1].span().start.index..points[1].span().end.index],
        "---\nx: 3.0\ny: 4.0\n"
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
