//! A payload serde refuses, read by the emitted Dart types under the Dart VM: each refusal the read
//! writes itself is a `FormatException`, the exception `fromJson` throws for a value that breaks a
//! rule, so a caller that catches it catches each. A tag no variant has under every tagging, a
//! unit variant's entry over a value and its content under a content key, an externally tagged
//! enum read from an object of no key or of several, a payload no variant of an untagged enum
//! reads, a name no variant of a plain enum has, a range of a key it does not write, a `Result` of
//! both keys, and a tuple of another length, a longer one among them.

#![cfg(all(feature = "dart", feature = "serde"))]

use core::fmt::Write as _;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::range_models::{Spanned, spanned_dart};
use super::result_models::{Reply, reply_dart};
use super::run_dart_validate::{dart, from_until, module};

/// What prints a row: the value read, written back, or the class of what the read throws and its
/// message.
const TOLD: &str = "
void told(String at, Object? Function() read) {
  String printed;
  try {
    printed = 'read ${jsonEncode(read())}';
  } on FormatException catch (refusal) {
    printed = '${refusal.runtimeType}: ${refusal.message}';
  } catch (refusal) {
    printed = '${refusal.runtimeType}: $refusal';
  }
  print('$at: $printed');
}
";

/// Every payload: a tag no variant has, as a name and as an entry, and a unit variant's entry
/// over a value, of an externally tagged enum; a tag no variant has under a tag alone, and under
/// a tag and a content key beside a unit variant's content; a payload no variant of an untagged
/// enum reads; a name no variant of a plain enum has; a range of a key it does not write; a
/// `Result` of both keys; an externally tagged enum read from an object of several keys and of
/// none; and a tuple of another length, a variant's, a tuple struct's and a member's, the outer
/// and the inner of a tuple that holds a tuple.
const ROWS: [Row; 18] = [
    Row {
        json: r#""Gale""#,
        read_as: "Drizzling",
        serde: serde_told::<Drizzling>,
        thrown: "Unknown Drizzling: Gale",
    },
    Row {
        json: r#"{"Gale":{}}"#,
        read_as: "Drizzling",
        serde: serde_told::<Drizzling>,
        thrown: "Unknown Drizzling: Gale",
    },
    Row {
        json: r#"{"Calm":1}"#,
        read_as: "Drizzling",
        serde: serde_told::<Drizzling>,
        thrown: "Drizzling: Calm takes no value",
    },
    Row {
        json: r#"{"kind":"Gale"}"#,
        read_as: "Gusting",
        serde: serde_told::<Gusting>,
        thrown: "Unknown Gusting: Gale",
    },
    Row {
        json: r#"{"kind":"Gale","held":{}}"#,
        read_as: "Squalling",
        serde: serde_told::<Squalling>,
        thrown: "Unknown Squalling: Gale",
    },
    Row {
        json: r#"{"kind":"Calm","held":5}"#,
        read_as: "Squalling",
        serde: serde_told::<Squalling>,
        thrown: "Squalling: Calm takes no value",
    },
    Row {
        json: "true",
        read_as: "Misting",
        serde: serde_told::<Misting>,
        thrown: "No variant of Misting matched",
    },
    Row {
        json: r#""Sleet""#,
        read_as: "Hailing",
        serde: serde_told::<Hailing>,
        thrown: "Unknown Hailing: Sleet",
    },
    Row {
        json: concat!(
            r#"{"closed":{"start":1,"end":2},"from":{"start":8},"#,
            r#""span":{"start":8,"end":13,"step":1},"to":{"end":13}}"#
        ),
        read_as: "Spanned",
        serde: serde_told::<Spanned>,
        thrown: "Unknown key of a Range: (start, end, step)",
    },
    Row {
        json: r#"{"outcome":{"Ok":7,"Err":"x"},"stored":{"Ok":null}}"#,
        read_as: "Reply",
        serde: serde_told::<Reply>,
        thrown: "Not a Result: (Ok, Err)",
    },
    Row {
        json: r#"{"Boxed":"a","Split":["b",1]}"#,
        read_as: "Fogging",
        serde: serde_told::<Fogging>,
        thrown: "more than one key for Fogging",
    },
    Row {
        json: "{}",
        read_as: "Fogging",
        serde: serde_told::<Fogging>,
        thrown: "empty object for Fogging",
    },
    Row {
        json: r#"{"Split":["b",1,2]}"#,
        read_as: "Fogging",
        serde: serde_told::<Fogging>,
        thrown: "invalid length 3, expected 2",
    },
    Row {
        json: r#"{"Split":["b"]}"#,
        read_as: "Fogging",
        serde: serde_told::<Fogging>,
        thrown: "invalid length 1, expected 2",
    },
    Row {
        json: r#"["b",1,2]"#,
        read_as: "Thawing",
        serde: serde_told::<Thawing>,
        thrown: "invalid length 3, expected 2",
    },
    Row {
        json: r#"["b"]"#,
        read_as: "Thawing",
        serde: serde_told::<Thawing>,
        thrown: "invalid length 1, expected 2",
    },
    Row {
        json: r#"{"reading":[["b",1,2],3]}"#,
        read_as: "Frosting",
        serde: serde_told::<Frosting>,
        thrown: "invalid length 3, expected 2",
    },
    Row {
        json: r#"{"reading":[["b",1]]}"#,
        read_as: "Frosting",
        serde: serde_told::<Frosting>,
        thrown: "invalid length 1, expected 2",
    },
];

