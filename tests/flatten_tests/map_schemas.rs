//! The Zod schema of a struct a map is flattened into as the only member of its object: the map's
//! record takes every key, but the tag an internally tagged enum reads ahead of the variant.

use super::maps::assert_serde_reads_what_zod_reads;
#[cfg(feature = "zod")]
use super::maps::{Bilge, Coaming, Keelson, TaggedThwart, Thwart, Transom};

#[test]
fn serde_reads_each_payload_zod_reads() {
    assert_serde_reads_what_zod_reads();
}

/// The record of a map alone in its object takes every key.
#[cfg(feature = "zod")]
#[test]
fn a_map_alone_writes_a_record_of_every_key() {
    #[cfg(feature = "typescript")]
    const EXPECTED: &str = "const Thwart$RawSchema = z.strictObject({\n}).and(z.lazy(() => z.record(z.string(), z.number().int())));\n\nexport const Thwart$Schema: ZodType<Thwart> = Thwart$RawSchema.meta({\n  examples: []\n});";
    #[cfg(not(feature = "typescript"))]
    const EXPECTED: &str = "export const Thwart$Schema = z.strictObject({\n}).and(z.lazy(() => z.record(z.string(), z.number().int()))).meta({\n  examples: []\n});";

    assert_eq!(Thwart::zod_schema(), EXPECTED);
}

/// The record of a map in a variant under a tag refuses the tag as a key, which serde reads ahead
/// of the variant; every other place hands the map every key, a struct's own tag included.
#[cfg(feature = "zod")]
#[test]
fn only_a_map_in_a_variant_under_a_tag_refuses_a_key() {
    assert!(
        Keelson::zod_schema()
            .contains("z.record(z.string().refine((key) => ![\"kind\"].includes(key)), "),
        "{}",
        Keelson::zod_schema()
    );
    for zod in [
        TaggedThwart::zod_schema(),
        Coaming::zod_schema(),
        Transom::zod_schema(),
        Bilge::zod_schema(),
    ] {
        assert!(zod.contains("z.record(z.string(), "), "{zod}");
        assert!(!zod.contains(".refine((key)"), "{zod}");
    }
}
