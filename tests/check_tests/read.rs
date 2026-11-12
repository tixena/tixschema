//! A serde read of a type that names a check answers an error for the payload of a value the
//! check refuses, in every build: the error holds what `validate()` answers of the value.

use core::fmt::Debug;

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::models::{
    Batch, ClosedDock, ClosedDockDraft, Code, Crate, CrateDraft, Grade, GradeDraft, Handover,
    HandoverDraft, Pallet, PalletDraft, Reference, ReferenceDraft, Route, RouteDraft, Shipment,
    ShipmentDraft, Stack, StackDraft, Transfer, TransferDraft, parcels,
};

fn texts(written: &[&str]) -> Vec<String> {
    written.iter().map(|&text| text.to_owned()).collect()
}

/// Holds the read of what serde writes for `value` to what `validate()` answers of `value`: an
/// error with every message where it reports any, and the value where it reports none.
fn read_as_validated<T>(value: &T, validated: Result<(), Vec<String>>)
where
    T: Debug + DeserializeOwned + Serialize,
{
    let payload = serde_json::to_string(value).unwrap();
    let read = serde_json::from_str::<T>(&payload);
    match validated {
        Ok(()) => assert_eq!(
            read.map(|held| serde_json::to_string(&held).unwrap())
                .map_err(|refused| refused.to_string()),
            Ok(payload)
        ),
        Err(reported) => assert_eq!(
            read.map(|held| format!("{held:?}"))
                .map_err(|refused| refused.to_string()),
            Err(reported.join("; ")),
            "{payload} is read"
        ),
    }
}

#[test]
fn a_struct_is_read_as_validate_answers() {
    for held in parcels() {
        read_as_validated(&held, held.validate().map(|_valid| ()));
    }
    for stops in [texts(&[]), texts(&["a"])] {
        let route = RouteDraft { stops };
        read_as_validated(&route, route.validate().map(|_valid| ()));
    }
    for (codes, limit) in [
        (texts(&["a", "a"]), 5),
        (texts(&["a", "b"]), 1),
        (texts(&["a"]), 1),
    ] {
        let stack = StackDraft(codes, limit);
        read_as_validated(&stack, stack.validate().map(|_valid| ()));
    }
    for codes in [texts(&["a", "b", "c"]), texts(&["a"])] {
        let pallet = PalletDraft { codes };
        read_as_validated(&pallet, pallet.validate().map(|_valid| ()));
    }
    read_as_validated(
        &ClosedDockDraft,
        ClosedDockDraft.validate().map(|_valid| ()),
    );
    assert!(
        parcels()
            .iter()
            .any(|held| !held.validation_issues().is_empty())
    );
}

#[test]
fn an_enum_is_read_as_validate_answers_under_each_form_serde_writes_one_in() {
    let air = |stops: &[&str]| ShipmentDraft::Air {
        flight: "F1".to_owned(),
        stops: texts(stops),
    };
    for shipment in [
        air(&["a", "b", "c"]),
        air(&["a", "a"]),
        air(&["a"]),
        ShipmentDraft::Ground {
            stops: texts(&["a", "b", "c"]),
        },
    ] {
        read_as_validated(&shipment, shipment.validate().map(|_valid| ()));
    }
    for transfer in [
        TransferDraft::Road { stops: Vec::new() },
        TransferDraft::Rail(String::new()),
        TransferDraft::Rail("north".to_owned()),
        TransferDraft::Held,
    ] {
        read_as_validated(&transfer, transfer.validate().map(|_valid| ()));
    }
    for handover in [
        HandoverDraft::Signed { by: String::new() },
        HandoverDraft::Left("porch".to_owned()),
    ] {
        read_as_validated(&handover, handover.validate().map(|_valid| ()));
    }
    for reference in [
        ReferenceDraft::Numbered(0),
        ReferenceDraft::Numbered(7),
        ReferenceDraft::Named {
            name: String::new(),
        },
        ReferenceDraft::Named {
            name: "north".to_owned(),
        },
    ] {
        read_as_validated(&reference, reference.validate().map(|_valid| ()));
    }
    for grade in [
        GradeDraft::Unrated,
        GradeDraft::Express,
        GradeDraft::Standard,
    ] {
        read_as_validated(&grade, grade.validate().map(|_valid| ()));
    }
}

#[test]
fn a_type_with_a_parameter_is_read_as_validate_answers_at_each_filling() {
    for codes in [texts(&[]), texts(&["a"])] {
        let filled = CrateDraft {
            codes,
            label: "c".to_owned(),
        };
        read_as_validated(&filled, filled.validate().map(|_valid| ()));
    }
    for codes in [Vec::<u32>::new(), vec![7]] {
        let counted = CrateDraft {
            codes,
            label: "c".to_owned(),
        };
        read_as_validated(&counted, counted.validate().map(|_valid| ()));
    }
}

#[test]
fn a_brand_is_read_as_validate_answers() {
    for text in ["abc", "ABC"] {
        let code = Code::model_schema_read_unjudged(serde_json::to_value(text.to_owned()).unwrap())
            .unwrap();
        read_as_validated(&code, code.validate());
    }
    assert_eq!(
        serde_json::from_str::<Code>(r#""abc""#)
            .map(Code::into_inner)
            .map_err(|refused| refused.to_string()),
        Err("a code is upper case".to_owned())
    );
    for codes in [texts(&[]), texts(&["a"])] {
        let batch =
            Batch::<String>::model_schema_read_unjudged(serde_json::to_value(codes).unwrap())
                .unwrap();
        read_as_validated(&batch, batch.validate());
    }
    for codes in [Vec::<u32>::new(), vec![7]] {
        let counted =
            Batch::<u32>::model_schema_read_unjudged(serde_json::to_value(codes).unwrap()).unwrap();
        read_as_validated(&counted, counted.validate());
    }
    assert_eq!(
        serde_json::from_str::<Batch<u32>>("[]")
            .map(Batch::into_inner)
            .map_err(|refused| refused.to_string()),
        Err("a batch holds at least one code".to_owned())
    );
}
