//! `from_value_with` on structs with named fields, held to what plain serde says of the same
//! value: a value serde reads in the form its type writes never reaches the callback, and every
//! other one reaches it once, with each issue at its own path.
//!
//! An issue list is asserted through `Unrecovered`'s own `Display`, one line per issue, since each
//! flagged type has its own `Issue` and no helper here could name them all.

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
mod aliased;
mod bounds;
mod enum_shapes;
mod flattened;
mod flattened_shapes;
mod generic_types;
mod hook_names;
mod method_parameter_names;
mod piped;
mod shadowing;
#[cfg(all(feature = "chrono", feature = "mongodb"))]
mod stored_record;
mod struct_shapes;

use alloc::borrow::Cow;
use core::error::Error;
use core::fmt::Display;
use core::str::FromStr;
use core::sync::atomic::{AtomicUsize, Ordering};
use std::collections::HashMap;

use serde::de::Error as _;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

/// How many times [`counted`] has run. One test reads [`Counted`], and nothing else runs it.
static NUMBER_READS: AtomicUsize = AtomicUsize::new(0);

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Version {
    number: i32,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Ledger {
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    note: Option<String>,
    versions: Vec<Version>,
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
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct Aliased {
    #[serde(alias = "fullName", alias = "label")]
    name: String,
}

/// A list and an optional model type, each stored under its name or its alias.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct AliasedParts {
    #[serde(alias = "newest", default, skip_serializing_if = "Option::is_none")]
    latest: Option<Version>,
    #[serde(alias = "tagList")]
    tags: Vec<String>,
}

/// A field serde writes wherever a predicate lets it be, and never reads.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Kept {
    id: String,
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct KeptText {
    id: String,
    #[serde(skip_deserializing, skip_serializing_if = "String::is_empty")]
    note: String,
}

/// Two fields serde neither writes nor reads: one under `skip`, one under its two halves.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Shelved {
    #[serde(skip)]
    cached: u8,
    id: String,
    #[serde(skip_serializing, skip_deserializing)]
    local: u8,
}

/// A variant's field serde writes wherever a predicate lets it be, and never reads.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum Memo {
    Noted {
        id: String,
        #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
        note: Option<String>,
    },
    Plain,
}

/// A slot serde writes wherever a predicate lets it be, and never reads. A schema surface refuses
/// the declaration.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Duo(
    String,
    #[serde(skip_deserializing, skip_serializing_if = "String::is_empty")] String,
);

/// Every way serde reads a field whose key is not there.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Sparse {
    #[serde(skip_deserializing)]
    computed: String,
    #[serde(default)]
    count: u32,
    #[serde(skip)]
    local: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    maybe: Option<String>,
    required: String,
}

#[model_schema(decode_with)]
#[derive(Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
struct Defaulted {
    count: u32,
    name: String,
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

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Strict {
    name: String,
}

/// Model types reached through a map, an `Option`, a `Box` and a list of lists.
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

/// Plain values in a list, a map, a list of lists and an optional list.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Tally {
    counts: Vec<i32>,
    grid: Vec<Vec<i32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    marks: Option<Vec<i32>>,
    scores: HashMap<String, i32>,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Shelf {
    payload: Value,
    title: String,
}

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Node {
    children: Vec<Self>,
    label: String,
}

/// No field serde reads: the walker has no value to read, and declares the one key written.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Derived {
    #[serde(skip_deserializing)]
    total: u32,
}

/// Flagged and never read: what the flag generates for it is never called.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Unread {
    label: String,
}

/// One field read through a hook that counts its own runs.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Counted {
    #[serde(deserialize_with = "counted")]
    number: i32,
}

/// Two fields that name `'static` and borrow nothing from the value read: a `Cow`, which serde
/// reads as an owned value, and a reference serde never reads.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Lettered {
    label: Cow<'static, str>,
    #[serde(skip)]
    origin: &'static str,
}

#[cfg(feature = "chrono")]
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Stamped {
    #[model_schema_prop(as_number)]
    at: chrono::DateTime<chrono::Utc>,
}

/// The read hook of [`Counted::number`]: the field type's own reader, counted.
fn counted<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    NUMBER_READS.fetch_add(1, Ordering::Relaxed);
    i32::deserialize(deserializer)
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

/// One line per issue, as `Unrecovered` displays them.
fn lines(report: &impl Display) -> Vec<String> {
    report.to_string().lines().map(str::to_owned).collect()
}

fn ledger() -> Ledger {
    Ledger {
        name: "Loan".to_owned(),
        note: None,
        versions: vec![Version { number: 1_i32 }, Version { number: 2_i32 }],
    }
}

/// Reads `stored` as a `Ledger` with a decider that answers `verdict` and changes nothing, and
/// returns the read, what the decider was handed, and how many times it ran.
fn ledger_read(
    stored: Value,
    verdict: ledger_schema::Verdict,
) -> (
    Result<Ledger, ledger_schema::Unrecovered<Value>>,
    Vec<String>,
    u32,
) {
    let mut seen: Vec<String> = Vec::new();
    let mut calls = 0_u32;
    let read = Ledger::from_value_with(stored, |_raw, found| {
        calls += 1;
        seen = lines(&ledger_schema::Unrecovered {
            issues: found.to_vec(),
        });
        verdict
    });
    (read, seen, calls)
}

