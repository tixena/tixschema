//! A member that names a `preprocess` and is read through another function of the macro's own is
//! read through that function, from the payload its own functions corrected: a member that holds
//! a model, a member of an untagged variant held to a bound, a date stored as a number, and a
//! member whose type declares the bound.

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
use alloc::sync::Arc;

#[cfg(all(feature = "serde", feature = "chrono"))]
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

use super::models::{trimmed, unwrapped};

/// A model held to a bound, whose label its own function trims.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Bay {
    #[model_schema_prop(minLength = 1, preprocess = ["trimmed"])]
    pub label: String,
}

/// A brand held to a bound of its own.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[model_schema(minLength = 2)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Bounded(pub String);

/// A brand held to a bound of its own, whose function trims what the bound measures.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[model_schema(minLength = 2, preprocess = ["trimmed"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Coded(pub String);

/// A member of a variant of each tagging, and a slot of one.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum Carried {
    Boxed {
        #[model_schema_prop(preprocess = ["trimmed"])]
        label: String,
    },
    Loose(#[model_schema_prop(preprocess = ["trimmed"])] String, u32),
}

/// A member that holds a model, which its holder reads with nothing judged.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Docked {
    #[model_schema_prop(preprocess = ["unwrapped"])]
    pub bay: Bay,
}

/// A type whose own function gives the payload a label, which the label's own then trims.
#[model_schema(preprocess = ["labelled"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Labelled {
    #[model_schema_prop(preprocess = ["trimmed"])]
    pub label: String,
}

/// A member whose function is named as a read names what it declares.
#[model_schema(preprocess = ["payload"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Named {
    #[model_schema_prop(preprocess = ["corrected", "deserializer"])]
    pub payload: String,
}

/// A list of a parameter's type, and two keys the payload may leave out: one read as its
/// default, one as `None`.
#[model_schema(default_types(CodeType = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Noted<CodeType> {
    #[model_schema_prop(preprocess = ["unwrapped"])]
    pub codes: Vec<CodeType>,
    #[model_schema_prop(preprocess = ["trimmed"])]
    #[serde(default)]
    pub dock: String,
    #[model_schema_prop(preprocess = ["trimmed"])]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// A member of an untagged variant held to a bound, a slot of one, and a list held to a count.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Picked {
    Coded {
        #[model_schema_prop(minLength = 2, preprocess = ["trimmed"])]
        code: String,
    },
    Listed {
        #[model_schema_prop(minLength = 1, preprocess = ["unwrapped"])]
        codes: Vec<String>,
    },
    Short(#[model_schema_prop(maxLength = 2, preprocess = ["trimmed"])] String),
    Whole(String),
}

/// A member whose own type declares the bound, held where its holder reads it as it judges
/// itself.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Shelved {
    #[model_schema_prop(preprocess = ["unwrapped"])]
    pub label: Arc<Bounded>,
}

/// A date stored as the milliseconds since the epoch.
#[cfg(all(feature = "serde", feature = "chrono"))]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Stamped {
    #[model_schema_prop(as_number, preprocess = ["asMillis"])]
    pub at: DateTime<Utc>,
}

/// A slot of a tuple struct.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Tagged(
    #[model_schema_prop(preprocess = ["trimmed"])] pub String,
    pub u32,
);

/// A text of digits as the number it writes.
#[cfg(all(feature = "serde", feature = "chrono"))]
fn as_millis(payload: Value) -> Value {
    if let Value::String(text) = &payload
        && let Ok(millis) = text.parse::<i64>()
    {
        return json!(millis);
    }
    payload
}

const fn corrected(payload: Value) -> Value {
    payload
}

fn deserializer(payload: Value) -> Value {
    trimmed(payload)
}

/// An object with a label where it holds none.
fn labelled(mut payload: Value) -> Value {
    if let Some(object) = payload.as_object_mut() {
        object
            .entry("label")
            .or_insert_with(|| json!("  unnamed  "));
    }
    payload
}

fn payload(held: Value) -> Value {
    unwrapped(held)
}

#[test]
fn the_function_of_a_type_is_called_ahead_of_the_function_of_a_field() {
    assert_eq!(
        serde_json::from_value::<Labelled>(json!({})).unwrap(),
        Labelled {
            label: "unnamed".to_owned()
        }
    );
    assert_eq!(
        serde_json::from_value::<Labelled>(json!({ "label": " A " })).unwrap(),
        Labelled {
            label: "A".to_owned()
        }
    );
}

#[test]
fn a_function_named_as_a_read_names_what_it_declares_is_called() {
    assert_eq!(
        serde_json::from_value::<Named>(json!({ "payload": { "payload": " A " } })).unwrap(),
        Named {
            payload: "A".to_owned()
        }
    );
}

#[test]
fn a_member_that_holds_a_model_is_read_from_the_corrected_payload() {
    let read: Docked =
        serde_json::from_value(json!({ "bay": { "payload": { "label": "  A  " } } })).unwrap();
    assert_eq!(read.bay.label, "A");
}

