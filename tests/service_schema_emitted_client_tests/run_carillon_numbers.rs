//! The Dart, Kotlin and Swift `http_rest` clients of `CarillonHttpService` answered each case of
//! [`NUMBER_CASES`] as the body of the success, and the Dart and Kotlin `mcp` servers and the
//! Dart, Kotlin and Swift `mcp` clients of `CarillonMcpService` called and answered with each.
//! Each answers what its Rust twin answers: `-0` for an integer is refused, `-0` for an `f64` is
//! read as `-0.0`, and an `f32` read as an infinity is written as `null`. Nothing here waits on a
//! clock.

#![cfg(any(feature = "dart", feature = "kotlin", feature = "swift"))]

use super::carillon_numbers::{
    NUMBER_CASES, number_payloads, number_replies, rust_http_answers, rust_number_client_answers,
};
#[cfg(any(feature = "dart", feature = "kotlin"))]
use super::carillon_numbers::{number_requests, rust_number_server_answers, summed};
use super::carillon_service::STRUCK;
use core::fmt::Write as _;
use serde_json::{Value, json};

/// The checks the brands of the message name, in Dart.
#[cfg(feature = "dart")]
const DART_CHECKS: &str = "
List<String> withinScale(CarillonPitch value) =>
    value.value <= 12 ? const <String>[] : <String>['${value.value} is outside of the scale'];

List<String> audible(CarillonTone value) =>
    value.value == 'mute' ? const <String>['mute is silent'] : const <String>[];

List<String> loud(CarillonOuterTone value) =>
    value.value.value == 'soft' ? const <String>['soft is quiet'] : const <String>[];
";

/// The client answered each body.
#[cfg(feature = "dart")]
const DART_HTTP_DRIVER: &str = "
class Answering implements CarillonHttpServiceHttpTransport {
  Answering(this.body);
  final String body;
  @override
  Future<({int status, List<(String, String)> headers, List<int> body, Stream<List<int>> bodyStream})> send(
    ({String method, String path, String query, List<(String, String)> headers, List<int> body, List<(String, dynamic)> parts}) request,
  ) async =>
      (status: 200, headers: const <(String, String)>[], body: utf8.encode(body), bodyStream: const Stream<List<int>>.empty());
}

Object? told(CarillonHttpServiceStrikeResult answered) => switch (answered) {
  CarillonHttpServiceStrikeResultOk() => 'read',
  CarillonHttpServiceStrikeResultFault(:final fault) => <String, dynamic>{
    'kind': fault.kind.toJson(),
    'field': fault.field,
  },
  _ => 'declared',
};

void main() async {
  final whole = CarillonChime.fromJson(jsonDecode(wholeText) as Map<String, dynamic>);
  final read = <Object?>[];
  for (final body in bodies) {
    read.add(told(await CarillonHttpServiceHttpClient(Answering(body)).strike(whole)));
  }
  print(jsonEncode(read));
}
";

/// The checks the brands of the message name, in Kotlin.
#[cfg(feature = "kotlin")]
const KOTLIN_CHECKS: &str = r#"
fun withinScale(value: CarillonPitch): List<String> =
    if (value.value <= 12u) emptyList() else listOf("${value.value} is outside of the scale")

fun audible(value: CarillonTone): List<String> =
    if (value.value == "mute") listOf("mute is silent") else emptyList()

fun loud(value: CarillonOuterTone): List<String> =
    if (value.value.value == "soft") listOf("soft is quiet") else emptyList()
"#;

/// The client answered each body.
#[cfg(feature = "kotlin")]
const KOTLIN_HTTP_DRIVER: &str = r#"
class Answering(private val body: String) : CarillonHttpServiceHttpTransport {
    override suspend fun send(request: CarillonHttpServiceHttpRequest): CarillonHttpServiceHttpResponse =
        CarillonHttpServiceHttpResponse(200, emptyList(), body.encodeToByteArray())
}

fun told(answered: CarillonHttpServiceStrikeResult): JsonElement = when (answered) {
    is CarillonHttpServiceStrikeResult.Ok -> JsonPrimitive("read")
    is CarillonHttpServiceStrikeResult.Fault -> buildJsonObject {
        put("kind", Json.encodeToJsonElement(serializer<CarillonHttpServiceFaultKind>(), answered.fault.kind))
        put("field", answered.fault.field?.let { JsonPrimitive(it) } ?: JsonNull)
    }
    else -> JsonPrimitive("declared")
}