#[test]
fn a_value_serde_wrote_is_read_and_the_decider_never_runs() {
    let stored = serde_json::to_value(ledger()).unwrap();
    let (read, seen, calls) = ledger_read(stored, ledger_schema::Verdict::Reject);
    assert_eq!(read, Ok(ledger()));
    assert_eq!(seen, Vec::<String>::new());
    assert_eq!(calls, 0);
}

#[test]
fn every_issue_reaches_the_decider_once_and_a_fixed_value_is_read() {
    let stored = json!({
        "name": "Loan",
        "versions": [{ "number": 1_i32 }, { "number": "2" }],
        "legacyField": true,
    });
    let mut seen: Vec<String> = Vec::new();
    let mut calls = 0_u32;
    let read = Ledger::from_value_with(stored, |raw, found| {
        calls += 1;
        seen = lines(&ledger_schema::Unrecovered {
            issues: found.to_vec(),
        });
        for issue in found {
            if let ledger_schema::Issue::Invalid {
                path,
                expected: ledger_schema::Expected::I32,
                found: Value::String(text),
                reason: _reason,
            } = issue
            {
                let Ok(number) = text.parse::<i32>() else {
                    return ledger_schema::Verdict::Reject;
                };
                path.set_in_value(raw, Value::from(number));
            } else if let ledger_schema::Issue::Unknown {
                path,
                found: _found,
            } = issue
            {
                path.remove_from_value(raw);
            } else {
                return ledger_schema::Verdict::Reject;
            }
        }
        ledger_schema::Verdict::Fixed
    });
    assert_eq!(
        seen,
        [
            "versions[1].number: invalid: expected I32, found String(\"2\"): invalid type: string \"2\", expected i32",
            "legacyField: unknown: found Bool(true)",
        ]
    );
    assert_eq!(read, Ok(ledger()));
    assert_eq!(calls, 1);
}

#[test]
fn a_missing_key_is_listed_and_a_rejected_read_fails_with_the_list() {
    let stored = json!({ "versions": [] });
    let (read, seen, calls) = ledger_read(stored, ledger_schema::Verdict::Reject);
    assert_eq!(seen, ["name: missing: expected String"]);
    assert_eq!(
        read,
        Err(ledger_schema::Unrecovered {
            issues: vec![ledger_schema::Issue::Missing {
                path: ledger_schema::Path(vec![ledger_schema::Segment::Key("name".to_owned())]),
                expected: ledger_schema::Expected::String,
            }],
        })
    );
    assert_eq!(calls, 1);
}

/// The decider fixes one of two problems and answers `Fixed`: the read fails with what the second
/// walk finds, and the decider is not asked again.
#[test]
fn a_fixed_value_gets_one_more_read_and_fails_with_the_second_list() {
    let stored = json!({ "name": 7_i32, "versions": [{ "number": "two" }] });
    let mut calls = 0_u32;
    let read = Ledger::from_value_with(stored, |raw, found| {
        calls += 1;
        for issue in found {
            if let ledger_schema::Issue::Invalid {
                path,
                expected: ledger_schema::Expected::String,
                found: _found,
                reason: _reason,
            } = issue
            {
                path.set_in_value(raw, Value::from("Loan"));
            }
        }
        ledger_schema::Verdict::Fixed
    });
    assert_eq!(
        lines(&read.unwrap_err()),
        [
            "versions[0].number: invalid: expected I32, found String(\"two\"): invalid type: string \"two\", expected i32"
        ]
    );
    assert_eq!(calls, 1);
}

/// A fix that leaves a new problem behind fails the read with that problem, the first list gone.
#[test]
fn an_issue_the_fix_itself_leaves_fails_the_read() {
    let stored = json!({ "name": "Loan", "versions": [], "legacyField": true });
    let read = Ledger::from_value_with(stored, |raw, _found| {
        ledger_schema::Path(vec![ledger_schema::Segment::Key("legacyField".to_owned())])
            .remove_from_value(raw);
        ledger_schema::Path(vec![ledger_schema::Segment::Key("name".to_owned())])
            .remove_from_value(raw);
        ledger_schema::Verdict::Fixed
    });
    assert_eq!(
        lines(&read.unwrap_err()),
        ["name: missing: expected String"]
    );
}

/// serde reads the value, so only the second walk's list can fail the read.
#[test]
fn a_value_serde_reads_still_fails_after_fixed_while_an_issue_is_left() {
    let stored = json!({ "name": "Loan", "versions": [{ "draft": true, "number": 1_i32 }] });
    serde_json::from_value::<Ledger>(stored.clone()).unwrap();
    let (read, seen, calls) = ledger_read(stored, ledger_schema::Verdict::Fixed);
    assert_eq!(seen, ["versions[0].draft: unknown: found Bool(true)"]);
    assert_eq!(lines(&read.unwrap_err()), seen);
    assert_eq!(calls, 1);
}

