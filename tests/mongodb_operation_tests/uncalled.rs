//! A flagged type nothing calls an operation on, in a module that denies every unused item: no
//! lint names an operation the flag adds as never used.

#![deny(unused)]

use bson::doc;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
struct Receipt {
    number: String,
}

#[test]
fn a_type_no_operation_is_called_on_builds_with_unused_items_denied() {
    let read = Receipt::from_bson_piped(doc! { "number": "R-1" }, &[]);
    assert_eq!(
        read,
        Ok(Receipt {
            number: "R-1".to_owned()
        })
    );
}
