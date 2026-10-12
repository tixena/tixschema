//! A value written through its field's own serde hook: a date a field stores as a BSON date is
//! written as one under `$lt`, where chrono's own `Serialize` writes text that matches no stored
//! date. A hooked field may be typed with `Self`, and its type may carry a bound on `Self`.

mod in_a_list;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::shown;
use crate::{date_hook, optional_date_hook};

/// Holds the depot above it behind a hook of its own, in an `Option` of a `Box`.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Depot {
    code: String,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "in_a_list")]
    parent: Option<Box<Self>>,
}

/// Holds itself in a variant's field, behind a hook of its own.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum Load {
    Bundle {
        #[serde(with = "in_a_list")]
        parts: Vec<Self>,
        strapped: bool,
    },
    Item {
        sku: String,
    },
}

/// Holds itself in a list, behind a hook of its own.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Node {
    #[serde(with = "in_a_list")]
    children: Vec<Self>,
    name: String,
}

/// A variant's field stored through a hook of its own.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum Opening {
    Never,
    #[serde(rename_all = "camelCase")]
    Once {
        #[serde(with = "date_hook")]
        opens_at: DateTime<Utc>,
    },
}

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

/// A hooked field on a type whose own bound names `Self`: [`Tracked`], which the type meets and
/// its struct of paths does not.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Scan
where
    Self: Sized + Tracked,
{
    #[serde(with = "date_hook")]
    scanned_at: DateTime<Utc>,
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

/// What [`Scan`]'s own bound asks of it.
trait Tracked {}

impl Tracked for Scan {}

fn leaf(name: &str) -> Node {
    Node {
        children: Vec::new(),
        name: name.to_owned(),
    }
}

/// The instant every case writes: `2026-10-07T17:40:00Z`.
fn now() -> DateTime<Utc> {
    "2026-10-07T17:40:00Z".parse().unwrap()
}

#[test]
fn a_hooked_date_in_an_option_is_written_as_a_bson_date() {
    assert_eq!(
        shown(
            Template::MONGO_FIELDS
                .publish_template_to()
                .nest(PublishWindow::MONGO_FIELDS.to_date().lt(now()).unwrap())
        ),
        r#"{ "publishTemplateTo.toDate": { "$lt": Date(1791394800000 ms) } }"#
    );
}

#[test]
fn a_hooked_date_is_written_as_a_bson_date() {
    assert_eq!(
        shown(
            Template::MONGO_FIELDS
                .publish_template_to()
                .nest(PublishWindow::MONGO_FIELDS.from_date().lt(now()).unwrap())
        ),
        r#"{ "publishTemplateTo.fromDate": { "$lt": Date(1791394800000 ms) } }"#
    );
}

#[test]
fn every_operator_of_a_hooked_path_writes_through_the_hook() {
    assert_eq!(
        shown(
            PublishWindow::MONGO_FIELDS
                .to_date()
                .gte(now())
                .unwrap()
                .and(PublishWindow::MONGO_FIELDS.from_date().lt(now()).unwrap())
        ),
        r#"{ "$and": [{ "toDate": { "$gte": Date(1791394800000 ms) } }, { "fromDate": { "$lt": Date(1791394800000 ms) } }] }"#
    );
    assert_eq!(
        shown(
            PublishWindow::MONGO_FIELDS
                .to_date()
                .is_in([now()])
                .unwrap()
        ),
        r#"{ "toDate": { "$in": [Date(1791394800000 ms)] } }"#
    );
    assert_eq!(
        shown(PublishWindow::MONGO_FIELDS.to_date().set(now()).unwrap()),
        r#"{ "$set": { "toDate": Date(1791394800000 ms) } }"#
    );
    assert_eq!(
        shown(PublishWindow::MONGO_FIELDS.to_date().unset()),
        r#"{ "$unset": { "toDate": "" } }"#
    );
}

/// A date field with no hook is written by chrono's own `Serialize`, as text.
#[test]
fn a_date_with_no_hook_is_written_as_text() {
    assert_eq!(
        shown(PublishWindow::MONGO_FIELDS.announced().lt(now()).unwrap()),
        r#"{ "announced": { "$lt": "2026-10-07T17:40:00Z" } }"#
    );
}

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
                .from_date()
                .eq(now())
                .unwrap()
                .into_document(),
        ),
        (
            "toDate",
            PublishWindow::MONGO_FIELDS
                .to_date()
                .eq(now())
                .unwrap()
                .into_document(),
        ),
        (
            "announced",
            PublishWindow::MONGO_FIELDS
                .announced()
                .eq(now())
                .unwrap()
                .into_document(),
        ),
    ] {
        let written = filter.get_document(key).unwrap().get("$eq");
        assert_eq!(written, stored.get(key), "for {key}");
    }
}

