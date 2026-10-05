//! `from_bson_with` on structs with named fields, held to what plain serde says of the same
//! document. Two binaries compile this module, each with the name `bson` bound to one major
//! version of the library, so every case runs against both.
//!
//! An issue is asserted by its kind, its path, its expected type and the value it holds. Its
//! `reason` is the `bson` library's own wording, which its two versions write differently, so a
//! test reads it only where it is about it.

/// The `with` module of [`Hooked::count`]: a number written as text.
mod as_text {
    use core::fmt::Display;

    use serde::Deserialize as _;
    use serde::de::Error as _;

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<u32, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(D::Error::custom)
    }

    pub(super) fn serialize<S, T>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
        T: Display,
    {
        serializer.collect_str(value)
    }
}

use core::any::TypeId;
use core::error::Error;
use core::fmt::Display;
use core::str::FromStr;
use std::collections::HashMap;

use bson::oid::ObjectId;
use bson::{Bson, Document, doc};
use chrono::{DateTime, Utc};
use serde::de::Error as _;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// What `$issues` say less each `reason`, whichever type's `$module` declares them.
macro_rules! told {
    ($module:ident, $issues:expr) => {
        $issues
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
                $module::Issue::Missing { path, expected } => {
                    ("Missing", path.to_string(), format!("{expected:?}"), None)
                }
                $module::Issue::Mistyped {
                    path,
                    expected,
                    found,
                } => (
                    "Mistyped",
                    path.to_string(),
                    format!("{expected:?}"),
                    Some(found.clone()),
                ),
                $module::Issue::NoVariant {
                    path,
                    found,
                    variants: _variants,
                } => (
                    "NoVariant",
                    path.to_string(),
                    String::new(),
                    Some(found.clone()),
                ),
                $module::Issue::Undescribed { reason: _reason } => {
                    ("Undescribed", String::new(), String::new(), None)
                }
                $module::Issue::Unknown { path, found } => (
                    "Unknown",
                    path.to_string(),
                    String::new(),
                    Some(found.clone()),
                ),
            })
            .collect::<Vec<Told>>()
    };
}

/// The `reason` of every issue among `$issues` that carries one, in order.
macro_rules! reasons {
    ($module:ident, $issues:expr) => {
        $issues
            .iter()
            .filter_map(|issue| {
                if let $module::Issue::Invalid {
                    path: _path,
                    expected: _expected,
                    found: _found,
                    reason,
                } = issue
                {
                    Some(reason.as_str())
                } else if let $module::Issue::Undescribed { reason } = issue {
                    Some(reason.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<&str>>()
    };
}

/// One issue less its `reason`: its kind, its path, its expected type, and the value it holds.
type Told = (&'static str, String, String, Option<Bson>);

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Version {
    number: i32,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    created_at: DateTime<Utc>,
    #[serde(rename = "recordId")]
    id: ObjectId,
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    note: Option<String>,
    versions: Vec<Version>,
}

/// Ids and numbers held one by one, in a list, a map and an optional field.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct CatalogByItem {
    counts: Vec<i32>,
    owners: HashMap<String, ObjectId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    parent: Option<ObjectId>,
    tags: Vec<ObjectId>,
}

/// One field per read hook, and one carrying a constraint, which hangs none.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Hooked {
    #[serde(deserialize_with = "upper_only")]
    code: String,
    #[serde(with = "as_text")]
    count: u32,
    #[model_schema_prop(minLength = 3)]
    name: String,
    #[serde(deserialize_with = "parsed", serialize_with = "shown")]
    port: u16,
    #[model_schema_prop(as_number)]
    seen_at: DateTime<Utc>,
}

/// A model type and a plain value, each stored under its name or an alias.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Aliased {
    #[serde(alias = "newest", default, skip_serializing_if = "Option::is_none")]
    latest: Option<Version>,
    #[serde(alias = "fullName", alias = "label")]
    name: String,
}

/// Model types reached through a map, an `Option`, a `Box` and a map of lists.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Bundle {
    boxed: Box<Version>,
    by_name: HashMap<String, Version>,
    by_team: HashMap<String, Vec<Version>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    latest: Option<Version>,
}

/// Plain values in a list of lists, an optional list and a map.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Tally {
    grid: Vec<Vec<i32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    marks: Option<Vec<i32>>,
    scores: HashMap<String, i32>,
}

/// No field serde reads: the walker has no value to read, and declares the one key written.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Derived {
    #[serde(skip_deserializing)]
    total: u32,
}

/// A type whose own `Deserialize` reads nothing, which no walk can see into.
#[model_schema(decode_with)]
#[derive(Debug, PartialEq, Serialize)]
struct Sealed {
    label: String,
}

impl<'de> Deserialize<'de> for Sealed {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Err(D::Error::custom("sealed: nothing reads this"))
    }
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Envelope {
    sealed: Sealed,
}

