//! The models of an optional flattened member whose type the Kotlin emitter recorded no struct's
//! keys for and whose serializer is written by hand, its descriptor naming no element: a struct
//! declared below its holder with a flattened member of its own, an enum under a tag alone, one
//! under a tag and a content key, an untagged enum, and an enum under a tag alone that names a
//! function on itself. The emitter looks for the two enums under a tag key that name no function
//! by their tag and content key, and has no key to look for any other by. serde reads each
//! wherever its type reads from the object it is handed, and as `None` wherever it does not.
//! TypeScript and Zod read each holder of [`UNRECORDED`] where serde reads it, and refuse a payload
//! that holds a key of that type where the type does not read, where serde reads `None`.
//! [`Grommeting`] holds a struct declared below it that breaks a rule, which serde refuses.
//! `kotlin_tests/tests.rs` pins the Kotlin text,
//! `service_schema_emitted_client_tests/run_kotlin_flattened.rs` runs it against serde, and
//! `generic_types_tests/declarations.rs` holds TypeScript and Zod to [`UNRECORDED`].

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::model_schema;

/// Each holder's payloads: the type held, no key, and a key the type reads beside a key it does
/// not read or held as a type it does not read, which serde reads as `None`.
pub const UNRECORDED: [Unrecorded; 14] = [
    Unrecorded {
        holder: "Fiddling",
        json: r#"{"label":"north","note":"n"}"#,
        read_back: written_back::<Fiddling>,
        taken_by_zod: true,
    },
    Unrecorded {
        holder: "Fiddling",
        json: "{}",
        read_back: written_back::<Fiddling>,
        taken_by_zod: true,
    },
    Unrecorded {
        holder: "Fiddling",
        json: r#"{"note":"n"}"#,
        read_back: written_back::<Fiddling>,
        taken_by_zod: false,
    },
    Unrecorded {
        holder: "Fiddling",
        json: r#"{"label":5,"note":"n"}"#,
        read_back: written_back::<Fiddling>,
        taken_by_zod: false,
    },
    Unrecorded {
        holder: "Snagging",
        json: r#"{"kind":"Bay","label":"n","note":"a"}"#,
        read_back: written_back::<Snagging>,
        taken_by_zod: true,
    },
    Unrecorded {
        holder: "Snagging",
        json: r#"{"note":"a"}"#,
        read_back: written_back::<Snagging>,
        taken_by_zod: true,
    },
    Unrecorded {
        holder: "Snagging",
        json: r#"{"kind":"Bay","note":"a"}"#,
        read_back: written_back::<Snagging>,
        taken_by_zod: false,
    },
    Unrecorded {
        holder: "Snagging",
        json: r#"{"label":"n","note":"a"}"#,
        read_back: written_back::<Snagging>,
        taken_by_zod: false,
    },
    Unrecorded {
        holder: "Bridling",
        json: r#"{"kind":"Bay","held":{"label":"n"},"note":"a"}"#,
        read_back: written_back::<Bridling>,
        taken_by_zod: true,
    },
    Unrecorded {
        holder: "Bridling",
        json: r#"{"note":"a"}"#,
        read_back: written_back::<Bridling>,
        taken_by_zod: true,
    },
    Unrecorded {
        holder: "Bridling",
        json: r#"{"kind":"Bay","note":"a"}"#,
        read_back: written_back::<Bridling>,
        taken_by_zod: false,
    },
    Unrecorded {
        holder: "Slipping",
        json: r#"{"note":"a","label":"n"}"#,
        read_back: written_back::<Slipping>,
        taken_by_zod: true,
    },
    Unrecorded {
        holder: "Slipping",
        json: r#"{"note":"a"}"#,
        read_back: written_back::<Slipping>,
        taken_by_zod: true,
    },
    Unrecorded {
        holder: "Slipping",
        json: r#"{"label":5,"note":"a"}"#,
        read_back: written_back::<Slipping>,
        taken_by_zod: false,
    },
];

/// What serde writes back of each of [`UNRECORDED`], read as its holder: the type held where it
/// reads, and no key of it where it does not, which is a payload TypeScript and Zod refuse.
pub const READ_BACK: [&str; 14] = [
    r#"{"label":"north","note":"n"}"#,
    "{}",
    "{}",
    "{}",
    r#"{"kind":"Bay","label":"n","note":"a"}"#,
    r#"{"note":"a"}"#,
    r#"{"note":"a"}"#,
    r#"{"note":"a"}"#,
    r#"{"kind":"Bay","held":{"label":"n"},"note":"a"}"#,
    r#"{"note":"a"}"#,
    r#"{"note":"a"}"#,
    r#"{"note":"a","label":"n"}"#,
    r#"{"note":"a"}"#,
    r#"{"note":"a"}"#,
];

/// What [`Clapping`] is read from: the enum held, no key of it, its tag with its variant's member
/// absent, and that member with no tag.
pub const CLAPPING: [&str; 4] = [
    r#"{"kind":"Bay","label":"n","note":"a"}"#,
    r#"{"note":"a"}"#,
    r#"{"kind":"Bay","note":"a"}"#,
    r#"{"label":"n","note":"a"}"#,
];

