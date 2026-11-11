//! The Zod text a `preprocess` on a type writes: the wrap around each shape's own expression.

#![cfg(feature = "zod")]

use super::models::{
    Carrier, ClosedDock, Crate, Grade, Handover, Label, Manifest, Parcel, Pinned, Reference,
    Routed, Shipment, Stack, Transfer, Weighed, Wrapped,
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
            "z.preprocess(function paired(held) { if (held instanceof Number) return [held, held.valueOf()]; if (typeof held !== \"object\" || held === null || (!Array.isArray(held) && Object.getPrototypeOf(held) !== Object.prototype)) return [held, held]; const copied = Array.isArray(held) ? [...held] : { ...held }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, paired(item)[1]); return [held, copied]; }, z.preprocess(function (pair, ctx) { if (!Array.isArray(pair)) return pair; const answered = Reflect.apply(unwrapped, undefined, [pair[1], ctx]); return answered instanceof Promise ? answered.then((plain) => [pair[0], plain]) : [pair[0], answered]; }, z.preprocess(function reboxed(pair) { if (!Array.isArray(pair)) return pair; const original = pair[0]; const out = pair[1]; if (original instanceof Number) return Object.is(out, original.valueOf()) ? original : out; if (typeof out !== \"object\" || out === null || typeof original !== \"object\" || original === null || Array.isArray(out) !== Array.isArray(original) || (!Array.isArray(out) && Object.getPrototypeOf(out) !== Object.prototype)) return out; const copied = Array.isArray(out) ? [...out] : { ...out }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, reboxed([Reflect.get(original, key), item])); return copied; }, z.strictObject({\n})))",
        ),
        (
            Stack::zod_schema(),
            "z.preprocess(function paired(held) { if (held instanceof Number) return [held, held.valueOf()]; if (typeof held !== \"object\" || held === null || (!Array.isArray(held) && Object.getPrototypeOf(held) !== Object.prototype)) return [held, held]; const copied = Array.isArray(held) ? [...held] : { ...held }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, paired(item)[1]); return [held, copied]; }, z.preprocess(function (pair, ctx) { if (!Array.isArray(pair)) return pair; const answered = Reflect.apply(unwrapped, undefined, [pair[1], ctx]); return answered instanceof Promise ? answered.then((plain) => [pair[0], plain]) : [pair[0], answered]; }, z.preprocess(function reboxed(pair) { if (!Array.isArray(pair)) return pair; const original = pair[0]; const out = pair[1]; if (original instanceof Number) return Object.is(out, original.valueOf()) ? original : out; if (typeof out !== \"object\" || out === null || typeof original !== \"object\" || original === null || Array.isArray(out) !== Array.isArray(original) || (!Array.isArray(out) && Object.getPrototypeOf(out) !== Object.prototype)) return out; const copied = Array.isArray(out) ? [...out] : { ...out }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, reboxed([Reflect.get(original, key), item])); return copied; }, z.tuple([z.array(z.string()), z.number().int().min(0).max(4294967295)])))",
        ),
        (
            Parcel::zod_schema(),
            "z.preprocess(function paired(held) { if (held instanceof Number) return [held, held.valueOf()]; if (typeof held !== \"object\" || held === null || (!Array.isArray(held) && Object.getPrototypeOf(held) !== Object.prototype)) return [held, held]; const copied = Array.isArray(held) ? [...held] : { ...held }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, paired(item)[1]); return [held, copied]; }, z.preprocess(function (pair, ctx) { if (!Array.isArray(pair)) return pair; const answered = Reflect.apply(withoutFragile, undefined, [pair[1], ctx]); return answered instanceof Promise ? answered.then((plain) => [pair[0], plain]) : [pair[0], answered]; }, z.preprocess(function reboxed(pair) { if (!Array.isArray(pair)) return pair; const original = pair[0]; const out = pair[1]; if (original instanceof Number) return Object.is(out, original.valueOf()) ? original : out; if (typeof out !== \"object\" || out === null || typeof original !== \"object\" || original === null || Array.isArray(out) !== Array.isArray(original) || (!Array.isArray(out) && Object.getPrototypeOf(out) !== Object.prototype)) return out; const copied = Array.isArray(out) ? [...out] : { ...out }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, reboxed([Reflect.get(original, key), item])); return copied; }, z.strictObject({\n",
        ),
        (
            Shipment::zod_schema(),
            "z.preprocess(function paired(held) { if (held instanceof Number) return [held, held.valueOf()]; if (typeof held !== \"object\" || held === null || (!Array.isArray(held) && Object.getPrototypeOf(held) !== Object.prototype)) return [held, held]; const copied = Array.isArray(held) ? [...held] : { ...held }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, paired(item)[1]); return [held, copied]; }, z.preprocess(function (pair, ctx) { if (!Array.isArray(pair)) return pair; const answered = Reflect.apply(unwrapped, undefined, [pair[1], ctx]); return answered instanceof Promise ? answered.then((plain) => [pair[0], plain]) : [pair[0], answered]; }, z.preprocess(function reboxed(pair) { if (!Array.isArray(pair)) return pair; const original = pair[0]; const out = pair[1]; if (original instanceof Number) return Object.is(out, original.valueOf()) ? original : out; if (typeof out !== \"object\" || out === null || typeof original !== \"object\" || original === null || Array.isArray(out) !== Array.isArray(original) || (!Array.isArray(out) && Object.getPrototypeOf(out) !== Object.prototype)) return out; const copied = Array.isArray(out) ? [...out] : { ...out }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, reboxed([Reflect.get(original, key), item])); return copied; }, z.discriminatedUnion(",
        ),
        (
            Handover::zod_schema(),
            "z.preprocess(function paired(held) { if (held instanceof Number) return [held, held.valueOf()]; if (typeof held !== \"object\" || held === null || (!Array.isArray(held) && Object.getPrototypeOf(held) !== Object.prototype)) return [held, held]; const copied = Array.isArray(held) ? [...held] : { ...held }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, paired(item)[1]); return [held, copied]; }, z.preprocess(function (pair, ctx) { if (!Array.isArray(pair)) return pair; const answered = Reflect.apply(unwrapped, undefined, [pair[1], ctx]); return answered instanceof Promise ? answered.then((plain) => [pair[0], plain]) : [pair[0], answered]; }, z.preprocess(function reboxed(pair) { if (!Array.isArray(pair)) return pair; const original = pair[0]; const out = pair[1]; if (original instanceof Number) return Object.is(out, original.valueOf()) ? original : out; if (typeof out !== \"object\" || out === null || typeof original !== \"object\" || original === null || Array.isArray(out) !== Array.isArray(original) || (!Array.isArray(out) && Object.getPrototypeOf(out) !== Object.prototype)) return out; const copied = Array.isArray(out) ? [...out] : { ...out }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, reboxed([Reflect.get(original, key), item])); return copied; }, z.discriminatedUnion(",
        ),
        (
            Transfer::zod_schema(),
            if cfg!(feature = "serde") {
                "z.preprocess(function paired(held) { if (held instanceof Number) return [held, held.valueOf()]; if (typeof held !== \"object\" || held === null || (!Array.isArray(held) && Object.getPrototypeOf(held) !== Object.prototype)) return [held, held]; const copied = Array.isArray(held) ? [...held] : { ...held }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, paired(item)[1]); return [held, copied]; }, z.preprocess(function (pair, ctx) { if (!Array.isArray(pair)) return pair; const answered = Reflect.apply(unwrapped, undefined, [pair[1], ctx]); return answered instanceof Promise ? answered.then((plain) => [pair[0], plain]) : [pair[0], answered]; }, z.preprocess(function reboxed(pair) { if (!Array.isArray(pair)) return pair; const original = pair[0]; const out = pair[1]; if (original instanceof Number) return Object.is(out, original.valueOf()) ? original : out; if (typeof out !== \"object\" || out === null || typeof original !== \"object\" || original === null || Array.isArray(out) !== Array.isArray(original) || (!Array.isArray(out) && Object.getPrototypeOf(out) !== Object.prototype)) return out; const copied = Array.isArray(out) ? [...out] : { ...out }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, reboxed([Reflect.get(original, key), item])); return copied; }, z.union([z.literal(\"Held\"), "
            } else {
                "z.preprocess(function paired(held) { if (held instanceof Number) return [held, held.valueOf()]; if (typeof held !== \"object\" || held === null || (!Array.isArray(held) && Object.getPrototypeOf(held) !== Object.prototype)) return [held, held]; const copied = Array.isArray(held) ? [...held] : { ...held }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, paired(item)[1]); return [held, copied]; }, z.preprocess(function (pair, ctx) { if (!Array.isArray(pair)) return pair; const answered = Reflect.apply(unwrapped, undefined, [pair[1], ctx]); return answered instanceof Promise ? answered.then((plain) => [pair[0], plain]) : [pair[0], answered]; }, z.preprocess(function reboxed(pair) { if (!Array.isArray(pair)) return pair; const original = pair[0]; const out = pair[1]; if (original instanceof Number) return Object.is(out, original.valueOf()) ? original : out; if (typeof out !== \"object\" || out === null || typeof original !== \"object\" || original === null || Array.isArray(out) !== Array.isArray(original) || (!Array.isArray(out) && Object.getPrototypeOf(out) !== Object.prototype)) return out; const copied = Array.isArray(out) ? [...out] : { ...out }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, reboxed([Reflect.get(original, key), item])); return copied; }, z.discriminatedUnion("
            },
        ),
        (
            Reference::zod_schema(),
            if cfg!(feature = "serde") {
                "z.preprocess(function paired(held) { if (held instanceof Number) return [held, held.valueOf()]; if (typeof held !== \"object\" || held === null || (!Array.isArray(held) && Object.getPrototypeOf(held) !== Object.prototype)) return [held, held]; const copied = Array.isArray(held) ? [...held] : { ...held }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, paired(item)[1]); return [held, copied]; }, z.preprocess(function (pair, ctx) { if (!Array.isArray(pair)) return pair; const answered = Reflect.apply(unwrapped, undefined, [pair[1], ctx]); return answered instanceof Promise ? answered.then((plain) => [pair[0], plain]) : [pair[0], answered]; }, z.preprocess(function reboxed(pair) { if (!Array.isArray(pair)) return pair; const original = pair[0]; const out = pair[1]; if (original instanceof Number) return Object.is(out, original.valueOf()) ? original : out; if (typeof out !== \"object\" || out === null || typeof original !== \"object\" || original === null || Array.isArray(out) !== Array.isArray(original) || (!Array.isArray(out) && Object.getPrototypeOf(out) !== Object.prototype)) return out; const copied = Array.isArray(out) ? [...out] : { ...out }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, reboxed([Reflect.get(original, key), item])); return copied; }, z.union([z.strictObject({ name: z.string(), }), \
                 z.number().int().min(0).max(4294967295)])))"
            } else {
                "z.preprocess(function paired(held) { if (held instanceof Number) return [held, held.valueOf()]; if (typeof held !== \"object\" || held === null || (!Array.isArray(held) && Object.getPrototypeOf(held) !== Object.prototype)) return [held, held]; const copied = Array.isArray(held) ? [...held] : { ...held }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, paired(item)[1]); return [held, copied]; }, z.preprocess(function (pair, ctx) { if (!Array.isArray(pair)) return pair; const answered = Reflect.apply(unwrapped, undefined, [pair[1], ctx]); return answered instanceof Promise ? answered.then((plain) => [pair[0], plain]) : [pair[0], answered]; }, z.preprocess(function reboxed(pair) { if (!Array.isArray(pair)) return pair; const original = pair[0]; const out = pair[1]; if (original instanceof Number) return Object.is(out, original.valueOf()) ? original : out; if (typeof out !== \"object\" || out === null || typeof original !== \"object\" || original === null || Array.isArray(out) !== Array.isArray(original) || (!Array.isArray(out) && Object.getPrototypeOf(out) !== Object.prototype)) return out; const copied = Array.isArray(out) ? [...out] : { ...out }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, reboxed([Reflect.get(original, key), item])); return copied; }, z.discriminatedUnion("
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
            .starts_with("z.preprocess(function paired(held) { if (held instanceof Number) return [held, held.valueOf()]; if (typeof held !== \"object\" || held === null || (!Array.isArray(held) && Object.getPrototypeOf(held) !== Object.prototype)) return [held, held]; const copied = Array.isArray(held) ? [...held] : { ...held }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, paired(item)[1]); return [held, copied]; }, z.preprocess(function (pair, ctx) { if (!Array.isArray(pair)) return pair; const answered = Reflect.apply(unwrapped, undefined, [pair[1], ctx]); return answered instanceof Promise ? answered.then((plain) => [pair[0], plain]) : [pair[0], answered]; }, z.preprocess(function (pair, ctx) { if (!Array.isArray(pair)) return pair; const answered = Reflect.apply(withoutFragile, undefined, [pair[1], ctx]); return answered instanceof Promise ? answered.then((plain) => [pair[0], plain]) : [pair[0], answered]; }, z.preprocess(function reboxed(pair) { if (!Array.isArray(pair)) return pair; const original = pair[0]; const out = pair[1]; if (original instanceof Number) return Object.is(out, original.valueOf()) ? original : out; if (typeof out !== \"object\" || out === null || typeof original !== \"object\" || original === null || Array.isArray(out) !== Array.isArray(original) || (!Array.isArray(out) && Object.getPrototypeOf(out) !== Object.prototype)) return out; const copied = Array.isArray(out) ? [...out] : { ...out }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, reboxed([Reflect.get(original, key), item])); return copied; }, z.strictObject({\n"),
        "got: {schema}"
    );
    assert!(schema.contains("\n}))))"), "got: {schema}");
}

