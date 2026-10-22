//! The models a map is flattened into as the only member of its object, and the payloads serde
//! reads and refuses for them. A map flattened beside another member stops the build, so a map is
//! handed every key of its object: its holder's own tag too, and none of the tag an internally
//! tagged enum reads ahead of the variant.

use alloc::collections::BTreeMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Every payload, one that serde reads and one with a count of the wrong type for each place a
/// map is flattened.
pub const PAYLOADS: [Payload; 16] = [
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
];

/// Each model a payload is read as, beside serde's read of a payload as it.
const READS: [(&str, ReadAs); 8] = [
    ("Thwart", reads::<Thwart>),
    ("TaggedThwart", reads::<TaggedThwart>),
    ("Gunwale", reads::<Gunwale>),
    ("Strake", reads::<Strake>),
    ("Coaming", reads::<Coaming>),
    ("Keelson", reads::<Keelson>),
    ("Transom", reads::<Transom>),
    ("Bilge", reads::<Bilge>),
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
