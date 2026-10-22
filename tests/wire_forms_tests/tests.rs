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

/// The JSON Schema of each model admits every payload a client sends.
#[cfg(feature = "jsonschema")]
#[test]
fn the_json_schema_admits_every_payload_a_client_sends() {
    for row in WIRE_FORMS {
        let found = json_schema_of(row.held_by);
        assert!(found.is_some(), "no model named {}", row.held_by);
        let document = found.unwrap_or_default();
        for sent in row.sent() {
            let value: serde_json::Value = serde_json::from_str(&sent).unwrap();
            assert!(
                admits(&document, &value),
                "the JSON Schema of {} refuses {sent}, which a client sends: {document}",
                row.held_by
            );
        }
    }
}

/// The JSON Schema refuses a value held in another form than the one serde writes, as serde
/// refuses it: a date under `as_number` as its text, and a time and a date and time with no zone
/// with a zone.
#[cfg(feature = "jsonschema")]
#[test]
fn the_json_schema_refuses_a_value_in_another_form_than_serde_writes() {
    for (held_by, payload) in [
        ("ConveyedMillis", r#"{"value":"2024-01-02T03:04:05.678Z"}"#),
        (
            "ConveyedHour",
            r#"{"keyed":{},"value":"10:20:30Z","values":[]}"#,
        ),
        (
            "ConveyedHour",
            r#"{"keyed":{"10:20:30Z":true},"value":"10:20:30","values":[]}"#,
        ),
        (
            "ConveyedStamp",
            r#"{"keyed":{},"value":"2024-01-05T10:20:30+00:00","values":[]}"#,
        ),
    ] {
        let read = match held_by {
            "ConveyedMillis" => serde_json::from_str::<ConveyedMillis>(payload).is_ok(),
            "ConveyedHour" => serde_json::from_str::<ConveyedHour>(payload).is_ok(),
            _ => serde_json::from_str::<ConveyedStamp>(payload).is_ok(),
        };
        assert!(!read, "serde reads {payload} as {held_by}");
        let document = json_schema_of(held_by).unwrap_or_default();
        let value: serde_json::Value = serde_json::from_str(payload).unwrap();
        assert!(
            !admits(&document, &value),
            "the JSON Schema of {held_by} admits {payload}, which serde refuses: {document}"
        );
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
