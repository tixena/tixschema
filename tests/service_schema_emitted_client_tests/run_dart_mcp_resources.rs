//! The emitted Dart `mcp` server run by the Dart VM over resources, templates, a listing and a
//! completion: it answers the bytes the Rust dispatcher answers to the same requests, for services
//! that declare them and for one that declares none, alone and beside a second service on one
//! endpoint and over plain HTTP terms, and every answer is held to the official 2026-07-28 schema.

#![cfg(feature = "dart")]

use super::mcp_library_rows::{
    HttpAnswer, HttpRow, ORIGIN, Row, assert_answers, assert_http, bare_answer, depot_answer,
    http_rows, library_answer, rows, workspace_answer,
};
use super::mcp_library_service::{book_error_dart, book_info_dart, book_request_dart};
use super::mcp_shelf_service::{assert_faults, fault_requests, shelf_label_dart};
use super::run_dart_validate::{analyzed, dart};
use super::runtime::ran;
use super::{
    BareServiceSchema, DepotServiceSchema, LibraryServiceSchema, MirrorServiceSchema,
    ShelfServiceSchema,
};
use crate::mcp_streams::dart_respond;
use serde_json::{Value, json};

const RUNTIME_VAR: &str = "TIXSCHEMA_DART";

const DRIVER: &str = r#"
Never panicked() => throw 'the handler came apart';

final libraryHandlers = LibraryServiceHandlers<void>(
  getBook: (ctx, req) async {
    if (req.book_id == 'panic') panicked();
    if (req.book_id == 'book-1') return BookInfo(pages: 7);
    throw const BookErrorNotFound();
  },
  libraryGuide: (ctx, req) async => '# The library',
  bookCover: (ctx, bookId) async {
    if (bookId == 'book-1') return (Uint8List.fromList([37, 80, 68, 70, 45]), 'application/pdf');
    if (bookId == 'book-2') return (Uint8List.fromList([0, 255, 128, 1, 2, 3, 4]), 'image/png');
    if (bookId == 'panic') panicked();
    throw const BookErrorNotFound();
  },
  listBooks: (ctx, cursor) async {
    if (cursor == null) {
      return LibraryServiceMcpResourcePage(
        resources: [LibraryServiceMcpResource(uri: 'library://covers/book-1', name: 'cover-1.pdf', mimeType: 'application/pdf')],
        nextCursor: 'page-2',
      );
    }
    if (cursor == 'page-2') {
      return LibraryServiceMcpResourcePage(
        resources: [
          LibraryServiceMcpResource(
            uri: 'library://covers/book-2',
            name: 'cover-2.pdf',
            mimeType: 'application/pdf',
            size: 48213,
            title: 'Second cover',
          ),
        ],
      );
    }
    if (cursor == 'panic') panicked();
    throw const BookErrorNotFound();
  },
  suggestBooks: (ctx, req) async {
    if (req.typed == 'many') return [for (var at = 0; at < 150; at += 1) 'many-$at'];
    if (req.typed == 'panic') panicked();
    if (req.typed == 'none') throw const BookErrorNotFound();
    if (req.typed == 'kind') return [req.chosen['kind'] ?? ''];
    return ['book-1', 'book-2'].where((name) => name.startsWith(req.typed)).toList();
  },
);

final depotHandlers = DepotServiceHandlers<void>(
  getCrate: (ctx, crateId) async {
    if (crateId == 'crate-1') return '{"total":120}';
    throw const BookErrorNotFound();
  },
  getBin: (ctx, req) async => '${req.aisle}/${req.bin_id}',
  listCrates: (ctx, cursor) async {
    if (cursor != null) throw const BookErrorNotFound();
    return DepotServiceMcpResourcePage(
      resources: [DepotServiceMcpResource(uri: 'depot://crates/crate-1', name: 'crate-1', mimeType: 'application/json')],
    );
  },
);

