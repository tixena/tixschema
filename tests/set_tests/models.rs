//! The models a set is written on, and the payloads that repeat an item of one. Every surface but
//! Rust refuses a repeat, each under the place of its set; Rust's read keeps one item.

use alloc::collections::{BTreeMap, BTreeSet};
use std::collections::HashSet;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Every payload, a set that repeats an item in each place a set is held, and one that repeats
/// none.
pub const PAYLOADS: [Payload; 13] = [
    Payload {
        json: r#"{"tags":["a","a"]}"#,
        name: "repeated tags",
        read_as: "Ticket",
    },
    Payload {
        json: r#"{"tags":["a","b"]}"#,
        name: "distinct tags",
        read_as: "Ticket",
    },
    Payload {
        json: r#"{"ticket":{"tags":["a","a"]}}"#,
        name: "held tags",
        read_as: "Folder",
    },
    Payload {
        json: r#"{"btree_set":["a","a"],"hash_set":["b","b"],"vec":["c","c"]}"#,
        name: "counted",
        read_as: "EveryList",
    },
    Payload {
        json: r#"{"grid":[[1,2],[3,3]],"keyed":{"k":[4,4]},"listed":[[1,2],[1,2]],"maybe":[5,5],"points":[{"x":1,"y":2},{"y":2,"x":1}]}"#,
        name: "nested repeated",
        read_as: "Nesting",
    },
    Payload {
        json: r#"{"grid":[[1,2]],"keyed":{"k":[4]},"listed":[[1,2],[2,1]],"points":[{"x":1,"y":2},{"x":2,"y":1}]}"#,
        name: "nested distinct",
        read_as: "Nesting",
    },
    Payload {
        json: r#"["a","a"]"#,
        name: "brand",
        read_as: "Lanyard",
    },
    Payload {
        json: "[[1,1],2]",
        name: "slot",
        read_as: "Pouch",
    },
    Payload {
        json: r#"{"kind":"Loose","held":{"members":["a","a"]}}"#,
        name: "variant member",
        read_as: "Shelving",
    },
    Payload {
        json: r#"{"kind":"Bundle","held":[1,1]}"#,
        name: "variant slot",
        read_as: "Shelving",
    },
    Payload {
        json: r#"{"labels":["a","a"]}"#,
        name: "untagged labels",
        read_as: "Sorting",
    },
    Payload {
        json: r#"{"numbers":[1,1]}"#,
        name: "untagged numbers",
        read_as: "Sorting",
    },
    Payload {
        json: r#"{"coins":[{"id":"a"},{"id":"a"}]}"#,
        name: "filled",
        read_as: "Wallet",
    },
];

/// What the JSON Schema, Zod and each client print for [`PAYLOADS`]: a payload read, or refused
/// with each set that repeats an item under its place.
pub const PRINTED: &str = "\
repeated tags: refused
    'tags': repeats an item: a set holds each item once
distinct tags: read
held tags: refused
    'ticket.tags': repeats an item: a set holds each item once
counted: refused
    'btree_set': repeats an item: a set holds each item once
    'hash_set': repeats an item: a set holds each item once
nested repeated: refused
    'grid.1': repeats an item: a set holds each item once
    'keyed.k': repeats an item: a set holds each item once
    'listed': repeats an item: a set holds each item once
    'maybe': repeats an item: a set holds each item once
    'points': repeats an item: a set holds each item once
nested distinct: read
brand: refused
    repeats an item: a set holds each item once
slot: refused
    '0': repeats an item: a set holds each item once
variant member: refused
    'held.members': repeats an item: a set holds each item once
variant slot: refused
    'held': repeats an item: a set holds each item once
untagged labels: refused
    'labels': repeats an item: a set holds each item once
untagged numbers: refused
    'numbers': repeats an item: a set holds each item once
filled: refused
    'coins': repeats an item: a set holds each item once
";

/// A model a set holds: two are one item where their members are, whatever order a payload writes
/// the members in.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Point {
    pub x: u32,
    pub y: u32,
}

/// A set with no rule.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Ticket {
    pub tags: HashSet<String>,
}

/// A model that holds a set.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Folder {
    pub ticket: Ticket,
}

/// Each set beside a list, each counted.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EveryList {
    #[model_schema_prop(minLength = 2)]
    pub btree_set: BTreeSet<String>,
    #[model_schema_prop(minLength = 2)]
    pub hash_set: HashSet<String>,
    #[model_schema_prop(minLength = 2)]
    pub vec: Vec<String>,
}

/// A set in a list, as a map's value, of lists, under `Option`, and of a model.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Nesting {
    pub grid: Vec<HashSet<u32>>,
    pub keyed: BTreeMap<String, BTreeSet<u32>>,
    pub listed: HashSet<Vec<u32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maybe: Option<BTreeSet<u32>>,
    pub points: BTreeSet<Point>,
}