fun main() = runBlocking {
    val whole = Json.decodeFromString(serializer<CarillonChime>(), WHOLE)
    println(JsonArray(BODIES.map { told(CarillonHttpServiceHttpClient(Answering(it)).strike(whole)) }).toString())
}
"#;

/// The client answered each body.
#[cfg(feature = "swift")]
const SWIFT_HTTP_DRIVER: &str = r#"
func withinScale(_ value: CarillonPitch) -> [String] {
  value.value <= 12 ? [] : ["\(value.value) is outside of the scale"]
}

func audible(_ value: CarillonTone) -> [String] {
  value.value == "mute" ? ["mute is silent"] : []
}

func loud(_ value: CarillonOuterTone) -> [String] {
  value.value.value == "soft" ? ["soft is quiet"] : []
}

struct Answering: CarillonHttpServiceHttpTransport {
  let body: String
  func send(_ request: CarillonHttpServiceHttpRequest) async throws -> CarillonHttpServiceHttpResponse {
    CarillonHttpServiceHttpResponse(status: 200, headers: [], body: Data(body.utf8))
  }
}

func told(_ answered: Result<CarillonChime, CarillonHttpServiceStrikeFailure>) -> String {
  switch answered {
  case .success:
    return "\"read\""
  case .failure(.fault(let fault)):
    let field = fault.field.map { "\"\($0)\"" } ?? "null"
    return "{\"kind\":\"\(fault.kind.rawValue)\",\"field\":\(field)}"
  case .failure:
    return "\"declared\""
  }
}

let whole = try! JSONDecoder().decode(CarillonChime.self, from: Data(wholeText.utf8))
var read: [String] = []
for body in bodies {
  read.append(told(await CarillonHttpServiceHttpClient(transport: Answering(body: body)).strike(whole)))
}
print("[" + read.joined(separator: ",") + "]")
"#;

/// `items` as the elements of a list literal, each written by `literal`.
fn listed(items: &[String], literal: fn(&str) -> String) -> String {
    items.iter().fold(String::new(), |mut written, item| {
        let _ = writeln!(written, "  {},", literal(item));
        written
    })
}

/// Each case whose answer in `answers` is not the one in `rust`, beside both.
fn apart(answers: &[Value], rust: &[Value]) -> Vec<Value> {
    assert_eq!(answers.len(), rust.len(), "{answers:?}");
    NUMBER_CASES
        .into_iter()
        .zip(answers.iter().zip(rust))
        .filter(|(_, (answer, held))| answer != held)
        .map(|((name, _, _), (answer, held))| json!({ "case": name, "answer": answer, "rust": held }))
        .collect()
}

/// Holds what a client answered to each body or reply to what its Rust twin answered.
fn assert_read_as(read: &Value, rust: &[Value]) {
    let differing = apart(read.as_array().unwrap(), rust);
    assert!(differing.is_empty(), "{differing:#?}");
}

/// Holds what a server answered to each request, as [`summed`] tells it, and how many strikes
/// reached its handler, to what the Rust dispatcher answers.
#[cfg(any(feature = "dart", feature = "kotlin"))]
fn assert_served_as_rust(report: &Value) {
    let answered: Vec<Value> = report["served"]
        .as_array()
        .unwrap()
        .iter()
        .map(|text| summed(&serde_json::from_str(text.as_str().unwrap()).unwrap()))
        .collect();
    let rust = rust_number_server_answers();
    let differing = apart(&answered, &rust);
    assert!(differing.is_empty(), "{differing:#?}");
    let struck: Vec<Value> = rust
        .iter()
        .map(|answer| json!(u8::from(answer["structured"].is_string())))
        .collect();
    assert_eq!(report["struck"], json!(struck), "{report}");
}

