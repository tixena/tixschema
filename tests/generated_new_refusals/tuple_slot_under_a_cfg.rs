//! `new = true` on a tuple struct with a slot under `#[cfg]` stops the build: no `new` is written
//! for it.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(new = true)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Reading(u8, #[cfg(unix)] u8, u8);

fn main() {}
