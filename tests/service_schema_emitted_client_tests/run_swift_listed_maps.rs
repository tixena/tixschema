//! A map under a key Swift reads through a wrapper, a number, a plain enum and a `bool`, in a list,
//! in a list of optionals, in an optional list and in an optional, read and written by the emitted
//! Swift types compiled with warnings as errors: as a field, as a member of a variant under each
//! tagging serde accepts, as a brand's value, in a tuple struct's one slot, in a tuple's slot, as a
//! map's value and in a variant's slot, in a type that declares a parameter and in one that
//! declares none. Each payload the serde read reads prints what it prints, and a key that is no
//! number is refused, as the serde read refuses it. A null as the value of a map under a key Swift
//! wraps is kept, as serde keeps it; a null in the one slot of an untagged variant is left out:
//! Swift reads it otherwise than serde for a reason of its own.

#![cfg(feature = "swift")]

use std::collections::HashMap;

#[cfg(feature = "chrono")]
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::listed_maps_models::{
    AdjacentGalley, AdjacentStowedGalley, Attic, Caddy, Cellar, Creel, Cupboard, Dresser, Galley,
    Hopper, Larder, Pantry, Rung, Sideboard, StowedAttic, StowedCaddy, StowedCellar, StowedCreel,
    StowedCupboard, StowedDresser, StowedGalley, StowedLarder, StowedPantry, StowedSideboard,
    TaggedGalley, TaggedStowedGalley, Trug, UntaggedGalley, UntaggedStowedGalley,
    adjacent_galley_swift, adjacent_stowed_galley_swift, attic_swift, caddy_swift, cellar_swift,
    creel_swift, cupboard_swift, dresser_swift, galley_swift, hopper_swift, larder_swift,
    pantry_swift, rung_swift, sideboard_swift, stowed_attic_swift, stowed_caddy_swift,
    stowed_cellar_swift, stowed_creel_swift, stowed_cupboard_swift, stowed_dresser_swift,
    stowed_galley_swift, stowed_larder_swift, stowed_pantry_swift, stowed_sideboard_swift,
    tagged_galley_swift, tagged_stowed_galley_swift, trug_swift, untagged_galley_swift,
    untagged_stowed_galley_swift,
};
#[cfg(feature = "chrono")]
use super::listed_maps_models::{
    Dateline, Plank, Timeline, dateline_swift, plank_swift, timeline_swift,
};
use super::run_swift_read::normalized;
use super::runtime::judged;

/// The payloads of each variant of `$held`, an enum under no tag attribute or under a tag and a
/// content key, read as `$read_as`, each named for `$tagging`.
macro_rules! galley_rows {
    ($held:ident, $read_as:literal, $tagging:literal) => {
        vec![
            read_row(
                concat!("a list in a variant's one slot, ", $tagging),
                $read_as,
                &$held::Listed(vec![numbered(&text())]),
            ),
            read_row(
                concat!("a list of optionals in a variant's one slot, ", $tagging),
                $read_as,
                &$held::ListedOrNone(vec![Some(flagged(&text())), None]),
            ),
            read_row(
                concat!("an optional in a variant's one slot, ", $tagging),
                $read_as,
                &$held::Maybe(Some(rungs(&text()))),
            ),
            read_row(
                concat!("an optional in a variant's one slot, none, ", $tagging),
                $read_as,
                &$held::Maybe(None),
            ),
            read_row(
                concat!("an optional list in a variant's one slot, ", $tagging),
                $read_as,
                &$held::MaybeListed(Some(vec![numbered(&text())])),
            ),
            read_row(
                concat!("an optional list in a variant's one slot, none, ", $tagging),
                $read_as,
                &$held::MaybeListed(None),
            ),
            read_row(
                concat!("a list and an optional in a variant's slots, ", $tagging),
                $read_as,
                &$held::Paired(vec![flagged(&text())], Some(rungs(&text()))),
            ),
            read_row(
                concat!("each shape as a member of a variant, ", $tagging),
                $read_as,
                &$held::Stocked {
                    flag_lists: vec![flagged(&text())],
                    maybe_number_lists: Some(vec![numbered(&text())]),
                    maybe_rungs: Some(rungs(&text())),
                    number_lists_or_none: vec![Some(numbered(&text())), None],
                },
            ),
            read_row(
                concat!(
                    "each shape as a member of a variant, no optional, ",
                    $tagging
                ),
                $read_as,
                &$held::Stocked {
                    flag_lists: Vec::new(),
                    maybe_number_lists: None,
                    maybe_rungs: None,
                    number_lists_or_none: vec![None],
                },
            ),
            refused_row(
                concat!("a variant under a key that is no number, ", $tagging),
                $read_as,
                &$held::Listed(vec![numbered(&text())]),
            ),
        ]
    };
}

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What a run reads a payload with, and prints the answer with: the value written back, or each
/// message of a refusal. `verbatim` is the function [`Pantry`] names.
const HARNESS: &str = r#"
func verbatim(_ payload: Any) -> Any { payload }

