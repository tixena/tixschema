//! `CarillonService`'s message, read whole and refused in each case of [`CASES`], by the Dart and
//! Kotlin `ws_rpc` attachments and by the Dart, Kotlin and Swift `ws_rpc` clients handed it as a
//! reply. Each refusal answers `failed-validation` naming the field the Rust dispatcher names for
//! the same payload, and an attachment calls no handler for it. Each run resolves on the replies it
//! counts, with no clock.

#[cfg(any(feature = "dart", feature = "kotlin", feature = "swift"))]
use super::carillon_service::{CASES, STRUCK, frame, payload, rust_fields};
#[cfg(any(feature = "dart", feature = "kotlin", feature = "swift"))]
use core::fmt::Write as _;
#[cfg(any(feature = "dart", feature = "kotlin", feature = "swift"))]
use serde_json::{Value, json};

#[cfg(feature = "dart")]
use super::CarillonServiceSchema;
#[cfg(feature = "dart")]
use super::carillon_service::{
    carillon_beam_dart, carillon_bell_dart, carillon_chime_dart, carillon_error_dart,
    carillon_frame_dart, carillon_peal_dart, carillon_pitch_dart, carillon_swing_dart,
    carillon_turn_dart, carillon_yoke_dart,
};
#[cfg(feature = "dart")]
use super::run_dart_validate::analyzed;
#[cfg(feature = "dart")]
use super::runtime::ran;

/// The attachment handed each frame, and what it replied and how many strikes reached the
/// handler; then the client handed each payload as the reply, and what it answered.
#[cfg(feature = "dart")]
const DART_DRIVER: &str = "
List<String> withinScale(CarillonPitch value) =>
    value.value <= 12 ? const <String>[] : <String>['${value.value} is outside of the scale'];

Object? told(CarillonServiceStrikeResult answered) => switch (answered) {
  CarillonServiceStrikeResultOk() => 'read',
  CarillonServiceStrikeResultFault(:final fault) => <String, dynamic>{
    'kind': fault.kind.toJson(),
    'field': fault.field,
  },
  _ => 'declared',
};

void main() async {
  var struck = 0;
  final handlers = CarillonServiceHandlers<void>(
    strike: (ctx, req) async {
      struck += 1;
      return req;
    },
  );
  final attached = <Map<String, dynamic>>[];
  for (final frame in frames) {
    struck = 0;
    final inbound = StreamController<Map<String, dynamic>>();
    final replied = Completer<Map<String, dynamic>>();
    final detach = attachCarillonServiceWsDispatcher<void>(
      (inbound: inbound.stream, send: replied.complete),
      null,
      handlers,
      onFault: (fault) {},
    );
    inbound.add(jsonDecode(frame) as Map<String, dynamic>);
    final reply = await replied.future;
    detach();
    attached.add(<String, dynamic>{'reply': reply, 'struck': struck});
  }
  final read = <Object?>[];
  final struckWhole = CarillonChime.fromJson(jsonDecode(whole) as Map<String, dynamic>);
  for (final value in payloads) {
    final outbound = StreamController<dynamic>();
    final inbound = StreamController<dynamic>();
    outbound.stream.listen((text) {
      final id = (jsonDecode(text as String) as Map<String, dynamic>)['id'];
      inbound.add(jsonEncode(<String, dynamic>{
        'kind': 'reply',
        'id': id,
        'service': 'CarillonService',
        'ok': true,
        'value': jsonDecode(value),
      }));
    });
    final transport = CarillonServiceWsTransport(
      sink: outbound.sink,
      stream: inbound.stream,
      heartbeat: CarillonServiceWsHeartbeat.off(),
    );
    read.add(told(await CarillonServiceWsClient(transport).strike(struckWhole)));
    transport.close();
    await outbound.close();
    await inbound.close();
  }
  print(jsonEncode(<String, dynamic>{'attached': attached, 'read': read}));
}
";

