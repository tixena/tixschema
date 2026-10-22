//! The models a map is flattened into as the only member of its object, and the payloads serde
//! reads and refuses for them. A map flattened beside another member stops the build, so a map is
//! handed every key of its object: its holder's own tag too, and none of the tag an internally
//! tagged enum reads ahead of the variant. An optional map is read as `None` wherever an entry it
//! is handed does not read, and leaves the entry in the object.

use alloc::collections::BTreeMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Every payload, one that serde reads and one with a count of the wrong type for each place a
/// map is flattened. An optional map is read as `None` wherever an entry does not read, so serde
/// reads every object its holder reads.
pub const PAYLOADS: [Payload; 56] = [
    Payload {
        json: r#"{"north":2}"#,
        name: "alone",
        read_as: "Thwart",
    },
    Payload {
        json: r#"{"north":"x"}"#,
        name: "alone, a count of text",
        read_as: "Thwart",
    },
    Payload {
        json: r#"{"kind":"TaggedThwart","north":"2"}"#,
        name: "under its holder's tag",
        read_as: "TaggedThwart",
    },
    Payload {
        json: r#"{"kind":"TaggedThwart","north":2}"#,
        name: "under its holder's tag, a count of a number",
        read_as: "TaggedThwart",
    },
    Payload {
        json: r#"{"north":2}"#,
        name: "beside a member serde skips",
        read_as: "Gunwale",
    },
    Payload {
        json: r#"{"north":"x"}"#,
        name: "beside a member serde skips, a count of text",
        read_as: "Gunwale",
    },
    Payload {
        json: r#"{"north":2}"#,
        name: "keyed by a brand",
        read_as: "Strake",
    },
    Payload {
        json: r#"{"north":"x"}"#,
        name: "keyed by a brand, a count of text",
        read_as: "Strake",
    },
    Payload {
        json: r#"{"Open":{"north":2}}"#,
        name: "in a variant",
        read_as: "Coaming",
    },
    Payload {
        json: r#"{"Open":{"north":"x"}}"#,
        name: "in a variant, a count of text",
        read_as: "Coaming",
    },
    Payload {
        json: r#"{"kind":"Open","north":2}"#,
        name: "in a variant under a tag",
        read_as: "Keelson",
    },
    Payload {
        json: r#"{"kind":"Open","north":"x"}"#,
        name: "in a variant under a tag, a count of text",
        read_as: "Keelson",
    },
    Payload {
        json: r#"{"kind":"Open","held":{"north":2}}"#,
        name: "in a variant under a content key",
        read_as: "Transom",
    },
    Payload {
        json: r#"{"kind":"Open","held":{"north":"x"}}"#,
        name: "in a variant under a content key, a count of text",
        read_as: "Transom",
    },
    Payload {
        json: r#"{"north":2}"#,
        name: "in an untagged variant",
        read_as: "Bilge",
    },
    Payload {
        json: r#"{"north":"x"}"#,
        name: "in an untagged variant, a count of text",
        read_as: "Bilge",
    },
    Payload {
        json: "{}",
        name: "optional, no entry",
        read_as: "Locker",
    },
    Payload {
        json: r#"{"north":2}"#,
        name: "optional, a count",
        read_as: "Locker",
    },
    Payload {
        json: r#"{"north":"x"}"#,
        name: "optional, a count of text",
        read_as: "Locker",
    },
    Payload {
        json: r#"{"north":2,"south":"x"}"#,
        name: "optional, a count and a count of text",
        read_as: "Locker",
    },
    Payload {
        json: "[]",
        name: "optional, a list",
        read_as: "Locker",
    },
    Payload {
        json: "{}",
        name: "optional with no default, no entry",
        read_as: "Scupper",
    },
    Payload {
        json: r#"{"north":2}"#,
        name: "optional with no default, a count",
        read_as: "Scupper",
    },
    Payload {
        json: r#"{"north":"x"}"#,
        name: "optional with no default, a count of text",
        read_as: "Scupper",
    },
    Payload {
        json: "{}",
        name: "optional and keyed by an enum, no entry",
        read_as: "Windlass",
    },
    Payload {
        json: r#"{"Port":2}"#,
        name: "optional and keyed by an enum, a count",
        read_as: "Windlass",
    },
    Payload {
        json: r#"{"Port":"x"}"#,
        name: "optional and keyed by an enum, a count of text",
        read_as: "Windlass",
    },
    Payload {
        json: r#"{"north":2}"#,
        name: "optional and keyed by an enum, a key it does not name",
        read_as: "Windlass",
    },
    Payload {
        json: "{}",
        name: "optional and keyed by a number, no entry",
        read_as: "Fathom",
    },
    Payload {
        json: r#"{"7":2}"#,
        name: "optional and keyed by a number, a count",
        read_as: "Fathom",
    },
    Payload {
        json: "{}",
        name: "optional through an alias, no entry",
        read_as: "Sheave",
    },
    Payload {
        json: r#"{"north":2}"#,
        name: "optional through an alias, a count",
        read_as: "Sheave",
    },
    Payload {
        json: r#"{"north":"x"}"#,
        name: "optional through an alias, a count of text",
        read_as: "Sheave",
    },
    Payload {
        json: "{}",
        name: "made optional by its alias, no entry",
        read_as: "Lazarette",
    },
    Payload {
        json: r#"{"north":2}"#,
        name: "made optional by its alias, a count",
        read_as: "Lazarette",
    },
    Payload {
        json: r#"{"north":"x"}"#,
        name: "made optional by its alias, a count of text",
        read_as: "Lazarette",
    },
    Payload {
        json: "{}",
        name: "an optional brand, no entry",
        read_as: "Pintle",
    },
    Payload {
        json: r#"{"north":2}"#,
        name: "an optional brand, a count",
        read_as: "Pintle",
    },
    Payload {
        json: r#"{"north":"x"}"#,
        name: "an optional brand, a count of text",
        read_as: "Pintle",
    },
    Payload {
        json: r#"{"kind":"Taffrail"}"#,
        name: "optional under its holder's tag",
        read_as: "Taffrail",
    },
    Payload {
        json: r#"{"kind":"Taffrail","north":2}"#,
        name: "optional under its holder's tag, a count",
        read_as: "Taffrail",
    },
    Payload {
        json: r#"{"Open":{}}"#,
        name: "optional in a variant, no entry",
        read_as: "Fairlead",
    },
    Payload {
        json: r#"{"Open":{"north":2}}"#,
        name: "optional in a variant, a count",
        read_as: "Fairlead",
    },
    Payload {
        json: r#"{"Open":{"north":"x"}}"#,
        name: "optional in a variant, a count of text",
        read_as: "Fairlead",
    },
    Payload {
        json: r#"{"kind":"Open"}"#,
        name: "optional in a variant under a tag, no entry",
        read_as: "Chock",
    },
    Payload {
        json: r#"{"kind":"Open","north":2}"#,
        name: "optional in a variant under a tag, a count",
        read_as: "Chock",
    },
    Payload {
        json: r#"{"kind":"Open","north":"x"}"#,
        name: "optional in a variant under a tag, a count of text",
        read_as: "Chock",
    },
    Payload {
        json: r#"{"north":2}"#,
        name: "optional in a variant under a tag, no tag",
        read_as: "Chock",
    },
    Payload {
        json: r#"{"kind":"Open","held":{}}"#,
        name: "optional in a variant under a content key, no entry",
        read_as: "Davit",
    },
    Payload {
        json: r#"{"kind":"Open","held":{"north":2}}"#,
        name: "optional in a variant under a content key, a count",
        read_as: "Davit",
    },
    Payload {
        json: r#"{"kind":"Open","held":{"north":"x"}}"#,
        name: "optional in a variant under a content key, a count of text",
        read_as: "Davit",
    },
    Payload {
        json: r#"{"kind":"Open"}"#,
        name: "optional in a variant under a content key, no content",
        read_as: "Davit",
    },
    Payload {
        json: "{}",
        name: "optional in an untagged variant, no entry",
        read_as: "Capstan",
    },
    Payload {
        json: r#"{"north":2}"#,
        name: "optional in an untagged variant, a count",
        read_as: "Capstan",
    },
    Payload {
        json: r#"{"north":"x"}"#,
        name: "optional in an untagged variant, a count of text",
        read_as: "Capstan",
    },
    Payload {
        json: "3",
        name: "optional in an untagged variant, a number",
        read_as: "Capstan",
    },
];

