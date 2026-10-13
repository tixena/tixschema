//! The `preprocess` the Dart backend emits, run by the Dart VM.
//!
//! The models are the ones the Rust read and the Zod schema are held to in
//! `tests/pattern_preprocess_tests/`: each payload is read in Dart with `fromJson`, and what Dart
//! prints is compared with what the Rust read prints of the same payload.

#![cfg(feature = "dart")]

/// The models of `tests/pattern_preprocess_tests/`, with the function each `preprocess` names.
#[path = "../pattern_preprocess_tests/models.rs"]
mod models;

use alloc::collections::BTreeMap;
use core::cell::RefCell;
use core::fmt::Write as _;
use core::future::{Future as _, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::{model_schema, service_schema};

use self::models::{
    POSITIONS, Parcel, Pinned, SAMPLE, SAMPLE_ROWS, Tagged, carrier_dart, closed_dock_dart,
    consigned_dart, crate_dart, crated_dart, delivery_dart, either_dart, grade_dart, handover_dart,
    label_dart, manifest_dart, node_dart, parcel_dart, pinned_dart, positions, reference_dart,
    routed_dart, sample_report, shelf_dart, shipment_dart, slot_dart, stack_dart, tagged_dart,
    tagged_schema, transfer_dart, trimmed, unnamed_dart, weighed_dart, with_its_keys, wrapped_dart,
};
use super::run_dart_validate::{dart, from_until, module};
use super::runtime::ran;
use super::tests::{ShelfError, shelf_error_dart};

/// The functions of the counted tally, each counting its calls under the name of what it
/// corrects, and what prints the counts of one read.
const COUNTED: &str = "
final calls = <String, int>{'label': 0, 'note': 0, 'pair': 0, 'tally': 0};

dynamic counted(String named, dynamic payload) {
  calls[named] = calls[named]! + 1;
  return payload;
}

dynamic countedLabel(dynamic payload) => counted('label', payload);

dynamic countedNote(dynamic payload) => counted('note', payload);

dynamic countedPair(dynamic payload) => counted('pair', payload);

dynamic countedTally(dynamic payload) => counted('tally', payload);

void tallied(String name, Object? Function() read) {
  calls.updateAll((named, count) => 0);
  var answered = 'read';
  try {
    read();
  } on TypeError {
    answered = 'not read';
  }
  print('$name: $answered; ${calls.entries.map((entry) => '${entry.key} ${entry.value}').join(', ')}');
}
";

/// What says which member of the result a call answered: the parcel read, or the fault's kind
/// and detail.
const DESCRIBED: &str = "
Map<String, dynamic> described(IntakeClientServiceWeighResult result) {
  return switch (result) {
    IntakeClientServiceWeighResultOk(:final value) => {'kind': 'ok', 'parcel': value.toJson()},
    IntakeClientServiceWeighResultOperation() => {'kind': 'operation'},
    IntakeClientServiceWeighResultFault(:final fault) => {
        'kind': 'fault',
        'faultKind': fault.kind.name,
        'detail': fault.detail,
      },
  };
}
";

/// The two functions of the documents, as a consumer writes them and as the README shows them.
const FUNCTIONS: &str = "
dynamic trimmed(dynamic payload) => payload is String ? payload.trim() : payload;

dynamic withoutFragile(dynamic payload) =>
    payload is Map<String, dynamic> ? {'fragileItemCodes': <dynamic>[], ...payload} : payload;
";

/// A stub transport that answers `weigh` with a parcel labelled as the request asks, which holds
/// no `fragileItemCodes`, and what prints the answer of each call.
const HTTP_DRIVER: &str = "
class _Scale implements IntakeClientServiceHttpTransport {
  @override
  Future<({int status, List<(String, String)> headers, List<int> body, Stream<List<int>> bodyStream})> send(
    ({String method, String path, String query, List<(String, String)> headers, List<int> body, List<(String, dynamic)> parts}) request,
  ) async {
    final label = jsonDecode(utf8.decode(request.body)) as String;
    return (
      status: 200,
      headers: <(String, String)>[],
      body: utf8.encode(jsonEncode(<String, dynamic>{
        'label': label,
        'itemCodeList': <String>['x'],
      })),
      bodyStream: const Stream<List<int>>.empty(),
    );
  }
}

void main() async {
  final client = IntakeClientServiceHttpClient(_Scale());
  print(jsonEncode(<String, dynamic>{
    'spaced': described(await client.weigh('  A  ')),
    'blank': described(await client.weigh('   ')),
  }));
}
";

/// The function the check of the weighed model names.
const CHECK: &str = "
List<String> withinLimit(Weighed value) =>
    value.grams > 1000 ? [\"'grams': ${value.grams} is over the limit of 1000\"] : const [];
";

/// The functions the other models name, one per correction.
const CORRECTIONS: &str = "
dynamic orUnnamed(dynamic payload) => payload ?? 'unnamed';

dynamic trimmedPlate(dynamic payload) => payload is Map<String, dynamic> && payload['plate'] is String
    ? {...payload, 'plate': (payload['plate'] as String).trim()}
    : payload;

dynamic unwrapped(dynamic payload) =>
    payload is Map<String, dynamic> && payload.containsKey('payload') ? payload['payload'] : payload;

dynamic withItsKeys(dynamic payload) =>
    payload is Map<String, dynamic> ? {...payload, 'keys': (payload.keys.toList()..sort())} : payload;
";

/// The functions named after the names a read declares: each trims a text.
const NAMED_FUNCTIONS: &str = "
dynamic codeTypeFromJson(dynamic payload) => payload is String ? payload.trim() : payload;

dynamic corrected(dynamic payload) => payload is String ? payload.trim() : payload;

dynamic json(dynamic payload) => payload is String ? payload.trim() : payload;

dynamic map(dynamic payload) => payload is String ? payload.trim() : payload;

dynamic payload(dynamic payload) => payload is Map<String, dynamic> && payload.containsKey('held') ? payload['held'] : payload;

dynamic read(dynamic payload) => payload is String ? (int.tryParse(payload.trim()) ?? payload) : payload;

dynamic tag(dynamic payload) => payload is Map<String, dynamic> && payload.containsKey('held') ? payload['held'] : payload;
";

/// The rows of the shared table Dart does not read as the Rust read does, each with why.
const NOT_READ: [(&str, &str); 2] = [
    (
        "a type that reaches itself",
        "the backend writes `Self` as a type of that name",
    ),
    (
        "a plain enum",
        "the read of a plain enum takes a `String`, so an envelope does not reach its function",
    ),
];

/// What prints a row as every side prints it: its name and the value read, written back, or each
/// message of the refusal on a line of its own.
const ROW: &str = "
void row(String name, Object? Function() written) {
  try {
    print('$name: read ${jsonEncode(written())}');
  } on FormatException catch (refused) {
    print('$name: refused');
    for (final violation in refused.message.split('; ')) {
      print('    $violation');
    }
  }
}
";

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_DART";

/// One row per shape declared here: its name, the Dart expression that reads `@` and writes it
/// back, and the payload. The type the Rust read reads it as is the one the expression names.
const SHAPE_ROWS: [(&str, &str, &str); 12] = [
    (
        "a flattened type",
        "Pinned.fromJson(@).toJson()",
        r#"{"dock":"north","tag":"t"}"#,
    ),
    (
        "a function on a flattened member",
        "Sorted.fromJson(@).toJson()",
        r#"{"dock":"north","pier":"p","tag":"t"}"#,
    ),
    (
        "a type flattened into a variant under a tag",
        "Berth.fromJson(@).toJson()",
        r#"{"kind":"Loose","dock":"north","tag":"t"}"#,
    ),
    (
        "a slot's function under a tag",
        "Berth.fromJson(@).toJson()",
        r#"{"kind":"Slot","tag":"t"}"#,
    ),
    (
        "a type flattened into a variant under its name",
        "Quay.fromJson(@).toJson()",
        r#"{"Loose":{"dock":"north","tag":"t"}}"#,
    ),
    (
        "a type flattened into a struct that writes a tag",
        "Moored.fromJson(@).toJson()",
        r#"{"kind":"Moored","dock":"north","tag":"t"}"#,
    ),
    (
        "a flattened map",
        "Ledger.fromJson(@).toJson()",
        r#"{"dock":"north","tag":7}"#,
    ),
    (
        "a slot under a content key",
        "Dropped.fromJson(@).toJson()",
        r#"{"kind":"Left","detail":"  door  "}"#,
    ),
    (
        "a slot of several",
        "Quay.fromJson(@).toJson()",
        r#"{"Split":["  a  ",2]}"#,
    ),
    (
        "a slot of a tuple struct",
        "Bundle.fromJson(@).toJson()",
        r#"["  a  ",2]"#,
    ),
    (
        "a plain enum from a text",
        "Speed.fromJson(@).toJson()",
        r#""  Express  ""#,
    ),
    (
        "a brand with a parameter",
        "Mark<String>.fromJson(@, (e) => e as String).toJson((e) => e)",
        r#""  A  ""#,
    ),
];

/// What every side prints for the shapes declared here: where serde hands a function fewer keys
/// than the payload holds, a slot's function, and a text a plain enum's function corrects.
const SHAPES: &str = "\
a flattened type: read {\"dock\":\"north\",\"keys\":[\"tag\"],\"tag\":\"t\"}
a function on a flattened member: read {\"dock\":\"north\",\"keys\":[\"tag\"],\"tag\":\"t\",\"pier\":\"p\"}
a type flattened into a variant under a tag: read {\"kind\":\"Loose\",\"dock\":\"north\",\"keys\":[\"tag\"],\"tag\":\"t\"}
a slot's function under a tag: read {\"kind\":\"Slot\",\"keys\":[\"tag\"],\"tag\":\"t\"}
a type flattened into a variant under its name: read {\"Loose\":{\"dock\":\"north\",\"keys\":[\"tag\"],\"tag\":\"t\"}}
a type flattened into a struct that writes a tag: read {\"kind\":\"Moored\",\"dock\":\"north\",\"keys\":[\"kind\",\"tag\"],\"tag\":\"t\"}
a flattened map: read {\"dock\":\"north\",\"tag\":7}
a slot under a content key: read {\"kind\":\"Left\",\"detail\":\"door\"}
a slot of several: read {\"Split\":[\"a\",2]}
a slot of a tuple struct: read [\"a\",2]
a plain enum from a text: read \"Express\"
a brand with a parameter: read \"A\"
";

/// What every side prints for the reads of the counted tally: whether it was read, then how
/// often each function was called.
const TALLIES: &str = "\
every key held: read; label 1, note 1, pair 1, tally 1
no note: read; label 1, note 0, pair 1, tally 1
a null note: read; label 1, note 1, pair 1, tally 1
no key: not read; label 0, note 0, pair 0, tally 1
";

/// Answers each `weigh` the client sends over an in-memory stream pair with a parcel labelled as
/// the request asks, which holds no `fragileItemCodes`, and prints the answer of each call.
const WS_DRIVER: &str = "
void main() async {
  final sentRaw = <dynamic>[];
  final outbound = StreamController<dynamic>();
  outbound.stream.listen(sentRaw.add);
  final inbound = StreamController<dynamic>();
  final transport = IntakeClientServiceWsTransport(
    sink: outbound.sink,
    stream: inbound.stream,
    heartbeat: IntakeClientServiceWsHeartbeat.off(),
  );
  final client = IntakeClientServiceWsClient(transport);

  Future<Map<String, dynamic>> answered(String label) async {
    final weighed = client.weigh(label);
    await Future<void>.delayed(Duration.zero);
    final frame = jsonDecode(sentRaw.last as String) as Map<String, dynamic>;
    inbound.add(jsonEncode(<String, dynamic>{
      'kind': 'reply',
      'id': frame['id'],
      'service': 'IntakeClientService',
      'ok': true,
      'value': <String, dynamic>{
        'label': label,
        'itemCodeList': <String>['x'],
      },
    }));
    return described(await weighed);
  }

  final spaced = await answered('  A  ');
  final blank = await answered('   ');
  transport.close();
  print(jsonEncode(<String, dynamic>{'spaced': spaced, 'blank': blank}));
}
";

thread_local! {
    /// How often each function of the counted tally was called on this thread.
    static CALLS: RefCell<BTreeMap<&'static str, u32>> = const { RefCell::new(BTreeMap::new()) };
}

/// The members of [`Tagged`], with no function on the type.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Listed {
    pub keys: Vec<String>,
    pub tag: String,
}

/// A flattened member that names a function, between two members of its holder.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Sorted {
    pub dock: String,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub listed: Listed,
    pub pier: String,
}

/// An enum under a tag key: a variant that flattens a type, and one whose slot names a function.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Berth {
    Loose {
        dock: String,
        #[serde(flatten)]
        tagged: Tagged,
    },
    Slot(#[model_schema_prop(preprocess = ["withItsKeys"])] Listed),
}

/// An enum serde writes under its variant's name: a variant that flattens a type, and a slot of
/// several that names a function.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Quay {
    Loose {
        dock: String,
        #[serde(flatten)]
        tagged: Tagged,
    },
    Split(#[model_schema_prop(preprocess = ["trimmed"])] String, u32),
}

/// A struct that writes a tag of its own, and flattens a type.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub struct Moored {
    pub dock: String,
    #[serde(flatten)]
    pub tagged: Tagged,
}

/// A map flattened beside a member of its holder.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Ledger {
    pub dock: String,
    #[serde(flatten)]
    pub extras: BTreeMap<String, u32>,
}

/// An enum under a tag key and a content key, whose slot names a function.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum Dropped {
    Left(#[model_schema_prop(preprocess = ["trimmed"])] String),
}

/// A tuple struct, one slot of which names a function.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Bundle(
    #[model_schema_prop(preprocess = ["trimmed"])] pub String,
    pub u32,
);

/// An enum of unit variants whose function corrects the text serde writes for it.
#[model_schema(preprocess = ["trimmed"])]
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum Speed {
    Express,
    Standard,
}

/// A brand with a parameter.
#[model_schema(default_types(IdType = String), preprocess = ["trimmed"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Mark<IdType>(pub IdType);

/// A type whose functions count their calls: one on the type, and one on a member of each kind
/// whose read names its payload more than once.
#[model_schema(preprocess = ["countedTally"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tally {
    #[model_schema_prop(preprocess = ["countedLabel"])]
    pub label: String,
    #[model_schema_prop(preprocess = ["countedNote"])]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[model_schema_prop(preprocess = ["countedPair"])]
    pub seat_pair: (String, u32),
}

/// A class whose functions are named after the names its read declares.
#[model_schema(preprocess = ["payload"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Renamed {
    #[model_schema_prop(preprocess = ["json"])]
    pub label: String,
    #[model_schema_prop(preprocess = ["corrected"])]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// An enum whose functions are named after the names its dispatch declares.
#[model_schema(preprocess = ["tag"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Rerouted {
    Held,
    Rail(#[model_schema_prop(preprocess = ["map"])] String),
}

/// An untagged enum whose function is named after the name its attempt declares.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Recounted {
    Amount(#[model_schema_prop(minimum = 1, preprocess = ["read"])] u32),
    Text(String),
}

/// A type with a parameter whose function is named after the converter of the parameter.
#[model_schema(default_types(CodeType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Keyed<CodeType> {
    pub codes: Vec<CodeType>,
    #[model_schema_prop(preprocess = ["codeTypeFromJson"])]
    pub label: String,
}

/// One operation that answers the parcel of the documents.
#[service_schema(transports = ["http_rest"])]
pub trait IntakeClientService<Ctx> {
    #[service_schema_op(http(method = "POST", path = "/weigh", error_status(Missing = 404)))]
    async fn weigh(&self, ctx: &Ctx, label: String) -> Result<Parcel, ShelfError>;
}

/// A back end that answers the contract with a parcel labelled as it is asked.
pub struct Scale;

impl IntakeClientService<()> for Scale {
    async fn weigh(&self, _ctx: &(), label: String) -> Result<Parcel, ShelfError> {
        ready(()).await;
        Ok(Parcel {
            fragile_item_codes: Vec::new(),
            item_code_list: vec!["x".to_owned()],
            label,
        })
    }
}

/// Counts one call under `named`, and answers `payload` as it is.
fn counted(named: &'static str, payload: Value) -> Value {
    CALLS.with_borrow_mut(|calls| *calls.entry(named).or_default() += 1);
    payload
}

fn counted_label(payload: Value) -> Value {
    counted("label", payload)
}

fn counted_note(payload: Value) -> Value {
    counted("note", payload)
}

fn counted_pair(payload: Value) -> Value {
    counted("pair", payload)
}

fn counted_tally(payload: Value) -> Value {
    counted("tally", payload)
}

/// What an object holds under `held`, and any other payload as it is.
fn held(payload: Value) -> Value {
    if let Value::Object(mut entries) = payload {
        entries
            .remove("held")
            .unwrap_or_else(|| Value::Object(entries))
    } else {
        payload
    }
}

fn code_type_from_json(payload: Value) -> Value {
    trimmed(payload)
}

fn corrected(payload: Value) -> Value {
    trimmed(payload)
}

fn json(payload: Value) -> Value {
    trimmed(payload)
}

fn map(payload: Value) -> Value {
    trimmed(payload)
}

fn payload(payload: Value) -> Value {
    held(payload)
}

/// The number a text holds, and any other payload as it is.
fn read(payload: Value) -> Value {
    let number = payload
        .as_str()
        .and_then(|text| text.trim().parse::<u32>().ok());
    number.map_or(payload, Value::from)
}

fn tag(payload: Value) -> Value {
    held(payload)
}

/// What a serde read of `payload` as a `T` prints under `row`: the value written back, or each
/// message of the refusal on a line of its own.
fn rust_row<T>(row: &str, payload: &str) -> String
where
    T: DeserializeOwned + Serialize,
{
    match serde_json::from_str::<T>(payload) {
        Ok(read) => format!("{row}: read {}\n", serde_json::to_string(&read).unwrap()),
        Err(refused) => {
            let mut printed = format!("{row}: refused\n");
            for message in refused.to_string().split("; ") {
                writeln!(printed, "    {message}").unwrap();
            }
            printed
        }
    }
}

/// The line of the driver that prints `name`: `reader` over `payload`, where its `@` is.
fn dart_row(name: &str, reader: &str, payload: &str) -> String {
    format!(
        "  row('{}', () => {});\n",
        name.replace('\'', "\\'"),
        reader.replace('@', &format!("jsonDecode(r'{payload}')"))
    )
}

/// The `main` that runs `rows`.
fn driver(rows: &str) -> String {
    format!("void main() {{\n{rows}}}\n")
}

/// The Dart expression that reads `@` as the type the schema `schema` describes and writes it
/// back.
fn reader_of(schema: &str) -> String {
    match schema {
        "Crate<String>" => {
            "Crate<String>.fromJson(@, (e) => e as String).toJson((e) => e)".to_owned()
        }
        "Crate<u32>" => "Crate<int>.fromJson(@, (e) => e as int).toJson((e) => e)".to_owned(),
        named => format!("{named}.fromJson(@).toJson()"),
    }
}

/// The emitted definitions of the models, without the one the SDK does not compile.
fn model_definitions() -> Vec<String> {
    vec![
        parcel_dart::dart_definition(),
        manifest_dart::dart_definition(),
        tagged_dart::dart_definition(),
        carrier_dart::dart_definition(),
        closed_dock_dart::dart_definition(),
        consigned_dart::dart_definition(),
        crate_dart::dart_definition(),
        crated_dart::dart_definition(),
        delivery_dart::dart_definition(),
        either_dart::dart_definition(),
        grade_dart::dart_definition(),
        handover_dart::dart_definition(),
        label_dart::dart_definition(),
        reference_dart::dart_definition(),
        pinned_dart::dart_definition(),
        routed_dart::dart_definition(),
        shelf_dart::dart_definition(),
        shipment_dart::dart_definition(),
        slot_dart::dart_definition(),
        stack_dart::dart_definition(),
        transfer_dart::dart_definition(),
        unnamed_dart::dart_definition(),
        weighed_dart::dart_definition(),
        wrapped_dart::dart_definition(),
    ]
}

/// The emitted definitions of the shapes declared here.
fn shape_definitions() -> Vec<String> {
    vec![
        tagged_dart::dart_definition(),
        pinned_dart::dart_definition(),
        listed_dart::dart_definition(),
        sorted_dart::dart_definition(),
        berth_dart::dart_definition(),
        quay_dart::dart_definition(),
        moored_dart::dart_definition(),
        ledger_dart::dart_definition(),
        dropped_dart::dart_definition(),
        bundle_dart::dart_definition(),
        speed_dart::dart_definition(),
        mark_dart::dart_definition(),
    ]
}

/// The sample of the documents: the four rows a JSON text holds, read in Dart, print what the
/// Rust read prints of them.
#[test]
fn the_sample_reads_as_the_rust_read_does() {
    assert_eq!(sample_report(), SAMPLE, "what the Rust read answers");
    let rows: String = SAMPLE_ROWS
        .iter()
        .map(|&(row, payload)| {
            let reader = if row == "row 5" {
                "Manifest.fromJson(@).toJson()"
            } else {
                "Parcel.fromJson(@).toJson()"
            };
            dart_row(row, reader, payload)
        })
        .collect();
    let Some(wrote) = ran(
        "dart",
        RUNTIME_VAR,
        "dart",
        "sample.dart",
        &module(
            &[
                parcel_dart::dart_definition(),
                manifest_dart::dart_definition(),
            ],
            &[FUNCTIONS, ROW],
            &driver(&rows),
        ),
    ) else {
        return;
    };
    assert_eq!(wrote, SAMPLE);
}

/// A key the payload does not hold is read as it was with no function named: the cast of what
/// the key holds fails, where the Rust read answers a missing field.
#[test]
fn a_label_the_payload_does_not_hold_is_not_read() {
    let absent = r#"{"itemCodeList":["x"],"fragileItemCodes":["x"]}"#;
    assert_eq!(
        serde_json::from_str::<Parcel>(absent)
            .unwrap_err()
            .to_string(),
        "missing field `label`",
        "what the Rust read answers"
    );
    let entry = format!(
        "void main() {{\n  try {{\n    Parcel.fromJson(jsonDecode(r'{absent}'));\n    \
         print('read');\n  }} on TypeError catch (refused) {{\n    print('not read: \
         $refused');\n  }}\n}}\n"
    );
    let Some(wrote) = ran(
        "dart",
        RUNTIME_VAR,
        "dart",
        "absent.dart",
        &module(&[parcel_dart::dart_definition()], &[FUNCTIONS], &entry),
    ) else {
        return;
    };
    assert_eq!(
        wrote,
        "not read: type 'Null' is not a subtype of type 'String' in type cast\n"
    );
}

/// Every position another type reads a preprocessed type in, and every shape the key is written
/// on, read in Dart: each row but the ones listed prints the line of the shared table.
#[test]
fn every_position_and_every_shape_prints_the_line_of_the_shared_table() {
    let read = positions();
    let printed: String = read.iter().map(|(_, _, _, line)| line.as_str()).collect();
    assert_eq!(printed, POSITIONS, "what the Rust read answers");
    let skipped = |name: &str| NOT_READ.iter().any(|&(row, _)| row == name);
    let rows: String = read
        .iter()
        .filter(|(name, ..)| !skipped(name))
        .map(|(name, schema, payload, _)| dart_row(name, &reader_of(schema), payload))
        .collect();
    let expected: String = read
        .iter()
        .filter(|(name, ..)| !skipped(name))
        .map(|(_, _, _, line)| line.as_str())
        .collect();
    assert_eq!(
        read.iter().filter(|(name, ..)| skipped(name)).count(),
        NOT_READ.len()
    );
    let Some(wrote) = ran(
        "dart",
        RUNTIME_VAR,
        "dart",
        "positions.dart",
        &module(
            &model_definitions(),
            &[FUNCTIONS, CORRECTIONS, CHECK, ROW],
            &driver(&rows),
        ),
    ) else {
        return;
    };
    assert_eq!(wrote, expected);
}

/// The two rows of the shared table Dart does not read: what the backend writes for a type that
/// reaches itself, and what the read of a plain enum answers for an envelope.
#[test]
fn the_rows_dart_does_not_read_are_the_ones_named() {
    let node = node_dart::dart_definition();
    assert!(
        node.contains("final List<Self> children;"),
        "{}: {node}",
        NOT_READ[0].1
    );
    let entry = "void main() {\n  try {\n    Grade.fromJson(jsonDecode(r'{\"payload\":\"Express\"}'));\n    \
                 print('read');\n  } on TypeError {\n    print('not read');\n  }\n  \
                 print(jsonEncode(Grade.fromJson('Express').toJson()));\n}\n";
    let Some(wrote) = ran(
        "dart",
        RUNTIME_VAR,
        "dart",
        "plain.dart",
        &module(&[grade_dart::dart_definition()], &[CORRECTIONS], entry),
    ) else {
        return;
    };
    assert_eq!(wrote, "not read\n\"Express\"\n", "{}", NOT_READ[1].1);
}

/// Where serde hands a function fewer keys than the payload holds, Dart hands it the same keys:
/// a flattened type, a function on a flattened member, a variant under a tag key. A slot's
/// function is called on the slot, and a plain enum's on the text.
#[test]
fn each_shape_declared_here_prints_what_the_rust_read_prints() {
    let rows = SHAPE_ROWS;
    let printed = [
        rust_row::<Pinned>(rows[0].0, rows[0].2),
        rust_row::<Sorted>(rows[1].0, rows[1].2),
        rust_row::<Berth>(rows[2].0, rows[2].2),
        rust_row::<Berth>(rows[3].0, rows[3].2),
        rust_row::<Quay>(rows[4].0, rows[4].2),
        rust_row::<Moored>(rows[5].0, rows[5].2),
        rust_row::<Ledger>(rows[6].0, rows[6].2),
        rust_row::<Dropped>(rows[7].0, rows[7].2),
        rust_row::<Quay>(rows[8].0, rows[8].2),
        rust_row::<Bundle>(rows[9].0, rows[9].2),
        rust_row::<Speed>(rows[10].0, rows[10].2),
        rust_row::<Mark<String>>(rows[11].0, rows[11].2),
    ]
    .concat();
    assert_eq!(printed, SHAPES, "what the Rust read answers");
    let read: String = rows
        .iter()
        .map(|&(name, reader, payload)| dart_row(name, reader, payload))
        .collect();
    let Some(wrote) = ran(
        "dart",
        RUNTIME_VAR,
        "dart",
        "shapes.dart",
        &module(
            &shape_definitions(),
            &[FUNCTIONS, CORRECTIONS, ROW],
            &driver(&read),
        ),
    ) else {
        return;
    };
    assert_eq!(wrote, SHAPES);
}

/// What the Rust read of `payload` as a tally prints under `name`: whether it was read, then how
/// often each function was called.
fn rust_tally(name: &str, payload: &str) -> String {
    CALLS.with_borrow_mut(|calls| {
        *calls = BTreeMap::from([("label", 0), ("note", 0), ("pair", 0), ("tally", 0)]);
    });
    let answered = if serde_json::from_str::<Tally>(payload).is_ok() {
        "read"
    } else {
        "not read"
    };
    let counts = CALLS.with_borrow(|calls| {
        calls
            .iter()
            .map(|(named, count)| format!("{named} {count}"))
            .collect::<Vec<_>>()
            .join(", ")
    });
    format!("{name}: {answered}; {counts}\n")
}

/// One read calls each function once, a member's whose read names its payload twice included,
/// and calls none for a key the payload does not hold. A key that holds `null` is held.
#[test]
fn one_read_calls_each_function_once_and_none_for_a_key_not_held() {
    let cases = [
        (
            "every key held",
            r#"{"label":"A","note":"n","seatPair":["a",1]}"#,
        ),
        ("no note", r#"{"label":"A","seatPair":["a",1]}"#),
        (
            "a null note",
            r#"{"label":"A","note":null,"seatPair":["a",1]}"#,
        ),
        ("no key", "{}"),
    ];
    let printed: String = cases
        .iter()
        .map(|&(name, payload)| rust_tally(name, payload))
        .collect();
    assert_eq!(printed, TALLIES, "what the Rust read answers");
    let mut entry = String::from("void main() {\n");
    for (name, payload) in cases {
        writeln!(
            entry,
            "  tallied('{name}', () => Tally.fromJson(jsonDecode(r'{payload}')));"
        )
        .unwrap();
    }
    entry.push_str("}\n");
    let Some(wrote) = ran(
        "dart",
        RUNTIME_VAR,
        "dart",
        "tally.dart",
        &module(&[tally_dart::dart_definition()], &[COUNTED], &entry),
    ) else {
        return;
    };
    assert_eq!(wrote, TALLIES);
}

/// A function named after a name the read declares is the one called: the read numbers its own,
/// the converter of a type parameter included.
#[test]
fn a_function_named_after_a_name_of_the_read_is_called() {
    let cases = [
        (
            "a class",
            "Renamed.fromJson(@).toJson()",
            r#"{"held":{"label":"  A  ","note":"  n  "}}"#,
        ),
        (
            "an enum under its variant's name",
            "Rerouted.fromJson(@).toJson()",
            r#"{"held":{"Rail":"  north  "}}"#,
        ),
        (
            "an untagged enum",
            "Recounted.fromJson(@).toJson()",
            r#""  7  ""#,
        ),
        (
            "a type with a parameter",
            "Keyed<String>.fromJson(@, (e) => e as String).toJson((e) => e)",
            r#"{"codes":["x"],"label":"  A  "}"#,
        ),
    ];
    let printed = [
        rust_row::<Renamed>(cases[0].0, cases[0].2),
        rust_row::<Rerouted>(cases[1].0, cases[1].2),
        rust_row::<Recounted>(cases[2].0, cases[2].2),
        rust_row::<Keyed<String>>(cases[3].0, cases[3].2),
    ]
    .concat();
    assert_eq!(
        printed,
        "a class: read {\"label\":\"A\",\"note\":\"n\"}\nan enum under its variant's name: read \
         {\"Rail\":\"north\"}\nan untagged enum: read 7\na type with a parameter: read \
         {\"codes\":[\"x\"],\"label\":\"A\"}\n",
        "what the Rust read answers"
    );
    let rows: String = cases
        .iter()
        .map(|&(name, reader, payload)| dart_row(name, reader, payload))
        .collect();
    let definitions = [
        renamed_dart::dart_definition(),
        rerouted_dart::dart_definition(),
        recounted_dart::dart_definition(),
        keyed_dart::dart_definition(),
    ];
    let Some(wrote) = ran(
        "dart",
        RUNTIME_VAR,
        "dart",
        "named.dart",
        &module(&definitions, &[NAMED_FUNCTIONS, ROW], &driver(&rows)),
    ) else {
        return;
    };
    assert_eq!(wrote, printed);
}

/// A `preprocess` with no function under its name does not build, and the compiler names it.
#[test]
fn a_preprocess_with_no_dart_function_does_not_compile() {
    let rows = dart_row("row 1", "Parcel.fromJson(@).toJson()", SAMPLE_ROWS[0].1);
    let source = module(&[parcel_dart::dart_definition()], &[ROW], &driver(&rows));
    let Some((built, said)) = dart("missing-preprocess", &[], &source) else {
        return;
    };
    assert!(!built, "{said}");
    for named in ["trimmed", "withoutFragile"] {
        assert!(
            said.contains(&format!("Error: Method not found: '{named}'.")),
            "{said}"
        );
    }
}

/// The emitted source of the sample, and of every shape that names a function and whose codec
/// the analyzer passes. The tally is run above and left out here: the read of its tuple casts a
/// value whose type the analyzer already knows, as the read of a tuple struct does.
#[test]
fn the_emitted_source_of_a_preprocess_has_no_issue() {
    let rows = dart_row("row 1", "Parcel.fromJson(@).toJson()", SAMPLE_ROWS[0].1);
    let sample = module(
        &[
            parcel_dart::dart_definition(),
            manifest_dart::dart_definition(),
        ],
        &[FUNCTIONS, ROW],
        &driver(&rows),
    );
    let Some((clean, said)) = dart("analyze-sample", &["analyze"], &sample) else {
        return;
    };
    assert!(clean, "{said}\n--- source ---\n{sample}");
    assert!(said.contains("No issues found!"), "{said}");
    let definitions = [
        parcel_dart::dart_definition(),
        manifest_dart::dart_definition(),
        closed_dock_dart::dart_definition(),
        crate_dart::dart_definition(),
        grade_dart::dart_definition(),
        handover_dart::dart_definition(),
        label_dart::dart_definition(),
        reference_dart::dart_definition(),
        shipment_dart::dart_definition(),
        transfer_dart::dart_definition(),
        unnamed_dart::dart_definition(),
        weighed_dart::dart_definition(),
        wrapped_dart::dart_definition(),
        dropped_dart::dart_definition(),
        speed_dart::dart_definition(),
        mark_dart::dart_definition(),
        renamed_dart::dart_definition(),
        rerouted_dart::dart_definition(),
        recounted_dart::dart_definition(),
        keyed_dart::dart_definition(),
    ];
    let shapes = module(
        &definitions,
        &[FUNCTIONS, CORRECTIONS, CHECK, NAMED_FUNCTIONS, ROW],
        "void main() {\n  row('none', () => null);\n}\n",
    );
    let Some((passed, reported)) = dart("analyze-preprocess", &["analyze"], &shapes) else {
        return;
    };
    assert!(passed, "{reported}\n--- source ---\n{shapes}");
    assert!(reported.contains("No issues found!"), "{reported}");
}

/// Writing calls no function: the back end writes the label as it was handed it.
#[test]
fn the_back_end_writes_a_label_with_its_spaces() {
    let mut weighing = pin!(Scale.weigh(&(), "  A  ".to_owned()));
    let mut polling = PollContext::from_waker(Waker::noop());
    let answered = match weighing.as_mut().poll(&mut polling) {
        Poll::Ready(answer) => Some(answer),
        Poll::Pending => None,
    };
    assert_eq!(
        serde_json::to_string(&answered.unwrap().unwrap()).unwrap(),
        r#"{"fragileItemCodes":[],"itemCodeList":["x"],"label":"  A  "}"#
    );
}

/// The library a client of [`IntakeClientService`] is run in: `imports`, the classes it reads,
/// the functions they name, `client` and `entry`.
fn client_module(imports: &[&str], client: String, entry: &str) -> String {
    let mut parts: Vec<String> = imports.iter().map(|&import| import.to_owned()).collect();
    parts.extend([
        parcel_dart::dart_definition(),
        shelf_error_dart::dart_definition(),
        FUNCTIONS.to_owned(),
        IntakeClientServiceSchema::dart_definition(),
        client,
        DESCRIBED.to_owned(),
        entry.to_owned(),
    ]);
    parts.join("\n\n")
}

/// What a client answers for a reply whose label has spaces at its ends and which holds no
/// `fragileItemCodes`, and for one whose label is spaces alone, where it answers the latter with
/// a fault of `kind`.
fn answers(kind: &str) -> Value {
    serde_json::json!({
        "spaced": {
            "kind": "ok",
            "parcel": {"fragileItemCodes": [], "itemCodeList": ["x"], "label": "A"},
        },
        "blank": {
            "kind": "fault",
            "faultKind": kind,
            "detail": "FormatException: 'label': too short: minimum length is 1, got 0",
        },
    })
}

/// The HTTP client reads a reply through the functions of its type: the label is trimmed, the
/// missing key is given, and the bound judges what the function answered.
#[test]
fn the_http_client_reads_a_reply_through_its_functions() {
    let Some(wrote) = ran(
        "dart",
        RUNTIME_VAR,
        "dart",
        "scale_http.dart",
        &client_module(
            &["import 'dart:convert';"],
            IntakeClientServiceSchema::dart_http_client(),
            HTTP_DRIVER,
        ),
    ) else {
        return;
    };
    let written: Value = serde_json::from_str(wrote.trim()).unwrap();
    assert_eq!(written, answers("undeserializablePayload"));
}

/// The WebSocket client reads a reply through the functions of its type.
#[test]
fn the_ws_client_reads_a_reply_through_its_functions() {
    let Some(wrote) = ran(
        "dart",
        RUNTIME_VAR,
        "dart",
        "scale_ws.dart",
        &client_module(
            &["import 'dart:async';", "import 'dart:convert';"],
            IntakeClientServiceSchema::dart_ws_client(),
            WS_DRIVER,
        ),
    ) else {
        return;
    };
    let written: Value = serde_json::from_str(wrote.trim()).unwrap();
    assert_eq!(written, answers("failedValidation"));
}

/// The README's section on a `preprocess` in Dart, held to what is emitted and printed here. Its
/// parcel declares its members in another order than the parcel of the models does, and the
/// emitter writes a class on one line, so each line the README shows is looked for in the read
/// that is emitted, with the white space taken out of both.
#[test]
fn the_readme_shows_the_read_that_is_emitted_and_what_it_answers() {
    let readme = include_str!("../../README.md");
    let section = from_until(
        readme,
        "#### A `preprocess` in the Dart client",
        "### Optional TypeScript Keys",
    );
    let packed = |text: &str| text.split_whitespace().collect::<String>();
    let parcel_class = parcel_dart::dart_definition();
    let emitted = packed(from_until(
        &parcel_class,
        "factory Parcel.fromJsonUnjudged(",
        " Map<String, dynamic> toJson()",
    ));
    let shown = from_until(
        section,
        "```dart\nfactory Parcel.fromJsonUnjudged(",
        "\n```",
    )
    .strip_prefix("```dart\n")
    .unwrap();
    assert_eq!(shown.lines().count(), 8, "{shown}");
    for line in shown.lines() {
        assert!(
            emitted.contains(&packed(line)),
            "the read that is emitted holds no `{line}`: {emitted}"
        );
    }
    assert_eq!(packed(shown).len(), emitted.len(), "{shown}");
    assert!(
        section.contains(&format!("```dart{FUNCTIONS}```")),
        "the README does not show the functions the tests run: {FUNCTIONS}"
    );
    for signature in [
        "dynamic trimmed(dynamic payload)",
        "dynamic withoutFragile(dynamic payload)",
    ] {
        assert!(
            FUNCTIONS.contains(&format!("\n{signature} =>")),
            "{signature}"
        );
        assert!(section.contains(&format!("`{signature}`")), "{signature}");
    }
    for answered in [
        "| 1 | read; `label` is `\"A\"` |",
        "| 2 | refused with a `FormatException`: `'label': too short: minimum length is 1, got 0` |",
        "| 3 | read; `fragile_item_codes` is `[]` |",
        "| 5 | read; `parcel.label` is `\"A\"` |",
        "`type 'Null' is not a subtype of type 'String' in type cast`",
        "`Error: Method not found: 'trimmed'.`",
        "`Tagged.fromJsonUnjudged(({...json}..remove('dock')))`",
    ] {
        assert!(
            section.contains(answered),
            "the README does not show: {answered}"
        );
    }
    assert!(
        pinned_dart::dart_definition()
            .contains("Tagged.fromJsonUnjudged(({...json}..remove('dock')))"),
        "what a holder hands a flattened type"
    );
    assert!(section.contains("last resort"), "{section}");
}
