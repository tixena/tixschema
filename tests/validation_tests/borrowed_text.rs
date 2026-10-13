//! A length or a pattern on a borrowed `str` is held by `validate()` as one on a `String` is.

/// One declaration with its string owned and one with it borrowed, under the same bound.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
mod a_bound_on_a_borrowed_str {
    use serde::{Deserialize, Deserializer, Serialize};
    use tixschema::model_schema;

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Account<'text> {
        #[model_schema_prop(minLength = 3)]
        #[serde(borrow, default, skip_serializing_if = "Option::is_none")]
        pub alias: Option<&'text str>,
        #[model_schema_prop(minLength = 3, maxLength = 8, pattern = "^[a-z]+$")]
        pub name: &'text str,
        #[model_schema_prop(maxLength = 4)]
        pub region: &'static str,
        #[model_schema_prop(minLength = 2)]
        #[serde(borrow)]
        pub tags: Vec<&'text str>,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Greeting<'text> {
        #[model_schema_prop(minLength = 3)]
        pub name: &'text str,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(untagged)]
    pub enum Handle<'text> {
        Long {
            #[model_schema_prop(minLength = 3)]
            text: &'text str,
        },
        Short {
            #[model_schema_prop(maxLength = 2)]
            text: &'text str,
        },
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Named<'text> {
        #[model_schema_prop(minLength = 3, maxLength = 8, pattern = "^[a-z]+$")]
        pub name: &'text str,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct NamedOwned {
        #[model_schema_prop(minLength = 3, maxLength = 8, pattern = "^[a-z]+$")]
        pub name: String,
    }

