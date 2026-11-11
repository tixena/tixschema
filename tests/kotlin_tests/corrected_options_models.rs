//! The models of an optional flattened member that names a function: a struct the Kotlin emitter
//! recorded keys for, and an enum under a tag alone, whose serializer is written by hand and
//! declares no key, each read from what a function that answers the payload as it was handed
//! answers, and the struct under a function that answers `null`, each beside a strict holder and
//! a `nonstrict` one, and a struct flattened with no `Option`. serde hands the function the object
//! without its holder's own keys, reads `None` where it answers `null`, and refuses an answer the
//! type does not read, a key the type does not declare among them. The Zod schema of the holder
//! hands the function the same object and reads the type from what it answers; where that read
//! fails, a strict holder takes an object that holds no key beyond its own, with the type absent,
//! and a `nonstrict` one takes the type absent only where the function answers `null`, its own
//! object closed beside the type, as serde reads it, so [`OPEN_CORRECTED`] reads on every surface
//! as serde reads it.
//! `kotlin_tests/tests.rs` pins the Kotlin text and `swift_tests/flattened_options.rs` the Swift
//! text, `run_kotlin_flattened.rs`, `run_swift_flattened_options.rs` and
//! `run_dart_flattened_options.rs` of `service_schema_emitted_client_tests` hold the Kotlin, the
//! Swift and the Dart read to Zod's, the last pinning the Dart text too, and
//! `generic_types_tests/declarations.rs` holds TypeScript and Zod to [`CORRECTED`] and
//! [`OPEN_CORRECTED`].

use alloc::collections::BTreeMap;

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

/// The payloads of [`CORRECTED`], each read as the `nonstrict` holder of the same member:
/// [`Parcelled`] for `Reeved`, [`Marled`] for `Moused` and [`Served`] for `Wormed`, then a key no
/// member declares beside each, which the function hands a strict type or answers `null` over, and
/// the payloads of [`Snaked`], whose function reads the object without its holder's own keys, and
/// of [`Seized`], which flattens its struct with no `Option`. Zod reads each as serde reads it.
pub const OPEN_CORRECTED: [Corrected; 17] = [
    Corrected {
        holder: "Parcelled",
        json: r#"{"label":"north","note":"n"}"#,
        read_back: read_of::<Parcelled>,
        serde: r#"{"note":"n","label":"north"}"#,
        typed: true,
        zod: r#"{"label":"north","note":"n"}"#,
    },
    Corrected {
        holder: "Parcelled",
        json: r#"{"note":"n"}"#,
        read_back: read_of::<Parcelled>,
        serde: "refused: missing field `label` at line 1 column 12",
        typed: true,
        zod: "refused",
    },
    Corrected {
        holder: "Parcelled",
        json: r#"{"label":5,"note":"n"}"#,
        read_back: read_of::<Parcelled>,
        serde: "refused: invalid type: integer `5`, expected a string at line 1 column 22",
        typed: false,
        zod: "refused",
    },
    Corrected {
        holder: "Marled",
        json: r#"{"kind":"Bay","label":"n","note":"a"}"#,
        read_back: read_of::<Marled>,
        serde: r#"{"kind":"Bay","label":"n","note":"a"}"#,
        typed: true,
        zod: r#"{"kind":"Bay","label":"n","note":"a"}"#,
    },
    Corrected {
        holder: "Marled",
        json: r#"{"note":"a"}"#,
        read_back: read_of::<Marled>,
        serde: "refused: missing field `kind` at line 1 column 12",
        typed: true,
        zod: "refused",
    },
    Corrected {
        holder: "Marled",
        json: r#"{"kind":"Bay","note":"a"}"#,
        read_back: read_of::<Marled>,
        serde: "refused: missing field `label` at line 1 column 25",
        typed: false,
        zod: "refused",
    },
    Corrected {
        holder: "Marled",
        json: r#"{"label":"n","note":"a"}"#,
        read_back: read_of::<Marled>,
        serde: "refused: missing field `kind` at line 1 column 24",
        typed: false,
        zod: "refused",
    },
    Corrected {
        holder: "Served",
        json: r#"{"label":"north","note":"n"}"#,
        read_back: read_of::<Served>,
        serde: r#"{"note":"n"}"#,
        typed: true,
        zod: r#"{"note":"n"}"#,
    },
    Corrected {
        holder: "Served",
        json: r#"{"note":"n"}"#,
        read_back: read_of::<Served>,
        serde: r#"{"note":"n"}"#,
        typed: true,
        zod: r#"{"note":"n"}"#,
    },
    Corrected {
        holder: "Served",
        json: r#"{"label":5,"note":"n"}"#,
        read_back: read_of::<Served>,
        serde: r#"{"note":"n"}"#,
        typed: false,
        zod: r#"{"note":"n"}"#,
    },
    Corrected {
        holder: "Parcelled",
        json: r#"{"label":"north","note":"n","stray":1}"#,
        read_back: read_of::<Parcelled>,
        serde: "refused: unknown field `stray`, expected `label` at line 1 column 38",
        typed: false,
        zod: "refused",
    },
    Corrected {
        holder: "Marled",
        json: r#"{"kind":"Bay","label":"n","note":"a","stray":1}"#,
        read_back: read_of::<Marled>,
        serde: "refused: unknown field `stray`, expected `label` at line 1 column 47",
        typed: false,
        zod: "refused",
    },
    Corrected {
        holder: "Served",
        json: r#"{"label":"north","note":"n","stray":1}"#,
        read_back: read_of::<Served>,
        serde: r#"{"note":"n"}"#,
        typed: false,
        zod: r#"{"note":"n"}"#,
    },
    Corrected {
        holder: "Snaked",
        json: r#"{"label":"north","note":"n"}"#,
        read_back: read_of::<Snaked>,
        serde: r#"{"note":"n","label":"north"}"#,
        typed: true,
        zod: r#"{"label":"north","note":"n"}"#,
    },
    Corrected {
        holder: "Snaked",
        json: r#"{"note":"n"}"#,
        read_back: read_of::<Snaked>,
        serde: "refused: missing field `label` at line 1 column 12",
        typed: true,
        zod: "refused",
    },
    Corrected {
        holder: "Seized",
        json: r#"{"label":"north","note":"n"}"#,
        read_back: read_of::<Seized>,
        serde: r#"{"note":"n","label":"north"}"#,
        typed: true,
        zod: r#"{"label":"north","note":"n"}"#,
    },
    Corrected {
        holder: "Seized",
        json: r#"{"label":"north","note":"n","stray":1}"#,
        read_back: read_of::<Seized>,
        serde: "refused: unknown field `stray`, expected `label` at line 1 column 38",
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

/// [`Reeved`] `nonstrict`.
#[model_schema(nonstrict)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Parcelled {
    pub note: String,
    #[model_schema_prop(preprocess = ["asGiven"])]
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub tholepin: Option<Tholepin>,
}

/// [`Wormed`] `nonstrict`: its holder reads past the keys of the `None` serde reads.
#[model_schema(nonstrict)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Served {
    pub note: String,
    #[model_schema_prop(preprocess = ["asNull"])]
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub tholepin: Option<Tholepin>,
}

