//! The models of the flattened Swift reads `swift_tests/flattened.rs` pins and
//! `service_schema_emitted_client_tests/run_swift_flattened.rs` runs: a map flattened after a
//! struct, beside a member of its holder's own and in a variant under a tag key, a map flattened
//! ahead of a struct, two flattened maps, a map flattened after each member that takes none of the
//! keys it reads, and a map under a key Swift wraps, in a type that declares no parameter and in
//! one that declares one, of models, of dates and one that names a function. serde reads the
//! members in the order declared: a struct takes the keys it reads, and a map is handed every key
//! no member ahead of it took. Every holder has a member of its own, whose key no map is handed.

use alloc::collections::BTreeMap;

#[cfg(feature = "chrono")]
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::model_schema;

/// What [`Stopping`] is read from.
pub const STOPPING: &str = r#"{"label":"n","north":2,"note":"a"}"#;

/// What [`Frapping`] is read from.
pub const FRAPPING: &str = r#"{"north":2,"note":"a"}"#;

/// What [`Kedging`], [`Marling`], [`Mousing`] and [`Snaking`] are read from: a map of text.
pub const TEXTS: &str = r#"{"label":"n","north":"s","note":"a"}"#;

/// What [`Warping`] is read from.
pub const WARPING: &str = r#"{"north":2,"note":"a","south":3}"#;

/// What [`Bowsing`] is read from.
pub const BOWSING: &str = r#"{"label":"n","marker":"m","north":"s","note":"a"}"#;

/// What [`Clewing`] is read from.
pub const CLEWING: &str = r#"{"kind":"Held","label":"n","north":2,"note":"a"}"#;

/// What [`Parceling`] and [`Toggling`] are read from.
pub const PARCELING: &str = r#"{"North":2,"label":"n","note":"a"}"#;

/// What [`Worming`] is read from.
pub const WORMING: &str = r#"{"label":"n","north":{"mark":"x"},"note":"a"}"#;

/// What [`Faking`] is read from.
pub const FAKING: &str = r#"{"North":{"mark":"x"},"label":"n","note":"a"}"#;

/// What [`Serving`] is read from.
#[cfg(feature = "chrono")]
pub const SERVING: &str = r#"{"label":"n","north":"2024-01-02T03:04:05.678Z","note":"a"}"#;

/// A plain enum a map is keyed by, which Swift reads through a wrapper.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Quarter {
    North,
    South,
}

/// The member the structs below flatten.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Pennant {
    pub label: String,
}

/// A model with a rule, held as the value of a map.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Notch {
    #[model_schema_prop(minLength = 1)]
    pub mark: String,
}

/// A map flattened after a struct, with a member of its holder's own between them: serde hands
/// the map the keys neither took.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Stopping {
    #[serde(flatten)]
    pub flag: Pennant,
    pub note: String,
    #[serde(flatten)]
    pub rest: BTreeMap<String, u8>,
}

/// A map, the one flattened member, beside a member of its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Frapping {
    pub note: String,
    #[serde(flatten)]
    pub rest: BTreeMap<String, u8>,
}

/// A map flattened ahead of a struct, where serde hands it every key but its holder's own, the
/// struct's among them.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Kedging {
    #[serde(flatten)]
    pub extra: BTreeMap<String, String>,
    pub note: String,
    #[serde(flatten)]
    pub pennant: Pennant,
}

/// Two flattened maps, where serde hands each the same keys.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Warping {
    #[serde(flatten)]
    pub first: BTreeMap<String, u8>,
    pub note: String,
    #[serde(flatten)]
    pub second: BTreeMap<String, u8>,
}

/// A struct that names a function on itself, which serde reads as a value.
#[model_schema(preprocess = ["asGiven"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Reeving {
    pub label: String,
}

/// A struct with a flattened member of its own, which serde reads as a map.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Hanking {
    pub marker: String,
    #[serde(flatten)]
    pub pennant: Pennant,
}

/// A map flattened after a struct that names a function on itself, where serde hands it every key
/// but its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Marling {
    #[serde(flatten)]
    pub issued: Reeving,
    pub note: String,
    #[serde(flatten)]
    pub rest: BTreeMap<String, String>,
}

/// A map flattened after a member that names a function, where serde hands it every key but its
/// holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Mousing {
    #[model_schema_prop(preprocess = ["asGiven"])]
    #[serde(flatten)]
    pub flag: Pennant,
    pub note: String,
    #[serde(flatten)]
    pub rest: BTreeMap<String, String>,
}

/// A map flattened after a struct with a flattened member of its own, where serde hands it every
/// key but its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Bowsing {
    #[serde(flatten)]
    pub hanking: Hanking,
    pub note: String,
    #[serde(flatten)]
    pub rest: BTreeMap<String, String>,
}

