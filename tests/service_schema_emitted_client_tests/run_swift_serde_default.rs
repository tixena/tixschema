//! A key under `#[serde(default)]` the payload leaves out, read by the emitted Swift types
//! compiled with warnings as errors: each payload of the shared table prints what the serde read
//! prints of it.

#![cfg(feature = "swift")]

use super::run_swift_read::normalized;
use super::runtime::judged;
use super::serde_default_models::{
    Bollard, CASES, Harbour, Marina, PREPROCESSED, PREPROCESSED_REPORT, REPORT, REQUIRED,
    REQUIRED_REPORT, VARIANT_CASES, VARIANT_REPORT, adjacent_berthing_swift, basin_swift,
    berthing_swift, bollard_swift, config_swift, external_berthing_swift, filled_cases,
    filled_expected, filled_report, fleet_swift, gangway_swift, harbour_swift, jetty_swift,
    link_swift, logbook_swift, manifest_swift, marina_swift, pontoon_swift, preprocessed_values,
    quay_swift, read_values, untagged_berthing_swift, unwritten_defaults, wharf_swift,
};
#[cfg(feature = "indexmap")]
use super::serde_default_models::{indexed_fleet_swift, indexed_logbook_swift};
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use super::serde_default_models::{report, variant_report};

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// The function `Bollard` names, as a consumer writes it in Swift.
const TRIMMED_DOCK: &str = "func trimmedDock(_ payload: Any) -> Any {
  guard let text = payload as? String else { return payload }
  return text.trimmingCharacters(in: .whitespaces)
}";

/// What a run reads a payload with, and prints the answer with: the value written back, or each
/// message of a refusal.
const HARNESS: &str = r#"
func row<Read: Codable>(_ name: String, _ type: Read.Type, _ payload: String) {
  do {
    let held = try JSONDecoder().decode(type, from: Data(payload.utf8))
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]
    print("\(name): read \(String(data: try encoder.encode(held), encoding: .utf8)!)")
  } catch DecodingError.dataCorrupted(let context) {
    print("\(name): refused")
    for line in context.debugDescription.components(separatedBy: "; ") { print("    \(line)") }
  } catch {
    print("\(name): \(error)")
  }
}
"#;

/// What prints a row of a key the type requires: whether the read answered or threw.
const REQUIRES: &str = r#"
func requires<Read: Codable>(_ name: String, _ type: Read.Type, _ payload: String) {
  do {
    _ = try JSONDecoder().decode(type, from: Data(payload.utf8))
    print("\(name): read")
  } catch {
    print("\(name): refused")
  }
}
"#;

/// Every row of the table: a key left out is read as the default serde reads, a key held as it
/// was, and a default that breaks its bound is refused with its member named.
#[test]
fn each_payload_reads_in_swift_as_the_serde_read_reads_it() {
    for ((_, payload), read) in PREPROCESSED.iter().zip(preprocessed_values()) {
        assert_eq!(serde_json::from_str::<Bollard>(payload).unwrap(), read);
    }
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    assert_eq!(report(), REPORT, "what the serde read answers");
    let (every_default, _) = read_values();
    assert!(REPORT.contains(&format!(
        "every default at once: read {}\n",
        serde_json::to_string(&every_default).unwrap()
    )));
    let bollards = PREPROCESSED
        .iter()
        .map(|&(name, payload)| (name, "Bollard", payload));
    let rows: Vec<String> = CASES
        .iter()
        .copied()
        .chain(bollards)
        .map(|(name, read_as, payload)| format!("row(\"{name}\", {read_as}.self, #\"{payload}\"#)"))
        .collect();
    let source = [
        "import Foundation".to_owned(),
        gangway_swift::swift_definition(),
        wharf_swift::swift_definition(),
        jetty_swift::swift_definition(),
        pontoon_swift::swift_definition(),
        bollard_swift::swift_definition(),
        TRIMMED_DOCK.to_owned(),
        HARNESS.to_owned(),
        rows.join("\n"),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-serde-default",
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
        normalized(&run.stdout),
        normalized(&format!("{REPORT}{PREPROCESSED_REPORT}"))
    );
}

