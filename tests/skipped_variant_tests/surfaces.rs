//! What serde reads and writes of the enums of [`super::models`], and what each schema surface
//! writes for them.

use serde_json::Value;

use super::documents::admits;
use super::models::{
    Berthage, Cartage, Dockage, Keelage, Lighterage, Pilotage, ROWS, Tonnage, Wharfage,
    unread_writes,
};

/// The name of each variant serde neither writes nor reads.
const SKIPPED: [&str; 6] = ["Closed", "Hidden", "Gone", "Lone", "Green", "Bulk"];

/// The TypeScript type [`Berthage`] publishes: its variant serde writes alone.
const BERTHAGE_TS: &str = r#"/**
 * Under a tag alone: a variant of members and a newtype variant over text, neither written nor
 * read.
 * 
 */
export type Berthage = {
  /**
   * Open
   * 
   */
  kind: "Open";
  /**
   * note
   * 
   */
  note: string;
};

/** The Rust variant name `value` carries, or "" where it carries none. */
export function Berthage$Variant(value: unknown): string {
  if (typeof value !== "object" || value === null) return "";
  switch ((value as { kind?: unknown }).kind) {
    case "Open": return "Open";
    default: return "";
  }
}"#;

/// The Zod schema [`Berthage`] publishes: its variant serde writes alone.
const BERTHAGE_ZOD: &str = r#"const Berthage$RawSchema = z.discriminatedUnion("kind", [z.strictObject({
  kind: z.literal("Open"),
  note: z.string(),
})]);

export const Berthage$Schema: ZodType<Berthage> = Berthage$RawSchema.meta({
  examples: []
});"#;

/// Each enum by its name, with the three texts its surfaces publish and its JSON Schema.
fn published() -> [(&'static str, String, String, Value); 8] {
    [
        (
            "Berthage",
            Berthage::ts_definition(),
            Berthage::zod_schema(),
            Berthage::json_schema(),
        ),
        (
            "Wharfage",
            Wharfage::ts_definition(),
            Wharfage::zod_schema(),
            Wharfage::json_schema(),
        ),
        (
            "Pilotage",
            Pilotage::ts_definition(),
            Pilotage::zod_schema(),
            Pilotage::json_schema(),
        ),
        (
            "Lighterage",
            Lighterage::ts_definition(),
            Lighterage::zod_schema(),
            Lighterage::json_schema(),
        ),
        (
            "Keelage",
            Keelage::ts_definition(),
            Keelage::zod_schema(),
            Keelage::json_schema(),
        ),
        (
            "Cartage",
            Cartage::ts_definition(),
            Cartage::zod_schema(),
            Cartage::json_schema(),
        ),
        (
            "Tonnage",
            Tonnage::ts_definition(),
            Tonnage::zod_schema(),
            Tonnage::json_schema(),
        ),
        (
            "Dockage",
            Dockage::ts_definition(),
            Dockage::zod_schema(),
            Dockage::json_schema(),
        ),
    ]
}

/// serde reads each payload exactly where the table says, and writes none of the variants it
/// skips.
#[test]
fn serde_reads_and_writes_each_enum_as_the_table_states() {
    for row in &ROWS {
        assert_eq!(
            (row.read)(row.json),
            row.serde,
            "serde's read of {} as {}",
            row.json,
            row.read_as
        );
    }
    let writes = unread_writes();
    let (skipped, written) = writes.split_at(writes.len() - 1);
    for said in skipped {
        assert!(
            said.ends_with("cannot be serialized"),
            "serde writes a variant it skips: {said}"
        );
    }
    assert_eq!(
        written,
        [r#"{"kind":"Closed","reason":"r"}"#],
        "serde writes a variant it never reads"
    );
    assert_eq!(
        serde_json::to_string(&Berthage::Open {
            note: "a".to_owned()
        })
        .unwrap(),
        ROWS[0].json,
        "serde writes the variant it does not skip"
    );
}

/// The TypeScript type and the Zod schema of an enum under a tag alone hold the variant serde
/// writes, and neither the variant of members nor the newtype variant it skips.
#[test]
fn a_variant_under_skip_is_described_by_no_text() {
    assert_eq!(Berthage::ts_definition(), BERTHAGE_TS);
    assert_eq!(Berthage::zod_schema(), BERTHAGE_ZOD);
    for (name, ts, zod, json) in published().into_iter().take(6) {
        for skipped in SKIPPED {
            for (surface, text) in [("TypeScript", &ts), ("Zod", &zod)] {
                assert!(
                    !text.contains(skipped),
                    "the {surface} text of {name} names {skipped}:\n{text}"
                );
            }
            assert!(
                !json.to_string().contains(skipped),
                "the JSON Schema of {name} names {skipped}: {json}"
            );
        }
    }
}

/// A variant serde drops in one direction alone is described as every variant is.
#[test]
fn a_variant_skipped_in_one_direction_alone_is_described() {
    for (name, ts, zod, json) in published().into_iter().skip(6) {
        assert!(
            ts.contains("kind: \"Closed\";"),
            "the TypeScript text of {name}:\n{ts}"
        );
        assert!(
            zod.contains("kind: z.literal(\"Closed\")"),
            "the Zod text of {name}:\n{zod}"
        );
        assert!(
            json.to_string().contains("\"Closed\""),
            "the JSON Schema of {name}: {json}"
        );
    }
}

/// The JSON Schema of each enum admits exactly the payloads the table says Zod takes.
#[test]
fn each_json_schema_admits_what_zod_takes() {
    let documents = published();
    for row in &ROWS {
        let (_, _, _, document) = documents
            .iter()
            .find(|(name, ..)| *name == row.read_as)
            .unwrap();
        assert_eq!(
            admits(document, &serde_json::from_str(row.json).unwrap()),
            row.zod,
            "the JSON Schema of {} on {}: {document}",
            row.read_as,
            row.json
        );
    }
}

/// A plain enum's members are the names serde writes or reads.
#[test]
fn a_plain_enums_members_leave_a_skipped_variant_out() {
    assert_eq!(Keelage::enum_members(), ["Red"]);
}
