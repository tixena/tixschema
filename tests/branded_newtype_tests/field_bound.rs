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

    /// One field per placement: bare, under an `Option` over a brand declared below, in a list,
    /// over a path declared below, over an alias of a string, and one that writes no bound.
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
            held("a", Some("TOOLONG"), &["abc", "abcd"], "d", "t").validate(),
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
            held("ab", Some("abcd"), &["abc"], "dir", "title").validate(),
            Ok(())
        );
        assert_eq!(held("ab", None, &[], "dir", "title").validate(), Ok(()));
    }

    /// The bound is the validator's to answer: serde reads the value that breaks it.
    #[test]
    fn serde_reads_a_value_the_bound_refuses() {
        let read: BoundHolder = serde_json::from_value(serde_json::json!({
            "bare": "a", "dir": "d", "free": "", "many": ["abcd"], "maybe": "", "titled": "t"
        }))
        .unwrap();
        assert_eq!(read, held("a", Some(""), &["abcd"], "d", "t"));
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
            "  many: z.array(BoundAbove$Schema.check(z.maxLength(3, { error: (issue) => `too long: \
             maximum length is 3, got ${String(issue.input).length}` }))),\n"
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
                "many": {
                    "type": "array",
                    "items": narrowed(serde_json::json!({ "maxLength": 3_u32 })),
                },
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