#[test]
fn a_record_serde_reads_with_an_undeclared_nested_key_reaches_the_decider() {
    let stored = json!({ "name": "Loan", "versions": [{ "draft": true, "number": 1_i32 }] });
    serde_json::from_value::<Ledger>(stored.clone()).unwrap();
    let (read, seen, calls) = ledger_read(stored, ledger_schema::Verdict::Reject);
    assert_eq!(seen, ["versions[0].draft: unknown: found Bool(true)"]);
    assert_eq!(lines(&read.unwrap_err()), seen);
    assert_eq!(calls, 1);
}

/// serde reads a struct from an array of its fields in order, which is not how the struct writes
/// itself, and refuses one held as text.
#[test]
fn a_model_held_in_another_shape_is_mistyped_and_one_serde_refuses_is_invalid() {
    let stored = json!({ "name": "Loan", "versions": [[3_i32], "three"] });
    let (_read, seen, calls) = ledger_read(stored, ledger_schema::Verdict::Reject);
    assert_eq!(
        seen,
        [
            "versions[0]: mistyped: expected Model(\"Version\"), found Array [Number(3)]",
            "versions[1]: invalid: expected Model(\"Version\"), found String(\"three\"): invalid type: string \"three\", expected struct Version",
        ]
    );
    assert_eq!(calls, 1);
}

#[test]
fn a_value_that_is_no_object_is_listed_at_the_value_itself() {
    let read = Version::from_value_with(json!("three"), |_raw, _found| {
        version_schema::Verdict::Reject
    });
    assert_eq!(
        lines(&read.unwrap_err()),
        [
            "the value itself: invalid: expected Model(\"Version\"), found String(\"three\"): invalid type: string \"three\", expected struct Version"
        ]
    );
}

#[test]
fn a_list_field_that_holds_no_list_is_one_issue_at_the_field() {
    let stored = json!({ "name": "Loan", "versions": { "number": 1_i32 } });
    let (_read, seen, _calls) = ledger_read(stored, ledger_schema::Verdict::Reject);
    assert_eq!(
        seen,
        [
            "versions: invalid: expected Array(Model(\"Version\")), found Object {\"number\": Number(1)}: not an array"
        ]
    );
}

/// Each read hook is the function serde's derive calls, so its refusal is the walker's: the
/// author's function, the author's module, and a function generic over what it reads.
#[test]
fn a_hooked_field_is_read_through_its_hook() {
    let stored = json!({ "code": "ab", "count": 5_i32, "name": "Loan", "port": 80_i32 });
    let read = Hooked::from_value_with(stored, |_raw, _found| hooked_schema::Verdict::Reject);
    assert_eq!(
        lines(&read.unwrap_err()),
        [
            "code: invalid: expected String, found String(\"ab\"): \"ab\" is not upper case",
            "count: invalid: expected U32, found Number(5): invalid type: integer `5`, expected a string",
            "port: invalid: expected U16, found Number(80): invalid type: integer `80`, expected a string",
        ]
    );
}

/// A field written back through its own hook is compared in the form that hook writes.
#[test]
fn a_hooked_field_held_as_its_hook_writes_it_is_no_issue() {
    let stored = json!({ "code": "AB", "count": "5", "name": "Loan", "port": "80" });
    let mut calls = 0_u32;
    let read = Hooked::from_value_with(stored, |_raw, _found| {
        calls += 1;
        hooked_schema::Verdict::Reject
    });
    assert_eq!(
        read,
        Ok(Hooked {
            code: "AB".to_owned(),
            count: 5,
            name: "Loan".to_owned(),
            port: 80,
        })
    );
    assert_eq!(calls, 0);
}

/// A read that calls no callback runs a field's reader twice: in serde's read of the whole value,
/// and in the walk's own read of the field. The report reads nothing: it is told what serde said.
#[test]
fn a_read_that_calls_no_callback_runs_a_fields_reader_twice() {
    let mut calls = 0_u32;
    let read = Counted::from_value_with(json!({ "number": 7_i32 }), |_raw, _found| {
        calls += 1;
        counted_schema::Verdict::Reject
    });
    assert_eq!(read, Ok(Counted { number: 7 }));
    assert_eq!(calls, 0);
    assert_eq!(NUMBER_READS.load(Ordering::Relaxed), 2);
}

/// A constraint on a struct's field hangs no read hook: serde reads the value, and the walker
/// holds what it read to the bound, where a schema surface is on to publish its validator.
#[test]
fn a_constraint_on_a_struct_field_is_the_walkers_to_check() {
    let stored = json!({ "code": "AB", "count": "5", "name": "ab", "port": "80" });
    assert_eq!(
        serde_json::from_value::<Hooked>(stored.clone())
            .unwrap()
            .name,
        "ab"
    );
    let mut calls = 0_u32;
    let read = Hooked::from_value_with(stored, |_raw, _found| {
        calls += 1;
        hooked_schema::Verdict::Reject
    });
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    {
        assert_eq!(
            lines(&read.unwrap_err()),
            [
                "name: invalid: expected String, found String(\"ab\"): too short: minimum length \
                 is 3, got 2"
            ]
        );
        assert_eq!(calls, 1);
    }
    #[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
    {
        assert_eq!(read.unwrap().name, "ab");
        assert_eq!(calls, 0);
    }
}

