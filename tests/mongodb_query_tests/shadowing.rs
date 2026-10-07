//! A flagged type whose fields name types their author called `Element`, `Field`, `Filter`,
//! `Model` and `Update`. Under `mongodb` the flag adds types of those names to `{type}_schema`,
//! and what that module already held goes on reading the author's.

use bson::doc;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::shown;

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

/// The author's `Filter` is the value a path over it takes, and the `Filter` the flag added is
/// what the operator answers with. The same holds of the author's `Update`.
#[test]
fn the_authors_type_and_the_added_one_of_its_name_are_two_types() {
    use search_schema::MongoPath;

    let filter: search_schema::Field<Search, Filter> =
        search_schema::Field::plain(MongoPath::under(MongoPath::ROOT, "filter"));
    let matching: search_schema::Filter<Search> = filter.eq(search().filter).unwrap();
    assert_eq!(
        shown(matching),
        r#"{ "filter": { "$eq": { "name": "c" } } }"#
    );
    let update: search_schema::Field<Search, Update> =
        search_schema::Field::plain(MongoPath::under(MongoPath::ROOT, "update"));
    let changed: search_schema::Update<Search> = update.set(search().update).unwrap();
    assert_eq!(
        shown(changed),
        r#"{ "$set": { "update": { "name": "e" } } }"#
    );
}
