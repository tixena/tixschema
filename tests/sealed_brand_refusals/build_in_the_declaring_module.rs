//! A brand is not built by its tuple constructor, in the module that declares it.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(minLength = 1, maxLength = 80)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct LabelText(String);

fn main() {
    let _label = LabelText(String::new());
}
