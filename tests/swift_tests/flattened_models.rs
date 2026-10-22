//! The models of the flattened Swift reads `swift_tests/flattened.rs` pins and
//! `service_schema_emitted_client_tests/run_swift_flattened.rs` runs: a map that is the one member
//! of its struct and of a variant written under its name, under a key Swift wraps, in a type that
//! declares no parameter and in one that declares one, of models, of dates and one that names a
//! function; and a member that names a function, beside a member of its holder's own, ahead of a
//! struct and after each member that takes none of the keys it reads; and, after a struct, a
//! struct with a flattened member of its own, a member that names a function, a model with a rule
//! and a struct that reads the same key; and a struct flattened ahead of every struct, plain and
//! naming a function, beside a member of its holder's own that reads its key, and in a holder of no
//! member of its own whose holder reads that key; and a struct whose every member is a flattened
//! struct. serde reads the members in the order declared: a struct takes the keys it reads, and
//! every flattened member is handed every key neither its holder's own members nor a member ahead
//! of it took. Every holder of a member that names a function has a member of its own, whose key
//! no flattened member is handed, but [`Stropping`] and [`Dredging`]. [`Boxing`] and [`Baling`]
//! each hold a member named after the container their written `encode(to:)` declares, of their own
//! and flattened.
//! [`Grafting`] and [`Pointing`] are enums under a tag alone whose variants hold a struct as their
//! one slot and flatten one beside no member of their own, and in [`Pointing`] beside one: serde
//! takes the tag off the object first, so a struct that reads a key named as the tag is refused.
//! [`Trussing`], [`Halyard`], [`Tailing`], [`Slacking`], [`Reeling`] and [`Winching`] each hold an
//! optional map as the one member of their struct or variant, which serde reads as `None` wherever
//! an entry does not read. [`Anchoring`], [`Dredging`] and [`Fending`] flatten [`Careen`], an
//! externally tagged enum serde reads from the first entry under a variant's name or an alias.

use alloc::collections::BTreeMap;

#[cfg(feature = "chrono")]
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::model_schema;

/// What [`Twinning`] and [`Plying`] are read from.
pub const STOPPING: &str = r#"{"label":"n","north":2,"note":"a"}"#;

/// What [`Stopping`] is read from.
pub const COUNTS: &str = r#"{"north":2,"south":3}"#;

/// What [`Frapping`] is read from.
pub const FRAPPING: &str = r#"{"north":2,"note":"a"}"#;

/// What [`Kedging`], [`Marling`] and [`Mousing`] are read from.
pub const TEXTS: &str = r#"{"label":"n","north":"s","note":"a"}"#;

/// What [`Bowsing`] and [`Splaying`] are read from.
pub const BOWSING: &str = r#"{"label":"n","marker":"m","north":"s","note":"a"}"#;

/// What [`Clewing`] is read from.
pub const CLEWING: &str = r#"{"Held":{"north":2}}"#;

/// What [`Parceling`] and [`Toggling`] are read from.
pub const PARCELING: &str = r#"{"North":2}"#;

/// What [`Worming`] is read from.
pub const WORMING: &str = r#"{"north":{"mark":"x"}}"#;

/// What [`Faking`] is read from.
pub const FAKING: &str = r#"{"North":{"mark":"x"}}"#;

/// What [`Serving`] is read from.
#[cfg(feature = "chrono")]
pub const SERVING: &str = r#"{"north":"2024-01-02T03:04:05.678Z"}"#;

/// What [`Snaking`] is read from.
pub const SNAKING: &str = r#"{"north":"s"}"#;

/// What [`Gripping`] is read from.
pub const GRIPPING: &str = r#"{"label":"n","mark":"x","note":"a"}"#;

/// What [`Hanking`] is read from.
pub const HANKING: &str = r#"{"label":"n","marker":"m"}"#;

/// What [`Whipping`], [`Marrying`] and [`Ratlining`] are read from: the one key their own member
/// reads.
pub const LABEL: &str = r#"{"label":"n"}"#;