/// Each model a payload is read as, beside serde's read of a payload as it.
const READS: [(&str, ReadAs); 20] = [
    ("Thwart", reads::<Thwart>),
    ("TaggedThwart", reads::<TaggedThwart>),
    ("Gunwale", reads::<Gunwale>),
    ("Strake", reads::<Strake>),
    ("Coaming", reads::<Coaming>),
    ("Keelson", reads::<Keelson>),
    ("Transom", reads::<Transom>),
    ("Bilge", reads::<Bilge>),
    ("Locker", reads::<Locker>),
    ("Scupper", reads::<Scupper>),
    ("Windlass", reads::<Windlass>),
    ("Fathom", reads::<Fathom>),
    ("Sheave", reads::<Sheave>),
    ("Lazarette", reads::<Lazarette>),
    ("Pintle", reads::<Pintle>),
    ("Taffrail", reads::<Taffrail>),
    ("Fairlead", reads::<Fairlead>),
    ("Chock", reads::<Chock>),
    ("Davit", reads::<Davit>),
    ("Capstan", reads::<Capstan>),
];

/// What Zod prints for [`PAYLOADS`]: a payload read, or refused with each issue under its place.
pub const PRINTED: &str = "\
alone: read
alone, a count of text: refused
    'north': Invalid input: expected number, received string
