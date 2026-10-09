//! Tests of `#[model_schema(name = "...")]`: every shape is published under the override.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(name = "RenamedItem")]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ItemUnderRustName {
    pub count: u32,
    pub label: String,
}

#[model_schema(name = "RenamedPair")]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PairUnderRustName(pub String, pub u32);

#[model_schema(name = "RenamedBrand")]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct BrandUnderRustName(pub String);

#[model_schema(name = "RenamedSlot")]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlotUnderRustName {
    Primary,
    Secondary,
}

#[model_schema(name = "RenamedShape")]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ShapeUnderRustName {
    Named { side: u8 },
    Unit,
}

/// A renamed type that names itself: the recursion is what forces the JSON surface to publish a
/// definition and point at it, which is the only place the definition's name is written out.
#[model_schema(name = "RenamedTree")]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TreeUnderRustName {
    pub children: Vec<Self>,
    pub label: String,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReferencesRenamedItems {
    pub brand: BrandUnderRustName,
    pub item: ItemUnderRustName,
    pub pair: PairUnderRustName,
    pub shape: ShapeUnderRustName,
    pub slot: SlotUnderRustName,
    pub tree: TreeUnderRustName,
}

/// Declaration order, taken both ways round: this struct names a renamed struct and enum that
/// have not expanded yet, so the module each reference resolves to is derived from the item's
/// Rust ident alone; [`NamesRenamedItemsDeclaredEarlier`] names the same two once registered.
/// Both have to reach the same modules.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NamesRenamedItemsDeclaredLater {
    pub gauge: LaterRenamedGauge,
    pub grades: Vec<LaterRenamedGrade>,
}

/// A rename moves what the item is *exported* as. It does not move the module its schema is
/// published in — an override is not recoverable from the Rust ident, so a module named after the
/// override would be one no forward reference could ever name.
#[model_schema(name = "RenamedLaterGauge")]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LaterRenamedGauge {
    pub reading: u32,
}

#[model_schema(name = "RenamedLaterGrade")]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LaterRenamedGrade {
    High,
    Low,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NamesRenamedItemsDeclaredEarlier {
    pub gauge: LaterRenamedGauge,
    pub grades: Vec<LaterRenamedGrade>,
}

#[test]
fn every_renamed_shape_expands_in_this_feature_combination() {
    let value = ReferencesRenamedItems {
        brand: BrandUnderRustName("three".to_owned()),
        item: ItemUnderRustName {
            count: 1,
            label: "one".to_owned(),
        },
        pair: PairUnderRustName("two".to_owned(), 2),
        shape: ShapeUnderRustName::Named { side: 4 },
        slot: SlotUnderRustName::Primary,
        tree: TreeUnderRustName {
            children: vec![],
            label: "five".to_owned(),
        },
    };
    assert_eq!(value.item.count, 1);
    assert_eq!(value.pair.1, 2);
    assert_eq!(value.brand.0, "three");
    assert_eq!(value.slot, SlotUnderRustName::Primary);
    assert!(value.tree.children.is_empty());
}

#[cfg(feature = "typescript")]
#[test]
fn every_renamed_shape_is_written_under_the_override() {
    for (ts, declared) in [
        (
            ItemUnderRustName::ts_definition(),
            "export type RenamedItem",
        ),
        (
            PairUnderRustName::ts_definition(),
            "export type RenamedPair",
        ),
        (
            BrandUnderRustName::ts_definition(),
            "export type RenamedBrand",
        ),
        (
            SlotUnderRustName::ts_definition(),
            "export type RenamedSlot",
        ),
        (
            ShapeUnderRustName::ts_definition(),
            "export type RenamedShape",
        ),
    ] {
        assert!(ts.contains(declared), "{declared} missing from: {ts}");
    }
}

#[cfg(all(feature = "typescript", feature = "zod"))]
#[test]
fn a_renamed_brand_is_tagged_by_the_override() {
    let ts = BrandUnderRustName::ts_definition();
    assert!(ts.contains("$brand<\"RenamedBrand\">"), "got: {ts}");
    let zod = BrandUnderRustName::zod_schema();
    assert!(zod.contains(".brand<\"RenamedBrand\">()"), "got: {zod}");
}

