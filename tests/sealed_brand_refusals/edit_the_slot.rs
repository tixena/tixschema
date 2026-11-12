//! A brand's slot is neither assigned nor borrowed mutably.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(minLength = 1, maxLength = 80)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct LabelText(String);

fn main() {
    let mut label = LabelText::new("ok").unwrap();
    label.0 = String::new();
    let _borrowed = &mut label.0;
}
