//! `SundialClientService` called through the emitted `http_rest` and `ws_rpc` clients of each
//! language, each transport answering with what the Rust dispatcher of its kind writes: each
//! client sends the message, the `char` header and the date header in the form those dispatchers
//! read, refuses a `char` of two characters before it sends anything, and reads the date the reply
//! and its header hold.

#![cfg(all(
    feature = "chrono",
    any(
        all(feature = "typescript", feature = "zod"),
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
))]

use super::sundial_service::{AT, LETTER, NO_LETTER, REQ, held, http_answer, ws_reply};
#[cfg(all(feature = "typescript", feature = "zod"))]
use super::sundial_service::{SundialClientServiceSchema, SundialError};

/// The Dart driver: the `http_rest` and `ws_rpc` clients, each answered with what the Rust
/// dispatchers write.
#[cfg(feature = "dart")]
const DART_DRIVER: &str = r#"
class _Answering implements SundialClientServiceHttpTransport {
  Object? sent;

  @override
  Future<({int status, List<(String, String)> headers, List<int> body, Stream<List<int>> bodyStream})> send(
    ({String method, String path, String query, List<(String, String)> headers, List<int> body, List<(String, dynamic)> parts}) request,
  ) async {
    sent = <String, dynamic>{
      'method': request.method,
      'path': request.path,
      'query': request.query,
      'headers': [for (final (name, value) in request.headers) [name, value]],
      'body': utf8.decode(request.body),
    };
    return (
      status: @status@,
      headers: <(String, String)>[@headers@],
      body: utf8.encode(@body@),
      bodyStream: const Stream<List<int>>.empty(),
    );
  }
}

Object? _read(SundialClientServiceReadDialResult answered) => answered is SundialClientServiceReadDialResultOk
    ? [answered.value.$1.toUtc().toIso8601String(), answered.value.$2.toUtc().toIso8601String()]
    : '$answered';

void main() async {
  final transport = _Answering();
  final http = SundialClientServiceHttpClient(transport);
  final answered = await http.readDial(@req@, @letter@, DateTime.parse(@at@));
  print(jsonEncode(transport.sent));
  print(jsonEncode(_read(answered)));
  transport.sent = null;
  try {
    await http.readDial(@req@, @no_letter@, DateTime.parse(@at@));
  } on FormatException {
    print(jsonEncode(transport.sent ?? 'refused'));
  }

  String? frame;
  final outbound = StreamController<dynamic>();
  final inbound = StreamController<dynamic>();
  outbound.stream.listen((text) {
    frame = text as String;
    final id = (jsonDecode(text) as Map<String, dynamic>)['id'];
    inbound.add(@reply@.replaceAll('"ID"', jsonEncode(id)));
  });
  final socket = SundialClientServiceWsTransport(
    sink: outbound.sink,
    stream: inbound.stream,
    heartbeat: SundialClientServiceWsHeartbeat.off(),
  );
  final ws = SundialClientServiceWsClient(socket);
  final replied = await ws.readDial(@req@, @letter@, DateTime.parse(@at@));
  print(jsonEncode(frame));
  print(jsonEncode(_read(replied)));
  frame = null;
  try {
    await ws.readDial(@req@, @no_letter@, DateTime.parse(@at@));
  } on FormatException {
    print(jsonEncode(frame ?? 'refused'));
  }
  socket.close();
  await outbound.close();
  await inbound.close();
}
"#;

/// The Kotlin driver: the `http_rest` and `ws_rpc` clients, each answered with what the Rust
/// dispatchers write.
#[cfg(feature = "kotlin")]
const KOTLIN_DRIVER: &str = r#"
class Answering : SundialClientServiceHttpTransport {
    var sent: JsonObject? = null
    override suspend fun send(request: SundialClientServiceHttpRequest): SundialClientServiceHttpResponse {
        sent = buildJsonObject {
            put("method", request.method)
            put("path", request.path)
            put("query", request.query)
            put("headers", buildJsonArray {
                for ((name, value) in request.headers) add(buildJsonArray { add(name); add(value) })
            })
            put("body", request.body.decodeToString())
        }
        return SundialClientServiceHttpResponse(@status@, listOf(@headers@), @body@.encodeToByteArray())
    }
}

