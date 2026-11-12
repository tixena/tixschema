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
    AdjacentStow, Bale, BaleDraft, BalePair, BalePairDraft, Baled, BaledDraft, Bales, BalesByName,
    BalesByNameDraft, BalesByNumber, BalesByNumberDraft, BalesDraft, Clipped, ClippedDraft, Count,
    CountDraft, Girth, Girthed, GirthedDraft, LabelsByNumber, LabelsByNumberDraft, Laden,
    LadenDraft, MaybeBale, MaybeBaleDraft, Moniker, MonikerDraft, Rigger, RiggerDraft, Sack,
    SackDraft, Sacked, SackedDraft, Single, SingleDraft, Sling, SlingDraft, Slung, SlungDraft,
    Stow, StowedByName, StowedByNameDraft, StowedByNumber, StowedByNumberDraft, StowedList,
    StowedListDraft, StowedMaybe, StowedMaybeDraft, StowedPair, StowedPairDraft, Strop, StropDraft,
    TaggedStow, TaggedStowDraft, Tote, ToteDraft, Twine, TwineDraft, Twined, Unskipped,
    UnskippedDraft, UntaggedStow, UntaggedStowDraft,
};
#[cfg(feature = "chrono")]
use super::lone_slots_models::{
    StowedStamp, StowedStampDraft, Ticked, TickedDraft, TickedMillis, TickedMillisDraft,
};

