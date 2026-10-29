//! A length on a brand over a brand declared below it: the brand below counts a list, which the
//! brand above cannot know when it expands, so the length is refused.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(no_display, minLength = 1)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct SomeBehind(pub Behind);

#[model_schema(no_display)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Behind(pub Vec<String>);

fn main() {}
