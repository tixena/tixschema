//! Maps keyed by a value serde writes as text of its own, held under a key and flattened, and the
//! payloads serde reads and refuses for them. Each is read by the JSON Schema, and by the Zod
//! schema under `node`, as serde reads it, but where serde and Zod part on the text of a number.

use alloc::collections::BTreeMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Every payload, with whether serde parts from Zod on it: one each model reads, and one that
/// breaks its key or its value in each map.
pub const PAYLOADS: [Payload; 25] = [
    Payload {
        json: r#"{"counts":{"7":2},"flags":{"true":2},"letters":{"n":2},"turns":{"t":2}}"#,
        name: "each map",
        parted: false,
        read_as: "Marlinspike",
        widened: false,
    },
    Payload {
        json: r#"{"counts":{"7":"x"},"flags":{},"letters":{},"turns":{}}"#,
        name: "a number key, a count of text",
        parted: false,
        read_as: "Marlinspike",
        widened: false,
    },
    Payload {
        json: r#"{"counts":{"x":2},"flags":{},"letters":{},"turns":{}}"#,
        name: "a number key of text",
        parted: false,
        read_as: "Marlinspike",
        widened: false,
    },
    Payload {
        json: r#"{"counts":{"7.5":2},"flags":{},"letters":{},"turns":{}}"#,
        name: "a number key with a fraction",
        parted: false,
        read_as: "Marlinspike",
        widened: false,
    },
    Payload {
        json: r#"{"counts":{"+7":2},"flags":{},"letters":{},"turns":{}}"#,
        name: "a number key under a plus",
        parted: false,
        read_as: "Marlinspike",
        widened: false,
    },
    Payload {
        json: r#"{"counts":{"1e2":2},"flags":{},"letters":{},"turns":{}}"#,
        name: "a number key with an exponent",
        parted: false,
        read_as: "Marlinspike",
        widened: false,
    },
    Payload {
        json: r#"{"counts":{"07":2},"flags":{},"letters":{},"turns":{}}"#,
        name: "a number key with a leading zero",
        parted: true,
        read_as: "Marlinspike",
        widened: false,
    },
    Payload {
        json: r#"{"counts":{"7.0":2},"flags":{},"letters":{},"turns":{}}"#,
        name: "a number key with a fraction of zeros",
        parted: true,
        read_as: "Marlinspike",
        widened: false,
    },
    Payload {
        json: r#"{"counts":{"-7":2},"flags":{},"letters":{},"turns":{}}"#,
        name: "a number key below the range of a u32",
        parted: false,
        read_as: "Marlinspike",
        widened: true,
    },
    Payload {
        json: r#"{"counts":{},"flags":{"true":"x"},"letters":{},"turns":{}}"#,
        name: "a bool key, a count of text",
        parted: false,
        read_as: "Marlinspike",
        widened: false,
    },
    Payload {
        json: r#"{"counts":{},"flags":{"yes":2},"letters":{},"turns":{}}"#,
        name: "a bool key of other text",
        parted: false,
        read_as: "Marlinspike",
        widened: false,
    },
    Payload {
        json: r#"{"counts":{},"flags":{},"letters":{"n":"x"},"turns":{}}"#,
        name: "a char key, a count of text",
        parted: false,
        read_as: "Marlinspike",
        widened: false,
    },
    Payload {
        json: r#"{"counts":{},"flags":{},"letters":{"nn":2},"turns":{}}"#,
        name: "a char key of two chars",
        parted: false,
        read_as: "Marlinspike",
        widened: false,
    },
    Payload {
        json: r#"{"counts":{},"flags":{},"letters":{},"turns":{"tt":2}}"#,
        name: "a key of a brand over a char, of two chars",
        parted: false,
        read_as: "Marlinspike",
        widened: false,
    },
    Payload {
        json: r#"{"t":2}"#,
        name: "flattened, a key of a brand over a char",
        parted: false,
        read_as: "Trunnel",
        widened: false,
    },
    Payload {
        json: r#"{"tt":2}"#,
        name: "flattened, a key of a brand over a char, of two chars",
        parted: false,
        read_as: "Trunnel",
        widened: false,
    },
    Payload {
        json: r#"{"t":"x"}"#,
        name: "flattened, a key of a brand over a char, a count of text",
        parted: false,
        read_as: "Trunnel",
        widened: false,
    },
    Payload {
        json: r#"{"kind":"Lines","body":{"depths":{"7":2}}}"#,
        name: "under a tag and a content key",
        parted: false,
        read_as: "Scuttle",
        widened: false,
    },
    Payload {
        json: r#"{"kind":"Lines","body":{"depths":{"x":2}}}"#,
        name: "under a tag and a content key, a key of text",
        parted: false,
        read_as: "Scuttle",
        widened: false,
    },
    Payload {
        json: r#"{"body":{"depths":{"7":2}},"kind":"Lines"}"#,
        name: "under a tag and a content key, the content first",
        parted: true,
        read_as: "Scuttle",
        widened: false,
    },
    Payload {
        json: r#"{"Lines":{"depths":{"7":2}}}"#,
        name: "under its name",
        parted: false,
        read_as: "Baggywrinkle",
        widened: false,
    },
    Payload {
        json: r#"{"Lines":{"depths":{"x":2}}}"#,
        name: "under its name, a key of text",
        parted: false,
        read_as: "Baggywrinkle",
        widened: false,
    },
    Payload {
        json: r#"{"days":{"2026-10-21":2}}"#,
        name: "a date key",
        parted: false,
        read_as: "Gudgeon",
        widened: false,
    },
    Payload {
        json: r#"{"days":{"x":2}}"#,
        name: "a date key of other text",
        parted: false,
        read_as: "Gudgeon",
        widened: false,
    },
    Payload {
        json: r#"{"days":{"2026-10-21":"x"}}"#,
        name: "a date key, a count of text",
        parted: false,
        read_as: "Gudgeon",
        widened: false,
    },
];

