//! A tuple struct that declares one slot, which serde writes as the slot's value, read by the
//! generated Zod schema under node, over each payload of
//! [`lone_slot_rows`](super::lone_slot_rows): each payload serde reads is read and written back as
//! serde writes it, and each that breaks a rule is refused.

use core::fmt::Write as _;

use super::lone_slot_rows::{held_to_answers, rows};
use super::lone_slots_models::{
    AdjacentStow, Bale, BalePair, Baled, Bales, BalesByName, BalesByNumber, Clipped, Count, Girth,
    Girthed, LabelsByNumber, Laden, MaybeBale, Moniker, Rigger, Sack, Sacked, Single, Sling, Slung,
    Stow, StowedByName, StowedByNumber, StowedList, StowedMaybe, StowedPair, Strop, TaggedStow,
    Tote, Twine, Twined, Unskipped, UntaggedStow,
};
#[cfg(feature = "chrono")]
use super::lone_slots_models::{StowedStamp, Ticked, TickedMillis};
use super::runtime::{node_modules, ran_with_modules, stand_down_modules};

/// What the run reads a payload with, and prints the answer with: the value the schema reads, or
/// that it refuses it.
const HARNESS: &str = "
function row(name: string, schema: ZodType, payload: string): void {
  const parsed = schema.safeParse(JSON.parse(payload));
  console.log(parsed.success ? `${name}: read ${JSON.stringify(parsed.data)}` : `${name}: refused`);
}
";

/// The Zod schema that reads `read_as`, a type as Swift writes it: a factory called at the schema
/// of its type argument.
fn schema_of(read_as: &str) -> String {
    let Some((named, argument)) = read_as.split_once('<') else {
        return format!("{read_as}$Schema");
    };
    let handed = match argument.trim_end_matches('>') {
        "UInt32" | "UInt8" => "z.number()",
        "String" => "z.string()",
        _ => "Bale$Schema",
    };
    format!("{named}$SchemaFactory({handed})")
}

/// The TypeScript type and the Zod schema of every model of the table.
fn definitions() -> Vec<String> {
    vec![
        Bale::ts_definition(),
        Bale::zod_schema(),
        Sack::<String>::ts_definition(),
        Sack::<String>::zod_schema(),
        Count::ts_definition(),
        Count::zod_schema(),
        Moniker::ts_definition(),
        Moniker::zod_schema(),
        Baled::ts_definition(),
        Baled::zod_schema(),
        Bales::ts_definition(),
        Bales::zod_schema(),
        MaybeBale::ts_definition(),
        MaybeBale::zod_schema(),
        BalesByName::ts_definition(),
        BalesByName::zod_schema(),
        BalesByNumber::ts_definition(),
        BalesByNumber::zod_schema(),
        LabelsByNumber::ts_definition(),
        LabelsByNumber::zod_schema(),
        BalePair::ts_definition(),
        BalePair::zod_schema(),
        Unskipped::ts_definition(),
        Unskipped::zod_schema(),
        Clipped::ts_definition(),
        Clipped::zod_schema(),
        Single::<String>::ts_definition(),
        Single::<String>::zod_schema(),
        Sacked::<String>::ts_definition(),
        Sacked::<String>::zod_schema(),
        StowedList::<String>::ts_definition(),
        StowedList::<String>::zod_schema(),
        StowedMaybe::<String>::ts_definition(),
        StowedMaybe::<String>::zod_schema(),
        StowedByName::<String>::ts_definition(),
        StowedByName::<String>::zod_schema(),
        StowedByNumber::<String>::ts_definition(),
        StowedByNumber::<String>::zod_schema(),
        StowedPair::<String>::ts_definition(),
        StowedPair::<String>::zod_schema(),
        Tote::ts_definition(),
        Tote::zod_schema(),
        Sling::ts_definition(),
        Sling::zod_schema(),
        Slung::ts_definition(),
        Slung::zod_schema(),
        Girth::ts_definition(),
        Girth::zod_schema(),
        Strop::ts_definition(),
        Strop::zod_schema(),
        Girthed::ts_definition(),
        Girthed::zod_schema(),
        Rigger::ts_definition(),
        Rigger::zod_schema(),
        Twine::ts_definition(),
        Twine::zod_schema(),
        Twined::ts_definition(),
        Twined::zod_schema(),
        Laden::ts_definition(),
        Laden::zod_schema(),
        Stow::ts_definition(),
        Stow::zod_schema(),
        AdjacentStow::ts_definition(),
        AdjacentStow::zod_schema(),
        UntaggedStow::ts_definition(),
        UntaggedStow::zod_schema(),
        TaggedStow::ts_definition(),
        TaggedStow::zod_schema(),
        #[cfg(feature = "chrono")]
        Ticked::ts_definition(),
        #[cfg(feature = "chrono")]
        Ticked::zod_schema(),
        #[cfg(feature = "chrono")]
        TickedMillis::ts_definition(),
        #[cfg(feature = "chrono")]
        TickedMillis::zod_schema(),
        #[cfg(feature = "chrono")]
        StowedStamp::<String>::ts_definition(),
        #[cfg(feature = "chrono")]
        StowedStamp::<String>::zod_schema(),
    ]
}

/// Every payload serde reads is read by the Zod schema and written back as serde writes it, and
/// every payload whose value breaks a rule is refused.
#[test]
fn a_struct_of_one_slot_reads_under_zod_as_serde_does_wherever_it_is_held() {
    const REQUIRED: &[&str] = &["zod"];
    let Some(modules) = node_modules(REQUIRED) else {
        stand_down_modules(
            REQUIRED,
            "the generated Zod schemas of a struct of one slot",
        );
        return;
    };
    let rows = rows();
    let mut source = String::from(
        "import { z, type ZodType, type $brand } from \"zod\";\nimport type { SomeType } from \
         \"zod/v4/core\";\n\n",
    );
    source.push_str(&definitions().join("\n\n"));
    source.push_str(HARNESS);
    for row in &rows {
        let _ = writeln!(
            source,
            "row({}, {}, {});",
            serde_json::to_string(row.name).unwrap(),
            schema_of(row.read_as),
            serde_json::to_string(&row.payload).unwrap()
        );
    }
    let Some(printed) = ran_with_modules(
        "lone-slots",
        "TIXSCHEMA_NODE",
        "node",
        "slots.mts",
        &source,
        &modules,
    ) else {
        return;
    };
    held_to_answers(&rows, &printed, "Zod");
}
