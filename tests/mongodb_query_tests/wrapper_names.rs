//! `Model`, `OptionalModel` and `ModelList` reach the nested model's paths by dereferencing, and
//! keep fields of their own. A nested model may declare fields of those very names: this is that
//! model, read from outside the module that declares the kinds.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::{Keys, shown};

/// The paths of a [`Holder`] that is the row itself.
const HOLDER: HolderPaths<Holder> = holder_paths(holder_schema::MongoPath::ROOT);

/// Holds [`Named`] every way a path kind wraps a nested model.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Holder {
    many: Vec<Named>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    maybe: Option<Named>,
    one: Named,
}

/// The paths of a [`Holder`], under whatever leads to it in a row of `Root`.
struct HolderPaths<Root> {
    many: holder_schema::ModelList<Root, Named, NamedPaths<Root>>,
    maybe: holder_schema::OptionalModel<Root, Named, NamedPaths<Root>>,
    one: holder_schema::Model<Root, Named, NamedPaths<Root>>,
}

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

/// The paths of a [`Named`], under whatever leads to it in a row of `Root`.
struct NamedPaths<Root> {
    fields: named_schema::Field<Root, String>,
    path: named_schema::Field<Root, String>,
    present: named_schema::Field<Root, String>,
    root: named_schema::Field<Root, String>,
    whole: named_schema::Field<Root, String>,
    write: named_schema::Field<Root, String>,
}

const fn holder_paths<Root>(prefix: Keys) -> HolderPaths<Root> {
    HolderPaths {
        many: holder_schema::ModelList::plain(
            holder_schema::MongoPath::under(prefix, "many"),
            named_paths(holder_schema::MongoPath::under(prefix, "many").segments),
        ),
        maybe: holder_schema::OptionalModel::plain(
            holder_schema::MongoPath::under(prefix, "maybe"),
            named_paths(holder_schema::MongoPath::under(prefix, "maybe").segments),
        ),
        one: holder_schema::Model::plain(
            holder_schema::MongoPath::under(prefix, "one"),
            named_paths(holder_schema::MongoPath::under(prefix, "one").segments),
        ),
    }
}

const fn named_paths<Root>(prefix: Keys) -> NamedPaths<Root> {
    NamedPaths {
        fields: named_schema::Field::plain(named_schema::MongoPath::under(prefix, "fields")),
        path: named_schema::Field::plain(named_schema::MongoPath::under(prefix, "path")),
        present: named_schema::Field::plain(named_schema::MongoPath::under(prefix, "present")),
        root: named_schema::Field::plain(named_schema::MongoPath::under(prefix, "root")),
        whole: named_schema::Field::plain(named_schema::MongoPath::under(prefix, "whole")),
        write: named_schema::Field::plain(named_schema::MongoPath::under(prefix, "write")),
    }
}

fn text() -> String {
    "x".to_owned()
}

#[test]
fn a_nested_field_named_like_a_path_kinds_own_is_reached_and_writes_its_key() {
    let holder = HOLDER;
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
    let holder = HOLDER;
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
