//! A serde read of a type that declares a `preprocess` calls each function it names on the
//! payload before the value is built: the sample of the documents, and one type per shape.

use core::cell::Cell;
use core::fmt::Debug;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

use super::models::{
    Carrier, ClosedDock, Consigned, Crate, Crated, Delivery, Either, Grade, Handover, Label,
    Manifest, Node, Parcel, Pinned, Reference, Routed, SAMPLE_ROWS, Shelf, Shipment, Slot, Stack,
    Tagged, Transfer, Unnamed, Weighed, Wrapped, sample_report,
};
#[cfg(feature = "serde")]
use super::models::{POSITIONS, positions};

thread_local! {
    /// How often the function a counted label names was called on this thread.
    static LABEL_CALLS: Cell<u32> = const { Cell::new(0) };
    /// How often the function a counted parcel names was called on this thread.
    static PARCEL_CALLS: Cell<u32> = const { Cell::new(0) };
}

/// A parcel whose functions count their calls.
#[model_schema(preprocess = ["countedParcel"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
struct CountedParcel {
    #[model_schema_prop(preprocess = ["countedLabel"])]
    label: String,
}

/// How often the two functions of a counted parcel were called on this thread: the type's, then
/// its label's.
fn counted_calls() -> (u32, u32) {
    (PARCEL_CALLS.get(), LABEL_CALLS.get())
}

fn counted_label(payload: Value) -> Value {
    LABEL_CALLS.set(LABEL_CALLS.get() + 1);
    payload
}

fn counted_parcel(payload: Value) -> Value {
    PARCEL_CALLS.set(PARCEL_CALLS.get() + 1);
    payload
}

fn texts(written: &[&str]) -> Vec<String> {
    written.iter().map(|&text| text.to_owned()).collect()
}

fn parcel(label: &str, fragile: &[&str]) -> Parcel {
    Parcel {
        fragile_item_codes: texts(fragile),
        item_code_list: texts(&["x"]),
        label: label.to_owned(),
    }
}

/// Holds the read of `value` to `value`, from the payload serde writes for it and from that
/// payload inside an envelope, as a text and as a value.
fn read_out_of_its_envelope<T>(value: &T)
where
    T: Debug + DeserializeOwned + PartialEq + Serialize,
{
    let plain = serde_json::to_value(value).unwrap();
    let enveloped = json!({ "payload": plain });
    for payload in [plain, enveloped] {
        assert_eq!(
            serde_json::from_str::<T>(&payload.to_string()).unwrap(),
            *value,
            "{payload} as a text"
        );
        assert_eq!(
            serde_json::from_value::<T>(payload.clone()).unwrap(),
            *value,
            "{payload} as a value"
        );
    }
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn the_sample_reads_as_the_documents_say() {
    assert_eq!(sample_report(), super::models::SAMPLE);
}

/// A build that publishes no validator for `label` reads row 2 with the label its function left.
#[cfg(not(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
)))]
#[test]
fn the_sample_reads_with_no_bound_held_where_none_is_published() {
    assert_eq!(
        sample_report(),
        super::models::SAMPLE.replace(
            "row 2: refused\n    'label': too short: minimum length is 1, got 0\n",
            "row 2: read {\"fragileItemCodes\":[\"x\"],\"itemCodeList\":[\"x\"],\"label\":\"\"}\n"
        )
    );
}

#[test]
fn a_text_and_a_value_are_read_alike() {
    let (_, row_1) = SAMPLE_ROWS[0];
    let from_text: Parcel = serde_json::from_str(row_1).unwrap();
    let from_value: Parcel =
        serde_json::from_value(serde_json::from_str::<Value>(row_1).unwrap()).unwrap();
    assert_eq!(from_text, parcel("A", &["x"]));
    assert_eq!(from_value, from_text);
}

#[test]
fn the_function_of_a_type_adds_the_key_its_payload_leaves_out() {
    let (_, row_3) = SAMPLE_ROWS[2];
    assert_eq!(
        serde_json::from_str::<Parcel>(row_3).unwrap(),
        parcel("A", &[])
    );
}