/// The Dart module the `http_rest` client runs in.
#[cfg(feature = "dart")]
fn dart_http_source() -> String {
    use super::CarillonHttpServiceSchema;
    use super::carillon_service::{
        carillon_beam_dart, carillon_bell_dart, carillon_chime_dart, carillon_deep_dart,
        carillon_error_dart, carillon_frame_dart, carillon_outer_tone_dart, carillon_peal_dart,
        carillon_pitch_dart, carillon_swing_dart, carillon_tone_dart, carillon_turn_dart,
        carillon_yoke_dart,
    };

    let raw = |text: &str| format!("r'''{text}'''");
    [
        "import 'dart:async';".to_owned(),
        "import 'dart:convert';".to_owned(),
        carillon_bell_dart::dart_definition(),
        carillon_peal_dart::dart_definition(),
        carillon_pitch_dart::dart_definition(),
        carillon_swing_dart::dart_definition(),
        carillon_turn_dart::dart_definition(),
        carillon_deep_dart::dart_definition(),
        carillon_beam_dart::dart_definition(),
        carillon_frame_dart::dart_definition(),
        carillon_yoke_dart::dart_definition(),
        carillon_tone_dart::dart_definition(),
        carillon_outer_tone_dart::dart_definition(),
        carillon_chime_dart::dart_definition(),
        carillon_error_dart::dart_definition(),
        CarillonHttpServiceSchema::dart_definition(),
        CarillonHttpServiceSchema::dart_http_client(),
        format!(
            "final bodies = <String>[\n{}];",
            listed(&number_payloads(), raw)
        ),
        format!("final wholeText = {};", raw(STRUCK)),
        DART_CHECKS.to_owned(),
        DART_HTTP_DRIVER.to_owned(),
    ]
    .join("\n\n")
}

#[cfg(feature = "dart")]
#[test]
fn the_dart_http_client_reads_each_number_as_the_rust_client_reads_it() {
    let Some(wrote) = super::runtime::ran(
        "dart",
        "TIXSCHEMA_DART",
        "dart",
        "carillon_numbers_http.dart",
        &dart_http_source(),
    ) else {
        return;
    };
    assert_read_as(
        &serde_json::from_str(wrote.trim()).unwrap(),
        &rust_http_answers(),
    );
}

#[cfg(feature = "dart")]
#[test]
fn the_dart_text_of_the_carillon_http_client_passes_the_analyzer() {
    super::run_dart_validate::analyzed("carillon-numbers-http-analyze", &dart_http_source());
}

#[cfg(feature = "dart")]
#[test]
fn the_dart_server_and_client_read_and_write_each_number_as_the_rust_dispatcher_and_client_do() {
    let source = super::run_carillon_mcp::dart_source(&number_requests(), &number_replies());
    let Some(wrote) = super::runtime::ran(
        "dart",
        "TIXSCHEMA_DART",
        "dart",
        "carillon_numbers_mcp.dart",
        &source,
    ) else {
        return;
    };
    let report: Value = serde_json::from_str(wrote.trim()).unwrap();
    assert_served_as_rust(&report);
    assert_read_as(&report["read"], &rust_number_client_answers());
}

/// The Kotlin file the `http_rest` client runs in.
#[cfg(feature = "kotlin")]
fn kotlin_http_source() -> String {
    use super::CarillonHttpServiceSchema;
    use super::carillon_http_service_schema::{
        carillon_http_service_fault_fields_kotlin, carillon_http_service_fault_kind_kotlin,
    };
    use super::carillon_service::{
        carillon_beam_kotlin, carillon_bell_kotlin, carillon_chime_kotlin, carillon_deep_kotlin,
        carillon_error_kotlin, carillon_frame_kotlin, carillon_outer_tone_kotlin,
        carillon_peal_kotlin, carillon_pitch_kotlin, carillon_swing_kotlin, carillon_tone_kotlin,
        carillon_turn_kotlin, carillon_yoke_kotlin,
    };

    let raw = |text: &str| format!("\"\"\"{text}\"\"\"");
    [
        "import kotlinx.coroutines.*\n\
         import kotlinx.serialization.*\n\
         import kotlinx.serialization.json.*\n\
         import kotlinx.serialization.descriptors.*\n\
         import kotlinx.serialization.encoding.*\n\
         import kotlinx.serialization.builtins.*"
            .to_owned(),
        carillon_bell_kotlin::kotlin_definition(),
        carillon_peal_kotlin::kotlin_definition(),
        carillon_pitch_kotlin::kotlin_definition(),
        carillon_swing_kotlin::kotlin_definition(),
        carillon_turn_kotlin::kotlin_definition(),
        carillon_deep_kotlin::kotlin_definition(),
        carillon_beam_kotlin::kotlin_definition(),
        carillon_frame_kotlin::kotlin_definition(),
        carillon_yoke_kotlin::kotlin_definition(),
        carillon_tone_kotlin::kotlin_definition(),
        carillon_outer_tone_kotlin::kotlin_definition(),
        carillon_chime_kotlin::kotlin_definition(),
        carillon_error_kotlin::kotlin_definition(),
        carillon_http_service_fault_fields_kotlin::kotlin_definition(),
        carillon_http_service_fault_kind_kotlin::kotlin_definition(),
        CarillonHttpServiceSchema::kotlin_http_client(),
        format!("val BODIES = listOf(\n{})", listed(&number_payloads(), raw)),
        format!("val WHOLE = {}", raw(STRUCK)),
        KOTLIN_CHECKS.to_owned(),
        KOTLIN_HTTP_DRIVER.to_owned(),
    ]
    .join("\n\n")
}

