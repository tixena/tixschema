//! `ts_optional` is the whole of what decides between `field?: T` and `field: T | undefined`.

mod under_the_serde_feature {
    use super::member;
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    /// The key is dropped by a predicate. `a` asks for the optional key as well; `b` does not.
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    struct UnderAPredicate {
        #[model_schema_prop(ts_optional)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        a: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        b: Option<String>,
    }

    /// The key is dropped unconditionally on the way out.
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    struct UnderSkipSerializing {
        #[model_schema_prop(ts_optional)]
        #[serde(skip_serializing)]
        a: Option<String>,
        #[serde(skip_serializing)]
        b: Option<String>,
    }

    #[test]
    fn the_flag_decides_the_spelling_the_omission_attribute_leaves_open() {
        let predicate = UnderAPredicate::ts_definition();
        assert!(
            member(&predicate, "a?: string | undefined;"),
            "Got: {predicate}"
        );
        assert!(
            member(&predicate, "b: string | undefined;"),
            "Got: {predicate}"
        );

        let skip_serializing = UnderSkipSerializing::ts_definition();
        assert!(
            member(&skip_serializing, "a?: string | undefined;"),
            "Got: {skip_serializing}"
        );
        assert!(
            member(&skip_serializing, "b: string | undefined;"),
            "Got: {skip_serializing}"
        );
    }
}

/// Whether the emission writes this member, matched as a whole line so a spelling that merely
/// starts the same cannot pass for it — `nickname?: string | undefined;` and
/// `nickname: string | undefined;` are the two answers this file is about, and neither may be read
/// off a prefix of the other.
fn member(ts: &str, line: &str) -> bool {
    ts.lines().any(|written| written.trim() == line)
}