/// The read hook of [`Hooked::code`]: text that carries no lower-case letter.
fn upper_only<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let text = String::deserialize(deserializer)?;
    if text.chars().any(char::is_lowercase) {
        return Err(D::Error::custom(format!("{text:?} is not upper case")));
    }
    Ok(text)
}

/// A read hook generic over what it reads, which only the field it hangs on pins.
fn parsed<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: FromStr,
    T::Err: Display,
{
    String::deserialize(deserializer)?
        .parse()
        .map_err(D::Error::custom)
}

/// The write hook paired with [`parsed`].
fn shown<S, T>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
    T: Display,
{
    serializer.collect_str(value)
}

/// Whether the name `bson` is bound to version 2 of the library in the binary this is built into.
fn built_against_version_2() -> bool {
    TypeId::of::<Bson>() == TypeId::of::<bson2::Bson>()
}

/// What `value` is stored as, written through the serializer both versions of the library have.
fn written<T>(value: &T) -> Document
where
    T: Serialize,
{
    value
        .serialize(bson::Serializer::new())
        .unwrap()
        .as_document()
        .unwrap()
        .clone()
}

/// What plain serde says of `stored_row`, read through the deserializer both versions have.
fn serde_reads<T>(stored_row: &Document) -> bool
where
    T: for<'de> Deserialize<'de>,
{
    T::deserialize(bson::Deserializer::new(Bson::Document(stored_row.clone()))).is_ok()
}

fn oid(hex: &str) -> ObjectId {
    ObjectId::parse_str(hex).unwrap()
}

fn string(held: &str) -> Bson {
    Bson::String(held.to_owned())
}

fn invalid(path: &str, expected: &str, found: Bson) -> Told {
    ("Invalid", path.to_owned(), expected.to_owned(), Some(found))
}

fn missing(path: &str, expected: &str) -> Told {
    ("Missing", path.to_owned(), expected.to_owned(), None)
}

fn mistyped(path: &str, expected: &str, found: Bson) -> Told {
    (
        "Mistyped",
        path.to_owned(),
        expected.to_owned(),
        Some(found),
    )
}

fn unknown(path: &str, found: Bson) -> Told {
    ("Unknown", path.to_owned(), String::new(), Some(found))
}

fn record(numbers: &[i32]) -> Record {
    Record {
        created_at: DateTime::from_timestamp_millis(1_759_600_000_000).unwrap(),
        id: oid("6a7cc592ca0574e6efdfe217"),
        name: "Loan".to_owned(),
        note: None,
        versions: numbers.iter().map(|&number| Version { number }).collect(),
    }
}

/// A row as `Record` writes it, holding one version.
fn plain_row() -> Document {
    doc! {
        "recordId": oid("6a7cc592ca0574e6efdfe217"),
        "name": "Loan",
        "createdAt": "2025-10-04T17:46:40Z",
        "versions": [{ "number": 1_i32 }],
    }
}

fn hooked() -> Hooked {
    Hooked {
        code: "AB".to_owned(),
        count: 5,
        name: "Loan".to_owned(),
        port: 80,
        seen_at: DateTime::from_timestamp_millis(1_759_600_000_000).unwrap(),
    }
}

/// Repairs what older writers left in a `Record` row, inner versions included, and rejects
/// anything else.
fn repair_record(
    row: &mut Document,
    issues: &[record_schema::Issue<Bson>],
) -> record_schema::Verdict {
    use record_schema::{Expected, Issue, Verdict};

    for issue in issues {
        if let Issue::Mistyped {
            path,
            expected: Expected::ObjectId,
            found: Bson::String(hex),
        } = issue
        {
            let Ok(id) = ObjectId::parse_str(hex) else {
                return Verdict::Reject;
            };
            path.set_in_document(row, Bson::ObjectId(id));
        } else if let Issue::Invalid {
            path,
            expected: Expected::DateTime,
            found: Bson::DateTime(date),
            reason: _reason,
        } = issue
        {
            let Some(created) = DateTime::<Utc>::from_timestamp_millis(date.timestamp_millis())
            else {
                return Verdict::Reject;
            };
            path.set_in_document(row, Bson::String(created.to_rfc3339()));
        } else if let Issue::Invalid {
            path,
            expected: Expected::DateTime,
            found: Bson::Int64(millis),
            reason: _reason,
        } = issue
        {
            let Some(created) = DateTime::<Utc>::from_timestamp_millis(*millis) else {
                return Verdict::Reject;
            };
            path.set_in_document(row, Bson::String(created.to_rfc3339()));
        } else if let Issue::Invalid {
            path,
            expected: Expected::I32,
            found: Bson::String(text),
            reason: _reason,
        } = issue
        {
            let Ok(number) = text.parse::<i32>() else {
                return Verdict::Reject;
            };
            path.set_in_document(row, Bson::Int32(number));
        } else if let Issue::Unknown {
            path,
            found: _found,
        } = issue
        {
            path.remove_from_document(row);
        } else {
            return Verdict::Reject;
        }
    }
    Verdict::Fixed
}

