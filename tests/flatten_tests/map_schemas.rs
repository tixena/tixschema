//! The Zod schema of a struct a map is flattened into as the only member of its object: the map's
//! record takes every key.

#[cfg(feature = "zod")]
use super::maps::{Bilge, Coaming, Thwart, Transom};
use super::maps::{assert_serde_reads_what_zod_reads, assert_zod_refuses_each_key_serde_drops};

#[test]
fn serde_reads_each_payload_zod_reads() {
    assert_serde_reads_what_zod_reads();
}

/// Zod refuses a payload of an optional struct exactly where serde reads the struct as `None`
/// and drops a key.
#[test]
fn zod_refuses_each_key_serde_drops_from_an_optional_struct() {
    assert_zod_refuses_each_key_serde_drops();
}

/// The record of a map alone in its object takes every key.
#[cfg(feature = "zod")]
#[test]
fn a_map_alone_writes_a_record_of_every_key() {
    #[cfg(feature = "typescript")]
    const EXPECTED: &str = "const Thwart$RawSchema = z.strictObject({\n}).and(z.lazy(() => z.record(z.string(), z.number().int().min(0).max(255))).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); return !absent && [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) ? { ...issue, path: [] } : issue; })); }, { when: () => true }));\n\nexport const Thwart$Schema: ZodType<Thwart> = Thwart$RawSchema.meta({\n  examples: []\n});";
    #[cfg(not(feature = "typescript"))]
    const EXPECTED: &str = "export const Thwart$Schema = z.strictObject({\n}).and(z.lazy(() => z.record(z.string(), z.number().int().min(0).max(255))).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === \"object\" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === \"object\" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); return !absent && [\"invalid_type\", \"invalid_union\", \"invalid_value\"].includes(issue.code) ? { ...issue, path: [] } : issue; })); }, { when: () => true })).meta({\n  examples: []\n});";

    assert_eq!(Thwart::zod_schema(), EXPECTED);
}

/// The record of a map in a variant takes every key, whatever the variant is written under.
#[cfg(feature = "zod")]
#[test]
fn a_map_in_a_variant_writes_a_record_of_every_key() {
    for zod in [
        Coaming::zod_schema(),
        Transom::zod_schema(),
        Bilge::zod_schema(),
    ] {
        assert!(zod.contains("z.record(z.string(), "), "{zod}");
        assert!(!zod.contains(".refine((key)"), "{zod}");
    }
}
