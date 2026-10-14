//! What the Swift backend writes for a std sequence other than `Vec`: the array serde writes for
//! it, declared, read, written and walked by `validate()` as a `Vec` is. Each type is held to the
//! text of its twin written with `Vec`, the twin's name read as its own. What a read answers is
//! run under a Swift toolchain in `service_schema_emitted_client_tests/run_swift_sequences.rs`.

use alloc::collections::{BTreeSet, BinaryHeap, VecDeque};
use std::collections::{HashMap, HashSet};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Each std sequence but `Vec`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Lone {
    pub binary_heap: BinaryHeap<String>,
    pub btree_set: BTreeSet<String>,
    pub hash_set: HashSet<String>,
    pub vec_deque: VecDeque<u32>,
}

/// [`Lone`] written with `Vec`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LoneAsVec {
    pub binary_heap: Vec<String>,
    pub btree_set: Vec<String>,
    pub hash_set: Vec<String>,
    pub vec_deque: Vec<u32>,
}

/// The `indexmap` set.
#[cfg(feature = "indexmap")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IndexedLone {
    pub index_set: indexmap::IndexSet<String>,
}

/// [`IndexedLone`] written with `Vec`.
#[cfg(feature = "indexmap")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IndexedLoneAsVec {
    pub index_set: Vec<String>,
}

/// A model with a bound, held in a sequence.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Buoy {
    #[model_schema_prop(minLength = 1)]
    pub mark: String,
}

/// A model a holder fills with its own parameter, and with a sequence.
#[model_schema(default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Tether<IdType> {
    pub id: IdType,
}

/// A sequence in a list and of a list, under `Option`, as a map's value, in a tuple and of tuples,
/// as a type argument, and of a model with a bound.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Bundled {
    pub buoys: VecDeque<Buoy>,
    pub buoys_by_dock: HashMap<String, BTreeSet<Buoy>>,
    pub by_dock: HashMap<String, HashSet<String>>,
    pub grid: Vec<HashSet<String>>,
    pub layered: BTreeSet<BTreeSet<u32>>,
    pub listed: HashSet<Vec<String>>,
    pub pair: (VecDeque<String>, u8),
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare: Option<BTreeSet<u32>>,
    pub stowed: (VecDeque<(String, Buoy)>, u8),
    pub tether: Tether<HashSet<String>>,
}

/// [`Bundled`] written with `Vec`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BundledAsVec {
    pub buoys: Vec<Buoy>,
    pub buoys_by_dock: HashMap<String, Vec<Buoy>>,
    pub by_dock: HashMap<String, Vec<String>>,
    pub grid: Vec<Vec<String>>,
    pub layered: Vec<Vec<u32>>,
    pub listed: Vec<Vec<String>>,
    pub pair: (Vec<String>, u8),
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare: Option<Vec<u32>>,
    pub stowed: (Vec<(String, Buoy)>, u8),
    pub tether: Tether<Vec<String>>,
}

/// A sequence of a model filled with the struct's parameter.
#[model_schema(default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Tethered<IdType> {
    pub queue: VecDeque<Tether<IdType>>,
}

/// [`Tethered`] written with `Vec`.
#[model_schema(default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TetheredAsVec<IdType> {
    pub queue: Vec<Tether<IdType>>,
}

/// A sequence as a variant's member, as its one slot and as one of its slots.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum Hauling {
    Loose { members: HashSet<String> },
    Queue(VecDeque<String>),
    Split(BTreeSet<u32>, u8),
}

/// [`Hauling`] written with `Vec`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum HaulingAsVec {
    Loose { members: Vec<String> },
    Queue(Vec<String>),
    Split(Vec<u32>, u8),
}

/// A sequence as a slot of a tuple struct.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Queued(pub VecDeque<String>, pub u8);

/// [`Queued`] written with `Vec`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct QueuedAsVec(pub Vec<String>, pub u8);

/// A brand over a sequence, its items counted.
#[model_schema(no_display, minLength = 1)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Crew(pub HashSet<String>);

/// [`Crew`] written with `Vec`.
#[model_schema(no_display, minLength = 1)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct CrewAsVec(pub Vec<String>);

/// An alias of a sequence.
#[model_schema()]
pub type Roster = BTreeSet<String>;

