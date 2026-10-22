//! The models of the flattened Kotlin reads `kotlin_tests/tests.rs` pins and
//! `service_schema_emitted_client_tests/run_kotlin_flattened.rs` runs: a map flattened as the one
//! member of its struct, a flattened struct beside a member of its own, and an optional member
//! beside a flattened one, with its twin under `nullable`. Beside them, a flattened member after
//! a struct and beside a member of its holder's own. serde reads the members in the order
//! declared: a struct takes the keys it reads, one with a member serde never writes among them,
//! and not its own tag or the key of a member serde never reads, a member that names a function
//! takes none, and every flattened member is handed the entries neither its holder's own members
//! nor a member flattened ahead of it took. An externally tagged enum takes the first entry under
//! the name of a variant it reads, beside a member of its holder's own, ahead of a member that
//! names a function and ahead of a struct, and one declared below its holder is read from the
//! object. serde writes such an enum's unit variant as an entry, its name over `null`, the enum
//! optional and under a function too. An optional struct with a flattened member of its own, the
//! one member, serde reads wherever the struct reads and as `None` wherever it does not. An
//! optional map, the one member, serde reads as `None` wherever an entry does not read as a count,
//! and under a function from what the function answers. A map keyed by a plain enum serde writes
//! and reads under the name each variant is written under, and refuses, or reads as `None` where
//! it is optional, a key no variant is written under. A variant of an enum under each tagging
//! reads and writes a flattened member at its own level, as a struct does. A struct under its own
//! tag serde writes with the tag ahead of its members, alone and flattened ahead of a member of its
//! holder's own, and reads with no tag or with any value under it, alone, with a flattened member
//! of its own and flattened, where TypeScript and Zod refuse every tag but the struct's name.

use alloc::collections::BTreeMap;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::model_schema;

use super::tuple_variants_models::{Labelling, labelling_schema};

/// What every holder of a member flattened after or beside another below is read from, but those
/// named apart.
pub const PAYLOAD: &str = r#"{"label":"n","north":2}"#;

/// What [`Hitching`] is read from.
pub const HITCHING: &str = r#"{"label":"n","north":2,"note":"a"}"#;

/// What [`Stropping`] is read from.
pub const STROPPING: &str = r#"{"label":"n","note":"a"}"#;

/// What [`Stowing`] is read from: a variant's entry ahead of the holder's own key and after it,
/// beside a key no member reads and beside a second variant's entry, a unit variant's and a
/// newtype variant's entry, and no variant's entry, which serde refuses.
pub const STOWING: [&str; 7] = [
    r#"{"Bay":{"label":"n"},"note":"a"}"#,
    r#"{"note":"a","Bay":{"label":"n"}}"#,
    r#"{"note":"a","Bay":{"label":"n"},"north":2}"#,
    r#"{"Bay":{"label":"n"},"Dock":{"label":"s"},"note":"a"}"#,
    r#"{"Calm":null,"note":"a"}"#,
    r#"{"Held":"x","note":"a"}"#,
    r#"{"note":"a"}"#,
];

/// What [`Belting`] is read from: a variant's entry, and no variant's entry, of a key no variant is
/// read under and of none, where serde reads no value.
pub const BELTING: [&str; 3] = [
    r#"{"Bay":{"label":"n"},"note":"a"}"#,
    r#"{"Other":1,"note":"a"}"#,
    r#"{"note":"a"}"#,
];

/// What [`Heaving`] is read from: one variant's entry, two variants' entries in each order, and an
/// alias after a variant's entry.
pub const HEAVING: [&str; 4] = [
    r#"{"Bay":{"label":"n"},"north":2}"#,
    r#"{"Bay":{"label":"n"},"Dock":{"label":"s"},"north":2}"#,
    r#"{"Dock":{"label":"s"},"Bay":{"label":"n"},"north":2}"#,
    r#"{"Bay":{"label":"n"},"Quay":{"label":"s"},"north":2}"#,
];

