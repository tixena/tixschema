//! A row whose field names a type its author called `OperationError`. Under `mongodb` the flag
//! adds a type of that name to `{type}_schema`, and what that module already held goes on reading
//! the author's.

use bson::doc;
use bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::seeded_id;

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
struct Attempt {
    failed_with: OperationError,
    #[serde(rename = "_id")]
    id: ObjectId,
}

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
struct OperationError {
    code: u32,
}

#[test]
fn the_authors_type_and_the_added_one_of_its_name_are_two_types() {
    let stored = doc! { "failed_with": { "code": 7_i64 }, "_id": seeded_id() };
    assert_eq!(
        Attempt::mongo_read_row(stored, &[]).unwrap(),
        Attempt {
            failed_with: OperationError { code: 7_u32 },
            id: seeded_id(),
        }
    );
    let refused: attempt_schema::OperationError =
        Attempt::mongo_read_row(doc! { "_id": seeded_id() }, &[]).unwrap_err();
    assert!(
        matches!(
            &refused,
            attempt_schema::OperationError::Unreadable { row: _row, issues } if issues.len() == 1
        ),
        "got: {refused:?}"
    );
}