#[cfg(feature = "zod")]
#[test]
fn every_renamed_shape_publishes_its_zod_schema_under_the_override() {
    for (zod, declared) in [
        (
            ItemUnderRustName::zod_schema(),
            "export const RenamedItem$Schema",
        ),
        (
            PairUnderRustName::zod_schema(),
            "export const RenamedPair$Schema",
        ),
        (
            BrandUnderRustName::zod_schema(),
            "export const RenamedBrand$Schema",
        ),
        (
            SlotUnderRustName::zod_schema(),
            "export const RenamedSlot$Schema",
        ),
        (
            ShapeUnderRustName::zod_schema(),
            "export const RenamedShape$Schema",
        ),
    ] {
        assert!(zod.contains(declared), "{declared} missing from: {zod}");
    }
}

#[cfg(feature = "typescript")]
#[test]
fn a_reference_to_a_renamed_item_resolves_the_override() {
    let ts = ReferencesRenamedItems::ts_definition();
    for referenced in [
        "item: RenamedItem;",
        "pair: RenamedPair;",
        "brand: RenamedBrand;",
        "slot: RenamedSlot;",
        "shape: RenamedShape;",
        "tree: RenamedTree;",
    ] {
        assert!(ts.contains(referenced), "{referenced} missing from: {ts}");
    }
}

#[cfg(feature = "zod")]
#[test]
fn a_zod_reference_to_a_renamed_item_resolves_the_override() {
    let zod = ReferencesRenamedItems::zod_schema();
    for referenced in [
        "item: RenamedItem$Schema",
        "pair: RenamedPair$Schema",
        "brand: RenamedBrand$Schema",
        "slot: RenamedSlot$Schema",
        "shape: RenamedShape$Schema",
        "tree: RenamedTree$Schema",
    ] {
        assert!(zod.contains(referenced), "{referenced} missing from: {zod}");
    }
}