/// What [`Cleating`] is read from: a variant under its `rename`, one under the enum's `rename_all`,
/// and one under its Rust name, which serde reads no variant under.
pub const CLEATING: [&str; 3] = [
    r#"{"pier":{"label":"n"},"north":2}"#,
    r#"{"open_bay":{"label":"n"},"north":2}"#,
    r#"{"OpenBay":{"label":"n"},"north":2}"#,
];

/// What [`Rousing`] and [`Gasketing`] are read from: the key of the member [`Unwritten`] reads and
/// never writes among them.
pub const UNWRITTEN: &str = r#"{"label":"n","hidden":1,"north":2}"#;

/// What [`Dipping`] is read from: the tag of [`Ensign`] among the keys.
pub const ENSIGN: &str = r#"{"kind":"Ensign","label":"n","north":2}"#;

/// What [`Striking`] is read from: the tag of [`Ensign`] alone, which serde reads no value from.
pub const STRUCK: &str = r#"{"kind":"Ensign","note":"a"}"#;

/// What [`Flag`] is read from: its tag after its member, which serde writes back ahead of it.
pub const FLAG: &str = r#"{"label":"n","kind":"Flag"}"#;

/// What [`Gonfalon`] is read from: its tag after its member.
pub const GONFALON: &str = r#"{"label":"n","kind":"Gonfalon"}"#;

/// What [`Semaphore`] is read from: its tag after its member under its alias.
pub const SEMAPHORE: &str = r#"{"name":"n","kind":"Semaphore"}"#;

/// What [`Flag`] is read from: its own tag, no tag, a tag of another value and one of another
/// kind. serde reads each and writes the struct's name back; TypeScript and Zod, whose tag is the
/// literal `"Flag"`, refuse all but the first.
pub const FLAG_TAGS: [&str; 4] = [
    r#"{"kind":"Flag","label":"n"}"#,
    r#"{"label":"n"}"#,
    r#"{"kind":"Other","label":"n"}"#,
    r#"{"kind":5,"label":"n"}"#,
];

/// [`FLAG_TAGS`] for [`Pennant`], beside its own member.
pub const PENNANT_TAGS: [&str; 4] = [
    r#"{"kind":"Pennant","label":"n","note":"a"}"#,
    r#"{"label":"n","note":"a"}"#,
    r#"{"kind":"Other","label":"n","note":"a"}"#,
    r#"{"kind":5,"label":"n","note":"a"}"#,
];

/// [`FLAG_TAGS`] for [`Ensign`] flattened in [`Dressing`], beside the holder's own member.
pub const DRESSING_TAGS: [&str; 4] = [
    r#"{"kind":"Ensign","label":"n","north":2}"#,
    r#"{"label":"n","north":2}"#,
    r#"{"kind":"Other","label":"n","north":2}"#,
    r#"{"kind":5,"label":"n","north":2}"#,
];

/// What [`Dressing`] is read from: the tag of [`Ensign`] after every other key.
pub const DRESSING: &str = r#"{"north":2,"label":"n","kind":"Ensign"}"#;

/// What [`Gammoning`] is read from: the key of the member [`Marline`] never reads among them.
pub const MARLINE: &str = r#"{"label":"n","north":2,"spare":1}"#;

/// What [`Scudding`] is read from.
pub const SCUDDING: &str = r#"{"Bay":{"label":"n"},"label":"m","north":2}"#;

/// A variant's entry under its `alias`, beside a member of its holder's own and ahead of a member
/// that names a function, which serde reads as the variant.
pub const UNDER_AN_ALIAS: [&str; 2] = [
    r#"{"Quay":{"label":"s"},"note":"a"}"#,
    r#"{"Quay":{"label":"s"},"north":2}"#,
];

/// What [`ProbeLocker`] and its twins are read from: no entry, an entry that reads as a count, an
/// entry that does not, the two side by side, and `null`, which a count does not read.
pub const LOCKER: [&str; 5] = [
    "{}",
    r#"{"north":2}"#,
    r#"{"north":"x"}"#,
    r#"{"north":2,"south":"x"}"#,
    r#"{"north":null}"#,
];

/// What [`Cathead`] and [`Hawsepipe`] are read from: no entry, a key a variant of [`Chock`] is
/// written under, both, a key none is written under, and a count of text.
pub const CHOCKED: [&str; 5] = [
    "{}",
    r#"{"Port":2}"#,
    r#"{"Port":2,"Starboard":3}"#,
    r#"{"Bow":2}"#,
    r#"{"Port":"x"}"#,
];

