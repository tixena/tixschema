//! A member serde neither writes nor reads, typed `u64` and `usize`, and a variant serde neither
//! writes nor reads that holds one. No client declares either, so neither width guard refuses
//! them.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A struct whose `u64` and `usize` are taken off the wire in both directions.
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ballast {
    pub note: String,
    #[serde(skip_serializing, skip_deserializing)]
    pub tonnage: usize,
    #[serde(skip)]
    pub weight: u64,
}

/// An enum whose variant under `skip` holds a `u64`.
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BallastLoad {
    Open {
        note: String,
    },
    #[serde(skip)]
    Shifted {
        weight: u64,
    },
}

/// serde writes neither member nor the variant, and reads none of them: a key a payload holds for
/// either member is passed over, and the variant's name is no variant it reads.
pub fn assert_serde_neither_writes_nor_reads() {
    let ballast = Ballast {
        note: "aft".to_owned(),
        tonnage: 7,
        weight: 3,
    };
    assert_eq!(
        serde_json::to_string(&ballast).unwrap(),
        r#"{"note":"aft"}"#,
        "serde writes no member taken off the wire"
    );
    let read: Ballast = serde_json::from_str(r#"{"note":"aft","tonnage":7,"weight":3}"#).unwrap();
    assert_eq!(
        (read.note.as_str(), read.tonnage, read.weight),
        ("aft", 0, 0),
        "serde reads no member taken off the wire"
    );
    let open = BallastLoad::Open {
        note: "aft".to_owned(),
    };
    assert_eq!(
        serde_json::to_string(&open).unwrap(),
        r#"{"Open":{"note":"aft"}}"#,
        "serde writes the variant on the wire"
    );
    let shifted = BallastLoad::Shifted { weight: 3 };
    assert!(
        serde_json::to_string(&shifted).is_err(),
        "serde writes no value of the variant taken off the wire"
    );
    assert!(
        matches!(shifted, BallastLoad::Shifted { weight: 3 }),
        "the variant holds its member in Rust"
    );
    assert!(
        serde_json::from_str::<BallastLoad>(r#"{"Shifted":{"weight":3}}"#).is_err(),
        "serde reads no value of the variant taken off the wire"
    );
}
