//! The models of the flattened Dart reads `dart_tests/flattened.rs` pins and
//! `service_schema_emitted_client_tests/run_dart_flattened.rs` runs: a map that is the one member
//! of its struct; and a member that names a function, which is handed the keys serde hands it,
//! after a struct, beside a member of its holder's own and in a variant under a tag key, ahead of a
//! struct, after each member that takes none of the keys it reads, and after an externally tagged
//! enum, an enum under a tag key and an untagged one; and an externally tagged enum beside a member
//! of its holder's own, and that enum optional; and an optional map, the one member of its struct
//! or of a variant, keyed by a plain enum, of models with a rule and under a function. serde reads
//! the members in the order declared: a struct takes the keys it reads, an externally tagged enum
//! the first entry under the name of a variant it reads, wherever it is, and every flattened member
//! is handed every entry no member ahead of it took. A struct after a struct that reads the same
//! key is refused, and an optional map that names no function is `None` wherever an entry does not
//! read.

use alloc::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::model_schema;

/// What every struct below but those named apart is read from.
pub const PAYLOAD: &str = r#"{"label":"n","north":2}"#;

/// What [`Spare`] is read from.
pub const SPARE: &str = r#"{"north":2,"south":3}"#;

/// What [`Clamping`] is read from.
pub const CLAMPING: &str = r#"{"label":"n","north":2,"note":"a"}"#;

/// What [`Fastening`] is read from.
pub const FASTENING: &str = r#"{"north":2,"note":"a"}"#;

/// What [`Loading`] is read from: one variant's entry, two variants' entries in each order, a
/// variant's entry under its alias, an alias after a variant's entry, a key ahead of a variant's
/// entry, and two keys ahead of two variants' entries.
pub const LOADING: [&str; 7] = [
    r#"{"Bay":{"label":"n"},"north":2}"#,
    r#"{"Bay":{"label":"n"},"Dock":{"label":"s"},"north":2}"#,
    r#"{"Dock":{"label":"s"},"Bay":{"label":"n"},"north":2}"#,
    r#"{"Quay":{"label":"s"},"north":2}"#,
    r#"{"Bay":{"label":"n"},"Quay":{"label":"s"},"north":2}"#,
    r#"{"north":2,"Bay":{"label":"n"}}"#,
    r#"{"north":2,"south":3,"Dock":{"label":"s"},"Bay":{"label":"n"}}"#,
];

/// What [`Battening`] is read from: a key no member reads ahead of a variant's entry, two such
/// keys, the holder's own key and another ahead of two variants' entries, and no variant's entry.
pub const BATTENING: [&str; 4] = [
    r#"{"north":2,"Bay":{"label":"n"},"note":"a"}"#,
    r#"{"north":2,"south":3,"Bay":{"label":"n"},"note":"a"}"#,
    r#"{"note":"a","north":2,"Dock":{"label":"s"},"Bay":{"label":"n"}}"#,
    r#"{"north":2,"note":"a"}"#,
];

/// What [`Caulking`] is read from: no key but the holder's own, a key no member reads, a variant's
/// entry after one, and a variant's entry that does not read.
pub const CAULKING: [&str; 4] = [
    r#"{"note":"a"}"#,
    r#"{"north":2,"note":"a"}"#,
    r#"{"north":2,"Bay":{"label":"n"},"note":"a"}"#,
    r#"{"Bay":{"label":5},"note":"a"}"#,
];

/// What [`Kedging`] is read from: a variant under its `rename`, one under the enum's `rename_all`,
/// and one under its Rust name, which serde reads no variant under.
pub const KEDGING: [&str; 3] = [
    r#"{"pier":{"label":"n"},"north":2}"#,
    r#"{"open_bay":{"label":"n"},"north":2}"#,
    r#"{"OpenBay":{"label":"n"},"north":2}"#,
];

/// What [`Trimming`] is read from.
pub const TRIMMING: &str = r#"{"Bay":{"label":"n"},"label":"m","north":2}"#;

/// What [`Mousing`] is read from.
pub const MOUSING: &str = r#"{"kind":"Rope","label":"n","north":2}"#;

