//! A tuple struct that declares one slot, which serde writes as the slot's value, read and written
//! by the emitted Swift types compiled with warnings as errors: over a number, a text, a model, a
//! list, an optional, a map Swift keys natively and one under a key it wraps, a date and a tuple,
//! in a type that declares a parameter and in one that declares none, and held in a field, in a
//! list and in a variant's one slot under each tagging serde accepts. Each payload the serde read
//! reads prints what it prints, and a value that breaks a rule is refused as the Rust `validate()`
//! reports it, the one slot under no step.

#![cfg(feature = "swift")]

use core::fmt::Write as _;
use std::collections::HashMap;

#[cfg(feature = "chrono")]
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde::de::DeserializeOwned;

#[cfg(feature = "chrono")]
use super::lone_slots_models::{
    StowedStamp, Ticked, TickedMillis, stowed_stamp_swift, ticked_millis_swift, ticked_swift,
};
use super::lone_slots_models::{
    AdjacentStow, Bale, BalePair, Baled, Bales, BalesByName, BalesByNumber, Clipped, Count,
    LabelsByNumber, MaybeBale, Moniker, Sack, Sacked, Single, Stow, StowedByName, StowedByNumber,
    StowedList, StowedMaybe, StowedPair, TaggedStow, Tote, UntaggedStow, Unskipped,
    adjacent_stow_swift, bale_pair_swift, bale_swift, baled_swift, bales_by_name_swift,
    bales_by_number_swift, bales_swift, clipped_swift, count_swift, labels_by_number_swift,
    maybe_bale_swift, moniker_swift, sack_swift, sacked_swift, single_swift, stow_swift,
    stowed_by_name_swift, stowed_by_number_swift, stowed_list_swift, stowed_maybe_swift,
    stowed_pair_swift, tagged_stow_swift, tote_swift, unskipped_swift, untagged_stow_swift,
};
use super::run_swift_read::normalized;
use super::runtime::judged;

/// The payload serde writes for the value `$held`, which breaks a rule, held to what the Rust
/// `validate()` answers of it.
macro_rules! refused {
    ($name:expr, $read_as:literal, $held:expr) => {{
        let held = $held;
        refused_row($name, $read_as, &held, held.validate())
    }};
}

/// The payloads of each variant of `$held`, an enum under no tag attribute or under a tag and a
/// content key, read as `$read_as`, each named for `$tagging`.
macro_rules! stow_rows {
    ($held:ty, $read_as:literal, $tagging:literal) => {{
        type Held = $held;
        vec![
            read_row(
                concat!("a number, ", $tagging),
                $read_as,
                &Held::Counted(Count(5)),
            ),
            read_row(
                concat!("a list of models, ", $tagging),
                $read_as,
                &Held::Listed(vec![Baled(bale("l"))]),
            ),
            read_row(
                concat!("a text, ", $tagging),
                $read_as,
                &Held::Named(Moniker("a".to_owned())),
            ),
            read_row(
                concat!("a model, ", $tagging),
                $read_as,
                &Held::Packed(Baled(bale("l"))),
            ),
            read_row(
                concat!("a model with a parameter, ", $tagging),
                $read_as,
                &Held::Sacked(Sacked(sack("l"))),
            ),
            refused!(
                concat!("a number below its bound, ", $tagging),
                $read_as,
                Held::Counted(Count(0))
            ),
            refused!(
                concat!("a list of models, unlabelled, ", $tagging),
                $read_as,
                Held::Listed(vec![Baled(bale(""))])
            ),
            refused!(
                concat!("an empty text, ", $tagging),
                $read_as,
                Held::Named(Moniker(String::new()))
            ),
            refused!(
                concat!("a model, unlabelled, ", $tagging),
                $read_as,
                Held::Packed(Baled(bale("")))
            ),
        ]
    }};
}

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_SWIFT";

/// What a run reads a payload with, and prints the answer with: the value written back, or each
/// message of a refusal.
const HARNESS: &str = r#"
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

