//! A brand is not built by its tuple constructor, from a module that does not declare it.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

mod declared {
    use super::*;

    #[model_schema(minLength = 1, maxLength = 80)]
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(transparent)]
    pub struct LabelText(String);
}

fn main() {
    let _label = declared::LabelText(String::new());
}