class Replying : SundialClientServiceWsSocket {
    var sent: String? = null
    override var onMessage: ((String) -> Unit)? = null
    override var onClose: (() -> Unit)? = null
    override fun send(text: String) {
        sent = text
        val id = Json.parseToJsonElement(text).jsonObject.getValue("id").jsonPrimitive.content
        onMessage?.invoke(@reply@.replace("\"ID\"", "\"" + id + "\""))
    }
    override fun close() {
        onClose?.invoke()
    }
}

fun read(answered: SundialClientServiceReadDialResult): JsonElement = when (answered) {
    is SundialClientServiceReadDialResult.Ok -> buildJsonArray { add(answered.value.value); add(answered.value.headerOut0) }
    else -> JsonPrimitive(answered.toString())
}

fun main() = runBlocking {
    val transport = Answering()
    val http = SundialClientServiceHttpClient(transport)
    val answered = http.readDial(@req@, @letter@, @at@)
    println(transport.sent.toString())
    println(read(answered).toString())
    transport.sent = null
    try {
        http.readDial(@req@, @no_letter@, @at@)
    } catch (refusal: SerializationException) {
        println(transport.sent?.toString() ?: JsonPrimitive("refused").toString())
    }

    val socket = Replying()
    val ws = SundialClientServiceWsTransport(socket, SundialClientServiceWsOptions(heartbeat = null))
    val replied = SundialClientServiceWsClient(ws).readDial(@req@, @letter@, @at@)
    println(JsonPrimitive(socket.sent).toString())
    println(read(replied).toString())
    socket.sent = null
    try {
        SundialClientServiceWsClient(ws).readDial(@req@, @no_letter@, @at@)
    } catch (refusal: SerializationException) {
        println(JsonPrimitive(socket.sent ?: "refused").toString())
    }
    ws.close()
}
"#;

/// The Swift driver: the `http_rest` and `ws_rpc` clients, each answered with what the Rust
/// dispatchers write.
#[cfg(feature = "swift")]
const SWIFT_DRIVER: &str = r#"
struct Sent: Codable {
  let method: String
  let path: String
  let query: String
  let headers: [[String]]
  let body: String
}

actor Answering: SundialClientServiceHttpTransport {
  private(set) var sent: Sent?

  func clear() {
    sent = nil
  }

  func send(_ request: SundialClientServiceHttpRequest) async throws -> SundialClientServiceHttpResponse {
    sent = Sent(
      method: request.method,
      path: request.path,
      query: request.query,
      headers: request.headers.map { [$0.0, $0.1] },
      body: String(decoding: request.body, as: UTF8.self)
    )
    return SundialClientServiceHttpResponse(status: @status@, headers: [@headers@], body: Data(@body@.utf8))
  }
}

final class Replying: SundialClientServiceWsSocket, @unchecked Sendable {
  var onMessage: (@Sendable (String) -> Void)?
  var onClose: (@Sendable () -> Void)?
  var sent: String?

  func send(_ text: String) {
    sent = text
    let frame = try! JSONSerialization.jsonObject(with: Data(text.utf8)) as! [String: Any]
    let id = frame["id"] as! String
    onMessage?(@reply@.replacingOccurrences(of: "\"ID\"", with: "\"\(id)\""))
  }

  func close() {
    onClose?()
  }
}

func printed(_ value: some Encodable) {
  print(String(decoding: try! JSONEncoder().encode(value), as: UTF8.self))
}

let iso = ISO8601DateFormatter()
iso.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
let at = iso.date(from: @at@)!

func read(_ answered: Result<(Date, Date), SundialClientServiceReadDialFailure>, _ iso: ISO8601DateFormatter) -> [String] {
  switch answered {
  case .success(let (value, seen)): return [iso.string(from: value), iso.string(from: seen)]
  case .failure(let failure): return ["\(failure)"]
  }
}

let transport = Answering()
let http = SundialClientServiceHttpClient(transport: transport)
let answered = await http.readDial(@req@, letter: @letter@, at: at)
printed(await transport.sent)
printed(read(answered, iso))
await transport.clear()
_ = await http.readDial(@req@, letter: @no_letter@, at: at)
if let sent = await transport.sent { printed(sent) } else { printed("refused") }

