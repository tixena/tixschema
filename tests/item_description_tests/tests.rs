//! Tests of the `JSDoc` an item with no doc comment is published under.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[cfg(feature = "typescript")]
#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
pub struct PlainStruct {
    pub label: String,
}

#[cfg(feature = "typescript")]
#[model_schema(name = "RenamedStruct")]
#[derive(Serialize, Deserialize, Debug)]
pub struct StructUnderRustName {
    pub label: String,
}

/// A documented struct.
#[cfg(feature = "typescript")]
#[model_schema(name = "RenamedDocStruct")]
#[derive(Serialize, Deserialize, Debug)]
pub struct DocStructUnderRustName {
    pub label: String,
}

#[cfg(feature = "typescript")]
#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
pub struct PlainTuple(pub String, pub u32);

#[cfg(feature = "typescript")]
#[model_schema(name = "RenamedTuple")]
#[derive(Serialize, Deserialize, Debug)]
pub struct TupleUnderRustName(pub String, pub u32);

#[cfg(any(feature = "typescript", feature = "zod"))]
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum PlainSlot {
    Primary,
    Secondary,
}

#[cfg(any(feature = "typescript", feature = "zod"))]
#[model_schema(name = "RenamedSlot")]
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum SlotUnderRustName {
    Primary,
    Secondary,
}

#[cfg(feature = "typescript")]
#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
pub enum PlainShape {
    Named { side: u8 },
    Unit,
}

#[cfg(feature = "typescript")]
#[model_schema(name = "RenamedShape")]
#[derive(Serialize, Deserialize, Debug)]
pub enum ShapeUnderRustName {
    Named { side: u8 },
    Unit,
}

#[cfg(feature = "typescript")]
#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "kind", content = "payload")]
pub enum PlainAdjacent {
    Named { side: u8 },
    Unit,
}

#[cfg(feature = "typescript")]
#[model_schema(name = "RenamedAdjacent")]
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "kind", content = "payload")]
pub enum AdjacentUnderRustName {
    Named { side: u8 },
    Unit,
}

#[cfg(feature = "typescript")]
#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum PlainEither {
    Count(u32),
    Label(String),
}

#[cfg(feature = "typescript")]
#[model_schema(name = "RenamedEither")]
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum EitherUnderRustName {
    Count(u32),
    Label(String),
}

#[cfg(feature = "zod")]
#[model_schema()]
#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct PlainBrand(pub String);

#[cfg(feature = "zod")]
#[model_schema(name = "RenamedBrand")]
#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct BrandUnderRustName(pub String);

// The one shape with no surface name of its own: an alias is exported under the `Type` suffix, and
// falls back to that exported name the way every declared item falls back to its own.
#[cfg(feature = "typescript")]
#[model_schema()]
pub type PlainAlias = u32;

#[cfg(feature = "typescript")]
fn assert_jsdoc_opens_with(ts: &str, expected: &str) {
    let header = format!("/**\n * {expected}\n");
    assert!(ts.starts_with(&header), "expected {expected} to open: {ts}");
}

/// The surface with the item's ident re-export taken off. That line is the one place an item's
/// Rust ident is written on purpose — a reference standing before the item has only the ident to
/// spell. Everything else is written under the name it's exported as, which the assertions below
/// are about.
#[cfg(any(feature = "typescript", feature = "zod"))]
fn without_ident_reexport(surface: &str, ident: &str, exported: &str) -> String {
    surface
        .replace(&format!("\n\nexport type {ident} = {exported};"), "")
        .replace(
            &format!("\n\nexport const {ident}$Schema = {exported}$Schema;"),
            "",
        )
}

/// The ` * ` lines a definition's `JSDoc` block is written from, delimiters and surrounding
/// indentation set aside — the one part of the emitted TypeScript the shapes may be held against
/// each other over. The rendered JSON schema under `jsonschema` is appended after that body
/// rather than written from it.
#[cfg(feature = "typescript")]
fn jsdoc_body_lines(ts: &str) -> Vec<String> {
    let (block, _) = ts
        .strip_prefix("/**\n")
        .and_then(|rest| rest.split_once(" */"))
        .unwrap();
    block
        .lines()
        .map(|line| line.trim().to_owned())
        .take_while(|line| line != "* JSON Schema:")
        .filter(|line| line.starts_with('*'))
        .collect()
}

