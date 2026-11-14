//! Tests that a documented item's Rust example stays out of the emitted TypeScript.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A documented struct.
/// ```rust example
/// let item = DocumentedStructDraft {
///     label: "alpha".to_string(),
/// };
/// println!("Item: {:?}", item);
/// ```
#[cfg(feature = "serde")]
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug)]
pub struct DocumentedStruct {
    pub label: String,
}

/// A documented struct whose example imports over several lines.
/// ```rust example
/// use std::collections::{
///     BTreeMap,
///     HashMap,
/// };
/// let item = WrappedImportStructDraft {
///     label: "alpha".to_string(),
/// };
/// println!("Item: {:?}", item);
/// ```
#[cfg(feature = "serde")]
#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
pub struct WrappedImportStruct {
    pub label: String,
}

/// A documented tuple struct.
/// ```rust example
/// let item = DocumentedTupleDraft("alpha".to_string(), 7);
/// println!("Item: {:?}", item);
/// ```
#[cfg(feature = "serde")]
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug)]
pub struct DocumentedTuple(pub String, pub u32);

/// A documented adjacently tagged enum.
/// ```rust example
/// let item = DocumentedAdjacentDraft::Unit;
/// println!("Item: {:?}", item);
/// ```
#[cfg(feature = "serde")]
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "kind", content = "payload")]
pub enum DocumentedAdjacent {
    Named { side: u8 },
    Unit,
}

/// A documented externally tagged enum.
/// ```rust example
/// let item = DocumentedExternalDraft::Unit;
/// println!("Item: {:?}", item);
/// ```
#[cfg(feature = "serde")]
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug)]
pub enum DocumentedExternal {
    Named { side: u8 },
    Unit,
}

/// A documented internally tagged enum.
/// ```rust example
/// let item = DocumentedInternalDraft::Unit;
/// println!("Item: {:?}", item);
/// ```
#[cfg(feature = "serde")]
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "kind")]
pub enum DocumentedInternal {
    Named { side: u8 },
    Unit,
}

/// A documented untagged enum.
/// ```rust example
/// let item = DocumentedUntaggedDraft::Count(3);
/// println!("Item: {:?}", item);
/// ```
#[cfg(feature = "serde")]
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum DocumentedUntagged {
    Count(u32),
    Label(String),
}

/// A documented plain enum.
/// ```rust example
/// let item = DocumentedSlot::Primary;
/// println!("Item: {:?}", item);
/// ```
#[cfg(feature = "serde")]
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum DocumentedSlot {
    Primary,
    Secondary,
}

/// A documented alias.
/// ```rust example
/// let item: DocumentedAlias = 3;
/// println!("Item: {:?}", item);
/// ```
#[cfg(any(feature = "serde", feature = "zod"))]
#[model_schema(enable = ["jsonschema"])]
pub type DocumentedAlias = u32;

/// An alias of a documented struct, which writes no block of its own.
#[cfg(feature = "serde")]
#[model_schema(enable = ["jsonschema"])]
pub type DocumentedStructRef = DocumentedStruct;

/// An alias of a primitive, which writes no block.
#[model_schema(enable = ["jsonschema"])]
pub type UndocumentedCount = u32;

/// A struct whose documented member is a field.
#[cfg(feature = "serde")]
#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
pub struct DocumentedFieldHolder {
    /// A documented field.
    /// ```rust example
    /// let item = DocumentedFieldHolderDraft {
    ///     label: "alpha".to_string(),
    /// };
    /// println!("Item: {:?}", item);
    /// ```
    pub label: String,
}

/// An internally tagged enum whose documented member is a variant.
#[cfg(feature = "serde")]
#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "kind")]
pub enum DocumentedInternalVariant {
    /// A documented variant.
    /// ```rust example
    /// let item = DocumentedInternalVariantDraft::Unit;
    /// println!("Item: {:?}", item);
    /// ```
    Named {
        side: u8,
    },
    Unit,
}

/// An externally tagged enum whose documented member is a variant.
#[cfg(feature = "serde")]
#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
pub enum DocumentedExternalVariant {
    /// A documented variant.
    /// ```rust example
    /// let item = DocumentedExternalVariantDraft::Unit;
    /// println!("Item: {:?}", item);
    /// ```
    Named {
        side: u8,
    },
    Unit,
}

