//! An enum with a braced variant serde reads no member of: the macro reads that variant through a
//! read of its own and closes the enum, so a key no member declares is refused in every variant.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A variant whose members serde skips, one built from its own default.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Counted {
    Reset {
        #[serde(skip, default = "seven")]
        start: u32,
        #[serde(skip_serializing, skip_deserializing)]
        total: u32,
    },
    Running {
        count: u32,
    },
}

/// A generic variant whose member serde skips, built from `Default`.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Kept<T> {
    Cleared {
        #[serde(skip)]
        held: T,
    },
    Stored {
        count: u32,
    },
}

/// [`Kept`] under a bound its author wrote, which asks nothing of `T`.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
#[serde(bound(deserialize = ""))]
pub enum Shelved<T> {
    Cleared {
        #[serde(skip)]
        items: Vec<T>,
    },
    Stored {
        count: u32,
    },
}

/// A type with no `Default`, which `Shelved` holds under the bound its author wrote.
#[derive(Debug)]
pub struct Unmade;

/// An enum that borrows, whose variant of no member read names its lifetime.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Labelled<'text> {
    Blank {
        #[serde(skip)]
        note: Option<&'text str>,
    },
    Named {
        label: &'text str,
    },
}

/// An enum under a tag alone that publishes `validate()`.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(tag = "kind")]
pub enum Dimmer {
    Off {
        #[serde(skip)]
        since: u32,
    },
    On {
        #[model_schema_prop(minimum = 1)]
        level: u8,
    },
}

/// An enum under a tag and a content key, closed by its author.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields, tag = "kind", content = "body")]
pub enum Switch {
    Off {
        #[serde(skip)]
        since: u32,
    },
    On {
        level: u8,
    },
}

fn seven() -> u32 {
    7
}

fn refusal<T>(json: &str) -> String
where
    T: DeserializeOwned,
{
    serde_json::from_str::<T>(json)
        .map(drop)
        .unwrap_err()
        .to_string()
}

#[test]
fn a_variant_whose_every_member_serde_skips_builds_each_from_its_default_and_refuses_a_key() {
    assert_eq!(
        serde_json::from_str::<Counted>(r#"{"Reset":{}}"#).unwrap(),
        CountedDraft::Reset { start: 7, total: 0 }
            .validate()
            .unwrap()
    );
    assert_eq!(
        serde_json::to_string(&CountedDraft::Reset { start: 1, total: 2 }).unwrap(),
        r#"{"Reset":{}}"#
    );
    for (json, named) in [
        (
            r#"{"Reset":{"start":1}}"#,
            "unknown field `start`, there are no fields",
        ),
        (
            r#"{"Running":{"count":1,"stray":1}}"#,
            "unknown field `stray`, expected `count`",
        ),
    ] {
        let told = refusal::<Counted>(json);
        assert!(told.starts_with(named), "{json}: {told}");
    }
}

#[test]
fn a_generic_enum_reads_its_variant_of_no_member_under_the_bounds_serde_asks() {
    assert_eq!(
        serde_json::from_str::<Kept<String>>(r#"{"Cleared":{}}"#).unwrap(),
        KeptDraft::Cleared {
            held: String::new()
        }
        .validate()
        .unwrap()
    );
    let shelved = serde_json::from_str::<Shelved<Unmade>>(r#"{"Cleared":{}}"#).unwrap();
    assert!(matches!(&*shelved, ShelvedDraft::Cleared { items } if items.is_empty()));
    for told in [
        refusal::<Kept<u32>>(r#"{"Cleared":{"stray":1}}"#),
        refusal::<Kept<u32>>(r#"{"Stored":{"count":1,"stray":1}}"#),
        refusal::<Shelved<Unmade>>(r#"{"Cleared":{"stray":1}}"#),
    ] {
        assert!(told.starts_with("unknown field `stray`"), "{told}");
    }
}

#[test]
fn an_enum_that_borrows_reads_its_variant_of_no_member_and_refuses_a_key() {
    assert_eq!(
        serde_json::from_str::<Labelled<'_>>(r#"{"Blank":{}}"#).unwrap(),
        LabelledDraft::Blank { note: None }.validate().unwrap()
    );
    assert_eq!(
        serde_json::from_str::<Labelled<'_>>(r#"{"Named":{"label":"dock"}}"#).unwrap(),
        LabelledDraft::Named { label: "dock" }.validate().unwrap()
    );
    for json in [
        r#"{"Blank":{"stray":1}}"#,
        r#"{"Named":{"label":"dock","stray":1}}"#,
    ] {
        let told = serde_json::from_str::<Labelled<'_>>(json)
            .map(drop)
            .unwrap_err()
            .to_string();
        assert!(told.starts_with("unknown field `stray`"), "{json}: {told}");
    }
}

#[test]
fn an_enum_that_publishes_validate_reads_its_variant_of_no_member_and_refuses_a_key() {
    assert_eq!(
        serde_json::from_str::<Dimmer>(r#"{"kind":"Off"}"#).unwrap(),
        DimmerDraft::Off { since: 0 }.validate().unwrap()
    );
    let beside_off = refusal::<Dimmer>(r#"{"kind":"Off","stray":1}"#);
    assert!(
        beside_off.starts_with("unknown field `stray`, there are no fields"),
        "{beside_off}"
    );
    assert!(refusal::<Dimmer>(r#"{"kind":"On","level":0}"#).contains("level"));
    let beside_on = refusal::<Dimmer>(r#"{"kind":"On","level":1,"stray":1}"#);
    assert!(
        beside_on.starts_with("unknown field `stray`"),
        "{beside_on}"
    );
}

#[test]
fn an_enum_its_author_closed_reads_its_variant_of_no_member_and_refuses_a_key() {
    assert_eq!(
        serde_json::from_str::<Switch>(r#"{"kind":"Off","body":{}}"#).unwrap(),
        SwitchDraft::Off { since: 0 }.validate().unwrap()
    );
    for (json, named) in [
        (
            r#"{"kind":"Off","body":{"stray":1}}"#,
            "unknown field `stray`, there are no fields",
        ),
        (
            r#"{"kind":"Off","body":{},"stray":1}"#,
            r#"invalid value: string "stray", expected "kind" or "body""#,
        ),
        (
            r#"{"kind":"On","body":{"level":1,"stray":1}}"#,
            "unknown field `stray`, expected `level`",
        ),
    ] {
        let told = refusal::<Switch>(json);
        assert!(told.starts_with(named), "{json}: {told}");
    }
}
