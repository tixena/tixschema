//! `CarillonMcpService` called by the TypeScript, Dart and Kotlin `mcp` servers with each case of
//! [`MCP_CASES`] as the arguments of `strike`, and the TypeScript, Dart, Kotlin and Swift `mcp`
//! clients answered each as the success of the call and each case of [`TOLL_CASES`] as the answer
//! of `toll`. Each server answers what the Rust dispatcher answers, writing the message read whole
//! as it writes it, and calls no handler for a call it refuses, and each client answers what the
//! Rust client answers. Nothing here waits on a clock.

use super::carillon_mcp_service::{
    MCP_CASES, TOLL_CASES, call, mcp_payload, over_mcp, reply, rust_client_answers,
    rust_server_answers, rust_toll_answers, told, toll_replies,
};
use super::carillon_service::STRUCK;
use core::fmt::Write as _;
use serde_json::{Value, json};

#[cfg(feature = "dart")]
use super::CarillonMcpServiceSchema;
#[cfg(feature = "dart")]
use super::carillon_service::{
    carillon_beam_dart, carillon_bell_dart, carillon_chime_dart, carillon_error_dart,
    carillon_frame_dart, carillon_outer_tone_dart, carillon_peal_dart, carillon_pitch_dart,
    carillon_swing_dart, carillon_tone_dart, carillon_turn_dart, carillon_yoke_dart,
};
#[cfg(feature = "dart")]
use super::run_dart_validate::analyzed;
#[cfg(feature = "dart")]
use super::runtime::ran;

/// The server handed each request, then the client answered each reply.
const NODE_DRIVER: &str = r#"
function withinScale(value, ctx) {
  if (value > 12) ctx.issues.push({ code: "custom", message: `${value} is outside of the scale`, input: value });
}

function audible(value, ctx) {
  if (value === "mute") ctx.issues.push({ code: "custom", message: "mute is silent", input: value });
}

function loud(value, ctx) {
  if (value === "soft") ctx.issues.push({ code: "custom", message: "soft is quiet", input: value });
}

