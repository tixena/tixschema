//! `from_value_with` on types with `#[serde(flatten)]` fields. A flattened field's keys sit among
//! the keys of the object that flattens it, so it is walked in what the outer type's own fields
//! left of that object, its issues sit at that object's paths, and the keys it declares count as
//! declared. Each case holds the walker's list to what plain serde says of the same value.

use alloc::collections::BTreeMap;
use std::collections::HashMap;

use serde::de::Error as _;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

// The JSON schema of a type holding a `Version` names its module from here, beside the type.
#[cfg(feature = "jsonschema")]
use super::version_schema;
use super::{Version, lines};

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

/// The list each variant of an untagged enum earned in the one `NoVariant` among `$issues`.
macro_rules! tried {
    ($module:ident, $issues:expr) => {
        $issues
            .iter()
            .filter_map(|issue| {
                if let $module::Issue::NoVariant {
                    path: _path,
                    found: _found,
                    variants,
                } = issue
                {
                    Some(variants)
                } else {
                    None
                }
            })
            .flatten()
            .map(|(variant, list)| {
                (
                    *variant,
                    lines(&$module::Unrecovered {
                        issues: list.clone(),
                    }),
                )
            })
            .collect::<Vec<(&str, Vec<String>)>>()
    };
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Audit {
    created_by: String,
    revision: i32,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Extra {
    note: String,
    weight: u32,
}

/// One flattened struct beside a key of the type's own.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Note {
    #[serde(flatten)]
    audit: Audit,
    text: String,
}

/// Internally tagged: `{"kind": "Solid", "color": "red"}`.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind")]
enum Paint {
    Clear,
    Solid { color: String },
}

/// Externally tagged: `{"Curved": {"radius": 1.5}}`, `"Straight"`.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum Edge {
    Curved { radius: f64 },
    Straight,
}

/// Adjacently tagged: `{"kind": "Dotted", "data": {"gap": 2}}`, `{"kind": "Solid"}`.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data")]
enum Trim {
    Dotted { gap: i32 },
    Solid,
}

/// An externally tagged enum and an adjacently tagged one, each flattened.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Framed {
    #[serde(flatten)]
    edge: Edge,
    title: String,
    #[serde(flatten)]
    trim: Trim,
}

/// A struct, an optional struct and a tagged enum, each flattened beside a key of the type's own.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Sheet {
    #[serde(flatten)]
    audit: Audit,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    extra: Option<Extra>,
    #[serde(flatten)]
    paint: Paint,
    title: String,
}

/// A type that flattens one that flattens others.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Binder {
    #[serde(flatten)]
    sheet: Sheet,
    shelf: String,
}

/// A type that flattens fields, held under a key.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Cabinet {
    top: Sheet,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Stamp {
    version: Version,
}

/// An optional flattened type that holds a model type of its own.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Proof {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    stamp: Option<Stamp>,
    title: String,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Origin {
    source: String,
}

/// Two flattened structs.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Filed {
    #[serde(flatten)]
    audit: Audit,
    #[serde(flatten)]
    origin: Origin,
    title: String,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct ByMail {
    address: String,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct ByPhone {
    digits: String,
}

/// Untagged, every variant an object: two model types and a variant with a field of its own.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
enum Channel {
    Mail(ByMail),
    Pager { number: i32 },
    Phone(ByPhone),
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Notice {
    #[serde(flatten)]
    channel: Channel,
    subject: String,
}

/// A flattened map of plain values.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Counts {
    #[serde(flatten)]
    by_name: HashMap<String, i32>,
    title: String,
}

/// A flattened map of model types, beside a flattened struct.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Releases {
    #[serde(flatten)]
    audit: Audit,
    #[serde(flatten)]
    by_name: BTreeMap<String, Version>,
}

/// A model type that carries no flag: it only ever fills a parameter.
#[model_schema()]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Body {
    text: String,
}

/// A flattened type parameter.
#[model_schema(decode_with, default_types(T = Body))]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Letter<T> {
    #[serde(flatten)]
    body: T,
    id: String,
}

/// A flattened type parameter, then a flattened map: two fields that each take the rest.
#[model_schema(decode_with, default_types(T = Body))]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Packet<T> {
    #[serde(flatten)]
    body: T,
    #[serde(flatten)]
    counts: HashMap<String, i32>,
    id: String,
}

/// Externally tagged, with a struct variant that flattens a field.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum Logged {
    Gone,
    Made {
        #[serde(flatten)]
        audit: Audit,
        title: String,
    },
}

/// Internally tagged, with a struct variant that flattens a field.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind")]
enum Posted {
    Gone,
    Made {
        #[serde(flatten)]
        audit: Audit,
        title: String,
    },
}

/// Adjacently tagged, with a struct variant that flattens a field.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data")]
enum Queued {
    Gone,
    Made {
        #[serde(flatten)]
        audit: Audit,
        title: String,
    },
}

/// Untagged, with a struct variant that flattens a field.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
enum Seen {
    Gone {
        at: u32,
    },
    Made {
        #[serde(flatten)]
        audit: Audit,
        title: String,
    },
}

/// An optional flattened map, which no walk reaches.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Spare {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    rest: Option<HashMap<String, i32>>,
    title: String,
}

/// A flattened map of numbers beside a flattened tagged enum, whose text tag serde hands to the
/// map as well.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Mixed {
    #[serde(flatten)]
    counts: HashMap<String, i32>,
    #[serde(flatten)]
    paint: Paint,
}

/// A flattened map read by a hook named like what the walk of a flattened type keeps its keys as.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Marked {
    #[serde(flatten)]
    audit: Audit,
    #[serde(flatten, deserialize_with = "declared")]
    rest: HashMap<String, i32>,
}

