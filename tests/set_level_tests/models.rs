//! The models a set is written on, held to one list level in every client, and the payloads each
//! client reads and writes back as serde does.

use alloc::collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque};
use std::collections::HashSet;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Payloads read and written back as written, each with the name of the type it is read as.
pub const ROUND_TRIPS: &[(&str, &str)] = &[
    ("ProbeSets", r#"{"labels":["a"],"parcels":[["a"]]}"#),
    ("Groups", r#"{"groups":[["a"],["b"]]}"#),
    (
        "Wrappers",
        r#"{"binary_heap":["a"],"btree_set":["b"],"hash_set":["c"],"vec_deque":["e"]}"#,
    ),
    #[cfg(feature = "indexmap")]
    ("Indexed", r#"{"index_set":["d"]}"#),
    (
        "Shelf",
        r#"{"entries":[{"name":"ab"}],"keyed":{"k":["a"]},"maybe":null,"nullable":[null],"slot":[["a"],1]}"#,
    ),
];

/// A payload whose set of models breaks a bound, with the type it is read as.
pub const BROKEN: (&str, &str) = (
    "Shelf",
    r#"{"entries":[{"name":"a"}],"keyed":{},"maybe":null,"nullable":[],"slot":[[],1]}"#,
);

/// What Rust's read refuses [`BROKEN`] with.
pub const BROKEN_MESSAGE: &str = "'entries.0.name': too short: minimum length is 2, got 1";

/// A set held alone and in a list.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProbeSets {
    pub labels: HashSet<String>,
    pub parcels: Vec<HashSet<String>>,
}

/// A list of sets.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Groups {
    pub groups: Vec<HashSet<String>>,
}

/// Each wrapper that serde writes as an array.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Wrappers {
    pub binary_heap: BinaryHeap<String>,
    pub btree_set: BTreeSet<String>,
    pub hash_set: HashSet<String>,
    pub vec_deque: VecDeque<String>,
}

/// The `indexmap` set.
#[cfg(feature = "indexmap")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Indexed {
    pub index_set: indexmap::IndexSet<String>,
}

/// A model a set holds, with a bound on a member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Entry {
    #[model_schema_prop(minLength = 2)]
    pub name: String,
}

/// A set of models, an optional set, a set of optional items, a set in a map's value and a set in
/// a tuple slot.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Shelf {
    pub entries: HashSet<Entry>,
    pub keyed: BTreeMap<String, HashSet<String>>,
    #[model_schema_prop(nullable)]
    pub maybe: Option<HashSet<String>>,
    pub nullable: HashSet<Option<String>>,
    pub slot: (HashSet<String>, u8),
}

fn written<Held>(json: &str) -> String
where
    Held: DeserializeOwned + Serialize,
{
    serde_json::to_string(&serde_json::from_str::<Held>(json).unwrap()).unwrap()
}

/// Holds Rust's read of each payload to writing it back byte for byte, and of [`BROKEN`] to its
/// refusal.
pub fn assert_rust_writes_back() {
    assert_eq!(
        serde_json::from_str::<Shelf>(BROKEN.1)
            .unwrap_err()
            .to_string(),
        BROKEN_MESSAGE,
        "a bound broken by an item of a set"
    );
    for &(name, json) in ROUND_TRIPS {
        let back = match name {
            "ProbeSets" => written::<ProbeSets>(json),
            "Groups" => written::<Groups>(json),
            "Wrappers" => written::<Wrappers>(json),
            #[cfg(feature = "indexmap")]
            "Indexed" => written::<Indexed>(json),
            _ => written::<Shelf>(json),
        };
        assert_eq!(back, json, "{name}");
    }
}
