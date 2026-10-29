//! The read the Dart backend emits, run by the Dart VM.
//!
//! `fromJson` of each shape reads the payload serde writes for a value, and refuses it with what
//! the Dart `validate()` answers of the value `fromJsonUnjudged` builds from the same payload,
//! which [`super::run_dart_validate`] holds to Rust's own. The sample is compared with what the
//! Rust read answers, and a reply that breaks a rule is put through both clients.

#![cfg(feature = "dart")]

use alloc::collections::BTreeMap;
use core::fmt::Write as _;
use core::future::{Future as _, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

use super::run_dart_validate::{
    CHECKS, Case, Crate, Deck, Gauge, Pair, Parcel, SHAPE_FUNCTIONS, Stamp, Terminal, Yard,
    brand_cases, compared, dart, from_until, manifest_dart, module, named_shape_cases,
    named_variant_cases, outside, parcel, parcel_dart, pinned, shape_definitions, sound,
    table_cases, tagged_and_untagged_variant_cases, texts, unlabelled,
};
use super::runtime::ran;
use super::tests::{ShelfError, shelf_error_dart};

/// The class the emitter writes for a struct whose one field is named after the unjudged read,
/// which `src/features/dart/tests.rs` holds the emitter to.
const LEDGER: &str = include_str!("../dart_tests/field_named_after_the_unjudged_read.txt");

/// What prints a read: its name and whether it was read, then each message it is refused with.
const JUDGED: &str = "
void judged(String name, Object? Function() read) {
  try {
    read();
    print('$name: read');
  } on FormatException catch (refused) {
    print('$name: refused (FormatException)');
    for (final violation in refused.message.split('; ')) {
      print('    $violation');
    }
  }
}
";

/// The function the check of [`Tray`] names.
const KNOWN_TRAY: &str = "
List<String> knownTray(Tray value) => value.value == 'XX' ? const ['a tray is known'] : const [];
";

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_DART";

/// The five reads of the sample, as the Dart VM prints them.
const SAMPLE: &str = "\
Parcel alone: refused (FormatException)
    'fragileItemCodes.1': y is outside of itemCodeList
Terminal, four broken parcels: refused (FormatException)
    'firstParcel.label': too short: minimum length is 1, got 0
    'parcelList.1.fragileItemCodes.1': y is outside of itemCodeList
    'parcelsByDock.north.label': too short: minimum length is 1, got 0
    'spareParcel.fragileItemCodes.1': y is outside of itemCodeList
List<Terminal>, the second broken: refused (FormatException)
    'firstParcel.label': too short: minimum length is 1, got 0
    'parcelList.1.fragileItemCodes.1': y is outside of itemCodeList
    'parcelsByDock.north.label': too short: minimum length is 1, got 0
    'spareParcel.fragileItemCodes.1': y is outside of itemCodeList
Crate<Parcel>, the second code broken: refused (FormatException)
    'fragileItemCodes.1': y is outside of itemCodeList
Terminal with no spareParcel key, all good: read
";

/// A stub transport that answers `weigh` with a parcel labelled as the request asks, and what
/// prints the answer of each call.
const HTTP_DRIVER: &str = "
class _Desk implements ParcelClientServiceHttpTransport {
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
        'fragileItemCodes': <String>['x'],
      })),
      bodyStream: const Stream<List<int>>.empty(),
    );
  }
}

void main() async {
  final client = ParcelClientServiceHttpClient(_Desk());
  print(jsonEncode(<String, dynamic>{
    'sound': described(await client.weigh('A')),
    'unlabelled': described(await client.weigh('')),
  }));
}
";

