//! The emitted `ws_rpc` and `http_rest` dispatchers of `BelfryService` handed the message of its
//! operation of no argument, `{}`, and the message of its operation of a message of its own, each
//! read whole, with a key no member declares, and the latter with a member left out: the
//! TypeScript dispatchers under node, and the Dart and Kotlin `ws_rpc` attachments. Each reads the
//! message and calls the handler, and answers the rest with the Rust dispatchers'
//! `failed-validation` naming the field, calling no handler. The TypeScript clients name a key no
//! member declares in the message they are handed, refusing it before it is sent, and in a reply.
//! Each run resolves on the replies it counts, with no clock.

#[cfg(any(feature = "dart", feature = "kotlin"))]
use core::fmt::Write as _;

use super::BelfryServiceSchema;
use super::belfry_service::{
    BellError, BellPeal, BellRope, BellToll, PAYLOADS, TOLLS, frame, over_http, over_ws,
};
#[cfg(feature = "dart")]
use super::belfry_service::{bell_error_dart, bell_peal_dart, bell_rope_dart, bell_toll_dart};
#[cfg(feature = "kotlin")]
use super::belfry_service::{
    bell_error_kotlin, bell_peal_kotlin, bell_rope_kotlin, bell_toll_kotlin,
    ring_bell_request_kotlin,
};
#[cfg(feature = "kotlin")]
use super::belfry_service_schema;
#[cfg(feature = "dart")]
use super::run_dart_validate::analyzed;
#[cfg(feature = "dart")]
use super::runtime::ran;
#[cfg(feature = "kotlin")]
use super::runtime::ran_kotlin;
use super::runtime::{compiled_with_modules, node_modules, ran_with_modules, stand_down_modules};
use serde_json::{Value, json};

const REQUIRED_PACKAGES: &[&str] = &["zod"];

