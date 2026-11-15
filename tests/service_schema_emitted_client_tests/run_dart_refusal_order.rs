//! The `ws_rpc` attachment and client and the `mcp` server of `HoistService` run by the Dart VM: a
//! message declared above the service, below it and in another crate is refused with one issue per
//! broken rule, each at its own field, as Rust's own read lists them.

#![cfg(feature = "dart")]

use super::mcp_library_rows::request;
use super::refusal_order_service::{BROKEN_LOAD, HoistServiceSchema, OPERATIONS, broken_issues};
use super::refusal_order_service::{
    above_item_dart, above_load_dart, below_item_dart, below_load_dart, below_renamed_dart,
    hoist_error_dart, shipment_mcp_task_dart,
};
use super::run_dart_validate::analyzed;
use super::runtime::ran;
use refusal_order_apart::{apart_item_dart, apart_load_dart, apart_renamed_dart};
use serde_json::{Value, json};

const RUNTIME_VAR: &str = "TIXSCHEMA_DART";

/// Hands each operation a broken message through the attachment, the `mcp` server and the client.
const DRIVER: &str = "
final broken = jsonDecode(brokenText) as Map<String, dynamic>;

HoistServiceHandlers<void> handlers() => HoistServiceHandlers<void>(
  above: (ctx, load) async => load,
  apart: (ctx, load) async => load,
  below: (ctx, load) async => load,
  moved: (ctx, load) async => load,
  shifted: (ctx, load) async => load,
  tasked: (ctx, load) async => load,
);

Future<List<Object?>> attached() async {
  final written = <Map<String, dynamic>>[];
  final served = StreamController<Map<String, dynamic>>();
  final settled = Completer<void>();
  final detach = attachHoistServiceWsDispatcher<void>(
    (inbound: served.stream, send: (reply) { written.add(reply); if (written.length == 6) settled.complete(); }),
    null,
    handlers(),
    onFault: (fault) {},
  );
  for (final operation in ['above', 'apart', 'below', 'moved', 'shifted', 'tasked']) {
    served.add(<String, dynamic>{
      'kind': 'request',
      'id': operation,
      'service': 'HoistService',
      'operation': operation,
      'payload': broken,
    });
  }
  await settled.future;
  detach();
  return written;
}

Future<Map<String, Object?>> served() async {
  final dispatch = createHoistServiceMcpDispatcher<void>(handlers());
  final inFlight = HoistServiceMcpInFlight();
  final answers = <String, Object?>{};
  for (final entry in (jsonDecode(mcpRequests) as Map<String, dynamic>).entries) {
    answers[entry.key] = await dispatch(entry.value as String, (call) {}, (message) async {}, inFlight);
  }
  return answers;
}

Future<Map<String, Object?>> read() async {
  final results = <String, Object?>{};
  for (final operation in ['above', 'apart', 'below', 'moved', 'shifted', 'tasked']) {
    final sent = <dynamic>[];
    final outbound = StreamController<dynamic>();
    outbound.stream.listen(sent.add);
    final inbound = StreamController<dynamic>();
    final client = HoistServiceWsClient(HoistServiceWsTransport(
      sink: outbound.sink,
      stream: inbound.stream,
      heartbeat: HoistServiceWsHeartbeat.off(),
    ));
    final load = switch (operation) {
      'above' => client.above(AboveLoadDraft(items: [], label: 'a', weight: 1).validate()),
      'apart' => client.apart(ApartLoadDraft(items: [], label: 'a', weight: 1).validate()),
      'below' => client.below(BelowLoadDraft(items: [], label: 'a', weight: 1).validate()),
      'moved' => client.moved(PublishedBelowDraft(items: [], label: 'a', weight: 1).validate()),
      'shifted' => client.shifted(PublishedApartDraft(items: [], label: 'a', weight: 1).validate()),
      _ => client.tasked(ShipmentMcpTaskDraft(items: [], label: 'a', weight: 1).validate()),
    };
    await Future<void>.delayed(Duration.zero);
    final id = (jsonDecode(sent.last as String) as Map<String, dynamic>)['id'];
    inbound.add(jsonEncode(<String, dynamic>{
      'kind': 'reply',
      'id': id,
      'service': 'HoistService',
      'ok': true,
      'value': broken,
    }));
    final fault = switch (await load) {
      HoistServiceAboveResultFault(:final fault) => fault,
      HoistServiceApartResultFault(:final fault) => fault,
      HoistServiceBelowResultFault(:final fault) => fault,
      HoistServiceMovedResultFault(:final fault) => fault,
      HoistServiceShiftedResultFault(:final fault) => fault,
      HoistServiceTaskedResultFault(:final fault) => fault,
      _ => null,
    };
    results[operation] = fault == null
        ? null
        : <String, Object?>{
            'kind': fault.kind.name,
            'issues': [for (final issue in fault.issues ?? []) <String, Object?>{'detail': issue.detail, 'field': issue.field}],
          };
  }
  return results;
}

