//! A holder reads what fills a type parameter of a generic model it holds with no judgement and
//! judges the whole value once: the emitted Dart, Kotlin and Swift reads of a holder of each shape
//! answer the messages the Rust read answers, each broken value named at its full path. Each
//! program is compiled with warnings as errors and run.

#![cfg(all(feature = "dart", feature = "kotlin", feature = "swift"))]

use alloc::collections::BTreeMap;
use core::fmt::Write as _;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::run_dart_validate::{analyzed, dart, module};
use super::runtime::{judged, ran_kotlin};

/// A parcel that keeps every rule.
const SOUND: &str = r#"{"fragileItemCodes":[],"itemCodeList":[],"itemLabel":"a"}"#;

/// A parcel that breaks its bound and its check.
const BROKEN: &str = r#"{"fragileItemCodes":["y"],"itemCodeList":[],"itemLabel":""}"#;

/// A parcel that breaks its bound alone.
const UNLABELLED: &str = r#"{"fragileItemCodes":[],"itemCodeList":[],"itemLabel":""}"#;

/// The checks of the models, in Dart.
const DART_CHECKS: &str = r#"
List<String> withinBinParcel(BinParcelDraft value) => [
  for (final (index, code) in value.fragile_item_codes.indexed)
    if (!value.item_code_list.contains(code))
      "'fragileItemCodes.$index': $code is outside of itemCodeList",
];

List<String> binLabelInUpperCase(BinLabel value) =>
    value.value.toUpperCase() != value.value ? ['a label is upper case'] : [];

List<String> atMostTwoBinBranches<Id>(BinBranchDraft<Id> value) =>
    value.branches.length > 2 ? ["'branches': a branch holds at most two"] : [];
"#;

/// The checks of the models, in Kotlin.
const KOTLIN_CHECKS: &str = r#"
fun withinBinParcel(value: BinParcelDraft): List<String> = value.fragileItemCodes.withIndex()
    .filter { !value.itemCodeList.contains(it.value) }
    .map { "'fragileItemCodes.${it.index}': ${it.value} is outside of itemCodeList" }

fun binLabelInUpperCase(value: BinLabel): List<String> =
    if (value.value.any { it.isLowerCase() }) listOf("a label is upper case") else emptyList()

fun <Id> atMostTwoBinBranches(value: BinBranchDraft<Id>): List<String> =
    if (value.branches.size > 2) listOf("'branches': a branch holds at most two") else emptyList()
"#;

/// The checks of the models, in Swift.
const SWIFT_CHECKS: &str = r#"
func withinBinParcel(_ value: BinParcelDraft) -> [String] {
  value.fragileItemCodes.enumerated()
    .filter { !value.itemCodeList.contains($0.element) }
    .map { "'fragileItemCodes.\($0.offset)': \($0.element) is outside of itemCodeList" }
}

func binLabelInUpperCase(_ value: BinLabel) -> [String] {
  value.value.contains { $0.isLowercase } ? ["a label is upper case"] : []
}

func atMostTwoBinBranches<Id>(_ value: BinBranchDraft<Id>) -> [String] {
  value.branches.count > 2 ? ["'branches': a branch holds at most two"] : []
}
"#;

/// A model with a bound and a check.
#[model_schema(check = ["withinBinParcel"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BinParcel {
    pub fragile_item_codes: Vec<String>,
    pub item_code_list: Vec<String>,
    #[model_schema_prop(minLength = 1)]
    pub item_label: String,
}

/// A string brand with a bound and a check.
#[model_schema(minLength = 2, check = ["binLabelInUpperCase"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct BinLabel(String);

/// A generic struct with a rule of its own.
#[model_schema(default_types(Item = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BinPage<Item> {
    pub items: Vec<Item>,
    #[model_schema_prop(minLength = 1)]
    pub page_title: String,
}

/// A generic enum with no rule of its own.
#[model_schema(default_types(Item = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum BinSlot<Item> {
    Empty,
    Filled { held: Item, spares: Vec<Item> },
    Single(Item),
}

