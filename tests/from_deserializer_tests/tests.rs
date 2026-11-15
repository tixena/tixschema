use no_serde_json_apart::{Hold, HoldDraft, Leg, LegDraft, Mode, Shipment, ShipmentDraft, Tag};
use serde_json::Deserializer;
use serde_json::de::StrRead;

fn json(text: &str) -> Deserializer<StrRead<'_>> {
    Deserializer::from_str(text)
}

#[test]
fn a_struct_answers_every_broken_rule_as_a_list() {
    assert_eq!(
        Shipment::from_deserializer(&mut json(r#"{"label":"","parcelCount":200}"#)),
        Err(vec![
            "'label': too short: minimum length is 1, got 0".to_owned(),
            "'parcelCount': too large: maximum is 100, got 200".to_owned(),
        ])
    );
}

#[test]
fn a_payload_that_keeps_every_rule_answers_the_valid_value() {
    let read = Shipment::from_deserializer(&mut json(r#"{"label":"A","parcelCount":3}"#)).unwrap();
    assert_eq!(
        read,
        ShipmentDraft {
            label: "A".to_owned(),
            parcel_count: 3
        }
        .validate()
        .unwrap()
    );
}

#[test]
fn malformed_json_answers_the_one_entry_serde_json_words() {
    let said = serde_json::from_str::<ShipmentDraft>("{")
        .unwrap_err()
        .to_string();
    assert_eq!(Shipment::from_deserializer(&mut json("{")), Err(vec![said]));
}

#[test]
fn the_messages_are_what_validation_issues_answers_entry_by_entry() {
    let broken = ShipmentDraft {
        label: String::new(),
        parcel_count: 200,
    };
    assert_eq!(
        Shipment::from_deserializer(&mut json(r#"{"label":"","parcelCount":200}"#)),
        Err(broken.validation_issues())
    );
    let broken_leg = LegDraft::Sea {
        vessel: String::new(),
    };
    assert_eq!(
        Leg::from_deserializer(&mut json(r#"{"kind":"sea","vessel":""}"#)),
        Err(broken_leg.validation_issues())
    );
    let broken_hold = HoldDraft::<u32> {
        items: vec![1],
        label: String::new(),
    };
    assert_eq!(
        Hold::<u32>::from_deserializer(&mut json(r#"{"items":[1],"label":""}"#)),
        Err(broken_hold.validation_issues())
    );
}

#[test]
fn an_enum_with_a_draft_reads_and_answers_by_variant() {
    assert_eq!(
        Leg::from_deserializer(&mut json(r#"{"kind":"air","stops":3}"#))
            .unwrap()
            .model_schema_variant(),
        "Air"
    );
    assert_eq!(
        Leg::from_deserializer(&mut json(r#"{"kind":"air","stops":30}"#)),
        Err(vec!["'stops': too large: maximum is 9, got 30".to_owned()])
    );
}

#[test]
fn a_plain_enum_answers_the_deserializers_own_error_as_the_one_entry() {
    assert_eq!(
        Mode::from_deserializer(&mut json(r#""Air""#)),
        Ok(Mode::Air)
    );
    let said = serde_json::from_str::<Mode>(r#""Rail""#)
        .unwrap_err()
        .to_string();
    assert_eq!(
        Mode::from_deserializer(&mut json(r#""Rail""#)),
        Err(vec![said])
    );
}

#[test]
fn a_generic_struct_read_at_a_filling_judges_its_own_rules() {
    let read = Hold::<u32>::from_deserializer(&mut json(r#"{"items":[1,2],"label":"A"}"#)).unwrap();
    assert_eq!(read.items, vec![1, 2]);
    assert_eq!(
        Hold::<u32>::from_deserializer(&mut json(r#"{"items":[1],"label":""}"#)),
        Err(vec![
            "'label': too short: minimum length is 1, got 0".to_owned()
        ])
    );
}

#[test]
fn a_brand_answers_the_message_of_its_bound() {
    assert_eq!(
        Tag::from_deserializer(&mut json(r#""ab""#)).unwrap(),
        Tag::new("ab".to_owned()).unwrap()
    );
    assert_eq!(
        Tag::from_deserializer(&mut json(r#""a""#)),
        Err(vec!["too short: minimum length is 2, got 1".to_owned()])
    );
}

#[test]
fn the_crate_apart_reads_through_a_function_that_names_no_format() {
    assert_eq!(
        no_serde_json_apart::shipment(&mut json(r#"{"label":"","parcelCount":1}"#)),
        Err(vec![
            "'label': too short: minimum length is 1, got 0".to_owned()
        ])
    );
}

/// The guide shows the sample this file runs.
#[test]
fn the_guide_shows_the_sample() {
    let guide = include_str!("../../docs/guide/validation.md");
    for shown in [
        "let shipment = Shipment::from_deserializer(&mut serde_json::Deserializer::from_str(",
        r##"    r#"{"label":"","parcelCount":200}"#,"##,
        r#"    "'label': too short: minimum length is 1, got 0".to_owned(),"#,
        r#"    "'parcelCount': too large: maximum is 100, got 200".to_owned(),"#,
    ] {
        assert!(guide.contains(shown), "the guide does not show: {shown}");
    }
}
