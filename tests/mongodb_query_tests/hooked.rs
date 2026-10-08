//! A value written through its field's own serde hook: a date a field stores as a BSON date is
//! written as one under `$lt`, where chrono's own `Serialize` writes text that matches no stored
//! date.
//!
//! The hook is the consumer's attribute, and each major version of the `bson` library names its
//! own, so each binary supplies the two at its root: `date_hook` over a date, and
//! `optional_date_hook` over an `Option` of one. The function that hands a value to the hook is
//! generated on the type, beside the paths.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::shown;
use crate::{date_hook, optional_date_hook};

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublishWindow {
    /// No hook: chrono's own `Serialize` writes this one as text.
    announced: DateTime<Utc>,
    #[serde(with = "date_hook")]
    from_date: DateTime<Utc>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "optional_date_hook"
    )]
    to_date: Option<DateTime<Utc>>,
}

/// Dates stored as the milliseconds since the epoch, through the hook `as_number` hangs on a
/// field: over a date, and over an `Option` of one.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Stamped {
    #[model_schema_prop(as_number)]
    created_at: DateTime<Utc>,
    #[model_schema_prop(as_number)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    seen_at: Option<DateTime<Utc>>,
}

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Template {
    name: String,
    publish_template_to: PublishWindow,
}

/// The instant every case writes: `2026-10-07T17:40:00Z`.
fn now() -> DateTime<Utc> {
    "2026-10-07T17:40:00Z".parse().unwrap()
}

/// The consumer's own filter, `"publishTemplateTo.toDate": { "$lt": now }`: a path below a nested
/// model, over a hook that reads an `Option`.
#[test]
fn a_hooked_date_in_an_option_is_written_as_a_bson_date() {
    assert_eq!(
        shown(
            Template::MONGO_FIELDS
                .publish_template_to
                .to_date
                .lt(now())
                .unwrap()
        ),
        r#"{ "publishTemplateTo.toDate": { "$lt": Date(1791394800000 ms) } }"#
    );
}

#[test]
fn a_hooked_date_is_written_as_a_bson_date() {
    assert_eq!(
        shown(
            Template::MONGO_FIELDS
                .publish_template_to
                .from_date
                .lt(now())
                .unwrap()
        ),
        r#"{ "publishTemplateTo.fromDate": { "$lt": Date(1791394800000 ms) } }"#
    );
}

/// Every operator of a hooked path writes through the hook, and the ones that write no value
/// are as they are on any path.
#[test]
fn every_operator_of_a_hooked_path_writes_through_the_hook() {
    assert_eq!(
        shown(
            PublishWindow::MONGO_FIELDS
                .to_date
                .gte(now())
                .unwrap()
                .and(PublishWindow::MONGO_FIELDS.from_date.lt(now()).unwrap())
        ),
        r#"{ "$and": [{ "toDate": { "$gte": Date(1791394800000 ms) } }, { "fromDate": { "$lt": Date(1791394800000 ms) } }] }"#
    );
    assert_eq!(
        shown(PublishWindow::MONGO_FIELDS.to_date.is_in([now()]).unwrap()),
        r#"{ "toDate": { "$in": [Date(1791394800000 ms)] } }"#
    );
    assert_eq!(
        shown(PublishWindow::MONGO_FIELDS.to_date.set(now()).unwrap()),
        r#"{ "$set": { "toDate": Date(1791394800000 ms) } }"#
    );
    assert_eq!(
        shown(PublishWindow::MONGO_FIELDS.to_date.unset()),
        r#"{ "$unset": { "toDate": "" } }"#
    );
}

/// A date field with no hook is written by chrono's own `Serialize`, as text.
#[test]
fn a_date_with_no_hook_is_written_as_text() {
    assert_eq!(
        shown(PublishWindow::MONGO_FIELDS.announced.lt(now()).unwrap()),
        r#"{ "announced": { "$lt": "2026-10-07T17:40:00Z" } }"#
    );
}

/// What a hooked path writes under an operator is what serde stores at that path's key, so the
/// filter matches the stored row.
#[test]
fn a_hooked_path_writes_what_serde_stores_for_the_field() {
    let window = PublishWindow {
        announced: now(),
        from_date: now(),
        to_date: Some(now()),
    };
    let row = window.serialize(bson::Serializer::new()).unwrap();
    let stored = row.as_document().unwrap();
    assert_eq!(
        shown(stored.clone()),
        r#"{ "announced": "2026-10-07T17:40:00Z", "fromDate": Date(1791394800000 ms), "toDate": Date(1791394800000 ms) }"#
    );
    for (key, filter) in [
        (
            "fromDate",
            PublishWindow::MONGO_FIELDS
                .from_date
                .eq(now())
                .unwrap()
                .into_document(),
        ),
        (
            "toDate",
            PublishWindow::MONGO_FIELDS
                .to_date
                .eq(now())
                .unwrap()
                .into_document(),
        ),
        (
            "announced",
            PublishWindow::MONGO_FIELDS
                .announced
                .eq(now())
                .unwrap()
                .into_document(),
        ),
    ] {
        let written = filter.get_document(key).unwrap().get("$eq");
        assert_eq!(written, stored.get(key), "for {key}");
    }
}

/// A field under `as_number` is stored as a number, through the hook the flag hangs on it, and
/// so is what its path writes: over a date, and over an `Option` of one.
#[test]
fn a_date_under_as_number_is_written_as_a_number() {
    let stamped = Stamped::MONGO_FIELDS;
    assert_eq!(
        shown(stamped.created_at.lt(now()).unwrap()),
        r#"{ "createdAt": { "$lt": Int64(1791394800000) } }"#
    );
    assert_eq!(
        shown(stamped.seen_at.lt(now()).unwrap()),
        r#"{ "seenAt": { "$lt": Int64(1791394800000) } }"#
    );
    assert_eq!(
        shown(stamped.seen_at.exists(false)),
        r#"{ "seenAt": { "$exists": false } }"#
    );
    let row = Stamped {
        created_at: now(),
        seen_at: Some(now()),
    }
    .serialize(bson::Serializer::new())
    .unwrap();
    assert_eq!(
        shown(row.as_document().unwrap().clone()),
        r#"{ "createdAt": Int64(1791394800000), "seenAt": Int64(1791394800000) }"#
    );
}