async function main() {
  let struck = 0;
  const impl = {
    async strike(ctx, req) {
      struck += 1;
      return { ok: true, value: req };
    },
    async toll(ctx, req) {
      return { ok: true, value: [req, req.weight] };
    },
  };
  const served = [];
  const strikes = [];
  for (const text of REQUESTS) {
    struck = 0;
    const dispatch = createCarillonMcpServiceMcpDispatcher(impl);
    const answered = await dispatch(text, () => ({}), async () => {}, new CarillonMcpServiceMcpInFlight());
    served.push(answered === undefined ? null : answered);
    strikes.push(struck);
  }
  const read = [];
  for (const text of REPLIES) {
    const client = createCarillonMcpServiceMcpClient({ async send() { return text; } });
    const answered = await client.strike({ ...JSON.parse(WHOLE), depth: -9223372036854775808n });
    read.push(
      answered.ok
        ? "read"
        : "isServiceFault" in answered.error
          ? { kind: answered.error.fault.kind, field: answered.error.fault.field ?? null }
          : "declared",
    );
  }
  const tolled = [];
  for (const text of TOLLS) {
    const client = createCarillonMcpServiceMcpClient({ async send() { return text; } });
    const answered = await client.toll({ weight: 3 });
    tolled.push(
      answered.ok
        ? { rung: answered.value[1] }
        : "isServiceFault" in answered.error
          ? { kind: answered.error.fault.kind, field: answered.error.fault.field ?? null }
          : { crackedAt: answered.error[1] },
    );
  }
  console.log(JSON.stringify({ served, struck: strikes, read, tolled }));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// The server handed each request in a runtime without `JSON.rawJSON`.
const NODE_FALLBACK_DRIVER: &str = r#"
Reflect.deleteProperty(JSON, "rawJSON");

function withinScale(value, ctx) {
  if (value > 12) ctx.issues.push({ code: "custom", message: `${value} is outside of the scale`, input: value });
}

async function main() {
  const impl = {
    async strike(ctx, req) {
      return { ok: true, value: req };
    },
    async toll(ctx, req) {
      return { ok: true, value: [req, req.weight] };
    },
  };
  const served = [];
  for (const text of REQUESTS) {
    const dispatch = createCarillonMcpServiceMcpDispatcher(impl);
    served.push(await dispatch(text, () => ({}), async () => {}, new CarillonMcpServiceMcpInFlight()));
  }
  console.log(JSON.stringify(served));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// The server handed each request, then the client answered each reply.
#[cfg(feature = "dart")]
const DART_DRIVER: &str = "
List<String> withinScale(CarillonPitch value) =>
    value.value <= 12 ? const <String>[] : <String>['${value.value} is outside of the scale'];

List<String> audible(CarillonTone value) =>
    value.value == 'mute' ? const <String>['mute is silent'] : const <String>[];

List<String> loud(CarillonOuterTone value) =>
    value.value.value == 'soft' ? const <String>['soft is quiet'] : const <String>[];

class Canned implements CarillonMcpServiceMcpTransport {
  Canned(this.answer);
  final String answer;
  @override
  Future<String> send(({String text, String protocolVersion, String method, String? name}) message) async => answer;
}

Object? told(CarillonMcpServiceStrikeResult answered) => switch (answered) {
  CarillonMcpServiceStrikeResultOk() => 'read',
  CarillonMcpServiceStrikeResultFault(:final fault) => <String, dynamic>{
    'kind': fault.kind.toJson(),
    'field': fault.field,
  },
  _ => 'declared',
};

Object? tolled(CarillonMcpServiceTollResult answered) => switch (answered) {
  CarillonMcpServiceTollResultOk(:final value) => <String, dynamic>{'rung': value.$2},
  CarillonMcpServiceTollResultOperation(:final error) => <String, dynamic>{'crackedAt': error.$2},
  CarillonMcpServiceTollResultFault(:final fault) => <String, dynamic>{
    'kind': fault.kind.toJson(),
    'field': fault.field,
  },
};

void main() async {
  var struck = 0;
  final handlers = CarillonMcpServiceHandlers<void>(
    strike: (ctx, req) async {
      struck += 1;
      return req;
    },
    toll: (ctx, req) async => (req, req.weight),
  );
  final served = <Object?>[];
  final strikes = <int>[];
  for (final text in requests) {
    struck = 0;
    final dispatch = createCarillonMcpServiceMcpDispatcher<void>(handlers);
    served.add(await dispatch(text, (call) {}, (message) async {}, CarillonMcpServiceMcpInFlight()));
    strikes.add(struck);
  }
  final whole = CarillonChime.fromJson(jsonDecode(wholeText) as Map<String, dynamic>);
  final read = <Object?>[];
  for (final text in replies) {
    read.add(told(await CarillonMcpServiceMcpClient(Canned(text)).strike(whole)));
  }
  final rung = <Object?>[];
  for (final text in tolls) {
    rung.add(tolled(await CarillonMcpServiceMcpClient(Canned(text)).toll(CarillonBell(weight: 3))));
  }
  print(jsonEncode(<String, dynamic>{'served': served, 'struck': strikes, 'read': read, 'tolled': rung}));
}
";

/// The server handed each request, then the client answered each reply.
#[cfg(feature = "kotlin")]
const KOTLIN_DRIVER: &str = r#"
fun withinScale(value: CarillonPitch): List<String> =
    if (value.value <= 12u) emptyList() else listOf("${value.value} is outside of the scale")

fun audible(value: CarillonTone): List<String> =
    if (value.value == "mute") listOf("mute is silent") else emptyList()

fun loud(value: CarillonOuterTone): List<String> =
    if (value.value.value == "soft") listOf("soft is quiet") else emptyList()

object Handlers : CarillonMcpServiceHandlers {
    val struck = java.util.concurrent.atomic.AtomicInteger(0)
    override suspend fun strike(req: CarillonChime): CarillonMcpServiceStrikeResult {
        struck.incrementAndGet()
        return CarillonMcpServiceStrikeResult.Ok(req)
    }
    override suspend fun toll(req: CarillonBell): CarillonMcpServiceTollResult =
        CarillonMcpServiceTollResult.Ok(CarillonMcpServiceTollResultValue(req, req.weight))
}

class Canned(private val answer: String) : CarillonMcpServiceMcpTransport {
    override suspend fun send(message: CarillonMcpServiceMcpMessage): String = answer
}

fun told(answered: CarillonMcpServiceStrikeResult): JsonElement = when (answered) {
    is CarillonMcpServiceStrikeResult.Ok -> JsonPrimitive("read")
    is CarillonMcpServiceStrikeResult.Fault -> buildJsonObject {
        put("kind", Json.encodeToJsonElement(serializer<CarillonMcpServiceFaultKind>(), answered.fault.kind))
        put("field", answered.fault.field?.let { JsonPrimitive(it) } ?: JsonNull)
    }
    else -> JsonPrimitive("declared")
}

fun tolled(answered: CarillonMcpServiceTollResult): JsonElement = when (answered) {
    is CarillonMcpServiceTollResult.Ok -> buildJsonObject { put("rung", answered.value.headerOut0.toLong()) }
    is CarillonMcpServiceTollResult.Declared -> buildJsonObject { put("crackedAt", answered.error.errorHeaderOut0.toLong()) }
    is CarillonMcpServiceTollResult.Fault -> buildJsonObject {
        put("kind", Json.encodeToJsonElement(serializer<CarillonMcpServiceFaultKind>(), answered.fault.kind))
        put("field", answered.fault.field?.let { JsonPrimitive(it) } ?: JsonNull)
    }
}

fun main() = runBlocking {
    val nowhere: CarillonMcpServiceMcpWriter = { }
    val served = mutableListOf<JsonElement>()
    val strikes = mutableListOf<JsonElement>()
    for (text in REQUESTS) {
        Handlers.struck.set(0)
        val dispatch = createCarillonMcpServiceMcpDispatcher { _ -> Handlers }
        val answered = dispatch(text, nowhere, CarillonMcpServiceMcpInFlight())
        served.add(if (answered == null) JsonNull else JsonPrimitive(answered))
        strikes.add(JsonPrimitive(Handlers.struck.get()))
    }
    val whole = Json.decodeFromString(serializer<CarillonChime>(), WHOLE)
    val read = REPLIES.map { told(CarillonMcpServiceMcpClient(Canned(it)).strike(whole)) }
    val rung = TOLLS.map { tolled(CarillonMcpServiceMcpClient(Canned(it)).toll(CarillonBell(3u))) }
    println(buildJsonObject {
        put("served", JsonArray(served))
        put("struck", JsonArray(strikes))
        put("read", JsonArray(read))
        put("tolled", JsonArray(rung))
    }.toString())
}
"#;

/// The client answered each reply.
#[cfg(feature = "swift")]
const SWIFT_DRIVER: &str = r#"
func withinScale(_ value: CarillonPitch) -> [String] {
  value.value <= 12 ? [] : ["\(value.value) is outside of the scale"]
}

func audible(_ value: CarillonTone) -> [String] {
  value.value == "mute" ? ["mute is silent"] : []
}

func loud(_ value: CarillonOuterTone) -> [String] {
  value.value.value == "soft" ? ["soft is quiet"] : []
}

struct Canned: CarillonMcpServiceMcpTransport {
  let answer: String
  func send(_ message: CarillonMcpServiceMcpMessage) async throws -> String { answer }
}

func told(_ answered: Result<CarillonChime, CarillonMcpServiceStrikeFailure>) -> String {
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

func tolled(_ answered: Result<(CarillonBell, UInt32), CarillonMcpServiceTollFailure>) -> String {
  switch answered {
  case .success(let value):
    return "{\"rung\":\(value.1)}"
  case .failure(.declared(let declared)):
    return "{\"crackedAt\":\(declared.1)}"
  case .failure(.fault(let fault)):
    let field = fault.field.map { "\"\($0)\"" } ?? "null"
    return "{\"kind\":\"\(fault.kind.rawValue)\",\"field\":\(field)}"
  }
}

let whole = try! JSONDecoder().decode(CarillonChime.self, from: Data(wholeText.utf8))
var read: [String] = []
for text in replies {
  read.append(told(await CarillonMcpServiceMcpClient(transport: Canned(answer: text)).strike(whole)))
}
var rung: [String] = []
for text in tolls {
  rung.append(tolled(await CarillonMcpServiceMcpClient(transport: Canned(answer: text)).toll(CarillonBell(weight: 3))))
}
print("{\"read\":[" + read.joined(separator: ",") + "],\"tolled\":[" + rung.joined(separator: ",") + "]}")
"#;

/// Each request a server is handed, in the order of [`MCP_CASES`].
fn requests() -> Vec<String> {
    MCP_CASES
        .into_iter()
        .map(|name| call(1, &mcp_payload(name)))
        .collect()
}

/// Each answer a client is handed, in the order of [`MCP_CASES`].
fn replies() -> Vec<String> {
    MCP_CASES
        .into_iter()
        .map(|name| reply(&mcp_payload(name)))
        .collect()
}

/// `items` as the elements of a list literal, each written by `literal`.
fn listed(items: &[String], literal: fn(&str) -> String) -> String {
    items.iter().fold(String::new(), |mut written, item| {
        let _ = writeln!(written, "  {},", literal(item));
        written
    })
}

/// Holds what a server answered to each request, and how many strikes reached its handler, to
/// what the Rust dispatcher answers, and the text of its answer to the message read whole, whose
/// `i64` is beyond what a JavaScript `number` holds, to the text the Rust dispatcher writes.
fn assert_served_as_rust(report: &Value) {
    let whole = MCP_CASES
        .into_iter()
        .position(|name| name == "read whole")
        .unwrap();
    assert_eq!(
        report["served"][whole].as_str(),
        over_mcp(&call(1, &mcp_payload("read whole"))).0.as_deref(),
        "{report}"
    );
    let answered: Vec<Value> = MCP_CASES
        .into_iter()
        .zip(report["served"].as_array().unwrap())
        .zip(report["struck"].as_array().unwrap())
        .map(|((name, text), struck)| {
            let answer: Value = serde_json::from_str(text.as_str().unwrap()).unwrap();
            json!({ "case": name, "answer": told(&answer), "struck": struck })
        })
        .collect();
    let expected: Vec<Value> = MCP_CASES
        .into_iter()
        .zip(rust_server_answers())
        .map(|(name, answer)| {
            let struck = u8::from(answer == "read");
            json!({ "case": name, "answer": answer, "struck": struck })
        })
        .collect();
    assert_eq!(answered, expected, "{report}");
}

/// Each answer beside the case of `cases` it answers.
fn named(cases: &[&str], answers: &[Value]) -> Vec<Value> {
    cases
        .iter()
        .zip(answers)
        .map(|(name, answer)| json!({ "case": name, "answer": answer }))
        .collect()
}

/// Holds what a client answered to each reply to what the Rust client answers, and to each answer
/// of `toll` to what the Rust client answers it.
fn assert_read_as_rust(read: &Value, tolled: &Value) {
    assert_eq!(
        named(&MCP_CASES, read.as_array().unwrap()),
        named(&MCP_CASES, &rust_client_answers()),
        "{read}"
    );
    assert_eq!(
        named(&TOLL_CASES, tolled.as_array().unwrap()),
        named(&TOLL_CASES, &rust_toll_answers()),
        "{tolled}"
    );
}

/// The TypeScript module the server and the client run in.
fn node_bundle() -> String {
    use super::CarillonMcpServiceSchema;
    use super::carillon_service::{
        CarillonBeam, CarillonBell, CarillonChime, CarillonError, CarillonFrame, CarillonOuterTone,
        CarillonPeal, CarillonPitch, CarillonSwing, CarillonTone, CarillonTurn, CarillonYoke,
    };
    [
        "import { z, type ZodType, type $brand } from \"zod\";".to_owned(),
        CarillonBell::ts_definition(),
        CarillonBell::zod_schema(),
        CarillonPeal::ts_definition(),
        CarillonPeal::zod_schema(),
        CarillonPitch::ts_definition(),
        CarillonPitch::zod_schema(),
        CarillonSwing::ts_definition(),
        CarillonSwing::zod_schema(),
        CarillonTurn::ts_definition(),
        CarillonTurn::zod_schema(),
        CarillonBeam::ts_definition(),
        CarillonBeam::zod_schema(),
        CarillonFrame::ts_definition(),
        CarillonFrame::zod_schema(),
        CarillonYoke::ts_definition(),
        CarillonYoke::zod_schema(),
        CarillonTone::ts_definition(),
        CarillonTone::zod_schema(),
        CarillonOuterTone::ts_definition(),
        CarillonOuterTone::zod_schema(),
        CarillonChime::ts_definition(),
        CarillonChime::zod_schema(),
        CarillonError::ts_definition(),
        CarillonError::zod_schema(),
        CarillonMcpServiceSchema::ts_definition(),
        CarillonMcpServiceSchema::ts_service(),
        CarillonMcpServiceSchema::ts_mcp_service(),
        CarillonMcpServiceSchema::ts_mcp_client(),
    ]
    .join("\n\n")
}

#[test]
fn the_typescript_text_of_the_carillon_mcp_service_type_checks() {
    use super::runtime::{compiled_with_modules, node_modules, stand_down_modules};

    const REQUIRED_PACKAGES: &[&str] = &["zod"];
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the type-check of the carillon mcp service's text",
        );
        return;
    };
    let source = format!(
        "{}\n\nexport function withinScale(value: number, ctx: {{ issues: unknown[] }}): void {{\n  \
         void value;\n  void ctx;\n}}\n\nexport function audible(value: string, ctx: {{ issues: \
         unknown[] }}): void {{\n  void value;\n  void ctx;\n}}\n\nexport function loud(value: \
         string, ctx: {{ issues: unknown[] }}): void {{\n  void value;\n  void ctx;\n}}\n",
        node_bundle()
    );
    let Some((succeeded, said)) = compiled_with_modules(
        "carillon-mcp-types",
        "TIXSCHEMA_TSC",
        &[("bundle.ts", &source)],
        &modules,
    ) else {
        return;
    };
    assert!(succeeded, "the emitted text does not type-check:\n{said}");
}

#[test]
fn the_typescript_server_and_client_answer_what_the_rust_dispatcher_and_client_answer() {
    use super::runtime::{node_modules, ran_with_modules, stand_down_modules};

    const REQUIRED_PACKAGES: &[&str] = &["zod"];
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the emitted TypeScript carillon mcp service",
        );
        return;
    };
    let raw = |text: &str| format!("String.raw`{text}`");
    let source = [
        node_bundle(),
        format!("const REQUESTS = [\n{}];", listed(&requests(), raw)),
        format!("const REPLIES = [\n{}];", listed(&replies(), raw)),
        format!("const TOLLS = [\n{}];", listed(&toll_replies(), raw)),
        format!("const WHOLE = {};", raw(STRUCK)),
        NODE_DRIVER.to_owned(),
    ]
    .join("\n\n");
    let Some(wrote) = ran_with_modules(
        "carillon-mcp",
        "TIXSCHEMA_NODE",
        "node",
        "carillon_mcp.mts",
        &source,
        &modules,
    ) else {
        return;
    };
    let report: Value = serde_json::from_str(wrote.trim()).unwrap();
    assert_served_as_rust(&report);
    assert_read_as_rust(&report["read"], &report["tolled"]);
}

