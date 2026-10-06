//! A field written as a raw identifier publishes the key serde writes for it, with no `r#`.

/// Named after Rust keywords, the reason a field is written raw.
#[cfg(all(feature = "serde", feature = "typescript"))]
mod on_a_schema_surface {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct RawCamel {
        pub r#my_type: String,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "kind")]
    pub enum RawChoice {
        Held {
            #[model_schema_prop(minLength = 2)]
            r#type: String,
        },
        Other {
            count: i32,
        },
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct RawHolder {
        pub r#struct: RawKeyed,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct RawKeyed {
        #[model_schema_prop(minLength = 2)]
        pub r#match: String,
        pub r#type: String,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct RawRenamed {
        #[serde(rename = "kind")]
        pub r#type: String,
    }

    /// The lines of `definition` that declare a member, without the doc comments around them.
    fn members(definition: &str) -> Vec<&str> {
        definition
            .lines()
            .map(str::trim)
            .filter(|line| line.contains(": ") && line.ends_with(';'))
            .collect()
    }

    #[test]
    fn a_raw_identifier_field_publishes_the_key_serde_writes() {
        let keyed = RawKeyed {
            r#match: "ab".to_owned(),
            r#type: "x".to_owned(),
        };
        assert_eq!(
            serde_json::to_value(&keyed).unwrap(),
            serde_json::json!({ "match": "ab", "type": "x" })
        );
        let definition = RawKeyed::ts_definition();
        assert!(!definition.contains("r#"), "got: {definition}");
        assert_eq!(
            members(&definition),
            ["match: string;", "type: string;"],
            "got: {definition}"
        );
    }

    #[test]
    fn a_rename_all_is_applied_to_the_name_without_its_prefix() {
        let definition = RawCamel::ts_definition();
        assert_eq!(
            members(&definition),
            ["myType: string;"],
            "got: {definition}"
        );
    }

    #[test]
    fn a_struct_variants_member_publishes_without_the_prefix() {
        let definition = RawChoice::ts_definition();
        assert!(!definition.contains("r#"), "got: {definition}");
        assert!(definition.contains("type: string;"), "got: {definition}");
    }

    #[test]
    fn an_explicit_rename_still_names_the_key() {
        let definition = RawRenamed::ts_definition();
        assert_eq!(members(&definition), ["kind: string;"], "got: {definition}");
    }

    #[test]
    fn a_constraint_on_a_raw_identifier_field_is_checked_under_the_key() {
        let keyed = RawKeyed {
            r#match: "a".to_owned(),
            r#type: "x".to_owned(),
        };
        assert_eq!(
            keyed.validate().unwrap_err(),
            ["'match': too short: minimum length is 2, got 1"]
        );
        assert_eq!(
            RawHolder { r#struct: keyed }.validate().unwrap_err(),
            ["'struct.match': too short: minimum length is 2, got 1"]
        );
        let held = RawChoice::Held {
            r#type: "x".to_owned(),
        };
        assert_eq!(
            held.validate().unwrap_err(),
            ["'type': too short: minimum length is 2, got 1"]
        );
    }

    #[cfg(feature = "zod")]
    #[test]
    fn the_zod_schema_names_the_key_serde_writes() {
        let keyed = RawKeyed::zod_schema();
        assert!(!keyed.contains("r#"), "got: {keyed}");
        assert!(keyed.contains("  type: z.string(),"), "got: {keyed}");
        assert!(keyed.contains("  match: z.string().min(2"), "got: {keyed}");
        assert!(
            RawCamel::zod_schema().contains("  myType: z.string(),"),
            "got: {}",
            RawCamel::zod_schema()
        );
    }

    #[cfg(feature = "jsonschema")]
    #[test]
    fn the_json_schema_names_the_key_serde_writes() {
        assert_eq!(
            RawKeyed::json_schema(),
            serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "match": { "type": "string", "minLength": 2_i32 },
                    "type": { "type": "string" },
                },
                "required": ["match", "type"],
            })
        );
    }
}

/// `type` and `match` are reserved in none of the three languages, so each names the member as
/// serde names the key.
#[cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
mod on_a_mobile_surface {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct RawMobile {
        pub r#match: String,
        pub r#type: String,
    }

    #[test]
    fn serde_writes_the_keys_the_three_languages_name() {
        let written = serde_json::to_value(RawMobile {
            r#match: "a".to_owned(),
            r#type: "b".to_owned(),
        })
        .unwrap();
        assert_eq!(written, serde_json::json!({ "match": "a", "type": "b" }));
    }

    #[cfg(feature = "dart")]
    #[test]
    fn dart_names_the_member_and_the_key_without_the_prefix() {
        let definition = raw_mobile_dart::dart_definition();
        assert!(!definition.contains("r#"), "got: {definition}");
        assert!(
            definition.contains("final String type;"),
            "got: {definition}"
        );
        assert!(
            definition.contains("json['match'] as String"),
            "got: {definition}"
        );
    }

    #[cfg(feature = "swift")]
    #[test]
    fn swift_names_the_member_and_the_key_without_the_prefix() {
        let definition = raw_mobile_swift::swift_definition();
        assert!(!definition.contains("r#"), "got: {definition}");
        assert!(
            definition.contains("public let type: String;"),
            "got: {definition}"
        );
        assert!(definition.contains("case match;"), "got: {definition}");
    }

    #[cfg(feature = "kotlin")]
    #[test]
    fn kotlin_names_the_member_without_the_prefix() {
        let definition = raw_mobile_kotlin::kotlin_definition();
        assert!(
            definition.contains("data class RawMobile(val match: String, val type: String)"),
            "got: {definition}"
        );
    }
}