fn reject(_row: &mut Document, _issues: &[record_schema::Issue<Bson>]) -> record_schema::Verdict {
    record_schema::Verdict::Reject
}

/// Reads `stored_row` with `decide`, and returns the read, what `decide` was handed less each
/// `reason`, and how many times it ran.
fn read_with(
    stored_row: Document,
    decide: fn(&mut Document, &[record_schema::Issue<Bson>]) -> record_schema::Verdict,
) -> (
    Result<Record, record_schema::Unrecovered<Bson>>,
    Vec<Told>,
    u32,
) {
    let mut seen: Vec<Told> = Vec::new();
    let mut calls = 0_u32;
    let read = Record::from_bson_with(stored_row, |raw, found| {
        calls += 1;
        seen = told!(record_schema, found);
        decide(raw, found)
    });
    (read, seen, calls)
}

/// Bson 1: the happy path, written by serde itself.
#[test]
fn bson_1_what_serde_wrote_is_read_and_the_decider_never_runs() {
    let stored_row = written(&record(&[1_i32, 2_i32]));
    assert_eq!(
        stored_row,
        doc! {
            "createdAt": "2025-10-04T17:46:40Z",
            "recordId": oid("6a7cc592ca0574e6efdfe217"),
            "name": "Loan",
            "versions": [{ "number": 1_i32 }, { "number": 2_i32 }],
        }
    );
    let (read, seen, calls) = read_with(stored_row, reject);
    assert_eq!(read, Ok(record(&[1_i32, 2_i32])));
    assert_eq!(seen, Vec::<Told>::new());
    assert_eq!(calls, 0);
}

/// Bson 2: an id stored as its hex string where an `ObjectId` belongs. serde reads it, and a
/// MongoDB query for the id does not match it.
#[test]
fn bson_2_an_id_stored_as_text_is_mistyped_and_fixed() {
    let mut stored_row = plain_row();
    stored_row.insert("recordId", "6a7cc592ca0574e6efdfe217");
    assert!(serde_reads::<Record>(&stored_row));
    let (read, seen, calls) = read_with(stored_row, repair_record);
    assert_eq!(
        seen,
        [mistyped(
            "recordId",
            "ObjectId",
            string("6a7cc592ca0574e6efdfe217")
        )]
    );
    assert_eq!(read, Ok(record(&[1_i32])));
    assert_eq!(calls, 1);
}

/// Bson 3: a date stored as a BSON date, and one stored as epoch milliseconds.
#[test]
fn bson_3_a_date_stored_as_a_bson_date_or_as_an_epoch_is_invalid_and_fixed() {
    for date in [
        Bson::DateTime(bson::DateTime::from_millis(1_759_600_000_000)),
        Bson::Int64(1_759_600_000_000),
    ] {
        let mut stored_row = plain_row();
        stored_row.insert("createdAt", date.clone());
        assert!(!serde_reads::<Record>(&stored_row));
        let (read, seen, calls) = read_with(stored_row, repair_record);
        assert_eq!(seen, [invalid("createdAt", "DateTime", date)]);
        assert_eq!(read, Ok(record(&[1_i32])));
        assert_eq!(calls, 1);
    }
}

/// Bson 4: a malformed id the callback refuses.
#[test]
fn bson_4_a_malformed_id_is_rejected_with_the_list() {
    let mut stored_row = plain_row();
    stored_row.insert("recordId", "not-an-id");
    let (read, seen, calls) = read_with(stored_row, reject);
    let listed = [invalid("recordId", "ObjectId", string("not-an-id"))];
    assert_eq!(seen, listed);
    assert_eq!(told!(record_schema, read.unwrap_err().issues), listed);
    assert_eq!(calls, 1);
}

/// The `reason` of bson 4's issue is the library's own wording of serde's refusal, which version 3
/// writes inside a sentence of its own.
#[test]
fn a_reason_is_the_wording_of_the_bson_version_built_against() {
    let mut stored_row = plain_row();
    stored_row.insert("recordId", "not-an-id");
    let (read, _seen, _calls) = read_with(stored_row, reject);
    let wording = if built_against_version_2() {
        "invalid value: string \"not-an-id\", expected 24-character, big-endian hex string"
    } else {
        "BSON error. Kind: A deserialization-related error occurred. Message: invalid value: \
         string \"not-an-id\", expected 24-character, big-endian hex string."
    };
    assert_eq!(reasons!(record_schema, read.unwrap_err().issues), [wording]);
}

