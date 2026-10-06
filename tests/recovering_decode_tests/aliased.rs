//! `from_value_with` on a type with a field typed by an alias. An alias `#[model_schema]` was
//! written on above is walked as the type it names, so an issue sits at the entry or the item it
//! is in. An alias tixschema never sees names a type with no walker, which is read whole.
//!
//! The aliases are declared in a module of their own, and nothing they are written with is in
//! scope where the fields are: the walker reaches what an alias holds from the field's own type.

mod declared;

use core::any::TypeId;
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
use core::time::Duration;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use tixschema::model_schema;

use self::declared::{Counts, Marks, Versions};
#[cfg(feature = "jsonschema")]
use self::declared::{counts_schema, marks_schema, versions_schema};
#[cfg(feature = "jsonschema")]
use self::properties_data_schema as properties_schema;
use super::lines;

/// What `from_value_with` lists for `$stored`, read as `$model` by a decider that rejects it.
macro_rules! listed {
    ($model:ty, $module:ident, $stored:expr) => {
        lines(
            &<$model>::from_value_with($stored, |_raw, _found| $module::Verdict::Reject)
                .unwrap_err(),
        )
    };
}

/// What `from_value_with` reads `$stored` as, by a decider that counts each run of its own into
/// `$calls`.
macro_rules! read_counting {
    ($model:ty, $module:ident, $stored:expr, $calls:ident) => {
        <$model>::from_value_with($stored, |_raw, _found| {
            $calls += 1;
            $module::Verdict::Reject
        })
    };
}

/// A plain alias of a JSON map, which no `#[model_schema]` is written on.
pub type Properties = Map<String, Value>;

/// An alias that takes a parameter: what it names depends on what fills it.
#[model_schema(default_types(T = i32))]
type Pairs<T> = HashMap<String, T>;

/// What publishes `Properties` to the schema surfaces, under its name.
#[model_schema(name = "Properties")]
pub type PropertiesData = HashMap<String, String>;

/// A map behind a plain alias, flattened: it takes every key nothing else declares.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Block {
    name: String,
    #[serde(flatten)]
    properties: Properties,
}

/// Fields typed with what is declared below this type: tixschema has seen none of it yet.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Early {
    counts: LaterCounts,
    flagged: LaterFlagged,
    pairs: Pairs<i32>,
    plain: LaterPlain,
}

#[model_schema()]
type LaterCounts = HashMap<String, i32>;

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct LaterFlagged {
    number: i32,
}

#[model_schema()]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct LaterPlain {
    number: i32,
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

/// A type of another crate. A schema surface refuses the field for a reason of its own, so the
/// type is declared where the build has none.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Timed {
    timeout: Duration,
}

#[test]
fn what_serde_wrote_through_an_alias_is_read_and_the_decider_never_runs() {
    let mut calls = 0_u32;
    let tally = Tally {
        counts: HashMap::from([("a".to_owned(), 1_i32)]),
        marks: Marks::from([("k".to_owned(), vec![declared::Mark { weight: 2_i32 }])]),
    };
    let written = serde_json::to_value(&tally).unwrap();
    assert_eq!(
        written,
        json!({ "counts": { "a": 1_i32 }, "marks": { "k": [{ "weight": 2_i32 }] } })
    );
    assert_eq!(
        read_counting!(Tally, tally_schema, written, calls),
        Ok(tally)
    );
    let empty = json!({ "counts": {}, "marks": {} });
    read_counting!(Tally, tally_schema, empty, calls).unwrap();
    let stock = json!({ "all": [{ "number": 1_i32 }] });
    read_counting!(Stock, stock_schema, stock, calls).unwrap();
    assert_eq!(calls, 0);
}

/// The issue sits at the entry it is in, and names what that entry holds.
#[test]
fn an_alias_seen_above_is_walked_as_the_map_it_names() {
    let stored = json!({ "counts": { "a": "x", "b": 2_i32 }, "marks": {} });
    serde_json::from_value::<Tally>(stored.clone()).unwrap_err();
    assert_eq!(
        listed!(Tally, tally_schema, stored),
        [
            "counts.a: invalid: expected I32, found String(\"x\"): invalid type: string \"x\", expected i32"
        ]
    );
    assert_eq!(
        listed!(Tally, tally_schema, json!({ "counts": 5_i32, "marks": {} })),
        ["counts: invalid: expected Map(I32), found Number(5): not an object"]
    );
}

/// A model type the alias holds is reached from the field's type and walked by its own walker.
#[test]
fn a_model_type_inside_an_alias_is_walked_by_its_own_walker() {
    let stored = json!({
        "counts": {},
        "marks": { "k": [{ "weight": 1_i32 }, { "extra": true, "weight": "w" }] },
    });
    assert_eq!(
        listed!(Tally, tally_schema, stored),
        [
            "marks.k[1].weight: invalid: expected I32, found String(\"w\"): invalid type: string \"w\", expected i32",
            "marks.k[1].extra: unknown: found Bool(true)",
        ]
    );
    assert_eq!(
        listed!(
            Stock,
            stock_schema,
            json!({ "all": [{ "number": 1_i32 }, { "number": "2" }] })
        ),
        [
            "all[1].number: invalid: expected I32, found String(\"2\"): invalid type: string \"2\", expected i32"
        ]
    );
}

