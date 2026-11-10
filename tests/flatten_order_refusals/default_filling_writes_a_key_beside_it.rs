//! A `default_types` entry of a generic type that flattens its parameter beside a field of the key
//! the filling writes.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Numbered {
    pub id: u32,
}

#[model_schema(default_types(T = Numbered))]
#[derive(Debug, Deserialize, Serialize)]
pub struct Wrap<T> {
    pub id: u32,
    #[serde(flatten)]
    pub inner: T,
}

fn main() {}
