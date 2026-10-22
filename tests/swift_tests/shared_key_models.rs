//! The models of a struct flattened beside a key the types around it read too: a struct with a
//! flattened member of its own beside a member of its holder's own that reads the same key, and
//! after a struct that reads it; a struct ahead of a member of its holder's own that reads its
//! key, a struct flattened twice around a member of its holder's own, an optional struct beside a
//! member of its holder's own that reads its key, a struct that reads the tag of the variant
//! under a tag alone that flattens it, and an enum under a tag alone whose variant reads a key of
//! its holder's own. serde hands each the object without the keys taken around it, and refuses
//! each payload but the optional struct's, which it reads as `None`.
//! TypeScript and the Zod schema of the holder read each key on every side, and take each
//! payload, the optional struct read where its key is there. Each client reads every payload as
//! Zod reads it, in `service_schema_emitted_client_tests`, and
//! `generic_types_tests/declarations.rs` holds TypeScript and Zod to [`SHARED_KEYS`].

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Each holder's payload, a key a flattened struct reads read by a type around it as well: what
/// serde says of it, its refusal or what it writes back of it, and what the Zod schema of the
/// holder reads of it, which TypeScript takes too and every client reads and writes back.
pub const SHARED_KEYS: [SharedKey; 7] = [
    SharedKey {
        holder: "Bowline",
        json: r#"{"label":"n","tag":"t"}"#,
        serde: "missing field `label` at line 1 column 23",
        told: told_of::<Bowline>,
        zod: r#"{"label":"n","tag":"t"}"#,
    },
    SharedKey {
        holder: "Clinching",
        json: r#"{"label":"n","tag":"t"}"#,
        serde: "missing field `label` at line 1 column 23",
        told: told_of::<Clinching>,
        zod: r#"{"label":"n","tag":"t"}"#,
    },
    SharedKey {
        holder: "Parrel",
        json: r#"{"label":"n"}"#,
        serde: "missing field `label` at line 1 column 13",
        told: told_of::<Parrel>,
        zod: r#"{"label":"n"}"#,
    },
    SharedKey {
        holder: "Gudgeon",
        json: r#"{"label":"n","note":"a"}"#,
        serde: "missing field `label` at line 1 column 24",
        told: told_of::<Gudgeon>,
        zod: r#"{"label":"n","note":"a"}"#,
    },
    SharedKey {
        holder: "Rubrail",
        json: r#"{"label":"n"}"#,
        serde: r#"{"label":"n"}"#,
        told: told_of::<Rubrail>,
        zod: r#"{"label":"n"}"#,
    },
    SharedKey {
        holder: "Halliard",
        json: r#"{"kind":"Held"}"#,
        serde: "missing field `kind`",
        told: told_of::<Halliard>,
        zod: r#"{"kind":"Held"}"#,
    },
    SharedKey {
        holder: "Bitts",
        json: r#"{"label":"n","type":"Held"}"#,
        serde: "missing field `label` at line 1 column 27",
        told: told_of::<Bitts>,
        zod: r#"{"label":"n","type":"Held"}"#,
    },
];

/// A payload of a holder below, what serde says of it, and what the Zod schema of the holder
/// reads of it.
pub struct SharedKey {
    pub holder: &'static str,
    pub json: &'static str,
    /// What serde answers of the payload: its refusal, or what it writes back.
    pub serde: &'static str,
    pub told: fn(&str) -> String,
    /// What the Zod schema of the holder reads of the payload, its keys sorted.
    pub zod: &'static str,
}

/// The struct [`Spritsail`] flattens.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Earring {
    pub tag: String,
}

/// A struct with a flattened member of its own, which reads the key [`Bowline`]'s own member and
/// [`Gantline`] read.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Spritsail {
    #[serde(flatten)]
    pub earring: Earring,
    pub label: String,
}

/// [`Spritsail`] flattened beside a member of its holder's own that reads the same key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Bowline {
    pub label: String,
    #[serde(flatten)]
    pub spritsail: Spritsail,
}

/// The struct [`Clinching`] flattens ahead of [`Spritsail`], which takes the key both read.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Gantline {
    pub label: String,
}

/// [`Spritsail`] flattened after a struct that reads the same key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Clinching {
    #[serde(flatten)]
    pub gantline: Gantline,
    #[serde(flatten)]
    pub spritsail: Spritsail,
}

/// The struct the holders below flatten, with no flattened member of its own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Garnet {
    pub label: String,
}

/// [`Garnet`] flattened ahead of a member of its holder's own that reads the same key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Parrel {
    #[serde(flatten)]
    pub flag: Garnet,
    pub label: String,
}

/// [`Garnet`] flattened twice, around a member of its holder's own: the second reads the key the
/// first took.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Gudgeon {
    #[serde(flatten)]
    pub flag: Garnet,
    pub note: String,
    #[serde(flatten)]
    pub twice: Garnet,
}

/// [`Garnet`] optional, beside a member of its holder's own that reads the same key: the Zod
/// schema of the holder tries the branch that holds it first, which reads that key, where serde
/// reads `None`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Rubrail {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub flag: Option<Garnet>,
    pub label: String,
}

/// A struct that reads a key named as the tag of [`Halliard`].
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Toggled {
    pub kind: String,
}

/// [`Toggled`] flattened into a variant of no member of its own under a tag alone, which serde
/// hands the object without the tag.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Halliard {
    Held {
        #[serde(flatten)]
        flag: Toggled,
    },
}

/// An enum under a tag alone whose variant reads a key [`Bitts`]'s own member reads too.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type")]
pub enum Snotter {
    Held { label: String },
}

/// [`Snotter`] flattened beside a member of its holder's own that reads the key its variant
/// reads.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Bitts {
    pub label: String,
    #[serde(flatten)]
    pub snotter: Snotter,
}

/// What serde says of `json` read as a `T`: its refusal, or what it writes back.
fn told_of<T>(json: &str) -> String
where
    T: DeserializeOwned + Serialize,
{
    serde_json::from_str::<T>(json).map_or_else(
        |refused| refused.to_string(),
        |read| serde_json::to_string(&read).unwrap(),
    )
}

/// serde answers each payload of [`SHARED_KEYS`] as the row states: the type around each struct
/// takes the key, the struct is handed the object without it and is refused, and the optional
/// one is read as `None`.
#[test]
fn serde_hands_a_struct_the_object_without_a_key_taken_around_it() {
    for row in SHARED_KEYS {
        assert_eq!(
            (row.told)(row.json),
            row.serde,
            "serde no longer answers `{}` as {} as the row states",
            row.json,
            row.holder
        );
    }
}