final bareHandlers = BareServiceHandlers<void>(
  getPlain: (ctx, req) async {
    if (req.book_id == 'panic') panicked();
    if (req.book_id == 'book-1') return BookInfo(pages: 7);
    throw const BookErrorNotFound();
  },
);

const workspace = LibraryServiceMcpEndpoint(
  {...libraryServiceMcpNames, ...depotServiceMcpNames},
  [libraryServiceMcpCatalog, depotServiceMcpCatalog],
);

void main() async {
  final input = jsonDecode(INPUT) as Map<String, dynamic>;
  final libraryRoute = createLibraryServiceMcpRoute<void>(libraryHandlers);
  final depotRoute = createDepotServiceMcpRoute<void>(depotHandlers);
  Future<LibraryServiceMcpAnswer?> route(int part, String method, Map<String, dynamic> params) async {
    if (part == 0) return libraryRoute(null, method, params);
    if (part == 1) return depotRoute(null, method, params);
    return null;
  }

  final host = LibraryServiceMcpHost(writer: (message) async {}, inFlight: LibraryServiceMcpInFlight());
  final aloneLibrary = createLibraryServiceMcpDispatcher<void>(libraryHandlers);
  final aloneDepot = createDepotServiceMcpDispatcher<void>(depotHandlers);
  final aloneBare = createBareServiceMcpDispatcher<void>(bareHandlers);
  final requests = [for (final text in input['requests'] as List<dynamic>) text as String];
  Future<List<String?>> answers(Future<String?> Function(String text) ask) async =>
      [for (final text in requests) await ask(text)];

  final over = createLibraryServiceMcpHttpDispatcher<void>(
    libraryHandlers,
    LibraryServiceMcpHttpSettings(allowedOrigins: [input['origin'] as String]),
    (request, call) async => (ctx: null, denied: null, serverName: null),
  );
  final http = <Map<String, dynamic>>[];
  for (final held in input['http'] as List<dynamic>) {
    final row = held as Map<String, dynamic>;
    final response = await respondLibraryService(
      over,
      LibraryServiceMcpHttpRequest(
        method: 'POST',
        path: '/mcp',
        headers: [for (final pair in row['headers'] as List<dynamic>) ((pair as List<dynamic>)[0] as String, pair[1] as String)],
        body: utf8.encode(row['body'] as String),
      ),
    );
    http.add({
      'status': response.status,
      'headers': [for (final (name, value) in response.headers) [name, value]],
      'body': utf8.decode(response.body),
    });
  }
  print(jsonEncode({
    'names': [
      libraryServiceMcpNames.toList(),
      depotServiceMcpNames.toList(),
      bareServiceMcpNames.toList(),
    ],
    'workspace': await answers((text) => libraryServiceMcpServe<void>(text, 'Workspace', null, workspace, (call) {}, host, (ctx, part, method, params) => route(part, method, params))),
    'library': await answers((text) => aloneLibrary(text, (call) {}, host.writer, host.inFlight)),
    'depot': await answers((text) => aloneDepot(text, (call) {}, host.writer, DepotServiceMcpInFlight())),
    'bare': await answers((text) => aloneBare(text, (call) {}, host.writer, BareServiceMcpInFlight())),
    'http': http,
  }));
}
"#;

/// Answers the fault table's requests through the dispatcher of `ShelfService` alone and through
/// its route.
const FAULT_DRIVER: &str = "
final shelfHandlers = ShelfServiceHandlers<void>(
  getSlot: (ctx, req) async => '${req.aisle.value}/${req.slot}',
  getCount: (ctx, bin) async => '$bin',
  suggestSlots: (ctx, req) async => [req.typed.value],
);

