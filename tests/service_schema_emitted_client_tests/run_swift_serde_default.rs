//! A key under `#[serde(default)]` the payload leaves out, read by the emitted Swift types
//! compiled with warnings as errors: each payload of the shared table prints what the serde read
//! prints of it.

#![cfg(feature = "swift")]

use super::run_swift_read::normalized;
use super::runtime::judged;
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use super::serde_default_models::report;
use super::serde_default_models::{
    Basin, Bollard, CASES, Marina, PREPROCESSED, PREPROCESSED_REPORT, REPORT, basin_swift,
    bollard_swift, gangway_swift, jetty_swift, marina_swift, pontoon_swift, preprocessed_values,
    read_values, unwritten_defaults, wharf_swift,
};

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

/// A default with no JSON keeps its key required: `Marina` derives no `Serialize`, and the
/// `moored` of `Basin` names its parameter. The `dock` of `Basin` names none, and is read as its
/// default.
#[test]
fn a_default_with_no_json_keeps_its_key_required() {
    let (read_marina, read_basin) = unwritten_defaults();
    assert_eq!(
        serde_json::from_str::<Marina>(r#"{"label":"m"}"#).unwrap(),
        read_marina
    );
    assert_eq!(
        serde_json::from_str::<Basin<String>>(r#"{"moored":[]}"#).unwrap(),
        read_basin
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
    let basin = basin_swift::swift_definition();
    assert!(
        basin.contains(
            "if container.contains(.dock) { self.dock = try container.decode(String.self, \
             forKey: .dock) } else { let dockDefaultPayload = try \
             JSONSerialization.jsonObject(with: Data(\"\\\"\\\"\".utf8), options: \
             [.fragmentsAllowed]); self.dock = try SwiftSchemaPayload.read(String.self, from: \
             dockDefaultPayload, at: container.codingPath + [CodingKeys.dock]) }; \
             self.moored = try container.decode([Moored].self, forKey: .moored) }"
        ),
        "{basin}"
    );
}
