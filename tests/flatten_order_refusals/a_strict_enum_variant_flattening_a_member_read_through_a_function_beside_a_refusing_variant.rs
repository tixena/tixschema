//! A strict enum under a tag alone whose struct variant flattens an `Option` read through a
//! `preprocess`, beside a newtype variant over a strict struct, which refuses a key no member
//! declares: each variant is judged by itself, and serde reads what the function answers.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Tholepin {
    pub label: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Lid {
    pub size: u32,
}

fn as_null(_payload: Value) -> Value {
    Value::Null
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Corner {
    Noted {
        note: String,
        #[model_schema_prop(preprocess = ["asNull"])]
        #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
        tholepin: Option<Tholepin>,
    },
    Boxed(Lid),
}

fn main() {}
