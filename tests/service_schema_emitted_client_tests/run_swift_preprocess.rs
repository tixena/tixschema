//! The emitted Swift `preprocess` run under a Swift toolchain, with warnings as errors. A read
//! calls each function a type and its members name, on the payload, ahead of the value it builds.
//! What it prints is held to the table the Rust read and the Zod schemas print, and to the Rust
//! read of the same payload.

#![cfg(feature = "swift")]

use core::fmt::{Debug, Write as _};

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::preprocess_models::{
    POSITIONS, Pinned, SAMPLE, SAMPLE_ROWS, carrier_swift, closed_dock_swift, consigned_swift,
    crate_swift, crated_swift, delivery_swift, either_swift, grade_swift, handover_swift,
    label_swift, manifest_swift, node_swift, parcel_swift, pinned_swift, positions,
    reference_swift, routed_swift, sample_report, shelf_swift, shipment_swift, slot_swift,
    stack_swift, tagged_swift, transfer_swift, unnamed_swift, weighed_swift, wrapped_swift,
};
use super::run_swift_read::normalized;
use super::runtime::judged;
use super::swift_preprocess_models::shipping_client_service_schema::{
    shipping_client_service_fault_fields_swift, shipping_client_service_fault_kind_swift,
};
use super::swift_preprocess_models::{
    Ahead, Barged, Binned, Boxed, Branch, Choice, Chosen, Dated, Docked, Ferried, Hull, Loaded,
    Lodged, Lone, Loosed, Marked, Moved, Paired, Quayside, Sampled, Sealing, Shifted,
    ShippingBackEnd, ShippingClientService as _, ShippingClientServiceSchema, ShippingError,
    Shouldered, Spared, Stamped, Sticker, Stowed, Strapped, Tallied, Twice, ahead_swift,
    banded_swift, barged_swift, behind_swift, binned_swift, boxed_swift, branch_swift,
    choice_swift, chosen_swift, dated_swift, docked_swift, ferried_swift, hull_swift,
    listing_swift, loaded_swift, lodged_swift, lone_swift, loosed_swift, marked_swift, moved_swift,
    paired_swift, quayside_swift, sampled_swift, sealing_swift, shifted_swift,
    shipping_error_swift, shouldered_swift, sided_swift, spared_swift, stamped_swift,
    sticker_swift, stowed_swift, strapped_swift, tallied_calls, tallied_swift, twice_swift,
};
#[cfg(feature = "chrono")]
use super::swift_preprocess_models::{Timed, timed_swift};
use super::tests::poll_once;

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// The two functions of the parcel, as the README shows them.
const SHOWN_FUNCTIONS: &str = r#"func trimmed(_ payload: Any) -> Any {
  guard let text = payload as? String else { return payload }
  return text.trimmingCharacters(in: .whitespaces)
}

func withoutFragile(_ payload: Any) -> Any {
  guard var members = payload as? [String: Any], members["fragileItemCodes"] == nil else { return payload }
  members["fragileItemCodes"] = [String]()
  return members
}"#;

/// The developer's other functions, written by hand in Swift.
const FUNCTIONS: &str = r#"
enum Calls {
  nonisolated(unsafe) static var label = 0
  nonisolated(unsafe) static var spare = 0
}

func asADate(_ payload: Any) -> Any {
  Date()
}

func countedLabel(_ payload: Any) -> Any {
  Calls.label += 1
  return payload
}

func countedSpare(_ payload: Any) -> Any {
  Calls.spare += 1
  return payload
}

func kindsOf(_ payload: Any) -> Any {
  guard let items = payload as? [Any] else { return payload }
  return items.map { (item: Any) -> String in
    if item is NSNull { return "NSNull" }
    if item is Bool { return "Bool" }
    if item is Int64 { return "Int64" }
    if item is Double { return "Double" }
    if item is String { return "String" }
    if item is [Any] { return "[Any]" }
    if item is [String: Any] { return "[String: Any]" }
    return "none of them"
  }
}

func orNoBranches(_ payload: Any) -> Any {
  payload is NSNull ? [Any]() : payload
}

func orUnnamed(_ payload: Any) -> Any {
  payload is NSNull ? "unnamed" : payload
}

func textual(_ payload: Any) -> Any {
  if let whole = payload as? Int64 { return String(whole) }
  return payload
}

func labelled(_ payload: Any) -> Any {
  guard let text = payload as? String else { return payload }
  return ["label": text]
}

func hulled(_ payload: Any) -> Any {
  guard let text = payload as? String else { return payload }
  return ["kind": "Plain", "label": text]
}

func trimmedPlate(_ payload: Any) -> Any {
  guard var members = payload as? [String: Any], let plate = members["plate"] as? String else { return payload }
  members["plate"] = plate.trimmingCharacters(in: .whitespaces)
  return members
}

func unwrapped(_ payload: Any) -> Any {
  guard let members = payload as? [String: Any] else { return payload }
  return members["payload"] ?? members
}

func withChildren(_ payload: Any) -> Any {
  guard var members = payload as? [String: Any], members["children"] == nil else { return payload }
  members["children"] = [Any]()
  return members
}

func withItsKeys(_ payload: Any) -> Any {
  guard var members = payload as? [String: Any] else { return payload }
  members["keys"] = members.keys.sorted()
  return members
}

func withinLimit(_ value: Weighed) -> [String] {
  value.grams > 1000 ? ["'grams': \(value.grams) is over the limit of 1000"] : []
}
"#;

/// What a run reads a payload with, and prints the answer with: the value written back, each
/// message of a refusal, or the error a payload that does not read is answered with, at the
/// coding path it names.
const HARNESS: &str = r#"
func place(_ path: [CodingKey]) -> String {
  path.isEmpty ? "" : " at [" + path.map { $0.intValue.map(String.init) ?? $0.stringValue }.joined(separator: ", ") + "]"
}