/// Without `JSON.rawJSON`, the TypeScript server writes an `i64` a `number` holds exactly as the
/// Rust dispatcher writes it, and answers one no `number` holds exactly as a handler that
/// panicked, as every TypeScript dispatcher answers it.
#[test]
fn without_raw_json_the_typescript_server_writes_an_i64_a_number_holds_and_refuses_any_other() {
    use super::runtime::{node_modules, ran_with_modules, stand_down_modules};

    const REQUIRED_PACKAGES: &[&str] = &["zod"];
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the emitted TypeScript carillon mcp service without JSON.rawJSON",
        );
        return;
    };
    let within = STRUCK.replace(r#""depth":-9223372036854775808"#, r#""depth":-5"#);
    let requests = [call(1, STRUCK), call(1, &within)];
    let raw = |text: &str| format!("String.raw`{text}`");
    let source = [
        node_bundle(),
        format!("const REQUESTS = [\n{}];", listed(&requests, raw)),
        NODE_FALLBACK_DRIVER.to_owned(),
    ]
    .join("\n\n");
    let Some(wrote) = ran_with_modules(
        "carillon-mcp-fallback",
        "TIXSCHEMA_NODE",
        "node",
        "carillon_mcp_fallback.mts",
        &source,
        &modules,
    ) else {
        return;
    };
    let served: Vec<String> = serde_json::from_str(wrote.trim()).unwrap();
    assert_eq!(Some(&served[1]), over_mcp(&requests[1]).0.as_ref());
    let detail = "-9223372036854775808 is written exactly only through JSON.rawJSON, which this \
                  runtime has not";
    assert_eq!(
        serde_json::from_str::<Value>(&served[0]).unwrap(),
        json!({
            "error": {
                "code": -32_603_i32,
                "data": {
                    "fault": { "detail": detail, "kind": "handler-panic", "operation": "strike" },
                    "isServiceFault": true,
                },
                "message": format!("strike: {detail}"),
            },
            "id": 1_u32,
            "jsonrpc": "2.0",
        })
    );
}

