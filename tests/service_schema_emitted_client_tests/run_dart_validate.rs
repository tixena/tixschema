//! The `validate()` the Dart backend emits, run by the Dart VM.
//!
//! A value held under a named field, in a list, in a map or in an optional is read in Dart, with
//! `fromJsonUnjudged`, from the payload serde writes for it, and what Dart answers is compared
//! with what Rust's `validate()` answers of the same value. A slot and a member of a variant are
//! named by the steps serde writes on the way to them, and their messages are written out here.

#![cfg(feature = "dart")]

use alloc::collections::BTreeMap;
use core::fmt::Write as _;
use std::env;
use std::fs;
use std::process::{Command, id};

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::runtime::ran;

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_DART";

/// The functions the checks of the shared table's model name, written by hand as a consumer
/// writes them, and as the README shows them.
pub const CHECKS: &str = r#"
List<String> withoutDuplicates(List<String> value) {
  final seen = <String>{};
  for (final code in value) {
    if (!seen.add(code)) return ['duplicate item code $code'];
  }
  return const [];
}

List<String> withinParcel(Parcel value) => [
      for (final (index, code) in value.fragile_item_codes.indexed)
        if (!value.item_code_list.contains(code)) "'fragileItemCodes.$index': $code is outside of itemCodeList",
    ];
"#;

/// What prints a case: its name and whether it passes, then each message it is refused with.
const REPORT: &str = "
void report(String name, List<String> violations) {
  if (violations.isEmpty) {
    print('$name: passes');
    return;
  }
  print('$name: refused');
  for (final violation in violations) {
    print('    $violation');
  }
}
";

/// The functions the checks of the shapes name.
pub const SHAPE_FUNCTIONS: &str = r#"
List<String> upperCase(Code value) =>
    value.value.toUpperCase() == value.value ? const [] : const ['a code is upper case'];

List<String> routed(Freight value) =>
    value is FreightHeld ? const ['a held freight has no route'] : const [];

List<String> rated(Rating value) =>
    value == Rating.unrated ? const ['a parcel is rated'] : const [];

List<String> withACode<CodeType>(Crate<CodeType> value) =>
    value.codes.isEmpty ? const ["'codes': a crate holds at least one code"] : const [];

List<String> takingParcels(ClosedDock value) => const ['a closed dock takes no parcel'];

List<String> shortList(List<String> value) =>
    value.length > 2 ? ["'2': ${value[2]} is one code too many"] : const [];

List<String> knownParcels(List<Parcel> value) =>
    value.isNotEmpty && value.first.label == 'unknown' ? const ["'0': the first parcel is known"] : const [];
"#;

/// The 13 lines of the shared table, a member's lines in the order the parcel declares it.
const TABLE: &str = "\
case 1: passes
case 2: refused
    'label': too short: minimum length is 1, got 0
case 3: refused
    'itemCodeList': duplicate item code x
case 4: refused
    'fragileItemCodes.1': y is outside of itemCodeList
case 5: refused
    'itemCodeList': duplicate item code x
    'label': too short: minimum length is 1, got 0
    'fragileItemCodes.0': y is outside of itemCodeList
case 6: refused
    'parcel.fragileItemCodes.1': y is outside of itemCodeList
";

/// One value: the Dart expression that builds it, and what `validate()` answers of it.
#[derive(Clone)]
pub struct Case {
    pub answered: Vec<String>,
    pub built: String,
    pub name: &'static str,
}

/// The model of the shared table: a bound, a field's check and the type's check.
#[model_schema(check = ["withinParcel"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Parcel {
    pub fragile_item_codes: Vec<String>,
    #[model_schema_prop(check = ["withoutDuplicates"])]
    pub item_code_list: Vec<String>,
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

/// The holder of the shared table's last case.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Manifest {
    pub parcel: Parcel,
}

/// A brand with a string check and a check of its own.
#[model_schema(minLength = 2, pattern = "^[A-Z]+$", check = ["upperCase"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Code(pub String);

/// A brand with no rule, which a field's own bound measures.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Mark(pub String);

/// One field per key a bound is written with, on each kind of value it measures.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gauge {
    #[model_schema_prop(minLength = 1, maxLength = 3)]
    pub code_list: Vec<String>,
    #[model_schema_prop(minimum = 1, maximum = 120)]
    pub item_count: u32,
    #[model_schema_prop(minLength = 2, maxLength = 8, pattern = "^[a-z]+$")]
    pub item_label: String,
    #[model_schema_prop(minLength = 2)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_note: Option<String>,
    #[model_schema_prop(minLength = 1)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_mark: Option<Mark>,
    #[model_schema_prop(maximum = 9)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare_count: Option<u32>,
    #[model_schema_prop(minimum = 0.5, maximum = 3.0)]
    pub unit_weight: f64,
}

/// A string held to one character.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Initial {
    #[model_schema_prop(maxLength = 1)]
    pub letter: String,
}

/// The name a dock is keyed by.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Dock {
    North,
    South,
}

/// A model held in a field, a list, a map and an optional.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Terminal {
    pub first_parcel: Parcel,
    pub lists_by_dock: BTreeMap<String, Vec<Parcel>>,
    pub parcel_list: Vec<Parcel>,
    pub parcels_by_dock: BTreeMap<String, Parcel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare_parcel: Option<Parcel>,
}