/// `CASES` and `FRAMES` are declared above this by the Rust side: each operation and payload, and
/// the frame that carries it.
const NODE_DRIVER: &str = r#"
async function main() {
  let rung = 0;
  const impl = {
    async ringBell() {
      rung += 1;
      return { ok: true, value: { strokes: rung } };
    },
    async tollBell(ctx, req) {
      rung += 1;
      return { ok: true, value: { strokes: req.strokes * req.rope.length } };
    },
  };
  const served = createBelfryServiceHttpDispatcher(impl);
  const http = [];
  for (const [operation, payload] of CASES) {
    rung = 0;
    const response = await served({}, {
      method: "POST",
      path: `/${operation}`,
      query: "",
      headers: [["content-type", "application/json"]],
      body: new TextEncoder().encode(payload),
    });
    http.push({ status: response.status, body: JSON.parse(new TextDecoder().decode(response.body)), rung });
  }
  const ws = [];
  for (const frame of FRAMES) {
    rung = 0;
    let listener;
    let answer;
    const replied = new Promise((resolve) => { answer = resolve; });
    const socket = {
      send(text) { answer(text); },
      close() {},
      addEventListener(type, added) { if (type === "message") listener = added; },
      removeEventListener() {},
    };
    const detach = attachBelfryServiceWsDispatcher(socket, {}, impl, () => {});
    listener({ data: frame });
    const reply = JSON.parse(await replied);
    detach();
    ws.push({ reply, rung });
  }
  console.log(JSON.stringify({ http, ws }));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

/// Each TypeScript client handed `{}`, then a key no member declares beside the members and below
/// them, over the TypeScript dispatchers; then a reply holding such a key. `sent` counts what
/// reached each transport.
const CLIENT_DRIVER: &str = r#"
function fakeSocket(onSend) {
  const listeners = { message: new Set(), close: new Set() };
  return {
    send(text) { onSend(text); },
    close() { for (const listener of listeners.close) listener(); },
    addEventListener(type, listener) { listeners[type].add(listener); },
    removeEventListener(type, listener) { listeners[type].delete(listener); },
    deliver(text) { for (const listener of listeners.message) listener({ data: text }); },
  };
}

const impl = {
  async ringBell() { return { ok: true, value: { strokes: 1 } }; },
  async tollBell(ctx, req) { return { ok: true, value: { strokes: req.strokes * req.rope.length } }; },
};
const strayReply = { strokes: 1, stray: 1 };

function told(answered) {
  if (answered.ok) return { read: answered.value };
  if (!answered.error.isServiceFault) return { declared: answered.error };
  return { kind: answered.error.fault.kind, field: answered.error.fault.field ?? null };
}

async function calls(client) {
  return [
    told(await client.ringBell({})),
    told(await client.ringBell({ stray: 1 })),
    told(await client.tollBell({ rope: { length: 3 }, strokes: 2, stray: 1 })),
    told(await client.tollBell({ rope: { length: 3, stray: 1 }, strokes: 2 })),
  ];
}

async function main() {
  const sent = { bus: 0, ws: 0, http: 0 };
  const dispatch = createBelfryServiceDispatcher(impl);
  const bus = createBelfryServiceClient({
    async notify() {},
    async request(operation, payload, headers) {
      sent.bus += 1;
      return JSON.parse(JSON.stringify(await dispatch({}, operation, JSON.parse(JSON.stringify(payload)), headers)));
    },
  });
  const strayBus = createBelfryServiceClient({
    async notify() {},
    async request() { return { answered: { ok: true, value: strayReply }, headers: [] }; },
  });

  let serverSocket;
  const clientSocket = fakeSocket((frame) => { sent.ws += 1; queueMicrotask(() => serverSocket.deliver(frame)); });
  serverSocket = fakeSocket((frame) => queueMicrotask(() => clientSocket.deliver(frame)));
  attachBelfryServiceWsDispatcher(serverSocket, {}, impl, () => {});
  const ws = createBelfryServiceClient(createBelfryServiceWsTransport(clientSocket, { heartbeat: false }));
  const straySocket = fakeSocket((frame) => {
    const { id } = JSON.parse(frame);
    const reply = { kind: "reply", id, service: "BelfryService", ok: true, value: strayReply };
    queueMicrotask(() => straySocket.deliver(JSON.stringify(reply)));
  });
  const strayWs = createBelfryServiceClient(createBelfryServiceWsTransport(straySocket, { heartbeat: false }));

  const served = createBelfryServiceHttpDispatcher(impl);
  const http = createBelfryServiceHttpClient({ async send(request) { sent.http += 1; return served({}, request); } });
  const strayHttp = createBelfryServiceHttpClient({
    async send() {
      return {
        status: 200,
        headers: [["content-type", "application/json"]],
        body: new TextEncoder().encode(JSON.stringify(strayReply)),
      };
    },
  });

  const report = {
    bus: await calls(bus),
    ws: await calls(ws),
    http: await calls(http),
    replies: {
      bus: told(await strayBus.ringBell({})),
      ws: told(await strayWs.ringBell({})),
      http: told(await strayHttp.ringBell({})),
    },
    sent,
  };
  clientSocket.close();
  straySocket.close();
  console.log(JSON.stringify(report));
  process.exit(0);
}
main().catch((error) => { console.error(error); process.exit(1); });
"#;

#[cfg(feature = "dart")]
const DART_DRIVER: &str = "
void main() async {
  var rung = 0;
  final handlers = BelfryServiceHandlers<void>(
    ringBell: (ctx, req) async {
      rung += 1;
      return BellPeal(strokes: rung);
    },
    tollBell: (ctx, req) async {
      rung += 1;
      return BellPeal(strokes: req.strokes * req.rope.length);
    },
  );
  final told = <Map<String, dynamic>>[];
  for (final frame in frames) {
    rung = 0;
    final inbound = StreamController<Map<String, dynamic>>();
    final replied = Completer<Map<String, dynamic>>();
    final detach = attachBelfryServiceWsDispatcher<void>(
      (inbound: inbound.stream, send: replied.complete),
      null,
      handlers,
      onFault: (fault) {},
    );
    inbound.add(jsonDecode(frame) as Map<String, dynamic>);
    final reply = await replied.future;
    detach();
    told.add(<String, dynamic>{'reply': reply, 'rung': rung});
  }
  print(jsonEncode(told));
}
";

#[cfg(feature = "kotlin")]
const KOTLIN_DRIVER: &str = r#"
class ManualSocket(private val onSend: (String) -> Unit) : BelfryServiceWsSocket {
    override var onMessage: ((String) -> Unit)? = null
    override var onClose: (() -> Unit)? = null
    override fun send(text: String) = onSend(text)
    override fun close() { onClose?.invoke() }
    fun deliver(text: String) { onMessage?.invoke(text) }
}

object Handlers : BelfryServiceHandlers {
    val rung = java.util.concurrent.atomic.AtomicInteger(0)
    override suspend fun ringBell(req: RingBellRequest): BelfryServiceRingBellResult =
        BelfryServiceRingBellResult.Ok(BellPealDraft(rung.incrementAndGet().toUInt()).validate())
    override suspend fun tollBell(req: BellToll): BelfryServiceTollBellResult {
        rung.incrementAndGet()
        return BelfryServiceTollBellResult.Ok(BellPealDraft(req.strokes * req.rope.length).validate())
    }
}

fun main() = runBlocking {
    val told = mutableListOf<JsonElement>()
    for (frame in FRAMES) {
        Handlers.rung.set(0)
        val replied = CompletableDeferred<String>()
        val socket = ManualSocket { text -> replied.complete(text) }
        val transport = BelfryServiceWsTransport(socket, BelfryServiceWsOptions(heartbeat = null))
        attachBelfryServiceWsDispatcher(transport.frames, Handlers, onFault = {})
        socket.deliver(frame)
        val reply = Json.parseToJsonElement(replied.await())
        transport.close()
        told.add(buildJsonObject {
            put("reply", reply)
            put("rung", Handlers.rung.get())
        })
    }
    println(JsonArray(told).toString())
}
"#;

/// Each operation and payload the TypeScript dispatchers are handed.
fn cases() -> Vec<(&'static str, &'static str)> {
    PAYLOADS
        .iter()
        .map(|payload| ("ring-bell", *payload))
        .chain(TOLLS.iter().map(|payload| ("toll-bell", *payload)))
        .collect()
}

/// What a dispatcher must answer of `payload` sent to `operation`, as the Rust `ws_rpc`
/// dispatcher answers it: a reply that reads whole, with as many rings reached; and of a message
/// it refuses the fault's kind and field, no ring reached.
fn assert_answered_as_rust(reply: &Value, rung: &Value, (operation, payload): (&str, &str)) {
    let (rust, rust_rung) = over_ws(operation, payload);
    assert_eq!(rung, &json!(rust_rung), "{operation} {payload}: {reply}");
    if rust["ok"] == true {
        let mut expected = rust;
        expected["id"] = reply["id"].clone();
        assert_eq!(reply, &expected, "{operation} {payload}");
    } else {
        let fault = &reply["error"]["fault"];
        let rust_fault = &rust["error"]["fault"];
        assert_eq!(reply["ok"], false, "{operation} {payload}: {reply}");
        assert_eq!(
            fault["kind"], rust_fault["kind"],
            "{operation} {payload}: {reply}"
        );
        assert_eq!(
            fault["field"], rust_fault["field"],
            "{operation} {payload}: {reply}"
        );
        assert!(fault["field"].is_string(), "{operation} {payload}: {reply}");
    }
}

/// The frames the Dart and Kotlin attachments are handed, one for each of [`cases`].
#[cfg(any(feature = "dart", feature = "kotlin"))]
fn frames() -> Vec<String> {
    cases()
        .into_iter()
        .map(|(operation, payload)| frame(operation, payload))
        .collect()
}

fn node_bundle() -> String {
    [
        "import { z, type ZodType, type $brand } from \"zod\";".to_owned(),
        BellPeal::ts_definition(),
        BellPeal::zod_schema(),
        BellError::ts_definition(),
        BellError::zod_schema(),
        BellRope::ts_definition(),
        BellRope::zod_schema(),
        BellToll::ts_definition(),
        BellToll::zod_schema(),
        BelfryServiceSchema::ts_definition(),
        BelfryServiceSchema::ts_service(),
        BelfryServiceSchema::ts_client(),
        BelfryServiceSchema::ts_ws_client(),
        BelfryServiceSchema::ts_ws_service(),
        BelfryServiceSchema::ts_http_service(),
        BelfryServiceSchema::ts_http_client(),
    ]
    .join("\n\n")
}

#[test]
fn the_typescript_text_of_the_belfry_service_type_checks() {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the type-check of the belfry service's text",
        );
        return;
    };
    let Some((succeeded, said)) = compiled_with_modules(
        "belfry-types",
        "TIXSCHEMA_TSC",
        &[("bundle.ts", &node_bundle())],
        &modules,
    ) else {
        return;
    };
    assert!(succeeded, "the emitted text does not type-check:\n{said}");
}

