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
            "'name_config': too short: minimum length is 1, got 0"
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

/// A length written where a list is named, not written: a brand over a list at a field, a brand
/// over a brand over a list, and a generic brand whose declared default is a list.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
mod a_length_through_a_name {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema(no_display)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct Keywords(pub Vec<String>);

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Article {
        #[model_schema_prop(minLength = 1)]
        pub below: LaterKeywords,
        #[model_schema_prop(minLength = 1, maxLength = 2)]
        pub keywords: Keywords,
    }

    #[model_schema(no_display)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct LaterKeywords(pub Vec<String>);

    #[model_schema(no_display, minLength = 1)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct SomeKeywords(pub Keywords);

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Collected {
        #[model_schema_prop(maxLength = 2)]
        pub some: SomeKeywords,
    }

    #[model_schema(no_display, minLength = 1, default_types(T = Vec<String>))]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct Held<T>(pub T);

    fn words(count: usize) -> Vec<String> {
        vec!["w".to_owned(); count]
    }

    fn article(keywords: usize, below: usize) -> Article {
        Article {
            keywords: Keywords(words(keywords)),
            below: LaterKeywords(words(below)),
        }
    }

    #[test]
    fn a_field_typed_with_a_brand_over_a_list_counts_its_items() {
        assert_eq!(article(1, 1).validate(), Ok(()));
        assert_eq!(
            article(0, 0).validate().unwrap_err(),
            vec![
                "'below': too short: minimum length is 1, got 0",
                "'keywords': too short: minimum length is 1, got 0",
            ]
        );
        assert_eq!(
            article(3, 1).validate().unwrap_err(),
            vec!["'keywords': too long: maximum length is 2, got 3"]
        );
    }

