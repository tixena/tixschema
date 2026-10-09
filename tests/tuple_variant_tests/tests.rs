//! Tests of tuple variants of enums on every surface.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// A generated schema module reaches its siblings through the enclosing module, and a function body
/// is not one, so a type another type references is declared here rather than inside a test.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Inner {
    pub field: String,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Outer {
    Paired(Inner, i64),
    Wrapped(Inner),
}

/// The content key of a single-element tuple variant is a slot: serde always writes the key, so a
/// `None` at the outermost level reaches the wire as `null` rather than dropping the key. The
/// three variants below cover the `Option` around the array, the `Option` among its items, and no
/// `Option` at all.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "value")]
pub enum SlotContent {
    Covered(Option<Vec<u32>>),
    Element(Vec<Option<u32>>),
    Plain(String),
}

/// Every position of a multi-element tuple variant is a slot for the same reason: serde writes each
/// one, so a `None` among them reaches the wire as a `null` in place rather than shortening the
/// tuple. The non-`Option` element beside it carries no null.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "value")]
pub enum SlotElements {
    Pair(String, Option<u32>),
}

/// A struct variant of an adjacently tagged enum: serde nests its fields in an object under the
/// content key rather than splicing them beside the tag. `Named`'s optional `y` covers key
/// optionality at the nested depth. The empty-content case (`Empty {}`) trips
/// `clippy::empty_enum_variants_with_brackets` here, so it's covered instead by
/// `adjacently_tagged_empty_named_variant_nests_an_empty_content_object` in
/// `src/model_schema/tests.rs`, built through `parse_quote!`.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "kind", content = "data")]
pub enum AdjacentNamed {
    Named {
        x: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        y: Option<u32>,
    },
    Unit,
}

/// An externally tagged enum (no `#[serde(tag = ..., content = ...)]`): serde writes the variant
/// name as the sole key of an object holding the content, or the bare name for a unit variant. The
/// four variants below cover what that key can hold, plus the keyless case.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum External {
    Bare,
    Fields { a: String, b: bool },
    Pair(u32, u32),
    Single(String),
}

/// The same variants written with the tagging attributes, which keeps the adjacent form beside the
/// external one so the two can be read against each other.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type", content = "value")]
pub enum Adjacent {
    Bare,
    Fields { a: String, b: bool },
    Pair(u32, u32),
    Single(String),
}

/// A variant's key is its wire name, and a rename can spell that as something no JavaScript
/// identifier can hold.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RenamedExternal {
    BigThing(u32),
    #[serde(rename = "application/pdf")]
    Mime(String),
    UnitThing,
}

/// The type an internally tagged newtype variant wraps. Its fields are what serde writes beside
/// the tag, so they are the ones the surfaces owe.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TagPayload {
    pub a: String,
    pub b: bool,
}

/// An internally tagged enum (`#[serde(tag = ...)]`, no `content`) writes a variant's data as
/// members of the object the tag is written in. The three variants cover what can be written
/// there: nothing, a struct variant's own fields, or a newtype variant's inner members.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum Internal {
    Bare,
    Fields { a: String, b: bool },
    Wrapped(TagPayload),
}

/// The same form with no newtype variant, which is every member a `z.discriminatedUnion` can hold.
/// Only the Zod surface names that union, so the fixture is declared where it is read.
#[cfg(feature = "zod")]
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum InternalNamedOnly {
    Fields { a: String },
}

/// The shape the crate refuses to describe, declared without `#[model_schema()]` so serde's own
/// answer for it can be read off the wire.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum InternalScalar {
    Single(String),
}

/// A plain enum, and a bare tag over it. Also declared without `#[model_schema()]`: a name says
/// nothing about what serde writes for it, and this one writes no object, so the crate refuses the
/// declaration — leaving the wire form readable only from a plain serde type.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum InternalHue {
    Red,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum InternalOverEnum {
    EnumInner(InternalHue),
}

/// A newtype over a `String`: serde writes it as the string it wraps, and the registry cannot tell
/// it apart from a struct — both register as having no enum members. So the declaration compiles
/// and the divergence is left for the merge to catch.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct InternalSlug(pub String);

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum InternalOverBrand {
    Branded(InternalSlug),
}

/// An untagged enum every member of which serde writes as an object, wrapped by a bare tag. The
/// content joins the tag the same way a `#[serde(flatten)]` base does, so what lands beside the tag
/// is the members of whichever union member matched.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct InternalFirst {
    pub a: String,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct InternalSecond {
    pub b: bool,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum InternalEither {
    First(InternalFirst),
    Second(InternalSecond),
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum InternalOverUntagged {
    Wrapped(InternalEither),
}

/// The same wrapping over a union with one member serde writes as a string. Only the JSON-schema
/// merge reads the pair, so both fixtures are declared where they are read.
#[cfg(feature = "jsonschema")]
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum InternalScalarEither {
    Obj(InternalFirst),
    Text(String),
}

#[cfg(feature = "jsonschema")]
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum InternalOverScalarUntagged {
    Wrapped(InternalScalarEither),
}

/// A recursive enum with no tagging attributes: what sits under a key is the enum itself. Only the
/// Zod surface spells the deferral, so the fixture is declared where it is read.
#[cfg(feature = "zod")]
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RecursiveExternal {
    Arr(Vec<Self>),
    Txt(String),
}

/// The positions a dropped slot can sit in, beside the variant that drops none: a leading drop is
/// the one the slots behind move up into, a trailing one only shortens the array, a middle one is
/// where the renumbering shows, and a lone slot dropped leaves the variant a unit.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ExternalDroppedSlots {
    Every(#[serde(skip)] String, #[serde(skip)] u32),
    Kept(u8, bool),
    Lead(#[serde(skip)] String, u32),
    Lone(#[serde(skip)] String),
    Middle(String, #[serde(skip)] Option<String>, u32),
    Trailing(String, #[serde(skip)] u32),
}

/// The kept variant on its own, which is what the enum above must still render it as.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ExternalKeptOnly {
    Kept(u8, bool),
}

/// The same drops under a content key, where the array is written under `value` rather than the
/// variant's own name. `Lone` is declared as the unit it would have been written as — under this
/// tagging alone the collapse has no payload to describe and is refused; this is the remedy that
/// refusal names.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type", content = "value")]
pub enum AdjacentDroppedSlots {
    Every(#[serde(skip)] String, #[serde(skip)] u32),
    Kept(u8, bool),
    Lead(#[serde(skip)] String, u32),
    Lone,
}

/// The kept variant on its own, for the same reading in the adjacent form.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type", content = "value")]
pub enum AdjacentKeptOnly {
    Kept(u8, bool),
}

/// The adjacent lone-slot collapse, declared without a schema because the surfaces refuse it — the
/// wire captured below is what they refuse it for.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type", content = "value")]
pub enum AdjacentLoneSlotWire {
    Lone(#[serde(skip)] String),
}

/// The same variant declared as the unit it is written as, which is the remedy the refusal names.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type", content = "value")]
pub enum AdjacentUnitWire {
    Lone,
}

/// A value serde writes as an object, which is the only content a bare tag can carry beside it.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct DroppedSlotInner {
    pub a: u32,
}

/// The bare-tag form, where the only tuple variant serde accepts is the newtype one — and where a
/// lone slot taken off the wire leaves nothing to sit beside the tag, whatever the slot held.
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum InternalDroppedSlot {
    Kept(DroppedSlotInner),
    Lone(#[serde(skip)] String),
}

/// The `oneOf` member an externally tagged variant renders as: the one whose sole required key is
/// the variant's name.
#[cfg(feature = "jsonschema")]
fn external_member(schema: &serde_json::Value, variant: &str) -> serde_json::Value {
    let member = schema["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["required"] == serde_json::json!([variant]))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    assert!(
        !member.is_null(),
        "No member keyed `{variant}`. Got: {schema}"
    );
    member
}

/// The JSON type name a value carries, as a schema spells it.
#[cfg(feature = "jsonschema")]
const fn json_type_name(value: &serde_json::Value) -> &'static str {
    match *value {
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Null => "null",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::Object(_) => "object",
        serde_json::Value::String(_) => "string",
    }
}

/// Single-element tuple variants.
#[test]
fn test_single_tuple_variant_typescript() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum SingleTuple {
        Decimal(f64),
        Flag(bool),
        Number(i64),
        Text(String),
    }

    let ts = SingleTuple::ts_definition();

    // The variant name is the key and its content sits under it, unwrapped.
    assert!(ts.contains("\"Text\": string"), "Missing Text. Got: {ts}");
    assert!(
        ts.contains("\"Number\": number"),
        "Missing Number. Got: {ts}"
    );
    assert!(ts.contains("\"Flag\": boolean"), "Missing Flag. Got: {ts}");
    assert!(
        ts.contains("\"Decimal\": number"),
        "Missing Decimal. Got: {ts}"
    );
}

/// Single-element tuple variants Zod schema.
#[cfg(feature = "zod")]
#[test]
fn test_single_tuple_variant_zod() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum SingleTupleZod {
        Flag(bool),
        Number(i64),
        Text(String),
    }

    let zod = SingleTupleZod::zod_schema();

    // The key carries the discriminator, so there is no one field every member shares for
    // `z.discriminatedUnion` to switch on.
    assert!(zod.contains("z.union(["), "Missing union. Got: {zod}");
    assert!(
        !zod.contains("z.discriminatedUnion"),
        "No shared field to discriminate on. Got: {zod}"
    );

    assert!(
        zod.contains("\"Text\": z.string()"),
        "Missing Text. Got: {zod}"
    );
    assert!(
        zod.contains("\"Number\": z.number().int()"),
        "Missing Number. Got: {zod}"
    );
    assert!(
        zod.contains("\"Flag\": z.boolean()"),
        "Missing Flag. Got: {zod}"
    );
}

/// Multi-element tuple variants.
#[test]
fn test_multi_tuple_variant_typescript() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum MultiTuple {
        Pair(String, i64),
        Quad(String, i64, bool, f64),
        Triple(String, i64, bool),
    }

    let ts = MultiTuple::ts_definition();

    // The elements are the array under the variant's key.
    assert!(
        ts.contains("\"Pair\": [string, number]"),
        "Missing Pair tuple type. Got: {ts}"
    );
    assert!(
        ts.contains("\"Triple\": [string, number, boolean]"),
        "Missing Triple tuple type. Got: {ts}"
    );
    assert!(
        ts.contains("\"Quad\": [string, number, boolean, number]"),
        "Missing Quad tuple type. Got: {ts}"
    );
}

/// Multi-element tuple variants Zod schema.
#[cfg(feature = "zod")]
#[test]
fn test_multi_tuple_variant_zod() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum MultiTupleZod {
        Pair(String, i64),
        Triple(String, i64, bool),
    }

    let zod = MultiTupleZod::zod_schema();

    assert!(zod.contains("z.tuple("), "Missing z.tuple");
    assert!(
        zod.contains("z.tuple([z.string(), z.number().int()])"),
        "Missing Pair tuple"
    );
    assert!(
        zod.contains("z.tuple([z.string(), z.number().int(), z.boolean()])"),
        "Missing Triple tuple"
    );
}

/// Plain enum (all unit variants) -> string union.
#[test]
fn test_plain_enum_string_union() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum DataType {
        Alphanumeric,
        Boolean,
        Decimal,
        Image,
        Integer,
    }

    let ts = DataType::ts_definition();

    assert!(ts.contains("\"Alphanumeric\""), "Missing Alphanumeric");
    assert!(ts.contains("\"Image\""), "Missing Image");
    assert!(ts.contains("\"Decimal\""), "Missing Decimal");
    assert!(ts.contains("\"Integer\""), "Missing Integer");
    assert!(ts.contains("\"Boolean\""), "Missing Boolean");

    assert!(
        !ts.contains("type:") || ts.contains("export type"),
        "Should not have type discriminator field"
    );
}

/// Plain enum Zod schema uses z.enum.
#[cfg(feature = "zod")]
#[test]
fn test_plain_enum_zod() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum PlainEnumZod {
        Active,
        Inactive,
        Pending,
    }

    let zod = PlainEnumZod::zod_schema();

    assert!(zod.contains("z.enum("), "Should use z.enum for plain enums");
    assert!(
        !zod.contains("z.discriminatedUnion"),
        "Should not use discriminatedUnion for plain enums"
    );
}

/// Mixed variants (comprehensive).
#[test]
fn test_mixed_variants_typescript() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum Mixed {
        Empty,
        Named { field_a: String, field_b: bool },
        Pair(String, i64),
        Text(String),
    }

    let ts = Mixed::ts_definition();
    // The `$Variant` reader appended after the type intentionally spells `case "Empty":`, so the
    // no-key check below is scoped to the type declaration alone.
    let type_declaration = ts.split("\n\n").next().unwrap();

    // Unit variant is the bare name: serde writes no key for it.
    assert!(ts.contains("\"Empty\""), "Missing Empty. Got: {ts}");
    assert!(
        !type_declaration.contains("\"Empty\":"),
        "A unit variant carries no key. Got: {ts}"
    );

    assert!(ts.contains("\"Text\": string"), "Missing Text. Got: {ts}");

    assert!(
        ts.contains("\"Pair\": [string, number]"),
        "Missing Pair tuple. Got: {ts}"
    );

    // A struct variant's fields sit in an object under its key.
    assert!(ts.contains("\"Named\": {"), "Missing Named. Got: {ts}");
    assert!(ts.contains("field_a: string"), "Missing field_a. Got: {ts}");
    assert!(
        ts.contains("field_b: boolean"),
        "Missing field_b. Got: {ts}"
    );
}

