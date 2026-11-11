//! `CarillonService`'s message, read whole and refused in each case of [`CASES`], by the
//! TypeScript, Dart and Kotlin `ws_rpc` attachments and by the TypeScript, Dart, Kotlin and Swift
//! `ws_rpc` clients handed it as a reply. Each answers what the Rust dispatcher answers for the
//! same payload, but for the cases the TypeScript table of its own records: the message read, a
//! frame dropped, or the fault's kind and the field it names, and an attachment calls no handler
//! for a frame it refuses or drops and replies to a message it reads as the Rust dispatcher does.
//! Each client then refuses to send an `f32` read as an infinity, and the Dart and Kotlin
//! transports, handed a reply naming a request whose frame they did not write, hold no call for it
//! and hand the reply to `frames`. Each run resolves on the replies it counts, with no clock.

use super::carillon_service::{CASES, STRUCK, frame, over_ws, payload, rust_answers};
use core::fmt::Write as _;
use serde_json::{Value, json};

#[cfg(feature = "dart")]
use super::CarillonServiceSchema;
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

/// The attachment handed each frame through a transport, and what it replied and how many strikes
/// reached the handler; then the client handed each payload as the reply; then the client handed an
/// `f32` it reads as an infinity to send, over a socket that counts what it is sent, and then a
/// reply naming the id of that unsent request. A frame the transport hands no attachment is told by
/// the one sent after it, which names another service, reaching the attachments first; a reply the
/// client drops, and a message it sends, by the declared error sent after it settling the call; a
/// reply the transport holds a request for, by the frame sent after it reaching `frames` first.
#[cfg(feature = "dart")]
const DART_DRIVER: &str = r#"
List<String> withinScale(CarillonPitch value) =>
    value.value <= 12 ? const <String>[] : <String>['${value.value} is outside of the scale'];

List<String> audible(CarillonTone value) =>
    value.value == 'mute' ? const <String>['mute is silent'] : const <String>[];

List<String> loud(CarillonOuterTone value) =>
    value.value.value == 'soft' ? const <String>['soft is quiet'] : const <String>[];

