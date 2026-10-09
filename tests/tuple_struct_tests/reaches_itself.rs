//! A tuple struct whose slot reaches the struct itself, or a type declared below it.
//!
//! `z.tuple` takes an array, which has no getter to defer a member behind, so the slot is written
//! behind `z.lazy`: read as it stands, it names a binding that is still being built and the
//! generated module throws as it is imported.

#![cfg(all(feature = "serde", feature = "zod", feature = "typescript"))]

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Declared above the type its first slot holds.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AheadOfItsSlot(pub DeclaredBelow, pub u32);

#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclaredBelow {
    pub label: String,
}

/// One slot, so serde writes the list alone.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NestsAlone(pub Vec<Self>);

#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NestsInASlot(pub String, pub Vec<Self>);

#[model_schema(default_types(IdType = String))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NestsUnderAParameter<IdType>(pub IdType, pub Vec<Self>);

/// The README shows this declaration and this line of its schema.
#[test]
fn a_slot_reaching_the_struct_is_read_behind_a_lazy() {
    let written = "const NestsInASlot$RawSchema = z.tuple([z.string(), z.lazy(() => \
                   z.array(NestsInASlot$Schema))]);";
    let zod = NestsInASlot::zod_schema();
    assert!(zod.contains(written), "got: {zod}");
    let readme = include_str!("../../README.md");
    assert!(
        readme.contains(written),
        "the README no longer shows: {written}"
    );
    assert!(
        readme.contains("pub struct NestsInASlot(pub String, pub Vec<Self>);"),
        "the README no longer declares the struct"
    );
}

#[test]
fn a_lone_slot_reaching_the_struct_is_read_behind_a_lazy() {
    let zod = NestsAlone::zod_schema();
    assert!(
        zod.contains("const NestsAlone$RawSchema = z.lazy(() => z.array(NestsAlone$Schema));"),
        "got: {zod}"
    );
}

#[test]
fn a_slot_reaching_a_type_declared_below_is_read_behind_a_lazy() {
    let zod = AheadOfItsSlot::zod_schema();
    assert!(
        zod.contains(
            "const AheadOfItsSlot$RawSchema = z.tuple([z.lazy(() => DeclaredBelow$Schema), \
             z.number().int()]);"
        ),
        "got: {zod}"
    );
}

#[test]
fn a_generic_one_reaches_itself_through_a_function_stating_what_it_parses() {
    let zod = NestsUnderAParameter::<String>::zod_schema();
    for written in [
        "function NestsUnderAParameter$SchemaSelf<IdType extends ZodType>(\n  idType: IdType,\n): \
         ZodType<NestsUnderAParameter<IdType[\"_zod\"][\"output\"]>>;\nfunction \
         NestsUnderAParameter$SchemaSelf(\n  idType: ZodType,\n): ZodType {\n  return \
         NestsUnderAParameter$SchemaFactory(idType);\n}\n\n",
        ") =>\n  z.tuple([idType, z.array(z.lazy(() => \
         NestsUnderAParameter$SchemaSelf(idType)))]);",
        "interface NestsUnderAParameter$SchemaOf<IdType extends ZodType> extends ReturnType<",
    ] {
        assert!(zod.contains(written), "want: {written}\ngot: {zod}");
    }
}

#[test]
fn serde_writes_each_as_the_nested_arrays_its_schema_describes() {
    let nested = NestsInASlot(
        "a".to_owned(),
        vec![NestsInASlot("b".to_owned(), Vec::new())],
    );
    assert_eq!(
        serde_json::to_value(&nested).unwrap(),
        serde_json::json!(["a", [["b", []]]])
    );
    assert_eq!(
        serde_json::to_value(NestsAlone(vec![NestsAlone(Vec::new())])).unwrap(),
        serde_json::json!([[]])
    );
    assert_eq!(
        serde_json::to_value(AheadOfItsSlot(
            DeclaredBelow {
                label: "x".to_owned()
            },
            1
        ))
        .unwrap(),
        serde_json::json!([{ "label": "x" }, 1_u32])
    );
    assert_eq!(
        serde_json::to_value(NestsUnderAParameter(
            "a".to_owned(),
            vec![NestsUnderAParameter("b".to_owned(), Vec::new())]
        ))
        .unwrap(),
        serde_json::json!(["a", [["b", []]]])
    );
}
