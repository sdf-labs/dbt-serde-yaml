#![allow(dead_code)]
use std::collections::BTreeMap;

use dbt_serde_yaml::{Spanned, Value};
use serde_derive::Deserialize;

pub fn main() {
    let yaml = indoc::indoc! {r#"
        models:
            - name: hello
              description: "Staging models"
              config:
                analysis-paths:: # should warn about unknown key
                    - "analysis"
                materialiaazed: '	'
                xxx: yyy  # should warn about unused key
    "#};

    let value: dbt_serde_yaml::Value = dbt_serde_yaml::from_str(&yaml).unwrap();
    // for model in value.get("models").unwrap().as_sequence().unwrap() {
    //     println!(
    //         "Name: {:?}, Description: {:?}, config: {:#?}",
    //         model.get("name").unwrap(),
    //         model.get("description").unwrap(),
    //         model.get("config").unwrap(),
    //     );
    // }

    #[derive(Deserialize, Debug)]
    struct Model {
        name: Spanned<String>,
        description: Option<Spanned<String>>,
        other: Spanned<Option<String>>,
        config: BTreeMap<String, Value>,
    }

    #[derive(Deserialize, Debug)]
    struct Models {
        models: Vec<Model>,
    }
    // #[derive(Deserialize, Debug)]
    // struct Models {
    //     __models__: BTreeMap<String, Value>,
    // }

    let models: Models = serde::Deserialize::deserialize(&value).unwrap();
    println!("{:#?}", models);

    let models: Models = value.to_typed(|_, _, _| {}, |_| Ok(None)).unwrap();
    println!("{:#?}", models);
}
