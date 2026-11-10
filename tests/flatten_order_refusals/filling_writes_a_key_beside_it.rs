//! A filling of a generic type that flattens its parameter beside a field of the key the filling
//! writes.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Numbered {
    pub id: u32,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Wrap<T> {
    pub id: u32,
    #[serde(flatten)]
    pub inner: T,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    pub held: Wrap<Numbered>,
}

fn main() {}
