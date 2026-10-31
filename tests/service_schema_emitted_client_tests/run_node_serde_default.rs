//! A list under `#[serde(default)]` that serde leaves out of what it writes, read by the generated
//! Zod schema under node: a payload without the key holds `[]`, as serde reads it.

#![cfg(feature = "zod")]

use core::fmt::Write as _;

use super::omitted_models::{Muster, OMITTED, OMITTED_REPORT, Roster, Squad};
use super::runtime::{node_modules, ran_with_modules, stand_down_modules};

/// What the run reads a payload with, and prints the answer with: the value the schema reads, or
/// that it refuses it.
const HARNESS: &str = "
function row(name: string, schema: ZodType, payload: string): void {
  const parsed = schema.safeParse(JSON.parse(payload));
  console.log(parsed.success ? `${name}: read ${JSON.stringify(parsed.data)}` : `${name}: refused`);
}
";

/// Each payload of [`OMITTED`] is read by the Zod schema of its type as serde reads it, `roles`
/// held in the value parsed.
#[test]
fn an_omitted_key_with_a_default_reads_under_zod_as_its_default() {
    const REQUIRED: &[&str] = &["zod"];
    let Some(modules) = node_modules(REQUIRED) else {
        stand_down_modules(REQUIRED, "the generated Zod schemas of an omitted key");
        return;
    };
    let mut source = String::from("import { z, type ZodType, type $brand } from \"zod\";\n\n");
    source.push_str(
        &[
            Roster::ts_definition(),
            Roster::zod_schema(),
            Muster::ts_definition(),
            Muster::zod_schema(),
            Squad::ts_definition(),
            Squad::zod_schema(),
        ]
        .join("\n\n"),
    );
    source.push_str(HARNESS);
    for &(name, read_as, payload) in &OMITTED {
        let _ = writeln!(
            source,
            "row({}, {read_as}$Schema, {});",
            serde_json::to_string(name).unwrap(),
            serde_json::to_string(payload).unwrap()
        );
    }
    let Some(printed) = ran_with_modules(
        "serde-default-omitted",
        "TIXSCHEMA_NODE",
        "node",
        "omitted.mts",
        &source,
        &modules,
    ) else {
        return;
    };
    assert_eq!(printed, OMITTED_REPORT);
}
