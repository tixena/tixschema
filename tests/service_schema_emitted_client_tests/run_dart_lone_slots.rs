//! A tuple struct that declares one slot, which serde writes as the slot's value, read and written
//! by the emitted Dart types under the Dart VM, over each payload of
//! [`lone_slot_rows`](super::lone_slot_rows), and passed by the analyzer.

#![cfg(feature = "dart")]

use core::fmt::Write as _;

use super::lone_slot_rows::{held_to_rows, rows};
use super::lone_slots_models::{
    adjacent_stow_dart, bale_dart, bale_pair_dart, baled_dart, bales_by_name_dart,
    bales_by_number_dart, bales_dart, clipped_dart, count_dart, girth_dart, girthed_dart,
    labels_by_number_dart, laden_dart, maybe_bale_dart, moniker_dart, rigger_dart, sack_dart,
    sacked_dart, single_dart, sling_dart, slung_dart, stow_dart, stowed_by_name_dart,
    stowed_by_number_dart, stowed_list_dart, stowed_maybe_dart, stowed_pair_dart, strop_dart,
    tagged_stow_dart, tote_dart, twine_dart, twined_dart, unskipped_dart, untagged_stow_dart,
};
#[cfg(feature = "chrono")]
use super::lone_slots_models::{stowed_stamp_dart, ticked_dart, ticked_millis_dart};
use super::run_dart_validate::{dart, module};

/// What a run reads a payload with, and prints the answer with: the value written back, or each
/// message of a refusal.
const HARNESS: &str = "
void row(String name, Object? Function() read) {
  try {
    print('$name: read ${jsonEncode(read())}');
  } on FormatException catch (refused) {
    print('$name: refused');
    for (final line in refused.message.split('; ')) {
      print('    $line');
    }
  } catch (error) {
    print('$name: $error');
  }
}
";

/// The Dart expression that reads `payload` as `read_as`, a type as Swift writes it, and writes
/// what it read back: a type argument is handed its own read and write.
fn read_back(read_as: &str, payload: &str) -> String {
    let json = format!("jsonDecode(r'''{payload}''')");
    let Some((named, argument)) = read_as.split_once('<') else {
        return format!("{read_as}.fromJson({json}).toJson()");
    };
    let (typed, read, written) = match argument.trim_end_matches('>') {
        "UInt32" | "UInt8" => ("int", "(held) => held as int", "(held) => held"),
        "String" => ("String", "(held) => held as String", "(held) => held"),
        _ => (
            "Bale",
            "(held) => Bale.fromJson(held)",
            "(held) => held.toJson()",
        ),
    };
    format!("{named}<{typed}>.fromJson({json}, {read}).toJson({written})")
}

/// The emitted definition of every model of the table.
fn definitions() -> Vec<String> {
    vec![
        bale_dart::dart_definition(),
        sack_dart::dart_definition(),
        count_dart::dart_definition(),
        moniker_dart::dart_definition(),
        baled_dart::dart_definition(),
        bales_dart::dart_definition(),
        maybe_bale_dart::dart_definition(),
        bales_by_name_dart::dart_definition(),
        bales_by_number_dart::dart_definition(),
        labels_by_number_dart::dart_definition(),
        bale_pair_dart::dart_definition(),
        unskipped_dart::dart_definition(),
        clipped_dart::dart_definition(),
        single_dart::dart_definition(),
        sacked_dart::dart_definition(),
        stowed_list_dart::dart_definition(),
        stowed_maybe_dart::dart_definition(),
        stowed_by_name_dart::dart_definition(),
        stowed_by_number_dart::dart_definition(),
        stowed_pair_dart::dart_definition(),
        tote_dart::dart_definition(),
        sling_dart::dart_definition(),
        slung_dart::dart_definition(),
        girth_dart::dart_definition(),
        strop_dart::dart_definition(),
        girthed_dart::dart_definition(),
        rigger_dart::dart_definition(),
        twine_dart::dart_definition(),
        twined_dart::dart_definition(),
        laden_dart::dart_definition(),
        stow_dart::dart_definition(),
        adjacent_stow_dart::dart_definition(),
        untagged_stow_dart::dart_definition(),
        tagged_stow_dart::dart_definition(),
        #[cfg(feature = "chrono")]
        ticked_dart::dart_definition(),
        #[cfg(feature = "chrono")]
        ticked_millis_dart::dart_definition(),
        #[cfg(feature = "chrono")]
        stowed_stamp_dart::dart_definition(),
    ]
}

/// Every payload reads and writes in Dart as the serde read reads and writes it: the slot's value,
/// with nothing around it. A value that breaks a rule is refused as the Rust `validate()` reports
/// it, the one slot under no step.
#[test]
fn a_struct_of_one_slot_reads_and_writes_in_dart_as_serde_does_wherever_it_is_held() {
    let rows = rows();
    let mut main = String::from("void main() {\n");
    for row in &rows {
        let _ = writeln!(
            main,
            "  row('{}', () => {});",
            row.name.replace('\'', "\\'"),
            read_back(row.read_as, &row.payload)
        );
    }
    main.push_str("}\n");
    let source = module(&definitions(), &[HARNESS], &main);
    let Some((ran, printed)) = dart("lone-slots", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    held_to_rows(&rows, &printed, "Dart");
}

/// The analyzer passes the text of every model of the table.
#[test]
fn the_dart_text_of_a_struct_of_one_slot_has_no_issue() {
    let source = module(
        &definitions(),
        &[],
        "void main() => print(jsonEncode(Count.fromJson(jsonDecode('5')).toJson()));\n",
    );
    let Some((clean, said)) = dart("lone-slots-analyze", &["analyze"], &source) else {
        return;
    };
    assert!(clean, "{said}");
    assert!(said.contains("No issues found!"), "{said}");
}