/// The Dart module the server and the client run in.
#[cfg(feature = "dart")]
fn dart_source() -> String {
    let raw = |text: &str| format!("r'''{text}'''");
    [
        "import 'dart:async';".to_owned(),
        "import 'dart:convert';".to_owned(),
        carillon_bell_dart::dart_definition(),
        carillon_peal_dart::dart_definition(),
        carillon_pitch_dart::dart_definition(),
        carillon_swing_dart::dart_definition(),
        carillon_turn_dart::dart_definition(),
        carillon_beam_dart::dart_definition(),
        carillon_frame_dart::dart_definition(),
        carillon_yoke_dart::dart_definition(),
        carillon_tone_dart::dart_definition(),
        carillon_outer_tone_dart::dart_definition(),
        carillon_chime_dart::dart_definition(),
        carillon_error_dart::dart_definition(),
        CarillonMcpServiceSchema::dart_definition(),
        CarillonMcpServiceSchema::dart_ws_client(),
        CarillonMcpServiceSchema::dart_mcp_service(),
        CarillonMcpServiceSchema::dart_mcp_client(),
        format!("final requests = <String>[\n{}];", listed(&requests(), raw)),
        format!("final replies = <String>[\n{}];", listed(&replies(), raw)),
        format!(
            "final tolls = <String>[\n{}];",
            listed(&toll_replies(), raw)
        ),
        format!("final wholeText = {};", raw(STRUCK)),
        DART_DRIVER.to_owned(),
    ]
    .join("\n\n")
}

