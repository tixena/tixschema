//! A member serde reads and never writes, read and written back by the emitted Dart types under
//! the Dart VM, in a struct of its own and beside a flattened member, against what serde reads and
//! writes back.

#![cfg(feature = "dart")]

use super::run_dart_validate::{dart, module};
use super::unwritten_models::{PAYLOAD, WRITTEN, braiding_dart, furling_dart, reefing_dart};

/// What prints each struct read from the payload: the member read, and the value written back.
const MAIN: &str = "
void main() {
  final furling = Furling.fromJson(jsonDecode(r'PAYLOAD'));
  print('in a struct of its own: read ${furling.spare}, written ${jsonEncode(furling.toJson())}');
  final reefing = Reefing.fromJson(jsonDecode(r'PAYLOAD'));
  print('beside a flattened member: read ${reefing.spare}, written ${jsonEncode(reefing.toJson())}');
}
";

/// Each struct reads the member and writes it back as serde does: never.
#[test]
fn a_member_serde_never_writes_is_read_and_not_written_in_dart() {
    let source = module(
        &[
            braiding_dart::dart_definition(),
            furling_dart::dart_definition(),
            reefing_dart::dart_definition(),
        ],
        &[],
        &MAIN.replace("PAYLOAD", PAYLOAD),
    );
    let Some((ran, printed)) = dart("unwritten", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(
        printed,
        format!(
            "in a struct of its own: read 3, written {WRITTEN}\n\
             beside a flattened member: read 3, written {WRITTEN}\n"
        )
    );
}