/// Mixed variants Zod schema.
#[cfg(feature = "zod")]
#[test]
fn test_mixed_variants_zod() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum MixedZod {
        Empty,
        Named { field_a: String, field_b: bool },
        Pair(String, i64),
        Text(String),
    }

    let zod = MixedZod::zod_schema();

    assert!(zod.contains("z.union(["), "Missing union. Got: {zod}");

    // A unit variant is the bare name, so it is a literal rather than an object.
    assert!(
        zod.contains("z.literal(\"Empty\")"),
        "Missing Empty literal. Got: {zod}"
    );

    assert!(
        zod.contains("\"Text\": z.string()"),
        "Missing Text. Got: {zod}"
    );

    assert!(
        zod.contains("\"Pair\": z.tuple("),
        "Missing Pair tuple. Got: {zod}"
    );

    assert!(
        zod.contains("\"Named\": z.strictObject("),
        "Missing Named object. Got: {zod}"
    );
    assert!(
        zod.contains("field_a: z.string()"),
        "Missing field_a. Got: {zod}"
    );
    assert!(
        zod.contains("field_b: z.boolean()"),
        "Missing field_b. Got: {zod}"
    );
}

/// JSON Schema generation for tuple variants.
#[cfg(feature = "jsonschema")]
#[test]
fn test_tuple_json_schema() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum TupleSchema {
        Double(String, i64),
        Single(String),
    }

    let schema = TupleSchema::json_schema();
    let schema_str = serde_json::to_string_pretty(&schema).unwrap();

    assert!(schema_str.contains("\"oneOf\""), "Missing oneOf");

    // The variant name is the property; there is no content key beside a tag.
    assert!(
        !schema_str.contains("\"value\""),
        "Externally tagged members carry no content key. Got: {schema_str}"
    );
    assert_eq!(
        external_member(&schema, "Double")["properties"]["Double"]["prefixItems"]
            .as_array()
            .unwrap()
            .len(),
        2,
        "Multi-tuple should use prefixItems. Got: {schema_str}"
    );
    assert_eq!(
        external_member(&schema, "Single")["properties"]["Single"],
        serde_json::json!({ "type": "string" })
    );
}

/// Custom content field name via serde.
#[test]
fn test_custom_content_field() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(tag = "kind", content = "data")]
    pub enum CustomContent {
        Number(i64),
        Text(String),
    }

    let ts = CustomContent::ts_definition();

    assert!(ts.contains("kind: \"Text\""), "Should use 'kind' as tag");
    assert!(ts.contains("kind: \"Number\""), "Should use 'kind' as tag");

    assert!(
        ts.contains("data: string"),
        "Should use 'data' as content field"
    );
    assert!(
        ts.contains("data: number"),
        "Should use 'data' as content field"
    );
}

/// Custom content field Zod schema.
#[cfg(feature = "zod")]
#[test]
fn test_custom_content_field_zod() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(tag = "kind", content = "data")]
    pub enum CustomContentZod {
        Number(i64),
        Text(String),
    }

    let zod = CustomContentZod::zod_schema();

    assert!(
        zod.contains("z.discriminatedUnion(\"kind\""),
        "Should use 'kind' as discriminator"
    );

    assert!(
        zod.contains("data: z.string()"),
        "Should use 'data' as content field"
    );
    assert!(
        zod.contains("data: z.number()"),
        "Should use 'data' as content field"
    );
}

#[test]
fn test_adjacent_named_variant_matches_the_serde_wire() {
    let named = AdjacentNamed::Named {
        x: "y".to_owned(),
        y: Some(3),
    };
    let named_json = serde_json::to_value(&named).unwrap();
    assert_eq!(
        named_json,
        serde_json::json!({ "kind": "Named", "data": { "x": "y", "y": 3_u32 } })
    );
    assert_eq!(
        serde_json::from_value::<AdjacentNamed>(named_json).unwrap(),
        named
    );

    assert_eq!(
        serde_json::to_value(AdjacentNamed::Unit).unwrap(),
        serde_json::json!({ "kind": "Unit" })
    );
}