/// What [`Snubbing`] is read from: a key under its variant's `rename`, one under the enum's
/// `rename_all`, and one under a variant's Rust name, which serde reads no variant under.
pub const SNUBBED: [&str; 3] = [r#"{"bow":2}"#, r#"{"open_water":2}"#, r#"{"OpenWater":2}"#];

/// What [`Scarfing`] is read from: the key [`Sheeting`] reads beside another, no key, and the key
/// held as a number, which [`Sheeting`] does not read.
pub const SCARFING: [&str; 3] = [r#"{"label":"n","north":2}"#, "{}", r#"{"label":5}"#];

/// What [`Gaffing`] is read from: a map with no entry, an entry that reads as a count and one that
/// does not, an optional map with each, a struct beside a member of the variant's own, in the
/// order serde writes them and in another, the member under its alias and under both of its
/// names, and the struct's key absent.
pub const GAFFING: [&str; 11] = [
    r#"{"Counted":{}}"#,
    r#"{"Counted":{"north":2}}"#,
    r#"{"Counted":{"north":"x"}}"#,
    r#"{"Spare":{}}"#,
    r#"{"Spare":{"north":2}}"#,
    r#"{"Spare":{"north":"x"}}"#,
    r#"{"Labelled":{"label":"n","note":"a"}}"#,
    r#"{"Labelled":{"note":"a","label":"n","north":2}}"#,
    r#"{"Labelled":{"label":"n","remark":"a"}}"#,
    r#"{"Labelled":{"label":"n","note":"a","remark":"b"}}"#,
    r#"{"Labelled":{"note":"a"}}"#,
];

/// What [`Bowlining`] is read from: [`GAFFING`]'s variants under a tag and a content key, and a
/// member that names a function flattened after a struct.
pub const BOWLINING: [&str; 9] = [
    r#"{"kind":"Counted","held":{}}"#,
    r#"{"kind":"Counted","held":{"north":2}}"#,
    r#"{"kind":"Counted","held":{"north":"x"}}"#,
    r#"{"kind":"Spare","held":{"north":2}}"#,
    r#"{"kind":"Spare","held":{"north":"x"}}"#,
    r#"{"kind":"Labelled","held":{"note":"a","label":"n"}}"#,
    r#"{"kind":"Labelled","held":{"note":"a"}}"#,
    r#"{"held":{"label":"n","north":2},"kind":"Tallied"}"#,
    r#"{"kind":"Tallied","held":{"north":2}}"#,
];

/// What [`Swaying`] is read from: the keys of its struct beside a member of the variant's own, a
/// count, a count of text, no key, the struct's key alone, which neither the struct nor a count
/// reads, and the keys of the struct and the member beside a key neither reads.
pub const SWAYING: [&str; 6] = [
    r#"{"note":"a","label":"n"}"#,
    r#"{"north":2}"#,
    r#"{"north":"x"}"#,
    "{}",
    r#"{"label":"n"}"#,
    r#"{"label":"n","note":"a","north":2}"#,
];

/// What [`Whelping`] is read from: a struct beside a member of the variant's own beside the tag,
/// in the order serde writes them and in another, a struct alone beside the tag and beside a key
/// no member reads, and the tag alone.
pub const WHELPING: [&str; 5] = [
    r#"{"kind":"Labelled","label":"n","note":"a"}"#,
    r#"{"note":"a","label":"n","kind":"Labelled"}"#,
    r#"{"kind":"Hoisted","label":"n"}"#,
    r#"{"label":"n","north":2,"kind":"Hoisted"}"#,
    r#"{"kind":"Hoisted"}"#,
];

/// What [`Broaching`] is read from at `u8`: no entry, a count, and a count of text.
pub const BROACHING: [&str; 3] = [
    r#"{"Counted":{}}"#,
    r#"{"Counted":{"north":2}}"#,
    r#"{"Counted":{"north":"x"}}"#,
];

/// A map, the one member, which serde hands every key of the object.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Netting {
    #[serde(flatten)]
    pub rest: HashMap<String, u8>,
}

/// An optional map, the one member: serde reads it as `None` wherever an entry it is handed does
/// not read as a count, and writes `None` as no entry.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProbeLocker {
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub counts: Option<BTreeMap<String, u8>>,
}

