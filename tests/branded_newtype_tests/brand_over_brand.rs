//! A length or a pattern written on a field typed with a brand over a brand measures the text the
//! innermost string holds, on every surface.

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
mod measured_through_the_inner_brand {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema()]
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct Strand(pub String);

    #[model_schema()]
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct Braid(pub Strand);

    #[model_schema()]
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct Cord(pub Braid);

    #[model_schema()]
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub struct CordHolder {
        #[model_schema_prop(minLength = 3)]
        pub badge: Braid,
        #[model_schema_prop(maxLength = 4, pattern = "^[a-z]+$")]
        pub medal: Cord,
        #[model_schema_prop(minLength = 2)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub spare: Option<Braid>,
    }

    fn held(badge: &str, medal: &str, spare: Option<&str>) -> CordHolder {
        CordHolder {
            badge: Braid(Strand(badge.to_owned())),
            medal: Cord(Braid(Strand(medal.to_owned()))),
            spare: spare.map(|text| Braid(Strand(text.to_owned()))),
        }
    }

    #[test]
    fn validate_measures_the_text_through_each_level() {
        assert_eq!(held("abc", "abcd", Some("ab")).validate(), Ok(()));
        assert_eq!(
            held("a", "abcde", Some("a")).validate(),
            Err(vec![
                "'badge': too short: minimum length is 3, got 1".to_owned(),
                "'medal': too long: maximum length is 4, got 5".to_owned(),
                "'spare': too short: minimum length is 2, got 1".to_owned(),
            ])
        );
        assert_eq!(
            held("abc", "AB", None).validate(),
            Err(vec![
                "'medal': does not match pattern '^[a-z]+$'".to_owned()
            ])
        );
    }

    #[test]
    fn the_read_refuses_what_validate_refuses() {
        let broken = held("a", "abcde", Some("a"));
        let read = serde_json::from_value::<CordHolder>(
            serde_json::json!({ "badge": "a", "medal": "abcde", "spare": "a" }),
        );
        assert_eq!(
            read.map_err(|error| error.to_string()),
            Err(broken.validate().unwrap_err().join("; "))
        );
    }

    #[cfg(feature = "zod")]
    #[test]
    fn the_zod_schema_checks_the_text_at_the_field() {
        let zod = CordHolder::zod_schema();
        let short = "{ error: (issue) => `too short: minimum length is 3, got ${Array.isArray(issue.input) ? issue.input.length : String(issue.input).length}` }";
        assert!(
            zod.contains(&format!(
                "  badge: Braid$Schema.check(z.minLength(3, {short})),\n"
            )),
            "{zod}"
        );
        assert!(
            zod.contains("medal: Cord$Schema.check(z.maxLength(4,")
                && zod.contains("z.regex(/^[a-z]+$/"),
            "{zod}"
        );
    }

    #[cfg(feature = "jsonschema")]
    #[test]
    fn the_json_schema_narrows_the_text_at_the_field() {
        let schema = CordHolder::json_schema();
        let narrowed =
            |by: serde_json::Value| serde_json::json!({ "allOf": [{ "type": "string" }, by] });
        assert_eq!(
            schema["properties"]["badge"],
            narrowed(serde_json::json!({ "minLength": 3_u32, "minItems": 3_u32 }))
        );
        assert_eq!(
            schema["properties"]["medal"],
            narrowed(serde_json::json!({
                "maxLength": 4_u32, "maxItems": 4_u32, "pattern": "^[a-z]+$"
            }))
        );
    }
}
