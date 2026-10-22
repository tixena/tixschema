//! A member serde reads and never writes, `#[serde(default, skip_serializing)]`: in a struct of
//! its own, beside a flattened member, flattened itself, and in an enum's variant under each
//! tagging that writes a variant's named members. Each client reads the payloads below and writes
//! them back as serde does, without the member.

use alloc::collections::BTreeMap;
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

/// Each struct below with a flattened member serde reads and never writes, the payload it is read
/// from, and what serde writes back of it: none of what the member holds.
pub const FLATTENED: [(&str, &str, &str); 3] = [
    (
        "Flemishing",
        r#"{"label":"north","note":"n"}"#,
        r#"{"note":"n"}"#,
    ),
    (
        "Splaying",
        r#"{"label":"north","note":"n"}"#,
        r#"{"note":"n"}"#,
    ),
    ("Ranging", r#"{"north":2}"#, "{}"),
];

/// A payload serde refuses to read as each struct below with a flattened member that is no
/// `Option`: it reads the member as it reads any flattened member, its `default` aside.
pub const REFUSED: [(&str, &str); 2] = [
    ("Splaying", r#"{"note":"n"}"#),
    ("Ranging", r#"{"north":"x"}"#),
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
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
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

/// A flattened struct serde reads and never writes, beside a member of its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Flemishing {
    #[serde(flatten, default, skip_serializing)]
    pub braiding: Option<Braiding>,
    pub note: String,
}

/// A flattened struct that is no `Option`, serde reads and never writes, beside a member of its
/// holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Splaying {
    #[serde(flatten, default, skip_serializing)]
    pub braiding: Braiding,
    pub note: String,
}

/// A flattened map serde reads and never writes, the one member of its object.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Ranging {
    #[serde(flatten, default, skip_serializing)]
    pub counts: BTreeMap<String, u8>,
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

/// serde reads the flattened member of each struct of [`FLATTENED`] from its payload and writes
/// back what the table says.
#[test]
fn serde_reads_a_flattened_member_and_never_writes_it() {
    let flemishing: Flemishing = serde_json::from_str(FLATTENED[0].1).unwrap();
    let splaying: Splaying = serde_json::from_str(FLATTENED[1].1).unwrap();
    let ranging: Ranging = serde_json::from_str(FLATTENED[2].1).unwrap();
    assert_eq!(
        flemishing,
        Flemishing {
            braiding: Some(Braiding {
                label: "north".to_owned(),
            }),
            note: "n".to_owned(),
        },
        "serde reads the flattened struct"
    );
    assert_eq!(
        splaying.braiding,
        Braiding {
            label: "north".to_owned(),
        },
        "serde reads the flattened struct that is no `Option`"
    );
    assert_eq!(
        ranging,
        Ranging {
            counts: BTreeMap::from([("north".to_owned(), 2)]),
        },
        "serde reads the flattened map"
    );
    assert_eq!(
        [
            serde_json::to_string(&flemishing).unwrap(),
            serde_json::to_string(&splaying).unwrap(),
            serde_json::to_string(&ranging).unwrap(),
        ],
        FLATTENED.map(|(_, _, written)| written.to_owned()),
        "serde writes back no flattened member it never writes"
    );
}

/// serde refuses each payload of [`REFUSED`]: a flattened member that is no `Option` is read
/// whatever its `default` says.
#[test]
fn serde_refuses_a_flattened_member_that_does_not_read() {
    assert!(
        serde_json::from_str::<Splaying>(REFUSED[0].1)
            .unwrap_err()
            .to_string()
            .starts_with("missing field `label`"),
        "serde refuses a flattened struct without its keys"
    );
    assert!(
        serde_json::from_str::<Ranging>(REFUSED[1].1)
            .unwrap_err()
            .to_string()
            .starts_with("invalid type: string \"x\", expected u8"),
        "serde refuses a flattened map with an entry that does not read"
    );
}