/// What [`Dunnage`], [`Scuppering`] and [`Hawsehole`] are read from: no entry, an entry that reads
/// as a count, an entry that does not, the two side by side, and `null`, which a count does not
/// read.
pub const DUNNAGE: [&str; 5] = [
    "{}",
    r#"{"north":2}"#,
    r#"{"north":"x"}"#,
    r#"{"north":2,"south":"x"}"#,
    r#"{"north":null}"#,
];

/// What [`Steeving`] is read from: its variant with no entry, with an entry that reads as a count,
/// and with one that does not.
pub const STEEVING: [&str; 3] = [
    r#"{"Open":{}}"#,
    r#"{"Open":{"north":2}}"#,
    r#"{"Open":{"north":"x"}}"#,
];

/// What [`Freeboard`] is read from: its variant with no entry under the content key, with an entry
/// that reads as a count, and with one that does not.
pub const FREEBOARD: [&str; 3] = [
    r#"{"kind":"Open","held":{}}"#,
    r#"{"kind":"Open","held":{"north":2}}"#,
    r#"{"kind":"Open","held":{"north":"x"}}"#,
];

/// What [`Bilging`] is read from: a key a variant of [`Garboard`] is written under, a key none is
/// written under, and a count of text.
pub const BILGING: [&str; 3] = [r#"{"North":2}"#, r#"{"West":2}"#, r#"{"North":"x"}"#];

/// What [`Limber`] is read from: a model that keeps its rule, one that breaks it, and one that does
/// not read. A build with no schema surface holds no rule in Rust, and declares neither.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
pub const LIMBER: [&str; 3] = [
    r#"{"north":{"mark":"x"}}"#,
    r#"{"north":{"mark":""}}"#,
    r#"{"north":{"mark":5}}"#,
];

/// The member the structs below flatten.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Marking {
    pub label: String,
}

/// The keys a function was handed, which the members below that name one read.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Keying {
    pub keys: Vec<String>,
}

/// A map, the one member of its struct, which serde hands every key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Spare {
    #[serde(flatten)]
    pub rest: BTreeMap<String, u8>,
}

/// A member that names a function flattened after a struct, with a member of its holder's own
/// between them: serde hands the function the keys neither took.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Clamping {
    #[serde(flatten)]
    pub marking: Marking,
    pub note: String,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub other: Keying,
}

/// A member that names a function, the one flattened member, beside a member of its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Fastening {
    pub note: String,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub other: Keying,
}

/// A member that names a function flattened ahead of a struct, where serde hands the function
/// every key, the struct's among them.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Hoisting {
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub extra: Keying,
    #[serde(flatten)]
    pub marking: Marking,
}

/// A struct that names a function on itself, which serde reads as a value.
#[model_schema(preprocess = ["asGiven"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Ticketing {
    pub label: String,
}

/// A struct with a flattened member of its own, which serde reads as a map.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Bracing {
    #[serde(flatten)]
    pub marking: Marking,
}

/// A member that names a function flattened after a struct that names one on itself, where serde
/// hands the function every key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Lacing {
    #[serde(flatten)]
    pub issued: Ticketing,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub rest: Keying,
}

/// A member that names a function flattened after another that names one, where serde hands the
/// second every key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Rigging {
    #[model_schema_prop(preprocess = ["asGiven"])]
    #[serde(flatten)]
    pub marking: Marking,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub rest: Keying,
}

/// A member that names a function flattened after a struct with a flattened member of its own,
/// where serde hands the function every key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Belaying {
    #[serde(flatten)]
    pub bracing: Bracing,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub rest: Keying,
}

/// A member that names a function flattened after a struct, where serde hands the function the
/// keys the struct did not take.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Twin {
    #[serde(flatten)]
    pub marking: Marking,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub other: Keying,
}

/// A struct flattened after a struct that reads the same key, where serde hands the second the
/// object without it and refuses the payload.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Doubling {
    #[serde(flatten)]
    pub marking: Marking,
    #[serde(flatten)]
    pub twice: Marking,
}

/// An externally tagged enum, which serde reads from the first entry under a variant's name or an
/// alias of one.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Hatching {
    Bay {
        label: String,
    },
    #[serde(alias = "Quay")]
    Dock {
        label: String,
    },
}

/// A member that names a function flattened after an externally tagged enum, where serde hands
/// the function every entry but the one the enum took.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Loading {
    #[serde(flatten)]
    pub hatching: Hatching,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub rest: Keying,
}

