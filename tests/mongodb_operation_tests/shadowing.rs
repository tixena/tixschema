//! A row whose fields name types its author called `OperationError` and `Read`. Under `mongodb`
//! the flag adds a type of each name to `{type}_schema`, and what that module already held goes
//! on reading the author's.

use bson::oid::ObjectId;
use bson::{Document, doc};
use mongodb::Collection;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::seeded_id;

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
struct Attempt {
    failed_with: OperationError,
    #[serde(rename = "_id")]
    id: ObjectId,
    last: Read,
}

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
struct OperationError {
    code: u32,
}

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
struct Read {
    rows: u32,
}

/// Compiles only where what `find` answers is the read the flag adds, over the author's types.
/// It is never called, so no collection is asked for.
fn every_attempt(held: &Collection<Document>) -> attempt_schema::Read<'_, Attempt, Vec<Attempt>> {
    Attempt::find(held, attempt_schema::Filter::raw(doc! {}))
}

#[test]
fn the_authors_types_and_the_added_ones_of_their_names_are_two_types_each() {
    let stored = doc! {
        "failed_with": { "code": 7_i64 },
        "_id": seeded_id(),
        "last": { "rows": 2_i64 },
    };
    assert_eq!(
        Attempt::mongo_read_row(stored, &[]).unwrap(),
        Attempt {
            failed_with: OperationError { code: 7_u32 },
            id: seeded_id(),
            last: Read { rows: 2_u32 },
        }
    );
    let refused: attempt_schema::OperationError =
        Attempt::mongo_read_row(doc! { "_id": seeded_id() }, &[]).unwrap_err();
    assert!(
        matches!(
            &refused,
            attempt_schema::OperationError::Unreadable { row: _row, issues } if issues.len() == 2
        ),
        "got: {refused:?}"
    );
    let _: fn(&Collection<Document>) -> attempt_schema::Read<'_, Attempt, Vec<Attempt>> =
        every_attempt;
}