#[cfg(feature = "chrono")]
#[test]
fn an_as_number_field_is_read_through_the_hook_as_number_hangs() {
    let read = Stamped::from_value_with(json!({ "at": "2025-10-04T17:46:40Z" }), |_raw, _found| {
        stamped_schema::Verdict::Reject
    });
    assert_eq!(
        lines(&read.unwrap_err()),
        [
            "at: invalid: expected DateTime, found String(\"2025-10-04T17:46:40Z\"): invalid type: string \"2025-10-04T17:46:40Z\", expected a unix timestamp in milliseconds"
        ]
    );
    let stored = json!({ "at": 1_759_600_000_000_i64 });
    let stamped =
        Stamped::from_value_with(stored, |_raw, _found| stamped_schema::Verdict::Reject).unwrap();
    assert_eq!(stamped.at.timestamp_millis(), 1_759_600_000_000);
}

#[test]
fn a_key_stored_under_an_alias_is_its_field() {
    for stored in [
        json!({ "name": "Ada" }),
        json!({ "fullName": "Ada" }),
        json!({ "label": "Ada" }),
    ] {
        let mut calls = 0_u32;
        let read = Aliased::from_value_with(stored, |_raw, _found| {
            calls += 1;
            aliased_schema::Verdict::Reject
        });
        assert_eq!(
            read,
            Ok(Aliased {
                name: "Ada".to_owned()
            })
        );
        assert_eq!(calls, 0);
    }
}

/// The path is the key the value is stored under, so a decider that sets it fixes the stored key.
#[test]
fn an_issue_under_an_alias_is_listed_at_the_alias() {
    let read = Aliased::from_value_with(json!({ "fullName": 7_i32 }), |raw, found| {
        for issue in found {
            if let aliased_schema::Issue::Invalid {
                path,
                expected: _expected,
                found: _found,
                reason: _reason,
            } = issue
            {
                assert_eq!(path.to_string(), "fullName");
                path.set_in_value(raw, Value::from("Ada"));
            }
        }
        aliased_schema::Verdict::Fixed
    });
    assert_eq!(
        read,
        Ok(Aliased {
            name: "Ada".to_owned()
        })
    );
    let absent =
        Aliased::from_value_with(json!({}), |_raw, _found| aliased_schema::Verdict::Reject);
    assert_eq!(
        lines(&absent.unwrap_err()),
        ["name: missing: expected String"]
    );
}

/// A value walked into under an alias is listed under that alias, item by item.
#[test]
fn a_list_and_a_model_type_stored_under_an_alias_are_walked_at_the_alias() {
    let stored = json!({ "newest": { "number": "1" }, "tagList": ["a", 2_i32] });
    let read =
        AliasedParts::from_value_with(stored, |_raw, _found| aliased_parts_schema::Verdict::Reject);
    assert_eq!(
        lines(&read.unwrap_err()),
        [
            "newest.number: invalid: expected I32, found String(\"1\"): invalid type: string \"1\", expected i32",
            "tagList[1]: invalid: expected String, found Number(2): invalid type: integer `2`, expected a string",
        ]
    );
    let absent = AliasedParts::from_value_with(json!({ "newest": null }), |_raw, _found| {
        aliased_parts_schema::Verdict::Reject
    });
    assert_eq!(
        lines(&absent.unwrap_err()),
        ["tags: missing: expected Array(String)"]
    );
}

/// An `Option`, a `default` and a `skip_deserializing` field are read with their key missing, and
/// only the required one is reported.
#[test]
fn a_key_serde_reads_the_absence_of_is_not_missing() {
    let mut calls = 0_u32;
    let read = Sparse::from_value_with(json!({ "required": "here" }), |_raw, _found| {
        calls += 1;
        sparse_schema::Verdict::Reject
    });
    assert_eq!(
        read,
        Ok(Sparse {
            computed: String::new(),
            count: 0,
            local: 0,
            maybe: None,
            required: "here".to_owned(),
        })
    );
    assert_eq!(calls, 0);

    let empty = Sparse::from_value_with(json!({}), |_raw, _found| sparse_schema::Verdict::Reject);
    assert_eq!(
        lines(&empty.unwrap_err()),
        ["required: missing: expected String"]
    );
}