/// An externally tagged enum flattened beside a member of its holder's own, which serde reads from
/// the first entry under a variant's name wherever it is in the object, ignoring every key no
/// member reads.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Battening {
    #[serde(flatten)]
    pub hatching: Hatching,
    pub note: String,
}

/// [`Battening`] with its enum optional, which serde reads no value of where no entry is under the
/// name of a variant, and where the entry under one does not read.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Caulking {
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub hatching: Option<Hatching>,
    pub note: String,
}

/// An externally tagged enum whose variants are written under the enum's `rename_all` and under a
/// `rename` of their own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Warping {
    #[serde(rename = "pier")]
    Jetty {
        label: String,
    },
    OpenBay {
        label: String,
    },
}

/// A member that names a function flattened after [`Warping`].
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Kedging {
    #[serde(flatten)]
    pub kedge: Warping,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub rest: Keying,
}

/// A struct and a member that names a function flattened after an externally tagged enum, where
/// serde hands the struct every entry but the enum's, and the function every entry neither took.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Trimming {
    #[serde(flatten)]
    pub hatching: Hatching,
    #[serde(flatten)]
    pub marking: Marking,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub rest: Keying,
}

/// An enum under a tag key alone, which serde reads as a map and so takes no key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Frapping {
    Rope { label: String },
}

/// A member that names a function flattened after [`Frapping`], where serde hands the function
/// every key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Mousing {
    #[serde(flatten)]
    pub frapping: Frapping,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub rest: Keying,
}

/// An untagged enum, which serde reads as a map and so takes no key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Chocking {
    Rope { label: String },
}

/// A member that names a function flattened after [`Chocking`], where serde hands the function
/// every key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Shoring {
    #[serde(flatten)]
    pub chocking: Chocking,
    #[model_schema_prop(preprocess = ["withItsKeys"])]
    #[serde(flatten)]
    pub rest: Keying,
}

/// An optional map, the one member of its struct: serde reads it as `None` wherever an entry it is
/// handed does not read as a count, and writes `None` as no entry.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Dunnage {
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub counts: Option<BTreeMap<String, u8>>,
}

/// [`Dunnage`]'s map, the one member of a variant written under its name.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Steeving {
    Open {
        #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
        counts: Option<BTreeMap<String, u8>>,
    },
}

/// [`Dunnage`]'s map, the one member of a variant under a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum Freeboard {
    Open {
        #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
        counts: Option<BTreeMap<String, u8>>,
    },
}

/// [`Dunnage`]'s map, the one member of an untagged variant.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Scuppering {
    Open {
        #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
        counts: Option<BTreeMap<String, u8>>,
    },
}

/// The keys of [`Bilging`]'s map.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Garboard {
    North,
    South,
}

/// An optional map keyed by [`Garboard`]: serde reads it as `None` where a key names no variant.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Bilging {
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub counts: Option<BTreeMap<Garboard, u8>>,
}

/// The model [`Limber`]'s map holds, whose `validate()` holds its mark to a length.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Scantling {
    #[model_schema_prop(minLength = 1)]
    pub mark: String,
}

/// An optional map of [`Scantling`]s: serde reads it as `None` where a model does not read, and
/// reads a model that breaks its rule with no judgement, the holder's read refusing it.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Limber {
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub notches: Option<BTreeMap<String, Scantling>>,
}

/// [`Dunnage`]'s map under a function that answers the payload as it was handed: serde reads the
/// map from what it answers, and refuses an entry that does not read as a count.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Hawsehole {
    #[model_schema_prop(preprocess = ["asGiven"])]
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub counts: Option<BTreeMap<String, u8>>,
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

/// A map that is the one member of its struct holds every key.
#[test]
fn serde_hands_a_map_alone_every_key() {
    let spare: Spare = serde_json::from_str(SPARE).unwrap();
    assert_eq!(written(&spare.rest), SPARE);
}

