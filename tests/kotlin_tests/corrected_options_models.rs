//! The models of an optional flattened member that names a function: a struct the Kotlin emitter
//! recorded keys for, and an enum under a tag alone, whose serializer is written by hand and
//! declares no key, each read from what a function that answers the payload as it was handed
//! answers, and the struct under a function that answers `null`. serde hands the function the
//! object without its holder's own keys, reads `None` where it answers `null`, and refuses an
//! answer the type does not read. The Zod schema of the holder hands the function the whole object
//! and reads the type from what it answers, and where that read fails takes an object that holds
//! no key beyond its holder's own, with the type absent. `kotlin_tests/tests.rs` pins the Kotlin
//! text and `swift_tests/flattened_options.rs` the Swift text, `run_kotlin_flattened.rs`,
//! `run_swift_flattened_options.rs` and `run_dart_flattened_options.rs` of
//! `service_schema_emitted_client_tests` hold the Kotlin, the Swift and the Dart read to Zod's, the
//! last pinning the Dart text too, and `generic_types_tests/declarations.rs` holds TypeScript and
//! Zod to [`CORRECTED`].

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::model_schema;

/// Each holder's payloads: the type held, no key of it, and a key of it that does not read or
/// beside a key it reads that is absent. What serde reads of each, written back, and what Zod reads
/// of it, its keys in order, are stated beside whether TypeScript takes it.
pub const CORRECTED: [Corrected; 10] = [
    Corrected {
        holder: "Reeved",
        json: r#"{"label":"north","note":"n"}"#,
        read_back: read_of::<Reeved>,
        serde: r#"{"note":"n","label":"north"}"#,
        typed: true,
        zod: r#"{"label":"north","note":"n"}"#,
    },
    Corrected {
        holder: "Reeved",
        json: r#"{"note":"n"}"#,
        read_back: read_of::<Reeved>,
        serde: "refused: missing field `label` at line 1 column 12",
        typed: true,
        zod: r#"{"note":"n"}"#,
    },
    Corrected {
        holder: "Reeved",
        json: r#"{"label":5,"note":"n"}"#,
        read_back: read_of::<Reeved>,
        serde: "refused: invalid type: integer `5`, expected a string at line 1 column 22",
        typed: false,
        zod: "refused",
    },
    Corrected {
        holder: "Moused",
        json: r#"{"kind":"Bay","label":"n","note":"a"}"#,
        read_back: read_of::<Moused>,
        serde: r#"{"kind":"Bay","label":"n","note":"a"}"#,
        typed: true,
        zod: r#"{"kind":"Bay","label":"n","note":"a"}"#,
    },
    Corrected {
        holder: "Moused",
        json: r#"{"note":"a"}"#,
        read_back: read_of::<Moused>,
        serde: "refused: missing field `kind` at line 1 column 12",
        typed: true,
        zod: r#"{"note":"a"}"#,
    },
    Corrected {
        holder: "Moused",
        json: r#"{"kind":"Bay","note":"a"}"#,
        read_back: read_of::<Moused>,
        serde: "refused: missing field `label` at line 1 column 25",
        typed: false,
        zod: "refused",
    },
    Corrected {
        holder: "Moused",
        json: r#"{"label":"n","note":"a"}"#,
        read_back: read_of::<Moused>,
        serde: "refused: missing field `kind` at line 1 column 24",
        typed: false,
        zod: "refused",
    },
    Corrected {
        holder: "Wormed",
        json: r#"{"label":"north","note":"n"}"#,
        read_back: read_of::<Wormed>,
        serde: r#"{"note":"n"}"#,
        typed: true,
        zod: "refused",
    },
    Corrected {
        holder: "Wormed",
        json: r#"{"note":"n"}"#,
        read_back: read_of::<Wormed>,
        serde: r#"{"note":"n"}"#,
        typed: true,
        zod: r#"{"note":"n"}"#,
    },
    Corrected {
        holder: "Wormed",
        json: r#"{"label":5,"note":"n"}"#,
        read_back: read_of::<Wormed>,
        serde: r#"{"note":"n"}"#,
        typed: false,
        zod: "refused",
    },
];

/// A payload of a holder below, what serde reads of it, whether TypeScript takes it as the
/// holder's type, and what the Zod schema of the holder reads of it.
pub struct Corrected {
    pub holder: &'static str,
    pub json: &'static str,
    pub read_back: fn(&str) -> String,
    pub serde: &'static str,
    pub typed: bool,
    pub zod: &'static str,
}

/// The struct [`Reeved`] and [`Wormed`] flatten.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Tholepin {
    pub label: String,
}

/// [`Tholepin`] optional, read from what a function answers, beside a member of its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Reeved {
    pub note: String,
    #[model_schema_prop(preprocess = ["asGiven"])]
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub tholepin: Option<Tholepin>,
}

/// [`Tholepin`] optional, under a function that answers `null`, which serde reads as `None`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Wormed {
    pub note: String,
    #[model_schema_prop(preprocess = ["asNull"])]
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub tholepin: Option<Tholepin>,
}

/// An enum under a tag alone.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Frapped {
    Bay { label: String },
}

/// [`Frapped`] optional, read from what a function answers, ahead of a member of its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Moused {
    #[model_schema_prop(preprocess = ["asGiven"])]
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub frapped: Option<Frapped>,
    pub note: String,
}

/// The payload as it was handed.
const fn as_given(payload: Value) -> Value {
    payload
}

/// `null`, whatever `payload` is.
fn as_null(_payload: Value) -> Value {
    Value::Null
}

/// What serde reads of `json` as a `T`, written back, or its refusal.
fn read_of<T>(json: &str) -> String
where
    T: DeserializeOwned + Serialize,
{
    serde_json::from_str::<T>(json).map_or_else(
        |refused| format!("refused: {refused}"),
        |read| serde_json::to_string(&read).unwrap(),
    )
}

/// serde reads each payload from what its holder's function answers, as [`CORRECTED`] states,
/// and TypeScript, which calls no function, takes every payload the Zod schema takes.
#[test]
fn serde_reads_an_optional_flattened_type_from_what_its_function_answers() {
    for row in CORRECTED {
        assert_eq!(
            (row.read_back)(row.json),
            row.serde,
            "serde no longer reads `{}` as {} did",
            row.json,
            row.holder
        );
        assert!(
            row.typed || row.zod == "refused",
            "Zod takes `{}` as {}, which TypeScript is stated to refuse",
            row.json,
            row.holder
        );
    }
}
