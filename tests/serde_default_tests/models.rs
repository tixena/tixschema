//! The models a key under `#[serde(default)]` is written on, and what a serde read prints for each
//! payload of the table every surface is held to.

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use core::fmt::Write as _;

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

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

/// A default whose type names a parameter has no JSON, and one whose type names none has.
#[model_schema(default_types(Moored = String))]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Basin<Moored> {
    #[serde(default)]
    pub dock: String,
    #[serde(default)]
    pub moored: Vec<Moored>,
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

/// What `gate` is read as where the payload leaves its key out.
pub fn north_dock() -> String {
    "north".to_owned()
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

/// What serde reads of a `Marina` that holds `label` alone and of a `Basin` that holds no key:
/// each member its default, though no surface is handed the default of `Marina` or of `moored`.
pub fn unwritten_defaults() -> (Marina, Basin<String>) {
    (
        Marina {
            dock: String::new(),
            label: "m".to_owned(),
        },
        Basin {
            dock: String::new(),
            moored: Vec::new(),
        },
    )
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

/// What a serde read of `payload` as a `T` prints under `name`: the value written back, or each
/// message of the refusal on a line of its own.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
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