/// One payload the run reads: its name, the Swift type that reads it, the payload, and what the
/// Swift read is held to.
struct Row {
    expected: String,
    name: &'static str,
    payload: String,
    read_as: &'static str,
}

/// The payload serde writes for `held`, held to what the serde read prints of it: the value
/// written back.
fn read_row<Held>(name: &'static str, read_as: &'static str, held: &Held) -> Row
where
    Held: DeserializeOwned + Serialize,
{
    let payload = serde_json::to_string(held).unwrap();
    let read: Held = serde_json::from_str(&payload).unwrap();
    Row {
        expected: format!("{name}: read {}\n", serde_json::to_string(&read).unwrap()),
        name,
        payload,
        read_as,
    }
}

/// The payload serde writes for `held`, which breaks a rule, held to what the Rust `validate()`
/// answers of it.
fn refused_row<Held>(
    name: &'static str,
    read_as: &'static str,
    held: &Held,
    answered: Result<(), Vec<String>>,
) -> Row
where
    Held: DeserializeOwned + Serialize,
{
    let payload = serde_json::to_string(held).unwrap();
    assert!(serde_json::from_str::<Held>(&payload).is_err(), "{payload}");
    let mut expected = format!("{name}: refused\n");
    for message in answered.unwrap_err() {
        let _ = writeln!(expected, "    {message}");
    }
    Row {
        expected,
        name,
        payload,
        read_as,
    }
}

/// A bale labelled `label`.
fn bale(label: &str) -> Bale {
    Bale {
        label: label.to_owned(),
    }
}

/// A sack labelled `label`, holding `7`.
fn sack(label: &str) -> Sack<u8> {
    Sack {
        label: label.to_owned(),
        stowed: 7,
    }
}

/// A date with a fraction of a second.
#[cfg(feature = "chrono")]
fn date() -> DateTime<Utc> {
    DateTime::from_timestamp_millis(1_760_329_020_250).unwrap()
}

/// The payloads of each struct of one slot that declares no parameter.
fn plain_rows() -> Vec<Row> {
    let mut rows = vec![
        read_row("a number", "Count", &Count(5)),
        read_row("a text", "Moniker", &Moniker("a".to_owned())),
        read_row("a model", "Baled", &Baled(bale("l"))),
        read_row("a list", "Bales", &Bales(vec![bale("l"), bale("m")])),
        read_row("an optional", "MaybeBale", &MaybeBale(Some(bale("l")))),
        read_row("an optional, none", "MaybeBale", &MaybeBale(None)),
        read_row(
            "a map keyed natively",
            "BalesByName",
            &BalesByName(HashMap::from([("n".to_owned(), bale("l"))])),
        ),
        read_row(
            "a map under a key Swift wraps",
            "BalesByNumber",
            &BalesByNumber(HashMap::from([(1, bale("l")), (2, bale("m"))])),
        ),
        read_row(
            "a map of texts under a key Swift wraps",
            "LabelsByNumber",
            &LabelsByNumber(HashMap::from([(1, "a".to_owned()), (2, "b".to_owned())])),
        ),
        read_row("a tuple", "BalePair", &BalePair((bale("l"), 3))),
        read_row("a slot under skip", "Unskipped", &Unskipped("s".to_owned())),
        read_row(
            "two slots, one skipped",
            "Clipped",
            &Clipped("a".to_owned(), 0),
        ),
        refused!("a number below its bound", "Count", Count(0)),
        refused!("an empty text", "Moniker", Moniker(String::new())),
        refused!("a model, unlabelled", "Baled", Baled(bale(""))),
        refused!("an empty list", "Bales", Bales(Vec::new())),
        refused!(
            "a list, one unlabelled",
            "Bales",
            Bales(vec![bale("l"), bale("")])
        ),
        refused!(
            "an optional, unlabelled",
            "MaybeBale",
            MaybeBale(Some(bale("")))
        ),
        refused!(
            "a map keyed natively, unlabelled",
            "BalesByName",
            BalesByName(HashMap::from([("n".to_owned(), bale(""))]))
        ),
        refused!(
            "a map under a key Swift wraps, unlabelled",
            "BalesByNumber",
            BalesByNumber(HashMap::from([(1, bale(""))]))
        ),
        refused!("a tuple, unlabelled", "BalePair", BalePair((bale(""), 3))),
        refused!(
            "two slots, one skipped, empty",
            "Clipped",
            Clipped(String::new(), 0)
        ),
    ];
    #[cfg(feature = "chrono")]
    rows.extend([
        read_row("a date", "Ticked", &Ticked(date())),
        read_row("a date as a number", "TickedMillis", &TickedMillis(date())),
    ]);
    rows
}

