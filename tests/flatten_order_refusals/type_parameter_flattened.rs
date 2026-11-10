//! A type that flattens a member whose keys are those of one of its type parameters: the
//! parameter, an `Option` of it, a variant's member, and a generic untagged enum over it.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Stamped<T> {
    pub created_at: String,
    #[serde(flatten)]
    pub record: T,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Maybe<T> {
    pub created_at: String,
    #[serde(flatten)]
    pub record: Option<T>,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub enum Entry<T> {
    Stamped {
        created_at: String,
        #[serde(flatten)]
        record: T,
    },
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Either<T> {
    Count { count: u32 },
    Held(T),
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Shelf<T> {
    pub label: String,
    #[serde(flatten)]
    pub either: Either<T>,
}

fn main() {}