/// What the type writes for itself carries the key of a field it never reads back, which is a key
/// it declares. A field serde neither writes nor reads declares none.
#[test]
fn a_key_the_type_writes_and_does_not_read_is_declared() {
    let written = serde_json::to_value(Sparse {
        computed: "sum".to_owned(),
        count: 3,
        local: 9,
        maybe: Some("x".to_owned()),
        required: "here".to_owned(),
    })
    .unwrap();
    assert_eq!(
        written,
        json!({ "computed": "sum", "count": 3_i32, "maybe": "x", "required": "here" })
    );
    let mut calls = 0_u32;
    let read = Sparse::from_value_with(written, |_raw, _found| {
        calls += 1;
        sparse_schema::Verdict::Reject
    });
    read.unwrap();
    assert_eq!(calls, 0);

    let skipped = Sparse::from_value_with(
        json!({ "local": 9_i32, "required": "here" }),
        |_raw, _found| sparse_schema::Verdict::Reject,
    );
    assert_eq!(
        lines(&skipped.unwrap_err()),
        ["local: unknown: found Number(9)"]
    );
}

/// A field serde writes wherever its predicate lets it be and never reads has its key in what the
/// type wrote, which is a key the type declares. Where the predicate held it back, nothing changes.
#[test]
fn a_key_the_type_writes_under_a_predicate_and_does_not_read_is_declared() {
    let kept = Kept {
        id: "i".to_owned(),
        note: Some("n".to_owned()),
    };
    let written = serde_json::to_value(&kept).unwrap();
    assert_eq!(written, json!({ "id": "i", "note": "n" }));
    let mut calls = 0_u32;
    let read = Kept::from_value_with(written, |_raw, _found| {
        calls += 1;
        kept_schema::Verdict::Reject
    });
    let unread = Kept {
        id: "i".to_owned(),
        note: None,
    };
    assert_eq!(read.as_ref(), Ok(&unread));
    let held_back = Kept::from_value_with(json!({ "id": "i" }), |_raw, _found| {
        calls += 1;
        kept_schema::Verdict::Reject
    });
    assert_eq!(held_back, Ok(unread));

    let text = serde_json::to_value(KeptText {
        id: "i".to_owned(),
        note: "n".to_owned(),
    })
    .unwrap();
    assert_eq!(text, json!({ "id": "i", "note": "n" }));
    let read_text = KeptText::from_value_with(text, |_raw, _found| {
        calls += 1;
        kept_text_schema::Verdict::Reject
    });
    assert_eq!(
        read_text,
        Ok(KeptText {
            id: "i".to_owned(),
            note: String::new(),
        })
    );
    assert_eq!(calls, 0);
}

/// A variant's field under the same attributes is a key the variant declares.
#[test]
fn a_key_a_variant_writes_under_a_predicate_and_does_not_read_is_declared() {
    let written = serde_json::to_value(Memo::Noted {
        id: "i".to_owned(),
        note: Some("n".to_owned()),
    })
    .unwrap();
    assert_eq!(written, json!({ "Noted": { "id": "i", "note": "n" } }));
    let mut calls = 0_u32;
    let read = Memo::from_value_with(written, |_raw, _found| {
        calls += 1;
        memo_schema::Verdict::Reject
    });
    assert_eq!(
        read,
        Ok(Memo::Noted {
            id: "i".to_owned(),
            note: None,
        })
    );
    assert_eq!(calls, 0);
}

/// A field serde neither writes nor reads declares no key, under `skip` or under
/// `skip_serializing` beside `skip_deserializing`: its key in a record is one serde reads past.
#[test]
fn a_key_of_a_field_the_type_neither_writes_nor_reads_is_unknown() {
    let stored = json!({ "cached": 1_i32, "id": "i", "local": 9_i32 });
    assert_eq!(
        Shelved::deserialize(&stored).unwrap(),
        Shelved {
            cached: 0,
            id: "i".to_owned(),
            local: 0,
        }
    );
    let read = Shelved::from_value_with(stored, |_raw, _found| shelved_schema::Verdict::Reject);
    assert_eq!(
        lines(&read.unwrap_err()),
        [
            "cached: unknown: found Number(1)",
            "local: unknown: found Number(9)"
        ]
    );
}

/// serde reads no array it wrote with such a slot in it: it writes the slot and reads an array
/// without it. The walker refuses the array as serde does, and reads one written without the slot.
#[cfg(not(any(feature = "typescript", feature = "zod", feature = "jsonschema")))]
#[test]
fn a_slot_the_type_writes_under_a_predicate_and_does_not_read_is_refused_as_serde_refuses_it() {
    let written = serde_json::to_value(Duo("a".to_owned(), "n".to_owned())).unwrap();
    assert_eq!(written, json!(["a", "n"]));
    let refusal = Duo::deserialize(&written).unwrap_err().to_string();
    let read = Duo::from_value_with(written, |_raw, _found| duo_schema::Verdict::Reject);
    assert_eq!(
        lines(&read.unwrap_err()),
        [
            "[1]: unknown: found String(\"n\")".to_owned(),
            format!("undescribed: {refusal}")
        ]
    );
    let mut calls = 0_u32;
    let short = Duo::from_value_with(json!(["a"]), |_raw, _found| {
        calls += 1;
        duo_schema::Verdict::Reject
    });
    assert_eq!(short, Ok(Duo("a".to_owned(), String::new())));
    assert_eq!(calls, 0);
}