#[test]
fn the_typescript_dispatchers_read_each_message_and_name_the_field_they_refuse() {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the emitted TypeScript dispatchers of the belfry service",
        );
        return;
    };
    let framed: Vec<String> = cases()
        .into_iter()
        .map(|(operation, payload)| frame(operation, payload))
        .collect();
    let given = format!(
        "const CASES = {};\nconst FRAMES = {};",
        json!(cases()),
        json!(framed)
    );
    let source = format!("{}\n\n{given}\n{NODE_DRIVER}", node_bundle());
    let Some(wrote) = ran_with_modules(
        "belfry",
        "TIXSCHEMA_NODE",
        "node",
        "belfry.mts",
        &source,
        &modules,
    ) else {
        return;
    };
    let report: Value = serde_json::from_str(wrote.trim()).unwrap();
    for (at, (operation, payload)) in cases().into_iter().enumerate() {
        let (status, body, rung) = over_http(operation, payload);
        let served = &report["http"][at];
        assert_eq!(
            served["status"], status,
            "http {operation} {payload}: {served}"
        );
        assert_eq!(served["rung"], rung, "http {operation} {payload}: {served}");
        if status == 200 {
            assert_eq!(served["body"], body, "http {operation} {payload}");
        } else {
            assert_eq!(
                served["body"]["kind"], body["kind"],
                "http {operation} {payload}: {served}"
            );
            assert_eq!(
                served["body"]["field"], body["field"],
                "http {operation} {payload}: {served}"
            );
        }
        let replied = &report["ws"][at];
        assert_answered_as_rust(&replied["reply"], &replied["rung"], (operation, payload));
    }
}