/// Answers each `weigh` the client sends over an in-memory stream pair with a parcel labelled as
/// the request asks, and prints the answer of each call.
const WS_DRIVER: &str = "
void main() async {
  final sentRaw = <dynamic>[];
  final outbound = StreamController<dynamic>();
  outbound.stream.listen(sentRaw.add);
  final inbound = StreamController<dynamic>();
  final transport = ParcelClientServiceWsTransport(
    sink: outbound.sink,
    stream: inbound.stream,
    heartbeat: ParcelClientServiceWsHeartbeat.off(),
  );
  final client = ParcelClientServiceWsClient(transport);

  Future<Map<String, dynamic>> answered(String label) async {
    final weighed = client.weigh(label);
    await Future<void>.delayed(Duration.zero);
    final frame = jsonDecode(sentRaw.last as String) as Map<String, dynamic>;
    inbound.add(jsonEncode(<String, dynamic>{
      'kind': 'reply',
      'id': frame['id'],
      'service': 'ParcelClientService',
      'ok': true,
      'value': <String, dynamic>{
        'label': label,
        'itemCodeList': <String>['x'],
        'fragileItemCodes': <String>['x'],
      },
    }));
    return described(await weighed);
  }

  final sound = await answered('A');
  final unlabelled = await answered('');
  transport.close();
  print(jsonEncode(<String, dynamic>{'sound': sound, 'unlabelled': unlabelled}));
}
";

/// What says which member of the result a call answered, and the fault's kind and detail.
const DESCRIBED: &str = "
Map<String, dynamic> described(ParcelClientServiceWeighResult result) {
  return switch (result) {
    ParcelClientServiceWeighResultOk(:final value) => {'kind': 'ok', 'label': value.label},
    ParcelClientServiceWeighResultOperation() => {'kind': 'operation'},
    ParcelClientServiceWeighResultFault(:final fault) => {
        'kind': 'fault',
        'faultKind': fault.kind.name,
        'detail': fault.detail,
      },
  };
}
";

/// A model with a bound.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Carton {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

/// A brand with a bound, and a check a value that holds the bound can break.
#[model_schema(minLength = 2, check = ["knownTray"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Tray(pub String);

/// A member whose own bound is held beside what the brand it holds declares.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Rack {
    #[model_schema_prop(maxLength = 9)]
    pub tray: Tray,
}

/// An enum serde writes as the value of its variant: a member's own bound and a brand's own
/// bound each choose a variant, and a model and a brand's check choose none.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Sticker {
    Boxed(Carton),
    Positive(#[model_schema_prop(minimum = 1)] i64),
    Trayed(Tray),
    Whole(i64),
    Written(String),
}

/// A plain enum one member of which is named after the unjudged read.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum Stage {
    FromJsonUnjudged,
    Judged,
}

/// One operation that answers a model held to a bound and a check.
#[service_schema(transports = ["http_rest"])]
pub trait ParcelClientService<Ctx> {
    #[service_schema_op(http(method = "POST", path = "/weigh", error_status(Missing = 404)))]
    async fn weigh(&self, ctx: &Ctx, label: String) -> Result<Parcel, ShelfError>;
}

/// A back end that answers the contract with a parcel labelled as it is asked.
pub struct Desk;

impl ParcelClientService<()> for Desk {
    async fn weigh(&self, _ctx: &(), label: String) -> Result<Parcel, ShelfError> {
        ready(()).await;
        Ok(parcel(&label, &["x"], &["x"]))
    }
}

fn known_tray(value: &Tray) -> Result<(), Vec<String>> {
    if value.0 == "XX" {
        return Err(vec!["a tray is known".to_owned()]);
    }
    Ok(())
}

/// The Dart class the variant `taken` is read as.
const fn class_of(taken: &Sticker) -> &'static str {
    match taken {
        Sticker::Boxed(_) => "StickerBoxed",
        Sticker::Positive(_) => "StickerPositive",
        Sticker::Trayed(_) => "StickerTrayed",
        Sticker::Whole(_) => "StickerWhole",
        Sticker::Written(_) => "StickerWritten",
    }
}

/// `held` as a case the judged read answers: `built` reads it with `fromJson` where it reads it
/// unjudged. `None` for a case built in code.
fn judged_case(held: &Case) -> Option<Case> {
    held.built.contains(".fromJsonUnjudged(").then(|| Case {
        answered: held.answered.clone(),
        built: held.built.replacen(".fromJsonUnjudged(", ".fromJson(", 1),
        name: held.name,
    })
}

