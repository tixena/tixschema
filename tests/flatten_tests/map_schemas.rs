//! The Zod schema of a struct a map is flattened into: the map's record refuses as a key each key
//! the members of its object and the members flattened ahead of it take, which serde hands the map
//! none of.

use super::maps::assert_serde_reads_what_zod_reads;
#[cfg(feature = "zod")]
use super::maps::{
    Billing, Bulkhead, Compartment, Cording, Davit, Hatchway, Pigeonhole, Rigging, Scuttle,
    Sheeting, Shelflist, Stocktake, Stowing, TaggedStocktake, Tiers, Trussing,
};

#[test]
fn serde_reads_each_payload_zod_reads() {
    assert_serde_reads_what_zod_reads();
}

/// The record of a map beside a member of its own object refuses the member's key.
#[cfg(feature = "zod")]
#[test]
fn a_map_beside_a_member_refuses_the_members_key() {
    #[cfg(feature = "typescript")]
    const EXPECTED: &str = "const Stocktake$RawSchema = z.strictObject({\n  note: z.string(),\n}).and(z.lazy(() => z.record(z.string().refine((key) => ![\"note\"].includes(key)), z.number().int())));\n\nexport const Stocktake$Schema: ZodType<Stocktake> = Stocktake$RawSchema.meta({\n  examples: []\n});";
    #[cfg(not(feature = "typescript"))]
    const EXPECTED: &str = "export const Stocktake$Schema = z.strictObject({\n  note: z.string(),\n}).and(z.lazy(() => z.record(z.string().refine((key) => ![\"note\"].includes(key)), z.number().int()))).meta({\n  examples: []\n});";

    assert_eq!(Stocktake::zod_schema(), EXPECTED);
}

/// The record of a map after a flattened struct refuses the struct's keys.
#[cfg(feature = "zod")]
#[test]
fn a_map_after_a_struct_refuses_the_structs_keys() {
    #[cfg(feature = "typescript")]
    const EXPECTED: &str = "const Stowing$RawSchema = z.strictObject({\n}).and(z.lazy(() => Marking$Schema)).and(z.lazy(() => z.record(z.string().refine((key) => ![\"label\"].includes(key)), z.number().int())));\n\nexport const Stowing$Schema: ZodType<Stowing> = Stowing$RawSchema.meta({\n  examples: []\n});";
    #[cfg(not(feature = "typescript"))]
    const EXPECTED: &str = "export const Stowing$Schema = z.strictObject({\n}).and(z.lazy(() => Marking$Schema)).and(z.lazy(() => z.record(z.string().refine((key) => ![\"label\"].includes(key)), z.number().int()))).meta({\n  examples: []\n});";

    assert_eq!(Stowing::zod_schema(), EXPECTED);
}

/// A map handed every key of its object writes the record it wrote: two maps are each handed the
/// same keys.
#[cfg(feature = "zod")]
#[test]
fn a_map_handed_every_key_writes_the_record_it_wrote() {
    #[cfg(feature = "typescript")]
    const EXPECTED: &str = "const Tiers$RawSchema = z.strictObject({\n}).and(z.lazy(() => z.record(z.string(), z.number().int()))).and(z.lazy(() => z.record(z.string(), z.number().int())));\n\nexport const Tiers$Schema: ZodType<Tiers> = Tiers$RawSchema.meta({\n  examples: []\n});";
    #[cfg(not(feature = "typescript"))]
    const EXPECTED: &str = "export const Tiers$Schema = z.strictObject({\n}).and(z.lazy(() => z.record(z.string(), z.number().int()))).and(z.lazy(() => z.record(z.string(), z.number().int()))).meta({\n  examples: []\n});";

    assert_eq!(Tiers::zod_schema(), EXPECTED);
}

/// Each map refuses the keys serde hands it none of, in every place a map is flattened: the
/// object's own members first, then each key a member flattened ahead of it takes. A struct that
/// flattens a member of its own takes none, and a struct's own tag is handed to the map.
#[cfg(feature = "zod")]
#[test]
fn each_map_refuses_the_keys_taken_ahead_of_it() {
    let refusing =
        |names: &str| format!("z.record(z.string().refine((key) => ![{names}].includes(key)), ");
    for (zod, record) in [
        (Trussing::zod_schema(), refusing("\"note\", \"label\"")),
        (TaggedStocktake::zod_schema(), refusing("\"note\"")),
        (Sheeting::zod_schema(), "z.record(z.string(), ".to_owned()),
        (Cording::zod_schema(), refusing("\"label\"")),
        (Rigging::zod_schema(), refusing("\"label\"")),
        (Pigeonhole::zod_schema(), refusing("\"Pier\"")),
        (Davit::zod_schema(), refusing("\"kind\", \"held\"")),
        (Billing::<String>::zod_schema(), refusing("\"id\"")),
        (Hatchway::zod_schema(), refusing("\"note\"")),
        (Compartment::zod_schema(), refusing("\"kind\", \"note\"")),
        (Bulkhead::zod_schema(), refusing("\"note\"")),
        (Scuttle::zod_schema(), refusing("\"note\"")),
        (
            Shelflist::zod_schema(),
            "z.record(ShelfCode$Schema.refine((key) => ![\"note\"].includes(key)), ".to_owned(),
        ),
    ] {
        assert!(zod.contains(&record), "no `{record}` in: {zod}");
    }
}