#[test]
fn test_adjacent_named_variant_typescript_nests_under_content_key() {
    let ts = AdjacentNamed::ts_definition();
    assert!(ts.contains("kind: \"Named\""), "Got: {ts}");
    assert!(ts.contains("data: {"), "Got: {ts}");
    assert!(ts.contains("x: string"), "Got: {ts}");
    assert!(ts.contains("y: number | undefined"), "Got: {ts}");
    assert!(ts.contains("kind: \"Unit\""), "Got: {ts}");
}

#[cfg(feature = "zod")]
#[test]
fn test_adjacent_named_variant_zod_nests_under_content_key() {
    let zod = AdjacentNamed::zod_schema();
    assert!(zod.contains("data: z.strictObject({"), "Got: {zod}");
    assert!(zod.contains("x: z.string()"), "Got: {zod}");
    assert!(
        zod.contains(
            "y: z.union([z.null().transform(() => undefined), z.number().int(), z.undefined()])"
        ),
        "Got: {zod}"
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn test_adjacent_named_variant_json_schema_nests_under_content_key() {
    let schema = AdjacentNamed::json_schema();
    let one_of = schema["oneOf"].as_array().unwrap();

    let named_branch = one_of
        .iter()
        .find(|branch| branch["properties"]["kind"]["const"] == "Named")
        .unwrap();
    assert_eq!(
        named_branch["required"],
        serde_json::json!(["kind", "data"])
    );
    let data = &named_branch["properties"]["data"];
    assert_eq!(data["type"], "object");
    assert_eq!(data["additionalProperties"], false);
    assert_eq!(data["required"], serde_json::json!(["x"]));
    assert_eq!(
        data["properties"]["x"],
        serde_json::json!({ "type": "string" })
    );
    assert!(
        data["properties"].as_object().unwrap().contains_key("y"),
        "Got: {data}"
    );
}

/// Tuple variant with Vec (array type in tuple).
#[test]
fn test_tuple_with_vec() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum TupleWithVec {
        Data(Vec<String>),
        Image(String, Vec<u8>),
    }

    let ts = TupleWithVec::ts_definition();

    assert!(
        ts.contains("\"Image\": [string, Array<number>]"),
        "Image should have [string, Array<number>]. Got: {ts}"
    );

    assert!(
        ts.contains("\"Data\": Array<string>"),
        "Data should have Array<string>. Got: {ts}"
    );
}

/// Tuple with Vec Zod schema.
#[cfg(feature = "zod")]
#[test]
fn test_tuple_with_vec_zod() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum TupleWithVecZod {
        Data(Vec<String>),
        Image(String, Vec<u8>),
    }

    let zod = TupleWithVecZod::zod_schema();

    assert!(
        zod.contains("\"Image\": z.tuple([z.string(), z.array(z.number().int())])"),
        "Image should have z.tuple. Got: {zod}"
    );

    assert!(
        zod.contains("\"Data\": z.array(z.string())"),
        "Data should have z.array. Got: {zod}"
    );
}

/// A struct variant's fields sit in an object under the variant's key.
#[test]
fn test_named_struct_variant_content_is_an_object_under_the_key() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum PaymentMethod {
        BankTransfer {
            account_number: String,
            routing_number: String,
        },
        CreditCard {
            card_number: String,
            expiry: String,
        },
    }

    let ts = PaymentMethod::ts_definition();

    assert!(
        ts.contains("\"CreditCard\": {"),
        "Missing CreditCard. Got: {ts}"
    );
    assert!(
        ts.contains("card_number: string"),
        "Missing card_number. Got: {ts}"
    );
    assert!(ts.contains("expiry: string"), "Missing expiry. Got: {ts}");

    assert!(
        ts.contains("\"BankTransfer\": {"),
        "Missing BankTransfer. Got: {ts}"
    );
    assert!(
        ts.contains("account_number: string"),
        "Missing account_number. Got: {ts}"
    );
    assert!(
        ts.contains("routing_number: string"),
        "Missing routing_number. Got: {ts}"
    );
}

/// Optional types in tuple variants.
#[test]
fn test_optional_in_tuple() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum OptionalTuple {
        Maybe(Option<String>),
        MaybePair(String, Option<i64>),
    }

    let ts = OptionalTuple::ts_definition();

    // The variant's key is a slot, so the `None` is a `null` under it.
    assert!(
        ts.contains("\"Maybe\": string | null"),
        "Maybe should have nullable string. Got: {ts}"
    );

    // Multi tuple with optional element: each position is a slot too, so the same null flavor.
    assert!(
        ts.contains("\"MaybePair\": [string, number | null]"),
        "MaybePair should have tuple with optional element. Got: {ts}"
    );
}

/// Tuple variant with nested custom type.
#[test]
fn test_tuple_with_custom_type() {
    let ts = Outer::ts_definition();

    assert!(
        ts.contains("\"Wrapped\": Inner"),
        "Wrapped should reference Inner type. Got: {ts}"
    );
    assert!(
        ts.contains("\"Paired\": [Inner, number]"),
        "Paired should have tuple with Inner. Got: {ts}"
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn test_tuple_variant_sibling_element_carries_the_sibling_schema() {
    let variant = external_member(&Outer::json_schema(), "Paired");

    assert_eq!(
        variant["properties"]["Paired"]["prefixItems"][0],
        Inner::json_schema()
    );
}

/// Serde serialization compatibility.
#[test]
fn test_serde_serialization_compatibility() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
    #[serde(tag = "type", content = "value")]
    pub enum SerdeCompat {
        Number(i64),
        Pair(String, i64),
        Text(String),
    }

    let text = SerdeCompat::Text("hello".to_owned());
    let text_json = serde_json::to_value(&text).unwrap();
    assert_eq!(text_json["type"], "Text");
    assert_eq!(text_json["value"], "hello");

    let number = SerdeCompat::Number(42);
    let number_json = serde_json::to_value(&number).unwrap();
    assert_eq!(number_json["type"], "Number");
    assert_eq!(number_json["value"], 42_i64);

    let pair = SerdeCompat::Pair("hello".to_owned(), 42);
    let pair_json = serde_json::to_value(&pair).unwrap();
    assert_eq!(pair_json["type"], "Pair");
    assert!(pair_json["value"].is_array());
    assert_eq!(pair_json["value"][0], "hello");
    assert_eq!(pair_json["value"][1], 42_i64);
}

/// Complex `FixedValue` enum from original issue.
#[test]
fn test_fixed_value_original_issue() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum FixedValue {
        Alphanumeric(String),
        Boolean(bool),
        Decimal(f64),
        Image(String, Vec<u8>),
        Integer(i64),
    }

    let ts = FixedValue::ts_definition();

    assert!(
        !ts.contains("\n  : "),
        "Should not have empty field name (found line starting with colon)"
    );
    assert!(
        !ts.contains("\"\":"),
        "JSON Schema should not have empty property names"
    );

    assert!(
        ts.contains("\"Alphanumeric\": string"),
        "Missing Alphanumeric. Got: {ts}"
    );

    assert!(
        ts.contains("\"Image\": [string, Array<number>]"),
        "Image should have tuple. Got: {ts}"
    );

    assert!(
        ts.contains("\"Decimal\": number"),
        "Missing Decimal. Got: {ts}"
    );
    assert!(
        ts.contains("\"Integer\": number"),
        "Missing Integer. Got: {ts}"
    );
    assert!(
        ts.contains("\"Boolean\": boolean"),
        "Missing Boolean. Got: {ts}"
    );
}

/// `FixedValueExt` with all variant types.
#[test]
fn test_fixed_value_ext_comprehensive() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum FixedValueExt {
        Alphanumeric(String),
        Boolean(bool),
        Complex { a: String, b: bool },
        Decimal(f64),
        Image(String, Vec<u8>),
        Integer(i64),
        SingleValue,
        Tuple(i64, bool),
    }

    let ts = FixedValueExt::ts_definition();
    // The `$Variant` reader appended after the type intentionally spells `case "SingleValue":`,
    // so the no-key check below is scoped to the type declaration alone.
    let type_declaration = ts.split("\n\n").next().unwrap();

    assert!(
        ts.contains("\"Alphanumeric\": string"),
        "Missing Alphanumeric. Got: {ts}"
    );

    assert!(
        ts.contains("\"Image\": [string, Array<number>]"),
        "Missing Image. Got: {ts}"
    );
    assert!(
        ts.contains("\"Tuple\": [number, boolean]"),
        "Missing Tuple. Got: {ts}"
    );

    // Unit variant carries no key at all.
    assert!(
        ts.contains("\"SingleValue\""),
        "Missing SingleValue. Got: {ts}"
    );
    assert!(
        !type_declaration.contains("\"SingleValue\":"),
        "A unit variant carries no key. Got: {ts}"
    );

    assert!(ts.contains("\"Complex\": {"), "Missing Complex. Got: {ts}");
    assert!(ts.contains("a: string"), "Missing field a. Got: {ts}");
    assert!(ts.contains("b: boolean"), "Missing field b. Got: {ts}");
}