/// A model held under each place a list and a map nest it at, and a member with a count, a check
/// and items of its own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Yard {
    pub by_dock: BTreeMap<Dock, Parcel>,
    pub by_number: BTreeMap<u32, Parcel>,
    #[model_schema_prop(check = ["shortList"])]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_list: Option<Vec<String>>,
    pub grid: Vec<Vec<Parcel>>,
    #[model_schema_prop(minLength = 2, check = ["knownParcels"])]
    pub parcel_list: Vec<Parcel>,
    pub spares: Vec<Option<Parcel>>,
}

/// A member serde writes no key for, beside one it writes under a key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Docket {
    pub docket_code: String,
    #[serde(flatten)]
    pub held_parcel: Parcel,
}

/// A brand with a parameter, whose string check is its declared default's.
#[model_schema(minLength = 2, default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Tag<IdType>(pub IdType);

/// A brand over a list, whose lengths count its items.
#[model_schema(no_display, minLength = 1, maxLength = 2)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Batch(pub Vec<String>);

/// A type with a parameter, whose check is written once over every filling.
#[model_schema(default_types(CodeType = String), check = ["withACode"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Crate<CodeType> {
    pub codes: Vec<CodeType>,
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

/// A unit struct with a check.
#[model_schema(check = ["takingParcels"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ClosedDock;

/// An enum no variant of which holds a value, with a check.
#[model_schema(check = ["rated"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Rating {
    Express,
    Unrated,
}

/// A field named after the method every emitted type has.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Audit {
    pub validate: bool,
}

/// An alias of a model.
#[model_schema()]
pub type Consignment = Parcel;

/// A tuple struct of several slots, one of which holds a model.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Pair(
    #[model_schema_prop(minLength = 1)] pub String,
    #[model_schema_prop(minimum = 1)] pub u32,
    pub Parcel,
);

/// A struct of one slot, which serde writes as the slot's value.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Sleeve(#[model_schema_prop(minLength = 1)] pub String);

/// A tuple written as a field's type.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Deck {
    pub deck_pair: (String, Parcel),
}

/// An enum serde writes under its variant's name.
#[model_schema(check = ["routed"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all_fields = "camelCase")]
pub enum Freight {
    Boxed(Parcel),
    #[serde(rename = "crated")]
    Crated(Parcel),
    Held,
    Loose {
        #[model_schema_prop(minLength = 1)]
        freight_label: String,
    },
    Sealed(#[model_schema_prop(minLength = 1)] String),
    Split(
        #[model_schema_prop(minLength = 1)] String,
        #[model_schema_prop(minimum = 1)] u32,
    ),
}

/// An enum whose variants serde writes in lower case.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Dispatch {
    ByRoad {
        #[model_schema_prop(minLength = 1)]
        road: String,
    },
}

/// An enum under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "held", rename_all_fields = "camelCase")]
pub enum Haul {
    Boxed(#[model_schema_prop(minLength = 1)] String),
    Loose {
        #[model_schema_prop(minLength = 1)]
        haul_label: String,
    },
    Split(
        #[model_schema_prop(minLength = 1)] String,
        #[model_schema_prop(minimum = 1)] u32,
    ),
}

/// An enum under a tag alone.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all_fields = "camelCase")]
pub enum Lift {
    Boxed(Parcel),
    Loose {
        #[model_schema_prop(minLength = 1)]
        lift_label: String,
    },
}

/// An enum serde writes as the value of its variant.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Reference {
    Named {
        #[model_schema_prop(minLength = 1)]
        name: String,
    },
    Numbered(#[model_schema_prop(minimum = 1)] u32),
}

/// An enum whose own string check reaches the fields written as text.
#[model_schema(minLength = 2, pattern = "^[a-z]+$")]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Stamp {
    Inked {
        #[model_schema_prop(maxLength = 9, pattern = "[aeiou]")]
        ink: String,
        pages: u32,
    },
    Pressed {
        seal: String,
    },
}

fn without_duplicates(value: &[String]) -> Result<(), Vec<String>> {
    for (index, code) in value.iter().enumerate() {
        if value[..index].contains(code) {
            return Err(vec![format!("duplicate item code {code}")]);
        }
    }
    Ok(())
}

