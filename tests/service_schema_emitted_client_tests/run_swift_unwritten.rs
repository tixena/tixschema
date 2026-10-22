//! A member serde reads and never writes, read and written back by the emitted Swift types
//! compiled with warnings as errors, in a struct of its own, beside a flattened member and in an
//! enum's variant under each tagging that writes a variant's named members, against what serde
//! reads and writes back.

#![cfg(feature = "swift")]

use super::runtime::judged;
use super::unwritten_models::{PAYLOAD, WRITTEN, braiding_swift, furling_swift, reefing_swift};
#[cfg(feature = "serde")]
use super::unwritten_models::{
    VARIANTS, dousing_swift, ratcheting_swift, shackling_swift, wearing_swift,
};

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// The JSON a value is written back as, its keys sorted.
const WRITTEN_BACK: &str = "
func written<Read: Encodable>(_ value: Read) -> String {
  let encoder = JSONEncoder()
  encoder.outputFormatting = [.sortedKeys]
  return String(data: try! encoder.encode(value), encoding: .utf8)!
}
";

/// What prints each struct read from the payload: the member read, and the value written back.
const MAIN: &str = r##"
let furling = try! JSONDecoder().decode(Furling.self, from: Data(#"PAYLOAD"#.utf8))
print("in a struct of its own: read \(furling.spare.map { "\($0)" } ?? "nil"), written \(written(furling))")
let reefing = try! JSONDecoder().decode(Reefing.self, from: Data(#"PAYLOAD"#.utf8))
print("beside a flattened member: read \(reefing.spare.map { "\($0)" } ?? "nil"), written \(written(reefing))")
"##;

/// Each struct reads the member and writes it back as serde does: never.
#[test]
fn a_member_serde_never_writes_is_read_and_not_written_in_swift() {
    let source = [
        "import Foundation".to_owned(),
        braiding_swift::swift_definition(),
        furling_swift::swift_definition(),
        reefing_swift::swift_definition(),
        WRITTEN_BACK.to_owned(),
        MAIN.replace("PAYLOAD", PAYLOAD),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-unwritten",
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
    assert_eq!(
        run.stdout,
        format!(
            "in a struct of its own: read 3, written {WRITTEN}\n\
             beside a flattened member: read 3, written {WRITTEN}\n"
        )
    );
}

/// What prints each enum of [`VARIANTS`] read from its payload: the member of its variant
/// `Furled` read, and the value written back. serde writes each value of the table with its keys
/// in the sorted order [`WRITTEN_BACK`] writes them in.
#[cfg(feature = "serde")]
fn variants_main() -> String {
    let lines: Vec<String> = VARIANTS
        .iter()
        .map(|(name, payload, _)| {
            format!(
                "let read{name} = try! JSONDecoder().decode({name}.self, from: \
                 Data(#\"{payload}\"#.utf8))\n\
                 guard case .furled(let furled{name}) = read{name} else {{ fatalError(\"{name} \
                 read another variant\") }}\n\
                 print(\"{name}: read \\(furled{name}.spare.map {{ \"\\($0)\" }} ?? \"nil\"), \
                 written \\(written(read{name}))\")"
            )
        })
        .collect();
    lines.join("\n")
}

/// Each enum reads the member of its variant and writes it back as serde does: never.
#[cfg(feature = "serde")]
#[test]
fn a_variants_member_serde_never_writes_is_read_and_not_written_in_swift() {
    let source = [
        "import Foundation".to_owned(),
        dousing_swift::swift_definition(),
        wearing_swift::swift_definition(),
        shackling_swift::swift_definition(),
        ratcheting_swift::swift_definition(),
        WRITTEN_BACK.to_owned(),
        variants_main(),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-unwritten-variants",
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
    let expected = VARIANTS
        .iter()
        .map(|(name, _, written)| format!("{name}: read 3, written {written}\n"))
        .collect::<Vec<_>>()
        .concat();
    assert_eq!(run.stdout, expected);
}
