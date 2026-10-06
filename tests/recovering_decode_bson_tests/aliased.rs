//! `from_bson_with` on a type with a field typed by an alias. An alias `#[model_schema]` was
//! written on above is walked as the type it names, so an issue sits at the entry or the item it
//! is in. An alias tixschema never sees names a type with no walker, which is read whole.
//!
//! The aliases are declared in a module of their own, and nothing they are written with is in
//! scope where the fields are: the walker reaches what an alias holds from the field's own type.

mod declared;

use core::any::TypeId;
use std::collections::HashMap;

use bson::{Bson, doc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use self::declared::{Counts, Marks, Versions};
#[cfg(feature = "jsonschema")]
use self::declared::{counts_schema, marks_schema, versions_schema};
#[cfg(feature = "jsonschema")]
use self::properties_data_schema as properties_schema;
use super::{Told, invalid, string, unknown, written};

/// What `from_bson_with` lists for `$stored`, less each `reason`, read as `$model` by a decider
/// that rejects it.
macro_rules! listed {
    ($model:ty, $module:ident, $stored:expr) => {
        <$model>::from_bson_with($stored, |_raw, _found| $module::Verdict::Reject)
            .unwrap_err()
            .issues
            .iter()
            .map(|issue| match issue {
                $module::Issue::Invalid {
                    path,
                    expected,
                    found,
                    reason: _reason,
                } => (
                    "Invalid",
                    path.to_string(),
                    format!("{expected:?}"),
                    Some(found.clone()),
                ),
                $module::Issue::Unknown { path, found } => (
                    "Unknown",
                    path.to_string(),
                    String::new(),
                    Some(found.clone()),
                ),
                $module::Issue::Missing { .. }
                | $module::Issue::Mistyped { .. }
                | $module::Issue::NoVariant { .. }
                | $module::Issue::Undescribed { .. } => {
                    ("Other", String::new(), String::new(), None)
                }
            })
            .collect::<Vec<Told>>()
    };
}

/// What `from_bson_with` reads `$stored` as, by a decider that counts each run of its own into
/// `$calls`.
macro_rules! read_counting {
    ($model:ty, $module:ident, $stored:expr, $calls:ident) => {
        <$model>::from_bson_with($stored, |_raw, _found| {
            $calls += 1;
            $module::Verdict::Reject
        })
    };
}

/// A plain alias of a document, which no `#[model_schema]` is written on.
pub type Properties = bson::Document;

/// What publishes `Properties` to the schema surfaces, under its name.
#[model_schema(name = "Properties")]
pub type PropertiesData = HashMap<String, String>;

/// A document behind a plain alias, flattened: it takes every key nothing else declares.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Block {
    name: String,
    #[serde(flatten)]
    properties: Properties,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Page {
    properties: Properties,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Stock {
    all: Versions,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
struct Tally {
    counts: Counts,
    marks: Marks,
}

#[test]
fn what_serde_wrote_through_an_alias_is_read_and_the_decider_never_runs() {
    let mut calls = 0_u32;
    let tally = Tally {
        counts: HashMap::from([("a".to_owned(), 1_i32)]),
        marks: Marks::from([("k".to_owned(), vec![declared::Mark { weight: 2_i32 }])]),
    };
    let stored_row = written(&tally);
    assert_eq!(
        stored_row,
        doc! { "counts": { "a": 1_i32 }, "marks": { "k": [{ "weight": 2_i32 }] } }
    );
    assert_eq!(
        read_counting!(Tally, tally_schema, stored_row, calls),
        Ok(tally)
    );
    let empty = doc! { "counts": {}, "marks": {} };
    read_counting!(Tally, tally_schema, empty, calls).unwrap();
    let stock = doc! { "all": [{ "number": 1_i32 }] };
    read_counting!(Stock, stock_schema, stock, calls).unwrap();
    assert_eq!(calls, 0);
}

/// The issue sits at the entry it is in, and names what that entry holds.
#[test]
fn an_alias_seen_above_is_walked_as_the_map_it_names() {
    assert_eq!(
        listed!(
            Tally,
            tally_schema,
            doc! { "counts": { "a": "x", "b": 2_i32 }, "marks": {} }
        ),
        [invalid("counts.a", "I32", string("x"))]
    );
    assert_eq!(
        listed!(Tally, tally_schema, doc! { "counts": 5_i32, "marks": {} }),
        [invalid("counts", "Map(I32)", Bson::Int32(5_i32))]
    );
}

/// A model type the alias holds is reached from the field's type and walked by its own walker.
#[test]
fn a_model_type_inside_an_alias_is_walked_by_its_own_walker() {
    let stored_row = doc! {
        "counts": {},
        "marks": { "k": [{ "weight": 1_i32 }, { "extra": true, "weight": "w" }] },
    };
    assert_eq!(
        listed!(Tally, tally_schema, stored_row),
        [
            invalid("marks.k[1].weight", "I32", string("w")),
            unknown("marks.k[1].extra", Bson::Boolean(true)),
        ]
    );
    assert_eq!(
        listed!(
            Stock,
            stock_schema,
            doc! { "all": [{ "number": 1_i32 }, { "number": "2" }] }
        ),
        [invalid("all[1].number", "I32", string("2"))]
    );
}

/// serde reads a document from any document, so nothing in one is an issue, and a value that is
/// no document is the one issue, at the field.
#[test]
fn an_alias_tixschema_never_sees_is_read_whole() {
    let mut calls = 0_u32;
    let any = doc! { "properties": { "a": 5_i32, "b": [true, { "c": Bson::Null }] } };
    read_counting!(Page, page_schema, any, calls).unwrap();
    read_counting!(Page, page_schema, doc! { "properties": {} }, calls).unwrap();
    assert_eq!(calls, 0);
    assert_eq!(
        listed!(Page, page_schema, doc! { "properties": 5_i32 }),
        [invalid("properties", "Unknown", Bson::Int32(5_i32))]
    );
}

/// Flattened, the document takes every key nothing else declares, so none of them is unknown.
#[test]
fn a_flattened_alias_tixschema_never_sees_takes_every_other_key() {
    let mut calls = 0_u32;
    let stored_row = doc! { "name": "n", "a": 1_i32, "b": [true] };
    assert_eq!(
        read_counting!(Block, block_schema, stored_row, calls),
        Ok(Block {
            name: "n".to_owned(),
            properties: doc! { "a": 1_i32, "b": [true] },
        })
    );
    assert_eq!(calls, 0);
    assert_eq!(
        listed!(Block, block_schema, doc! { "name": 7_i32, "a": 1_i32 }),
        [invalid("name", "String", Bson::Int32(7_i32))]
    );
}

/// What is published under the alias's name is a type of its own: tixschema never sees the one
/// the field is typed with.
#[test]
fn the_type_published_for_the_alias_is_not_the_one_the_field_holds() {
    assert_ne!(TypeId::of::<PropertiesData>(), TypeId::of::<Properties>());
}
