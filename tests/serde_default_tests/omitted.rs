//! A list under `#[serde(default)]` that serde leaves out of what it writes: the models, and what
//! every client prints for each payload.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Each payload of [`Roster`], [`Muster`] and [`Squad`] every surface reads: its name, the type that reads
/// it, and the payload.
pub const OMITTED: [(&str, &str, &str); 6] = [
    ("roles left out", "Roster", r#"{"id":"1"}"#),
    ("roles held", "Roster", r#"{"id":"1","roles":["a"]}"#),
    ("roles left out, never written", "Muster", r#"{"id":"1"}"#),
    (
        "roles held, never written",
        "Muster",
        r#"{"id":"1","roles":["a"]}"#,
    ),
    ("roles left out, under the whole", "Squad", r#"{"id":"1"}"#),
    (
        "roles held, under the whole",
        "Squad",
        r#"{"id":"1","roles":["a"]}"#,
    ),
];

/// What the Zod schema prints for [`OMITTED`]: the value read, with every member it holds.
pub const OMITTED_REPORT: &str = "\
roles left out: read {\"id\":\"1\",\"roles\":[]}
roles held: read {\"id\":\"1\",\"roles\":[\"a\"]}
roles left out, never written: read {\"id\":\"1\",\"roles\":[]}
roles held, never written: read {\"id\":\"1\",\"roles\":[\"a\"]}
roles left out, under the whole: read {\"id\":\"1\",\"roles\":[\"guest\"]}
roles held, under the whole: read {\"id\":\"1\",\"roles\":[\"a\"]}
";

/// What each emitted client prints for [`OMITTED`]: the value read, written back. A client writes
/// the member it reads, and never the one serde never writes.
#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
pub const OMITTED_WRITTEN: &str = "\
roles left out: read {\"id\":\"1\",\"roles\":[]}
roles held: read {\"id\":\"1\",\"roles\":[\"a\"]}
roles left out, never written: read {\"id\":\"1\"}
roles held, never written: read {\"id\":\"1\"}
roles left out, under the whole: read {\"id\":\"1\",\"roles\":[\"guest\"]}
roles held, under the whole: read {\"id\":\"1\",\"roles\":[\"a\"]}
";

/// A list read as `[]` where the payload leaves its key out, and left out of what serde writes
/// where it is empty.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Roster {
    pub id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
}

/// A list read as `[]` where the payload leaves its key out, and never written by serde.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Muster {
    pub id: String,
    #[serde(default, skip_serializing)]
    pub roles: Vec<String>,
}

/// A list under `skip_serializing_if` with no `default` of its own, in a struct whose `default`
/// builds `["guest"]`: serde reads a key left out as the whole's value of the member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct Squad {
    pub id: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
}

impl Default for Squad {
    fn default() -> Self {
        Self {
            id: String::new(),
            roles: vec!["guest".to_owned()],
        }
    }
}