#[test]
fn a_parcel_is_corrected_wherever_another_type_holds_it() {
    let held = json!({ "label": "  A  ", "itemCodeList": ["x"] });
    let read = parcel("A", &[]);
    assert_eq!(
        serde_json::from_value::<Manifest>(json!({ "parcel": held })).unwrap(),
        Manifest {
            parcel: read.clone()
        }
    );
    let shelf: Shelf = serde_json::from_value(json!({
        "by_dock": { "north": held },
        "list": [held],
        "spare": held,
    }))
    .unwrap();
    assert_eq!(shelf.by_dock.get("north"), Some(&read));
    assert_eq!((shelf.list.len(), shelf.list.first()), (1, Some(&read)));
    assert_eq!(shelf.spare.as_ref(), Some(&read));
    assert_eq!(
        serde_json::from_value::<Slot>(json!({ "Held": held })).unwrap(),
        Slot::Held(read.clone())
    );
    assert_eq!(
        serde_json::from_value::<Consigned>(json!({ "kind": "Held", "detail": held })).unwrap(),
        Consigned::Held(read.clone())
    );
    assert_eq!(
        serde_json::from_value::<Either>(held).unwrap(),
        Either::Parcel(read.clone())
    );
    assert_eq!(
        serde_json::from_value::<Crated>(
            json!({ "dock": "north", "label": "  A  ", "itemCodeList": ["x"] })
        )
        .unwrap(),
        Crated {
            dock: "north".to_owned(),
            parcel: read.clone()
        }
    );
    assert_eq!(
        serde_json::from_value::<Delivery>(
            json!({ "kind": "Held", "label": "  A  ", "itemCodeList": ["x"] })
        )
        .unwrap(),
        Delivery::Held(read)
    );
}

#[test]
fn a_type_that_reaches_itself_is_corrected_at_every_depth() {
    let read: Node = serde_json::from_value(json!({
        "id": "a",
        "children": [{ "id": "b" }, { "id": "c", "children": [{ "id": "d" }] }],
    }))
    .unwrap();
    let leaf = |id: &str| Node {
        children: Vec::new(),
        id: id.to_owned(),
    };
    assert_eq!(
        read,
        Node {
            children: vec![
                leaf("b"),
                Node {
                    children: vec![leaf("d")],
                    id: "c".to_owned(),
                },
            ],
            id: "a".to_owned(),
        }
    );
}

#[test]
fn every_shape_takes_the_key_on_the_type() {
    read_out_of_its_envelope(&Stack(texts(&["a", "b"]), 2));
    read_out_of_its_envelope(&ClosedDock);
    read_out_of_its_envelope(&Label("A".to_owned()));
    for grade in [Grade::Express, Grade::Standard] {
        read_out_of_its_envelope(&grade);
    }
    for transfer in [
        Transfer::Held,
        Transfer::Rail("north".to_owned()),
        Transfer::Road {
            stops: texts(&["a"]),
        },
    ] {
        read_out_of_its_envelope(&transfer);
    }
    for shipment in [
        Shipment::Air {
            flight: "F1".to_owned(),
        },
        Shipment::Ground,
    ] {
        read_out_of_its_envelope(&shipment);
    }
    for handover in [
        Handover::Left("porch".to_owned()),
        Handover::Signed { by: "A".to_owned() },
    ] {
        read_out_of_its_envelope(&handover);
    }
    for reference in [
        Reference::Named {
            name: "north".to_owned(),
        },
        Reference::Numbered(7),
    ] {
        read_out_of_its_envelope(&reference);
    }
}

#[test]
fn a_type_with_a_parameter_takes_the_key_at_each_filling() {
    read_out_of_its_envelope(&Crate {
        codes: texts(&["a"]),
        label: "c".to_owned(),
    });
    read_out_of_its_envelope(&Crate {
        codes: vec![7_u32],
        label: "c".to_owned(),
    });
}