let socket = Replying()
let ws = SundialClientServiceWsClient(socket: socket, options: .init(heartbeat: nil))
let replied = await ws.readDial(@req@, letter: @letter@, at: at)
printed(socket.sent)
printed(read(replied, iso))
socket.sent = nil
_ = await ws.readDial(@req@, letter: @no_letter@, at: at)
printed(socket.sent ?? "refused")
await ws.close()
"#;

/// The TypeScript driver: the `http_rest` client over a transport and the transport-agnostic client
/// over the `ws_rpc` transport, each answered with what the Rust dispatchers write.
#[cfg(all(feature = "typescript", feature = "zod"))]
const TYPESCRIPT_DRIVER: &str = r#"
let sent;
const http = createSundialClientServiceHttpClient({
  async send(request) {
    sent = {
      method: request.method,
      path: request.path,
      query: request.query,
      headers: request.headers,
      body: new TextDecoder().decode(request.body),
    };
    return { status: @status@, headers: [@headers@], body: new TextEncoder().encode(@body@) };
  },
});
const listeners = [];
const socket = {
  send(text) {
    sent = text;
    const id = JSON.parse(text).id;
    for (const listener of listeners) listener({ data: @reply@.replace('"ID"', JSON.stringify(id)) });
  },
  close() {},
  addEventListener(type, listener) {
    if (type === "message") listeners.push(listener);
  },
  removeEventListener() {},
};
const bus = createSundialClientServiceClient(createSundialClientServiceWsTransport(socket, { heartbeat: false }));
function read(answered) {
  return answered.ok ? answered.value.map((date) => date.toISOString()) : answered.error;
}
for (const client of [http, bus]) {
  sent = undefined;
  const answered = await client.readDial(@req@, @letter@, new Date(@at@));
  console.log(JSON.stringify(sent));
  console.log(JSON.stringify(read(answered)));
  sent = undefined;
  await client.readDial(@req@, @no_letter@, new Date(@at@));
  console.log(JSON.stringify(sent ?? "refused"));
}
"#;

/// `text` as a string literal TypeScript, Dart, Swift and Kotlin all read: the JSON one, none of
/// the texts here holding a `$`.
fn literal(text: &str) -> String {
    serde_json::to_string(text).unwrap()
}

/// `driver` with each `@name@` written: the status, the headers, as `headers` lists them, and the
/// body the Rust `http_rest` dispatcher answers, the reply frame the Rust `ws_rpc` one answers, and
/// the values handed, each a string literal.
fn driven(driver: &str, headers: &str) -> String {
    let (status, _, body) = http_answer();
    driver
        .replace("@status@", &status.to_string())
        .replace("@headers@", headers)
        .replace("@body@", &literal(&body))
        .replace("@reply@", &literal(&ws_reply()))
        .replace("@req@", &literal(REQ))
        .replace("@no_letter@", &literal(NO_LETTER))
        .replace("@letter@", &literal(LETTER))
        .replace("@at@", &literal(AT))
}

