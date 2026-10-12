//! A flagged type whose fields name types their author called `Element`, `Field`, `Filter`,
//! `Model` and `Update`, one whose fields name `AsFilter`, `AsUpdate`, `Nested` and `Nests`, and
//! an enum under a tag whose arms hold types called `Arm` and `ArmFilter`.
//! Under `mongodb` the flag adds types of those names to `{type}_schema`, and what that module
//! already held goes on reading the author's, as the struct of paths does.
//! A type parameter of one of those names is the parameter wherever the struct of paths names
//! it.

use bson::doc;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::shown;

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct Arm {
    name: String,
}

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct ArmFilter {
    name: String,
}

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct AsFilter {
    name: String,
}

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct AsUpdate {
    name: String,
}

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct Element {
    name: String,
}

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct Field {
    name: String,
}

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct Filter {
    name: String,
}

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct Model {
    name: String,
}

/// An enum under a tag whose arms hold a type named as each thing the flag adds for an arm.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum Limb {
    Lower(ArmFilter),
    Upper(Arm),
}

/// Holds a type named as each thing `nest` is declared with.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct Marked {
    as_filter: AsFilter,
    as_update: AsUpdate,
    nested: Nested,
    nests: Nests,
}

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct Nested {
    name: String,
}

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct Nests {
    name: String,
}

/// An enum whose parameter is named as the filter its `is_{variant}` methods answer.
#[model_schema(decode_with, default_types(Filter = String))]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum Kept<Filter> {
    Gone,
    Held { by: Filter },
}

/// A type whose parameters are named as two of the path kinds its members are.
#[model_schema(decode_with, default_types(Field = String, Model = String))]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct Paged<Field, Model> {
    items: Vec<Model>,
    lead: Field,
}

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct Search {
    element: Element,
    field: Field,
    filter: Filter,
    model: Model,
    update: Update,
}

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct Update {
    name: String,
}

fn named(name: &str) -> String {
    name.to_owned()
}

fn search() -> Search {
    Search {
        element: Element { name: named("a") },
        field: Field { name: named("b") },
        filter: Filter { name: named("c") },
        model: Model { name: named("d") },
        update: Update { name: named("e") },
    }
}

#[test]
fn a_type_named_like_a_query_type_is_read_as_the_authors() {
    let stored = doc! {
        "element": { "name": "a" },
        "field": { "name": "b" },
        "filter": { "name": "c" },
        "model": { "name": "d" },
        "update": { "name": "e" },
    };
    assert_eq!(Search::from_bson_piped(stored, &[]), Ok(search()));
}

#[test]
fn the_authors_type_and_the_added_one_of_its_name_are_two_types() {
    let paths = Search::MONGO_FIELDS;
    let matching: search_schema::Filter<Search> = paths.filter().eq(search().filter).unwrap();
    assert_eq!(
        shown(matching),
        r#"{ "filter": { "$eq": { "name": "c" } } }"#
    );
    let changed: search_schema::Update<Search> = paths.update().set(search().update).unwrap();
    assert_eq!(
        shown(changed),
        r#"{ "$set": { "update": { "name": "e" } } }"#
    );
    assert_eq!(
        shown(
            paths
                .field()
                .nest(Field::MONGO_FIELDS.name().eq(named("b")).unwrap())
        ),
        r#"{ "field.name": { "$eq": "b" } }"#
    );
}

#[test]
fn a_parameter_named_like_a_query_type_is_read_as_the_parameter() {
    let paged = Paged::<u32, String>::MONGO_FIELDS;
    assert_eq!(
        shown(paged.lead().gt(3_u32).unwrap()),
        r#"{ "lead": { "$gt": Int64(3) } }"#
    );
    assert_eq!(
        shown(paged.items().contains(named("a")).unwrap()),
        r#"{ "items": { "$eq": "a" } }"#
    );
    let kept = Kept::<u32>::MONGO_FIELDS;
    assert_eq!(shown(kept.is_gone()), r#"{ "kind": { "$eq": "gone" } }"#);
    assert_eq!(
        shown(kept.held().by().eq(1_u32).unwrap()),
        r#"{ "by": { "$eq": Int64(1) } }"#
    );
}

#[test]
fn a_type_named_like_what_nest_is_declared_with_is_read_as_the_authors() {
    let stored = doc! {
        "as_filter": { "name": "a" },
        "as_update": { "name": "b" },
        "nested": { "name": "c" },
        "nests": { "name": "d" },
    };
    let marked = Marked {
        as_filter: AsFilter { name: named("a") },
        as_update: AsUpdate { name: named("b") },
        nested: Nested { name: named("c") },
        nests: Nests { name: named("d") },
    };
    assert_eq!(Marked::from_bson_piped(stored, &[]), Ok(marked.clone()));
    let paths = Marked::MONGO_FIELDS;
    assert_eq!(
        shown(
            paths
                .nested()
                .nest(Nested::MONGO_FIELDS.name().eq(named("c")).unwrap())
        ),
        r#"{ "nested.name": { "$eq": "c" } }"#
    );
    assert_eq!(
        shown(
            paths
                .as_update()
                .nest(AsUpdate::MONGO_FIELDS.name().set(named("z")).unwrap())
        ),
        r#"{ "$set": { "as_update.name": "z" } }"#
    );
    assert_eq!(
        shown(paths.nests().eq(marked.nests).unwrap()),
        r#"{ "nests": { "$eq": { "name": "d" } } }"#
    );
    assert_eq!(
        shown(paths.as_filter().set(marked.as_filter).unwrap()),
        r#"{ "$set": { "as_filter": { "name": "a" } } }"#
    );
}

/// The arm function names the author's type, and the module's own `Arm` lists the variants.
#[test]
fn a_type_named_like_what_an_arm_is_declared_with_is_read_as_the_authors() {
    let stored = doc! { "kind": "upper", "name": "left" };
    let left = Limb::Upper(Arm {
        name: named("left"),
    });
    assert_eq!(Limb::from_bson_piped(stored, &[]), Ok(left));
    let upper: limb_schema::ArmFilter<Limb, Arm> = Limb::MONGO_FIELDS.kind__upper();
    assert_eq!(
        shown(upper.and(Arm::MONGO_FIELDS.name().eq(named("left")).unwrap())),
        r#"{ "$and": [{ "kind": { "$eq": "upper" } }, { "name": { "$eq": "left" } }] }"#
    );
    let lower: limb_schema::ArmFilter<Limb, ArmFilter> = Limb::MONGO_FIELDS.kind__lower();
    assert_eq!(
        shown(lower.and(ArmFilter::MONGO_FIELDS.name().eq(named("right")).unwrap())),
        r#"{ "$and": [{ "kind": { "$eq": "lower" } }, { "name": { "$eq": "right" } }] }"#
    );
    assert_eq!(
        format!("{:?}", [limb_schema::Arm::Lower, limb_schema::Arm::Upper]),
        "[Lower, Upper]"
    );
}
