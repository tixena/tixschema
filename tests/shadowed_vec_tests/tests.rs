//! Tests of a model type declared beside a type named `Vec`.

use alloc::collections::VecDeque;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Holder {
    pub inner: Inner,
    #[model_schema_prop(minimum = 1)]
    pub level: i32,
    #[model_schema_prop(minLength = 1)]
    pub queued: VecDeque<i32>,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Inner {
    pub number: i32,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct Vec {
    pub count: i32,
}

#[test]
fn a_struct_declared_beside_it_builds_reads_and_validates() {
    let read: Holder = serde_json::from_value(
        serde_json::json!({ "inner": { "number": 1_i32 }, "level": 1_i32, "queued": [1_i32] }),
    )
    .unwrap();
    assert_eq!(read.validate(), Ok(()));
    let broken = Holder {
        inner: Inner { number: 1 },
        level: 0,
        queued: VecDeque::new(),
    };
    let reported = broken.validate().unwrap_err();
    assert_eq!(reported.len(), 2);
    assert_eq!(
        serde_json::from_value::<Holder>(
            serde_json::json!({ "inner": { "number": 1_i32 }, "level": 0_i32, "queued": [] }),
        )
        .unwrap_err()
        .to_string(),
        reported.join("; ")
    );
    assert!(Holder::ts_definition().contains("  inner: Inner;"));
    assert!(Vec::ts_definition().contains("  count: number;"));
}

#[cfg(feature = "jsonschema")]
#[test]
fn the_type_itself_is_described() {
    assert_eq!(
        Vec::json_schema(),
        serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "properties": { "count": { "type": "integer" } },
            "required": ["count"],
        })
    );
}