/// [`ProbeLocker`] under a function that answers the payload as it was handed: serde reads the
/// map from what it answers, and refuses an entry that does not read as a count.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CorrectedLocker {
    #[model_schema_prop(preprocess = ["asGiven"])]
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub counts: Option<BTreeMap<String, u8>>,
}

/// [`ProbeLocker`] under a function that answers `null`, which serde reads as `None`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EmptiedLocker {
    #[model_schema_prop(preprocess = ["asNull"])]
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub counts: Option<BTreeMap<String, u8>>,
}

/// The keys of [`Cathead`]'s map.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Chock {
    Port,
    Starboard,
}

/// A map keyed by a plain enum, the one member: serde writes each key as the name its variant is
/// written under, and refuses a key no variant is written under.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Cathead {
    #[serde(flatten)]
    pub lines: BTreeMap<Chock, u8>,
}

/// [`Cathead`]'s map optional: serde reads it as `None` where a key names no variant of [`Chock`].
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Hawsepipe {
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub lines: Option<BTreeMap<Chock, u8>>,
}

/// The keys of [`Snubbing`]'s map, written under the enum's `rename_all` and under a `rename` of
/// their own.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Hawser {
    #[serde(rename = "bow")]
    Forward,
    OpenWater,
}

/// A map keyed by [`Hawser`], the one member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Snubbing {
    #[serde(flatten)]
    pub lines: BTreeMap<Hawser, u8>,
}

/// A flattened struct beside a member of its own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Knotting {
    #[serde(flatten)]
    pub labelling: Labelling,
    pub note: String,
}

/// An optional member beside a flattened one, its key left out where it holds no value.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Splicing {
    #[serde(flatten)]
    pub labelling: Labelling,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare: Option<u32>,
}

/// [`Splicing`] with its optional member under `nullable`, whose key is always written.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Seizing {
    #[serde(flatten)]
    pub labelling: Labelling,
    #[model_schema_prop(nullable)]
    pub spare: Option<u32>,
}

/// The keys a function was handed, which the members below that name one read.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Bunting {
    pub keys: Vec<String>,
}

/// A member that reads keys [`Labelling`] does not.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Noting {
    pub note: String,
}

/// A member that names a function flattened after a struct: serde hands the function the keys the
/// struct did not take.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Whipping {
    #[serde(flatten)]
    pub labelling: Labelling,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub tally: Bunting,
}

/// A member that names a function flattened after a struct, with a member of its holder's own
/// between them: serde hands the function the keys neither took.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Hitching {
    #[serde(flatten)]
    pub labelling: Labelling,
    pub note: String,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub tally: Bunting,
}

/// A member that names a function flattened ahead of a struct: serde hands the function every key,
/// the struct's among them, and the struct the keys it reads.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Bending {
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub extra: Bunting,
    #[serde(flatten)]
    pub labelling: Labelling,
}

/// A struct that names a function on itself, which serde reads as a value and so takes no key.
#[model_schema(preprocess = ["asGiven"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Ticketed {
    pub label: String,
}

/// A member that names a function flattened after [`Ticketed`]: serde hands the function every
/// key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Lanyarding {
    #[serde(flatten)]
    pub issued: Ticketed,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub tally: Bunting,
}

/// A struct with a flattened member of its own, which serde reads as a map and so takes no key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Sheeting {
    #[serde(flatten)]
    pub labelling: Labelling,
}

/// A member that names a function flattened after [`Sheeting`]: serde hands the function every
/// key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Brailing {
    #[serde(flatten)]
    pub sheeting: Sheeting,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub tally: Bunting,
}

/// [`Sheeting`] optional, the one member: serde reads it wherever [`Sheeting`] reads from the
/// object, and as `None` wherever it does not.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Scarfing {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub sheeting: Option<Sheeting>,
}