/// [`Roster`] written with `Vec`.
#[model_schema()]
pub type RosterAsVec = Vec<String>;

/// `swift` with every `named` read as one name.
fn named_as(swift: &str, named: &str) -> String {
    swift.replace(named, "Held")
}

/// The text of the type written with sequences, and of its twin written with `Vec`, each name
/// read as one.
fn twins(sequences: (&str, &str), vectors: (&str, &str)) -> (String, String) {
    (
        named_as(sequences.0, sequences.1),
        named_as(vectors.0, vectors.1),
    )
}

/// The JSON serde writes for `held`, read back by serde as it was written.
fn read_back<Held>(held: &Held) -> String
where
    Held: DeserializeOwned + Serialize,
{
    let written = serde_json::to_string(held).unwrap();
    let read: Held = serde_json::from_str(&written).unwrap();
    assert_eq!(serde_json::to_string(&read).unwrap(), written);
    written
}

/// Each text, collected into whichever sequence the position holds.
fn texts<Held>(each: &[&str]) -> Held
where
    Held: FromIterator<String>,
{
    each.iter().map(|&text| text.to_owned()).collect()
}

/// A buoy with `mark`.
fn buoy(mark: &str) -> Buoy {
    Buoy {
        mark: mark.to_owned(),
    }
}

#[test]
fn each_fixture_writes_the_json_its_twin_written_with_vec_writes() {
    assert_eq!(
        read_back(&Lone {
            binary_heap: texts(&["a"]),
            btree_set: texts(&["b"]),
            hash_set: texts(&["c"]),
            vec_deque: VecDeque::from([1]),
        }),
        read_back(&LoneAsVec {
            binary_heap: texts(&["a"]),
            btree_set: texts(&["b"]),
            hash_set: texts(&["c"]),
            vec_deque: vec![1],
        })
    );
    #[cfg(feature = "indexmap")]
    assert_eq!(
        read_back(&IndexedLone {
            index_set: texts(&["a"]),
        }),
        read_back(&IndexedLoneAsVec {
            index_set: texts(&["a"]),
        })
    );
    assert_eq!(
        read_back(&Bundled {
            buoys: VecDeque::from([buoy("a")]),
            buoys_by_dock: HashMap::from([("d".to_owned(), BTreeSet::from([buoy("b")]))]),
            by_dock: HashMap::from([("d".to_owned(), texts(&["c"]))]),
            grid: vec![texts(&["e"])],
            layered: BTreeSet::from([BTreeSet::from([1])]),
            listed: HashSet::from([texts(&["f"])]),
            pair: (texts(&["g"]), 1),
            spare: Some(BTreeSet::from([2])),
            stowed: (VecDeque::from([("h".to_owned(), buoy("i"))]), 2),
            tether: Tether {
                id: texts(&["j"]),
            },
        }),
        read_back(&BundledAsVec {
            buoys: vec![buoy("a")],
            buoys_by_dock: HashMap::from([("d".to_owned(), vec![buoy("b")])]),
            by_dock: HashMap::from([("d".to_owned(), texts(&["c"]))]),
            grid: vec![texts(&["e"])],
            layered: vec![vec![1]],
            listed: vec![texts(&["f"])],
            pair: (texts(&["g"]), 1),
            spare: Some(vec![2]),
            stowed: (vec![("h".to_owned(), buoy("i"))], 2),
            tether: Tether {
                id: texts(&["j"]),
            },
        })
    );
    assert_eq!(
        read_back(&Tethered {
            queue: VecDeque::from([Tether { id: "k".to_owned() }]),
        }),
        read_back(&TetheredAsVec {
            queue: vec![Tether { id: "k".to_owned() }],
        })
    );
    for (held, as_vec) in [
        (
            Hauling::Loose {
                members: texts(&["a"]),
            },
            HaulingAsVec::Loose {
                members: texts(&["a"]),
            },
        ),
        (
            Hauling::Queue(texts(&["b"])),
            HaulingAsVec::Queue(texts(&["b"])),
        ),
        (
            Hauling::Split(BTreeSet::from([1]), 2),
            HaulingAsVec::Split(vec![1], 2),
        ),
    ] {
        assert_eq!(read_back(&held), read_back(&as_vec));
    }
    assert_eq!(
        read_back(&Queued(texts(&["a"]), 1)),
        read_back(&QueuedAsVec(texts(&["a"]), 1))
    );
    assert_eq!(
        read_back(&Crew(texts(&["a"]))),
        read_back(&CrewAsVec(texts(&["a"])))
    );
    let roster: Roster = texts(&["a"]);
    let roster_as_vec: RosterAsVec = texts(&["a"]);
    assert_eq!(read_back(&roster), read_back(&roster_as_vec));
}