/// The payloads of each struct of one slot that declares a parameter.
fn generic_rows() -> Vec<Row> {
    let mut rows = vec![
        read_row("the parameter, a number", "Single<UInt32>", &Single(5_u32)),
        read_row(
            "the parameter, a text",
            "Single<String>",
            &Single("a".to_owned()),
        ),
        read_row("the parameter, a model", "Single<Bale>", &Single(bale("l"))),
        read_row(
            "a model that holds the parameter",
            "Sacked<UInt8>",
            &Sacked(sack("l")),
        ),
        read_row(
            "a list of the parameter",
            "StowedList<String>",
            &StowedList(vec!["a".to_owned()]),
        ),
        read_row(
            "an optional parameter",
            "StowedMaybe<String>",
            &StowedMaybe(Some("a".to_owned())),
        ),
        read_row(
            "an optional parameter, none",
            "StowedMaybe<String>",
            &StowedMaybe::<String>(None),
        ),
        read_row(
            "a map of the parameter keyed natively",
            "StowedByName<UInt32>",
            &StowedByName(HashMap::from([("n".to_owned(), 1_u32)])),
        ),
        read_row(
            "a map of the parameter under a key Swift wraps",
            "StowedByNumber<String>",
            &StowedByNumber(HashMap::from([(1, "a".to_owned()), (2, "b".to_owned())])),
        ),
        read_row(
            "a model beside the parameter",
            "StowedPair<String>",
            &StowedPair((bale("l"), "a".to_owned())),
        ),
        refused!(
            "a model that holds the parameter, unlabelled",
            "Sacked<UInt8>",
            Sacked(sack(""))
        ),
        refused!(
            "an empty list of the parameter",
            "StowedList<String>",
            StowedList::<String>(Vec::new())
        ),
        refused!(
            "a model beside the parameter, unlabelled",
            "StowedPair<String>",
            StowedPair((bale(""), "a".to_owned()))
        ),
    ];
    #[cfg(feature = "chrono")]
    rows.push(read_row(
        "a date beside the parameter",
        "StowedStamp<String>",
        &StowedStamp((date(), "a".to_owned())),
    ));
    rows
}

/// [`Tote`] whose models are labelled `label` and whose count is `count`.
fn tote(label: &str, count: u32) -> Tote {
    Tote {
        baled: Baled(bale(label)),
        counts: vec![Count(1), Count(count)],
        moniker: Moniker(label.to_owned()),
        sacked: Sacked(sack(label)),
        single: Single(9),
    }
}

