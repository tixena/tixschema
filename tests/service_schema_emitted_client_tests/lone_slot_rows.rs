//! The payloads of a tuple struct that declares one slot, which serde writes as the slot's value,
//! that the Zod schema and each client are held to: over a number, a text, a model, a list, an
//! optional, a map Swift keys natively and one under a key it wraps, a date and a tuple, in a type
//! that declares a parameter and in one that declares none, and held in a field, in a list, as a
//! map's value, as a type argument, flattened and in a variant's one slot under each tagging serde
//! accepts, and a struct of one slot and a brand over an externally tagged enum flattened and
//! beside a tag. Each payload the serde read reads prints what it prints, and a value that breaks a
//! rule is refused as the Rust `validate()` reports it, the one slot under no step.

use core::fmt::Write as _;
use std::collections::HashMap;

#[cfg(feature = "chrono")]
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::lone_slots_models::{
    AdjacentStow, Bale, BalePair, Baled, Bales, BalesByName, BalesByNumber, Clipped, Count, Girth,
    Girthed, LabelsByNumber, Laden, MaybeBale, Moniker, Rigger, Sack, Sacked, Single, Sling, Slung,
    Stow, StowedByName, StowedByNumber, StowedList, StowedMaybe, StowedPair, Strop, TaggedStow,
    Tote, Twine, Twined, Unskipped, UntaggedStow,
};
#[cfg(feature = "chrono")]
use super::lone_slots_models::{StowedStamp, Ticked, TickedMillis};

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
            read_row(
                concat!("a struct of one slot over a list, ", $tagging),
                $read_as,
                &Held::Stacked(Bales(vec![bale("l")])),
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
            refused!(
                concat!("an empty list, ", $tagging),
                $read_as,
                Held::Stacked(Bales(Vec::new()))
            ),
        ]
    }};
}

/// One payload a run reads: its name, the type that reads it as Swift writes it, the payload, and
/// what the read is held to.
pub struct Row {
    pub expected: String,
    pub name: &'static str,
    pub payload: String,
    pub read_as: &'static str,
}