/// Payloads each type reads, as serde reads them: an externally tagged enum from an object of one
/// key, a tuple from an array of its length, and an enum declared below its holder from the
/// object of its entry alone.
const READ: [Readable; 5] = [
    Readable {
        json: r#"{"Boxed":"a"}"#,
        read_as: "Fogging",
        serde: serde_written::<Fogging>,
    },
    Readable {
        json: r#"{"Split":["b",1]}"#,
        read_as: "Fogging",
        serde: serde_written::<Fogging>,
    },
    Readable {
        json: r#"["b",1]"#,
        read_as: "Thawing",
        serde: serde_written::<Thawing>,
    },
    Readable {
        json: r#"{"reading":[["b",1],3]}"#,
        read_as: "Frosting",
        serde: serde_written::<Frosting>,
    },
    Readable {
        json: r#"{"note":"a","Bay":{"label":"n"}}"#,
        read_as: "Overcast",
        serde: serde_written::<Overcast>,
    },
];

/// A payload whose enum, declared below its holder, is handed an object of another key beside its
/// entry, which serde reads.
const BESIDE: &str = r#"{"note":"a","Bay":{"label":"n"},"north":2}"#;

/// An externally tagged enum, with a unit variant, a variant of named members and one of a value.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Drizzling {
    Calm,
    Gust { knots: u32 },
    Held(String),
}

/// An enum under a tag alone.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Gusting {
    Calm,
    Gust { knots: u32 },
}

/// An enum under a tag and a content key, with a unit variant.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum Squalling {
    Calm,
    Gust { knots: u32 },
    Held(String),
}

/// An untagged enum of a number and of an object.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Misting {
    Count(u32),
    Named { label: String },
}

/// A plain enum.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Hailing {
    Heavy,
    Light,
}

/// An externally tagged enum with a variant of several slots.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Fogging {
    Boxed(String),
    Split(String, u32),
}

/// A tuple struct of two slots.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Thawing(pub String, pub u32);

/// A member that holds a tuple, which holds a tuple.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Frosting {
    pub reading: ((String, u32), u8),
}

/// An externally tagged enum flattened beside a member of its holder's own, declared below the
/// holder.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Overcast {
    pub note: String,
    #[serde(flatten)]
    pub stratus: Hazing,
}

/// The enum [`Overcast`] flattens.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Hazing {
    Bay { label: String },
}

/// One payload a type reads, the type and what serde writes back of it.
struct Readable {
    json: &'static str,
    /// The Dart type whose `fromJson` reads the payload.
    read_as: &'static str,
    /// What serde writes back of the payload.
    serde: fn(&str) -> String,
}

/// One payload, the type it is read as, what serde answers of it and what the Dart read throws.
struct Row {
    json: &'static str,
    /// The Dart type whose `fromJson` reads the payload.
    read_as: &'static str,
    /// What serde answers of the payload.
    serde: fn(&str) -> String,
    /// The message of the `FormatException` the Dart read throws.
    thrown: &'static str,
}

/// serde's refusal of `json` as `T`, or `read` where it reads it.
fn serde_told<T>(json: &str) -> String
where
    T: DeserializeOwned,
{
    serde_json::from_str::<T>(json)
        .map_or_else(|refusal| refusal.to_string(), |_| "read".to_owned())
}

