//! A `#[serde(transparent)]` struct with named fields is the brand the tuple form is, over the one
//! field serde reads it as.

/// The same brand declared both ways. serde writes both as the string alone, so every surface
/// describes both the same way.
#[cfg(all(
    feature = "serde",
    any(
        feature = "typescript",
        feature = "jsonschema",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
))]
mod beside_the_tuple_form {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub struct NamedHolder {
        pub marked: NamedMarked,
        pub slug: NamedSlug,
    }

    /// Beside the field it is the value of, one serde neither writes nor reads.
    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct NamedMarked {
        #[serde(skip)]
        pub note: u8,
        pub text: String,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct NamedSlug {
        pub text: String,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct TupleSlug(pub String);

    /// What is written for the tuple form, under the names the named form is written with.
    fn under_the_named_name(written: &str) -> String {
        written
            .replace("TupleSlug", "NamedSlug")
            .replace("tuple_slug", "named_slug")
            .replace("tupleSlug", "namedSlug")
    }

    #[test]
    fn serde_writes_and_reads_the_value_alone() {
        let slug = NamedSlug {
            text: "x".to_owned(),
        };
        assert_eq!(serde_json::to_value(&slug).unwrap(), serde_json::json!("x"));
        assert_eq!(
            serde_json::to_value(TupleSlug("x".to_owned())).unwrap(),
            serde_json::json!("x")
        );
        assert_eq!(
            serde_json::from_value::<NamedSlug>(serde_json::json!("x")).unwrap(),
            slug
        );
        let holder = NamedHolder {
            marked: NamedMarked {
                note: 7,
                text: "y".to_owned(),
            },
            slug,
        };
        assert_eq!(
            serde_json::to_value(&holder).unwrap(),
            serde_json::json!({ "marked": "y", "slug": "x" })
        );
    }

    #[cfg(feature = "typescript")]
    #[test]
    fn typescript_describes_both_alike() {
        assert_eq!(
            NamedSlug::ts_definition(),
            under_the_named_name(&TupleSlug::ts_definition())
        );
        let holder = NamedHolder::ts_definition();
        assert!(holder.contains("  marked: NamedMarked;"), "got: {holder}");
        assert!(holder.contains("  slug: NamedSlug;"), "got: {holder}");
    }

    #[cfg(feature = "zod")]
    #[test]
    fn zod_describes_both_alike() {
        assert_eq!(
            NamedSlug::zod_schema(),
            under_the_named_name(&TupleSlug::zod_schema())
        );
    }

    #[cfg(feature = "jsonschema")]
    #[test]
    fn json_schema_describes_the_value_and_a_type_that_holds_it() {
        assert_eq!(
            NamedSlug::json_schema(),
            serde_json::json!({ "type": "string" })
        );
        assert_eq!(NamedSlug::json_schema(), TupleSlug::json_schema());
        assert_eq!(
            NamedMarked::json_schema(),
            serde_json::json!({ "type": "string" })
        );
        assert_eq!(
            NamedHolder::json_schema(),
            serde_json::json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "marked": { "type": "string" },
                    "slug": { "type": "string" },
                },
                "required": ["marked", "slug"],
            })
        );
    }

    #[cfg(feature = "dart")]
    #[test]
    fn dart_describes_both_alike() {
        assert_eq!(
            named_slug_dart::dart_definition(),
            under_the_named_name(&tuple_slug_dart::dart_definition())
        );
    }

    #[cfg(feature = "swift")]
    #[test]
    fn swift_describes_both_alike() {
        assert_eq!(
            named_slug_swift::swift_definition(),
            under_the_named_name(&tuple_slug_swift::swift_definition())
        );
    }

    #[cfg(feature = "kotlin")]
    #[test]
    fn kotlin_describes_both_alike() {
        assert_eq!(
            named_slug_kotlin::kotlin_definition(),
            under_the_named_name(&tuple_slug_kotlin::kotlin_definition())
        );
    }

    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    #[test]
    fn it_is_displayed_as_the_value_it_holds() {
        let slug = NamedSlug {
            text: "x".to_owned(),
        };
        assert_eq!(slug.to_string(), "x");
    }
}
