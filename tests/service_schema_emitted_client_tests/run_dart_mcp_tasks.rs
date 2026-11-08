//! The emitted Dart `mcp` server and client run by the Dart VM on `JobService`: the server's
//! answer to each task request, HTTP request, scope query and `subscriptions/listen` is the one
//! the Rust dispatcher gives, and the client reads the handle and the state of a task back as
//! ordinary successes.

#![cfg(feature = "dart")]

use super::JobServiceSchema;
use super::mcp_task_rows::{
    assert_client, assert_matches, client_input, export_report_dart, export_request_dart, input,
    job_error_dart,
};
use super::run_dart_validate::analyzed;
use super::runtime::ran;
use crate::mcp_streams::dart_respond;
use serde_json::Value;

const RUNTIME_VAR: &str = "TIXSCHEMA_DART";

const SERVER_DRIVER: &str = "
final reached = <Object?>[];

McpTask task(String id, [String updated = '2026-11-01T20:00:00Z']) => McpTask(
      taskId: id,
      createdAt: '2026-11-01T20:00:00Z',
      lastUpdatedAt: updated,
      ttlMs: 3600000,
      pollIntervalMs: 2000,
    );

final handlers = JobServiceHandlers<void>(
  startExport: (ctx, req) async {
    reached.add(<Object?>['start-export', req.folder]);
    if (req.folder == 'missing') throw const JobErrorNotFound();
    return task('7f3a');
  },
  exportStatus: (ctx, req) async {
    reached.add(<Object?>['export-status', req]);
    if (req == 'boom') throw 'the handler came apart';
    switch (req) {
      case '7f3a':
        return McpTaskWorking<ExportReport>(task(req));
      case 'done':
        return McpTaskCompleted<ExportReport>(task(req, '2026-11-01T20:04:10Z'), const ExportReport(files: 3));
      case 'needs':
        return McpTaskInputRequired<ExportReport>(task(req), <String, dynamic>{
          'overwrite': <String, dynamic>{
            'method': 'elicitation/create',
            'params': <String, dynamic>{
              'mode': 'form',
              'message': 'Overwrite the folder?',
              'requestedSchema': <String, dynamic>{'type': 'object', 'properties': <String, dynamic>{}},
            },
          },
        });
      case 'broken':
        return McpTaskFailed<ExportReport>(task(req), const McpTaskError(code: -32603, message: 'the folder vanished'));
      case 'stopped':
        return McpTaskCancelled<ExportReport>(task(req));
      default:
        throw const JobErrorNotFound();
    }
  },
  cancelExport: (ctx, req) async {
    reached.add(<Object?>['cancel-export', req]);
    if (req == 'gone') throw const JobErrorNotFound();
  },
  answerExport: (ctx, req) async {
    reached.add(<Object?>['answer-export', req.task_id, req.key, req.action, req.content]);
    if (req.task_id == 'gone') throw const JobErrorNotFound();
  },
);

JobServiceMcpChanges changesOf(List<dynamic> script) {
  final queue = [
    for (final spec in script)
      (spec as Map<String, dynamic>)['kind'] == 'task'
          ? JobServiceMcpTaskMoved(spec['taskId'] as String, spec['state'] as Map<String, dynamic>) as JobServiceMcpChange
          : const JobServiceMcpKeepAlive(),
  ];
  return () async => queue.isEmpty ? null : queue.removeAt(0);
}

void main() async {
  final input = jsonDecode(inputText) as Map<String, dynamic>;
  final dispatch = createJobServiceMcpDispatcher<void>(handlers);
  final inFlight = JobServiceMcpInFlight();
  final http = createJobServiceMcpHttpDispatcher<void>(
    handlers,
    const JobServiceMcpHttpSettings(allowedOrigins: []),
    (request, call) async => (ctx: null, denied: null),
  );
  final answers = <String, dynamic>{};
  final calls = <String, dynamic>{};
  for (final entry in (input['requests'] as Map<String, dynamic>).entries) {
    reached.clear();
    answers[entry.key] = await dispatch(entry.value as String, (call) {}, (message) async {}, inFlight);
    calls[entry.key] = List<Object?>.of(reached);
  }
  reached.clear();
  final answered = <String, dynamic>{};
  for (final row in (input['http'] as List).cast<Map<String, dynamic>>()) {
    final response = await respondJobService(http, JobServiceMcpHttpRequest(
      method: 'POST',
      path: '/mcp',
      headers: [for (final pair in row['headers'] as List) ((pair as List)[0] as String, pair[1] as String)],
      body: utf8.encode(row['body'] as String),
    ));
    answered[row['name'] as String] = {'status': response.status, 'body': utf8.decode(response.body)};
  }
  final scopes = [
    for (final query in input['queries'] as List)
      jobServiceMcpScopes((query as List)[0] as String, query[1] as String?),
  ];
  final listens = <Object?>[];
  for (final row in (input['listens'] as List).cast<Map<String, dynamic>>()) {
    final written = <String>[];
    final reply = await dispatch(row['request'] as String, (call) {}, (message) async => written.add(message),
        JobServiceMcpInFlight(), changesOf(row['script'] as List<dynamic>));
    listens.add({'reply': reply, 'written': written});
  }
  print(jsonEncode({'answers': answers, 'reached': calls, 'http': answered, 'scopes': scopes, 'listens': listens}));
}
";