/// Bson 5: a number stored as a 64-bit integer where the field writes a 32-bit one. Numbers are
/// one bracket, so a query for the number matches it.
#[test]
fn bson_5_a_number_stored_as_another_number_type_is_no_issue() {
    let mut stored_row = plain_row();
    stored_row.insert("versions", vec![doc! { "number": 2_i64 }]);
    let (read, seen, calls) = read_with(stored_row, reject);
    assert_eq!(read, Ok(record(&[2_i32])));
    assert_eq!(seen, Vec::<Told>::new());
    assert_eq!(calls, 0);
}

/// Bson 6: a MongoDB row with the `_id` every row carries, read into a type that does not declare
/// `_id`.
#[test]
fn bson_6_a_rows_own_id_is_a_key_like_any_other() {
    let mut stored_row = plain_row();
    stored_row.insert("_id", oid("6a7cc592ca0574e6efdfe299"));
    assert!(serde_reads::<Record>(&stored_row));
    let (read, seen, calls) = read_with(stored_row, reject);
    assert_eq!(
        seen,
        [unknown(
            "_id",
            Bson::ObjectId(oid("6a7cc592ca0574e6efdfe299"))
        )]
    );
    assert_eq!(
        read,
        Err(record_schema::Unrecovered {
            issues: vec![record_schema::Issue::Unknown {
                path: record_schema::Path(vec![record_schema::Segment::Key("_id".to_owned())]),
                found: Bson::ObjectId(oid("6a7cc592ca0574e6efdfe299")),
            }],
        })
    );
    assert_eq!(calls, 1);
}

/// Bson 7: the decider, on a row with problems in the record, in one of its versions, and in the
/// row's own `_id`, which the decider can read from the row it is handed.
#[test]
fn bson_7_the_decider_repairs_a_row_with_problems_at_three_levels() {
    let stored_row = doc! {
        "_id": oid("6a7cc592ca0574e6efdfe299"),
        "recordId": "6a7cc592ca0574e6efdfe217",
        "name": "Loan",
        "createdAt": Bson::DateTime(bson::DateTime::from_millis(1_759_600_000_000)),
        "versions": [{ "number": 1_i32 }, { "number": "2", "draft": true }],
    };
    let mut seen: Vec<Told> = Vec::new();
    let mut calls = 0_u32;
    let read = Record::from_bson_with(stored_row, |raw, found| {
        calls += 1;
        seen = told!(record_schema, found);
        assert_eq!(
            raw.get_object_id("_id").ok(),
            Some(oid("6a7cc592ca0574e6efdfe299"))
        );
        repair_record(raw, found)
    });
    assert_eq!(
        seen,
        [
            invalid(
                "createdAt",
                "DateTime",
                Bson::DateTime(bson::DateTime::from_millis(1_759_600_000_000))
            ),
            mistyped("recordId", "ObjectId", string("6a7cc592ca0574e6efdfe217")),
            invalid("versions[1].number", "I32", string("2")),
            unknown("versions[1].draft", Bson::Boolean(true)),
            unknown("_id", Bson::ObjectId(oid("6a7cc592ca0574e6efdfe299"))),
        ]
    );
    assert_eq!(read, Ok(record(&[1_i32, 2_i32])));
    assert_eq!(calls, 1);
}

/// One chance: the decider fixes the id and not the number, so the read fails with what the second
/// walk finds, and the decider is not asked again.
#[test]
fn a_fixed_row_gets_one_more_read_and_fails_with_the_second_list() {
    fn ids_only(
        raw: &mut Document,
        found: &[record_schema::Issue<Bson>],
    ) -> record_schema::Verdict {
        for issue in found {
            if let record_schema::Issue::Mistyped {
                path,
                expected: _expected,
                found: Bson::String(hex),
            } = issue
            {
                path.set_in_document(raw, Bson::ObjectId(oid(hex)));
            }
        }
        record_schema::Verdict::Fixed
    }

    let mut stored_row = plain_row();
    stored_row.insert("recordId", "6a7cc592ca0574e6efdfe217");
    stored_row.insert("versions", vec![doc! { "number": "two" }]);
    let (read, seen, calls) = read_with(stored_row, ids_only);
    assert_eq!(seen.len(), 2);
    assert_eq!(
        told!(record_schema, read.unwrap_err().issues),
        [invalid("versions[0].number", "I32", string("two"))]
    );
    assert_eq!(calls, 1);
}