Object? told(CarillonServiceStrikeResult answered) => switch (answered) {
  CarillonServiceStrikeResultOk() => 'read',
  CarillonServiceStrikeResultFault(:final fault) => <String, dynamic>{
    'kind': fault.kind.toJson(),
    'field': fault.field,
  },
  _ => 'dropped',
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
    final outbound = StreamController<dynamic>();
    final inbound = StreamController<dynamic>();
    final transport = CarillonServiceWsTransport(
      sink: outbound.sink,
      stream: inbound.stream,
      heartbeat: CarillonServiceWsHeartbeat.off(),
    );
    final detach = attachCarillonServiceWsDispatcher<void>(
      transport.frames,
      null,
      handlers,
      onFault: (fault) {},
    );
    final firstPassed = Completer<Object?>();
    final passing = transport.frames.inbound.listen((passed) {
      if (!firstPassed.isCompleted) firstPassed.complete(passed['id']);
    });
    final replied = Completer<Map<String, dynamic>>();
    outbound.stream.listen((text) {
      final sent = jsonDecode(text as String) as Map<String, dynamic>;
      if (sent['id'] == 'ID' && !replied.isCompleted) replied.complete(sent);
    });
    inbound.add(frame);
    inbound.add(elsewhere);
    final Object? reply = await firstPassed.future == 'ID' ? await replied.future : 'dropped';
    attached.add(<String, dynamic>{'reply': reply, 'struck': struck});
    await passing.cancel();
    detach();
    transport.close();
    await inbound.close();
    await outbound.close();
  }
  final read = <Object?>[];
  final struckWhole = CarillonChime.fromJson(jsonDecode(whole) as Map<String, dynamic>);
  for (final value in payloads) {
    final outbound = StreamController<dynamic>();
    final inbound = StreamController<dynamic>();
    outbound.stream.listen((text) {
      final id = jsonEncode((jsonDecode(text as String) as Map<String, dynamic>)['id']);
      inbound.add('{"kind":"reply","id":$id,"service":"CarillonService","ok":true,"value":$value}');
      inbound.add('{"kind":"reply","id":$id,"service":"CarillonService","ok":false,"error":{"errorCode":"cracked"}}');
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
  var sent = 0;
  final counted = StreamController<dynamic>(sync: true);
  final answering = StreamController<dynamic>();
  counted.stream.listen((text) {
    sent += 1;
    final id = jsonEncode((jsonDecode(text as String) as Map<String, dynamic>)['id']);
    answering.add('{"kind":"reply","id":$id,"service":"CarillonService","ok":false,"error":{"errorCode":"cracked"}}');
  });
  final refusing = CarillonServiceWsTransport(
    sink: counted.sink,
    stream: answering.stream,
    heartbeat: CarillonServiceWsHeartbeat.off(),
  );
  final infinite = CarillonChime.fromJson(jsonDecode(whole.replaceFirst('"ring":1.5', '"ring":1e39')) as Map<String, dynamic>);
  final unsent = await CarillonServiceWsClient(refusing).strike(infinite);
  final firstPassed = Completer<Object?>();
  final passing = refusing.frames.inbound.listen((passed) {
    if (!firstPassed.isCompleted) firstPassed.complete(passed['id']);
  });
  answering.add('{"kind":"reply","id":"1","service":"CarillonService","ok":false,"error":{"errorCode":"cracked"}}');
  answering.add(elsewhere);
  final passedFirst = await firstPassed.future;
  await passing.cancel();
  refusing.close();
  await counted.close();
  await answering.close();
  final refused = switch (unsent) {
    CarillonServiceStrikeResultFault(:final fault) => <String, dynamic>{
      'kind': fault.kind.toJson(),
      'detail': fault.detail,
      'sent': sent,
      'passedFirst': passedFirst,
    },
    _ => told(unsent),
  };
  print(jsonEncode(<String, dynamic>{'attached': attached, 'read': read, 'refused': refused}));
}
"#;

/// The attachment handed each frame, then the client handed each payload as the reply, then the
/// client handed an `f32` it reads as an infinity to send, each told apart as the Dart driver tells
/// them; then the client over a socket whose `send` throws, and a reply naming the id of the
/// request it did not write, told as the Dart driver tells the reply to its unsent request.
#[cfg(feature = "kotlin")]
const KOTLIN_DRIVER: &str = r#"
fun withinScale(value: CarillonPitch): List<String> =
    if (value.value <= 12u) emptyList() else listOf("${value.value} is outside of the scale")

fun audible(value: CarillonTone): List<String> =
    if (value.value == "mute") listOf("mute is silent") else emptyList()

fun loud(value: CarillonOuterTone): List<String> =
    if (value.value.value == "soft") listOf("soft is quiet") else emptyList()

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
        onMessage?.invoke("""{"kind":"reply","id":"$id","service":"CarillonService","ok":false,"error":{"errorCode":"cracked"}}""")
    }
    override fun close() { onClose?.invoke() }
}