#[test]
fn the_typescript_clients_name_a_key_no_member_declares() {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the emitted TypeScript clients of the belfry service",
        );
        return;
    };
    let source = format!("{}\n\n{CLIENT_DRIVER}", node_bundle());
    let Some(wrote) = ran_with_modules(
        "belfry-clients",
        "TIXSCHEMA_NODE",
        "node",
        "belfry-clients.mts",
        &source,
        &modules,
    ) else {
        return;
    };
    let report: Value = serde_json::from_str(wrote.trim()).unwrap();
    let refused = |operation: &str, payload: &str| {
        let (answered, _) = over_ws(operation, payload);
        json!({ "kind": "failed-validation", "field": answered["error"]["fault"]["field"] })
    };
    let expected = json!([
        { "read": { "strokes": 1_u8 } },
        refused("ring-bell", PAYLOADS[1]),
        refused("toll-bell", TOLLS[1]),
        refused("toll-bell", TOLLS[2]),
    ]);
    assert_eq!(expected[1]["field"], "stray");
    assert_eq!(expected[3]["field"], "rope.stray");
    for client in ["bus", "ws", "http"] {
        assert_eq!(report[client], expected, "{client}");
        assert_eq!(
            report["sent"][client], 1_u8,
            "{client} sent a refused message"
        );
    }
    let named = json!({ "kind": "failed-validation", "field": "stray" });
    assert_eq!(report["replies"]["bus"], named);
    assert_eq!(report["replies"]["ws"], named);
    assert_eq!(
        report["replies"]["http"],
        json!({ "kind": "undeserializable-payload", "field": null })
    );
}