/// A generic struct that holds another at its own parameter, and a map of it.
#[model_schema(default_types(Held = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BinCabinet<Held> {
    pub by_dock: BTreeMap<String, Held>,
    pub page: BinPage<Held>,
}

/// Two parameters.
#[model_schema(default_types(First = String, Second = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BinPair<First, Second> {
    pub first: First,
    pub second: Vec<Second>,
}

/// A brand over its own parameter.
#[model_schema(default_types(Inner = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct BinWrapper<Inner>(Inner);

/// A generic type that holds itself, written as `Self`, with a check of its own.
#[model_schema(default_types(Id = String), check = ["atMostTwoBinBranches"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BinBranch<Id> {
    pub branches: Vec<Self>,
    pub id: Id,
}

/// An untagged generic enum, whose variant serde chooses by what reads.
#[model_schema(default_types(Item = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BinEither<Item> {
    Labelled(Item),
    Plain(String),
}

/// The holder of each shape, every member written with what fills the parameters.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BinShelf {
    pub binder: BinPage<BinPage<BinParcel>>,
    pub branch: BinBranch<BinParcel>,
    pub cabinet: BinCabinet<BinParcel>,
    pub either: BinEither<BinLabel>,
    pub first_page: BinPage<BinParcel>,
    pub labels: BinPage<BinLabel>,
    pub lists: BinPage<Vec<BinParcel>>,
    pub pair: BinPair<BinParcel, BinLabel>,
    pub slot: BinSlot<BinParcel>,
    pub stack: Vec<BinPage<BinParcel>>,
    pub words: BinPage<String>,
    pub wrapped: BinWrapper<BinParcel>,
}

fn within_bin_parcel(value: &BinParcelDraft) -> Result<(), Vec<String>> {
    let outside: Vec<String> = value
        .fragile_item_codes
        .iter()
        .enumerate()
        .filter(|(_, code)| !value.item_code_list.contains(code))
        .map(|(index, code)| {
            format!("'fragileItemCodes.{index}': {code} is outside of itemCodeList")
        })
        .collect();
    if outside.is_empty() {
        Ok(())
    } else {
        Err(outside)
    }
}

fn bin_label_in_upper_case(value: &BinLabel) -> Result<(), Vec<String>> {
    if value.as_inner().chars().any(char::is_lowercase) {
        return Err(vec!["a label is upper case".to_owned()]);
    }
    Ok(())
}

fn at_most_two_bin_branches<Id>(value: &BinBranchDraft<Id>) -> Result<(), Vec<String>> {
    if value.branches.len() > 2 {
        return Err(vec!["'branches': a branch holds at most two".to_owned()]);
    }
    Ok(())
}