/// A plain enum whose documented member is a variant, commented inside the union rather than over
/// a property — the one member body not written as ` * ` lines.
#[cfg(feature = "serde")]
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum DocumentedSlotVariant {
    /// A documented slot.
    /// ```rust example
    /// let item = DocumentedSlotVariantDraft::Primary;
    /// println!("Item: {:?}", item);
    /// ```
    Primary,
    Secondary,
}

/// ```rust example
/// let item = ExampleOnlyStructDraft {
///     label: "alpha".to_string(),
/// };
/// println!("Item: {:?}", item);
/// ```
#[cfg(feature = "serde")]
#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
pub struct ExampleOnlyStruct {
    /// ```rust example
    /// let label = "alpha".to_string();
    /// println!("Label: {}", label);
    /// ```
    pub label: String,
}

// An internally tagged enum whose one documented member says nothing but an example. Commented with
// `//`, so the item itself is held against its twin as the undocumented one it is.
#[cfg(feature = "serde")]
#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "kind")]
pub enum ExampleOnlyVariantHolder {
    /// ```rust example
    /// let item = ExampleOnlyVariantHolderDraft::Unit;
    /// println!("Item: {:?}", item);
    /// ```
    Named {
        side: u8,
    },
    Unit,
}

/// ```rust example
/// let item: ExampleOnlyAlias = 3;
/// println!("Item: {:?}", item);
/// ```
#[cfg(feature = "serde")]
#[model_schema()]
pub type ExampleOnlyAlias = u32;

/// ```rust example
/// let item = ExampleOnlySlot::Primary;
/// println!("Item: {:?}", item);
/// ```
#[cfg(feature = "serde")]
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum ExampleOnlySlot {
    Primary,
    Secondary,
}

/// ```rust example
/// let item = ExampleOnlyBrand("alpha".to_string());
/// println!("Item: {:?}", item);
/// ```
// A brand publishes a description and no `JSDoc` header of its own, so it is read on the Zod
// surface alone and is gated on the feature that surface is.
#[cfg(all(feature = "serde", feature = "zod"))]
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct ExampleOnlyBrand(String);

// The undocumented twins of the shapes above.
#[cfg(feature = "serde")]
#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
pub struct UndocumentedStruct {
    pub label: String,
}

#[cfg(feature = "serde")]
#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "kind")]
pub enum UndocumentedVariantHolder {
    Named { side: u8 },
    Unit,
}

#[cfg(feature = "serde")]
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum UndocumentedSlot {
    Primary,
    Secondary,
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct UndocumentedBrand(String);

#[cfg(feature = "serde")]
#[model_schema()]
pub type UndocumentedAlias = u32;

/// Every documented shape above, paired with the description its `JSDoc` is left with once the
/// example is dropped. The plain enum rides along as the shape that already dropped it.
#[cfg(feature = "serde")]
fn documented_shapes() -> Vec<(String, &'static str)> {
    vec![
        (DocumentedStruct::ts_definition(), "A documented struct."),
        (
            DocumentedTuple::ts_definition(),
            "A documented tuple struct.",
        ),
        (
            DocumentedAdjacent::ts_definition(),
            "A documented adjacently tagged enum.",
        ),
        (
            DocumentedExternal::ts_definition(),
            "A documented externally tagged enum.",
        ),
        (
            DocumentedInternal::ts_definition(),
            "A documented internally tagged enum.",
        ),
        (
            DocumentedUntagged::ts_definition(),
            "A documented untagged enum.",
        ),
        (DocumentedSlot::ts_definition(), "A documented plain enum."),
        (
            documented_alias_schema::Schema::ts_definition(),
            "A documented alias.",
        ),
    ]
}

/// Every shape whose example is written on a member rather than on the type, paired with the
/// description that member's `JSDoc` is left with once the example is dropped.
#[cfg(feature = "serde")]
fn documented_members() -> Vec<(String, &'static str)> {
    vec![
        (
            DocumentedFieldHolder::ts_definition(),
            "A documented field.",
        ),
        (
            DocumentedInternalVariant::ts_definition(),
            "A documented variant.",
        ),
        (
            DocumentedExternalVariant::ts_definition(),
            "A documented variant.",
        ),
        (DocumentedSlotVariant::ts_definition(), "A documented slot."),
    ]
}

#[cfg(feature = "serde")]
#[test]
fn a_documented_item_never_carries_its_rust_example_into_the_emitted_typescript() {
    // The alias publishes `ts_definition` from its own module rather than from the alias, so naming
    // the type here is what keeps the fixture from being pruned as unused.
    let aliased: DocumentedAlias = 3;
    assert_eq!(aliased, 3);

    for (ts, description) in documented_shapes() {
        for leaked in ["```", "let item", "println!"] {
            assert!(!ts.contains(leaked), "{leaked} reached: {ts}");
        }
        assert!(ts.contains(description), "docs dropped from: {ts}");
    }
}

#[cfg(feature = "serde")]
#[test]
fn a_documented_member_never_carries_its_rust_example_into_the_emitted_typescript() {
    for (ts, description) in documented_members() {
        for leaked in ["```", "let item", "println!"] {
            assert!(!ts.contains(leaked), "{leaked} reached: {ts}");
        }
        assert!(ts.contains(description), "docs dropped from: {ts}");
    }
}

/// An alias builds its block as a type does: the value reaches its `.meta` and nothing else of the
/// block does.
#[cfg(feature = "zod")]
#[test]
fn a_documented_alias_publishes_its_rust_example_as_one_example() {
    let zod = documented_alias_schema::Schema::zod_schema();
    for leaked in ["```", "let item", "println!", "example:"] {
        assert!(!zod.contains(leaked), "{leaked} reached: {zod}");
    }
    assert!(zod.contains(".meta({\n  examples: [3]\n});"), "Got: {zod}");
    assert_eq!(
        documented_alias_schema::Schema::schema_examples(),
        vec![serde_json::json!(3_u32)]
    );
    assert_eq!(
        documented_alias_schema::Schema::schema_example(),
        serde_json::json!(3_u32)
    );
}

/// An alias that names a published type and writes no block states no examples of its own.
#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn an_alias_with_no_block_publishes_the_examples_of_the_type_it_names() {
    let held: DocumentedStructRef = DocumentedStructDraft {
        label: "alpha".to_owned(),
    }
    .into();
    assert_eq!(held.label, "alpha");
    let zod = documented_struct_ref_schema::Schema::zod_schema();
    assert!(!zod.contains(".meta("), "Got: {zod}");
    assert!(
        zod.contains("const DocumentedStructRefType$RawSchema = DocumentedStruct$Schema;"),
        "Got: {zod}"
    );
}

