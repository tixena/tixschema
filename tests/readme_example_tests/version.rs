//! The README's "A Model Type Declared in Another Module" entry: the module that declares the type.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Version {
    pub number: i32,
}
