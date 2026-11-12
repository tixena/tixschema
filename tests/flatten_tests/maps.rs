//! The models a map is flattened into as the only member of its object, and the payloads serde
//! reads and refuses for them. A map flattened beside another member or beside a tag stops the
//! build, so a map is handed every key of its object. An `Option` of a map flattened stops the
//! build as well: serde would read back as `Some` of a map of no entry the `None` it wrote.

use alloc::collections::BTreeMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Every payload, one that serde reads and one with a count of the wrong type for each place a
/// map is flattened.
pub const PAYLOADS: [Payload; 15] = [
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
        json: r#"{"n":2}"#,
        name: "keyed by a char",
        read_as: "Ratlines",
    },
    Payload {
        json: r#"{"nn":2}"#,
        name: "keyed by a char, a key of two chars",
        read_as: "Ratlines",
    },
    Payload {
        json: r#"{"n":"x"}"#,
        name: "keyed by a char, a count of text",
        read_as: "Ratlines",
    },
];

/// Each model a payload is read as, beside serde's read of a payload as it.
const READS: [(&str, ReadAs); 7] = [
    ("Thwart", reads::<Thwart>),
    ("Gunwale", reads::<Gunwale>),
    ("Strake", reads::<Strake>),
    ("Coaming", reads::<Coaming>),
    ("Transom", reads::<Transom>),
    ("Bilge", reads::<Bilge>),
    ("Ratlines", reads::<Ratlines>),
];

/// What Zod prints for [`PAYLOADS`]: a payload read, or refused with each issue under its place.
pub const PRINTED: &str = "\
alone: read
alone, a count of text: refused
    Invalid input: expected number, received string
beside a member serde skips: read
beside a member serde skips, a count of text: refused
    Invalid input: expected number, received string
keyed by a brand: read
keyed by a brand, a count of text: refused
    Invalid input: expected number, received string
in a variant: read
in a variant, a count of text: refused
    'Open': Invalid input: expected number, received string
in a variant under a content key: read
in a variant under a content key, a count of text: refused
    'held': Invalid input: expected number, received string
in an untagged variant: read
in an untagged variant, a count of text: refused
    Invalid input: expected number, received string
keyed by a char: read
keyed by a char, a key of two chars: refused
    Unrecognized key: \"nn\"
keyed by a char, a count of text: refused
    Invalid input: expected number, received string
";

/// Each payload of [`Hawser`]: its name, the JSON, and what serde reads of it. serde reads every
/// one, the struct as `None` where its key does not read.
pub const OPTIONAL_STRUCT_PAYLOADS: [(&str, &str, &str); 3] = [
    (
        "an optional struct, no key",
        "{}",
        "Hawser { burgee: None }",
    ),
    (
        "an optional struct, a label",
        r#"{"label":"a"}"#,
        r#"Hawser { burgee: Some(Burgee { label: "a" }) }"#,
    ),
    (
        "an optional struct, a label of a number",
        r#"{"label":3}"#,
        "Hawser { burgee: None }",
    ),
];

/// What Zod prints for [`OPTIONAL_STRUCT_PAYLOADS`]: it refuses the key serde drops.
pub const OPTIONAL_STRUCT_PRINTED: &str = "\
an optional struct, no key: read
an optional struct, a label: read
an optional struct, a label of a number: refused
    Unrecognized key: \"label\"
";

/// A map that is the only member of its object.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Thwart {
    #[serde(flatten)]
    pub counts: BTreeMap<String, u8>,
}

/// A map beside a member serde neither writes nor reads.
#[model_schema(enable = ["jsonschema"])]
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
pub struct StrakeCode(String);

/// A map keyed by a brand.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Strake {
    #[serde(flatten)]
    pub counts: BTreeMap<StrakeCode, u8>,
}

/// A map in a variant written under its name.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Coaming {
    Open {
        #[serde(flatten)]
        counts: BTreeMap<String, u8>,
    },
}

/// A map in a variant under a tag and a content key.
#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum Transom {
    Open {
        #[serde(flatten)]
        counts: BTreeMap<String, u8>,
    },
}

/// A map in an untagged variant.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Bilge {
    Open {
        #[serde(flatten)]
        counts: BTreeMap<String, u8>,
    },
}

/// A map keyed by a `char`, which serde reads from the text of each key.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Ratlines {
    #[serde(flatten)]
    pub turns: BTreeMap<char, u8>,
}

/// The struct the README flattens through an `Option`.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Burgee {
    pub label: String,
}

/// An optional struct alone in its object: serde reads it as `None` wherever its key does not
/// read, and drops the key.
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Hawser {
    #[serde(default, flatten, skip_serializing_if = "Option::is_none")]
    pub burgee: Option<Burgee>,
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

/// Holds serde's read of each of [`OPTIONAL_STRUCT_PAYLOADS`] to the value it states, and the
/// verdict [`OPTIONAL_STRUCT_PRINTED`] states for it: Zod refuses exactly the payloads serde does
/// not write back as they were, the key it dropped.
pub fn assert_zod_refuses_each_key_serde_drops() {
    for (name, json, read) in OPTIONAL_STRUCT_PAYLOADS {
        let value: Hawser = serde_json::from_str(json).unwrap();
        assert_eq!(format!("{value:?}"), read, "serde on `{name}`");
        assert_eq!(
            OPTIONAL_STRUCT_PRINTED
                .lines()
                .any(|line| *line == format!("{name}: refused")),
            serde_json::to_string(&value).unwrap() != json,
            "serde and Zod part on `{name}`: serde writes {value:?} back"
        );
    }
}