#[test]
fn an_alias_of_a_primitive_with_no_block_is_that_primitive() {
    let count: UndocumentedCount = 4;
    assert_eq!(count, 4);
}

/// An alias of a primitive that writes no block has nothing to show through.
#[cfg(feature = "zod")]
#[test]
fn an_alias_of_a_primitive_with_no_block_publishes_no_examples() {
    let zod = undocumented_count_schema::Schema::zod_schema();
    assert!(zod.contains(".meta({\n  examples: []\n});"), "Got: {zod}");
}

/// Every shape documented with nothing but an example, paired with the undocumented twin it is
/// held against: the `TypeScript` one writes is the other's with the fixture's own name in it.
#[cfg(feature = "serde")]
fn example_only_twins() -> Vec<(String, String, &'static str, &'static str)> {
    vec![
        (
            ExampleOnlyStruct::ts_definition(),
            UndocumentedStruct::ts_definition(),
            "ExampleOnlyStruct",
            "UndocumentedStruct",
        ),
        (
            ExampleOnlyVariantHolder::ts_definition(),
            UndocumentedVariantHolder::ts_definition(),
            "ExampleOnlyVariantHolder",
            "UndocumentedVariantHolder",
        ),
        (
            ExampleOnlySlot::ts_definition(),
            UndocumentedSlot::ts_definition(),
            "ExampleOnlySlot",
            "UndocumentedSlot",
        ),
        (
            example_only_alias_schema::Schema::ts_definition(),
            undocumented_alias_schema::Schema::ts_definition(),
            "ExampleOnlyAlias",
            "UndocumentedAlias",
        ),
    ]
}

#[cfg(feature = "serde")]
#[test]
fn an_example_only_shape_writes_what_its_undocumented_twin_writes() {
    // The aliases publish `ts_definition` from their own modules rather than from the alias, so
    // naming the types here is what keeps the fixtures from being pruned as unused.
    let aliased: ExampleOnlyAlias = 3;
    let aliased_twin: UndocumentedAlias = 3;
    assert_eq!(aliased, aliased_twin);

    for (example_only, twin, named, twin_named) in example_only_twins() {
        assert_eq!(example_only.replace(named, twin_named), twin, "for {named}");
    }
}