/// The attachment handed each frame, then the client handed each payload as the reply.
#[cfg(feature = "kotlin")]
const KOTLIN_DRIVER: &str = r#"
fun withinScale(value: CarillonPitch): List<String> =
    if (value.value <= 12u) emptyList() else listOf("${value.value} is outside of the scale")

class ManualSocket(private val onSend: (String) -> Unit) : CarillonServiceWsSocket {
    override var onMessage: ((String) -> Unit)? = null
    override var onClose: (() -> Unit)? = null
    override fun send(text: String) = onSend(text)
    override fun close() { onClose?.invoke() }
    fun deliver(text: String) { onMessage?.invoke(text) }
}

class Replying(private val value: String) : CarillonServiceWsSocket {
    override var onMessage: ((String) -> Unit)? = null
    override var onClose: (() -> Unit)? = null
    override fun send(text: String) {
        val id = Json.parseToJsonElement(text).jsonObject.getValue("id").jsonPrimitive.content
        onMessage?.invoke("""{"kind":"reply","id":"$id","service":"CarillonService","ok":true,"value":$value}""")
    }
    override fun close() { onClose?.invoke() }
}

object Handlers : CarillonServiceHandlers {
    val struck = java.util.concurrent.atomic.AtomicInteger(0)
    override suspend fun strike(req: CarillonChime): CarillonServiceStrikeResult {
        struck.incrementAndGet()
        return CarillonServiceStrikeResult.Ok(req)
    }
}

fun told(answered: CarillonServiceStrikeResult): JsonElement = when (answered) {
    is CarillonServiceStrikeResult.Ok -> JsonPrimitive("read")
    is CarillonServiceStrikeResult.Fault -> buildJsonObject {
        put("kind", Json.encodeToJsonElement(serializer<CarillonServiceFaultKind>(), answered.fault.kind))
        put("field", answered.fault.field?.let { JsonPrimitive(it) } ?: JsonNull)
    }
    else -> JsonPrimitive("declared")
}

fun main() = runBlocking {
    val attached = mutableListOf<JsonElement>()
    for (frame in FRAMES) {
        Handlers.struck.set(0)
        val replied = CompletableDeferred<String>()
        val socket = ManualSocket { text -> replied.complete(text) }
        val transport = CarillonServiceWsTransport(socket, CarillonServiceWsOptions(heartbeat = null))
        attachCarillonServiceWsDispatcher(transport.frames, Handlers, onFault = {})
        socket.deliver(frame)
        val reply = Json.parseToJsonElement(replied.await())
        transport.close()
        attached.add(buildJsonObject {
            put("reply", reply)
            put("struck", Handlers.struck.get())
        })
    }
    val whole = Json.decodeFromString(serializer<CarillonChime>(), WHOLE)
    val read = mutableListOf<JsonElement>()
    for (value in PAYLOADS) {
        val transport = CarillonServiceWsTransport(Replying(value), CarillonServiceWsOptions(heartbeat = null))
        read.add(told(CarillonServiceWsClient(transport).strike(whole)))
        transport.close()
    }
    println(buildJsonObject {
        put("attached", JsonArray(attached))
        put("read", JsonArray(read))
    }.toString())
}
"#;

/// The client handed each payload as the reply.
#[cfg(feature = "swift")]
const SWIFT_DRIVER: &str = r#"
func withinScale(_ value: CarillonPitch) -> [String] {
  value.value <= 12 ? [] : ["\(value.value) is outside of the scale"]
}

final class Replying: CarillonServiceWsSocket, @unchecked Sendable {
  var onMessage: (@Sendable (String) -> Void)?
  var onClose: (@Sendable () -> Void)?
  let value: String

  init(_ value: String) {
    self.value = value
  }

  func send(_ text: String) {
    let frame = try! JSONSerialization.jsonObject(with: Data(text.utf8)) as! [String: Any]
    let id = frame["id"] as! String
    onMessage?("{\"kind\":\"reply\",\"id\":\"\(id)\",\"service\":\"CarillonService\",\"ok\":true,\"value\":\(value)}")
  }

  func close() {
    onClose?()
  }
}