#[test]
fn a_container_default_reads_every_missing_key() {
    let mut calls = 0_u32;
    let read = Defaulted::from_value_with(json!({}), |_raw, _found| {
        calls += 1;
        defaulted_schema::Verdict::Reject
    });
    assert_eq!(read, Ok(Defaulted::default()));
    assert_eq!(calls, 0);
}

/// serde refuses the value and the walk has nothing to say why: one `Undescribed`, with serde's
/// own message.
#[test]
fn a_refusal_the_walk_cannot_see_into_is_undescribed() {
    let stored = json!({ "sealed": { "label": "x" } });
    let read = Envelope::from_value_with(stored, |_raw, _found| envelope_schema::Verdict::Reject);
    assert_eq!(
        read,
        Err(envelope_schema::Unrecovered {
            issues: vec![envelope_schema::Issue::Undescribed {
                reason: "sealed: nothing reads this".to_owned(),
            }],
        })
    );
}

#[test]
fn an_undeclared_key_serde_itself_refuses_is_listed_beside_serdes_message() {
    let stored = json!({ "name": "Loan", "extra": 1_i32 });
    let read = Strict::from_value_with(stored, |_raw, _found| strict_schema::Verdict::Reject);
    assert_eq!(
        lines(&read.unwrap_err()),
        [
            "extra: unknown: found Number(1)",
            "undescribed: unknown field `extra`, expected `name`",
        ]
    );
}

#[test]
fn a_model_type_is_walked_through_a_map_an_option_and_a_box() {
    let stored = json!({
        "boxed": { "number": "3" },
        "byName": { "first": { "number": "1" } },
        "byTeam": { "core": [{ "number": 1_i32 }, { "number": "2" }] },
        "latest": { "draft": true, "number": 2_i32 },
    });
    let read = Bundle::from_value_with(stored, |_raw, _found| bundle_schema::Verdict::Reject);
    assert_eq!(
        lines(&read.unwrap_err()),
        [
            "boxed.number: invalid: expected I32, found String(\"3\"): invalid type: string \"3\", expected i32",
            "byName.first.number: invalid: expected I32, found String(\"1\"): invalid type: string \"1\", expected i32",
            "byTeam.core[1].number: invalid: expected I32, found String(\"2\"): invalid type: string \"2\", expected i32",
            "latest.draft: unknown: found Bool(true)",
        ]
    );
}

/// An optional model type is walked when its key is there and does not hold `null`.
#[test]
fn an_optional_model_type_that_is_absent_or_null_is_no_issue() {
    for latest in [json!(null), json!({ "number": 2_i32 })] {
        let stored = json!({
            "boxed": { "number": 3_i32 },
            "byName": {},
            "byTeam": {},
            "latest": latest,
        });
        let mut calls = 0_u32;
        let read = Bundle::from_value_with(stored, |_raw, _found| {
            calls += 1;
            bundle_schema::Verdict::Reject
        });
        read.unwrap();
        assert_eq!(calls, 0);
    }
    let stored = json!({ "boxed": { "number": 3_i32 }, "byName": [], "byTeam": {} });
    let read = Bundle::from_value_with(stored, |_raw, _found| bundle_schema::Verdict::Reject);
    assert_eq!(
        lines(&read.unwrap_err()),
        ["byName: invalid: expected Map(Model(\"Version\")), found Array []: not an object"]
    );
}

/// A plain value inside a list or a map is read on its own, at its own path, with its own type.
#[test]
fn a_plain_value_in_a_list_or_a_map_is_listed_at_its_own_path() {
    let stored = json!({
        "counts": [1_i32, "2"],
        "grid": [[1_i32, "x"], 7_i32],
        "marks": [true],
        "scores": { "ada": "9" },
    });
    let read = Tally::from_value_with(stored, |raw, found| {
        let seen = lines(&tally_schema::Unrecovered {
            issues: found.to_vec(),
        });
        assert_eq!(
            seen,
            [
                "counts[1]: invalid: expected I32, found String(\"2\"): invalid type: string \"2\", expected i32",
                "grid[0][1]: invalid: expected I32, found String(\"x\"): invalid type: string \"x\", expected i32",
                "grid[1]: invalid: expected Array(I32), found Number(7): not an array",
                "marks[0]: invalid: expected I32, found Bool(true): invalid type: boolean `true`, expected i32",
                "scores.ada: invalid: expected I32, found String(\"9\"): invalid type: string \"9\", expected i32",
            ]
        );
        for issue in found {
            if let tally_schema::Issue::Invalid {
                path,
                expected,
                found: _found,
                reason: _reason,
            } = issue
            {
                let fixed = if *expected == tally_schema::Expected::I32 {
                    json!(2_i32)
                } else {
                    json!([])
                };
                path.set_in_value(raw, fixed);
            }
        }
        tally_schema::Verdict::Fixed
    });
    assert_eq!(
        read,
        Ok(Tally {
            counts: vec![1_i32, 2_i32],
            grid: vec![vec![1_i32, 2_i32], vec![]],
            marks: Some(vec![2_i32]),
            scores: HashMap::from([("ada".to_owned(), 2_i32)]),
        })
    );
}

