//! Tests of a model type declared beside a type named `String`.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Holder {
    pub inner: Inner,
    #[model_schema_prop(minimum = 1)]
    pub level: i32,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Inner {
    pub number: i32,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct String {
    pub count: i32,
}

#[test]
fn a_struct_declared_beside_it_builds_reads_and_validates() {
    let read: Holder =
        serde_json::from_value(serde_json::json!({ "inner": { "number": 1_i32 }, "level": 0_i32 }))
            .unwrap();
    assert_eq!(read.validate().unwrap_err().len(), 1);
    assert!(Holder::ts_definition().contains("  inner: Inner;"));
    assert!(String::ts_definition().contains("  count: number;"));
}

#[cfg(feature = "jsonschema")]
#[test]
fn the_type_itself_is_described() {
    assert_eq!(
        String::json_schema(),
        serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "properties": { "count": { "type": "integer" } },
            "required": ["count"],
        })
    );
}