func told(_ answered: Result<CarillonChime, CarillonServiceStrikeFailure>) -> String {
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
for value in payloads {
  let ws = CarillonServiceWsClient(socket: Replying(value), options: .init(heartbeat: nil))
  read.append(told(await ws.strike(whole)))
  await ws.close()
}
print("[" + read.joined(separator: ",") + "]")
"#;

/// Each payload of [`CASES`].
#[cfg(any(feature = "dart", feature = "kotlin", feature = "swift"))]
fn payloads() -> Vec<String> {
    CASES.into_iter().map(payload).collect()
}

/// What a surface must answer for each payload: the payload read, and of one the Rust dispatcher
/// refuses the fault's kind and the field it names.
#[cfg(any(feature = "dart", feature = "kotlin", feature = "swift"))]
fn read_as_rust() -> Vec<Value> {
    rust_fields()
        .into_iter()
        .map(|field| {
            if field.is_null() {
                json!("read")
            } else {
                json!({ "kind": "failed-validation", "field": field })
            }
        })
        .collect()
}

/// Holds what an attachment replied to each frame to [`read_as_rust`]: the message handed back
/// with one strike reached, or the fault's kind and field with none.
#[cfg(any(feature = "dart", feature = "kotlin"))]
fn assert_attached_as_rust(attached: &Value) {
    let whole: Value = serde_json::from_str(STRUCK).unwrap();
    let told: Vec<Value> = CASES
        .into_iter()
        .zip(attached.as_array().unwrap())
        .map(|(case, told)| {
            let reply = &told["reply"];
            let answer = if reply["ok"] == true && reply["value"] == whole {
                json!("read")
            } else {
                json!({
                    "kind": reply["error"]["fault"]["kind"],
                    "field": reply["error"]["fault"]["field"],
                })
            };
            json!({ "case": case.0, "answer": answer, "struck": told["struck"] })
        })
        .collect();
    let expected: Vec<Value> = CASES
        .into_iter()
        .zip(read_as_rust())
        .map(|(case, answer)| {
            let struck = u8::from(answer == "read");
            json!({ "case": case.0, "answer": answer, "struck": struck })
        })
        .collect();
    assert_eq!(told, expected, "{attached}");
}

/// Holds what a client answered to each payload to [`read_as_rust`].
#[cfg(any(feature = "dart", feature = "kotlin", feature = "swift"))]
fn assert_read_as_rust(read: &Value) {
    let named = |answers: &[Value]| -> Vec<Value> {
        CASES
            .into_iter()
            .zip(answers)
            .map(|(case, answer)| json!({ "case": case.0, "answer": answer }))
            .collect()
    };
    assert_eq!(
        named(read.as_array().unwrap()),
        named(&read_as_rust()),
        "{read}"
    );
}

/// `items` as the elements of a list literal, each written by `literal`.
#[cfg(any(feature = "dart", feature = "kotlin", feature = "swift"))]
fn listed(items: &[String], literal: fn(&str) -> String) -> String {
    items.iter().fold(String::new(), |mut written, item| {
        let _ = writeln!(written, "  {},", literal(item));
        written
    })
}

/// The Dart module the attachment and the client run in.
#[cfg(feature = "dart")]
fn dart_source() -> String {
    let frames: Vec<String> = payloads().iter().map(|written| frame(written)).collect();
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
        carillon_chime_dart::dart_definition(),
        carillon_error_dart::dart_definition(),
        CarillonServiceSchema::dart_definition(),
        CarillonServiceSchema::dart_ws_client(),
        format!("final frames = <String>[\n{}];", listed(&frames, raw)),
        format!("final payloads = <String>[\n{}];", listed(&payloads(), raw)),
        format!("final whole = {};", raw(STRUCK)),
        DART_DRIVER.to_owned(),
    ]
    .join("\n\n")
}

#[cfg(feature = "dart")]
#[test]
fn the_dart_attachment_and_client_name_the_field_the_rust_dispatcher_names() {
    let Some(wrote) = ran(
        "dart",
        "TIXSCHEMA_DART",
        "dart",
        "carillon.dart",
        &dart_source(),
    ) else {
        return;
    };
    let told: Value = serde_json::from_str(wrote.trim()).unwrap();
    assert_attached_as_rust(&told["attached"]);
    assert_read_as_rust(&told["read"]);
}