/// Untagged, with a variant that has no field to walk: serde neither writes nor reads its one.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
enum Trace {
    Blank {
        #[serde(skip)]
        cached: u8,
    },
    Made {
        #[serde(flatten)]
        audit: Audit,
    },
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Traced {
    title: String,
    #[serde(flatten)]
    trace: Trace,
}

/// A type that flattens one that flattens a map, which takes every key it is handed.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Report {
    #[serde(flatten)]
    counts: Counts,
    id: String,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Captioned {
    name: String,
}

/// Untagged, with a map among its variants, which takes every key the enum is handed.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
enum Scores {
    Captioned(Captioned),
    Tallied(HashMap<String, i32>),
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Scoreboard {
    id: String,
    #[serde(flatten)]
    scores: Scores,
}

/// Declares a key the type that flattens it declares too.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Part {
    #[serde(default)]
    title: i32,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Whole {
    #[serde(flatten)]
    part: Part,
    title: String,
}

/// Internally tagged, with a struct variant that flattens a type that takes every key.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind")]
enum Tabled {
    Gone,
    Made {
        #[serde(flatten)]
        counts: Counts,
        id: String,
    },
}

/// An optional flattened enum, internally tagged.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Painted {
    own: String,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    paint: Option<Paint>,
}

/// An optional flattened enum, untagged.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Reached {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    channel: Option<Channel>,
    own: String,
}

/// Two optional flattened enums, internally tagged and untagged, beside a flattened struct.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Layered {
    #[serde(flatten)]
    audit: Audit,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    channel: Option<Channel>,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    paint: Option<Paint>,
    title: String,
}

/// Internally tagged, with a variant that holds a model type under a key.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind")]
enum Finish {
    Bare,
    Coated { version: Version },
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Finished {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    finish: Option<Finish>,
    own: String,
}

/// An optional flattened enum, externally tagged.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Edged {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    edge: Option<Edge>,
    own: String,
}

/// An optional flattened enum, adjacently tagged.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Trimmed {
    own: String,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    trim: Option<Trim>,
}

/// The read hook of [`Marked::rest`]: counts, refused where one is negative.
fn declared<'de, D>(deserializer: D) -> Result<HashMap<String, i32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let counts = HashMap::<String, i32>::deserialize(deserializer)?;
    if counts.values().any(|count| count.is_negative()) {
        return Err(D::Error::custom("`declared` reads no negative count"));
    }
    Ok(counts)
}

/// What plain serde says of `stored`, read as `T`.
fn serde_reads<T>(stored: &Value) -> bool
where
    T: for<'de> Deserialize<'de>,
{
    T::deserialize(stored).is_ok()
}

fn sheet(extra: Option<Extra>, paint: Paint) -> Sheet {
    Sheet {
        audit: Audit {
            created_by: "ada".to_owned(),
            revision: 1_i32,
        },
        extra,
        paint,
        title: "t".to_owned(),
    }
}

/// What serde writes for a type that flattens fields carries no key the walker does not know.
#[test]
fn what_serde_wrote_for_a_type_that_flattens_is_read_and_the_decider_never_runs() {
    let mut calls = 0_u32;
    let whole = sheet(
        Some(Extra {
            note: "n".to_owned(),
            weight: 2,
        }),
        Paint::Solid {
            color: "red".to_owned(),
        },
    );
    let written = serde_json::to_value(&whole).unwrap();
    assert_eq!(
        written,
        json!({
            "color": "red",
            "createdBy": "ada",
            "kind": "Solid",
            "note": "n",
            "revision": 1_i32,
            "title": "t",
            "weight": 2_i32,
        })
    );
    assert_eq!(
        read_counting!(Sheet, sheet_schema, written, calls),
        Ok(whole)
    );

    let filed = Filed {
        audit: Audit {
            created_by: "ada".to_owned(),
            revision: 1_i32,
        },
        origin: Origin {
            source: "import".to_owned(),
        },
        title: "t".to_owned(),
    };
    let filed_written = serde_json::to_value(&filed).unwrap();
    assert_eq!(
        read_counting!(Filed, filed_schema, filed_written, calls),
        Ok(filed)
    );

    let notice = Notice {
        channel: Channel::Pager { number: 7_i32 },
        subject: "s".to_owned(),
    };
    let notice_written = serde_json::to_value(&notice).unwrap();
    assert_eq!(notice_written, json!({ "number": 7_i32, "subject": "s" }));
    assert_eq!(
        read_counting!(Notice, notice_schema, notice_written, calls),
        Ok(notice)
    );

    let counts = Counts {
        by_name: HashMap::from([("a".to_owned(), 1_i32), ("b".to_owned(), 2_i32)]),
        title: "t".to_owned(),
    };
    let counts_written = serde_json::to_value(&counts).unwrap();
    assert_eq!(
        counts_written,
        json!({ "a": 1_i32, "b": 2_i32, "title": "t" })
    );
    assert_eq!(
        read_counting!(Counts, counts_schema, counts_written, calls),
        Ok(counts)
    );

    let letter = Letter {
        body: Body {
            text: "x".to_owned(),
        },
        id: "i".to_owned(),
    };
    let letter_written = serde_json::to_value(&letter).unwrap();
    assert_eq!(letter_written, json!({ "id": "i", "text": "x" }));
    assert_eq!(
        read_counting!(Letter<Body>, letter_schema, letter_written, calls),
        Ok(letter)
    );
    assert_eq!(calls, 0);
}

#[test]
fn a_flattened_optional_type_that_is_absent_is_no_issue() {
    let mut calls = 0_u32;
    let written = serde_json::to_value(sheet(None, Paint::Clear)).unwrap();
    assert_eq!(
        written,
        json!({ "createdBy": "ada", "kind": "Clear", "revision": 1_i32, "title": "t" })
    );
    assert_eq!(
        read_counting!(Sheet, sheet_schema, written, calls),
        Ok(sheet(None, Paint::Clear))
    );
    assert_eq!(calls, 0);
}

#[test]
fn a_key_no_flattened_type_declares_is_unknown_once() {
    let mut calls = 0_u32;
    let stored = json!({
        "createdBy": "ada",
        "kind": "Clear",
        "legacy": true,
        "revision": 1_i32,
        "title": "t",
    });
    assert!(serde_reads::<Sheet>(&stored));
    let read = read_counting!(Sheet, sheet_schema, stored, calls);
    assert_eq!(
        lines(&read.unwrap_err()),
        ["legacy: unknown: found Bool(true)"]
    );
    assert_eq!(calls, 1);
}

