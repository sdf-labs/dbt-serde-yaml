#![allow(dead_code)]

use dbt_serde_yaml::from_str;
use dbt_serde_yaml_derive::UntaggedEnumDeserialize;
use serde::Deserialize;

pub fn main() {
    #[derive(Debug, serde_derive::Deserialize)]
    struct AThing {
        key1: String,
        key2: i32,
    }

    #[derive(UntaggedEnumDeserialize, Debug)]
    #[serde(untagged)]
    enum Thing<T> {
        B,
        D,
        A(T),
        //C { key1: i32, key2: i32 },
    }

    use dbt_serde_yaml::Value;

    let yaml_data = r#"
key1: '1'
key2: 42
    "#;

    // Deserialize the YAML string into a Value type
    let value: Value = from_str(yaml_data).expect("Failed to deserialize YAML");
    let thing: Thing<AThing> =
        Deserialize::deserialize(value).expect("Failed to deserialize into Thing");

    // Print the resulting Value
    println!("{:?}", thing);
}