#[test]
fn each_sequence_is_declared_an_array() {
    let swift = lone_swift::swift_definition();
    for declared in [
        "public let binaryHeap: [String]",
        "public let btreeSet: [String]",
        "public let hashSet: [String]",
        "public let vecDeque: [UInt32]",
    ] {
        assert!(swift.contains(declared), "{declared} in: {swift}");
    }
    assert!(
        swift.contains("self.binaryHeap = try container.decode([String].self, forKey: .binaryHeap)"),
        "{swift}"
    );
}

#[test]
fn each_sequence_is_written_as_a_vec_is() {
    let (held, as_vec) = twins(
        (&lone_swift::swift_definition(), "Lone"),
        (&lone_as_vec_swift::swift_definition(), "LoneAsVec"),
    );
    assert_eq!(held, as_vec);
}

#[cfg(feature = "indexmap")]
#[test]
fn an_index_set_is_written_as_a_vec_is() {
    let swift = indexed_lone_swift::swift_definition();
    assert!(swift.contains("public let indexSet: [String]"), "{swift}");
    let (held, as_vec) = twins(
        (&swift, "IndexedLone"),
        (
            &indexed_lone_as_vec_swift::swift_definition(),
            "IndexedLoneAsVec",
        ),
    );
    assert_eq!(held, as_vec);
}

#[test]
fn a_sequence_nested_optional_held_and_of_a_model_is_written_as_a_vec_is() {
    let swift = bundled_swift::swift_definition();
    for declared in [
        "public let buoys: [Buoy]",
        "public let buoysByDock: [String: [Buoy]]",
        "public let byDock: [String: [String]]",
        "let slot0: [BundledStowedTupleSlot0Tuple]",
        "public let grid: [[String]]",
        "public let layered: [[UInt32]]",
        "public let listed: [[String]]",
        "public let spare: [UInt32]?",
        "public let tether: Tether<[String]>",
        "let slot0: [String]",
    ] {
        assert!(swift.contains(declared), "{declared} in: {swift}");
    }
    let (held, as_vec) = twins(
        (&swift, "Bundled"),
        (&bundled_as_vec_swift::swift_definition(), "BundledAsVec"),
    );
    assert_eq!(held, as_vec);
}

#[test]
fn a_sequence_of_a_model_filled_with_a_parameter_is_written_as_a_vec_is() {
    let swift = tethered_swift::swift_definition();
    assert!(
        swift.contains("public let queue: [Tether<IdType>]"),
        "{swift}"
    );
    let (held, as_vec) = twins(
        (&swift, "Tethered"),
        (&tethered_as_vec_swift::swift_definition(), "TetheredAsVec"),
    );
    assert_eq!(held, as_vec);
}

#[test]
fn a_sequence_in_a_variant_a_slot_a_brand_and_an_alias_is_written_as_a_vec_is() {
    let pairs = [
        (
            (hauling_swift::swift_definition(), "Hauling"),
            (hauling_as_vec_swift::swift_definition(), "HaulingAsVec"),
        ),
        (
            (queued_swift::swift_definition(), "Queued"),
            (queued_as_vec_swift::swift_definition(), "QueuedAsVec"),
        ),
        (
            (crew_swift::swift_definition(), "Crew"),
            (crew_as_vec_swift::swift_definition(), "CrewAsVec"),
        ),
        (
            (roster_swift::swift_definition(), "Roster"),
            (roster_as_vec_swift::swift_definition(), "RosterAsVec"),
        ),
    ];
    for ((swift, named), (vector, vector_named)) in &pairs {
        let (held, as_vec) = twins((swift, named), (vector, vector_named));
        assert_eq!(held, as_vec);
    }
}