/// What [`JUDGED`] prints for `cases`.
fn printed_for(cases: &[Case]) -> String {
    let mut printed = String::new();
    for held in cases {
        if held.answered.is_empty() {
            writeln!(printed, "{}: read", held.name).unwrap();
        } else {
            writeln!(printed, "{}: refused (FormatException)", held.name).unwrap();
            for message in &held.answered {
                writeln!(printed, "    {message}").unwrap();
            }
        }
    }
    printed
}

/// The `main` that reads each of `cases` and prints whether it was read.
fn driver(cases: &[Case]) -> String {
    let mut written = String::from("void main() {\n");
    for held in cases {
        writeln!(written, "  judged('{}', () => {});", held.name, held.built).unwrap();
    }
    written.push_str("}\n");
    written
}

#[rustversion::since(1.98)]
fn mapped_or_default<T, U>(value: Option<T>, map: impl FnOnce(T) -> U) -> U
where
    U: Default,
{
    value.map_or_default(map)
}

#[rustversion::before(1.98)]
fn mapped_or_default<T, U>(value: Option<T>, map: impl FnOnce(T) -> U) -> U
where
    U: Default,
{
    value.map(map).unwrap_or_default()
}

/// What the Rust read answers of `payload` as a `T`: nothing, or its messages.
fn rust_read<T>(payload: &str) -> Vec<String>
where
    T: DeserializeOwned,
{
    mapped_or_default(serde_json::from_str::<T>(payload).err(), |refused| {
        refused
            .to_string()
            .split("; ")
            .map(ToOwned::to_owned)
            .collect()
    })
}

/// A terminal that holds a broken parcel in a field, a list, a map and an optional.
fn broken_terminal() -> Terminal {
    Terminal {
        first_parcel: unlabelled(),
        lists_by_dock: BTreeMap::new(),
        parcel_list: vec![sound(), outside()],
        parcels_by_dock: BTreeMap::from([("north".to_owned(), unlabelled())]),
        spare_parcel: Some(outside()),
    }
}

/// A terminal that holds sound parcels, and no spare one.
fn sound_terminal() -> Terminal {
    Terminal {
        first_parcel: sound(),
        lists_by_dock: BTreeMap::new(),
        parcel_list: vec![sound()],
        parcels_by_dock: BTreeMap::from([("north".to_owned(), sound())]),
        spare_parcel: None,
    }
}

/// The four messages a read of [`broken_terminal`] is refused with, in the order the terminal
/// declares its members.
fn four_messages() -> Vec<String> {
    texts(&[
        "'firstParcel.label': too short: minimum length is 1, got 0",
        "'parcelList.1.fragileItemCodes.1': y is outside of itemCodeList",
        "'parcelsByDock.north.label': too short: minimum length is 1, got 0",
        "'spareParcel.fragileItemCodes.1': y is outside of itemCodeList",
    ])
}

/// The five reads of the sample.
fn sample_cases() -> Vec<Case> {
    let filled = Crate {
        codes: vec![sound(), outside()],
        label: "A".to_owned(),
    };
    let terminals = serde_json::to_string(&[sound_terminal(), broken_terminal()]).unwrap();
    vec![
        pinned(
            "Parcel alone",
            &outside(),
            &["'fragileItemCodes.1': y is outside of itemCodeList"],
            "Parcel.fromJson(@)",
        ),
        Case {
            answered: four_messages(),
            built: format!(
                "Terminal.fromJson(jsonDecode(r'{}'))",
                serde_json::to_string(&broken_terminal()).unwrap()
            ),
            name: "Terminal, four broken parcels",
        },
        Case {
            answered: four_messages(),
            built: format!(
                "(jsonDecode(r'{terminals}') as List<dynamic>).map((e) => \
                 Terminal.fromJson(e)).toList()"
            ),
            name: "List<Terminal>, the second broken",
        },
        pinned(
            "Crate<Parcel>, the second code broken",
            &filled,
            &["'fragileItemCodes.1': y is outside of itemCodeList"],
            "Crate<Parcel>.fromJson(@, (e) => Parcel.fromJson(e))",
        ),
        pinned(
            "Terminal with no spareParcel key, all good",
            &sound_terminal(),
            &[],
            "Terminal.fromJson(@)",
        ),
    ]
}

