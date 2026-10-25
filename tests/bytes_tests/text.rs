//! What the TypeScript type and the Zod schema publish for a `Vec<u8>` and for each sequence that
//! is not one.

#[cfg(feature = "zod")]
use super::models::BytesCounted;
use super::models::{
    BytesBox, BytesEither, BytesHolder, BytesImage, BytesLookalikes, BytesOptional,
    bytes_payload_schema,
};

/// The Zod schema of a `Vec<u8>`: the array serde writes, or a `Uint8Array`, read as a `Uint8Array`.
#[cfg(feature = "zod")]
const BYTES: &str = "z.codec(z.union([z.instanceof(Uint8Array), z.array(z.number().int().min(0).max(255))]), z.instanceof(Uint8Array), { decode: (value) => Uint8Array.from(value), encode: (value) => Array.from(value) })";

#[test]
fn a_vec_of_u8_is_a_uint8array() {
    let published = BytesImage::ts_definition();
    assert!(published.contains("  data: Uint8Array;"), "{published}");
    assert!(published.contains("  contentType: string;"), "{published}");
}

#[test]
fn a_vec_of_u8_is_a_uint8array_in_every_place_a_member_holds_one() {
    let published = BytesHolder::ts_definition();
    for line in [
        "  aliased: BytesPayloadType;",
        "  data: Uint8Array;",
        "  filled: BytesBox<Uint8Array>;",
        "  keyed: Partial<Record<string, Uint8Array>>;",
        "  outcome: { Ok: Uint8Array } | { Err: string };",
        "  paired: [Uint8Array, string];",
        "  rows: Array<Uint8Array>;",
    ] {
        assert!(published.contains(line), "{line}\n{published}");
    }
    let alias = bytes_payload_schema::Schema::ts_definition();
    assert!(
        alias.contains("export type BytesPayloadType = Uint8Array;"),
        "{alias}"
    );
    assert!(
        BytesEither::ts_definition().contains("export type BytesEither = Uint8Array | string;"),
        "{}",
        BytesEither::ts_definition()
    );
    assert!(BytesBox::<String>::ts_definition().contains("  held: Held;"));
    assert!(BytesOptional::ts_definition().contains("  maybe: Uint8Array | undefined;"));
}

#[test]
fn every_other_sequence_stays_an_array() {
    let published = BytesLookalikes::ts_definition();
    for line in [
        "  boxed: Array<number>;",
        "  digest: Array<number>;",
        "  distinct: Array<number>;",
        "  gaps: Array<number | null>;",
        "  queued: Array<number>;",
        "  sevens: Array<7>;",
        "  signed: Array<number>;",
        "  wide: Array<number>;",
    ] {
        assert!(published.contains(line), "{line}\n{published}");
    }
    assert!(!published.contains("Uint8Array"), "{published}");
}

#[cfg(feature = "zod")]
#[test]
fn the_zod_schema_of_a_vec_of_u8_reads_the_array_and_answers_the_uint8array() {
    let schema = BytesImage::zod_schema();
    assert!(schema.contains(&format!("  data: {BYTES},")), "{schema}");
}

#[cfg(feature = "zod")]
#[test]
fn the_zod_schema_of_a_vec_of_u8_holds_it_wherever_a_member_holds_one() {
    let schema = BytesHolder::zod_schema();
    for line in [
        format!("  data: {BYTES},"),
        format!("  filled: BytesBox$SchemaFactory({BYTES}),"),
        format!("  keyed: z.record(z.string(), {BYTES}),"),
        format!("  rows: z.array({BYTES}),"),
        format!("  paired: z.tuple([{BYTES}, z.string()]),"),
        format!("z.strictObject({{ Ok: {BYTES} }})"),
    ] {
        assert!(schema.contains(&line), "{line}\n{schema}");
    }
    assert!(BytesEither::zod_schema().contains(&format!("z.union([{BYTES}, z.string()])")));
    assert!(BytesOptional::zod_schema().contains(&format!(
        "maybe: z.union([z.null().transform(() => undefined), {BYTES}, z.undefined()]).prefault(undefined),"
    )));
}

#[cfg(feature = "zod")]
#[test]
fn the_zod_schema_of_every_other_sequence_stays_an_array() {
    let schema = BytesLookalikes::zod_schema();
    let item = "z.number().int().min(0).max(255)";
    for line in [
        format!("  boxed: z.array({item}),"),
        format!("  digest: z.array({item}).length(4),"),
        format!("  gaps: z.array(z.nullable({item})),"),
        format!("  queued: z.array({item}),"),
        "  signed: z.array(z.number().int().min(-128).max(127)),".to_owned(),
        "  wide: z.array(z.number().int().min(0).max(65535)),".to_owned(),
    ] {
        assert!(schema.contains(&line), "{line}\n{schema}");
    }
    assert!(!schema.contains("Uint8Array"), "{schema}");
}

#[cfg(feature = "zod")]
#[test]
fn a_count_of_a_vec_of_u8_holds_the_uint8array_and_the_array_with_one_sentence() {
    let schema = BytesCounted::zod_schema();
    let short = "too short: minimum length is 2, got ${";
    assert!(
        schema.contains(&format!(
            "z.instanceof(Uint8Array).refine((value) => value.length >= 2, {{ error: (issue) => `{short}(Array.isArray(issue.input) || issue.input instanceof Uint8Array) ? issue.input.length : 0}}` }})"
        )),
        "{schema}"
    );
    assert!(
        schema.contains(&format!(
            "z.array(z.number().int().min(0).max(255)).min(2, {{ error: (issue) => `{short}Array.isArray(issue.input) ? issue.input.length : 0}}` }})"
        )),
        "{schema}"
    );
    assert_eq!(
        schema
            .matches(".refine((value) => value.length <= 4")
            .count(),
        2,
        "{schema}"
    );
}
