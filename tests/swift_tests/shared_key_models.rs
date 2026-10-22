//! The models of a struct with a flattened member of its own that reads a key the types around it
//! took: flattened beside a member of its holder's own that reads the same key, and after a struct
//! that reads it. serde hands the struct the object without that key and refuses the payload.
//! TypeScript and the Zod schema of the holder read the key on both sides, and take it.
//! `run_swift_flattened.rs` of `service_schema_emitted_client_tests` holds the Swift read to
//! Zod's, and `generic_types_tests/declarations.rs` holds TypeScript and Zod to [`SHARED_KEYS`].

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Each holder's payload, the key its struct with a flattened member reads read by a type around
/// it as well: what serde says of it, and what the Zod schema of the holder reads of it, its keys
/// in order, which TypeScript takes too.
pub const SHARED_KEYS: [SharedKey; 2] = [
    SharedKey {
        holder: "Bowline",
        json: r#"{"label":"n","tag":"t"}"#,
        refused: "missing field `label` at line 1 column 23",
        serde: refused_of::<Bowline>,
        zod: r#"{"label":"n","tag":"t"}"#,
    },
    SharedKey {
        holder: "Clinching",
        json: r#"{"label":"n","tag":"t"}"#,
        refused: "missing field `label` at line 1 column 23",
        serde: refused_of::<Clinching>,
        zod: r#"{"label":"n","tag":"t"}"#,
    },
];

/// A payload of a holder below, what serde says of it, and what the Zod schema of the holder
/// reads of it.
pub struct SharedKey {
    pub holder: &'static str,
    pub json: &'static str,
    pub refused: &'static str,
    pub serde: fn(&str) -> String,
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

/// What serde says of `json` read as a `T`, which it is expected to refuse.
fn refused_of<T>(json: &str) -> String
where
    T: DeserializeOwned,
{
    serde_json::from_str::<T>(json)
        .err()
        .map_or_else(|| "read".to_owned(), |refused| refused.to_string())
}

/// serde refuses each payload of [`SHARED_KEYS`]: the type around the struct takes the key, and
/// the struct is handed the object without it.
#[test]
fn serde_refuses_a_key_taken_around_a_struct_with_a_flattened_member() {
    for row in SHARED_KEYS {
        assert_eq!(
            (row.serde)(row.json),
            row.refused,
            "serde no longer refuses `{}` as {}",
            row.json,
            row.holder
        );
    }
}