/// The response headers the Rust dispatcher wrote, as a list of pairs written by `pair`.
fn pairs(written: &dyn Fn(&str, &str) -> String) -> String {
    http_answer()
        .1
        .iter()
        .map(|(name, value)| written(&literal(name), &literal(value)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The TypeScript `http_rest` client and the transport-agnostic client over the `ws_rpc`
/// transport, run by node beside the real `zod`.
#[cfg(all(feature = "typescript", feature = "zod"))]
#[test]
fn the_typescript_clients_send_and_read_each_date_as_the_rust_dispatchers_do() {
    use super::runtime::{node_modules, ran_with_modules, stand_down_modules};

    let Some(modules) = node_modules(&["zod"]) else {
        stand_down_modules(&["zod"], "the TypeScript clients of a date");
        return;
    };
    let driver = driven(
        TYPESCRIPT_DRIVER,
        &pairs(&|name, value| format!("[{name}, {value}]")),
    );
    let module = [
        "import { z, type ZodType } from \"zod\";".to_owned(),
        SundialError::ts_definition(),
        SundialError::zod_schema(),
        SundialClientServiceSchema::ts_definition(),
        SundialClientServiceSchema::ts_client(),
        SundialClientServiceSchema::ts_ws_client(),
        SundialClientServiceSchema::ts_http_client(),
        driver,
    ]
    .join("\n\n");
    let Some(printed) = ran_with_modules(
        "sundial",
        "TIXSCHEMA_NODE",
        "node",
        "sundial.mts",
        &module,
        &modules,
    ) else {
        return;
    };
    held(&printed);
}

/// The Dart `http_rest` and `ws_rpc` clients, run under the Dart SDK.
#[cfg(feature = "dart")]
#[test]
fn the_dart_clients_send_and_read_each_date_as_the_rust_dispatchers_do() {
    use super::run_dart_wire_forms::dart_at_utc;
    use super::sundial_service::{SundialClientServiceSchema, sundial_error_dart};

    let driver = driven(
        DART_DRIVER,
        &pairs(&|name, value| format!("({name}, {value})")),
    );
    let source = [
        "import 'dart:async';".to_owned(),
        "import 'dart:convert';".to_owned(),
        sundial_error_dart::dart_definition(),
        SundialClientServiceSchema::dart_definition(),
        SundialClientServiceSchema::dart_http_client(),
        SundialClientServiceSchema::dart_ws_client(),
        driver,
    ]
    .join("\n\n");
    let Some((ran, printed)) = dart_at_utc("sundial", &source) else {
        return;
    };
    assert!(ran, "{printed}");
    held(&printed);
}

/// The Swift `http_rest` and `ws_rpc` clients, compiled with warnings as errors.
#[cfg(feature = "swift")]
#[test]
fn the_swift_clients_send_and_read_each_date_as_the_rust_dispatchers_do() {
    use super::runtime::judged;
    use super::sundial_service::sundial_client_service_schema::{
        sundial_client_service_fault_fields_swift, sundial_client_service_fault_kind_swift,
    };
    use super::sundial_service::{SundialClientServiceSchema, sundial_error_swift};

    let driver = driven(
        SWIFT_DRIVER,
        &pairs(&|name, value| format!("({name}, {value})")),
    );
    let source = [
        "import Foundation".to_owned(),
        sundial_error_swift::swift_definition(),
        sundial_client_service_fault_fields_swift::swift_definition(),
        sundial_client_service_fault_kind_swift::swift_definition(),
        SundialClientServiceSchema::swift_http_client(),
        SundialClientServiceSchema::swift_ws_client(),
        driver,
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-sundial",
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
        "`swift -warnings-as-errors main.swift` failed.\n--- stdout ---\n{}\n--- stderr ---\n{}",
        run.stdout, run.stderr
    );
    held(&run.stdout);
}

/// The Kotlin `http_rest` and `ws_rpc` clients, compiled with `-Werror`.
#[cfg(feature = "kotlin")]
#[test]
fn the_kotlin_clients_send_and_read_each_date_as_the_rust_dispatchers_do() {
    use super::runtime::ran_kotlin;
    use super::sundial_service::sundial_client_service_schema::{
        sundial_client_service_fault_fields_kotlin, sundial_client_service_fault_kind_kotlin,
    };
    use super::sundial_service::{SundialClientServiceSchema, sundial_error_kotlin};

    let driver = driven(
        KOTLIN_DRIVER,
        &pairs(&|name, value| format!("{name} to {value}")),
    );
    let source = [
        "import kotlinx.coroutines.*\n\
         import kotlinx.coroutines.flow.*\n\
         import kotlinx.serialization.*\n\
         import kotlinx.serialization.json.*\n\
         import kotlinx.serialization.descriptors.*\n\
         import kotlinx.serialization.encoding.*\n\
         import kotlinx.serialization.builtins.*"
            .to_owned(),
        sundial_error_kotlin::kotlin_definition(),
        sundial_client_service_fault_fields_kotlin::kotlin_definition(),
        sundial_client_service_fault_kind_kotlin::kotlin_definition(),
        SundialClientServiceSchema::kotlin_http_client(),
        SundialClientServiceSchema::kotlin_ws_client(),
        driver,
    ]
    .join("\n\n");
    let Some(printed) = ran_kotlin(&source) else {
        return;
    };
    held(&printed);
}
