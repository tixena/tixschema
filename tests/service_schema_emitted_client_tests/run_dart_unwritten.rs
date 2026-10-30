//! A member serde reads and never writes, read and written back by the emitted Dart types under
//! the Dart VM, in a struct of its own, beside a flattened member and in an enum's variant under
//! each tagging that writes a variant's named members, against what serde reads and writes back.

#![cfg(feature = "dart")]

use super::run_dart_validate::{analyzed, dart, module};
use super::unwritten_models::{
    FLATTENED, PAYLOAD, REFUSED, WRITTEN, braiding_dart, flemishing_dart, furling_dart,
    ranging_dart, reefing_dart, splaying_dart,
};
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

/// What prints each struct of [`FLATTENED`] read from its payload: what its flattened member read
/// holds, and the value written back, and whether each payload of [`REFUSED`] is refused.
const FLATTENED_MAIN: &str = "
bool refused(void Function() read) {
  try {
    read();
    return false;
  } catch (_) {
    return true;
  }
}

void main() {
  final flemishing = Flemishing.fromJson(jsonDecode(r'PAYLOAD_FLEMISHING'));
  print('Flemishing: read ${flemishing.braiding?.label}, written ${jsonEncode(flemishing.toJson())}');
  final splaying = Splaying.fromJson(jsonDecode(r'PAYLOAD_SPLAYING'));
  print('Splaying: read ${splaying.braiding.label}, written ${jsonEncode(splaying.toJson())}');
  final ranging = Ranging.fromJson(jsonDecode(r'PAYLOAD_RANGING'));
  print('Ranging: read ${ranging.counts['north']}, written ${jsonEncode(ranging.toJson())}');
  print('Splaying refused: ${refused(() => Splaying.fromJson(jsonDecode(r'REFUSED_SPLAYING')))}');
  print('Ranging refused: ${refused(() => Ranging.fromJson(jsonDecode(r'REFUSED_RANGING')))}');
}
";

/// [`FLATTENED_MAIN`] with each payload of [`FLATTENED`] and [`REFUSED`] in its place.
fn flattened_main() -> String {
    FLATTENED_MAIN
        .replace("PAYLOAD_FLEMISHING", FLATTENED[0].1)
        .replace("PAYLOAD_SPLAYING", FLATTENED[1].1)
        .replace("PAYLOAD_RANGING", FLATTENED[2].1)
        .replace("REFUSED_SPLAYING", REFUSED[0].1)
        .replace("REFUSED_RANGING", REFUSED[1].1)
}

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
    analyzed("unwritten-analyze", &source);
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

/// A flattened struct and a flattened map serde never writes are read, and written as serde
/// writes them: never.
#[test]
fn a_flattened_member_serde_never_writes_is_read_and_not_written_in_dart() {
    let source = module(
        &[
            braiding_dart::dart_definition(),
            flemishing_dart::dart_definition(),
            splaying_dart::dart_definition(),
            ranging_dart::dart_definition(),
        ],
        &[],
        &flattened_main(),
    );
    analyzed("unwritten-flattened-analyze", &source);
    let Some((ran, printed)) = dart("unwritten-flattened", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(
        printed,
        format!(
            "Flemishing: read north, written {}\nSplaying: read north, written {}\nRanging: \
             read 2, written {}\nSplaying refused: true\nRanging refused: true\n",
            FLATTENED[0].2, FLATTENED[1].2, FLATTENED[2].2
        ),
        "each flattened member is read and not written"
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
    analyzed("unwritten-variants-analyze", &source);
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