/// The brand's second function trims what its first took out of the envelope.
#[test]
fn the_functions_of_a_type_run_in_the_order_written() {
    assert_eq!(
        serde_json::from_value::<Label>(json!({ "payload": "  A  " })).unwrap(),
        Label("A".to_owned())
    );
    assert_eq!(
        serde_json::from_value::<Wrapped>(json!({ "payload": { "label": "A" } })).unwrap(),
        Wrapped {
            fragile_item_codes: Vec::new(),
            label: "A".to_owned(),
        }
    );
}

#[test]
fn one_read_calls_each_function_once() {
    let before = counted_calls();
    let read: CountedParcel = serde_json::from_str(r#"{"label":"A"}"#).unwrap();
    assert_eq!(read.label, "A");
    let from_text = counted_calls();
    assert_eq!(from_text, (before.0 + 1, before.1 + 1));
    let again: CountedParcel = serde_json::from_value(json!({ "label": "A" })).unwrap();
    assert_eq!(again, read);
    assert_eq!(counted_calls(), (from_text.0 + 1, from_text.1 + 1));
}

/// serde reads an absent key as it does with no function named, and the field's is not called.
#[test]
fn the_function_of_a_field_is_not_called_for_a_key_the_payload_does_not_hold() {
    let before = counted_calls();
    let refused = serde_json::from_str::<CountedParcel>("{}").unwrap_err();
    assert_eq!(refused.to_string(), "missing field `label`");
    assert_eq!(counted_calls(), (before.0 + 1, before.1));
}

/// Every position another type reads a preprocessed type in, and every shape the key is written
/// on, prints the one text the Zod schemas print under node.
#[cfg(feature = "serde")]
#[test]
fn every_position_and_every_shape_reads_the_corrected_payload() {
    let printed: String = positions()
        .into_iter()
        .map(|(_name, _schema, _payload, read)| read)
        .collect();
    assert_eq!(printed, POSITIONS);
}

/// The function of a flattened type is handed the keys the holder's own members do not take.
#[test]
fn the_function_of_a_flattened_type_is_handed_what_its_holder_does_not_take() {
    assert_eq!(
        serde_json::from_str::<Pinned>(r#"{"dock":"north","tag":"t"}"#).unwrap(),
        Pinned {
            dock: "north".to_owned(),
            tagged: Tagged {
                keys: texts(&["tag"]),
                tag: "t".to_owned(),
            },
        }
    );
}

/// A choice flattened into its holder is corrected by its own function, which is handed the keys
/// the holder's own members do not take.
#[test]
fn a_flattened_choice_is_corrected_by_its_own_function() {
    assert_eq!(
        serde_json::from_str::<Routed>(r#"{"dock":"north","plate":"  AB 1  "}"#).unwrap(),
        Routed {
            dock: "north".to_owned(),
            via: Carrier::Road {
                plate: "AB 1".to_owned()
            },
        }
    );
}

/// A check judges the value the read built, which the function corrected first.
#[test]
fn a_check_judges_what_the_function_answered() {
    assert_eq!(
        serde_json::from_str::<Weighed>(r#"{"payload":{"grams":900}}"#).unwrap(),
        Weighed { grams: 900 }
    );
    let refused = serde_json::from_str::<Weighed>(r#"{"payload":{"grams":2000}}"#).unwrap_err();
    assert_eq!(
        refused.to_string(),
        Weighed { grams: 2000 }.validate().unwrap_err().join("; ")
    );
}

/// A key that holds `null` is one the payload holds, and the field's function is handed it.
#[test]
fn the_function_of_a_field_is_handed_a_null_the_payload_holds() {
    assert_eq!(
        serde_json::from_str::<Unnamed>(r#"{"label":null}"#).unwrap(),
        Unnamed {
            label: "unnamed".to_owned()
        }
    );
    assert_eq!(
        serde_json::from_str::<Unnamed>("{}")
            .unwrap_err()
            .to_string(),
        "missing field `label` at line 1 column 2"
    );
}

/// A value built in code is never read, so nothing corrects it.
#[test]
fn a_value_built_in_code_is_written_as_it_is() {
    let built = parcel("  A  ", &[]);
    assert_eq!(
        serde_json::to_value(&built).unwrap(),
        json!({ "fragileItemCodes": [], "itemCodeList": ["x"], "label": "  A  " })
    );
}