class Counted : CarillonServiceWsSocket {
    override var onMessage: ((String) -> Unit)? = null
    override var onClose: (() -> Unit)? = null
    var sent = 0
    override fun send(text: String) {
        sent += 1
        val id = Json.parseToJsonElement(text).jsonObject.getValue("id").jsonPrimitive.content
        onMessage?.invoke("""{"kind":"reply","id":"$id","service":"CarillonService","ok":false,"error":{"errorCode":"cracked"}}""")
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
    else -> JsonPrimitive("dropped")
}

fun idOf(text: String): String? = (Json.parseToJsonElement(text).jsonObject["id"] as? JsonPrimitive)?.contentOrNull

fun main() = runBlocking {
    val attached = mutableListOf<JsonElement>()
    for (frame in FRAMES) {
        Handlers.struck.set(0)
        val replied = CompletableDeferred<String>()
        val socket = ManualSocket { text -> if (idOf(text) == "ID") replied.complete(text) }
        val transport = CarillonServiceWsTransport(socket, CarillonServiceWsOptions(heartbeat = null))
        val passed = Channel<String>(Channel.UNLIMITED)
        val passing = launch(start = CoroutineStart.UNDISPATCHED) { transport.frames.inbound.collect { passed.send(it) } }
        attachCarillonServiceWsDispatcher(transport.frames, Handlers, onFault = {})
        socket.deliver(frame)
        socket.deliver(ELSEWHERE)
        val reply = if (idOf(passed.receive()) == "ID") Json.parseToJsonElement(replied.await()) else JsonPrimitive("dropped")
        passing.cancel()
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
    val counted = Counted()
    val refusing = CarillonServiceWsTransport(counted, CarillonServiceWsOptions(heartbeat = null))
    val infinite = Json.decodeFromString(serializer<CarillonChime>(), WHOLE.replace("\"ring\":1.5", "\"ring\":1e39"))
    val unsent = CarillonServiceWsClient(refusing).strike(infinite)
    refusing.close()
    val refused = if (unsent is CarillonServiceStrikeResult.Fault) buildJsonObject {
        put("kind", Json.encodeToJsonElement(serializer<CarillonServiceFaultKind>(), unsent.fault.kind))
        put("detail", unsent.fault.detail)
        put("sent", counted.sent)
    } else told(unsent)
    val throwing = ManualSocket { throw IllegalStateException("the socket is closed") }
    val unwriting = CarillonServiceWsTransport(throwing, CarillonServiceWsOptions(heartbeat = null))
    val unwritten = CarillonServiceWsClient(unwriting).strike(whole)
    val passedOn = Channel<String>(Channel.UNLIMITED)
    val passingOn = launch(start = CoroutineStart.UNDISPATCHED) { unwriting.frames.inbound.collect { passedOn.send(it) } }
    throwing.deliver("""{"kind":"reply","id":"1","service":"CarillonService","ok":false,"error":{"errorCode":"cracked"}}""")
    throwing.deliver(ELSEWHERE)
    val passedFirst = idOf(passedOn.receive())
    passingOn.cancel()
    unwriting.close()
    val thrown = if (unwritten is CarillonServiceStrikeResult.Fault) buildJsonObject {
        put("kind", Json.encodeToJsonElement(serializer<CarillonServiceFaultKind>(), unwritten.fault.kind))
        put("detail", unwritten.fault.detail)
        put("passedFirst", passedFirst)
    } else told(unwritten)
    println(buildJsonObject {
        put("attached", JsonArray(attached))
        put("read", JsonArray(read))
        put("refused", refused)
        put("thrown", thrown)
    }.toString())
}
"#;

/// The client handed each payload as the reply, a reply it drops told by the declared error sent
/// after it settling the call.
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
    onMessage?("{\"kind\":\"reply\",\"id\":\"\(id)\",\"service\":\"CarillonService\",\"ok\":false,\"error\":{\"errorCode\":\"cracked\"}}")
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
    return "\"dropped\""
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

/// The TypeScript attachment handed each frame, then the client handed each payload as the reply,
/// then the client handed an `f32` it reads as an infinity to send. A frame the attachment drops is
/// told by the one sent after it, whose label the handler counts no strike for, answering first; a
/// reply the client drops, by the declared error sent after it.
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
      if (req.label !== "next") struck += 1;
      return { ok: true, value: req };
    },
  };
  const attached = [];
  for (const frame of FRAMES) {
    struck = 0;
    let listener;
    let answer;
    let reply = "dropped";
    const answered = new Promise((resolve) => { answer = resolve; });
    const socket = {
      send(text) {
        const sent = JSON.parse(text);
        if (sent.id === "ID") reply = text;
        if (sent.id === "NEXT") answer();
      },
      close() {},
      addEventListener(type, added) { if (type === "message") listener = added; },
      removeEventListener() {},
    };
    const detach = attachCarillonServiceWsDispatcher(socket, {}, impl, () => {});
    listener({ data: frame });
    listener({ data: NEXT });
    await answered;
    detach();
    attached.push({ reply, struck });
  }
  const read = [];
  for (const value of PAYLOADS) {
    let listener;
    const socket = {
      send(text) {
        const { id } = JSON.parse(text);
        queueMicrotask(() => {
          listener({ data: `{"kind":"reply","id":${JSON.stringify(id)},"service":"CarillonService","ok":true,"value":${value}}` });
          listener({ data: `{"kind":"reply","id":${JSON.stringify(id)},"service":"CarillonService","ok":false,"error":{"errorCode":"cracked"}}` });
        });
      },
      close() {},
      addEventListener(type, added) { if (type === "message") listener = added; },
      removeEventListener() {},
    };
    const client = createCarillonServiceClient(createCarillonServiceWsTransport(socket, { heartbeat: false }));
    const answered = await client.strike({ ...JSON.parse(WHOLE), depth: -9223372036854775808n });
    read.push(
      answered.ok
        ? "read"
        : answered.error.isServiceFault
          ? { kind: answered.error.fault.kind, field: answered.error.fault.field ?? null }
          : "dropped",
    );
  }
  let sent = 0;
  const counting = { send() { sent += 1; }, close() {}, addEventListener() {}, removeEventListener() {} };
  const refusing = createCarillonServiceClient(createCarillonServiceWsTransport(counting, { heartbeat: false }));
  const unsent = await refusing.strike({ ...JSON.parse(WHOLE), depth: -9223372036854775808n, ring: 1e39 });
  const refused = unsent.ok
    ? "read"
    : { kind: unsent.error.fault.kind, field: unsent.error.fault.field, detail: unsent.error.fault.detail, sent };
  console.log(JSON.stringify({ attached, read, refused }));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// The cases the TypeScript attachment and client answer apart from the Rust dispatcher, and the