/// A struct with a member serde reads and never writes, which takes the keys it reads.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Unwritten {
    #[serde(default, skip_serializing)]
    pub hidden: u8,
    pub label: String,
}

/// A member that names a function flattened after [`Unwritten`]: serde hands the function the keys
/// the struct did not take.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Rousing {
    #[serde(flatten)]
    pub hauled: Unwritten,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub tally: Bunting,
}

/// A struct flattened after [`Unwritten`] that reads a key it took: serde hands the second the
/// object without it and refuses the payload.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Gasketing {
    #[serde(flatten)]
    pub hauled: Unwritten,
    #[serde(flatten)]
    pub labelling: Labelling,
}

/// A struct under its own tag, which serde writes, and reads past without taking its key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub struct Ensign {
    pub label: String,
}

/// A member that names a function flattened after [`Ensign`]: serde hands the function the tag.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Dipping {
    #[serde(flatten)]
    pub ensign: Ensign,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub tally: Bunting,
}

/// [`Ensign`] optional, beside a member of its holder's own: serde reads no value where the object
/// holds its tag and none of the keys it reads.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Striking {
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub ensign: Option<Ensign>,
    pub note: String,
}

/// A struct under its own tag, which serde writes ahead of the struct's members.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub struct Flag {
    pub label: String,
}

/// [`Flag`] with a type parameter, its tag written ahead of its members at every filling.
#[model_schema(default_types(Held = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub struct Gonfalon<Held> {
    pub label: Held,
}

/// [`Flag`] with a member read under its alias as well.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub struct Semaphore {
    #[serde(alias = "name")]
    pub label: String,
}

/// [`Flag`] with a flattened member, whose serializer tixschema writes.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub struct Pennant {
    #[serde(flatten)]
    pub labelling: Labelling,
    pub note: String,
}

/// [`Ensign`] flattened ahead of a member of its holder's own: serde writes the struct's tag first.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Dressing {
    #[serde(flatten)]
    pub ensign: Ensign,
    pub north: u8,
}

/// A struct with a member serde writes and never reads, which does not take that member's key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Marline {
    pub label: String,
    #[serde(skip_deserializing)]
    pub spare: u8,
}

/// A member that names a function flattened after [`Marline`]: serde hands the function the key
/// of the member the struct never reads.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Gammoning {
    #[serde(flatten)]
    pub marline: Marline,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub tally: Bunting,
}

/// A struct flattened after a struct that reads other keys, which serde reads.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Stropping {
    #[serde(flatten)]
    pub labelling: Labelling,
    #[serde(flatten)]
    pub noting: Noting,
}

/// A struct flattened after a struct that reads the same key: serde hands the second the object
/// without it and refuses the payload.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Stoppering {
    #[serde(flatten)]
    pub labelling: Labelling,
    #[serde(flatten)]
    pub twice: Labelling,
}

/// A struct flattened beside a member of its holder's own that reads the same key: serde hands the
/// struct the object without it and refuses the payload.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Nipping {
    pub label: String,
    #[serde(flatten)]
    pub labelling: Labelling,
}

/// An optional struct flattened after a struct that reads the same key: serde hands the second
/// the object without it and reads no value.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Slinging {
    #[serde(flatten)]
    pub labelling: Labelling,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub spare: Option<Labelling>,
}

/// An externally tagged enum, which serde reads from the first entry under a variant's name or an
/// alias of one.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Tacking {
    Bay {
        label: String,
    },
    Calm,
    #[serde(alias = "Quay")]
    Dock {
        label: String,
    },
    Held(String),
}

/// An externally tagged enum flattened beside a member of its holder's own: serde hands the enum
/// the first entry under the name of a variant, and leaves every other key to the holder.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Stowing {
    pub note: String,
    #[serde(flatten)]
    pub tacking: Tacking,
}

/// [`Stowing`] with its enum optional, which serde reads no value of where no entry is under the
/// name of a variant.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Belting {
    pub note: String,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub tacking: Option<Tacking>,
}

