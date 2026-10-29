//! The Zod text a `preprocess` on a type writes: the wrap around each shape's own expression.

#![cfg(feature = "zod")]

use super::models::{
    Carrier, ClosedDock, Crate, Grade, Handover, Label, Manifest, Parcel, Reference, Routed,
    Shipment, Stack, Transfer, Weighed, Wrapped,
};

/// What `schema` binds first: the text after the `=` of its first binding, which is the raw
/// `const` where TypeScript is written and the exported one where it is not.
fn bound(schema: &str) -> &str {
    schema.split_once("Schema = ").unwrap().1
}

/// The wrap opens each binding, ahead of the type's own expression.
#[test]
fn the_wrap_opens_the_expression_of_every_shape() {
    for (schema, opens) in [
        (
            ClosedDock::zod_schema(),
            "z.preprocess(unwrapped, z.strictObject({\n}))",
        ),
        (
            Stack::zod_schema(),
            "z.preprocess(unwrapped, z.tuple([z.array(z.string()), z.number().int().min(0).max(4294967295)]))",
        ),
        (
            Parcel::zod_schema(),
            "z.preprocess(withoutFragile, z.strictObject({\n",
        ),
        (
            Shipment::zod_schema(),
            "z.preprocess(unwrapped, z.discriminatedUnion(",
        ),
        (
            Handover::zod_schema(),
            "z.preprocess(unwrapped, z.discriminatedUnion(",
        ),
        (
            Transfer::zod_schema(),
            if cfg!(feature = "serde") {
                "z.preprocess(unwrapped, z.union([z.literal(\"Held\"), "
            } else {
                "z.preprocess(unwrapped, z.discriminatedUnion("
            },
        ),
        (
            Reference::zod_schema(),
            if cfg!(feature = "serde") {
                "z.preprocess(unwrapped, z.union([z.strictObject({ name: z.string(), }), \
                 z.number().int().min(0).max(4294967295)]))"
            } else {
                "z.preprocess(unwrapped, z.discriminatedUnion("
            },
        ),
    ] {
        assert!(
            bound(&schema).starts_with(opens),
            "no `{opens}` opens: {schema}"
        );
    }
}

/// The first function written is the outermost, so it is the first one called.
#[test]
fn several_functions_nest_in_the_order_written() {
    let schema = Wrapped::zod_schema();
    assert!(
        bound(&schema)
            .starts_with("z.preprocess(unwrapped, z.preprocess(withoutFragile, z.strictObject({\n"),
        "got: {schema}"
    );
    assert!(schema.contains("\n})))"), "got: {schema}");
}

/// A brand's wrap is inside `.brand()`, and every wrap inside the `.meta` its binding carries.
#[test]
fn the_wrap_is_inside_the_brand_and_the_meta() {
    let brand = Label::zod_schema();
    let branded = if cfg!(feature = "typescript") {
        "z.preprocess(unwrapped, z.preprocess(trimmed, z.string())).brand<\"Label\">().meta({\n"
    } else {
        "z.preprocess(unwrapped, z.preprocess(trimmed, z.string())).brand().meta({\n"
    };
    assert!(bound(&brand).starts_with(branded), "got: {brand}");
    let plain = Grade::zod_schema();
    assert!(
        bound(&plain)
            .starts_with("z.preprocess(unwrapped, z.enum([\"Express\", \"Standard\"])).meta({\n"),
        "got: {plain}"
    );
    for schema in [Parcel::zod_schema(), Stack::zod_schema()] {
        let (wrapped, carried) = schema.split_once(".meta({").unwrap();
        assert!(wrapped.contains("z.preprocess("), "got: {schema}");
        assert!(!carried.contains("z.preprocess("), "got: {schema}");
    }
}

/// The wrap is written in the builder of a type with a parameter, so every filling carries it.
#[test]
fn the_wrap_is_inside_the_builder_of_a_generic_type() {
    let schema = Crate::<String>::zod_schema();
    assert!(
        schema.contains(
            ") =>\n  z.preprocess(unwrapped, z.strictObject({\n  codes: z.array(codeType),\n  \
             label: z.string(),\n}));\n"
        ),
        "got: {schema}"
    );
    assert_eq!(schema.matches("z.preprocess(").count(), 1, "got: {schema}");
}

/// A check is written on the type's own expression, and the wrap around both.
#[test]
fn the_wrap_is_around_the_expression_and_its_check() {
    let schema = Weighed::zod_schema();
    assert!(
        bound(&schema).starts_with(
            "z.preprocess(unwrapped, z.strictObject({\n  grams: z.number().int().min(0).max(4294967295),\n}).check((ctx) \
             => {\n"
        ),
        "got: {schema}"
    );
    let (wrapped, _carried) = schema.split_once(".meta({").unwrap();
    assert!(
        wrapped.contains("withinLimit(own.value, own);"),
        "got: {schema}"
    );
    assert!(wrapped.contains("\n}))"), "got: {schema}");
}

/// A field's wrap is the text it was before a type took the key.
#[test]
fn the_wrap_of_a_field_is_written_as_it_was() {
    let schema = Parcel::zod_schema();
    assert!(
        schema.contains(
            "  label: z.preprocess(trimmed, z.string().min(1, { error: (issue) => `too short: \
             minimum length is 1, got ${String(issue.input).length}` })),\n"
        ),
        "got: {schema}"
    );
}

/// A holder names the binding, which is the one schema that calls the function, or the members the
/// binding publishes with the function written around each.
#[test]
fn a_holder_reads_the_binding_that_calls_the_function() {
    let manifest = Manifest::zod_schema();
    assert!(
        manifest.contains("  parcel: Parcel$Schema,\n"),
        "got: {manifest}"
    );
    assert!(!manifest.contains("z.preprocess("), "got: {manifest}");

    // A choice flattened into its holder is joined by the members its binding publishes, one
    // branch each, never by text copied from the choice.
    let routed = Routed::zod_schema();
    if cfg!(feature = "serde") {
        for member in ["Carrier$Members[0]", "Carrier$Members[1]"] {
            assert!(
                routed.contains(&format!("Routed$OwnSchema.and(z.lazy(() => {member}))")),
                "got: {routed}"
            );
        }
        assert!(!routed.contains("z.preprocess("), "got: {routed}");
        let carrier = Carrier::zod_schema();
        assert!(
            carrier.contains("export const Carrier$Members = [\n  z.preprocess(trimmedPlate, z."),
            "got: {carrier}"
        );
    }
    assert!(!routed.contains("plate"), "got: {routed}");
}
