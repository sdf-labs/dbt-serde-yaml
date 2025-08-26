use dbt_serde_yaml::from_str;
use dbt_serde_yaml::UntaggedEnumDeserialize;
use serde_derive::Deserialize;
use serde_derive::Serialize;

struct BThing<'f> {
    callback: Option<&'f mut dyn FnMut(&str) -> String>,
}

impl<'f> BThing<'f> {
    fn new(callback: Option<&'f mut dyn FnMut(&str) -> String>) -> Self {
        BThing { callback }
    }

    fn call(&mut self, input: &str) -> String {
        if let Some(callback) = self.callback.as_mut() {
            callback(input)
        } else {
            String::from("No callback set")
        }
    }
}

pub fn main() {
    #[derive(Debug, Serialize, Deserialize)]
    struct AThing {
        key1: String,
        key2: i32,
        key3: Inner,
    }

    #[derive(Debug, Serialize, UntaggedEnumDeserialize)]
    #[serde(untagged)]
    enum Inner {
        V(Vec<String>),
        I(i32),
        T(Vec<Thing>),
    }

    #[derive(Debug, Serialize, UntaggedEnumDeserialize)]
    #[serde(untagged)]
    enum Thing {
        A(AThing),
        B,
    }
    use dbt_serde_yaml::Value;
    let yaml_data = r#"
        key1: value1
        key2: 42
        key3:
          - item1
          - item2
    "#;
    let value: Value = from_str(yaml_data).expect("Failed to deserialize YAML");
    let thing: Thing = value.into_typed(|_, _, _| {}, |_| Ok(None)).unwrap();

    println!("{0:?}\n", thing);

    let mut callback = |input: &str| -> String { format!("Callback called with input: {}", input) };
    let mut b_thing = BThing::new(Some(&mut callback));

    let result = b_thing.call("test input");
    println!("BThing call result: {}", result);
}