func written<Held: Encodable>(_ held: Held) -> String {
  let encoder = JSONEncoder()
  encoder.outputFormatting = [.sortedKeys]
  return String(data: try! encoder.encode(held), encoding: .utf8)!
}

func attempt<Read: Decodable>(_ name: String, _ type: Read.Type, _ payload: String, _ shown: (Read) -> String) {
  do {
    let held = try JSONDecoder().decode(type, from: Data(payload.utf8))
    print("\(name): \(shown(held))")
  } catch DecodingError.dataCorrupted(let context) {
    print("\(name): refused\(place(context.codingPath))")
    for line in context.debugDescription.components(separatedBy: "; ") { print("    \(line)") }
  } catch DecodingError.keyNotFound(let key, let context) {
    print("\(name): no key \(key.stringValue)\(place(context.codingPath))")
  } catch DecodingError.valueNotFound(_, let context) {
    print("\(name): no value\(place(context.codingPath))")
  } catch DecodingError.typeMismatch(_, let context) {
    print("\(name): mismatch\(place(context.codingPath))")
  } catch {
    print("\(name): \(error)")
  }
}

func row<Read: Codable>(_ name: String, _ type: Read.Type, _ payload: String) {
  attempt(name, type, payload) { "read \(written($0))" }
}

func counted<Read: Codable>(_ name: String, _ type: Read.Type, _ payload: String) {
  let before = (Calls.label, Calls.spare)
  row(name, type, payload)
  print("    label \(Calls.label - before.0), spare \(Calls.spare - before.1)")
}
"#;

/// A transport that answers every request with one body, and what a call's answer is printed as.
const HTTP_DRIVER: &str = r#"
struct Answering: ShippingClientServiceHttpTransport {
  let body: String
  func send(_ request: ShippingClientServiceHttpRequest) async throws -> ShippingClientServiceHttpResponse {
    ShippingClientServiceHttpResponse(status: 200, headers: [], body: Data(body.utf8))
  }
}

func described(_ name: String, _ answer: Result<Parcel, ShippingClientServiceShippedFailure>) {
  switch answer {
  case .success(let parcel):
    print("\(name): the parcel \(parcel.label), fragile \(parcel.fragileItemCodes)")
  case .failure(.declared):
    print("\(name): the declared error")
  case .failure(.fault(let fault)):
    print("\(name): the fault \(fault.kind.rawValue)")
    print("    \(fault.detail)")
  }
}

for (name, body) in replies {
  let client = ShippingClientServiceHttpClient(transport: Answering(body: body))
  described(name, await client.shipped("A"))
}
"#;

/// A socket that hands the transport one reply for the request it sent, and what a call's answer
/// is printed as.
#[cfg(slow_tests)]
const WS_DRIVER: &str = r#"
final class InMemorySocket: ShippingClientServiceWsSocket, @unchecked Sendable {
  private let lock = NSLock()
  private var _sent: [String] = []

  var onMessage: (@Sendable (String) -> Void)?
  var onClose: (@Sendable () -> Void)?

  var sent: [String] {
    lock.lock()
    defer { lock.unlock() }
    return _sent
  }

  func send(_ text: String) {
    lock.lock()
    _sent.append(text)
    lock.unlock()
  }

  func close() {}
}

func described(_ name: String, _ answer: Result<Parcel, ShippingClientServiceShippedFailure>) {
  switch answer {
  case .success(let parcel):
    print("\(name): the parcel \(parcel.label), fragile \(parcel.fragileItemCodes)")
  case .failure(.declared):
    print("\(name): the declared error")
  case .failure(.fault(let fault)):
    print("\(name): the fault \(fault.kind.rawValue)")
    print("    \(fault.detail)")
  }
}

for (name, body) in replies {
  let socket = InMemorySocket()
  let transport = ShippingClientServiceWsTransport(socket: socket, options: .init(heartbeat: nil))
  async let answer = transport.shipped("A")
  while socket.sent.isEmpty { try? await Task.sleep(nanoseconds: 5_000_000) }
  let sent = try! JSONDecoder().decode(ShippingClientServiceWsFrameProbe.self, from: Data(socket.sent[0].utf8))
  socket.onMessage?("{\"kind\":\"reply\",\"id\":\"\(sent.id!)\",\"service\":\"ShippingClientService\",\"ok\":true,\"value\":\(body)}")
  described(name, await answer)
  await transport.close()
}
"#;