/// field each names: Zod names a member of a variant, an arm of a `Result` holding a text, and a
/// flattened member at the union, the `Result` and the member that holds the value; and reads a
/// number written with a fraction or an exponent, or `-0`, for an integer, which `JSON.parse`
/// reads as the integer.
const NODE_APART: [(&str, Option<&str>); 15] = [
    ("a boolean for a number in a variant", Some("peal")),
    (
        "a text for a number in a flattened member",
        Some("frame.oak"),
    ),
    ("a text in an arm of a Result", Some("tally")),
    ("an integral fraction for a u32", None),
    ("an exponent for a u32", None),
    ("a capital exponent for a u32", None),
    ("a fraction of zero for a u32", None),
    ("a fraction of zero below zero for a u32", None),
    ("a zero below zero for a u32", None),
    ("an integral fraction for an i64", None),
    ("an exponent for an i64", None),
    ("a capital exponent for an i64", None),
    ("a fraction of zero for an i64", None),
    ("a fraction of zero below zero for an i64", None),
    ("a zero below zero for an i64", None),
];

/// Each payload of [`CASES`].
fn payloads() -> Vec<String> {
    CASES.into_iter().map(payload).collect()
}

/// What a surface must answer for each payload: what the Rust dispatcher answers, but for the
/// cases `apart` names, which the surface reads where it holds `None` and refuses naming the field
/// it holds otherwise.
fn read_as_rust(apart: &[(&str, Option<&str>)]) -> Vec<Value> {
    CASES
        .into_iter()
        .zip(rust_answers())
        .map(|(case, answer)| {
            let held = apart
                .iter()
                .find_map(|(named, held)| (*named == case.0).then_some(*held));
            match held {
                Some(None) => json!("read"),
                Some(Some(named)) => json!({ "kind": "failed-validation", "field": named }),
                None => answer,
            }
        })
        .collect()
}

