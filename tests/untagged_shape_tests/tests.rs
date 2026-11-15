use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(nonstrict)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrueArm {
    pub advanced: bool,
    pub formula: String,
    #[model_schema_prop(literal = true)]
    pub is_calculated: bool,
}

#[model_schema(nonstrict)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum Calculated {
    Computed(TrueArm),
    NotCalculated {
        #[model_schema_prop(literal = false)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        is_calculated: Option<bool>,
    },
}

#[model_schema(minLength = 2)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Label(String);

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Either {
    Labelled(Label),
    Plain(String),
}

#[test]
fn a_literal_chooses_no_variant() {
    let payload = r#"{"isCalculated": false, "advanced": false, "formula": "a + b"}"#;
    assert_eq!(
        Calculated::from_deserializer(&mut serde_json::Deserializer::from_str(payload)),
        Err(vec![
            "'isCalculated': not the literal true, got false".to_owned()
        ])
    );
    let said = serde_json::from_str::<Calculated>(payload)
        .unwrap_err()
        .to_string();
    assert!(
        said.contains("'isCalculated': not the literal true, got false"),
        "got: {said}"
    );
}

#[test]
fn a_payload_with_no_formula_is_read_by_shape_as_the_other_variant() {
    let read = serde_json::from_str::<Calculated>(r#"{"isCalculated": false}"#).unwrap();
    assert_eq!(read.model_schema_variant(), "NotCalculated");
}

#[test]
fn a_brands_bound_chooses_no_variant() {
    assert_eq!(
        Either::from_deserializer(&mut serde_json::Deserializer::from_str(r#""a""#)),
        Err(vec!["too short: minimum length is 2, got 1".to_owned()])
    );
    let read = serde_json::from_str::<Either>(r#""ab""#).unwrap();
    assert_eq!(read.model_schema_variant(), "Labelled");
}

#[test]
fn the_guide_shows_the_declarations() {
    let guide = include_str!("../../docs/guide/literals-and-type-names.md");
    for shown in [
        "pub struct TrueArm {",
        "#[serde(rename_all = \"camelCase\", untagged)]",
        "pub enum Calculated {",
        "'isCalculated': not the literal true, got false",
    ] {
        assert!(guide.contains(shown), "the guide does not show: {shown}");
    }
}
