//! A struct of paths answers one function per field and carries none of its own beside them. A
//! model may declare fields named as what a path kind keeps for itself, and as the functions the
//! kinds and the structs of paths are built and read with: this is that model, read from outside
//! the module that declares the kinds.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::shown;

/// One field per name a path kind keeps for itself, or builds and reads a path with.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Named {
    built: String,
    fields: String,
    key: String,
    nest: String,
    path: String,
    plain: String,
    present: String,
    root: String,
    row: String,
    whole: String,
    write: String,
}

/// Holds [`Named`] every way a path kind wraps a nested model.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Holder {
    many: Vec<Named>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    maybe: Option<Named>,
    one: Named,
}

fn text() -> String {
    "x".to_owned()
}

#[test]
fn a_nested_field_named_like_a_path_kinds_own_is_reached_and_writes_its_key() {
    let (holder, named) = (Holder::MONGO_FIELDS, Named::MONGO_FIELDS);
    for (written, expected) in [
        (
            holder.one().nest(named.whole().eq(text()).unwrap()),
            r#"{ "one.whole": { "$eq": "x" } }"#,
        ),
        (
            holder.one().nest(named.fields().eq(text()).unwrap()),
            r#"{ "one.fields": { "$eq": "x" } }"#,
        ),
        (
            holder.maybe().nest(named.present().eq(text()).unwrap()),
            r#"{ "maybe.present": { "$eq": "x" } }"#,
        ),
        (
            holder.maybe().nest(named.whole().eq(text()).unwrap()),
            r#"{ "maybe.whole": { "$eq": "x" } }"#,
        ),
        (
            holder.many().nest(named.fields().eq(text()).unwrap()),
            r#"{ "many.fields": { "$eq": "x" } }"#,
        ),
        (
            holder.many().nest(named.path().eq(text()).unwrap()),
            r#"{ "many.path": { "$eq": "x" } }"#,
        ),
        (
            holder.one().nest(named.write().eq(text()).unwrap()),
            r#"{ "one.write": { "$eq": "x" } }"#,
        ),
        (
            holder.one().nest(named.root().eq(text()).unwrap()),
            r#"{ "one.root": { "$eq": "x" } }"#,
        ),
    ] {
        assert_eq!(shown(written), expected);
    }
}

/// A struct of paths has no function of its own for one of a field's name to meet.
#[test]
fn a_field_named_like_a_function_a_path_is_built_or_read_with_builds() {
    let (holder, named) = (Holder::MONGO_FIELDS, Named::MONGO_FIELDS);
    for (written, expected) in [
        (
            named.built().eq(text()).unwrap(),
            r#"{ "built": { "$eq": "x" } }"#,
        ),
        (
            named.key().eq(text()).unwrap(),
            r#"{ "key": { "$eq": "x" } }"#,
        ),
        (
            named.nest().eq(text()).unwrap(),
            r#"{ "nest": { "$eq": "x" } }"#,
        ),
        (
            named.plain().eq(text()).unwrap(),
            r#"{ "plain": { "$eq": "x" } }"#,
        ),
        (
            named.row().eq(text()).unwrap(),
            r#"{ "row": { "$eq": "x" } }"#,
        ),
    ] {
        assert_eq!(shown(written), expected);
    }
    assert_eq!(
        shown(holder.many().nest(named.nest().eq(text()).unwrap())),
        r#"{ "many.nest": { "$eq": "x" } }"#
    );
}

/// The path kinds' own operators stay where they are beside a nested model of those names.
#[test]
fn a_path_kind_keeps_its_own_operators_beside_fields_of_those_names() {
    let holder = Holder::MONGO_FIELDS;
    assert_eq!(
        shown(holder.maybe().exists(false)),
        r#"{ "maybe": { "$exists": false } }"#
    );
    assert_eq!(
        shown(holder.many().size(2)),
        r#"{ "many": { "$size": Int64(2) } }"#
    );
    assert_eq!(
        shown(
            holder
                .one()
                .nest(Named::MONGO_FIELDS.root().set(text()).unwrap())
        ),
        r#"{ "$set": { "one.root": "x" } }"#
    );
}
