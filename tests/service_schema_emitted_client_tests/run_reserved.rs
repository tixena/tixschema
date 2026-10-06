//! A service whose operations, arguments and message fields are named after words TypeScript,
//! Dart, Swift or Kotlin reserves, and after locals a client writes itself, called through each
//! emitted client.
//!
//! Each client is handed a transport that records what it is sent. Every recorded request is then
//! read by the Rust dispatcher, whose handler answers with what it was handed: the client reached
//! the URL, the headers and the body the Rust side reads.

use super::lookup_http_rest_transport;
use super::runtime::ran;
#[cfg(feature = "kotlin")]
use super::runtime::ran_kotlin;
#[cfg(feature = "kotlin")]
use super::tests::lookup_client_service_schema::{
    lookup_client_service_fault_fields_kotlin, lookup_client_service_fault_kind_kotlin,
};
#[cfg(feature = "swift")]
use super::tests::lookup_client_service_schema::{
    lookup_client_service_fault_fields_swift, lookup_client_service_fault_kind_swift,
};
use super::tests::{LookupBackEnd, LookupClientServiceSchema};
#[cfg(feature = "kotlin")]
use super::tests::{
    for_request_kotlin, import_request_kotlin, lookup_error_kotlin, lookup_found_kotlin,
    lookup_thing_kotlin, shadow_request_kotlin,
};
#[cfg(feature = "swift")]
use super::tests::{
    for_request_swift, import_request_swift, lookup_error_swift, lookup_found_swift,
    lookup_thing_swift, shadow_request_swift,
};
#[cfg(feature = "dart")]
use super::tests::{lookup_error_dart, lookup_found_dart, lookup_thing_dart};
use core::future::Future;
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};

/// Records each request and answers `200`, then makes the three calls.
#[cfg(feature = "dart")]
const DART_DRIVER: &str = "
class _LookupRecorder implements LookupClientServiceHttpTransport {
  final List<Map<String, dynamic>> sent = <Map<String, dynamic>>[];

  @override
  Future<({int status, List<(String, String)> headers, List<int> body, Stream<List<int>> bodyStream})> send(
    ({String method, String path, String query, List<(String, String)> headers, List<int> body, List<(String, dynamic)> parts}) request,
  ) async {
    sent.add(<String, dynamic>{
      'method': request.method,
      'path': request.path,
      'query': request.query,
      'headers': [for (final (name, value) in request.headers) [name, value]],
      'body': utf8.decode(request.body),
    });
    return (
      status: 200,
      headers: <(String, String)>[],
      body: utf8.encode(jsonEncode(<String, dynamic>{'name': ''})),
      bodyStream: const Stream<List<int>>.empty(),
    );
  }
}

void main() async {
  final recorder = _LookupRecorder();
  final client = LookupClientServiceHttpClient(recorder);
  await client.import(ImportRequest(class_: 'k', object: 'o', default_: 'd', var_: 'v'));
  await client.for_(ForRequest(in_: 'i', final_: 'f'), 't', 'n');
  await client.object(LookupThing(class_: 'c', var_: 'w'));
  await client.shadow(ShadowRequest(message: 'm', sending: 's'), 'b', 'p', 't');
  print(jsonEncode(recorder.sent));
}
";

/// The Kotlin twin of [`DART_DRIVER`].
#[cfg(feature = "kotlin")]
const KOTLIN_DRIVER: &str = r#"
class LookupRecorder : LookupClientServiceHttpTransport {
    val sent = mutableListOf<LookupClientServiceHttpRequest>()
    override suspend fun send(request: LookupClientServiceHttpRequest): LookupClientServiceHttpResponse {
        sent.add(request)
        return LookupClientServiceHttpResponse(200, emptyList(), """{"name":""}""".encodeToByteArray())
    }
}

