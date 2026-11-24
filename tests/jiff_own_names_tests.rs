//! A type of the crate's own named as a jiff type is: under `jiff`, a bare name is jiff's unless
//! this crate declared a `#[model_schema]` type under it above the type that holds it. A binary of
//! its own: the names the crate declares are read for the whole crate, so a `Timestamp` of its
//! own would be every `Timestamp` field of any other test here.

#![cfg(feature = "jiff")]

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    /// A crate's own `Timestamp`, declared above the type that holds it.
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    struct Timestamp {
        seconds: i64,
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    struct Visit {
        at: Timestamp,
    }

    #[test]
    fn a_crates_own_timestamp_is_written_as_that_type() {
        let visit = VisitDraft {
            at: TimestampDraft { seconds: 1 }.validate().unwrap(),
        }
        .validate()
        .unwrap();
        assert_eq!(
            serde_json::to_string(&visit).unwrap(),
            r#"{"at":{"seconds":1}}"#
        );
    }

    #[cfg(feature = "typescript")]
    #[test]
    fn a_crates_own_timestamp_is_named_by_its_own_type_in_typescript() {
        let declared = Visit::ts_definition();
        assert!(
            declared.lines().any(|line| line == "  at: Timestamp;"),
            "got: {declared}"
        );
    }

    #[cfg(feature = "zod")]
    #[test]
    fn a_crates_own_timestamp_is_read_through_its_own_schema() {
        let zod = Visit::zod_schema();
        assert!(zod.contains("at: Timestamp$Schema"), "got: {zod}");
        assert!(!zod.contains("z.coerce.date()"), "got: {zod}");
    }
}
