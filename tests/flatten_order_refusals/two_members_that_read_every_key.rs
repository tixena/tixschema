//! Two tag-alone enums flattened together: each reads every key, and the strict one, with the
//! struct its newtype variant holds, refuses the other's in every order.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Lid {
    pub hinge: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Shape {
    Circle { radius: u32 },
    Boxed(Lid),
}

#[model_schema(nonstrict)]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "tone")]
pub enum Hue {
    Warm { warmth: u32 },
    Cold { cold: u32 },
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    #[serde(flatten)]
    pub hue: Hue,
    #[serde(flatten)]
    pub shape: Shape,
}

fn main() {}
