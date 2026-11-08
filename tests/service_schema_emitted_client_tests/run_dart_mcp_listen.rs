//! The emitted Dart `mcp` server run by the Dart VM over `subscriptions/listen`: it writes and
//! answers the bytes the Rust dispatcher writes and answers to the same requests, handed the same
//! changes, on stdio, on an endpoint of two services and over plain HTTP terms, and every message
//! is held to the official 2026-07-28 schema. Each stream ends on the events it observes.

#![cfg(feature = "dart")]

use super::mcp_library_service::{book_error_dart, book_info_dart, book_request_dart};
use super::mcp_listen_rows::{assert_matches, input};
use super::run_dart_validate::analyzed;
use super::runtime::ran;
use super::{DepotServiceSchema, LibraryServiceSchema};
use crate::mcp_streams::dart_respond;
use serde_json::Value;

const RUNTIME_VAR: &str = "TIXSCHEMA_DART";

const DRIVER: &str = "
class RecordingStream implements LibraryServiceMcpHttpStream {
  RecordingStream([this.goneAfter]);
  final int? goneAfter;
  final List<Map<String, dynamic>> seen = <Map<String, dynamic>>[];
  final List<Completer<void>> waiting = <Completer<void>>[];

  void wake() {
    final woken = waiting.toList();
    waiting.clear();
    for (final one in woken) {
      one.complete();
    }
  }

  Future<void> until(bool Function() done) async {
    while (!done()) {
      final next = Completer<void>();
      waiting.add(next);
      await next.future;
    }
  }

  void take() {
    final refused = goneAfter != null && seen.length >= goneAfter!;
    if (refused) throw StateError('the client has gone');
  }

  @override
  Future<void> begin(int status, List<(String, String)> headers) async {
    take();
    seen.add({
      'began': [status, [for (final (name, value) in headers) [name, value]]],
    });
    wake();
  }

  @override
  Future<void> write(String event) async {
    take();
    seen.add({'wrote': event});
    wake();
  }
}

class Feed {
  final List<LibraryServiceMcpChange> pending = <LibraryServiceMcpChange>[];
  Completer<LibraryServiceMcpChange?>? waiting;
  bool closed = false;

  void settle() {
    final next = waiting;
    if (next == null) return;
    if (pending.isNotEmpty) {
      waiting = null;
      next.complete(pending.removeAt(0));
    } else if (closed) {
      waiting = null;
      next.complete(null);
    }
  }

  Future<LibraryServiceMcpChange?> next() {
    final next = Completer<LibraryServiceMcpChange?>();
    waiting = next;
    settle();
    return next.future;
  }

  void push(LibraryServiceMcpChange change) {
    pending.add(change);
    settle();
  }

  void close() {
    closed = true;
    settle();
  }
}

final libraryHandlers = LibraryServiceHandlers<void>(
  getBook: (ctx, req) async => throw const BookErrorNotFound(),
  libraryGuide: (ctx, req) async => '# The library',
  bookCover: (ctx, bookId) async => throw const BookErrorNotFound(),
  listBooks: (ctx, cursor) async => throw const BookErrorNotFound(),
  suggestBooks: (ctx, req) async => throw const BookErrorNotFound(),
);

final depotHandlers = DepotServiceHandlers<void>(
  getCrate: (ctx, crateId) async => throw const BookErrorNotFound(),
  getBin: (ctx, req) async => '${req.aisle}/${req.bin_id}',
  listCrates: (ctx, cursor) async => throw const BookErrorNotFound(),
);

const workspace = LibraryServiceMcpEndpoint(
  {...libraryServiceMcpNames, ...depotServiceMcpNames},
  [libraryServiceMcpCatalog, depotServiceMcpCatalog],
);

LibraryServiceMcpChange changeOf(Map<String, dynamic> spec) => switch (spec['kind']) {
      'resourcesListChanged' => const LibraryServiceMcpResourcesListChanged(),
      'resourceUpdated' => LibraryServiceMcpResourceUpdated(spec['uri'] as String),
      'task' => LibraryServiceMcpTaskMoved(spec['taskId'] as String, spec['state'] as Map<String, dynamic>),
      _ => const LibraryServiceMcpKeepAlive(),
    };