#[cfg(feature = "dart")]
#[test]
fn the_dart_server_and_client_answer_what_the_rust_dispatcher_and_client_answer() {
    let Some(wrote) = ran(
        "dart",
        "TIXSCHEMA_DART",
        "dart",
        "carillon_mcp.dart",
        &dart_source(),
    ) else {
        return;
    };
    let report: Value = serde_json::from_str(wrote.trim()).unwrap();
    assert_served_as_rust(&report);
    assert_read_as_rust(&report["read"], &report["tolled"]);
}

#[cfg(feature = "dart")]
#[test]
fn the_dart_text_of_the_carillon_mcp_service_passes_the_analyzer() {
    analyzed("carillon-mcp-analyze", &dart_source());
}

/// The Kotlin file the server and the client run in.
#[cfg(feature = "kotlin")]
fn kotlin_source() -> String {
    use super::carillon_mcp_service::CarillonMcpServiceSchema;
    use super::carillon_mcp_service::carillon_mcp_service_schema::{
        carillon_mcp_service_fault_fields_kotlin, carillon_mcp_service_fault_kind_kotlin,
    };
    use super::carillon_service::{
        carillon_beam_kotlin, carillon_bell_kotlin, carillon_chime_kotlin, carillon_error_kotlin,
        carillon_frame_kotlin, carillon_outer_tone_kotlin, carillon_peal_kotlin,
        carillon_pitch_kotlin, carillon_swing_kotlin, carillon_tone_kotlin, carillon_turn_kotlin,
        carillon_yoke_kotlin,
    };

    let raw = |text: &str| format!("\"\"\"{text}\"\"\"");
    [
        "import kotlinx.coroutines.*\n\
         import kotlinx.coroutines.flow.*\n\
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
        carillon_beam_kotlin::kotlin_definition(),
        carillon_frame_kotlin::kotlin_definition(),
        carillon_yoke_kotlin::kotlin_definition(),
        carillon_tone_kotlin::kotlin_definition(),
        carillon_outer_tone_kotlin::kotlin_definition(),
        carillon_chime_kotlin::kotlin_definition(),
        carillon_error_kotlin::kotlin_definition(),
        carillon_mcp_service_fault_fields_kotlin::kotlin_definition(),
        carillon_mcp_service_fault_kind_kotlin::kotlin_definition(),
        CarillonMcpServiceSchema::kotlin_http_client(),
        CarillonMcpServiceSchema::kotlin_ws_client(),
        CarillonMcpServiceSchema::kotlin_mcp_service(),
        CarillonMcpServiceSchema::kotlin_mcp_client(),
        format!("val REQUESTS = listOf(\n{})", listed(&requests(), raw)),
        format!("val REPLIES = listOf(\n{})", listed(&replies(), raw)),
        format!("val TOLLS = listOf(\n{})", listed(&toll_replies(), raw)),
        format!("val WHOLE = {}", raw(STRUCK)),
        KOTLIN_DRIVER.to_owned(),
    ]
    .join("\n\n")
}