/// The Dart module the attachment runs in: the emitted text, the frames and the driver.
#[cfg(feature = "dart")]
fn dart_source() -> String {
    let mut given = String::from("final frames = <String>[\n");
    for written in frames() {
        let _ = writeln!(given, "  r'''{written}''',");
    }
    given.push_str("];");
    [
        "import 'dart:async';".to_owned(),
        "import 'dart:convert';".to_owned(),
        bell_peal_dart::dart_definition(),
        bell_error_dart::dart_definition(),
        bell_rope_dart::dart_definition(),
        bell_toll_dart::dart_definition(),
        BelfryServiceSchema::dart_definition(),
        BelfryServiceSchema::dart_ws_client(),
        given,
        DART_DRIVER.to_owned(),
    ]
    .join("\n\n")
}

#[cfg(feature = "dart")]
#[test]
fn the_dart_attachment_reads_each_message_and_names_the_field_it_refuses() {
    let Some(wrote) = ran(
        "dart",
        "TIXSCHEMA_DART",
        "dart",
        "belfry.dart",
        &dart_source(),
    ) else {
        return;
    };
    let told: Value = serde_json::from_str(wrote.trim()).unwrap();
    for (at, case) in cases().into_iter().enumerate() {
        assert_answered_as_rust(&told[at]["reply"], &told[at]["rung"], case);
    }
}

#[cfg(feature = "dart")]
#[test]
fn the_dart_text_of_the_belfry_service_passes_the_analyzer() {
    analyzed("belfry-analyze", &dart_source());
}

#[cfg(feature = "kotlin")]
#[test]
fn the_kotlin_attachment_reads_each_message_and_names_the_field_it_refuses() {
    let mut given = String::from("val FRAMES = listOf(\n");
    for written in frames() {
        let _ = writeln!(given, "    \"\"\"{written}\"\"\",");
    }
    given.push(')');
    let source = [
        "import kotlinx.coroutines.*\n\
         import kotlinx.coroutines.flow.*\n\
         import kotlinx.serialization.*\n\
         import kotlinx.serialization.json.*\n\
         import kotlinx.serialization.descriptors.*\n\
         import kotlinx.serialization.encoding.*\n\
         import kotlinx.serialization.builtins.*"
            .to_owned(),
        bell_peal_kotlin::kotlin_definition(),
        bell_error_kotlin::kotlin_definition(),
        bell_rope_kotlin::kotlin_definition(),
        bell_toll_kotlin::kotlin_definition(),
        ring_bell_request_kotlin::kotlin_definition(),
        belfry_service_schema::belfry_service_fault_fields_kotlin::kotlin_definition(),
        belfry_service_schema::belfry_service_fault_kind_kotlin::kotlin_definition(),
        BelfryServiceSchema::kotlin_http_client(),
        BelfryServiceSchema::kotlin_ws_client(),
        given,
        KOTLIN_DRIVER.to_owned(),
    ]
    .join("\n\n");
    let Some(wrote) = ran_kotlin(&source) else {
        return;
    };
    let told: Value = serde_json::from_str(wrote.trim()).unwrap();
    for (at, case) in cases().into_iter().enumerate() {
        assert_answered_as_rust(&told[at]["reply"], &told[at]["rung"], case);
    }
}