LibraryServiceMcpChanges changesOf(List<dynamic> script) {
  final queue = [for (final spec in script) changeOf(spec as Map<String, dynamic>)];
  return () async => queue.isEmpty ? null : queue.removeAt(0);
}

({List<String> written, Future<void> Function(String) write}) lines() {
  final written = <String>[];
  return (written: written, write: (message) async => written.add(message));
}

Map<String, dynamic> httpReport(LibraryServiceMcpHttpResponse response) => {
      'status': response.status,
      'headers': [for (final (name, value) in response.headers) [name, value]],
      'body': utf8.decode(response.body),
    };

LibraryServiceMcpHttpRequest httpRequest(Map<String, dynamic> row) => LibraryServiceMcpHttpRequest(
      method: row['method'] as String,
      path: row['path'] as String,
      headers: [for (final pair in row['headers'] as List<dynamic>) ((pair as List<dynamic>)[0] as String, pair[1] as String)],
      body: utf8.encode(row['body'] as String),
    );

void main() async {
  final input = jsonDecode(INPUT) as Map<String, dynamic>;
  final dispatch = createLibraryServiceMcpDispatcher<void>(libraryHandlers);
  final libraryRoute = createLibraryServiceMcpRoute<void>(libraryHandlers);
  final depotRoute = createDepotServiceMcpRoute<void>(depotHandlers);
  void open(LibraryServiceMcpCall call) {}
  final listens = input['listens'] as List<dynamic>;

  final stdio = <Map<String, dynamic>>[];
  for (final held in listens) {
    final row = held as Map<String, dynamic>;
    final out = lines();
    final reply = await dispatch(row['request'] as String, open, out.write, LibraryServiceMcpInFlight(), changesOf(row['script'] as List<dynamic>));
    stdio.add({'reply': reply, 'written': out.written});
  }

  final first = listens[0] as Map<String, dynamic>;
  final out = lines();
  final served = await libraryServiceMcpServe<void>(
    first['request'] as String,
    'Workspace',
    null,
    workspace,
    open,
    LibraryServiceMcpHost(writer: out.write, inFlight: LibraryServiceMcpInFlight(), changes: changesOf(first['script'] as List<dynamic>)),
    (ctx, part, method, params) async =>
        part == 0 ? libraryRoute(null, method, params) : part == 1 ? depotRoute(null, method, params) : null,
  );
  final workspaceReport = {'reply': served, 'written': out.written};

  const settings = LibraryServiceMcpHttpSettings(allowedOrigins: []);
  final http = <Map<String, dynamic>>[];
  for (final held in input['http'] as List<dynamic>) {
    final row = held as Map<String, dynamic>;
    final over = createLibraryServiceMcpHttpDispatcher<void>(
      libraryHandlers,
      settings,
      (request, call) async => (ctx: null, denied: null),
      changesOf(row['script'] as List<dynamic>),
    );
    http.add(httpReport(await respondLibraryService(over, httpRequest(row))));
  }

  final firstHttp = (input['http'] as List<dynamic>)[0] as Map<String, dynamic>;
  final plainOver = createLibraryServiceMcpHttpDispatcher<void>(
    libraryHandlers,
    settings,
    (request, call) async => (ctx: null, denied: null),
  );
  final plainResponse = await respondLibraryService(plainOver, httpRequest(firstHttp));
  final plainReply = await dispatch(first['request'] as String, open, lines().write, LibraryServiceMcpInFlight());

  final listening = await dispatch(input['discover'] as String, open, lines().write, LibraryServiceMcpInFlight(), changesOf([]));
  final plainDiscover = await dispatch(input['discover'] as String, open, lines().write, LibraryServiceMcpInFlight());

  final cancelLines = input['cancel'] as List<dynamic>;
  final cancelWritten = <String>[];
  final firstWritten = Completer<void>();
  Future<void> cancelOut(String message) async {
    cancelWritten.add(message);
    if (!firstWritten.isCompleted) firstWritten.complete();
  }

  final inFlight = LibraryServiceMcpInFlight();
  final pending = dispatch(cancelLines[0] as String, open, cancelOut, inFlight, () => Completer<LibraryServiceMcpChange?>().future);
  await firstWritten.future;
  final start = cancelWritten.length;
  final stopped = await dispatch(cancelLines[1] as String, open, cancelOut, inFlight);
  if (stopped != null) throw 'a notification was answered';
  final cancelledReply = await pending;

  final gone = input['gone'] as Map<String, dynamic>;
  final goneReply = await dispatch(
    gone['request'] as String,
    open,
    (message) async => throw 'the client has gone',
    LibraryServiceMcpInFlight(),
    changesOf(gone['script'] as List<dynamic>),
  );

  final streamedRow = input['streamed'] as Map<String, dynamic>;
  final streamedRequest = httpRequest(streamedRow);
  final streams = <String, dynamic>{};
  {
    final stream = RecordingStream();
    final feed = Feed();
    final over = createLibraryServiceMcpHttpDispatcher<void>(
      libraryHandlers,
      settings,
      (request, call) async => (ctx: null, denied: null),
      feed.next,
    );
    var ended = false;
    final running = over(streamedRequest, stream).then((answered) {
      ended = true;
      return answered;
    });
    final snapshots = <List<Map<String, dynamic>>>[];
    await stream.until(() => stream.seen.length >= 2);
    snapshots.add(stream.seen.toList());
    var expected = 2;
    for (final change in <LibraryServiceMcpChange>[
      const LibraryServiceMcpResourcesListChanged(),
      const LibraryServiceMcpResourceUpdated('library://covers/book-1'),
      const LibraryServiceMcpKeepAlive(),
    ]) {
      feed.push(change);
      expected += 1;
      await stream.until(() => stream.seen.length >= expected);
      snapshots.add(stream.seen.toList());
    }
    final endedWhileOpen = ended;
    feed.close();
    final answered = await running;
    streams['ordered'] = {
      'endedWhileOpen': endedWhileOpen,
      'full': stream.seen.toList(),
      'returned': answered != null,
      'snapshots': snapshots,
    };
  }
  {
    final stream = RecordingStream(1);
    LibraryServiceMcpCall? taken;
    final over = createLibraryServiceMcpHttpDispatcher<void>(
      libraryHandlers,
      settings,
      (request, call) async {
        taken = call;
        return (ctx: null, denied: null);
      },
      () => Completer<LibraryServiceMcpChange?>().future,
    );
    final answered = await over(streamedRequest, stream);
    streams['gone'] = {'cancelled': taken!.cancelled, 'returned': answered != null, 'seen': stream.seen.toList()};
  }

  print(jsonEncode({
    'streams': streams,
    'stdio': stdio,
    'workspace': workspaceReport,
    'http': http,
    'plain': {'http': httpReport(plainResponse), 'stdio': plainReply},
    'discover': {'listening': listening, 'plain': plainDiscover},
    'cancelled': {'reply': cancelledReply, 'running': 0, 'start': start, 'written': cancelWritten},
    'gone': {'reply': goneReply, 'written': <String>[]},
  }));
}
";

fn module() -> String {
    let literal = Value::String(input().to_string());
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
        dart_respond("LibraryService"),
        format!("const INPUT = {dollar_safe};"),
        DRIVER.to_owned(),
    ]
    .join("\n\n")
}

/// The report of the one Dart run; `None` where no Dart toolchain was reachable.
fn driven() -> Option<Value> {
    let wrote = ran("dart", RUNTIME_VAR, "dart", "mcp-listen.dart", &module())?;
    Some(serde_json::from_str(wrote.trim()).unwrap())
}

#[test]
fn the_dart_server_writes_and_answers_what_the_rust_dispatcher_does_on_a_listen() {
    let Some(report) = driven() else {
        return;
    };
    assert_matches(&report);
}

#[test]
fn the_listen_text_passes_the_dart_analyzer() {
    analyzed("mcp-listen-analyze", &module());
}
