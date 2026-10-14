//! The models a key under `#[serde(default)]` is written on, and what a serde read prints for each
//! payload of the table every surface is held to.

use alloc::collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque};
use core::fmt::Write as _;
use core::hash::Hash;
use std::collections::{HashMap, HashSet};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize, Serializer};
use tixschema::model_schema;

/// Each payload of a type that declares a parameter, read at the filling `String`: its name, the
/// type that reads it, and the payload.
pub const FILLED: [(&str, &str, &str); 11] = [
    ("row 1: no dataLinks", "Config", "{}"),
    (
        "row 9: dataLinks held",
        "Config",
        r#"{"dataLinks":[{"id":"a"}]}"#,
    ),
    ("row 2: every list and set left out", "Fleet", "{}"),
    (
        "row 2: every list and set held",
        "Fleet",
        r#"{"binary_heap":[{"id":"a"}],"btree_set":[{"id":"b"}],"hash_set":[{"id":"c"}],"vec":[{"id":"d"}],"vec_deque":[{"id":"e"}]}"#,
    ),
    ("row 3: every map left out", "Logbook", "{}"),
    (
        "row 3: every map held",
        "Logbook",
        r#"{"btree_map":{"k":{"id":"a"}},"hash_map":{"k":{"id":"b"}}}"#,
    ),
    ("a list of the parameter left out", "Basin", "{}"),
    (
        "a list of the parameter held",
        "Basin",
        r#"{"dock":"d","moored":["m"]}"#,
    ),
    (
        "rows 4 to 6: every key held",
        "Harbour",
        r#"{"berths":["b"],"flagship":"f","slips":["s"]}"#,
    ),
    (
        "a list of the parameter in a variant left out",
        "Manifest",
        r#"{"kind":"Listed","pier":3}"#,
    ),
    (
        "a list of the parameter in a variant held",
        "Manifest",
        r#"{"kind":"Listed","moored":["m"],"pier":3}"#,
    ),
];

