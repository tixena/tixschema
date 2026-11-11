//! A fixed-width integer through an alias, read by the emitted Swift types compiled with warnings
//! as errors: [`SixtyReached`] holds an alias of an `i64` as a member and in a list, [`Plumb`] a
//! newtype variant over an alias of an `Option<u32>`, [`Echogram`] an alias of a list and one
//! of a map of an integer, and [`Sonar`] an alias that declares a parameter, of a list of it and of
//! it alone, filled with a `u8`. A number written with a fraction or an exponent, and one outside
//! the integer's range, are refused at the member's or the item's coding path in serde's words, as
//! serde refuses each, and every integer serde reads there is read.

#![cfg(all(feature = "swift", feature = "serde"))]

use std::collections::HashMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::null_models::{Plumb, plumb_swift, sounded_swift};
use super::run_swift_fitted_parameters::serde_answer;
use super::run_swift_integral_floats::sorted;
use super::runtime::judged;
use super::wide_integer_models::{SixtyReached, sixty_reached_swift, sixty_revision_swift};

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What a run reads a payload with, and prints the answer with: the value written back with its
/// keys sorted, the coding path and the description of a `dataCorrupted`, and the case, the type
/// and the coding path of any other `DecodingError`, whose description Foundation words differently
/// from one version to the next.
const HARNESS: &str = r#"
func path(_ context: DecodingError.Context) -> String {
  context.codingPath.map { $0.intValue.map(String.init) ?? $0.stringValue }.joined(separator: ".")
}

func row<Read: Codable>(_ name: String, _ type: Read.Type, _ payload: String) {
  do {
    let held = try JSONDecoder().decode(type, from: Data(payload.utf8))
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]
    print("\(name): read \(String(data: try encoder.encode(held), encoding: .utf8)!)")
  } catch DecodingError.dataCorrupted(let context) {
    print("\(name): refused at '\(path(context))': \(context.debugDescription)")
  } catch DecodingError.typeMismatch(let type, let context) {
    print("\(name): typeMismatch(\(type)) at '\(path(context))'")
  } catch {
    print("\(name): \(error)")
  }
}
"#;

/// An alias of a list of a `u8`, declared above the type that holds it.
#[model_schema()]
pub type Echoes = Vec<u8>;

/// An alias of a map of an `i32`, declared above the type that holds it.
#[model_schema()]
pub type Pings = HashMap<String, i32>;

/// An integer through an alias of a list and one of a map.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Echogram {
    pub echoes: Echoes,
    pub pings: Pings,
}

/// An alias that declares a parameter, of a list of it.
#[model_schema()]
pub type Gauged<Held> = Vec<Held>;

/// An alias that declares a parameter, of it alone.
#[model_schema()]
pub type Plumbed<Held> = Held;

/// An integer through the aliases that declare a parameter, and a text through one.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Sonar {
    pub gauged: Gauged<u8>,
    pub named: Plumbed<String>,
    pub plumbed: Plumbed<u8>,
}

/// One payload the run reads: its name, the Swift type that reads it, the payload, and the line
/// the Swift read is held to.
struct Row {
    expected: String,
    name: String,
    payload: String,
    read_as: &'static str,
}