#[test]
fn an_issue_inside_each_flattened_type_is_listed_at_the_outer_objects_path() {
    let stored = json!({ "createdBy": 7_i32, "kind": "Solid", "title": "t" });
    assert!(!serde_reads::<Sheet>(&stored));
    assert_eq!(
        listed!(Sheet, sheet_schema, stored),
        [
            "createdBy: invalid: expected String, found Number(7): invalid type: integer `7`, expected a string",
            "revision: missing: expected I32",
            "color: missing: expected String",
        ]
    );
}

#[test]
fn a_decider_repairs_a_flattened_types_value_by_the_path_it_is_handed() {
    let stored = json!({ "createdBy": 7_i32, "pinned": true, "revision": 1_i32, "text": "hello" });
    assert!(!serde_reads::<Note>(&stored));
    let mut seen: Vec<String> = Vec::new();
    let read = Note::from_value_with(stored, |raw, found| {
        seen = lines(&note_schema::Unrecovered {
            issues: found.to_vec(),
        });
        for issue in found {
            if let note_schema::Issue::Invalid {
                path,
                expected: note_schema::Expected::String,
                found: Value::Number(number),
                reason: _reason,
            } = issue
            {
                path.set_in_value(raw, Value::from(number.to_string()));
            } else if let note_schema::Issue::Unknown {
                path,
                found: _found,
            } = issue
            {
                path.remove_from_value(raw);
            } else {
                return note_schema::Verdict::Reject;
            }
        }
        note_schema::Verdict::Fixed
    });
    assert_eq!(
        seen,
        [
            "createdBy: invalid: expected String, found Number(7): invalid type: integer `7`, expected a string",
            "pinned: unknown: found Bool(true)",
        ]
    );
    assert_eq!(
        read,
        Ok(Note {
            audit: Audit {
                created_by: "7".to_owned(),
                revision: 1_i32,
            },
            text: "hello".to_owned(),
        })
    );
}

#[test]
fn a_flattened_optional_type_half_there_is_mistyped_at_the_object() {
    let stored = json!({
        "createdBy": "ada",
        "kind": "Clear",
        "note": "n",
        "revision": 1_i32,
        "title": "t",
    });
    assert_eq!(
        serde_json::from_value::<Sheet>(stored.clone()).unwrap(),
        sheet(None, Paint::Clear)
    );
    let read = Sheet::from_value_with(stored.clone(), |_raw, _found| sheet_schema::Verdict::Reject);
    assert_eq!(
        read,
        Err(sheet_schema::Unrecovered {
            issues: vec![sheet_schema::Issue::Mistyped {
                path: sheet_schema::Path(Vec::new()),
                expected: sheet_schema::Expected::Optional(Box::new(
                    sheet_schema::Expected::Model("Extra")
                )),
                found: stored,
            }],
        })
    );
}

/// The decider is handed the object the half-written part sits in, and takes the stray key out.
#[test]
fn a_decider_takes_out_the_half_of_an_optional_type_the_object_holds() {
    let stored = json!({
        "createdBy": "ada",
        "kind": "Clear",
        "note": "n",
        "revision": 1_i32,
        "title": "t",
    });
    let read = Sheet::from_value_with(stored, |raw, found| {
        for issue in found {
            if let sheet_schema::Issue::Mistyped {
                path,
                expected: _expected,
                found: _found,
            } = issue
                && path.0.is_empty()
                && let Some(object) = raw.as_object_mut()
            {
                object.remove("note");
            } else {
                return sheet_schema::Verdict::Reject;
            }
        }
        sheet_schema::Verdict::Fixed
    });
    assert_eq!(read, Ok(sheet(None, Paint::Clear)));
}

#[test]
fn a_flattened_optional_type_that_is_there_lists_its_own_issues() {
    let stored = json!({ "title": "t", "version": { "draft": true, "number": 3_i32 } });
    assert!(serde_reads::<Proof>(&stored));
    assert_eq!(
        listed!(Proof, proof_schema, stored),
        ["version.draft: unknown: found Bool(true)"]
    );
    let refused = json!({ "title": "t", "version": { "number": "3" } });
    assert_eq!(
        serde_json::from_value::<Proof>(refused.clone()).unwrap(),
        Proof {
            stamp: None,
            title: "t".to_owned(),
        }
    );
    assert_eq!(
        listed!(Proof, proof_schema, refused),
        [
            "the value itself: mistyped: expected Optional(Model(\"Stamp\")), found Object {\"title\": String(\"t\"), \"version\": Object {\"number\": String(\"3\")}}"
        ]
    );
}

#[test]
fn a_flattened_tagged_enum_reads_its_tag_in_the_outer_object() {
    let unknown = json!({
        "createdBy": "ada",
        "kind": "Striped",
        "revision": 1_i32,
        "title": "t",
    });
    assert!(!serde_reads::<Sheet>(&unknown));
    assert_eq!(
        listed!(Sheet, sheet_schema, unknown),
        [
            "kind: invalid: expected Variants([\"Clear\", \"Solid\"]), found String(\"Striped\"): unknown variant `Striped`, expected `Clear` or `Solid`"
        ]
    );
    let absent = json!({ "createdBy": "ada", "revision": 1_i32, "title": "t" });
    assert!(!serde_reads::<Sheet>(&absent));
    assert_eq!(
        listed!(Sheet, sheet_schema, absent),
        ["kind: missing: expected Variants([\"Clear\", \"Solid\"])"]
    );
}

#[test]
fn a_flattened_externally_or_adjacently_tagged_enum_is_walked_under_its_own_keys() {
    let mut calls = 0_u32;
    let framed = Framed {
        edge: Edge::Curved { radius: 1.5_f64 },
        title: "t".to_owned(),
        trim: Trim::Dotted { gap: 2_i32 },
    };
    let written = serde_json::to_value(&framed).unwrap();
    assert_eq!(
        written,
        json!({
            "Curved": { "radius": 1.5_f64 },
            "data": { "gap": 2_i32 },
            "kind": "Dotted",
            "title": "t",
        })
    );
    assert_eq!(
        read_counting!(Framed, framed_schema, written, calls),
        Ok(framed)
    );
    let units = Framed {
        edge: Edge::Straight,
        title: "t".to_owned(),
        trim: Trim::Solid,
    };
    let units_written = serde_json::to_value(&units).unwrap();
    assert_eq!(
        units_written,
        json!({ "Straight": null, "kind": "Solid", "title": "t" })
    );
    assert_eq!(
        read_counting!(Framed, framed_schema, units_written, calls),
        Ok(units)
    );
    assert_eq!(calls, 0);

    let stored = json!({
        "Curved": { "radius": "wide" },
        "data": {},
        "kind": "Dotted",
        "legacy": true,
        "title": "t",
    });
    assert!(!serde_reads::<Framed>(&stored));
    assert_eq!(
        listed!(Framed, framed_schema, stored),
        [
            "Curved.radius: invalid: expected F64, found String(\"wide\"): invalid type: string \"wide\", expected f64",
            "data.gap: missing: expected I32",
            "legacy: unknown: found Bool(true)",
        ]
    );
    let neither = json!({ "kind": "Solid", "title": "t" });
    assert!(!serde_reads::<Framed>(&neither));
    assert_eq!(
        listed!(Framed, framed_schema, neither),
        ["the value itself: missing: expected Variants([\"Curved\", \"Straight\"])"]
    );
}

