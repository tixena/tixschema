//! A member serde reads and never writes, `#[serde(default, skip_serializing)]`: in a struct of
//! its own, beside a flattened member, and in an enum's variant under each tagging that writes a
//! variant's named members. Each client reads the payloads below and writes them back as serde
//! does, without the member.

use core::fmt::Debug;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// The payload each struct is read from: the member serde never writes, held.
pub const PAYLOAD: &str = r#"{"label":"north","spare":3}"#;

/// What serde writes back of [`PAYLOAD`], read as either struct.
pub const WRITTEN: &str = r#"{"label":"north"}"#;

/// Each enum below, the payload its variant `Furled` is read from, the member serde never writes
/// held, and what serde writes back of it.
pub const VARIANTS: [(&str, &str, &str); 4] = [
    (
        "Dousing",
        r#"{"Furled":{"label":"north","spare":3}}"#,
        r#"{"Furled":{"label":"north"}}"#,
    ),
    (
        "Wearing",
        r#"{"kind":"Furled","label":"north","spare":3}"#,
        r#"{"kind":"Furled","label":"north"}"#,
    ),
    (
        "Shackling",
        r#"{"kind":"Furled","load":{"label":"north","spare":3}}"#,
        r#"{"kind":"Furled","load":{"label":"north"}}"#,
    ),
    ("Ratcheting", PAYLOAD, WRITTEN),
];

/// A member serde reads and never writes.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Furling {
    pub label: String,
    #[serde(default, skip_serializing)]
    pub spare: Option<u32>,
}

/// The member [`Reefing`] flattens.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Braiding {
    pub label: String,
}

/// [`Furling`]'s member beside a flattened one.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Reefing {
    #[serde(flatten)]
    pub braiding: Braiding,
    #[serde(default, skip_serializing)]
    pub spare: Option<u32>,
}

/// [`Furling`]'s member in a variant, with no tag attribute, beside a variant with no such member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Dousing {
    Furled {
        label: String,
        #[serde(default, skip_serializing)]
        spare: Option<u32>,
    },
    Struck {
        label: String,
    },
}

/// [`Furling`]'s member in a variant under a tag alone, beside a variant with no such member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Wearing {
    Furled {
        label: String,
        #[serde(default, skip_serializing)]
        spare: Option<u32>,
    },
    Struck {
        label: String,
    },
}

/// [`Furling`]'s member in a variant under a tag and a content key, beside a variant with no such
/// member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "load")]
pub enum Shackling {
    Furled {
        label: String,
        #[serde(default, skip_serializing)]
        spare: Option<u32>,
    },
    Struck {
        label: String,
    },
}

/// [`Furling`]'s member in a variant of an untagged enum, beside a variant with no such member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Ratcheting {
    Furled {
        label: String,
        #[serde(default, skip_serializing)]
        spare: Option<u32>,
    },
    Struck {
        label: String,
    },
}

/// What serde writes back of `payload`, which it reads as `held`.
fn written_back<Read>(payload: &str, held: &Read) -> String
where
    Read: Debug + DeserializeOwned + PartialEq + Serialize,
{
    let read: Read = serde_json::from_str(payload).unwrap();
    assert_eq!(&read, held, "{payload} is read with the member held");
    serde_json::to_string(&read).unwrap()
}

/// serde reads the member from each payload of [`VARIANTS`], in the variant `Furled`, and writes
/// back what the table says.
#[test]
fn serde_reads_the_member_of_a_variant_and_never_writes_it() {
    let label = || "north".to_owned();
    let serde_wrote = [
        written_back(
            VARIANTS[0].1,
            &Dousing::Furled {
                label: label(),
                spare: Some(3),
            },
        ),
        written_back(
            VARIANTS[1].1,
            &Wearing::Furled {
                label: label(),
                spare: Some(3),
            },
        ),
        written_back(
            VARIANTS[2].1,
            &Shackling::Furled {
                label: label(),
                spare: Some(3),
            },
        ),
        written_back(
            VARIANTS[3].1,
            &Ratcheting::Furled {
                label: label(),
                spare: Some(3),
            },
        ),
    ];
    assert_eq!(
        serde_wrote,
        VARIANTS.map(|(_, _, written)| written.to_owned()),
        "serde writes back no member it never writes"
    );
}

/// serde reads the member from [`PAYLOAD`] and writes [`WRITTEN`] back, in either struct.
#[test]
fn serde_reads_the_member_and_never_writes_it() {
    let furling: Furling = serde_json::from_str(PAYLOAD).unwrap();
    let reefing: Reefing = serde_json::from_str(PAYLOAD).unwrap();
    assert_eq!(
        furling,
        Furling {
            label: "north".to_owned(),
            spare: Some(3),
        }
    );
    assert_eq!(
        reefing,
        Reefing {
            braiding: Braiding {
                label: "north".to_owned(),
            },
            spare: Some(3),
        }
    );
    assert_eq!(serde_json::to_string(&furling).unwrap(), WRITTEN);
    assert_eq!(serde_json::to_string(&reefing).unwrap(), WRITTEN);
}
