//! The emitted Swift read run under a Swift toolchain, with warnings as errors. `init(from:)`
//! refuses what `validate()` refuses, with every message of the value in one error, and
//! `init(swiftSchemaUnjudged:)` builds the value from the same payload. A refusal is held to what
//! the Rust `validate()` answers of the value, and to the Rust read of the payload.

#![cfg(feature = "swift")]

use alloc::collections::BTreeMap;
use core::fmt::{Debug, Write as _};

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::run_swift_validate::{
    FUNCTIONS, SHOWN_FUNCTIONS, broken, definitions, depot, gauge, package,
};
use super::runtime::judged;
use super::swift_read_models::tracking_client_service_schema::{
    tracking_client_service_fault_fields_swift, tracking_client_service_fault_kind_swift,
};
use super::swift_read_models::{
    Anchorage, Decking, Manifest, Mooring, Pin, Signet, Sorted, Towed, TrackingBackEnd,
    TrackingClientService as _, TrackingClientServiceSchema, TrackingError, Urgency, Vetted,
    anchorage_swift, decking_swift, manifest_swift, mooring_swift, pin_swift, signet_swift,
    sorted_swift, towed_swift, tracking_error_swift, urgency_swift, vetted_swift,
};
use super::swift_validate_models::{
    Base, Berth, Cargo, ClosedDock, Code, Consignment, Crate, Docket, Freight, Grade, Haul,
    Inspection, LaterCode, Ledger, Lift, Parcel, SealedCode, Title, Waybill, Yard, parcel_swift,
};
use super::tests::poll_once;

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// A parcel that keeps every rule.
const GOOD: &str = r#"{"fragileItemCodes":["x"],"itemCodeList":["x"],"label":"A"}"#;

/// A package that keeps every rule.
const PACKED: &str = r#"{"itemLabel":"A","parcelCode":"ab"}"#;

/// A parcel with no label.
const UNLABELLED: &str = r#"{"fragileItemCodes":["x"],"itemCodeList":["x"],"label":""}"#;

/// A parcel with a fragile code outside of its codes.
const OUTSIDE: &str = r#"{"fragileItemCodes":["x","y"],"itemCodeList":["x"],"label":"A"}"#;

/// The messages of the reply that breaks two rules, as the read joins them.
const TWO_MESSAGES: &str = concat!(
    "'label': too short: minimum length is 1, got 0; ",
    "'fragileItemCodes.0': y is outside of itemCodeList",
);

/// The functions the checks of the read's own models name, written by hand in Swift.
const READ_FUNCTIONS: &str = r#"
func signetPressed(_ value: Signet) -> [String] {
  value.value.contains(where: { $0.isLowercase }) ? ["a signet is pressed in upper case"] : []
}

func withAnUrgency(_ value: Urgency) -> [String] {
  if case .low = value { return ["a low urgency is not carried"] }
  return []
}
"#;

/// What a run reads a payload with through `Decodable`, and prints the answer with.
const JUDGING: &str = r#"
func place(_ path: [CodingKey]) -> String {
  path.map { $0.intValue.map(String.init) ?? $0.stringValue }.joined(separator: ", ")
}

func written<Held: Encodable>(_ held: Held) -> String {
  let encoder = JSONEncoder()
  encoder.outputFormatting = [.sortedKeys]
  return String(data: try! encoder.encode(held), encoding: .utf8)!
}

func attempt<Read>(_ name: String, _ read: () throws -> Read, _ shown: (Read) -> String) {
  do {
    let held = try read()
    print("\(name): \(shown(held))")
  } catch DecodingError.dataCorrupted(let context) {
    print("\(name): refused at [\(place(context.codingPath))]")
    for line in context.debugDescription.components(separatedBy: "; ") { print("    \(line)") }
  } catch DecodingError.keyNotFound(let key, let context) {
    print("\(name): no key \(key.stringValue) at [\(place(context.codingPath))]")
  } catch DecodingError.valueNotFound(_, let context) {
    print("\(name): no value at [\(place(context.codingPath))]")
  } catch DecodingError.typeMismatch(_, let context) {
    print("\(name): mismatch at [\(place(context.codingPath))]: \(context.debugDescription)")
  } catch {
    print("\(name): \(error)")
  }
}

func judged<Read: Decodable>(_ name: String, _ type: Read.Type, _ payload: String) {
  attempt(name, { try JSONDecoder().decode(type, from: Data(payload.utf8)) }, { _ in "read" })
}

func reread<Read: Codable>(_ name: String, _ type: Read.Type, _ payload: String) {
  attempt(name, { try JSONDecoder().decode(type, from: Data(payload.utf8)) }, { "read \(written($0))" })
}

func chosen<Read: Decodable>(_ name: String, _ type: Read.Type, _ payload: String, _ named: (Read) -> String) {
  attempt(name, { try JSONDecoder().decode(type, from: Data(payload.utf8)) }, named)
}
"#;

/// What a run reads a payload with through the unjudged initializer of the type it names.
const UNJUDGED: &str = r#"
protocol ReadUnjudged {
  init(swiftSchemaUnjudged decoder: Decoder) throws
}

