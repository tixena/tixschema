//! The models a variant under its `alias` is read under where its enum is flattened, under each
//! tagging, beside a member of its holder's own: the enums of `kotlin_tests/aliased_models.rs`,
//! each held by a struct of its own. serde reads a variant under each of its names, and a member
//! of the variant under each of its names, from the entries the holder's own member did not take.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::aliased_models::{Careening, Scending, Surging, written_back};
#[cfg(any(feature = "zod", feature = "typescript", feature = "jsonschema"))]
use super::aliased_models::{careening_schema, scending_schema, surging_schema};

/// What [`Riding`] is read from: a variant's entry under its alias with a member under its alias,
/// a variant of one value under its alias after the holder's member, a unit variant's entry over
/// `null` under its alias after the holder's member, and a member under its name and its alias,
/// which serde refuses.
pub const RIDING: [&str; 4] = [
    r#"{"Pier":{"name":"n"},"note":"a"}"#,
    r#"{"note":"a","Tied":"x"}"#,
    r#"{"note":"a","Slack":null}"#,
    r#"{"Berth":{"label":"n","name":"m"},"note":"a"}"#,
];

/// What [`Swinging`] is read from: a tag under its alias with a member under its alias, a unit
/// variant's tag under its alias after the holder's member, and a member under its name and its
/// alias, which serde refuses.
pub const SWINGING: [&str; 3] = [
    r#"{"kind":"Pier","name":"n","note":"a"}"#,
    r#"{"note":"a","kind":"Still"}"#,
    r#"{"kind":"Pier","label":"n","name":"m","note":"a"}"#,
];

/// What [`Drifting`] is read from: a tag under its alias with a member under its alias, a unit
/// variant's tag under its alias, and a variant of one value under its alias.
pub const DRIFTING: [&str; 3] = [
    r#"{"kind":"Pier","data":{"name":"n"},"note":"a"}"#,
    r#"{"kind":"Still","note":"a"}"#,
    r#"{"kind":"Tied","data":"x","note":"a"}"#,
];

/// [`Surging`], externally tagged, flattened beside a member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Riding {
    pub note: String,
    #[serde(flatten)]
    pub surging: Surging,
}

/// [`Careening`], under a tag alone, flattened beside a member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Swinging {
    #[serde(flatten)]
    pub careening: Careening,
    pub note: String,
}

/// [`Scending`], under a tag and a content key, flattened beside a member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Drifting {
    pub note: String,
    #[serde(flatten)]
    pub scending: Scending,
}

/// serde reads a flattened enum's variant under each of its names, and a member of the variant
/// under each of its names, and writes each back under the name it writes.
#[test]
fn serde_reads_a_flattened_variant_under_each_alias() {
    assert_eq!(
        RIDING.map(written_back::<Riding>),
        [
            r#"{"note":"a","Berth":{"label":"n"}}"#,
            r#"{"note":"a","Held":"x"}"#,
            r#"{"note":"a","Calm":null}"#,
            "refused",
        ]
    );
    assert_eq!(
        SWINGING.map(written_back::<Swinging>),
        [
            r#"{"kind":"Berth","label":"n","note":"a"}"#,
            r#"{"kind":"Calm","note":"a"}"#,
            "refused",
        ]
    );
    assert_eq!(
        DRIFTING.map(written_back::<Drifting>),
        [
            r#"{"note":"a","kind":"Berth","data":{"label":"n"}}"#,
            r#"{"note":"a","kind":"Calm"}"#,
            r#"{"note":"a","kind":"Held","data":"x"}"#,
        ]
    );
}