fn page(items: &[&str]) -> String {
    format!(r#"{{"items":[{}],"pageTitle":"t"}}"#, items.join(","))
}

/// A shelf every member of which keeps every rule, but the one `changed` writes.
fn shelf(changed: Option<(&str, &str)>) -> String {
    let members: Vec<(&str, String)> = vec![
        ("binder", page(&[])),
        ("branch", format!(r#"{{"branches":[],"id":{SOUND}}}"#)),
        (
            "cabinet",
            format!(r#"{{"byDock":{{}},"page":{}}}"#, page(&[])),
        ),
        ("either", r#""AB""#.to_owned()),
        ("firstPage", page(&[SOUND])),
        ("labels", page(&[])),
        ("lists", page(&[])),
        ("pair", format!(r#"{{"first":{SOUND},"second":[]}}"#)),
        ("slot", r#""empty""#.to_owned()),
        ("stack", "[]".to_owned()),
        ("words", page(&[])),
        ("wrapped", SOUND.to_owned()),
    ];
    let written: Vec<String> = members
        .iter()
        .map(|(key, value)| {
            let held = changed
                .filter(|(at, _)| at == key)
                .map_or(value.as_str(), |(_, written)| written);
            format!(r#""{key}":{held}"#)
        })
        .collect();
    format!("{{{}}}", written.join(","))
}

/// Each case, its name and the payload of a shelf broken in one member.
fn cases() -> Vec<(&'static str, String)> {
    let binder = page(&[&page(&[SOUND, UNLABELLED]), &page(&[UNLABELLED])]);
    let branch =
        format!(r#"{{"branches":[{{"branches":[],"id":{UNLABELLED}}}],"id":{UNLABELLED}}}"#);
    let leaf = format!(r#"{{"branches":[],"id":{SOUND}}}"#);
    let crowded = format!(r#"{{"branches":[{leaf},{leaf},{leaf}],"id":{SOUND}}}"#);
    let cabinet = format!(
        r#"{{"byDock":{{"north":{UNLABELLED}}},"page":{}}}"#,
        page(&[UNLABELLED])
    );
    let lists = page(&[&format!("[{SOUND},{UNLABELLED}]")]);
    let pair = format!(r#"{{"first":{UNLABELLED},"second":["OK","b"]}}"#);
    let slot = format!(r#"{{"filled":{{"held":{UNLABELLED},"spares":[{SOUND},{UNLABELLED}]}}}}"#);
    let stack = format!("[{},{}]", page(&[SOUND]), page(&[UNLABELLED]));
    [
        ("sound", None),
        ("binder", Some(("binder", binder))),
        ("branch", Some(("branch", branch))),
        ("crowded", Some(("branch", crowded))),
        ("cabinet", Some(("cabinet", cabinet))),
        ("either", Some(("either", r#""a""#.to_owned()))),
        ("eitherLower", Some(("either", r#""ab""#.to_owned()))),
        (
            "firstPage",
            Some(("firstPage", page(&[UNLABELLED, SOUND, BROKEN]))),
        ),
        (
            "labels",
            Some(("labels", page(&[r#""a""#, r#""OK""#, r#""b""#]))),
        ),
        ("lists", Some(("lists", lists))),
        ("pair", Some(("pair", pair))),
        ("slot", Some(("slot", slot))),
        ("stack", Some(("stack", stack))),
        (
            "words",
            Some(("words", r#"{"items":["a"],"pageTitle":""}"#.to_owned())),
        ),
        ("wrapped", Some(("wrapped", BROKEN.to_owned()))),
    ]
    .into_iter()
    .map(|(name, changed)| {
        let payload = shelf(
            changed
                .as_ref()
                .map(|(at, written)| (*at, written.as_str())),
        );
        (name, payload)
    })
    .collect()
}

/// What the Rust read answers of each case, one line per message.
fn expected() -> String {
    cases()
        .iter()
        .fold(String::new(), |mut lines, (name, payload)| {
            match serde_json::from_str::<BinShelf>(payload) {
                Ok(_) => {
                    let _ = writeln!(lines, "{name}: read");
                }
                Err(refused) => {
                    for message in refused.to_string().split("; ") {
                        let _ = writeln!(lines, "{name}: refused: {message}");
                    }
                }
            }
            lines
        })
}

/// The calls of each case in a program, `call` writing one from its name and its payload.
fn calls(call: &dyn Fn(&str, &str) -> String) -> String {
    cases()
        .iter()
        .map(|(name, payload)| call(name, payload))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_holder_answers_in_rust_every_broken_value_at_its_full_path() {
    let answered = expected();
    for line in [
        "sound: read",
        "binder: refused: 'binder.items.0.items.1.itemLabel': too short: minimum length is 1, got 0",
        "either: refused: 'either': too short: minimum length is 2, got 1",
        "slot: refused: 'slot.filled.spares.1.itemLabel': too short: minimum length is 1, got 0",
        "wrapped: refused: 'wrapped.fragileItemCodes.0': y is outside of itemCodeList",
    ] {
        assert!(answered.contains(line), "no `{line}` in:\n{answered}");
    }
}

#[test]
fn the_dart_read_of_a_holder_answers_what_the_rust_read_answers() {
    let definitions = vec![
        bin_parcel_dart::dart_definition(),
        bin_label_dart::dart_definition(),
        bin_page_dart::dart_definition(),
        bin_slot_dart::dart_definition(),
        bin_cabinet_dart::dart_definition(),
        bin_pair_dart::dart_definition(),
        bin_wrapper_dart::dart_definition(),
        bin_branch_dart::dart_definition(),
        bin_either_dart::dart_definition(),
        bin_shelf_dart::dart_definition(),
    ];
    let entry = format!(
        "void row(String name, String payload) {{\n  try {{\n    BinShelf.fromJson(jsonDecode(payload) as Map<String, dynamic>);\n    print('$name: read');\n  }} on FormatException catch (refused) {{\n    for (final message in refused.message.split('; ')) {{\n      print('$name: refused: $message');\n    }}\n  }}\n}}\n\nvoid main() {{\n{}\n}}\n",
        calls(&|name, payload| format!("  row('{name}', r'{payload}');"))
    );
    let source = module(&definitions, &[DART_CHECKS], &entry);
    analyzed("parameter-holders-analyze", &source);
    let Some((ran, printed)) = dart("parameter-holders", &[], &source) else {
        return;
    };
    assert!(ran, "{printed}");
    assert_eq!(printed, expected());
}

#[test]
fn the_kotlin_read_of_a_holder_answers_what_the_rust_read_answers() {
    let mut parts = vec![
        "import kotlinx.serialization.*\nimport kotlinx.serialization.json.*\nimport kotlinx.serialization.descriptors.*\nimport kotlinx.serialization.encoding.*\nimport kotlinx.serialization.builtins.*".to_owned(),
        bin_parcel_kotlin::kotlin_definition(),
        bin_label_kotlin::kotlin_definition(),
        bin_page_kotlin::kotlin_definition(),
        bin_slot_kotlin::kotlin_definition(),
        bin_cabinet_kotlin::kotlin_definition(),
        bin_pair_kotlin::kotlin_definition(),
        bin_wrapper_kotlin::kotlin_definition(),
        bin_branch_kotlin::kotlin_definition(),
        bin_either_kotlin::kotlin_definition(),
        bin_shelf_kotlin::kotlin_definition(),
        KOTLIN_CHECKS.to_owned(),
    ];
    parts.push(format!(
        "fun row(name: String, payload: String) {{\n    try {{\n        Json.decodeFromString<BinShelf>(payload)\n        println(\"$name: read\")\n    }} catch (refused: SerializationException) {{\n        (refused.message ?: \"\").split(\"; \").forEach {{ println(\"$name: refused: $it\") }}\n    }}\n}}\n\nfun main() {{\n{}\n}}\n",
        calls(&|name, payload| format!("    row(\"{name}\", \"\"\"{payload}\"\"\")"))
    ));
    let Some(printed) = ran_kotlin(&parts.join("\n\n")) else {
        return;
    };
    assert_eq!(printed, expected());
}

#[test]
fn the_swift_read_of_a_holder_answers_what_the_rust_read_answers() {
    let definitions = [
        bin_parcel_swift::swift_definition(),
        bin_label_swift::swift_definition(),
        bin_page_swift::swift_definition(),
        bin_slot_swift::swift_definition(),
        bin_cabinet_swift::swift_definition(),
        bin_pair_swift::swift_definition(),
        bin_wrapper_swift::swift_definition(),
        bin_branch_swift::swift_definition(),
        bin_either_swift::swift_definition(),
        bin_shelf_swift::swift_definition(),
    ];
    let source = format!(
        "import Foundation\n\n{}\n{SWIFT_CHECKS}\nfunc row(_ name: String, _ payload: String) {{\n  do {{\n    _ = try JSONDecoder().decode(BinShelf.self, from: Data(payload.utf8))\n    print(\"\\(name): read\")\n  }} catch DecodingError.dataCorrupted(let context) {{\n    for message in context.debugDescription.components(separatedBy: \"; \") {{\n      print(\"\\(name): refused: \\(message)\")\n    }}\n  }} catch {{\n    print(\"\\(name): refused: \\(error)\")\n  }}\n}}\n\n{}\n",
        definitions.join("\n\n"),
        calls(&|name, payload| format!("row(\"{name}\", #\"{payload}\"#)"))
    );
    let Some(run) = judged(
        "parameter-holders",
        "TIXSCHEMA_SWIFT",
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    ) else {
        return;
    };
    assert!(run.succeeded, "{}\n--- source ---\n{source}", run.stderr);
    assert_eq!(run.stdout, expected());
}