/// The holder judges the parcel it read with nothing judged, and names it by its place.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn a_model_read_from_a_corrected_payload_is_judged_once_by_its_holder() {
    let refused =
        serde_json::from_value::<Docked>(json!({ "bay": { "payload": { "label": "   " } } }))
            .unwrap_err()
            .to_string();
    assert_eq!(
        refused,
        "'bay.label': too short: minimum length is 1, got 0"
    );
}

#[test]
fn a_member_of_a_variant_and_a_slot_are_read_from_the_corrected_payload() {
    assert_eq!(
        serde_json::from_value::<Carried>(json!({ "kind": "Boxed", "detail": { "label": " A " } }))
            .unwrap(),
        Carried::Boxed {
            label: "A".to_owned()
        }
    );
    assert_eq!(
        serde_json::from_value::<Carried>(json!({ "kind": "Loose", "detail": [" A ", 2_u32] }))
            .unwrap(),
        Carried::Loose("A".to_owned(), 2)
    );
    assert_eq!(
        serde_json::from_value::<Tagged>(json!([" A ", 2_u32])).unwrap(),
        Tagged("A".to_owned(), 2)
    );
}

#[test]
fn a_key_the_payload_leaves_out_is_read_as_it_is_with_no_function() {
    assert_eq!(
        serde_json::from_value::<Noted<String>>(json!({ "codes": { "payload": ["a"] } })).unwrap(),
        Noted {
            codes: vec!["a".to_owned()],
            dock: String::new(),
            note: None
        }
    );
    let held = json!({ "codes": [7_u32], "dock": " north ", "note": " n " });
    assert_eq!(
        serde_json::from_value::<Noted<u32>>(held).unwrap(),
        Noted {
            codes: vec![7_u32],
            dock: "north".to_owned(),
            note: Some("n".to_owned())
        }
    );
}

/// The bound of each variant's member chooses the variant, on what the member's function
/// answered: a code two letters long once trimmed, a list of one code once out of its envelope.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn the_bound_of_an_untagged_member_is_held_to_the_corrected_payload() {
    let picked = |payload: Value| serde_json::from_value::<Picked>(payload).unwrap();
    assert_eq!(
        picked(json!({ "code": " AB " })),
        Picked::Coded {
            code: "AB".to_owned()
        }
    );
    assert_eq!(
        picked(json!({ "codes": { "payload": ["a"] } })),
        Picked::Listed {
            codes: vec!["a".to_owned()]
        }
    );
    assert_eq!(picked(json!(" AB ")), Picked::Short("AB".to_owned()));
    // Three letters once trimmed: the slot's bound refuses them, and the next variant reads the
    // payload as it is.
    assert_eq!(picked(json!(" ABC ")), Picked::Whole(" ABC ".to_owned()));
    for refused in [
        json!({ "code": " A " }),
        json!({ "codes": { "payload": [] } }),
    ] {
        assert_eq!(
            serde_json::from_value::<Picked>(refused)
                .unwrap_err()
                .to_string(),
            "data did not match any variant of untagged enum Picked"
        );
    }
}

/// The brand's bound refuses the text its holder's function took out of the envelope, under the
/// field that holds it.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn a_member_whose_type_declares_the_bound_is_read_from_the_corrected_payload() {
    assert_eq!(
        serde_json::from_value::<Shelved>(json!({ "label": { "payload": "AB" } })).unwrap(),
        Shelved {
            label: Arc::new(Bounded("AB".to_owned()))
        }
    );
    assert_eq!(
        serde_json::from_value::<Shelved>(json!({ "label": { "payload": "A" } }))
            .unwrap_err()
            .to_string(),
        "'label': too short: minimum length is 2, got 1"
    );
}

/// The bound of a brand measures what the brand's own function answered, and refuses it once,
/// in the words `validate()` says of the same value built in code.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn the_bound_of_a_brand_measures_what_its_function_answered() {
    assert_eq!(
        serde_json::from_value::<Coded>(json!(" AB ")).unwrap(),
        Coded("AB".to_owned())
    );
    assert_eq!(
        serde_json::from_value::<Coded>(json!(" A "))
            .unwrap_err()
            .to_string(),
        Coded("A".to_owned()).validate().unwrap_err().join("; ")
    );
}

#[cfg(all(feature = "serde", feature = "chrono"))]
#[test]
fn a_date_stored_as_a_number_is_read_from_the_corrected_payload() {
    let at = DateTime::from_timestamp_millis(1_700_000_000_000).unwrap();
    for stored in [json!(1_700_000_000_000_i64), json!("1700000000000")] {
        assert_eq!(
            serde_json::from_value::<Stamped>(json!({ "at": stored })).unwrap(),
            Stamped { at }
        );
    }
    assert_eq!(
        serde_json::to_value(Stamped { at }).unwrap(),
        json!({ "at": 1_700_000_000_000_i64 })
    );
}
