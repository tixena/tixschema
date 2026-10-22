//! The Zod schema of a struct a map is flattened into as the only member of its object: the map's
//! record takes every key. An optional map's absence takes every key too, with any value.

#[cfg(feature = "zod")]
use super::maps::{Bilge, Coaming, Locker, Sheave, Stanchion, Thole, Thwart, Transom};
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
    const EXPECTED: &str = "const Thwart$RawSchema = z.strictObject({\n}).and(z.lazy(() => z.record(z.string(), z.number().int().min(0).max(255))));\n\nexport const Thwart$Schema: ZodType<Thwart> = Thwart$RawSchema.meta({\n  examples: []\n});";
    #[cfg(not(feature = "typescript"))]
    const EXPECTED: &str = "export const Thwart$Schema = z.strictObject({\n}).and(z.lazy(() => z.record(z.string(), z.number().int().min(0).max(255)))).meta({\n  examples: []\n});";

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

/// An optional map is the map, or, where serde reads it as `None`, every key with any value and
/// none of them in the value read.
#[cfg(feature = "zod")]
#[test]
fn an_optional_map_is_the_map_or_every_key_dropped() {
    #[cfg(feature = "typescript")]
    const EXPECTED: &str = "const Locker$OwnSchema = z.strictObject({\n});\n\nconst Locker$RawSchema = z.union([\n  Locker$OwnSchema.and(z.lazy(() => z.record(z.string(), z.number().int().min(0).max(255)))),\n  Locker$OwnSchema.and(z.record(z.string(), z.unknown()).transform(() => ({}))),\n]);\n\nexport const Locker$Schema: ZodType<Locker> = Locker$RawSchema.meta({\n  examples: []\n});";
    #[cfg(not(feature = "typescript"))]
    const EXPECTED: &str = "const Locker$OwnSchema = z.strictObject({\n});\n\nexport const Locker$Schema = z.union([\n  Locker$OwnSchema.and(z.lazy(() => z.record(z.string(), z.number().int().min(0).max(255)))),\n  Locker$OwnSchema.and(z.record(z.string(), z.unknown()).transform(() => ({}))),\n]).meta({\n  examples: []\n});";

    assert_eq!(Locker::zod_schema(), EXPECTED);
}

/// The absence of an optional map named through an alias is dropped as the map's own is.
#[cfg(feature = "zod")]
#[test]
fn the_absence_of_an_optional_map_named_through_an_alias_is_dropped() {
    let dropped = ".and(z.record(z.string(), z.unknown()).transform(() => ({})))";
    let sheave = Sheave::zod_schema();
    assert!(
        sheave.contains(&format!("  Sheave$OwnSchema{dropped},\n")),
        "{sheave}"
    );
}

/// The absence of an optional generic brand filled at a map, and of an optional untagged enum with
/// a map among its variants, is dropped as a map's own is.
#[cfg(feature = "zod")]
#[test]
fn the_absence_of_an_optional_generic_brand_or_untagged_enum_over_a_map_is_dropped() {
    let dropped = ".and(z.record(z.string(), z.unknown()).transform(() => ({})))";
    for (name, zod) in [
        ("Thole", Thole::zod_schema()),
        ("Stanchion", Stanchion::zod_schema()),
    ] {
        assert!(
            zod.contains(&format!("  {name}$OwnSchema{dropped},\n")),
            "{zod}"
        );
    }
}

/// The README quotes the absence of an optional map as the emitter writes it.
#[cfg(feature = "zod")]
#[test]
fn the_readme_quotes_the_absence_of_an_optional_map_as_it_is_written() {
    let absence = "Locker$OwnSchema.and(z.record(z.string(), z.unknown()).transform(() => ({})))";
    assert!(
        Locker::zod_schema().contains(absence),
        "{}",
        Locker::zod_schema()
    );
    assert!(
        include_str!("../../README.md").contains(&format!("`{absence}`")),
        "the README no longer quotes: {absence}"
    );
}
