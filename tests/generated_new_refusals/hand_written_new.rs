//! A struct that declares its own `new` without `new = false` collides with the generated one.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TimeOfDay {
    pub hour: u8,
}

impl TimeOfDay {
    pub fn new(hour: u8) -> Result<Self, Vec<String>> {
        TimeOfDayDraft { hour }.validate()
    }
}

fn main() {}