#[test]
fn a_flattened_untagged_enum_is_walked_as_the_variant_serde_reads() {
    let stored = json!({ "address": "a@b", "legacy": true, "subject": "s" });
    assert!(serde_reads::<Notice>(&stored));
    assert_eq!(
        listed!(Notice, notice_schema, stored),
        ["legacy: unknown: found Bool(true)"]
    );
    let own_fields = json!({ "legacy": true, "number": 7_i32, "subject": "s" });
    assert!(serde_reads::<Notice>(&own_fields));
    assert_eq!(
        listed!(Notice, notice_schema, own_fields),
        ["legacy: unknown: found Bool(true)"]
    );
}

#[test]
fn a_flattened_untagged_enum_no_variant_reads_is_one_no_variant() {
    let stored = json!({ "digits": 5_i32, "subject": "s" });
    assert!(!serde_reads::<Notice>(&stored));
    let report =
        Notice::from_value_with(stored, |_raw, _found| notice_schema::Verdict::Reject).unwrap_err();
    assert_eq!(
        lines(&report),
        [
            "the value itself: no variant: found Object {\"digits\": Number(5)}, tried Mail, Pager, Phone"
        ]
    );
    assert_eq!(
        tried!(notice_schema, report.issues),
        [
            ("Mail", vec!["address: missing: expected String".to_owned()]),
            ("Pager", vec!["number: missing: expected I32".to_owned()]),
            (
                "Phone",
                vec![
                    "digits: invalid: expected String, found Number(5): invalid type: integer `5`, expected a string"
                        .to_owned()
                ]
            ),
        ]
    );
}

#[test]
fn an_untagged_enums_fields_walker_returns_the_keys_of_the_variant_serde_reads() {
    let mut out: Vec<channel_schema::Issue<Value>> = Vec::new();
    let mail = json!({ "address": "a@b", "legacy": true });
    assert_eq!(
        Channel::decode_with_value_fields(
            mail.as_object().unwrap(),
            &[],
            channel_schema::issue_from_parts,
            &mut out,
        ),
        ["address"]
    );
    let pager = json!({ "legacy": true, "number": 7_i32 });
    assert_eq!(
        Channel::decode_with_value_fields(
            pager.as_object().unwrap(),
            &[],
            channel_schema::issue_from_parts,
            &mut out,
        ),
        ["number"]
    );
    assert_eq!(out, Vec::new());
    let neither = json!({ "digits": 5_i32, "legacy": true });
    assert_eq!(
        Channel::decode_with_value_fields(
            neither.as_object().unwrap(),
            &[Ok("channel".to_owned())],
            channel_schema::issue_from_parts,
            &mut out,
        ),
        ["digits", "legacy"]
    );
    assert_eq!(
        lines(&channel_schema::Unrecovered { issues: out }),
        [
            "channel: no variant: found Object {\"digits\": Number(5), \"legacy\": Bool(true)}, tried Mail, Pager, Phone"
        ]
    );
}

#[test]
fn a_flattened_map_takes_every_key_nothing_else_declares() {
    let stored = json!({ "a": "x", "b": 2_i32, "title": "t" });
    assert!(!serde_reads::<Counts>(&stored));
    assert_eq!(
        listed!(Counts, counts_schema, stored),
        ["a: invalid: expected I32, found String(\"x\"): invalid type: string \"x\", expected i32"]
    );
}

#[test]
fn a_flattened_map_of_model_types_walks_each_value_at_its_key() {
    let mut calls = 0_u32;
    let releases = Releases {
        audit: Audit {
            created_by: "ada".to_owned(),
            revision: 1_i32,
        },
        by_name: BTreeMap::from([("first".to_owned(), Version { number: 1_i32 })]),
    };
    let written = serde_json::to_value(&releases).unwrap();
    assert_eq!(
        written,
        json!({ "createdBy": "ada", "first": { "number": 1_i32 }, "revision": 1_i32 })
    );
    assert_eq!(
        read_counting!(Releases, releases_schema, written, calls),
        Ok(releases)
    );
    assert_eq!(calls, 0);

    let stored = json!({
        "createdBy": "ada",
        "first": { "draft": true, "number": "1" },
        "revision": 1_i32,
    });
    assert!(!serde_reads::<Releases>(&stored));
    assert_eq!(
        listed!(Releases, releases_schema, stored),
        [
            "first.number: invalid: expected I32, found String(\"1\"): invalid type: string \"1\", expected i32",
            "first.draft: unknown: found Bool(true)",
        ]
    );
}

#[test]
fn a_flattened_type_parameter_is_read_whole_from_the_keys_nothing_else_declares() {
    let stored = json!({ "id": "i", "more": 1_i32, "text": 5_i32 });
    assert!(!serde_reads::<Letter<Body>>(&stored));
    assert_eq!(
        listed!(Letter<Body>, letter_schema, stored),
        [
            "the value itself: invalid: expected TypeParam(\"T\"), found Object {\"more\": Number(1), \"text\": Number(5)}: invalid type: integer `5`, expected a string"
        ]
    );

    let mut calls = 0_u32;
    let extra_key = json!({ "id": "i", "more": 1_i32, "text": "x" });
    assert_eq!(
        read_counting!(Letter<Body>, letter_schema, extra_key, calls),
        Ok(Letter {
            body: Body {
                text: "x".to_owned(),
            },
            id: "i".to_owned(),
        })
    );
    // Filled with a map, the parameter is read from the keys the type itself does not declare.
    let counted = json!({ "id": "i", "x": 1_i32 });
    assert!(serde_reads::<Letter<HashMap<String, i32>>>(&counted));
    assert_eq!(
        read_counting!(Letter<HashMap<String, i32>>, letter_schema, counted, calls),
        Ok(Letter {
            body: HashMap::from([("x".to_owned(), 1_i32)]),
            id: "i".to_owned(),
        })
    );
    assert_eq!(calls, 0);
}

