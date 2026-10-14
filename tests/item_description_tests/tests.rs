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

// One item of each kind, declared wherever a test below reads them: the regression test under
// `jsonschema`, and the pinned definitions under `serde`.

/// Who an invoice is billed to.
#[cfg(all(
    feature = "typescript",
    feature = "zod",
    any(feature = "jsonschema", feature = "serde")
))]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Customer {
    #[model_schema_prop(minLength = 1)]
    pub name: String,
    pub open_invoices: u32,
}

/// One billed item.
#[cfg(all(
    feature = "typescript",
    feature = "zod",
    any(feature = "jsonschema", feature = "serde")
))]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LineItem {
    pub code: String,
    pub quantity: u32,
}

/// An invoice.
#[cfg(all(
    feature = "typescript",
    feature = "zod",
    any(feature = "jsonschema", feature = "serde")
))]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Invoice {
    pub customer: Customer,
    pub items: Vec<LineItem>,
    pub number: String,
}

/// A page of entries.
#[cfg(all(
    feature = "typescript",
    feature = "zod",
    any(feature = "jsonschema", feature = "serde")
))]
#[model_schema(default_types(Entry = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoicePage<Entry> {
    pub entries: Vec<Entry>,
    pub total_count: u32,
}

/// Where an invoice stands.
#[cfg(all(
    feature = "typescript",
    feature = "zod",
    any(feature = "jsonschema", feature = "serde")
))]
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum InvoiceStatus {
    Draft,
    Paid,
}

/// What happened to an invoice.
#[cfg(all(
    feature = "typescript",
    feature = "zod",
    any(feature = "jsonschema", feature = "serde")
))]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum InvoiceEvent {
    Issued { number: String },
    Voided { reason: String },
}

/// What an invoice is paid with.
#[cfg(all(
    feature = "typescript",
    feature = "zod",
    any(feature = "jsonschema", feature = "serde")
))]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum InvoicePayment {
    Amount(u32),
    Note(String),
}

/// The number an invoice is filed under.
#[cfg(all(
    feature = "typescript",
    feature = "zod",
    any(feature = "jsonschema", feature = "serde")
))]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct InvoiceNumber(pub String);

