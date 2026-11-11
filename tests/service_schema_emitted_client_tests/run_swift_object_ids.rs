//! An `ObjectId` in each position serde writes one, read and written back by the emitted Swift
//! types compiled with warnings as errors beside the `ObjectId` the README declares, public and
//! `Sendable` as they are, against serde: a key beside `$oid`, and a `$oid` that is not 24
//! hexadecimal digits in lower case, are refused where serde refuses them, and a flattened one is
//! read from its `$oid` key alone, as serde reads past any other.

#![cfg(all(feature = "swift", feature = "mongodb"))]

use core::fmt::Write as _;

use super::object_id_models::{
    belated_swift, cachet_swift, countersigned_swift, docketing_swift, embossed_swift,
    endorsed_swift, endorsing_swift, impressed_swift, late_sigil_swift, notation_swift, rows,
    sigil_swift, sigiled_swift, sigillary_swift, sigillum_swift, stampage_swift, wafer_swift,
    wafered_swift, wafering_swift, wafers_swift,
};
use super::object_id_runs::{printed, readme_object_id};
use super::run_swift_tagged_slots::sorted;
use super::runtime::judged;

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What prints a row: the value read and written back with its keys sorted, or `refused`.
const TOLD: &str = r#"
func told<Read: Codable>(_ at: Int, _ type: Read.Type, _ payload: String) {
  do {
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys, .withoutEscapingSlashes]
    let read = try JSONDecoder().decode(type, from: Data(payload.utf8))
    print("\(at): read \(String(data: try encoder.encode(read), encoding: .utf8)!)")
  } catch {
    print("\(at): refused")
  }
}
"#;

/// Each payload of [`rows`] is read by the Swift read where the row states a client reads it, and
/// written back as serde writes it, and refused where it states none does.
#[test]
fn an_object_id_is_read_and_written_back_by_swift_as_serde_reads_it() {
    let mut main = String::new();
    let mut expected = String::new();
    for (at, row) in rows().iter().enumerate() {
        let _ = writeln!(main, "told({at}, {}.self, #\"{}\"#)", row.read_as, row.json);
        let told = printed(row.swift, row.read_as, row.json);
        let sorted_back = told
            .strip_prefix("read ")
            .map(|written| format!("read {}", sorted(written)));
        let _ = writeln!(expected, "{at}: {}", sorted_back.unwrap_or(told));
    }
    let source = [
        "import Foundation".to_owned(),
        readme_object_id("swift"),
        notation_swift::swift_definition(),
        countersigned_swift::swift_definition(),
        sigil_swift::swift_definition(),
        sigillum_swift::swift_definition(),
        endorsed_swift::swift_definition(),
        endorsing_swift::swift_definition(),
        cachet_swift::swift_definition(),
        embossed_swift::swift_definition(),
        docketing_swift::swift_definition(),
        stampage_swift::swift_definition(),
        sigiled_swift::swift_definition(),
        sigillary_swift::swift_definition(),
        wafer_swift::swift_definition(),
        wafered_swift::swift_definition(),
        wafering_swift::swift_definition(),
        wafers_swift::swift_definition(),
        impressed_swift::swift_definition(),
        belated_swift::swift_definition(),
        late_sigil_swift::swift_definition(),
        TOLD.to_owned(),
        main,
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-object-ids",
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
        "`swift -warnings-as-errors main.swift` failed.\n--- stdout ---\n{}\n--- stderr \
         ---\n{}\n--- source ---\n{source}",
        run.stdout, run.stderr
    );
    assert_eq!(run.stdout, expected, "the Swift read is not serde's");
}

/// The Swift read hands a flattened `ObjectId`, and a brand and an alias over one, the entry of
/// its `$oid` key alone, and looks for an optional one by that key in the whole object, reading it
/// as `nil` beside the `nonstrict` `Stampage` where it does not read.
#[test]
fn the_swift_text_hands_a_flattened_object_id_its_key_alone() {
    let kept = |stem: &str| {
        format!(
            "let {stem}Payload = (try SwiftSchemaPayload(from: decoder).value as? [String: Any] \
             ?? [:]).filter {{ [\"$oid\"].contains($0.key) }}; "
        )
    };
    for (written, held) in [
        (
            docketing_swift::swift_definition(),
            format!(
                "{}self.oid = try SwiftSchemaPayload.read(ObjectId.self, from: oidPayload, at: \
                 decoder.codingPath)",
                kept("oid")
            ),
        ),
        (
            sigiled_swift::swift_definition(),
            format!(
                "{}self.mark = try SwiftSchemaPayload.read(Sigil.self, from: markPayload, at: \
                 decoder.codingPath)",
                kept("mark")
            ),
        ),
        (
            sigillary_swift::swift_definition(),
            format!(
                "{}self.oid = try SwiftSchemaPayload.read(SigillumType.self, from: oidPayload, \
                 at: decoder.codingPath)",
                kept("oid")
            ),
        ),
        (
            stampage_swift::swift_definition(),
            format!(
                "if try decoder.container(keyedBy: SwiftSchemaAnyKey.self).allKeys.contains(where: \
                 {{ [\"$oid\"].contains($0.stringValue) }}) {{ do {{ {}self.oid = try \
                 SwiftSchemaPayload.read(ObjectId.self, from: oidPayload, at: decoder.codingPath) \
                 }} catch {{ self.oid = nil }} }} else {{ self.oid = nil }}",
                kept("oid")
            ),
        ),
    ] {
        assert!(written.contains(&held), "no `{held}` in: {written}");
    }
}