/// A row serde reads is not read while the walk still lists an issue in it: `Fixed` over a row
/// left as it was fails the read with the same list.
#[test]
fn a_row_serde_reads_still_fails_after_fixed_while_an_issue_is_left() {
    fn untouched(
        _raw: &mut Document,
        _found: &[record_schema::Issue<Bson>],
    ) -> record_schema::Verdict {
        record_schema::Verdict::Fixed
    }

    let mut stored_row = plain_row();
    stored_row.insert("recordId", "6a7cc592ca0574e6efdfe217");
    assert!(serde_reads::<Record>(&stored_row));
    let (read, seen, calls) = read_with(stored_row, untouched);
    assert_eq!(
        seen,
        [mistyped(
            "recordId",
            "ObjectId",
            string("6a7cc592ca0574e6efdfe217")
        )]
    );
    assert_eq!(told!(record_schema, read.unwrap_err().issues), seen);
    assert_eq!(calls, 1);
}

/// MongoDB's comparison rule: numbers are one bracket, strings and symbols one, and every other
/// type a bracket of its own.
#[test]
fn a_stored_type_is_matched_by_the_types_in_its_own_bracket() {
    use version_schema::same_bracket;

    let numbers = [
        Bson::Int32(1),
        Bson::Int64(1),
        Bson::Double(1.0),
        Bson::Decimal128(bson::Decimal128::from_bytes([0_u8; 16])),
    ];
    let texts = [string("a"), Bson::Symbol("a".to_owned())];
    let alone = [
        Bson::Boolean(true),
        Bson::Null,
        Bson::ObjectId(oid("6a7cc592ca0574e6efdfe217")),
        Bson::DateTime(bson::DateTime::from_millis(0)),
        Bson::Array(Vec::new()),
        Bson::Document(Document::new()),
    ];
    for bracket in [numbers.as_slice(), texts.as_slice()] {
        for stored_as in bracket {
            for written_as in bracket {
                assert!(same_bracket(stored_as, written_as));
            }
        }
    }
    for number in &numbers {
        for held in &texts {
            assert!(!same_bracket(number, held));
            assert!(!same_bracket(held, number));
        }
    }
    for (stored_at, stored_as) in alone.iter().enumerate() {
        for (written_at, written_as) in alone.iter().enumerate() {
            assert_eq!(same_bracket(stored_as, written_as), stored_at == written_at);
        }
        for bracketed in numbers.iter().chain(&texts) {
            assert!(!same_bracket(stored_as, bracketed));
            assert!(!same_bracket(bracketed, stored_as));
        }
    }
}

/// serde reads a struct from a list of its fields in order, which is not the document the struct
/// writes, and refuses one stored as text.
#[test]
fn a_model_stored_as_another_type_is_mistyped_and_one_serde_refuses_is_invalid() {
    let mut stored_row = plain_row();
    stored_row.insert(
        "versions",
        vec![Bson::Array(vec![Bson::Int32(3)]), string("three")],
    );
    let (_read, seen, calls) = read_with(stored_row, reject);
    assert_eq!(
        seen,
        [
            mistyped(
                "versions[0]",
                "Model(\"Version\")",
                Bson::Array(vec![Bson::Int32(3)])
            ),
            invalid("versions[1]", "Model(\"Version\")", string("three")),
        ]
    );
    assert_eq!(calls, 1);
}

/// A key serde needs is `Missing`, and a field stored as something other than the list or the map
/// it is written as is one issue at the field, in this walker's own words.
#[test]
fn a_missing_key_and_a_field_stored_as_neither_its_list_nor_its_map_are_listed_at_the_field() {
    let mut stored_row = plain_row();
    stored_row.remove("name");
    stored_row.insert("versions", doc! { "number": 1_i32 });
    let (read, seen, _calls) = read_with(stored_row, reject);
    assert_eq!(
        seen,
        [
            missing("name", "String"),
            invalid(
                "versions",
                "Array(Model(\"Version\"))",
                Bson::Document(doc! { "number": 1_i32 })
            ),
        ]
    );
    assert_eq!(
        reasons!(record_schema, read.unwrap_err().issues),
        ["not an array"]
    );

    let catalog = CatalogByItem::from_bson_with(
        doc! { "counts": [], "owners": [], "tags": "none" },
        |_raw, _found| catalog_by_item_schema::Verdict::Reject,
    );
    let issues = catalog.unwrap_err().issues;
    assert_eq!(
        told!(catalog_by_item_schema, issues),
        [
            invalid("owners", "Map(ObjectId)", Bson::Array(Vec::new())),
            invalid("tags", "Array(ObjectId)", string("none")),
        ]
    );
    assert_eq!(
        reasons!(catalog_by_item_schema, issues),
        ["not an object", "not an array"]
    );
}