/// What [`Seaming`] is read from.
pub const SEAMING: &str = r#"{"label":"n","mark":"x"}"#;

/// What [`Boxing`] is read from.
pub const BOXING: &str = r#"{"container":"c","label":"n"}"#;

/// What [`Baling`] is read from.
pub const BALING: &str = r#"{"label":"n","note":"a"}"#;

/// What [`Grafting`] is read from under the variant that flattens [`Kinded`].
pub const GRAFTING_HELD: &str = r#"{"kind":"Held"}"#;

/// What [`Grafting`] is read from under the variant that holds [`Kinded`] as its one slot.
pub const GRAFTING_SLOT: &str = r#"{"kind":"Slot"}"#;

/// What [`Pointing`] is read from under the variant that flattens [`Pennant`].
pub const POINTING_HELD: &str = r#"{"kind":"Held","label":"n"}"#;

/// What [`Pointing`] is read from under the variant that holds [`Pennant`] as its one slot.
pub const POINTING_SLOT: &str = r#"{"kind":"Slot","label":"n"}"#;

/// What [`Pointing`] is read from under the variant that flattens [`Pennant`] beside a member of
/// its own.
pub const POINTING_NOTED: &str = r#"{"kind":"Noted","label":"n","note":"a"}"#;

/// What [`Pointing`] is read from under the variant that flattens [`Notch`].
pub const POINTING_MARKED: &str = r#"{"kind":"Marked","mark":"x"}"#;

/// What [`Pointing`] is read from under the variant that holds [`Notch`] as its one slot.
pub const POINTING_NOTCHED: &str = r#"{"kind":"Notched","mark":"x"}"#;

/// What [`Trussing`], [`Slacking`], [`Reeling`] and [`Winching`] are read from with no entry.
pub const NO_ENTRY: &str = "{}";

/// What [`Trussing`] and [`Slacking`] are read from: one entry.
pub const TRUSSING: &str = r#"{"north":2}"#;

/// What [`Trussing`] and [`Slacking`] are read from with an entry that is no number.
pub const TRUSSING_MISTYPED: &str = r#"{"north":"x"}"#;

/// What [`Trussing`] is read from with one entry that reads and one that does not.
pub const TRUSSING_PARTIAL: &str = r#"{"north":2,"south":"x"}"#;

/// What [`Halyard`] is read from with no entry.
pub const HALYARD_EMPTY: &str = r#"{"Open":{}}"#;

/// What [`Halyard`] is read from: one entry.
pub const HALYARD: &str = r#"{"Open":{"north":2}}"#;

/// What [`Halyard`] is read from with an entry that is no number.
pub const HALYARD_MISTYPED: &str = r#"{"Open":{"north":"x"}}"#;

/// What [`Tailing`] is read from: one entry.
pub const TAILING: &str = r#"{"kind":"Open","held":{"north":2}}"#;

/// What [`Tailing`] is read from with an entry that is no number.
pub const TAILING_MISTYPED: &str = r#"{"kind":"Open","held":{"north":"x"}}"#;

/// What [`Reeling`] is read from with a key that is no [`Quarter`].
pub const REELING_UNKEYED: &str = r#"{"West":2}"#;

/// What [`Reeling`] is read from with an entry that is no number.
pub const REELING_MISTYPED: &str = r#"{"North":"x"}"#;

/// What [`Winching`] is read from with a model whose rule is broken.
pub const WINCHING_BROKEN: &str = r#"{"north":{"mark":""}}"#;

/// What [`Winching`] is read from with a model that does not read.
pub const WINCHING_MISTYPED: &str = r#"{"north":{"mark":5}}"#;

/// What [`Anchoring`] is read from: a variant's entry beside a key no member reads, one whose
/// model breaks its rule, a unit variant's entry, an entry under an alias, and entries under two
/// variants' names.
pub const ANCHORING: [&str; 5] = [
    r#"{"note":"a","Bay":{"label":"n"},"north":2}"#,
    r#"{"Bay":{"label":""},"note":"a"}"#,
    r#"{"Calm":null,"note":"a"}"#,
    r#"{"Quay":{"label":"s"},"note":"a"}"#,
    r#"{"Calm":null,"Dock":{"label":"s"},"note":"a"}"#,
];

