//! A stored type beside a `Filter` of the author's own.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// The author's own type under a name the MongoDB library declares.
#[derive(Debug, Eq, PartialEq)]
pub struct Filter {
    pub number: String,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Invoice {
    pub number: String,
}