/// The sample: a model alone, a holder of four broken ones, a list of holders, a model behind a
/// type parameter and an absent optional key. Where a type is read at the top of a payload, the
/// Rust read answers the same messages.
#[test]
fn the_sample_is_refused_with_every_message_at_its_full_place() {
    let cases = sample_cases();
    assert_eq!(printed_for(&cases), SAMPLE);
    assert_eq!(
        rust_read::<Parcel>(&serde_json::to_string(&outside()).unwrap()),
        ["'fragileItemCodes.1': y is outside of itemCodeList"],
        "what the Rust read answers"
    );
    assert_eq!(
        rust_read::<Terminal>(&serde_json::to_string(&broken_terminal()).unwrap()),
        four_messages(),
        "what the Rust read answers"
    );
    assert_eq!(
        rust_read::<Terminal>(&serde_json::to_string(&sound_terminal()).unwrap()),
        Vec::<String>::new(),
        "what the Rust read answers"
    );
    assert!(
        !serde_json::to_string(&sound_terminal())
            .unwrap()
            .contains("spareParcel"),
        "serde writes no key for an absent spare parcel"
    );
    let Some(wrote) = ran(
        "dart",
        RUNTIME_VAR,
        "dart",
        "sample.dart",
        &module(
            &shape_definitions(),
            &[CHECKS, SHAPE_FUNCTIONS, JUDGED],
            &driver(&cases),
        ),
    ) else {
        return;
    };
    assert_eq!(wrote, SAMPLE);
}

/// The bounds, the nested places, the slots and an enum's own string check, each with what
/// `validate()` answers of it.
fn measured_cases() -> Vec<Case> {
    let gauge = Gauge {
        code_list: Vec::new(),
        item_count: 0,
        item_label: "A".to_owned(),
        short_note: Some("a".to_owned()),
        sort_mark: None,
        spare_count: Some(10),
        unit_weight: 4.0,
    };
    let yard = Yard {
        by_dock: BTreeMap::new(),
        by_number: BTreeMap::from([(7, unlabelled())]),
        code_list: Some(texts(&["a", "b", "c"])),
        grid: vec![vec![sound(), unlabelled()]],
        parcel_list: vec![parcel("unknown", &["x"], &["y"])],
        spares: vec![None, Some(unlabelled())],
    };
    let deck = Deck {
        deck_pair: ("a".to_owned(), unlabelled()),
    };
    let inked = Stamp::Inked {
        ink: "b".to_owned(),
        pages: 0,
    };
    vec![
        compared("a gauge", &gauge, gauge.validate(), "Gauge.fromJson(@)"),
        compared("a yard", &yard, yard.validate(), "Yard.fromJson(@)"),
        pinned(
            "Pair, each slot broken",
            &Pair(String::new(), 0, unlabelled()),
            &[
                "'0': too short: minimum length is 1, got 0",
                "'1': too small: minimum is 1, got 0",
                "'2.label': too short: minimum length is 1, got 0",
            ],
            "Pair.fromJson(@)",
        ),
        pinned(
            "Deck, a tuple in a field",
            &deck,
            &["'deckPair.1.label': too short: minimum length is 1, got 0"],
            "Deck.fromJson(@)",
        ),
        pinned(
            "a short ink with no vowel",
            &inked,
            &[
                "'ink': too short: minimum length is 2, got 1",
                "'ink': does not match pattern '[aeiou]'",
            ],
            "Stamp.fromJson(@)",
        ),
    ]
}

/// Every case `validate()` is run over, read with `fromJson`: the read is refused with the list
/// `validate()` answers of the value, and reads the value that list is empty for.
#[test]
fn each_shape_is_refused_with_what_its_validate_answers() {
    let over: Vec<Case> = [
        table_cases(),
        brand_cases(),
        named_shape_cases(),
        named_variant_cases(),
        tagged_and_untagged_variant_cases(),
    ]
    .concat();
    let mut cases: Vec<Case> = over.iter().filter_map(judged_case).collect();
    assert_eq!(
        over.len() - cases.len(),
        2,
        "the cases built in code, which no read answers"
    );
    cases.extend(measured_cases());
    let Some(wrote) = ran(
        "dart",
        RUNTIME_VAR,
        "dart",
        "shapes_read.dart",
        &module(
            &shape_definitions(),
            &[CHECKS, SHAPE_FUNCTIONS, JUDGED],
            &driver(&cases),
        ),
    ) else {
        return;
    };
    assert_eq!(wrote, printed_for(&cases));
}