#[test]
fn an_optional_list_holding_neither_a_list_nor_null_names_the_option() {
    let stored = json!({ "counts": [], "grid": [], "marks": "none", "scores": {} });
    let read = Tally::from_value_with(stored, |_raw, _found| tally_schema::Verdict::Reject);
    assert_eq!(
        lines(&read.unwrap_err()),
        ["marks: invalid: expected Optional(Array(I32)), found String(\"none\"): not an array"]
    );
}

#[test]
fn a_json_value_field_holds_anything_and_is_never_listed() {
    for payload in [json!(null), json!([1_i32, "two", null]), json!({ "a": {} })] {
        let mut calls = 0_u32;
        let read = Shelf::from_value_with(
            json!({ "payload": payload, "title": "t" }),
            |_raw, _found| {
                calls += 1;
                shelf_schema::Verdict::Reject
            },
        );
        read.unwrap();
        assert_eq!(calls, 0);
    }
}

#[test]
fn a_type_that_holds_itself_is_walked_to_any_depth() {
    let stored = json!({
        "children": [{ "children": [{ "children": [], "label": 3_i32 }], "label": "a" }],
        "label": "root",
    });
    let read = Node::from_value_with(stored, |_raw, _found| node_schema::Verdict::Reject);
    assert_eq!(
        lines(&read.unwrap_err()),
        [
            "children[0].children[0].label: invalid: expected String, found Number(3): invalid type: integer `3`, expected a string"
        ]
    );
}

#[test]
fn a_struct_with_no_field_to_read_still_lists_an_undeclared_key() {
    let written = serde_json::to_value(Derived { total: 7 }).unwrap();
    let read = Derived::from_value_with(written, |_raw, _found| derived_schema::Verdict::Reject);
    assert_eq!(read, Ok(Derived { total: 0 }));
    let keyed = Derived::from_value_with(json!({ "extra": true }), |_raw, _found| {
        derived_schema::Verdict::Reject
    });
    assert_eq!(
        lines(&keyed.unwrap_err()),
        ["extra: unknown: found Bool(true)"]
    );
}

#[test]
fn a_field_that_names_a_lifetime_and_borrows_nothing_is_read() {
    let mut calls = 0_u32;
    let read = Lettered::from_value_with(json!({ "label": "x" }), |_raw, _found| {
        calls += 1;
        lettered_schema::Verdict::Reject
    });
    assert_eq!(
        read,
        Ok(Lettered {
            label: Cow::Borrowed("x"),
            origin: "",
        })
    );
    assert_eq!(calls, 0);
}

#[test]
fn a_flagged_type_never_read_is_still_a_type() {
    let unread = Unread {
        label: "kept".to_owned(),
    };
    assert_eq!(unread.label, "kept");
}

/// `Path` puts a value at the place an issue names, and takes one away from it.
#[test]
fn a_path_sets_and_removes_inside_a_json_value() {
    use ledger_schema::{Path, Segment};

    let number = Path(vec![
        Segment::Key("versions".to_owned()),
        Segment::Index(1),
        Segment::Key("number".to_owned()),
    ]);
    assert_eq!(number.to_string(), "versions[1].number");
    assert_eq!(Path::default().to_string(), "");

    let mut raw = json!({ "versions": [{ "number": 1_i32 }, {}] });
    assert!(number.set_in_value(&mut raw, json!(2_i32)));
    assert_eq!(
        raw,
        json!({ "versions": [{ "number": 1_i32 }, { "number": 2_i32 }] })
    );
    assert!(number.remove_from_value(&mut raw));
    assert!(!number.remove_from_value(&mut raw));

    let third = Path(vec![Segment::Key("versions".to_owned()), Segment::Index(2)]);
    assert!(!third.set_in_value(&mut raw, json!({})));
    assert!(!third.remove_from_value(&mut raw));
    let first = Path(vec![Segment::Key("versions".to_owned()), Segment::Index(0)]);
    assert!(first.remove_from_value(&mut raw));
    assert_eq!(raw, json!({ "versions": [{}] }));

    assert!(Path::default().set_in_value(&mut raw, json!("whole")));
    assert_eq!(raw, json!("whole"));
    assert!(!Path::default().remove_from_value(&mut raw));
    assert!(!number.set_in_value(&mut raw, json!(2_i32)));
}

/// `?` carries a failed read into a caller's own error type, and it displays one line per issue.
#[test]
fn unrecovered_is_an_error_a_caller_can_propagate() {
    fn read(stored: Value) -> Result<Ledger, Box<dyn Error>> {
        Ok(Ledger::from_value_with(stored, |_raw, _found| {
            ledger_schema::Verdict::Reject
        })?)
    }

    let failure = read(json!({ "legacyField": true, "versions": 3_i32 })).unwrap_err();
    assert_eq!(
        failure.to_string(),
        "name: missing: expected String\n\
         versions: invalid: expected Array(Model(\"Version\")), found Number(3): not an array\n\
         legacyField: unknown: found Bool(true)"
    );
    assert!(failure.source().is_none());
    assert!(
        format!("{failure:?}")
            .starts_with("Unrecovered { issues: [Missing { path: Path([Key(\"name\")])")
    );
}