fun main() = runBlocking {
    val transport = LookupRecorder()
    val client = LookupClientServiceHttpClient(transport)
    client.import(ImportRequest(`class` = "k", `object` = "o", default = "d", `var` = "v"))
    client.`for`(ForRequest(`in` = "i", final = "f"), "t", "n")
    client.`object`(LookupThing(`class` = "c", `var` = "w"))
    client.shadow(ShadowRequest(message = "m", sending = "s"), body = "b", path = "p", status = "t")
    val report = buildJsonArray {
        for (request in transport.sent) {
            add(buildJsonObject {
                put("method", request.method)
                put("path", request.path)
                put("query", request.query)
                put("headers", buildJsonArray {
                    for ((name, value) in request.headers) {
                        add(buildJsonArray { add(name); add(value) })
                    }
                })
                put("body", request.body.decodeToString())
            })
        }
    }
    println(report.toString())
}
"#;

/// The imports the emitted Kotlin and its driver reach for.
#[cfg(feature = "kotlin")]
const KOTLIN_IMPORTS: &str = "import kotlinx.coroutines.*\n\
     import kotlinx.coroutines.flow.*\n\
     import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// The TypeScript twin of [`DART_DRIVER`], under node.
const NODE_DRIVER: &str = r#"
const sent = [];
const transport = {
  async send(request) {
    sent.push(request);
    return { status: 200, headers: [], body: JSON.stringify({ name: "" }) };
  },
};
const client = createLookupClientServiceHttpClient(transport);
await client.import({ class: "k", object: "o", default: "d", var: "v" });
await client.for({ in: "i", final: "f" }, "t", "n");
await client.object({ class: "c", var: "w" });
await client.shadow({ message: "m", sending: "s" }, "b", "p", "t");
console.log(JSON.stringify(sent));
"#;

/// The schemas the emitted TypeScript client names, passing every value through untouched.
const NODE_SCHEMA_STUBS: &str =
    "const pass = { safeParse: (value) => ({ success: true, data: value }) };
const ForRequest$Schema = pass;
const ImportRequest$Schema = pass;
const LookupError$Schema = pass;
const LookupFound$Schema = pass;
const LookupThing$Schema = pass;
const ShadowRequest$Schema = pass;
";

/// The Swift twin of [`DART_DRIVER`].
#[cfg(feature = "swift")]
const SWIFT_DRIVER: &str = r##"
struct Sent: Codable {
  let method: String
  let path: String
  let query: String
  let headers: [[String]]
  let body: String
}

