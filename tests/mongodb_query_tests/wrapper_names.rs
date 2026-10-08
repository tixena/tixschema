//! `Model`, `OptionalModel` and `ModelList` reach the nested model's paths by dereferencing, and
//! keep fields of their own. A nested model may declare fields of those very names: this is that
//! model, read from outside the module that declares the kinds.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::shown;

/// One field per name a path kind keeps for itself.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Named {
    fields: String,
    path: String,
    present: String,
    root: String,
    whole: String,
    write: String,
}

/// Holds [`Named`] every way a path kind wraps a nested model.
#[model_schema(decode_with)]
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
    let holder = Holder::MONGO_FIELDS;
    for (written, expected) in [
        (
            holder.one.whole.eq(text()).unwrap(),
            r#"{ "one.whole": { "$eq": "x" } }"#,
        ),
        (
            holder.one.fields.eq(text()).unwrap(),
            r#"{ "one.fields": { "$eq": "x" } }"#,
        ),
        (
            holder.maybe.present.eq(text()).unwrap(),
            r#"{ "maybe.present": { "$eq": "x" } }"#,
        ),
        (
            holder.maybe.whole.eq(text()).unwrap(),
            r#"{ "maybe.whole": { "$eq": "x" } }"#,
        ),
        (
            holder.many.fields.eq(text()).unwrap(),
            r#"{ "many.fields": { "$eq": "x" } }"#,
        ),
        (
            holder.many.path.eq(text()).unwrap(),
            r#"{ "many.path": { "$eq": "x" } }"#,
        ),
        (
            holder.one.write.eq(text()).unwrap(),
            r#"{ "one.write": { "$eq": "x" } }"#,
        ),
        (
            holder.one.root.eq(text()).unwrap(),
            r#"{ "one.root": { "$eq": "x" } }"#,
        ),
    ] {
        assert_eq!(shown(written), expected);
    }
}

/// The path kinds' own operators stay where they are beside a nested model of those names.
#[test]
fn a_path_kind_keeps_its_own_operators_beside_fields_of_those_names() {
    let holder = Holder::MONGO_FIELDS;
    assert_eq!(
        shown(holder.maybe.exists(false)),
        r#"{ "maybe": { "$exists": false } }"#
    );
    assert_eq!(
        shown(holder.many.size(2)),
        r#"{ "many": { "$size": Int64(2) } }"#
    );
    assert_eq!(
        shown(holder.one.root.set(text()).unwrap()),
        r#"{ "$set": { "one.root": "x" } }"#
    );
}
