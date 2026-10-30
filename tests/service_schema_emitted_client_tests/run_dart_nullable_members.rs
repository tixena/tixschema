//! An absent key of a nullable member, read by the emitted Dart types under the Dart VM: each
//! payload prints what the serde read prints of it.

#![cfg(feature = "dart")]

use core::fmt::Write as _;

use super::nullable_member_models::{
    ROWS, agrees, berth_dart, expected, hawser_dart, keel_dart, slipway_dart,
};
use super::run_dart_validate::{analyzed, dart, module};

/// What prints a row: the value read, written back, or that the read threw.
const ROW: &str = "
void row(String name, Object? Function() written) {
  try {
    print('$name: read ${jsonEncode(written())}');
  } catch (_) {
    print('$name: refused');
  }
}
";

#[test]
fn an_absent_key_of_a_nullable_member_is_read_in_dart_as_serde_reads_it() {
    let rows = ROWS
        .iter()
        .fold(String::new(), |mut written, &(name, read_as, payload)| {
            let _ = writeln!(
                written,
                "  row('{name}', () => {read_as}.fromJson(jsonDecode(r'{payload}')).toJson());"
            );
            written
        });
    let source = module(
        &[
            hawser_dart::dart_definition(),
            berth_dart::dart_definition(),
            keel_dart::dart_definition(),
            slipway_dart::dart_definition(),
        ],
        &[ROW],
        &format!("void main() {{\n{rows}}}\n"),
    );
    analyzed("nullable-members-analyze", &source);
    let Some((ran, printed)) = dart("nullable-members", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert!(
        agrees(&printed, &expected()),
        "{printed}\n{:#?}",
        expected()
    );
}
