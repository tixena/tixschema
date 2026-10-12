//! The aliases the fields beside this module are typed with. Nothing they are written with is in
//! scope where those fields are.

use indexmap::{IndexMap, IndexSet};
use tixschema::model_schema;

use super::Tracked;
#[cfg(feature = "jsonschema")]
use super::tracked_schema;

#[model_schema()]
pub type RankedEntries = IndexMap<String, Vec<Tracked>>;

#[model_schema()]
pub type RankedLabels = IndexSet<String>;

#[model_schema()]
pub type RankedScores = IndexMap<String, i32>;