#[test]
fn a_second_flattened_field_that_takes_the_rest_is_walked_by_nothing() {
    let mut calls = 0_u32;
    let stored = json!({ "a": 1_i32, "id": "i", "text": "x" });
    assert_eq!(
        read_counting!(Packet<Body>, packet_schema, stored, calls),
        Ok(Packet {
            body: Body {
                text: "x".to_owned(),
            },
            counts: HashMap::from([("a".to_owned(), 1_i32)]),
            id: "i".to_owned(),
        })
    );
    assert_eq!(calls, 0);
    let refused = json!({ "a": "bad", "id": "i", "text": "x" });
    assert!(!serde_reads::<Packet<Body>>(&refused));
    assert_eq!(
        listed!(Packet<Body>, packet_schema, refused),
        ["undescribed: invalid type: string \"bad\", expected i32"]
    );
}

/// Each of two flattened types contributes its keys, and one undeclared key is listed once.
#[test]
fn two_flattened_types_each_declare_their_keys() {
    let stored = json!({
        "createdBy": "ada",
        "legacy": true,
        "revision": 1_i32,
        "source": "import",
        "title": "t",
    });
    assert!(serde_reads::<Filed>(&stored));
    let mut out: Vec<filed_schema::Issue<Value>> = Vec::new();
    assert_eq!(
        Filed::decode_with_value_fields(
            stored.as_object().unwrap(),
            &[],
            filed_schema::issue_from_parts,
            &mut out,
        ),
        ["title", "createdBy", "revision", "source"]
    );
    assert_eq!(out, Vec::new());
    assert_eq!(
        listed!(Filed, filed_schema, stored),
        ["legacy: unknown: found Bool(true)"]
    );
}

#[test]
fn a_flattened_type_that_flattens_others_is_walked_in_the_same_object() {
    let stored = json!({
        "createdBy": 7_i32,
        "kind": "Clear",
        "legacy": true,
        "revision": 1_i32,
        "shelf": "s",
        "title": "t",
    });
    assert!(!serde_reads::<Binder>(&stored));
    assert_eq!(
        listed!(Binder, binder_schema, stored),
        [
            "createdBy: invalid: expected String, found Number(7): invalid type: integer `7`, expected a string",
            "legacy: unknown: found Bool(true)",
        ]
    );
    let held = json!({
        "top": { "createdBy": 7_i32, "kind": "Clear", "legacy": true, "revision": 1_i32, "title": "t" },
    });
    assert!(!serde_reads::<Cabinet>(&held));
    assert_eq!(
        listed!(Cabinet, cabinet_schema, held),
        [
            "top.createdBy: invalid: expected String, found Number(7): invalid type: integer `7`, expected a string",
            "top.legacy: unknown: found Bool(true)",
        ]
    );
}

/// What serde writes for a struct variant that flattens a field is read under every tagging.
#[test]
fn what_serde_wrote_for_a_variant_that_flattens_is_read_and_the_decider_never_runs() {
    let mut calls = 0_u32;
    let audit = || Audit {
        created_by: "ada".to_owned(),
        revision: 1_i32,
    };
    let logged = serde_json::to_value(Logged::Made {
        audit: audit(),
        title: "t".to_owned(),
    })
    .unwrap();
    assert_eq!(
        logged,
        json!({ "Made": { "createdBy": "ada", "revision": 1_i32, "title": "t" } })
    );
    read_counting!(Logged, logged_schema, logged, calls).unwrap();
    let posted = serde_json::to_value(Posted::Made {
        audit: audit(),
        title: "t".to_owned(),
    })
    .unwrap();
    assert_eq!(
        posted,
        json!({ "createdBy": "ada", "kind": "Made", "revision": 1_i32, "title": "t" })
    );
    read_counting!(Posted, posted_schema, posted, calls).unwrap();
    let queued = serde_json::to_value(Queued::Made {
        audit: audit(),
        title: "t".to_owned(),
    })
    .unwrap();
    assert_eq!(
        queued,
        json!({ "data": { "createdBy": "ada", "revision": 1_i32, "title": "t" }, "kind": "Made" })
    );
    read_counting!(Queued, queued_schema, queued, calls).unwrap();
    let seen = serde_json::to_value(Seen::Made {
        audit: audit(),
        title: "t".to_owned(),
    })
    .unwrap();
    assert_eq!(
        seen,
        json!({ "createdBy": "ada", "revision": 1_i32, "title": "t" })
    );
    read_counting!(Seen, seen_schema, seen, calls).unwrap();
    assert_eq!(calls, 0);
}

#[test]
fn a_variants_flattened_field_is_walked_in_the_variants_object_under_every_tagging() {
    let internal = json!({ "createdBy": 7_i32, "kind": "Made", "legacy": 1_i32, "title": "t" });
    assert!(!serde_reads::<Posted>(&internal));
    assert_eq!(
        listed!(Posted, posted_schema, internal),
        [
            "createdBy: invalid: expected String, found Number(7): invalid type: integer `7`, expected a string",
            "revision: missing: expected I32",
            "legacy: unknown: found Number(1)",
        ]
    );
    let external = json!({ "Made": { "createdBy": 7_i32, "legacy": 1_i32, "title": "t" } });
    assert!(!serde_reads::<Logged>(&external));
    assert_eq!(
        listed!(Logged, logged_schema, external),
        [
            "Made.createdBy: invalid: expected String, found Number(7): invalid type: integer `7`, expected a string",
            "Made.revision: missing: expected I32",
            "Made.legacy: unknown: found Number(1)",
        ]
    );
    let adjacent = json!({
        "data": { "createdBy": 7_i32, "legacy": 1_i32, "title": "t" },
        "kind": "Made",
    });
    assert!(!serde_reads::<Queued>(&adjacent));
    assert_eq!(
        listed!(Queued, queued_schema, adjacent),
        [
            "data.createdBy: invalid: expected String, found Number(7): invalid type: integer `7`, expected a string",
            "data.revision: missing: expected I32",
            "data.legacy: unknown: found Number(1)",
        ]
    );
    let untagged = json!({ "createdBy": "ada", "legacy": 1_i32, "revision": 1_i32, "title": "t" });
    assert!(serde_reads::<Seen>(&untagged));
    assert_eq!(
        listed!(Seen, seen_schema, untagged),
        ["legacy: unknown: found Number(1)"]
    );
}

