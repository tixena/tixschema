//! `()` read and written back by the emitted Dart types under the Dart VM, against serde: a
//! field, a brand's slot, a list's item, a tuple's position, an `Option` and a variant's one value,
//! each read from `null` and written as `null`, and each value serde refuses in its place refused
//! with a `FormatException`, an absent key among them. A tuple struct of no slot keeps `[]`. The
//! analyzer passes every type.

#![cfg(all(feature = "dart", feature = "serde"))]

use core::fmt::Write as _;

use super::run_dart_validate::{dart, from_until, module};
use super::unit_models::{
    REFUSED, WRITTEN, probe_maybe_unit_dart, probe_nothing_dart, probe_slotless_dart,
    probe_unit_adjacent_dart, probe_unit_choice_dart, probe_unit_either_dart,
    probe_unit_either_holder_dart, probe_units_dart, serde_refusal, serde_written,
};

/// The struct the README's sentences declare, as `unit_models.rs` declares it.
const DECLARED: &str =
    "struct ProbeUnits { listed: Vec<()>, nothing: ProbeNothing, paired: (u32, ()), unit: () }";

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

/// What the Dart read answers of each payload of [`REFUSED`], in its order. A tuple struct of no
/// slot read from `null` is refused by the cast of its array, a refusal the language makes.
const DART_REFUSED: [&str; 14] = [
    "FormatException: invalid type: [], expected unit",
    "FormatException: invalid type: [], expected unit",
    "FormatException: invalid type: [], expected unit",
    "FormatException: invalid type: [], expected unit",
    "FormatException: invalid type: {}, expected unit",
    "FormatException: invalid type: \"x\", expected unit",
    "FormatException: missing field `unit`",
    "FormatException: missing field `nothing`",
    "FormatException: invalid type: [], expected unit",
    "FormatException: invalid type: [], expected unit",
    "FormatException: invalid type: [], expected unit",
    "FormatException: missing field `body`",
    "FormatException: missing field `either`",
    "_TypeError: type 'Null' is not a subtype of type 'List<dynamic>' in type cast",
];

/// The emitted types every row reads.
fn definitions() -> Vec<String> {
    vec![
        probe_nothing_dart::dart_definition(),
        probe_units_dart::dart_definition(),
        probe_maybe_unit_dart::dart_definition(),
        probe_unit_choice_dart::dart_definition(),
        probe_unit_adjacent_dart::dart_definition(),
        probe_unit_either_dart::dart_definition(),
        probe_unit_either_holder_dart::dart_definition(),
        probe_slotless_dart::dart_definition(),
    ]
}

/// What `told` prints of `json` read as `read_as` and written back.
fn told(at: usize, read_as: &str, json: &str) -> String {
    format!("  told('{at}', () => {read_as}.fromJson(jsonDecode(r'''{json}''')).toJson());\n")
}

/// Each payload serde writes is read and written back as serde writes it back, and each payload
/// serde refuses is refused.
#[test]
fn unit_is_read_from_null_and_written_as_null_in_dart() {
    let mut main = String::from("void main() {\n");
    let mut expected = String::new();
    let mut at = 0;
    for (read_as, json) in WRITTEN {
        main.push_str(&told(at, read_as, json));
        let _ = writeln!(expected, "{at}: read {}", serde_written(read_as, json));
        at += 1;
    }
    for ((read_as, json, said), refused) in REFUSED.into_iter().zip(DART_REFUSED) {
        assert_eq!(
            serde_refusal(read_as, json).as_deref(),
            Some(said),
            "serde no longer refuses {json}"
        );
        main.push_str(&told(at, read_as, json));
        let _ = writeln!(expected, "{at}: {refused}");
        at += 1;
    }
    main.push_str("}\n");
    let source = module(&definitions(), &[TOLD], &main);
    let Some((ran, printed)) = dart("units", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, expected, "the Dart read is not serde's");
}

/// The analyzer passes the types the rows read.
#[test]
fn the_emitted_types_have_no_issue() {
    let source = module(
        &definitions(),
        &[TOLD],
        "void main() => told('unit', () => ProbeNothing.fromJson(null).toJson());",
    );
    let Some((clean, said)) = dart("units-analyze", &["analyze"], &source) else {
        return;
    };
    assert!(clean, "{said}");
    assert!(said.contains("No issues found!"), "{said}");
}

/// The README states the read of `()` in the section of each client, with the payload the runs
/// read.
#[test]
fn each_clients_section_shows_the_payload_the_runs_read() {
    let readme = include_str!("../../README.md");
    let (read_as, json) = WRITTEN[0];
    assert_eq!(
        read_as, "ProbeUnits",
        "the first row no longer reads the struct"
    );
    for (opens, closes) in [
        (
            "#### The Dart read refuses what `validate()` refuses",
            "#### What stays yours",
        ),
        (
            "#### A Swift read refuses what `validate()` refuses",
            "### Literal Values",
        ),
        (
            "#### The read in the Kotlin client",
            "#### `preprocess` in the Kotlin client",
        ),
    ] {
        let section = from_until(readme, opens, closes);
        for shown in [DECLARED, json] {
            assert!(
                section.contains(&format!("`{shown}`")),
                "not under {opens}: {shown}"
            );
        }
    }
}
