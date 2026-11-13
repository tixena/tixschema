//! A declared error of each kind the service's dispatcher cannot see when it is expanded: types
//! declared below the service, and held by a second Dart library. A handler throws the error as it
//! built it or as `validate()` answered it, and the attachment answers both alike; a plain enum has
//! no draft, so both throws are the value itself.

#![cfg(feature = "dart")]

use core::future::{Future as _, ready};
use core::pin::pin;
use core::task::{Context, Poll, Waker};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::{model_schema, service_schema};

use super::run_dart_valid_types::dart_files;
use super::runtime::ran;

const FRAMES: &str = "const FRAMES = [('1', 'spool', 'built'), ('2', 'spool', 'validated'), \
     ('3', 'bare', 'built'), ('4', 'bare', 'validated'), ('5', 'bare', 'invalid')];";

const HANDLED: &str = "
Future<Map<String, dynamic>> serveAll(List<String> frames) async {
  final written = <Map<String, dynamic>>[];
  final faults = <String>[];
  final settled = Completer<void>();
  void answered() {
    if (written.length == frames.length && !settled.isCompleted) settled.complete();
  }
  final served = StreamController<Map<String, dynamic>>();
  final detach = attachSpoolServiceWsDispatcher<void>(
    (inbound: served.stream, send: (reply) { written.add(reply); answered(); }),
    null,
    SpoolServiceHandlers<void>(
      spool: (ctx, label) async {
        throw SpoolFault.jammed;
      },
      bare: (ctx, label) async {
        if (label == 'built') throw BareFaultDraft(code: 'x');
        if (label == 'validated') throw BareFaultDraft(code: 'x').validate();
        throw BareFaultDraft(code: '');
      },
    ),
    onFault: (fault) { faults.add(fault.kind.name); },
  );
  for (final frame in frames) {
    served.add(jsonDecode(frame) as Map<String, dynamic>);
  }
  await settled.future;
  detach();
  return <String, dynamic>{'written': written, 'faults': faults};
}

void main() async {
  final frames = <String>[
    for (final (id, operation, label) in FRAMES)
      jsonEncode(<String, dynamic>{
        'kind': 'request', 'id': id, 'service': 'SpoolService', 'operation': operation, 'payload': label,
      }),
  ];
  print(jsonEncode(await serveAll(frames)));
}
";

#[service_schema(transports = ["ws_rpc"])]
pub trait SpoolService<Ctx> {
    async fn bare(&self, ctx: &Ctx, label: String) -> Result<String, BareFault>;

    async fn spool(&self, ctx: &Ctx, label: String) -> Result<String, SpoolFault>;
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SpoolFault {
    Empty,
    Jammed,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BareFault {
    #[model_schema_prop(minLength = 1)]
    pub code: String,
}

/// Answers each operation with the declared error, built through its draft.
pub struct Jammer;

impl SpoolService<()> for Jammer {
    async fn bare(&self, _ctx: &(), label: String) -> Result<String, BareFault> {
        ready(()).await;
        Err(BareFaultDraft { code: label }.validate().unwrap())
    }

    async fn spool(&self, _ctx: &(), _label: String) -> Result<String, SpoolFault> {
        ready(()).await;
        Err(SpoolFaultDraft::Jammed.validate().unwrap())
    }
}

fn errors() -> String {
    [
        spool_fault_dart::dart_definition(),
        bare_fault_dart::dart_definition(),
    ]
    .join("\n\n")
}

fn service() -> String {
    [
        SpoolServiceSchema::dart_definition(),
        SpoolServiceSchema::dart_ws_client(),
    ]
    .join("\n\n")
}

fn answered(printed: &str) -> Value {
    serde_json::from_str(printed.trim()).unwrap()
}

fn reply(answers: &Value, id: &str) -> Value {
    let mut found = answers["written"]
        .as_array()
        .unwrap()
        .iter()
        .find(|written| written["id"] == id)
        .unwrap()
        .clone();
    found.as_object_mut().unwrap().remove("id");
    found
}

fn holds(answers: &Value) {
    assert_eq!(reply(answers, "1"), reply(answers, "2"), "{answers}");
    assert_eq!(reply(answers, "3"), reply(answers, "4"), "{answers}");
    assert_eq!(reply(answers, "1")["ok"], false, "{answers}");
    assert_eq!(reply(answers, "1")["error"], json!("Jammed"), "{answers}");
    assert_eq!(reply(answers, "3")["ok"], false, "{answers}");
    assert_eq!(
        reply(answers, "3")["error"],
        json!({"code": "x"}),
        "{answers}"
    );
    assert_eq!(reply(answers, "5")["ok"], false, "{answers}");
    assert_eq!(
        reply(answers, "5")["error"]["isServiceFault"],
        true,
        "{answers}"
    );
    assert_eq!(answers["faults"].as_array().unwrap().len(), 1, "{answers}");
}

#[test]
fn an_error_declared_below_the_service_is_answered_as_built_and_as_validated() {
    let module = [
        "import 'dart:async';\nimport 'dart:convert';".to_owned(),
        errors(),
        service(),
        FRAMES.to_owned(),
        HANDLED.to_owned(),
    ]
    .join("\n\n");
    super::run_dart_validate::analyzed("declared-errors-below", &module);
    let Some(printed) = ran(
        "declared-errors-below",
        "TIXSCHEMA_DART",
        "dart",
        "errors.dart",
        &module,
    ) else {
        return;
    };
    holds(&answered(&printed));
}

#[test]
fn an_error_of_a_second_library_is_answered_as_built_and_as_validated() {
    let files = [
        (
            "errors.dart",
            format!("import 'dart:convert';\n\n{}", errors()),
        ),
        (
            "service.dart",
            format!(
                "import 'dart:async';\nimport 'dart:convert';\nimport 'errors.dart';\n\n{}",
                service()
            ),
        ),
        (
            "main.dart",
            format!(
                "import 'dart:async';\nimport 'dart:convert';\nimport 'errors.dart';\nimport \
                 'service.dart';\n\n{FRAMES}\n\n{HANDLED}"
            ),
        ),
    ];
    let Some((clean, said)) =
        dart_files("declared-errors-apart", &["analyze"], &files, "main.dart")
    else {
        return;
    };
    assert!(clean && said.contains("No issues found!"), "{said}");
    let (ok, printed) = dart_files("declared-errors-apart-run", &[], &files, "main.dart").unwrap();
    assert!(ok, "{printed}");
    holds(&answered(&printed));
}

#[test]
fn the_rust_twin_answers_the_valid_error() {
    let mut spooled = pin!(Jammer.spool(&(), "x".to_owned()));
    let mut barred = pin!(Jammer.bare(&(), "y".to_owned()));
    let mut waking = Context::from_waker(Waker::noop());
    let spool = spooled.as_mut().poll(&mut waking);
    let bare = barred.as_mut().poll(&mut waking);
    assert!(
        matches!(&spool, Poll::Ready(Err(fault)) if matches!(&**fault, SpoolFaultDraft::Jammed))
    );
    assert!(matches!(&bare, Poll::Ready(Err(fault)) if fault.code == "y"));
}