const CLIENT_DRIVER: &str = "
final seen = <Object?>[];
final queue = <String>[];

class Canned implements JobServiceMcpTransport {
  @override
  Future<String> send(({String text, String protocolVersion, String method, String? name}) message) async {
    seen.add(jsonDecode(message.text));
    return queue.removeAt(0);
  }
}

Object? handle(McpTask task) => <String, dynamic>{...task.toJson()}..remove('status');

Object? started(JobServiceStartExportResult result) => switch (result) {
      JobServiceStartExportResultOk(:final value) => <String, dynamic>{'ok': handle(value)},
      JobServiceStartExportResultOperation(:final error) => <String, dynamic>{'declared': error.toJson()},
      JobServiceStartExportResultFault(:final fault) => <String, dynamic>{'fault': fault.kind.wireValue},
    };

Object? status(JobServiceExportStatusResult result) => switch (result) {
      JobServiceExportStatusResultOk(:final value) => <String, dynamic>{'ok': value.toJson((e) => e.toJson())},
      JobServiceExportStatusResultOperation(:final error) => <String, dynamic>{'declared': error.toJson()},
      JobServiceExportStatusResultFault(:final fault) => <String, dynamic>{'fault': fault.kind.wireValue},
    };

Object? cancelled(JobServiceCancelExportResult result) => switch (result) {
      JobServiceCancelExportResultOk() => <String, dynamic>{'ok': null},
      JobServiceCancelExportResultOperation(:final error) => <String, dynamic>{'declared': error.toJson()},
      JobServiceCancelExportResultFault(:final fault) => <String, dynamic>{'fault': fault.kind.wireValue},
    };

Object? answered(JobServiceAnswerExportResult result) => switch (result) {
      JobServiceAnswerExportResultOk() => <String, dynamic>{'ok': null},
      JobServiceAnswerExportResultOperation(:final error) => <String, dynamic>{'declared': error.toJson()},
      JobServiceAnswerExportResultFault(:final fault) => <String, dynamic>{'fault': fault.kind.wireValue},
    };

void main() async {
  final input = jsonDecode(inputText) as Map<String, dynamic>;
  final client = JobServiceMcpClient(Canned());
  final calls = <String, Future<Object?> Function()>{
    'started': () async => started(await client.startExport(const ExportRequest(folder: 'reports'))),
    'working': () async => status(await client.exportStatus('7f3a')),
    'completed': () async => status(await client.exportStatus('done')),
    'waiting': () async => status(await client.exportStatus('needs')),
    'failed': () async => status(await client.exportStatus('broken')),
    'cancelled': () async => status(await client.exportStatus('stopped')),
    'cancel': () async => cancelled(await client.cancelExport('7f3a')),
    'answer': () async => answered(await client.answerExport(
        const AnswerExportRequest(task_id: 'needs', key: 'overwrite', action: 'accept', content: '{}'))),
    'declared': () async => status(await client.exportStatus('gone')),
    'panicked': () async => status(await client.exportStatus('boom')),
  };
  final outcomes = <String, Object?>{};
  for (final call in (input['calls'] as List).cast<Map<String, dynamic>>()) {
    queue.add(call['answer'] as String);
    final label = call['label'] as String;
    outcomes[label] = await calls[label]!();
  }
  print(jsonEncode({'outcomes': outcomes, 'seen': seen}));
}
";

/// The imports, the models, the service and its server and client, and `driver`.
fn module(driver: &str, given: &Value) -> String {
    [
        "import 'dart:async';".to_owned(),
        "import 'dart:convert';".to_owned(),
        job_error_dart::dart_definition(),
        export_request_dart::dart_definition(),
        export_report_dart::dart_definition(),
        JobServiceSchema::dart_definition(),
        JobServiceSchema::dart_mcp_content(),
        JobServiceSchema::dart_ws_client(),
        JobServiceSchema::dart_mcp_service(),
        JobServiceSchema::dart_mcp_client(),
        dart_respond("JobService"),
        format!("const inputText = r'''{given}''';"),
        driver.to_owned(),
    ]
    .join("\n\n")
}

fn driven(named: &str, driver: &str, given: &Value) -> Option<Value> {
    let wrote = ran(
        "dart",
        RUNTIME_VAR,
        "dart",
        &format!("{named}.dart"),
        &module(driver, given),
    )?;
    Some(serde_json::from_str(wrote.trim()).unwrap())
}

#[test]
fn the_dart_server_answers_the_bytes_the_rust_dispatcher_answers() {
    let Some(report) = driven("mcp_tasks_server", SERVER_DRIVER, &input()) else {
        return;
    };
    assert_matches(&report);
}

#[test]
fn the_dart_client_reads_the_handle_and_the_state_the_rust_dispatcher_answers() {
    let Some(report) = driven("mcp_tasks_client", CLIENT_DRIVER, &client_input()) else {
        return;
    };
    assert_client(&report);
}

#[test]
fn the_dart_text_of_the_job_service_passes_the_analyzer() {
    analyzed("mcp-tasks-server-analyze", &module(SERVER_DRIVER, &input()));
    analyzed(
        "mcp-tasks-client-analyze",
        &module(CLIENT_DRIVER, &client_input()),
    );
}