/// How many lines an invoice holds.
#[cfg(all(
    feature = "typescript",
    feature = "zod",
    any(feature = "jsonschema", feature = "serde")
))]
#[model_schema()]
pub type LineCount = u32;

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
/// each other over.
#[cfg(feature = "typescript")]
fn jsdoc_body_lines(ts: &str) -> Vec<String> {
    let (block, _) = ts
        .strip_prefix("/**\n")
        .and_then(|rest| rest.split_once(" */"))
        .unwrap();
    block
        .lines()
        .map(|line| line.trim().to_owned())
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

#[cfg(all(feature = "typescript", feature = "jsonschema", feature = "zod"))]
#[test]
fn ts_definition_writes_the_description_and_no_json_schema() {
    let count: LineCount = 2;
    assert_eq!(count, 2);

    for (ts, opener) in [
        (
            Invoice::ts_definition(),
            "/**\n * An invoice.\n * \n */\nexport type Invoice = {",
        ),
        (
            InvoicePage::<String>::ts_definition(),
            "/**\n * A page of entries.\n * \n */\nexport type InvoicePage<Entry> = {",
        ),
        (
            InvoiceStatus::ts_definition(),
            "/**\n * Where an invoice stands.\n * \n */\nexport type InvoiceStatus =",
        ),
        (
            InvoiceEvent::ts_definition(),
            "/**\n * What happened to an invoice.\n * \n */\nexport type InvoiceEvent =",
        ),
        (
            InvoicePayment::ts_definition(),
            "/**\n * What an invoice is paid with.\n * \n */\nexport type InvoicePayment =",
        ),
        (
            line_count_schema::Schema::ts_definition(),
            "/**\n * How many lines an invoice holds.\n * \n */\nexport type LineCountType =",
        ),
    ] {
        assert!(!ts.contains("JSON Schema"), "got: {ts}");
        assert!(ts.starts_with(opener), "got: {ts}");
    }

    // Published as a brand under `serde`, it writes no `JSDoc` block to open with.
    let brand = InvoiceNumber::ts_definition();
    assert!(!brand.contains("JSON Schema"), "got: {brand}");
}

// One text per item, held in every build with `serde`, `typescript` and `zod`, whatever
// `jsonschema` is.
#[cfg(all(feature = "serde", feature = "typescript", feature = "zod"))]
#[test]
fn ts_definition_is_one_text_with_jsonschema_on_and_off() {
    let count: LineCount = 2;
    assert_eq!(count, 2);

    for (ts, expected) in [
        (
            Invoice::ts_definition(),
            "/**\n * An invoice.\n * \n */\nexport type Invoice = {\n  /**\n   * customer\n   * \n   */\n  customer: Customer;\n  /**\n   * items\n   * \n   */\n  items: Array<LineItem>;\n  /**\n   * number\n   * \n   */\n  number: string;\n};",
        ),
        (
            Customer::ts_definition(),
            "/**\n * Who an invoice is billed to.\n * \n */\nexport type Customer = {\n  /**\n   * name\n   * \n   * Minimum length: 1\n   */\n  name: string;\n  /**\n   * openInvoices\n   * \n   */\n  openInvoices: number;\n};",
        ),
        (
            LineItem::ts_definition(),
            "/**\n * One billed item.\n * \n */\nexport type LineItem = {\n  /**\n   * code\n   * \n   */\n  code: string;\n  /**\n   * quantity\n   * \n   */\n  quantity: number;\n};",
        ),
        (
            InvoicePage::<String>::ts_definition(),
            "/**\n * A page of entries.\n * \n */\nexport type InvoicePage<Entry> = {\n  /**\n   * entries\n   * \n   */\n  entries: Array<Entry>;\n  /**\n   * totalCount\n   * \n   */\n  totalCount: number;\n};",
        ),
        (
            InvoiceStatus::ts_definition(),
            "/**\n * Where an invoice stands.\n * \n */\nexport type InvoiceStatus =\n  | \"draft\"\n  | \"paid\";\n\n/** The Rust variant name `value` carries, or \"\" where it carries none. */\nexport function InvoiceStatus$Variant(value: unknown): string {\n  switch (value) {\n    case \"draft\": return \"Draft\";\n    case \"paid\": return \"Paid\";\n    default: return \"\";\n  }\n}",
        ),
        (
            InvoiceEvent::ts_definition(),
            "/**\n * What happened to an invoice.\n * \n */\nexport type InvoiceEvent = {\n  /**\n   * issued\n   * \n   */\n  kind: \"issued\";\n  /**\n   * number\n   * \n   */\n  number: string;\n} | {\n  /**\n   * voided\n   * \n   */\n  kind: \"voided\";\n  /**\n   * reason\n   * \n   */\n  reason: string;\n};\n\n/** The Rust variant name `value` carries, or \"\" where it carries none. */\nexport function InvoiceEvent$Variant(value: unknown): string {\n  if (typeof value !== \"object\" || value === null) return \"\";\n  switch ((value as { kind?: unknown }).kind) {\n    case \"issued\": return \"Issued\";\n    case \"voided\": return \"Voided\";\n    default: return \"\";\n  }\n}",
        ),
        (
            InvoicePayment::ts_definition(),
            "/**\n * What an invoice is paid with.\n * \n */\nexport type InvoicePayment = number | string;\n\n/** The Rust variant name `value` carries, or \"\" where it carries none. */\nexport function InvoicePayment$Variant(_value: unknown): string {\n  return \"\";\n}",
        ),
        (
            InvoiceNumber::ts_definition(),
            "export type InvoiceNumber = string & $brand<\"InvoiceNumber\">;",
        ),
        (
            line_count_schema::Schema::ts_definition(),
            "/**\n * How many lines an invoice holds.\n * \n */\nexport type LineCountType = number;\n\nexport type LineCount = LineCountType;",
        ),
    ] {
        assert_eq!(ts, expected);
    }
}

#[cfg(all(feature = "serde", feature = "typescript", feature = "zod"))]
#[test]
fn the_members_keep_the_jsdoc_they_had() {
    let ts = Invoice::ts_definition();
    let (_, members) = ts.split_once("export type Invoice = {\n").unwrap();
    assert_eq!(
        members,
        "  /**\n   * customer\n   * \n   */\n  customer: Customer;\n  /**\n   * items\n   * \n   */\n  items: Array<LineItem>;\n  /**\n   * number\n   * \n   */\n  number: string;\n};"
    );
}

#[cfg(all(
    feature = "jsonschema",
    feature = "serde",
    feature = "typescript",
    feature = "zod"
))]
#[test]
fn the_json_schema_is_the_text_it_was() {
    assert_eq!(
        Invoice::json_schema().to_string(),
        "{\"type\":\"object\",\"additionalProperties\":false,\"properties\":{\"customer\":{\"type\":\"object\",\"additionalProperties\":false,\"properties\":{\"name\":{\"type\":\"string\",\"minLength\":1},\"openInvoices\":{\"type\":\"integer\"}},\"required\":[\"name\",\"openInvoices\"]},\"items\":{\"type\":\"array\",\"items\":{\"type\":\"object\",\"additionalProperties\":false,\"properties\":{\"code\":{\"type\":\"string\"},\"quantity\":{\"type\":\"integer\"}},\"required\":[\"code\",\"quantity\"]}},\"number\":{\"type\":\"string\"}},\"required\":[\"customer\",\"items\",\"number\"]}"
    );
    assert_eq!(
        invoice_page_schema::Schema::json_schema_with(&[serde_json::json!({ "type": "integer" })])
            .to_string(),
        "{\"type\":\"object\",\"additionalProperties\":false,\"properties\":{\"entries\":{\"type\":\"array\",\"items\":{\"type\":\"integer\"}},\"totalCount\":{\"type\":\"integer\"}},\"required\":[\"entries\",\"totalCount\"]}"
    );
}

#[cfg(all(feature = "serde", feature = "typescript", feature = "zod"))]
#[test]
fn the_zod_schema_is_the_text_it_was() {
    assert_eq!(
        Invoice::zod_schema(),
        "const Invoice$RawSchema = z.strictObject({\n  customer: Customer$Schema,\n  items: z.array(LineItem$Schema),\n  number: z.string(),\n});\n\nexport const Invoice$Schema: ZodType<Invoice> = Invoice$RawSchema.meta({\n  examples: []\n});"
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
