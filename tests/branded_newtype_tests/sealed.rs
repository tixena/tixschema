//! A brand is built by `new` or `TryFrom` alone, and both hold the value to `validate()`. `new`
//! answers `Result<Self, Vec<String>>` on every brand, a brand that carries no rule included.

use core::marker::PhantomData;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

const TOO_SHORT: &str = "too short: minimum length is 1, got 0";

#[model_schema(minLength = 1, maxLength = 80)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct LabelText(String);

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct CorrelationId(String);

#[model_schema(minLength = 1, default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct DocumentId<IdType>(IdType);

#[model_schema(no_display, minLength = 1, maxLength = 2)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Parts(Vec<String>);

#[model_schema(no_display, minLength = 1)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Marked(String, #[serde(skip)] PhantomData<u8>);

#[model_schema(minLength = 1)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Named {
    #[serde(skip)]
    note: u8,
    text: String,
}

#[test]
fn new_answers_the_messages_validate_gives() {
    assert_eq!(LabelText::new("").unwrap_err(), vec![TOO_SHORT.to_owned()]);
    let built = LabelText::new("ok").unwrap();
    assert_eq!(built.as_inner(), "ok");
    assert_eq!(built.validation_issues(), Vec::<String>::new());
}

#[test]
fn try_from_builds_through_new() {
    assert_eq!(
        LabelText::try_from("ok".to_owned()).unwrap(),
        LabelText::new("ok").unwrap()
    );
    assert_eq!(
        LabelText::try_from(String::new()).unwrap_err(),
        vec![TOO_SHORT.to_owned()]
    );
}

#[test]
fn into_inner_gives_the_value_back() {
    assert_eq!(LabelText::new("ok").unwrap().into_inner(), "ok");
}

#[test]
fn a_brand_with_no_rule_takes_every_value_and_answers_a_result() {
    let built: Result<CorrelationId, Vec<String>> = CorrelationId::new("");
    assert_eq!(built.unwrap().as_inner(), "");
    assert_eq!(CorrelationId::new("x").unwrap().as_inner(), "x");
}

#[test]
fn a_generic_brand_is_sealed() {
    assert_eq!(
        DocumentId::new(String::new()).unwrap_err(),
        vec![TOO_SHORT.to_owned()]
    );
    assert_eq!(DocumentId::new("doc".to_owned()).unwrap().as_inner(), "doc");
    assert_eq!(DocumentId::new(7_u32).unwrap().into_inner(), 7);
}

#[test]
fn a_brand_over_a_list_is_sealed() {
    assert_eq!(
        Parts::new(Vec::new()).unwrap_err(),
        vec![TOO_SHORT.to_owned()]
    );
    assert_eq!(
        Parts::new(vec!["a".to_owned(), "b".to_owned(), "c".to_owned()]).unwrap_err(),
        vec!["too long: maximum length is 2, got 3".to_owned()]
    );
    assert_eq!(
        Parts::try_from(vec!["a".to_owned()]).unwrap().as_inner(),
        &["a".to_owned()]
    );
}

#[test]
fn a_brand_with_a_marker_slot_is_sealed() {
    assert_eq!(Marked::new("").unwrap_err(), vec![TOO_SHORT.to_owned()]);
    assert_eq!(Marked::new("ok").unwrap().as_inner(), "ok");
    assert_eq!(
        Marked::try_from("ok".to_owned()).unwrap().into_inner(),
        "ok"
    );
}

#[test]
fn a_brand_with_a_named_field_is_sealed() {
    assert_eq!(Named::new("").unwrap_err(), vec![TOO_SHORT.to_owned()]);
    assert_eq!(Named::new("ok").unwrap().as_inner(), "ok");
    assert_eq!(Named::try_from("ok".to_owned()).unwrap().into_inner(), "ok");
}

#[test]
fn the_unjudged_reads_stay_callable_and_skip_the_rule() {
    let unjudged = LabelText::model_schema_read_unjudged(serde_json::json!("")).unwrap();
    assert_eq!(unjudged.validation_issues(), vec![TOO_SHORT.to_owned()]);
}