#[cfg(any(feature = "typescript", feature = "zod"))]
#[test]
fn a_reference_to_a_renamed_item_never_carries_its_rust_name() {
    #[cfg(feature = "typescript")]
    let ts = ReferencesRenamedItems::ts_definition();
    #[cfg(not(feature = "typescript"))]
    let ts = String::new();
    #[cfg(feature = "zod")]
    let zod = ReferencesRenamedItems::zod_schema();
    #[cfg(not(feature = "zod"))]
    let zod = String::new();
    for surface in [&ts, &zod] {
        assert!(
            !surface.contains("UnderRustName"),
            "rust name reached: {surface}"
        );
    }
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_renamed_item_names_its_json_definition_by_the_override() {
    let schema = TreeUnderRustName::json_schema();
    let pointer = serde_json::json!({ "$ref": "#/$defs/RenamedTree" });
    assert_eq!(
        schema.get("$ref").unwrap(),
        &pointer["$ref"],
        "got: {schema}"
    );
    let definition = schema.get("$defs").unwrap().get("RenamedTree").unwrap();
    assert_eq!(
        definition
            .get("properties")
            .unwrap()
            .get("children")
            .unwrap()
            .get("items")
            .unwrap(),
        &pointer,
        "got: {schema}"
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn the_module_a_json_reference_resolves_to_is_the_one_named_for_the_rust_ident() {
    let referencing = ReferencesRenamedItems::json_schema();
    let properties = referencing.get("properties").unwrap();
    assert_eq!(
        properties.get("item").unwrap(),
        &item_under_rust_name_schema::Schema::json_schema()
    );
    assert_eq!(
        properties.get("slot").unwrap(),
        &slot_under_rust_name_schema::Schema::json_schema()
    );
    assert_eq!(
        properties.get("brand").unwrap(),
        &brand_under_rust_name_schema::Schema::json_schema()
    );
}

#[test]
fn renamed_items_declared_under_their_reference_expand_in_this_feature_combination() {
    let later = NamesRenamedItemsDeclaredLater {
        gauge: LaterRenamedGauge { reading: 7 },
        grades: vec![LaterRenamedGrade::High],
    };
    assert_eq!(later.gauge.reading, 7);
    assert_eq!(later.grades, vec![LaterRenamedGrade::High]);

    let earlier = NamesRenamedItemsDeclaredEarlier {
        gauge: LaterRenamedGauge { reading: 8 },
        grades: vec![LaterRenamedGrade::Low],
    };
    assert_eq!(earlier.gauge.reading, 8);
    assert_eq!(earlier.grades, vec![LaterRenamedGrade::Low]);
}

#[cfg(feature = "jsonschema")]
#[test]
fn an_item_reference_describes_the_same_on_either_side_of_the_item() {
    assert_eq!(
        NamesRenamedItemsDeclaredLater::json_schema(),
        NamesRenamedItemsDeclaredEarlier::json_schema()
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn a_forward_item_reference_carries_what_the_item_publishes() {
    let schema = NamesRenamedItemsDeclaredLater::json_schema();
    let properties = schema.get("properties").unwrap();
    assert_eq!(
        properties.get("gauge").unwrap(),
        &later_renamed_gauge_schema::Schema::json_schema(),
        "in: {schema}"
    );
    assert_eq!(
        properties.get("grades").unwrap().get("items").unwrap(),
        &later_renamed_grade_schema::Schema::json_schema(),
        "in: {schema}"
    );
}

#[cfg(feature = "typescript")]
#[test]
fn a_forward_referenced_renamed_item_exports_under_the_override() {
    for (ts, declared) in [
        (
            LaterRenamedGauge::ts_definition(),
            "export type RenamedLaterGauge",
        ),
        (
            LaterRenamedGrade::ts_definition(),
            "export type RenamedLaterGrade",
        ),
    ] {
        assert!(ts.contains(declared), "{declared} missing from: {ts}");
    }
}

#[cfg(feature = "typescript")]
#[test]
fn every_name_a_forward_item_reference_writes_is_defined_by_the_emission() {
    let forward = NamesRenamedItemsDeclaredLater::ts_definition();
    let emission = [
        forward.clone(),
        NamesRenamedItemsDeclaredEarlier::ts_definition(),
        LaterRenamedGauge::ts_definition(),
        LaterRenamedGrade::ts_definition(),
    ]
    .join("\n\n");
    for (written, referenced) in [
        ("gauge: LaterRenamedGauge;", "LaterRenamedGauge"),
        ("grades: Array<LaterRenamedGrade>;", "LaterRenamedGrade"),
    ] {
        assert!(
            forward.contains(written),
            "{written} missing from: {forward}"
        );
        assert!(
            emission.contains(&format!("export type {referenced} ")),
            "{referenced} is referenced but never defined in: {emission}"
        );
    }
}

#[cfg(feature = "zod")]
#[test]
fn every_schema_a_forward_item_reference_names_is_defined_by_the_emission() {
    let forward = NamesRenamedItemsDeclaredLater::zod_schema();
    let emission = [
        forward.clone(),
        NamesRenamedItemsDeclaredEarlier::zod_schema(),
        LaterRenamedGauge::zod_schema(),
        LaterRenamedGrade::zod_schema(),
    ]
    .join("\n\n");
    for (written, referenced) in [
        (
            "get gauge() { return LaterRenamedGauge$Schema; },",
            "LaterRenamedGauge",
        ),
        (
            "get grades() { return z.array(LaterRenamedGrade$Schema); },",
            "LaterRenamedGrade",
        ),
    ] {
        assert!(
            forward.contains(written),
            "{written} missing from: {forward}"
        );
        assert!(
            emission.contains(&format!("export const {referenced}$Schema")),
            "{referenced} is referenced but never defined in: {emission}"
        );
    }
}

#[cfg(any(feature = "typescript", feature = "zod"))]
#[test]
fn an_item_exported_under_its_ident_publishes_no_reexport() {
    #[cfg(feature = "typescript")]
    for ts in [
        ReferencesRenamedItems::ts_definition(),
        NamesRenamedItemsDeclaredLater::ts_definition(),
    ] {
        assert_eq!(ts.matches("export type ").count(), 1, "got: {ts}");
    }
    #[cfg(feature = "zod")]
    for zod in [
        ReferencesRenamedItems::zod_schema(),
        NamesRenamedItemsDeclaredLater::zod_schema(),
    ] {
        assert_eq!(zod.matches("export const ").count(), 1, "got: {zod}");
    }
}

#[cfg(feature = "zod")]
#[test]
fn a_forward_referenced_renamed_item_publishes_its_zod_schema_under_the_override() {
    for (zod, declared) in [
        (
            LaterRenamedGauge::zod_schema(),
            "export const RenamedLaterGauge$Schema",
        ),
        (
            LaterRenamedGrade::zod_schema(),
            "export const RenamedLaterGrade$Schema",
        ),
    ] {
        assert!(zod.contains(declared), "{declared} missing from: {zod}");
    }
}