#[test]
fn an_untagged_variant_that_flattens_lists_the_flattened_types_issues_as_its_own() {
    let stored = json!({ "createdBy": 7_i32, "title": "t" });
    assert!(!serde_reads::<Seen>(&stored));
    let report =
        Seen::from_value_with(stored, |_raw, _found| seen_schema::Verdict::Reject).unwrap_err();
    assert_eq!(
        tried!(seen_schema, report.issues),
        [
            (
                "Gone",
                vec![
                    "at: missing: expected U32".to_owned(),
                    "createdBy: unknown: found Number(7)".to_owned(),
                    "title: unknown: found String(\"t\")".to_owned(),
                ]
            ),
            (
                "Made",
                vec![
                    "createdBy: invalid: expected String, found Number(7): invalid type: integer `7`, expected a string"
                        .to_owned(),
                    "revision: missing: expected I32".to_owned(),
                ]
            ),
        ]
    );
}

#[test]
fn an_optional_flattened_map_is_walked_by_nothing_and_takes_every_key() {
    let mut calls = 0_u32;
    let stored = json!({ "a": "x", "title": "t" });
    assert_eq!(
        read_counting!(Spare, spare_schema, stored, calls),
        Ok(Spare {
            rest: None,
            title: "t".to_owned(),
        })
    );
    assert_eq!(calls, 0);
}

#[test]
fn a_flattened_map_that_cannot_hold_the_tag_of_an_enum_beside_it_is_undescribed() {
    let stored = json!({ "kind": "Clear" });
    assert!(!serde_reads::<Mixed>(&stored));
    assert_eq!(
        listed!(Mixed, mixed_schema, stored),
        ["undescribed: invalid type: string \"Clear\", expected i32"]
    );
}

#[test]
fn a_hook_named_like_the_keys_a_flattening_walker_keeps_is_the_authors_function() {
    let mut calls = 0_u32;
    let stored = json!({ "createdBy": "ada", "revision": 1_i32, "x": 1_i32 });
    assert_eq!(
        read_counting!(Marked, marked_schema, stored, calls),
        Ok(Marked {
            audit: Audit {
                created_by: "ada".to_owned(),
                revision: 1_i32,
            },
            rest: HashMap::from([("x".to_owned(), 1_i32)]),
        })
    );
    assert_eq!(calls, 0);
    let negative = json!({ "createdBy": "ada", "revision": 1_i32, "x": -1_i32 });
    assert!(!serde_reads::<Marked>(&negative));
    assert_eq!(
        listed!(Marked, marked_schema, negative),
        [
            "the value itself: invalid: expected Map(I32), found Object {\"x\": Number(-1)}: `declared` reads no negative count"
        ]
    );
}

#[test]
fn a_flattened_untagged_variant_with_no_field_declares_no_key() {
    let stored = json!({ "legacy": true, "title": "t" });
    assert_eq!(
        serde_json::from_value::<Traced>(stored.clone()).unwrap(),
        Traced {
            title: "t".to_owned(),
            trace: Trace::Blank { cached: 0 },
        }
    );
    assert_eq!(
        listed!(Traced, traced_schema, stored),
        ["legacy: unknown: found Bool(true)"]
    );
}

/// serde hands a flattened type the keys the outer type's own fields did not take.
#[test]
fn a_flattened_type_that_flattens_a_map_is_handed_no_key_of_the_outer_type() {
    let report = Report {
        counts: Counts {
            by_name: HashMap::from([("a".to_owned(), 1_i32)]),
            title: "t".to_owned(),
        },
        id: "i".to_owned(),
    };
    let written = serde_json::to_value(&report).unwrap();
    assert_eq!(written, json!({ "a": 1_i32, "id": "i", "title": "t" }));
    let mut calls = 0_u32;
    assert_eq!(
        read_counting!(Report, report_schema, written, calls),
        Ok(report)
    );
    assert_eq!(calls, 0);
}

#[test]
fn a_flattened_untagged_enum_with_a_map_variant_is_handed_no_key_of_the_outer_type() {
    let scoreboard = Scoreboard {
        id: "i".to_owned(),
        scores: Scores::Tallied(HashMap::from([("a".to_owned(), 1_i32)])),
    };
    let written = serde_json::to_value(&scoreboard).unwrap();
    assert_eq!(written, json!({ "a": 1_i32, "id": "i" }));
    let mut calls = 0_u32;
    assert_eq!(
        read_counting!(Scoreboard, scoreboard_schema, written, calls),
        Ok(scoreboard)
    );
    assert_eq!(calls, 0);
}

#[test]
fn a_key_the_outer_type_and_a_flattened_type_both_declare_is_the_outer_types_alone() {
    let stored = json!({ "title": "t" });
    assert_eq!(
        serde_json::from_value::<Whole>(stored.clone()).unwrap(),
        Whole {
            part: Part { title: 0_i32 },
            title: "t".to_owned(),
        }
    );
    let mut calls = 0_u32;
    assert_eq!(
        read_counting!(Whole, whole_schema, stored, calls),
        Ok(Whole {
            part: Part { title: 0_i32 },
            title: "t".to_owned(),
        })
    );
    assert_eq!(calls, 0);

    let numbered = json!({ "title": 5_i32 });
    assert!(!serde_reads::<Whole>(&numbered));
    assert_eq!(
        listed!(Whole, whole_schema, numbered),
        [
            "title: invalid: expected String, found Number(5): invalid type: integer `5`, expected a string"
        ]
    );
    let undeclared = json!({ "legacy": true, "title": "t" });
    assert!(serde_reads::<Whole>(&undeclared));
    assert_eq!(
        listed!(Whole, whole_schema, undeclared),
        ["legacy: unknown: found Bool(true)"]
    );
}

