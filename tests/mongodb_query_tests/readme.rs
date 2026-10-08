//! The README's example of a field stored through a serde hook of its own, held as one text with
//! the code that compiles, and run.
//!
//! The hook is each binary's `date_hook`, which each major version of the `bson` library names
//! its own way: the README names both, and each is held to the binary that binds it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use crate::date_hook;

use window_schema::{Filter, WriteError};

/// The function the README builds two filters with, character for character.
const DECLARED_CLOSING: &str =
    "/// Windows that close before an instant, and windows that open before it.
fn closing_before(instant: DateTime<Utc>) -> Result<[Filter<Window>; 2], WriteError> {
    let paths = Window::MONGO_FIELDS;
    Ok([paths.closes_at.lt(instant)?, paths.opens_at.lt(instant)?])
}";

/// The lines the README binds each major version's hook under one name with: version 2's, and
/// version 3's.
const DECLARED_HOOKS: [&str; 2] = [
    "use bson::serde_helpers::chrono_datetime_as_bson_datetime as date_hook;",
    "use bson::serde_helpers::datetime::FromChrono04DateTime as date_hook;",
];

/// The line the README brings the query types into scope with.
const DECLARED_IMPORT: &str = "use window_schema::{Filter, WriteError};";

/// The type the README declares, character for character.
const DECLARED_WINDOW: &str = r#"#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Window {
    #[serde(with = "date_hook")]
    pub closes_at: DateTime<Utc>,
    pub opens_at: DateTime<Utc>,
}"#;

/// The two filters the README shows, as `bson::Document` displays them.
const SHOWN_FILTERS: [&str; 2] = [
    r#"{ "closesAt": { "$lt": DateTime("2026-10-07 17:40:00.0 +00:00:00") } }"#,
    r#"{ "opensAt": { "$lt": "2026-10-07T17:40:00Z" } }"#,
];

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Window {
    #[serde(with = "date_hook")]
    pub closes_at: DateTime<Utc>,
    pub opens_at: DateTime<Utc>,
}

/// Windows that close before an instant, and windows that open before it.
fn closing_before(instant: DateTime<Utc>) -> Result<[Filter<Window>; 2], WriteError> {
    let paths = Window::MONGO_FIELDS;
    Ok([paths.closes_at.lt(instant)?, paths.opens_at.lt(instant)?])
}

fn readme() -> &'static str {
    include_str!("../../README.md")
}

#[test]
fn the_readme_declares_the_hooked_field_that_compiles_here() {
    let source = include_str!("readme.rs");
    for pinned in [DECLARED_CLOSING, DECLARED_IMPORT, DECLARED_WINDOW] {
        assert_eq!(
            source.matches(pinned).count(),
            2,
            "this is pinned but no longer declared here character for character:\n{pinned}"
        );
        assert!(
            readme().contains(pinned),
            "the README no longer declares this verbatim:\n{pinned}"
        );
    }
}

/// Each binary binds its own major version's hook as `date_hook`, by the name the README gives
/// that version's.
#[test]
fn the_readme_names_the_hook_each_binary_binds() {
    let [under_2, under_3] = DECLARED_HOOKS;
    assert!(
        include_str!("../mongodb_query_bson2_tests.rs")
            .contains("chrono_datetime_as_bson_datetime as date_hook,"),
        "the binary built against version 2 no longer binds the hook the README names"
    );
    assert!(
        include_str!("../../bson3/tests/mongodb_query_bson3_tests.rs").contains(under_3),
        "the binary built against version 3 no longer binds the hook the README names"
    );
    for shown in [under_2, under_3] {
        assert!(
            readme().contains(shown),
            "the README no longer shows this verbatim:\n{shown}"
        );
    }
}

#[test]
fn the_readme_shows_what_a_hooked_date_and_a_plain_one_are_compared_as() {
    let instant: DateTime<Utc> = "2026-10-07T17:40:00Z".parse().unwrap();
    let written = closing_before(instant)
        .unwrap()
        .map(|filter| filter.into_document().to_string());
    assert_eq!(written, SHOWN_FILTERS);
    let shown = SHOWN_FILTERS.join("\n");
    assert!(
        readme().contains(&shown),
        "the README no longer shows these filters verbatim:\n{shown}"
    );
}