/// A brand's wrap is inside `.brand()`, and every wrap inside the `.meta` its binding carries.
#[test]
fn the_wrap_is_inside_the_brand_and_the_meta() {
    let brand = Label::zod_schema();
    let branded = if cfg!(feature = "typescript") {
        "z.preprocess(function paired(held) { if (held instanceof Number) return [held, held.valueOf()]; if (typeof held !== \"object\" || held === null || (!Array.isArray(held) && Object.getPrototypeOf(held) !== Object.prototype)) return [held, held]; const copied = Array.isArray(held) ? [...held] : { ...held }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, paired(item)[1]); return [held, copied]; }, z.preprocess(function (pair, ctx) { if (!Array.isArray(pair)) return pair; const answered = Reflect.apply(unwrapped, undefined, [pair[1], ctx]); return answered instanceof Promise ? answered.then((plain) => [pair[0], plain]) : [pair[0], answered]; }, z.preprocess(function (pair, ctx) { if (!Array.isArray(pair)) return pair; const answered = Reflect.apply(trimmed, undefined, [pair[1], ctx]); return answered instanceof Promise ? answered.then((plain) => [pair[0], plain]) : [pair[0], answered]; }, z.preprocess(function reboxed(pair) { if (!Array.isArray(pair)) return pair; const original = pair[0]; const out = pair[1]; if (original instanceof Number) return Object.is(out, original.valueOf()) ? original : out; if (typeof out !== \"object\" || out === null || typeof original !== \"object\" || original === null || Array.isArray(out) !== Array.isArray(original) || (!Array.isArray(out) && Object.getPrototypeOf(out) !== Object.prototype)) return out; const copied = Array.isArray(out) ? [...out] : { ...out }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, reboxed([Reflect.get(original, key), item])); return copied; }, z.string())))).brand<\"Label\">().meta({\n"
    } else {
        "z.preprocess(function paired(held) { if (held instanceof Number) return [held, held.valueOf()]; if (typeof held !== \"object\" || held === null || (!Array.isArray(held) && Object.getPrototypeOf(held) !== Object.prototype)) return [held, held]; const copied = Array.isArray(held) ? [...held] : { ...held }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, paired(item)[1]); return [held, copied]; }, z.preprocess(function (pair, ctx) { if (!Array.isArray(pair)) return pair; const answered = Reflect.apply(unwrapped, undefined, [pair[1], ctx]); return answered instanceof Promise ? answered.then((plain) => [pair[0], plain]) : [pair[0], answered]; }, z.preprocess(function (pair, ctx) { if (!Array.isArray(pair)) return pair; const answered = Reflect.apply(trimmed, undefined, [pair[1], ctx]); return answered instanceof Promise ? answered.then((plain) => [pair[0], plain]) : [pair[0], answered]; }, z.preprocess(function reboxed(pair) { if (!Array.isArray(pair)) return pair; const original = pair[0]; const out = pair[1]; if (original instanceof Number) return Object.is(out, original.valueOf()) ? original : out; if (typeof out !== \"object\" || out === null || typeof original !== \"object\" || original === null || Array.isArray(out) !== Array.isArray(original) || (!Array.isArray(out) && Object.getPrototypeOf(out) !== Object.prototype)) return out; const copied = Array.isArray(out) ? [...out] : { ...out }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, reboxed([Reflect.get(original, key), item])); return copied; }, z.string())))).brand().meta({\n"
    };
    assert!(bound(&brand).starts_with(branded), "got: {brand}");
    let plain = Grade::zod_schema();
    assert!(
        bound(&plain)
            .starts_with("z.preprocess(function paired(held) { if (held instanceof Number) return [held, held.valueOf()]; if (typeof held !== \"object\" || held === null || (!Array.isArray(held) && Object.getPrototypeOf(held) !== Object.prototype)) return [held, held]; const copied = Array.isArray(held) ? [...held] : { ...held }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, paired(item)[1]); return [held, copied]; }, z.preprocess(function (pair, ctx) { if (!Array.isArray(pair)) return pair; const answered = Reflect.apply(unwrapped, undefined, [pair[1], ctx]); return answered instanceof Promise ? answered.then((plain) => [pair[0], plain]) : [pair[0], answered]; }, z.preprocess(function reboxed(pair) { if (!Array.isArray(pair)) return pair; const original = pair[0]; const out = pair[1]; if (original instanceof Number) return Object.is(out, original.valueOf()) ? original : out; if (typeof out !== \"object\" || out === null || typeof original !== \"object\" || original === null || Array.isArray(out) !== Array.isArray(original) || (!Array.isArray(out) && Object.getPrototypeOf(out) !== Object.prototype)) return out; const copied = Array.isArray(out) ? [...out] : { ...out }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, reboxed([Reflect.get(original, key), item])); return copied; }, z.enum([\"Express\", \"Standard\"])))).meta({\n"),
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
            ") =>\n  z.preprocess(function paired(held) { if (held instanceof Number) return [held, held.valueOf()]; if (typeof held !== \"object\" || held === null || (!Array.isArray(held) && Object.getPrototypeOf(held) !== Object.prototype)) return [held, held]; const copied = Array.isArray(held) ? [...held] : { ...held }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, paired(item)[1]); return [held, copied]; }, z.preprocess(function (pair, ctx) { if (!Array.isArray(pair)) return pair; const answered = Reflect.apply(unwrapped, undefined, [pair[1], ctx]); return answered instanceof Promise ? answered.then((plain) => [pair[0], plain]) : [pair[0], answered]; }, z.preprocess(function reboxed(pair) { if (!Array.isArray(pair)) return pair; const original = pair[0]; const out = pair[1]; if (original instanceof Number) return Object.is(out, original.valueOf()) ? original : out; if (typeof out !== \"object\" || out === null || typeof original !== \"object\" || original === null || Array.isArray(out) !== Array.isArray(original) || (!Array.isArray(out) && Object.getPrototypeOf(out) !== Object.prototype)) return out; const copied = Array.isArray(out) ? [...out] : { ...out }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, reboxed([Reflect.get(original, key), item])); return copied; }, z.strictObject({\n  codes: z.array(codeType),\n  \
             label: z.string(),\n}))));\n"
        ),
        "got: {schema}"
    );
    assert_eq!(
        schema.matches("Reflect.apply(unwrapped, ").count(),
        1,
        "got: {schema}"
    );
}