actor LookupRecorder: LookupClientServiceHttpTransport {
  private(set) var sent: [Sent] = []

  func send(_ request: LookupClientServiceHttpRequest) async throws -> LookupClientServiceHttpResponse {
    sent.append(Sent(
      method: request.method,
      path: request.path,
      query: request.query,
      headers: request.headers.map { [$0.0, $0.1] },
      body: String(decoding: request.body, as: UTF8.self)
    ))
    return LookupClientServiceHttpResponse(status: 200, headers: [], body: Data(#"{"name":""}"#.utf8))
  }
}

let recorder = LookupRecorder()
let client = LookupClientServiceHttpClient(transport: recorder)
_ = await client.`import`(ImportRequest(`class`: "k", object: "o", `default`: "d", `var`: "v"))
_ = await client.`for`(ForRequest(`in`: "i", final: "f"), type: "t", `default`: "n")
_ = await client.object(LookupThing(`class`: "c", `var`: "w"))
_ = await client.shadow(ShadowRequest(message: "m", sending: "s"), body: "b", path: "p", status: "t")
let sent = await recorder.sent
print(String(data: try! JSONEncoder().encode(sent), encoding: .utf8)!)
"##;

/// Answers every fault `499`, under a header naming its kind.
struct RefusedAs499;

impl lookup_http_rest_transport::FaultHandler for RefusedAs499 {
    fn on_fault(
        &self,
        fault: &super::tests::lookup_client_service_schema::ServiceFault,
    ) -> lookup_http_rest_transport::OutgoingResponse {
        lookup_http_rest_transport::OutgoingResponse::new(
            499,
            vec![("x-fault-kind".to_owned(), format!("{}", fault.kind()))],
            b"refused".to_vec(),
        )
    }
}

/// The dispatcher never suspends, so one poll answers it.
fn poll_once<Answering>(answering: Answering) -> Option<Answering::Output>
where
    Answering: Future,
{
    let mut pinned = pin!(answering);
    match pinned
        .as_mut()
        .poll(&mut PollContext::from_waker(Waker::noop()))
    {
        Poll::Ready(answer) => Some(answer),
        Poll::Pending => None,
    }
}

/// What the Rust handler names when it is handed each request a driver recorded, in order.
fn read_by_rust(printed: &str) -> Vec<String> {
    let sent: Vec<serde_json::Value> = serde_json::from_str(printed.trim()).unwrap();
    sent.iter()
        .map(|request| {
            let text = |key: &str| request[key].as_str().unwrap().to_owned();
            let headers = request["headers"]
                .as_array()
                .unwrap()
                .iter()
                .map(|pair| {
                    (
                        pair[0].as_str().unwrap().to_owned(),
                        pair[1].as_str().unwrap().to_owned(),
                    )
                })
                .collect();
            let incoming = lookup_http_rest_transport::IncomingRequest::new(
                text("method"),
                text("path"),
                text("query"),
                headers,
                text("body").into_bytes(),
            );
            let response = poll_once(lookup_http_rest_transport::dispatch(
                &LookupBackEnd,
                &(),
                &incoming,
                &lookup_http_rest_transport::DefaultFaultHandler,
            ))
            .unwrap();
            let answered: serde_json::Value = serde_json::from_slice(response.body()).unwrap();
            format!("{} {answered}", response.status())
        })
        .collect()
}

/// What every client's three calls come to once the Rust handlers have read them.
fn every_argument_read() -> Vec<String> {
    [
        r#"200 {"name":"k|o|d|v"}"#,
        r#"200 {"name":"i|f|t|n"}"#,
        r#"200 {"name":"c|w"}"#,
        r#"200 {"name":"m|s|b|p|t"}"#,
    ]
    .map(str::to_owned)
    .to_vec()
}

#[cfg(feature = "dart")]
#[test]
fn the_dart_client_calls_a_service_named_after_reserved_words() {
    let module = [
        "import 'dart:async';".to_owned(),
        "import 'dart:convert';".to_owned(),
        lookup_found_dart::dart_definition(),
        lookup_error_dart::dart_definition(),
        lookup_thing_dart::dart_definition(),
        LookupClientServiceSchema::dart_definition(),
        LookupClientServiceSchema::dart_http_client(),
        LookupClientServiceSchema::dart_ws_client(),
        DART_DRIVER.to_owned(),
    ]
    .join("\n\n");
    let Some(printed) = ran("dart", "TIXSCHEMA_DART", "dart", "client.dart", &module) else {
        return;
    };
    assert_eq!(read_by_rust(&printed), every_argument_read(), "{printed}");
}

#[cfg(feature = "kotlin")]
#[test]
fn the_kotlin_client_calls_a_service_named_after_reserved_words() {
    let module = [
        KOTLIN_IMPORTS.to_owned(),
        lookup_found_kotlin::kotlin_definition(),
        lookup_error_kotlin::kotlin_definition(),
        lookup_thing_kotlin::kotlin_definition(),
        import_request_kotlin::kotlin_definition(),
        for_request_kotlin::kotlin_definition(),
        shadow_request_kotlin::kotlin_definition(),
        lookup_client_service_fault_fields_kotlin::kotlin_definition(),
        lookup_client_service_fault_kind_kotlin::kotlin_definition(),
        LookupClientServiceSchema::kotlin_http_client(),
        LookupClientServiceSchema::kotlin_ws_client(),
        KOTLIN_DRIVER.to_owned(),
    ]
    .join("\n\n");
    let Some(printed) = ran_kotlin(&module) else {
        return;
    };
    assert_eq!(read_by_rust(&printed), every_argument_read(), "{printed}");
}

#[test]
fn the_typescript_client_calls_a_service_named_after_reserved_words() {
    let module = format!(
        "{NODE_SCHEMA_STUBS}\n{client}\n{NODE_DRIVER}",
        client = LookupClientServiceSchema::ts_http_client()
    );
    let Some(printed) = ran("node", "TIXSCHEMA_NODE", "node", "client.mts", &module) else {
        return;
    };
    assert_eq!(read_by_rust(&printed), every_argument_read(), "{printed}");
}

#[cfg(feature = "swift")]
#[test]
fn the_swift_client_calls_a_service_named_after_reserved_words() {
    let module = [
        "import Foundation".to_owned(),
        lookup_found_swift::swift_definition(),
        lookup_error_swift::swift_definition(),
        lookup_thing_swift::swift_definition(),
        import_request_swift::swift_definition(),
        for_request_swift::swift_definition(),
        shadow_request_swift::swift_definition(),
        lookup_client_service_fault_fields_swift::swift_definition(),
        lookup_client_service_fault_kind_swift::swift_definition(),
        LookupClientServiceSchema::swift_http_client(),
        LookupClientServiceSchema::swift_ws_client(),
        SWIFT_DRIVER.to_owned(),
    ]
    .join("\n\n");
    let Some(printed) = ran("swift", "TIXSCHEMA_SWIFT", "swift", "main.swift", &module) else {
        return;
    };
    assert_eq!(read_by_rust(&printed), every_argument_read(), "{printed}");
}

#[test]
fn the_rust_dispatcher_reads_each_argument_from_where_its_name_puts_it() {
    let sent = serde_json::json!([
        {
            "method": "POST",
            "path": "/items/i",
            "query": "",
            "headers": [["x-kind", "t"], ["x-tenant", "n"]],
            "body": r#"{"final":"f"}"#,
        },
        {
            "method": "GET",
            "path": "/classes/k/o",
            "query": "default=d&var=v",
            "headers": [],
            "body": "",
        },
        {
            "method": "PUT",
            "path": "/things/c",
            "query": "",
            "headers": [],
            "body": r#"{"class":"c","var":"w"}"#,
        },
        {
            "method": "POST",
            "path": "/shadows/m",
            "query": "",
            "headers": [["x-body", "b"], ["x-path", "p"], ["x-status", "t"]],
            "body": r#"{"sending":"s"}"#,
        },
    ]);
    assert_eq!(
        read_by_rust(&sent.to_string()),
        [
            r#"200 {"name":"i|f|t|n"}"#,
            r#"200 {"name":"k|o|d|v"}"#,
            r#"200 {"name":"c|w"}"#,
            r#"200 {"name":"m|s|b|p|t"}"#,
        ]
    );
}

#[test]
fn the_route_table_names_each_placeholder_as_the_path_writes_it() {
    let table: Vec<_> = lookup_http_rest_transport::ROUTES
        .iter()
        .map(|route| {
            (
                route.method(),
                route.path(),
                route.operation(),
                route.ok_status(),
                route.error_statuses(),
            )
        })
        .collect();
    let declared: &[u16] = &[422];
    assert_eq!(
        table,
        [
            ("POST", "/items/{in}", "for", 200, declared),
            ("GET", "/classes/{class}/{object}", "import", 200, declared),
            ("PUT", "/things/{class}", "object", 200, declared),
            ("POST", "/shadows/{message}", "shadow", 200, declared),
        ]
    );
}

#[test]
fn a_request_without_the_header_a_raw_identifier_is_bound_to_is_refused() {
    let incoming = lookup_http_rest_transport::IncomingRequest::new(
        "POST".to_owned(),
        "/items/i".to_owned(),
        String::new(),
        vec![("x-tenant".to_owned(), "n".to_owned())],
        br#"{"final":"f"}"#.to_vec(),
    );
    assert_eq!(incoming.headers().len(), 1);
    let response = poll_once(lookup_http_rest_transport::dispatch(
        &LookupBackEnd,
        &(),
        &incoming,
        &RefusedAs499,
    ))
    .unwrap();
    assert_eq!(response.status(), 499);
    assert_eq!(response.body(), b"refused");
    assert_eq!(response.headers().len(), 1, "{:?}", response.headers());
}