/// One payload a [`Sticker`] is read from, with what the Rust read answers of it: the class of
/// the variant it takes, or the messages it is refused with.
fn stuck(name: &'static str, payload: &str) -> (Case, String) {
    let taken = serde_json::from_str::<Sticker>(payload);
    let answered = mapped_or_default(taken.as_ref().err(), |refused| vec![refused.to_string()]);
    let line = taken.map_or_else(
        |_| {
            format!(
                "{name}: refused (FormatException)\n    {}\n",
                answered.join("; ")
            )
        },
        |read| format!("{name}: {}\n", class_of(&read)),
    );
    (
        Case {
            answered,
            built: format!("Sticker.fromJson(jsonDecode(r'{payload}'))"),
            name,
        },
        line,
    )
}

/// A payload that breaks the bound of the first variant it fits is the next one, as serde reads
/// it: a member's own bound and a brand's own bound choose. A model that breaks its rule and a
/// brand that breaks its check are the variant they fit, and the enum's read refuses them.
#[test]
fn an_untagged_enum_takes_the_variant_serde_takes() {
    let rows = [
        stuck("a labelled carton", r#"{"label":"A"}"#),
        stuck("an unlabelled carton", r#"{"label":""}"#),
        stuck("a number the bound holds", "3"),
        stuck("a number the bound refuses", "0"),
        stuck("a tray", r#""TR""#),
        stuck("a text the bound of a tray refuses", r#""T""#),
        stuck("a tray its check refuses", r#""XX""#),
    ];
    let answered: String = rows.iter().map(|(_, line)| line.as_str()).collect();
    assert_eq!(
        answered,
        "\
a labelled carton: StickerBoxed
an unlabelled carton: refused (FormatException)
    'label': too short: minimum length is 1, got 0
a number the bound holds: StickerPositive
a number the bound refuses: StickerWhole
a tray: StickerTrayed
a text the bound of a tray refuses: StickerWritten
a tray its check refuses: refused (FormatException)
    a tray is known
",
        "what the Rust read answers"
    );
    let mut entry = String::from(
        "void taken(String name, Object Function() read) {\n  try {\n    print('$name: \
         ${read().runtimeType}');\n  } on FormatException catch (refused) {\n    print('$name: \
         refused (FormatException)');\n    print('    ${refused.message}');\n  }\n}\n\nvoid \
         main() {\n",
    );
    for (held, _) in &rows {
        writeln!(entry, "  taken('{}', () => {});", held.name, held.built).unwrap();
    }
    entry.push_str("}\n");
    let definitions = [
        carton_dart::dart_definition(),
        tray_dart::dart_definition(),
        sticker_dart::dart_definition(),
    ];
    let Some(wrote) = ran(
        "dart",
        RUNTIME_VAR,
        "dart",
        "sticker.dart",
        &module(&definitions, &[KNOWN_TRAY], &entry),
    ) else {
        return;
    };
    assert_eq!(wrote, answered);
}

/// `validate()` of a rack runs the tray's rules beside the member's, so the rack reads the tray
/// unjudged. Rust and Dart answer the same messages, the field's key in front of each.
#[test]
fn a_brand_under_a_members_own_bound_judges_itself_as_it_is_read() {
    let (known, short) = (r#"{"tray":"XX"}"#, r#"{"tray":"T"}"#);
    assert_eq!(
        rust_read::<Rack>(known),
        ["'tray': a tray is known"],
        "what the Rust read answers"
    );
    assert_eq!(
        rust_read::<Rack>(short),
        ["'tray': too short: minimum length is 2, got 1"],
        "what the Rust read answers"
    );
    let held = Rack {
        tray: Tray("XX".to_owned()),
    };
    assert_eq!(
        held.validate(),
        Err(vec!["'tray': a tray is known".to_owned()]),
        "what Rust's `validate()` answers"
    );
    let cases = [
        Case {
            answered: texts(&["'tray': a tray is known"]),
            built: format!("Rack.fromJson(jsonDecode(r'{known}'))"),
            name: "a rack of a known tray",
        },
        Case {
            answered: texts(&["'tray': too short: minimum length is 2, got 1"]),
            built: format!("Rack.fromJson(jsonDecode(r'{short}'))"),
            name: "a rack of a short tray",
        },
        pinned(
            "a rack of a tray",
            &Rack {
                tray: Tray("TR".to_owned()),
            },
            &[],
            "Rack.fromJson(@)",
        ),
    ];
    let definitions = [tray_dart::dart_definition(), rack_dart::dart_definition()];
    let Some(wrote) = ran(
        "dart",
        RUNTIME_VAR,
        "dart",
        "rack.dart",
        &module(&definitions, &[KNOWN_TRAY, JUDGED], &driver(&cases)),
    ) else {
        return;
    };
    assert_eq!(wrote, printed_for(&cases));
}

/// A field named `fromJsonUnjudged` keeps its name beside the constructor, and a member of a
/// plain enum moves off the name of the function. Both keep what serde writes.
#[test]
fn a_member_named_after_the_unjudged_read_builds_and_keeps_its_key() {
    let entry = "void main() {\n  final ledger = \
                 Ledger.fromJson(jsonDecode(r'{\"fromJsonUnjudged\":true}'));\n  \
                 print(ledger.fromJsonUnjudged);\n  print(jsonEncode(ledger.toJson()));\n  \
                 print(Stage.fromJson('FromJsonUnjudged') == Stage.fromJsonUnjudged_);\n  \
                 print(jsonEncode(Stage.fromJsonUnjudged('FromJsonUnjudged').toJson()));\n}\n";
    assert_eq!(
        serde_json::to_string(&[Stage::FromJsonUnjudged, Stage::Judged]).unwrap(),
        r#"["FromJsonUnjudged","Judged"]"#,
        "the values serde writes"
    );
    let Some(wrote) = ran(
        "dart",
        RUNTIME_VAR,
        "dart",
        "named.dart",
        &module(
            &[LEDGER.trim_end().to_owned(), stage_dart::dart_definition()],
            &[],
            entry,
        ),
    ) else {
        return;
    };
    assert_eq!(
        wrote,
        "true\n{\"fromJsonUnjudged\":true}\ntrue\n\"FromJsonUnjudged\"\n"
    );
}

/// The emitted source of the shapes declared here, with every shape.
#[test]
fn the_emitted_source_of_the_reads_has_no_issue() {
    let mut definitions = shape_definitions();
    definitions.extend([
        carton_dart::dart_definition(),
        tray_dart::dart_definition(),
        sticker_dart::dart_definition(),
        stage_dart::dart_definition(),
        LEDGER.trim_end().to_owned(),
    ]);
    let source = module(
        &definitions,
        &[CHECKS, SHAPE_FUNCTIONS, KNOWN_TRAY, JUDGED],
        &driver(&sample_cases()[..1]),
    );
    let Some((clean, said)) = dart("analyze-reads", &["analyze"], &source) else {
        return;
    };
    assert!(clean, "{said}\n--- source ---\n{source}");
    assert!(said.contains("No issues found!"), "{said}");
}

/// The library a client of [`ParcelClientService`] is run in: `imports`, the classes it reads,
/// `client` and `entry`.
fn client_module(imports: &[&str], client: String, entry: &str) -> String {
    let mut parts: Vec<String> = imports.iter().map(|&import| import.to_owned()).collect();
    parts.extend([
        parcel_dart::dart_definition(),
        shelf_error_dart::dart_definition(),
        CHECKS.to_owned(),
        ParcelClientServiceSchema::dart_definition(),
        client,
        DESCRIBED.to_owned(),
        entry.to_owned(),
    ]);
    parts.join("\n\n")
}

/// What a client answers for a reply that keeps every rule, and for one that breaks a bound,
/// where it answers the latter with a fault of `kind`.
fn answers(kind: &str) -> serde_json::Value {
    serde_json::json!({
        "sound": {"kind": "ok", "label": "A"},
        "unlabelled": {
            "kind": "fault",
            "faultKind": kind,
            "detail": "FormatException: 'label': too short: minimum length is 1, got 0",
        },
    })
}

/// The reply both clients are handed below is one the Rust back end writes: writing judges
/// nothing.
#[test]
fn the_back_end_writes_the_reply_that_breaks_a_rule() {
    let mut weighing = pin!(Desk.weigh(&(), String::new()));
    let mut polling = PollContext::from_waker(Waker::noop());
    let answered = match weighing.as_mut().poll(&mut polling) {
        Poll::Ready(answer) => Some(answer),
        Poll::Pending => None,
    };
    assert_eq!(
        serde_json::to_string(&answered.unwrap().unwrap()).unwrap(),
        r#"{"fragileItemCodes":["x"],"itemCodeList":["x"],"label":""}"#
    );
}

/// The HTTP client answers a reply that breaks a rule with the fault it answers a body it
/// cannot read with.
#[test]
fn the_http_client_answers_a_fault_for_a_reply_that_breaks_a_rule() {
    assert_eq!(
        rust_read::<Parcel>(r#"{"label":"","itemCodeList":["x"],"fragileItemCodes":["x"]}"#),
        ["'label': too short: minimum length is 1, got 0"],
        "what the Rust read answers of the reply"
    );
    let Some(wrote) = ran(
        "dart",
        RUNTIME_VAR,
        "dart",
        "desk_http.dart",
        &client_module(
            &["import 'dart:convert';"],
            ParcelClientServiceSchema::dart_http_client(),
            HTTP_DRIVER,
        ),
    ) else {
        return;
    };
    let written: serde_json::Value = serde_json::from_str(wrote.trim()).unwrap();
    assert_eq!(written, answers("undeserializablePayload"));
}

/// The WebSocket client answers a reply that breaks a rule with the fault it answers a value it
/// cannot read with.
#[test]
fn the_ws_client_answers_a_fault_for_a_reply_that_breaks_a_rule() {
    let Some(wrote) = ran(
        "dart",
        RUNTIME_VAR,
        "dart",
        "desk_ws.dart",
        &client_module(
            &["import 'dart:async';", "import 'dart:convert';"],
            ParcelClientServiceSchema::dart_ws_client(),
            WS_DRIVER,
        ),
    ) else {
        return;
    };
    let written: serde_json::Value = serde_json::from_str(wrote.trim()).unwrap();
    assert_eq!(written, answers("failedValidation"));
}

/// The README's section on the Dart read, held to what is declared, emitted and printed here.
/// The emitter writes a class on one line and the README breaks it, so the two are compared
/// with their white space taken out.
#[test]
fn the_readme_shows_the_reads_that_are_emitted_and_what_they_print() {
    let section = from_until(
        include_str!("../../README.md"),
        "#### The Dart read refuses what `validate()` refuses",
        "#### What stays yours",
    );
    let declared = from_until(section, "```rust\n", "\n```")
        .strip_prefix("```rust\n")
        .unwrap();
    assert!(
        include_str!("run_dart_validate.rs").contains(declared),
        "the README declares what the tests do not: {declared}"
    );
    let packed = |text: &str| text.split_whitespace().collect::<String>();
    let shown = packed(section);
    let parcel_class = parcel_dart::dart_definition();
    let manifest_class = manifest_dart::dart_definition();
    let closes = " Map<String, dynamic> toJson()";
    for reads in [
        from_until(&parcel_class, "factory Parcel.fromJson(", closes),
        from_until(
            &manifest_class,
            "factory Manifest.fromJsonUnjudged(",
            closes,
        ),
    ] {
        assert!(
            shown.contains(&packed(reads)),
            "the README does not show: {reads}"
        );
    }
    assert!(section.contains(SAMPLE), "{section}");
    for kind in ["`undeserializablePayload`", "`failedValidation`"] {
        assert!(section.contains(kind), "{section}");
    }
}
