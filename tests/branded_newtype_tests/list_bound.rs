//! A brand over a list takes `minLength` and `maxLength` on the type, where they count its items.

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
mod a_length_on_a_brand_over_a_list {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct ListPart {
        pub value: String,
    }

    #[model_schema(no_display, minLength = 1)]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct NameConfig(pub Vec<ListPart>);

    /// The named form, over a slice behind a `Box`.
    #[model_schema(no_display, minLength = 1, maxLength = 2)]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct Shelf {
        pub rows: Box<[String]>,
    }

    /// A brand that declares a type parameter and holds a list of it.
    #[model_schema(no_display, maxLength = 2, default_types(T = String))]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct Batch<T>(pub Vec<T>);

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Template {
        pub name_config: NameConfig,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub shelf: Option<Shelf>,
    }

    fn parts(count: usize) -> Vec<ListPart> {
        vec![
            ListPart {
                value: "part".to_owned()
            };
            count
        ]
    }

    #[test]
    fn validate_counts_the_list_the_brand_holds() {
        assert_eq!(
            NameConfig(Vec::new()).validate().unwrap_err(),
            vec!["too short: minimum length is 1, got 0"]
        );
        assert_eq!(NameConfig(parts(4)).validate(), Ok(()));

        let shelf = |rows: &[&str]| Shelf {
            rows: rows.iter().map(|row| (*row).to_owned()).collect(),
        };
        assert_eq!(
            shelf(&[]).validate().unwrap_err(),
            vec!["too short: minimum length is 1, got 0"]
        );
        assert_eq!(
            shelf(&["a", "b", "c"]).validate().unwrap_err(),
            vec!["too long: maximum length is 2, got 3"]
        );
        assert_eq!(
            shelf(&["a row of any length", ""]).validate(),
            Ok(()),
            "no row is measured"
        );

        assert_eq!(
            Batch(vec!["a".to_owned(), "b".to_owned(), "c".to_owned()])
                .validate()
                .unwrap_err(),
            vec!["too long: maximum length is 2, got 3"]
        );
    }

    /// A brand is checked as it is read, and a list it holds is counted there.
    #[test]
    fn the_read_refuses_a_list_that_breaks_the_brands_bound() {
        assert_eq!(
            serde_json::from_str::<NameConfig>("[]")
                .unwrap_err()
                .to_string(),
            "too short: minimum length is 1, got 0"
        );
        assert_eq!(
            serde_json::from_str::<NameConfig>(r#"[{"value":"a"}]"#).unwrap(),
            NameConfig(vec![ListPart {
                value: "a".to_owned()
            }])
        );
        assert_eq!(
            serde_json::from_str::<Shelf>(r#"["a","b","c"]"#)
                .unwrap_err()
                .to_string(),
            "too long: maximum length is 2, got 3"
        );
        assert_eq!(
            serde_json::from_str::<Batch<u32>>("[1,2,3]")
                .unwrap_err()
                .to_string(),
            "too long: maximum length is 2, got 3",
            "a count holds whatever fills the parameter"
        );
        assert_eq!(
            serde_json::from_str::<Batch<u32>>("[1,2]").unwrap(),
            Batch(vec![1, 2])
        );
    }

    /// The type that holds the brand names the field on the read and in `validate()`.
    #[test]
    fn a_field_typed_with_the_brand_is_held_to_the_brands_count() {
        assert_eq!(
            serde_json::from_str::<Template>(r#"{"name_config":[]}"#)
                .unwrap_err()
                .to_string(),
            "'name_config': too short: minimum length is 1, got 0 at line 1 column 18"
        );
        let built = Template {
            name_config: NameConfig(Vec::new()),
            shelf: Some(Shelf { rows: Box::new([]) }),
        };
        assert_eq!(
            built.validate().unwrap_err(),
            vec![
                "'name_config': too short: minimum length is 1, got 0",
                "'shelf': too short: minimum length is 1, got 0",
            ]
        );
    }

    #[cfg(feature = "zod")]
    #[test]
    fn the_zod_schema_counts_the_brands_array() {
        let counted = "{ error: (issue) => `too short: minimum length is 1, got \
                       ${Array.isArray(issue.input) ? issue.input.length : 0}` }";
        let capped = "{ error: (issue) => `too long: maximum length is 2, got \
                      ${Array.isArray(issue.input) ? issue.input.length : 0}` }";
        for (schema, written) in [
            (
                NameConfig::zod_schema(),
                format!("z.array(ListPart$Schema).min(1, {counted}).brand"),
            ),
            (
                Shelf::zod_schema(),
                format!("z.array(z.string()).min(1, {counted}).max(2, {capped}).brand"),
            ),
            (
                Batch::<String>::zod_schema(),
                format!("  z.array(t).max(2, {capped}).meta("),
            ),
        ] {
            assert!(schema.contains(&written), "{written} is not in:\n{schema}");
        }
    }

    #[cfg(feature = "jsonschema")]
    #[test]
    fn the_json_schema_writes_the_brands_count_on_its_array() {
        use serde_json::json;

        assert_eq!(
            NameConfig::json_schema(),
            json!({ "type": "array", "items": ListPart::json_schema(), "minItems": 1_u32 })
        );
        assert_eq!(
            Shelf::json_schema(),
            json!({
                "type": "array", "items": { "type": "string" },
                "minItems": 1_u32, "maxItems": 2_u32
            })
        );
        assert_eq!(
            Batch::<String>::json_schema(),
            json!({ "type": "array", "items": { "type": "string" }, "maxItems": 2_u32 })
        );
    }

    #[cfg(feature = "typescript")]
    #[test]
    fn the_typescript_type_of_the_brand_stays_an_array() {
        assert!(
            NameConfig::ts_definition().contains("export type NameConfig = Array<ListPart> & "),
            "{}",
            NameConfig::ts_definition()
        );
    }
}