/// The payloads of each holder of a struct of one slot.
fn holder_rows() -> Vec<Row> {
    let mut rows = vec![
        read_row("in a field and in a list", "Tote", &tote("l", 2)),
        refused!(
            "in a field and in a list, each broken",
            "Tote",
            tote("", 0)
        ),
    ];
    rows.extend(stow_rows!(Stow, "Stow", "no tag attribute"));
    rows.extend(stow_rows!(
        AdjacentStow,
        "AdjacentStow",
        "a tag and a content key"
    ));
    rows.extend([
        read_row("a number, untagged", "UntaggedStow", &UntaggedStow::Counted(Count(5))),
        read_row(
            "a list of models, untagged",
            "UntaggedStow",
            &UntaggedStow::Listed(vec![Baled(bale("l"))]),
        ),
        read_row(
            "a text, untagged",
            "UntaggedStow",
            &UntaggedStow::Named(Moniker("a".to_owned())),
        ),
        read_row(
            "a model, untagged",
            "UntaggedStow",
            &UntaggedStow::Packed(Baled(bale("l"))),
        ),
        refused!(
            "a number below its bound, untagged",
            "UntaggedStow",
            UntaggedStow::Counted(Count(0))
        ),
        refused!(
            "a model, unlabelled, untagged",
            "UntaggedStow",
            UntaggedStow::Packed(Baled(bale("")))
        ),
        read_row(
            "a model, a tag alone",
            "TaggedStow",
            &TaggedStow::Packed(Baled(bale("l"))),
        ),
        read_row(
            "a model with a parameter, a tag alone",
            "TaggedStow",
            &TaggedStow::Sacked(Sacked(sack("l"))),
        ),
        refused!(
            "a model, unlabelled, a tag alone",
            "TaggedStow",
            TaggedStow::Packed(Baled(bale("")))
        ),
    ]);
    rows
}

/// The text of every model compiles with warnings as errors, and every payload reads and writes in
/// Swift as the serde read reads and writes it: the slot's value, with nothing around it. A value
/// that breaks a rule is refused as the Rust `validate()` reports it, the one slot under no step.
#[test]
fn a_struct_of_one_slot_reads_and_writes_in_swift_as_serde_does_wherever_it_is_held() {
    // serde reads a slot it skips as its default.
    assert_eq!(serde_json::from_str::<Clipped>(r#"["a"]"#).unwrap().1, 0);
    let mut rows = plain_rows();
    rows.extend(generic_rows());
    rows.extend(holder_rows());
    let expected: String = rows.iter().map(|row| row.expected.as_str()).collect();
    for answered in [
        "a number: read 5\n",
        "a number below its bound: refused\n    too small: minimum is 1, got 0\n",
        "a model, unlabelled: refused\n    'label': too short",
        "two slots, one skipped, empty: refused\n    '0': too short",
        "a number below its bound, no tag attribute: refused\n    'Counted': too small",
    ] {
        assert!(
            expected.contains(answered),
            "what the serde read writes and the Rust `validate()` answers: {expected}"
        );
    }
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
        bale_swift::swift_definition(),
        sack_swift::swift_definition(),
        count_swift::swift_definition(),
        moniker_swift::swift_definition(),
        baled_swift::swift_definition(),
        bales_swift::swift_definition(),
        maybe_bale_swift::swift_definition(),
        bales_by_name_swift::swift_definition(),
        bales_by_number_swift::swift_definition(),
        labels_by_number_swift::swift_definition(),
        bale_pair_swift::swift_definition(),
        unskipped_swift::swift_definition(),
        clipped_swift::swift_definition(),
        single_swift::swift_definition(),
        sacked_swift::swift_definition(),
        stowed_list_swift::swift_definition(),
        stowed_maybe_swift::swift_definition(),
        stowed_by_name_swift::swift_definition(),
        stowed_by_number_swift::swift_definition(),
        stowed_pair_swift::swift_definition(),
        tote_swift::swift_definition(),
        stow_swift::swift_definition(),
        adjacent_stow_swift::swift_definition(),
        untagged_stow_swift::swift_definition(),
        tagged_stow_swift::swift_definition(),
    ];
    #[cfg(feature = "chrono")]
    parts.extend([
        ticked_swift::swift_definition(),
        ticked_millis_swift::swift_definition(),
        stowed_stamp_swift::swift_definition(),
    ]);
    parts.extend([HARNESS.to_owned(), calls.join("\n")]);
    let source = parts.join("\n\n");
    let Some(run) = judged(
        "swift-lone-slots",
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