func row<Read: Codable>(_ name: String, _ type: Read.Type, _ payload: String) {
  do {
    let held = try JSONDecoder().decode(type, from: Data(payload.utf8))
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]
    print("\(name): read \(String(data: try encoder.encode(held), encoding: .utf8)!)")
  } catch DecodingError.dataCorrupted(let context) {
    print("\(name): refused")
    for line in context.debugDescription.components(separatedBy: "; ") { print("    \(line)") }
  } catch {
    print("\(name): \(error)")
  }
}
"#;

/// The key serde writes for the number `1`.
const ONE_WRITTEN: &str = r#""1":"#;

/// What the Swift read prints for a payload whose first number key is `x`.
const BAD_KEY: &str = "refused\n    bad key x";

/// One payload the run reads: its name, the Swift type that reads it, the payload, and what the
/// Swift read is held to.
struct Row {
    expected: String,
    name: &'static str,
    payload: String,
    read_as: &'static str,
}

/// `payload`, which the serde read reads as a `Held`, held to what that read prints: the value
/// written back.
fn payload_row<Held>(name: &'static str, read_as: &'static str, payload: String) -> Row
where
    Held: DeserializeOwned + Serialize,
{
    let read: Held = serde_json::from_str(&payload).unwrap();
    Row {
        expected: format!("{name}: read {}\n", serde_json::to_string(&read).unwrap()),
        name,
        payload,
        read_as,
    }
}

/// The payload serde writes for `held`, held to what the serde read prints of it.
fn read_row<Held>(name: &'static str, read_as: &'static str, held: &Held) -> Row
where
    Held: DeserializeOwned + Serialize,
{
    payload_row::<Held>(name, read_as, serde_json::to_string(held).unwrap())
}

