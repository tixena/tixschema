//! A type with no `#[model_schema]` at all, held by a stored type: the build that holds it is one
//! with no schema surface, which describes no such field.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A plain serde type.
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
struct Note {
    text: String,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
struct Parcel {
    label: String,
    note: Note,
}

#[test]
fn a_plain_serde_type_in_a_stored_type_is_read_whole() {
    let mut handed: Vec<String> = Vec::new();
    let read = Parcel::from_bson_with(
        bson::doc! { "label": "P-1", "note": { "text": 7_i32 } },
        |_raw, found| {
            handed = found
                .iter()
                .map(|issue| {
                    if let parcel_schema::Issue::Invalid {
                        path,
                        expected: parcel_schema::Expected::Unknown,
                        found: _found,
                        reason: _reason,
                    } = issue
                    {
                        path.to_string()
                    } else {
                        format!("{issue:?}")
                    }
                })
                .collect();
            parcel_schema::Verdict::Reject
        },
    );
    assert_eq!(handed, ["note"]);
    assert_eq!(read.map_err(|refused| refused.issues.len()), Err(1));
}

#[test]
fn a_plain_serde_type_in_a_stored_type_reads_as_serde_reads_it() {
    assert_eq!(
        Parcel::from_bson_piped(
            bson::doc! { "label": "P-1", "note": { "text": "fragile" } },
            &[]
        ),
        Ok(Parcel {
            label: "P-1".to_owned(),
            note: Note {
                text: "fragile".to_owned()
            }
        })
    );
}
