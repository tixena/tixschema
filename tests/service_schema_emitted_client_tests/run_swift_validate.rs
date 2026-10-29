//! The emitted Swift `validate()` run under a Swift toolchain, with warnings as errors. A named
//! field, the index of a list and the key of a map are compared with what the Rust `validate()`
//! answers of the same value. A slot and a member of a variant are held to the names the steps
//! serde writes give them.

#![cfg(feature = "swift")]

use alloc::collections::BTreeMap;
use core::fmt::Write as _;

use serde::Serialize;

use super::runtime::judged;
use super::swift_validate_models::{
    Archive, Base, Berth, Cargo, Carried, Casket, ClosedDock, Code, Consignment, Crate, Depot,
    Docket, Docking, Freight, Gauge, Grade, Haul, Inspection, LaterCode, LaterTitle, Ledger, Lift,
    Manifest, Nameplate, Package, Parcel, Poster, Quay, Sealed, SealedCode, Slogans, SomeSlogans,
    Strongbox, Tally, Title, Waybill, Yard, archive_swift, base_swift, berth_swift, cargo_swift,
    carried_swift, casket_swift, closed_dock_swift, code_swift, consignment_swift, crate_swift,
    depot_swift, docket_swift, docking_swift, freight_swift, gauge_swift, grade_swift, haul_swift,
    inspection_swift, later_code_swift, later_title_swift, ledger_swift, lift_swift,
    manifest_swift, mottos_swift, nameplate_swift, package_swift, parcel_swift, poster_swift,
    quay_swift, sealed_code_swift, sealed_swift, slogans_swift, some_slogans_swift,
    strongbox_swift, tally_swift, title_swift, waybill_swift, yard_swift,
};

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// The two functions of the parcel, as the README shows them.
pub const SHOWN_FUNCTIONS: &str = r#"func withoutDuplicates(_ value: [String]) -> [String] {
  var seen = Set<String>()
  for code in value where !seen.insert(code).inserted {
    return ["duplicate item code \(code)"]
  }
  return []
}

func withinParcel(_ value: Parcel) -> [String] {
  value.fragileItemCodes.enumerated()
    .filter { !value.itemCodeList.contains($0.element) }
    .map { "'fragileItemCodes.\($0.offset)': \($0.element) is outside of itemCodeList" }
}"#;

/// What the Swift compiler answers for a bound on an alias of a string declared below the field.
const ALIAS_BELOW_ERROR: &str =
    "value of type 'LaterTitle' (aka 'String') has no member 'swiftSchemaBoundLength'";

/// The developer's other functions, written by hand in Swift.
pub const FUNCTIONS: &str = r#"
func takingParcels(_ value: ClosedDock) -> [String] {
  ["a closed dock takes no parcel"]
}

func withACode<CodeType>(_ value: Crate<CodeType>) -> [String] {
  value.codes.isEmpty ? ["'codes': a crate holds at least one code"] : []
}

func withEntries<IdType>(_ value: Casket<IdType>) -> [String] {
  value.entries.isEmpty ? ["'entries': a casket holds at least one entry"] : []
}

func withALift(_ value: Lift) -> [String] {
  if case .unladen = value { return ["a lift carries something"] }
  return []
}

func withoutASparePair(_ value: [Package]) -> [String] {
  value.count == 2 ? ["a ledger holds no spare pair"] : []
}

"#;

/// What a run reads a payload and prints an answer with, in the form the Rust side prints its own.
/// A payload is read through the unjudged initializer of its type, which judges nothing.
const HARNESS: &str = r#"
func report(_ name: String, _ violations: [String]) {
  if violations.isEmpty {
    print("\(name): passes")
  } else {
    print("\(name): refused")
    for violation in violations { print("    \(violation)") }
  }
}

protocol ReadUnjudged {
  init(swiftSchemaUnjudged decoder: Decoder) throws
}

struct Unjudged<Held: ReadUnjudged>: Decodable {
  let held: Held
  init(from decoder: Decoder) throws { held = try Held(swiftSchemaUnjudged: decoder) }
}

func read<Read: ReadUnjudged>(_ type: Read.Type, _ payload: String) -> Read {
  try! JSONDecoder().decode(Unjudged<Read>.self, from: Data(payload.utf8)).held
}
"#;

/// One value as the Swift side reads it, and what the Rust `validate()` answers of it.
struct Case {
    answered: Result<(), Vec<String>>,
    name: &'static str,
    read: String,
}

/// `held` as the payload serde writes for it, read in Swift as `swift_type`.
fn read_as<Held>(swift_type: &str, held: &Held) -> String
where
    Held: Serialize,
{
    format!(
        "read({swift_type}.self, #\"{}\"#)",
        serde_json::to_string(held).unwrap()
    )
}

