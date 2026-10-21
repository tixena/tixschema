//! The models a map is flattened into, and the payloads serde reads and refuses for them. serde
//! hands a flattened map the keys no member of its holder and no member flattened ahead of it
//! took: a struct takes the keys it reads, and a map, an enum under a tag and a struct that
//! flattens a member of its own take none.

use alloc::collections::BTreeMap;
use std::collections::HashMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Every payload, one that serde reads and one with a count that is no number for each place a
/// map is flattened.
pub const PAYLOADS: [Payload; 28] = [
    Payload {
        json: r#"{"note":"a","north":2}"#,
        name: "beside a member",
        read_as: "Stocktake",
    },
    Payload {
        json: r#"{"note":"a","north":"x"}"#,
        name: "beside a member, a count of text",
        read_as: "Stocktake",
    },
    Payload {
        json: r#"{"kind":"TaggedStocktake","note":"a","north":"2"}"#,
        name: "beside a member and its holder's tag",
        read_as: "TaggedStocktake",
    },
    Payload {
        json: r#"{"label":"n","north":2}"#,
        name: "after a struct",
        read_as: "Stowing",
    },
    Payload {
        json: r#"{"label":"n","north":"x"}"#,
        name: "after a struct, a count of text",
        read_as: "Stowing",
    },
    Payload {
        json: r#"{"label":"n","note":"a","north":2}"#,
        name: "after a struct and beside a member",
        read_as: "Trussing",
    },
    Payload {
        json: r#"{"label":"n","note":"a","north":"x"}"#,
        name: "after a struct and beside a member, a count of text",
        read_as: "Trussing",
    },
    Payload {
        json: r#"{"label":"n","north":2}"#,
        name: "ahead of a struct",
        read_as: "Ballast",
    },
    Payload {
        json: r#"{"label":"n","north":"2"}"#,
        name: "ahead of a struct, of text",
        read_as: "Dunnage",
    },
    Payload {
        json: r#"{"north":2,"south":3}"#,
        name: "two maps",
        read_as: "Tiers",
    },
    Payload {
        json: r#"{"north":2,"south":"x"}"#,
        name: "two maps, a count of text",
        read_as: "Tiers",
    },
    Payload {
        json: r#"{"label":"n","north":2}"#,
        name: "after a struct that flattens its own",
        read_as: "Sheeting",
    },
    Payload {
        json: r#"{"label":"n","north":2}"#,
        name: "after an alias of a struct",
        read_as: "Cording",
    },
    Payload {
        json: r#"{"label":"n","north":2}"#,
        name: "after an optional struct",
        read_as: "Rigging",
    },
    Payload {
        json: r#"{"north":2}"#,
        name: "after an optional struct left out",
        read_as: "Rigging",
    },
    Payload {
        json: r#"{"Pier":{"label":"n"},"north":2}"#,
        name: "after an enum",
        read_as: "Pigeonhole",
    },
    Payload {
        json: r#"{"kind":"Up","held":{"label":"n"},"north":2}"#,
        name: "after an enum under a tag and a content key",
        read_as: "Davit",
    },
    Payload {
        json: r#"{"note":"a","north":2}"#,
        name: "keyed by a brand",
        read_as: "Shelflist",
    },
    Payload {
        json: r#"{"note":"a","north":"x"}"#,
        name: "keyed by a brand, a count of text",
        read_as: "Shelflist",
    },
    Payload {
        json: r#"{"id":"a","north":2}"#,
        name: "beside a parameter",
        read_as: "Billing",
    },
    Payload {
        json: r#"{"Open":{"note":"a","north":2}}"#,
        name: "in a variant",
        read_as: "Hatchway",
    },
    Payload {
        json: r#"{"Open":{"note":"a","north":"x"}}"#,
        name: "in a variant, a count of text",
        read_as: "Hatchway",
    },
    Payload {
        json: r#"{"kind":"Open","note":"a","north":2}"#,
        name: "in a variant under a tag",
        read_as: "Compartment",
    },
    Payload {
        json: r#"{"kind":"Open","note":"a","north":"x"}"#,
        name: "in a variant under a tag, a count of text",
        read_as: "Compartment",
    },
    Payload {
        json: r#"{"kind":"Open","held":{"note":"a","north":2}}"#,
        name: "in a variant under a content key",
        read_as: "Bulkhead",
    },
    Payload {
        json: r#"{"kind":"Open","held":{"note":"a","north":"x"}}"#,
        name: "in a variant under a content key, a count of text",
        read_as: "Bulkhead",
    },
    Payload {
        json: r#"{"note":"a","north":2}"#,
        name: "in an untagged variant",
        read_as: "Scuttle",
    },
    Payload {
        json: r#"{"note":"a","north":"x"}"#,
        name: "in an untagged variant, a count of text",
        read_as: "Scuttle",
    },
];

/// Each model a payload is read as, beside serde's read of a payload as it.
const READS: [(&str, ReadAs); 18] = [
    ("Stocktake", reads::<Stocktake>),
    ("TaggedStocktake", reads::<TaggedStocktake>),
    ("Stowing", reads::<Stowing>),
    ("Trussing", reads::<Trussing>),
    ("Ballast", reads::<Ballast>),
    ("Dunnage", reads::<Dunnage>),
    ("Tiers", reads::<Tiers>),
    ("Sheeting", reads::<Sheeting>),
    ("Cording", reads::<Cording>),
    ("Rigging", reads::<Rigging>),
    ("Pigeonhole", reads::<Pigeonhole>),
    ("Davit", reads::<Davit>),
    ("Shelflist", reads::<Shelflist>),
    ("Billing", reads::<Billing<String>>),
    ("Hatchway", reads::<Hatchway>),
    ("Compartment", reads::<Compartment>),
    ("Bulkhead", reads::<Bulkhead>),
    ("Scuttle", reads::<Scuttle>),
];