#[test]
fn a_date_under_as_number_is_written_as_a_number() {
    let stamped = Stamped::MONGO_FIELDS;
    assert_eq!(
        shown(stamped.created_at().lt(now()).unwrap()),
        r#"{ "createdAt": { "$lt": Int64(1791394800000) } }"#
    );
    assert_eq!(
        shown(stamped.seen_at().lt(now()).unwrap()),
        r#"{ "seenAt": { "$lt": Int64(1791394800000) } }"#
    );
    assert_eq!(
        shown(stamped.seen_at().exists(false)),
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

#[test]
fn a_hooked_field_of_a_variant_is_written_as_a_bson_date() {
    let opening = Opening::MONGO_FIELDS;
    assert_eq!(
        shown(opening.once().opens_at().lt(now()).unwrap()),
        r#"{ "opensAt": { "$lt": Date(1791394800000 ms) } }"#
    );
    assert_eq!(
        shown(opening.is_never()),
        r#"{ "kind": { "$eq": "never" } }"#
    );
}

/// The function of a hooked path calls one of the type's own, and is written beside the type.
#[test]
fn a_type_with_a_hooked_field_declared_inside_a_function_builds() {
    #[model_schema(decode_with)]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Stamp {
        #[serde(with = "date_hook")]
        stamped_at: DateTime<Utc>,
    }

    assert_eq!(
        shown(Stamp::MONGO_FIELDS.stamped_at().lt(now()).unwrap()),
        r#"{ "stampedAt": { "$lt": Date(1791394800000 ms) } }"#
    );
}

/// The function of a hooked path is written beside the type, in an `impl` of its struct of
/// paths: the path is over the type its author wrote `Self` for.
#[test]
fn a_hooked_field_typed_with_self_is_a_path_over_the_type() {
    let children: node_schema::Field<Node, Vec<Node>> = Node::MONGO_FIELDS.children();
    assert_eq!(
        shown(children.eq(vec![leaf("leaf")]).unwrap()),
        r#"{ "children": { "$eq": [[{ "children": [[]], "name": "leaf" }]] } }"#
    );
    let root = Node {
        children: vec![leaf("first"), leaf("second")],
        name: "root".to_owned(),
    };
    let row = root.serialize(bson::Serializer::new()).unwrap();
    let stored = row.as_document().unwrap();
    let filter = children.eq(root.children).unwrap().into_document();
    assert_eq!(
        filter.get_document("children").unwrap().get("$eq"),
        stored.get("children")
    );
}

#[test]
fn a_hooked_option_of_a_box_of_self_is_an_optional_path() {
    let parent: depot_schema::OptionalField<Depot, Box<Depot>, Depot> =
        Depot::MONGO_FIELDS.parent();
    let north = Depot {
        code: "NORTH".to_owned(),
        parent: None,
    };
    assert_eq!(
        shown(parent.eq(Box::new(north)).unwrap()),
        r#"{ "parent": { "$eq": [{ "code": "NORTH" }] } }"#
    );
    assert_eq!(shown(parent.unset()), r#"{ "$unset": { "parent": "" } }"#);
}

#[test]
fn a_hooked_field_of_a_variant_typed_with_self_is_a_path_over_the_enum() {
    let load = Load::MONGO_FIELDS;
    let parts: load_schema::Field<Load, Vec<Load>> = load.bundle().parts();
    let item = Load::Item {
        sku: "A-1".to_owned(),
    };
    assert_eq!(
        shown(parts.eq(vec![item]).unwrap()),
        r#"{ "parts": { "$eq": [[{ "kind": "item", "sku": "A-1" }]] } }"#
    );
    assert_eq!(
        shown(load.bundle().strapped().eq(true).unwrap()),
        r#"{ "strapped": { "$eq": true } }"#
    );
}

/// Beside the type a bound on `Self` is a bound on the type, which its struct of paths need not
/// meet for the hooked path to be called.
#[test]
fn a_hooked_path_of_a_type_bound_on_self_is_called() {
    assert_eq!(
        shown(Scan::MONGO_FIELDS.scanned_at().lt(now()).unwrap()),
        r#"{ "scannedAt": { "$lt": Date(1791394800000 ms) } }"#
    );
}