/// A check is written on the type's own expression, and the wrap around both.
#[test]
fn the_wrap_is_around_the_expression_and_its_check() {
    let schema = Weighed::zod_schema();
    assert!(
        bound(&schema).starts_with(
            "z.preprocess(function paired(held) { if (held instanceof Number) return [held, held.valueOf()]; if (typeof held !== \"object\" || held === null || (!Array.isArray(held) && Object.getPrototypeOf(held) !== Object.prototype)) return [held, held]; const copied = Array.isArray(held) ? [...held] : { ...held }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, paired(item)[1]); return [held, copied]; }, z.preprocess(function (pair, ctx) { if (!Array.isArray(pair)) return pair; const answered = Reflect.apply(unwrapped, undefined, [pair[1], ctx]); return answered instanceof Promise ? answered.then((plain) => [pair[0], plain]) : [pair[0], answered]; }, z.preprocess(function reboxed(pair) { if (!Array.isArray(pair)) return pair; const original = pair[0]; const out = pair[1]; if (original instanceof Number) return Object.is(out, original.valueOf()) ? original : out; if (typeof out !== \"object\" || out === null || typeof original !== \"object\" || original === null || Array.isArray(out) !== Array.isArray(original) || (!Array.isArray(out) && Object.getPrototypeOf(out) !== Object.prototype)) return out; const copied = Array.isArray(out) ? [...out] : { ...out }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, reboxed([Reflect.get(original, key), item])); return copied; }, z.strictObject({\n  grams: z.number().int().min(0).max(4294967295),\n}).check((ctx) \
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
            "  label: z.preprocess(function paired(held) { if (held instanceof Number) return [held, held.valueOf()]; if (typeof held !== \"object\" || held === null || (!Array.isArray(held) && Object.getPrototypeOf(held) !== Object.prototype)) return [held, held]; const copied = Array.isArray(held) ? [...held] : { ...held }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, paired(item)[1]); return [held, copied]; }, z.preprocess(function (pair, ctx) { if (!Array.isArray(pair)) return pair; const answered = Reflect.apply(trimmed, undefined, [pair[1], ctx]); return answered instanceof Promise ? answered.then((plain) => [pair[0], plain]) : [pair[0], answered]; }, z.preprocess(function reboxed(pair) { if (!Array.isArray(pair)) return pair; const original = pair[0]; const out = pair[1]; if (original instanceof Number) return Object.is(out, original.valueOf()) ? original : out; if (typeof out !== \"object\" || out === null || typeof original !== \"object\" || original === null || Array.isArray(out) !== Array.isArray(original) || (!Array.isArray(out) && Object.getPrototypeOf(out) !== Object.prototype)) return out; const copied = Array.isArray(out) ? [...out] : { ...out }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, reboxed([Reflect.get(original, key), item])); return copied; }, z.string().min(1, { error: (issue) => `too short: \
             minimum length is 1, got ${String(issue.input).length}` })))),\n"
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
    // branch each, never by text copied from the choice, and each member is handed the keys the
    // holder's own members do not take.
    let routed = Routed::zod_schema();
    if cfg!(feature = "serde") {
        for member in ["Carrier$Members[0]", "Carrier$Members[1]"] {
            assert!(
                routed.contains("Routed$OwnSchema.and(z.lazy(() => z.preprocess((held) => ")
                    && routed
                        .contains(&format!(".includes(key))) : held, {member})).superRefine(")),
                "got: {routed}"
            );
        }
        assert!(
            routed.contains("![\"dock\"].includes(key)"),
            "got: {routed}"
        );
        let carrier = Carrier::zod_schema();
        assert!(
            carrier.contains("export const Carrier$Members = [\n  z.preprocess(function paired(held) { if (held instanceof Number) return [held, held.valueOf()]; if (typeof held !== \"object\" || held === null || (!Array.isArray(held) && Object.getPrototypeOf(held) !== Object.prototype)) return [held, held]; const copied = Array.isArray(held) ? [...held] : { ...held }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, paired(item)[1]); return [held, copied]; }, z.preprocess(function (pair, ctx) { if (!Array.isArray(pair)) return pair; const answered = Reflect.apply(trimmedPlate, undefined, [pair[1], ctx]); return answered instanceof Promise ? answered.then((plain) => [pair[0], plain]) : [pair[0], answered]; }, z.preprocess(function reboxed(pair) { if (!Array.isArray(pair)) return pair; const original = pair[0]; const out = pair[1]; if (original instanceof Number) return Object.is(out, original.valueOf()) ? original : out; if (typeof out !== \"object\" || out === null || typeof original !== \"object\" || original === null || Array.isArray(out) !== Array.isArray(original) || (!Array.isArray(out) && Object.getPrototypeOf(out) !== Object.prototype)) return out; const copied = Array.isArray(out) ? [...out] : { ...out }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, reboxed([Reflect.get(original, key), item])); return copied; }, z."),
            "got: {carrier}"
        );
    }
    assert!(!routed.contains("plate"), "got: {routed}");
}

/// Each flattened type that names a function is handed the payload without the keys its holder's
/// own members take, and the holder's own members alone are left out of it.
#[cfg(feature = "serde")]
#[test]
fn a_flattened_type_is_handed_the_payload_without_its_holders_keys() {
    let pinned = Pinned::zod_schema();
    assert!(
        pinned.contains(
            ".and(z.lazy(() => z.preprocess((held) => typeof held === \"object\" && held !== null \
             && !Array.isArray(held) ? Object.fromEntries(Object.entries(held).filter(([key]) => \
             ![\"dock\"].includes(key))) : held, Tagged$Schema)).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); return !absent && [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) ? { ...issue, path: [] } : issue; })); }, { when: () => true }))"
        ),
        "got: {pinned}"
    );
    let manifest = Manifest::zod_schema();
    assert!(!manifest.contains("Object.entries"), "got: {manifest}");
}