/// [`Stowing`] with its enum under a function that answers the payload as it was handed, which
/// serde writes as it writes [`Stowing`]'s.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Yawing {
    pub note: String,
    #[model_schema_prop(preprocess = ["asGiven"])]
    #[serde(flatten)]
    pub tacking: Tacking,
}

/// A member that names a function flattened after an externally tagged enum, where serde hands
/// the function every entry but the one the enum took.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Heaving {
    #[serde(flatten)]
    pub tacking: Tacking,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub tally: Bunting,
}

/// An externally tagged enum whose variants are written under the enum's `rename_all` and under a
/// `rename` of their own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Jibing {
    #[serde(rename = "pier")]
    Jetty {
        label: String,
    },
    OpenBay {
        label: String,
    },
}

/// A member that names a function flattened after [`Jibing`].
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Cleating {
    #[serde(flatten)]
    pub jibing: Jibing,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub tally: Bunting,
}

/// A struct and a member that names a function flattened after an externally tagged enum, where
/// serde hands the struct every entry but the enum's, and the function every entry neither took.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Scudding {
    #[serde(flatten)]
    pub hatch: Tacking,
    #[serde(flatten)]
    pub labelling: Labelling,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub tally: Bunting,
}

/// An externally tagged enum flattened beside a member of its holder's own and declared below the
/// holder, which the emitter has not written when it writes the holder.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Luffing {
    pub note: String,
    #[serde(flatten)]
    pub quaying: Quaying,
}

/// The enum [`Luffing`] flattens.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Quaying {
    Bay { label: String },
}

/// A map flattened alone in a variant written under its name, an optional one, and a struct
/// flattened beside a member of the variant's own, read under an alias too: serde reads and
/// writes each at the variant's own level.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Gaffing {
    Counted {
        #[serde(flatten)]
        counts: BTreeMap<String, u8>,
    },
    Labelled {
        #[serde(flatten)]
        labelling: Labelling,
        #[serde(alias = "remark")]
        note: String,
    },
    Spare {
        #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
        counts: Option<BTreeMap<String, u8>>,
    },
}

/// [`Gaffing`] under a tag and a content key, with a variant whose member that names a function
/// is flattened after a struct: serde hands the function the keys the struct did not take.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum Bowlining {
    Counted {
        #[serde(flatten)]
        counts: BTreeMap<String, u8>,
    },
    Labelled {
        #[serde(flatten)]
        labelling: Labelling,
        note: String,
    },
    Spare {
        #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
        counts: Option<BTreeMap<String, u8>>,
    },
    Tallied {
        #[serde(flatten)]
        labelling: Labelling,
        #[model_schema_prop(preprocess = ["withItsKeys"])]
        #[serde(flatten)]
        tally: Bunting,
    },
}

/// [`Gaffing`]'s variants untagged, the struct first: serde takes the first variant that reads.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Swaying {
    Annotated {
        #[serde(flatten)]
        labelling: Labelling,
        note: String,
    },
    Counted {
        #[serde(flatten)]
        counts: BTreeMap<String, u8>,
    },
    Spare {
        #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
        counts: Option<BTreeMap<String, u8>>,
    },
}

/// A struct flattened in a variant under a tag alone, beside a member of the variant's own and
/// alone: serde reads it from the object without the tag.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Whelping {
    Hoisted {
        #[serde(flatten)]
        labelling: Labelling,
    },
    Labelled {
        #[serde(flatten)]
        labelling: Labelling,
        note: String,
    },
}

/// A map of the enum's own parameter flattened alone in a variant written under its name.
#[model_schema(default_types(Count = u8))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Broaching<Count> {
    Counted {
        #[serde(flatten)]
        counts: BTreeMap<String, Count>,
    },
}

/// The payload as it was handed.
const fn as_given(payload: Value) -> Value {
    payload
}

/// `null`, whatever `payload` is.
fn as_null(_payload: Value) -> Value {
    Value::Null
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

/// What serde reads of `payload` as `T`, written back, or `refused` where it refuses it.
pub fn read_of<T>(payload: &str) -> String
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    serde_json::from_str::<T>(payload).map_or_else(
        |_| "refused".to_owned(),
        |read| serde_json::to_string(&read).unwrap(),
    )
}
