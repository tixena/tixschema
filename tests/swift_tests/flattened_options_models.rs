//! The models of the flattened `Option` members `swift_tests/flattened_options.rs` pins and
//! `service_schema_emitted_client_tests/run_swift_flattened_options.rs` runs: a struct beside a
//! member of its holder's own, alone in its holder and in a variant under a tag key, a struct of
//! two keys, a struct declared below its holder, a map of dates alone in its holder, a struct
//! flattened bare beside a member of its holder's own, a model `validate()` walks alone and beside
//! a member of its holder's own, a struct that reads the keys no member around it took, and a
//! struct read from what a function answers. serde writes the keys of a flattened member that
//! holds a value beside its holder's own, and nothing for one that holds none, and reads one that
//! names no function as `None` wherever the type it holds does not read from the object it is
//! handed. `run_kotlin_flattened.rs` reads [`Crowning`] and [`Fidding`] as well.

#[cfg(feature = "chrono")]
use alloc::collections::BTreeMap;

#[cfg(feature = "chrono")]
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::model_schema;

/// What [`Lapping`] and [`Crowning`] are read from: the flattened struct held.
pub const LAPPING: &str = r#"{"label":"north","note":"n"}"#;

/// What [`Lapping`] is read from with the flattened struct absent.
pub const LAPPING_ABSENT: &str = r#"{"note":"n"}"#;

/// What [`Lapping`] and [`Tarring`] are read from with the flattened struct's key held as a
/// number, which its struct does not read.
pub const LAPPING_MISTYPED: &str = r#"{"label":5,"note":"n"}"#;

/// What [`Flaking`], [`Kinking`] and [`Ratline`] are read from: the flattened struct held. What
/// [`Painter`] is read from with one of its flattened struct's two keys.
pub const FLAKING: &str = r#"{"label":"north"}"#;

/// What [`Flaking`], [`Kinking`] and [`Fidding`] are read from with the flattened struct absent.
pub const FLAKING_ABSENT: &str = "{}";

/// What [`Flaking`], [`Kinking`] and [`Ratline`] are read from with the flattened struct's key
/// held as a number, which its struct does not read.
pub const FLAKING_MISTYPED: &str = r#"{"label":5}"#;

/// What [`Painter`] is read from: the flattened struct held, both its keys.
pub const PAINTER: &str = r#"{"label":"north","turns":2}"#;

/// What [`Kinking`] is read from with a label its rule refuses.
pub const KINKING_SHORT: &str = r#"{"label":"n"}"#;

/// What [`Tarring`] is read from with a label its rule refuses.
pub const TARRING_SHORT: &str = r#"{"label":"n","note":"n"}"#;

/// What [`Sennit`] is read from: the flattened struct held.
pub const SENNIT: &str = r#"{"kind":"Laid","label":"north","note":"n"}"#;

/// What [`Sennit`] is read from with the flattened struct absent.
pub const SENNIT_ABSENT: &str = r#"{"kind":"Laid","note":"n"}"#;

/// What [`Sennit`] is read from with the flattened struct's key held as a number.
pub const SENNIT_MISTYPED: &str = r#"{"kind":"Laid","label":5,"note":"n"}"#;

/// What [`Swaging`] is read from: a date under a key of the flattened map.
#[cfg(feature = "chrono")]
pub const SWAGING: &str = r#"{"north":"2024-01-02T03:04:05.678Z"}"#;

/// What [`Swaging`] is read from with an entry that is no date.
#[cfg(feature = "chrono")]
pub const SWAGING_MISTYPED: &str = r#"{"north":"x"}"#;

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

/// The member [`Painter`] flattens, which reads two keys.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Gasket {
    pub label: String,
    pub turns: u32,
}

/// A flattened `Option` of a struct of two keys, the one member of its holder.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Painter {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub gasket: Option<Gasket>,
}

/// A flattened `Option` of [`Nippering`], declared below it, whose keys its read does not know.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Ratline {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub nippering: Option<Nippering>,
}

/// The member [`Ratline`] flattens, declared below it.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Nippering {
    pub label: String,
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

/// The member [`Kinking`] and [`Tarring`] flatten, whose `validate()` holds its label to a length.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Plaiting {
    #[model_schema_prop(minLength = 2)]
    pub label: String,
}

/// A flattened `Option` of a model `validate()` walks, the one member of its holder.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Kinking {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub plaiting: Option<Plaiting>,
}

/// A flattened `Option` of a model `validate()` walks, beside a member of its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Tarring {
    pub note: String,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub plaiting: Option<Plaiting>,
}

/// A flattened `Option` of [`Crowning`], which reads its own flattened struct from the object
/// without the keys taken around it.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Fidding {
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub crowning: Option<Crowning>,
}

/// A flattened `Option` read from what a function answers, beside a member of its holder's own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Veering {
    pub note: String,
    #[model_schema_prop(preprocess = ["asGiven"])]
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub tabling: Option<Tabling>,
}