fn within_parcel(value: &Parcel) -> Result<(), Vec<String>> {
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

fn upper_case(value: &Code) -> Result<(), Vec<String>> {
    if value.0.chars().any(char::is_lowercase) {
        return Err(vec!["a code is upper case".to_owned()]);
    }
    Ok(())
}

fn routed(value: &Freight) -> Result<(), Vec<String>> {
    if matches!(value, Freight::Held) {
        return Err(vec!["a held freight has no route".to_owned()]);
    }
    Ok(())
}

fn rated(value: &Rating) -> Result<(), Vec<String>> {
    if matches!(value, Rating::Unrated) {
        return Err(vec!["a parcel is rated".to_owned()]);
    }
    Ok(())
}

fn with_a_code<CodeType>(value: &Crate<CodeType>) -> Result<(), Vec<String>> {
    if value.codes.is_empty() {
        return Err(vec!["'codes': a crate holds at least one code".to_owned()]);
    }
    Ok(())
}

fn taking_parcels(_value: &ClosedDock) -> Result<(), Vec<String>> {
    Err(vec!["a closed dock takes no parcel".to_owned()])
}

fn short_list(value: &[String]) -> Result<(), Vec<String>> {
    value.get(2).map_or(Ok(()), |third| {
        Err(vec![format!("'2': {third} is one code too many")])
    })
}

fn known_parcels(value: &[Parcel]) -> Result<(), Vec<String>> {
    if value.first().is_some_and(|first| first.label == "unknown") {
        return Err(vec!["'0': the first parcel is known".to_owned()]);
    }
    Ok(())
}

pub fn texts(written: &[&str]) -> Vec<String> {
    written.iter().map(|&text| text.to_owned()).collect()
}

pub fn parcel(label: &str, item_code_list: &[&str], fragile_item_codes: &[&str]) -> Parcel {
    Parcel {
        fragile_item_codes: texts(fragile_item_codes),
        item_code_list: texts(item_code_list),
        label: label.to_owned(),
    }
}

/// A parcel that keeps every rule.
pub fn sound() -> Parcel {
    parcel("A", &["x"], &["x"])
}

/// A parcel with no label.
pub fn unlabelled() -> Parcel {
    parcel("", &["x"], &["x"])
}

/// A parcel whose second fragile code is none of its codes.
pub fn outside() -> Parcel {
    parcel("A", &["x"], &["x", "y"])
}

/// `reader` with the payload serde writes for `value` in the place of its `@`.
fn reading<T>(reader: &str, value: &T) -> String
where
    T: Serialize,
{
    reader.replace(
        '@',
        &format!("jsonDecode(r'{}')", serde_json::to_string(value).unwrap()),
    )
}

/// A case Dart reads from the payload serde writes for `value`, with what Rust's `validate()`
/// answers of that value.
pub fn compared<T>(
    name: &'static str,
    value: &T,
    answered: Result<(), Vec<String>>,
    reader: &str,
) -> Case
where
    T: Serialize,
{
    Case {
        answered: answered.err().unwrap_or_default(),
        built: reading(reader, value),
        name,
    }
}

/// A case Dart reads from the payload serde writes for `value`, with its messages written out.
pub fn pinned<T>(name: &'static str, value: &T, answered: &[&str], reader: &str) -> Case
where
    T: Serialize,
{
    Case {
        answered: texts(answered),
        built: reading(reader, value),
        name,
    }
}

/// What every side prints for `cases`: a case's name and whether it passes, then each message it
/// is refused with on a line of its own.
fn report(cases: &[Case]) -> String {
    let mut printed = String::new();
    for held in cases {
        if held.answered.is_empty() {
            writeln!(printed, "{}: passes", held.name).unwrap();
        } else {
            writeln!(printed, "{}: refused", held.name).unwrap();
            for message in &held.answered {
                writeln!(printed, "    {message}").unwrap();
            }
        }
    }
    printed
}

/// The `main` that builds each of `cases` and prints what its `validate()` answers.
fn driver(cases: &[Case]) -> String {
    let mut written = String::from("void main() {\n");
    for held in cases {
        writeln!(
            written,
            "  report('{}', ({}).validate());",
            held.name, held.built
        )
        .unwrap();
    }
    written.push_str("}\n");
    written
}

/// One Dart library: the emitted `definitions`, the functions their checks name, and `entry`.
pub fn module(definitions: &[String], functions: &[&str], entry: &str) -> String {
    let mut parts = vec!["import 'dart:convert';".to_owned()];
    if definitions
        .iter()
        .any(|written| written.contains("Uint8List"))
    {
        parts.push("import 'dart:typed_data';".to_owned());
    }
    parts.extend(definitions.iter().cloned());
    parts.extend(functions.iter().map(|&written| written.to_owned()));
    parts.push(entry.to_owned());
    parts.join("\n\n")
}

/// The emitted definitions of the shared table's model.
fn table_definitions() -> Vec<String> {
    vec![
        parcel_dart::dart_definition(),
        manifest_dart::dart_definition(),
    ]
}

/// The emitted definitions of every shape.
pub fn shape_definitions() -> Vec<String> {
    let mut definitions = table_definitions();
    definitions.extend([
        code_dart::dart_definition(),
        mark_dart::dart_definition(),
        gauge_dart::dart_definition(),
        initial_dart::dart_definition(),
        dock_dart::dart_definition(),
        yard_dart::dart_definition(),
        tag_dart::dart_definition(),
        batch_dart::dart_definition(),
        crate_dart::dart_definition(),
        closed_dock_dart::dart_definition(),
        rating_dart::dart_definition(),
        audit_dart::dart_definition(),
        consignment_dart::dart_definition(),
        sleeve_dart::dart_definition(),
        deck_dart::dart_definition(),
        freight_dart::dart_definition(),
        dispatch_dart::dart_definition(),
        haul_dart::dart_definition(),
        reference_dart::dart_definition(),
        stamp_dart::dart_definition(),
        pair_dart::dart_definition(),
        terminal_dart::dart_definition(),
        docket_dart::dart_definition(),
        lift_dart::dart_definition(),
    ]);
    definitions
}

/// What the Dart VM prints for `cases` over the shapes, or `None` where no runtime was reachable.
fn printed(named: &str, cases: &[Case]) -> Option<String> {
    ran(
        "dart",
        RUNTIME_VAR,
        "dart",
        named,
        &module(
            &shape_definitions(),
            &[CHECKS, REPORT, SHAPE_FUNCTIONS],
            &driver(cases),
        ),
    )
}

/// Runs `dart` with `arguments` over `source`, in a directory of its own, and answers whether it
/// succeeded with everything it wrote. `None` where no runtime was reachable, which the probe has
/// then said.
pub fn dart(named: &str, arguments: &[&str], source: &str) -> Option<(bool, String)> {
    ran(
        "dart",
        RUNTIME_VAR,
        "dart",
        "reachable.dart",
        "void main() {}",
    )?;
    let at = env::temp_dir().join(format!("tixschema-emitted-dart-{named}-{}", id()));
    if at.exists() {
        fs::remove_dir_all(&at).unwrap();
    }
    fs::create_dir_all(&at).unwrap();
    fs::write(at.join("module.dart"), source).unwrap();
    let runtime = env::var(RUNTIME_VAR).unwrap_or_else(|_| "dart".to_owned());
    let reported = Command::new(runtime)
        .args(arguments)
        .arg("module.dart")
        .current_dir(&at)
        .output()
        .unwrap();
    fs::remove_dir_all(&at).unwrap();
    Some((
        reported.status.success(),
        format!(
            "{}{}",
            String::from_utf8_lossy(&reported.stdout),
            String::from_utf8_lossy(&reported.stderr)
        ),
    ))
}

/// The six cases of the shared table, each with what Rust's `validate()` answers of it.
pub fn table_cases() -> Vec<Case> {
    let parcels = [
        sound(),
        unlabelled(),
        parcel("A", &["x", "x"], &["x"]),
        outside(),
        parcel("", &["x", "x"], &["y"]),
    ];
    let mut cases: Vec<Case> = ["case 1", "case 2", "case 3", "case 4", "case 5"]
        .into_iter()
        .zip(parcels)
        .map(|(name, held)| compared(name, &held, held.validate(), "Parcel.fromJsonUnjudged(@)"))
        .collect();
    let manifest = Manifest { parcel: outside() };
    cases.push(compared(
        "case 6",
        &manifest,
        manifest.validate(),
        "Manifest.fromJsonUnjudged(@)",
    ));
    cases
}

#[test]
fn the_shared_table_prints_the_lines_rust_prints() {
    let cases = table_cases();
    assert_eq!(report(&cases), TABLE, "what Rust's `validate()` answers");
    let Some(wrote) = ran(
        "dart",
        RUNTIME_VAR,
        "dart",
        "table.dart",
        &module(&table_definitions(), &[CHECKS, REPORT], &driver(&cases)),
    ) else {
        return;
    };
    assert_eq!(wrote, TABLE);
}

#[test]
fn the_emitted_source_of_the_shared_table_has_no_issue() {
    let source = module(
        &table_definitions(),
        &[CHECKS, REPORT],
        &driver(&table_cases()),
    );
    let Some((clean, said)) = dart("analyze-table", &["analyze"], &source) else {
        return;
    };
    assert!(clean, "{said}\n--- source ---\n{source}");
    assert!(said.contains("No issues found!"), "{said}");
}

#[test]
fn the_emitted_source_of_every_shape_has_no_issue() {
    let source = module(
        &shape_definitions(),
        &[CHECKS, REPORT, SHAPE_FUNCTIONS],
        "void main() {\n  report('none', const []);\n  print(jsonEncode(const <String>[]));\n}\n",
    );
    let Some((clean, said)) = dart("analyze-shapes", &["analyze"], &source) else {
        return;
    };
    assert!(clean, "{said}\n--- source ---\n{source}");
    assert!(said.contains("No issues found!"), "{said}");
}

/// A check with no function under its name does not build, and the compiler names it.
#[test]
fn a_check_with_no_dart_function_does_not_compile() {
    let source = module(&table_definitions(), &[REPORT], &driver(&table_cases()));
    let Some((built, said)) = dart("missing", &[], &source) else {
        return;
    };
    assert!(!built, "{said}");
    assert!(
        said.contains("Error: The method 'withinParcel' isn't defined for the type 'Parcel'."),
        "{said}"
    );
    assert!(
        said.contains("Error: The method 'withoutDuplicates' isn't defined for the type 'Parcel'."),
        "{said}"
    );
}

/// A gauge whose optional string and optional mark hold `short_note` and an empty mark, or
/// nothing, and whose optional number holds `spare_count`.
fn gauge(
    code_list: &[&str],
    item_count: u32,
    item_label: &str,
    short_note: Option<&str>,
    spare_count: Option<u32>,
    unit_weight: f64,
) -> Gauge {
    Gauge {
        code_list: texts(code_list),
        item_count,
        item_label: item_label.to_owned(),
        short_note: short_note.map(ToOwned::to_owned),
        sort_mark: short_note.map(|_| Mark(String::new())),
        spare_count,
        unit_weight,
    }
}

/// Each of the five keys, broken below, broken above and held, on a string, a list, a number and
/// a branded string, bare and under an optional, with a whole float among the values quoted back.
#[test]
fn each_bound_prints_the_sentence_rust_prints() {
    let below = gauge(&[], 0, "A", Some("a"), Some(9), 0.25);
    let above = gauge(&["a", "b", "c", "d"], 121, "abcdefghi", None, Some(10), 4.0);
    let held = gauge(&["a"], 1, "ab", None, None, 3.0);
    let cases = [
        compared(
            "below",
            &below,
            below.validate(),
            "Gauge.fromJsonUnjudged(@)",
        ),
        compared(
            "above",
            &above,
            above.validate(),
            "Gauge.fromJsonUnjudged(@)",
        ),
        compared("held", &held, held.validate(), "Gauge.fromJsonUnjudged(@)"),
    ];
    assert_eq!(
        report(&cases),
        "\
below: refused
    'codeList': too short: minimum length is 1, got 0
    'itemCount': too small: minimum is 1, got 0
    'itemLabel': too short: minimum length is 2, got 1
    'itemLabel': does not match pattern '^[a-z]+$'
    'shortNote': too short: minimum length is 2, got 1
    'sortMark': too short: minimum length is 1, got 0
    'unitWeight': too small: minimum is 0.5, got 0.25
above: refused
    'codeList': too long: maximum length is 3, got 4
    'itemCount': too large: maximum is 120, got 121
    'itemLabel': too long: maximum length is 8, got 9
    'spareCount': too large: maximum is 9, got 10
    'unitWeight': too large: maximum is 3, got 4
held: passes
",
        "what Rust's `validate()` answers"
    );
    let Some(wrote) = printed("bounds.dart", &cases) else {
        return;
    };
    assert_eq!(wrote, report(&cases));
}

/// A string's length is its Unicode code points: a letter with an accent and a character outside
/// the Basic Multilingual Plane each count one.
#[test]
fn a_length_counts_code_points() {
    let initial = |letter: &str| Initial {
        letter: letter.to_owned(),
    };
    let cases = [
        pinned(
            "a letter with an accent",
            &initial("\u{e9}"),
            &[],
            "Initial.fromJsonUnjudged(@)",
        ),
        pinned(
            "a character outside the plane",
            &initial("\u{1f600}"),
            &[],
            "Initial.fromJsonUnjudged(@)",
        ),
        pinned(
            "two letters",
            &initial("ab"),
            &["'letter': too long: maximum length is 1, got 2"],
            "Initial.fromJsonUnjudged(@)",
        ),
        pinned(
            "two characters outside the plane",
            &initial("\u{1f600}\u{1f600}"),
            &["'letter': too long: maximum length is 1, got 2"],
            "Initial.fromJsonUnjudged(@)",
        ),
    ];
    let Some(wrote) = printed("code_points.dart", &cases) else {
        return;
    };
    assert_eq!(wrote, report(&cases));
}

/// A model held in a field, an optional, a list and a map, each message at its full place.
#[test]
fn a_holder_reports_what_it_holds_at_the_place_rust_names() {
    let terminal = Terminal {
        first_parcel: unlabelled(),
        lists_by_dock: BTreeMap::from([("north".to_owned(), vec![sound(), unlabelled()])]),
        parcel_list: vec![sound(), outside()],
        parcels_by_dock: BTreeMap::from([
            ("north".to_owned(), unlabelled()),
            ("south".to_owned(), sound()),
        ]),
        spare_parcel: Some(outside()),
    };
    let empty = Terminal {
        first_parcel: sound(),
        lists_by_dock: BTreeMap::new(),
        parcel_list: Vec::new(),
        parcels_by_dock: BTreeMap::new(),
        spare_parcel: None,
    };
    let cases = [
        compared(
            "four broken parcels",
            &terminal,
            terminal.validate(),
            "Terminal.fromJsonUnjudged(@)",
        ),
        compared(
            "nothing held",
            &empty,
            empty.validate(),
            "Terminal.fromJsonUnjudged(@)",
        ),
    ];
    assert_eq!(
        report(&cases),
        "\
four broken parcels: refused
    'firstParcel.label': too short: minimum length is 1, got 0
    'listsByDock.north.1.label': too short: minimum length is 1, got 0
    'parcelList.1.fragileItemCodes.1': y is outside of itemCodeList
    'parcelsByDock.north.label': too short: minimum length is 1, got 0
    'spareParcel.fragileItemCodes.1': y is outside of itemCodeList
nothing held: passes
",
        "what Rust's `validate()` answers"
    );
    let Some(wrote) = printed("holder.dart", &cases) else {
        return;
    };
    assert_eq!(wrote, report(&cases));
}

/// A key that is no string is the text serde writes for it, a list in a list names both indexes,
/// a `null` among the items of a list is passed, and a member runs its count, then its items,
/// then its check.
#[test]
fn a_nested_place_and_a_members_order_are_rusts() {
    let yard = Yard {
        by_dock: BTreeMap::from([(Dock::North, unlabelled()), (Dock::South, sound())]),
        by_number: BTreeMap::from([(7, unlabelled())]),
        code_list: Some(texts(&["a", "b", "c"])),
        grid: vec![vec![sound(), unlabelled()]],
        parcel_list: vec![parcel("unknown", &["x"], &["y"])],
        spares: vec![None, Some(unlabelled())],
    };
    let bare = Yard {
        by_dock: BTreeMap::new(),
        by_number: BTreeMap::new(),
        code_list: None,
        grid: Vec::new(),
        parcel_list: vec![sound(), sound()],
        spares: vec![None],
    };
    let cases = [
        compared("a yard", &yard, yard.validate(), "Yard.fromJsonUnjudged(@)"),
        compared(
            "a bare yard",
            &bare,
            bare.validate(),
            "Yard.fromJsonUnjudged(@)",
        ),
    ];
    assert_eq!(
        report(&cases),
        "\
a yard: refused
    'byDock.north.label': too short: minimum length is 1, got 0
    'byNumber.7.label': too short: minimum length is 1, got 0
    'codeList.2': c is one code too many
    'grid.0.1.label': too short: minimum length is 1, got 0
    'parcelList.0.fragileItemCodes.0': y is outside of itemCodeList
    'parcelList': too short: minimum length is 2, got 1
    'parcelList.0': the first parcel is known
    'spares.1.label': too short: minimum length is 1, got 0
a bare yard: passes
",
        "what Rust's `validate()` answers"
    );
    let Some(wrote) = printed("yard.dart", &cases) else {
        return;
    };
    assert_eq!(wrote, report(&cases));
}

/// The brands, each with what Rust's `validate()` answers of it: a string check and a check of
/// its own, a parameter at its declared default and at another filling, and a list.
pub fn brand_cases() -> Vec<Case> {
    let tagged = "Tag<String>.fromJsonUnjudged(@, (e) => e as String)";
    vec![
        compared(
            "Code, short and in lower case",
            &Code("a".to_owned()),
            Code("a".to_owned()).validate(),
            "Code.fromJsonUnjudged(@)",
        ),
        compared(
            "Code",
            &Code("AB".to_owned()),
            Code("AB".to_owned()).validate(),
            "Code.fromJsonUnjudged(@)",
        ),
        compared(
            "Tag<String>, short",
            &Tag("a".to_owned()),
            Tag("a".to_owned()).validate(),
            tagged,
        ),
        compared(
            "Tag<String>",
            &Tag("ab".to_owned()),
            Tag("ab".to_owned()).validate(),
            tagged,
        ),
        compared(
            "Tag<int>, at no default",
            &Tag(7_u32),
            Tag(7_u32).validate(),
            "Tag<int>.fromJsonUnjudged(@, (e) => e as int)",
        ),
        compared(
            "Batch, empty",
            &Batch(Vec::new()),
            Batch(Vec::new()).validate(),
            "Batch.fromJsonUnjudged(@)",
        ),
        compared(
            "Batch, three codes",
            &Batch(texts(&["a", "b", "c"])),
            Batch(texts(&["a", "b", "c"])).validate(),
            "Batch.fromJsonUnjudged(@)",
        ),
    ]
}

/// A type with a parameter at two fillings, a unit struct, a plain enum, a member serde writes no
/// key for and an alias, each with what Rust's `validate()` answers of it.
pub fn named_shape_cases() -> Vec<Case> {
    let filled = |codes: &[&str], label: &str| Crate {
        codes: texts(codes),
        label: label.to_owned(),
    };
    let counted = Crate {
        codes: Vec::<u32>::new(),
        label: "c".to_owned(),
    };
    let docket = Docket {
        docket_code: "d".to_owned(),
        held_parcel: unlabelled(),
    };
    let consignment: Consignment = unlabelled();
    let text_filled = "Crate<String>.fromJsonUnjudged(@, (e) => e as String)";
    vec![
        compared(
            "Crate<String>, empty and unlabelled",
            &filled(&[], ""),
            filled(&[], "").validate(),
            text_filled,
        ),
        compared(
            "Crate<String>",
            &filled(&["a"], "c"),
            filled(&["a"], "c").validate(),
            text_filled,
        ),
        compared(
            "Crate<int>, empty",
            &counted,
            counted.validate(),
            "Crate<int>.fromJsonUnjudged(@, (e) => e as int)",
        ),
        compared(
            "ClosedDock",
            &ClosedDock,
            ClosedDock.validate(),
            "ClosedDock.fromJsonUnjudged(@)",
        ),
        compared(
            "Rating, unrated",
            &Rating::Unrated,
            Rating::Unrated.validate(),
            "Rating.fromJsonUnjudged(@)",
        ),
        compared(
            "Rating, express",
            &Rating::Express,
            Rating::Express.validate(),
            "Rating.fromJsonUnjudged(@)",
        ),
        compared(
            "Docket, a parcel written beside its code",
            &docket,
            docket.validate(),
            "Docket.fromJsonUnjudged(@)",
        ),
        compared(
            "Consignment",
            &consignment,
            consignment.validate(),
            "Consignment.fromJsonUnjudged(@)",
        ),
    ]
}

/// Each shape whose messages name no slot and no variant, compared with Rust.
#[test]
fn each_shape_runs_its_rules_as_rust_runs_them() {
    let mut cases = brand_cases();
    cases.extend(named_shape_cases());
    assert_eq!(
        report(&cases),
        "\
Code, short and in lower case: refused
    too short: minimum length is 2, got 1
    does not match pattern '^[A-Z]+$'
    a code is upper case
Code: passes
Tag<String>, short: refused
    too short: minimum length is 2, got 1
Tag<String>: passes
Tag<int>, at no default: passes
Batch, empty: refused
    too short: minimum length is 1, got 0
Batch, three codes: refused
    too long: maximum length is 2, got 3
Crate<String>, empty and unlabelled: refused
    'label': too short: minimum length is 1, got 0
    'codes': a crate holds at least one code
Crate<String>: passes
Crate<int>, empty: refused
    'codes': a crate holds at least one code
ClosedDock: refused
    a closed dock takes no parcel
Rating, unrated: refused
    a parcel is rated
Rating, express: passes
Docket, a parcel written beside its code: refused
    'label': too short: minimum length is 1, got 0
Consignment: refused
    'label': too short: minimum length is 1, got 0
",
        "what Rust's `validate()` answers"
    );
    cases.push(pinned(
        "Dock, which names no check",
        &Dock::North,
        &[],
        "Dock.fromJsonUnjudged(@)",
    ));
    let Some(wrote) = printed("shapes.dart", &cases) else {
        return;
    };
    assert_eq!(wrote, report(&cases));
}

/// A slot is named by its position in the array serde writes, and the one slot of a struct serde
/// writes as the value itself by nothing. A position of a tuple written as a field's type is
/// named after the field.
#[test]
fn a_slot_is_named_by_the_step_serde_writes() {
    let deck = Deck {
        deck_pair: ("a".to_owned(), unlabelled()),
    };
    assert_eq!(
        serde_json::to_string(&Sleeve("a".to_owned())).unwrap(),
        r#""a""#,
        "serde writes a struct of one slot as the slot's value"
    );
    let cases = [
        pinned(
            "Pair, each slot broken",
            &Pair(String::new(), 0, unlabelled()),
            &[
                "'0': too short: minimum length is 1, got 0",
                "'1': too small: minimum is 1, got 0",
                "'2.label': too short: minimum length is 1, got 0",
            ],
            "Pair.fromJsonUnjudged(@)",
        ),
        pinned(
            "Pair",
            &Pair("a".to_owned(), 1, sound()),
            &[],
            "Pair.fromJsonUnjudged(@)",
        ),
        pinned(
            "Sleeve, empty",
            &Sleeve(String::new()),
            &["too short: minimum length is 1, got 0"],
            "Sleeve.fromJsonUnjudged(@)",
        ),
        pinned(
            "Sleeve",
            &Sleeve("a".to_owned()),
            &[],
            "Sleeve.fromJsonUnjudged(@)",
        ),
        pinned(
            "Deck, a tuple in a field",
            &deck,
            &["'deckPair.1.label': too short: minimum length is 1, got 0"],
            "Deck.fromJsonUnjudged(@)",
        ),
    ];
    let Some(wrote) = printed("slots.dart", &cases) else {
        return;
    };
    assert_eq!(wrote, report(&cases));
}

/// The variants of an enum serde writes under the variant's name, each message written out.
pub fn named_variant_cases() -> Vec<Case> {
    vec![
        pinned(
            "no tag, a model in the one slot",
            &Freight::Boxed(unlabelled()),
            &["'Boxed.label': too short: minimum length is 1, got 0"],
            "Freight.fromJsonUnjudged(@)",
        ),
        pinned(
            "no tag, a renamed variant",
            &Freight::Crated(unlabelled()),
            &["'crated.label': too short: minimum length is 1, got 0"],
            "Freight.fromJsonUnjudged(@)",
        ),
        pinned(
            "no tag, no value",
            &Freight::Held,
            &["a held freight has no route"],
            "Freight.fromJsonUnjudged(@)",
        ),
        pinned(
            "no tag, a named member",
            &Freight::Loose {
                freight_label: String::new(),
            },
            &["'Loose.freightLabel': too short: minimum length is 1, got 0"],
            "Freight.fromJsonUnjudged(@)",
        ),
        pinned(
            "no tag, the one slot",
            &Freight::Sealed(String::new()),
            &["'Sealed': too short: minimum length is 1, got 0"],
            "Freight.fromJsonUnjudged(@)",
        ),
        pinned(
            "no tag, a slot of several",
            &Freight::Split(String::new(), 0),
            &[
                "'Split.0': too short: minimum length is 1, got 0",
                "'Split.1': too small: minimum is 1, got 0",
            ],
            "Freight.fromJsonUnjudged(@)",
        ),
        pinned(
            "no tag, held",
            &Freight::Split("a".to_owned(), 1),
            &[],
            "Freight.fromJsonUnjudged(@)",
        ),
        pinned(
            "no tag, a variant in lower case",
            &Dispatch::ByRoad {
                road: String::new(),
            },
            &["'byroad.road': too short: minimum length is 1, got 0"],
            "Dispatch.fromJsonUnjudged(@)",
        ),
    ]
}

/// The variants of an enum under a tag, and of one serde writes as the variant's value, each
/// message written out. A value of the latter that breaks a bound is built in code: no read
/// takes the variant for it.
pub fn tagged_and_untagged_variant_cases() -> Vec<Case> {
    vec![
        pinned(
            "tag and content, the one slot",
            &Haul::Boxed(String::new()),
            &["'held': too short: minimum length is 1, got 0"],
            "Haul.fromJsonUnjudged(@)",
        ),
        pinned(
            "tag and content, a named member",
            &Haul::Loose {
                haul_label: String::new(),
            },
            &["'held.haulLabel': too short: minimum length is 1, got 0"],
            "Haul.fromJsonUnjudged(@)",
        ),
        pinned(
            "tag and content, a slot of several",
            &Haul::Split("a".to_owned(), 0),
            &["'held.1': too small: minimum is 1, got 0"],
            "Haul.fromJsonUnjudged(@)",
        ),
        pinned(
            "tag alone, the one slot",
            &Lift::Boxed(unlabelled()),
            &["'label': too short: minimum length is 1, got 0"],
            "Lift.fromJsonUnjudged(@)",
        ),
        pinned(
            "tag alone, a named member",
            &Lift::Loose {
                lift_label: String::new(),
            },
            &["'liftLabel': too short: minimum length is 1, got 0"],
            "Lift.fromJsonUnjudged(@)",
        ),
        Case {
            answered: texts(&["'name': too short: minimum length is 1, got 0"]),
            built: "ReferenceNamed(name: '')".to_owned(),
            name: "untagged, a named member",
        },
        Case {
            answered: texts(&["too small: minimum is 1, got 0"]),
            built: "ReferenceNumbered(0)".to_owned(),
            name: "untagged, the one slot",
        },
        pinned(
            "untagged, held",
            &Reference::Numbered(1),
            &[],
            "Reference.fromJsonUnjudged(@)",
        ),
    ]
}

/// A member of a variant is named by the steps serde writes on the way to it: the name serde
/// writes the variant under where it writes one, the content key where the enum declares one,
/// and nothing where the member sits beside the tag or is the value itself.
#[test]
fn a_member_of_a_variant_is_named_by_the_steps_serde_writes() {
    let mut cases = named_variant_cases();
    cases.extend(tagged_and_untagged_variant_cases());
    let Some(wrote) = printed("variants.dart", &cases) else {
        return;
    };
    assert_eq!(wrote, report(&cases));
}

/// The enum's own string check holds each field written as text, beside the field's own: the
/// longer minimum, the shorter maximum and both patterns. A number is not reached.
#[test]
fn an_enums_string_check_reaches_the_fields_it_reaches_in_rust() {
    let inked = |ink: &str| Stamp::Inked {
        ink: ink.to_owned(),
        pages: 0,
    };
    let cases = [
        pinned(
            "a short ink with no vowel",
            &inked("b"),
            &[
                "'ink': too short: minimum length is 2, got 1",
                "'ink': does not match pattern '[aeiou]'",
            ],
            "Stamp.fromJsonUnjudged(@)",
        ),
        pinned(
            "a long ink in upper case",
            &inked("AEIOUAEIOU"),
            &[
                "'ink': too long: maximum length is 9, got 10",
                "'ink': does not match pattern '[aeiou]'",
                "'ink': does not match pattern '^[a-z]+$'",
            ],
            "Stamp.fromJsonUnjudged(@)",
        ),
        pinned("an ink", &inked("blue"), &[], "Stamp.fromJsonUnjudged(@)"),
        pinned(
            "a short seal",
            &Stamp::Pressed {
                seal: "w".to_owned(),
            },
            &["'seal': too short: minimum length is 2, got 1"],
            "Stamp.fromJsonUnjudged(@)",
        ),
    ];
    let Some(wrote) = printed("stamps.dart", &cases) else {
        return;
    };
    assert_eq!(wrote, report(&cases));
}

/// The member moves off the name of the method, and the key stays the one serde writes.
#[test]
fn a_field_named_validate_builds_and_keeps_its_key() {
    let audit = Audit { validate: true };
    let entry = format!(
        "void main() {{\n  final audit = {};\n  print(jsonEncode(audit.toJson()));\n  \
         print(audit.validate());\n}}\n",
        reading("Audit.fromJson(@)", &audit)
    );
    let Some(wrote) = ran(
        "dart",
        RUNTIME_VAR,
        "dart",
        "audit.dart",
        &module(&[audit_dart::dart_definition()], &[], &entry),
    ) else {
        return;
    };
    assert_eq!(
        wrote,
        format!("{}\n[]\n", serde_json::to_string(&audit).unwrap())
    );
}

/// What `text` holds from `opens` up to `closes`.
pub fn from_until<'text>(text: &'text str, opens: &str, closes: &str) -> &'text str {
    let rest = text
        .find(opens)
        .and_then(|start| text.get(start..))
        .unwrap();
    rest.split_once(closes).unwrap().0
}

