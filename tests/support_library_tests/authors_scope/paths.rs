//! A type beside the author's import of `std::path::Path`.

use std::path::Path;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(enable = ["decode_with"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InvoiceLookup {
    pub number: String,
}

/// The author's `Path`, read beside the type.
pub fn authors(written: &str) -> &Path {
    Path::new(written)
}
