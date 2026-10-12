//! A key written on an `IndexSet` measures the list, as on a `HashSet`: `minLength` counts its
//! items.

use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Labelled {
    #[model_schema_prop(minLength = 1)]
    pub labels: IndexSet<String>,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct LabelledTwin {
    #[model_schema_prop(minLength = 1)]
    pub labels: HashSet<String>,
}

#[test]
fn its_items_are_counted() {
    let empty = Labelled {
        labels: IndexSet::new(),
    };
    let twin = LabelledTwin {
        labels: HashSet::new(),
    };
    assert_eq!(
        empty.validate().unwrap_err(),
        vec!["'labels': too short: minimum length is 1, got 0"]
    );
    assert_eq!(empty.validate(), twin.validate());
    let held = Labelled {
        labels: IndexSet::from(["only".to_owned()]),
    };
    assert_eq!(held.validate(), Ok(()));
}

#[cfg(feature = "zod")]
#[test]
fn zod_writes_the_count_on_the_array() {
    let labelled = Labelled::zod_schema();
    assert!(
        labelled.contains("labels: z.array(z.string()).min(1"),
        "got: {labelled}"
    );
    assert_eq!(labelled, LabelledTwin::zod_schema().replace("Twin", ""));
}

#[cfg(feature = "jsonschema")]
#[test]
fn the_json_schema_writes_the_count_as_min_items() {
    let labels = &Labelled::json_schema()["properties"]["labels"];
    assert_eq!(labels["minItems"], serde_json::json!(1_i32));
    assert_eq!(Labelled::json_schema(), LabelledTwin::json_schema());
}
