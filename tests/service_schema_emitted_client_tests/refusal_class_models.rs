//! The models the refusal of each Dart, Swift and Kotlin type is held to: a struct, an enum, a
//! generic type, a brand and an alias, each with a bound a value breaks, and what the Rust side
//! answers of the same values.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// The payload of a skid that breaks both its bounds.
pub const BROKEN_SKID: &str = r#"{"label":"","parcelCount":200}"#;

/// The payload of a skid whose label is no text: a refusal of shape.
pub const UNREAD_SKID: &str = r#"{"label":1,"parcelCount":2}"#;

/// The payload of a counted haulage over its bound.
pub const BROKEN_HAULAGE: &str = r#"{"Counted":{"crates":9}}"#;

/// The payload of a crib whose title breaks its bound.
pub const BROKEN_CRIB: &str = r#"{"items":[],"title":""}"#;

/// The payload of a brand below its bound.
pub const BROKEN_LABEL: &str = r#""a""#;

/// A struct with a bound on each member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Skid {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
    #[model_schema_prop(maximum = 100)]
    pub parcel_count: u32,
}

/// An enum with a bound in each variant.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Haulage {
    Counted {
        #[model_schema_prop(maximum = 5)]
        crates: u32,
    },
    Loose {
        #[model_schema_prop(minLength = 1)]
        label: String,
    },
}

/// A generic type with a bound on a member of its own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Crib<Item> {
    pub items: Vec<Item>,
    #[model_schema_prop(minLength = 1)]
    pub title: String,
}

/// A brand with a bound.
#[model_schema(minLength = 2)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct SkepLabel(String);

/// An alias of the struct.
#[model_schema()]
pub type Truss = Skid;

/// What Rust's `validate()` answers of the skid every table breaks, which [`Truss`] answers as well:
/// an alias is the type it names.
pub fn skid_issues() -> Vec<String> {
    let issues = SkidDraft {
        label: String::new(),
        parcel_count: 200,
    }
    .validate()
    .unwrap_err();
    let refused = serde_json::from_str::<Truss>(BROKEN_SKID)
        .unwrap_err()
        .to_string();
    assert!(refused.starts_with(&issues.join("; ")), "{refused}");
    issues
}

/// What Rust's `validate()` answers of a loose haulage with an empty label.
pub fn loose_issues() -> Vec<String> {
    HaulageDraft::Loose {
        label: String::new(),
    }
    .validate()
    .unwrap_err()
}

/// What Rust's `validate()` answers of the haulage [`BROKEN_HAULAGE`] holds.
pub fn counted_issues() -> Vec<String> {
    HaulageDraft::Counted { crates: 9 }.validate().unwrap_err()
}

/// What Rust's `validate()` answers of the crib [`BROKEN_CRIB`] holds.
pub fn crib_issues() -> Vec<String> {
    CribDraft::<Skid> {
        items: Vec::new(),
        title: String::new(),
    }
    .validate()
    .unwrap_err()
}

/// What Rust answers of the brand [`BROKEN_LABEL`] holds.
pub fn label_issues() -> Vec<String> {
    SkepLabel::new("a").unwrap_err()
}

/// What prints a case: its name, the class it was refused with and each issue, one per line.
pub fn told(name: &str, refusal: &str, issues: &[String]) -> String {
    let mut printed = format!("{name}: {refusal}\n");
    for issue in issues {
        printed.push_str("    ");
        printed.push_str(issue);
        printed.push('\n');
    }
    printed
}
