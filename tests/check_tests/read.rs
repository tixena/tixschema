//! A serde read of a type that names a check answers an error for the payload of a value the
//! check refuses, in every build: the error holds what `validate()` answers of the value.

use core::fmt::Debug;

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::models::{
    Batch, ClosedDock, Code, Crate, Grade, Handover, Pallet, Reference, Route, Shipment, Stack,
    Transfer, parcels,
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
        read_as_validated(&held, held.validate());
    }
    for stops in [texts(&[]), texts(&["a"])] {
        let route = Route { stops };
        read_as_validated(&route, route.validate());
    }
    for (codes, limit) in [
        (texts(&["a", "a"]), 5),
        (texts(&["a", "b"]), 1),
        (texts(&["a"]), 1),
    ] {
        let stack = Stack(codes, limit);
        read_as_validated(&stack, stack.validate());
    }
    for codes in [texts(&["a", "b", "c"]), texts(&["a"])] {
        let pallet = Pallet { codes };
        read_as_validated(&pallet, pallet.validate());
    }
    read_as_validated(&ClosedDock, ClosedDock.validate());
    assert!(parcels().iter().any(|held| held.validate().is_err()));
}

#[test]
fn an_enum_is_read_as_validate_answers_under_each_form_serde_writes_one_in() {
    let air = |stops: &[&str]| Shipment::Air {
        flight: "F1".to_owned(),
        stops: texts(stops),
    };
    for shipment in [
        air(&["a", "b", "c"]),
        air(&["a", "a"]),
        air(&["a"]),
        Shipment::Ground {
            stops: texts(&["a", "b", "c"]),
        },
    ] {
        read_as_validated(&shipment, shipment.validate());
    }
    for transfer in [
        Transfer::Road { stops: Vec::new() },
        Transfer::Rail(String::new()),
        Transfer::Rail("north".to_owned()),
        Transfer::Held,
    ] {
        read_as_validated(&transfer, transfer.validate());
    }
    for handover in [
        Handover::Signed { by: String::new() },
        Handover::Left("porch".to_owned()),
    ] {
        read_as_validated(&handover, handover.validate());
    }
    for reference in [
        Reference::Numbered(0),
        Reference::Numbered(7),
        Reference::Named {
            name: String::new(),
        },
        Reference::Named {
            name: "north".to_owned(),
        },
    ] {
        read_as_validated(&reference, reference.validate());
    }
    for grade in [Grade::Unrated, Grade::Express, Grade::Standard] {
        read_as_validated(&grade, grade.validate());
    }
}

#[test]
fn a_type_with_a_parameter_is_read_as_validate_answers_at_each_filling() {
    for codes in [texts(&[]), texts(&["a"])] {
        let filled = Crate {
            codes,
            label: "c".to_owned(),
        };
        read_as_validated(&filled, filled.validate());
    }
    for codes in [Vec::<u32>::new(), vec![7]] {
        let counted = Crate {
            codes,
            label: "c".to_owned(),
        };
        read_as_validated(&counted, counted.validate());
    }
}

#[test]
fn a_brand_is_read_as_validate_answers() {
    for text in ["abc", "ABC"] {
        let code = Code(text.to_owned());
        read_as_validated(&code, code.validate());
    }
    assert_eq!(
        serde_json::from_str::<Code>(r#""abc""#)
            .map(|read| read.0)
            .map_err(|refused| refused.to_string()),
        Err("a code is upper case".to_owned())
    );
    for codes in [texts(&[]), texts(&["a"])] {
        let batch = Batch(codes);
        read_as_validated(&batch, batch.validate());
    }
    for codes in [Vec::<u32>::new(), vec![7]] {
        let counted = Batch(codes);
        read_as_validated(&counted, counted.validate());
    }
    assert_eq!(
        serde_json::from_str::<Batch<u32>>("[]")
            .map(|read| read.0)
            .map_err(|refused| refused.to_string()),
        Err("a batch holds at least one code".to_owned())
    );
}