/// Empty tuple variant (edge case).
#[test]
fn test_empty_tuple_variant() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum EmptyTuple {
        Normal(String),
        // Note: Rust doesn't allow `Empty()` syntax directly, but we test the logic anyway
        // by having a unit variant alongside tuple variants
        Unit,
    }

    let ts = EmptyTuple::ts_definition();
    // The `$Variant` reader appended after the type intentionally spells `case "Unit":`, so the
    // no-key check below is scoped to the type declaration alone.
    let type_declaration = ts.split("\n\n").next().unwrap();

    assert!(
        ts.contains("\"Normal\": string"),
        "Missing Normal. Got: {ts}"
    );
    assert!(ts.contains("\"Unit\""), "Missing Unit. Got: {ts}");
    assert!(
        !type_declaration.contains("\"Unit\":"),
        "A unit variant carries no key. Got: {ts}"
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn test_optional_tuple_variant_element_json_schema_null_flavor() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum Row {
        Link(Option<String>, Vec<isize>, String, Option<String>),
    }

    let variant = external_member(&Row::json_schema(), "Link");

    let value = &variant["properties"]["Link"];
    assert_eq!(value["type"].as_str(), Some("array"));

    let prefix = value["prefixItems"].as_array().unwrap();
    assert_eq!(prefix.len(), 4, "Arity stays 4. Got: {prefix:?}");

    let nullable_string =
        serde_json::json!({ "anyOf": [{ "type": "string" }, { "type": "null" }] });
    assert_eq!(
        prefix[0], nullable_string,
        "Slot 0 (Option<String>) should be anyOf null. Got: {}",
        prefix[0]
    );
    assert_eq!(
        prefix[3], nullable_string,
        "Slot 3 (Option<String>) should be anyOf null. Got: {}",
        prefix[3]
    );
    // Non-optional slot stays plain — no null wrapping.
    assert_eq!(prefix[2], serde_json::json!({ "type": "string" }));
}

/// `JSDoc` comments in generated TypeScript.
#[test]
fn test_jsdoc_comments() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum JsDoc {
        Multi(String, i64),
        Single(String),
    }

    let ts = JsDoc::ts_definition();

    assert!(ts.contains("/**"), "Should have JSDoc comments");
    assert!(ts.contains("*/"), "Should have JSDoc end");

    // Each member carries its variant's own docs above the key it is named by.
    assert!(
        ts.contains("* Multi\n"),
        "Multi should be documented. Got: {ts}"
    );
    assert!(
        ts.contains("\"Multi\": [string, number]"),
        "Missing Multi. Got: {ts}"
    );
    assert!(
        ts.contains("* Single\n"),
        "Single should be documented. Got: {ts}"
    );
    assert!(
        ts.contains("\"Single\": string"),
        "Missing Single. Got: {ts}"
    );
}

#[test]
fn test_single_tuple_variant_option_content_writes_null_under_the_key() {
    assert_eq!(
        serde_json::to_value(SlotContent::Covered(None)).unwrap(),
        serde_json::json!({ "type": "Covered", "value": null }),
        "A `None` content keeps the key and writes `null` under it"
    );
    assert_eq!(
        serde_json::to_value(SlotContent::Element(vec![None])).unwrap(),
        serde_json::json!({ "type": "Element", "value": [null] }),
        "A `None` among the items is a `null` among them"
    );
}

/// TypeScript describes that content key as the slot it is.
#[test]
fn test_single_tuple_variant_option_content_typescript_null_flavor() {
    let ts = SlotContent::ts_definition();

    assert!(
        ts.contains("value: Array<number> | null"),
        "Covered's content is the slot the `None` fills with `null`. Got: {ts}"
    );
    assert!(
        ts.contains("value: Array<number | null>"),
        "Element's `None` stays among the items. Got: {ts}"
    );
    assert!(
        ts.contains("value: string;"),
        "Plain's content carries no null. Got: {ts}"
    );
}

/// The Zod schema of that content key admits the `null` serde writes.
#[cfg(feature = "zod")]
#[test]
fn test_single_tuple_variant_option_content_zod_null_flavor() {
    let zod = SlotContent::zod_schema();

    assert!(
        zod.contains("value: z.nullable(z.array(z.number().int()))"),
        "Covered's content admits the `null`. Got: {zod}"
    );
    assert!(
        zod.contains("value: z.array(z.nullable(z.number().int()))"),
        "Element's `null` stays among the items. Got: {zod}"
    );
    assert!(
        zod.contains("value: z.string()"),
        "Plain's content carries no null. Got: {zod}"
    );
}

