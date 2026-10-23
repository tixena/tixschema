//! serde reads `null` as the first variant of an untagged enum whose payload reads it, a newtype
//! variant over an `Option` holding `None` among them, and the JSON Schema admits each payload
//! serde reads.

#[cfg(feature = "jsonschema")]
use super::documents::admits;
#[cfg(feature = "jsonschema")]
use super::null_models::{Drogued, Fathom, Kedged, Plumb};
use super::null_models::{NULL_ROWS, serde_told};

/// serde takes the variant each row states, writes back the payload it read as it was, and
/// refuses each other payload.
#[test]
fn serde_reads_null_as_the_first_variant_whose_payload_reads_it() {
    for row in &NULL_ROWS {
        assert_eq!(
            serde_told(row.read_as, row.json),
            row.told(str::to_owned),
            "serde answers {} read as {} otherwise",
            row.json,
            row.read_as
        );
    }
}

/// The JSON Schema of each enum admits each payload serde reads, `null` among them.
#[cfg(feature = "jsonschema")]
#[test]
fn the_json_schema_admits_each_payload_serde_reads() {
    for row in NULL_ROWS.iter().filter(|row| row.taken.is_some()) {
        let document = match row.read_as {
            "Fathom" => Fathom::json_schema(),
            "Kedged" => Kedged::json_schema(),
            "Drogued" => Drogued::json_schema(),
            _ => Plumb::json_schema(),
        };
        let payload: serde_json::Value = serde_json::from_str(row.json).unwrap();
        assert!(
            admits(&document, &payload),
            "the JSON Schema of {} refuses {}, which serde reads: {document}",
            row.read_as,
            row.json
        );
    }
}
