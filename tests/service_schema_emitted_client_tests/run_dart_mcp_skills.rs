//! The emitted Dart `mcp` server run by the Dart VM over skills: a folder named on the service and
//! two skills an operation writes at run time. It answers the bytes the Rust dispatcher answers to
//! the same requests, alone and on an endpoint and over plain HTTP terms, and every answer is held
//! to the official 2026-07-28 schema.

#![cfg(feature = "dart")]

use super::LessonServiceSchema;
use super::mcp_lesson_service::{assert_http, http_rows, lesson_answer, rows};
use super::mcp_library_rows::{HttpAnswer, ORIGIN, assert_answers};
use super::mcp_library_service::{book_error_dart, book_info_dart, book_request_dart};
use super::run_dart_validate::analyzed;
use super::runtime::ran;
use crate::mcp_streams::dart_respond;
use serde_json::{Value, json};

const RUNTIME_VAR: &str = "TIXSCHEMA_DART";

const DRIVER: &str = r"
Never panicked() => throw 'the handler came apart';

final lessonHandlers = LessonServiceHandlers<void>(
  releaseNotesSkill: (ctx, path) async {
    if (path == 'SKILL.md') return '# Release notes\n\nRead the changes since 2026-11-01, then write one line per change.\n';
    if (path == 'references/changes.md') return '- renamed a document\n';
    if (path == 'panic') panicked();
    throw const BookErrorNotFound();
  },
  figuresSkill: (ctx, path) async {
    if (path == 'SKILL.md') return (Uint8List.fromList(utf8.encode('# Figures\n')), 'text/markdown');
    if (path == 'figures.bin') return (Uint8List.fromList([0, 255, 128, 1]), 'application/octet-stream');
    throw const BookErrorNotFound();
  },
  getLesson: (ctx, req) async {
    if (req.book_id == 'lesson-1') return BookInfoDraft(pages: 3).validate();
    throw const BookErrorNotFound();
  },
);

void main() async {
  final input = jsonDecode(INPUT) as Map<String, dynamic>;
  final route = createLessonServiceMcpRoute<void>(lessonHandlers);
  const endpoint = LessonServiceMcpEndpoint({...lessonServiceMcpNames}, [lessonServiceMcpCatalog]);
  final host = LessonServiceMcpHost(writer: (message) async {}, inFlight: LessonServiceMcpInFlight());
  final alone = createLessonServiceMcpDispatcher<void>(lessonHandlers);
  final requests = [for (final text in input['requests'] as List<dynamic>) text as String];
  Future<List<String?>> answers(Future<String?> Function(String text) ask) async =>
      [for (final text in requests) await ask(text)];
  final over = createLessonServiceMcpHttpDispatcher<void>(
    lessonHandlers,
    LessonServiceMcpHttpSettings(allowedOrigins: [input['origin'] as String]),
    (request, call) async => (ctx: null, denied: null, serverName: null),
  );
  final http = <Map<String, dynamic>>[];
  for (final held in input['http'] as List<dynamic>) {
    final row = held as Map<String, dynamic>;
    final response = await respondLessonService(
      over,
      LessonServiceMcpHttpRequest(
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
    'names': lessonServiceMcpNames.toList(),
    'endpoint': await answers((text) => lessonServiceMcpServe<void>(text, lessonServiceMcpServerName, lessonServiceMcpInstructions, endpoint, (call) {}, host, (ctx, part, method, params) => route(ctx, method, params))),
    'alone': await answers((text) => alone(text, (call) {}, host.writer, host.inFlight)),
    'http': http,
  }));
}
";

fn module() -> String {
    let requests: Vec<String> = rows().into_iter().map(|row| row.text).collect();
    let http: Vec<Value> = http_rows()
        .into_iter()
        .map(|row| {
            json!({
                "headers": row.headers.iter().map(|(name, value)| json!([name, value])).collect::<Vec<_>>(),
                "body": row.body,
            })
        })
        .collect();
    let input = json!({ "origin": ORIGIN, "requests": requests, "http": http });
    let literal = Value::String(input.to_string());
    let dollar_safe = literal.to_string().replace('$', "\\$");
    [
        "import 'dart:async';".to_owned(),
        "import 'dart:convert';".to_owned(),
        "import 'dart:typed_data';".to_owned(),
        book_request_dart::dart_definition(),
        book_info_dart::dart_definition(),
        book_error_dart::dart_definition(),
        LessonServiceSchema::dart_definition(),
        LessonServiceSchema::dart_ws_client(),
        LessonServiceSchema::dart_mcp_service(),
        dart_respond("LessonService"),
        format!("const INPUT = {dollar_safe};"),
        DRIVER.to_owned(),
    ]
    .join("\n\n")
}

/// The report of the one Dart run; `None` where no Dart toolchain was reachable.
fn driven() -> Option<Value> {
    let wrote = ran("dart", RUNTIME_VAR, "dart", "mcp-skills.dart", &module())?;
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
    assert_answers("alone", lesson_answer, &texts(&report["alone"]), &rows);
    assert_answers(
        "endpoint",
        lesson_answer,
        &texts(&report["endpoint"]),
        &rows,
    );
}

#[test]
fn an_endpoint_names_every_skill_as_the_rust_registry_does() {
    let Some(report) = driven() else {
        return;
    };
    assert_eq!(
        report["names"],
        json!(
            LessonServiceSchema::MCP_NAMES
                .iter()
                .map(|(kind, name)| format!("{kind}:{name}"))
                .collect::<Vec<_>>()
        )
    );
}

#[test]
fn the_dart_server_answers_the_skills_over_plain_http_terms_as_rust_does() {
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
    assert_http("dart", &answered);
}

#[test]
fn the_skills_text_passes_the_dart_analyzer() {
    analyzed("mcp-skills-analyze", &module());
}
