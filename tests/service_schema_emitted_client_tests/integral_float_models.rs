//! The models `run_swift_integral_floats.rs` reads a number written with a fraction or an exponent
//! on: a fixed-width integer as a member, a list's item, a map's value and under an `Option`, and a
//! type parameter filled with a float, a text and an optional integer. The members of each are
//! declared in the order of their names, the order Swift's encoder writes them in when it sorts
//! them.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// An integer in each place a member holds one.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Fathometer {
    pub depth: u32,
    pub fathoms: Vec<i64>,
    pub marks: HashMap<String, i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tide: Option<i64>,
}

/// A member of the type its parameter is filled with.
#[model_schema(default_types(Held = f64))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Spar<Held> {
    pub held: Held,
}

/// [`Spar`] filled with a float, an optional integer and a text.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Lighthouse {
    pub light: Spar<f64>,
    pub mark: Spar<Option<u32>>,
    pub name: Spar<String>,
}