    /// [`Handle`] with no bound written, read by serde alone.
    #[derive(Debug, Deserialize, Eq, PartialEq)]
    #[serde(untagged)]
    pub enum HandleUnbounded<'text> {
        Long { text: &'text str },
        Short { text: &'text str },
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(tag = "kind")]
    pub enum Step<'text> {
        Rename {
            #[model_schema_prop(pattern = "^[a-z]+$")]
            to: &'text str,
        },
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(untagged)]
    pub enum Trimmed<'text> {
        Count {
            count: u32,
        },
        Text {
            #[model_schema_prop(minLength = 3)]
            #[serde(deserialize_with = "trimmed")]
            text: &'text str,
        },
    }

    fn trimmed<'de, 'text, D>(deserializer: D) -> Result<&'text str, D::Error>
    where
        D: Deserializer<'de>,
        'de: 'text,
    {
        <&'text str>::deserialize(deserializer).map(str::trim)
    }

    /// The README prints this report beside the declaration it comes from.
    #[test]
    fn the_readme_shows_what_validate_reports_for_a_borrowed_str() {
        assert_eq!(
            Greeting { name: "al" }.validate().unwrap_err(),
            vec!["'name': too short: minimum length is 3, got 2"]
        );
        let readme = include_str!("../../README.md");
        let shown = "// Err([\"'name': too short: minimum length is 3, got 2\"])";
        assert!(readme.contains(shown), "the README no longer shows {shown}");
    }

    #[test]
    fn validate_reports_each_borrowed_value_that_breaks_its_bound() {
        let broken = Account {
            alias: Some("al"),
            name: "Al",
            region: "north",
            tags: vec!["only"],
        };
        assert_eq!(
            broken.validate().unwrap_err(),
            vec![
                "'alias': too short: minimum length is 3, got 2",
                "'name': too short: minimum length is 3, got 2",
                "'name': does not match pattern '^[a-z]+$'",
                "'region': too long: maximum length is 4, got 5",
                "'tags': too short: minimum length is 2, got 1",
            ]
        );

        let kept = Account {
            alias: None,
            name: "alice",
            region: "east",
            tags: vec!["o", "k"],
        };
        kept.validate().unwrap();
    }

    #[test]
    fn a_borrowed_value_is_held_to_its_bound_as_it_is_read() {
        assert_eq!(
            serde_json::from_str::<Account<'_>>(
                r#"{"alias":"al","name":"alice","region":"east","tags":["only"]}"#,
            )
            .unwrap_err()
            .to_string(),
            "'alias': too short: minimum length is 3, got 2; \
             'tags': too short: minimum length is 2, got 1"
        );
        let read: Account<'_> = serde_json::from_str(
            r#"{"alias":"alf","name":"alice","region":"east","tags":["a","b"]}"#,
        )
        .unwrap();
        assert_eq!(read.alias, Some("alf"));
        read.validate().unwrap();

        let without_alias: Account<'_> =
            serde_json::from_str(r#"{"name":"alice","region":"east","tags":["a","b"]}"#).unwrap();
        assert_eq!(without_alias.alias, None);
        without_alias.validate().unwrap();
    }

    #[test]
    fn validate_answers_a_borrowed_str_in_the_words_it_answers_a_string() {
        let borrowed = Named { name: "A!" };
        let owned = NamedOwned {
            name: "A!".to_owned(),
        };
        assert_eq!(
            borrowed.validate().unwrap_err(),
            owned.validate().unwrap_err()
        );
    }

    #[cfg(feature = "zod")]
    #[test]
    fn the_zod_schema_of_a_borrowed_str_is_the_one_a_string_writes() {
        let zod = Named::zod_schema();
        assert!(
            zod.contains(
                "name: z.string().min(3, { error: (issue) => `too short: minimum length is 3, \
                 got ${String(issue.input).length}` }).max(8, { error: (issue) => `too long: \
                 maximum length is 8, got ${String(issue.input).length}` }).check(z.regex(\
                 /^[a-z]+$/, { error: \"does not match pattern '^[a-z]+$'\" }))"
            ),
            "{zod}"
        );
        assert_eq!(NamedOwned::zod_schema().replace("NamedOwned", "Named"), zod);
    }

    #[cfg(feature = "jsonschema")]
    #[test]
    fn the_json_schema_of_a_borrowed_str_is_the_one_a_string_writes() {
        let schema = Named::json_schema();
        assert_eq!(
            schema["properties"]["name"],
            serde_json::json!({
                "type": "string",
                "minLength": 3_i32,
                "maxLength": 8_i32,
                "pattern": "^[a-z]+$",
            })
        );
        assert_eq!(
            schema["properties"],
            NamedOwned::json_schema()["properties"]
        );
    }

    #[cfg(feature = "typescript")]
    #[test]
    fn the_typescript_comment_states_the_bound_validate_holds() {
        let typescript = Named::ts_definition();
        assert!(typescript.contains("Minimum length: 3"), "{typescript}");
        assert!(typescript.contains("Maximum length: 8"), "{typescript}");
        Named { name: "al" }.validate().unwrap_err();
    }

    #[test]
    fn a_tagged_variants_borrowed_member_is_held_as_it_is_read() {
        let built = Step::Rename { to: "ALICE" };
        assert_eq!(
            built.validate().unwrap_err(),
            vec!["'to': does not match pattern '^[a-z]+$'"]
        );
        assert_eq!(
            serde_json::from_str::<Step<'_>>(r#"{"kind":"Rename","to":"ALICE"}"#)
                .unwrap_err()
                .to_string(),
            built.validate().unwrap_err().join("; ")
        );
    }

    #[test]
    fn an_untagged_borrowed_member_chooses_no_variant_and_is_held_once_the_value_is_read() {
        const BREAKS_LONG: &str = r#"{"text":"al"}"#;

        // The variant is chosen by shape alone, a borrowed member being read through no hook, and
        // the value read is then refused with what `validate()` answers of it.
        let built = Handle::Long { text: "al" };
        assert_eq!(
            built.validate().unwrap_err(),
            vec!["'text': too short: minimum length is 3, got 2"]
        );
        assert_eq!(
            serde_json::from_str::<Handle<'_>>(BREAKS_LONG)
                .unwrap_err()
                .to_string(),
            built.validate().unwrap_err().join("; ")
        );
        assert_eq!(
            serde_json::from_str::<HandleUnbounded<'_>>(BREAKS_LONG).unwrap(),
            HandleUnbounded::Long { text: "al" }
        );

        let kept: Handle<'_> = serde_json::from_str(r#"{"text":"alice"}"#).unwrap();
        assert_eq!(kept, Handle::Long { text: "alice" });
        kept.validate().unwrap();
    }

    /// Nothing is hung beside a reader of the author's own, which serde admits one of per field.
    #[test]
    fn an_untagged_borrowed_member_is_read_by_the_reader_its_author_wrote() {
        // `got 1`: the author's reader trimmed the text before the read held it to its bound.
        assert_eq!(
            serde_json::from_str::<Trimmed<'_>>(r#"{"text":" a "}"#)
                .unwrap_err()
                .to_string(),
            Trimmed::Text { text: "a" }
                .validate()
                .unwrap_err()
                .join("; ")
        );
        let kept: Trimmed<'_> = serde_json::from_str(r#"{"text":" alice "}"#).unwrap();
        assert_eq!(kept, Trimmed::Text { text: "alice" });
    }
}