/// A brand over a set.
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Lanyard(pub HashSet<String>);

/// A set in a slot of a tuple struct.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Pouch(pub BTreeSet<u32>, pub u8);

/// A set as a variant's member and as its one slot.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum Shelving {
    Bundle(BTreeSet<u32>),
    Loose { members: HashSet<String> },
}

/// An untagged enum whose variants a set's own key tells apart: a repeat chooses no variant.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Sorting {
    Labels { labels: HashSet<String> },
    Numbers { numbers: BTreeSet<u32> },
}

/// A model a holder fills with its own type parameter.
#[model_schema(default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Coin<IdType> {
    pub id: IdType,
}

/// A set of a model filled with the holder's own type parameter, which a client reads at a
/// string.
#[model_schema(default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Wallet<IdType: Ord> {
    pub coins: BTreeSet<Coin<IdType>>,
}

/// One payload: its name, the type it is read as, and the JSON.
pub struct Payload {
    pub json: &'static str,
    pub name: &'static str,
    pub read_as: &'static str,
}

/// The JSON of the payload of [`PAYLOADS`] named `name`.
fn json_of(name: &str) -> &'static str {
    PAYLOADS
        .iter()
        .find(|payload| payload.name == name)
        .unwrap()
        .json
}

/// What Rust's read answers for the payload named `name`, read as a `T`: the value, or the text
/// of the refusal.
fn read<T>(name: &str) -> Result<T, String>
where
    T: DeserializeOwned,
{
    serde_json::from_str(json_of(name)).map_err(|refused| refused.to_string())
}

/// Each text, collected into whichever set the position holds.
fn texts<Held>(each: &[&str]) -> Held
where
    Held: FromIterator<String>,
{
    each.iter().map(|&text| text.to_owned()).collect()
}

/// Holds Rust's read of each of [`PAYLOADS`] to what it answers: a set keeps one item of a
/// repeat, and a count is held to what is left of it.
pub fn assert_rust_keeps_one_item() {
    let ticket = |tags: &[&str]| Ticket { tags: texts(tags) };
    assert_eq!(read("repeated tags"), Ok(ticket(&["a"])));
    assert_eq!(read("distinct tags"), Ok(ticket(&["a", "b"])));
    assert_eq!(
        read("held tags"),
        Ok(Folder {
            ticket: ticket(&["a"])
        })
    );
    assert_eq!(
        read::<EveryList>("counted"),
        Err(
            "'btree_set': too short: minimum length is 2, got 1; 'hash_set': too short: \
             minimum length is 2, got 1"
                .to_owned()
        )
    );
    let point = |x: u32, y: u32| Point { x, y };
    assert_eq!(
        read("nested repeated"),
        Ok(Nesting {
            grid: vec![HashSet::from([1, 2]), HashSet::from([3])],
            keyed: BTreeMap::from([("k".to_owned(), BTreeSet::from([4]))]),
            listed: HashSet::from([vec![1, 2]]),
            maybe: Some(BTreeSet::from([5])),
            points: BTreeSet::from([point(1, 2)]),
        })
    );
    assert_eq!(
        read("nested distinct"),
        Ok(Nesting {
            grid: vec![HashSet::from([1, 2])],
            keyed: BTreeMap::from([("k".to_owned(), BTreeSet::from([4]))]),
            listed: HashSet::from([vec![1, 2], vec![2, 1]]),
            maybe: None,
            points: BTreeSet::from([point(1, 2), point(2, 1)]),
        })
    );
    assert_eq!(read("brand"), Ok(Lanyard(texts(&["a"]))));
    assert_eq!(read("slot"), Ok(Pouch(BTreeSet::from([1]), 2)));
    assert_eq!(
        read("variant member"),
        Ok(Shelving::Loose {
            members: texts(&["a"])
        })
    );
    assert_eq!(
        read("variant slot"),
        Ok(Shelving::Bundle(BTreeSet::from([1])))
    );
    assert_eq!(
        read("untagged labels"),
        Ok(Sorting::Labels {
            labels: texts(&["a"])
        })
    );
    assert_eq!(
        read("untagged numbers"),
        Ok(Sorting::Numbers {
            numbers: BTreeSet::from([1])
        })
    );
    assert_eq!(
        read("filled"),
        Ok(Wallet {
            coins: BTreeSet::from([Coin { id: "a".to_owned() }])
        })
    );
}

/// Whether `read_as` names a type that declares a parameter, which a client reads at a string.
pub fn declares_a_parameter(read_as: &str) -> bool {
    read_as == "Wallet"
}
