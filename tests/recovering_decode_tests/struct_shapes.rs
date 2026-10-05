//! `from_value_with` on the structs serde does not write as an object of their fields: a tuple
//! struct, a single-slot one, a brand and a unit struct, and on a tuple wherever a field's type
//! holds one. Each is walked in the form serde writes it.

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

/// A single-slot tuple struct with no `transparent`: serde writes it as a brand is written.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Wrapper(String);

/// A brand whose own reader refuses text shorter than three characters. Only a schema surface
/// hangs that check.
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

/// serde refuses a JSON array longer than the tuple, so the read carries its message beside the
/// position.
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

/// A value that is no array is read whole with the type's own reader: serde's verdict, at the
/// value, expecting the type.
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

/// The array holds no position for a slot serde never reads, serde reads a `default` slot when
/// its position is absent, and a slot that holds a model type is walked by that type's walker.
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

/// A brand and a single-slot tuple struct are each read as the one value they hold, at the path
/// they sit at, and the issue names what they hold.
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

/// The brand is read with its own reader, so the check written on it runs and its message is the
/// issue's.
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

/// A single-slot struct over a model type is that type at the path it sits at, with or without
/// `transparent`, and over a list or an `Option` of one it is walked as a field of that type is.
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

/// What a single-slot struct over a model type lists inside an object the caller holds is what
/// that type lists there, and the keys are that type's.
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