/// A function after a struct is handed the keys neither the struct nor its holder took, and one
/// ahead of a struct, and after a member that takes no key, every key but its holder's own.
#[test]
fn serde_hands_a_function_the_keys_no_member_ahead_of_it_took() {
    let clamping: Clamping = serde_json::from_str(CLAMPING).unwrap();
    assert_eq!(clamping.other.keys, ["north"]);
    let fastening: Fastening = serde_json::from_str(FASTENING).unwrap();
    assert_eq!(fastening.other.keys, ["north"]);
    let hoisting: Hoisting = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(hoisting.extra.keys, ["label", "north"]);
    assert_eq!(hoisting.marking.label, "n");
    let lacing: Lacing = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(lacing.rest.keys, ["label", "north"]);
    let rigging: Rigging = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(rigging.rest.keys, ["label", "north"]);
    let belaying: Belaying = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(belaying.rest.keys, ["label", "north"]);
    let twin: Twin = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(twin.other.keys, ["north"]);
    assert_eq!(written(&twin), r#"{"label":"n","keys":["north"]}"#);
    let refused = serde_json::from_str::<Doubling>(PAYLOAD).unwrap_err();
    assert_eq!(
        refused.to_string(),
        "missing field `label` at line 1 column 23"
    );
}

/// What serde reads of `payload` as `T`, through `parts`, or that it refuses it.
fn read_of<T>(payload: &str, parts: fn(&T) -> String) -> String
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_str::<T>(payload).map_or_else(|_| "refused".to_owned(), |read| parts(&read))
}

/// What a function after an externally tagged enum is handed of each payload, and the variant the
/// enum read: serde takes the first entry under a variant's name or an alias of one, never one
/// under the variant's Rust name, and hands the function every other entry.
#[test]
fn serde_hands_a_function_after_an_externally_tagged_enum_every_entry_but_the_variants() {
    let loaded: Vec<String> = LOADING
        .iter()
        .map(|payload| {
            read_of(payload, |loading: &Loading| {
                format!("{} {:?}", written(&loading.hatching), loading.rest.keys)
            })
        })
        .collect();
    assert_eq!(
        loaded,
        [
            r#"{"Bay":{"label":"n"}} ["north"]"#,
            r#"{"Bay":{"label":"n"}} ["Dock", "north"]"#,
            r#"{"Dock":{"label":"s"}} ["Bay", "north"]"#,
            r#"{"Dock":{"label":"s"}} ["north"]"#,
            r#"{"Bay":{"label":"n"}} ["Quay", "north"]"#,
            r#"{"Bay":{"label":"n"}} ["north"]"#,
            r#"{"Dock":{"label":"s"}} ["north", "south", "Bay"]"#,
        ]
    );
    let kedged: Vec<String> = KEDGING
        .iter()
        .map(|payload| {
            read_of(payload, |kedging: &Kedging| {
                format!("{} {:?}", written(&kedging.kedge), kedging.rest.keys)
            })
        })
        .collect();
    assert_eq!(
        kedged,
        [
            r#"{"pier":{"label":"n"}} ["north"]"#,
            r#"{"open_bay":{"label":"n"}} ["north"]"#,
            "refused",
        ]
    );
    let trimming: Trimming = serde_json::from_str(TRIMMING).unwrap();
    assert_eq!(trimming.marking.label, "m");
    assert_eq!(trimming.rest.keys, ["north"]);
}

/// An externally tagged enum beside a member of its holder's own is read from the first entry
/// under a variant's name, after any key no member reads, and refused where no entry is under one.
#[test]
fn serde_reads_a_flattened_externally_tagged_enum_from_its_variants_entry_wherever_it_is() {
    let battened: Vec<String> = BATTENING
        .iter()
        .map(|payload| {
            read_of(payload, |battening: &Battening| {
                format!("{} {}", written(&battening.hatching), battening.note)
            })
        })
        .collect();
    assert_eq!(
        battened,
        [
            r#"{"Bay":{"label":"n"}} a"#,
            r#"{"Bay":{"label":"n"}} a"#,
            r#"{"Dock":{"label":"s"}} a"#,
            "refused",
        ]
    );
}

/// An optional externally tagged enum beside a member of its holder's own is read as no value where
/// no entry is under a variant's name, and where the entry under one does not read, which the Zod
/// schema refuses.
#[test]
fn serde_reads_an_optional_flattened_externally_tagged_enum_as_none_where_no_entry_reads() {
    let caulked = CAULKING.map(|payload| {
        read_of(payload, |caulking: &Caulking| {
            format!("{} {}", written(&caulking.hatching), written(caulking))
        })
    });
    assert_eq!(
        caulked,
        [
            r#"null {"note":"a"}"#,
            r#"null {"note":"a"}"#,
            r#"{"Bay":{"label":"n"}} {"Bay":{"label":"n"},"note":"a"}"#,
            r#"null {"note":"a"}"#,
        ],
        "serde reads no value where no entry reads"
    );
}