    #[test]
    fn the_read_refuses_what_validate_refuses_for_a_brand_over_a_list() {
        assert_eq!(
            serde_json::from_str::<Article>(r#"{"keywords":[],"below":["a"]}"#)
                .unwrap_err()
                .to_string(),
            "'keywords': too short: minimum length is 1, got 0"
        );
        assert_eq!(
            serde_json::from_str::<Article>(r#"{"keywords":["w"],"below":["w"]}"#).unwrap(),
            article(1, 1)
        );
    }

    #[test]
    fn a_brand_over_a_brand_over_a_list_takes_the_two_lengths_on_the_type() {
        assert_eq!(
            SomeKeywords(Keywords(Vec::new())).validate().unwrap_err(),
            vec!["too short: minimum length is 1, got 0"]
        );
        assert_eq!(SomeKeywords(Keywords(words(2))).validate(), Ok(()));
        assert_eq!(
            serde_json::from_str::<SomeKeywords>("[]")
                .unwrap_err()
                .to_string(),
            "too short: minimum length is 1, got 0"
        );
        let collected = |count| Collected {
            some: SomeKeywords(Keywords(words(count))),
        };
        assert_eq!(
            collected(3).validate().unwrap_err(),
            vec!["'some': too long: maximum length is 2, got 3"]
        );
        assert_eq!(collected(1).validate(), Ok(()));
    }

    #[test]
    fn a_generic_brand_whose_default_is_a_list_counts_the_default_filling() {
        assert_eq!(
            Held(Vec::<String>::new()).validate().unwrap_err(),
            vec!["too short: minimum length is 1, got 0"]
        );
        assert_eq!(Held(words(1)).validate(), Ok(()));
        assert_eq!(
            serde_json::from_str::<Held<Vec<String>>>("[]")
                .unwrap_err()
                .to_string(),
            "too short: minimum length is 1, got 0"
        );
    }

    #[cfg(feature = "zod")]
    #[test]
    fn the_zod_schema_counts_the_items_of_the_named_array() {
        let said = "Array.isArray(issue.input) ? issue.input.length : String(issue.input).length";
        let article = Article::zod_schema();
        assert!(
            article.contains(&format!(
                "keywords: Keywords$Schema.check(z.minLength(1, {{ error: (issue) => `too \
                 short: minimum length is 1, got ${{{said}}}` }})"
            )),
            "{article}"
        );
        let some = SomeKeywords::zod_schema();
        assert!(
            some.contains("Keywords$Schema.check(z.minLength(1, { error: (issue) =>"),
            "{some}"
        );
        let held = Held::<String>::zod_schema();
        assert!(
            held.contains(
                "z.array(z.string()).min(1, { error: (issue) => `too short: minimum length is 1, \
                 got ${Array.isArray(issue.input) ? issue.input.length : 0}` })"
            ),
            "{held}"
        );
    }

    #[cfg(feature = "jsonschema")]
    #[test]
    fn the_json_schema_writes_the_count_beside_the_name() {
        use serde_json::json;

        let listed = json!({ "type": "array", "items": { "type": "string" } });
        assert_eq!(
            Article::json_schema()["properties"]["keywords"],
            json!({ "allOf": [listed, {
                "minLength": 1_u32, "minItems": 1_u32, "maxLength": 2_u32, "maxItems": 2_u32
            }] })
        );
        assert_eq!(
            SomeKeywords::json_schema(),
            json!({ "allOf": [listed, { "minLength": 1_u32, "minItems": 1_u32 }] })
        );
        assert_eq!(
            Held::<String>::json_schema(),
            json!({ "allOf": [listed, { "minItems": 1_u32 }] })
        );
    }
}

/// A length written where an alias of an `IndexSet` is named, and on a brand over that alias.
#[cfg(all(
    feature = "serde",
    feature = "indexmap",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
mod a_length_through_an_alias_of_an_index_set {
    use indexmap::IndexSet;
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema()]
    pub type Queue = IndexSet<String>;

    #[model_schema(no_display, minLength = 1)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct SomeQueue(pub Queue);

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct Roster {
        #[model_schema_prop(minLength = 1, maxLength = 2)]
        pub queue: Queue,
        #[model_schema_prop(maxLength = 2)]
        pub some: SomeQueue,
    }

    fn queue(count: usize) -> Queue {
        (0..count).map(|number| number.to_string()).collect()
    }

    fn roster(queued: usize, some: usize) -> Roster {
        Roster {
            queue: queue(queued),
            some: SomeQueue(queue(some)),
        }
    }

    #[test]
    fn validate_counts_the_items() {
        assert_eq!(roster(1, 1).validate(), Ok(()));
        assert_eq!(
            roster(0, 0).validate().unwrap_err(),
            vec![
                "'queue': too short: minimum length is 1, got 0",
                "'some': too short: minimum length is 1, got 0",
            ]
        );
        assert_eq!(
            roster(3, 3).validate().unwrap_err(),
            vec![
                "'queue': too long: maximum length is 2, got 3",
                "'some': too long: maximum length is 2, got 3",
            ]
        );
        assert_eq!(
            SomeQueue(queue(0)).validate().unwrap_err(),
            vec!["too short: minimum length is 1, got 0"]
        );
    }

    #[test]
    fn the_read_refuses_what_validate_refuses() {
        assert_eq!(
            serde_json::from_str::<Roster>(r#"{"queue":[],"some":["a"]}"#)
                .unwrap_err()
                .to_string(),
            "'queue': too short: minimum length is 1, got 0"
        );
        assert_eq!(
            serde_json::from_str::<Roster>(r#"{"queue":["a","a","b","b","c"],"some":["a"]}"#)
                .unwrap_err()
                .to_string(),
            "'queue': too long: maximum length is 2, got 3",
            "a set keeps each item once before it is counted"
        );
        assert_eq!(
            serde_json::from_str::<SomeQueue>("[]")
                .unwrap_err()
                .to_string(),
            "too short: minimum length is 1, got 0"
        );
        assert_eq!(
            serde_json::from_str::<Roster>(r#"{"queue":["a"],"some":["a"]}"#).unwrap(),
            Roster {
                queue: IndexSet::from(["a".to_owned()]),
                some: SomeQueue(IndexSet::from(["a".to_owned()])),
            }
        );
    }

    #[cfg(feature = "zod")]
    #[test]
    fn the_zod_schema_counts_the_items() {
        let said = "Array.isArray(issue.input) ? issue.input.length : String(issue.input).length";
        let roster = Roster::zod_schema();
        assert!(
            roster.contains(&format!(
                "queue: QueueType$Schema.check(z.minLength(1, {{ error: (issue) => `too \
                 short: minimum length is 1, got ${{{said}}}` }})"
            )),
            "{roster}"
        );
        let some = SomeQueue::zod_schema();
        assert!(
            some.contains("QueueType$Schema.check(z.minLength(1, { error: (issue) =>"),
            "{some}"
        );
    }

    #[cfg(feature = "jsonschema")]
    #[test]
    fn the_json_schema_writes_the_count_beside_the_name() {
        let narrowing = &Roster::json_schema()["properties"]["queue"]["allOf"][1];
        assert_eq!(
            narrowing,
            &serde_json::json!({
                "minLength": 1_u32, "minItems": 1_u32, "maxLength": 2_u32, "maxItems": 2_u32
            })
        );
    }
}
