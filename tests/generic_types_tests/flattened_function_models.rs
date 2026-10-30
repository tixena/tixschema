//! The holders of two flattened types whose functions list the keys they are handed, and what a
//! Rust read and the Zod schemas print for them.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::model_schema;

use super::corrected::{Pinned, Position, position, with_its_keys};

/// What the Rust read and the Zod schemas print for the holders of flattened types whose functions
/// list the keys they are handed.
#[cfg(feature = "serde")]
pub const FLATTENED_POSITIONS: &str = "\
a flattened type that reads its keys: read {\"dock\":\"north\",\"keys\":[\"tag\"],\"tag\":\"t\"}
two flattened types that read their keys: read {\"dock\":\"north\",\"mark\":\"m\",\"seen\":[\"mark\",\"tag\"],\"keys\":[\"mark\",\"tag\"],\"tag\":\"t\"}
";

/// A type whose function writes, under `seen`, the keys of the object it was handed.
#[model_schema(preprocess = ["withItsSeen"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Keyed {
    pub mark: String,
    pub seen: Vec<String>,
}

/// A type whose function writes, under `keys`, the keys of the object it was handed.
#[model_schema(preprocess = ["withItsKeys"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Marked {
    pub keys: Vec<String>,
    pub tag: String,
}

/// Two types flattened into their holder: each one's function is handed the keys the holder's own
/// members do not take.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Twinned {
    pub dock: String,
    #[serde(flatten)]
    pub keyed: Keyed,
    #[serde(flatten)]
    pub marked: Marked,
}

/// An object that lists, under `seen`, the keys it was handed with, in order.
pub fn with_its_seen(mut payload: Value) -> Value {
    if let Some(object) = payload.as_object_mut() {
        let mut keys: Vec<String> = object.keys().cloned().collect();
        keys.sort_unstable();
        object.insert(
            "seen".to_owned(),
            Value::Array(keys.into_iter().map(Value::String).collect()),
        );
    }
    payload
}

/// The holders of flattened types whose functions list the keys they are handed, as positions.
#[cfg(feature = "serde")]
pub fn flattened_positions() -> Vec<Position> {
    vec![
        position::<Pinned>(
            "a flattened type that reads its keys",
            "Pinned",
            r#"{"dock":"north","tag":"t"}"#,
        ),
        position::<Twinned>(
            "two flattened types that read their keys",
            "Twinned",
            r#"{"dock":"north","mark":"m","tag":"t"}"#,
        ),
    ]
}