/// serde reads a JSON map from any object, so nothing in one is an issue, and a value that is no
/// object is the one issue, at the field.
#[test]
fn an_alias_tixschema_never_sees_is_read_whole() {
    let mut calls = 0_u32;
    let any = json!({ "properties": { "a": 5_i32, "b": [true, { "c": null }] } });
    read_counting!(Page, page_schema, any, calls).unwrap();
    read_counting!(Page, page_schema, json!({ "properties": {} }), calls).unwrap();
    assert_eq!(calls, 0);
    assert_eq!(
        listed!(Page, page_schema, json!({ "properties": 5_i32 })),
        [
            "properties: invalid: expected Unknown, found Number(5): invalid type: integer `5`, expected a map"
        ]
    );
}

/// Flattened, the map takes every key nothing else declares, so none of them is unknown.
#[test]
fn a_flattened_alias_tixschema_never_sees_takes_every_other_key() {
    let mut calls = 0_u32;
    let stored = json!({ "a": 1_i32, "b": [true], "name": "n" });
    assert_eq!(
        read_counting!(Block, block_schema, stored, calls),
        Ok(Block {
            name: "n".to_owned(),
            properties: Map::from_iter([
                ("a".to_owned(), json!(1_i32)),
                ("b".to_owned(), json!([true])),
            ]),
        })
    );
    assert_eq!(calls, 0);
    assert_eq!(
        listed!(Block, block_schema, json!({ "a": 1_i32, "name": 7_i32 })),
        [
            "name: invalid: expected String, found Number(7): invalid type: integer `7`, expected a string"
        ]
    );
}

/// The flag changes nothing a schema surface publishes: the field keeps the alias's name.
#[cfg(feature = "typescript")]
#[test]
fn a_field_typed_with_an_alias_still_publishes_under_the_aliass_name() {
    let tally = Tally::ts_definition();
    assert!(tally.contains("  counts: CountsType;"), "got: {tally}");
    assert!(tally.contains("  marks: MarksType;"), "got: {tally}");
    let page = Page::ts_definition();
    assert!(page.contains("  properties: Properties;"), "got: {page}");
}

/// What is published under the alias's name is a type of its own: tixschema never sees the one
/// the field is typed with.
#[test]
fn the_type_published_for_the_alias_is_not_the_one_the_field_holds() {
    assert_ne!(TypeId::of::<PropertiesData>(), TypeId::of::<Properties>());
}

/// A name tixschema has not seen above the type is asked for its walker. A flagged model type
/// answers with its own, whatever the order. An alias declared below, an alias that takes a
/// parameter and a model type with no flag declared below have none, and are read whole.
#[test]
fn a_type_declared_below_is_walked_by_its_own_walker_or_read_whole() {
    let stored = json!({
        "counts": { "a": "x" },
        "flagged": { "extra": true, "number": "n" },
        "pairs": { "b": "y" },
        "plain": { "extra": true, "number": "n" },
    });
    assert_eq!(
        listed!(Early, early_schema, stored),
        [
            "counts: invalid: expected Unknown, found Object {\"a\": String(\"x\")}: invalid type: string \"x\", expected i32",
            "flagged.number: invalid: expected I32, found String(\"n\"): invalid type: string \"n\", expected i32",
            "flagged.extra: unknown: found Bool(true)",
            "pairs: invalid: expected Unknown, found Object {\"b\": String(\"y\")}: invalid type: string \"y\", expected i32",
            "plain: invalid: expected Unknown, found Object {\"extra\": Bool(true), \"number\": String(\"n\")}: invalid type: string \"n\", expected i32",
        ]
    );
    let mut calls = 0_u32;
    let whole = json!({
        "counts": { "a": 1_i32 },
        "flagged": { "number": 2_i32 },
        "pairs": { "b": 3_i32 },
        "plain": { "number": 4_i32 },
    });
    read_counting!(Early, early_schema, whole, calls).unwrap();
    assert_eq!(calls, 0);
}

/// A type of another crate has no walker either, and is read whole.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[test]
fn a_type_of_another_crate_is_read_whole() {
    let mut calls = 0_u32;
    let whole = json!({ "timeout": { "nanos": 0_i32, "secs": 5_i32 } });
    assert_eq!(
        read_counting!(Timed, timed_schema, whole, calls),
        Ok(Timed {
            timeout: Duration::from_secs(5),
        })
    );
    assert_eq!(calls, 0);
    assert_eq!(
        listed!(Timed, timed_schema, json!({ "timeout": 5_i32 })),
        [
            "timeout: invalid: expected Unknown, found Number(5): invalid type: integer `5`, expected struct Duration"
        ]
    );
}
