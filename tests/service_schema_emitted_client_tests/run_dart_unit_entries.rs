//! An externally tagged enum flattened beside a member of its holder's own, optional and under a
//! function too, each variant read in either order and written back by the emitted Dart types
//! under the Dart VM, against what serde reads and writes back: a unit variant is an entry of its
//! name over `null`. The enum read alone takes that entry too. The models are those the Kotlin
//! runs are held to, compiled with the `kotlin` feature.

#![cfg(all(feature = "dart", feature = "kotlin", feature = "serde"))]

use core::fmt::Write as _;

use super::flattened_models::{
    Belting, Stowing, Tacking, Yawing, belting_dart, read_of, stowing_dart, tacking_dart,
    yawing_dart,
};
use super::run_dart_validate::{dart, module};

/// What prints a row: the value read, written back, or that the read is refused.
const TOLD: &str = "
void row(String name, String payload, String Function(dynamic) told) {
  String printed;
  try {
    printed = told(jsonDecode(payload));
  } catch (_) {
    printed = 'refused';
  }
  print('$name $payload: $printed');
}
";

/// The function [`Yawing`] names, which answers the payload as it was handed.
const AS_GIVEN: &str = "dynamic asGiven(dynamic payload) => payload;";

/// Each variant's entry ahead of the holder's own key and after it, a variant under its alias, and
/// a unit variant's entry over a value, which serde refuses.
const HELD: [&str; 9] = [
    r#"{"Bay":{"label":"n"},"note":"a"}"#,
    r#"{"note":"a","Bay":{"label":"n"}}"#,
    r#"{"Calm":null,"note":"a"}"#,
    r#"{"note":"a","Calm":null}"#,
    r#"{"Dock":{"label":"s"},"note":"a"}"#,
    r#"{"note":"a","Quay":{"label":"s"}}"#,
    r#"{"Held":"x","note":"a"}"#,
    r#"{"note":"a","Held":"x"}"#,
    r#"{"note":"a","Calm":1}"#,
];

/// A unit variant's entry over an empty object, which serde reads flattened and the Zod schema,
/// which takes the entry over `null` alone, refuses: the Dart read refuses it with the schema.
const OVER_AN_OBJECT: &str = r#"{"note":"a","Calm":{}}"#;

/// The enum read alone: a unit variant's name, its entry over `null`, and its entry over a value
/// and over an empty object, which serde refuses.
const ALONE: [&str; 4] = [
    r#""Calm""#,
    r#"{"Calm":null}"#,
    r#"{"Calm":1}"#,
    r#"{"Calm":{}}"#,
];

/// Each type a row reads, the payloads it reads, and what serde writes back of each.
fn rows() -> Vec<(&'static str, &'static str, String)> {
    let mut rows = Vec::new();
    let mut of = |named: &'static str, payloads: &[&'static str], serde: fn(&str) -> String| {
        rows.extend(
            payloads
                .iter()
                .map(|payload| (named, *payload, serde(payload))),
        );
    };
    of("Tacking", &ALONE, read_of::<Tacking>);
    of("Stowing", &HELD, read_of::<Stowing>);
    // serde reads the entry over a value as no enum where the enum is optional, and the Zod schema
    // refuses it.
    of("Belting", &HELD[..8], read_of::<Belting>);
    of("Yawing", &HELD, read_of::<Yawing>);
    rows
}

/// serde reads a flattened unit variant from its entry over `null` in either order, and writes it
/// as that entry; it refuses the entry over a value, and reads it over an empty object.
#[test]
fn serde_writes_and_reads_a_flattened_unit_variant_as_its_entry_over_null() {
    assert_eq!(
        [read_of::<Stowing>(HELD[2]), read_of::<Stowing>(HELD[3])],
        [r#"{"note":"a","Calm":null}"#; 2],
        "serde reads the entry in either order and writes it"
    );
    assert_eq!(
        read_of::<Stowing>(HELD[8]),
        "refused",
        "serde refuses the entry over a value"
    );
    assert_eq!(
        read_of::<Stowing>(OVER_AN_OBJECT),
        r#"{"note":"a","Calm":null}"#,
        "serde reads the entry over an empty object"
    );
    assert_eq!(
        ALONE.map(read_of::<Tacking>),
        [r#""Calm""#, r#""Calm""#, "refused", "refused"],
        "serde reads the enum alone from the name and from the entry over null"
    );
}

/// Each variant of a flattened enum is read in Dart from its entry in either order, a unit
/// variant's over `null`, and written back as serde writes it back, in a holder of the enum,
/// of the enum optional and of the enum under a function. The enum read alone takes a unit
/// variant's entry over `null`. Each read refuses the entry over a value where serde refuses it,
/// and over an empty object, as the Zod schema refuses it.
#[test]
fn a_flattened_unit_variant_is_written_and_read_in_dart_as_serde_writes_and_reads_it() {
    let mut expected = String::new();
    let mut main = String::from("void main() {\n");
    for (named, payload, serde) in rows() {
        let _ = writeln!(expected, "{named} {payload}: {serde}");
        let _ = writeln!(
            main,
            "  row('{named}', r'{payload}', (json) => jsonEncode({named}.fromJson(json).toJson()));"
        );
    }
    let _ = writeln!(expected, "Stowing {OVER_AN_OBJECT}: refused");
    let _ = writeln!(
        main,
        "  row('Stowing', r'{OVER_AN_OBJECT}', (json) => jsonEncode(Stowing.fromJson(json).toJson()));"
    );
    main.push_str("}\n");
    let source = module(
        &[
            tacking_dart::dart_definition(),
            stowing_dart::dart_definition(),
            belting_dart::dart_definition(),
            yawing_dart::dart_definition(),
        ],
        &[AS_GIVEN, TOLD],
        &main,
    );
    let Some((ran, printed)) = dart("unit-entries", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(
        printed, expected,
        "the Dart reads and writes beside serde's"
    );
}

/// The analyzer passes the enum and its holders. [`Belting`] is run and left out: its read
/// compares the object it hands the enum with `null`, which the analyzer reports.
#[test]
fn the_emitted_holders_of_a_flattened_unit_variant_have_no_issue() {
    let source = module(
        &[
            tacking_dart::dart_definition(),
            stowing_dart::dart_definition(),
            yawing_dart::dart_definition(),
        ],
        &[AS_GIVEN],
        "void main() => print(jsonEncode(Stowing(note: 'a', tacking: TackingCalm()).toJson()));\n",
    );
    let Some((clean, said)) = dart("unit-entries-analyze", &["analyze"], &source) else {
        return;
    };
    assert!(clean, "{said}");
    assert!(said.contains("No issues found!"), "{said}");
}