/// The payload as it was handed.
const fn as_given(payload: Value) -> Value {
    payload
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

/// serde reads a flattened `Option` as `None` wherever the type it holds does not read from the
/// object it is handed: a key it reads absent, held as a type it does not read, or one of two keys
/// with the other absent. A model `validate()` walks is read with no judgement where its keys are
/// held, and its holder's read refuses one that breaks its rule.
#[test]
fn serde_reads_a_flattened_option_as_none_wherever_its_type_does_not_read() {
    for json in [LAPPING_ABSENT, LAPPING_MISTYPED] {
        let lapping: Lapping = serde_json::from_str(json).unwrap();
        assert_eq!(
            lapping,
            Lapping {
                note: "n".to_owned(),
                tabling: None,
            },
            "{json}"
        );
    }
    for json in [FLAKING_ABSENT, FLAKING_MISTYPED] {
        let flaking: Flaking = serde_json::from_str(json).unwrap();
        assert_eq!(flaking, Flaking { tabling: None }, "{json}");
        let kinking: Kinking = serde_json::from_str(json).unwrap();
        assert_eq!(kinking, Kinking { plaiting: None }, "{json}");
        let ratline: Ratline = serde_json::from_str(json).unwrap();
        assert_eq!(ratline, Ratline { nippering: None }, "{json}");
    }
    let sennit: Sennit = serde_json::from_str(SENNIT_MISTYPED).unwrap();
    assert_eq!(
        sennit,
        Sennit::Laid {
            tabling: None,
            note: "n".to_owned(),
        }
    );
    let painter = |json: &str| serde_json::from_str::<Painter>(json).unwrap().gasket;
    assert_eq!(painter(FLAKING), None);
    assert_eq!(
        painter(PAINTER),
        Some(Gasket {
            label: "north".to_owned(),
            turns: 2,
        })
    );
    assert_eq!(
        serde_json::from_str::<Ratline>(FLAKING).unwrap().nippering,
        Some(Nippering {
            label: "north".to_owned(),
        })
    );
    let plaiting = Some(Plaiting {
        label: "north".to_owned(),
    });
    let read_tarring = |json: &str| serde_json::from_str::<Tarring>(json).unwrap().plaiting;
    assert_eq!(
        serde_json::from_str::<Kinking>(FLAKING).unwrap().plaiting,
        plaiting
    );
    assert_eq!(read_tarring(LAPPING_ABSENT), None);
    assert_eq!(read_tarring(LAPPING_MISTYPED), None);
    assert_eq!(read_tarring(LAPPING), plaiting);
    for (json, refused) in [
        (
            KINKING_SHORT,
            serde_json::from_str::<Kinking>(KINKING_SHORT).map(drop),
        ),
        (
            TARRING_SHORT,
            serde_json::from_str::<Tarring>(TARRING_SHORT).map(drop),
        ),
    ] {
        assert_eq!(
            refused.unwrap_err().to_string(),
            "'label': too short: minimum length is 2, got 1",
            "{json}"
        );
    }
    for json in [FLAKING_ABSENT, LAPPING_ABSENT] {
        let fidding: Fidding = serde_json::from_str(json).unwrap();
        assert_eq!(fidding, Fidding { crowning: None });
    }
    let fidding: Fidding = serde_json::from_str(LAPPING).unwrap();
    assert_eq!(
        fidding.crowning,
        Some(Crowning {
            note: "n".to_owned(),
            tabling: north(),
        })
    );
}

/// serde reads a flattened `Option` that names a function from what the function answers, and
/// refuses an answer its type does not read.
#[test]
fn serde_refuses_a_flattened_option_whose_function_answers_what_its_type_does_not_read() {
    let veering: Veering = serde_json::from_str(LAPPING).unwrap();
    assert_eq!(veering.tabling, Some(north()));
    let refused = serde_json::from_str::<Veering>(LAPPING_ABSENT).unwrap_err();
    assert_eq!(
        refused.to_string(),
        "missing field `label` at line 1 column 12"
    );
}

/// serde reads the flattened map of dates from every key, writes each date back as the text it
/// read, reads it as `None` where an entry is no date, and writes nothing for a map it does not
/// hold.
#[cfg(feature = "chrono")]
#[test]
fn serde_writes_a_flattened_option_of_dates_as_the_text_it_read() {
    let swaging: Swaging = serde_json::from_str(SWAGING).unwrap();
    assert_eq!(swaging.stamps.as_ref().map(BTreeMap::len), Some(1));
    assert_eq!(serde_json::to_string(&swaging).unwrap(), SWAGING);
    let mistyped: Swaging = serde_json::from_str(SWAGING_MISTYPED).unwrap();
    assert_eq!(mistyped, Swaging { stamps: None });
    let none = Swaging { stamps: None };
    assert_eq!(serde_json::to_string(&none).unwrap(), "{}");
}
