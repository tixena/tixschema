//! The models of a fixed array `[T; N]` held in each place a read reads it: beside a flattened
//! member, as an item of a list, named by an alias, in a tuple's position, in a variant's slot, as
//! a map's value, under an `Option`, filling a type parameter, and of a type parameter's filling.
//! serde and the Zod schema read each from an array of `N` items alone, and refuse an array of
//! another length. [`FIXED`] holds a payload of each and what serde reads of it.
//! `kotlin_tests/read.rs` pins the Kotlin text, and
//! `service_schema_emitted_client_tests/run_kotlin_refusals.rs` runs it against serde.

use alloc::collections::BTreeMap;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A payload of each model, of the length written and of another.
pub const FIXED: [Fixed; 20] = [
    Fixed {
        json: r#"{"held":[],"label":"n","pair":[1,2]}"#,
        read_as: "Reefband",
        serde: written_back::<Reefband>,
        thrown: None,
    },
    Fixed {
        json: r#"{"held":[],"label":"n","pair":[1,2,3]}"#,
        read_as: "Reefband",
        serde: written_back::<Reefband>,
        thrown: Some("invalid length 3, expected 2"),
    },
    Fixed {
        json: r#"{"held":["a",[1,2]]}"#,
        read_as: "Buntline",
        serde: written_back::<Buntline>,
        thrown: None,
    },
    Fixed {
        json: r#"{"held":["a",[1,2,3]]}"#,
        read_as: "Buntline",
        serde: written_back::<Buntline>,
        thrown: Some("invalid length 3, expected 2"),
    },
    Fixed {
        json: r#"{"Paired":[1,2]}"#,
        read_as: "Shroud",
        serde: written_back::<Shroud>,
        thrown: None,
    },
    Fixed {
        json: r#"{"Paired":[1]}"#,
        read_as: "Shroud",
        serde: written_back::<Shroud>,
        thrown: Some("invalid length 1, expected 2"),
    },
    Fixed {
        json: r#"{"byDock":{"north":[1,2]}}"#,
        read_as: "Shrouding",
        serde: written_back::<Shrouding>,
        thrown: None,
    },
    Fixed {
        json: r#"{"byDock":{"north":[1,2,3]}}"#,
        read_as: "Shrouding",
        serde: written_back::<Shrouding>,
        thrown: Some("invalid length 3, expected 2"),
    },
    Fixed {
        json: r#"{"maybe":[1,2]}"#,
        read_as: "Fishing",
        serde: written_back::<Fishing>,
        thrown: None,
    },
    Fixed {
        json: "{}",
        read_as: "Fishing",
        serde: written_back::<Fishing>,
        thrown: None,
    },
    Fixed {
        json: r#"{"maybe":[1]}"#,
        read_as: "Fishing",
        serde: written_back::<Fishing>,
        thrown: Some("invalid length 1, expected 2"),
    },
    Fixed {
        json: r#"{"catted":{"held":[1,2]}}"#,
        read_as: "Sheaving",
        serde: written_back::<Sheaving>,
        thrown: None,
    },
    Fixed {
        json: r#"{"catted":{"held":[1,2,3]}}"#,
        read_as: "Sheaving",
        serde: written_back::<Sheaving>,
        thrown: Some("invalid length 3, expected 2"),
    },
    Fixed {
        json: r#"{"cringled":{"pair":["a","b"]}}"#,
        read_as: "Garland",
        serde: written_back::<Garland>,
        thrown: None,
    },
    Fixed {
        json: r#"{"cringled":{"pair":["a"]}}"#,
        read_as: "Garland",
        serde: written_back::<Garland>,
        thrown: Some("invalid length 1, expected 2"),
    },
    Fixed {
        json: r#"{"pairs":[[1,2],[3,4]]}"#,
        read_as: "Earring",
        serde: written_back::<Earring>,
        thrown: None,
    },
    Fixed {
        json: r#"{"pairs":[[1,2],[3]]}"#,
        read_as: "Earring",
        serde: written_back::<Earring>,
        thrown: Some("invalid length 1, expected 2"),
    },
    Fixed {
        json: r#"{"bitter":[1,2]}"#,
        read_as: "Becketing",
        serde: written_back::<Becketing>,
        thrown: None,
    },
    Fixed {
        json: r#"{"bitter":[1,2,3]}"#,
        read_as: "Becketing",
        serde: written_back::<Becketing>,
        thrown: Some("invalid length 3, expected 2"),
    },
    Fixed {
        json: r#"{"held":[1,2,3],"label":"n"}"#,
        read_as: "Bunt",
        serde: written_back::<Bunt>,
        thrown: None,
    },
];

/// One payload, the type it is read as, what serde reads of it and what the Kotlin read throws.
pub struct Fixed {
    pub json: &'static str,
    pub read_as: &'static str,
    /// What serde writes back of the payload, or `refused`.
    pub serde: fn(&str) -> String,
    /// What the Kotlin read throws, or `None` where it reads the payload.
    pub thrown: Option<&'static str>,
}

/// The member [`Reefband`] flattens, and a list of any length.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Bunt {
    pub held: Vec<u8>,
    pub label: String,
}

/// A fixed array beside a flattened member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Reefband {
    #[serde(flatten)]
    pub bunt: Bunt,
    pub pair: [u8; 2],
}

/// A fixed array an alias names.
#[model_schema()]
pub type Bitter = [u8; 2];

/// A member typed with an alias of a fixed array.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Becketing {
    pub bitter: Bitter,
}

/// A fixed array as an item of a list.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Earring {
    pub pairs: Vec<[u8; 2]>,
}

/// A fixed array in a tuple's position.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Buntline {
    pub held: (String, [u8; 2]),
}

/// A fixed array in a variant's slot.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Shroud {
    Paired([u8; 2]),
    Plain,
}

/// A fixed array as a map's value.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Shrouding {
    pub by_dock: BTreeMap<String, [u8; 2]>,
}

/// A fixed array under an `Option`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Fishing {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maybe: Option<[u8; 2]>,
}

/// A type parameter, which [`Sheaving`] fills with a fixed array.
#[model_schema(default_types(T = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Catting<T> {
    pub held: T,
}

/// A type parameter filled with a fixed array.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Sheaving {
    pub catted: Catting<[u8; 2]>,
}

/// A fixed array of a type parameter, which [`Garland`] fills.
#[model_schema(default_types(T = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Cringling<T> {
    pub pair: [T; 2],
}

/// A fixed array of a type parameter's filling.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Garland {
    pub cringled: Cringling<String>,
}

/// What serde writes back of `json` read as `T`, or `refused`.
fn written_back<T>(json: &str) -> String
where
    T: DeserializeOwned + Serialize,
{
    serde_json::from_str::<T>(json).map_or_else(
        |_| "refused".to_owned(),
        |read| serde_json::to_string(&read).unwrap(),
    )
}

/// serde refuses each payload of another length, and writes back as it was each of the length
/// written.
#[test]
fn serde_reads_a_fixed_array_from_an_array_of_its_length_alone() {
    for row in &FIXED {
        let told = (row.serde)(row.json);
        let expected = if row.thrown.is_some() {
            "refused"
        } else {
            row.json
        };
        assert_eq!(
            told, expected,
            "serde no longer reads `{}` as {} as it did",
            row.json, row.read_as
        );
    }
}