/// What [`Dredging`] is read from.
pub const DREDGING: &str = r#"{"Dock":{"label":"s"},"north":2}"#;

/// What [`Fending`] is read from: no variant's entry, one, one whose model breaks its rule, and
/// entries under a variant's name and an alias of another.
pub const FENDING: [&str; 4] = [
    r#"{"note":"a"}"#,
    r#"{"Bay":{"label":"n"},"note":"a"}"#,
    r#"{"Bay":{"label":""},"note":"a"}"#,
    r#"{"Bay":{"label":"n"},"Quay":{"label":"s"},"note":"a"}"#,
];

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

/// The keys a function was handed, which the members below that name one read.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Counting {
    pub keys: Vec<String>,
}

/// A map, the one member of its struct, which serde hands every key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Stopping {
    #[serde(flatten)]
    pub rest: BTreeMap<String, u8>,
}

/// A map, the one member of a variant written under its name, which serde hands every key of the
/// variant's object.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Clewing {
    Held {
        #[serde(flatten)]
        rest: BTreeMap<String, u8>,
    },
}

/// A map under a key Swift wraps.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Parceling {
    #[serde(flatten)]
    pub rest: BTreeMap<Quarter, u8>,
}

/// [`Parceling`] in a type that declares a parameter, which the map holds.
#[model_schema(default_types(Held = u8))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Toggling<Held> {
    #[serde(flatten)]
    pub rest: BTreeMap<Quarter, Held>,
}

/// A map of models with a rule.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Worming {
    #[serde(flatten)]
    pub rest: BTreeMap<String, Notch>,
}

/// A map of models with a rule under a key Swift wraps.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Faking {
    #[serde(flatten)]
    pub rest: BTreeMap<Quarter, Notch>,
}

/// A map of dates, which Swift reads from the text serde writes.
#[cfg(feature = "chrono")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Serving {
    #[serde(flatten)]
    pub rest: BTreeMap<String, DateTime<Utc>>,
}

/// A map that names a function, which serde hands every key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Snaking {
    #[model_schema_prop(preprocess = ["asGiven"])]
    #[serde(flatten)]
    pub rest: BTreeMap<String, String>,
}

/// A member that names a function, the one flattened member, beside a member of its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Frapping {
    pub note: String,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub tally: Counting,
}

/// A member that names a function flattened ahead of a struct, where serde hands the function
/// every key but its holder's own, the struct's among them.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Kedging {
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub extra: Counting,
    pub note: String,
    #[serde(flatten)]
    pub pennant: Pennant,
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

/// A member that names a function flattened after a struct that names one on itself, where serde
/// hands the function every key but its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Marling {
    #[serde(flatten)]
    pub issued: Reeving,
    pub note: String,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub tally: Counting,
}

/// A member that names a function flattened after another that names one, where serde hands the
/// second every key but its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Mousing {
    #[model_schema_prop(preprocess = ["asGiven"])]
    #[serde(flatten)]
    pub flag: Pennant,
    pub note: String,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub tally: Counting,
}

/// A member that names a function flattened after a struct with a flattened member of its own,
/// where serde hands the function every key but its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Bowsing {
    #[serde(flatten)]
    pub hanking: Hanking,
    pub note: String,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub tally: Counting,
}

/// A struct with a flattened member of its own that names a function, which serde reads as a map
/// and so takes no key: the function is handed the keys neither the struct nor its holders took.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Footing {
    pub marker: String,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub tally: Counting,
}

/// A struct flattened after a struct, where serde hands it the keys neither the first nor its
/// holder took.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Splaying {
    #[serde(flatten)]
    pub flag: Pennant,
    #[serde(flatten)]
    pub footing: Footing,
    pub note: String,
}