/// The payload of a [`SixtyReached`] whose alias of an `i64` is written as `member`, with the
/// list of them written as `items` where there is one.
fn reached(member: &str, items: Option<&str>) -> String {
    let listed = items.map_or_else(String::new, |written| format!(r#","aliases":{written}"#));
    format!(
        r#"{{"aliased":{member}{listed},"fixed":214,"from":{{"start":0}},"inclusive":{{"start":0,"end":0}},"ranged":{{"start":0,"end":0}},"resulted":{{"Ok":7}},"to":{{"end":0}},"wrong":{{"Ok":"o"}}}}"#
    )
}

/// `payload`, which serde reads as `Read`, held to what serde writes back of it, its keys sorted.
fn read_row<Read>(name: &str, read_as: &'static str, payload: String) -> Row
where
    Read: DeserializeOwned + Serialize,
{
    let read: Read = serde_json::from_str(&payload).unwrap();
    let written = sorted(serde_json::to_value(&read).unwrap());
    Row {
        expected: format!("{name}: read {written}\n"),
        name: name.to_owned(),
        payload,
        read_as,
    }
}

/// `payload`, which serde refuses as `Read`, held to the coding path and the words serde refuses
/// it with.
fn refused_row<Read>(name: &str, read_as: &'static str, payload: String) -> Row
where
    Read: DeserializeOwned + Serialize,
{
    let expected = serde_answer::<Read>(name, &payload);
    assert!(expected.contains(": refused at '"), "serde reads {payload}");
    Row {
        expected,
        name: name.to_owned(),
        payload,
        read_as,
    }
}

/// The payload of an [`Echogram`] whose alias of a list holds `echo` and whose alias of a map
/// holds `ping`.
fn echogram(echo: &str, ping: &str) -> String {
    format!(r#"{{"echoes":[1,{echo}],"pings":{{"a":{ping}}}}}"#)
}

/// The payload of a [`Sonar`] whose alias of a list holds `item` and whose alias of the integer
/// alone is written as `alone`.
fn sonar(item: &str, alone: &str) -> String {
    format!(r#"{{"gauged":[{item}],"named":"a","plumbed":{alone}}}"#)
}

/// `payload`, which serde refuses as a [`Plumb`], held to the refusal of an untagged enum no
/// variant of which reads it.
fn unmatched_row(name: &str, payload: &str) -> Row {
    assert!(
        serde_json::from_str::<Plumb>(payload).is_err(),
        "serde reads {payload}"
    );
    Row {
        expected: format!("{name}: typeMismatch(Plumb) at ''\n"),
        name: name.to_owned(),
        payload: payload.to_owned(),
        read_as: "Plumb",
    }
}

/// Each payload the run reads, and what the Swift read is held to.
fn rows() -> Vec<Row> {
    vec![
        read_row::<SixtyReached>("1 as the alias", "SixtyReached", reached("1", None)),
        read_row::<SixtyReached>(
            "the least and the greatest i64 as the alias",
            "SixtyReached",
            reached(&i64::MIN.to_string(), Some(&format!("[{},0]", i64::MAX))),
        ),
        refused_row::<SixtyReached>("1.0 as the alias", "SixtyReached", reached("1.0", None)),
        refused_row::<SixtyReached>("1e2 as the alias", "SixtyReached", reached("1e2", None)),
        refused_row::<SixtyReached>(
            "a number beyond an i64 as the alias",
            "SixtyReached",
            reached("9223372036854775808", None),
        ),
        refused_row::<SixtyReached>(
            "1.0 in the list of aliases",
            "SixtyReached",
            reached("0", Some("[1,1.0]")),
        ),
        refused_row::<SixtyReached>(
            "a number beyond an i64 in the list of aliases",
            "SixtyReached",
            reached("0", Some("[1,-9223372036854775809]")),
        ),
        read_row::<Echogram>(
            "the greatest u8 and the least i32 through the aliases of a list and a map",
            "Echogram",
            echogram(&u8::MAX.to_string(), &i32::MIN.to_string()),
        ),
        refused_row::<Echogram>(
            "1.0 in the alias of a list",
            "Echogram",
            echogram("1.0", "1"),
        ),
        refused_row::<Echogram>(
            "a number beyond a u8 in the alias of a list",
            "Echogram",
            echogram("256", "1"),
        ),
        refused_row::<Echogram>("-0 in the alias of a map", "Echogram", echogram("1", "-0")),
        read_row::<Plumb>("1 as the alias of an Option", "Plumb", "1".to_owned()),
        read_row::<Plumb>("null as the alias of an Option", "Plumb", "null".to_owned()),
        unmatched_row("1.0 as the alias of an Option", "1.0"),
        unmatched_row("1e2 as the alias of an Option", "1e2"),
        unmatched_row("-1 as the alias of an Option", "-1"),
        read_row::<Sonar>(
            "1 through the aliases that declare a parameter",
            "Sonar",
            sonar("1", "1"),
        ),
        refused_row::<Sonar>(
            "1.0 in the alias of a list that declares a parameter",
            "Sonar",
            sonar("1.0", "1"),
        ),
        refused_row::<Sonar>(
            "a number beyond a u8 in the alias of a list that declares a parameter",
            "Sonar",
            sonar("256", "1"),
        ),
        refused_row::<Sonar>(
            "1.0 as the alias that declares a parameter",
            "Sonar",
            sonar("1", "1.0"),
        ),
        refused_row::<Sonar>(
            "a number beyond a u8 as the alias that declares a parameter",
            "Sonar",
            sonar("1", "256"),
        ),
    ]
}

/// The text of every model compiles with warnings as errors, and the Swift read of every payload
/// answers what serde answers of it, an untagged enum's refusal in Foundation's words.
#[test]
fn an_integer_through_an_alias_is_refused_where_serde_refuses_it_in_swift() {
    let rows = rows();
    let calls: Vec<String> = rows
        .iter()
        .map(|row| {
            format!(
                "row(\"{}\", {}.self, #\"{}\"#)",
                row.name, row.read_as, row.payload
            )
        })
        .collect();
    let source = [
        "import Foundation".to_owned(),
        sixty_revision_swift::swift_definition(),
        sixty_reached_swift::swift_definition(),
        sounded_swift::swift_definition(),
        plumb_swift::swift_definition(),
        echoes_swift::swift_definition(),
        pings_swift::swift_definition(),
        echogram_swift::swift_definition(),
        gauged_swift::swift_definition(),
        plumbed_swift::swift_definition(),
        sonar_swift::swift_definition(),
        HARNESS.to_owned(),
        calls.join("\n"),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-aliased-integers",
        RUNTIME_VAR,
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    ) else {
        return;
    };
    assert!(
        run.succeeded,
        "`swift -warnings-as-errors main.swift` failed.\n--- stdout ---\n{}\n--- stderr ---\n{}\n--- source ---\n{source}",
        run.stdout, run.stderr
    );
    let printed: Vec<String> = run.stdout.lines().map(|line| format!("{line}\n")).collect();
    assert_eq!(printed.len(), rows.len(), "{}", run.stdout);
    for (row, line) in rows.iter().zip(&printed) {
        assert_eq!(line, &row.expected);
    }
}
