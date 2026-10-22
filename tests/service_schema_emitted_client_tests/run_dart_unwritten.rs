//! A member serde reads and never writes, read and written back by the emitted Dart types under
//! the Dart VM, in a struct of its own, beside a flattened member and in an enum's variant under
//! each tagging that writes a variant's named members, against what serde reads and writes back.

#![cfg(feature = "dart")]

use super::run_dart_validate::{dart, module};
use super::unwritten_models::{PAYLOAD, WRITTEN, braiding_dart, furling_dart, reefing_dart};
#[cfg(feature = "serde")]
use super::unwritten_models::{
    VARIANTS, dousing_dart, ratcheting_dart, shackling_dart, wearing_dart,
};

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

/// What prints each enum of [`VARIANTS`] read from its payload: the member of its variant
/// `Furled` read, and the value written back.
#[cfg(feature = "serde")]
fn variants_main() -> String {
    let lines: Vec<String> = VARIANTS
        .iter()
        .map(|(name, payload, _)| {
            format!(
                "  final read{name} = {name}.fromJson(jsonDecode(r'{payload}'));\n  \
                 print('{name}: read ${{(read{name} as {name}Furled).spare}}, written \
                 ${{jsonEncode(read{name}.toJson())}}');"
            )
        })
        .collect();
    format!("void main() {{\n{}\n}}\n", lines.join("\n"))
}

/// Each enum reads the member of its variant and writes it back as serde does: never.
#[cfg(feature = "serde")]
#[test]
fn a_variants_member_serde_never_writes_is_read_and_not_written_in_dart() {
    let source = module(
        &[
            dousing_dart::dart_definition(),
            wearing_dart::dart_definition(),
            shackling_dart::dart_definition(),
            ratcheting_dart::dart_definition(),
        ],
        &[],
        &variants_main(),
    );
    let Some((ran, printed)) = dart("unwritten-variants", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    let expected = VARIANTS
        .iter()
        .map(|(name, _, written)| format!("{name}: read 3, written {written}\n"))
        .collect::<Vec<_>>()
        .concat();
    assert_eq!(printed, expected);
}