#[cfg(feature = "typescript")]
#[test]
fn an_undocumented_item_names_itself_in_jsdoc_as_it_is_exported() {
    for (ts, exported) in [
        (StructUnderRustName::ts_definition(), "RenamedStruct"),
        (TupleUnderRustName::ts_definition(), "RenamedTuple"),
        (SlotUnderRustName::ts_definition(), "RenamedSlot"),
        (ShapeUnderRustName::ts_definition(), "RenamedShape"),
        (AdjacentUnderRustName::ts_definition(), "RenamedAdjacent"),
        (EitherUnderRustName::ts_definition(), "RenamedEither"),
    ] {
        assert_jsdoc_opens_with(&ts, exported);
        assert!(
            ts.contains(&format!("export type {exported} ")),
            "{exported} not exported by: {ts}"
        );
    }
}

#[cfg(feature = "typescript")]
#[test]
fn an_undocumented_item_never_writes_its_rust_ident() {
    for (ts, ident, exported) in [
        (
            StructUnderRustName::ts_definition(),
            "StructUnderRustName",
            "RenamedStruct",
        ),
        (
            TupleUnderRustName::ts_definition(),
            "TupleUnderRustName",
            "RenamedTuple",
        ),
        (
            SlotUnderRustName::ts_definition(),
            "SlotUnderRustName",
            "RenamedSlot",
        ),
        (
            ShapeUnderRustName::ts_definition(),
            "ShapeUnderRustName",
            "RenamedShape",
        ),
        (
            AdjacentUnderRustName::ts_definition(),
            "AdjacentUnderRustName",
            "RenamedAdjacent",
        ),
        (
            EitherUnderRustName::ts_definition(),
            "EitherUnderRustName",
            "RenamedEither",
        ),
    ] {
        let described = without_ident_reexport(&ts, ident, exported);
        assert!(
            !described.contains("UnderRustName"),
            "rust ident reached: {described}"
        );
    }
}

#[cfg(feature = "typescript")]
#[test]
fn an_item_exported_under_its_rust_ident_keeps_the_header_it_had() {
    for (ts, declared) in [
        (PlainStruct::ts_definition(), "PlainStruct"),
        (PlainTuple::ts_definition(), "PlainTuple"),
        (PlainSlot::ts_definition(), "PlainSlot"),
        (PlainShape::ts_definition(), "PlainShape"),
        (PlainAdjacent::ts_definition(), "PlainAdjacent"),
        (PlainEither::ts_definition(), "PlainEither"),
    ] {
        assert_jsdoc_opens_with(&ts, declared);
    }
}

#[cfg(feature = "typescript")]
#[test]
fn every_undocumented_shape_writes_the_same_two_jsdoc_lines() {
    // The alias publishes `ts_definition` from its own module rather than from the alias, so naming
    // the type here is what keeps the fixture from being pruned as unused.
    let aliased: PlainAlias = 3;
    assert_eq!(aliased, 3);

    for (ts, exported) in [
        (PlainStruct::ts_definition(), "PlainStruct"),
        (PlainTuple::ts_definition(), "PlainTuple"),
        (PlainSlot::ts_definition(), "PlainSlot"),
        (PlainShape::ts_definition(), "PlainShape"),
        (PlainAdjacent::ts_definition(), "PlainAdjacent"),
        (PlainEither::ts_definition(), "PlainEither"),
        (
            plain_alias_schema::Schema::ts_definition(),
            "PlainAliasType",
        ),
    ] {
        assert_eq!(
            jsdoc_body_lines(&ts),
            vec![format!("* {exported}"), "*".to_owned()],
            "for {exported}: {ts}"
        );
    }
}

#[cfg(feature = "typescript")]
#[test]
fn a_documented_item_keeps_its_docs_over_either_name() {
    let ts = DocStructUnderRustName::ts_definition();
    assert_jsdoc_opens_with(&ts, "A documented struct.");
    assert!(
        ts.contains("export type RenamedDocStruct "),
        "override missing from: {ts}"
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_plain_enum_describes_itself_as_it_is_exported() {
    for (zod, exported) in [
        (SlotUnderRustName::zod_schema(), "RenamedSlot"),
        (PlainSlot::zod_schema(), "PlainSlot"),
    ] {
        assert!(
            zod.contains(&format!("description: \"{exported}\"")),
            "{exported} not described by: {zod}"
        );
    }
    let slot = SlotUnderRustName::zod_schema();
    assert!(
        !without_ident_reexport(&slot, "SlotUnderRustName", "RenamedSlot")
            .contains("UnderRustName"),
        "rust ident reached the description: {slot}"
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_brand_describes_itself_as_it_is_exported() {
    for (zod, exported) in [
        (BrandUnderRustName::zod_schema(), "RenamedBrand"),
        (PlainBrand::zod_schema(), "PlainBrand"),
    ] {
        assert!(
            zod.contains(&format!("description: \"{exported}\"")),
            "{exported} not described by: {zod}"
        );
    }
}