/// The payload serde writes for `held` with its first number key `x`, which the serde read
/// refuses, held to [`BAD_KEY`].
fn refused_row<Held>(name: &'static str, read_as: &'static str, held: &Held) -> Row
where
    Held: DeserializeOwned + Serialize,
{
    let written = serde_json::to_string(held).unwrap();
    assert!(written.contains(ONE_WRITTEN), "{written}");
    let payload = written.replacen(ONE_WRITTEN, r#""x":"#, 1);
    assert!(serde_json::from_str::<Held>(&payload).is_err(), "{payload}");
    Row {
        expected: format!("{name}: {BAD_KEY}\n"),
        name,
        payload,
        read_as,
    }
}

/// A map of `held` under the keys `1` and `2`.
fn numbered<Held>(held: &Held) -> HashMap<u32, Held>
where
    Held: Clone,
{
    HashMap::from([(1, held.clone()), (2, held.clone())])
}

/// A map of `held` under both flags.
fn flagged<Held>(held: &Held) -> HashMap<bool, Held>
where
    Held: Clone,
{
    HashMap::from([(true, held.clone()), (false, held.clone())])
}

/// A map of `held` under both rungs.
fn rungs<Held>(held: &Held) -> HashMap<Rung, Held>
where
    Held: Clone,
{
    HashMap::from([(Rung::Low, held.clone()), (Rung::High, held.clone())])
}

/// The text `a`.
fn text() -> String {
    "a".to_owned()
}

/// [`Pantry`] holding each shape, or none of the optionals.
fn pantry(present: bool) -> Pantry {
    Pantry {
        defaulted: vec![numbered(&text())],
        flag_lists: vec![flagged(&text()), HashMap::new()],
        flag_lists_or_none: vec![Some(flagged(&text())), None],
        maybe_flag_lists: present.then(|| vec![flagged(&text())]),
        maybe_flags: present.then(|| flagged(&text())),
        maybe_number_lists: present.then(|| vec![numbered(&text())]),
        maybe_numbers: present.then(|| numbered(&text())),
        maybe_rung_lists: present.then(|| vec![rungs(&text())]),
        maybe_rungs: present.then(|| rungs(&text())),
        number_lists: vec![numbered(&text())],
        number_lists_or_none: vec![None, Some(numbered(&text()))],
        rung_lists: vec![rungs(&text())],
        rung_lists_or_none: vec![Some(rungs(&text())), None],
        verbatim_lists: vec![numbered(&text())],
    }
}

/// [`StowedPantry`] holding each shape, or none of the optionals.
fn stowed_pantry(present: bool) -> StowedPantry<String> {
    StowedPantry {
        defaulted: vec![numbered(&text())],
        flag_lists: vec![flagged(&text())],
        flag_lists_or_none: vec![None, Some(flagged(&text()))],
        maybe_flag_lists: present.then(|| vec![flagged(&text())]),
        maybe_flags: present.then(|| flagged(&text())),
        maybe_number_lists: present.then(|| vec![numbered(&text())]),
        maybe_numbers: present.then(|| numbered(&text())),
        maybe_rung_lists: present.then(|| vec![rungs(&text())]),
        maybe_rungs: present.then(|| rungs(&text())),
        number_lists: vec![numbered(&text()), HashMap::new()],
        number_lists_or_none: vec![Some(numbered(&text()))],
        rung_lists: vec![rungs(&text())],
        rung_lists_or_none: vec![None],
        verbatim_lists: vec![numbered(&text())],
    }
}

/// The JSON serde writes for `held`, without the key `defaulted`.
fn without_defaulted<Held>(held: &Held) -> String
where
    Held: Serialize,
{
    let mut written = serde_json::to_value(held).unwrap();
    if let Value::Object(entries) = &mut written {
        entries.remove("defaulted");
    }
    written.to_string()
}

/// The payloads of each struct of fields, in a type that declares a parameter and in one that
/// declares none, and of a list of maps of dates.
fn struct_rows() -> Vec<Row> {
    vec![
        read_row("each shape as a field", "Pantry", &pantry(true)),
        payload_row::<Pantry>(
            "each shape as a field, no optional and no default",
            "Pantry",
            without_defaulted(&pantry(false)),
        ),
        read_row(
            "each shape as a field that declares a parameter",
            "StowedPantry<String>",
            &stowed_pantry(true),
        ),
        payload_row::<StowedPantry<String>>(
            "each shape as a field that declares a parameter, no optional and no default",
            "StowedPantry<String>",
            without_defaulted(&stowed_pantry(false)),
        ),
        refused_row(
            "a field under a key that is no number",
            "Pantry",
            &pantry(false),
        ),
        #[cfg(feature = "chrono")]
        read_row(
            "a null entry under a key Swift wraps and under one it keys natively",
            "Dateline",
            &Dateline {
                maybe_by_name: HashMap::from([
                    ("n".to_owned(), None),
                    ("m".to_owned(), Some(Plank { label: text() })),
                ]),
                maybe_by_number: HashMap::from([(1, Some(date())), (2, None)]),
            },
        ),
        #[cfg(feature = "chrono")]
        read_row(
            "a list of maps of dates",
            "Timeline",
            &Timeline {
                by_name_lists: vec![HashMap::from([("n".to_owned(), date())])],
                by_number_lists: vec![numbered(&date())],
                maybe_by_name: Some(HashMap::from([("m".to_owned(), date())])),
            },
        ),
    ]
}

/// The payloads of each tuple, tuple struct of several slots and map that holds the maps, in a
/// type that declares a parameter and in one that declares none.
fn holder_rows() -> Vec<Row> {
    vec![
        read_row(
            "each shape in a tuple's slot",
            "Larder",
            &Larder {
                flag_lists_or_none: (vec![Some(flagged(&text())), None], 1),
                maybe_numbers: (Some(numbered(&text())), 2),
                maybe_rung_lists: (Some(vec![rungs(&text())]), 3),
                number_lists: (vec![numbered(&text())], 4),
            },
        ),
        read_row(
            "each shape in a tuple's slot, no optional",
            "Larder",
            &Larder {
                flag_lists_or_none: (vec![None], 1),
                maybe_numbers: (None, 2),
                maybe_rung_lists: (None, 3),
                number_lists: (Vec::new(), 4),
            },
        ),
        read_row(
            "each shape in a tuple's slot that declares a parameter",
            "StowedLarder<String>",
            &StowedLarder {
                flag_lists_or_none: (vec![Some(flagged(&text()))], 1),
                maybe_numbers: (Some(numbered(&text())), 2),
                maybe_rung_lists: (None, 3),
                number_lists: (vec![numbered(&text())], 4),
            },
        ),
        read_row(
            "each shape in a slot of a tuple struct",
            "Cellar",
            &Cellar(
                vec![numbered(&text())],
                vec![Some(flagged(&text())), None],
                Some(vec![rungs(&text())]),
                Some(numbered(&text())),
            ),
        ),
        read_row(
            "each shape in a slot of a tuple struct that declares a parameter",
            "StowedCellar<String>",
            &StowedCellar(
                vec![numbered(&text())],
                vec![None],
                None,
                Some(numbered(&text())),
            ),
        ),
        read_row(
            "each shape as a map's value",
            "Attic",
            &Attic {
                flag_lists_by_number: numbered(&vec![flagged(&text())]),
                maybe_number_lists_by_name: HashMap::from([
                    ("m".to_owned(), Some(vec![numbered(&text())])),
                    ("n".to_owned(), None),
                ]),
                maybe_rungs_by_rung: HashMap::from([
                    (Rung::Low, Some(rungs(&text()))),
                    (Rung::High, None),
                ]),
                number_lists_by_name: HashMap::from([("n".to_owned(), vec![numbered(&text())])]),
                rung_lists_or_none_by_name: HashMap::from([(
                    "r".to_owned(),
                    vec![Some(rungs(&text())), None],
                )]),
            },
        ),
        read_row(
            "each shape as a map's value that declares a parameter",
            "StowedAttic<String>",
            &StowedAttic {
                flag_lists_by_number: numbered(&vec![flagged(&text())]),
                maybe_number_lists_by_name: HashMap::from([("n".to_owned(), None)]),
                maybe_rungs_by_rung: HashMap::from([
                    (Rung::Low, Some(rungs(&text()))),
                    (Rung::High, None),
                ]),
                number_lists_by_name: HashMap::new(),
                rung_lists_or_none_by_name: HashMap::from([("r".to_owned(), vec![None])]),
            },
        ),
        refused_row(
            "a map's value under a key that is no number",
            "Attic",
            &Attic {
                flag_lists_by_number: HashMap::new(),
                maybe_number_lists_by_name: HashMap::new(),
                maybe_rungs_by_rung: HashMap::new(),
                number_lists_by_name: HashMap::from([("n".to_owned(), vec![numbered(&text())])]),
                rung_lists_or_none_by_name: HashMap::new(),
            },
        ),
    ]
}

/// A date with a fraction of whole milliseconds, which chrono writes with three digits.
#[cfg(feature = "chrono")]
fn date() -> DateTime<Utc> {
    DateTime::from_timestamp_millis(1_704_164_645_678).unwrap()
}

/// The payloads of each tuple struct of one slot and of each brand.
fn slot_and_brand_rows() -> Vec<Row> {
    vec![
        read_row(
            "a list in a tuple struct's one slot",
            "Cupboard",
            &Cupboard(vec![numbered(&text())]),
        ),
        refused_row(
            "a list in a tuple struct's one slot under a key that is no number",
            "Cupboard",
            &Cupboard(vec![numbered(&text())]),
        ),
        read_row(
            "a list of optionals in a tuple struct's one slot",
            "Sideboard",
            &Sideboard(vec![Some(flagged(&text())), None]),
        ),
        read_row(
            "an optional list in a tuple struct's one slot",
            "Dresser",
            &Dresser(Some(vec![rungs(&text())])),
        ),
        read_row(
            "an optional list in a tuple struct's one slot, none",
            "Dresser",
            &Dresser(None),
        ),
        read_row(
            "an optional in a tuple struct's one slot",
            "Caddy",
            &Caddy(Some(numbered(&text()))),
        ),
        read_row(
            "an optional in a tuple struct's one slot, none",
            "Caddy",
            &Caddy(None),
        ),
        read_row(
            "a list in a tuple struct's one slot that declares a parameter",
            "StowedCupboard<String>",
            &StowedCupboard(vec![numbered(&text())]),
        ),
        read_row(
            "a list of optionals in a tuple struct's one slot that declares a parameter",
            "StowedSideboard<String>",
            &StowedSideboard(vec![None, Some(flagged(&text()))]),
        ),
        read_row(
            "an optional list in a tuple struct's one slot that declares a parameter",
            "StowedDresser<String>",
            &StowedDresser(Some(vec![rungs(&text())])),
        ),
        read_row(
            "an optional in a tuple struct's one slot that declares a parameter",
            "StowedCaddy<String>",
            &StowedCaddy(Some(numbered(&text()))),
        ),
        read_row(
            "an optional in a tuple struct's one slot that declares a parameter, none",
            "StowedCaddy<String>",
            &StowedCaddy::<String>(None),
        ),
        read_row(
            "a brand over a list",
            "Creel",
            &Creel(vec![numbered(&text())]),
        ),
        refused_row(
            "a brand over a list under a key that is no number",
            "Creel",
            &Creel(vec![numbered(&text())]),
        ),
        read_row(
            "a brand over a list of optionals",
            "Trug",
            &Trug(vec![Some(flagged(&text())), None]),
        ),
        read_row(
            "a brand with a named member over a list",
            "Hopper",
            &Hopper {
                rung_lists: vec![rungs(&text())],
            },
        ),
        read_row(
            "a brand over a list that declares a parameter",
            "StowedCreel<String>",
            &StowedCreel(vec![rungs(&text())]),
        ),
    ]
}

/// The payloads of each variant that holds the maps under no tag attribute and under a tag and a
/// content key, in a type that declares a parameter and in one that declares none.
fn variant_rows() -> Vec<Row> {
    type Stowed = StowedGalley<String>;
    type AdjacentStowed = AdjacentStowedGalley<String>;
    let mut rows = galley_rows!(Galley, "Galley", "no tag attribute");
    rows.extend(galley_rows!(
        Stowed,
        "StowedGalley<String>",
        "no tag attribute, a parameter"
    ));
    rows.extend(galley_rows!(
        AdjacentGalley,
        "AdjacentGalley",
        "a tag and a content key"
    ));
    rows.extend(galley_rows!(
        AdjacentStowed,
        "AdjacentStowedGalley<String>",
        "a tag and a content key, a parameter"
    ));
    rows
}

/// The payloads of each variant that holds the maps under a tag alone and untagged, in a type that
/// declares a parameter and in one that declares none.
fn buffered_rows() -> Vec<Row> {
    vec![
        read_row(
            "each shape as a member of a variant, a tag alone",
            "TaggedGalley",
            &TaggedGalley::Stocked {
                maybe_rung_lists: Some(vec![rungs(&text())]),
                maybe_rungs: Some(rungs(&text())),
                rung_lists: vec![rungs(&text())],
                rung_lists_or_none: vec![Some(rungs(&text())), None],
            },
        ),
        read_row(
            "each shape as a member of a variant, no optional, a tag alone",
            "TaggedGalley",
            &TaggedGalley::Stocked {
                maybe_rung_lists: None,
                maybe_rungs: None,
                rung_lists: Vec::new(),
                rung_lists_or_none: vec![None],
            },
        ),
        read_row(
            "each shape as a member of a variant, a tag alone, a parameter",
            "TaggedStowedGalley<String>",
            &TaggedStowedGalley::Stocked {
                maybe_rung_lists: Some(vec![rungs(&text())]),
                maybe_rungs: None,
                rung_lists: vec![rungs(&text())],
                rung_lists_or_none: vec![None, Some(rungs(&text()))],
            },
        ),
        read_row(
            "a list in a variant's one slot, untagged",
            "UntaggedGalley",
            &UntaggedGalley::Listed(vec![rungs(&text())]),
        ),
        read_row(
            "a list of optionals in a variant's one slot, untagged",
            "UntaggedGalley",
            &UntaggedGalley::ListedOrNone(vec![Some(rungs(&text())), None]),
        ),
        read_row(
            "an optional in a variant's one slot, untagged",
            "UntaggedGalley",
            &UntaggedGalley::Maybe(Some(rungs(&text()))),
        ),
        read_row(
            "each shape as a member of a variant, untagged",
            "UntaggedGalley",
            &UntaggedGalley::Stocked {
                maybe_rung_lists: Some(vec![rungs(&text())]),
                maybe_rungs: Some(rungs(&text())),
                rung_lists: vec![rungs(&text())],
                rung_lists_or_none: vec![Some(rungs(&text())), None],
            },
        ),
        read_row(
            "a list in a variant's one slot, untagged, a parameter",
            "UntaggedStowedGalley<String>",
            &UntaggedStowedGalley::Listed(vec![rungs(&text())]),
        ),
        read_row(
            "a list of optionals in a variant's one slot, untagged, a parameter",
            "UntaggedStowedGalley<String>",
            &UntaggedStowedGalley::ListedOrNone(vec![None, Some(rungs(&text()))]),
        ),
        read_row(
            "an optional in a variant's one slot, untagged, a parameter",
            "UntaggedStowedGalley<String>",
            &UntaggedStowedGalley::Maybe(Some(rungs(&text()))),
        ),
        read_row(
            "each shape as a member of a variant, untagged, a parameter",
            "UntaggedStowedGalley<String>",
            &UntaggedStowedGalley::Stocked {
                maybe_rung_lists: None,
                maybe_rungs: Some(rungs(&text())),
                rung_lists: vec![rungs(&text())],
                rung_lists_or_none: vec![None],
            },
        ),
    ]
}

/// The text of every model compiles with warnings as errors, and every payload reads and writes in
/// Swift as the serde read reads and writes it: each map through the wrapper written for its keys,
/// handed on as its storage with no `try` around a conversion that throws nothing. A key that is
/// no number is refused by the wrapper, as the serde read refuses it.
#[test]
fn a_list_or_an_optional_of_maps_swift_wraps_reads_and_writes_in_swift_as_serde_does() {
    let mut rows = struct_rows();
    rows.extend(holder_rows());
    rows.extend(slot_and_brand_rows());
    rows.extend(variant_rows());
    rows.extend(buffered_rows());
    let expected: String = rows.iter().map(|row| row.expected.as_str()).collect();
    assert!(
        expected.contains(
            r#"each shape as a field, no optional and no default: read {"defaulted":[],"#
        ),
        "what the serde read writes: {expected}"
    );
    let calls: Vec<String> = rows
        .iter()
        .map(|row| {
            format!(
                "row(\"{}\", {}.self, #\"{}\"#)",
                row.name, row.read_as, row.payload
            )
        })
        .collect();
    let mut parts = vec![
        "import Foundation".to_owned(),
        rung_swift::swift_definition(),
        pantry_swift::swift_definition(),
        stowed_pantry_swift::swift_definition(),
        larder_swift::swift_definition(),
        stowed_larder_swift::swift_definition(),
        cellar_swift::swift_definition(),
        stowed_cellar_swift::swift_definition(),
        attic_swift::swift_definition(),
        stowed_attic_swift::swift_definition(),
        cupboard_swift::swift_definition(),
        sideboard_swift::swift_definition(),
        dresser_swift::swift_definition(),
        caddy_swift::swift_definition(),
        stowed_cupboard_swift::swift_definition(),
        stowed_sideboard_swift::swift_definition(),
        stowed_dresser_swift::swift_definition(),
        stowed_caddy_swift::swift_definition(),
        creel_swift::swift_definition(),
        trug_swift::swift_definition(),
        hopper_swift::swift_definition(),
        stowed_creel_swift::swift_definition(),
        galley_swift::swift_definition(),
        stowed_galley_swift::swift_definition(),
        adjacent_galley_swift::swift_definition(),
        adjacent_stowed_galley_swift::swift_definition(),
        tagged_galley_swift::swift_definition(),
        tagged_stowed_galley_swift::swift_definition(),
        untagged_galley_swift::swift_definition(),
        untagged_stowed_galley_swift::swift_definition(),
    ];
    #[cfg(feature = "chrono")]
    parts.extend([
        timeline_swift::swift_definition(),
        plank_swift::swift_definition(),
        dateline_swift::swift_definition(),
    ]);
    parts.extend([HARNESS.to_owned(), calls.join("\n")]);
    let source = parts.join("\n\n");
    let Some(run) = judged(
        "swift-listed-maps",
        RUNTIME_VAR,
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &source,
    ) else {
        return;
    };
    assert!(
        run.succeeded,
        "`swift -warnings-as-errors main.swift` failed.\n--- stdout ---\n{}\n--- stderr ---\n{}\n--- source ---\n{source}",
        run.stdout, run.stderr
    );
    assert_eq!(normalized(&run.stdout), normalized(&expected));
}