under its holder's tag: read
under its holder's tag, a count of a number: refused
    'north': Invalid input: expected string, received number
beside a member serde skips: read
beside a member serde skips, a count of text: refused
    'north': Invalid input: expected number, received string
keyed by a brand: read
keyed by a brand, a count of text: refused
    'north': Invalid input: expected number, received string
in a variant: read
in a variant, a count of text: refused
    'Open.north': Invalid input: expected number, received string
in a variant under a tag: read
in a variant under a tag, a count of text: refused
    'north': Invalid input: expected number, received string
in a variant under a content key: read
in a variant under a content key, a count of text: refused
    'held.north': Invalid input: expected number, received string
in an untagged variant: read
in an untagged variant, a count of text: refused
    'north': Invalid input: expected number, received string
optional, no entry: read
optional, a count: read
optional, a count of text: read
optional, a count and a count of text: read
optional, a list: refused
    Invalid input
optional with no default, no entry: read
optional with no default, a count: read
optional with no default, a count of text: read
optional and keyed by an enum, no entry: read
optional and keyed by an enum, a count: read
optional and keyed by an enum, a count of text: read
optional and keyed by an enum, a key it does not name: read
optional and keyed by a number, no entry: read
optional and keyed by a number, a count: read
optional through an alias, no entry: read
optional through an alias, a count: read
optional through an alias, a count of text: read
made optional by its alias, no entry: read
made optional by its alias, a count: read
made optional by its alias, a count of text: read
an optional brand, no entry: read
an optional brand, a count: read
an optional brand, a count of text: read
optional under its holder's tag: read
optional under its holder's tag, a count: read
optional in a variant, no entry: read
optional in a variant, a count: read
optional in a variant, a count of text: read
optional in a variant under a tag, no entry: read
optional in a variant under a tag, a count: read
optional in a variant under a tag, a count of text: read
optional in a variant under a tag, no tag: refused
    Invalid input
optional in a variant under a content key, no entry: read
optional in a variant under a content key, a count: read
optional in a variant under a content key, a count of text: read
optional in a variant under a content key, no content: refused
    'held': Invalid input
optional in an untagged variant, no entry: read
optional in an untagged variant, a count: read
optional in an untagged variant, a count of text: read
optional in an untagged variant, a number: refused
    Invalid input
";

/// A map that is the only member of its object.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Thwart {
    #[serde(flatten)]
    pub counts: BTreeMap<String, u8>,
}

/// A map in a struct written under its own tag, which serde hands the map: its values are text,
/// as the tag is.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub struct TaggedThwart {
    #[serde(flatten)]
    pub extra: BTreeMap<String, String>,
}

/// A map beside a member serde neither writes nor reads.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Gunwale {
    #[serde(skip)]
    pub cache: u32,
    #[serde(flatten)]
    pub counts: BTreeMap<String, u8>,
}

/// A brand a map is keyed by.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct StrakeCode(pub String);

/// A map keyed by a brand.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Strake {
    #[serde(flatten)]
    pub counts: BTreeMap<StrakeCode, u8>,
}

