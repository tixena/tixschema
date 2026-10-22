//! A payload serde refuses, read by the emitted Dart types under the Dart VM: each refusal the read
//! writes itself is a `FormatException`, the exception `fromJson` throws for a value that breaks a
//! rule, so a caller that catches it catches each. A tag no variant has under every tagging, a
//! unit variant's entry over a value and its content under a content key, a payload no variant of
//! an untagged enum reads, a name no variant of a plain enum has, a range of a key it does not
//! write and a `Result` of both keys.

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
/// enum reads; a name no variant of a plain enum has; a range of a key it does not write; and a
/// `Result` of both keys.
const ROWS: [Row; 10] = [
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
];

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
    ]
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
        ],
        "serde no longer refuses each payload as it did"
    );
    let mut expected = String::new();
    let mut main = String::from("void main() {\n");
    for (at, row) in ROWS.iter().enumerate() {
        let _ = writeln!(expected, "{at}: FormatException: {}", row.thrown);
        let _ = writeln!(
            main,
            "  told('{at}', () => {}.fromJson(jsonDecode(r'{}')).toJson());",
            row.read_as, row.json
        );
    }
    main.push_str("}\n");
    let source = module(&definitions(), &[TOLD], &main);
    let Some((ran, printed)) = dart("refusals", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, expected, "the Dart read is not the refusal above");
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
    assert!(
        section.contains("`enum Drizzling { Calm, Gust { knots: u32 }, Held(String) }`"),
        "the README no longer declares the enum the run reads"
    );
    for json in [r#""Gale""#, r#"{"Gale":{}}"#, r#"{"Calm":1}"#] {
        assert!(
            ROWS.iter()
                .any(|row| row.read_as == "Drizzling" && row.json == json),
            "no row reads {json} as `Drizzling`"
        );
        assert!(
            section.contains(&format!("`{json}`")),
            "not in the README: {json}"
        );
    }
    for shown in [
        "`Unknown Drizzling: Gale`",
        "`Drizzling: Calm takes no value`",
        "`No variant of Misting matched`",
        "`Unknown key of a Range: (start, end, step)`",
        "`Not a Result: (Ok, Err)`",
    ] {
        let message = shown.trim_matches('`');
        assert!(
            ROWS.iter().any(|row| row.thrown == message),
            "no row throws `{message}`"
        );
        assert!(section.contains(shown), "not in the README: {shown}");
    }
}

/// The analyzer passes the enums the rows read.
#[test]
fn the_emitted_enums_have_no_issue() {
    let source = module(
        &[
            drizzling_dart::dart_definition(),
            gusting_dart::dart_definition(),
            squalling_dart::dart_definition(),
            misting_dart::dart_definition(),
            hailing_dart::dart_definition(),
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
