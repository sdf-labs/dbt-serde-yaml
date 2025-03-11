dbt Serde YAML
==========

Rust library for using the [Serde] serialization framework with data in [YAML]
file format.

This is dbt Lab's fork of the original serde-yaml crate (<https://github.com/dtolnay/serde-yaml>).

[Serde]: https://github.com/serde-rs/serde
[YAML]: https://yaml.org/

## Dependency

```toml
[dependencies]
serde = "1.0"
dbt-serde_yaml = "0.0.1"
```

Release notes are available under [GitHub releases].

[GitHub releases]: https://github.com/sdf-labs/dbt-serde-yaml/releases

## Using dbt Serde YAML

[API documentation is available in rustdoc form][docs.rs] but the general idea
is:

[docs.rs]: https://docs.rs/dbt-serde_yaml

```rust
use std::collections::BTreeMap;

fn main() -> Result<(), dbt_serde_yaml::Error> {
    // You have some type.
    let mut map = BTreeMap::new();
    map.insert("x".to_string(), 1.0);
    map.insert("y".to_string(), 2.0);

    // Serialize it to a YAML string.
    let yaml = dbt_serde_yaml::to_string(&map)?;
    assert_eq!(yaml, "x: 1.0\ny: 2.0\n");

    // Deserialize it back to a Rust type.
    let deserialized_map: BTreeMap<String, f64> = dbt_serde_yaml::from_str(&yaml)?;
    assert_eq!(map, deserialized_map);
    Ok(())
}
```

It can also be used with Serde's derive macros to handle structs and enums
defined in your program.

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
dbt-serde_yaml = "0.0.1"
```

Structs serialize in the obvious way:

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Point {
    x: f64,
    y: f64,
}

fn main() -> Result<(), dbt_serde_yaml::Error> {
    let point = Point { x: 1.0, y: 2.0 };

    let yaml = dbt_serde_yaml::to_string(&point)?;
    assert_eq!(yaml, "x: 1.0\ny: 2.0\n");

    let deserialized_point: Point = dbt_serde_yaml::from_str(&yaml)?;
    assert_eq!(point, deserialized_point);
    Ok(())
}
```

Enums serialize using YAML's `!tag` syntax to identify the variant name.

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
enum Enum {
    Unit,
    Newtype(usize),
    Tuple(usize, usize, usize),
    Struct { x: f64, y: f64 },
}

fn main() -> Result<(), dbt_serde_yaml::Error> {
    let yaml = "
        - !Newtype 1
        - !Tuple [0, 0, 0]
        - !Struct {x: 1.0, y: 2.0}
    ";
    let values: Vec<Enum> = dbt_serde_yaml::from_str(yaml).unwrap();
    assert_eq!(values[0], Enum::Newtype(1));
    assert_eq!(values[1], Enum::Tuple(0, 0, 0));
    assert_eq!(values[2], Enum::Struct { x: 1.0, y: 2.0 });

    // The last two in YAML's block style instead:
    let yaml = "
        - !Tuple
          - 0
          - 0
          - 0
        - !Struct
          x: 1.0
          y: 2.0
    ";
    let values: Vec<Enum> = dbt_serde_yaml::from_str(yaml).unwrap();
    assert_eq!(values[0], Enum::Tuple(0, 0, 0));
    assert_eq!(values[1], Enum::Struct { x: 1.0, y: 2.0 });

    // Variants with no data can be written using !Tag or just the string name.
    let yaml = "
        - Unit  # serialization produces this one
        - !Unit
    ";
    let values: Vec<Enum> = dbt_serde_yaml::from_str(yaml).unwrap();
    assert_eq!(values[0], Enum::Unit);
    assert_eq!(values[1], Enum::Unit);

    Ok(())
}
```

Code locations can be captured using the `Spanned` wrapper type.

```rust
use serde::{Serialize, Deserialize};
use dbt_serde_yaml::Spanned;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Point {
    x: Spanned<f64>,
    y: Spanned<f64>,
}

fn main() -> Result<(), dbt_serde_yaml::Error> {
    let yaml = "
        x: 1.0
        y: 2.0
    ";
    let point: Point = dbt_serde_yaml::from_str(yaml)?;
    assert_eq!(*point.x, 2.0);
    assert_eq!(yaml[point.x.span().start.index..point.x.span().end.index].trim(), "1.0");
    assert_eq!(yaml[point.y.span().start.index..point.y.span().end.index].trim(), "2.0");
    Ok(())
}
```

<br>

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
