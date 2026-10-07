//! A value written through its field's own serde hook: a date a field stores as a BSON date is
//! written as one under `$lt`, where chrono's own `Serialize` writes text that matches no stored
//! date.
//!
//! The hook is the consumer's attribute, and each major version of the `bson` library names its
//! own, so each binary supplies the two at its root: `date_hook` over a date, and
//! `optional_date_hook` over an `Option` of one.

use bson::Bson;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::{Keys, shown};
use crate::{date_hook, optional_date_hook};

/// The paths of a [`Template`] that is the row itself.
const TEMPLATE: TemplatePaths<Template> = template_paths(template_schema::MongoPath::ROOT);

/// The paths of a [`PublishWindow`] that is the row itself.
const WINDOW: PublishWindowPaths<PublishWindow> =
    publish_window_paths(publish_window_schema::MongoPath::ROOT);

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

/// The paths of a [`PublishWindow`], under whatever leads to it in a row of `Root`.
struct PublishWindowPaths<Root> {
    announced: publish_window_schema::Field<Root, DateTime<Utc>>,
    from_date: publish_window_schema::Field<Root, DateTime<Utc>>,
    to_date: publish_window_schema::OptionalField<Root, DateTime<Utc>>,
}

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Template {
    name: String,
    publish_template_to: PublishWindow,
}

/// The paths of a [`Template`], under whatever leads to it in a row of `Root`.
struct TemplatePaths<Root> {
    publish_template_to: template_schema::Model<Root, PublishWindow, PublishWindowPaths<Root>>,
}

/// The instant every case writes: `2026-10-07T17:40:00Z`.
fn now() -> DateTime<Utc> {
    "2026-10-07T17:40:00Z".parse().unwrap()
}

const fn publish_window_paths<Root>(prefix: Keys) -> PublishWindowPaths<Root> {
    PublishWindowPaths {
        announced: publish_window_schema::Field::plain(publish_window_schema::MongoPath::under(
            prefix,
            "announced",
        )),
        from_date: publish_window_schema::Field::hooked(
            publish_window_schema::MongoPath::under(prefix, "fromDate"),
            write_from_date,
        ),
        to_date: publish_window_schema::OptionalField::hooked(
            publish_window_schema::MongoPath::under(prefix, "toDate"),
            write_to_date,
        ),
    }
}

const fn template_paths<Root>(prefix: Keys) -> TemplatePaths<Root> {
    TemplatePaths {
        publish_template_to: template_schema::Model::plain(
            template_schema::MongoPath::under(prefix, "publishTemplateTo"),
            publish_window_paths(
                template_schema::MongoPath::under(prefix, "publishTemplateTo").segments,
            ),
        ),
    }
}

/// `from_date` as its hook writes it.
fn write_from_date(value: DateTime<Utc>) -> Result<Bson, publish_window_schema::WriteError> {
    date_hook::serialize(&value, bson::Serializer::new())
}

/// `to_date` as its hook writes it: the hook reads the field's own type, an `Option`.
fn write_to_date(value: DateTime<Utc>) -> Result<Bson, publish_window_schema::WriteError> {
    optional_date_hook::serialize(&Some(value), bson::Serializer::new())
}

/// The consumer's own filter, `"publishTemplateTo.toDate": { "$lt": now }`: a path below a nested
/// model, over a hook that reads an `Option`.
#[test]
fn a_hooked_date_in_an_option_is_written_as_a_bson_date() {
    assert_eq!(
        shown(TEMPLATE.publish_template_to.to_date.lt(now()).unwrap()),
        r#"{ "publishTemplateTo.toDate": { "$lt": Date(1791394800000 ms) } }"#
    );
}

#[test]
fn a_hooked_date_is_written_as_a_bson_date() {
    assert_eq!(
        shown(TEMPLATE.publish_template_to.from_date.lt(now()).unwrap()),
        r#"{ "publishTemplateTo.fromDate": { "$lt": Date(1791394800000 ms) } }"#
    );
}

/// Every operator of a hooked path writes through the hook, and the ones that write no value
/// are as they are on any path.
#[test]
fn every_operator_of_a_hooked_path_writes_through_the_hook() {
    assert_eq!(
        shown(
            WINDOW
                .to_date
                .gte(now())
                .unwrap()
                .and(WINDOW.from_date.lt(now()).unwrap())
        ),
        r#"{ "$and": [{ "toDate": { "$gte": Date(1791394800000 ms) } }, { "fromDate": { "$lt": Date(1791394800000 ms) } }] }"#
    );
    assert_eq!(
        shown(WINDOW.to_date.is_in([now()]).unwrap()),
        r#"{ "toDate": { "$in": [Date(1791394800000 ms)] } }"#
    );
    assert_eq!(
        shown(WINDOW.to_date.set(now()).unwrap()),
        r#"{ "$set": { "toDate": Date(1791394800000 ms) } }"#
    );
    assert_eq!(
        shown(WINDOW.to_date.unset()),
        r#"{ "$unset": { "toDate": "" } }"#
    );
}

/// A date field with no hook is written by chrono's own `Serialize`, as text.
#[test]
fn a_date_with_no_hook_is_written_as_text() {
    assert_eq!(
        shown(WINDOW.announced.lt(now()).unwrap()),
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
            WINDOW.from_date.eq(now()).unwrap().into_document(),
        ),
        ("toDate", WINDOW.to_date.eq(now()).unwrap().into_document()),
        (
            "announced",
            WINDOW.announced.eq(now()).unwrap().into_document(),
        ),
    ] {
        let written = filter.get_document(key).unwrap().get("$eq");
        assert_eq!(written, stored.get(key), "for {key}");
    }
}
