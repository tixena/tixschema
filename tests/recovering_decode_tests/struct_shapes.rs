//! `from_value_with` on the structs serde does not write as an object of their fields: a tuple
//! struct, a single-slot one, a brand, a `transparent` struct with a named field or with several
//! slots, and a unit struct, and on a tuple wherever a field's type holds one. Each is walked in
//! the form serde writes it.

#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
use core::marker::PhantomData;
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

// The JSON schema of a type holding a `Version` names its module from here, beside the type.
#[cfg(feature = "jsonschema")]
use super::version_schema;
use super::{Version, lines};

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Pair(String, u32);

/// A tuple struct, and a tuple as a field, in a list and under an `Option`.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Placement {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    maybe: Option<(String, u32)>,
    pair: Pair,
    spot: (String, u32),
    spots: Vec<(String, u32)>,
}

/// A map of tuples, which no schema surface describes.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Keyed {
    by_name: HashMap<String, (String, u32)>,
}

/// A slot serde reads the absence of, one it never reads, and one that holds a model type.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Loose(
    String,
    #[serde(skip)] u8,
    Version,
    #[serde(default)] Vec<u32>,
);

/// A brand: serde writes it as the text it holds.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct RecordId(String);

/// A brand over another brand: serde writes it as the text the inner one holds.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct OwnerRef(RecordId);

/// A brand over a tuple struct: serde writes it as the array the tuple struct is.
#[model_schema(decode_with, no_display)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct PairRef(Pair);

/// A brand over another brand and one over a tuple struct, each under a key.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Referenced {
    owner: OwnerRef,
    pair: PairRef,
}

/// A flattened brand over text, which serde refuses to flatten at every read. A schema surface
/// refuses the declaration.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Carded {
    #[serde(flatten)]
    id: RecordId,
    name: String,
}

/// A flattened tuple struct, which serde refuses to flatten as it refuses a brand over text.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Paired {
    name: String,
    #[serde(flatten)]
    pair: Pair,
}

/// A single-slot tuple struct with no `transparent`: serde writes it as a brand is written.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Wrapper(String);

/// A brand whose own reader refuses text shorter than three characters. Only a schema surface
/// hangs that check, and the declaration is refused in a build with none.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[model_schema(decode_with, minLength = 3)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct Code(String);

/// A brand over a model type.
#[model_schema(decode_with, no_display)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct Pinned(Version);

/// A single-slot tuple struct over a model type, with no `transparent`.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Kept(Version);

/// A brand over a list of model types.
#[model_schema(decode_with, no_display)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct History(Vec<Version>);

/// A single-slot tuple struct over an optional model type.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Latest(Option<Version>);

/// A `transparent` struct with a named field: serde writes it as the text the field holds.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct Slug {
    text: String,
}

/// A `transparent` struct with a named field over a model type.
#[model_schema(decode_with, no_display)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct Current {
    version: Version,
}

/// A `transparent` struct with a named field over a list of model types.
#[model_schema(decode_with, no_display)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct Revisions {
    versions: Vec<Version>,
}

/// A `transparent` struct beside whose field is one serde neither writes nor reads.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct Cached {
    #[serde(skip)]
    hits: u32,
    text: String,
}

/// A `transparent` struct whose field is read and written through a hook: a number as text.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct Counted {
    #[serde(with = "super::as_text")]
    count: u32,
}

/// The fields serde's derive passes over without a `skip` when it picks the one a `transparent`
/// struct is written as: one with a `default`, and a `PhantomData`, which no schema surface
/// describes.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct Etched {
    #[serde(default, skip_serializing)]
    hits: u32,
    marker: PhantomData<u8>,
    text: String,
}

/// A `transparent` tuple struct beside whose value is a slot serde neither writes nor reads.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct Tags(Vec<String>, #[serde(skip)] u8);

/// A `transparent` tuple struct whose value is its second slot.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct Measured(#[serde(skip)] u8, String);

/// A `transparent` tuple struct with a slot serde never reads, over a model type.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct Stamped(Version, #[serde(skip)] u8);

