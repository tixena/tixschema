//! A brand does not derive `Default`: the derive would build a value `validate()` never saw.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(minLength = 1, maxLength = 80)]
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(transparent)]
pub struct LabelText(String);

fn main() {}
