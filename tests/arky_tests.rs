//! `enable = ["arky"]`: a model converted to arky's frontmatter `Value` and read back.

#![cfg(feature = "arky")]

#[cfg(test)]
mod tests {
    use arky_core::parser::ast::{Mapping, Value};
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema(enable = ["arky"])]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "kind")]
    pub enum Source {
        Book { isbn: String },
        Web { url: String },
    }

    #[model_schema(enable = ["arky"])]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub enum Status {
        Draft,
        Published,
    }

    #[model_schema(enable = ["arky"])]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Note {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub due_on: Option<String>,
        pub source: Source,
        pub status: Status,
        pub tags: Vec<String>,
        #[model_schema_prop(minLength = 1)]
        pub title: String,
        pub views: u32,
    }

    #[model_schema(enable = ["arky"], default_types(Id = String))]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Tagged<Id> {
        pub id: Id,
        pub label: String,
    }

    #[model_schema(enable = ["arky"])]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct Sku(pub String);

    fn note() -> Note {
        Note {
            due_on: None,
            source: Source::Web {
                url: "https://example.com/a".to_owned(),
            },
            status: Status::Draft,
            tags: vec!["parser".to_owned()],
            title: "Notes".to_owned(),
            views: 7,
        }
    }

    fn text(written: &str) -> Value {
        Value::String(written.to_owned())
    }

    #[test]
    fn a_model_converts_to_the_value_serde_writes() {
        let value: Value = (&note()).try_into().unwrap();
        assert_eq!(
            value,
            Value::Mapping(Mapping::from([
                (
                    "source".to_owned(),
                    Value::Mapping(Mapping::from([
                        ("kind".to_owned(), text("Web")),
                        ("url".to_owned(), text("https://example.com/a")),
                    ])),
                ),
                ("status".to_owned(), text("Draft")),
                ("tags".to_owned(), Value::Sequence(vec![text("parser")])),
                ("title".to_owned(), text("Notes")),
                ("views".to_owned(), Value::Integer(7)),
            ]))
        );
    }

    #[test]
    fn a_value_reads_back_as_the_model() {
        let value = Value::try_from(&note()).unwrap();
        assert_eq!(Note::try_from(&value).unwrap(), note());
    }

    #[test]
    fn a_model_with_a_parameter_converts_at_any_filling() {
        let tagged = Tagged {
            id: 7_u32,
            label: "seven".to_owned(),
        };
        let value = Value::try_from(&tagged).unwrap();
        assert_eq!(Tagged::<u32>::try_from(&value).unwrap(), tagged);
    }

    #[test]
    fn a_brand_converts_to_the_value_it_wraps() {
        let sku = Sku("SKU-7".to_owned());
        let value = Value::try_from(&sku).unwrap();
        assert_eq!(value, text("SKU-7"));
        assert_eq!(Sku::try_from(&value).unwrap(), sku);
    }

    #[test]
    fn a_value_that_is_no_model_is_refused() {
        Note::try_from(&text("Notes")).unwrap_err();
    }

    /// The read is the model's own, which runs `validate()` where a schema surface publishes it.
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    #[test]
    fn the_read_refuses_what_validate_refuses() {
        let mut empty = note();
        empty.title = String::new();
        let value = Value::try_from(&empty).unwrap();
        assert_eq!(
            Note::try_from(&value).unwrap_err().to_string(),
            "'title': too short: minimum length is 1, got 0"
        );
    }
}