/// What serde writes back of `json` read as `T`, or its refusal.
fn serde_written<T>(json: &str) -> String
where
    T: DeserializeOwned + Serialize,
{
    serde_json::from_str::<T>(json).map_or_else(
        |refusal| refusal.to_string(),
        |read| format!("read {}", serde_json::to_string(&read).unwrap_or_default()),
    )
}

/// The emitted types every row reads.
fn definitions() -> Vec<String> {
    vec![
        drizzling_dart::dart_definition(),
        gusting_dart::dart_definition(),
        squalling_dart::dart_definition(),
        misting_dart::dart_definition(),
        hailing_dart::dart_definition(),
        spanned_dart::dart_definition(),
        reply_dart::dart_definition(),
        fogging_dart::dart_definition(),
        thawing_dart::dart_definition(),
        frosting_dart::dart_definition(),
        overcast_dart::dart_definition(),
        hazing_dart::dart_definition(),
    ]
}

/// Runs, under `named`, the Dart program that prints what `fromJson` of each of `rows`, a payload
/// and the type it is read as, answers, and answers whether it ran and what it printed.
fn told_by_dart(named: &str, rows: &[(&str, &str)]) -> Option<(bool, String)> {
    let mut main = String::from("void main() {\n");
    for (at, (json, read_as)) in rows.iter().enumerate() {
        let _ = writeln!(
            main,
            "  told('{at}', () => {read_as}.fromJson(jsonDecode(r'{json}')).toJson());"
        );
    }
    main.push_str("}\n");
    let source = module(&definitions(), &[TOLD], &main);
    dart(named, &[], &source)
}

/// Each payload serde refuses is refused by the Dart read with a `FormatException`, and the
/// message the read tixschema writes throws it with.
#[test]
fn each_refusal_the_dart_read_writes_is_a_format_exception() {
    let serde_refused: Vec<String> = ROWS.iter().map(|row| (row.serde)(row.json)).collect();
    assert_eq!(
        serde_refused,
        [
            "unknown variant `Gale`, expected one of `Calm`, `Gust`, `Held` at line 1 column 6",
            "unknown variant `Gale`, expected one of `Calm`, `Gust`, `Held` at line 1 column 7",
            "invalid type: integer `1`, expected unit at line 1 column 9",
            "unknown variant `Gale`, expected `Calm` or `Gust` at line 1 column 14",
            "unknown variant `Gale`, expected one of `Calm`, `Gust`, `Held` at line 1 column 14",
            "invalid type: integer `5`, expected unit variant Squalling::Calm at line 1 column 23",
            "data did not match any variant of untagged enum Misting",
            "unknown variant `Sleet`, expected `Heavy` or `Light` at line 1 column 7",
            "unknown field `step`, expected `start` or `end` at line 1 column 82",
            "expected value at line 1 column 18",
            "expected value at line 1 column 12",
            "expected value at line 1 column 2",
            "trailing characters at line 1 column 17",
            "invalid length 1, expected tuple variant Fogging::Split with 2 elements at line 1 \
             column 14",
            "trailing characters at line 1 column 8",
            "invalid length 1, expected tuple struct Thawing with 2 elements at line 1 column 5",
            "trailing characters at line 1 column 20",
            "invalid length 1, expected a tuple of size 2 at line 1 column 20",
        ],
        "serde no longer refuses each payload as it did"
    );
    let mut expected = String::new();
    for (at, row) in ROWS.iter().enumerate() {
        let _ = writeln!(expected, "{at}: FormatException: {}", row.thrown);
    }
    let rows: Vec<(&str, &str)> = ROWS.iter().map(|row| (row.json, row.read_as)).collect();
    let Some((ran, printed)) = told_by_dart("refusals", &rows) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, expected, "the Dart read is not the refusal above");
}

/// An externally tagged enum is read from an object of one key, a tuple from an array of its
/// length, and an enum declared below its holder from the object of its entry alone, as serde
/// reads each, and each is written back as serde writes it.
#[test]
fn an_object_of_one_key_and_a_tuple_of_its_length_are_read_as_serde_reads_them() {
    let serde_read: Vec<String> = READ.iter().map(|row| (row.serde)(row.json)).collect();
    assert_eq!(
        serde_read,
        [
            r#"read {"Boxed":"a"}"#,
            r#"read {"Split":["b",1]}"#,
            r#"read ["b",1]"#,
            r#"read {"reading":[["b",1],3]}"#,
            r#"read {"note":"a","Bay":{"label":"n"}}"#,
        ],
        "serde no longer reads each payload as it did"
    );
    let rows: Vec<(&str, &str)> = READ.iter().map(|row| (row.json, row.read_as)).collect();
    let Some((ran, printed)) = told_by_dart("refusals-read", &rows) else {
        return;
    };
    assert!(ran, "{printed}");
    let mut expected = String::new();
    for (at, told) in serde_read.iter().enumerate() {
        let _ = writeln!(expected, "{at}: {told}");
    }
    assert_eq!(printed, expected, "the Dart read is not serde's");
}

