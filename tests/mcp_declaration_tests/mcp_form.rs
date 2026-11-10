//! `mcp_form_value()` and `mcp_form_flat()` on every kind of model type.

#![cfg(feature = "serde")]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tixschema::model_schema;

#[model_schema]
#[derive(Serialize, Deserialize)]
pub enum Choice {
    First,
    Second,
}

#[model_schema]
#[derive(Serialize, Deserialize)]
pub enum Carrying {
    Nothing,
    Something(String),
}

#[model_schema]
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Tagged {
    Idle,
}

#[model_schema]
#[derive(Serialize, Deserialize)]
pub struct Address {
    pub street: String,
}

#[model_schema]
#[derive(Serialize, Deserialize)]
pub struct EveryKindOfField {
    pub choice: Choice,
    pub choices: Vec<Choice>,
    pub count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maybe: Option<Choice>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub ratio: f64,
    pub signed: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub some_choices: Option<Vec<Choice>>,
    pub text: String,
    pub yes: bool,
}

#[model_schema]
#[derive(Serialize, Deserialize)]
pub struct HoldsAStruct {
    pub address: Address,
}

#[model_schema]
#[derive(Serialize, Deserialize)]
pub struct HoldsAMap {
    pub counts: HashMap<String, u32>,
}

#[model_schema]
#[derive(Serialize, Deserialize)]
pub struct HoldsTexts {
    pub tags: Vec<String>,
}

#[model_schema]
#[derive(Serialize, Deserialize)]
pub struct HoldsAnEnumWithData {
    pub carrying: Carrying,
}

#[model_schema]
#[derive(Serialize, Deserialize)]
pub struct HoldsATaggedEnum {
    pub tagged: Tagged,
}

#[model_schema]
#[derive(Serialize, Deserialize)]
pub struct Flattening {
    #[serde(flatten)]
    pub address: Address,
    pub text: String,
}

#[model_schema]
#[derive(Serialize, Deserialize)]
pub struct Brand(String);

#[model_schema]
#[derive(Serialize, Deserialize)]
pub struct Pair(String, u32);

#[model_schema]
pub type Shown = EveryKindOfField;

#[test]
fn an_enum_is_a_value_where_no_variant_holds_data() {
    assert!(Choice::mcp_form_value());
    assert!(!Carrying::mcp_form_value());
    assert!(!Tagged::mcp_form_value());
    assert!(!Choice::mcp_form_flat());
}

#[test]
fn a_struct_is_never_a_value() {
    assert!(!Address::mcp_form_value());
    assert!(!EveryKindOfField::mcp_form_value());
}

#[test]
fn a_struct_of_text_numbers_yes_or_no_and_choices_is_flat() {
    assert!(Address::mcp_form_flat());
    assert!(EveryKindOfField::mcp_form_flat());
    assert!(Shown::mcp_form_flat());
}

#[test]
fn a_struct_that_holds_anything_else_is_not_flat() {
    assert!(!HoldsAStruct::mcp_form_flat());
    assert!(!HoldsAMap::mcp_form_flat());
    assert!(!HoldsTexts::mcp_form_flat());
    assert!(!HoldsAnEnumWithData::mcp_form_flat());
    assert!(!HoldsATaggedEnum::mcp_form_flat());
}

#[test]
fn a_flattened_member_makes_a_struct_not_flat() {
    assert!(!Flattening::mcp_form_flat());
}

#[test]
fn a_tuple_struct_is_not_flat() {
    assert!(!Brand::mcp_form_flat());
    assert!(!Pair::mcp_form_flat());
}