/// Each id and number is read on its own: an issue sits at the item, with the item's own type, and
/// a decider that sets each path repairs the row.
#[test]
fn a_plain_value_in_a_list_a_map_or_an_option_is_listed_and_fixed_at_its_own_path() {
    let stored_row = doc! {
        "tags": [oid("6a7cc592ca0574e6efdfe217"), "6a7cc592ca0574e6efdfe218"],
        "owners": { "alice": "6a7cc592ca0574e6efdfe219" },
        "parent": "6a7cc592ca0574e6efdfe21a",
        "counts": [1_i32, "2"],
    };
    let mut seen: Vec<Told> = Vec::new();
    let read = CatalogByItem::from_bson_with(stored_row, |raw, found| {
        use catalog_by_item_schema::{Issue, Verdict};

        seen = told!(catalog_by_item_schema, found);
        for issue in found {
            if let Issue::Mistyped {
                path,
                expected: _expected,
                found: Bson::String(hex),
            } = issue
            {
                path.set_in_document(raw, Bson::ObjectId(oid(hex)));
            } else if let Issue::Invalid {
                path,
                expected: _expected,
                found: Bson::String(written_as),
                reason: _reason,
            } = issue
            {
                let Ok(number) = written_as.parse::<i32>() else {
                    return Verdict::Reject;
                };
                path.set_in_document(raw, Bson::Int32(number));
            } else {
                return Verdict::Reject;
            }
        }
        Verdict::Fixed
    });
    assert_eq!(
        seen,
        [
            invalid("counts[1]", "I32", string("2")),
            mistyped(
                "owners.alice",
                "ObjectId",
                string("6a7cc592ca0574e6efdfe219")
            ),
            mistyped(
                "parent",
                "Optional(ObjectId)",
                string("6a7cc592ca0574e6efdfe21a")
            ),
            mistyped("tags[1]", "ObjectId", string("6a7cc592ca0574e6efdfe218")),
        ]
    );
    assert_eq!(
        read,
        Ok(CatalogByItem {
            counts: vec![1_i32, 2_i32],
            owners: HashMap::from([("alice".to_owned(), oid("6a7cc592ca0574e6efdfe219"))]),
            parent: Some(oid("6a7cc592ca0574e6efdfe21a")),
            tags: vec![
                oid("6a7cc592ca0574e6efdfe217"),
                oid("6a7cc592ca0574e6efdfe218")
            ],
        })
    );
}

/// Each read hook is the function serde's derive calls, handed the library's deserializer, so its
/// refusal is the walker's: the author's function, the author's module, a function generic over
/// what it reads, and the module `as_number` hangs.
#[test]
fn a_hooked_field_is_read_through_its_hook() {
    let date = Bson::DateTime(bson::DateTime::from_millis(1_759_600_000_000));
    let stored_row = doc! {
        "code": "ab",
        "count": 5_i32,
        "name": "Loan",
        "port": 80_i32,
        "seenAt": date.clone(),
    };
    let read = Hooked::from_bson_with(stored_row, |_raw, _found| hooked_schema::Verdict::Reject);
    let issues = read.unwrap_err().issues;
    assert_eq!(
        told!(hooked_schema, issues),
        [
            invalid("code", "String", string("ab")),
            invalid("count", "U32", Bson::Int32(5)),
            invalid("port", "U16", Bson::Int32(80)),
            invalid("seenAt", "DateTime", date),
        ]
    );
    let refusals = [
        "\"ab\" is not upper case",
        "invalid type: integer `5`, expected a string",
        "invalid type: integer `80`, expected a string",
        "expected a unix timestamp in milliseconds",
    ];
    let reasons = reasons!(hooked_schema, issues);
    assert_eq!(reasons.len(), refusals.len());
    for (reason, refusal) in reasons.iter().zip(refusals) {
        assert!(reason.contains(refusal), "for {refusal}, got: {reason}");
    }
}

/// A field written back through its own hook is compared in the type that hook writes: text for
/// the two numbers, and a number for the date.
#[test]
fn a_hooked_field_stored_as_its_hook_writes_it_is_no_issue() {
    let stored_row = doc! {
        "code": "AB",
        "count": "5",
        "name": "Loan",
        "port": "80",
        "seenAt": 1_759_600_000_000_i64,
    };
    assert_eq!(written(&hooked()), stored_row);
    let mut calls = 0_u32;
    let read = Hooked::from_bson_with(stored_row, |_raw, _found| {
        calls += 1;
        hooked_schema::Verdict::Reject
    });
    assert_eq!(read, Ok(hooked()));
    assert_eq!(calls, 0);
}

/// A constraint on a struct's field hangs no read hook: serde reads the value, so the walker does,
/// and the bound is `validate()`'s to report.
#[test]
fn a_constraint_on_a_struct_field_is_not_the_walkers_to_check() {
    let mut stored_row = written(&hooked());
    stored_row.insert("name", "Lo");
    let mut calls = 0_u32;
    let read = Hooked::from_bson_with(stored_row, |_raw, _found| {
        calls += 1;
        hooked_schema::Verdict::Reject
    })
    .unwrap();
    assert_eq!(read.name, "Lo");
    assert_eq!(calls, 0);
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    assert_eq!(
        read.validate(),
        Err(vec![
            "'name': too short: minimum length is 3, got 2".to_owned()
        ])
    );
}

