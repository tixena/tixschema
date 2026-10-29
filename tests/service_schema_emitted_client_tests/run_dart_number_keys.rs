//! A map keyed by a brand over a number or a flag, or by an alias of a number, read and written
//! back by the emitted Dart types under the Dart VM, against serde, over each payload of
//! [`number_key_rows`](super::number_key_rows). A key serde refuses is refused, and a number or a
//! flag written between quotes as a value is refused.

#![cfg(feature = "dart")]

use core::fmt::Write as _;

use super::number_key_rows::{
    adjacent_pier_dart, anchor_dart, beacon_dart, burgee_dart, cutter_dart, dinghy_dart,
    gauge_dart, hull_dart, ketch_dart, lanyard_dart, moorings_dart, pennant_dart, pier_dart, rows,
    rudder_dart, signal_dart, skiff_dart, slips_dart, sloop_dart, soundings_dart, yawl_dart,
};
use super::run_dart_validate::{dart, module};

/// What prints a row: the value read and written back, or `refused`.
const TOLD: &str = "
void told(String at, Object? Function() read) {
  String printed;
  try {
    printed = 'read ${jsonEncode(read())}';
  } on FormatException catch (_) {
    printed = 'refused';
  }
  print('$at: $printed');
}

void toldAsValue(String at, Object? Function() read) =>
    told(at, () {
      try {
        return read();
      } on TypeError catch (_) {
        throw FormatException('refused');
      }
    });
";

fn definitions() -> Vec<String> {
    vec![
        adjacent_pier_dart::dart_definition(),
        beacon_dart::dart_definition(),
        rudder_dart::dart_definition(),
        anchor_dart::dart_definition(),
        burgee_dart::dart_definition(),
        cutter_dart::dart_definition(),
        dinghy_dart::dart_definition(),
        gauge_dart::dart_definition(),
        hull_dart::dart_definition(),
        ketch_dart::dart_definition(),
        lanyard_dart::dart_definition(),
        moorings_dart::dart_definition(),
        pennant_dart::dart_definition(),
        pier_dart::dart_definition(),
        signal_dart::dart_definition(),
        skiff_dart::dart_definition(),
        slips_dart::dart_definition(),
        sloop_dart::dart_definition(),
        soundings_dart::dart_definition(),
        yawl_dart::dart_definition(),
    ]
}

/// Each payload is read and written back by the Dart read as serde reads and writes it.
#[test]
fn a_map_keyed_by_a_number_brand_or_alias_is_read_and_written_back_by_dart_as_serde_does() {
    let rows = rows();
    let mut main = String::from("void main() {\n");
    let mut expected = String::new();
    for (at, row) in rows.iter().enumerate() {
        let _ = writeln!(
            main,
            "  {}('{at}', () => {}.fromJson(jsonDecode(r'{}')).toJson());",
            if row.named == "Anchor" {
                "toldAsValue"
            } else {
                "told"
            },
            row.named,
            row.json
        );
        let _ = writeln!(expected, "{at}: {}", row.serde);
    }
    main.push_str("}\n");
    assert!(expected.contains("refused"), "{expected}");
    let source = module(&definitions(), &[TOLD], &main);
    let Some((ran, printed)) = dart("number-keys", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, expected, "the Dart read is not serde's");
}

/// The analyzer passes the types.
#[test]
fn the_emitted_types_have_no_issue() {
    let source = module(
        &definitions(),
        &[TOLD],
        "void main() => told('0', () => Soundings.fromJson(jsonDecode(r'{\"gauge\":{\"1.5\":1}}')).toJson());\n",
    );
    let Some((clean, said)) = dart("number-keys-analyze", &["analyze"], &source) else {
        return;
    };
    assert!(clean, "{said}");
    assert!(said.contains("No issues found!"), "{said}");
}