#[cfg(feature = "dart")]
#[test]
fn the_dart_text_of_the_carillon_service_passes_the_analyzer() {
    analyzed("carillon-analyze", &dart_source());
}

#[cfg(feature = "kotlin")]
#[test]
fn the_kotlin_attachment_and_client_name_the_field_the_rust_dispatcher_names() {
    use super::carillon_service::carillon_service_schema::{
        carillon_service_fault_fields_kotlin, carillon_service_fault_kind_kotlin,
    };
    use super::carillon_service::{
        CarillonServiceSchema, carillon_beam_kotlin, carillon_bell_kotlin, carillon_chime_kotlin,
        carillon_error_kotlin, carillon_frame_kotlin, carillon_peal_kotlin, carillon_pitch_kotlin,
        carillon_swing_kotlin, carillon_turn_kotlin, carillon_yoke_kotlin,
    };
    use super::runtime::ran_kotlin;

    let frames: Vec<String> = payloads().iter().map(|written| frame(written)).collect();
    let raw = |text: &str| format!("\"\"\"{text}\"\"\"");
    let source = [
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
        carillon_chime_kotlin::kotlin_definition(),
        carillon_error_kotlin::kotlin_definition(),
        carillon_service_fault_fields_kotlin::kotlin_definition(),
        carillon_service_fault_kind_kotlin::kotlin_definition(),
        CarillonServiceSchema::kotlin_http_client(),
        CarillonServiceSchema::kotlin_ws_client(),
        format!("val FRAMES = listOf(\n{})", listed(&frames, raw)),
        format!("val PAYLOADS = listOf(\n{})", listed(&payloads(), raw)),
        format!("val WHOLE = {}", raw(STRUCK)),
        KOTLIN_DRIVER.to_owned(),
    ]
    .join("\n\n");
    let Some(wrote) = ran_kotlin(&source) else {
        return;
    };
    let told: Value = serde_json::from_str(wrote.trim()).unwrap();
    assert_attached_as_rust(&told["attached"]);
    assert_read_as_rust(&told["read"]);
}

#[cfg(feature = "swift")]
#[test]
fn the_swift_client_names_the_field_the_rust_dispatcher_names() {
    use super::carillon_service::carillon_service_schema::{
        carillon_service_fault_fields_swift, carillon_service_fault_kind_swift,
    };
    use super::carillon_service::{
        CarillonServiceSchema, carillon_beam_swift, carillon_bell_swift, carillon_chime_swift,
        carillon_error_swift, carillon_frame_swift, carillon_peal_swift, carillon_pitch_swift,
        carillon_swing_swift, carillon_turn_swift, carillon_yoke_swift,
    };
    use super::runtime::judged;

    let raw = |text: &str| format!("#\"{text}\"#");
    let source = [
        "import Foundation".to_owned(),
        carillon_bell_swift::swift_definition(),
        carillon_peal_swift::swift_definition(),
        carillon_pitch_swift::swift_definition(),
        carillon_swing_swift::swift_definition(),
        carillon_turn_swift::swift_definition(),
        carillon_beam_swift::swift_definition(),
        carillon_frame_swift::swift_definition(),
        carillon_yoke_swift::swift_definition(),
        carillon_chime_swift::swift_definition(),
        carillon_error_swift::swift_definition(),
        carillon_service_fault_fields_swift::swift_definition(),
        carillon_service_fault_kind_swift::swift_definition(),
        CarillonServiceSchema::swift_http_client(),
        CarillonServiceSchema::swift_ws_client(),
        format!("let payloads: [String] = [\n{}]", listed(&payloads(), raw)),
        format!("let wholeText = {}", raw(STRUCK)),
        SWIFT_DRIVER.to_owned(),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-carillon",
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
    assert_read_as_rust(&serde_json::from_str(run.stdout.trim()).unwrap());
}