/// A default with no JSON keeps its key required: `Marina` derives no `Serialize`, and each
/// member of `Harbour` names its parameter outside a list read as empty.
#[test]
fn a_default_with_no_json_keeps_its_key_required() {
    let (read_marina, _) = unwritten_defaults();
    assert_eq!(
        serde_json::from_str::<Marina>(r#"{"label":"m"}"#).unwrap(),
        read_marina
    );
    let marina = marina_swift::swift_definition();
    assert!(
        marina.contains(
            "let container = try decoder.container(keyedBy: CodingKeys.self); \
             self.dock = try container.decode(String.self, forKey: .dock); \
             self.label = try container.decode(String.self, forKey: .label) }"
        ),
        "{marina}"
    );
    let harbour = harbour_swift::swift_definition();
    assert!(
        harbour.contains(
            "let container = try decoder.container(keyedBy: CodingKeys.self); self.berths = try \
             container.decode([Moored].self, forKey: .berths); self.flagship = try \
             container.decode(Moored.self, forKey: .flagship); self.slips = try \
             container.decode([Moored].self, forKey: .slips) }"
        ),
        "{harbour}"
    );
}

/// A list of the parameter is read from the JSON of its default, `[]`, through the read it has.
#[test]
fn an_empty_collection_of_a_parameter_is_read_from_its_json() {
    let basin = basin_swift::swift_definition();
    assert!(
        basin.contains(
            "if container.contains(.moored) { self.moored = try container.decode([Moored].self, \
             forKey: .moored) } else { let mooredDefaultPayload = try \
             JSONSerialization.jsonObject(with: Data(\"[]\".utf8), options: \
             [.fragmentsAllowed]); self.moored = try SwiftSchemaPayload.read([Moored].self, \
             from: mooredDefaultPayload, at: container.codingPath + [CodingKeys.moored]) }"
        ),
        "{basin}"
    );
}

/// Each list and map of a model filled with the parameter is read from `[]` or `{}` through the
/// read it has.
#[test]
fn an_empty_collection_of_a_model_filled_with_a_parameter_is_read_from_its_json() {
    let fleet = fleet_swift::swift_definition();
    assert!(
        fleet.contains(
            "else { let vecDefaultPayload = try JSONSerialization.jsonObject(with: \
             Data(\"[]\".utf8), options: [.fragmentsAllowed]); self.vec = try \
             SwiftSchemaPayload.read([Link<IdType>].self, from: vecDefaultPayload, at: \
             container.codingPath + [CodingKeys.vec]) }"
        ),
        "{fleet}"
    );
    let logbook = logbook_swift::swift_definition();
    assert!(
        logbook.contains(
            "else { let hashMapDefaultPayload = try JSONSerialization.jsonObject(with: \
             Data(\"{}\".utf8), options: [.fragmentsAllowed]); self.hashMap = try \
             SwiftSchemaPayload.read([String: Link<IdType>].self, from: hashMapDefaultPayload, \
             at: container.codingPath + [CodingKeys.hashMap]) }"
        ),
        "{logbook}"
    );
}

/// Each payload of a struct that declares a parameter reads at `String` as serde reads it, and a
/// payload that leaves out a key `Harbour` requires is refused.
#[test]
fn each_payload_of_a_struct_that_declares_a_parameter_reads_in_swift_as_serde_reads_it() {
    assert_eq!(
        filled_report(),
        filled_expected(),
        "what the serde read answers"
    );
    let (_, harbours) = unwritten_defaults();
    for ((_, payload), harbour) in REQUIRED.iter().zip(harbours) {
        assert_eq!(
            serde_json::from_str::<Harbour<String>>(payload).unwrap(),
            harbour
        );
    }
    let filled = filled_cases().into_iter().map(|(name, read_as, payload)| {
        format!("row(\"{name}\", {read_as}<String>.self, #\"{payload}\"#)")
    });
    let required = REQUIRED.iter().map(|(name, payload)| {
        format!("requires(\"{name}\", Harbour<String>.self, #\"{payload}\"#)")
    });
    let rows: Vec<String> = filled.chain(required).collect();
    let definitions = [
        link_swift::swift_definition(),
        basin_swift::swift_definition(),
        config_swift::swift_definition(),
        fleet_swift::swift_definition(),
        logbook_swift::swift_definition(),
        harbour_swift::swift_definition(),
        manifest_swift::swift_definition(),
        #[cfg(feature = "indexmap")]
        indexed_fleet_swift::swift_definition(),
        #[cfg(feature = "indexmap")]
        indexed_logbook_swift::swift_definition(),
    ];
    let source = [
        &["import Foundation".to_owned()],
        definitions.as_slice(),
        &[HARNESS.to_owned(), REQUIRES.to_owned(), rows.join("\n")],
    ]
    .concat()
    .join("\n\n");
    let Some(run) = judged(
        "swift-serde-default-filled",
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
        normalized(&run.stdout),
        normalized(&format!("{}{REQUIRED_REPORT}", filled_expected()))
    );
}

/// Every row of the variant table, under each tagging: a member of a variant left out is read as
/// the default serde reads, a member held as it was, and a default that breaks its bound is
/// refused with its member named.
#[test]
fn each_variant_payload_reads_in_swift_as_the_serde_read_reads_it() {
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    assert_eq!(
        variant_report(),
        VARIANT_REPORT,
        "what the serde read answers"
    );
    let rows: Vec<String> = VARIANT_CASES
        .iter()
        .map(|(name, read_as, payload)| format!("row(\"{name}\", {read_as}.self, #\"{payload}\"#)"))
        .collect();
    let source = [
        "import Foundation".to_owned(),
        berthing_swift::swift_definition(),
        external_berthing_swift::swift_definition(),
        adjacent_berthing_swift::swift_definition(),
        untagged_berthing_swift::swift_definition(),
        quay_swift::swift_definition(),
        HARNESS.to_owned(),
        rows.join("\n"),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-serde-default-variants",
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
    assert_eq!(normalized(&run.stdout), normalized(VARIANT_REPORT));
}