/// A key stored under an alias is its field, and an issue under it is listed at the alias, so a
/// decider that sets the path fixes the key the row holds.
#[test]
fn a_key_stored_under_an_alias_is_its_field_and_an_issue_is_listed_at_the_alias() {
    for key in ["name", "fullName", "label"] {
        let mut stored_row = Document::new();
        stored_row.insert(key, "Ada");
        let mut calls = 0_u32;
        let read = Aliased::from_bson_with(stored_row, |_raw, _found| {
            calls += 1;
            aliased_schema::Verdict::Reject
        });
        assert_eq!(
            read,
            Ok(Aliased {
                latest: None,
                name: "Ada".to_owned()
            })
        );
        assert_eq!(calls, 0);
    }

    let stored_row = doc! { "fullName": 7_i32, "newest": { "number": "1" } };
    let mut seen: Vec<Told> = Vec::new();
    let read = Aliased::from_bson_with(stored_row, |raw, found| {
        seen = told!(aliased_schema, found);
        for issue in found {
            if let aliased_schema::Issue::Invalid {
                path,
                expected,
                found: _found,
                reason: _reason,
            } = issue
            {
                let fixed = if *expected == aliased_schema::Expected::I32 {
                    Bson::Int32(1)
                } else {
                    string("Ada")
                };
                path.set_in_document(raw, fixed);
            }
        }
        aliased_schema::Verdict::Fixed
    });
    assert_eq!(
        seen,
        [
            invalid("newest.number", "I32", string("1")),
            invalid("fullName", "String", Bson::Int32(7)),
        ]
    );
    assert_eq!(
        read,
        Ok(Aliased {
            latest: Some(Version { number: 1 }),
            name: "Ada".to_owned()
        })
    );
}

#[test]
fn a_model_type_is_walked_through_a_map_an_option_and_a_box() {
    let stored_row = doc! {
        "boxed": { "number": "3" },
        "byName": { "first": { "number": "1" } },
        "byTeam": { "core": [{ "number": 1_i32 }, { "number": "2" }] },
        "latest": { "draft": true, "number": 2_i32 },
    };
    let read = Bundle::from_bson_with(stored_row, |_raw, _found| bundle_schema::Verdict::Reject);
    assert_eq!(
        told!(bundle_schema, read.unwrap_err().issues),
        [
            invalid("boxed.number", "I32", string("3")),
            invalid("byName.first.number", "I32", string("1")),
            invalid("byTeam.core[1].number", "I32", string("2")),
            unknown("latest.draft", Bson::Boolean(true)),
        ]
    );
}

/// An optional model type is walked when its key is there and does not hold `null`.
#[test]
fn an_optional_model_type_that_is_absent_or_null_is_no_issue() {
    for latest in [
        None,
        Some(Bson::Null),
        Some(Bson::Document(doc! { "number": 2_i32 })),
    ] {
        let mut stored_row = doc! { "boxed": { "number": 3_i32 }, "byName": {}, "byTeam": {} };
        if let Some(held) = latest {
            stored_row.insert("latest", held);
        }
        let mut calls = 0_u32;
        let read = Bundle::from_bson_with(stored_row, |_raw, _found| {
            calls += 1;
            bundle_schema::Verdict::Reject
        });
        read.unwrap();
        assert_eq!(calls, 0);
    }
}

/// A plain value inside a list of lists, an optional list or a map is read on its own, at its own
/// path, and an optional list stored as neither a list nor `null` names the `Option`.
#[test]
fn a_plain_value_in_a_nested_or_optional_list_or_a_map_is_listed_at_its_own_path() {
    let stored_row = doc! {
        "grid": [[1_i32, "x"], 7_i32],
        "marks": [true],
        "scores": { "ada": "9" },
    };
    let read = Tally::from_bson_with(stored_row, |_raw, _found| tally_schema::Verdict::Reject);
    assert_eq!(
        told!(tally_schema, read.unwrap_err().issues),
        [
            invalid("grid[0][1]", "I32", string("x")),
            invalid("grid[1]", "Array(I32)", Bson::Int32(7)),
            invalid("marks[0]", "I32", Bson::Boolean(true)),
            invalid("scores.ada", "I32", string("9")),
        ]
    );

    let unlisted = Tally::from_bson_with(
        doc! { "grid": [], "marks": "none", "scores": {} },
        |_raw, _found| tally_schema::Verdict::Reject,
    );
    assert_eq!(
        told!(tally_schema, unlisted.unwrap_err().issues),
        [invalid("marks", "Optional(Array(I32))", string("none"))]
    );

    let mut calls = 0_u32;
    let absent = Tally::from_bson_with(
        doc! { "grid": [], "marks": Bson::Null, "scores": {} },
        |_raw, _found| {
            calls += 1;
            tally_schema::Verdict::Reject
        },
    );
    assert_eq!(
        absent,
        Ok(Tally {
            grid: Vec::new(),
            marks: None,
            scores: HashMap::new(),
        })
    );
    assert_eq!(calls, 0);
}