void main() async {
  print(jsonEncode(<String, Object?>{'attached': await attached(), 'served': await served(), 'read': await read()}));
}
";

/// The definitions of every type, the service and the surfaces under test, the data, and `driver`.
fn module() -> String {
    let requests: serde_json::Map<String, Value> = OPERATIONS
        .iter()
        .zip(1_u64..)
        .map(|(operation, id)| {
            let broken: Value = serde_json::from_str(BROKEN_LOAD).unwrap();
            let asked = request(
                &json!(id),
                "tools/call",
                &json!({ "name": operation, "arguments": broken }),
            );
            ((*operation).to_owned(), Value::String(asked.to_string()))
        })
        .collect();
    [
        "import 'dart:async';".to_owned(),
        "import 'dart:convert';".to_owned(),
        above_item_dart::dart_definition(),
        above_load_dart::dart_definition(),
        apart_item_dart::dart_definition(),
        apart_load_dart::dart_definition(),
        below_item_dart::dart_definition(),
        below_load_dart::dart_definition(),
        below_renamed_dart::dart_definition(),
        apart_renamed_dart::dart_definition(),
        shipment_mcp_task_dart::dart_definition(),
        hoist_error_dart::dart_definition(),
        HoistServiceSchema::dart_definition(),
        HoistServiceSchema::dart_ws_client(),
        HoistServiceSchema::dart_mcp_service(),
        format!("const brokenText = r'''{BROKEN_LOAD}''';"),
        format!("const mcpRequests = r'''{}''';", Value::Object(requests)),
        DRIVER.to_owned(),
    ]
    .join("\n\n")
}

/// The issues Rust lists, as a fault writes them.
fn expected() -> Value {
    Value::Array(
        broken_issues()
            .into_iter()
            .map(|(field, detail)| json!({ "detail": detail, "field": field }))
            .collect(),
    )
}

#[test]
fn a_message_declared_above_below_or_in_another_crate_is_listed_issue_by_issue() {
    let Some(wrote) = ran(
        "dart-refusal-order",
        RUNTIME_VAR,
        "dart",
        "order.dart",
        &module(),
    ) else {
        return;
    };
    let told: Value = serde_json::from_str(wrote.trim()).unwrap();
    for (at, operation) in OPERATIONS.iter().enumerate() {
        let reply = &told["attached"][at];
        assert_eq!(reply["id"], *operation, "{told}");
        assert_eq!(
            reply["error"]["fault"]["kind"], "failed-validation",
            "{told}"
        );
        assert_eq!(
            reply["error"]["fault"]["issues"],
            expected(),
            "{operation}: {told}"
        );

        let answer: Value =
            serde_json::from_str(told["served"][operation].as_str().unwrap()).unwrap();
        let text = answer["result"]["content"][0]["text"].as_str().unwrap();
        let carried: Value = serde_json::from_str(text).unwrap();
        assert_eq!(carried["fault"]["kind"], "failed-validation", "{told}");
        assert_eq!(
            carried["fault"]["issues"],
            expected(),
            "{operation}: {told}"
        );

        assert_eq!(
            told["read"][operation]["kind"], "failedValidation",
            "{told}"
        );
        assert_eq!(
            told["read"][operation]["issues"],
            expected(),
            "{operation}: {told}"
        );
    }
}

#[test]
fn the_dart_text_of_the_hoist_service_passes_the_analyzer() {
    analyzed("refusal-order-analyze", &module());
}