#[cfg(feature = "kotlin")]
#[test]
fn the_kotlin_http_client_reads_each_number_as_the_rust_client_reads_it() {
    let Some(wrote) = super::runtime::ran_kotlin(&kotlin_http_source()) else {
        return;
    };
    assert_read_as(
        &serde_json::from_str(wrote.trim()).unwrap(),
        &rust_http_answers(),
    );
}

#[cfg(feature = "kotlin")]
#[test]
fn the_kotlin_server_and_client_read_and_write_each_number_as_the_rust_dispatcher_and_client_do() {
    let source = super::run_carillon_mcp::kotlin_source(&number_requests(), &number_replies());
    let Some(wrote) = super::runtime::ran_kotlin(&source) else {
        return;
    };
    let report: Value = serde_json::from_str(wrote.trim()).unwrap();
    assert_served_as_rust(&report);
    assert_read_as(&report["read"], &rust_number_client_answers());
}

/// The Swift program the `http_rest` client runs in.
#[cfg(feature = "swift")]
fn swift_http_source() -> String {
    use super::CarillonHttpServiceSchema;
    use super::carillon_http_service_schema::{
        carillon_http_service_fault_fields_swift, carillon_http_service_fault_kind_swift,
    };
    use super::carillon_service::{
        carillon_beam_swift, carillon_bell_swift, carillon_chime_swift, carillon_deep_swift,
        carillon_error_swift, carillon_frame_swift, carillon_outer_tone_swift, carillon_peal_swift,
        carillon_pitch_swift, carillon_swing_swift, carillon_tone_swift, carillon_turn_swift,
        carillon_yoke_swift,
    };

    let raw = |text: &str| format!("#\"{text}\"#");
    [
        "import Foundation".to_owned(),
        carillon_bell_swift::swift_definition(),
        carillon_peal_swift::swift_definition(),
        carillon_pitch_swift::swift_definition(),
        carillon_swing_swift::swift_definition(),
        carillon_turn_swift::swift_definition(),
        carillon_deep_swift::swift_definition(),
        carillon_beam_swift::swift_definition(),
        carillon_frame_swift::swift_definition(),
        carillon_yoke_swift::swift_definition(),
        carillon_tone_swift::swift_definition(),
        carillon_outer_tone_swift::swift_definition(),
        carillon_chime_swift::swift_definition(),
        carillon_error_swift::swift_definition(),
        carillon_http_service_fault_fields_swift::swift_definition(),
        carillon_http_service_fault_kind_swift::swift_definition(),
        CarillonHttpServiceSchema::swift_http_client(),
        format!(
            "let bodies: [String] = [\n{}]",
            listed(&number_payloads(), raw)
        ),
        format!("let wholeText = {}", raw(STRUCK)),
        SWIFT_HTTP_DRIVER.to_owned(),
    ]
    .join("\n\n")
}

/// Runs `source` under the Swift toolchain with warnings as errors, and answers what it printed.
#[cfg(feature = "swift")]
fn swift_ran(named: &str, source: &str) -> Option<Value> {
    let run = super::runtime::judged(
        named,
        "TIXSCHEMA_SWIFT",
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        source,
    )?;
    assert!(
        run.succeeded,
        "`swift -warnings-as-errors main.swift` failed.\n--- stdout ---\n{}\n--- stderr ---\n{}\n--- source ---\n{source}",
        run.stdout, run.stderr
    );
    Some(serde_json::from_str(run.stdout.trim()).unwrap())
}

#[cfg(feature = "swift")]
#[test]
fn the_swift_http_client_reads_each_number_as_the_rust_client_reads_it() {
    let Some(read) = swift_ran("swift-carillon-numbers-http", &swift_http_source()) else {
        return;
    };
    assert_read_as(&read, &rust_http_answers());
}

#[cfg(feature = "swift")]
#[test]
fn the_swift_mcp_client_reads_each_number_as_the_rust_client_reads_it() {
    let source = super::run_carillon_mcp::swift_source(&number_replies());
    let Some(report) = swift_ran("swift-carillon-numbers-mcp", &source) else {
        return;
    };
    assert_read_as(&report["read"], &rust_number_client_answers());
}