/// The slots serde's derive passes over without a `skip` when it picks the one a `transparent`
/// tuple struct is written as: one with a `default`, and a `PhantomData`, which no schema surface
/// describes.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
struct Marked<T>(
    PhantomData<T>,
    String,
    #[serde(default, skip_serializing)] u32,
);

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Filed {
    measured: Measured,
    stamped: Stamped,
    tags: Tags,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Titled {
    cached: Cached,
    counted: Counted,
    current: Current,
    revisions: Revisions,
    slug: Slug,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Ping;

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Labelled {
    history: History,
    id: RecordId,
    kept: Kept,
    latest: Latest,
    ping: Ping,
    pinned: Pinned,
    wrapper: Wrapper,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Entry {
    name: String,
    versions: Vec<Version>,
}

#[model_schema()]
type EntryAlias = Entry;

/// A field typed with an alias of a flagged model type.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Shelf {
    featured: EntryAlias,
}

fn placement() -> Placement {
    Placement {
        maybe: Some(("m".to_owned(), 2)),
        pair: Pair("p".to_owned(), 3),
        spot: ("s".to_owned(), 4),
        spots: vec![("t".to_owned(), 5)],
    }
}

fn labelled() -> Labelled {
    Labelled {
        history: History(vec![Version { number: 1 }]),
        id: RecordId("r-1".to_owned()),
        kept: Kept(Version { number: 2 }),
        latest: Latest(None),
        ping: Ping,
        pinned: Pinned(Version { number: 3 }),
        wrapper: Wrapper("w".to_owned()),
    }
}

fn titled() -> Titled {
    Titled {
        cached: Cached {
            hits: 0,
            text: "c".to_owned(),
        },
        counted: Counted { count: 5 },
        current: Current {
            version: Version { number: 1 },
        },
        revisions: Revisions {
            versions: vec![Version { number: 2 }],
        },
        slug: Slug {
            text: "s".to_owned(),
        },
    }
}

fn filed() -> Filed {
    Filed {
        measured: Measured(0, "m".to_owned()),
        stamped: Stamped(Version { number: 1 }, 0),
        tags: Tags(vec!["a".to_owned(), "b".to_owned()], 0),
    }
}

/// What `from_value_with` lists for `stored`, read as a `Pair` by a decider that rejects it.
fn pair_issues(stored: Value) -> Vec<String> {
    let read = Pair::from_value_with(stored, |_raw, _found| pair_schema::Verdict::Reject);
    lines(&read.unwrap_err())
}

/// What `from_value_with` lists for `stored`, read as a `Labelled` by a decider that rejects it.
fn labelled_issues(stored: Value) -> Vec<String> {
    let read = Labelled::from_value_with(stored, |_raw, _found| labelled_schema::Verdict::Reject);
    lines(&read.unwrap_err())
}

/// What `from_value_with` lists for `stored`, read as a `Titled` by a decider that rejects it.
fn titled_issues(stored: Value) -> Vec<String> {
    let read = Titled::from_value_with(stored, |_raw, _found| titled_schema::Verdict::Reject);
    lines(&read.unwrap_err())
}

/// What `from_value_with` lists for `stored`, read as a `Filed` by a decider that rejects it.
fn filed_issues(stored: Value) -> Vec<String> {
    let read = Filed::from_value_with(stored, |_raw, _found| filed_schema::Verdict::Reject);
    lines(&read.unwrap_err())
}

#[test]
fn what_serde_wrote_for_each_shape_is_read_and_the_decider_never_runs() {
    let mut calls = 0_u32;
    let stored = serde_json::to_value(placement()).unwrap();
    assert_eq!(stored.get("pair"), Some(&json!(["p", 3_i32])));
    assert_eq!(stored.get("spot"), Some(&json!(["s", 4_i32])));
    let placed = Placement::from_value_with(stored, |_raw, _found| {
        calls += 1;
        placement_schema::Verdict::Reject
    });
    assert_eq!(placed, Ok(placement()));

    let written = serde_json::to_value(labelled()).unwrap();
    assert_eq!(
        written,
        json!({
            "history": [{ "number": 1_i32 }],
            "id": "r-1",
            "kept": { "number": 2_i32 },
            "latest": null,
            "ping": {},
            "pinned": { "number": 3_i32 },
            "wrapper": "w",
        })
    );
    let read = Labelled::from_value_with(written, |_raw, _found| {
        calls += 1;
        labelled_schema::Verdict::Reject
    });
    assert_eq!(read, Ok(labelled()));
    assert_eq!(calls, 0);
}

#[test]
fn a_tuple_struct_is_walked_by_position() {
    assert_eq!(
        pair_issues(json!(["a", "x"])),
        [
            "[1]: invalid: expected U32, found String(\"x\"): invalid type: string \"x\", expected u32"
        ]
    );
    assert_eq!(
        Pair::from_value_with(json!(["a", 7_i32]), |_raw, _found| {
            pair_schema::Verdict::Reject
        }),
        Ok(Pair("a".to_owned(), 7))
    );
}

#[test]
fn a_position_that_is_absent_is_missing() {
    assert_eq!(pair_issues(json!(["a"])), ["[1]: missing: expected U32"]);
    assert_eq!(
        pair_issues(json!([])),
        [
            "[0]: missing: expected String",
            "[1]: missing: expected U32"
        ]
    );
}

#[test]
fn a_position_the_type_does_not_declare_is_unknown_beside_serdes_message() {
    assert_eq!(
        pair_issues(json!(["a", 1_i32, true])),
        [
            "[2]: unknown: found Bool(true)",
            "undescribed: invalid length 3, expected fewer elements in array",
        ]
    );
}

#[test]
fn a_decider_repairs_a_tuple_struct_by_the_position_it_is_handed() {
    let read = Pair::from_value_with(json!(["a", "7", true]), |raw, found| {
        for issue in found {
            if let pair_schema::Issue::Invalid {
                path,
                expected: pair_schema::Expected::U32,
                found: _found,
                reason: _reason,
            } = issue
            {
                path.set_in_value(raw, json!(7_i32));
            } else if let pair_schema::Issue::Unknown {
                path,
                found: _found,
            } = issue
            {
                path.remove_from_value(raw);
            } else {
                return pair_schema::Verdict::Reject;
            }
        }
        pair_schema::Verdict::Fixed
    });
    assert_eq!(read, Ok(Pair("a".to_owned(), 7)));
}

#[test]
fn a_tuple_struct_held_as_an_object_is_listed_at_the_value_itself() {
    assert_eq!(
        pair_issues(json!({ "0": "a", "1": 7_i32 })),
        [
            "the value itself: invalid: expected Model(\"Pair\"), found Object {\"0\": String(\"a\"), \"1\": Number(7)}: invalid type: map, expected tuple struct Pair"
        ]
    );
}

#[test]
fn a_tuple_is_walked_by_position_in_a_field_a_list_and_an_option() {
    let stored = json!({
        "maybe": [true, 2_i32],
        "pair": ["a", "x"],
        "spot": [5_i32, 1_i32],
        "spots": [["t", 5_i32], [6_i32, 7_i32, 8_i32], ["u"]],
    });
    let read = Placement::from_value_with(stored, |_raw, _found| placement_schema::Verdict::Reject);
    assert_eq!(
        lines(&read.unwrap_err()),
        [
            "maybe[0]: invalid: expected String, found Bool(true): invalid type: boolean `true`, expected a string",
            "pair[1]: invalid: expected U32, found String(\"x\"): invalid type: string \"x\", expected u32",
            "spot[0]: invalid: expected String, found Number(5): invalid type: integer `5`, expected a string",
            "spots[1][0]: invalid: expected String, found Number(6): invalid type: integer `6`, expected a string",
            "spots[1][2]: unknown: found Number(8)",
            "spots[2][1]: missing: expected U32",
        ]
    );
}

#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[test]
fn a_tuple_is_walked_by_position_in_a_map() {
    let stored = json!({ "by_name": { "first": ["a", "1"], "second": 7_i32 } });
    let read = Keyed::from_value_with(stored, |_raw, _found| keyed_schema::Verdict::Reject);
    assert_eq!(
        lines(&read.unwrap_err()),
        [
            "by_name.first[1]: invalid: expected U32, found String(\"1\"): invalid type: string \"1\", expected u32",
            "by_name.second: invalid: expected Tuple([String, U32]), found Number(7): not an array",
        ]
    );
    let kept = Keyed::from_value_with(
        json!({ "by_name": { "first": ["a", 1_i32] } }),
        |_raw, _found| keyed_schema::Verdict::Reject,
    );
    assert_eq!(
        kept,
        Ok(Keyed {
            by_name: HashMap::from([("first".to_owned(), ("a".to_owned(), 1))]),
        })
    );
}

#[test]
fn a_tuple_held_as_no_array_is_one_issue_at_the_value() {
    let mut stored = serde_json::to_value(placement()).unwrap();
    stored["maybe"] = json!("none");
    stored["spot"] = json!({ "0": "s" });
    stored["spots"] = json!([7_i32]);
    let read = Placement::from_value_with(stored, |_raw, _found| placement_schema::Verdict::Reject);
    assert_eq!(
        lines(&read.unwrap_err()),
        [
            "maybe: invalid: expected Optional(Tuple([String, U32])), found String(\"none\"): not an array",
            "spot: invalid: expected Tuple([String, U32]), found Object {\"0\": String(\"s\")}: not an array",
            "spots[0]: invalid: expected Tuple([String, U32]), found Number(7): not an array",
        ]
    );
}

#[test]
fn an_optional_tuple_that_is_absent_or_null_is_no_issue() {
    for maybe in [None, Some(Value::Null)] {
        let mut stored = serde_json::to_value(placement()).unwrap();
        let fields = stored.as_object_mut().unwrap();
        fields.remove("maybe");
        if let Some(held) = maybe {
            fields.insert("maybe".to_owned(), held);
        }
        let mut calls = 0_u32;
        let read = Placement::from_value_with(stored, |_raw, _found| {
            calls += 1;
            placement_schema::Verdict::Reject
        });
        assert_eq!(read.unwrap().maybe, None);
        assert_eq!(calls, 0);
    }
}

#[test]
fn a_decider_repairs_a_tuple_by_the_position_it_is_handed() {
    let mut stored = serde_json::to_value(placement()).unwrap();
    stored["spot"] = json!([4_i32, 4_i32, "extra"]);
    let read = Placement::from_value_with(stored, |raw, found| {
        for issue in found {
            if let placement_schema::Issue::Invalid {
                path,
                expected: placement_schema::Expected::String,
                found: _found,
                reason: _reason,
            } = issue
            {
                assert_eq!(path.to_string(), "spot[0]");
                path.set_in_value(raw, json!("s"));
            } else if let placement_schema::Issue::Unknown {
                path,
                found: _found,
            } = issue
            {
                assert_eq!(path.to_string(), "spot[2]");
                path.remove_from_value(raw);
            } else {
                return placement_schema::Verdict::Reject;
            }
        }
        placement_schema::Verdict::Fixed
    });
    assert_eq!(read, Ok(placement()));
}

#[test]
fn a_slot_is_a_position_only_where_serde_reads_one() {
    let written =
        serde_json::to_value(Loose("a".to_owned(), 9, Version { number: 1 }, vec![2])).unwrap();
    assert_eq!(written, json!(["a", { "number": 1_i32 }, [2_i32]]));
    let mut calls = 0_u32;
    let read = Loose::from_value_with(json!(["a", { "number": 1_i32 }]), |_raw, _found| {
        calls += 1;
        loose_schema::Verdict::Reject
    });
    assert_eq!(
        read,
        Ok(Loose("a".to_owned(), 0, Version { number: 1 }, Vec::new()))
    );
    assert_eq!(calls, 0);

    let listed = Loose::from_value_with(
        json!(["a", { "draft": true, "number": "1" }, [2_i32, "3"]]),
        |_raw, _found| loose_schema::Verdict::Reject,
    );
    assert_eq!(
        lines(&listed.unwrap_err()),
        [
            "[1].number: invalid: expected I32, found String(\"1\"): invalid type: string \"1\", expected i32",
            "[1].draft: unknown: found Bool(true)",
            "[2][1]: invalid: expected U32, found String(\"3\"): invalid type: string \"3\", expected u32",
        ]
    );
}

#[test]
fn a_brand_and_a_single_slot_struct_are_read_as_the_value_they_hold() {
    let mut stored = serde_json::to_value(labelled()).unwrap();
    stored["id"] = json!(5_i32);
    stored["wrapper"] = json!(["w"]);
    assert_eq!(
        labelled_issues(stored),
        [
            "id: invalid: expected String, found Number(5): invalid type: integer `5`, expected a string",
            "wrapper: invalid: expected String, found Array [String(\"w\")]: invalid type: sequence, expected a string",
        ]
    );
    let read = RecordId::from_value_with(json!(true), |_raw, _found| {
        record_id_schema::Verdict::Reject
    });
    assert_eq!(
        lines(&read.unwrap_err()),
        [
            "the value itself: invalid: expected String, found Bool(true): invalid type: boolean `true`, expected a string"
        ]
    );
    assert_eq!(
        Wrapper::from_value_with(json!("w"), |_raw, _found| {
            wrapper_schema::Verdict::Reject
        }),
        Ok(Wrapper("w".to_owned()))
    );
}

#[test]
fn a_brand_over_another_brand_is_read_as_the_value_the_inner_one_holds() {
    let mut calls = 0_u32;
    let read = OwnerRef::from_value_with(json!("abc"), |_raw, _found| {
        calls += 1;
        owner_ref_schema::Verdict::Reject
    });
    assert_eq!(read.ok(), Some(OwnerRef(RecordId("abc".to_owned()))));
    assert_eq!(calls, 0);

    let refused = OwnerRef::from_value_with(json!(5_i32), |_raw, _found| {
        owner_ref_schema::Verdict::Reject
    });
    assert_eq!(
        lines(&refused.unwrap_err()),
        [
            "the value itself: invalid: expected String, found Number(5): invalid type: integer `5`, expected a string"
        ]
    );
}

#[test]
fn a_brand_over_a_tuple_struct_walks_as_that_tuple_struct() {
    let mut calls = 0_u32;
    let referenced = Referenced {
        owner: OwnerRef(RecordId("abc".to_owned())),
        pair: PairRef(Pair("a".to_owned(), 1)),
    };
    let written = serde_json::to_value(&referenced).unwrap();
    assert_eq!(written, json!({ "owner": "abc", "pair": ["a", 1_i32] }));
    let read = Referenced::from_value_with(written, |_raw, _found| {
        calls += 1;
        referenced_schema::Verdict::Reject
    });
    assert_eq!(read, Ok(referenced));
    assert_eq!(calls, 0);

    let alone = PairRef::from_value_with(json!(["a", "x"]), |_raw, _found| {
        pair_ref_schema::Verdict::Reject
    });
    assert_eq!(
        lines(&alone.unwrap_err()),
        [
            "[1]: invalid: expected U32, found String(\"x\"): invalid type: string \"x\", expected u32"
        ]
    );
    let held = Referenced::from_value_with(
        json!({ "owner": 5_i32, "pair": ["a", "x", true] }),
        |_raw, _found| referenced_schema::Verdict::Reject,
    );
    assert_eq!(
        lines(&held.unwrap_err()),
        [
            "owner: invalid: expected String, found Number(5): invalid type: integer `5`, expected a string",
            "pair[1]: invalid: expected U32, found String(\"x\"): invalid type: string \"x\", expected u32",
            "pair[2]: unknown: found Bool(true)",
        ]
    );
}

#[test]
fn a_brand_over_text_a_slot_over_a_list_and_a_tuple_struct_list_nothing_and_return_no_key() {
    let held = json!({ "0": "a", "legacy": true, "number": "3" });
    let object = held.as_object().unwrap();
    for keyed in [
        RecordId::decode_with_value_fields::<record_id_schema::Issue<Value>>,
        Wrapper::decode_with_value_fields::<record_id_schema::Issue<Value>>,
        History::decode_with_value_fields::<record_id_schema::Issue<Value>>,
        Pair::decode_with_value_fields::<record_id_schema::Issue<Value>>,
        OwnerRef::decode_with_value_fields::<record_id_schema::Issue<Value>>,
        PairRef::decode_with_value_fields::<record_id_schema::Issue<Value>>,
    ] {
        let mut out = Vec::new();
        let declared = keyed(object, &[], record_id_schema::issue_from_parts, &mut out);
        assert_eq!(declared, Vec::<&str>::new());
        assert_eq!(out, Vec::new());
    }
}

#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[test]
fn a_struct_flattening_a_type_serde_cannot_flatten_lists_nothing_for_it_beside_serdes_refusal() {
    let stored = json!({ "name": "n" });
    let refusal = Carded::deserialize(&stored).unwrap_err().to_string();
    assert_eq!(refusal, "can only flatten structs and maps");
    let carded = Carded::from_value_with(stored.clone(), |_raw, _found| {
        carded_schema::Verdict::Reject
    });
    assert_eq!(
        lines(&carded.unwrap_err()),
        [format!("undescribed: {refusal}")]
    );

    assert_eq!(
        Paired::deserialize(&stored).unwrap_err().to_string(),
        refusal
    );
    let paired = Paired::from_value_with(stored, |_raw, _found| paired_schema::Verdict::Reject);
    assert_eq!(
        lines(&paired.unwrap_err()),
        [format!("undescribed: {refusal}")]
    );
}

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn a_constrained_brand_is_refused_with_its_own_message() {
    let read = Code::from_value_with(json!("ab"), |_raw, _found| code_schema::Verdict::Reject);
    assert_eq!(
        lines(&read.unwrap_err()),
        [
            "the value itself: invalid: expected String, found String(\"ab\"): too short: minimum length is 3, got 2"
        ]
    );
    assert_eq!(
        Code::from_value_with(json!("abc"), |_raw, _found| code_schema::Verdict::Reject),
        Ok(Code("abc".to_owned()))
    );
}

#[test]
fn a_single_slot_struct_over_a_model_type_walks_as_that_type() {
    let mut stored = serde_json::to_value(labelled()).unwrap();
    stored["history"] = json!([{ "number": 1_i32 }, { "number": "2" }]);
    stored["kept"] = json!({ "number": true });
    stored["latest"] = json!({ "draft": true, "number": 4_i32 });
    stored["pinned"] = json!({ "number": "3" });
    assert_eq!(
        labelled_issues(stored),
        [
            "history[1].number: invalid: expected I32, found String(\"2\"): invalid type: string \"2\", expected i32",
            "kept.number: invalid: expected I32, found Bool(true): invalid type: boolean `true`, expected i32",
            "latest.draft: unknown: found Bool(true)",
            "pinned.number: invalid: expected I32, found String(\"3\"): invalid type: string \"3\", expected i32",
        ]
    );
    let mut unlisted = serde_json::to_value(labelled()).unwrap();
    unlisted["history"] = json!("none");
    unlisted["pinned"] = json!("three");
    assert_eq!(
        labelled_issues(unlisted),
        [
            "history: invalid: expected Array(Model(\"Version\")), found String(\"none\"): not an array",
            "pinned: invalid: expected Model(\"Version\"), found String(\"three\"): invalid type: string \"three\", expected struct Version",
        ]
    );
}

#[test]
fn a_single_slot_struct_over_a_model_type_hands_its_fields_walk_to_that_type() {
    let held = json!({ "legacy": true, "number": "3" });
    let object = held.as_object().unwrap();
    for keyed in [
        Pinned::decode_with_value_fields::<pinned_schema::Issue<Value>>,
        Kept::decode_with_value_fields::<pinned_schema::Issue<Value>>,
    ] {
        let mut out = Vec::new();
        let declared = keyed(object, &[], pinned_schema::issue_from_parts, &mut out);
        assert_eq!(declared, ["number"]);
        assert_eq!(
            lines(&pinned_schema::Unrecovered { issues: out }),
            [
                "number: invalid: expected I32, found String(\"3\"): invalid type: string \"3\", expected i32"
            ]
        );
    }
}

#[test]
fn what_serde_wrote_for_a_transparent_struct_is_read_and_the_decider_never_runs() {
    let mut calls = 0_u32;
    let slug = Slug {
        text: "x".to_owned(),
    };
    let written = serde_json::to_value(&slug).unwrap();
    assert_eq!(written, json!("x"));
    let read = Slug::from_value_with(written, |_raw, _found| {
        calls += 1;
        slug_schema::Verdict::Reject
    });
    assert_eq!(read, Ok(slug));

    let stored = serde_json::to_value(titled()).unwrap();
    assert_eq!(
        stored,
        json!({
            "cached": "c",
            "counted": "5",
            "current": { "number": 1_i32 },
            "revisions": [{ "number": 2_i32 }],
            "slug": "s",
        })
    );
    let titled_read = Titled::from_value_with(stored, |_raw, _found| {
        calls += 1;
        titled_schema::Verdict::Reject
    });
    assert_eq!(titled_read, Ok(titled()));
    assert_eq!(calls, 0);
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_transparent_structs_json_schema_admits_what_the_decode_reads() {
    assert_eq!(Slug::json_schema(), json!({ "type": "string" }));
}

/// A value serde refuses is one issue at the path the struct sits at, naming the type of its field.
#[test]
fn a_value_a_transparent_struct_refuses_is_one_issue_at_the_path_the_struct_sits_at() {
    let read = Slug::from_value_with(json!(5_i32), |_raw, _found| slug_schema::Verdict::Reject);
    assert_eq!(
        lines(&read.unwrap_err()),
        [
            "the value itself: invalid: expected String, found Number(5): invalid type: integer `5`, expected a string"
        ]
    );

    let mut stored = serde_json::to_value(titled()).unwrap();
    stored["cached"] = json!({ "text": "c" });
    stored["slug"] = json!(5_i32);
    assert_eq!(
        titled_issues(stored),
        [
            "cached: invalid: expected String, found Object {\"text\": String(\"c\")}: invalid type: map, expected a string",
            "slug: invalid: expected String, found Number(5): invalid type: integer `5`, expected a string",
        ]
    );
}

#[test]
fn a_transparent_struct_whose_field_carries_a_hook_is_read_through_it() {
    let read = Counted::from_value_with(json!("five"), |_raw, _found| {
        counted_schema::Verdict::Reject
    });
    assert_eq!(
        lines(&read.unwrap_err()),
        [
            "the value itself: invalid: expected U32, found String(\"five\"): invalid digit found in string"
        ]
    );
    let mut stored = serde_json::to_value(titled()).unwrap();
    stored["counted"] = json!(5_i32);
    assert_eq!(
        titled_issues(stored),
        [
            "counted: invalid: expected U32, found Number(5): invalid type: integer `5`, expected a string"
        ]
    );
}

#[test]
fn a_transparent_struct_over_a_model_type_walks_as_that_type() {
    let read = Current::from_value_with(json!({ "draft": true, "number": "3" }), |_raw, _found| {
        current_schema::Verdict::Reject
    });
    assert_eq!(
        lines(&read.unwrap_err()),
        [
            "number: invalid: expected I32, found String(\"3\"): invalid type: string \"3\", expected i32",
            "draft: unknown: found Bool(true)",
        ]
    );

    let mut stored = serde_json::to_value(titled()).unwrap();
    stored["current"] = json!({ "number": "3" });
    stored["revisions"] = json!([{ "number": 2_i32 }, { "number": "4" }]);
    assert_eq!(
        titled_issues(stored),
        [
            "current.number: invalid: expected I32, found String(\"3\"): invalid type: string \"3\", expected i32",
            "revisions[1].number: invalid: expected I32, found String(\"4\"): invalid type: string \"4\", expected i32",
        ]
    );
    let mut unlisted = serde_json::to_value(titled()).unwrap();
    unlisted["current"] = json!("one");
    unlisted["revisions"] = json!("none");
    assert_eq!(
        titled_issues(unlisted),
        [
            "current: invalid: expected Model(\"Version\"), found String(\"one\"): invalid type: string \"one\", expected struct Version",
            "revisions: invalid: expected Array(Model(\"Version\")), found String(\"none\"): not an array",
        ]
    );
}

#[test]
fn a_transparent_struct_over_a_model_type_hands_its_fields_walk_to_that_type() {
    let held = json!({ "legacy": true, "number": "3" });
    let mut out: Vec<current_schema::Issue<Value>> = Vec::new();
    let declared = Current::decode_with_value_fields(
        held.as_object().unwrap(),
        &[],
        current_schema::issue_from_parts,
        &mut out,
    );
    assert_eq!(declared, ["number"]);
    assert_eq!(
        lines(&current_schema::Unrecovered { issues: out }),
        [
            "number: invalid: expected I32, found String(\"3\"): invalid type: string \"3\", expected i32"
        ]
    );
}

#[test]
fn a_decider_repairs_a_transparent_struct_at_the_path_it_is_handed() {
    let mut stored = serde_json::to_value(titled()).unwrap();
    stored["current"] = json!({ "number": "1" });
    stored["slug"] = json!(5_i32);
    let read = Titled::from_value_with(stored, |raw, found| {
        for issue in found {
            if let titled_schema::Issue::Invalid {
                path,
                expected,
                found: _found,
                reason: _reason,
            } = issue
            {
                let fixed = if *expected == titled_schema::Expected::I32 {
                    assert_eq!(path.to_string(), "current.number");
                    json!(1_i32)
                } else {
                    assert_eq!(path.to_string(), "slug");
                    json!("s")
                };
                path.set_in_value(raw, fixed);
            } else {
                return titled_schema::Verdict::Reject;
            }
        }
        titled_schema::Verdict::Fixed
    });
    assert_eq!(read, Ok(titled()));
}

#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[test]
fn a_transparent_struct_is_walked_as_the_field_serde_reads_past_a_default_and_a_marker() {
    let etched = Etched {
        hits: 3,
        marker: PhantomData,
        text: "e".to_owned(),
    };
    assert_eq!(serde_json::to_value(&etched).unwrap(), json!("e"));
    let mut calls = 0_u32;
    let read = Etched::from_value_with(json!("e"), |_raw, _found| {
        calls += 1;
        etched_schema::Verdict::Reject
    });
    assert_eq!(
        read,
        Ok(Etched {
            hits: 0,
            marker: PhantomData,
            text: "e".to_owned(),
        })
    );
    assert_eq!(calls, 0);

    let refused =
        Etched::from_value_with(json!(7_i32), |_raw, _found| etched_schema::Verdict::Reject);
    assert_eq!(
        lines(&refused.unwrap_err()),
        [
            "the value itself: invalid: expected String, found Number(7): invalid type: integer `7`, expected a string"
        ]
    );
}

#[test]
fn what_serde_wrote_for_a_transparent_tuple_struct_is_read_and_the_decider_never_runs() {
    let tags = vec!["a".to_owned(), "b".to_owned(), "c".to_owned()];
    let written = serde_json::to_value(Tags(tags.clone(), 9)).unwrap();
    assert_eq!(written, json!(["a", "b", "c"]));
    let plain: Tags = serde_json::from_value(written.clone()).unwrap();
    assert_eq!(plain, Tags(tags, 0));
    let mut calls = 0_u32;
    let read = Tags::from_value_with(written, |_raw, _found| {
        calls += 1;
        tags_schema::Verdict::Reject
    });
    assert_eq!(read, Ok(plain));

    let stored = serde_json::to_value(filed()).unwrap();
    assert_eq!(
        stored,
        json!({
            "measured": "m",
            "stamped": { "number": 1_i32 },
            "tags": ["a", "b"],
        })
    );
    let filed_read = Filed::from_value_with(stored, |_raw, _found| {
        calls += 1;
        filed_schema::Verdict::Reject
    });
    assert_eq!(filed_read, Ok(filed()));
    assert_eq!(calls, 0);
}

#[test]
fn a_value_a_transparent_tuple_struct_refuses_is_one_issue_at_the_path_the_struct_sits_at() {
    serde_json::from_value::<Tags>(json!(5_i32)).unwrap_err();
    let read = Tags::from_value_with(json!(5_i32), |_raw, _found| tags_schema::Verdict::Reject);
    assert_eq!(
        lines(&read.unwrap_err()),
        ["the value itself: invalid: expected Array(String), found Number(5): not an array"]
    );

    let mut stored = serde_json::to_value(filed()).unwrap();
    stored["measured"] = json!(["m"]);
    stored["tags"] = json!(["a", 7_i32]);
    assert_eq!(
        filed_issues(stored),
        [
            "measured: invalid: expected String, found Array [String(\"m\")]: invalid type: sequence, expected a string",
            "tags[1]: invalid: expected String, found Number(7): invalid type: integer `7`, expected a string",
        ]
    );
    let mut unlisted = serde_json::to_value(filed()).unwrap();
    unlisted["tags"] = json!(5_i32);
    assert_eq!(
        filed_issues(unlisted),
        ["tags: invalid: expected Array(String), found Number(5): not an array"]
    );
}

#[test]
fn a_transparent_tuple_struct_is_read_as_its_value_wherever_the_slot_is_written() {
    let written = serde_json::to_value(Measured(9, "m".to_owned())).unwrap();
    assert_eq!(written, json!("m"));
    let plain: Measured = serde_json::from_value(written.clone()).unwrap();
    assert_eq!(plain, Measured(0, "m".to_owned()));
    let mut calls = 0_u32;
    let read = Measured::from_value_with(written, |_raw, _found| {
        calls += 1;
        measured_schema::Verdict::Reject
    });
    assert_eq!(read, Ok(plain));
    assert_eq!(calls, 0);

    serde_json::from_value::<Measured>(json!(["m"])).unwrap_err();
    let refused = Measured::from_value_with(json!(["m"]), |_raw, _found| {
        measured_schema::Verdict::Reject
    });
    assert_eq!(
        lines(&refused.unwrap_err()),
        [
            "the value itself: invalid: expected String, found Array [String(\"m\")]: invalid type: sequence, expected a string"
        ]
    );
}

#[test]
fn a_transparent_tuple_struct_over_a_model_type_walks_as_that_type() {
    let read = Stamped::from_value_with(json!({ "draft": true, "number": "3" }), |_raw, _found| {
        stamped_schema::Verdict::Reject
    });
    assert_eq!(
        lines(&read.unwrap_err()),
        [
            "number: invalid: expected I32, found String(\"3\"): invalid type: string \"3\", expected i32",
            "draft: unknown: found Bool(true)",
        ]
    );

    let mut stored = serde_json::to_value(filed()).unwrap();
    stored["stamped"] = json!({ "number": "3" });
    assert_eq!(
        filed_issues(stored),
        [
            "stamped.number: invalid: expected I32, found String(\"3\"): invalid type: string \"3\", expected i32"
        ]
    );
    let mut unlisted = serde_json::to_value(filed()).unwrap();
    unlisted["stamped"] = json!("one");
    assert_eq!(
        filed_issues(unlisted),
        [
            "stamped: invalid: expected Model(\"Version\"), found String(\"one\"): invalid type: string \"one\", expected struct Version"
        ]
    );

    let held = json!({ "legacy": true, "number": "3" });
    let mut out: Vec<stamped_schema::Issue<Value>> = Vec::new();
    let declared = Stamped::decode_with_value_fields(
        held.as_object().unwrap(),
        &[],
        stamped_schema::issue_from_parts,
        &mut out,
    );
    assert_eq!(declared, ["number"]);
    assert_eq!(
        lines(&stamped_schema::Unrecovered { issues: out }),
        [
            "number: invalid: expected I32, found String(\"3\"): invalid type: string \"3\", expected i32"
        ]
    );
}

#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[test]
fn a_transparent_tuple_struct_is_walked_as_the_slot_serde_reads_past_a_default_and_a_marker() {
    let written = serde_json::to_value(Marked::<u8>(PhantomData, "m".to_owned(), 3)).unwrap();
    assert_eq!(written, json!("m"));
    let plain: Marked<u8> = serde_json::from_value(written.clone()).unwrap();
    assert_eq!(plain, Marked(PhantomData, "m".to_owned(), 0));
    let mut calls = 0_u32;
    let read = Marked::<u8>::from_value_with(written, |_raw, _found| {
        calls += 1;
        marked_schema::Verdict::Reject
    });
    assert_eq!(read, Ok(plain));
    assert_eq!(calls, 0);

    serde_json::from_value::<Marked<u8>>(json!(7_i32)).unwrap_err();
    let refused =
        Marked::<u8>::from_value_with(json!(7_i32), |_raw, _found| marked_schema::Verdict::Reject);
    assert_eq!(
        lines(&refused.unwrap_err()),
        [
            "the value itself: invalid: expected String, found Number(7): invalid type: integer `7`, expected a string"
        ]
    );
}

/// A unit struct is the `{}` tixschema makes it write: no key in it is the type's own.
#[test]
fn a_unit_struct_is_an_object_in_which_every_key_is_unknown() {
    let read = Ping::from_value_with(json!({ "x": 1_i32 }), |raw, found| {
        for issue in found {
            if let ping_schema::Issue::Unknown {
                path,
                found: _found,
            } = issue
            {
                assert_eq!(path.to_string(), "x");
                path.remove_from_value(raw);
            }
        }
        ping_schema::Verdict::Fixed
    });
    assert_eq!(read, Ok(Ping));

    let mut stored = serde_json::to_value(labelled()).unwrap();
    stored["ping"] = json!({ "x": 1_i32 });
    assert_eq!(
        labelled_issues(stored),
        ["ping.x: unknown: found Number(1)"]
    );

    let mut out: Vec<ping_schema::Issue<Value>> = Vec::new();
    let held = json!({ "x": 1_i32 });
    let declared = Ping::decode_with_value_fields(
        held.as_object().unwrap(),
        &[],
        ping_schema::issue_from_parts,
        &mut out,
    );
    assert_eq!(declared, Vec::<&str>::new());
    assert_eq!(out, Vec::new());
}

#[test]
fn a_unit_struct_held_as_text_is_listed_at_the_value_itself() {
    let read = Ping::from_value_with(json!("ping"), |_raw, _found| ping_schema::Verdict::Reject);
    assert_eq!(
        lines(&read.unwrap_err()),
        [
            "the value itself: invalid: expected Model(\"Ping\"), found String(\"ping\"): invalid type: string \"ping\", expected an empty object for `Ping`"
        ]
    );
}

/// An alias of a flagged model type is that type, so the field is walked into.
#[test]
fn a_field_typed_with_an_alias_of_a_model_type_is_walked_through_the_alias() {
    let stored = json!({ "featured": { "name": 7_i32, "versions": [{ "number": "1" }] } });
    let read = Shelf::from_value_with(stored, |_raw, _found| shelf_schema::Verdict::Reject);
    assert_eq!(
        lines(&read.unwrap_err()),
        [
            "featured.name: invalid: expected String, found Number(7): invalid type: integer `7`, expected a string",
            "featured.versions[0].number: invalid: expected I32, found String(\"1\"): invalid type: string \"1\", expected i32",
        ]
    );
    let absent = Shelf::from_value_with(json!({}), |_raw, _found| shelf_schema::Verdict::Reject);
    assert_eq!(
        lines(&absent.unwrap_err()),
        ["featured: missing: expected Model(\"EntryAlias\")"]
    );
}
