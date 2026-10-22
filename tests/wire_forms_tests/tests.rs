//! The JSON Schema of each model of [`WIRE_FORMS`], held to every payload a client sends.

#[cfg(feature = "jsonschema")]
use super::documents::admits;
#[cfg(feature = "mongodb")]
use super::models::ConveyedObjectId;
use super::models::{
    ConveyedDay, ConveyedF32, ConveyedF64, ConveyedFlag, ConveyedHour, ConveyedI8, ConveyedI16,
    ConveyedI32, ConveyedI64, ConveyedInstant, ConveyedIsize, ConveyedLetter, ConveyedLiteralCount,
    ConveyedLiteralFlag, ConveyedLiteralText, ConveyedMillis, ConveyedSlotted, ConveyedStamp,
    ConveyedText, ConveyedU8, ConveyedU16, ConveyedU32, ConveyedU64, ConveyedUsize, WIRE_FORMS,
};

/// The models a target reads no payload of in the form serde writes, each with that target: the
/// JSON Schema of a date under `as_number` describes the date's text, and serde writes and reads
/// the epoch-milliseconds number alone. Reported, and left as it is.
const NO_COMMON_FORM: &[(&str, &str)] = &[("ConveyedMillis", "the JSON Schema")];

/// The JSON Schema of each model admits every payload a client sends, and refuses each of a model
/// [`NO_COMMON_FORM`] names it for.
#[cfg(feature = "jsonschema")]
#[test]
fn the_json_schema_admits_every_payload_a_client_sends() {
    for row in WIRE_FORMS {
        let found = json_schema_of(row.held_by);
        assert!(found.is_some(), "no model named {}", row.held_by);
        let document = found.unwrap_or_default();
        let reported = NO_COMMON_FORM.contains(&(row.held_by, "the JSON Schema"));
        for sent in row.sent() {
            let value: serde_json::Value = serde_json::from_str(&sent).unwrap();
            assert_eq!(
                admits(&document, &value),
                !reported,
                "the JSON Schema of {} answers {sent} otherwise than the table states: {document}",
                row.held_by
            );
        }
    }
}

/// The JSON Schema of the model `held_by` names, and `None` where none has that name.
#[cfg(feature = "jsonschema")]
fn json_schema_of(held_by: &str) -> Option<serde_json::Value> {
    match held_by {
        "ConveyedFlag" => Some(ConveyedFlag::json_schema()),
        "ConveyedLetter" => Some(ConveyedLetter::json_schema()),
        "ConveyedText" => Some(ConveyedText::json_schema()),
        "ConveyedU8" => Some(ConveyedU8::json_schema()),
        "ConveyedU16" => Some(ConveyedU16::json_schema()),
        "ConveyedU32" => Some(ConveyedU32::json_schema()),
        "ConveyedI8" => Some(ConveyedI8::json_schema()),
        "ConveyedI16" => Some(ConveyedI16::json_schema()),
        "ConveyedI32" => Some(ConveyedI32::json_schema()),
        "ConveyedI64" => Some(ConveyedI64::json_schema()),
        "ConveyedIsize" => Some(ConveyedIsize::json_schema()),
        "ConveyedU64" => Some(ConveyedU64::<u64>::json_schema()),
        "ConveyedUsize" => Some(ConveyedUsize::<usize>::json_schema()),
        "ConveyedF32" => Some(ConveyedF32::json_schema()),
        "ConveyedF64" => Some(ConveyedF64::json_schema()),
        "ConveyedLiteralCount" => Some(ConveyedLiteralCount::json_schema()),
        "ConveyedLiteralFlag" => Some(ConveyedLiteralFlag::json_schema()),
        "ConveyedLiteralText" => Some(ConveyedLiteralText::json_schema()),
        "ConveyedDay" => Some(ConveyedDay::json_schema()),
        "ConveyedHour" => Some(ConveyedHour::json_schema()),
        "ConveyedStamp" => Some(ConveyedStamp::json_schema()),
        "ConveyedInstant" => Some(ConveyedInstant::json_schema()),
        "ConveyedMillis" => Some(ConveyedMillis::json_schema()),
        "ConveyedSlotted" => Some(ConveyedSlotted::json_schema()),
        #[cfg(feature = "mongodb")]
        "ConveyedObjectId" => Some(ConveyedObjectId::json_schema()),
        _ => None,
    }
}