/// What serde writes back of each of [`CLAPPING`], read as [`Clapping`]: the enum where it reads,
/// and no key of it where it does not.
pub const CLAPPING_READ_BACK: [&str; 4] = [
    r#"{"kind":"Bay","label":"n","note":"a"}"#,
    r#"{"note":"a"}"#,
    r#"{"note":"a"}"#,
    r#"{"note":"a"}"#,
];

/// What [`Grommeting`] is read from: the struct held with its rule broken, held with its rule kept,
/// no key of it, and a key of it that does not read.
pub const GROMMETING: [&str; 4] = [
    r#"{"label":"north","note":"n"}"#,
    r#"{"label":"north","note":"nn"}"#,
    "{}",
    r#"{"label":5,"note":"nn"}"#,
];

/// What serde answers of each of [`GROMMETING`], read as [`Grommeting`]: the rule's message where
/// the struct reads and breaks it, and what it writes back of every other.
pub const GROMMETING_READ_BACK: [&str; 4] = [
    "'note': too short: minimum length is 2, got 1",
    r#"{"label":"north","note":"nn"}"#,
    "{}",
    "{}",
];

/// A payload of a holder below, what serde writes back of it, and whether TypeScript and Zod take
/// it as the holder's type.
pub struct Unrecorded {
    pub holder: &'static str,
    pub json: &'static str,
    pub read_back: fn(&str) -> String,
    pub taken_by_zod: bool,
}

/// The struct [`Crowned`] flattens.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Bight {
    pub label: String,
}

/// [`Crowned`] optional, the one member, declared above it.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Fiddling {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub crowned: Option<Crowned>,
}

/// A struct with a flattened member of its own, declared below its holder.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Crowned {
    #[serde(flatten)]
    pub bight: Bight,
    pub note: String,
}

/// An enum under a tag alone.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Hitched {
    Bay { label: String },
}

/// [`Hitched`] optional, beside a member of its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Snagging {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub hitched: Option<Hitched>,
    pub note: String,
}

/// An enum under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum Bridled {
    Bay { label: String },
}

/// [`Bridled`] optional, beside a member of its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Bridling {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub bridled: Option<Bridled>,
    pub note: String,
}

/// An untagged enum.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Slipped {
    Bay { label: String },
}

/// [`Slipped`] optional, beside a member of its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Slipping {
    pub note: String,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub slipped: Option<Slipped>,
}

/// An enum under a tag alone that names a function on itself.
#[model_schema(preprocess = ["asGiven"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Clapped {
    Bay { label: String },
}

/// [`Clapped`] optional, beside a member of its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Clapping {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub clapped: Option<Clapped>,
    pub note: String,
}

/// The struct [`Gromet`] flattens.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Earing {
    pub label: String,
}

/// [`Gromet`] optional, the one member, declared above it.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Grommeting {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub gromet: Option<Gromet>,
}

/// A struct with a rule and a flattened member of its own, declared below its holder.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Gromet {
    #[serde(flatten)]
    pub earing: Earing,
    #[model_schema_prop(minLength = 2)]
    pub note: String,
}

/// The payload as it was handed.
const fn as_given(payload: Value) -> Value {
    payload
}

/// What serde writes back of `json`, read as a `T`.
fn written_back<T>(json: &str) -> String
where
    T: DeserializeOwned + Serialize,
{
    serde_json::to_string(&serde_json::from_str::<T>(json).unwrap()).unwrap()
}

/// serde writes each payload back as it was exactly where TypeScript and Zod take it, and drops a
/// key of the type held from every other.
#[test]
fn serde_drops_a_key_of_the_type_held_from_exactly_the_payloads_zod_refuses() {
    for (row, read_back) in UNRECORDED.iter().zip(READ_BACK) {
        assert_eq!(
            (row.read_back)(row.json),
            read_back,
            "serde no longer reads `{}` as {} did",
            row.json,
            row.holder
        );
        assert_eq!(
            read_back == row.json,
            row.taken_by_zod,
            "serde and Zod part otherwise on `{}` as {}",
            row.json,
            row.holder
        );
    }
}

/// serde refuses a payload whose [`Gromet`] reads and breaks its rule, reads one that keeps it,
/// and reads `None` where no key of it is there and where a key of it does not read.
#[test]
fn serde_refuses_a_flattened_option_that_reads_and_breaks_its_rule() {
    let told = GROMMETING.map(|json| {
        serde_json::from_str::<Grommeting>(json).map_or_else(
            |refused| refused.to_string(),
            |read| serde_json::to_string(&read).unwrap(),
        )
    });
    assert_eq!(
        told, GROMMETING_READ_BACK,
        "serde no longer reads each payload as it did"
    );
}

/// serde reads [`Clapping`] from what [`Clapped`]'s function answers wherever it reads, and as
/// `None` wherever it does not.
#[test]
fn serde_reads_an_enum_that_names_a_function_as_none_wherever_it_does_not_read() {
    assert_eq!(
        CLAPPING.map(written_back::<Clapping>),
        CLAPPING_READ_BACK,
        "serde no longer reads each payload as it did"
    );
}