#[test]
fn an_issue_in_a_flattened_type_that_flattens_a_map_is_listed_at_its_key() {
    let stored = json!({ "a": "x", "id": "i", "title": "t" });
    assert!(!serde_reads::<Report>(&stored));
    assert_eq!(
        listed!(Report, report_schema, stored),
        ["a: invalid: expected I32, found String(\"x\"): invalid type: string \"x\", expected i32"]
    );
    // The map takes a key nothing declares, so serde refuses a value in it that is no count.
    let undeclared = json!({ "a": 1_i32, "id": "i", "legacy": [1_i32], "title": "t" });
    assert!(!serde_reads::<Report>(&undeclared));
    assert_eq!(
        listed!(Report, report_schema, undeclared),
        [
            "legacy: invalid: expected I32, found Array [Number(1)]: invalid type: sequence, expected i32"
        ]
    );
}

#[test]
fn a_flattened_untagged_enum_no_variant_reads_holds_what_the_enum_was_handed() {
    let stored = json!({ "a": "x", "id": "i" });
    assert!(!serde_reads::<Scoreboard>(&stored));
    let report =
        Scoreboard::from_value_with(stored, |_raw, _found| scoreboard_schema::Verdict::Reject)
            .unwrap_err();
    assert_eq!(
        lines(&report),
        [
            "the value itself: no variant: found Object {\"a\": String(\"x\")}, tried Captioned, Tallied"
        ]
    );
    assert_eq!(
        tried!(scoreboard_schema, report.issues),
        [
            (
                "Captioned",
                vec!["name: missing: expected String".to_owned()]
            ),
            (
                "Tallied",
                vec![
                    "a: invalid: expected I32, found String(\"x\"): invalid type: string \"x\", expected i32"
                        .to_owned()
                ]
            ),
        ]
    );
}

#[test]
fn a_variants_flattened_type_that_flattens_a_map_is_handed_no_key_of_the_variant() {
    let made = Tabled::Made {
        counts: Counts {
            by_name: HashMap::from([("a".to_owned(), 1_i32)]),
            title: "t".to_owned(),
        },
        id: "i".to_owned(),
    };
    let written = serde_json::to_value(&made).unwrap();
    assert_eq!(
        written,
        json!({ "a": 1_i32, "id": "i", "kind": "Made", "title": "t" })
    );
    let mut calls = 0_u32;
    assert_eq!(
        read_counting!(Tabled, tabled_schema, written, calls),
        Ok(made)
    );
    assert_eq!(calls, 0);
    let stored = json!({ "a": "x", "id": "i", "kind": "Made", "title": "t" });
    assert!(!serde_reads::<Tabled>(&stored));
    assert_eq!(
        listed!(Tabled, tabled_schema, stored),
        ["a: invalid: expected I32, found String(\"x\"): invalid type: string \"x\", expected i32"]
    );
}

#[test]
fn a_flattened_optional_enum_that_is_absent_is_no_issue() {
    let mut calls = 0_u32;
    let painted = Painted {
        own: "x".to_owned(),
        paint: None,
    };
    let painted_written = serde_json::to_value(&painted).unwrap();
    assert_eq!(painted_written, json!({ "own": "x" }));
    assert_eq!(
        read_counting!(Painted, painted_schema, painted_written, calls),
        Ok(painted)
    );
    let reached = Reached {
        channel: None,
        own: "x".to_owned(),
    };
    let reached_written = serde_json::to_value(&reached).unwrap();
    assert_eq!(reached_written, json!({ "own": "x" }));
    assert_eq!(
        read_counting!(Reached, reached_schema, reached_written, calls),
        Ok(reached)
    );
    assert_eq!(calls, 0);
}

#[test]
fn an_absent_flattened_optional_enum_beside_another_flattened_type_is_no_issue() {
    let layered = Layered {
        audit: Audit {
            created_by: "ada".to_owned(),
            revision: 1_i32,
        },
        channel: None,
        paint: None,
        title: "t".to_owned(),
    };
    let written = serde_json::to_value(&layered).unwrap();
    assert_eq!(
        written,
        json!({ "createdBy": "ada", "revision": 1_i32, "title": "t" })
    );
    let mut calls = 0_u32;
    assert_eq!(
        read_counting!(Layered, layered_schema, written, calls),
        Ok(layered)
    );
    assert_eq!(calls, 0);
}

#[test]
fn an_absent_flattened_optional_enum_leaves_the_objects_keys_to_the_outer_type() {
    let tagged = json!({ "legacy": true, "own": "x" });
    assert!(serde_reads::<Painted>(&tagged));
    assert_eq!(
        listed!(Painted, painted_schema, tagged),
        ["legacy: unknown: found Bool(true)"]
    );
    let untagged = json!({ "legacy": true, "own": "x" });
    assert!(serde_reads::<Reached>(&untagged));
    assert_eq!(
        listed!(Reached, reached_schema, untagged),
        ["legacy: unknown: found Bool(true)"]
    );
    let no_variant = json!({ "address": 5_i32, "own": "x" });
    assert_eq!(
        serde_json::from_value::<Reached>(no_variant.clone()).unwrap(),
        Reached {
            channel: None,
            own: "x".to_owned(),
        }
    );
    assert_eq!(
        listed!(Reached, reached_schema, no_variant),
        ["address: unknown: found Number(5)"]
    );
}

