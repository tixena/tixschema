//! The README's "A Model Type Declared in Another Module" entry: the module that names the type,
//! with the import the entry is about.

use crate::version::{Version, version_schema};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// The entry's second module as the README writes it.
const README_DECLARATION: &str = "pub mod record {
    use crate::version::{Version, version_schema};

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Record {
        pub versions: Vec<Version>,
    }
}";

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Record {
    pub versions: Vec<Version>,
}

#[test]
fn a_type_in_another_module_is_described_once_its_schema_module_is_imported() {
    assert!(
        include_str!("../../README.md").contains(README_DECLARATION),
        "the README no longer declares this verbatim:\n{README_DECLARATION}"
    );
    assert_eq!(
        Record::json_schema().pointer("/properties/versions/items"),
        Some(&Version::json_schema())
    );
}