/// A map in a variant written under its name.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Coaming {
    Open {
        #[serde(flatten)]
        counts: BTreeMap<String, u8>,
    },
}

/// A map in a variant under a tag, which serde reads ahead of the variant and hands the map none
/// of.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Keelson {
    Open {
        #[serde(flatten)]
        counts: BTreeMap<String, u8>,
    },
}

/// A map in a variant under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum Transom {
    Open {
        #[serde(flatten)]
        counts: BTreeMap<String, u8>,
    },
}

/// A map in an untagged variant.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Bilge {
    Open {
        #[serde(flatten)]
        counts: BTreeMap<String, u8>,
    },
}

/// An optional map alone in its object. serde reads every object as it: the map where each entry
/// reads as a count, and `None` where one does not.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Locker {
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub counts: Option<BTreeMap<String, u8>>,
}

/// The same map with no `default`, which serde reads as it reads [`Locker`].
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Scupper {
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub counts: Option<BTreeMap<String, u8>>,
}

/// An optional map keyed by a plain enum: a key no member of the enum names reads the map as
/// `None` as well.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Windlass {
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub lines: Option<BTreeMap<Hawse, u8>>,
}

/// The keys of [`Windlass`]'s map.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Hawse {
    Port,
    Starboard,
}

/// An optional map keyed by a number, whose keys serde reads none of once flattened: it reads
/// every entry as `None`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Fathom {
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub depths: Option<BTreeMap<u32, u8>>,
}

/// A map under a name of its own.
#[model_schema()]
pub type SheaveTallies = BTreeMap<String, u8>;

/// An optional map named through an alias.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Sheave {
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub counts: Option<SheaveTallies>,
}

/// An optional map under a name of its own.
#[model_schema()]
pub type SpareTallies = Option<BTreeMap<String, u8>>;

/// A map made optional by the alias it is named through.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Lazarette {
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub counts: SpareTallies,
}

/// A brand over a map.
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct PintleCounts(pub BTreeMap<String, u8>);

/// An optional brand over a map.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Pintle {
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub counts: Option<PintleCounts>,
}

/// An optional map in a struct written under its own tag, which serde hands the map: a count of
/// text, so the map reads as `None` wherever the tag is written.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub struct Taffrail {
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub counts: Option<BTreeMap<String, u8>>,
}

/// An optional map in a variant written under its name.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Fairlead {
    Open {
        #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
        counts: Option<BTreeMap<String, u8>>,
    },
}

/// An optional map in a variant under a tag, which serde reads ahead of the variant and hands the
/// map none of.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Chock {
    Open {
        #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
        counts: Option<BTreeMap<String, u8>>,
    },
}

/// An optional map in a variant under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum Davit {
    Open {
        #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
        counts: Option<BTreeMap<String, u8>>,
    },
}

/// An optional map in an untagged variant.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Capstan {
    Open {
        #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
        counts: Option<BTreeMap<String, u8>>,
    },
}

/// Whether serde reads a payload as one model.
type ReadAs = fn(&str) -> bool;

/// One payload: its name, the type it is read as, and the JSON.
pub struct Payload {
    pub json: &'static str,
    pub name: &'static str,
    pub read_as: &'static str,
}

/// Whether serde reads `json` as a `T`.
fn reads<T>(json: &str) -> bool
where
    T: DeserializeOwned,
{
    serde_json::from_str::<T>(json).is_ok()
}

/// Whether serde reads `payload` as the type it is read as.
pub fn serde_reads(payload: &Payload) -> bool {
    let (_, read) = READS
        .iter()
        .find(|(model, _)| *model == payload.read_as)
        .unwrap();
    read(payload.json)
}

/// Holds serde's read of each of [`PAYLOADS`] to the verdict [`PRINTED`] states for it: Zod
/// reads exactly the payloads serde reads.
pub fn assert_serde_reads_what_zod_reads() {
    for payload in &PAYLOADS {
        let read = format!("{}: read", payload.name);
        let refused = format!("{}: refused", payload.name);
        let printed = PRINTED
            .lines()
            .find(|line| *line == read || *line == refused)
            .unwrap();
        assert_eq!(
            serde_reads(payload),
            printed == read,
            "serde and Zod part on `{}`: Zod prints `{printed}`",
            payload.name
        );
    }
}
