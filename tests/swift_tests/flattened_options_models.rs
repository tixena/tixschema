//! The models of the flattened `Option` members `swift_tests/flattened_options.rs` pins and
//! `service_schema_emitted_client_tests/run_swift_flattened_options.rs` runs: a struct beside a
//! member of its holder's own, alone in its holder and in a variant under a tag key, a map of
//! dates alone in its holder, and a struct flattened bare beside a member of its holder's own.
//! serde writes the keys of a flattened member that holds a value beside its holder's own, and
//! nothing for one that holds none.

#[cfg(feature = "chrono")]
use alloc::collections::BTreeMap;

#[cfg(feature = "chrono")]
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// What [`Lapping`] and [`Crowning`] are read from: the flattened struct held.
pub const LAPPING: &str = r#"{"label":"north","note":"n"}"#;

/// What [`Lapping`] is read from with the flattened struct absent.
pub const LAPPING_ABSENT: &str = r#"{"note":"n"}"#;

/// What [`Flaking`] is read from: the flattened struct held.
pub const FLAKING: &str = r#"{"label":"north"}"#;

/// What [`Sennit`] is read from: the flattened struct held.
pub const SENNIT: &str = r#"{"kind":"Laid","label":"north","note":"n"}"#;

/// What [`Sennit`] is read from with the flattened struct absent.
pub const SENNIT_ABSENT: &str = r#"{"kind":"Laid","note":"n"}"#;

/// What [`Swaging`] is read from: a date under a key of the flattened map.
#[cfg(feature = "chrono")]
pub const SWAGING: &str = r#"{"north":"2024-01-02T03:04:05.678Z"}"#;

/// The member the structs below flatten.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Tabling {
    pub label: String,
}

/// A flattened `Option` beside a member of its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Lapping {
    pub note: String,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub tabling: Option<Tabling>,
}

/// A flattened `Option`, the one member of its holder.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Flaking {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub tabling: Option<Tabling>,
}

/// A flattened `Option` beside a member of its variant's own, under a tag key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Sennit {
    Laid {
        #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
        tabling: Option<Tabling>,
        note: String,
    },
}

/// A flattened `Option` of a map of dates, the one member of its holder, which Swift writes as
/// the text serde writes.
#[cfg(feature = "chrono")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Swaging {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub stamps: Option<BTreeMap<String, DateTime<Utc>>>,
}

/// A struct flattened bare beside a member of its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Crowning {
    pub note: String,
    #[serde(flatten)]
    pub tabling: Tabling,
}

/// The [`Tabling`] each payload above holds.
pub fn north() -> Tabling {
    Tabling {
        label: "north".to_owned(),
    }
}

/// serde reads the flattened `Option` where its keys are held, writes its keys back beside its
/// holder's own, and writes nothing for one that holds none.
#[test]
fn serde_writes_a_flattened_option_only_where_it_holds_a_value() {
    let read = |text: &str| serde_json::from_str::<Lapping>(text).unwrap();
    let written = |value: &Lapping| serde_json::to_string(value).unwrap();
    assert_eq!(read(LAPPING).tabling, Some(north()));
    assert_eq!(written(&read(LAPPING)), r#"{"note":"n","label":"north"}"#);
    assert_eq!(read(LAPPING_ABSENT).tabling, None);
    assert_eq!(written(&read(LAPPING_ABSENT)), LAPPING_ABSENT);
    let flaking: Flaking = serde_json::from_str(FLAKING).unwrap();
    assert_eq!(serde_json::to_string(&flaking).unwrap(), FLAKING);
    let none: Flaking = serde_json::from_str("{}").unwrap();
    assert_eq!(none, Flaking { tabling: None });
    assert_eq!(serde_json::to_string(&none).unwrap(), "{}");
    let sennit: Sennit = serde_json::from_str(SENNIT).unwrap();
    assert_eq!(serde_json::to_string(&sennit).unwrap(), SENNIT);
    let absent: Sennit = serde_json::from_str(SENNIT_ABSENT).unwrap();
    assert_eq!(
        absent,
        Sennit::Laid {
            tabling: None,
            note: "n".to_owned(),
        }
    );
    assert_eq!(serde_json::to_string(&absent).unwrap(), SENNIT_ABSENT);
    let crowning: Crowning = serde_json::from_str(LAPPING).unwrap();
    assert_eq!(
        serde_json::to_string(&crowning).unwrap(),
        r#"{"note":"n","label":"north"}"#
    );
}

/// serde reads the flattened map of dates from every key, writes each date back as the text it
/// read, and writes nothing for a map it does not hold.
#[cfg(feature = "chrono")]
#[test]
fn serde_writes_a_flattened_option_of_dates_as_the_text_it_read() {
    let swaging: Swaging = serde_json::from_str(SWAGING).unwrap();
    assert_eq!(swaging.stamps.as_ref().map(BTreeMap::len), Some(1));
    assert_eq!(serde_json::to_string(&swaging).unwrap(), SWAGING);
    let none = Swaging { stamps: None };
    assert_eq!(serde_json::to_string(&none).unwrap(), "{}");
}