/// One case of a value both sides judge.
fn case<Held>(
    name: &'static str,
    swift_type: &str,
    held: &Held,
    answered: Result<(), Vec<String>>,
) -> Case
where
    Held: Serialize,
{
    Case {
        answered,
        name,
        read: read_as(swift_type, held),
    }
}

/// What the Rust side prints for `cases`: one line per case, and one per message under it.
fn rust_report(cases: &[Case]) -> String {
    let mut printed = String::new();
    for held in cases {
        match &held.answered {
            Ok(()) => writeln!(printed, "{}: passes", held.name).unwrap(),
            Err(violations) => {
                writeln!(printed, "{}: refused", held.name).unwrap();
                for violation in violations {
                    writeln!(printed, "    {violation}").unwrap();
                }
            }
        }
    }
    printed
}

/// One `report` line per value: its name, and what its `validate()` answers.
fn driver(read: &[(&str, String)]) -> String {
    read.iter()
        .map(|(name, held)| format!("report(\"{name}\", {held}.validate())"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The `report` lines of `cases`.
fn driver_of(cases: &[Case]) -> String {
    let read: Vec<(&str, String)> = cases
        .iter()
        .map(|held| (held.name, held.read.clone()))
        .collect();
    driver(&read)
}

/// Every emitted definition the runs read, in the order they are declared.
pub fn definitions() -> Vec<String> {
    vec![
        parcel_swift::swift_definition(),
        manifest_swift::swift_definition(),
        gauge_swift::swift_definition(),
        inspection_swift::swift_definition(),
        nameplate_swift::swift_definition(),
        code_swift::swift_definition(),
        sealed_code_swift::swift_definition(),
        title_swift::swift_definition(),
        package_swift::swift_definition(),
        base_swift::swift_definition(),
        depot_swift::swift_definition(),
        yard_swift::swift_definition(),
        ledger_swift::swift_definition(),
        waybill_swift::swift_definition(),
        later_code_swift::swift_definition(),
        crate_swift::swift_definition(),
        casket_swift::swift_definition(),
        archive_swift::swift_definition(),
        slogans_swift::swift_definition(),
        mottos_swift::swift_definition(),
        some_slogans_swift::swift_definition(),
        carried_swift::swift_definition(),
        poster_swift::swift_definition(),
        docket_swift::swift_definition(),
        consignment_swift::swift_definition(),
        sealed_swift::swift_definition(),
        strongbox_swift::swift_definition(),
        closed_dock_swift::swift_definition(),
        grade_swift::swift_definition(),
        freight_swift::swift_definition(),
        cargo_swift::swift_definition(),
        haul_swift::swift_definition(),
        lift_swift::swift_definition(),
        tally_swift::swift_definition(),
        berth_swift::swift_definition(),
    ]
}

/// The conformance the harness asks of each type `lines` read, written once per type.
fn conformances(lines: &str) -> String {
    let mut named: Vec<&str> = lines
        .split("read(")
        .skip(1)
        .filter_map(|after| after.split_once(".self")?.0.split('<').next())
        .collect();
    named.sort_unstable();
    named.dedup();
    named
        .iter()
        .map(|read| format!("extension {read}: ReadUnjudged {{}}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The module one run compiles: `declared`, the functions the developer writes, and `lines`.
fn module(declared: Vec<String>, functions: &str, lines: &str) -> String {
    let mut parts = vec!["import Foundation".to_owned()];
    parts.extend(declared);
    parts.push(functions.to_owned());
    parts.push(HARNESS.to_owned());
    parts.push(conformances(lines));
    parts.push(lines.to_owned());
    parts.join("\n\n")
}

/// What `lines` print, compiled with warnings as errors, or `None` where no runtime was reachable.
fn printed(lines: &str) -> Option<String> {
    let functions = format!("{SHOWN_FUNCTIONS}\n{FUNCTIONS}");
    let source = module(definitions(), &functions, lines);
    let run = judged(
        "swift-validate",
        RUNTIME_VAR,
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    )?;
    assert!(
        run.succeeded,
        "`swift -warnings-as-errors main.swift` failed.\n--- stdout ---\n{}\n--- stderr ---\n{}\n--- source ---\n{source}",
        run.stdout, run.stderr
    );
    Some(run.stdout)
}

fn parcel(label: &str, codes: &[&str], fragile: &[&str]) -> Parcel {
    Parcel {
        fragile_item_codes: fragile.iter().map(|&code| code.to_owned()).collect(),
        item_code_list: codes.iter().map(|&code| code.to_owned()).collect(),
        label: label.to_owned(),
    }
}

pub fn package(item_label: &str, parcel_code: &str) -> Package {
    Package {
        item_label: item_label.to_owned(),
        parcel_code: Code(parcel_code.to_owned()),
    }
}

/// A package that breaks its own bound and its code's.
pub fn broken() -> Package {
    package("", "a")
}

pub fn depot(held: &dyn Fn() -> Package, base_name: &str) -> Depot {
    Depot {
        by_number: BTreeMap::from([(10, held()), (7, held())]),
        deck_pair: ("port".to_owned(), held()),
        first_parcel: held(),
        grid: vec![vec![package("A", "ab"), held()], vec![held()]],
        parcel_list: vec![held(), held()],
        parcels_by_dock: BTreeMap::from([
            ("south".to_owned(), held()),
            ("north".to_owned(), held()),
        ]),
        spare_parcel: Some(held()),
        starting_base: Base {
            base_name: base_name.to_owned(),
        },
    }
}

pub fn gauge(draft_metres: f64, holds: usize, knots: u32, text: &str, spare: bool) -> Gauge {
    Gauge {
        draft_metres,
        hold_codes: vec!["h".to_owned(); holds],
        knots,
        load_tonnes: draft_metres.round(),
        port_code: text.to_ascii_uppercase(),
        short_name: text.to_owned(),
        spare_knots: spare.then_some(0),
        spare_name: spare.then(|| "s".to_owned()),
    }
}

#[test]
fn the_shared_table_prints_the_lines_the_rust_validate_prints() {
    let parcels = [
        parcel("A", &["x"], &["x"]),
        parcel("", &["x"], &["x"]),
        parcel("A", &["x", "x"], &["x"]),
        parcel("A", &["x"], &["x", "y"]),
        parcel("", &["x", "x"], &["y"]),
    ];
    let names = ["case 1", "case 2", "case 3", "case 4", "case 5"];
    let mut cases: Vec<Case> = names
        .into_iter()
        .zip(&parcels)
        .map(|(name, held)| case(name, "Parcel", held, held.validate()))
        .collect();
    let manifest = Manifest {
        parcel: parcel("A", &["x"], &["x", "y"]),
    };
    cases.push(case("case 6", "Manifest", &manifest, manifest.validate()));
    let expected = rust_report(&cases);
    assert_eq!(
        expected,
        "\
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
",
        "the 13 lines the Rust `validate()` prints for the six shared cases"
    );
    let Some(swift) = printed(&driver_of(&cases)) else {
        return;
    };
    assert_eq!(swift, expected);
}

#[test]
fn each_bound_prints_the_sentence_the_rust_validate_states() {
    let held = [
        (
            "Gauge, every bound broken from below",
            gauge(0.25, 0, 0, "a", true),
        ),
        (
            "Gauge, every bound broken from above",
            gauge(10.5, 3, 41, "abcde", false),
        ),
        ("Gauge, a whole float", gauge(1.0, 1, 0, "abc", false)),
        (
            "Gauge, a port code that ends with a line feed",
            Gauge {
                port_code: "ABC\n".to_owned(),
                ..gauge(2.5, 2, 40, "abc", false)
            },
        ),
        ("Gauge, held", gauge(2.5, 2, 40, "abc", false)),
    ];
    let cases: Vec<Case> = held
        .iter()
        .map(|(name, gauged)| case(name, "Gauge", gauged, gauged.validate()))
        .collect();
    let expected = rust_report(&cases);
    assert_eq!(
        expected,
        "\
Gauge, every bound broken from below: refused
    'draftMetres': too small: minimum is 0.5, got 0.25
    'holdCodes': too short: minimum length is 1, got 0
    'loadTonnes': too small: minimum is 2, got 0
    'portCode': does not match pattern '^[A-Z]{3}$'
    'shortName': too short: minimum length is 2, got 1
    'spareKnots': too small: minimum is 1, got 0
    'spareName': too short: minimum length is 2, got 1
Gauge, every bound broken from above: refused
    'draftMetres': too large: maximum is 9.5, got 10.5
    'holdCodes': too long: maximum length is 2, got 3
    'knots': too large: maximum is 40, got 41
    'portCode': does not match pattern '^[A-Z]{3}$'
    'shortName': too long: maximum length is 4, got 5
Gauge, a whole float: refused
    'loadTonnes': too small: minimum is 2, got 1
Gauge, a port code that ends with a line feed: refused
    'portCode': does not match pattern '^[A-Z]{3}$'
Gauge, held: passes
",
        "each of the five keys, on a string, a list and a number, and under an `Option`, and a \
         pattern's `$` at the end of the text alone"
    );
    let Some(swift) = printed(&driver_of(&cases)) else {
        return;
    };
    assert_eq!(swift, expected);
}

/// A length is counted in Unicode code points: one for a precomposed letter, two for a letter and
/// its combining mark, one for a character outside the Basic Multilingual Plane.
#[test]
fn a_length_is_counted_in_unicode_code_points() {
    let read: Vec<(&str, String)> = [
        ("a precomposed letter", "\u{e9}"),
        ("a letter and its combining mark", "e\u{301}"),
        (
            "a character outside the Basic Multilingual Plane",
            "\u{1f600}",
        ),
    ]
    .into_iter()
    .map(|(name, engraved)| {
        let held = Nameplate {
            engraved: engraved.to_owned(),
        };
        (name, read_as("Nameplate", &held))
    })
    .collect();
    let Some(swift) = printed(&driver(&read)) else {
        return;
    };
    assert_eq!(
        swift,
        "\
a precomposed letter: refused
    'engraved': too long: maximum length is 0, got 1
a letter and its combining mark: refused
    'engraved': too long: maximum length is 0, got 2
a character outside the Basic Multilingual Plane: refused
    'engraved': too long: maximum length is 0, got 1
"
    );
}

#[test]
fn a_holder_reports_each_model_it_holds_under_its_full_place() {
    let unlabelled = depot(&broken, "");
    let kept = depot(&|| package("A", "ab"), "home");
    let cases = [
        case(
            "Depot, every package broken",
            "Depot",
            &unlabelled,
            unlabelled.validate(),
        ),
        case("Depot, held", "Depot", &kept, kept.validate()),
    ];
    let expected = rust_report(&cases);
    assert_eq!(
        expected,
        "\
Depot, every package broken: refused
    'byNumber.7.itemLabel': too short: minimum length is 1, got 0
    'byNumber.7.parcelCode': too short: minimum length is 2, got 1
    'byNumber.10.itemLabel': too short: minimum length is 1, got 0
    'byNumber.10.parcelCode': too short: minimum length is 2, got 1
    'deckPair.1.itemLabel': too short: minimum length is 1, got 0
    'deckPair.1.parcelCode': too short: minimum length is 2, got 1
    'firstParcel.itemLabel': too short: minimum length is 1, got 0
    'firstParcel.parcelCode': too short: minimum length is 2, got 1
    'grid.0.1.itemLabel': too short: minimum length is 1, got 0
    'grid.0.1.parcelCode': too short: minimum length is 2, got 1
    'grid.1.0.itemLabel': too short: minimum length is 1, got 0
    'grid.1.0.parcelCode': too short: minimum length is 2, got 1
    'parcelList.0.itemLabel': too short: minimum length is 1, got 0
    'parcelList.0.parcelCode': too short: minimum length is 2, got 1
    'parcelList.1.itemLabel': too short: minimum length is 1, got 0
    'parcelList.1.parcelCode': too short: minimum length is 2, got 1
    'parcelsByDock.north.itemLabel': too short: minimum length is 1, got 0
    'parcelsByDock.north.parcelCode': too short: minimum length is 2, got 1
    'parcelsByDock.south.itemLabel': too short: minimum length is 1, got 0
    'parcelsByDock.south.parcelCode': too short: minimum length is 2, got 1
    'spareParcel.itemLabel': too short: minimum length is 1, got 0
    'spareParcel.parcelCode': too short: minimum length is 2, got 1
    'baseName': too short: minimum length is 1, got 0
Depot, held: passes
",
        "a field, a list of lists, a list, a map by its sorted keys, an optional, a tuple and a \
         flattened member"
    );
    let Some(swift) = printed(&driver_of(&cases)) else {
        return;
    };
    assert_eq!(swift, expected);
}

/// The walk goes on below a list, a map and an optional in any nesting, and a map under a plain
/// enum names an entry by the name serde writes the variant under.
#[test]
fn a_holder_walks_lists_maps_and_optionals_in_any_nesting() {
    let yard = Yard {
        by_grade: BTreeMap::from([(Grade::Standard, broken()), (Grade::Express, broken())]),
        docks_in_order: vec![
            BTreeMap::new(),
            BTreeMap::from([("north".to_owned(), broken())]),
        ],
        lists_by_dock: BTreeMap::from([("north".to_owned(), vec![package("A", "ab"), broken()])]),
        maybe_each: vec![None, Some(broken())],
        spare_list: Some(vec![broken()]),
    };
    let cases = [case("Yard", "Yard", &yard, yard.validate())];
    let expected = rust_report(&cases);
    assert_eq!(
        expected,
        "\
Yard: refused
    'byGrade.Express.itemLabel': too short: minimum length is 1, got 0
    'byGrade.Express.parcelCode': too short: minimum length is 2, got 1
    'byGrade.Standard.itemLabel': too short: minimum length is 1, got 0
    'byGrade.Standard.parcelCode': too short: minimum length is 2, got 1
    'docksInOrder.1.north.itemLabel': too short: minimum length is 1, got 0
    'docksInOrder.1.north.parcelCode': too short: minimum length is 2, got 1
    'listsByDock.north.1.itemLabel': too short: minimum length is 1, got 0
    'listsByDock.north.1.parcelCode': too short: minimum length is 2, got 1
    'maybeEach.1.itemLabel': too short: minimum length is 1, got 0
    'maybeEach.1.parcelCode': too short: minimum length is 2, got 1
    'spareList.0.itemLabel': too short: minimum length is 1, got 0
    'spareList.0.parcelCode': too short: minimum length is 2, got 1
"
    );
    let Some(swift) = printed(&driver_of(&cases)) else {
        return;
    };
    assert_eq!(swift, expected);
}

/// A member's rules run in the order the Rust `validate()` runs them: its bounds, what the value
/// it holds reports, then the checks it names.
#[test]
fn a_member_runs_its_bounds_what_it_holds_then_its_checks() {
    let ledger = Ledger {
        parcel_list: vec![broken(), package("", "ab")],
    };
    let cases = [case("Ledger", "Ledger", &ledger, ledger.validate())];
    let expected = rust_report(&cases);
    assert_eq!(
        expected,
        "\
Ledger: refused
    'parcelList.0.itemLabel': too short: minimum length is 1, got 0
    'parcelList.0.parcelCode': too short: minimum length is 2, got 1
    'parcelList.1.itemLabel': too short: minimum length is 1, got 0
    'parcelList': too short: minimum length is 3, got 2
    'parcelList': a ledger holds no spare pair
"
    );
    let Some(swift) = printed(&driver_of(&cases)) else {
        return;
    };
    assert_eq!(swift, expected);
}

/// A bound on a field typed with a brand is measured through the member its wrapper answers its
/// text with, declared above the field or below it, and on an alias of a string declared above as
/// the string it is.
#[test]
fn a_bound_on_a_named_type_measures_the_string_it_holds() {
    let waybill = |later_code: &str, short_code: &str, titled: &str| Waybill {
        later_code: LaterCode(later_code.to_owned()),
        short_code: Code(short_code.to_owned()),
        titled: Title::from(titled),
    };
    let broken = waybill("a", "abcd", "t");
    let refused = waybill("ab", "a", "ti");
    // A read hands back no waybill whose code its own type refuses, so this one is built.
    let cases = [
        case(
            "Waybill, each bound broken",
            "Waybill",
            &broken,
            broken.validate(),
        ),
        Case {
            answered: refused.validate(),
            name: "Waybill, a code its own type refuses",
            read: concat!(
                r#"Waybill(laterCode: LaterCode(value: "ab"), "#,
                r#"shortCode: Code(value: "a"), titled: "ti")"#,
            )
            .to_owned(),
        },
    ];
    let expected = rust_report(&cases);
    assert_eq!(
        expected,
        "\
Waybill, each bound broken: refused
    'laterCode': too short: minimum length is 2, got 1
    'shortCode': too long: maximum length is 3, got 4
    'titled': too short: minimum length is 2, got 1
Waybill, a code its own type refuses: refused
    'shortCode': too short: minimum length is 2, got 1
"
    );
    let Some(swift) = printed(&driver_of(&cases)) else {
        return;
    };
    assert_eq!(swift, expected);
}

#[test]
fn a_brand_runs_its_bound_and_one_with_a_parameter_at_its_declared_default_alone() {
    let short = Code("a".to_owned());
    let long = Code("ab".to_owned());
    let named = Docket("a".to_owned());
    let kept = Docket("ab".to_owned());
    let numbered = Docket(7_u32);
    let sealed = SealedCode(Code("abcde".to_owned()));
    let cases = [
        case("Code, one letter", "Code", &short, short.validate()),
        case("Code", "Code", &long, long.validate()),
        case(
            "SealedCode, five letters",
            "SealedCode",
            &sealed,
            sealed.validate(),
        ),
        case(
            "Docket<String>, one letter",
            "Docket<String>",
            &named,
            named.validate(),
        ),
        case("Docket<String>", "Docket<String>", &kept, kept.validate()),
        case(
            "Docket<UInt32>",
            "Docket<UInt32>",
            &numbered,
            numbered.validate(),
        ),
    ];
    let expected = rust_report(&cases);
    assert_eq!(
        expected,
        "\
Code, one letter: refused
    too short: minimum length is 2, got 1
Code: passes
SealedCode, five letters: refused
    too long: maximum length is 4, got 5
Docket<String>, one letter: refused
    too short: minimum length is 2, got 1
Docket<String>: passes
Docket<UInt32>: passes
"
    );
    let text = format!(
        "print({}.swiftSchemaBoundText)",
        read_as("SealedCode", &sealed)
    );
    let Some(swift) = printed(&format!("{}\n{text}", driver_of(&cases))) else {
        return;
    };
    assert_eq!(
        swift,
        format!("{expected}abcde\n"),
        "a brand over a brand answers its text through the brand it holds"
    );
}

/// An alias of a string declared below the type that holds it is a name the emitter has not
/// written yet, which it reads as a brand's wrapper and as a type with a `validate()`: the Swift
/// compiler names each member a string does not have.
#[test]
fn an_alias_of_a_string_declared_below_its_holder_does_not_build() {
    let docking = Docking {
        berth_title: LaterTitle::from("north"),
    };
    assert_eq!(docking.validate(), Ok(()));
    let quay = Quay {
        quay_title: LaterTitle::from("north"),
    };
    let declared = vec![
        docking_swift::swift_definition(),
        quay_swift::swift_definition(),
        later_title_swift::swift_definition(),
    ];
    let lines = driver(&[
        ("Docking", read_as("Docking", &docking)),
        ("Quay", read_as("Quay", &quay)),
    ]);
    let source = module(declared, "", &lines);
    let Some(run) = judged(
        "swift-validate",
        RUNTIME_VAR,
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    ) else {
        return;
    };
    assert!(!run.succeeded, "the module built: {}", run.stdout);
    for answered in [
        format!("error: {ALIAS_BELOW_ERROR}"),
        "error: value of type 'LaterTitle' (aka 'String') has no member 'validate'".to_owned(),
    ] {
        assert!(run.stderr.contains(&answered), "got: {}", run.stderr);
    }
}

/// The README shows the functions these runs compile, the names they print and the two errors
/// they read.
#[test]
fn the_readme_shows_what_these_runs_compile_and_read() {
    let readme = include_str!("../../README.md");
    for shown in [
        SHOWN_FUNCTIONS,
        ALIAS_BELOW_ERROR,
        "error: cannot find 'withinParcel' in scope",
        "`'Loose.freightLabel'`",
        "`'Boxed'`, and `'Boxed.itemLabel'` for a model it holds",
        "`'Split.1'`",
        "`'held.haulLabel'`",
        "`'held'`",
        "`'held.1'`",
        "`'liftLabel'`",
    ] {
        assert!(
            readme.contains(shown),
            "the README no longer shows: {shown}"
        );
    }
}

/// The cases of a type that holds another at its own parameter under a check, and of members typed
/// with the name of a list or of a text under a length.
fn counted_name_cases() -> Vec<Case> {
    let archived = |entries: &[&str]| Archive {
        casket: Casket {
            entries: entries.iter().map(|&entry| entry.to_owned()).collect(),
            owner_id: "o".to_owned(),
        },
    };
    let archived_by_number = Archive {
        casket: Casket {
            entries: Vec::new(),
            owner_id: 7_u32,
        },
    };
    let (unfilled, filled) = (archived(&[]), archived(&["a"]));
    let poster = |slogans: usize, mottos: usize, some: usize, carried: usize| Poster {
        slogans: Slogans(vec!["w".to_owned(); slogans]),
        mottos: vec!["w".to_owned(); mottos],
        some: SomeSlogans(Slogans(vec!["w".to_owned(); some])),
        carried: Carried(vec!["w".to_owned(); carried]),
    };
    let (poster_kept, empty, wide) = (poster(1, 1, 1, 1), poster(0, 0, 0, 0), poster(3, 1, 3, 1));
    vec![
        case("Poster", "Poster", &poster_kept, poster_kept.validate()),
        case("Poster, no item", "Poster", &empty, empty.validate()),
        case("Poster, too many items", "Poster", &wide, wide.validate()),
        case(
            "Archive<String>, no entry",
            "Archive<String>",
            &unfilled,
            unfilled.validate(),
        ),
        case(
            "Archive<String>",
            "Archive<String>",
            &filled,
            filled.validate(),
        ),
        case(
            "Archive<UInt32>, no entry",
            "Archive<UInt32>",
            &archived_by_number,
            archived_by_number.validate(),
        ),
    ]
}

#[test]
fn a_check_runs_at_every_filling_and_on_a_type_that_holds_nothing() {
    let named: Crate<String> = Crate {
        codes: Vec::new(),
        label: String::new(),
    };
    let numbered: Crate<u32> = Crate {
        codes: Vec::new(),
        label: "c".to_owned(),
    };
    let kept: Crate<u32> = Crate {
        codes: vec![4],
        label: "c".to_owned(),
    };
    let mut cases = counted_name_cases();
    cases.extend([
        case(
            "Crate<String>, empty",
            "Crate<String>",
            &named,
            named.validate(),
        ),
        case(
            "Crate<UInt32>, empty",
            "Crate<UInt32>",
            &numbered,
            numbered.validate(),
        ),
        case("Crate<UInt32>", "Crate<UInt32>", &kept, kept.validate()),
        case(
            "ClosedDock",
            "ClosedDock",
            &ClosedDock,
            ClosedDock.validate(),
        ),
    ]);
    let expected = rust_report(&cases);
    assert_eq!(
        expected,
        "\
Poster: passes
Poster, no item: refused
    'carried': too short: minimum length is 1, got 0
    'mottos': too short: minimum length is 1, got 0
    'slogans': too short: minimum length is 1, got 0
    'some': too short: minimum length is 1, got 0
Poster, too many items: refused
    'slogans': too long: maximum length is 2, got 3
    'some': too long: maximum length is 2, got 3
Archive<String>, no entry: refused
    'casket.entries': a casket holds at least one entry
Archive<String>: passes
Archive<UInt32>, no entry: refused
    'casket.entries': a casket holds at least one entry
Crate<String>, empty: refused
    'label': too short: minimum length is 1, got 0
    'codes': a crate holds at least one code
Crate<UInt32>, empty: refused
    'codes': a crate holds at least one code
Crate<UInt32>: passes
ClosedDock: refused
    a closed dock takes no parcel
"
    );
    let Some(swift) = printed(&driver_of(&cases)) else {
        return;
    };
    assert_eq!(swift, expected);
}

/// A field named `validate` is declared apart from the method and keeps its key, and a field named
/// after a local of the method is read as the field.
#[test]
fn a_member_named_after_the_method_or_a_local_of_it_is_read_and_judged() {
    let inspection = Inspection {
        errors: Vec::new(),
        validate: "v".to_owned(),
    };
    let cases = [case(
        "Inspection",
        "Inspection",
        &inspection,
        inspection.validate(),
    )];
    let expected = rust_report(&cases);
    assert_eq!(
        expected,
        "\
Inspection: refused
    'errors': too short: minimum length is 1, got 0
"
    );
    let shown = format!("print({}.validate_)", read_as("Inspection", &inspection));
    let Some(swift) = printed(&format!("{}\n{shown}", driver_of(&cases))) else {
        return;
    };
    assert_eq!(swift, format!("{expected}v\n"));
}

/// A slot of a tuple struct is named by its position in the array serde writes, and the one slot
/// of a struct serde writes as the slot's value by nothing: the sentence alone, or the key of
/// whatever holds the struct.
#[test]
fn a_slot_of_a_struct_is_named_by_the_step_serde_writes() {
    // serde writes a struct of one slot as that slot's value.
    assert_eq!(
        serde_json::to_value(Strongbox {
            seal: Sealed(String::new()),
        })
        .unwrap(),
        serde_json::json!({ "seal": "" })
    );
    let read: Vec<(&str, String)> = vec![
        (
            "a slot of a tuple struct of several",
            read_as(
                "Consignment",
                &Consignment(String::new(), package("", "ab")),
            ),
        ),
        (
            "the one slot of a struct",
            read_as("Sealed", &Sealed(String::new())),
        ),
        (
            "the one slot of a struct, held",
            read_as(
                "Strongbox",
                &Strongbox {
                    seal: Sealed(String::new()),
                },
            ),
        ),
    ];
    let Some(swift) = printed(&driver(&read)) else {
        return;
    };
    assert_eq!(
        swift,
        "\
a slot of a tuple struct of several: refused
    '0': too short: minimum length is 1, got 0
    '1.itemLabel': too short: minimum length is 1, got 0
the one slot of a struct: refused
    too short: minimum length is 1, got 0
the one slot of a struct, held: refused
    'seal': too short: minimum length is 1, got 0
"
    );
}

/// With no tag attribute serde writes a variant's payload under the variant's name, which is the
/// step ahead of a member's key and of a slot's position, and the whole name of the one slot. An
/// enum's own string check reaches the text its variants hold.
#[test]
fn a_member_of_a_variant_with_no_tag_is_named_under_the_variant() {
    let read: Vec<(&str, String)> = vec![
        (
            "the one slot",
            read_as("Freight", &Freight::Boxed(String::new())),
        ),
        (
            "a named member",
            read_as(
                "Freight",
                &Freight::Loose {
                    freight_label: String::new(),
                },
            ),
        ),
        (
            "a slot of several",
            read_as("Freight", &Freight::Split(String::new(), 0)),
        ),
        ("no member", read_as("Freight", &Freight::Unladen)),
        (
            "a model in the one slot",
            read_as("Cargo", &Cargo::Boxed(package("", "ab"))),
        ),
        ("no model", read_as("Cargo", &Cargo::Unladen)),
        (
            "an enum's check, a named member",
            read_as(
                "Berth",
                &Berth::Named {
                    quay_name: "a".to_owned(),
                    quay_number: 0,
                },
            ),
        ),
        (
            "an enum's check, a slot",
            read_as("Berth", &Berth::Tagged("a".to_owned())),
        ),
        ("an enum's check, no member", read_as("Berth", &Berth::Open)),
    ];
    let Some(swift) = printed(&driver(&read)) else {
        return;
    };
    assert_eq!(
        swift,
        "\
the one slot: refused
    'Boxed': too short: minimum length is 1, got 0
a named member: refused
    'Loose.freightLabel': too short: minimum length is 1, got 0
a slot of several: refused
    'Split.0': too short: minimum length is 1, got 0
    'Split.1': too small: minimum is 1, got 0
no member: passes
a model in the one slot: refused
    'Boxed.itemLabel': too short: minimum length is 1, got 0
no model: passes
an enum's check, a named member: refused
    'Named.quayName': too short: minimum length is 2, got 1
an enum's check, a slot: refused
    'Tagged': too short: minimum length is 2, got 1
an enum's check, no member: passes
"
    );
}

/// Under a tag and a content key the content key is the step, under a tag alone a member is
/// beside the tag and takes no step of the variant's, and an untagged variant writes its slot as
/// the value itself. A plain enum holds nothing to judge.
#[test]
fn a_member_of_a_variant_under_a_tag_is_named_by_the_steps_serde_writes() {
    // No variant of an untagged enum takes a payload that breaks its bound, so the two are built.
    for unread in [
        Tally::Named(String::new()),
        Tally::Counted { tally_count: 0 },
    ] {
        serde_json::from_value::<Tally>(serde_json::to_value(unread).unwrap()).unwrap_err();
    }
    let read: Vec<(&str, String)> = vec![
        (
            "tag and content, the one slot",
            read_as("Haul", &Haul::Boxed(String::new())),
        ),
        (
            "tag and content, a named member",
            read_as(
                "Haul",
                &Haul::Loose {
                    haul_label: String::new(),
                },
            ),
        ),
        (
            "tag and content, a slot of several",
            read_as("Haul", &Haul::Split(String::new(), 0)),
        ),
        (
            "tag and content, no member",
            read_as("Haul", &Haul::Unladen),
        ),
        (
            "tag alone, a named member",
            read_as(
                "Lift",
                &Lift::Loose {
                    lift_label: String::new(),
                },
            ),
        ),
        (
            "tag alone, the one slot",
            read_as("Lift", &Lift::Boxed(package("", "ab"))),
        ),
        (
            "tag alone, the enum's own check",
            read_as("Lift", &Lift::Unladen),
        ),
        ("untagged, a slot", r#"Tally.named("")"#.to_owned()),
        (
            "untagged, a named member",
            "Tally.counted(TallyCountedPayload(tallyCount: 0))".to_owned(),
        ),
        ("a plain enum", read_as("Grade", &Grade::Express)),
    ];
    let Some(swift) = printed(&driver(&read)) else {
        return;
    };
    assert_eq!(
        swift,
        "\
tag and content, the one slot: refused
    'held': too short: minimum length is 1, got 0
tag and content, a named member: refused
    'held.haulLabel': too short: minimum length is 1, got 0
tag and content, a slot of several: refused
    'held.0': too short: minimum length is 1, got 0
    'held.1': too small: minimum is 1, got 0
tag and content, no member: passes
tag alone, a named member: refused
    'liftLabel': too short: minimum length is 1, got 0
tag alone, the one slot: refused
    'itemLabel': too short: minimum length is 1, got 0
tag alone, the enum's own check: refused
    a lift carries something
untagged, a slot: refused
    too short: minimum length is 1, got 0
untagged, a named member: refused
    'tallyCount': too small: minimum is 1, got 0
a plain enum: passes
"
    );
}

/// The missing function is named by the Swift compiler, where the check calls it.
#[test]
fn a_check_with_no_swift_function_does_not_build() {
    let lines = driver(&[("Parcel", read_as("Parcel", &parcel("A", &["x"], &["x"])))]);
    let source = module(definitions(), "", &lines);
    let Some(run) = judged(
        "swift-validate",
        RUNTIME_VAR,
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    ) else {
        return;
    };
    assert!(!run.succeeded, "the module built: {}", run.stdout);
    for missing in ["withoutDuplicates", "withinParcel"] {
        let named = format!("error: cannot find '{missing}' in scope");
        assert!(run.stderr.contains(&named), "got: {}", run.stderr);
    }
}