#[test]
fn a_struct_with_no_field_to_read_still_lists_an_undeclared_key() {
    let read = Derived::from_bson_with(written(&Derived { total: 7 }), |_raw, _found| {
        derived_schema::Verdict::Reject
    });
    assert_eq!(read, Ok(Derived { total: 0 }));
    let keyed = Derived::from_bson_with(doc! { "extra": true }, |_raw, _found| {
        derived_schema::Verdict::Reject
    });
    assert_eq!(
        told!(derived_schema, keyed.unwrap_err().issues),
        [unknown("extra", Bson::Boolean(true))]
    );
}

/// serde refuses the row and the walk has nothing to say why: one `Undescribed`, carrying serde's
/// message as the library words it.
#[test]
fn a_refusal_the_walk_cannot_see_into_is_undescribed() {
    let read = Envelope::from_bson_with(doc! { "sealed": { "label": "x" } }, |_raw, _found| {
        envelope_schema::Verdict::Reject
    });
    let issues = read.unwrap_err().issues;
    assert_eq!(
        told!(envelope_schema, issues),
        [("Undescribed", String::new(), String::new(), None)]
    );
    let reasons = reasons!(envelope_schema, issues);
    assert_eq!(reasons.len(), 1);
    assert!(
        reasons
            .iter()
            .all(|reason| reason.contains("sealed: nothing reads this")),
        "got: {reasons:?}"
    );
}

/// `Path` puts a value at the place an issue names inside a document, and takes one away from it.
#[test]
fn a_path_sets_and_removes_inside_a_bson_document() {
    use record_schema::{Path, Segment};

    let key = |name: &str| Segment::Key(name.to_owned());
    let number = Path(vec![key("versions"), Segment::Index(1), key("number")]);
    let mut raw = doc! { "versions": [{ "number": 1_i32 }, {}] };
    assert!(number.set_in_document(&mut raw, Bson::Int32(2)));
    assert_eq!(
        raw,
        doc! { "versions": [{ "number": 1_i32 }, { "number": 2_i32 }] }
    );
    assert!(number.set_in_document(&mut raw, Bson::Int32(3)));
    assert_eq!(
        raw,
        doc! { "versions": [{ "number": 1_i32 }, { "number": 3_i32 }] }
    );
    assert!(number.remove_from_document(&mut raw));
    assert!(!number.remove_from_document(&mut raw));

    let third = Path(vec![key("versions"), Segment::Index(2)]);
    assert!(!third.set_in_document(&mut raw, Bson::Null));
    assert!(!third.remove_from_document(&mut raw));
    let first = Path(vec![key("versions"), Segment::Index(0)]);
    assert!(first.set_in_document(&mut raw, Bson::Null));
    assert_eq!(raw, doc! { "versions": [Bson::Null, {}] });
    assert!(first.remove_from_document(&mut raw));
    assert_eq!(raw, doc! { "versions": [{}] });

    let name = Path(vec![key("name")]);
    assert!(name.set_in_document(&mut raw, string("Loan")));
    assert!(name.set_in_document(&mut raw, string("Other")));
    assert_eq!(raw, doc! { "versions": [{}], "name": "Other" });

    // A key is looked up in a document and a position in a list, and neither under a plain value.
    for nowhere in [
        Path(vec![key("absent"), key("inner")]),
        Path(vec![key("name"), key("inner")]),
        Path(vec![key("versions"), key("inner")]),
        Path(vec![key("versions"), Segment::Index(0), Segment::Index(0)]),
        Path(vec![Segment::Index(0)]),
        Path::default(),
    ] {
        assert!(!nowhere.set_in_document(&mut raw, Bson::Null));
        assert!(!nowhere.remove_from_document(&mut raw));
    }

    assert!(name.remove_from_document(&mut raw));
    assert!(!name.remove_from_document(&mut raw));
    assert_eq!(raw, doc! { "versions": [{}] });
}

/// `?` carries a failed read into a caller's own error type, and it displays one line per issue.
#[test]
fn unrecovered_is_an_error_a_caller_can_propagate() {
    fn read(stored_row: Document) -> Result<Record, Box<dyn Error>> {
        Ok(Record::from_bson_with(stored_row, reject)?)
    }

    let mut stored_row = plain_row();
    stored_row.remove("name");
    stored_row.insert("legacyField", true);
    let failure = read(stored_row).unwrap_err();
    assert_eq!(
        failure.to_string(),
        "name: missing: expected String\nlegacyField: unknown: found Boolean(true)"
    );
    assert!(failure.source().is_none());
}