/// An enum declared below its holder is read from the object its holder hands it, and is refused
/// where that object holds a key beside its entry, which serde reads.
#[test]
fn an_enum_declared_below_its_holder_is_refused_beside_another_key() {
    assert_eq!(
        serde_written::<Overcast>(BESIDE),
        r#"read {"note":"a","Bay":{"label":"n"}}"#,
        "serde no longer reads the entry beside another key"
    );
    let Some((ran, printed)) = told_by_dart("refusals-beside", &[(BESIDE, "Overcast")]) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(
        printed, "0: FormatException: more than one key for Hazing\n",
        "the Dart read no longer refuses the key beside the entry"
    );
}

/// The README's sentence on the refusals of the Dart read shows payloads and messages the run above
/// measures.
#[test]
fn the_readme_shows_refusals_the_run_measures() {
    let section = from_until(
        include_str!("../../README.md"),
        "#### The Dart read refuses what `validate()` refuses",
        "#### What stays yours",
    );
    for declared in [
        "`enum Drizzling { Calm, Gust { knots: u32 }, Held(String) }`",
        "`enum Fogging { Boxed(String), Split(String, u32) }`",
        "`struct Overcast { note: String, #[serde(flatten)] stratus: Hazing }`",
        "`enum Hazing { Bay { label: String } }`",
    ] {
        assert!(
            section.contains(declared),
            "the README no longer declares a type the run reads: {declared}"
        );
    }
    for (read_as, json) in [
        ("Drizzling", r#""Gale""#),
        ("Drizzling", r#"{"Gale":{}}"#),
        ("Drizzling", r#"{"Calm":1}"#),
        ("Fogging", r#"{"Boxed":"a","Split":["b",1]}"#),
        ("Fogging", "{}"),
        ("Fogging", r#"{"Split":["b",1,2]}"#),
        ("Fogging", r#"{"Split":["b"]}"#),
    ] {
        assert!(
            ROWS.iter()
                .any(|row| row.read_as == read_as && row.json == json),
            "no row reads {json} as `{read_as}`"
        );
        assert!(
            section.contains(&format!("`{json}`")),
            "not in the README: {json}"
        );
    }
    assert!(
        section.contains(&format!(
            "`{BESIDE}` is refused with `more than one key for Hazing`"
        )),
        "the README no longer shows the refusal of a key beside the entry"
    );
    for shown in [
        "`Unknown Drizzling: Gale`",
        "`Drizzling: Calm takes no value`",
        "`No variant of Misting matched`",
        "`Unknown key of a Range: (start, end, step)`",
        "`Not a Result: (Ok, Err)`",
        "`more than one key for Fogging`",
        "`empty object for Fogging`",
        "`invalid length 3, expected 2`",
        "`invalid length 1, expected 2`",
    ] {
        let message = shown.trim_matches('`');
        assert!(
            ROWS.iter().any(|row| row.thrown == message),
            "no row throws `{message}`"
        );
        assert!(section.contains(shown), "not in the README: {shown}");
    }
}

/// The analyzer passes the enums, the tuples and the holder the rows read.
#[test]
fn the_emitted_enums_have_no_issue() {
    let source = module(
        &[
            drizzling_dart::dart_definition(),
            gusting_dart::dart_definition(),
            squalling_dart::dart_definition(),
            misting_dart::dart_definition(),
            hailing_dart::dart_definition(),
            fogging_dart::dart_definition(),
            thawing_dart::dart_definition(),
            frosting_dart::dart_definition(),
            overcast_dart::dart_definition(),
            hazing_dart::dart_definition(),
        ],
        &[TOLD],
        "void main() => told('calm', () => Drizzling.fromJson(jsonDecode('\"Calm\"')).toJson());\n",
    );
    let Some((clean, said)) = dart("refusals-analyze", &["analyze"], &source) else {
        return;
    };
    assert!(clean, "{said}");
    assert!(said.contains("No issues found!"), "{said}");
}