/// The frame sent after each one an attachment is handed: it names another service, so the
/// transport hands it on and no attachment of `CarillonService` answers it.
#[cfg(any(feature = "dart", feature = "kotlin"))]
fn elsewhere() -> String {
    frame("NEXT", STRUCK).replace(r#""service":"CarillonService""#, r#""service":"Elsewhere""#)
}

/// Holds what an attachment replied to each frame to [`read_as_rust`]: the message handed to the
/// handler, the frame dropped, or the fault's kind and field with no strike reached; and the reply
/// to each message the Rust dispatcher reads to the one it writes, an `f32` read as an infinity
/// written as `null`, as serde writes it.
fn assert_attached_as_rust(attached: &Value, apart: &[(&str, Option<&str>)]) {
    let is_apart = |named: &str| apart.iter().any(|(case, _)| *case == named);
    let told: Vec<Value> = CASES
        .into_iter()
        .zip(attached.as_array().unwrap())
        .map(|(case, told)| {
            let reply = &told["reply"];
            let answer = if reply == "dropped" {
                json!("dropped")
            } else if told["struck"] == 1_i32 {
                json!("read")
            } else {
                json!({
                    "kind": reply["error"]["fault"]["kind"],
                    "field": reply["error"]["fault"]["field"],
                })
            };
            let value = if is_apart(case.0) {
                &Value::Null
            } else {
                &reply["value"]
            };
            json!({ "case": case.0, "answer": answer, "struck": told["struck"], "value": value })
        })
        .collect();
    let expected: Vec<Value> = CASES
        .into_iter()
        .zip(read_as_rust(apart))
        .map(|(case, answer)| {
            let struck = u8::from(answer == "read");
            let value = over_ws(&payload(case))
                .0
                .filter(|_| !is_apart(case.0))
                .map_or(Value::Null, |reply| reply["value"].clone());
            json!({ "case": case.0, "answer": answer, "struck": struck, "value": value })
        })
        .collect();
    assert_eq!(told, expected, "{attached}");
}

/// Holds what a client answered to each payload to [`read_as_rust`].
fn assert_read_as_rust(read: &Value, apart: &[(&str, Option<&str>)]) {
    let named = |answers: &[Value]| -> Vec<Value> {
        CASES
            .into_iter()
            .zip(answers)
            .map(|(case, answer)| json!({ "case": case.0, "answer": answer }))
            .collect()
    };
    assert_eq!(
        named(read.as_array().unwrap()),
        named(&read_as_rust(apart)),
        "{read}"
    );
}

/// `items` as the elements of a list literal, each written by `literal`.
fn listed(items: &[String], literal: fn(&str) -> String) -> String {
    items.iter().fold(String::new(), |mut written, item| {
        let _ = writeln!(written, "  {},", literal(item));
        written
    })
}

/// The Dart module the attachment and the client run in.
#[cfg(feature = "dart")]
fn dart_source() -> String {
    let frames: Vec<String> = payloads()
        .iter()
        .map(|written| frame("ID", written))
        .collect();
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
        CarillonServiceSchema::dart_definition(),
        CarillonServiceSchema::dart_ws_client(),
        format!("final frames = <String>[\n{}];", listed(&frames, raw)),
        format!("final payloads = <String>[\n{}];", listed(&payloads(), raw)),
        format!("final whole = {};", raw(STRUCK)),
        format!("final elsewhere = {};", raw(&elsewhere())),
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
    assert_attached_as_rust(&told["attached"], &[]);
    assert_read_as_rust(&told["read"], &[]);
    assert_eq!(
        told["refused"],
        json!({
            "kind": "transport-failure",
            "detail": "Converting object to an encodable object failed: Infinity",
            "sent": 0_i32,
            "passedFirst": "1",
        })
    );
}

#[cfg(feature = "dart")]
#[test]
fn the_dart_text_of_the_carillon_service_passes_the_analyzer() {
    analyzed("carillon-analyze", &dart_source());
}

/// The Kotlin file the attachment and the client run in.
#[cfg(feature = "kotlin")]
fn kotlin_source() -> String {
    use super::carillon_service::carillon_service_schema::{
        carillon_service_fault_fields_kotlin, carillon_service_fault_kind_kotlin,
    };
    use super::carillon_service::{
        CarillonServiceSchema, carillon_beam_kotlin, carillon_bell_kotlin, carillon_chime_kotlin,
        carillon_error_kotlin, carillon_frame_kotlin, carillon_outer_tone_kotlin,
        carillon_peal_kotlin, carillon_pitch_kotlin, carillon_swing_kotlin, carillon_tone_kotlin,
        carillon_turn_kotlin, carillon_yoke_kotlin,
    };

    let frames: Vec<String> = payloads()
        .iter()
        .map(|written| frame("ID", written))
        .collect();
    let raw = |text: &str| format!("\"\"\"{text}\"\"\"");
    [
        "import kotlinx.coroutines.*\n\
         import kotlinx.coroutines.channels.*\n\
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
        carillon_service_fault_fields_kotlin::kotlin_definition(),
        carillon_service_fault_kind_kotlin::kotlin_definition(),
        CarillonServiceSchema::kotlin_http_client(),
        CarillonServiceSchema::kotlin_ws_client(),
        format!("val FRAMES = listOf(\n{})", listed(&frames, raw)),
        format!("val PAYLOADS = listOf(\n{})", listed(&payloads(), raw)),
        format!("val WHOLE = {}", raw(STRUCK)),
        format!("val ELSEWHERE = {}", raw(&elsewhere())),
        KOTLIN_DRIVER.to_owned(),
    ]
    .join("\n\n")
}

#[cfg(feature = "kotlin")]
#[test]
fn the_kotlin_attachment_and_client_name_the_field_the_rust_dispatcher_names() {
    let Some(wrote) = super::runtime::ran_kotlin(&kotlin_source()) else {
        return;
    };
    let told: Value = serde_json::from_str(wrote.trim()).unwrap();
    assert_attached_as_rust(&told["attached"], &[]);
    assert_read_as_rust(&told["read"], &[]);
    let refused = &told["refused"];
    assert_eq!(
        (&refused["kind"], &refused["sent"]),
        (&json!("transport-failure"), &json!(0_i32)),
        "{refused}"
    );
    assert!(
        refused["detail"].as_str().is_some_and(|detail| {
            detail.starts_with(
            "kotlinx.serialization.json.JsonEncodingException: Unexpected special floating-point \
             value Infinity"
        )
        }),
        "{refused}"
    );
    assert_eq!(
        told["thrown"],
        json!({
            "kind": "transport-failure",
            "detail": "java.lang.IllegalStateException: the socket is closed",
            "passedFirst": "1",
        })
    );
}

/// The Swift program the client runs in.
#[cfg(feature = "swift")]
fn swift_source() -> String {
    use super::carillon_service::carillon_service_schema::{
        carillon_service_fault_fields_swift, carillon_service_fault_kind_swift,
    };
    use super::carillon_service::{
        CarillonServiceSchema, carillon_beam_swift, carillon_bell_swift, carillon_chime_swift,
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
        carillon_beam_swift::swift_definition(),
        carillon_frame_swift::swift_definition(),
        carillon_yoke_swift::swift_definition(),
        carillon_tone_swift::swift_definition(),
        carillon_outer_tone_swift::swift_definition(),
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
    .join("\n\n")
}

#[cfg(feature = "swift")]
#[test]
fn the_swift_client_names_the_field_the_rust_dispatcher_names() {
    let source = swift_source();
    let Some(run) = super::runtime::judged(
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
    assert_read_as_rust(&serde_json::from_str(run.stdout.trim()).unwrap(), &[]);
}

/// The TypeScript module the attachment and the client run in.
fn node_bundle() -> String {
    use super::carillon_service::{
        CarillonBeam, CarillonBell, CarillonChime, CarillonError, CarillonFrame, CarillonOuterTone,
        CarillonPeal, CarillonPitch, CarillonServiceSchema, CarillonSwing, CarillonTone,
        CarillonTurn, CarillonYoke,
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
        CarillonServiceSchema::ts_definition(),
        CarillonServiceSchema::ts_service(),
        CarillonServiceSchema::ts_client(),
        CarillonServiceSchema::ts_ws_client(),
        CarillonServiceSchema::ts_ws_service(),
    ]
    .join("\n\n")
}

#[test]
fn the_typescript_text_of_the_carillon_service_type_checks() {
    use super::runtime::{compiled_with_modules, node_modules, stand_down_modules};

    const REQUIRED_PACKAGES: &[&str] = &["zod"];
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the type-check of the carillon service's text",
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
        "carillon-types",
        "TIXSCHEMA_TSC",
        &[("bundle.ts", &source)],
        &modules,
    ) else {
        return;
    };
    assert!(succeeded, "the emitted text does not type-check:\n{said}");
}

#[test]
fn the_typescript_attachment_and_client_name_the_field_the_rust_dispatcher_names() {
    use super::runtime::{node_modules, ran_with_modules, stand_down_modules};

    const REQUIRED_PACKAGES: &[&str] = &["zod"];
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(REQUIRED_PACKAGES, "the emitted TypeScript carillon service");
        return;
    };
    let frames: Vec<String> = payloads()
        .iter()
        .map(|written| frame("ID", written))
        .collect();
    let next = frame(
        "NEXT",
        &STRUCK.replace(r#""label":"a""#, r#""label":"next""#),
    );
    let raw = |text: &str| format!("String.raw`{text}`");
    let source = [
        node_bundle(),
        format!("const FRAMES = [\n{}];", listed(&frames, raw)),
        format!("const PAYLOADS = [\n{}];", listed(&payloads(), raw)),
        format!("const WHOLE = {};", raw(STRUCK)),
        format!("const NEXT = {};", raw(&next)),
        NODE_DRIVER.to_owned(),
    ]
    .join("\n\n");
    let Some(wrote) = ran_with_modules(
        "carillon",
        "TIXSCHEMA_NODE",
        "node",
        "carillon.mts",
        &source,
        &modules,
    ) else {
        return;
    };
    let mut told: Value = serde_json::from_str(wrote.trim()).unwrap();
    // A reply is handed over as its text: `JSON.parse` holds no `i64` exactly.
    for answered in told["attached"].as_array_mut().unwrap() {
        if let Some(text) = answered["reply"].as_str().filter(|text| *text != "dropped") {
            answered["reply"] = serde_json::from_str(text).unwrap();
        }
    }
    assert_attached_as_rust(&told["attached"], &NODE_APART);
    assert_read_as_rust(&told["read"], &NODE_APART);
    assert_eq!(
        told["refused"],
        json!({
            "kind": "failed-validation",
            "field": "ring",
            "detail": "'ring': expected a 32-bit float, found Infinity",
            "sent": 0_i32,
        })
    );
}