#[test]
fn a_flattened_optional_enum_that_is_there_is_walked_as_the_variant_it_names() {
    let mut calls = 0_u32;
    let painted = Painted {
        own: "x".to_owned(),
        paint: Some(Paint::Solid {
            color: "red".to_owned(),
        }),
    };
    let painted_written = serde_json::to_value(&painted).unwrap();
    assert_eq!(
        painted_written,
        json!({ "color": "red", "kind": "Solid", "own": "x" })
    );
    assert_eq!(
        read_counting!(Painted, painted_schema, painted_written, calls),
        Ok(painted)
    );
    let reached = Reached {
        channel: Some(Channel::Mail(ByMail {
            address: "a@b".to_owned(),
        })),
        own: "x".to_owned(),
    };
    let reached_written = serde_json::to_value(&reached).unwrap();
    assert_eq!(reached_written, json!({ "address": "a@b", "own": "x" }));
    assert_eq!(
        read_counting!(Reached, reached_schema, reached_written, calls),
        Ok(reached)
    );
    assert_eq!(calls, 0);

    let inside = json!({
        "kind": "Coated",
        "own": "x",
        "version": { "draft": true, "number": 3_i32 },
    });
    assert!(serde_reads::<Finished>(&inside));
    assert_eq!(
        listed!(Finished, finished_schema, inside),
        ["version.draft: unknown: found Bool(true)"]
    );
    let beside = json!({ "address": "a@b", "legacy": true, "own": "x" });
    assert!(serde_reads::<Reached>(&beside));
    assert_eq!(
        listed!(Reached, reached_schema, beside),
        ["legacy: unknown: found Bool(true)"]
    );
}

#[test]
fn a_flattened_optional_enum_named_and_not_read_is_mistyped_at_the_object() {
    for stored in [
        json!({ "kind": "Striped", "own": "x" }),
        json!({ "kind": "Solid", "own": "x" }),
    ] {
        assert_eq!(
            serde_json::from_value::<Painted>(stored.clone()).unwrap(),
            Painted {
                own: "x".to_owned(),
                paint: None,
            }
        );
        let read = Painted::from_value_with(stored.clone(), |_raw, _found| {
            painted_schema::Verdict::Reject
        });
        assert_eq!(
            read,
            Err(painted_schema::Unrecovered {
                issues: vec![painted_schema::Issue::Mistyped {
                    path: painted_schema::Path(Vec::new()),
                    expected: painted_schema::Expected::Optional(Box::new(
                        painted_schema::Expected::Model("Paint")
                    )),
                    found: stored,
                }],
            })
        );
    }
}

/// An optional externally tagged enum is absent when no key names a variant of it, as before.
#[test]
fn an_absent_flattened_optional_externally_tagged_enum_lists_nothing() {
    let mut calls = 0_u32;
    let absent = Edged {
        edge: None,
        own: "x".to_owned(),
    };
    let absent_written = serde_json::to_value(&absent).unwrap();
    assert_eq!(absent_written, json!({ "own": "x" }));
    assert_eq!(
        read_counting!(Edged, edged_schema, absent_written, calls),
        Ok(absent)
    );
    let there = Edged {
        edge: Some(Edge::Curved { radius: 1.5_f64 }),
        own: "x".to_owned(),
    };
    let there_written = serde_json::to_value(&there).unwrap();
    assert_eq!(
        there_written,
        json!({ "Curved": { "radius": 1.5_f64 }, "own": "x" })
    );
    assert_eq!(
        read_counting!(Edged, edged_schema, there_written, calls),
        Ok(there)
    );
    assert_eq!(calls, 0);
    let undeclared = json!({ "legacy": true, "own": "x" });
    assert!(serde_reads::<Edged>(&undeclared));
    assert_eq!(
        listed!(Edged, edged_schema, undeclared),
        ["legacy: unknown: found Bool(true)"]
    );
}

#[test]
fn an_absent_flattened_optional_adjacently_tagged_enum_lists_nothing() {
    let mut calls = 0_u32;
    let absent = Trimmed {
        own: "x".to_owned(),
        trim: None,
    };
    let absent_written = serde_json::to_value(&absent).unwrap();
    assert_eq!(absent_written, json!({ "own": "x" }));
    assert_eq!(
        read_counting!(Trimmed, trimmed_schema, absent_written, calls),
        Ok(absent)
    );
    let there = Trimmed {
        own: "x".to_owned(),
        trim: Some(Trim::Dotted { gap: 2_i32 }),
    };
    let there_written = serde_json::to_value(&there).unwrap();
    assert_eq!(
        there_written,
        json!({ "data": { "gap": 2_i32 }, "kind": "Dotted", "own": "x" })
    );
    assert_eq!(
        read_counting!(Trimmed, trimmed_schema, there_written, calls),
        Ok(there)
    );
    assert_eq!(calls, 0);
    let content_alone = json!({ "data": { "gap": 2_i32 }, "own": "x" });
    assert!(serde_reads::<Trimmed>(&content_alone));
    assert_eq!(
        listed!(Trimmed, trimmed_schema, content_alone),
        ["data: unknown: found Object {\"gap\": Number(2)}"]
    );
}

#[test]
fn each_flagged_type_answers_whether_an_object_names_a_value_of_it() {
    let named = |stored: Value, asked: fn(&serde_json::Map<String, Value>) -> bool| {
        asked(stored.as_object().unwrap())
    };
    assert!(named(
        json!({ "createdBy": 7_i32 }),
        Audit::decode_with_value_named
    ));
    assert!(!named(
        json!({ "legacy": true }),
        Audit::decode_with_value_named
    ));
    assert!(named(
        json!({ "kind": "Clear" }),
        Sheet::decode_with_value_named
    ));
    assert!(named(
        json!({ "note": "n" }),
        Sheet::decode_with_value_named
    ));
    assert!(!named(
        json!({ "legacy": true }),
        Sheet::decode_with_value_named
    ));
    assert!(named(
        json!({ "legacy": true }),
        Counts::decode_with_value_named
    ));
    assert!(!named(json!({}), Counts::decode_with_value_named));
    assert!(named(
        json!({ "kind": 5_i32 }),
        Paint::decode_with_value_named
    ));
    assert!(!named(
        json!({ "color": "red" }),
        Paint::decode_with_value_named
    ));
    assert!(named(
        json!({ "Straight": null }),
        Edge::decode_with_value_named
    ));
    assert!(!named(
        json!({ "radius": 1.5_f64 }),
        Edge::decode_with_value_named
    ));
    assert!(named(
        json!({ "kind": "Solid" }),
        Trim::decode_with_value_named
    ));
    assert!(!named(json!({ "data": {} }), Trim::decode_with_value_named));
    assert!(named(
        json!({ "address": "a@b" }),
        Channel::decode_with_value_named
    ));
    assert!(!named(
        json!({ "address": 5_i32 }),
        Channel::decode_with_value_named
    ));
    assert!(!named(json!({}), Channel::decode_with_value_named));
}