/// The lines that read a type whose function lists its keys in each place it is read beside the
/// keys of another type, and print the keys the function was handed.
const KEYS_DRIVER: &str = r##"
attempt("a flattened type", Pinned.self, #"{"dock":"north","tag":"t"}"#) { "\($0.tagged.keys)" }
attempt("a flattened type in a flattened holder", Quayside.self, #"{"berth":"b","dock":"north","tag":"t"}"#) { "\($0.pinned.tagged.keys)" }
attempt("two flattened types", Twice.self, #"{"dock":"north","side":"s","tag":"t"}"#) { "\($0.first.keys) \($0.second.keys)" }
attempt("a flattened member's own function", Docked.self, #"{"dock":"north","tag":"t"}"#) { "\($0.listing.keys)" }
attempt("a variant under a tag", Stowed.self, #"{"kind":"Held","tag":"t"}"#) {
  if case .held(let inner) = $0 { return "\(inner.keys)" }
  return ""
}
attempt("a slot's own function, under a tag", Marked.self, #"{"kind":"Held","tag":"t"}"#) {
  if case .held(let inner) = $0 { return "\(inner.keys)" }
  return ""
}
attempt("a variant under a tag that flattens", Boxed.self, #"{"kind":"Held","dock":"north","tag":"t"}"#) {
  if case .held(let inner) = $0 { return "\(inner.tagged.keys) at \(inner.dock)" }
  return ""
}
attempt("a flattened choice that holds the type", Chosen.self, #"{"choice_dock":"north","tag":"t"}"#) {
  if case .tagged(let inner) = $0.taken { return "\(inner.keys)" }
  return ""
}
attempt("a flattened enum under a tag that holds the type", Lodged.self, #"{"dock":"north","kind":"Held","tag":"t"}"#) {
  if case .held(let inner) = $0.stowed { return "\(inner.keys)" }
  return ""
}
attempt("a flattened brand over the type", Strapped.self, #"{"dock":"north","tag":"t"}"#) { "\($0.banded.value.keys)" }
"##;

/// The payloads a tallied value is read from, each under the name of its row.
const TALLIED: [(&str, &str); 4] = [
    ("no key", "{}"),
    ("a label", r#"{"label":"A"}"#),
    ("a label and a spare", r#"{"label":"A","spare":"s"}"#),
    ("a label and a null", r#"{"label":"A","spare":null}"#),
];

/// The payloads a nullable and an optional member are read from.
const SPARED: [(&str, &str); 4] = [
    ("neither key", "{}"),
    ("a null spare", r#"{"spare":null}"#),
    ("each a null", r#"{"note":null,"spare":null}"#),
    ("each held", r#"{"note":"n","spare":"s"}"#),
];

/// One payload a member's function is called for: the name of its row, the Swift type it is read
/// as, the payload, and what the Rust read of it prints.
type MemberRow = (&'static str, &'static str, String, String);

/// Every emitted definition the runs read: the models the Rust read and the Zod schemas are held
/// to, then this file's own.
fn definitions() -> Vec<String> {
    vec![
        parcel_swift::swift_definition(),
        tagged_swift::swift_definition(),
        carrier_swift::swift_definition(),
        closed_dock_swift::swift_definition(),
        consigned_swift::swift_definition(),
        crate_swift::swift_definition(),
        crated_swift::swift_definition(),
        delivery_swift::swift_definition(),
        either_swift::swift_definition(),
        grade_swift::swift_definition(),
        handover_swift::swift_definition(),
        label_swift::swift_definition(),
        manifest_swift::swift_definition(),
        node_swift::swift_definition(),
        reference_swift::swift_definition(),
        pinned_swift::swift_definition(),
        routed_swift::swift_definition(),
        shelf_swift::swift_definition(),
        shipment_swift::swift_definition(),
        slot_swift::swift_definition(),
        stack_swift::swift_definition(),
        transfer_swift::swift_definition(),
        unnamed_swift::swift_definition(),
        weighed_swift::swift_definition(),
        wrapped_swift::swift_definition(),
        stamped_swift::swift_definition(),
        sampled_swift::swift_definition(),
        tallied_swift::swift_definition(),
        spared_swift::swift_definition(),
        loaded_swift::swift_definition(),
        listing_swift::swift_definition(),
        docked_swift::swift_definition(),
        marked_swift::swift_definition(),
        stowed_swift::swift_definition(),
        quayside_swift::swift_definition(),
        sided_swift::swift_definition(),
        twice_swift::swift_definition(),
        boxed_swift::swift_definition(),
        choice_swift::swift_definition(),
        chosen_swift::swift_definition(),
        lodged_swift::swift_definition(),
        banded_swift::swift_definition(),
        strapped_swift::swift_definition(),
        ahead_swift::swift_definition(),
        behind_swift::swift_definition(),
        paired_swift::swift_definition(),
        lone_swift::swift_definition(),
        ferried_swift::swift_definition(),
        barged_swift::swift_definition(),
        shouldered_swift::swift_definition(),
        moved_swift::swift_definition(),
        shifted_swift::swift_definition(),
        loosed_swift::swift_definition(),
        sealing_swift::swift_definition(),
        branch_swift::swift_definition(),
        binned_swift::swift_definition(),
        dated_swift::swift_definition(),
        sticker_swift::swift_definition(),
        hull_swift::swift_definition(),
    ]
}

/// What `lines` print over `declared` and `functions`, compiled with warnings as errors, or
/// `None` where no runtime was reachable.
fn printed_over(declared: Vec<String>, functions: &[&str], lines: &str) -> Option<String> {
    let mut parts = vec!["import Foundation".to_owned()];
    parts.extend(declared);
    parts.extend(functions.iter().map(|&part| part.to_owned()));
    parts.push(HARNESS.to_owned());
    parts.push(lines.to_owned());
    let source = parts.join("\n\n");
    let run = judged(
        "swift-preprocess",
        RUNTIME_VAR,
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    )?;
    assert!(
        run.succeeded,
        "`swift -warnings-as-errors main.swift` failed.\n--- stdout ---\n{}\n--- stderr ---\n{}\n--- source ---\n{source}",
        run.stdout, run.stderr
    );
    Some(run.stdout)
}

/// What `lines` print over every definition and every function of these runs.
fn printed(lines: &str) -> Option<String> {
    printed_over(definitions(), &[SHOWN_FUNCTIONS, FUNCTIONS], lines)
}

/// The Swift type a row of the shared table is read as: the name its schema is given under, with
/// the argument of a filling as Swift spells it.
fn swift_type(schema: &str) -> String {
    schema.replace("<u32>", "<UInt32>")
}

/// The line that reads `payload` as `read_as` and prints the value written back under `name`.
fn row_line(name: &str, read_as: &str, payload: &str) -> String {
    format!("row(\"{name}\", {read_as}.self, #\"{payload}\"#)")
}

/// What the Rust read of `payload` as a `Read` prints under `name`: the value written back, or
/// each message of the refusal on a line of its own.
fn rust_row<Read>(name: &str, payload: &str) -> String
where
    Read: DeserializeOwned + Serialize,
{
    match serde_json::from_str::<Read>(payload) {
        Ok(read) => format!("{name}: read {}\n", serde_json::to_string(&read).unwrap()),
        Err(refused) => {
            let mut lines = format!("{name}: refused\n");
            for message in refused.to_string().split("; ") {
                writeln!(lines, "    {message}").unwrap();
            }
            lines
        }
    }
}

/// The four rows of the sample a JSON text holds, and the payload of another type: a number a
/// function answers the text of.
#[test]
fn the_sample_of_the_documents_reads_as_the_rust_read_does() {
    assert_eq!(sample_report(), SAMPLE);
    let numbered = r#"{"label":5}"#;
    let expected = format!("{SAMPLE}{}", rust_row::<Stamped>("a number", numbered));
    assert!(
        expected.ends_with("a number: read {\"label\":\"5\"}\n"),
        "got: {expected}"
    );
    let mut lines: Vec<String> = SAMPLE_ROWS
        .iter()
        .map(|&(name, payload)| {
            let read_as = if name == "row 5" {
                "Manifest"
            } else {
                "Parcel"
            };
            row_line(name, read_as, payload)
        })
        .collect();
    lines.push(row_line("a number", "Stamped", numbered));
    let Some(swift) = printed(&lines.join("\n")) else {
        return;
    };
    assert_eq!(normalized(&swift), normalized(&expected));
}

/// Every position another type reads a preprocessed type in, and every shape the key is written
/// on, prints the one text the Rust read and the Zod schemas print.
#[test]
fn every_position_and_every_shape_reads_the_corrected_payload() {
    let read = positions();
    let rust: String = read.iter().map(|(_, _, _, row)| row.as_str()).collect();
    assert_eq!(rust, POSITIONS);
    let lines = read
        .iter()
        .map(|&(name, schema, payload, _)| row_line(name, &swift_type(schema), payload))
        .collect::<Vec<_>>()
        .join("\n");
    let Some(swift) = printed(&lines) else {
        return;
    };
    assert_eq!(normalized(&swift), normalized(POSITIONS));
}

/// A type that names a function is handed a payload of any kind: a text is read as the class and
/// as the enum under a tag key its function turns it into, as the Rust read reads it.
#[test]
fn a_text_is_read_into_a_class_and_a_tagged_enum_through_the_function() {
    let cases = [
        ("a class from a text", "Sticker", r#""A""#),
        ("a class from its object", "Sticker", r#"{"label":"A"}"#),
        ("an enum under a tag key from a text", "Hull", r#""A""#),
        (
            "an enum under a tag key from its object",
            "Hull",
            r#"{"kind":"Tall","masts":3}"#,
        ),
    ];
    let rust = |name: &str, read_as: &str, payload: &str| match read_as {
        "Sticker" => rust_row::<Sticker>(name, payload),
        _ => rust_row::<Hull>(name, payload),
    };
    let expected: String = cases
        .iter()
        .map(|&(name, read_as, payload)| rust(name, read_as, payload))
        .collect();
    assert!(expected.contains("a class from a text: read {\"label\":\"A\"}"));
    let lines = cases
        .iter()
        .map(|&(name, read_as, payload)| row_line(name, read_as, payload))
        .collect::<Vec<_>>()
        .join("\n");
    let Some(swift) = printed(&lines) else {
        return;
    };
    assert_eq!(normalized(&swift), normalized(&expected));
}

/// A function is called where the payload holds the key, a `null` included, and for no key the
/// payload leaves out: the key is then read as it is with no function named.
#[test]
fn a_function_is_called_for_a_key_the_payload_holds_and_for_no_other() {
    let mut expected = String::new();
    let mut lines = Vec::new();
    for (name, payload) in TALLIED {
        let before = tallied_calls();
        let read = serde_json::from_str::<Tallied>(payload);
        let after = tallied_calls();
        match read {
            Ok(held) => writeln!(
                expected,
                "{name}: read {}",
                serde_json::to_string(&held).unwrap()
            )
            .unwrap(),
            Err(refused) => {
                assert_eq!(
                    refused.to_string(),
                    "missing field `label` at line 1 column 2"
                );
                writeln!(expected, "{name}: no key label").unwrap();
            }
        }
        writeln!(
            expected,
            "    label {}, spare {}",
            after.0 - before.0,
            after.1 - before.1
        )
        .unwrap();
        lines.push(format!(
            "counted(\"{name}\", Tallied.self, #\"{payload}\"#)"
        ));
    }
    for (name, payload) in SPARED {
        expected.push_str(&rust_row::<Spared>(name, payload));
        lines.push(row_line(name, "Spared", payload));
    }
    assert_eq!(
        expected,
        "\
no key: no key label
    label 0, spare 0
a label: read {\"label\":\"A\"}
    label 1, spare 0
a label and a spare: read {\"label\":\"A\",\"spare\":\"s\"}
    label 1, spare 1
a label and a null: read {\"label\":\"A\"}
    label 1, spare 1
neither key: read {\"note\":null}
a null spare: read {\"note\":null,\"spare\":\"unnamed\"}
each a null: read {\"note\":\"unnamed\",\"spare\":\"unnamed\"}
each held: read {\"note\":\"n\",\"spare\":\"s\"}
",
        "what the Rust read answers for each payload"
    );
    let Some(swift) = printed(&lines.join("\n")) else {
        return;
    };
    assert_eq!(normalized(&swift), normalized(&expected));
}

/// A function is handed the Swift values of a JSON value, each JSON kind as one Swift type. A
/// number an `Int64` holds is one, written with a fraction or an exponent or with none.
#[test]
fn a_function_is_handed_the_swift_values_of_a_json_value() {
    let each = r#"{"kinds":[null,true,5,2.5,"s",[],{}]}"#;
    let expected = rust_row::<Sampled>("each kind", each);
    assert_eq!(
        expected,
        concat!(
            r#"each kind: read {"kinds":["NSNull","Bool","Int64","Double","String","#,
            r#""[Any]","[String: Any]"]}"#,
            "\n",
        )
    );
    let numbers = r#"{"kinds":[2.0,1e3,-7,1e30]}"#;
    let lines = [
        row_line("each kind", "Sampled", each),
        row_line("numbers", "Sampled", numbers),
    ]
    .join("\n");
    let Some(swift) = printed(&lines) else {
        return;
    };
    assert_eq!(
        swift,
        format!(
            "{expected}{}\n",
            r#"numbers: read {"kinds":["Int64","Int64","Int64","Double"]}"#
        )
    );
}

/// The keys the function of `Read` was handed for `payload`, as `shown` reads them off the value.
fn rust_keys<Read>(name: &str, payload: &str, shown: &dyn Fn(&Read) -> String) -> String
where
    Read: DeserializeOwned,
{
    let held: Read = serde_json::from_str(payload).unwrap();
    format!("{name}: {}\n", shown(&held))
}

/// `keys` as Swift prints a list of texts.
fn listed<Shown>(keys: &Shown) -> String
where
    Shown: Debug,
{
    format!("{keys:?}")
}

/// What the Rust read hands the function of each type read beside the keys of another: a line per
/// payload of [`KEYS_DRIVER`], in its order.
fn rust_keys_report() -> String {
    let flat = r#"{"dock":"north","tag":"t"}"#;
    let tagged = r#"{"kind":"Held","tag":"t"}"#;
    [
        rust_keys::<Pinned>("a flattened type", flat, &|held| listed(&held.tagged.keys)),
        rust_keys::<Quayside>(
            "a flattened type in a flattened holder",
            r#"{"berth":"b","dock":"north","tag":"t"}"#,
            &|held| listed(&held.pinned.tagged.keys),
        ),
        rust_keys::<Twice>(
            "two flattened types",
            r#"{"dock":"north","side":"s","tag":"t"}"#,
            &|held| format!("{} {}", listed(&held.first.keys), listed(&held.second.keys)),
        ),
        rust_keys::<Docked>("a flattened member's own function", flat, &|held| {
            listed(&held.listing.keys)
        }),
        rust_keys::<Stowed>("a variant under a tag", tagged, &|held| match held {
            Stowed::Held(inner) => listed(&inner.keys),
            Stowed::Lost => String::new(),
        }),
        rust_keys::<Marked>(
            "a slot's own function, under a tag",
            tagged,
            &|held| match held {
                Marked::Held(inner) => listed(&inner.keys),
                Marked::Lost => String::new(),
            },
        ),
        rust_keys::<Boxed>(
            "a variant under a tag that flattens",
            r#"{"kind":"Held","dock":"north","tag":"t"}"#,
            &|held| match held {
                Boxed::Held {
                    dock,
                    tagged: inner,
                } => format!("{} at {dock}", listed(&inner.keys)),
                Boxed::Lost => String::new(),
            },
        ),
        rust_keys::<Chosen>(
            "a flattened choice that holds the type",
            r#"{"choice_dock":"north","tag":"t"}"#,
            &|held| match &held.taken {
                Choice::Tagged(inner) => listed(&inner.keys),
                Choice::Named { name } => name.clone(),
            },
        ),
        rust_keys::<Lodged>(
            "a flattened enum under a tag that holds the type",
            r#"{"dock":"north","kind":"Held","tag":"t"}"#,
            &|held| match &held.stowed {
                Stowed::Held(inner) => listed(&inner.keys),
                Stowed::Lost => String::new(),
            },
        ),
        rust_keys::<Strapped>("a flattened brand over the type", flat, &|held| {
            listed(&held.banded.0.keys)
        }),
    ]
    .concat()
}

/// A function is handed the keys no type around its own took: the members of a holder that
/// flattens the type, and of a holder of that holder, and the tag of an enum that holds it.
#[test]
fn a_function_is_handed_the_keys_no_type_around_its_own_took() {
    let expected = rust_keys_report();
    assert_eq!(
        expected,
        "\
a flattened type: [\"tag\"]
a flattened type in a flattened holder: [\"tag\"]
two flattened types: [\"side\", \"tag\"] [\"side\", \"tag\"]
a flattened member's own function: [\"tag\"]
a variant under a tag: [\"tag\"]
a slot's own function, under a tag: [\"tag\"]
a variant under a tag that flattens: [\"tag\"] at north
a flattened choice that holds the type: [\"tag\"]
a flattened enum under a tag that holds the type: [\"tag\"]
a flattened brand over the type: [\"tag\"]
",
        "what the Rust read hands each function"
    );
    let Some(swift) = printed(KEYS_DRIVER) else {
        return;
    };
    assert_eq!(swift, expected);
}

/// A type declared below the type that reads it is not known to take the keys taken where its
/// reader is written, so it is read, flattened, from the payload without the keys its holder's own
/// members take, and its function is handed the keys the Rust read hands it.
#[test]
fn a_type_declared_below_its_holder_is_handed_the_keys_its_holder_leaves() {
    let flat = r#"{"dock":"north","tag":"t"}"#;
    let expected = rust_keys::<Ahead>("declared below", flat, &|held| listed(&held.behind.keys));
    assert_eq!(expected, "declared below: [\"tag\"]\n");
    let lines = format!(
        "attempt(\"declared below\", Ahead.self, #\"{flat}\"#) {{ \"\\($0.behind.keys)\" }}"
    );
    let Some(swift) = printed(&lines) else {
        return;
    };
    assert_eq!(swift, expected);
}

/// A parcel labelled `label`, with no fragile code written.
fn unfragile(label: &str) -> String {
    format!(r#"{{"label":"{label}","itemCodeList":["x"]}}"#)
}

/// That parcel in an envelope.
fn enveloped(label: &str) -> String {
    format!(r#"{{"payload":{}}}"#, unfragile(label))
}

/// A loaded value whose every member is in an envelope, with `label` in each parcel it holds.
fn loaded(label: &str) -> String {
    format!(
        concat!(
            r#"{{"byNumber":{{"payload":{{"7":"seven"}}}},"firstParcel":{{"payload":{parcel}}},"#,
            r#""parcelList":{{"payload":[{parcel}]}},"#,
            r#""parcelsByDock":{{"payload":{{"north":{parcel}}}}},"#,
            r#""spareParcel":{{"payload":{parcel}}},"title":{{"payload":"  T  "}}}}"#,
        ),
        parcel = unfragile(label),
    )
}

/// The row `name`: `payload` read as a `Read`, which Swift reads as `read_as`.
fn member_row<Read>(name: &'static str, read_as: &'static str, payload: &str) -> MemberRow
where
    Read: DeserializeOwned + Serialize,
{
    (
        name,
        read_as,
        payload.to_owned(),
        rust_row::<Read>(name, payload),
    )
}

/// A payload for each position a type reads a member in.
fn member_rows() -> Vec<MemberRow> {
    let paired = format!(r#"["  A  ",{}]"#, enveloped(" B "));
    let shifted = format!(r#"{{"kind":"Held","detail":{}}}"#, enveloped(" A "));
    let binned = [r#"["x"]"#, "[7]"].map(|codes| {
        format!(
            r#"{{"codes":{{"payload":{codes}}},"first_parcel":{}}}"#,
            enveloped("  A  ")
        )
    });
    vec![
        member_row::<Loaded>(
            "a struct, every member in an envelope",
            "Loaded",
            &loaded("  A  "),
        ),
        member_row::<Loaded>(
            "a struct, every parcel it holds broken",
            "Loaded",
            &loaded("   "),
        ),
        member_row::<Paired>("a tuple struct", "Paired", &paired),
        member_row::<Lone>("the one slot of a struct", "Lone", r#""  A  ""#),
        member_row::<Ferried>(
            "the one slot of a struct that names a function too",
            "Ferried",
            r#"{"payload":"  A  "}"#,
        ),
        member_row::<Barged>(
            "the one slot of a struct, a model",
            "Barged",
            &enveloped(" B "),
        ),
        member_row::<Barged>(
            "the one slot of a struct, a model broken",
            "Barged",
            &enveloped("   "),
        ),
        member_row::<Shouldered>(
            "the one slot of a struct, a model that names a function",
            "Shouldered",
            &unfragile("  C  "),
        ),
        member_row::<Moved>("a slot under its name", "Moved", r#"{"Rail":"  A  "}"#),
        member_row::<Moved>(
            "a member under its name",
            "Moved",
            r#"{"Road":{"plate":"  A  "}}"#,
        ),
        member_row::<Moved>(
            "slots under their name",
            "Moved",
            r#"{"Split":["  A  ",2]}"#,
        ),
        member_row::<Shifted>("a model beside its tag", "Shifted", &shifted),
        member_row::<Shifted>(
            "a slot beside its tag",
            "Shifted",
            r#"{"kind":"Rail","detail":"  A  "}"#,
        ),
        member_row::<Shifted>(
            "slots beside their tag",
            "Shifted",
            r#"{"kind":"Split","detail":["  A  ",2]}"#,
        ),
        member_row::<Loosed>("an untagged slot", "Loosed", r#""  A  ""#),
        member_row::<Loosed>("an untagged member", "Loosed", r#"{"plate":"  A  "}"#),
        member_row::<Sealing<String>>(
            "a brand with a parameter, at a text",
            "Sealing<String>",
            r#"{"payload":"A"}"#,
        ),
        member_row::<Sealing<u32>>(
            "a brand with a parameter, at a number",
            "Sealing<UInt32>",
            r#"{"payload":7}"#,
        ),
        member_row::<Branch>(
            "a member that holds the type itself",
            "Branch",
            r#"{"branches":[{"branches":null,"id":"b"}],"id":"a"}"#,
        ),
        member_row::<Binned<String>>(
            "members of a type with a parameter, at a text",
            "Binned<String>",
            &binned[0],
        ),
        member_row::<Binned<u32>>(
            "members of a type with a parameter, at a number",
            "Binned<UInt32>",
            &binned[1],
        ),
    ]
}

/// A member's function is called in every position a type reads a member in: a field over a map
/// Swift reads through a wrapper, over a model, a list, a map and an optional of models, a slot of
/// a tuple struct, the one slot of a struct serde writes as its value, a slot of a variant under
/// each tagging, and a member that holds the type itself. A model read from a corrected payload is
/// judged once, with its holder.
#[test]
fn a_member_names_a_function_in_every_position_a_type_reads_one_in() {
    let rows = member_rows();
    let expected: String = rows.iter().map(|(_, _, _, read)| read.as_str()).collect();
    for answered in [
        concat!(
            "a struct, every parcel it holds broken: refused\n",
            "    'firstParcel.label': too short: minimum length is 1, got 0\n",
            "    'parcelList.0.label': too short: minimum length is 1, got 0\n",
            "    'parcelsByDock.north.label': too short: minimum length is 1, got 0\n",
            "    'spareParcel.label': too short: minimum length is 1, got 0\n",
        ),
        concat!(
            "a member that holds the type itself: read ",
            r#"{"branches":[{"branches":[],"id":"b"}],"id":"a"}"#,
        ),
    ] {
        assert!(expected.contains(answered), "got: {expected}");
    }
    let lines = rows
        .iter()
        .map(|(name, read_as, payload, _)| row_line(name, read_as, payload))
        .collect::<Vec<_>>()
        .join("\n");
    let Some(swift) = printed(&lines) else {
        return;
    };
    assert_eq!(normalized(&swift), normalized(&expected));
}

/// A date is read from what its function answers through the text chrono writes it as.
#[cfg(feature = "chrono")]
#[test]
fn a_date_is_read_from_what_its_function_answers() {
    let payload = r#"{"at":{"payload":"2023-11-14T22:13:20.500Z"}}"#;
    let read: Timed = serde_json::from_str(payload).unwrap();
    assert_eq!(read.at.timestamp(), 1_700_000_000);
    let mut declared = definitions();
    declared.push(timed_swift::swift_definition());
    let lines = format!(
        "attempt(\"a date\", Timed.self, #\"{payload}\"#) {{ \"\\(Int($0.at.timeIntervalSince1970))\" }}"
    );
    let Some(swift) = printed_over(declared, &[SHOWN_FUNCTIONS, FUNCTIONS], &lines) else {
        return;
    };
    assert_eq!(swift, "a date: 1700000000\n");
}

/// A payload the read refuses once a function has answered is reported at the coding path of the
/// value: the field's own, under the path of whatever holds the type.
#[test]
fn a_payload_refused_after_the_correction_is_reported_at_its_coding_path() {
    let kept = r#"{"parcel":{"label":"A","itemCodeList":[]}}"#;
    let reads = [
        (
            "a flag for a text",
            "Stamped",
            r#"{"label":true}"#.to_owned(),
        ),
        (
            "a null for a text",
            "Parcel",
            r#"{"label":null,"itemCodeList":[]}"#.to_owned(),
        ),
        (
            "a number for a text, in a held parcel",
            "[Manifest]",
            format!(r#"[{kept},{{"parcel":{{"label":5,"itemCodeList":[]}}}}]"#),
        ),
        (
            "no key, in a held parcel",
            "[Manifest]",
            format!(r#"[{kept},{{"parcel":{{"label":"A"}}}}]"#),
        ),
        (
            "no key, in a member read from an envelope",
            "Loaded",
            loaded("A").replacen(r#""label":"A","#, "", 1),
        ),
        (
            "a number for a text, in a slot",
            "[Paired]",
            format!("[[5,{}]]", enveloped("A")),
        ),
        (
            "a parcel that judges itself, in a corrected type",
            "[Crate<Parcel>]",
            r#"[{"payload":{"codes":[{"label":"A","itemCodeList":[]},{"label":"   ","itemCodeList":[]}],"label":"c"}}]"#
                .to_owned(),
        ),
        (
            "an answer that is no JSON",
            "Dated",
            r#"{"stamp":"x"}"#.to_owned(),
        ),
    ];
    serde_json::from_str::<Stamped>(&reads[0].2).unwrap_err();
    serde_json::from_str::<Loaded>(&reads[4].2).unwrap_err();
    serde_json::from_str::<Vec<Paired>>(&reads[5].2).unwrap_err();
    serde_json::from_str::<Dated>(&reads[7].2).unwrap();
    let lines = reads
        .iter()
        .map(|(name, read_as, payload)| row_line(name, read_as, payload))
        .collect::<Vec<_>>()
        .join("\n");
    let Some(swift) = printed(&lines) else {
        return;
    };
    assert_eq!(
        swift,
        "\
a flag for a text: mismatch at [label]
a null for a text: no value at [label]
a number for a text, in a held parcel: mismatch at [1, parcel, label]
no key, in a held parcel: no key itemCodeList at [1, parcel]
no key, in a member read from an envelope: no key label at [firstParcel]
a number for a text, in a slot: mismatch at [0, 0]
a parcel that judges itself, in a corrected type: refused at [0, codes, 1]
    'label': too short: minimum length is 1, got 0
an answer that is no JSON: refused at [stamp]
    a preprocess answered a value that is not JSON
"
    );
}

/// A name with no function of the module does not build.
#[test]
fn a_name_with_no_function_does_not_build() {
    let source = format!(
        "import Foundation\n\n{}\n",
        parcel_swift::swift_definition()
    );
    let Some(run) = judged(
        "swift-preprocess",
        RUNTIME_VAR,
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    ) else {
        return;
    };
    assert!(!run.succeeded, "the module built: {}", run.stdout);
    for answered in [
        "error: cannot find 'trimmed' in scope",
        "error: cannot find 'withoutFragile' in scope",
    ] {
        assert!(run.stderr.contains(answered), "got: {}", run.stderr);
    }
}

/// The definitions both clients of the shipping service are compiled with.
fn client_module(client: String, driver: &str) -> String {
    let listed = SAMPLE_ROWS
        .iter()
        .take(3)
        .map(|(name, body)| format!("  (\"{name}\", #\"{body}\"#),"))
        .collect::<Vec<_>>()
        .join("\n");
    [
        "import Foundation".to_owned(),
        parcel_swift::swift_definition(),
        shipping_error_swift::swift_definition(),
        shipping_client_service_fault_fields_swift::swift_definition(),
        shipping_client_service_fault_kind_swift::swift_definition(),
        ShippingClientServiceSchema::swift_http_client(),
        client,
        SHOWN_FUNCTIONS.to_owned(),
        format!("let replies: [(String, String)] = [\n{listed}\n]"),
        driver.to_owned(),
    ]
    .join("\n\n")
}

/// What `driver` prints for the first three rows of the sample, or `None` where no runtime was
/// reachable.
fn answered_by(client: String, driver: &str) -> Option<String> {
    let source = client_module(client, driver);
    let run = judged(
        "swift-preprocess",
        RUNTIME_VAR,
        "swift",
        &[],
        "main.swift",
        &source,
    )?;
    assert!(
        run.succeeded,
        "`swift main.swift` failed.\n--- stdout ---\n{}\n--- stderr ---\n{}\n--- source ---\n{source}",
        run.stdout, run.stderr
    );
    Some(run.stdout)
}

/// Holds what a client printed to the three rows: the corrected parcel of rows 1 and 3, and one
/// fault of `kind` whose detail quotes the bound row 2 breaks once corrected.
fn assert_three_replies(swift: &str, kind: &str) {
    let lines: Vec<&str> = swift.lines().collect();
    assert_eq!(lines.len(), 4, "got: {swift}");
    assert_eq!(lines[0], "row 1: the parcel A, fragile [\"x\"]");
    assert_eq!(lines[1], format!("row 2: the fault {kind}"));
    assert!(
        lines[2].contains("'label': too short: minimum length is 1, got 0"),
        "got: {swift}"
    );
    assert_eq!(lines[3], "row 3: the parcel A, fragile []");
}

/// The `http_rest` client reads a reply through the functions of its type.
#[test]
fn the_http_client_reads_a_reply_through_its_functions() {
    assert_eq!(
        poll_once(ShippingBackEnd.shipped(&(), String::new()))
            .unwrap()
            .map(|parcel| parcel.label),
        Err(ShippingError::NotFound)
    );
    let Some(swift) = answered_by(String::new(), HTTP_DRIVER) else {
        return;
    };
    assert_three_replies(&swift, "undeserializable-payload");
}

/// The `ws_rpc` client reads a reply through the functions of its type.
#[cfg(slow_tests)]
#[test]
fn the_ws_client_reads_a_reply_through_its_functions() {
    assert_eq!(
        poll_once(ShippingBackEnd.shipped(&(), "A".to_owned()))
            .unwrap()
            .map(|parcel| parcel.label),
        Ok("A".to_owned())
    );
    let Some(swift) = answered_by(ShippingClientServiceSchema::swift_ws_client(), WS_DRIVER) else {
        return;
    };
    assert_three_replies(&swift, "failed-validation");
}

/// The parcel of the documents, as the emitter writes it: the payload type once, the read of a
/// corrected payload, and the two reads that call the type's function.
#[test]
fn the_parcel_of_the_documents_is_written_with_its_two_functions() {
    let swift = parcel_swift::swift_definition();
    assert_eq!(
        swift
            .matches("private struct SwiftSchemaPayload: Decodable")
            .count(),
        1
    );
    for written in [
        concat!(
            "private struct SwiftSchemaRead: Decodable { let value: Parcel; ",
            "init(from decoder: Decoder) throws { value = try Parcel(swiftSchemaCorrected: decoder) } }; ",
            "private init(swiftSchemaCorrected decoder: Decoder) throws { ",
            "let container = try decoder.container(keyedBy: CodingKeys.self); ",
            "self.fragileItemCodes = try container.decode([String].self, forKey: .fragileItemCodes); ",
            "self.itemCodeList = try container.decode([String].self, forKey: .itemCodeList); ",
            "let labelPayload = try container.decode(SwiftSchemaPayload.self, forKey: .label).value; ",
            "self.label = try SwiftSchemaPayload.read(String.self, from: trimmed(labelPayload), ",
            "at: container.codingPath + [CodingKeys.label]) }; ",
        ),
        concat!(
            "public init(swiftSchemaUnjudged decoder: Decoder, swiftSchemaWithout taken: [String]) throws { ",
            "self = try SwiftSchemaPayload.read(SwiftSchemaRead.self, ",
            "from: withoutFragile(try SwiftSchemaPayload(from: decoder, without: taken).value), ",
            "at: decoder.codingPath).value }; ",
            "public init(swiftSchemaUnjudged decoder: Decoder) throws { ",
            "try self.init(swiftSchemaUnjudged: decoder, swiftSchemaWithout: []) }; ",
            "public init(from decoder: Decoder) throws { try self.init(swiftSchemaUnjudged: decoder); ",
        ),
    ] {
        assert!(swift.contains(written), "{written} is not in: {swift}");
    }
    let manifest = manifest_swift::swift_definition();
    assert!(!manifest.contains("SwiftSchemaPayload"), "got: {manifest}");
}

/// A struct of one slot that is no brand is read as serde writes it, as the slot's value: the
/// slot's function is called on the struct's own payload.
#[test]
fn the_one_slot_of_a_struct_calls_its_function_on_the_structs_own_payload() {
    assert_eq!(serde_json::from_str::<Lone>(r#""  A  ""#).unwrap().0, "A");
    let swift = lone_swift::swift_definition();
    let written = concat!(
        "public init(swiftSchemaUnjudged decoder: Decoder) throws { ",
        "let valuePayload = try SwiftSchemaPayload(from: decoder).value; ",
        "self.value = try SwiftSchemaPayload.read(String.self, from: trimmed(valuePayload), ",
        "at: decoder.codingPath) }; ",
    );
    assert!(swift.contains(written), "{written} is not in: {swift}");
    assert!(!swift.contains("LoneTuple"), "got: {swift}");
}

/// The README shows the functions these runs compile, the read they make and what it answers.
#[test]
fn the_readme_shows_what_these_runs_compile_and_read() {
    let readme = include_str!("../../README.md");
    for shown in [
        SHOWN_FUNCTIONS,
        "func trimmed(_ payload: Any) -> Any",
        "func withoutFragile(_ payload: Any) -> Any",
        "let labelPayload = try container.decode(SwiftSchemaPayload.self, forKey: .label).value",
        "self.label = try SwiftSchemaPayload.read(String.self, from: trimmed(labelPayload), at: container.codingPath + [CodingKeys.label])",
        "error: cannot find 'trimmed' in scope",
        "a preprocess answered a value that is not JSON",
        "init(swiftSchemaUnjudged decoder: Decoder, swiftSchemaWithout taken: [String]) throws",
    ] {
        assert!(
            readme.contains(shown),
            "the README no longer shows: {shown}"
        );
    }
}