/// What Zod prints for [`PAYLOADS`]: a payload read, or refused with each issue under its place.
pub const PRINTED: &str = "\
each map: read
a number key, a count of text: refused
    'counts.7': Invalid input: expected number, received string
a number key of text: refused
    'counts.x': Invalid key in record
a number key with a fraction: refused
    'counts.7.5': Invalid key in record
a number key under a plus: refused
    'counts.+7': Invalid key in record
a number key with an exponent: refused
    'counts.1e2': Invalid key in record
a number key with a leading zero: read
a number key with a fraction of zeros: read
a number key below the range of a u32: refused
    'counts.-7': Invalid key in record
a bool key, a count of text: refused
    'flags.true': Invalid input: expected number, received string
a bool key of other text: refused
    'flags': Unrecognized key: \"yes\"
a char key, a count of text: refused
    'letters.n': Invalid input: expected number, received string
a char key of two chars: refused
    'letters.nn': Invalid key in record
a key of a brand over a char, of two chars: refused
    'turns.tt': Invalid key in record
flattened, a key of a brand over a char: read
flattened, a key of a brand over a char, of two chars: refused
    Unrecognized key: \"tt\"
flattened, a key of a brand over a char, a count of text: refused
    't': Invalid input: expected number, received string
under a tag and a content key: read
under a tag and a content key, a key of text: refused
    'body.depths.x': Invalid key in record
under a tag and a content key, the content first: read
under its name: read
under its name, a key of text: refused
    'Lines.depths.x': Invalid key in record
a date key: read
a date key of other text: refused
    'days.x': Invalid key in record
a date key, a count of text: refused
    'days.2026-10-21': Invalid input: expected number, received string
";

/// Each model a payload is read as, beside serde's read of a payload as it.
const READS: [(&str, ReadAs); 5] = [
    ("Marlinspike", reads::<Marlinspike>),
    ("Trunnel", reads::<Trunnel>),
    ("Scuttle", reads::<Scuttle>),
    ("Baggywrinkle", reads::<Baggywrinkle>),
    ("Gudgeon", reads::<Gudgeon>),
];

/// An enum under its name with a map keyed by a number, which serde reads with no copy.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Baggywrinkle {
    Lines { depths: BTreeMap<u32, u8> },
}

/// A brand over a `char`, which serde reads from text of one character.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct Futtock(pub char);

/// A map keyed by a date under a key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Gudgeon {
    pub days: BTreeMap<chrono::NaiveDate, u8>,
}

/// Maps under a key, keyed by a number, a `bool`, a `char` and a brand over a `char`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Marlinspike {
    pub counts: BTreeMap<u32, u8>,
    pub flags: BTreeMap<bool, u8>,
    pub letters: BTreeMap<char, u8>,
    pub turns: BTreeMap<Futtock, u8>,
}

/// One payload, the model it is read as, and whether serde refuses it where Zod reads it.
pub struct Payload {
    pub json: &'static str,
    pub name: &'static str,
    /// serde refuses the payload, which Zod and the JSON Schema read: the text of a number key
    /// Zod reads as the number, or an enum's content ahead of its tag, which serde buffers.
    pub parted: bool,
    pub read_as: &'static str,
    /// The JSON Schema reads the payload, which serde and Zod refuse: a number key below the
    /// range of an unsigned key, which the pattern of a number key does not bound.
    pub widened: bool,
}

/// An enum under a tag and a content key with a map keyed by a number, which serde reads where
/// the tag comes first, as it writes it.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "body")]
pub enum Scuttle {
    Lines { depths: BTreeMap<u32, u8> },
}

/// A map keyed by a brand over a `char`, flattened alone into its object.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Trunnel {
    #[serde(flatten)]
    pub turns: BTreeMap<Futtock, u8>,
}

/// Whether serde reads a payload as one model.
type ReadAs = fn(&str) -> bool;

/// Whether serde reads `json` as a `T`.
fn reads<T>(json: &str) -> bool
where
    T: DeserializeOwned,
{
    serde_json::from_str::<T>(json).is_ok()
}

/// Whether serde reads `payload` as the model it names.
pub fn serde_reads(payload: &Payload) -> bool {
    let (_, read) = READS
        .iter()
        .find(|(model, _)| *model == payload.read_as)
        .unwrap();
    read(payload.json)
}

/// Whether Zod reads `payload`, as [`PRINTED`] states.
pub fn zod_reads(payload: &Payload) -> bool {
    let read = format!("{}: read", payload.name);
    let refused = format!("{}: refused", payload.name);
    PRINTED
        .lines()
        .find(|line| *line == read || *line == refused)
        .unwrap()
        == read
}

/// Holds serde's read of each of [`PAYLOADS`] to the verdict [`PRINTED`] states for it: Zod reads
/// exactly the payloads serde reads, but those serde parts from Zod on.
pub fn assert_serde_reads_what_zod_reads() {
    for payload in &PAYLOADS {
        assert_eq!(
            serde_reads(payload),
            zod_reads(payload) && !payload.parted,
            "serde and Zod part on `{}`",
            payload.name
        );
        assert!(
            !payload.parted || zod_reads(payload),
            "Zod refuses `{}`, which is marked as parted",
            payload.name
        );
        assert!(
            !payload.widened || !(zod_reads(payload) || payload.parted),
            "Zod reads `{}`, which is marked as read by the JSON Schema alone",
            payload.name
        );
    }
}
