//! A model type has a draft, which code builds and changes, and a valid type, which only the
//! draft's `validate()` makes.

use super::shapes::{
    AdjacentDraft, Audit, Colour, ColourDraft, Crate, CrateDraft, ExternalDraft, Internal,
    InternalDraft, LooseDraft, Marker, MarkerDraft, Note, NoteDraft, Pair, PairDraft, Tree,
    TreeDraft,
};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(minLength = 1, maxLength = 80)]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct LabelText(String);

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Shipment {
    pub label: String,
    #[model_schema_prop(minimum = 1, maximum = 100)]
    pub parcel_count: u32,
}

#[test]
fn a_draft_within_its_rules_validates() {
    let shipment = ShipmentDraft {
        label: "Shipping".to_owned(),
        parcel_count: 3,
    }
    .validate()
    .unwrap();
    assert_eq!(shipment.label, "Shipping");
    assert_eq!(shipment.parcel_count, 3);
}

#[test]
fn a_draft_that_breaks_a_rule_answers_the_violations() {
    let draft = ShipmentDraft {
        label: "Shipping".to_owned(),
        parcel_count: 200,
    };
    assert_eq!(
        draft.validation_issues(),
        vec!["'parcelCount': too large: maximum is 100, got 200".to_owned()]
    );
    assert_eq!(
        draft.validate().unwrap_err(),
        vec!["'parcelCount': too large: maximum is 100, got 200".to_owned()]
    );
}

#[test]
fn a_valid_value_turns_back_into_its_draft() {
    let draft = ShipmentDraft {
        label: "Shipping".to_owned(),
        parcel_count: 3,
    };
    let shipment = draft.clone().validate().unwrap();
    let back: ShipmentDraft = shipment.into();
    assert_eq!(back, draft);
}

#[test]
fn the_valid_type_writes_what_the_draft_writes() {
    let draft = ShipmentDraft {
        label: "Shipping".to_owned(),
        parcel_count: 3,
    };
    let shipment = draft.clone().validate().unwrap();
    assert_eq!(
        serde_json::to_string(&shipment).unwrap(),
        serde_json::to_string(&draft).unwrap()
    );
    assert_eq!(
        serde_json::to_string(&shipment).unwrap(),
        r#"{"label":"Shipping","parcelCount":3}"#
    );
}

#[test]
fn the_valid_type_reads_judged() {
    let read: Shipment = serde_json::from_str(r#"{"label":"A","parcelCount":3}"#).unwrap();
    assert_eq!(read.parcel_count, 3);
    let refused =
        serde_json::from_str::<Shipment>(r#"{"label":"A","parcelCount":200}"#).unwrap_err();
    assert!(
        refused
            .to_string()
            .starts_with("'parcelCount': too large: maximum is 100, got 200"),
        "{refused}"
    );
}

#[test]
fn a_draft_defaults_and_the_valid_type_does_not() {
    assert_eq!(ShipmentDraft::default().parcel_count, 0);
}

#[test]
fn a_generic_struct_has_the_shape() {
    let draft = CrateDraft::<u32> {
        items: vec![1],
        label: String::new(),
    };
    assert_eq!(
        draft.validation_issues(),
        vec!["'label': too short: minimum length is 1, got 0".to_owned()]
    );
    let ok = CrateDraft::<u32> {
        items: vec![1],
        label: "x".to_owned(),
    }
    .validate()
    .unwrap();
    assert_eq!(ok.items, vec![1]);
    let read: Crate<u32> = serde_json::from_str(r#"{"items":[1],"label":"x"}"#).unwrap();
    assert_eq!(read, ok);
    serde_json::from_str::<Crate<u32>>(r#"{"items":[1],"label":""}"#).unwrap_err();
}

#[test]
fn a_tuple_struct_has_the_shape() {
    PairDraft(0, 2).validate().unwrap_err();
    let pair = PairDraft(1, 2).validate().unwrap();
    assert_eq!(pair.0, 1);
    assert_eq!(serde_json::to_string(&pair).unwrap(), "[1,2]");
    serde_json::from_str::<Pair>("[0,2]").unwrap_err();
}

#[test]
fn a_unit_struct_has_the_shape() {
    let marker = MarkerDraft.validate().unwrap();
    assert_eq!(serde_json::to_string(&marker).unwrap(), "{}");
    let _read: Marker = serde_json::from_str("{}").unwrap();
}

#[test]
fn a_plain_enum_has_the_shape() {
    let green = ColourDraft::Green.validate().unwrap();
    assert!(matches!(&*green, ColourDraft::Green));
    assert_eq!(serde_json::to_string(&green).unwrap(), r#""Green""#);
    let read: Colour = serde_json::from_str(r#""Red""#).unwrap();
    assert!(matches!(&*read, ColourDraft::Red));
}

#[test]
fn every_tagging_has_the_shape() {
    let external = ExternalDraft::Card {
        number: "12".to_owned(),
    };
    assert_eq!(external.validation_issues().len(), 1);
    let valid = ExternalDraft::Cash.validate().unwrap();
    assert!(matches!(&*valid, ExternalDraft::Cash));
    assert_eq!(serde_json::to_string(&valid).unwrap(), r#""Cash""#);

    let internal = InternalDraft::Card {
        number: "1234".to_owned(),
    }
    .validate()
    .unwrap();
    assert_eq!(
        serde_json::to_string(&internal).unwrap(),
        r#"{"kind":"Card","number":"1234"}"#
    );
    serde_json::from_str::<Internal>(r#"{"kind":"Card","number":"1"}"#).unwrap_err();

    let adjacent = AdjacentDraft::Card {
        number: "1234".to_owned(),
    }
    .validate()
    .unwrap();
    assert_eq!(
        serde_json::to_string(&adjacent).unwrap(),
        r#"{"kind":"Card","value":{"number":"1234"}}"#
    );

    let loose = LooseDraft::Count(3).validate().unwrap();
    assert_eq!(serde_json::to_string(&loose).unwrap(), "3");
    LooseDraft::Card {
        number: "1".to_owned(),
    }
    .validate()
    .unwrap_err();
}

#[test]
fn a_flattened_member_has_the_shape() {
    let note = NoteDraft {
        audit: Audit::model_schema_read_unjudged(serde_json::json!({"revision": 2_u32})).unwrap(),
        text: "x".to_owned(),
    }
    .validate()
    .unwrap();
    assert_eq!(
        serde_json::to_string(&note).unwrap(),
        r#"{"revision":2,"text":"x"}"#
    );
    serde_json::from_str::<Note>(r#"{"revision":2,"text":""}"#).unwrap_err();
}

#[test]
fn a_recursive_type_has_the_shape() {
    let leaf = TreeDraft {
        name: "leaf".to_owned(),
        children: vec![],
    }
    .validate()
    .unwrap();
    let root = TreeDraft {
        name: "root".to_owned(),
        children: vec![leaf],
    }
    .validate()
    .unwrap();
    assert_eq!(root.children.len(), 1);
    let broken =
        serde_json::from_str::<Tree>(r#"{"name":"root","children":[{"name":"","children":[]}]}"#)
            .unwrap_err();
    assert!(broken.to_string().contains("too short"), "{broken}");
}