struct Unjudged<Held: ReadUnjudged>: Decodable {
  let held: Held
  init(from decoder: Decoder) throws { held = try Held(swiftSchemaUnjudged: decoder) }
}

func built<Read: ReadUnjudged & Encodable>(_ name: String, _ type: Read.Type, _ payload: String, _ judge: (Read) -> [String]) {
  let held = try! JSONDecoder().decode(Unjudged<Read>.self, from: Data(payload.utf8)).held
  print("\(name), unjudged: built \(written(held))")
  for violation in judge(held) { print("    \(violation)") }
}
"#;

/// The variant of each untagged enum a value holds, by its Rust name.
const VARIANTS: &str = r#"
func variant(_ held: Sorted) -> String {
  switch held {
  case .brief: return "Brief"
  case .coded: return "Coded"
  case .counted: return "Counted"
  case .listed: return "Listed"
  case .mapped: return "Mapped"
  case .whole: return "Whole"
  case .wider: return "Wider"
  case .wordy: return "Wordy"
  }
}

func variant(_ held: Vetted) -> String {
  switch held {
  case .signed: return "Signed"
  case .worded: return "Worded"
  }
}
"#;

/// A transport that answers every request with one body, and what a call's answer is printed as.
const HTTP_DRIVER: &str = r#"
struct Answering: TrackingClientServiceHttpTransport {
  let body: String
  func send(_ request: TrackingClientServiceHttpRequest) async throws -> TrackingClientServiceHttpResponse {
    TrackingClientServiceHttpResponse(status: 200, headers: [], body: Data(body.utf8))
  }
}

func described(_ name: String, _ answer: Result<Parcel, TrackingClientServiceTrackFailure>) {
  switch answer {
  case .success(let parcel):
    print("\(name): the parcel \(parcel.label)")
  case .failure(.declared):
    print("\(name): the declared error")
  case .failure(.fault(let fault)):
    print("\(name): the fault \(fault.kind.rawValue)")
    print("    \(fault.detail)")
  }
}

for (name, body) in replies {
  let client = TrackingClientServiceHttpClient(transport: Answering(body: body))
  described(name, await client.track("A"))
}
"#;