/// What every surface prints for [`FILLED`], as the serde read prints it.
pub const FILLED_REPORT: &str = "\
row 1: no dataLinks: read {\"dataLinks\":[],\"sequences\":[]}
row 9: dataLinks held: read {\"dataLinks\":[{\"id\":\"a\"}],\"sequences\":[]}
row 2: every list and set left out: read {\"binary_heap\":[],\"btree_set\":[],\"hash_set\":[],\"vec\":[],\"vec_deque\":[]}
row 2: every list and set held: read {\"binary_heap\":[{\"id\":\"a\"}],\"btree_set\":[{\"id\":\"b\"}],\"hash_set\":[{\"id\":\"c\"}],\"vec\":[{\"id\":\"d\"}],\"vec_deque\":[{\"id\":\"e\"}]}
row 3: every map left out: read {\"btree_map\":{},\"hash_map\":{}}
row 3: every map held: read {\"btree_map\":{\"k\":{\"id\":\"a\"}},\"hash_map\":{\"k\":{\"id\":\"b\"}}}
a list of the parameter left out: read {\"dock\":\"\",\"moored\":[]}
a list of the parameter held: read {\"dock\":\"d\",\"moored\":[\"m\"]}
rows 4 to 6: every key held: read {\"berths\":[\"b\"],\"flagship\":\"f\",\"slips\":[\"s\"]}
a list of the parameter in a variant left out: read {\"kind\":\"Listed\",\"moored\":[],\"pier\":3}
a list of the parameter in a variant held: read {\"kind\":\"Listed\",\"moored\":[\"m\"],\"pier\":3}
";

/// Each payload of the `indexmap` set and map, read at the filling `String`.
#[cfg(feature = "indexmap")]
pub const INDEXED: &[(&str, &str, &str)] = &[
    ("row 2: an IndexSet left out", "IndexedFleet", "{}"),
    (
        "row 2: an IndexSet held",
        "IndexedFleet",
        r#"{"index_set":[{"id":"a"}]}"#,
    ),
    ("row 3: an IndexMap left out", "IndexedLogbook", "{}"),
    (
        "row 3: an IndexMap held",
        "IndexedLogbook",
        r#"{"index_map":{"k":{"id":"b"}}}"#,
    ),
];

/// What every surface prints for [`INDEXED`], as the serde read prints it.
#[cfg(feature = "indexmap")]
pub const INDEXED_REPORT: &str = "\
row 2: an IndexSet left out: read {\"index_set\":[]}
row 2: an IndexSet held: read {\"index_set\":[{\"id\":\"a\"}]}
row 3: an IndexMap left out: read {\"index_map\":{}}
row 3: an IndexMap held: read {\"index_map\":{\"k\":{\"id\":\"b\"}}}
";

/// No `indexmap` payload where the feature is off.
#[cfg(not(feature = "indexmap"))]
pub const INDEXED: &[(&str, &str, &str)] = &[];

/// Nothing printed for [`INDEXED`] where the feature is off.
#[cfg(not(feature = "indexmap"))]
pub const INDEXED_REPORT: &str = "";

/// Each payload of `Harbour` that leaves out a key whose default has no JSON: serde reads it as
/// the default, and every other surface requires the key.
pub const REQUIRED: [(&str, &str); 3] = [
    ("row 4: no flagship", r#"{"berths":["b"],"slips":["s"]}"#),
    ("row 5: no berths", r#"{"flagship":"f","slips":["s"]}"#),
    ("row 6: no slips", r#"{"berths":["b"],"flagship":"f"}"#),
];

/// What every surface but Rust prints for [`REQUIRED`].
pub const REQUIRED_REPORT: &str = "\
row 4: no flagship: refused
row 5: no berths: refused
row 6: no slips: refused
";

/// Each payload every surface reads: its name, the type that reads it, and the payload.
pub const CASES: [(&str, &str, &str); 10] = [
    (
        "row 1: no dock",
        "Wharf",
        r#"{"gangway":{"label":"side","width":1},"gate":"south","label":"w"}"#,
    ),
    (
        "row 2: no gate",
        "Wharf",
        r#"{"dock":"d1","gangway":{"label":"side","width":1},"label":"w"}"#,
    ),
    (
        "a type of the consumer's own: no gangway",
        "Wharf",
        r#"{"dock":"d1","gate":"south","label":"w"}"#,
    ),
    ("every default at once", "Wharf", r#"{"label":"w"}"#),
    ("row 3: the container's default", "Jetty", "{}"),
    ("row 3: one key held", "Jetty", r#"{"dock":"west"}"#),
    (
        "row 4: every key held",
        "Wharf",
        r#"{"dock":"d1","gangway":{"label":"side","width":1},"gate":"south","label":"w"}"#,
    ),
    ("row 4: a key held empty", "Jetty", r#"{"moorings":[]}"#),
    (
        "row 5: a default that breaks a bound",
        "Pontoon",
        r#"{"label":"p"}"#,
    ),
    (
        "row 5: the key held",
        "Pontoon",
        r#"{"dock":"d","label":"p"}"#,
    ),
];

/// What every surface prints for [`CASES`], as the serde read prints it where a schema surface is
/// on: a build with none holds the member of row 5 to no bound in Rust.
pub const REPORT: &str = "\
row 1: no dock: read {\"dock\":\"\",\"gangway\":{\"label\":\"side\",\"width\":1},\"gate\":\"south\",\"label\":\"w\"}
row 2: no gate: read {\"dock\":\"d1\",\"gangway\":{\"label\":\"side\",\"width\":1},\"gate\":\"north\",\"label\":\"w\"}
a type of the consumer's own: no gangway: read {\"dock\":\"d1\",\"gangway\":{\"label\":\"main\",\"width\":2},\"gate\":\"south\",\"label\":\"w\"}
every default at once: read {\"dock\":\"\",\"gangway\":{\"label\":\"main\",\"width\":2},\"gate\":\"north\",\"label\":\"w\"}
row 3: the container's default: read {\"berths\":4,\"dock\":\"east\",\"moorings\":[\"a\",\"b\"]}
row 3: one key held: read {\"berths\":4,\"dock\":\"west\",\"moorings\":[\"a\",\"b\"]}
row 4: every key held: read {\"dock\":\"d1\",\"gangway\":{\"label\":\"side\",\"width\":1},\"gate\":\"south\",\"label\":\"w\"}
row 4: a key held empty: read {\"berths\":4,\"dock\":\"east\",\"moorings\":[]}
row 5: a default that breaks a bound: refused
    'dock': too short: minimum length is 1, got 0
row 5: the key held: read {\"dock\":\"d\",\"label\":\"p\"}
";

/// Each payload of an enum a member of whose variant is written under `#[serde(default)]`, under
/// each of the four taggings, and of a struct that holds the untagged one: its name, the type that
/// reads it, and the payload. An untagged enum takes no payload whose default breaks a bound: the
/// bound chooses its variant.
pub const VARIANT_CASES: [(&str, &str, &str); 21] = [
    ("tag: no dock", "Berthing", r#"{"kind":"Docked","pier":3}"#),
    (
        "tag: dock held",
        "Berthing",
        r#"{"kind":"Docked","dock":"d","pier":3}"#,
    ),
    (
        "tag: no dock, read from a function",
        "Berthing",
        r#"{"kind":"Moored","line":"l"}"#,
    ),
    (
        "tag: a default that breaks a bound",
        "Berthing",
        r#"{"kind":"Moored","dock":"d"}"#,
    ),
    ("tag: a unit variant", "Berthing", r#"{"kind":"Adrift"}"#),
    ("no tag: no dock", "ExternalBerthing", r#"{"Docked":{"pier":3}}"#),
    (
        "no tag: dock held",
        "ExternalBerthing",
        r#"{"Docked":{"dock":"d","pier":3}}"#,
    ),
    (
        "no tag: no dock, read from a function",
        "ExternalBerthing",
        r#"{"Moored":{"line":"l"}}"#,
    ),
    (
        "no tag: a default that breaks a bound",
        "ExternalBerthing",
        r#"{"Moored":{"dock":"d"}}"#,
    ),
    ("no tag: a unit variant", "ExternalBerthing", r#""Adrift""#),
    (
        "tag and content: no dock",
        "AdjacentBerthing",
        r#"{"kind":"Docked","data":{"pier":3}}"#,
    ),
    (
        "tag and content: dock held",
        "AdjacentBerthing",
        r#"{"kind":"Docked","data":{"dock":"d","pier":3}}"#,
    ),
    (
        "tag and content: no dock, read from a function",
        "AdjacentBerthing",
        r#"{"kind":"Moored","data":{"line":"l"}}"#,
    ),
    (
        "tag and content: a default that breaks a bound",
        "AdjacentBerthing",
        r#"{"kind":"Moored","data":{"dock":"d"}}"#,
    ),
    (
        "tag and content: a unit variant",
        "AdjacentBerthing",
        r#"{"kind":"Adrift"}"#,
    ),
    ("untagged: no dock", "UntaggedBerthing", r#"{"pier":3}"#),
    (
        "untagged: dock held",
        "UntaggedBerthing",
        r#"{"dock":"d","pier":3}"#,
    ),
    (
        "untagged: no dock, read from a function",
        "UntaggedBerthing",
        r#"{"line":"l"}"#,
    ),
    (
        "untagged: every key held",
        "UntaggedBerthing",
        r#"{"dock":"d","line":"l"}"#,
    ),
    (
        "a holder of the untagged enum: no dock",
        "Quay",
        r#"{"berth":{"pier":3},"label":"q"}"#,
    ),
    (
        "a holder of the untagged enum: dock held",
        "Quay",
        r#"{"berth":{"dock":"d","pier":3},"label":"q"}"#,
    ),
];

/// What every surface prints for [`VARIANT_CASES`], as the serde read prints it where a schema
/// surface is on: a build with none holds `line` to no bound in Rust.
pub const VARIANT_REPORT: &str = "\
tag: no dock: read {\"kind\":\"Docked\",\"dock\":\"\",\"pier\":3}
tag: dock held: read {\"kind\":\"Docked\",\"dock\":\"d\",\"pier\":3}
tag: no dock, read from a function: read {\"kind\":\"Moored\",\"dock\":\"north\",\"line\":\"l\"}
tag: a default that breaks a bound: refused
    'line': too short: minimum length is 1, got 0
tag: a unit variant: read {\"kind\":\"Adrift\"}
no tag: no dock: read {\"Docked\":{\"dock\":\"\",\"pier\":3}}
no tag: dock held: read {\"Docked\":{\"dock\":\"d\",\"pier\":3}}
no tag: no dock, read from a function: read {\"Moored\":{\"dock\":\"north\",\"line\":\"l\"}}
no tag: a default that breaks a bound: refused
    'Moored.line': too short: minimum length is 1, got 0
no tag: a unit variant: read \"Adrift\"
tag and content: no dock: read {\"kind\":\"Docked\",\"data\":{\"dock\":\"\",\"pier\":3}}
tag and content: dock held: read {\"kind\":\"Docked\",\"data\":{\"dock\":\"d\",\"pier\":3}}
tag and content: no dock, read from a function: read {\"kind\":\"Moored\",\"data\":{\"dock\":\"north\",\"line\":\"l\"}}
tag and content: a default that breaks a bound: refused
    'data.line': too short: minimum length is 1, got 0
tag and content: a unit variant: read {\"kind\":\"Adrift\"}
untagged: no dock: read {\"dock\":\"\",\"pier\":3}
untagged: dock held: read {\"dock\":\"d\",\"pier\":3}
untagged: no dock, read from a function: read {\"dock\":\"north\",\"line\":\"l\"}
untagged: every key held: read {\"dock\":\"d\",\"line\":\"l\"}
a holder of the untagged enum: no dock: read {\"berth\":{\"dock\":\"\",\"pier\":3},\"label\":\"q\"}
a holder of the untagged enum: dock held: read {\"berth\":{\"dock\":\"d\",\"pier\":3},\"label\":\"q\"}
";

/// Each payload of `Bollard` every client reads, by name.
pub const PREPROCESSED: [(&str, &str); 2] = [
    ("a default no function is called for", "{}"),
    ("a key its function corrects", r#"{"dock":"  east  "}"#),
];

/// What serde and every client print for [`PREPROCESSED`]: the default is read as it is, where
/// the Zod schema hands it to the function.
pub const PREPROCESSED_REPORT: &str = "\
a default no function is called for: read {\"dock\":\"  spare  \"}
a key its function corrects: read {\"dock\":\"east\"}
";

/// A member of a variant read as its default under a tag: `dock` as the `Default` of its type in
/// one variant and as what a function answers in another, beside a member whose default breaks
/// its bound.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "kind")]
pub enum Berthing {
    Adrift,
    Docked {
        #[serde(default)]
        dock: String,
        pier: u32,
    },
    Moored {
        #[serde(default = "north_dock")]
        dock: String,
        #[model_schema_prop(minLength = 1)]
        #[serde(default)]
        line: String,
    },
}

/// [`Berthing`] under no tag attribute.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum ExternalBerthing {
    Adrift,
    Docked {
        #[serde(default)]
        dock: String,
        pier: u32,
    },
    Moored {
        #[serde(default = "north_dock")]
        dock: String,
        #[model_schema_prop(minLength = 1)]
        #[serde(default)]
        line: String,
    },
}

/// [`Berthing`] under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "data")]
pub enum AdjacentBerthing {
    Adrift,
    Docked {
        #[serde(default)]
        dock: String,
        pier: u32,
    },
    Moored {
        #[serde(default = "north_dock")]
        dock: String,
        #[model_schema_prop(minLength = 1)]
        #[serde(default)]
        line: String,
    },
}

/// [`Berthing`] untagged, with no unit variant, which an untagged enum takes none of.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum UntaggedBerthing {
    Docked {
        #[serde(default)]
        dock: String,
        pier: u32,
    },
    Moored {
        #[serde(default = "north_dock")]
        dock: String,
        #[model_schema_prop(minLength = 1)]
        #[serde(default)]
        line: String,
    },
}

/// A struct that holds the untagged enum under a key of its own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Quay {
    pub berth: UntaggedBerthing,
    pub label: String,
}

/// A list of the enum's parameter in a variant, read as the empty list at every filling.
#[model_schema(default_types(Moored = String))]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "kind")]
pub enum Manifest<Moored> {
    Cleared,
    Listed {
        #[serde(default)]
        moored: Vec<Moored>,
        pier: u32,
    },
}

/// A gangway, a type of the consumer's own with a `Default` of its own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Gangway {
    pub label: String,
    pub width: u32,
}

/// A key read as its type's default, one read as what a function answers, and one read as the
/// default of a type of the consumer's own, beside a key the payload must hold.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Wharf {
    #[serde(default)]
    pub dock: String,
    #[serde(default)]
    pub gangway: Gangway,
    #[serde(default = "north_dock")]
    pub gate: String,
    pub label: String,
}

/// Every key read as the member of the container's own default.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct Jetty {
    pub berths: u32,
    pub dock: String,
    pub moorings: Vec<String>,
}

/// A default that breaks the bound of its member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Pontoon {
    #[model_schema_prop(minLength = 1)]
    #[serde(default)]
    pub dock: String,
    pub label: String,
}

/// A key under `#[serde(default)]` whose member names a `preprocess`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Bollard {
    #[model_schema_prop(preprocess = ["trimmedDock"])]
    #[serde(default = "spare_dock")]
    pub dock: String,
}

/// A default with no JSON: the type writes no `Serialize`, so no surface is handed one.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Marina {
    #[serde(default)]
    pub dock: String,
    pub label: String,
}

/// A list of the struct's parameter, whose default is the empty list at every filling, beside a
/// member whose type names no parameter.
#[model_schema(default_types(Moored = String))]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Basin<Moored> {
    #[serde(default)]
    pub dock: String,
    #[serde(default)]
    pub moored: Vec<Moored>,
}

/// Defaults whose JSON depends on the filling, or that a function of the author's writes.
#[model_schema(default_types(Moored = String))]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Harbour<Moored> {
    #[serde(default = "two_berths")]
    pub berths: Vec<Moored>,
    #[serde(default)]
    pub flagship: Moored,
    #[serde(default, serialize_with = "listed_slips")]
    pub slips: Vec<Moored>,
}

/// A model a holder fills with its own type parameter.
#[model_schema(default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct Link<IdType> {
    pub id: IdType,
}

/// A list of a model filled with the struct's parameter, read as empty where the payload leaves
/// its key out, beside a list of a type that names no parameter.
#[model_schema(default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Config<IdType> {
    #[serde(default)]
    pub data_links: Vec<Link<IdType>>,
    #[serde(default)]
    pub sequences: Vec<String>,
}

/// Every list and set of a model filled with the struct's parameter, each read as `[]` where the
/// payload leaves its key out.
#[model_schema(default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Fleet<IdType: Hash + Ord> {
    #[serde(default)]
    pub binary_heap: BinaryHeap<Link<IdType>>,
    #[serde(default)]
    pub btree_set: BTreeSet<Link<IdType>>,
    #[serde(default)]
    pub hash_set: HashSet<Link<IdType>>,
    #[serde(default)]
    pub vec: Vec<Link<IdType>>,
    #[serde(default)]
    pub vec_deque: VecDeque<Link<IdType>>,
}

/// Every map of a model filled with the struct's parameter, each read as `{}` where the payload
/// leaves its key out.
#[model_schema(default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Logbook<IdType> {
    #[serde(default)]
    pub btree_map: BTreeMap<String, Link<IdType>>,
    #[serde(default)]
    pub hash_map: HashMap<String, Link<IdType>>,
}

/// The `indexmap` set of a model filled with the struct's parameter.
#[cfg(feature = "indexmap")]
#[model_schema(default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IndexedFleet<IdType: Hash + Eq> {
    #[serde(default)]
    pub index_set: indexmap::IndexSet<Link<IdType>>,
}

/// The `indexmap` map of a model filled with the struct's parameter.
#[cfg(feature = "indexmap")]
#[model_schema(default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IndexedLogbook<IdType> {
    #[serde(default)]
    pub index_map: indexmap::IndexMap<String, Link<IdType>>,
}

impl Default for Gangway {
    fn default() -> Self {
        Self {
            label: "main".to_owned(),
            width: 2,
        }
    }
}

impl Default for Jetty {
    fn default() -> Self {
        Self {
            berths: 4,
            dock: "east".to_owned(),
            moorings: vec!["a".to_owned(), "b".to_owned()],
        }
    }
}

/// What `Harbour` writes its slips with: a function of the author's own.
pub fn listed_slips<Moored, S>(slips: &[Moored], serializer: S) -> Result<S::Ok, S::Error>
where
    Moored: Serialize,
    S: Serializer,
{
    serializer.collect_seq(slips)
}

/// What `gate` is read as where the payload leaves its key out.
pub fn north_dock() -> String {
    "north".to_owned()
}

/// What `Harbour` reads its berths as where the payload leaves their key out: one value for each
/// filling.
pub fn two_berths<Moored>() -> Vec<Moored>
where
    Moored: Default,
{
    vec![Moored::default(), Moored::default()]
}

/// What `Bollard` reads where the payload leaves `dock` out.
pub fn spare_dock() -> String {
    "  spare  ".to_owned()
}

/// The function `Bollard` names, in the snake case Rust calls.
pub fn trimmed_dock(payload: serde_json::Value) -> serde_json::Value {
    if let serde_json::Value::String(text) = payload {
        serde_json::Value::String(text.trim().to_owned())
    } else {
        payload
    }
}

/// What serde reads of each payload of [`PREPROCESSED`], in its order.
pub fn preprocessed_values() -> [Bollard; 2] {
    [
        Bollard { dock: spare_dock() },
        Bollard {
            dock: "east".to_owned(),
        },
    ]
}

/// What serde reads of a `Wharf` that holds `label` alone, every other member its default, and of
/// a `Pontoon` that holds both keys.
pub fn read_values() -> (Wharf, Pontoon) {
    (
        Wharf {
            dock: String::new(),
            gangway: Gangway::default(),
            gate: north_dock(),
            label: "w".to_owned(),
        },
        Pontoon {
            dock: "d".to_owned(),
            label: "p".to_owned(),
        },
    )
}

/// What serde reads of a `Marina` that holds `label` alone, and of each payload of [`REQUIRED`]:
/// each member left out its default, though no surface is handed the default of `Marina` or of
/// `Harbour`.
pub fn unwritten_defaults() -> (Marina, [Harbour<String>; 3]) {
    let held = |text: &str| vec![text.to_owned()];
    (
        Marina {
            dock: String::new(),
            label: "m".to_owned(),
        },
        [
            Harbour {
                berths: held("b"),
                flagship: String::new(),
                slips: held("s"),
            },
            Harbour {
                berths: two_berths(),
                flagship: "f".to_owned(),
                slips: held("s"),
            },
            Harbour {
                berths: held("b"),
                flagship: "f".to_owned(),
                slips: Vec::new(),
            },
        ],
    )
}

/// Every payload of [`FILLED`], then of [`INDEXED`].
pub fn filled_cases() -> Vec<(&'static str, &'static str, &'static str)> {
    FILLED.iter().chain(INDEXED).copied().collect()
}

/// What every surface prints for [`filled_cases`].
pub fn filled_expected() -> String {
    format!("{FILLED_REPORT}{INDEXED_REPORT}")
}

/// What a serde read prints for each payload of [`filled_cases`].
pub fn filled_report() -> String {
    filled_cases()
        .into_iter()
        .map(|(name, read_as, payload)| match read_as {
            "Basin" => printed::<Basin<String>>(name, payload),
            "Config" => printed::<Config<String>>(name, payload),
            "Fleet" => printed::<Fleet<String>>(name, payload),
            #[cfg(feature = "indexmap")]
            "IndexedFleet" => printed::<IndexedFleet<String>>(name, payload),
            #[cfg(feature = "indexmap")]
            "IndexedLogbook" => printed::<IndexedLogbook<String>>(name, payload),
            "Logbook" => printed::<Logbook<String>>(name, payload),
            "Manifest" => printed::<Manifest<String>>(name, payload),
            _ => printed::<Harbour<String>>(name, payload),
        })
        .collect()
}

/// What a serde read prints for each payload of [`CASES`].
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
pub fn report() -> String {
    CASES
        .iter()
        .map(|&(name, read_as, payload)| match read_as {
            "Wharf" => printed::<Wharf>(name, payload),
            "Jetty" => printed::<Jetty>(name, payload),
            _ => printed::<Pontoon>(name, payload),
        })
        .collect()
}

/// What a serde read prints for each payload of [`VARIANT_CASES`].
pub fn variant_report() -> String {
    VARIANT_CASES
        .iter()
        .map(|&(name, read_as, payload)| match read_as {
            "Berthing" => printed::<Berthing>(name, payload),
            "ExternalBerthing" => printed::<ExternalBerthing>(name, payload),
            "AdjacentBerthing" => printed::<AdjacentBerthing>(name, payload),
            "UntaggedBerthing" => printed::<UntaggedBerthing>(name, payload),
            _ => printed::<Quay>(name, payload),
        })
        .collect()
}

/// What a serde read of `payload` as a `T` prints under `name`: the value written back, or each
/// message of the refusal on a line of its own.
fn printed<T>(name: &str, payload: &str) -> String
where
    T: DeserializeOwned + Serialize,
{
    match serde_json::from_str::<T>(payload) {
        Ok(read) => format!("{name}: read {}\n", serde_json::to_string(&read).unwrap()),
        Err(refused) => {
            let mut said = format!("{name}: refused\n");
            for message in refused.to_string().split("; ") {
                let _ = writeln!(said, "    {message}");
            }
            said
        }
    }
}
