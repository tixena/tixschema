//! A serde read of a type that names a check answers an error for the payload of a value the
//! check refuses, in every build: the error holds what `validate()` answers of the value.

use core::fmt::Debug;

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::models::{
    Batch, ClosedDockDraft, Code, CrateDraft, GradeDraft, HandoverDraft, PalletDraft,
    ReferenceDraft, RouteDraft, ShipmentDraft, StackDraft, TransferDraft, parcels,
};

fn texts(written: &[&str]) -> Vec<String> {
    written.iter().map(|&text| text.to_owned()).collect()
}

/// Holds the read of what serde writes for `value` to what `validate()` answers of `value`: an
/// error with every message where it reports any, and the value where it reports none.
fn read_as_validated<D, V>(value: &D, validated: Result<V, Vec<String>>)
where
    D: Serialize,
    V: Debug + DeserializeOwned + Serialize,
{
    let payload = serde_json::to_string(value).unwrap();
    let read = serde_json::from_str::<V>(&payload);
    match validated {
        Ok(_) => assert_eq!(
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
        read_as_validated(&held, held.clone().into_draft().validate());
    }
    for stops in [texts(&[]), texts(&["a"])] {
        let route = RouteDraft { stops };
        read_as_validated(&route, route.clone().validate());
    }
    for (codes, limit) in [
        (texts(&["a", "a"]), 5),
        (texts(&["a", "b"]), 1),
        (texts(&["a"]), 1),
    ] {
        let stack = StackDraft(codes, limit);
        read_as_validated(&stack, stack.clone().validate());
    }
    for codes in [texts(&["a", "b", "c"]), texts(&["a"])] {
        let pallet = PalletDraft { codes };
        read_as_validated(&pallet, pallet.clone().validate());
    }
    read_as_validated(&ClosedDockDraft, ClosedDockDraft.clone().validate());
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
        read_as_validated(&shipment, shipment.clone().validate());
    }
    for transfer in [
        TransferDraft::Road { stops: Vec::new() },
        TransferDraft::Rail(String::new()),
        TransferDraft::Rail("north".to_owned()),
        TransferDraft::Held,
    ] {
        read_as_validated(&transfer, transfer.clone().validate());
    }
    for handover in [
        HandoverDraft::Signed { by: String::new() },
        HandoverDraft::Left("porch".to_owned()),
    ] {
        read_as_validated(&handover, handover.clone().validate());
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
        read_as_validated(&reference, reference.clone().validate());
    }
    for grade in [
        GradeDraft::Unrated,
        GradeDraft::Express,
        GradeDraft::Standard,
    ] {
        read_as_validated(&grade, grade.clone().validate());
    }
}

#[test]
fn a_type_with_a_parameter_is_read_as_validate_answers_at_each_filling() {
    for codes in [texts(&[]), texts(&["a"])] {
        let filled = CrateDraft {
            codes,
            label: "c".to_owned(),
        };
        read_as_validated(&filled, filled.clone().validate());
    }
    for codes in [Vec::<u32>::new(), vec![7]] {
        let counted = CrateDraft {
            codes,
            label: "c".to_owned(),
        };
        read_as_validated(&counted, counted.clone().validate());
    }
}

#[test]
fn a_brand_is_read_as_validate_answers() {
    for text in ["abc", "ABC"] {
        let code = Code::model_schema_read_unjudged(serde_json::to_value(text.to_owned()).unwrap())
            .unwrap();
        read_as_validated(&code, code.validate().map(|()| code.clone()));
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
        read_as_validated(&batch, batch.validate().map(|()| batch.clone()));
    }
    for codes in [Vec::<u32>::new(), vec![7]] {
        let counted =
            Batch::<u32>::model_schema_read_unjudged(serde_json::to_value(codes).unwrap()).unwrap();
        read_as_validated(&counted, counted.validate().map(|()| counted.clone()));
    }
    assert_eq!(
        serde_json::from_str::<Batch<u32>>("[]")
            .map(Batch::into_inner)
            .map_err(|refused| refused.to_string()),
        Err("a batch holds at least one code".to_owned())
    );
}
