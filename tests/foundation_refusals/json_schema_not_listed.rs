//! A type that does not list `"jsonschema"` has no `json_schema()`.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Signup {
    #[model_schema_prop(minLength = 3)]
    pub username: String,
}

fn main() {
    let _ = Signup::json_schema();
}