/// The payload serde writes for the value `$held`, which breaks a rule, held to what the Rust
/// `validate()` answers of it.
macro_rules! refused {
    ($name:expr, $read_as:literal, $held:expr) => {{
        let held = $held;
        refused_row($name, $read_as, &held, held.validate().map(|_valid| ()))
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
                &Held::Counted(CountDraft(5)),
            ),
            read_row(
                concat!("a list of models, ", $tagging),
                $read_as,
                &Held::Listed(vec![BaledDraft(bale("l"))]),
            ),
            read_row(
                concat!("a text, ", $tagging),
                $read_as,
                &Held::Named(MonikerDraft("a".to_owned())),
            ),
            read_row(
                concat!("a model, ", $tagging),
                $read_as,
                &Held::Packed(BaledDraft(bale("l"))),
            ),
            read_row(
                concat!("a model with a parameter, ", $tagging),
                $read_as,
                &Held::Sacked(SackedDraft(sack("l"))),
            ),
            read_row(
                concat!("a struct of one slot over a list, ", $tagging),
                $read_as,
                &Held::Stacked(BalesDraft(vec![bale("l")])),
            ),
            refused!(
                concat!("a number below its bound, ", $tagging),
                $read_as,
                Held::Counted(CountDraft(0))
            ),
            refused!(
                concat!("a list of models, unlabelled, ", $tagging),
                $read_as,
                Held::Listed(vec![BaledDraft(bale(""))])
            ),
            refused!(
                concat!("an empty text, ", $tagging),
                $read_as,
                Held::Named(MonikerDraft(String::new()))
            ),
            refused!(
                concat!("a model, unlabelled, ", $tagging),
                $read_as,
                Held::Packed(BaledDraft(bale("")))
            ),
            refused!(
                concat!("an empty list, ", $tagging),
                $read_as,
                Held::Stacked(BalesDraft(Vec::new()))
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
    BaleDraft {
        label: label.to_owned(),
    }
    .validate()
    .unwrap()
}

/// A sack labelled `label`, holding `stowed`.
fn sack_of<Stowed>(label: &str, stowed: Stowed) -> Sack<Stowed> {
    SackDraft {
        label: label.to_owned(),
        stowed,
    }
    .validate()
    .unwrap()
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
        read_row("a number", "Count", &CountDraft(5)),
        read_row("a text", "Moniker", &MonikerDraft("a".to_owned())),
        read_row("a model", "Baled", &BaledDraft(bale("l"))),
        read_row("a list", "Bales", &BalesDraft(vec![bale("l"), bale("m")])),
        read_row("an optional", "MaybeBale", &MaybeBaleDraft(Some(bale("l")))),
        read_row("an optional, none", "MaybeBale", &MaybeBaleDraft(None)),
        read_row(
            "a map keyed natively",
            "BalesByName",
            &BalesByNameDraft(HashMap::from([("n".to_owned(), bale("l"))])),
        ),
        read_row(
            "a map under a key Swift wraps",
            "BalesByNumber",
            &BalesByNumberDraft(HashMap::from([(1, bale("l")), (2, bale("m"))])),
        ),
        read_row(
            "a map of texts under a key Swift wraps",
            "LabelsByNumber",
            &LabelsByNumberDraft(HashMap::from([(1, "a".to_owned()), (2, "b".to_owned())])),
        ),
        read_row("a tuple", "BalePair", &BalePairDraft((bale("l"), 3))),
        read_row(
            "a slot under skip",
            "Unskipped",
            &UnskippedDraft("s".to_owned()),
        ),
        read_row(
            "two slots, one skipped",
            "Clipped",
            &ClippedDraft("a".to_owned(), 0),
        ),
        refused!("a number below its bound", "Count", CountDraft(0)),
        refused!("an empty text", "Moniker", MonikerDraft(String::new())),
        refused!("a model, unlabelled", "Baled", BaledDraft(bale(""))),
        refused!("an empty list", "Bales", BalesDraft(Vec::new())),
        refused!(
            "a list, one unlabelled",
            "Bales",
            BalesDraft(vec![bale("l"), bale("")])
        ),
        refused!(
            "an optional, unlabelled",
            "MaybeBale",
            MaybeBaleDraft(Some(bale("")))
        ),
        refused!(
            "a map keyed natively, unlabelled",
            "BalesByName",
            BalesByNameDraft(HashMap::from([("n".to_owned(), bale(""))]))
        ),
        refused!(
            "a map under a key Swift wraps, unlabelled",
            "BalesByNumber",
            BalesByNumberDraft(HashMap::from([(1, bale(""))]))
        ),
        refused!(
            "a tuple, unlabelled",
            "BalePair",
            BalePairDraft((bale(""), 3))
        ),
        refused!(
            "two slots, one skipped, empty",
            "Clipped",
            ClippedDraft(String::new(), 0)
        ),
        #[cfg(feature = "chrono")]
        read_row("a date", "Ticked", &TickedDraft(date())),
        #[cfg(feature = "chrono")]
        read_row(
            "a date as a number",
            "TickedMillis",
            &TickedMillisDraft(date()),
        ),
    ]
}

/// The payloads of each struct of one slot that declares a parameter.
fn generic_rows() -> Vec<Row> {
    vec![
        read_row(
            "the parameter, a number",
            "Single<UInt32>",
            &SingleDraft(5_u32),
        ),
        read_row(
            "the parameter, a text",
            "Single<String>",
            &SingleDraft("a".to_owned()),
        ),
        read_row(
            "the parameter, a model",
            "Single<Bale>",
            &SingleDraft(bale("l")),
        ),
        read_row(
            "a model that holds the parameter",
            "Sacked<UInt8>",
            &SackedDraft(sack("l")),
        ),
        read_row(
            "a list of the parameter",
            "StowedList<String>",
            &StowedListDraft(vec!["a".to_owned()]),
        ),
        read_row(
            "an optional parameter",
            "StowedMaybe<String>",
            &StowedMaybeDraft(Some("a".to_owned())),
        ),
        read_row(
            "an optional parameter, none",
            "StowedMaybe<String>",
            &StowedMaybeDraft::<String>(None),
        ),
        read_row(
            "a map of the parameter keyed natively",
            "StowedByName<UInt32>",
            &StowedByNameDraft(HashMap::from([("n".to_owned(), 1_u32)])),
        ),
        read_row(
            "a map of the parameter under a key Swift wraps",
            "StowedByNumber<String>",
            &StowedByNumberDraft(HashMap::from([(1, "a".to_owned()), (2, "b".to_owned())])),
        ),
        read_row(
            "a model beside the parameter",
            "StowedPair<String>",
            &StowedPairDraft((bale("l"), "a".to_owned())),
        ),
        refused!(
            "a model that holds the parameter, unlabelled",
            "Sacked<UInt8>",
            SackedDraft(sack(""))
        ),
        refused!(
            "an empty list of the parameter",
            "StowedList<String>",
            StowedListDraft::<String>(Vec::new())
        ),
        refused!(
            "a model beside the parameter, unlabelled",
            "StowedPair<String>",
            StowedPairDraft((bale(""), "a".to_owned()))
        ),
        #[cfg(feature = "chrono")]
        read_row(
            "a date beside the parameter",
            "StowedStamp<String>",
            &StowedStampDraft((date(), "a".to_owned())),
        ),
    ]
}

/// [`Tote`] whose models are labelled `label` and whose count is `count`.
fn tote(label: &str, count: u32) -> Tote {
    ToteDraft {
        baled: BaledDraft(bale(label)).validate().unwrap(),
        counts: vec![
            CountDraft(1).validate().unwrap(),
            CountDraft(count).validate().unwrap(),
        ],
        moniker: MonikerDraft(label.to_owned()).validate().unwrap(),
        sacked: SackedDraft(sack(label)).validate().unwrap(),
        single: SingleDraft(9).validate().unwrap(),
    }
    .validate()
    .unwrap()
}

/// [`Laden`] whose models and texts are labelled `label` and whose counts are `count`, but those it
/// holds as a type argument and the brand, each of which judges itself as it is read.
fn laden(label: &str, count: u32) -> Laden {
    let bales = || BalesDraft(vec![bale(label)]);
    LadenDraft {
        baled: BaledDraft(bale(label)).validate().unwrap(),
        bales: bales().validate().unwrap(),
        carried_bales: sack_of("c", BalesDraft(vec![bale("l")]).validate().unwrap()),
        carried_count: sack_of("c", CountDraft(2).validate().unwrap()),
        carried_model: sack_of("c", BaledDraft(bale("l")).validate().unwrap()),
        carried_moniker: sack_of("c", MonikerDraft("l".to_owned()).validate().unwrap()),
        count: CountDraft(count).validate().unwrap(),
        counts_by_name: HashMap::from([("n".to_owned(), CountDraft(count))]),
        models: vec![BaledDraft(bale(label)).validate().unwrap()],
        models_by_name: HashMap::from([("n".to_owned(), BaledDraft(bale(label)))]),
        monikers: vec![MonikerDraft(label.to_owned()).validate().unwrap()],
        monikers_by_name: HashMap::from([("n".to_owned(), MonikerDraft(label.to_owned()))]),
        stacks: vec![bales().validate().unwrap()],
        stacks_by_name: HashMap::from([("n".to_owned(), bales())]),
        twined: Twined::new(TwineDraft {
            twine: "t".to_owned(),
        })
        .unwrap(),
    }
    .validate()
    .unwrap()
}

/// The payloads of a struct of one slot and a brand over an externally tagged enum with a unit
/// variant, flattened and beside a tag, where serde writes the variant as an entry of its name.
fn entry_rows() -> Vec<Row> {
    vec![
        read_row(
            "over a unit variant, flattened",
            "Strop",
            &StropDraft {
                count: 1,
                slung: SlungDraft(SlingDraft::Slack.validate().unwrap())
                    .validate()
                    .unwrap(),
            },
        ),
        read_row(
            "over a variant that holds members, flattened",
            "Strop",
            &StropDraft {
                count: 1,
                slung: SlungDraft(SlingDraft::Taut { tension: 2 }.validate().unwrap())
                    .validate()
                    .unwrap(),
            },
        ),
        read_row(
            "a brand over a unit variant, flattened",
            "Girthed",
            &GirthedDraft {
                count: 1,
                girth: Girth::new(SlingDraft::Slack).unwrap(),
            },
        ),
        read_row(
            "a brand over a variant that holds members, flattened",
            "Girthed",
            &GirthedDraft {
                count: 1,
                girth: Girth::new(SlingDraft::Taut { tension: 2 }).unwrap(),
            },
        ),
        read_row(
            "over a unit variant, beside a tag",
            "Rigger",
            &RiggerDraft::Slung(
                SlungDraft(SlingDraft::Slack.validate().unwrap())
                    .validate()
                    .unwrap(),
            ),
        ),
        read_row(
            "a brand over a variant that holds members, beside a tag",
            "Rigger",
            &RiggerDraft::Girth(Girth::new(SlingDraft::Taut { tension: 2 }).unwrap()),
        ),
        read_row(
            "a brand over a unit variant, beside a tag",
            "Rigger",
            &RiggerDraft::Girth(Girth::new(SlingDraft::Slack).unwrap()),
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
            &LadenDraft {
                carried_count: sack_of("c", CountDraft(0).validate().unwrap()),
                ..laden("l", 2).into()
            },
            CountDraft(0).validate().map(|_valid| ()),
        ),
        refused_row(
            "a model, unlabelled, as a type argument",
            "Laden",
            &LadenDraft {
                carried_model: sack_of("c", BaledDraft(bale("")).validate().unwrap()),
                ..laden("l", 2).into()
            },
            BaledDraft(bale("")).validate().map(|_valid| ()),
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
            &UntaggedStowDraft::Counted(CountDraft(5).validate().unwrap()),
        ),
        read_row(
            "a list of models, untagged",
            "UntaggedStow",
            &UntaggedStowDraft::Listed(vec![BaledDraft(bale("l")).validate().unwrap()]),
        ),
        read_row(
            "a text, untagged",
            "UntaggedStow",
            &UntaggedStowDraft::Named(MonikerDraft("a".to_owned()).validate().unwrap()),
        ),
        read_row(
            "a model, untagged",
            "UntaggedStow",
            &UntaggedStowDraft::Packed(BaledDraft(bale("l")).validate().unwrap()),
        ),
        refused!(
            "a number below its bound, untagged",
            "UntaggedStow",
            UntaggedStowDraft::Counted(CountDraft(0).validate().unwrap())
        ),
        refused!(
            "a model, unlabelled, untagged",
            "UntaggedStow",
            UntaggedStowDraft::Packed(BaledDraft(bale("")).validate().unwrap())
        ),
        read_row(
            "a model, a tag alone",
            "TaggedStow",
            &TaggedStowDraft::Packed(BaledDraft(bale("l")).validate().unwrap()),
        ),
        read_row(
            "a model with a parameter, a tag alone",
            "TaggedStow",
            &TaggedStowDraft::Sacked(SackedDraft(sack("l")).validate().unwrap()),
        ),
        refused!(
            "a model, unlabelled, a tag alone",
            "TaggedStow",
            TaggedStowDraft::Packed(BaledDraft(bale("")).validate().unwrap())
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
    let expected = if client == "Dart" {
        refused_beside(&expected(rows))
    } else {
        expected(rows)
    };
    assert_eq!(
        lines(printed),
        lines(&expected),
        "the {client} read is not serde's:\n{printed}"
    );
}

/// `told` as the Dart read answers it where a class fills a type parameter a holder flattens: the
/// class refuses the key of the flattened member beside it, where serde hands it its own keys.
#[cfg(any(feature = "dart", feature = "kotlin"))]
fn refused_beside(told: &str) -> String {
    const NAMES: [&str; 4] = [
        "under a key, in a list, as a map's value, as a type argument and flattened",
        "under a key, in a list, as a map's value and flattened, each broken",
        "a number below its bound, as a type argument",
        "a model, unlabelled, as a type argument",
    ];
    let mut out = String::new();
    let mut skipping = false;
    for line in told.lines() {
        let indented = line.starts_with("    ");
        if skipping && indented {
            continue;
        }
        skipping = false;
        if NAMES
            .iter()
            .any(|name| line.starts_with(&format!("{name}: ")))
        {
            let name = line.split(": ").next().unwrap_or_default();
            out.push_str(name);
            out.push_str(": refused\n    Unknown key of a Bale: twine\n");
            skipping = true;
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
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