void main() async {
  final requests = [for (final text in jsonDecode(INPUT) as List<dynamic>) text as String];
  final route = createShelfServiceMcpRoute<void>(shelfHandlers);
  final alone = createShelfServiceMcpDispatcher<void>(shelfHandlers);
  final host = ShelfServiceMcpHost(writer: (message) async {}, inFlight: ShelfServiceMcpInFlight());
  const endpoint = ShelfServiceMcpEndpoint({...shelfServiceMcpNames}, [shelfServiceMcpCatalog]);
  print(jsonEncode({
    'alone': [for (final text in requests) await alone(text, (call) {}, host.writer, host.inFlight)],
    'endpoint': [
      for (final text in requests)
        await shelfServiceMcpServe<void>(text, 'Workspace', null, endpoint, (call) {}, host, (ctx, part, method, params) => route(ctx, method, params)),
    ],
  }));
}
";

fn fault_module() -> String {
    let literal = Value::String(json!(fault_requests()).to_string());
    let dollar_safe = literal.to_string().replace('$', "\\$");
    [
        "import 'dart:async';".to_owned(),
        "import 'dart:convert';".to_owned(),
        shelf_label_dart::dart_definition(),
        book_error_dart::dart_definition(),
        ShelfServiceSchema::dart_definition(),
        ShelfServiceSchema::dart_ws_client(),
        ShelfServiceSchema::dart_mcp_service(),
        format!("const INPUT = {dollar_safe};"),
        FAULT_DRIVER.to_owned(),
    ]
    .join("\n\n")
}

fn input(requests: &[Row], http: &[HttpRow]) -> Value {
    json!({
        "origin": ORIGIN,
        "requests": requests.iter().map(|row| row.text.clone()).collect::<Vec<_>>(),
        "http": http.iter().map(|row| json!({
            "headers": row.headers.iter().map(|(name, value)| json!([name, value])).collect::<Vec<_>>(),
            "body": row.body,
        })).collect::<Vec<_>>(),
    })
}

fn module() -> String {
    let literal = Value::String(input(&rows(), &http_rows()).to_string());
    let dollar_safe = literal.to_string().replace('$', "\\$");
    [
        "import 'dart:async';".to_owned(),
        "import 'dart:convert';".to_owned(),
        "import 'dart:typed_data';".to_owned(),
        book_request_dart::dart_definition(),
        book_info_dart::dart_definition(),
        book_error_dart::dart_definition(),
        LibraryServiceSchema::dart_definition(),
        LibraryServiceSchema::dart_ws_client(),
        LibraryServiceSchema::dart_mcp_content(),
        LibraryServiceSchema::dart_mcp_service(),
        DepotServiceSchema::dart_definition(),
        DepotServiceSchema::dart_ws_client(),
        DepotServiceSchema::dart_mcp_content(),
        DepotServiceSchema::dart_mcp_service(),
        BareServiceSchema::dart_definition(),
        BareServiceSchema::dart_ws_client(),
        BareServiceSchema::dart_mcp_service(),
        dart_respond("LibraryService"),
        format!("const INPUT = {dollar_safe};"),
        DRIVER.to_owned(),
    ]
    .join("\n\n")
}

/// The report of the one Dart run; `None` where no Dart toolchain was reachable.
fn driven() -> Option<Value> {
    let wrote = ran("dart", RUNTIME_VAR, "dart", "mcp-resources.dart", &module())?;
    Some(serde_json::from_str(wrote.trim()).unwrap())
}

fn texts(report: &Value) -> Vec<Option<String>> {
    report
        .as_array()
        .unwrap()
        .iter()
        .map(|answered| answered.as_str().map(str::to_owned))
        .collect()
}

#[test]
fn the_dart_server_answers_the_bytes_the_rust_dispatcher_answers() {
    let Some(report) = driven() else {
        return;
    };
    let rows = rows();
    assert_answers(
        "endpoint",
        workspace_answer,
        &texts(&report["workspace"]),
        &rows,
    );
    assert_answers("library", library_answer, &texts(&report["library"]), &rows);
    assert_answers("depot", depot_answer, &texts(&report["depot"]), &rows);
    assert_answers("bare", bare_answer, &texts(&report["bare"]), &rows);
}