/// One line a run prints: a value read, held as the JSON it is, or any other line.
#[derive(Debug, PartialEq)]
enum Line {
    Other(String),
    Read(String, Value),
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

/// A sack labelled `label`, holding `stowed`.
fn sack_of<Stowed>(label: &str, stowed: Stowed) -> Sack<Stowed> {
    Sack {
        label: label.to_owned(),
        stowed,
    }
}

/// A sack labelled `label`, holding `7`.
fn sack(label: &str) -> Sack<u8> {
    sack_of(label, 7)
}

/// A date with a fraction of a second.
#[cfg(feature = "chrono")]
fn date() -> DateTime<Utc> {
    DateTime::from_timestamp_millis(1_760_329_020_250).unwrap()
}

/// The payloads of each struct of one slot that declares no parameter.
fn plain_rows() -> Vec<Row> {
    vec![
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
        #[cfg(feature = "chrono")]
        read_row("a date", "Ticked", &Ticked(date())),
        #[cfg(feature = "chrono")]
        read_row("a date as a number", "TickedMillis", &TickedMillis(date())),
    ]
}

/// The payloads of each struct of one slot that declares a parameter.
fn generic_rows() -> Vec<Row> {
    vec![
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
        #[cfg(feature = "chrono")]
        read_row(
            "a date beside the parameter",
            "StowedStamp<String>",
            &StowedStamp((date(), "a".to_owned())),
        ),
    ]
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

/// [`Laden`] whose models and texts are labelled `label` and whose counts are `count`, but those it
/// holds as a type argument and the brand, each of which judges itself as it is read.
fn laden(label: &str, count: u32) -> Laden {
    let bales = || Bales(vec![bale(label)]);
    Laden {
        baled: Baled(bale(label)),
        bales: bales(),
        carried_bales: sack_of("c", Bales(vec![bale("l")])),
        carried_count: sack_of("c", Count(2)),
        carried_model: sack_of("c", Baled(bale("l"))),
        carried_moniker: sack_of("c", Moniker("l".to_owned())),
        count: Count(count),
        counts_by_name: HashMap::from([("n".to_owned(), Count(count))]),
        models: vec![Baled(bale(label))],
        models_by_name: HashMap::from([("n".to_owned(), Baled(bale(label)))]),
        monikers: vec![Moniker(label.to_owned())],
        monikers_by_name: HashMap::from([("n".to_owned(), Moniker(label.to_owned()))]),
        stacks: vec![bales()],
        stacks_by_name: HashMap::from([("n".to_owned(), bales())]),
        twined: Twined(Twine {
            twine: "t".to_owned(),
        }),
    }
}

/// The payloads of a struct of one slot and a brand over an externally tagged enum with a unit
/// variant, flattened and beside a tag, where serde writes the variant as an entry of its name.
fn entry_rows() -> Vec<Row> {
    vec![
        read_row(
            "over a unit variant, flattened",
            "Strop",
            &Strop {
                count: 1,
                slung: Slung(Sling::Slack),
            },
        ),
        read_row(
            "over a variant that holds members, flattened",
            "Strop",
            &Strop {
                count: 1,
                slung: Slung(Sling::Taut { tension: 2 }),
            },
        ),
        read_row(
            "a brand over a unit variant, flattened",
            "Girthed",
            &Girthed {
                count: 1,
                girth: Girth(Sling::Slack),
            },
        ),
        read_row(
            "a brand over a variant that holds members, flattened",
            "Girthed",
            &Girthed {
                count: 1,
                girth: Girth(Sling::Taut { tension: 2 }),
            },
        ),
        read_row(
            "over a unit variant, beside a tag",
            "Rigger",
            &Rigger::Slung(Slung(Sling::Slack)),
        ),
        read_row(
            "a brand over a variant that holds members, beside a tag",
            "Rigger",
            &Rigger::Girth(Girth(Sling::Taut { tension: 2 })),
        ),
        read_row(
            "a brand over a unit variant, beside a tag",
            "Rigger",
            &Rigger::Girth(Girth(Sling::Slack)),
        ),
    ]
}

/// The payloads of each holder of a struct of one slot.
fn holder_rows() -> Vec<Row> {
    let mut rows = vec![
        read_row("in a field and in a list", "Tote", &tote("l", 2)),
        refused!("in a field and in a list, each broken", "Tote", tote("", 0)),
        read_row(
            "under a key, in a list, as a map's value, as a type argument and flattened",
            "Laden",
            &laden("l", 2),
        ),
        refused!(
            "under a key, in a list, as a map's value and flattened, each broken",
            "Laden",
            laden("", 0)
        ),
        // What fills a type parameter judges itself as it is read, and no `validate()` of its
        // holder walks it.
        refused_row(
            "a number below its bound, as a type argument",
            "Laden",
            &Laden {
                carried_count: sack_of("c", Count(0)),
                ..laden("l", 2)
            },
            Count(0).validate(),
        ),
        refused_row(
            "a model, unlabelled, as a type argument",
            "Laden",
            &Laden {
                carried_model: sack_of("c", Baled(bale(""))),
                ..laden("l", 2)
            },
            Baled(bale("")).validate(),
        ),
    ];
    rows.extend(entry_rows());
    rows.extend(stow_rows!(Stow, "Stow", "no tag attribute"));
    rows.extend(stow_rows!(
        AdjacentStow,
        "AdjacentStow",
        "a tag and a content key"
    ));
    rows.extend([
        read_row(
            "a number, untagged",
            "UntaggedStow",
            &UntaggedStow::Counted(Count(5)),
        ),
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

/// Every payload of the table, in the order each run reads them.
pub fn rows() -> Vec<Row> {
    let mut rows = plain_rows();
    rows.extend(generic_rows());
    rows.extend(holder_rows());
    rows
}

/// What a run prints of `rows` where it reads each as serde does: each row's expected lines.
pub fn expected(rows: &[Row]) -> String {
    rows.iter().map(|row| row.expected.as_str()).collect()
}

/// The lines of `printed`, the value after each `: read ` held as the JSON it is.
fn lines(printed: &str) -> Vec<Line> {
    printed
        .lines()
        .map(|line| match line.split_once(": read ") {
            Some((name, json)) => Line::Read(
                name.to_owned(),
                serde_json::from_str(json)
                    .unwrap_or_else(|_| Value::String(format!("not JSON: {json}"))),
            ),
            None => Line::Other(line.to_owned()),
        })
        .collect()
}

/// Holds what `client` printed of `rows` to what serde and the Rust `validate()` answer of each,
/// each value read compared as the JSON it is.
#[cfg(any(feature = "dart", feature = "kotlin"))]
pub fn held_to_rows(rows: &[Row], printed: &str, client: &str) {
    assert_eq!(
        lines(printed),
        lines(&expected(rows)),
        "the {client} read is not serde's:\n{printed}"
    );
}

/// Holds what `client` printed of `rows`, which names no message of a refusal, to whether serde
/// reads each and to what it writes back of each it reads, compared as the JSON it is.
pub fn held_to_answers(rows: &[Row], printed: &str, client: &str) {
    let answers = rows
        .iter()
        .filter_map(|row| row.expected.lines().next())
        .fold(String::new(), |mut answers, first| {
            let _ = writeln!(answers, "{first}");
            answers
        });
    assert_eq!(
        lines(printed),
        lines(&answers),
        "the {client} read is not serde's:\n{printed}"
    );
}

/// The table holds what the serde read writes and the Rust `validate()` answers, the one slot of
/// a struct under no step and a slot among two under its position.
#[test]
fn the_table_holds_what_serde_and_validate_answer() {
    // serde reads a slot it skips as its default.
    assert_eq!(serde_json::from_str::<Clipped>(r#"["a"]"#).unwrap().1, 0);
    let expected = expected(&rows());
    for answered in [
        "a number: read 5\n",
        "a slot under skip: read \"s\"\n",
        "a number below its bound: refused\n    too small: minimum is 1, got 0\n",
        "a model, unlabelled: refused\n    'label': too short",
        "two slots, one skipped, empty: refused\n    '0': too short",
        "a number below its bound, no tag attribute: refused\n    'Counted': too small",
        "as a type argument: refused\n    too small: minimum is 1, got 0\n",
        "    'counts_by_name.n': too small: minimum is 1, got 0\n",
        "    'stacks.0.0.label': too short",
    ] {
        assert!(
            expected.contains(answered),
            "no `{answered}` in what the serde read writes and the Rust `validate()` answers: \
             {expected}"
        );
    }
}
