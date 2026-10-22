//! The models of the flattened Swift reads `swift_tests/flattened.rs` pins and
//! `service_schema_emitted_client_tests/run_swift_flattened.rs` runs: a map that is the one member
//! of its struct and of a variant under a tag key, under a key Swift wraps, in a type that
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
//! no flattened member is handed, but [`Stropping`]. [`Boxing`] and [`Baling`] each hold a member
//! named after the container their written `encode(to:)` declares, of their own and flattened.

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
pub const CLEWING: &str = r#"{"kind":"Held","north":2}"#;

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

/// A map, the one member of a variant under a tag key, which serde hands every key but the tag.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
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

/// A map that is the one member of its struct holds every key, and one in a variant under a tag
/// key every key but the tag, in each shape a map is read in.
#[test]
fn serde_hands_a_map_alone_every_key_but_a_tag_read_ahead_of_it() {
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
