//! A check or a function named after a member of its own type, in the emitted Swift types
//! compiled with warnings as errors: each row prints what Rust prints.

#![cfg(feature = "swift")]

use super::member_named_rows::{
    Kind, couple_swift, crate_swift, expected, freight_swift, gauge_swift, hold_swift,
    ledger_swift, parcel_swift, plain_swift, rating_swift, rows, stamp_swift,
};
use super::run_swift_read::normalized;
use super::runtime::judged;

const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

const FUNCTIONS: &str = r#"
func rated(_ value: Rating) -> [String] {
  if case .rated = value { return ["a rating is unrated"] }
  return []
}

func unrated(_ payload: Any) -> Any {
  if let text = payload as? String { return text.trimmingCharacters(in: .whitespaces) }
  return payload
}

func label(_ value: Parcel) -> [String] { value.label.isEmpty ? ["a parcel is labelled"] : [] }

func code(_ value: String) -> [String] { value == "dup" ? ["a code is not dup"] : [] }

func note(_ payload: Any) -> Any {
  if var members = payload as? [String: Any] {
    if members["note"] == nil { members["note"] = "none" }
    return members
  }
  return payload
}

func seal(_ payload: Any) -> Any {
  if let text = payload as? String { return text.trimmingCharacters(in: .whitespaces) }
  return payload
}

func validate(_ value: Gauge) -> [String] { value.size > 9 ? ["a gauge is at most 9"] : [] }

func value(_ value: Stamp) -> [String] { value.value.isEmpty ? ["a stamp is not empty"] : [] }

func tag(_ value: Crate) -> [String] { value.tag == "x" ? ["a crate is not x"] : [] }

func title(_ value: Freight) -> [String] {
  if case .held(let payload) = value, payload.title.isEmpty { return ["a freight is titled"] }
  return []
}

func codes<CodeType>(_ value: Hold<CodeType>) -> [String] {
  value.codes.isEmpty ? ["a hold has a code"] : []
}

func spare(_ value: [String]) -> [String] { value.count > 1 ? ["a hold has one spare"] : [] }

func pair(_ value: CouplePairTuple) -> [String] {
  value.slot1 < 0 ? ["a pair is not negative"] : []
}

func entries(_ value: [UInt32: String]) -> [String] {
  value.isEmpty ? ["a ledger has an entry"] : []
}

func ruled(_ value: Plain) -> [String] { value.size == 0 ? ["a plain is sized"] : [] }

func report(_ name: String, _ violations: [String]) {
  if violations.isEmpty {
    print("\(name): passes")
  } else {
    print("\(name): refused")
    for violation in violations { print("    \(violation)") }
  }
}

protocol ReadUnjudged {
  init(swiftSchemaUnjudged decoder: Decoder) throws
}

struct Unjudged<Held: ReadUnjudged>: Decodable {
  let held: Held
  init(from decoder: Decoder) throws { held = try Held(swiftSchemaUnjudged: decoder) }
}

func unjudged<Read: ReadUnjudged>(_ type: Read.Type, _ payload: String) -> Read {
  try! JSONDecoder().decode(Unjudged<Read>.self, from: Data(payload.utf8)).held
}

func written<Read: Codable>(_ name: String, _ type: Read.Type, _ payload: String) {
  let encoder = JSONEncoder()
  encoder.outputFormatting = [.sortedKeys]
  let held = try! JSONDecoder().decode(type, from: Data(payload.utf8))
  print("\(name): read \(String(data: try! encoder.encode(held), encoding: .utf8)!)")
}

extension Rating: ReadUnjudged {}
extension Parcel: ReadUnjudged {}
extension Gauge: ReadUnjudged {}
extension Stamp: ReadUnjudged {}
extension Crate: ReadUnjudged {}
extension Freight: ReadUnjudged {}
extension Hold: ReadUnjudged {}
extension Couple: ReadUnjudged {}
extension Ledger: ReadUnjudged {}
extension Plain: ReadUnjudged {}
"#;

fn definitions() -> Vec<String> {
    vec![
        "import Foundation".to_owned(),
        rating_swift::swift_definition(),
        parcel_swift::swift_definition(),
        gauge_swift::swift_definition(),
        stamp_swift::swift_definition(),
        crate_swift::swift_definition(),
        freight_swift::swift_definition(),
        hold_swift::swift_definition(),
        couple_swift::swift_definition(),
        ledger_swift::swift_definition(),
        plain_swift::swift_definition(),
    ]
}

fn source() -> String {
    let lines: Vec<String> = rows()
        .iter()
        .map(|row| match row.kind {
            Kind::Validate => format!(
                "report(\"{}\", unjudged({}.self, #\"{}\"#).validate())",
                row.name, row.read_as, row.payload
            ),
            Kind::Read => format!(
                "written(\"{}\", {}.self, #\"{}\"#)",
                row.name, row.read_as, row.payload
            ),
        })
        .collect();
    let mut parts = definitions();
    parts.push(FUNCTIONS.to_owned());
    parts.push(lines.join("\n"));
    parts.join("\n\n")
}

#[test]
fn a_check_named_after_a_member_runs_in_swift_as_it_runs_in_rust() {
    let source = source();
    let Some(run) = judged(
        "swift-member-named",
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
    assert_eq!(normalized(&run.stdout), normalized(&expected(&rows())));
}

#[test]
fn only_a_name_that_meets_a_member_is_called_through_a_forwarder() {
    let rating = rating_swift::swift_definition();
    assert!(
        rating.starts_with(
            "private func _ratingCheckRated(_ swiftSchemaChecked: Rating) -> [String] { rated(swiftSchemaChecked) }\n"
        ),
        "{rating}"
    );
    assert!(
        rating.contains("errors += _ratingCheckRated(self)"),
        "{rating}"
    );
    let plain = plain_swift::swift_definition();
    assert!(plain.contains("errors += ruled(self)"), "{plain}");
    assert!(!plain.contains("private func _"), "{plain}");
}