/// A socket that hands the transport one reply for the request it sent, and what a call's answer
/// is printed as.
#[cfg(slow_tests)]
const WS_DRIVER: &str = r#"
final class InMemorySocket: TrackingClientServiceWsSocket, @unchecked Sendable {
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

func described(_ name: String, _ answer: Result<Parcel, TrackingClientServiceTrackFailure>) {
  switch answer {
  case .success(let parcel):
    print("\(name): the parcel \(parcel.label)")
  case .failure(.declared):
    print("\(name): the declared error")
  case .failure(.fault(let fault)):
    print("\(name): the fault \(fault.kind.rawValue)")
    print("    \(fault.detail)")
  }
}

for (name, body) in replies {
  let socket = InMemorySocket()
  let transport = TrackingClientServiceWsTransport(socket: socket, options: .init(heartbeat: nil))
  async let answer = transport.track("A")
  while socket.sent.isEmpty { try? await Task.sleep(nanoseconds: 5_000_000) }
  let sent = try! JSONDecoder().decode(TrackingClientServiceWsFrameProbe.self, from: Data(socket.sent[0].utf8))
  socket.onMessage?("{\"kind\":\"reply\",\"id\":\"\(sent.id!)\",\"service\":\"TrackingClientService\",\"ok\":true,\"value\":\(body)}")
  described(name, await answer)
  await transport.close()
}
"#;

/// One value as Swift reads the payload serde writes for it, and what the Rust `validate()`
/// answers of it.
struct Row {
    answered: Result<(), Vec<String>>,
    name: &'static str,
    payload: String,
    swift_type: &'static str,
}

/// One row of a value both sides read, with what the Rust read answers for its payload where
/// `reads_alike` holds the two reads to one answer.
fn row<Held>(
    name: &'static str,
    swift_type: &'static str,
    held: &Held,
    answered: Result<(), Vec<String>>,
    reads_alike: bool,
) -> Row
where
    Held: Serialize + DeserializeOwned,
{
    let payload = serde_json::to_string(held).unwrap();
    if reads_alike {
        assert_eq!(
            rust_read::<Held>(&payload),
            answered,
            "{name}: the Rust read and the Rust `validate()` answer one list"
        );
    }
    Row {
        answered,
        name,
        payload,
        swift_type,
    }
}

/// What the Rust read answers for `payload`: nothing, or each message of its refusal.
fn rust_read<Read>(payload: &str) -> Result<(), Vec<String>>
where
    Read: DeserializeOwned,
{
    serde_json::from_str::<Read>(payload)
        .map(drop)
        .map_err(|refused| refused.to_string().split("; ").map(str::to_owned).collect())
}

/// `held` with every number written as a float, the form both sides' whole floats agree in,
/// and the keys of every object in order.
fn plain(held: &Value) -> Value {
    match held {
        Value::Number(number) => number.as_f64().map_or(Value::Null, Value::from),
        Value::Array(items) => Value::Array(items.iter().map(plain).collect()),
        Value::Object(entries) => {
            let sorted: BTreeMap<&String, Value> = entries
                .iter()
                .map(|(key, entry)| (key, plain(entry)))
                .collect();
            Value::Object(
                sorted
                    .into_iter()
                    .map(|(key, entry)| (key.clone(), entry))
                    .collect(),
            )
        }
        Value::Null | Value::Bool(_) | Value::String(_) => held.clone(),
    }
}

/// `printed` with the value after each `built ` and `read ` written as [`plain`] writes it.
pub fn normalized(printed: &str) -> String {
    let mut lines = String::new();
    for line in printed.lines() {
        let split = [": read ", ", unjudged: built "]
            .into_iter()
            .find_map(|marker| line.split_once(marker).map(|halves| (marker, halves)));
        match split {
            Some((marker, (name, json))) => {
                let held: Value = serde_json::from_str(json).unwrap();
                writeln!(lines, "{name}{marker}{}", plain(&held)).unwrap();
            }
            None => writeln!(lines, "{line}").unwrap(),
        }
    }
    lines
}

/// What the Rust side prints for `rows`: the read's answer, then the value the unjudged
/// initializer builds with what `validate()` answers of it.
fn rust_report(rows: &[Row]) -> String {
    let mut printed = String::new();
    for held in rows {
        let value: Value = serde_json::from_str(&held.payload).unwrap();
        let violations = held.answered.clone().err().unwrap_or_default();
        if violations.is_empty() {
            writeln!(printed, "{}: read", held.name).unwrap();
        } else {
            writeln!(printed, "{}: refused at []", held.name).unwrap();
        }
        for violation in &violations {
            writeln!(printed, "    {violation}").unwrap();
        }
        writeln!(printed, "{}, unjudged: built {}", held.name, plain(&value)).unwrap();
        for violation in &violations {
            writeln!(printed, "    {violation}").unwrap();
        }
    }
    printed
}

/// The lines that read each of `rows` both ways.
fn driver_of(rows: &[Row]) -> String {
    rows.iter()
        .map(|held| {
            format!(
                "judged(\"{name}\", {ty}.self, #\"{payload}\"#)\n\
                 built(\"{name}\", {ty}.self, #\"{payload}\"#) {{ $0.validate() }}",
                name = held.name,
                ty = held.swift_type,
                payload = held.payload,
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every emitted definition the runs read: those of the `validate()` runs with the manifest of
/// this file in place of theirs, then this file's own.
fn declared() -> Vec<String> {
    let mut written = definitions();
    written.retain(|definition| !definition.starts_with("public struct Manifest:"));
    written.extend([
        manifest_swift::swift_definition(),
        mooring_swift::swift_definition(),
        anchorage_swift::swift_definition(),
        decking_swift::swift_definition(),
        pin_swift::swift_definition(),
        signet_swift::swift_definition(),
        sorted_swift::swift_definition(),
        vetted_swift::swift_definition(),
        towed_swift::swift_definition(),
        urgency_swift::swift_definition(),
    ]);
    written
}

/// The types `lines` read through the unjudged initializer, each once, as the conformance the
/// harness asks of them.
fn conformances(lines: &str) -> String {
    let mut named: Vec<&str> = lines
        .split("built(\"")
        .skip(1)
        .filter_map(|after| {
            after
                .split_once("\", ")?
                .1
                .split_once(".self")?
                .0
                .split('<')
                .next()
        })
        .collect();
    named.sort_unstable();
    named.dedup();
    named
        .iter()
        .map(|read| format!("extension {read}: ReadUnjudged {{}}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// What `lines` print over `definitions`, compiled with warnings as errors, or `None` where no
/// runtime was reachable.
fn printed(definitions: Vec<String>, harness: &[&str], lines: &str) -> Option<String> {
    let mut parts = vec!["import Foundation".to_owned()];
    parts.extend(definitions);
    parts.extend([
        SHOWN_FUNCTIONS.to_owned(),
        FUNCTIONS.to_owned(),
        READ_FUNCTIONS.to_owned(),
    ]);
    parts.extend(harness.iter().map(|&part| part.to_owned()));
    parts.push(conformances(lines));
    parts.push(lines.to_owned());
    let source = parts.join("\n\n");
    let run = judged(
        "swift-read",
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

/// The manifest of the documents: a parcel in a field, a list, a map and an optional.
fn manifest_payload(first: &str, listed: &[&str], north: &str, spare: Option<&str>) -> String {
    let spared = spare.map_or_else(String::new, |held| format!(r#","spareParcel":{held}"#));
    format!(
        r#"{{"firstParcel":{first},"parcelList":[{}],"parcelsByDock":{{"north":{north}}}{spared}}}"#,
        listed.join(",")
    )
}

/// The five reads of the design: a model alone, a holder of four broken ones, a list of holders, a
/// model behind a type parameter, and a payload that breaks nothing.
#[test]
fn a_read_refuses_with_every_message_of_the_value_at_its_place() {
    let broken_manifest = manifest_payload(UNLABELLED, &[GOOD, OUTSIDE], UNLABELLED, Some(OUTSIDE));
    let kept_manifest = manifest_payload(GOOD, &[GOOD], GOOD, Some(GOOD));
    let unspared = manifest_payload(GOOD, &[], GOOD, None);
    let reads = [
        ("Parcel alone", "Parcel", OUTSIDE.to_owned()),
        (
            "Manifest, four broken parcels",
            "Manifest",
            broken_manifest.clone(),
        ),
        (
            "[Manifest], the second broken",
            "[Manifest]",
            format!("[{kept_manifest},{broken_manifest}]"),
        ),
        (
            "Crate<Parcel>, the second code broken",
            "Crate<Parcel>",
            format!(r#"{{"codes":[{GOOD},{OUTSIDE}],"label":"A"}}"#),
        ),
        (
            "Manifest with no spareParcel key, all good",
            "Manifest",
            unspared,
        ),
    ];
    let four = [
        "'firstParcel.label': too short: minimum length is 1, got 0",
        "'parcelList.1.fragileItemCodes.1': y is outside of itemCodeList",
        "'parcelsByDock.north.label': too short: minimum length is 1, got 0",
        "'spareParcel.fragileItemCodes.1': y is outside of itemCodeList",
    ];
    assert_eq!(
        rust_read::<Parcel>(OUTSIDE),
        Err(vec![
            "'fragileItemCodes.1': y is outside of itemCodeList".to_owned()
        ])
    );
    assert_eq!(
        rust_read::<Manifest>(&broken_manifest),
        Err(four.map(str::to_owned).to_vec()),
        "the Rust read answers the four messages the Swift read is held to"
    );
    let lines = reads
        .iter()
        .map(|(name, read_as, payload)| {
            format!("judged(\"{name}\", {read_as}.self, #\"{payload}\"#)")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let Some(swift) = printed(declared(), &[JUDGING], &lines) else {
        return;
    };
    assert_eq!(
        swift,
        "\
Parcel alone: refused at []
    'fragileItemCodes.1': y is outside of itemCodeList
Manifest, four broken parcels: refused at []
    'firstParcel.label': too short: minimum length is 1, got 0
    'parcelList.1.fragileItemCodes.1': y is outside of itemCodeList
    'parcelsByDock.north.label': too short: minimum length is 1, got 0
    'spareParcel.fragileItemCodes.1': y is outside of itemCodeList
[Manifest], the second broken: refused at [1]
    'firstParcel.label': too short: minimum length is 1, got 0
    'parcelList.1.fragileItemCodes.1': y is outside of itemCodeList
    'parcelsByDock.north.label': too short: minimum length is 1, got 0
    'spareParcel.fragileItemCodes.1': y is outside of itemCodeList
Crate<Parcel>, the second code broken: refused at [codes, 1]
    'fragileItemCodes.1': y is outside of itemCodeList
Manifest with no spareParcel key, all good: read
"
    );
}

/// A struct of bounds, and each holder of a model: in a field, a list of lists, a map under each
/// kind of key, an optional, a tuple and a flattened member.
fn struct_rows() -> Vec<Row> {
    let below = gauge(0.25, 0, 0, "a", true);
    let within = gauge(2.5, 2, 40, "abc", false);
    let unlabelled = depot(&broken, "");
    let kept = depot(&|| package("A", "ab"), "home");
    let yard = Yard {
        by_grade: BTreeMap::from([(Grade::Standard, broken()), (Grade::Express, broken())]),
        docks_in_order: vec![
            BTreeMap::new(),
            BTreeMap::from([("north".to_owned(), broken())]),
        ],
        lists_by_dock: BTreeMap::from([("north".to_owned(), vec![package("A", "ab"), broken()])]),
        maybe_each: vec![None, Some(broken())],
        spare_list: Some(vec![broken()]),
    };
    let ledger = Ledger {
        parcel_list: vec![broken(), package("", "ab")],
    };
    let waybill = Waybill {
        later_code: LaterCode("a".to_owned()),
        short_code: Code("abcd".to_owned()),
        titled: Title::from("t"),
    };
    let inspection = Inspection {
        errors: Vec::new(),
        validate: "v".to_owned(),
    };
    let decking = Decking {
        by_deck: BTreeMap::from([(-1, broken()), (4, package("A", "ab"))]),
    };
    let mooring = Mooring {
        berth_note: None,
        held_package: Some(broken()),
        spare_note: Some("s".to_owned()),
        spare_package: Some(package("", "ab")),
    };
    let anchorage = Anchorage {
        anchor_note: Some("a".to_owned()),
        held_package: Some(broken()),
        starting_base: Base {
            base_name: String::new(),
        },
    };
    vec![
        row("Gauge, below", "Gauge", &below, below.validate(), true),
        row("Gauge, held", "Gauge", &within, within.validate(), true),
        row(
            "Depot, every package broken",
            "Depot",
            &unlabelled,
            unlabelled.validate(),
            false,
        ),
        row("Depot, held", "Depot", &kept, kept.validate(), true),
        row("Yard", "Yard", &yard, yard.validate(), true),
        row("Ledger", "Ledger", &ledger, ledger.validate(), true),
        row("Waybill", "Waybill", &waybill, waybill.validate(), true),
        row(
            "Inspection",
            "Inspection",
            &inspection,
            inspection.validate(),
            true,
        ),
        row("Decking", "Decking", &decking, decking.validate(), true),
        row("Mooring", "Mooring", &mooring, mooring.validate(), true),
        row(
            "Anchorage",
            "Anchorage",
            &anchorage,
            anchorage.validate(),
            true,
        ),
    ]
}

/// A brand, a brand over a brand, a brand and a struct with a parameter, a unit struct and a
/// tuple struct.
fn wrapper_rows() -> Vec<Row> {
    let short = Code("a".to_owned());
    let sealed = SealedCode(Code("abcde".to_owned()));
    let named = Docket("a".to_owned());
    let numbered = Docket(7_u32);
    let empty: Crate<String> = Crate {
        codes: Vec::new(),
        label: String::new(),
    };
    let consignment = Consignment(String::new(), package("", "ab"));
    vec![
        row("Code", "Code", &short, short.validate(), true),
        row("SealedCode", "SealedCode", &sealed, sealed.validate(), true),
        row(
            "Docket<String>",
            "Docket<String>",
            &named,
            named.validate(),
            true,
        ),
        row(
            "Docket<UInt32>",
            "Docket<UInt32>",
            &numbered,
            numbered.validate(),
            true,
        ),
        row(
            "Crate<String>",
            "Crate<String>",
            &empty,
            empty.validate(),
            true,
        ),
        row(
            "ClosedDock",
            "ClosedDock",
            &ClosedDock,
            ClosedDock.validate(),
            true,
        ),
        row(
            "Consignment",
            "Consignment",
            &consignment,
            consignment.validate(),
            true,
        ),
    ]
}

/// An enum under each tagging, with a slot, a named member, several slots and a model.
fn enum_rows() -> Vec<Row> {
    let boxed = Freight::Boxed(String::new());
    let loose = Freight::Loose {
        freight_label: String::new(),
    };
    let split = Freight::Split(String::new(), 0);
    let cargo = Cargo::Boxed(package("", "ab"));
    let berth = Berth::Named {
        quay_name: "a".to_owned(),
        quay_number: 0,
    };
    let haul_boxed = Haul::Boxed(String::new());
    let haul_loose = Haul::Loose {
        haul_label: String::new(),
    };
    let haul_split = Haul::Split(String::new(), 0);
    let lift_loose = Lift::Loose {
        lift_label: String::new(),
    };
    let lift_boxed = Lift::Boxed(package("", "ab"));
    let towed = Towed::Boxed(broken());
    vec![
        row("Freight, a slot", "Freight", &boxed, boxed.validate(), true),
        row(
            "Freight, a member",
            "Freight",
            &loose,
            loose.validate(),
            true,
        ),
        row(
            "Freight, two slots",
            "Freight",
            &split,
            split.validate(),
            true,
        ),
        row(
            "Freight, no member",
            "Freight",
            &Freight::Unladen,
            Freight::Unladen.validate(),
            true,
        ),
        row("Cargo", "Cargo", &cargo, cargo.validate(), true),
        row("Berth", "Berth", &berth, berth.validate(), true),
        row(
            "Haul, a slot",
            "Haul",
            &haul_boxed,
            haul_boxed.validate(),
            true,
        ),
        row(
            "Haul, a member",
            "Haul",
            &haul_loose,
            haul_loose.validate(),
            true,
        ),
        row(
            "Haul, two slots",
            "Haul",
            &haul_split,
            haul_split.validate(),
            true,
        ),
        row(
            "Lift, a member",
            "Lift",
            &lift_loose,
            lift_loose.validate(),
            true,
        ),
        row(
            "Lift, a model",
            "Lift",
            &lift_boxed,
            lift_boxed.validate(),
            true,
        ),
        row(
            "Lift, its own check",
            "Lift",
            &Lift::Unladen,
            Lift::Unladen.validate(),
            true,
        ),
        row("Towed", "Towed", &towed, towed.validate(), true),
    ]
}

/// A plain enum that names no check, and one that names one.
fn plain_enum_rows() -> Vec<Row> {
    vec![
        row("Grade", "Grade", &Grade::Express, Ok(()), true),
        row(
            "Urgency, low",
            "Urgency",
            &Urgency::Low,
            Urgency::Low.validate(),
            true,
        ),
        row(
            "Urgency, high",
            "Urgency",
            &Urgency::High,
            Urgency::High.validate(),
            true,
        ),
    ]
}

/// Each shape the emitter writes refuses a payload with the list its `validate()` answers of the
/// value, and its unjudged initializer builds that value from the same payload.
#[test]
fn every_shape_refuses_what_its_validate_refuses_and_builds_it_unjudged() {
    let mut rows = struct_rows();
    rows.extend(wrapper_rows());
    rows.extend(enum_rows());
    rows.extend(plain_enum_rows());
    let expected = rust_report(&rows);
    let refused = rows.iter().filter(|read| read.answered.is_err()).count();
    assert_eq!(
        (rows.len(), refused),
        (34, 28),
        "each shape with a broken rule, and six values that keep every one"
    );
    let Some(swift) = printed(declared(), &[JUDGING, UNJUDGED], &driver_of(&rows)) else {
        return;
    };
    assert_eq!(normalized(&swift), expected);
}

/// A field with a bound of its own is refused with the holder's `validate()` text, and a model its
/// holder does not walk, the value a brand wraps, judges itself as it is read.
#[test]
fn a_model_its_holder_does_not_walk_judges_itself() {
    let waybill = Waybill {
        later_code: LaterCode("ab".to_owned()),
        short_code: Code("a".to_owned()),
        titled: Title::from("ti"),
    };
    assert_eq!(
        waybill.validate(),
        Err(vec![
            "'shortCode': too short: minimum length is 2, got 1".to_owned()
        ])
    );
    let sealed = SealedCode(Code("a".to_owned()));
    assert_eq!(sealed.validate(), Ok(()));
    let reads = [
        (
            "Waybill",
            "Waybill",
            serde_json::to_string(&waybill).unwrap(),
        ),
        (
            "SealedCode",
            "SealedCode",
            serde_json::to_string(&sealed).unwrap(),
        ),
    ];
    serde_json::from_str::<Waybill>(&reads[0].2).unwrap_err();
    serde_json::from_str::<SealedCode>(&reads[1].2).unwrap_err();
    let lines = reads
        .iter()
        .map(|(name, read_as, payload)| {
            format!("judged(\"{name}\", {read_as}.self, #\"{payload}\"#)")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let Some(swift) = printed(declared(), &[JUDGING], &lines) else {
        return;
    };
    assert_eq!(
        swift,
        "\
Waybill: refused at []
    'shortCode': too short: minimum length is 2, got 1
SealedCode: refused at []
    too short: minimum length is 2, got 1
"
    );
}

/// A payload per rule an absent key and a `null` are read by: what it is named, the type it is
/// read as, and the payload.
fn absent_and_null() -> [(&'static str, &'static str, String); 15] {
    let mut unnumbered = serde_json::to_value(depot(&|| package("A", "ab"), "home")).unwrap();
    unnumbered["byNumber"] = serde_json::json!({ "x": { "itemLabel": "A", "parcelCode": "ab" } });
    [
        ("Mooring, no key", "Mooring", "{}".to_owned()),
        (
            "Mooring, each a null",
            "Mooring",
            r#"{"berthNote":null,"heldPackage":null,"spareNote":null,"sparePackage":null}"#
                .to_owned(),
        ),
        (
            "Mooring, each held",
            "Mooring",
            format!(
                r#"{{"berthNote":"m","heldPackage":{PACKED},"spareNote":"s","sparePackage":{PACKED}}}"#
            ),
        ),
        (
            "Anchorage, no anchorNote",
            "Anchorage",
            r#"{"baseName":"home","heldPackage":null}"#.to_owned(),
        ),
        (
            "Anchorage, no heldPackage",
            "Anchorage",
            r#"{"anchorNote":null,"baseName":"home"}"#.to_owned(),
        ),
        (
            "Anchorage, each a null",
            "Anchorage",
            r#"{"anchorNote":null,"baseName":"home","heldPackage":null}"#.to_owned(),
        ),
        (
            "Manifest, no firstParcel",
            "Manifest",
            r#"{"parcelList":[],"parcelsByDock":{}}"#.to_owned(),
        ),
        (
            "Manifest, firstParcel a null",
            "Manifest",
            r#"{"firstParcel":null,"parcelList":[],"parcelsByDock":{}}"#.to_owned(),
        ),
        (
            "Manifest, no parcelList",
            "Manifest",
            format!(r#"{{"firstParcel":{GOOD},"parcelsByDock":{{}}}}"#),
        ),
        (
            "Manifest, no parcelsByDock",
            "Manifest",
            format!(r#"{{"firstParcel":{GOOD},"parcelList":[]}}"#),
        ),
        (
            "Manifest, no spareParcel",
            "Manifest",
            format!(r#"{{"firstParcel":{GOOD},"parcelList":[],"parcelsByDock":{{}}}}"#),
        ),
        (
            "Manifest, spareParcel a null",
            "Manifest",
            format!(
                r#"{{"firstParcel":{GOOD},"parcelList":[],"parcelsByDock":{{}},"spareParcel":null}}"#
            ),
        ),
        (
            "Towed, no content",
            "Towed",
            r#"{"kind":"Boxed"}"#.to_owned(),
        ),
        (
            "Decking, a key that is no number",
            "Decking",
            format!(r#"{{"byDeck":{{"x":{PACKED}}}}}"#),
        ),
        (
            "Depot, a key that is no number",
            "Depot",
            unnumbered.to_string(),
        ),
    ]
}

/// An absent key and a `null` are read as they were before a read was written: these answers are
/// the ones the unchanged emitter gave. serde reads the absent key of a nullable member as `None`
/// in both structs.
#[test]
fn an_absent_key_and_a_null_are_read_as_they_were() {
    let unnoted: Anchorage =
        serde_json::from_str(r#"{"baseName":"home","heldPackage":null}"#).unwrap();
    assert_eq!(unnoted.anchor_note, None);
    let unmoored: Mooring = serde_json::from_str("{}").unwrap();
    assert_eq!(unmoored.berth_note, None);
    let lines = absent_and_null()
        .iter()
        .map(|(name, read_as, payload)| {
            format!("reread(\"{name}\", {read_as}.self, #\"{payload}\"#)")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let Some(swift) = printed(declared(), &[JUDGING], &lines) else {
        return;
    };
    assert_eq!(
        swift,
        format!(
            "\
Mooring, no key: read {{\"berthNote\":null,\"heldPackage\":null}}
Mooring, each a null: read {{\"berthNote\":null,\"heldPackage\":null}}
Mooring, each held: read {{\"berthNote\":\"m\",\"heldPackage\":{PACKED},\"spareNote\":\"s\",\"sparePackage\":{PACKED}}}
Anchorage, no anchorNote: no key anchorNote at []
Anchorage, no heldPackage: no key heldPackage at []
Anchorage, each a null: read {{\"anchorNote\":null,\"baseName\":\"home\",\"heldPackage\":null}}
Manifest, no firstParcel: no key firstParcel at []
Manifest, firstParcel a null: no value at [firstParcel]
Manifest, no parcelList: no key parcelList at []
Manifest, no parcelsByDock: no key parcelsByDock at []
Manifest, no spareParcel: read {{\"firstParcel\":{GOOD},\"parcelList\":[],\"parcelsByDock\":{{}}}}
Manifest, spareParcel a null: read {{\"firstParcel\":{GOOD},\"parcelList\":[],\"parcelsByDock\":{{}}}}
Towed, no content: no key held at []
Decking, a key that is no number: mismatch at [byDeck, x]: Expected Int key but found String key instead.
Depot, a key that is no number: refused at [byNumber, x]
    bad key x
"
        )
    );
}

/// What the Rust read of `payload` as `Read` answers, in the words the Swift side prints: the
/// variant taken, by the name its `Debug` opens with, the messages of a refusal, or that no
/// variant took the payload.
fn rust_choice<Read>(payload: &str) -> String
where
    Read: DeserializeOwned + Debug,
{
    match serde_json::from_str::<Read>(payload) {
        Ok(held) => format!("{held:?}")
            .split(|written: char| !written.is_alphanumeric())
            .next()
            .unwrap_or_default()
            .to_owned(),
        Err(refused) => {
            let told = refused.to_string();
            if told.contains("did not match any variant") {
                return "mismatch at []: no untagged member matched".to_owned();
            }
            let mut lines = "refused at []".to_owned();
            for violation in told.split("; ") {
                write!(lines, "\n    {violation}").unwrap();
            }
            lines
        }
    }
}

/// An untagged enum takes the variant serde takes. A member's own bound and the bound of a brand
/// it holds take a variant out of the running, and a check and a model a variant holds take none
/// out: the variant is chosen, and the enum's read refuses the value.
#[test]
fn an_untagged_enum_takes_the_variant_serde_takes() {
    let sorted: [(&str, &str); 11] = [
        ("a slot within its bound", r#""abc""#),
        (
            "a slot past its bound, a brand within its own",
            r#""123456""#,
        ),
        ("a slot and a brand past their bounds", r#""abcdef""#),
        ("a member within its bound", r#"{"sortedCount":1}"#),
        ("a member past its bound", r#"{"sortedCount":0}"#),
        ("a model", r#"{"itemLabel":"A","parcelCode":"ab"}"#),
        (
            "a model that breaks a rule",
            r#"{"itemLabel":"","parcelCode":"ab"}"#,
        ),
        (
            "a list with a model that breaks a rule",
            r#"[{"itemLabel":"A","parcelCode":"ab"},{"itemLabel":"","parcelCode":"ab"}]"#,
        ),
        ("an empty list", "[]"),
        (
            "a map of models",
            r#"{"north":{"itemLabel":"A","parcelCode":"ab"}}"#,
        ),
        (
            "a map with a model that breaks a rule",
            r#"{"north":{"itemLabel":"","parcelCode":"ab"}}"#,
        ),
    ];
    let vetted: [(&str, &str); 2] = [
        ("a brand whose check holds", r#""UPPER""#),
        ("a brand whose check refuses", r#""lower""#),
    ];
    let mut expected = String::new();
    let mut lines = Vec::new();
    for (name, payload) in sorted {
        writeln!(expected, "{name}: {}", rust_choice::<Sorted>(payload)).unwrap();
        lines.push(format!(
            "chosen(\"{name}\", Sorted.self, #\"{payload}\"#) {{ variant($0) }}"
        ));
    }
    for (name, payload) in vetted {
        writeln!(expected, "{name}: {}", rust_choice::<Vetted>(payload)).unwrap();
        lines.push(format!(
            "chosen(\"{name}\", Vetted.self, #\"{payload}\"#) {{ variant($0) }}"
        ));
    }
    assert_eq!(
        expected,
        "\
a slot within its bound: Brief
a slot past its bound, a brand within its own: Coded
a slot and a brand past their bounds: Wordy
a member within its bound: Counted
a member past its bound: mismatch at []: no untagged member matched
a model: Whole
a model that breaks a rule: refused at []
    'itemLabel': too short: minimum length is 1, got 0
a list with a model that breaks a rule: refused at []
    '1.itemLabel': too short: minimum length is 1, got 0
an empty list: Listed
a map of models: Mapped
a map with a model that breaks a rule: refused at []
    'north.itemLabel': too short: minimum length is 1, got 0
a brand whose check holds: Signed
a brand whose check refuses: refused at []
    a signet is pressed in upper case
",
        "what the Rust read answers for each payload"
    );
    assert_eq!(Pin("12".to_owned()).validate(), Ok(()));
    assert!(Signet("s".to_owned()).validate().is_err());
    let Some(swift) = printed(declared(), &[JUDGING, VARIANTS], &lines.join("\n")) else {
        return;
    };
    assert_eq!(swift, expected);
}

/// The definitions both clients of the tracking service are compiled with.
fn client_module(client: String, replies: &[(&str, &str)], driver: &str) -> String {
    let listed = replies
        .iter()
        .map(|(name, body)| format!("  (\"{name}\", #\"{body}\"#),"))
        .collect::<Vec<_>>()
        .join("\n");
    [
        "import Foundation".to_owned(),
        parcel_swift::swift_definition(),
        tracking_error_swift::swift_definition(),
        tracking_client_service_fault_fields_swift::swift_definition(),
        tracking_client_service_fault_kind_swift::swift_definition(),
        TrackingClientServiceSchema::swift_http_client(),
        client,
        SHOWN_FUNCTIONS.to_owned(),
        format!("let replies: [(String, String)] = [\n{listed}\n]"),
        driver.to_owned(),
    ]
    .join("\n\n")
}

/// What `driver` prints for the two replies, or `None` where no runtime was reachable.
fn answered_by(client: String, driver: &str) -> Option<String> {
    let replies = [
        ("a parcel that keeps every rule", GOOD),
        (
            "a parcel that breaks two",
            r#"{"fragileItemCodes":["y"],"itemCodeList":["x"],"label":""}"#,
        ),
    ];
    let source = client_module(client, &replies, driver);
    let run = judged(
        "swift-read",
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

/// Holds what a client printed to one success and one fault of `kind` whose detail quotes
/// [`TWO_MESSAGES`], whatever Swift writes around them.
fn assert_one_fault(swift: &str, kind: &str) {
    let lines: Vec<&str> = swift.lines().collect();
    assert_eq!(lines.len(), 3, "got: {swift}");
    assert_eq!(lines[0], "a parcel that keeps every rule: the parcel A");
    assert_eq!(
        lines[1],
        format!("a parcel that breaks two: the fault {kind}")
    );
    assert!(lines[2].contains(TWO_MESSAGES), "got: {swift}");
}

/// The `http_rest` client answers the fault it answers for any body that does not read as the
/// reply: `undeserializable-payload`, with the read's messages in its detail.
#[test]
fn the_http_client_answers_its_fault_for_a_reply_that_breaks_a_rule() {
    assert_eq!(
        poll_once(TrackingBackEnd.track(&(), String::new()))
            .unwrap()
            .map(|parcel| parcel.label),
        Err(TrackingError::NotFound)
    );
    let Some(swift) = answered_by(String::new(), HTTP_DRIVER) else {
        return;
    };
    assert_one_fault(&swift, "undeserializable-payload");
}

/// The `ws_rpc` client answers the fault it answers for any reply that will not become the
/// declared success: `failed-validation`, with the read's messages in its detail.
#[cfg(slow_tests)]
#[test]
fn the_ws_client_answers_its_fault_for_a_reply_that_breaks_a_rule() {
    assert_eq!(
        poll_once(TrackingBackEnd.track(&(), "A".to_owned()))
            .unwrap()
            .map(|parcel| parcel.label),
        Ok("A".to_owned())
    );
    let Some(swift) = answered_by(TrackingClientServiceSchema::swift_ws_client(), WS_DRIVER) else {
        return;
    };
    assert_one_fault(&swift, "failed-validation");
}

/// The README shows the read these runs make, what it prints and what a client answers.
#[test]
fn the_readme_shows_what_these_runs_read() {
    let readme = include_str!("../../README.md");
    for shown in [
        "try JSONDecoder().decode(Manifest.self, from: payload)",
        "'firstParcel.label': too short: minimum length is 1, got 0",
        "'parcelList.1.fragileItemCodes.1': y is outside of itemCodeList",
        "'parcelsByDock.north.label': too short: minimum length is 1, got 0",
        "'spareParcel.fragileItemCodes.1': y is outside of itemCodeList",
        "init(swiftSchemaUnjudged decoder: Decoder) throws",
        "`undeserializable-payload`",
        "`failed-validation`",
    ] {
        assert!(
            readme.contains(shown),
            "the README no longer shows: {shown}"
        );
    }
}