#[cfg(feature = "serde")]
#[test]
fn an_example_only_item_and_member_name_themselves() {
    let ts = ExampleOnlyStruct::ts_definition();
    assert!(
        ts.starts_with("/**\n * ExampleOnlyStruct\n * \n"),
        "item did not name itself: {ts}"
    );
    assert!(
        ts.contains("  /**\n   * label\n   * \n"),
        "field unnamed: {ts}"
    );

    let variants = ExampleOnlyVariantHolder::ts_definition();
    assert!(
        variants.contains("  /**\n   * Named\n   * \n"),
        "variant unnamed: {variants}"
    );

    let alias = example_only_alias_schema::Schema::ts_definition();
    assert!(
        alias.starts_with("/**\n * ExampleOnlyAliasType\n * \n"),
        "alias did not name itself: {alias}"
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn an_example_only_shape_describes_itself_as_its_undocumented_twin_does() {
    for (zod, exported) in [
        (ExampleOnlySlot::zod_schema(), "ExampleOnlySlot"),
        (UndocumentedSlot::zod_schema(), "UndocumentedSlot"),
        (ExampleOnlyBrand::zod_schema(), "ExampleOnlyBrand"),
        (UndocumentedBrand::zod_schema(), "UndocumentedBrand"),
    ] {
        assert!(
            zod.contains(&format!("description: \"{exported}\"")),
            "{exported} did not describe itself: {zod}"
        );
    }
}

#[cfg(feature = "serde")]
#[test]
fn every_documented_shape_opens_its_jsdoc_identically() {
    for (ts, description) in documented_shapes() {
        let header = format!("/**\n * {description}\n * \n");
        assert!(
            ts.starts_with(&header),
            "expected {description} to open: {ts}"
        );
    }
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn an_example_importing_over_several_lines_builds_and_is_published() {
    assert_eq!(
        WrappedImportStruct::schema_examples(),
        vec![serde_json::json!({"label": "alpha"})]
    );
    assert!(
        WrappedImportStruct::zod_schema().contains("examples: [{\"label\":\"alpha\"}]"),
        "Got: {}",
        WrappedImportStruct::zod_schema()
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_type_with_examples_writes_them_under_the_json_schema_examples_keyword() {
    assert_eq!(
        DocumentedStruct::json_schema()["examples"],
        serde_json::json!([{"label": "alpha"}])
    );
    assert_eq!(
        documented_alias_schema::Schema::json_schema()["examples"],
        serde_json::json!([3_u32])
    );
}

#[test]
fn a_type_with_no_example_writes_no_json_schema_examples_keyword() {
    assert!(
        undocumented_count_schema::Schema::json_schema()
            .get("examples")
            .is_none()
    );
}

#[cfg(feature = "serde")]
#[test]
fn the_schema_module_and_the_type_write_one_document_with_the_examples_in_it() {
    let documents = [
        (
            DocumentedStruct::json_schema(),
            documented_struct_schema::Schema::json_schema(),
        ),
        (
            DocumentedTuple::json_schema(),
            documented_tuple_schema::Schema::json_schema(),
        ),
        (
            DocumentedAdjacent::json_schema(),
            documented_adjacent_schema::Schema::json_schema(),
        ),
        (
            DocumentedExternal::json_schema(),
            documented_external_schema::Schema::json_schema(),
        ),
        (
            DocumentedInternal::json_schema(),
            documented_internal_schema::Schema::json_schema(),
        ),
        (
            DocumentedUntagged::json_schema(),
            documented_untagged_schema::Schema::json_schema(),
        ),
        (
            DocumentedSlot::json_schema(),
            documented_slot_schema::Schema::json_schema(),
        ),
    ];
    for (of_the_type, of_the_module) in documents {
        assert!(of_the_module.get("examples").is_some(), "{of_the_module}");
        assert_eq!(of_the_type, of_the_module);
    }
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn a_brand_writes_its_examples_in_the_document_of_its_schema_module_too() {
    assert_eq!(
        ExampleOnlyBrand::json_schema(),
        example_only_brand_schema::Schema::json_schema()
    );
    assert_eq!(
        example_only_brand_schema::Schema::json_schema()["examples"],
        serde_json::json!(["alpha"])
    );
}

#[cfg(feature = "serde")]
#[test]
fn an_alias_of_a_documented_type_with_no_block_carries_its_examples_in_json_schema() {
    assert_eq!(
        documented_struct_ref_schema::Schema::json_schema()["examples"],
        DocumentedStruct::json_schema()["examples"]
    );
}
