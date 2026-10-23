//! An `ObjectId` in each position serde writes one, read and written back by the emitted Swift
//! types compiled with warnings as errors beside the `ObjectId` the README declares, public and
//! `Sendable` as they are, against serde: a key beside `$oid`, and a `$oid` that is not 24
//! hexadecimal digits in lower case, are refused where serde refuses them, and a flattened one is
//! read from its `$oid` key alone, as serde reads past any other.

#![cfg(all(feature = "swift", feature = "mongodb"))]

use core::fmt::Write as _;

use super::object_id_models::{
    belated_swift, bullae_swift, cachet_swift, cartouche_swift, cartouching_swift,
    countersealing_swift, countersigned_swift, crossed_swift, crossing_swift, docketing_swift,
    embossed_swift, endorsed_swift, endorsing_swift, enveloping_swift, enwafered_swift,
    folding_swift, impress_swift, impressing_swift, late_cartouche_swift, late_sigil_swift,
    latewaxed_swift, lettering_swift, notation_swift, relayed_swift, relaying_swift, rewaxed_swift,
    rewaxing_swift, rows, sigil_swift, sigiled_swift, sigillary_swift, sigillum_swift,
    stampage_swift, wafer_swift, wafered_swift, wafering_swift, waxing_swift, waxwork_swift,
    waxworking_swift,
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
        cartouche_swift::swift_definition(),
        impress_swift::swift_definition(),
        waxwork_swift::swift_definition(),
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
        enveloping_swift::swift_definition(),
        waxing_swift::swift_definition(),
        rewaxing_swift::swift_definition(),
        relaying_swift::swift_definition(),
        wafer_swift::swift_definition(),
        lettering_swift::swift_definition(),
        folding_swift::swift_definition(),
        cartouching_swift::swift_definition(),
        rewaxed_swift::swift_definition(),
        countersealing_swift::swift_definition(),
        latewaxed_swift::swift_definition(),
        waxworking_swift::swift_definition(),
        impressing_swift::swift_definition(),
        relayed_swift::swift_definition(),
        crossed_swift::swift_definition(),
        crossing_swift::swift_definition(),
        bullae_swift::swift_definition(),
        wafered_swift::swift_definition(),
        wafering_swift::swift_definition(),
        enwafered_swift::swift_definition(),
        belated_swift::swift_definition(),
        late_sigil_swift::swift_definition(),
        late_cartouche_swift::swift_definition(),
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
/// its `$oid` key alone, and looks for an optional one by that key in the whole object.
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
                 {{ [\"$oid\"].contains($0.stringValue) }}) {{ {}",
                kept("oid")
            ),
        ),
    ] {
        assert!(written.contains(&held), "no `{held}` in: {written}");
    }
}

/// A type that flattens its type parameter reads it from its `$oid` key alone where it is filled at
/// an `ObjectId`, or at a brand or a struct of one slot over one written above it, and as it read
/// it at any other filling.
#[test]
fn the_swift_text_reads_a_flattened_type_parameter_filled_at_an_object_id_from_its_key_alone() {
    let filled = |named: &str, otherwise: &str| {
        format!(
            "if [{named}].contains(String(describing: Held.self)) {{ let heldPayload = (try \
             SwiftSchemaPayload(from: decoder).value as? [String: Any] ?? [:]).filter {{ \
             [\"$oid\"].contains($0.key) }}; self.held = try SwiftSchemaPayload.read(Held.self, \
             from: heldPayload, at: decoder.codingPath) }} else {{ {otherwise} }}"
        )
    };
    let above = r#""ObjectId", "Sigil""#;
    for (written, held) in [
        (
            enveloping_swift::swift_definition(),
            filled(above, "self.held = try Held(from: decoder)"),
        ),
        (
            waxing_swift::swift_definition(),
            format!(
                "do {{ {} }} catch {{ self.held = nil }}",
                filled(above, "self.held = try Held(from: decoder)")
            ),
        ),
        (
            crossed_swift::swift_definition(),
            filled(
                r#""ObjectId", "Sigil", "Wafer""#,
                "let heldPayload = try SwiftSchemaPayload(from: decoder, without: taken + \
                 [\"spare\"]).value; self.held = try SwiftSchemaPayload.read(Held.self, from: \
                 heldPayload, at: decoder.codingPath)",
            ),
        ),
    ] {
        assert!(written.contains(&held), "no `{held}` in: {written}");
    }
    let countersigned = countersigned_swift::swift_definition();
    assert!(
        !countersigned.contains("String(describing:"),
        "a type that flattens no parameter tells no filling apart: {countersigned}"
    );
}

/// A type that reads its type parameter flattened under an `Option` tells the keys of what fills
/// it by its name, as it is read, among those of the types written above it whose keys are known:
/// an `ObjectId`'s and a brand's over one, a struct's, and an enum's under a tag key. It reads no
/// value where the object holds none of them, and the filling's read where it holds one, and a
/// filling whose keys are not known as `nil` wherever its read throws.
#[test]
fn the_swift_text_looks_a_flattened_option_of_a_type_parameter_up_by_the_keys_of_its_filling() {
    let written = waxing_swift::swift_definition();
    for held in [
        "let heldStanding: [String]? = ([",
        r#""Cartouche": ["wax"]"#,
        r#""Countersigned": ["held"]"#,
        r#""Impress": ["lot", "wax"]"#,
        r#""Notation": ["note"]"#,
        r#""ObjectId": ["$oid"]"#,
        r#""Sigil": ["$oid"]"#,
        r#""Waxwork": ["kind"]"#,
        r#"] as [String: [String]])[String(String(describing: Held.self).prefix { $0 != "<" })]; "#,
        "if let heldStanding { if try decoder.container(keyedBy: \
         SwiftSchemaAnyKey.self).allKeys.contains(where: { heldStanding.contains($0.stringValue) \
         }) { if [",
        "} else { self.held = nil } } else { do { if [",
    ] {
        assert!(written.contains(held), "no `{held}` in: {written}");
    }
    assert!(
        !written.contains(r#""LateCartouche""#),
        "a type declared below is told apart: {written}"
    );
}

/// A type that flattens its type parameter compiles beside no `ObjectId` and no brand over one,
/// and reads any other filling as it read it: [`Enveloping`] filled at [`Countersigned`] reads the
/// keys of both from the whole object.
///
/// [`Enveloping`]: super::object_id_models::Enveloping
/// [`Countersigned`]: super::object_id_models::Countersigned
#[test]
fn a_type_that_flattens_its_parameter_builds_and_reads_another_filling_with_no_object_id() {
    let payload = r#"{"held":"h","note":"n"}"#;
    let source = [
        "import Foundation".to_owned(),
        notation_swift::swift_definition(),
        countersigned_swift::swift_definition(),
        enveloping_swift::swift_definition(),
        TOLD.to_owned(),
        format!("told(0, Enveloping<Countersigned<String>>.self, #\"{payload}\"#)"),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-object-id-absent",
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
    assert_eq!(
        run.stdout,
        format!("0: read {payload}\n"),
        "the Swift read of another filling is not the whole object's"
    );
}
