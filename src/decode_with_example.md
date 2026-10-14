```rust
# extern crate bson2 as bson;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tixschema::model_schema;

tixschema::decode_with_foundation!();

#[model_schema(enable = ["decode_with"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct InvoiceLookup {
    pub number: String,
}

fn main() {
    let read = InvoiceLookup::from_value_with(json!({ "number": 42 }), |_raw, found| {
        assert_eq!(found.len(), 1);
        invoice_lookup_schema::Verdict::Reject
    });
    assert!(read.is_err());
}
```
