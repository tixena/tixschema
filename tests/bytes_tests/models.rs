//! The models of the `Vec<u8>` held as bytes: one per place a member holds one, and one per
//! sequence that stays an array.

use alloc::boxed::Box;
use alloc::collections::VecDeque;
use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A `Vec<u8>` as a member, beside the member it travels with.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BytesImage {
    pub content_type: String,
    pub data: Vec<u8>,
}

/// A `Vec<u8>` held by an alias, which the models below name.
#[model_schema()]
pub type BytesPayload = Vec<u8>;

/// A type that holds what fills its parameter.
#[model_schema(default_types(Held = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BytesBox<Held> {
    pub held: Held,
}

/// A `Vec<u8>` in each place a member of a struct holds one.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BytesHolder {
    pub aliased: BytesPayload,
    pub data: Vec<u8>,
    pub filled: BytesBox<Vec<u8>>,
    pub keyed: HashMap<String, Vec<u8>>,
    pub outcome: Result<Vec<u8>, String>,
    pub paired: (Vec<u8>, String),
    pub rows: Vec<Vec<u8>>,
}

/// An optional `Vec<u8>`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BytesOptional {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maybe: Option<Vec<u8>>,
}

/// A `Vec<u8>` under a count of its own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BytesCounted {
    #[model_schema_prop(minLength = 2, maxLength = 4)]
    pub data: Vec<u8>,
}

/// A `Vec<u8>` as the first member an untagged enum tries.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum BytesEither {
    Bytes(Vec<u8>),
    Text(String),
}

/// A `Vec<u8>` in the example the schema publishes.
///
/// ```rust example
/// BytesExample {
///     data: vec![137, 80, 78, 71],
/// }
/// ```
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BytesExample {
    pub data: Vec<u8>,
}

/// Each sequence that is not a `Vec<u8>`, which stays an array.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BytesLookalikes {
    pub boxed: Box<[u8]>,
    pub digest: [u8; 4],
    pub distinct: HashSet<u8>,
    pub gaps: Vec<Option<u8>>,
    pub queued: VecDeque<u8>,
    #[model_schema_prop(literal = 7)]
    pub sevens: Vec<u8>,
    pub signed: Vec<i8>,
    pub wide: Vec<u16>,
}