/// The JSON schema already said so, and keeps saying it unchanged.
#[cfg(feature = "jsonschema")]
#[test]
fn test_single_tuple_variant_option_content_json_schema_null_flavor() {
    let schema = SlotContent::json_schema();
    let variant_of = |discriminator: &str| {
        schema["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .find(|member| member["properties"]["type"]["const"] == discriminator)
            .unwrap()
            .clone()
    };

    let covered = variant_of("Covered");
    assert_eq!(
        covered["properties"]["value"],
        serde_json::json!({
            "anyOf": [
                { "type": "array", "items": { "type": "integer" } },
                { "type": "null" }
            ]
        })
    );
    assert!(
        covered["required"]
            .as_array()
            .unwrap()
            .contains(&serde_json::Value::String("value".to_owned())),
        "The key a `None` cannot drop stays required. Got: {}",
        covered["required"]
    );

    assert_eq!(
        variant_of("Element")["properties"]["value"],
        serde_json::json!({
            "type": "array",
            "items": { "anyOf": [{ "type": "integer" }, { "type": "null" }] }
        })
    );
    assert_eq!(
        variant_of("Plain")["properties"]["value"],
        serde_json::json!({ "type": "string" })
    );
}

#[test]
fn test_multi_tuple_variant_option_element_writes_null_in_place() {
    assert_eq!(
        serde_json::to_value(SlotElements::Pair("a".to_owned(), None)).unwrap(),
        serde_json::json!({ "type": "Pair", "value": ["a", null] }),
        "A `None` element keeps its position and writes `null` there"
    );
}

/// TypeScript describes those elements as the slots they are.
#[test]
fn test_multi_tuple_variant_option_element_typescript_null_flavor() {
    let ts = SlotElements::ts_definition();

    assert!(
        ts.contains("value: [string, number | null]"),
        "Pair's second element is the slot the `None` fills with `null`. Got: {ts}"
    );
}

/// The Zod schema of that tuple admits the `null` serde writes.
#[cfg(feature = "zod")]
#[test]
fn test_multi_tuple_variant_option_element_zod_null_flavor() {
    let zod = SlotElements::zod_schema();

    assert!(
        zod.contains("value: z.tuple([z.string(), z.nullable(z.number().int())])"),
        "Pair's tuple admits the `null` in place. Got: {zod}"
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn test_multi_tuple_variant_option_element_json_schema_null_flavor() {
    let schema = SlotElements::json_schema();
    let variant = schema["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["properties"]["type"]["const"] == "Pair")
        .unwrap();

    assert_eq!(
        variant["properties"]["value"],
        serde_json::json!({
            "type": "array",
            "prefixItems": [
                { "type": "string" },
                { "anyOf": [{ "type": "integer" }, { "type": "null" }] }
            ],
            "items": false,
            "minItems": 2_u64,
            "maxItems": 2_u64
        })
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn test_multi_tuple_variant_json_schema_matches_tuple_field() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct ArityField {
        pub pair: (u32, u32),
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub enum ArityVariant {
        Pair(u32, u32),
    }

    let member = external_member(&ArityVariant::json_schema(), "Pair");
    let variant_value = &member["properties"]["Pair"];

    assert_eq!(
        *variant_value,
        ArityField::json_schema()["properties"]["pair"],
        "A variant's tuple array must describe as the tuple field does. Got: {variant_value}"
    );
    assert_eq!(
        *variant_value,
        serde_json::json!({
            "type": "array",
            "prefixItems": [{ "type": "integer" }, { "type": "integer" }],
            "items": false,
            "minItems": 2_u64,
            "maxItems": 2_u64
        })
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn test_multi_tuple_variant_arity_bounds_reject_wrong_length_arrays() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(tag = "type", content = "value")]
    pub enum ArityProbe {
        Pair(u32, u32),
    }

    let schema = ArityProbe::json_schema();
    let value = &schema["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["properties"]["type"]["const"] == "Pair")
        .unwrap()["properties"]["value"];
    let admitted = value["minItems"].as_u64().unwrap()..=value["maxItems"].as_u64().unwrap();

    let written = serde_json::to_value(ArityProbe::Pair(1, 2)).unwrap();
    let written_len = u64::try_from(written["value"].as_array().unwrap().len()).unwrap();
    assert!(
        admitted.contains(&written_len),
        "What serde writes must validate. Got length {written_len} against {admitted:?}"
    );

    for rejected in [
        serde_json::json!([]),
        serde_json::json!([1_u32]),
        serde_json::json!([1_u32, 2_u32, 3_u32]),
    ] {
        let len = u64::try_from(rejected.as_array().unwrap().len()).unwrap();
        assert!(
            !admitted.contains(&len),
            "An array of {len} elements is not the tuple. Got {admitted:?}"
        );
    }
}

#[test]
fn test_attribute_less_enum_writes_the_externally_tagged_form() {
    assert_eq!(
        serde_json::to_value(External::Pair(1, 2)).unwrap(),
        serde_json::json!({ "Pair": [1_u32, 2_u32] }),
        "A tuple variant's name is the key and its elements are the array under it"
    );
    assert_eq!(
        serde_json::to_value(External::Single("a".to_owned())).unwrap(),
        serde_json::json!({ "Single": "a" }),
        "A newtype variant's content sits under the key, unwrapped"
    );
    assert_eq!(
        serde_json::to_value(External::Fields {
            a: "a".to_owned(),
            b: true
        })
        .unwrap(),
        serde_json::json!({ "Fields": { "a": "a", "b": true } }),
        "A struct variant's fields sit in an object under the key"
    );
    assert_eq!(
        serde_json::to_value(External::Bare).unwrap(),
        serde_json::json!("Bare"),
        "A unit variant carries no key at all: it is the bare name"
    );
}

#[test]
fn test_attribute_less_enum_typescript_is_the_externally_tagged_union() {
    let ts = External::ts_definition();
    // The `$Variant` reader appended after the type intentionally spells `case "Bare":`, so the
    // no-key check below is scoped to the type declaration alone.
    let type_declaration = ts.split("\n\n").next().unwrap();

    assert!(
        ts.contains("\"Pair\": [number, number]"),
        "The tuple sits under the variant's key. Got: {ts}"
    );
    assert!(
        ts.contains("\"Single\": string"),
        "The newtype content sits under the key, unwrapped. Got: {ts}"
    );
    assert!(
        ts.contains("\"Fields\": {"),
        "The struct fields sit in an object under the key. Got: {ts}"
    );
    assert!(ts.contains("\"Bare\""), "Missing Bare. Got: {ts}");
    assert!(
        !type_declaration.contains("\"Bare\":"),
        "A unit variant carries no key. Got: {ts}"
    );
    assert!(
        !ts.contains("type: \"Pair\""),
        "Nothing writes a tag beside the content. Got: {ts}"
    );
}

/// The externally tagged reader reads the sole key, or the bare string for a unit variant.
#[test]
#[cfg(feature = "typescript")]
fn test_attribute_less_enum_variant_reader_reads_the_sole_key_or_the_bare_string() {
    let ts = External::ts_definition();

    assert!(
        ts.contains("export function External$Variant(value: unknown): string {"),
        "Got: {ts}"
    );
    assert!(ts.contains("case \"Pair\": return \"Pair\";"), "Got: {ts}");
    assert!(
        ts.contains("case \"Bare\": return \"Bare\";"),
        "the unit variant's own bare string is a case label too. Got: {ts}"
    );
}

/// The Zod schema admits the same union.
#[cfg(feature = "zod")]
#[test]
fn test_attribute_less_enum_zod_is_the_externally_tagged_union() {
    let zod = External::zod_schema();

    assert!(zod.contains("z.union(["), "Missing union. Got: {zod}");
    assert!(
        !zod.contains("z.discriminatedUnion"),
        "No shared field to discriminate on. Got: {zod}"
    );
    assert!(
        zod.contains(
            "z.strictObject({\n  \"Pair\": z.tuple([z.number().int(), z.number().int()]),\n})"
        ),
        "Pair's member is the closed object holding its tuple. Got: {zod}"
    );
    assert!(
        zod.contains("z.strictObject({\n  \"Single\": z.string(),\n})"),
        "Single's member holds its content unwrapped. Got: {zod}"
    );
    assert!(
        zod.contains("\"Fields\": z.strictObject("),
        "Fields' content is the object its fields sit in. Got: {zod}"
    );
    assert!(
        zod.contains("z.literal(\"Bare\")"),
        "Bare is the name alone. Got: {zod}"
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn test_attribute_less_enum_json_schema_is_the_externally_tagged_union() {
    let schema = External::json_schema();

    assert_eq!(
        external_member(&schema, "Pair")["properties"]["Pair"],
        serde_json::json!({
            "type": "array",
            "prefixItems": [{ "type": "integer" }, { "type": "integer" }],
            "items": false,
            "minItems": 2_u64,
            "maxItems": 2_u64
        })
    );
    assert_eq!(
        external_member(&schema, "Single")["properties"]["Single"],
        serde_json::json!({ "type": "string" })
    );
    assert_eq!(
        external_member(&schema, "Fields")["properties"]["Fields"],
        serde_json::json!({
            "type": "object",
            "properties": { "a": { "type": "string" }, "b": { "type": "boolean" } },
            "required": ["a", "b"],
            "additionalProperties": false
        })
    );

    // Every keyed member is closed around its one key, which is what serde's single-key map
    // deserializer accepts.
    for variant in ["Pair", "Single", "Fields"] {
        let member = external_member(&schema, variant);
        assert_eq!(member["type"], "object", "Got: {member}");
        assert_eq!(member["additionalProperties"], false, "Got: {member}");
        assert_eq!(
            member["properties"].as_object().unwrap().len(),
            1,
            "A member carries the one key serde writes. Got: {member}"
        );
    }

    assert!(
        schema["oneOf"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!({ "type": "string", "const": "Bare" })),
        "The unit variant is the bare name. Got: {schema}"
    );
}

/// The round trip both schemas owe.
#[cfg(feature = "jsonschema")]
#[test]
fn test_attribute_less_enum_round_trips_against_its_schema() {
    let schema = External::json_schema();

    for (variant, value) in [
        ("Pair", External::Pair(1, 2)),
        ("Single", External::Single("a".to_owned())),
        (
            "Fields",
            External::Fields {
                a: "a".to_owned(),
                b: true,
            },
        ),
    ] {
        let written = serde_json::to_value(&value).unwrap();
        let member = external_member(&schema, variant);
        let content = &written[variant];

        assert_eq!(
            written.as_object().unwrap().len(),
            1,
            "A closed member admits exactly the one key. Got: {written}"
        );
        let declared = &member["properties"][variant];
        assert_eq!(
            declared["type"],
            json_type_name(content),
            "The key holds what serde writes under it. Got: {declared}"
        );

        assert_eq!(
            serde_json::from_value::<External>(written.clone()).unwrap(),
            value,
            "What serde writes must read back. Got: {written}"
        );
    }

    // The bare name is the whole value, and reads back as the variant it names.
    let bare = serde_json::to_value(External::Bare).unwrap();
    assert_eq!(
        serde_json::from_value::<External>(bare).unwrap(),
        External::Bare
    );

    // A payload the schema admits deserializes: the key the member requires, holding the content
    // it declares.
    assert_eq!(
        serde_json::from_value::<External>(serde_json::json!({ "Pair": [7_u32, 8_u32] })).unwrap(),
        External::Pair(7, 8)
    );

    // And the adjacent form the schema no longer describes is one the type cannot read either.
    assert!(
        serde_json::from_value::<External>(
            serde_json::json!({ "type": "Pair", "value": [1_u32, 2_u32] })
        )
        .is_err(),
        "The adjacent form is not what this enum reads"
    );
}

/// Naming the tagging attributes keeps the adjacent form untouched.
#[test]
fn test_explicitly_tagged_twin_keeps_the_adjacent_form() {
    assert_eq!(
        serde_json::to_value(Adjacent::Pair(1, 2)).unwrap(),
        serde_json::json!({ "type": "Pair", "value": [1_u32, 2_u32] })
    );

    let ts = Adjacent::ts_definition();
    assert!(ts.contains("type: \"Pair\""), "Got: {ts}");
    assert!(ts.contains("value: [number, number]"), "Got: {ts}");
    assert!(ts.contains("type: \"Bare\""), "Got: {ts}");
    assert!(ts.contains("a: string"), "Got: {ts}");
}

/// The adjacent reader reads the tag key only, whatever the content key beside it holds.
#[test]
#[cfg(feature = "typescript")]
fn test_adjacent_variant_reader_reads_the_tag_key_and_ignores_the_content_key() {
    let ts = Adjacent::ts_definition();

    assert!(
        ts.contains("export function Adjacent$Variant(value: unknown): string {"),
        "Got: {ts}"
    );
    assert!(
        ts.contains("switch ((value as { type?: unknown }).type) {"),
        "reads the tag key alone. Got: {ts}"
    );
    assert!(!ts.contains("value?: unknown"), "Got: {ts}");
    assert!(ts.contains("case \"Pair\": return \"Pair\";"), "Got: {ts}");
    assert!(ts.contains("case \"Bare\": return \"Bare\";"), "Got: {ts}");
}

/// What serde writes for a renamed variant, and the keys the surfaces carry for it.
#[test]
fn test_renamed_variant_key_is_the_wire_name() {
    assert_eq!(
        serde_json::to_value(RenamedExternal::Mime("x".to_owned())).unwrap(),
        serde_json::json!({ "application/pdf": "x" })
    );
    assert_eq!(
        serde_json::to_value(RenamedExternal::BigThing(1)).unwrap(),
        serde_json::json!({ "bigThing": 1_u32 })
    );
    assert_eq!(
        serde_json::to_value(RenamedExternal::UnitThing).unwrap(),
        serde_json::json!("unitThing")
    );

    let ts = RenamedExternal::ts_definition();
    assert!(
        ts.contains("\"application/pdf\": string"),
        "The renamed key is held as written. Got: {ts}"
    );
    assert!(
        ts.contains("\"bigThing\": number"),
        "`rename_all` reaches the key. Got: {ts}"
    );
    assert!(
        ts.contains("\"unitThing\""),
        "The unit variant is its renamed name. Got: {ts}"
    );
}

/// The Zod schema holds the same keys.
#[cfg(feature = "zod")]
#[test]
fn test_renamed_variant_key_is_the_wire_name_in_zod() {
    let zod = RenamedExternal::zod_schema();

    assert!(
        zod.contains("\"application/pdf\": z.string()"),
        "Got: {zod}"
    );
    assert!(zod.contains("\"bigThing\": z.number().int()"), "Got: {zod}");
    assert!(zod.contains("z.literal(\"unitThing\")"), "Got: {zod}");
}

#[test]
#[cfg(feature = "typescript")]
fn test_renamed_variant_key_reader_inverts_the_rename_and_the_rename_all() {
    let ts = RenamedExternal::ts_definition();

    assert!(
        ts.contains("case \"application/pdf\": return \"Mime\";"),
        "the variant's own rename inverts. Got: {ts}"
    );
    assert!(
        ts.contains("case \"bigThing\": return \"BigThing\";"),
        "the container's rename_all inverts. Got: {ts}"
    );
    assert!(
        ts.contains("case \"unitThing\": return \"UnitThing\";"),
        "Got: {ts}"
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn test_renamed_variant_key_is_the_wire_name_in_json_schema() {
    let schema = RenamedExternal::json_schema();

    assert_eq!(
        external_member(&schema, "application/pdf")["properties"]["application/pdf"],
        serde_json::json!({ "type": "string" })
    );
    assert_eq!(
        external_member(&schema, "bigThing")["properties"]["bigThing"],
        serde_json::json!({ "type": "integer" })
    );
    assert!(
        schema["oneOf"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!({ "type": "string", "const": "unitThing" })),
        "Got: {schema}"
    );
}

#[cfg(feature = "zod")]
#[test]
fn test_recursive_external_variant_defers_its_reference() {
    let zod = RecursiveExternal::zod_schema();

    assert!(
        zod.contains("get \"Arr\"() { return z.array(RecursiveExternal$Schema); },"),
        "Got: {zod}"
    );
    assert!(zod.contains("\"Txt\": z.string()"), "Got: {zod}");
}

#[cfg(feature = "zod")]
#[test]
fn test_explicitly_tagged_twin_keeps_the_adjacent_zod_form() {
    let zod = Adjacent::zod_schema();

    assert!(zod.contains("z.discriminatedUnion(\"type\""), "Got: {zod}");
    assert!(
        zod.contains("value: z.tuple([z.number().int(), z.number().int()])"),
        "Got: {zod}"
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn test_explicitly_tagged_twin_keeps_the_adjacent_json_schema_form() {
    let schema = Adjacent::json_schema();

    assert_eq!(schema["type"], "object");
    let variant = schema["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["properties"]["type"]["const"] == "Pair")
        .unwrap();
    assert_eq!(
        variant["properties"]["value"],
        serde_json::json!({
            "type": "array",
            "prefixItems": [{ "type": "integer" }, { "type": "integer" }],
            "items": false,
            "minItems": 2_u64,
            "maxItems": 2_u64
        })
    );
}

/// What serde writes for a bare tag.
#[test]
fn test_bare_tag_writes_the_variant_data_beside_the_tag() {
    assert_eq!(
        serde_json::to_value(Internal::Fields {
            a: "a".to_owned(),
            b: true
        })
        .unwrap(),
        serde_json::json!({ "type": "Fields", "a": "a", "b": true }),
        "A struct variant's fields sit beside the tag"
    );
    assert_eq!(
        serde_json::to_value(Internal::Wrapped(TagPayload {
            a: "a".to_owned(),
            b: true
        }))
        .unwrap(),
        serde_json::json!({ "type": "Wrapped", "a": "a", "b": true }),
        "A newtype variant's inner members sit beside the tag too: no key holds them"
    );
    assert_eq!(
        serde_json::to_value(Internal::Bare).unwrap(),
        serde_json::json!({ "type": "Bare" }),
        "A unit variant is the tag alone"
    );
}

#[test]
fn test_bare_tag_newtype_over_a_scalar_is_unserializable() {
    let refusal = serde_json::to_value(InternalScalar::Single("a".to_owned()))
        .unwrap_err()
        .to_string();
    assert!(
        refusal.contains("cannot serialize tagged newtype variant"),
        "Got: {refusal}"
    );
    assert!(refusal.contains("containing a string"), "Got: {refusal}");
}

/// TypeScript spreads the inner type beside the tag rather than putting it under a key.
#[test]
fn test_bare_tag_typescript_spreads_the_inner_type_beside_the_tag() {
    let ts = Internal::ts_definition();
    // The `$Variant` reader appended after the type takes a `value: unknown` parameter, so the
    // no-content-key check below is scoped to the type declaration alone.
    let type_declaration = ts.split("\n\n").next().unwrap();

    assert!(
        ts.contains("type: \"Wrapped\";\n} & TagPayload"),
        "The inner type joins the tag's object. Got: {ts}"
    );
    assert!(
        !type_declaration.contains("value:"),
        "Nothing writes a content key under a bare tag. Got: {ts}"
    );
    assert!(ts.contains("type: \"Fields\";"), "Got: {ts}");
    assert!(ts.contains("  a: string;"), "Got: {ts}");
    assert!(ts.contains("type: \"Bare\";"), "Got: {ts}");
}

/// The Zod member for a newtype variant is the tag's object intersected with the inner schema.
#[cfg(feature = "zod")]
#[test]
fn test_bare_tag_zod_intersects_the_inner_schema() {
    let zod = Internal::zod_schema();

    assert!(
        zod.contains(
            "z.strictObject({\n  type: z.literal(\"Wrapped\"),\n}).and(z.lazy(() => TagPayload$Schema))"
        ),
        "Got: {zod}"
    );
    assert!(zod.contains("z.union(["), "Got: {zod}");
    assert!(
        !zod.contains("z.discriminatedUnion"),
        "An intersection member cannot be discriminated on. Got: {zod}"
    );
    assert!(!zod.contains("value:"), "Got: {zod}");
}

#[cfg(feature = "zod")]
#[test]
fn test_a_newtype_variants_content_is_never_read_while_the_const_initializes() {
    for (zod, inner) in [
        (Internal::zod_schema(), "TagPayload"),
        (InternalOverBrand::zod_schema(), "InternalSlug"),
        (InternalOverUntagged::zod_schema(), "InternalEither"),
    ] {
        assert!(
            zod.contains(&format!(".and(z.lazy(() => {inner}$Schema))")),
            "expected a deferred content, got: {zod}"
        );
        assert!(
            !zod.contains(&format!(".and({inner}$Schema)")),
            "`{inner}$Schema` is read eagerly in: {zod}"
        );
    }
}

#[cfg(feature = "zod")]
#[test]
fn test_bare_tag_without_a_newtype_variant_still_discriminates() {
    let zod = InternalNamedOnly::zod_schema();

    assert!(
        zod.contains("z.discriminatedUnion(\"type\", [z.strictObject({\n  type: z.literal(\"Fields\"),\n  a: z.string(),\n})])"),
        "Got: {zod}"
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn test_bare_tag_json_schema_holds_the_inner_fields_beside_the_tag() {
    let schema = Internal::json_schema();
    let wrapped = schema["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["properties"]["type"]["const"] == "Wrapped")
        .unwrap();

    assert_eq!(
        wrapped["properties"],
        serde_json::json!({
            "type": { "type": "string", "const": "Wrapped" },
            "a": { "type": "string" },
            "b": { "type": "boolean" }
        })
    );
    assert_eq!(
        wrapped["required"],
        serde_json::json!(["type", "a", "b"]),
        "The tag and everything the inner type requires"
    );
    assert_eq!(wrapped["additionalProperties"], false);
    assert!(
        wrapped["properties"]["value"].is_null(),
        "There is no content key. Got: {wrapped}"
    );
}

/// The round trip the schema owes.
#[cfg(feature = "jsonschema")]
#[test]
fn test_bare_tag_round_trips_against_its_schema() {
    let schema = Internal::json_schema();

    for value in [
        Internal::Bare,
        Internal::Fields {
            a: "a".to_owned(),
            b: true,
        },
        Internal::Wrapped(TagPayload {
            a: "a".to_owned(),
            b: true,
        }),
    ] {
        let written = serde_json::to_value(&value).unwrap();
        let member = schema["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .find(|member| member["properties"]["type"]["const"] == written["type"])
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        assert!(!member.is_null(), "No member for {written}");

        let declared = member["properties"].as_object().unwrap();
        for key in written.as_object().unwrap().keys() {
            assert!(
                declared.contains_key(key),
                "The member admits every key serde writes. Missing `{key}` in {member}"
            );
        }
        for required in member["required"].as_array().unwrap() {
            assert!(
                written[required.as_str().unwrap()] != serde_json::Value::Null,
                "The member requires only keys serde writes. Got: {written}"
            );
        }

        assert_eq!(
            serde_json::from_value::<Internal>(written.clone()).unwrap(),
            value,
            "What serde writes must read back. Got: {written}"
        );
    }

    // And the adjacent form the schema no longer describes is one the type cannot read either.
    assert!(
        serde_json::from_value::<Internal>(
            serde_json::json!({ "type": "Wrapped", "value": { "a": "a", "b": true } })
        )
        .is_err(),
        "A content key is not what this enum reads"
    );
}

/// Naming a content key beside the tag keeps the adjacent form, content key and all.
#[test]
fn test_adjacent_twin_keeps_its_content_key() {
    assert_eq!(
        serde_json::to_value(Adjacent::Single("a".to_owned())).unwrap(),
        serde_json::json!({ "type": "Single", "value": "a" })
    );

    let ts = Adjacent::ts_definition();
    assert!(ts.contains("value: string"), "Got: {ts}");
}

/// What serde writes for a bare tag over a plain enum.
#[test]
fn test_bare_tag_over_a_plain_enum_writes_a_key_the_tag_does_not_name() {
    assert_eq!(
        serde_json::to_value(InternalOverEnum::EnumInner(InternalHue::Red)).unwrap(),
        serde_json::json!({ "type": "EnumInner", "Red": null })
    );
}

#[test]
fn test_bare_tag_over_a_string_newtype_is_unserializable() {
    let refusal = serde_json::to_value(InternalOverBrand::Branded(InternalSlug("s".to_owned())))
        .unwrap_err()
        .to_string();
    assert!(
        refusal.contains("cannot serialize tagged newtype variant"),
        "Got: {refusal}"
    );
    assert!(refusal.contains("containing a string"), "Got: {refusal}");
}

#[cfg(feature = "jsonschema")]
#[test]
#[should_panic(
    expected = "`InternalOverBrand`: the content of variant `Branded`, `InternalSlug` is not written as an object"
)]
fn test_bare_tag_over_a_string_newtype_is_refused_by_the_merge() {
    assert!(InternalOverBrand::json_schema().is_object());
}

/// The remedy the refusal names is one the author can act on.
#[cfg(feature = "jsonschema")]
#[test]
#[should_panic(expected = "name a `content` key so the content gets an object of its own")]
fn test_the_merge_refusal_names_the_remedy() {
    assert!(InternalOverBrand::json_schema().is_object());
}

#[cfg(feature = "jsonschema")]
#[test]
fn test_bare_tag_over_a_struct_documents_byte_identically() {
    assert_eq!(
        serde_json::to_string(&Internal::json_schema()).unwrap(),
        r#"{"type":"object","oneOf":[{"additionalProperties":false,"properties":{"type":{"type":"string","const":"Bare"}},"required":["type"]},{"additionalProperties":false,"properties":{"type":{"type":"string","const":"Fields"},"a":{"type":"string"},"b":{"type":"boolean"}},"required":["type","a","b"]},{"type":"object","properties":{"type":{"type":"string","const":"Wrapped"},"a":{"type":"string"},"b":{"type":"boolean"}},"required":["type","a","b"],"additionalProperties":false}]}"#
    );
}

/// What serde writes for a bare tag over an untagged enum.
#[test]
fn test_bare_tag_over_an_untagged_enum_writes_the_matched_members_keys() {
    assert_eq!(
        serde_json::to_value(InternalOverUntagged::Wrapped(InternalEither::First(
            InternalFirst { a: "x".to_owned() }
        )))
        .unwrap(),
        serde_json::json!({ "type": "Wrapped", "a": "x" })
    );
    assert_eq!(
        serde_json::to_value(InternalOverUntagged::Wrapped(InternalEither::Second(
            InternalSecond { b: true }
        )))
        .unwrap(),
        serde_json::json!({ "type": "Wrapped", "b": true })
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn test_bare_tag_over_an_untagged_enum_multiplies_over_its_members() {
    assert_eq!(
        serde_json::to_string(&InternalOverUntagged::json_schema()).unwrap(),
        r#"{"type":"object","oneOf":[{"type":"object","anyOf":[{"type":"object","properties":{"type":{"type":"string","const":"Wrapped"},"a":{"type":"string"}},"required":["type","a"],"additionalProperties":false},{"type":"object","properties":{"type":{"type":"string","const":"Wrapped"},"b":{"type":"boolean"}},"required":["type","b"],"additionalProperties":false}]}]}"#
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn test_bare_tag_over_an_untagged_enum_requires_every_key_serde_writes() {
    let schema = InternalOverUntagged::json_schema();
    let required: Vec<&serde_json::Value> = schema["oneOf"][0]["anyOf"]
        .as_array()
        .unwrap()
        .iter()
        .map(|branch| &branch["required"])
        .collect();
    assert_eq!(
        required,
        vec![
            &serde_json::json!(["type", "a"]),
            &serde_json::json!(["type", "b"])
        ],
        "Got: {schema}"
    );
}

#[cfg(feature = "jsonschema")]
#[test]
#[should_panic(
    expected = "`InternalOverScalarUntagged`: the content of variant `Wrapped`, `InternalScalarEither` writes a union member that is not an object — its branch 2 describes a `string`"
)]
fn test_a_string_member_of_a_tagged_untagged_enum_is_refused_by_the_merge() {
    assert!(InternalOverScalarUntagged::json_schema().is_object());
}

#[test]
fn test_serde_writes_and_reads_a_dropped_variant_slot_in_neither_direction() {
    let written = serde_json::to_value(ExternalDroppedSlots::Lead("s".to_owned(), 7_u32)).unwrap();
    assert_eq!(written, serde_json::json!({ "Lead": [7_u32] }));

    assert_eq!(
        serde_json::from_value::<ExternalDroppedSlots>(written).unwrap(),
        ExternalDroppedSlots::Lead(String::new(), 7_u32)
    );
    assert!(
        serde_json::from_str::<ExternalDroppedSlots>(r#"{"Lead":["s",7]}"#).is_err(),
        "the full-arity array must not read back"
    );
}

#[test]
fn test_serde_writes_the_variant_slots_it_still_carries_in_their_new_places() {
    assert_eq!(
        serde_json::to_value(ExternalDroppedSlots::Trailing("s".to_owned(), 1_u32)).unwrap(),
        serde_json::json!({ "Trailing": ["s"] })
    );
    assert_eq!(
        serde_json::to_value(ExternalDroppedSlots::Middle(
            "a".to_owned(),
            Some("s".to_owned()),
            7_u32
        ))
        .unwrap(),
        serde_json::json!({ "Middle": ["a", 7_u32] })
    );
    assert_eq!(
        serde_json::to_value(ExternalDroppedSlots::Every("s".to_owned(), 1_u32)).unwrap(),
        serde_json::json!({ "Every": [] })
    );
}

#[test]
fn test_serde_writes_a_variant_whose_lone_slot_is_dropped_as_its_name_alone() {
    assert_eq!(
        serde_json::to_value(ExternalDroppedSlots::Lone("s".to_owned())).unwrap(),
        serde_json::json!("Lone")
    );
    assert_eq!(
        serde_json::from_str::<ExternalDroppedSlots>(r#""Lone""#).unwrap(),
        ExternalDroppedSlots::Lone(String::new())
    );
    assert!(
        serde_json::from_str::<ExternalDroppedSlots>(r#"{"Lone":"s"}"#).is_err(),
        "the slot's own value must not read back"
    );
}

#[test]
fn test_serde_writes_a_dropped_variant_slot_the_same_way_under_a_content_key() {
    for (held, written) in [
        (
            AdjacentDroppedSlots::Lead("s".to_owned(), 7_u32),
            serde_json::json!({ "type": "Lead", "value": [7_u32] }),
        ),
        (
            AdjacentDroppedSlots::Every("s".to_owned(), 1_u32),
            serde_json::json!({ "type": "Every", "value": [] }),
        ),
    ] {
        assert_eq!(serde_json::to_value(held).unwrap(), written);
        assert!(
            serde_json::from_value::<AdjacentDroppedSlots>(written.clone()).is_ok(),
            "must read back: {written}"
        );
    }
}

#[test]
fn test_serde_refuses_the_adjacent_payload_it_writes_for_a_dropped_lone_slot() {
    assert_eq!(
        serde_json::to_value(AdjacentLoneSlotWire::Lone("s".to_owned())).unwrap(),
        serde_json::json!({ "type": "Lone" })
    );
    let refused = serde_json::from_str::<AdjacentLoneSlotWire>(r#"{"type":"Lone"}"#).unwrap_err();
    assert!(
        refused.to_string().contains("missing field `value`"),
        "Got: {refused}"
    );
    assert_eq!(
        serde_json::from_str::<AdjacentLoneSlotWire>(r#"{"type":"Lone","value":null}"#).unwrap(),
        AdjacentLoneSlotWire::Lone(String::new())
    );
    assert!(
        serde_json::from_str::<AdjacentLoneSlotWire>(r#"{"type":"Lone","value":"s"}"#).is_err(),
        "the slot's own value must not read back"
    );
}

#[test]
fn test_serde_reads_back_the_adjacent_payload_a_declared_unit_variant_writes() {
    let written = serde_json::to_value(AdjacentUnitWire::Lone).unwrap();
    assert_eq!(written, serde_json::json!({ "type": "Lone" }));
    assert_eq!(
        serde_json::from_value::<AdjacentUnitWire>(written).unwrap(),
        AdjacentUnitWire::Lone
    );
    assert_eq!(
        serde_json::from_str::<AdjacentUnitWire>(r#"{"type":"Lone","value":null}"#).unwrap(),
        AdjacentUnitWire::Lone
    );
}

#[test]
fn test_serde_writes_a_dropped_lone_slot_as_the_tag_alone_under_a_bare_tag() {
    let written = serde_json::to_value(InternalDroppedSlot::Lone("s".to_owned())).unwrap();
    assert_eq!(written, serde_json::json!({ "type": "Lone" }));
    assert_eq!(
        serde_json::from_value::<InternalDroppedSlot>(written).unwrap(),
        InternalDroppedSlot::Lone(String::new())
    );
}

#[cfg(feature = "typescript")]
#[test]
fn test_a_dropped_variant_slot_leaves_the_described_tuple_in_typescript() {
    let ts = ExternalDroppedSlots::ts_definition();
    // The `$Variant` reader appended after the type intentionally spells `case "Lone":`, so the
    // no-key check below is scoped to the type declaration alone.
    let type_declaration = ts.split("\n\n").next().unwrap();
    assert!(ts.contains("\"Lead\": [number];"), "Got: {ts}");
    assert!(ts.contains("\"Trailing\": [string];"), "Got: {ts}");
    assert!(ts.contains("\"Middle\": [string, number];"), "Got: {ts}");
    assert!(ts.contains("\"Every\": [];"), "Got: {ts}");
    assert!(!type_declaration.contains("\"Lone\":"), "Got: {ts}");
}

#[cfg(feature = "typescript")]
#[test]
fn test_a_variant_whose_lone_slot_is_dropped_describes_as_its_name_in_typescript() {
    let ts = ExternalDroppedSlots::ts_definition();
    // The `$Variant` reader appended after the type intentionally spells `case "Lone":`, so the
    // no-key check below is scoped to the type declaration alone.
    let type_declaration = ts.split("\n\n").next().unwrap();
    assert!(ts.contains("\"Lone\""), "Got: {ts}");
    assert!(!type_declaration.contains("\"Lone\":"), "Got: {ts}");
}

/// The same shrink on the Zod surface, where the arity is the tuple's own member list.
#[cfg(feature = "zod")]
#[test]
fn test_a_dropped_variant_slot_leaves_the_described_tuple_in_zod() {
    let zod = ExternalDroppedSlots::zod_schema();
    assert!(
        zod.contains("\"Lead\": z.tuple([z.number().int()])"),
        "Got: {zod}"
    );
    assert!(zod.contains("\"Every\": z.tuple([])"), "Got: {zod}");
    assert!(zod.contains("z.literal(\"Lone\")"), "Got: {zod}");
}

#[cfg(feature = "jsonschema")]
#[test]
fn test_a_dropped_variant_slot_shrinks_the_described_arity_in_json_schema() {
    let schema = ExternalDroppedSlots::json_schema();
    assert_eq!(
        external_member(&schema, "Lead")["properties"]["Lead"],
        serde_json::json!({
            "type": "array",
            "prefixItems": [{ "type": "integer" }],
            "items": false,
            "minItems": 1_u32,
            "maxItems": 1_u32
        }),
        "Got: {schema}"
    );
    assert_eq!(
        external_member(&schema, "Every")["properties"]["Every"],
        serde_json::json!({
            "type": "array",
            "prefixItems": [],
            "items": false,
            "minItems": 0_u32,
            "maxItems": 0_u32
        }),
        "Got: {schema}"
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn test_a_variant_whose_lone_slot_is_dropped_describes_as_a_string_in_json_schema() {
    let schema = ExternalDroppedSlots::json_schema();
    assert!(
        schema["oneOf"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!({ "type": "string", "const": "Lone" })),
        "Got: {schema}"
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn test_a_dropped_variant_slot_shrinks_the_content_key_in_json_schema() {
    let schema = AdjacentDroppedSlots::json_schema();
    let members = schema["oneOf"].as_array().unwrap();
    let member = |name: &str| {
        let found = members
            .iter()
            .find(|member| member["properties"]["type"]["const"] == name)
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        assert!(!found.is_null(), "No member `{name}`. Got: {schema}");
        found
    };
    assert_eq!(
        member("Lead")["properties"]["value"]["maxItems"],
        1_u32,
        "Got: {schema}"
    );
    let lone = member("Lone");
    assert_eq!(
        lone["required"],
        serde_json::json!(["type"]),
        "Got: {schema}"
    );
    assert!(lone["properties"]["value"].is_null(), "Got: {schema}");
}

#[test]
fn test_the_declared_unit_remedy_writes_the_payload_the_refused_collapse_wrote() {
    let written = serde_json::to_value(AdjacentDroppedSlots::Lone).unwrap();
    assert_eq!(written, serde_json::json!({ "type": "Lone" }));
    assert_eq!(
        serde_json::from_value::<AdjacentDroppedSlots>(written).unwrap(),
        AdjacentDroppedSlots::Lone
    );
}

#[cfg(feature = "jsonschema")]
#[test]
fn test_a_dropped_lone_slot_leaves_the_bare_tag_alone_in_json_schema() {
    let schema = InternalDroppedSlot::json_schema();
    let lone = schema["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .find(|member| member["properties"]["type"]["const"] == "Lone")
        .unwrap();
    assert_eq!(
        lone["required"],
        serde_json::json!(["type"]),
        "Got: {schema}"
    );
}

#[cfg(feature = "typescript")]
#[test]
fn test_a_variant_carrying_no_dropped_slot_describes_unchanged_in_typescript() {
    for (dropped, kept) in [
        (
            ExternalDroppedSlots::ts_definition(),
            ExternalKeptOnly::ts_definition(),
        ),
        (
            AdjacentDroppedSlots::ts_definition(),
            AdjacentKeptOnly::ts_definition(),
        ),
    ] {
        // Scoped to the type declaration alone: the `$Variant` reader appended after it on both
        // sides also opens and closes braces, which would throw off `rsplit_once` below.
        let dropped_type = dropped.split("\n\n").next().unwrap();
        let kept_type = kept.split("\n\n").next().unwrap();
        let member = kept_type
            .rsplit_once('{')
            .unwrap()
            .1
            .rsplit_once('}')
            .unwrap()
            .0
            .to_owned();
        assert!(
            dropped_type.contains(&member),
            "Missing:\n{member}\nGot:\n{dropped}"
        );
    }
}

/// The same reading on the Zod surface.
#[cfg(feature = "zod")]
#[test]
fn test_a_variant_carrying_no_dropped_slot_describes_unchanged_in_zod() {
    assert!(
        ExternalDroppedSlots::zod_schema()
            .contains("\"Kept\": z.tuple([z.number().int(), z.boolean()])"),
        "Got: {}",
        ExternalDroppedSlots::zod_schema()
    );
    assert!(
        AdjacentDroppedSlots::zod_schema()
            .contains("value: z.tuple([z.number().int(), z.boolean()])"),
        "Got: {}",
        AdjacentDroppedSlots::zod_schema()
    );
}

/// The same reading on the JSON surface, where the whole member is comparable at once.
#[cfg(feature = "jsonschema")]
#[test]
fn test_a_variant_carrying_no_dropped_slot_describes_unchanged_in_json_schema() {
    assert_eq!(
        external_member(&ExternalDroppedSlots::json_schema(), "Kept"),
        external_member(&ExternalKeptOnly::json_schema(), "Kept")
    );
    let adjacent_member = |schema: &serde_json::Value| {
        schema["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .find(|member| member["properties"]["type"]["const"] == "Kept")
            .cloned()
            .unwrap()
    };
    assert_eq!(
        adjacent_member(&AdjacentDroppedSlots::json_schema()),
        adjacent_member(&AdjacentKeptOnly::json_schema())
    );
}
