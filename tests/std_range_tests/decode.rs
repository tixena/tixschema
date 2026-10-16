//! A range field in the recovering decode, read as one whole value at its key, and in the typed
//! MongoDB paths, a `Field` of that value.

use core::ops::Range;

use serde::{Deserialize, Serialize};
use serde_json::json;
use tixschema::model_schema;

/// A range beside a field of another type.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Voyage {
    pub label: String,
    pub span: Range<u32>,
}

#[test]
fn a_range_that_does_not_read_is_one_issue_at_its_key() {
    let stored = json!({ "label": "a", "span": { "start": "8", "end": 13_u32 } });
    let mut seen = Vec::new();
    let read = Voyage::from_value_with(stored, |_raw, found| {
        seen = found.to_vec();
        voyage_schema::Verdict::Reject
    });
    read.unwrap_err();
    assert_eq!(seen.len(), 1, "{seen:?}");
    let invalid = if let Some(voyage_schema::Issue::Invalid {
        path,
        expected,
        found,
        reason: _reason,
    }) = seen.first()
    {
        Some((path, expected, found))
    } else {
        None
    };
    let (path, expected, found) = invalid.unwrap();
    assert_eq!(
        *path,
        voyage_schema::Path(vec![voyage_schema::Segment::Key("span".to_owned())])
    );
    assert_eq!(
        *expected,
        voyage_schema::Expected::Range("Range", Box::new(voyage_schema::Expected::U32))
    );
    assert_eq!(*found, json!({ "start": "8", "end": 13_u32 }));
    let sound = json!({ "label": "a", "span": { "start": 8_u32, "end": 13_u32 } });
    assert_eq!(
        Voyage::from_value_with(sound, |_raw, _found| voyage_schema::Verdict::Reject),
        Ok(Voyage {
            label: "a".to_owned(),
            span: 8..13,
        })
    );
}

#[cfg(feature = "mongodb")]
#[test]
fn a_range_is_a_field_of_one_whole_value() {
    let path: voyage_schema::Field<Voyage, Range<u32>> = Voyage::MONGO_FIELDS.span();
    let filter: bson::Document = path.eq(8..13).unwrap().into();
    let written = bson::to_document(&json!({ "start": 8_u32, "end": 13_u32 })).unwrap();
    assert_eq!(
        filter.get_document("span").unwrap().get_document("$eq"),
        Ok(&written),
        "{filter}"
    );
}