/// A map flattened after a struct in a variant under a tag key, where serde hands it the keys
/// neither the tag, the variant's own member nor the struct took.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Clewing {
    Held {
        #[serde(flatten)]
        flag: Pennant,
        note: String,
        #[serde(flatten)]
        rest: BTreeMap<String, u8>,
    },
}

/// A map under a key Swift wraps, flattened after a struct.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Parceling {
    #[serde(flatten)]
    pub flag: Pennant,
    pub note: String,
    #[serde(flatten)]
    pub rest: BTreeMap<Quarter, u8>,
}

/// [`Parceling`] in a type that declares a parameter, which the map holds.
#[model_schema(default_types(Held = u8))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Toggling<Held> {
    #[serde(flatten)]
    pub flag: Pennant,
    pub note: String,
    #[serde(flatten)]
    pub rest: BTreeMap<Quarter, Held>,
}

/// A map of models with a rule, flattened after a struct.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Worming {
    #[serde(flatten)]
    pub flag: Pennant,
    pub note: String,
    #[serde(flatten)]
    pub rest: BTreeMap<String, Notch>,
}

/// A map of models with a rule under a key Swift wraps, flattened after a struct.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Faking {
    #[serde(flatten)]
    pub flag: Pennant,
    pub note: String,
    #[serde(flatten)]
    pub rest: BTreeMap<Quarter, Notch>,
}

/// A map of dates, which Swift reads from the text serde writes, flattened after a struct.
#[cfg(feature = "chrono")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Serving {
    #[serde(flatten)]
    pub flag: Pennant,
    pub note: String,
    #[serde(flatten)]
    pub rest: BTreeMap<String, DateTime<Utc>>,
}

/// A map that names a function, flattened after a struct: serde hands the function the keys
/// neither took.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Snaking {
    #[serde(flatten)]
    pub flag: Pennant,
    pub note: String,
    #[model_schema_prop(preprocess = ["asGiven"])]
    #[serde(flatten)]
    pub rest: BTreeMap<String, String>,
}

/// The payload as it was handed.
const fn as_given(payload: Value) -> Value {
    payload
}

/// The text of `value` as serde writes it.
fn written<T>(value: &T) -> String
where
    T: Serialize,
{
    serde_json::to_string(value).unwrap()
}

/// `text` read as `T`.
fn read<T>(text: &str) -> T
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_str(text).unwrap()
}

/// A map after a struct holds the keys neither the struct nor its holder took, and a map ahead of
/// a struct, and after a member that takes no key, every key but its holder's own.
#[test]
fn serde_hands_a_flattened_map_the_keys_no_member_ahead_of_it_took() {
    assert_eq!(written(&read::<Stopping>(STOPPING).rest), r#"{"north":2}"#);
    assert_eq!(written(&read::<Frapping>(FRAPPING).rest), r#"{"north":2}"#);
    assert_eq!(
        written(&read::<Kedging>(TEXTS).extra),
        r#"{"label":"n","north":"s"}"#
    );
    let warping = read::<Warping>(WARPING);
    assert_eq!(warping.first, warping.second);
    assert_eq!(
        written(&read::<Marling>(TEXTS).rest),
        r#"{"label":"n","north":"s"}"#
    );
    assert_eq!(
        written(&read::<Mousing>(TEXTS).rest),
        r#"{"label":"n","north":"s"}"#
    );
    assert_eq!(
        written(&read::<Bowsing>(BOWSING).rest),
        r#"{"label":"n","marker":"m","north":"s"}"#
    );
    let Clewing::Held { flag, note, rest } = read::<Clewing>(CLEWING);
    assert_eq!((flag.label.as_str(), note.as_str()), ("n", "a"));
    assert_eq!(written(&rest), r#"{"north":2}"#);
    assert_eq!(
        written(&read::<Parceling>(PARCELING).rest),
        r#"{"North":2}"#
    );
    assert_eq!(
        read::<Toggling<u8>>(PARCELING),
        Toggling {
            flag: Pennant {
                label: "n".to_owned(),
            },
            note: "a".to_owned(),
            rest: BTreeMap::from([(Quarter::North, 2)]),
        }
    );
    assert_eq!(
        written(&read::<Worming>(WORMING).rest),
        r#"{"north":{"mark":"x"}}"#
    );
    assert_eq!(
        written(&read::<Faking>(FAKING).rest),
        r#"{"North":{"mark":"x"}}"#
    );
    assert_eq!(written(&read::<Snaking>(TEXTS).rest), r#"{"north":"s"}"#);
}

/// A map of dates after a struct holds the keys neither the struct nor its holder took.
#[cfg(feature = "chrono")]
#[test]
fn serde_hands_a_flattened_map_of_dates_the_keys_no_member_ahead_of_it_took() {
    assert_eq!(
        written(&read::<Serving>(SERVING).rest),
        r#"{"north":"2024-01-02T03:04:05.678Z"}"#
    );
}