/// A member that names a function flattened after a struct, where serde hands the function the
/// keys neither the struct nor its holder took.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Twinning {
    #[serde(flatten)]
    pub flag: Pennant,
    pub note: String,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub tally: Counting,
}

/// A model with a rule flattened after a struct, where serde hands it the keys neither the struct
/// nor its holder took.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Gripping {
    #[serde(flatten)]
    pub flag: Pennant,
    #[serde(flatten)]
    pub notch: Notch,
    pub note: String,
}

/// A struct flattened after a struct that reads the same key, where serde hands the second the
/// object without it and refuses the payload.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Plying {
    #[serde(flatten)]
    pub flag: Pennant,
    pub note: String,
    #[serde(flatten)]
    pub twice: Pennant,
}

/// A struct flattened ahead of a member of its holder's own that reads the struct's key, which
/// serde hands the struct the object without.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Whipping {
    #[serde(flatten)]
    pub flag: Pennant,
    pub label: String,
}

/// [`Whipping`] with a member that names a function, which serde hands the object without its
/// holder's own key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Marrying {
    #[model_schema_prop(preprocess = ["asGiven"])]
    #[serde(flatten)]
    pub flag: Pennant,
    pub label: String,
}

/// A struct flattened first in a holder of no member of its own, ahead of a member that names a
/// function, which takes the keys the types around the holder took.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Stropping {
    #[serde(flatten)]
    pub flag: Pennant,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub tally: Counting,
}

/// [`Stropping`] flattened beside a member that reads the key of its first struct, which serde
/// hands that struct the object without.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Ratlining {
    pub label: String,
    #[serde(flatten)]
    pub stropping: Stropping,
}

/// A struct whose every member is a flattened struct, the second a model with a rule, which serde
/// hands every key the first did not take.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Seaming {
    #[serde(flatten)]
    pub flag: Pennant,
    #[serde(flatten)]
    pub notch: Notch,
}

/// A member of its own named after the container its written `encode(to:)` declares, beside a
/// flattened struct.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Boxing {
    pub container: String,
    #[serde(flatten)]
    pub flag: Pennant,
}

/// A flattened struct named after the container its holder's written `encode(to:)` declares,
/// beside a member of its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Baling {
    #[serde(flatten)]
    pub container: Pennant,
    pub note: String,
}

/// A struct that reads a key named as the tag of [`Grafting`].
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Kinded {
    pub kind: String,
}

/// [`Kinded`] flattened into a variant of no member of its own and held as a variant's one slot,
/// under a tag alone: serde hands neither the tag.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Grafting {
    Held {
        #[serde(flatten)]
        flag: Kinded,
    },
    Slot(Kinded),
}

/// [`Pennant`] and [`Notch`], which read keys other than the tag, flattened into a variant of no
/// member of its own and held as a variant's one slot, under a tag alone, and [`Pennant`]
/// flattened beside a member of its variant's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Pointing {
    Held {
        #[serde(flatten)]
        flag: Pennant,
    },
    Marked {
        #[serde(flatten)]
        notch: Notch,
    },
    Notched(Notch),
    Noted {
        #[serde(flatten)]
        flag: Pennant,
        note: String,
    },
    Slot(Pennant),
}

/// An optional map, the one member of its struct.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Trussing {
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub counts: Option<BTreeMap<String, u8>>,
}

/// An optional map, the one member of a variant written under its name.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Halyard {
    Open {
        #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
        counts: Option<BTreeMap<String, u8>>,
    },
}

/// An optional map, the one member of a variant under a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum Tailing {
    Open {
        #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
        counts: Option<BTreeMap<String, u8>>,
    },
}

/// An optional map, the one member of an untagged variant.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Slacking {
    Open {
        #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
        counts: Option<BTreeMap<String, u8>>,
    },
}

/// An optional map under a key Swift wraps.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Reeling {
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub counts: Option<BTreeMap<Quarter, u8>>,
}

/// An optional map of models with a rule.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Winching {
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub notches: Option<BTreeMap<String, Notch>>,
}