/// What Zod prints for [`PAYLOADS`]: a payload read, or refused with each issue under its place.
pub const PRINTED: &str = "\
beside a member: read
beside a member, a count of text: refused
    'north': Invalid input: expected number, received string
beside a member and its holder's tag: read
after a struct: read
after a struct, a count of text: refused
    'north': Invalid input: expected number, received string
after a struct and beside a member: read
after a struct and beside a member, a count of text: refused
    'north': Invalid input: expected number, received string
ahead of a struct: refused
    'label': Invalid input: expected number, received string
ahead of a struct, of text: read
two maps: read
two maps, a count of text: refused
    'south': Invalid input: expected number, received string
    'south': Invalid input: expected number, received string
after a struct that flattens its own: refused
    'label': Invalid input: expected number, received string
after an alias of a struct: read
after an optional struct: read
after an optional struct left out: read
after an enum: read
after an enum under a tag and a content key: read
keyed by a brand: read
keyed by a brand, a count of text: refused
    'north': Invalid input: expected number, received string
beside a parameter: read
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
";

/// The struct a map is flattened after.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Marking {
    pub label: String,
}

/// A map beside a member of its own holder, which serde hands none of that member's key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Stocktake {
    #[serde(flatten)]
    pub counts: BTreeMap<String, u8>,
    pub note: String,
}

/// A map beside a member of a struct written under its own tag, which serde hands the map: its
/// values are text, as the tag is.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub struct TaggedStocktake {
    #[serde(flatten)]
    pub extra: BTreeMap<String, String>,
    pub note: String,
}

/// A map after a flattened struct, which takes its keys first.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Stowing {
    #[serde(flatten)]
    pub marking: Marking,
    #[serde(flatten)]
    pub rest: HashMap<String, u8>,
}

/// A map after a flattened struct, beside a member of its own holder.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Trussing {
    #[serde(flatten)]
    pub marking: Marking,
    pub note: String,
    #[serde(flatten)]
    pub rest: BTreeMap<String, u8>,
}

/// A map ahead of a flattened struct, which serde hands the struct's keys as well.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Ballast {
    #[serde(flatten)]
    pub counts: BTreeMap<String, u8>,
    #[serde(flatten)]
    pub marking: Marking,
}

/// [`Ballast`] with a map of text, which reads the struct's keys as well.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Dunnage {
    #[serde(flatten)]
    pub extra: BTreeMap<String, String>,
    #[serde(flatten)]
    pub marking: Marking,
}

/// Two maps, which serde hands the same keys.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Tiers {
    #[serde(flatten)]
    pub first: BTreeMap<String, u8>,
    #[serde(flatten)]
    pub second: BTreeMap<String, u8>,
}

/// A struct that flattens a member of its own, which serde reads as a map: it takes no key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Enclosure {
    #[serde(flatten)]
    pub marking: Marking,
}

/// A map after [`Enclosure`], which serde hands the keys the enclosure reads as well.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Sheeting {
    #[serde(flatten)]
    pub enclosure: Enclosure,
    #[serde(flatten)]
    pub rest: BTreeMap<String, u8>,
}

/// [`Marking`] under another name.
#[model_schema()]
pub type MarkingAlias = Marking;

/// A map after a struct flattened through an alias.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Cording {
    #[serde(flatten)]
    pub marking: MarkingAlias,
    #[serde(flatten)]
    pub rest: BTreeMap<String, u8>,
}

/// A map after an optional struct, which takes its keys where it is there.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Rigging {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub marking: Option<Marking>,
    #[serde(flatten)]
    pub rest: BTreeMap<String, u8>,
}

/// An enum serde writes under the name of its variant.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Landing {
    Pier { label: String },
}

/// A map after [`Landing`], which takes the key its variant is written under.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Pigeonhole {
    #[serde(flatten)]
    pub landing: Landing,
    #[serde(flatten)]
    pub rest: BTreeMap<String, u8>,
}

/// An enum serde writes under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum Winch {
    Up { label: String },
}

/// A map after [`Winch`], which takes the tag and the content.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Davit {
    #[serde(flatten)]
    pub hoist: Winch,
    #[serde(flatten)]
    pub rest: BTreeMap<String, u8>,
}

/// A brand a map is keyed by.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct ShelfCode(pub String);

/// A map keyed by a brand, beside a member of its own holder.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Shelflist {
    #[serde(flatten)]
    pub counts: BTreeMap<ShelfCode, u8>,
    pub note: String,
}

/// A map beside a member typed with a parameter of its holder.
#[model_schema(default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Billing<IdType> {
    #[serde(flatten)]
    pub counts: BTreeMap<String, u8>,
    pub id: IdType,
}

/// A map in a variant written under its name.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Hatchway {
    Open {
        #[serde(flatten)]
        counts: BTreeMap<String, u8>,
        note: String,
    },
}

/// A map in a variant under a tag, which serde reads ahead of the variant.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Compartment {
    Open {
        #[serde(flatten)]
        counts: BTreeMap<String, u8>,
        note: String,
    },
}

/// A map in a variant under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum Bulkhead {
    Open {
        #[serde(flatten)]
        counts: BTreeMap<String, u8>,
        note: String,
    },
}

/// A map in an untagged variant.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Scuttle {
    Open {
        #[serde(flatten)]
        counts: BTreeMap<String, u8>,
        note: String,
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
