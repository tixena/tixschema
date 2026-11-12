//! `needs_wrapper()` on every kind of model type, held to what serde writes for a value of it.

#![cfg(feature = "serde")]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tixschema::model_schema;

#[model_schema]
#[derive(Serialize, Deserialize)]
pub struct Named {
    pub id: String,
}

#[model_schema]
#[derive(Serialize, Deserialize)]
pub struct Brand(String);

#[model_schema]
#[derive(Serialize, Deserialize)]
pub struct Pair(String, u32);

#[model_schema]
#[derive(Serialize, Deserialize)]
pub struct Single(u32);

#[model_schema]
#[derive(Serialize, Deserialize)]
pub struct Boxed(Named);

#[model_schema]
#[derive(Serialize, Deserialize)]
pub struct OverMap(HashMap<String, u32>);

#[model_schema]
#[derive(Serialize, Deserialize)]
pub struct OverList(Vec<String>);

#[model_schema]
#[derive(Serialize, Deserialize)]
pub enum Plain {
    First,
    Second,
}

#[model_schema]
#[derive(Serialize, Deserialize)]
pub enum Mixed {
    Nothing,
    Something(Named),
}

#[model_schema]
#[derive(Serialize, Deserialize)]
pub enum Wrapping {
    Card(Named),
    Cash { amount: u32 },
}

#[model_schema]
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Tagged {
    Busy { jobs: u32 },
    Idle,
}

#[model_schema]
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", content = "content")]
pub enum Adjacent {
    Busy(u32),
    Idle,
}

#[model_schema]
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum Objects {
    Card(Named),
    Cash { amount: u32 },
}

#[model_schema]
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum Texts {
    Card(Named),
    Label(String),
}

#[model_schema]
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum Nested {
    Inner(Brand),
}

#[model_schema]
#[derive(Serialize, Deserialize)]
pub struct Page<Item> {
    pub items: Vec<Item>,
}

#[model_schema]
#[derive(Serialize, Deserialize)]
pub struct Holder<Item>(Item);

#[model_schema]
#[derive(Serialize, Deserialize)]
pub struct Paged<Item>(Page<Item>);

#[model_schema]
#[derive(Serialize, Deserialize)]
pub struct Filled(Holder<Named>);

#[model_schema]
pub type Names = Vec<String>;

#[model_schema]
pub type Shown = Named;

#[model_schema]
pub type Counts = HashMap<String, u32>;

#[test]
fn an_alias_stands_for_what_it_names() {
    let names: Names = vec![];
    let shown: Shown = named();
    let counts: Counts = HashMap::new();
    assert!(!writes_an_object(&names));
    assert!(writes_an_object(&shown));
    assert!(writes_an_object(&counts));
}

fn writes_an_object<Written>(value: &Written) -> bool
where
    Written: Serialize,
{
    matches!(serde_json::to_value(value).unwrap(), Value::Object(_))
}

fn named() -> Named {
    NamedDraft {
        id: "doc-1".to_owned(),
    }
}

#[test]
fn a_struct_with_named_members_is_an_object() {
    assert!(!Named::needs_wrapper());
    assert!(!named_schema::Schema::needs_wrapper());
    assert!(writes_an_object(&named()));
}

#[test]
fn a_brand_over_a_string_is_wrapped() {
    assert!(Brand::needs_wrapper());
    assert!(brand_schema::Schema::needs_wrapper());
    assert!(!writes_an_object(&BrandDraft("doc-1".to_owned())));
}

#[test]
fn a_newtype_is_what_its_slot_is() {
    assert!(!OverMap::needs_wrapper());
    assert!(writes_an_object(&OverMapDraft(HashMap::new())));
    assert!(OverList::needs_wrapper());
    assert!(!writes_an_object(&OverListDraft(vec![])));
}

#[test]
fn a_tuple_struct_is_its_one_slot_or_an_array() {
    assert!(Pair::needs_wrapper());
    assert!(!writes_an_object(&PairDraft("a".to_owned(), 1)));
    assert!(Single::needs_wrapper());
    assert!(!writes_an_object(&SingleDraft(1)));
    assert!(!Boxed::needs_wrapper());
    assert!(writes_an_object(&BoxedDraft(named())));
}

#[test]
fn a_plain_enum_is_a_string() {
    assert!(Plain::needs_wrapper());
    assert!(plain_schema::Schema::needs_wrapper());
    assert!(!writes_an_object(&PlainDraft::First));
    assert!(!writes_an_object(&PlainDraft::Second));
}

#[test]
fn an_externally_tagged_enum_is_a_string_only_where_a_variant_is_a_unit() {
    assert!(Mixed::needs_wrapper());
    assert!(!writes_an_object(&MixedDraft::Nothing));
    assert!(writes_an_object(&MixedDraft::Something(named())));
    assert!(!Wrapping::needs_wrapper());
    assert!(writes_an_object(&WrappingDraft::Card(named())));
    assert!(writes_an_object(&WrappingDraft::Cash { amount: 1 }));
}

#[test]
fn an_enum_under_a_tag_is_an_object() {
    assert!(!Tagged::needs_wrapper());
    assert!(writes_an_object(&Tagged::Idle));
    assert!(writes_an_object(&TaggedDraft::Busy { jobs: 1 }));
    assert!(!Adjacent::needs_wrapper());
    assert!(writes_an_object(&AdjacentDraft::Idle));
    assert!(writes_an_object(&AdjacentDraft::Busy(1)));
}

#[test]
fn an_untagged_enum_is_an_object_where_every_member_is() {
    assert!(!Objects::needs_wrapper());
    assert!(writes_an_object(&ObjectsDraft::Card(named())));
    assert!(writes_an_object(&ObjectsDraft::Cash { amount: 1 }));
    assert!(Texts::needs_wrapper());
    assert!(!writes_an_object(&TextsDraft::Label("a".to_owned())));
    assert!(Nested::needs_wrapper());
    assert!(!writes_an_object(&NestedDraft::Inner(
        BrandDraft("a".to_owned()).validate().unwrap()
    )));
}

#[test]
fn a_generic_type_answers_with_its_type_filled_in() {
    assert!(!Page::<String>::needs_wrapper());
    assert!(writes_an_object(&PageDraft::<String> { items: vec![] }));
    assert!(Holder::<String>::needs_wrapper());
    assert!(Holder::<Named>::needs_wrapper());
    assert!(!writes_an_object(&HolderDraft::<String>("a".to_owned())));
    assert!(!Paged::<String>::needs_wrapper());
    assert!(writes_an_object(&PagedDraft::<String>(
        PageDraft { items: vec![] }.validate().unwrap()
    )));
    assert!(Filled::needs_wrapper());
}

#[test]
fn an_alias_answers_through_its_schema_module() {
    assert!(names_schema::Schema::needs_wrapper());
    assert!(!shown_schema::Schema::needs_wrapper());
    assert!(!counts_schema::Schema::needs_wrapper());
}