/// An externally tagged enum with a rule, a unit variant and an alias, which serde reads flattened
/// from the first entry under a variant's name or an alias of one.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Careen {
    Bay {
        #[model_schema_prop(minLength = 1)]
        label: String,
    },
    Calm,
    #[serde(alias = "Quay")]
    Dock {
        label: String,
    },
}

/// [`Careen`] flattened beside a member of its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Anchoring {
    #[serde(flatten)]
    pub careen: Careen,
    pub note: String,
}

/// [`Careen`] flattened ahead of a member that names a function, which serde hands every entry
/// but the enum's.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Dredging {
    #[serde(flatten)]
    pub careen: Careen,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub tally: Counting,
}

/// [`Careen`] optional beside a member of its holder's own, which serde reads no value of where no
/// entry is under the name of a variant.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Fending {
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub careen: Option<Careen>,
    pub note: String,
}

/// The payload as it was handed.
const fn as_given(payload: Value) -> Value {
    payload
}

/// The keys of the object `payload`, under `keys`.
fn with_its_keys(payload: Value) -> Value {
    let Value::Object(object) = payload else {
        return payload;
    };
    let keys: Vec<Value> = object
        .into_iter()
        .map(|(key, _)| Value::String(key))
        .collect();
    serde_json::json!({ "keys": keys })
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

/// A map that is the one member of its struct or of a variant written under its name holds every
/// key, in each shape a map is read in.
#[test]
fn serde_hands_a_map_alone_every_key() {
    assert_eq!(written(&read::<Stopping>(COUNTS).rest), COUNTS);
    let Clewing::Held { rest } = read::<Clewing>(CLEWING);
    assert_eq!(written(&rest), r#"{"north":2}"#);
    assert_eq!(
        written(&read::<Parceling>(PARCELING).rest),
        r#"{"North":2}"#
    );
    assert_eq!(
        read::<Toggling<u8>>(PARCELING),
        Toggling {
            rest: BTreeMap::from([(Quarter::North, 2)]),
        }
    );
    assert_eq!(written(&read::<Worming>(WORMING).rest), WORMING);
    assert_eq!(written(&read::<Faking>(FAKING).rest), FAKING);
    assert_eq!(written(&read::<Snaking>(SNAKING).rest), SNAKING);
}

/// A map of dates alone holds every key.
#[cfg(feature = "chrono")]
#[test]
fn serde_hands_a_map_of_dates_alone_every_key() {
    assert_eq!(written(&read::<Serving>(SERVING).rest), SERVING);
}

/// A function beside a member of its holder's own, ahead of a struct, and after a member that
/// takes no key is handed every key but its holder's own.
#[test]
fn serde_hands_a_function_the_keys_no_member_ahead_of_it_took() {
    assert_eq!(read::<Frapping>(FRAPPING).tally.keys, ["north"]);
    let kedging = read::<Kedging>(TEXTS);
    assert_eq!(kedging.extra.keys, ["label", "north"]);
    assert_eq!(kedging.pennant.label, "n");
    assert_eq!(read::<Marling>(TEXTS).tally.keys, ["label", "north"]);
    assert_eq!(read::<Mousing>(TEXTS).tally.keys, ["label", "north"]);
    assert_eq!(
        read::<Bowsing>(BOWSING).tally.keys,
        ["label", "marker", "north"]
    );
}

/// A struct with a flattened member of its own, a member that names a function and a model, each
/// after a struct, are handed the keys the struct did not take, and a struct after a struct that
/// reads the same key is refused.
#[test]
fn serde_hands_a_member_flattened_after_a_struct_the_keys_the_struct_did_not_take() {
    let splaying = read::<Splaying>(BOWSING);
    assert_eq!(splaying.footing.tally.keys, ["north"]);
    assert_eq!(
        written(&splaying),
        r#"{"label":"n","marker":"m","keys":["north"],"note":"a"}"#
    );
    let twinning = read::<Twinning>(STOPPING);
    assert_eq!(twinning.tally.keys, ["north"]);
    assert_eq!(
        written(&twinning),
        r#"{"label":"n","note":"a","keys":["north"]}"#
    );
    assert_eq!(
        written(&read::<Gripping>(GRIPPING)),
        r#"{"label":"n","mark":"x","note":"a"}"#
    );
    let refused = serde_json::from_str::<Plying>(STOPPING).unwrap_err();
    assert_eq!(
        refused.to_string(),
        "missing field `label` at line 1 column 34"
    );
}

/// A struct flattened first, plain or naming a function, is handed the object without the keys of
/// its holder's own members and those the types around its holder took, so one that reads such a
/// key is refused. One whose keys collide with none is read.
#[test]
fn serde_hands_a_struct_flattened_first_the_keys_its_holder_and_the_types_around_did_not_take() {
    let refusal = |refused: serde_json::Error| refused.to_string();
    assert_eq!(
        refusal(serde_json::from_str::<Whipping>(LABEL).unwrap_err()),
        "missing field `label` at line 1 column 13"
    );
    assert_eq!(
        refusal(serde_json::from_str::<Marrying>(LABEL).unwrap_err()),
        "missing field `label` at line 1 column 13"
    );
    assert_eq!(
        refusal(serde_json::from_str::<Ratlining>(LABEL).unwrap_err()),
        "missing field `label` at line 1 column 13"
    );
    assert_eq!(
        written(&read::<Hanking>(HANKING)),
        r#"{"marker":"m","label":"n"}"#
    );
}

/// A struct whose every member is a flattened struct hands the second the keys the first did not
/// take, and writes each at its place.
#[test]
fn serde_hands_the_second_of_two_flattened_structs_the_keys_the_first_did_not_take() {
    let seaming = read::<Seaming>(SEAMING);
    assert_eq!(seaming.notch.mark, "x");
    assert_eq!(written(&seaming), SEAMING);
}

/// A member named after a container, of its holder's own or flattened, is read and written back
/// as any other.
#[test]
fn serde_reads_and_writes_a_member_named_container_as_any_other() {
    let boxing = read::<Boxing>(BOXING);
    assert_eq!(boxing.container, "c");
    assert_eq!(written(&boxing), BOXING);
    let baling = read::<Baling>(BALING);
    assert_eq!(baling.container.label, "n");
    assert_eq!(written(&baling), BALING);
}

/// A struct a variant under a tag alone holds as its one slot or flattens beside no member of its
/// own is handed the object without the tag, so one that reads a key named as the tag is refused,
/// and one that reads another key is read.
#[test]
fn serde_hands_what_a_variant_under_a_tag_alone_holds_the_object_without_the_tag() {
    let refusal = |text: &str| {
        serde_json::from_str::<Grafting>(text)
            .unwrap_err()
            .to_string()
    };
    assert_eq!(refusal(GRAFTING_HELD), "missing field `kind`");
    assert_eq!(refusal(GRAFTING_SLOT), "missing field `kind`");
    let label = || Pennant {
        label: "n".to_owned(),
    };
    let mark = || Notch {
        mark: "x".to_owned(),
    };
    for (text, read_as) in [
        (POINTING_HELD, Pointing::Held { flag: label() }),
        (POINTING_SLOT, Pointing::Slot(label())),
        (
            POINTING_NOTED,
            Pointing::Noted {
                flag: label(),
                note: "a".to_owned(),
            },
        ),
        (POINTING_MARKED, Pointing::Marked { notch: mark() }),
        (POINTING_NOTCHED, Pointing::Notched(mark())),
    ] {
        assert_eq!(read::<Pointing>(text), read_as);
        assert_eq!(written(&read_as), text);
    }
}

/// A flattened externally tagged enum is read from the first entry under a variant's name or an
/// alias, its model judged with its holder, and a member flattened after it is handed every entry
/// but that one. An optional enum reads `None` from a payload with no such entry. Of two entries
/// serde reads the first, where Swift refuses the payload as TypeScript and Zod do.
#[test]
fn serde_reads_a_flattened_enum_from_the_first_entry_under_a_variants_name() {
    let bay = || Careen::Bay {
        label: "n".to_owned(),
    };
    let dock = || Careen::Dock {
        label: "s".to_owned(),
    };
    let anchored = |text: &str| serde_json::from_str::<Anchoring>(text).map(|read| read.careen);
    assert_eq!(anchored(ANCHORING[0]).unwrap(), bay());
    assert_eq!(
        anchored(ANCHORING[1]).unwrap_err().to_string(),
        "'Bay.label': too short: minimum length is 1, got 0"
    );
    assert_eq!(anchored(ANCHORING[2]).unwrap(), Careen::Calm);
    assert_eq!(anchored(ANCHORING[3]).unwrap(), dock());
    assert_eq!(anchored(ANCHORING[4]).unwrap(), Careen::Calm);
    let dredging = read::<Dredging>(DREDGING);
    assert_eq!(dredging.careen, dock());
    assert_eq!(dredging.tally.keys, ["north"]);
    let fended = |text: &str| serde_json::from_str::<Fending>(text).map(|read| read.careen);
    assert_eq!(fended(FENDING[0]).unwrap(), None);
    assert_eq!(fended(FENDING[1]).unwrap(), Some(bay()));
    assert_eq!(
        fended(FENDING[2]).unwrap_err().to_string(),
        "'Bay.label': too short: minimum length is 1, got 0"
    );
    assert_eq!(fended(FENDING[3]).unwrap(), Some(bay()));
}

/// An optional map alone in its struct or in a variant, under its name, under a content key or
/// untagged, holds every entry where each reads, and is `None` wherever one does not: a value that
/// is no number, a key that is no [`Quarter`] and a model that does not read. A model whose rule is
/// broken is read with no judgement and refused with its holder, under the place serde names.
/// serde writes nothing for `None`.
#[test]
fn serde_reads_an_optional_map_as_none_wherever_an_entry_does_not_read() {
    let trussed = |read: Trussing| read.counts;
    assert_eq!(trussed(read(NO_ENTRY)), Some(BTreeMap::new()));
    assert_eq!(
        trussed(read(TRUSSING)),
        Some(BTreeMap::from([("north".to_owned(), 2)]))
    );
    for json in [TRUSSING_MISTYPED, TRUSSING_PARTIAL] {
        assert_eq!(trussed(read(json)), None, "{json}");
    }
    assert_eq!(written(&Trussing { counts: None }), NO_ENTRY);
    let open = |read: Halyard| {
        let Halyard::Open { counts } = read;
        counts
    };
    assert_eq!(open(read(HALYARD_EMPTY)), Some(BTreeMap::new()));
    assert_eq!(written(&read::<Halyard>(HALYARD)), HALYARD);
    assert_eq!(open(read(HALYARD_MISTYPED)), None);
    assert_eq!(written(&Halyard::Open { counts: None }), HALYARD_EMPTY);
    assert_eq!(written(&read::<Tailing>(TAILING)), TAILING);
    assert_eq!(
        written(&read::<Tailing>(TAILING_MISTYPED)),
        r#"{"kind":"Open","held":{}}"#
    );
    let slack = |read: Slacking| {
        let Slacking::Open { counts } = read;
        counts
    };
    assert_eq!(slack(read(NO_ENTRY)), Some(BTreeMap::new()));
    assert_eq!(written(&read::<Slacking>(TRUSSING)), TRUSSING);
    assert_eq!(slack(read(TRUSSING_MISTYPED)), None);
    assert_eq!(written(&read::<Reeling>(PARCELING)), PARCELING);
    for json in [REELING_UNKEYED, REELING_MISTYPED] {
        assert_eq!(read::<Reeling>(json).counts, None, "{json}");
    }
    assert_eq!(written(&read::<Winching>(WORMING)), WORMING);
    assert_eq!(read::<Winching>(WINCHING_MISTYPED).notches, None);
    assert_eq!(
        serde_json::from_str::<Winching>(WINCHING_BROKEN)
            .unwrap_err()
            .to_string(),
        "'north.mark': too short: minimum length is 1, got 0"
    );
}