/// What `definition` writes from its `validate()` to the end of its class.
fn validate_of(definition: &str) -> &str {
    let rest = definition
        .find("List<String> validate()")
        .and_then(|start| definition.get(start..))
        .unwrap();
    rest.strip_suffix(" }").unwrap()
}

/// The README's section on the Dart `validate()`, held to what is declared here, what is emitted
/// for it and what the run prints. The emitter writes a class on one line, and the README breaks
/// it: the two are compared with each run of spaces and line breaks read as one space.
#[test]
fn the_readme_shows_what_is_declared_emitted_and_printed() {
    let section = from_until(
        include_str!("../../README.md"),
        "### `validate()` in Dart",
        "### Literal Values",
    );
    let declared = from_until(section, "```rust\n", "\n```")
        .strip_prefix("```rust\n")
        .unwrap();
    for declaration in declared.split("\n\n") {
        assert!(
            include_str!("run_dart_validate.rs").contains(declaration),
            "the README declares what this file does not: {declaration}"
        );
    }
    let squeezed = section.split_whitespace().collect::<Vec<_>>().join(" ");
    // The method, then each helper the class holds, which the README shows once.
    for definition in table_definitions() {
        for member in validate_of(&definition).split(" static ") {
            assert!(
                squeezed.contains(member),
                "the README does not show: {member}"
            );
        }
    }
    assert!(section.contains(CHECKS.trim()), "{section}");
    assert!(section.contains(TABLE), "{section}");
    assert!(
        section.contains("Error: The method 'withinParcel' isn't defined for the type 'Parcel'."),
        "{section}"
    );
}
