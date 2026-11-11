//! The models of `swift_tests/fitted_parameters.rs` and of
//! `service_schema_emitted_client_tests/run_swift_fitted_parameters.rs`: a type parameter held by
//! a member, in a list, as a map's value, under an `Option`, in a variant's one slot and in one of
//! two, and in a tuple struct of one slot, filled with a fixed-width integer, the widest integer
//! Swift reads, and a model. The members of each are declared in the order of their names, the
//! order Swift's encoder writes them in when it sorts them.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A member of the type its parameter is filled with.
#[model_schema(default_types(Held = u32))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Tether<Held> {
    pub held: Held,
}

/// [`Tether`] over a `u32`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Mooring {
    pub inner: Tether<u32>,
}

/// [`Tether`] over an `i64`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WideMooring {
    pub inner: Tether<i64>,
}

/// A model a type parameter is filled with.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Knot {
    pub label: String,
}

/// [`Tether`] over a model.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KnotMooring {
    pub inner: Tether<Knot>,
}

/// A type parameter as a map's value, in a list and under an `Option`.
#[model_schema(default_types(Held = u8))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Hawser<Held> {
    pub keyed: HashMap<String, Held>,
    pub listed: Vec<Held>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maybe: Option<Held>,
}

/// [`Hawser`] over a `u8`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Bollard {
    pub lines: Hawser<u8>,
}

/// A type parameter in a variant's one slot, and beside a text in a variant of two.
#[model_schema(default_types(Held = u16))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Cleat<Held> {
    Bare(Held),
    Paired(Held, String),
}

/// [`Cleat`] over a `u16`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Wharf {
    pub cleat: Cleat<u16>,
}

/// A tuple struct of one slot over a type parameter.
#[model_schema(default_types(Held = i8))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Fender<Held>(pub Held);

/// [`Fender`] over an `i8`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Quay {
    pub fender: Fender<i8>,
}
