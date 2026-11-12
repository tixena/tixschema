//! A brand's slot takes no `pub`.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(minLength = 1, maxLength = 80)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct LabelText(pub String);

fn main() {}