/// A function after an enum under a tag key alone, or after an untagged one, is handed every key.
#[test]
fn serde_hands_a_function_after_an_enum_read_as_a_map_every_key() {
    let mousing: Mousing = serde_json::from_str(MOUSING).unwrap();
    assert_eq!(mousing.rest.keys, ["kind", "label", "north"]);
    let shoring: Shoring = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(shoring.rest.keys, ["label", "north"]);
}

/// An optional map alone in its struct, or the one member of a variant under its name, under a
/// content key or untagged, holds every entry where each reads, and is `None` wherever one does
/// not: a count of text, `null` and a key that names no variant of [`Garboard`]. serde writes
/// nothing for `None`. Under a function, it refuses an entry that does not read.
#[test]
fn serde_reads_an_optional_flattened_map_as_none_wherever_an_entry_does_not_read() {
    let dunnage = DUNNAGE.map(|payload| {
        read_of(payload, |read: &Dunnage| {
            format!("{} {}", written(&read.counts), written(read))
        })
    });
    assert_eq!(
        dunnage,
        [
            "{} {}",
            r#"{"north":2} {"north":2}"#,
            "null {}",
            "null {}",
            "null {}",
        ],
        "serde reads no map where an entry does not read"
    );
    let steeving = STEEVING.map(|payload| {
        read_of(payload, |read: &Steeving| {
            let Steeving::Open { counts } = read;
            format!("{} {}", written(counts), written(read))
        })
    });
    assert_eq!(
        steeving,
        [
            r#"{} {"Open":{}}"#,
            r#"{"north":2} {"Open":{"north":2}}"#,
            r#"null {"Open":{}}"#,
        ],
        "serde reads no map in a variant under its name where an entry does not read"
    );
    let freeboard = FREEBOARD.map(|payload| {
        read_of(payload, |read: &Freeboard| {
            let Freeboard::Open { counts } = read;
            format!("{} {}", written(counts), written(read))
        })
    });
    assert_eq!(
        freeboard,
        [
            r#"{} {"kind":"Open","held":{}}"#,
            r#"{"north":2} {"kind":"Open","held":{"north":2}}"#,
            r#"null {"kind":"Open","held":{}}"#,
        ],
        "serde reads no map under a content key where an entry does not read"
    );
    let scuppering = DUNNAGE.map(|payload| {
        read_of(payload, |read: &Scuppering| {
            let Scuppering::Open { counts } = read;
            format!("{} {}", written(counts), written(read))
        })
    });
    assert_eq!(
        scuppering, dunnage,
        "an untagged variant reads its map so too"
    );
    let bilging = BILGING.map(|payload| {
        read_of(payload, |read: &Bilging| {
            format!("{} {}", written(&read.counts), written(read))
        })
    });
    assert_eq!(
        bilging,
        [r#"{"North":2} {"North":2}"#, "null {}", "null {}"],
        "serde reads no map where a key names no variant"
    );
    let hawsehole = DUNNAGE.map(|payload| {
        read_of(payload, |read: &Hawsehole| {
            format!("{} {}", written(&read.counts), written(read))
        })
    });
    assert_eq!(
        hawsehole,
        [
            "{} {}",
            r#"{"north":2} {"north":2}"#,
            "refused",
            "refused",
            "refused",
        ],
        "serde refuses an entry of what a function answers that does not read"
    );
}

/// An optional map of models is `None` where a model does not read, and a model that breaks its
/// rule is read with no judgement and refused with its holder, under the place serde names.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn serde_reads_an_optional_flattened_map_of_models_as_none_where_a_model_does_not_read() {
    let limber = LIMBER.map(|payload| {
        serde_json::from_str::<Limber>(payload).map_or_else(
            |refused| refused.to_string(),
            |read| format!("{} {}", written(&read.notches), written(&read)),
        )
    });
    assert_eq!(
        limber,
        [
            r#"{"north":{"mark":"x"}} {"north":{"mark":"x"}}"#,
            "'north.mark': too short: minimum length is 1, got 0",
            "null {}",
        ],
        "serde judges a model of the map once, with its holder"
    );
}