#[test]
fn the_dart_server_answers_resource_requests_over_plain_http_terms_as_rust_does() {
    let Some(report) = driven() else {
        return;
    };
    let answered: Vec<HttpAnswer> = report["http"]
        .as_array()
        .unwrap()
        .iter()
        .map(|held| {
            (
                u16::try_from(held["status"].as_u64().unwrap()).unwrap(),
                held["headers"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|pair| {
                        (
                            pair[0].as_str().unwrap().to_owned(),
                            pair[1].as_str().unwrap().to_owned(),
                        )
                    })
                    .collect(),
                held["body"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    assert_http("library", &http_rows(), &answered);
}

#[test]
fn the_resource_text_passes_the_dart_analyzer() {
    analyzed("mcp-resources-analyze", &module());
}

#[test]
fn an_endpoint_names_a_resource_and_a_template_by_their_kind() {
    let Some(report) = driven() else {
        return;
    };
    assert_eq!(
        report["names"],
        json!([
            [
                "resource template:library://covers/{book_id}",
                "tool:get-book",
                "resource:guide://library"
            ],
            [
                "resource template:depot://aisles/{aisle}/bins/{bin_id}",
                "resource template:depot://crates/{crate_id}"
            ],
            ["tool:get-plain"],
        ])
    );
}

#[test]
fn a_const_endpoint_whose_services_share_a_resource_does_not_compile() {
    let endpoint = |second: &str| {
        [
            "import 'dart:async';".to_owned(),
            "import 'dart:convert';".to_owned(),
            "import 'dart:typed_data';".to_owned(),
            book_request_dart::dart_definition(),
            book_info_dart::dart_definition(),
            book_error_dart::dart_definition(),
            LibraryServiceSchema::dart_definition(),
            LibraryServiceSchema::dart_ws_client(),
            LibraryServiceSchema::dart_mcp_content(),
            LibraryServiceSchema::dart_mcp_service(),
            DepotServiceSchema::dart_definition(),
            DepotServiceSchema::dart_ws_client(),
            DepotServiceSchema::dart_mcp_content(),
            DepotServiceSchema::dart_mcp_service(),
            MirrorServiceSchema::dart_definition(),
            MirrorServiceSchema::dart_ws_client(),
            MirrorServiceSchema::dart_mcp_service(),
            format!(
                "const workspace = LibraryServiceMcpEndpoint(\n  \
                 {{...libraryServiceMcpNames, ...{second}ServiceMcpNames}},\n  \
                 [libraryServiceMcpCatalog, {second}ServiceMcpCatalog],\n);\n\n\
                 void main() {{\n  print(workspace.names.length);\n}}"
            ),
        ]
        .join("\n\n")
    };
    let Some((distinct, said)) = dart("resource-endpoint-distinct", &[], &endpoint("depot")) else {
        return;
    };
    assert!(distinct, "{said}");
    assert_eq!(said.trim(), "5");
    let (shared, refused) = dart("resource-endpoint-shared", &[], &endpoint("mirror")).unwrap();
    assert!(!shared, "{refused}");
    assert!(
        refused.contains(
            "The element '\"resource template:library://covers/{book_id}\"' conflicts with another existing element in the set"
        ),
        "{refused}"
    );
}

#[test]
fn a_resource_read_and_a_completion_the_dart_reads_refuse_answer_the_fault_the_rust_dispatcher_answers()
 {
    let Some(wrote) = ran(
        "dart",
        RUNTIME_VAR,
        "dart",
        "mcp-resource-faults.dart",
        &fault_module(),
    ) else {
        return;
    };
    let report: Value = serde_json::from_str(wrote.trim()).unwrap();
    assert_faults("alone", &texts(&report["alone"]));
    assert_faults("endpoint", &texts(&report["endpoint"]));
}
