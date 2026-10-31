//! A member named after a core type, beside a member of that type, in the emitted Dart types: run
//! by the Dart VM and passed by the analyzer, each payload read and written back as serde does.

#![cfg(feature = "dart")]

use alloc::collections::BTreeSet;
use core::fmt::Write as _;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

use super::run_dart_validate::{dart, module};

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Tally {
    pub count: i32,
    pub int: String,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Scaled {
    pub double: String,
    pub factor: f64,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Measured {
    pub num: String,
    pub ratio: f64,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Flagged {
    pub bool: String,
    pub on: bool,
    pub tags: BTreeSet<String>,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Reading {
    Blank,
    Whole { count: i32, int: String },
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Plain {
    pub count: i32,
    pub label: String,
}

struct Row {
    class: &'static str,
    name: &'static str,
    payload: Value,
    serde_writes: fn(&Value) -> Value,
}

fn rows() -> Vec<Row> {
    vec![
        Row {
            class: "Tally",
            name: "tally",
            payload: json!({"count": 3, "int": "x"}),
            serde_writes: written_by_serde::<Tally>,
        },
        Row {
            class: "Scaled",
            name: "scaled",
            payload: json!({"double": "d", "factor": 1.5}),
            serde_writes: written_by_serde::<Scaled>,
        },
        Row {
            class: "Measured",
            name: "measured",
            payload: json!({"num": "n", "ratio": 2.5}),
            serde_writes: written_by_serde::<Measured>,
        },
        Row {
            class: "Flagged",
            name: "flagged",
            payload: json!({"bool": "b", "on": true, "tags": ["a", "b"]}),
            serde_writes: written_by_serde::<Flagged>,
        },
        Row {
            class: "Reading",
            name: "reading",
            payload: json!({"kind": "Whole", "count": 7, "int": "i"}),
            serde_writes: written_by_serde::<Reading>,
        },
        Row {
            class: "Plain",
            name: "plain",
            payload: json!({"count": 1, "label": "l"}),
            serde_writes: written_by_serde::<Plain>,
        },
    ]
}

fn written_by_serde<T>(payload: &Value) -> Value
where
    T: DeserializeOwned + Serialize,
{
    serde_json::to_value(serde_json::from_value::<T>(payload.clone()).unwrap()).unwrap()
}

fn source() -> String {
    let definitions = vec![
        tally_dart::dart_definition(),
        scaled_dart::dart_definition(),
        measured_dart::dart_definition(),
        flagged_dart::dart_definition(),
        reading_dart::dart_definition(),
        plain_dart::dart_definition(),
    ];
    let mut main = String::from("void main() {\n");
    for row in rows() {
        writeln!(
            main,
            "  print('{}: ${{jsonEncode({}.fromJson(jsonDecode(r'''{}''')).toJson())}}');",
            row.name, row.class, row.payload
        )
        .unwrap();
    }
    main.push_str("}\n");
    module(&definitions, &[], &main)
}

#[test]
fn a_member_named_after_a_core_type_is_read_and_written_as_serde_does() {
    let Some((ran, printed)) = dart("core-named", &[], &source()) else {
        return;
    };
    assert!(ran, "{printed}");
    let printed_rows: Vec<(String, Value)> = printed
        .lines()
        .map(|line| {
            let (name, written) = line.split_once(": ").unwrap();
            (name.to_owned(), serde_json::from_str(written).unwrap())
        })
        .collect();
    let expected: Vec<(String, Value)> = rows()
        .into_iter()
        .map(|row| (row.name.to_owned(), (row.serde_writes)(&row.payload)))
        .collect();
    assert_eq!(printed_rows, expected);
}

#[test]
fn the_dart_text_of_a_member_named_after_a_core_type_has_no_issue() {
    let source = source();
    let Some((clean, said)) = dart("core-named-analyze", &["analyze"], &source) else {
        return;
    };
    assert!(clean, "{said}\n--- source ---\n{source}");
    assert!(said.contains("No issues found!"), "{said}");
}

#[test]
fn a_core_type_a_member_takes_the_name_of_is_named_through_a_typedef() {
    let tally = tally_dart::dart_definition();
    assert!(
        tally.starts_with("typedef _Tally$int = int; class Tally"),
        "{tally}"
    );
    assert!(tally.contains("final _Tally$int count;"), "{tally}");
    assert!(tally.contains("final String int;"), "{tally}");
    assert!(tally.contains("json['count'] as _Tally$int"), "{tally}");
    let measured = measured_dart::dart_definition();
    assert!(
        measured.contains("typedef _Measured$num = num;"),
        "{measured}"
    );
    assert!(
        measured.contains("(json['ratio'] as _Measured$num).toDouble()"),
        "{measured}"
    );
    assert!(
        !measured.contains("typedef _Measured$double"),
        "a name no member takes is not declared: {measured}"
    );
    let reading = reading_dart::dart_definition();
    assert!(
        reading.contains("typedef _ReadingWhole$int = int; class ReadingWhole"),
        "{reading}"
    );
}

#[test]
fn a_class_with_no_member_named_after_a_core_type_declares_no_typedef() {
    let plain = plain_dart::dart_definition();
    assert!(!plain.contains("typedef _"), "{plain}");
    assert!(plain.contains("final int count;"), "{plain}");
    assert!(plain.contains("json['count'] as int"), "{plain}");
}
