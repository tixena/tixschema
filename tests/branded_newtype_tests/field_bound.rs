//! A length or a pattern written on a field typed with a brand over a string holds that field's
//! value alone: the validator, the Zod schema and the JSON Schema read it, and the brand keeps none.

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
mod bounded_at_the_field {
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;
    use tixschema::model_schema;

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct BoundAbove(pub String);

    #[model_schema()]
    pub type BoundTitle = String;

    /// One field per placement: bare, under an `Option` over a brand declared below, over a path
    /// declared below, over an alias of a string, one that writes no bound, and a list of the
    /// brand, whose bound counts the list and reaches no item.
    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct BoundHolder {
        #[model_schema_prop(minLength = 2)]
        pub bare: BoundAbove,
        #[model_schema_prop(minLength = 2)]
        pub dir: BoundDir,
        pub free: BoundAbove,
        #[model_schema_prop(maxLength = 3)]
        pub many: Vec<BoundAbove>,
        #[model_schema_prop(minLength = 2, maxLength = 4, pattern = "^[a-z]+$")]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub maybe: Option<BoundBelow>,
        #[model_schema_prop(minLength = 2)]
        pub titled: BoundTitle,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct BoundBelow(pub String);

    #[model_schema(no_display)]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct BoundDir(pub PathBuf);

    fn held(
        bare: &str,
        maybe: Option<&str>,
        many: &[&str],
        dir: &str,
        titled: &str,
    ) -> BoundHolder {
        BoundHolder {
            bare: BoundAbove(bare.to_owned()),
            dir: BoundDir(PathBuf::from(dir)),
            free: BoundAbove(String::new()),
            many: many
                .iter()
                .map(|item| BoundAbove((*item).to_owned()))
                .collect(),
            maybe: maybe.map(|text| BoundBelow(text.to_owned())),
            titled: titled.to_owned(),
        }
    }

    #[test]
    fn validate_reports_each_bound_under_the_field_it_was_written_on() {
        assert_eq!(
            held("a", Some("TOOLONG"), &["abcd", "b", "c", "d"], "d", "t").validate(),
            Err(vec![
                "'bare': too short: minimum length is 2, got 1".to_owned(),
                "'dir': too short: minimum length is 2, got 1".to_owned(),
                "'many': too long: maximum length is 3, got 4".to_owned(),
                "'maybe': too long: maximum length is 4, got 7".to_owned(),
                "'maybe': does not match pattern '^[a-z]+$'".to_owned(),
                "'titled': too short: minimum length is 2, got 1".to_owned(),
            ])
        );
        assert_eq!(
            held("ab", Some("abcd"), &["abcd", "abcde", ""], "dir", "title").validate(),
            Ok(())
        );
        assert_eq!(held("ab", None, &[], "dir", "title").validate(), Ok(()));
    }

    /// The read refuses the value the bound refuses, with what `validate()` answers of it.
    #[test]
    fn serde_refuses_a_value_the_bound_refuses() {
        let read = serde_json::from_value::<BoundHolder>(serde_json::json!({
            "bare": "a", "dir": "d", "free": "", "many": ["abcd"], "maybe": "", "titled": "t"
        }));
        let refused = held("a", Some(""), &["abcd"], "d", "t")
            .validate()
            .unwrap_err()
            .join("; ");
        assert_eq!(read.map_err(|error| error.to_string()), Err(refused));
        let kept: BoundHolder = serde_json::from_value(serde_json::json!({
            "bare": "ab", "dir": "dir", "free": "", "many": ["abcd"], "maybe": "ab", "titled": "title"
        }))
        .unwrap();
        assert_eq!(kept, held("ab", Some("ab"), &["abcd"], "dir", "title"));
    }

    /// The brand itself keeps no bound: a field that writes none beside it holds any value.
    #[test]
    fn the_brand_keeps_no_bound_of_its_own() {
        assert_eq!(
            serde_json::from_value::<BoundAbove>(serde_json::json!("")).unwrap(),
            BoundAbove(String::new())
        );
        #[cfg(feature = "jsonschema")]
        assert_eq!(
            BoundAbove::json_schema(),
            serde_json::json!({ "type": "string" })
        );
    }

    #[cfg(feature = "zod")]
    #[test]
    fn the_zod_schema_checks_the_brand_at_the_field() {
        let zod = BoundHolder::zod_schema();
        let short = "{ error: (issue) => `too short: minimum length is 2, got ${String(issue.input).length}` }";
        for written in [
            format!("  bare: BoundAbove$Schema.check(z.minLength(2, {short})),\n"),
            format!("  get dir() {{ return BoundDir$Schema.check(z.minLength(2, {short})); }},\n"),
            "  free: BoundAbove$Schema,\n".to_owned(),
            "  many: z.array(BoundAbove$Schema).max(3, { error: (issue) => `too long: maximum \
             length is 3, got ${Array.isArray(issue.input) ? issue.input.length : 0}` }),\n"
                .to_owned(),
            format!(
                "BoundBelow$Schema.check(z.minLength(2, {short}), z.maxLength(4, {{ error: (issue) \
                 => `too long: maximum length is 4, got ${{String(issue.input).length}}` }}), \
                 z.regex(/^[a-z]+$/, {{ error: \"does not match pattern '^[a-z]+$'\" }}))"
            ),
            format!("  titled: BoundTitleType$Schema.check(z.minLength(2, {short})),\n"),
        ] {
            assert!(zod.contains(&written), "{written} is not in:\n{zod}");
        }
    }

    #[cfg(feature = "jsonschema")]
    #[test]
    fn the_json_schema_narrows_the_brand_at_the_field() {
        let schema = BoundHolder::json_schema();
        let narrowed =
            |by: serde_json::Value| serde_json::json!({ "allOf": [{ "type": "string" }, by] });
        assert_eq!(
            schema["properties"],
            serde_json::json!({
                "bare": narrowed(serde_json::json!({ "minLength": 2_u32 })),
                "dir": narrowed(serde_json::json!({ "minLength": 2_u32 })),
                "free": { "type": "string" },
                "many": { "type": "array", "items": { "type": "string" }, "maxItems": 3_u32 },
                "maybe": {
                    "anyOf": [
                        narrowed(serde_json::json!({
                            "minLength": 2_u32, "maxLength": 4_u32, "pattern": "^[a-z]+$"
                        })),
                        { "type": "null" },
                    ],
                },
                "titled": narrowed(serde_json::json!({ "minLength": 2_u32 })),
            })
        );
    }
}

/// A field typed with a brand that declares a bound is held to that bound and to the one the
/// field writes beside it.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
mod bounded_at_the_type_and_the_field {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    const WRONG_PATTERN: &str = "'slug': does not match pattern '^[a-z]+$'";

