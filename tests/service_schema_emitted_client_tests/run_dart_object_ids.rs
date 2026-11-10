//! An `ObjectId` in each position serde writes one, read and written back under the Dart VM by the
//! emitted Dart types beside the `ObjectId` the README declares, against serde: a key beside
//! `$oid`, and a `$oid` that is not 24 hexadecimal digits in lower case, are refused where serde
//! refuses them, and a flattened one is read from its `$oid` key alone, as serde reads past any
//! other.

#![cfg(all(feature = "dart", feature = "mongodb"))]

use core::fmt::Write as _;

use super::object_id_models::{
    belated_dart, cachet_dart, countersigned_dart, docketing_dart, embossed_dart, endorsed_dart,
    endorsing_dart, impressed_dart, late_sigil_dart, notation_dart, rows, sigil_dart, sigiled_dart,
    sigillary_dart, sigillum_dart, stampage_dart, wafer_dart, wafered_dart, wafering_dart,
    wafers_dart,
};
use super::object_id_runs::{printed, readme_object_id};
use super::run_dart_validate::{dart, module};

/// What prints a row: the value read and written back, or `refused`.
const TOLD: &str = "
void told(String at, Object? Function() read) {
  String printed;
  try {
    printed = 'read ${jsonEncode(read())}';
  } catch (_) {
    printed = 'refused';
  }
  print('$at: $printed');
}
";

/// The README's `ObjectId` and the emitted types the rows read.
fn definitions() -> Vec<String> {
    vec![
        readme_object_id("dart"),
        notation_dart::dart_definition(),
        countersigned_dart::dart_definition(),
        sigil_dart::dart_definition(),
        sigillum_dart::dart_definition(),
        endorsed_dart::dart_definition(),
        endorsing_dart::dart_definition(),
        cachet_dart::dart_definition(),
        embossed_dart::dart_definition(),
        docketing_dart::dart_definition(),
        stampage_dart::dart_definition(),
        sigiled_dart::dart_definition(),
        sigillary_dart::dart_definition(),
        wafer_dart::dart_definition(),
        wafered_dart::dart_definition(),
        wafering_dart::dart_definition(),
        wafers_dart::dart_definition(),
        impressed_dart::dart_definition(),
        belated_dart::dart_definition(),
        late_sigil_dart::dart_definition(),
    ]
}

/// Each payload of [`rows`] is read by the Dart read where the row states a client reads it, and
/// written back as serde writes it, and refused where it states none does.
#[test]
fn an_object_id_is_read_and_written_back_by_dart_as_serde_reads_it() {
    let mut main = String::from("void main() {\n");
    let mut expected = String::new();
    for (at, row) in rows().iter().enumerate() {
        let _ = writeln!(
            main,
            "  told('{at}', () => {}.fromJson(jsonDecode(r'''{}''')).toJson());",
            row.read_as, row.json
        );
        let _ = writeln!(
            expected,
            "{at}: {}",
            printed(row.dart, row.read_as, row.json)
        );
    }
    main.push_str("}\n");
    let source = module(&definitions(), &[TOLD], &main);
    let Some((ran, printed)) = dart("object-ids", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, expected, "the Dart read is not serde's");
}

/// The analyzer passes the README's `ObjectId` beside every emitted type the rows read: a holder
/// of one under a key, in a list, as a type argument, under a brand, as a map's value, flattened
/// and beside a tag.
#[test]
fn the_readme_object_id_has_no_issue_beside_the_emitted_types() {
    let source = module(
        &definitions(),
        &[],
        &format!(
            "void main() => \
             print(jsonEncode(Endorsed.fromJson(jsonDecode(r'''{}''')).toJson()));\n",
            rows()[0].json
        ),
    );
    let Some((clean, said)) = dart("object-ids-analyze", &["analyze"], &source) else {
        return;
    };
    assert!(clean, "{said}");
    assert!(said.contains("No issues found!"), "{said}");
}

/// The Dart read hands a flattened `ObjectId`, and a brand and an alias over one, the entry of its
/// `$oid` key alone, and looks for an optional one by that key.
#[test]
fn the_dart_text_hands_a_flattened_object_id_its_key_alone() {
    let only = "(switch (json) { final whole => { for (final key in const ['\\$oid']) if \
                (whole.containsKey(key)) key: whole[key] } })";
    for (written, held) in [
        (
            docketing_dart::dart_definition(),
            format!("oid: ObjectId.fromJson({only}),"),
        ),
        (
            sigiled_dart::dart_definition(),
            format!("mark: Sigil.fromJsonUnjudged({only}),"),
        ),
        (
            sigillary_dart::dart_definition(),
            format!("oid: SigillumType.fromJsonUnjudged({only}),"),
        ),
        (
            stampage_dart::dart_definition(),
            "oid: ((switch (({...json}..remove('id'))) { final whole => { for (final key in \
             const ['\\$oid']) if (whole.containsKey(key)) key: whole[key] } }).keys.any(const \
             ['\\$oid'].contains) ? "
                .to_owned(),
        ),
    ] {
        assert!(written.contains(&held), "no `{held}` in: {written}");
    }
}

/// A holder hands a type that reads its type parameter under a key a converter that reads the
/// `ObjectId` it is filled with.
#[test]
fn the_dart_text_hands_a_type_parameter_under_a_key_the_object_id_read() {
    let written = endorsed_dart::dart_definition();
    let held = "Countersigned.fromJsonUnjudged(json['carried'], (e) => ObjectId.fromJson(e))";
    assert!(written.contains(held), "no `{held}` in: {written}");
}