#[cfg(feature = "kotlin")]
#[test]
fn the_kotlin_server_and_client_answer_what_the_rust_dispatcher_and_client_answer() {
    let Some(wrote) = super::runtime::ran_kotlin(&kotlin_source()) else {
        return;
    };
    let report: Value = serde_json::from_str(wrote.trim()).unwrap();
    assert_served_as_rust(&report);
    assert_read_as_rust(&report["read"], &report["tolled"]);
}

/// The Swift program the client runs in.
#[cfg(feature = "swift")]
fn swift_source() -> String {
    use super::carillon_mcp_service::CarillonMcpServiceSchema;
    use super::carillon_mcp_service::carillon_mcp_service_schema::{
        carillon_mcp_service_fault_fields_swift, carillon_mcp_service_fault_kind_swift,
    };
    use super::carillon_service::{
        carillon_beam_swift, carillon_bell_swift, carillon_chime_swift, carillon_error_swift,
        carillon_frame_swift, carillon_outer_tone_swift, carillon_peal_swift, carillon_pitch_swift,
        carillon_swing_swift, carillon_tone_swift, carillon_turn_swift, carillon_yoke_swift,
    };

    let raw = |text: &str| format!("#\"{text}\"#");
    [
        "import Foundation".to_owned(),
        carillon_bell_swift::swift_definition(),
        carillon_peal_swift::swift_definition(),
        carillon_pitch_swift::swift_definition(),
        carillon_swing_swift::swift_definition(),
        carillon_turn_swift::swift_definition(),
        carillon_beam_swift::swift_definition(),
        carillon_frame_swift::swift_definition(),
        carillon_yoke_swift::swift_definition(),
        carillon_tone_swift::swift_definition(),
        carillon_outer_tone_swift::swift_definition(),
        carillon_chime_swift::swift_definition(),
        carillon_error_swift::swift_definition(),
        carillon_mcp_service_fault_fields_swift::swift_definition(),
        carillon_mcp_service_fault_kind_swift::swift_definition(),
        CarillonMcpServiceSchema::swift_http_client(),
        CarillonMcpServiceSchema::swift_mcp_client(),
        format!("let replies: [String] = [\n{}]", listed(&replies(), raw)),
        format!("let tolls: [String] = [\n{}]", listed(&toll_replies(), raw)),
        format!("let wholeText = {}", raw(STRUCK)),
        SWIFT_DRIVER.to_owned(),
    ]
    .join("\n\n")
}

#[cfg(feature = "swift")]
#[test]
fn the_swift_client_answers_what_the_rust_client_answers() {
    let source = swift_source();
    let Some(run) = super::runtime::judged(
        "swift-carillon-mcp",
        "TIXSCHEMA_SWIFT",
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
    let report: Value = serde_json::from_str(run.stdout.trim()).unwrap();
    assert_read_as_rust(&report["read"], &report["tolled"]);
}