    #[model_schema(minLength = 3)]
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct Slug(pub String);

    #[model_schema()]
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Holder {
        #[model_schema_prop(pattern = "^[a-z]+$")]
        pub slug: Slug,
    }

    #[model_schema()]
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Plain {
        pub slug: Slug,
    }

    #[model_schema()]
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(untagged)]
    pub enum Untagged {
        Named {
            #[model_schema_prop(pattern = "^[a-z]+$")]
            slug: Slug,
        },
    }

    #[model_schema()]
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(tag = "kind")]
    pub enum Tagged {
        Named {
            #[model_schema_prop(pattern = "^[a-z]+$")]
            slug: Slug,
        },
    }

    #[model_schema()]
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Tuple(#[model_schema_prop(pattern = "^[a-z]+$")] pub Slug);

    fn slug(text: &str) -> Slug {
        Slug(text.to_owned())
    }

    fn short(got: usize) -> String {
        format!("'slug': too short: minimum length is 3, got {got}")
    }

    #[test]
    fn a_struct_field_runs_the_type_bound_beside_its_own() {
        assert_eq!(Holder { slug: slug("ab") }.validate(), Err(vec![short(2)]));
        assert_eq!(
            Holder { slug: slug("AB") }.validate(),
            Err(vec![WRONG_PATTERN.to_owned(), short(2)])
        );
        assert_eq!(Holder { slug: slug("abc") }.validate(), Ok(()));
        assert_eq!(Plain { slug: slug("ab") }.validate(), Err(vec![short(2)]));
    }

    #[test]
    fn an_untagged_named_member_runs_the_type_bound_beside_its_own() {
        assert_eq!(
            Untagged::Named { slug: slug("ab") }.validate(),
            Err(vec![short(2)])
        );
        assert_eq!(
            Untagged::Named { slug: slug("AB") }.validate(),
            Err(vec![WRONG_PATTERN.to_owned(), short(2)])
        );
    }

    #[test]
    fn a_tagged_named_member_runs_the_type_bound_beside_its_own() {
        assert_eq!(
            Tagged::Named { slug: slug("ab") }.validate(),
            Err(vec![short(2)])
        );
        assert_eq!(
            Tagged::Named { slug: slug("AB") }.validate(),
            Err(vec![WRONG_PATTERN.to_owned(), short(2)])
        );
    }

    #[test]
    fn a_slot_runs_the_type_bound_beside_its_own() {
        assert_eq!(
            Tuple(slug("AB")).validate(),
            Err(vec![
                "does not match pattern '^[a-z]+$'".to_owned(),
                "too short: minimum length is 3, got 2".to_owned(),
            ])
        );
    }

    #[test]
    fn the_read_answers_what_validate_answers() {
        let built = Holder { slug: slug("AB") }
            .validate()
            .unwrap_err()
            .join("; ");
        let read = serde_json::from_str::<Holder>(r#"{"slug":"AB"}"#)
            .map(|_| ())
            .map_err(|error| error.to_string());
        assert!(
            read.as_ref().is_err_and(|text| text.starts_with(&built)),
            "{read:?} does not start with {built}"
        );
        let tagged = serde_json::from_str::<Tagged>(r#"{"kind":"Named","slug":"ab"}"#)
            .map(|_| ())
            .map_err(|error| error.to_string());
        assert!(
            tagged
                .as_ref()
                .is_err_and(|text| text.starts_with(&short(2))),
            "{tagged:?}"
        );
    }
}

/// The Crane of two codes, one with a bound of its own beside its brand's.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
mod bounded_crane {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema(minLength = 2)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct Code(pub String);

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Crane {
        #[model_schema_prop(maxLength = 5)]
        pub load_code: Code,
        pub spare_code: Code,
    }

    fn crane(load: &str, spare: &str) -> Crane {
        Crane {
            load_code: Code(load.to_owned()),
            spare_code: Code(spare.to_owned()),
        }
    }

    #[test]
    fn validate_reports_the_brand_bound_and_the_field_bound() {
        assert_eq!(
            crane("a", "a").validate(),
            Err(vec![
                "'loadCode': too short: minimum length is 2, got 1".to_owned(),
                "'spareCode': too short: minimum length is 2, got 1".to_owned(),
            ])
        );
        assert_eq!(
            crane("abcdefg", "ok").validate(),
            Err(vec![
                "'loadCode': too long: maximum length is 5, got 7".to_owned()
            ])
        );
        assert_eq!(crane("ab", "ok").validate(), Ok(()));
    }

    #[test]
    fn the_read_refuses_with_the_text_validate_answers() {
        let built = crane("a", "ok").validate().unwrap_err().join("; ");
        let read = serde_json::from_str::<Crane>(r#"{"loadCode":"a","spareCode":"ok"}"#)
            .unwrap_err()
            .to_string();
        assert!(
            read.starts_with(&built),
            "{read} does not start with {built}"
        );
    }

    #[cfg(feature = "zod")]
    #[test]
    fn the_zod_schema_checks_the_brand_and_the_field() {
        let zod = Crane::zod_schema();
        assert!(
            zod.contains("loadCode: Code$Schema.check(z.maxLength(5"),
            "{zod}"
        );
        assert!(zod.contains("spareCode: Code$Schema,"), "{zod}");
    }
}