/// `Expected` has these members and no other: the match names each one, so a member added or
/// taken away stops this building.
#[test]
fn expected_has_exactly_the_members_a_field_category_can_take() {
    use ledger_schema::Expected;

    let members = [
        Expected::Array(Box::new(Expected::Boolean)),
        Expected::Boolean,
        Expected::BooleanLiteral(true),
        Expected::Char,
        Expected::DateTime,
        Expected::F32,
        Expected::F64,
        Expected::I8,
        Expected::I16,
        Expected::I32,
        Expected::I64,
        Expected::Isize,
        Expected::Map(Box::new(Expected::String)),
        Expected::Model("Version"),
        Expected::NaiveDate,
        Expected::NaiveDateTime,
        Expected::NaiveTime,
        Expected::NumberLiteral(214.0),
        Expected::ObjectId,
        Expected::Optional(Box::new(Expected::String)),
        Expected::String,
        Expected::StringLiteral("Tixena"),
        Expected::Tuple(vec![Expected::String, Expected::U32]),
        Expected::TypeParam("T"),
        Expected::Unknown,
        Expected::U8,
        Expected::U16,
        Expected::U32,
        Expected::U64,
        Expected::Usize,
        Expected::Variants(&["Circle", "Label"]),
    ];
    for member in &members {
        let name = match member {
            Expected::Array(_) => "Array",
            Expected::Boolean => "Boolean",
            Expected::BooleanLiteral(_) => "BooleanLiteral",
            Expected::Char => "Char",
            Expected::DateTime => "DateTime",
            Expected::F32 => "F32",
            Expected::F64 => "F64",
            Expected::I8 => "I8",
            Expected::I16 => "I16",
            Expected::I32 => "I32",
            Expected::I64 => "I64",
            Expected::Isize => "Isize",
            Expected::Map(_) => "Map",
            Expected::Model(_) => "Model",
            Expected::NaiveDate => "NaiveDate",
            Expected::NaiveDateTime => "NaiveDateTime",
            Expected::NaiveTime => "NaiveTime",
            Expected::NumberLiteral(_) => "NumberLiteral",
            Expected::ObjectId => "ObjectId",
            Expected::Optional(_) => "Optional",
            Expected::String => "String",
            Expected::StringLiteral(_) => "StringLiteral",
            Expected::Tuple(_) => "Tuple",
            Expected::TypeParam(_) => "TypeParam",
            Expected::Unknown => "Unknown",
            Expected::U8 => "U8",
            Expected::U16 => "U16",
            Expected::U32 => "U32",
            Expected::U64 => "U64",
            Expected::Usize => "Usize",
            Expected::Variants(_) => "Variants",
        };
        assert!(format!("{member:?}").starts_with(name), "for {name}");
    }
    assert_eq!(members.len(), 31);
}

/// The handoff builds each kind of issue from the standard parts, and a kind it does not know
/// becomes `Undescribed`.
#[test]
fn issue_from_parts_builds_this_types_own_issue() {
    use ledger_schema::{Expected, Issue, Path, Segment, issue_from_parts};

    let here = || vec![Ok("versions".to_owned()), Err(1_usize)];
    let path = Path(vec![Segment::Key("versions".to_owned()), Segment::Index(1)]);
    let tokens: &'static [ledger_schema::ExpectedToken] = &[
        ("Optional", &[], 1),
        ("Array", &[], 1),
        ("Tuple", &[], 2),
        ("Model", &["Version"], 0),
        ("NumberLiteral", &["2.5"], 0),
    ];
    let expected = Expected::Optional(Box::new(Expected::Array(Box::new(Expected::Tuple(vec![
        Expected::Model("Version"),
        Expected::NumberLiteral(2.5),
    ])))));
    assert_eq!(
        issue_from_parts("Missing", here(), tokens, None::<Value>, None, Vec::new()),
        Issue::Missing {
            path: path.clone(),
            expected: expected.clone(),
        }
    );
    assert_eq!(
        issue_from_parts(
            "Mistyped",
            here(),
            tokens,
            Some(json!(1_i32)),
            None,
            Vec::new()
        ),
        Issue::Mistyped {
            path: path.clone(),
            expected,
            found: json!(1_i32),
        }
    );
    let inner = vec![Issue::Unknown {
        path: Path::default(),
        found: json!(true),
    }];
    assert_eq!(
        issue_from_parts(
            "NoVariant",
            here(),
            &[],
            Some(json!({})),
            None,
            vec![("Email", inner.clone())]
        ),
        Issue::NoVariant {
            path,
            found: json!({}),
            variants: vec![("Email", inner)],
        }
    );
    assert_eq!(
        issue_from_parts(
            "Renamed",
            here(),
            &[],
            None::<Value>,
            Some("why".to_owned()),
            Vec::new()
        ),
        Issue::Undescribed {
            reason: "why".to_owned(),
        }
    );
}