/// [`Moused`] `nonstrict`.
#[model_schema(nonstrict)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Marled {
    #[model_schema_prop(preprocess = ["asGiven"])]
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub frapped: Option<Frapped>,
    pub note: String,
}

/// [`Tholepin`] optional, under a function that answers `null` where the object it is handed holds
/// `note`: serde hands it the object without its holder's own `note`.
#[model_schema(nonstrict)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Snaked {
    pub note: String,
    #[model_schema_prop(preprocess = ["nullWhereNoted"])]
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub tholepin: Option<Tholepin>,
}

/// [`Tholepin`] flattened with no `Option`, read from what a function answers.
#[model_schema(nonstrict)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Seized {
    pub note: String,
    #[model_schema_prop(preprocess = ["asGiven"])]
    #[serde(flatten)]
    pub tholepin: Tholepin,
}

/// The payload as it was handed.
const fn as_given(payload: Value) -> Value {
    payload
}

/// `null`, whatever `payload` is.
fn as_null(_payload: Value) -> Value {
    Value::Null
}

/// `null` where `payload` holds `note`, and `payload` otherwise.
fn null_where_noted(payload: Value) -> Value {
    if payload.get("note").is_some() {
        Value::Null
    } else {
        payload
    }
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

/// What serde states of a payload, as the Zod read of the run is printed: the value with its keys
/// sorted, or `refused`.
fn as_zod_prints(serde: &str) -> String {
    if serde.starts_with("refused") {
        return "refused".to_owned();
    }
    let sorted: BTreeMap<String, Value> = serde_json::from_str(serde).unwrap();
    serde_json::to_string(&sorted).unwrap()
}

/// serde reads each payload from what its holder's function answers, as [`CORRECTED`] and
/// [`OPEN_CORRECTED`] state. Zod reads each payload of a `nonstrict` holder as serde reads it, and
/// TypeScript, which calls no function, takes every payload the Zod schema of a strict holder
/// takes.
#[test]
fn serde_reads_an_optional_flattened_type_from_what_its_function_answers() {
    for row in OPEN_CORRECTED {
        assert_eq!(
            (row.read_back)(row.json),
            row.serde,
            "serde no longer reads `{}` as {} did",
            row.json,
            row.holder
        );
        assert_eq!(
            as_zod_prints(row.serde),
            row.zod,
            "Zod is stated to read `{}` as {} otherwise than serde",
            row.json,
            row.holder
        );
    }
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
