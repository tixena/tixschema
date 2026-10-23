//! Generic types that reach themselves, read from another package: through the declaration files
//! a real TypeScript compiler emits for the bundle, which is all a second package ever sees.

#![cfg(all(unix, feature = "serde", feature = "typescript", feature = "zod"))]

/// The models a check is written on, which the `check` tests run through `validate()`.
#[path = "../check_tests/models.rs"]
mod checked;

/// The models a `preprocess` is written on, which the `preprocess` tests read through serde.
#[path = "../pattern_preprocess_tests/models.rs"]
mod corrected;

/// The models a key under `#[serde(default)]` is written on, and what a serde read prints for
/// each payload of their table.
#[path = "../serde_default_tests/models.rs"]
mod defaulted;

/// The models the four std ranges are written on, and the payloads serde writes and refuses.
#[path = "../std_range_tests/models.rs"]
mod ranged;

/// What a JSON Schema document admits.
#[cfg(feature = "jsonschema")]
#[path = "../std_range_tests/documents.rs"]
mod documents;

/// The models a std `Result` is written on, and the payloads serde writes and refuses.
#[path = "../std_result_tests/models.rs"]
mod resulted;

/// The models a set is written on, and the payloads that repeat an item of one.
#[path = "../set_tests/models.rs"]
mod sets;

/// The models a map is flattened into alone, and the payloads serde reads and refuses for them.
#[path = "../flatten_tests/maps.rs"]
mod flattened_maps;

/// The models of maps keyed by a value serde writes as text of its own, and the payloads serde
/// reads and refuses for them.
#[cfg(feature = "chrono")]
#[path = "../collection_tests/keyed_maps.rs"]
mod keyed_maps;

/// The holders of an optional flattened member whose type the Kotlin emitter recorded no keys for,
/// and whether TypeScript and Zod take each payload.
#[path = "../kotlin_tests/unrecorded_options_models.rs"]
mod unrecorded;

/// The holders of an optional flattened member that names a function, what serde reads of each
/// payload, and whether TypeScript and Zod take it.
#[path = "../kotlin_tests/corrected_options_models.rs"]
mod corrected_options;

/// The holders of a flattened struct that reads a key taken around it, what serde says of each
/// payload, and what Zod reads of it.
#[path = "../swift_tests/shared_key_models.rs"]
mod shared_keys;

/// A `char` and a chrono date, time and date and time held in each place, and what serde, Zod and
/// the Kotlin read answer of each payload.
#[cfg(feature = "chrono")]
#[path = "../kotlin_tests/text_forms_models.rs"]
mod text_forms;

/// Models whose names on the wire and doc comments hold the end of a comment, and what serde
/// writes of each.
#[path = "commented_models.rs"]
mod commented;

/// A date under `as_number` in a member of an untagged enum, and what serde and the Zod schema
/// answer of each payload.
#[cfg(feature = "chrono")]
#[path = "../chrono_tests/untagged_millis_models.rs"]
mod untagged_millis;

/// A model per type the wire holds as a text, a number or a `bool`, and what each client sends of
/// each value a caller hands it.
#[cfg(feature = "chrono")]
#[path = "../wire_forms_tests/models.rs"]
mod wire_forms;

/// The models of a `u64` and an `i64` a member writes itself, which travel as the string of their
/// digits.
#[path = "../digit_strings_tests/models.rs"]
mod digit_strings;

/// Models whose names on the wire hold a `$`, a quote, a double quote and a backslash, and what
/// serde writes of each.
#[path = "../dart_tests/quoted_models.rs"]
mod quoted;
/// The enums with a variant serde neither writes nor reads, or drops in one direction alone, and
/// what serde and Zod take of each payload.
#[path = "../skipped_variant_tests/models.rs"]
mod skipped_variants;

/// The models a `literal` is written on, and what each payload of their table reads as.
#[path = "../model_schema_prop_tests/literal_models.rs"]
mod literals;

/// An alias of an `Option` held outside a field, and the payloads serde writes for it.
#[path = "../semantic_types_tests/option_alias_models.rs"]
mod option_aliases;

/// Untagged enums with a newtype variant over an `Option`, and the variant serde takes for each
/// payload.
#[path = "../untagged_tests/null_models.rs"]
mod null_models;

#[cfg(feature = "chrono")]
use alloc::collections::BTreeSet;
use core::fmt::Write as _;

#[cfg(feature = "chrono")]
use chrono::{DateTime, Utc};
#[cfg(feature = "mongodb")]
use mongodb::bson::oid::ObjectId;
#[cfg(feature = "jsonschema")]
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::env::temp_dir;
use std::fs;
use std::io::Write as _;
use std::io::stderr;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, id};
use std::sync::Once;
use tixschema::model_schema;

/// What the bounded driver prints: each broken bound under its key in the bound's own words, and
/// a value inside every bound as it was sent.
const BOUNDED: &str = "[[\"later: too long: maximum length is 3, got 4\",\"many: too short: \
                       minimum length is 2, got 1\",\"sort: too short: minimum length is 1, got \
                       0\"],[\"sort: does not match pattern '^[a-z.]*$'\"],{\"later\":\"abc\",\
                       \"many\":[\"ab\",\"c\"],\"sort\":\"a.b\"},{\"later\":\"abc\",\"many\":[\"a\",\
                       \"b\"]}]";

/// Loads the compiled bundle and parses through the struct whose fields bound a brand.
const BOUNDED_DRIVER: &str = r#"import { DeclaredBounded$Schema } from "./lib/index.js";

const said = (value) => {
  const read = DeclaredBounded$Schema.safeParse(value);
  return read.success
    ? read.data
    : read.error.issues.map((issue) => `${issue.path.join(".")}: ${issue.message}`);
};
console.log(
  JSON.stringify([
    said({ later: "abcd", many: ["ab"], sort: "" }),
    said({ later: "abc", many: ["ab", "c"], sort: "A" }),
    said({ later: "abc", many: ["ab", "c"], sort: "a.b" }),
    said({ later: "abc", many: ["a", "b"] }),
  ]),
);
"#;

/// What the dated driver prints: what each position that holds a date reads each input as.
#[cfg(feature = "chrono")]
const DATED: &str = "{\"required\":{\"null\":\"refused\",\"true\":\"refused\",\"false\":\
                     \"refused\",\"text\":\"refused\",\"object\":\"refused\",\"date\":\
                     \"2026-10-12T12:00:00.000Z\",\"iso\":\"2026-10-12T12:00:00.000Z\",\
                     \"millis\":\"1970-01-01T00:00:00.000Z\"},\"option\":{\"null\":\
                     \"undefined\",\"absent\":\"undefined\",\"false\":\"refused\",\"iso\":\
                     \"2026-10-12T12:00:00.000Z\"},\"nullable\":{\"null\":null,\"absent\":\
                     \"refused\",\"false\":\"refused\",\"iso\":\"2026-10-12T12:00:00.000Z\"},\
                     \"list\":{\"nulls\":[null,\"2026-10-12T12:00:00.000Z\"],\"true\":\
                     \"refused\"},\"untagged\":{\"true\":true,\"iso\":\
                     \"2026-10-12T12:00:00.000Z\",\"null\":\"refused\"}}";

/// Loads the compiled bundle and parses through the struct that holds a date in each position:
/// required, in an `Option`, under `nullable`, among a list's items, and in an untagged enum.
#[cfg(feature = "chrono")]
const DATED_DRIVER: &str = r#"import { DeclaredStamped$Schema } from "./lib/index.js";

const whole = { at: "2026-10-12T12:00:00Z", mark: true, paused: null, seen: [] };
const shown = (value) => {
  if (value instanceof Date) return value.toISOString();
  if (Array.isArray(value)) return value.map(shown);
  return value === undefined ? "undefined" : value;
};
const read = (row, key) => {
  const parsed = DeclaredStamped$Schema.safeParse(row);
  return parsed.success ? shown(parsed.data[key]) : "refused";
};
const given = (key, value) => read({ ...whole, [key]: value }, key);
const { paused: _held, ...unpaused } = whole;
console.log(
  JSON.stringify({
    required: {
      null: given("at", null),
      true: given("at", true),
      false: given("at", false),
      text: given("at", "not a date"),
      object: given("at", { valueOf: () => 0 }),
      date: given("at", new Date("2026-10-12T12:00:00Z")),
      iso: given("at", "2026-10-12T12:00:00Z"),
      millis: given("at", 0),
    },
    option: {
      null: given("ended", null),
      absent: read(whole, "ended"),
      false: given("ended", false),
      iso: given("ended", "2026-10-12T12:00:00Z"),
    },
    nullable: {
      null: given("paused", null),
      absent: read(unpaused, "paused"),
      false: given("paused", false),
      iso: given("paused", "2026-10-12T12:00:00Z"),
    },
    list: {
      nulls: given("seen", [null, "2026-10-12T12:00:00Z"]),
      true: given("seen", [true]),
    },
    untagged: {
      true: given("mark", true),
      iso: given("mark", "2026-10-12T12:00:00Z"),
      null: given("mark", null),
    },
  }),
);
"#;

/// The functions the checks of the bundle name, as a consumer writes them in TypeScript. Each takes
/// the value and the payload it pushes its issues to. The check of a brand takes the value the
/// brand is written over.
const CHECKS: &str = r#"import type { ParsePayload } from "zod/v4/core";

export function withoutDuplicates(value: Array<string>, ctx: ParsePayload<Array<string>>): void {
  const seen = new Set<string>();
  for (const code of value) {
    if (seen.has(code)) {
      ctx.issues.push({ code: "custom", message: `duplicate item code ${code}`, path: [], input: value });
      return;
    }
    seen.add(code);
  }
}

export function withinParcel(value: Parcel, ctx: ParsePayload<Parcel>): void {
  value.fragileItemCodes.forEach((code, index) => {
    if (!value.itemCodeList.includes(code)) {
      ctx.issues.push({
        code: "custom",
        message: `${code} is outside of itemCodeList`,
        path: ["fragileItemCodes", index],
        input: value,
      });
    }
  });
}

export function withARoute(value: Route, ctx: ParsePayload<Route>): void {
  if (value.stops.length === 0) {
    ctx.issues.push({ code: "custom", message: "a route makes at least one stop", path: ["stops"], input: value });
  }
}

export function withinItsLimit(value: Stack, ctx: ParsePayload<Stack>): void {
  if (value[0].length > value[1]) {
    ctx.issues.push({ code: "custom", message: `a stack holds at most ${value[1]} codes`, path: [0], input: value });
  }
}

export function takingParcels(value: ClosedDock, ctx: ParsePayload<ClosedDock>): void {
  ctx.issues.push({ code: "custom", message: "a closed dock takes no parcel", path: [], input: value });
}

export function withFewStops(value: Shipment, ctx: ParsePayload<Shipment>): void {
  if (value.kind === "air" && value.stops.length > 2) {
    ctx.issues.push({ code: "custom", message: "an air shipment makes at most 2 stops", path: ["stops"], input: value });
  }
}

export function withAStop(value: Transfer, ctx: ParsePayload<Transfer>): void {
  if (typeof value === "string") return;
  if (value.Road !== undefined && value.Road.stops.length === 0) {
    ctx.issues.push({ code: "custom", message: "a road transfer makes a stop", path: ["Road", "stops"], input: value });
  }
  if (value.Rail !== undefined && value.Rail === "") {
    ctx.issues.push({ code: "custom", message: "a rail transfer names its line", path: ["Rail"], input: value });
  }
}

export function signedBySomeone(value: Handover, ctx: ParsePayload<Handover>): void {
  if (value.kind === "Signed" && value.detail.by === "") {
    ctx.issues.push({ code: "custom", message: "a handover is signed by someone", path: ["detail", "by"], input: value });
  }
}

export function aReference(value: Reference, ctx: ParsePayload<Reference>): void {
  if (typeof value === "number") {
    if (value === 0) {
      ctx.issues.push({ code: "custom", message: "a reference is not zero", path: [], input: value });
    }
    return;
  }
  if (value.name === "") {
    ctx.issues.push({ code: "custom", message: "a reference has a name", path: ["name"], input: value });
  }
}

export function rated(value: Grade, ctx: ParsePayload<Grade>): void {
  if (value === "Unrated") {
    ctx.issues.push({ code: "custom", message: "a parcel is rated", path: [], input: value });
  }
}

export function upperCase(value: string, ctx: ParsePayload<string>): void {
  if (value !== value.toUpperCase()) {
    ctx.issues.push({ code: "custom", message: "a code is upper case", path: [], input: value });
  }
}

export function withACode<CodeType>(value: Crate<CodeType>, ctx: ParsePayload<Crate<CodeType>>): void {
  if (value.codes.length === 0) {
    ctx.issues.push({ code: "custom", message: "a crate holds at least one code", path: ["codes"], input: value });
  }
}

export function holdingACode<CodeType>(value: Array<CodeType>, ctx: ParsePayload<Array<CodeType>>): void {
  if (value.length === 0) {
    ctx.issues.push({ code: "custom", message: "a batch holds at least one code", path: [], input: value });
  }
}

export function withEntries<IdType>(value: Ledger<IdType>, ctx: ParsePayload<Ledger<IdType>>): void {
  if (value.entries.length === 0) {
    ctx.issues.push({ code: "custom", message: "a ledger holds at least one entry", path: ["entries"], input: value });
  }
}

export function shortList(value: Array<string>, ctx: ParsePayload<Array<string>>): void {
  if (value.length > 2) {
    ctx.issues.push({ code: "custom", message: `${value[2]} is one code too many`, path: [2], input: value });
  }
}

export function stackedLow(value: Array<Package>, ctx: ParsePayload<Array<Package>>): void {
  if (value.length > 2) {
    ctx.issues.push({ code: "custom", message: "a stack is at most two parcels high", path: [], input: value });
  }
}"#;

/// Loads the compiled bundle, parses the payload of each case through the schema its type
/// publishes, and prints what the Rust side prints for the same case: a message under the path
/// of its issue, and alone where that path is empty.
const CHECKS_DRIVER: &str = r#"import { z } from "zod";
import * as lib from "./lib/index.js";

const schemas = {
  Parcel: lib.Parcel$Schema,
  Manifest: lib.Manifest$Schema,
  Route: lib.Route$Schema,
  Stack: lib.Stack$Schema,
  ClosedDock: lib.ClosedDock$Schema,
  Shipment: lib.Shipment$Schema,
  Transfer: lib.Transfer$Schema,
  Handover: lib.Handover$Schema,
  Reference: lib.Reference$Schema,
  Grade: lib.Grade$Schema,
  Code: lib.Code$Schema,
  "Crate<String>": lib.Crate$SchemaDefault,
  "Crate<u32>": lib.Crate$SchemaFactory(z.number().int()),
  "Batch<String>": lib.Batch$SchemaDefault,
  "Batch<u32>": lib.Batch$SchemaFactory(z.number().int()),
  "Ledger<String>": lib.Ledger$SchemaDefault,
  "Ledger<u32>": lib.Ledger$SchemaFactory(z.number().int()),
  Pallet: lib.Pallet$Schema,
  Package: lib.Package$Schema,
  Docket: lib.Docket$Schema,
  Bundle: lib.Bundle$Schema,
  Lone: lib.Lone$Schema,
  Strapped: lib.Strapped$Schema,
  Crane: lib.Crane$Schema,
  Hoist: lib.Hoist$Schema,
  Freight: lib.Freight$Schema,
  Seal: lib.Seal$Schema,
  Haul: lib.Haul$Schema,
  Lift: lib.Lift$Schema,
  Load: lib.Load$Schema,
  Mooring: lib.Mooring$Schema,
  Measured: lib.Measured$Schema,
  Departure: lib.Departure$Schema,
};
const printed = [];
for (const [name, schema, payload] of CASES) {
  const read = schemas[schema].safeParse(JSON.parse(payload));
  if (read.success) {
    printed.push(`${name}: passes`);
    continue;
  }
  printed.push(`${name}: refused`);
  for (const issue of read.error.issues) {
    const lead = issue.path.length > 0 ? `'${issue.path.join(".")}': ` : "";
    printed.push(`    ${lead}${issue.message}`);
  }
}
console.log(printed.join("\n"));
"#;

const COMPILER_VAR: &str = "TIXSCHEMA_TSC";

/// The functions the `preprocess` keys of the bundle name, as a consumer writes them in
/// TypeScript: each takes the payload and answers the payload the schema reads. `handed` keeps
/// what `orUnnamed` was called with, `undefined` under that word.
const CORRECTIONS: &str = r#"import type { ParsePayload } from "zod/v4/core";

export function trimmed(payload: unknown): unknown {
  return typeof payload === "string" ? payload.trim() : payload;
}

export function withoutFragile(payload: unknown): unknown {
  if (typeof payload !== "object" || payload === null || Array.isArray(payload)) {
    return payload;
  }
  return "fragileItemCodes" in payload ? payload : { ...payload, fragileItemCodes: [] };
}

export function unwrapped(payload: unknown): unknown {
  if (typeof payload !== "object" || payload === null || !("payload" in payload)) {
    return payload;
  }
  return payload.payload;
}

export function withChildren(payload: unknown): unknown {
  if (typeof payload !== "object" || payload === null || Array.isArray(payload)) {
    return payload;
  }
  return "children" in payload ? payload : { ...payload, children: [] };
}

export function trimmedPlate(payload: unknown): unknown {
  if (typeof payload !== "object" || payload === null || !("plate" in payload)) {
    return payload;
  }
  return typeof payload.plate === "string" ? { ...payload, plate: payload.plate.trim() } : payload;
}

export function withItsKeys(payload: unknown): unknown {
  if (typeof payload !== "object" || payload === null || Array.isArray(payload)) {
    return payload;
  }
  return { ...payload, keys: Object.keys(payload).sort() };
}

export const handed: Array<unknown> = [];

export function orUnnamed(payload: unknown): unknown {
  handed.push(payload === undefined ? "undefined" : payload);
  return payload === null ? "unnamed" : payload;
}

export function withinLimit(value: Weighed, ctx: ParsePayload<Weighed>): void {
  if (value.grams > 1000) {
    ctx.issues.push({
      code: "custom",
      message: `${value.grams} is over the limit of 1000`,
      path: ["grams"],
      input: value,
    });
  }
}"#;

/// Loads the compiled bundle of the `preprocess` models, parses the payload of each case through
/// the schema its type publishes, and prints what a Rust read prints for the same case.
const CORRECTIONS_DRIVER: &str = r#"import { z } from "zod";
import * as lib from "./lib/index.js";

const schemas = {
  Parcel: lib.Parcel$Schema,
  Manifest: lib.Manifest$Schema,
  Shelf: lib.Shelf$Schema,
  Crated: lib.Crated$Schema,
  Delivery: lib.Delivery$Schema,
  Consigned: lib.Consigned$Schema,
  Slot: lib.Slot$Schema,
  Either: lib.Either$Schema,
  Node: lib.Node$Schema,
  Routed: lib.Routed$Schema,
  ClosedDock: lib.ClosedDock$Schema,
  Stack: lib.Stack$Schema,
  Label: lib.Label$Schema,
  Grade: lib.Grade$Schema,
  Shipment: lib.Shipment$Schema,
  Transfer: lib.Transfer$Schema,
  Handover: lib.Handover$Schema,
  Reference: lib.Reference$Schema,
  "Crate<String>": lib.Crate$SchemaDefault,
  "Crate<u32>": lib.Crate$SchemaFactory(z.number().int()),
  Wrapped: lib.Wrapped$Schema,
  Weighed: lib.Weighed$Schema,
  Unnamed: lib.Unnamed$Schema,
};
const printed = [];
for (const [name, schema, payload] of CASES) {
  const read = schemas[schema].safeParse(JSON.parse(payload));
  if (read.success) {
    printed.push(`${name}: read ${JSON.stringify(read.data)}`);
    continue;
  }
  printed.push(`${name}: refused`);
  for (const issue of read.error.issues) {
    const lead = issue.path.length > 0 ? `'${issue.path.join(".")}': ` : "";
    printed.push(`    ${lead}${issue.message}`);
  }
}
console.log(printed.join("\n"));
"#;

/// The function `Bollard` names, as a consumer writes it in TypeScript.
const DOCKED: &str = r#"export function trimmedDock(payload: unknown): unknown {
  return typeof payload === "string" ? payload.trim() : payload;
}"#;

/// Parses each payload of the `preprocess` table through `Bollard`.
const DOCKED_DRIVER: &str = r#"import * as lib from "./lib/index.js";

const printed = [];
for (const [name, payload] of CASES) {
  printed.push(`${name}: read ${JSON.stringify(lib.Bollard$Schema.parse(JSON.parse(payload)))}`);
}
console.log(printed.join("\n"));
"#;

/// Loads the compiled bundle of the models a key under `#[serde(default)]` is written on, parses
/// the payload of each case through the schema its type publishes, and prints what a serde read
/// prints for the same case.
const DEFAULTS_DRIVER: &str = r#"import * as lib from "./lib/index.js";

const schemas = {
  Wharf: lib.Wharf$Schema,
  Jetty: lib.Jetty$Schema,
  Pontoon: lib.Pontoon$Schema,
};
const printed = [];
for (const [name, schema, payload] of CASES) {
  const read = schemas[schema].safeParse(JSON.parse(payload));
  if (read.success) {
    printed.push(`${name}: read ${JSON.stringify(read.data)}`);
    continue;
  }
  printed.push(`${name}: refused`);
  for (const issue of read.error.issues) {
    const lead = issue.path.length > 0 ? `'${issue.path.join(".")}': ` : "";
    printed.push(`    ${lead}${issue.message}`);
  }
}
console.log(printed.join("\n"));
"#;

/// Parses the payload of each case of the variant table through the schema its type publishes,
/// and prints what a serde read prints for the same case.
const VARIANTS_DRIVER: &str = r#"import * as lib from "./lib/index.js";

const schemas = {
  Berthing: lib.Berthing$Schema,
  ExternalBerthing: lib.ExternalBerthing$Schema,
  AdjacentBerthing: lib.AdjacentBerthing$Schema,
  UntaggedBerthing: lib.UntaggedBerthing$Schema,
  Quay: lib.Quay$Schema,
};
const printed = [];
for (const [name, schema, payload] of CASES) {
  const read = schemas[schema].safeParse(JSON.parse(payload));
  if (read.success) {
    printed.push(`${name}: read ${JSON.stringify(read.data)}`);
    continue;
  }
  printed.push(`${name}: refused`);
  for (const issue of read.error.issues) {
    const lead = issue.path.length > 0 ? `'${issue.path.join(".")}': ` : "";
    printed.push(`    ${lead}${issue.message}`);
  }
}
console.log(printed.join("\n"));
"#;

/// Parses each payload of the ranges through the schema of the type it is read as, the declared
/// default of one that declares a parameter, and prints what each read answers.
const RANGES_DRIVER: &str = r#"import * as lib from "./lib/index.js";

const schemas = {
  Compass: lib.Compass$Schema,
  Course: lib.Course$SchemaDefault,
  Detour: lib.Detour$Schema,
  Excursion: lib.Excursion$Schema,
  Waypost: lib.Waypost$Schema,
  Frontier: lib.Frontier$Schema,
  Positioned: lib.Positioned$Schema,
  Rostered: lib.Rostered$Schema,
  Signposted: lib.Signposted$Schema,
  Spanned: lib.Spanned$Schema,
  Stride: lib.Stride$SchemaDefault,
};
const sorted = (value) =>
  Array.isArray(value)
    ? value.map(sorted)
    : value !== null && typeof value === "object" && !(value instanceof Date)
      ? Object.fromEntries(
          Object.keys(value)
            .sort()
            .map((key) => [key, sorted(value[key])]),
        )
      : value;
const printed = [];
for (const [name, schema, payload] of PAYLOADS) {
  const read = schemas[schema].safeParse(JSON.parse(payload));
  if (read.success) {
    printed.push(`${name}: read ${JSON.stringify(sorted(read.data))}`);
  } else if (read.error.issues.some((issue) => issue.code === "unrecognized_keys")) {
    printed.push(`${name}: refused for a key it does not declare`);
  } else {
    printed.push(`${name}: refused`);
  }
}
console.log(printed.join("\n"));
"#;

/// Parses each payload through the schema of the model it is read as, and prints one line per
/// payload: the value read, or the issues of the refusal.
const QUOTED_DRIVER: &str = r#"import * as lib from "./lib/index.js";

for (const [readAs, payload] of PAYLOADS) {
  const read = lib[`${readAs}$Schema`].safeParse(JSON.parse(payload));
  console.log(JSON.stringify(read.success ? read.data : { refused: read.error.issues }));
}
"#;

/// Parses each payload through the schema of the model it is read as, and prints one line per
/// payload: the value read, `undefined` for a read that answers no value, or `refused`.
const VALUES_DRIVER: &str = r#"import * as lib from "./lib/index.js";

for (const [readAs, payload] of PAYLOADS) {
  const read = lib[`${readAs}$Schema`].safeParse(JSON.parse(payload));
  console.log(read.success ? (JSON.stringify(read.data) ?? "undefined") : "refused");
}
"#;

/// Parses each payload of the sets through the schema of the type it is read as, the declared
/// default of one that declares a parameter, and prints what each read answers: `read`, or the
/// refusal with each issue under its path.
const SETS_DRIVER: &str = r#"import * as lib from "./lib/index.js";

const printed = [];
for (const [name, schema, payload] of PAYLOADS) {
  const read = lib[schema].safeParse(JSON.parse(payload));
  if (read.success) {
    printed.push(`${name}: read`);
  } else {
    printed.push(`${name}: refused`);
    for (const issue of read.error.issues) {
      const place = issue.path.length === 0 ? "" : `'${issue.path.join(".")}': `;
      printed.push(`    ${place}${issue.message}`);
    }
  }
}
console.log(printed.join("\n"));
"#;

/// Parses each payload of a flattened map or an optional flattened struct through the schema of
/// the type it is read as, the declared default of one that declares a parameter, and prints what
/// each read answers: `read`, or the refusal with each issue under its path.
const FLATTENED_MAPS_DRIVER: &str = r#"import * as lib from "./lib/index.js";

const printed = [];
for (const [name, schema, payload] of PAYLOADS) {
  const read = lib[schema].safeParse(JSON.parse(payload));
  if (read.success) {
    printed.push(`${name}: read`);
  } else {
    printed.push(`${name}: refused`);
    for (const issue of read.error.issues) {
      const place = issue.path.length === 0 ? "" : `'${issue.path.join(".")}': `;
      printed.push(`    ${place}${issue.message}`);
    }
  }
}
console.log(printed.join("\n"));
"#;

/// Parses each payload through the schema of the type it is read as, and prints whether each read
/// takes it.
const TAKEN_DRIVER: &str = r#"import * as lib from "./lib/index.js";

console.log(
  JSON.stringify(PAYLOADS.map(([schema, payload]) => lib[schema].safeParse(JSON.parse(payload)).success)),
);
"#;

/// What the driver of [`digit_strings_bundle`] prints of each payload: what the Zod schema reads,
/// each `bigint` written as its digits and an `n`, or the message of each issue.
const DIGIT_STRINGS_DRIVER: &str = r#"import * as lib from "./lib/index.js";

const shown = (value) => JSON.stringify(value, (_, held) => (typeof held === "bigint" ? `${held}n` : held));
for (const [schema, payload] of PAYLOADS) {
  const read = lib[schema].safeParse(JSON.parse(payload));
  console.log(read.success ? shown(read.data) : read.error.issues.map((issue) => issue.message).join("; "));
}
WIRE_FORM
console.log(JSON.stringify(wireForm(lib.DigitTally$Schema.parse(JSON.parse(TALLY)))));
"#;

/// The digits serde writes of a `DigitTally` with an `i64` beyond what a JavaScript `number` holds
/// exactly in each place.
const DIGIT_TALLY: &str = r#"{"boxed":"-1152921504606846976","grid":["1","1152921504606846976"],"keyed":{"a":"-7"},"listed":["0","9223372036854775807"],"maybe":"-9223372036854775808","nested":[["3"],null],"paired":["9","x"],"set":["2","5"],"signed":"-1152921504606846976"}"#;

/// The value a client writes of a value its schema read, as each client's own helper writes it:
/// each `bigint` as the string of its digits.
const WIRE_FORM: &str = "function wireForm(value) {
  if (typeof value === \"bigint\") return value.toString();
  if (Array.isArray(value)) return value.map((held) => wireForm(held));
  if (typeof value === \"object\" && value !== null && Object.getPrototypeOf(value) === Object.prototype) {
    return Object.fromEntries(Object.entries(value).map(([key, held]) => [key, wireForm(held)]));
  }
  return value;
}";

/// The functions the checks of [`DeclaredSway`] and [`DeclaredLull`] name, which refuse the unit
/// variant's bare name.
const NOT_CALM: &str = r#"import type { ParsePayload } from "zod/v4/core";

export function notCalm(value: DeclaredTack, ctx: ParsePayload<DeclaredTack>): void {
  if (value === "Calm") {
    ctx.issues.push({ code: "custom", message: "a calm tack is refused", path: [], input: value });
  }
}

export function notCalmLull(value: DeclaredLull, ctx: ParsePayload<DeclaredLull>): void {
  if (value === "Calm") {
    ctx.issues.push({ code: "custom", message: "a calm lull is refused", path: [], input: value });
  }
}"#;

/// The functions the holders of [`corrected_options_bundle`] name, as a consumer writes them in
/// TypeScript.
const AS_GIVEN_OR_NULL: &str = "export function asGiven(payload: unknown): unknown {
  return payload;
}

export function asNull(_payload: unknown): unknown {
  return null;
}";

/// Parses each payload through the schema of the type it is read as, and prints what each read
/// answers, one line each: the object, its keys in order, or `refused`.
const READ_DRIVER: &str = r#"import * as lib from "./lib/index.js";

const sorted = (value) => Object.fromEntries(Object.keys(value).sort().map((key) => [key, value[key]]));
for (const [schema, payload] of PAYLOADS) {
  const read = lib[schema].safeParse(JSON.parse(payload));
  console.log(read.success ? JSON.stringify(sorted(read.data)) : "refused");
}
"#;

/// Parses each payload of a `Result` through the schema of the type it is read as, the declared
/// default of one that declares a parameter, and prints what each read answers: the value, or the
/// refusal, with each issue under its path where the type judges a model it holds.
const RESULTS_DRIVER: &str = r#"import * as lib from "./lib/index.js";

const schemas = {
  Attempted: lib.Attempted$SchemaDefault,
  Bureau: lib.Bureau$Schema,
  Errand: lib.Errand$Schema,
  Errata: lib.Errata$Schema,
  Missive: lib.Missive$Schema,
  Notebook: lib.Notebook$Schema,
  Postbox: lib.Postbox$Schema,
  Reply: lib.Reply$Schema,
  Summary: lib.Summary$Schema,
  Turn: lib.Turn$SchemaDefault,
};
const sorted = (value) =>
  Array.isArray(value)
    ? value.map(sorted)
    : value !== null && typeof value === "object"
      ? Object.fromEntries(
          Object.keys(value)
            .sort()
            .map((key) => [key, sorted(value[key])]),
        )
      : value;
const printed = [];
for (const [name, schema, payload, judged] of PAYLOADS) {
  const read = schemas[schema].safeParse(JSON.parse(payload));
  if (read.success) {
    printed.push(`${name}: read ${JSON.stringify(sorted(read.data))}`);
    continue;
  }
  printed.push(`${name}: refused`);
  if (judged) {
    const said = read.error.issues.map((issue) => `    '${issue.path.join(".")}': ${issue.message}`);
    printed.push(...said.sort());
  }
}
console.log(printed.join("\n"));
"#;

/// Prints each payload of the literal table as read through the Zod schema of its model: whether
/// it is read, then each issue where the row names them.
const LITERALS_DRIVER: &str = r#"import * as lib from "./lib/index.js";

const printed = [];
for (const [name, schema, payload, named] of PAYLOADS) {
  const read = lib[`${schema}$Schema`].safeParse(JSON.parse(payload));
  if (read.success) {
    printed.push(`${name}: read`);
    continue;
  }
  printed.push(`${name}: refused`);
  if (named) {
    printed.push(...read.error.issues.map((issue) => `    '${issue.path.join(".")}': ${issue.message}`));
  }
}
console.log(printed.join("\n"));
"#;

/// Each payload of a `()`, by the model it is read as: `null`, which serde writes for it, and the
/// empty array serde refuses, in each place `DeclaredUnits` holds one.
const UNITS: [(&str, &str, &str); 7] = [
    ("brand of null", "DeclaredNothing", "null"),
    ("brand of an empty array", "DeclaredNothing", "[]"),
    (
        "held as null",
        "DeclaredUnits",
        r#"{"listed":[null],"nothing":null,"paired":[1,null],"unit":null}"#,
    ),
    (
        "member of an empty array",
        "DeclaredUnits",
        r#"{"listed":[],"nothing":null,"paired":[1,null],"unit":[]}"#,
    ),
    (
        "item of an empty array",
        "DeclaredUnits",
        r#"{"listed":[[]],"nothing":null,"paired":[1,null],"unit":null}"#,
    ),
    (
        "position of an empty array",
        "DeclaredUnits",
        r#"{"listed":[],"nothing":null,"paired":[1,[]],"unit":null}"#,
    ),
    (
        "brand member of an empty array",
        "DeclaredUnits",
        r#"{"listed":[],"nothing":[],"paired":[1,null],"unit":null}"#,
    ),
];

/// Parses each payload of [`UNITS`] through the Zod schema of its model.
const UNITS_DRIVER: &str = r#"import * as lib from "./lib/index.js";

const printed = [];
for (const [name, schema, payload] of PAYLOADS) {
  const read = lib[`${schema}$Schema`].safeParse(JSON.parse(payload));
  printed.push(`${name}: ${read.success ? "read" : "refused"}`);
}
console.log(printed.join("\n"));
"#;

/// Parses each payload of a struct that declares a parameter through the schema of its declared
/// default, and each payload of `Harbour` that leaves out a key it requires.
const FILLED_DRIVER: &str = r#"import * as lib from "./lib/index.js";

const printed = [];
for (const [name, schema, payload] of FILLED) {
  const read = lib[`${schema}$SchemaDefault`].safeParse(JSON.parse(payload));
  printed.push(
    read.success
      ? `${name}: read ${JSON.stringify(read.data)}`
      : `${name}: refused ${JSON.stringify(read.error.issues)}`,
  );
}
for (const [name, payload] of REQUIRED) {
  const read = lib.Harbour$SchemaDefault.safeParse(JSON.parse(payload));
  printed.push(read.success ? `${name}: read ${JSON.stringify(read.data)}` : `${name}: refused`);
}
console.log(printed.join("\n"));
"#;

/// What the examples driver prints: the list each schema's `.meta` holds, empty where the type
/// writes no block and one value per block where it writes some.
const EXAMPLES: &str = "{\"plain\":[],\"exampled\":[{\"value\":1},{\"value\":2}],\"brand\":[],\
                        \"exampledBrand\":[\"a\",\"b\"],\"generic\":[],\"exampledGeneric\":[{\"id\":\
                        \"x\"},{\"id\":\"y\"}],\"exampledDefault\":[{\"id\":\"x\"},{\"id\":\"y\"}],\
                        \"slot\":[],\"alias\":[],\"keys\":[\"examples\"]}";

/// Loads the compiled bundle and reads the examples each schema's `.meta` holds.
const EXAMPLES_DRIVER: &str = r#"import { z } from "zod";
import {
  DeclaredExampled$Schema,
  DeclaredExampledLabel$Schema,
  DeclaredExampledPair$SchemaDefault,
  DeclaredExampledPair$SchemaFactory,
  DeclaredLabel$Schema,
  DeclaredLabelRefType$Schema,
  DeclaredLabelSlot$Schema,
  DeclaredNode$SchemaFactory,
  DeclaredPlain$Schema,
} from "./lib/index.js";

console.log(
  JSON.stringify({
    plain: DeclaredPlain$Schema.meta()?.examples,
    exampled: DeclaredExampled$Schema.meta()?.examples,
    brand: DeclaredLabel$Schema.meta()?.examples,
    exampledBrand: DeclaredExampledLabel$Schema.meta()?.examples,
    generic: DeclaredNode$SchemaFactory(z.string()).meta()?.examples,
    exampledGeneric: DeclaredExampledPair$SchemaFactory(z.number()).meta()?.examples,
    exampledDefault: DeclaredExampledPair$SchemaDefault.meta()?.examples,
    slot: DeclaredLabelSlot$Schema.meta()?.examples,
    alias: DeclaredLabelRefType$Schema.meta()?.examples,
    keys: Object.keys(DeclaredExampled$Schema.meta() ?? {}),
  }),
);
"#;

/// What the driver prints: each tuple struct takes a nested value and refuses one that is wrong
/// two levels down.
const LOADED: &str = "[true,false,true,false]";

/// Loads the compiled bundle and parses through the two tuple structs that reach themselves.
const DRIVER: &str = r#"import { z } from "zod";
import { DeclaredPlainSlots$Schema, DeclaredSlots$SchemaFactory } from "./lib/index.js";

const slots = DeclaredSlots$SchemaFactory(z.string());
console.log(
  JSON.stringify([
    DeclaredPlainSlots$Schema.safeParse(["a", [["b", []]]]).success,
    DeclaredPlainSlots$Schema.safeParse(["a", [["b", [7]]]]).success,
    slots.safeParse(["a", [["b", []]]]).success,
    slots.safeParse(["a", [[5, []]]]).success,
  ]),
);
"#;

const MODULES_VAR: &str = "TIXSCHEMA_NODE_MODULES";

/// Loads the compiled bundle and parses through the types that hold an optional one of themselves.
const OPTIONAL_DRIVER: &str = r#"import { z } from "zod";
import {
  DeclaredAhead$SchemaFactory,
  DeclaredChain$SchemaFactory,
  DeclaredMaps$SchemaFactory,
} from "./lib/index.js";

const chain = DeclaredChain$SchemaFactory(z.string());
const maps = DeclaredMaps$SchemaFactory(z.string());
const leaf = (id) => ({ held: {}, id, rows: [] });
const mapped = maps.parse({ held: { a: leaf("b") }, id: "a", maybe: null, rows: [{ k: leaf("c") }] });
const ahead = DeclaredAhead$SchemaFactory(z.string());
const absent = chain.parse({ id: "a", named: null });
const nulled = chain.parse({ id: "a", by_key: null, many: null, named: null, next: null });
const nested = {
  id: "a",
  by_key: { k: { id: "d", named: null } },
  many: [{ id: "c", named: {} }],
  named: { n: { id: "e", named: null } },
  next: { id: "b", named: null, next: { id: "f", named: null } },
};
console.log(
  JSON.stringify([
    "next" in absent && absent.next === undefined && "by_key" in absent && "many" in absent,
    nulled.next === undefined && nulled.many === undefined && nulled.by_key === undefined,
    chain.safeParse(nested).success,
    chain.safeParse({ id: "a", named: null, next: { id: 5, named: null } }).success,
    chain.safeParse({ id: "a", named: null, by_key: { k: 5 } }).success,
    chain.safeParse({ id: "a", named: { n: 5 } }).success,
    ahead.safeParse({
      id: "a",
      behind: [{ id: "f" }, { id: "g", ahead: { id: "b", behind: [] } }],
    }).success,
    ahead.safeParse({ id: "a", behind: [{ id: "f", ahead: 5 }] }).success,
    "maybe" in mapped && mapped.maybe === undefined && mapped.held.a.id === "b",
    maps.safeParse({ held: {}, id: "a", maybe: { m: leaf("d") }, rows: [] }).success,
    maps.safeParse({ held: { a: leaf(5) }, id: "a", rows: [] }).success,
    maps.safeParse({ held: {}, id: "a", maybe: { m: leaf(5) }, rows: [] }).success,
    maps.safeParse({ held: {}, id: "a", rows: [{ k: leaf(5) }] }).success,
  ]),
);
"#;

/// What that driver prints.
const OPTIONAL_LOADED: &str =
    "[true,true,true,false,false,false,true,false,true,true,false,false,false]";

const NODE_VAR: &str = "TIXSCHEMA_NODE";

/// What the driver of the two differences prints. Zod calls a field's function with `undefined`
/// for a key the payload does not hold, and with the `null` a payload holds; and it hands the
/// function of a flattened type the holder's whole object.
const UNHELD: &str = "{\"absent\":\"refused\",\"handed\":[\"undefined\",null],\"held\":{\"label\":\
                      \"unnamed\"},\"flattened\":{\"dock\":\"north\",\"keys\":[\"dock\",\"tag\"],\"tag\":\
                      \"t\"}}";

/// Loads the bundle of the `preprocess` models and parses an object without the key of a field
/// that names a function, one that holds `null` under it, and a holder of a flattened type.
const UNHELD_DRIVER: &str = r#"import * as lib from "./lib/index.js";

const said = (read) => (read.success ? read.data : "refused");
const absent = said(lib.Unnamed$Schema.safeParse({}));
const held = said(lib.Unnamed$Schema.safeParse({ label: null }));
const flattened = said(lib.Pinned$Schema.safeParse({ dock: "north", tag: "t" }));
console.log(JSON.stringify({ absent, handed: lib.handed, held, flattened }));
"#;

/// The second package: it names the first only through `./lib`, where the declarations are.
const CONSUMER: &str = r#"import {
  type DeclaredAhead,
  DeclaredAhead$SchemaFactory,
  type DeclaredChain,
  DeclaredChain$SchemaFactory,
  type DeclaredChoice,
  DeclaredChoice$SchemaFactory,
  DeclaredHolder$SchemaFactory,
  type DeclaredMaps,
  DeclaredMaps$SchemaDefault,
  DeclaredMaps$SchemaFactory,
  type DeclaredNode,
  DeclaredNode$SchemaDefault,
  DeclaredNode$SchemaFactory,
  type DeclaredSlots,
  DeclaredSlots$SchemaFactory,
} from "./lib/index.js";
import { z } from "zod";

const node = DeclaredNode$SchemaFactory(z.string());

export function childrenFromFactory(parsed: z.infer<typeof node>): DeclaredNode<string>[] {
  return parsed.children;
}

export function childrenFromDefault(
  parsed: z.infer<typeof DeclaredNode$SchemaDefault>
): DeclaredNode<string>[] {
  return parsed.children;
}

const holder = DeclaredHolder$SchemaFactory(z.string());

export function heldFromFactory(parsed: z.infer<typeof holder>): DeclaredNode<string>[] {
  return parsed.nodes;
}

const choice = DeclaredChoice$SchemaFactory(z.string());

export function chosenFromFactory(parsed: z.infer<typeof choice>): DeclaredChoice<string> {
  return parsed;
}

const slots = DeclaredSlots$SchemaFactory(z.string());

export function slotsFromFactory(parsed: z.infer<typeof slots>): DeclaredSlots<string>[] {
  return parsed[1];
}

const chain = DeclaredChain$SchemaFactory(z.string());

export function nextFromFactory(parsed: z.infer<typeof chain>): DeclaredChain<string> | undefined {
  return parsed.next;
}

export function keyedFromFactory(
  parsed: z.infer<typeof chain>
): Partial<Record<string, DeclaredChain<string>>> | undefined {
  return parsed.by_key;
}

const maps = DeclaredMaps$SchemaFactory(z.string());

export function heldMapFromFactory(
  parsed: z.infer<typeof maps>
): Partial<Record<string, DeclaredMaps<string>>> {
  return parsed.held;
}

export function maybeMapFromDefault(
  parsed: z.infer<typeof DeclaredMaps$SchemaDefault>
): Partial<Record<string, DeclaredMaps<string>>> | undefined {
  return parsed.maybe;
}

export function rowsFromFactory(
  parsed: z.infer<typeof maps>
): Partial<Record<string, DeclaredMaps<string>>>[] {
  return parsed.rows;
}

const ahead = DeclaredAhead$SchemaFactory(z.string());

export function aheadFromFactory(
  parsed: z.infer<typeof ahead>
): (DeclaredAhead<string> | undefined)[] {
  return parsed.behind.map((held) => held.ahead);
}
"#;

const CONSUMER_PROJECT: &str = r#"{
  "compilerOptions": {
    "strict": true,
    "module": "nodenext",
    "moduleResolution": "nodenext",
    "target": "ES2022",
    "skipLibCheck": true,
    "noEmit": true
  },
  "files": ["consumer.ts"]
}
"#;

const PACKAGE: &str = r#"{ "type": "module", "private": true }
"#;

/// What a consumer's module writes ahead of the generated text. `z` is bound as a constant, which
/// names no type: the weakest binding a consumer's own module may hand the bundle.
const PREAMBLE: &str = "import * as zod from \"zod\";\nimport type { ZodType, $brand } from \
                        \"zod\";\nimport type { SomeType } from \"zod/v4/core\";\n\nconst z = zod;";

/// The function `CodedCrate`'s check names, written once over every filling, as the README writes
/// the function of a type with a parameter.
const WITH_ITS_CODE: &str = r#"import type { ParsePayload } from "zod/v4/core";

export function withItsCode<CodeType>(value: CodedCrate<CodeType>, ctx: ParsePayload<CodedCrate<CodeType>>): void {
  if (value.codes.length === 0) {
    ctx.issues.push({ code: "custom", message: "a crate holds at least one code", path: ["codes"], input: value });
  }
}"#;

/// The first package: the bundle, compiled with declarations into `lib`.
const PROJECT: &str = r#"{
  "compilerOptions": {
    "strict": true,
    "declaration": true,
    "module": "nodenext",
    "moduleResolution": "nodenext",
    "target": "ES2022",
    "skipLibCheck": true,
    "rootDir": "src",
    "outDir": "lib"
  },
  "files": ["src/index.ts"]
}
"#;

/// Each payload of an externally tagged enum, the schema it is parsed through and the type it is
/// assigned to, and whether serde, Zod and TypeScript take it: one variant's entry, the name of a
/// variant that holds none, and the entries of two variants, which none of the three takes.
const HATCHES: [(&str, &str, &str, bool); 5] = [
    (
        "DeclaredHatch$Schema",
        "DeclaredHatch",
        r#"{"Bay":{"label":"n"}}"#,
        true,
    ),
    ("DeclaredHatch$Schema", "DeclaredHatch", r#""Calm""#, true),
    (
        "DeclaredHatch$Schema",
        "DeclaredHatch",
        r#"{"Bay":{"label":"n"},"Dock":{"label":"s"}}"#,
        false,
    ),
    (
        "DeclaredLading$SchemaDefault",
        "DeclaredLading<string>",
        r#"{"held_fast":"a"}"#,
        true,
    ),
    (
        "DeclaredLading$SchemaDefault",
        "DeclaredLading<string>",
        r#"{"held_fast":"a","loose":{"cargo":"b"}}"#,
        false,
    ),
];

/// Each payload of [`DeclaredYaw`], and whether serde and its Zod schema take it: each variant's
/// entry, a unit variant's over `null`, as serde writes it, and over a number. [`DeclaredSway`]
/// and [`DeclaredDrift`] take each but the unit variant's, which a check refuses.
const YAWS: [(&str, bool); 4] = [
    (r#"{"note":"a","Calm":null}"#, true),
    (r#"{"note":"a","Bay":{"label":"n"}}"#, true),
    (r#"{"note":"a","Held":"x"}"#, true),
    (r#"{"note":"a","Calm":1}"#, false),
];

/// Each payload of [`DeclaredMooring`], and whether serde, its Zod schema and its TypeScript type
/// take it: each variant's entry beside the tag, a unit variant's over `null` among them, and not
/// the tag alone, the entries of two variants or a unit variant's entry over a number.
const MOORINGS: [(&str, bool); 9] = [
    (r#"{"kind":"Hatched","Bay":{"label":"n"}}"#, true),
    (r#"{"kind":"Hatched","Calm":null}"#, true),
    (r#"{"kind":"Hatched"}"#, false),
    (
        r#"{"kind":"Hatched","Bay":{"label":"n"},"Dock":{"label":"s"}}"#,
        false,
    ),
    (r#"{"kind":"Hatched","Calm":1}"#, false),
    (r#"{"kind":"Idle"}"#, true),
    (r#"{"kind":"Tacked","Calm":null}"#, true),
    (r#"{"kind":"Tacked","Held":"x"}"#, true),
    (r#"{"kind":"Tacked","Calm":1}"#, false),
];

/// Each payload of [`DeclaredStamp`], and whether serde, its Zod schema and its TypeScript type
/// take it: an `ObjectId`'s `$oid` key beside the tag, as serde writes it, and not the tag alone or
/// the key under another name.
#[cfg(feature = "mongodb")]
const STAMPS: [(&str, bool); 4] = [
    (r#"{"kind":"Held","$oid":"507f1f77bcf86cd799439011"}"#, true),
    (r#"{"kind":"Held"}"#, false),
    (r#"{"kind":"Held","oid":"507f1f77bcf86cd799439011"}"#, false),
    (r#"{"kind":"Idle"}"#, true),
];

/// Each payload of an externally tagged enum that declares a parameter, written with an argument,
/// flattened beside a member and held by the lone slot of a variant under a tag alone, the schema
/// it is parsed through and the type it is assigned to, and whether serde, Zod and TypeScript take
/// it: each variant's entry at the argument, a unit variant's over `null` among them, and not the
/// member or the tag alone, an entry of another type, or beside the tag the entries of two
/// variants.
const LADINGS: [(&str, &str, &str, bool); 22] = [
    (
        "DeclaredBerth$Schema",
        "DeclaredBerth",
        r#"{"note":"n","empty":null}"#,
        true,
    ),
    (
        "DeclaredBerth$Schema",
        "DeclaredBerth",
        r#"{"note":"n","held_fast":7}"#,
        true,
    ),
    (
        "DeclaredBerth$Schema",
        "DeclaredBerth",
        r#"{"note":"n","loose":{"cargo":7}}"#,
        true,
    ),
    (
        "DeclaredBerth$Schema",
        "DeclaredBerth",
        r#"{"note":"n"}"#,
        false,
    ),
    (
        "DeclaredBerth$Schema",
        "DeclaredBerth",
        r#"{"held_fast":7}"#,
        false,
    ),
    (
        "DeclaredBerth$Schema",
        "DeclaredBerth",
        r#"{"note":"n","held_fast":"x"}"#,
        false,
    ),
    (
        "DeclaredQuay$SchemaDefault",
        "DeclaredQuay<boolean>",
        r#"{"note":"n","empty":null}"#,
        true,
    ),
    (
        "DeclaredQuay$SchemaDefault",
        "DeclaredQuay<boolean>",
        r#"{"note":"n","held_fast":true}"#,
        true,
    ),
    (
        "DeclaredQuay$SchemaDefault",
        "DeclaredQuay<boolean>",
        r#"{"note":"n","held_fast":7}"#,
        false,
    ),
    (
        "DeclaredPier$Schema",
        "DeclaredPier",
        r#"{"note":"n","empty":null}"#,
        true,
    ),
    (
        "DeclaredPier$Schema",
        "DeclaredPier",
        r#"{"note":"n","held_fast":7}"#,
        true,
    ),
    (
        "DeclaredPier$Schema",
        "DeclaredPier",
        r#"{"note":"n","held_fast":true}"#,
        false,
    ),
    (
        "DeclaredWharf$Schema",
        "DeclaredWharf",
        r#"{"kind":"Laden","empty":null}"#,
        true,
    ),
    (
        "DeclaredWharf$Schema",
        "DeclaredWharf",
        r#"{"kind":"Laden","held_fast":7}"#,
        true,
    ),
    (
        "DeclaredWharf$Schema",
        "DeclaredWharf",
        r#"{"kind":"Laden"}"#,
        false,
    ),
    (
        "DeclaredWharf$Schema",
        "DeclaredWharf",
        r#"{"kind":"Laden","empty":null,"held_fast":7}"#,
        false,
    ),
    (
        "DeclaredWharf$Schema",
        "DeclaredWharf",
        r#"{"kind":"Laden","held_fast":"x"}"#,
        false,
    ),
    (
        "DeclaredWharf$Schema",
        "DeclaredWharf",
        r#"{"kind":"Held","held_fast":true}"#,
        true,
    ),
    (
        "DeclaredWharf$Schema",
        "DeclaredWharf",
        r#"{"kind":"Held","empty":null}"#,
        true,
    ),
    (
        "DeclaredMarina$SchemaDefault",
        "DeclaredMarina<boolean>",
        r#"{"kind":"Laden","empty":null}"#,
        true,
    ),
    (
        "DeclaredMarina$SchemaDefault",
        "DeclaredMarina<boolean>",
        r#"{"kind":"Laden","held_fast":true}"#,
        true,
    ),
    (
        "DeclaredMarina$SchemaDefault",
        "DeclaredMarina<boolean>",
        r#"{"kind":"Laden","held_fast":7}"#,
        false,
    ),
];

/// Each payload of an untagged enum that declares a parameter, written with an argument and
/// flattened beside a member, the schema it is parsed through and the type it is assigned to, and
/// whether serde, Zod and TypeScript take it: each member at the argument, directly and at the
/// holder's own parameter, and not the member alone, a member of another type, or no member.
const LANDINGS: [(&str, &str, &str, bool); 12] = [
    (
        "DeclaredLanding$Schema",
        "DeclaredLanding",
        r#"{"note":"n","label":"l"}"#,
        true,
    ),
    (
        "DeclaredLanding$Schema",
        "DeclaredLanding",
        r#"{"note":"n","count":3}"#,
        true,
    ),
    (
        "DeclaredLanding$Schema",
        "DeclaredLanding",
        r#"{"note":"n"}"#,
        false,
    ),
    (
        "DeclaredLanding$Schema",
        "DeclaredLanding",
        r#"{"note":"n","label":3}"#,
        false,
    ),
    (
        "DeclaredJetty$SchemaDefault",
        "DeclaredJetty<DeclaredCarton>",
        r#"{"note":"n","label":"l"}"#,
        true,
    ),
    (
        "DeclaredJetty$SchemaDefault",
        "DeclaredJetty<DeclaredCarton>",
        r#"{"note":"n","count":3}"#,
        true,
    ),
    (
        "DeclaredJetty$SchemaDefault",
        "DeclaredJetty<DeclaredCarton>",
        r#"{"note":"n","label":3}"#,
        false,
    ),
    (
        "DeclaredSlipway$Schema",
        "DeclaredSlipway",
        r#"{"note":"n","load":3}"#,
        true,
    ),
    (
        "DeclaredSlipway$Schema",
        "DeclaredSlipway",
        r#"{"note":"n","count":3}"#,
        true,
    ),
    (
        "DeclaredSlipway$Schema",
        "DeclaredSlipway",
        r#"{"note":"n","load":"x"}"#,
        false,
    ),
    (
        "DeclaredSlipway$Schema",
        "DeclaredSlipway",
        r#"{"note":"n","load":true}"#,
        false,
    ),
    (
        "DeclaredSlipway$Schema",
        "DeclaredSlipway",
        r#"{"note":"n"}"#,
        false,
    ),
];

/// Each payload of an untagged enum of members of named fields, under an alias flattened beside a
/// member, the schema it is parsed through and the type it is assigned to, and whether Zod and
/// TypeScript take it: each member serde writes, through an alias of the enum and through an alias
/// that declares a parameter of its own, written with an argument, and not the keys of two members,
/// no member, or a member of another type.
const ALIASED_PICKS: [(&str, &str, &str, bool); 8] = [
    (
        "DeclaredDinghy$Schema",
        "DeclaredDinghy",
        r#"{"note":"n","oars":2}"#,
        true,
    ),
    (
        "DeclaredDinghy$Schema",
        "DeclaredDinghy",
        r#"{"note":"n","sail":"s"}"#,
        true,
    ),
    (
        "DeclaredDinghy$Schema",
        "DeclaredDinghy",
        r#"{"note":"n","oars":2,"sail":"s"}"#,
        false,
    ),
    (
        "DeclaredDinghy$Schema",
        "DeclaredDinghy",
        r#"{"note":"n"}"#,
        false,
    ),
    (
        "DeclaredGangway$Schema",
        "DeclaredGangway",
        r#"{"note":"n","load":true}"#,
        true,
    ),
    (
        "DeclaredGangway$Schema",
        "DeclaredGangway",
        r#"{"note":"n","count":3}"#,
        true,
    ),
    (
        "DeclaredGangway$Schema",
        "DeclaredGangway",
        r#"{"note":"n","load":true,"count":3}"#,
        false,
    ),
    (
        "DeclaredGangway$Schema",
        "DeclaredGangway",
        r#"{"note":"n","load":3}"#,
        false,
    ),
];

static STOOD_DOWN: Once = Once::new();

/// A struct whose fields bound a brand over a string, one declared below it and an optional one,
/// and count a list of one declared above.
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredBounded {
    #[model_schema_prop(maxLength = 3)]
    pub later: DeclaredLabel,
    #[model_schema_prop(minLength = 2)]
    pub many: Vec<DeclaredPath>,
    #[model_schema_prop(minLength = 1, pattern = "^[a-z.]*$")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort: Option<DeclaredPath>,
}

#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeclaredLabel(pub String);

#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeclaredPath(pub String);

/// A date beside a boolean in an untagged enum, the date first: an input that is no date has to
/// reach the member after it.
#[cfg(feature = "chrono")]
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DeclaredMark {
    At(DateTime<Utc>),
    Set(bool),
}

/// A date in each position a schema writes one: required, in an `Option`, in that untagged enum,
/// under `nullable`, and among the items of a list that may hold `null`.
#[cfg(feature = "chrono")]
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredStamped {
    pub at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended: Option<DateTime<Utc>>,
    pub mark: DeclaredMark,
    #[model_schema_prop(nullable)]
    pub paused: Option<DateTime<Utc>>,
    pub seen: Vec<Option<DateTime<Utc>>>,
}

/// `()` as a brand's value, which serde writes as `null`.
#[model_schema(no_display)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeclaredNothing(());

/// `()` in each place a value is held: a member, a list's item, a tuple's position and a brand.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclaredUnits {
    pub listed: Vec<()>,
    pub nothing: DeclaredNothing,
    pub paired: (u32, ()),
    pub unit: (),
}

/// A struct that writes no example.
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredPlain {
    pub value: u32,
}

/// A struct that writes two.
/// ```rust example
/// DeclaredExampled { value: 1 }
/// ```
/// ```rust example
/// DeclaredExampled { value: 2 }
/// ```
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredExampled {
    pub value: u32,
}

/// A brand that writes two.
/// ```rust example
/// DeclaredExampledLabel("a".to_owned())
/// ```
/// ```rust example
/// DeclaredExampledLabel("b".to_owned())
/// ```
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeclaredExampledLabel(pub String);

/// A generic struct that writes two.
/// ```rust example
/// DeclaredExampledPair { id: "x".to_owned() }
/// ```
/// ```rust example
/// DeclaredExampledPair { id: "y".to_owned() }
/// ```
#[model_schema(default_types(IdType = String))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredExampledPair<IdType> {
    pub id: IdType,
}

/// A one-slot tuple struct over a brand, which publishes the brand's own binding.
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredLabelSlot(pub DeclaredLabel);

/// An alias of a brand, which publishes it too.
#[model_schema()]
pub type DeclaredLabelRef = DeclaredLabel;

/// One of a cycle of two, declared above the type its list holds.
#[model_schema(default_types(IdType = String))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredAhead<IdType> {
    pub behind: Vec<DeclaredBehind<IdType>>,
    pub id: IdType,
}

/// The other of that cycle, holding an optional one of the type declared above it.
#[model_schema(default_types(IdType = String))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredBehind<IdType> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ahead: Option<Box<DeclaredAhead<IdType>>>,
    pub id: IdType,
}

/// A generic type holding an optional one of itself, an optional list and an optional map of
/// itself, and a map of itself that is `null` where there is none.
#[model_schema(default_types(IdType = String))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredChain<IdType> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by_key: Option<HashMap<String, Self>>,
    pub id: IdType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub many: Option<Vec<Self>>,
    #[model_schema_prop(nullable)]
    pub named: Option<HashMap<String, Self>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<Box<Self>>,
}

/// A generic type holding a map of itself, an optional map of itself below it, and a list of maps
/// of itself: the three a builder cannot read its own factory's type for.
#[model_schema(default_types(IdType = String))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredMaps<IdType> {
    pub held: HashMap<String, Self>,
    pub id: IdType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maybe: Option<HashMap<String, Self>>,
    pub rows: Vec<HashMap<String, Self>>,
}

/// An internally tagged enum whose variant holds a list of the enum.
#[model_schema(default_types(IdType = String))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum DeclaredChoice<IdType> {
    Group { items: Vec<Self> },
    Leaf { id: IdType },
}

/// A generic type that embeds the recursive one.
#[model_schema(default_types(IdType = String))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredHolder<IdType> {
    pub nodes: Vec<DeclaredNode<IdType>>,
}

#[model_schema(default_types(IdType = String))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredNode<IdType> {
    pub children: Vec<Self>,
    pub id: IdType,
}

/// A tuple struct whose second slot holds a list of the struct.
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredPlainSlots(pub String, pub Vec<Self>);

/// The same under a type parameter.
#[model_schema(default_types(IdType = String))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredSlots<IdType>(pub IdType, pub Vec<Self>);

/// A type with a parameter that names a check and holds the parameter itself in a field.
#[model_schema(default_types(CodeType = String), check = ["withItsCode"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CodedCrate<CodeType> {
    pub code: CodeType,
    pub codes: Vec<CodeType>,
}

/// Each model a payload of the ranges is read as: its name, whether serde reads a JSON as it, and
/// its JSON Schema.
#[cfg(feature = "jsonschema")]
type RangedModel = (&'static str, fn(&str) -> bool, fn() -> serde_json::Value);

/// The models the payloads of a `Result` are read as: the name, whether the type judges a model it
/// holds, what serde's read prints, and the JSON Schema.
#[cfg(feature = "jsonschema")]
type ResultedModel = (
    &'static str,
    bool,
    fn(&str, &str, bool) -> (bool, String),
    fn() -> serde_json::Value,
);

/// An externally tagged enum of two variants that hold a value and one that holds none.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeclaredHatch {
    Bay { label: String },
    Calm,
    Dock { label: String },
}

/// An externally tagged enum that declares a parameter, its variants under the enum's
/// `rename_all`.
#[model_schema(default_types(Cargo = String))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeclaredLading<Cargo> {
    Empty,
    HeldFast(Cargo),
    Loose { cargo: Cargo },
}

/// An externally tagged enum flattened under a member that names a function.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeclaredTack {
    Bay { label: String },
    Calm,
    Held(String),
}

/// [`DeclaredTack`] flattened beside a member of its holder's own, under a function that answers
/// the payload as it was handed: serde writes a unit variant as its name over `null`.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclaredYaw {
    pub note: String,
    #[model_schema_prop(preprocess = ["asGiven"])]
    #[serde(flatten)]
    pub tack: DeclaredTack,
}

/// [`DeclaredTack`] flattened beside a member of its holder's own, under a check that refuses its
/// unit variant: serde writes the variant as its name over `null`, and the check is handed the
/// enum's value.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclaredSway {
    pub note: String,
    #[model_schema_prop(check = ["notCalm"])]
    #[serde(flatten)]
    pub tack: DeclaredTack,
}

/// An externally tagged enum that names a check on itself, which refuses its unit variant.
#[model_schema(check = ["notCalmLull"])]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeclaredLull {
    Bay { label: String },
    Calm,
    Held(String),
}

/// [`DeclaredLull`] flattened beside a member of its holder's own: its check is run on the
/// enum's value, its unit variant's bare name.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclaredDrift {
    #[serde(flatten)]
    pub lull: DeclaredLull,
    pub note: String,
}

/// [`DeclaredHatch`] in the lone slot of a variant under a tag alone, and [`DeclaredTack`] there
/// under a function that answers the payload as it was handed: serde writes each variant beside
/// the tag as an entry, a unit variant's over `null`.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum DeclaredMooring {
    Hatched(DeclaredHatch),
    Idle,
    Tacked(#[model_schema_prop(preprocess = ["asGiven"])] DeclaredTack),
}

/// An `ObjectId` in the lone slot of a variant under a tag alone, which serde writes beside the
/// tag as its `$oid` key, as it writes a flattened one.
#[cfg(feature = "mongodb")]
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum DeclaredStamp {
    Held(ObjectId),
    Idle,
}

/// [`DeclaredLading`] written with an argument and flattened beside a member of its holder's own.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclaredBerth {
    #[serde(flatten)]
    pub lading: DeclaredLading<u32>,
    pub note: String,
}

/// [`DeclaredLading`] flattened at its holder's own parameter.
#[model_schema(default_types(Stowed = bool))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclaredQuay<Stowed> {
    #[serde(flatten)]
    pub lading: DeclaredLading<Stowed>,
    pub note: String,
}

/// An alias of [`DeclaredLading`] that declares a parameter of its own.
#[model_schema(default_types(Stowed = String))]
pub type DeclaredHold<Stowed> = DeclaredLading<Stowed>;

/// [`DeclaredHold`] written with an argument and flattened beside a member of its holder's own.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclaredPier {
    #[serde(flatten)]
    pub hold: DeclaredHold<u32>,
    pub note: String,
}

/// [`DeclaredLading`] and [`DeclaredHold`] written with an argument in the lone slot of a variant
/// under a tag alone: serde writes each variant beside the tag as an entry, a unit variant's over
/// `null`.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum DeclaredWharf {
    Held(DeclaredHold<bool>),
    Laden(DeclaredLading<u32>),
}

/// [`DeclaredLading`] at its holder's own parameter in the lone slot of a variant under a tag
/// alone.
#[model_schema(default_types(Stowed = bool))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum DeclaredMarina<Stowed> {
    Laden(DeclaredLading<Stowed>),
}

/// The struct an untagged enum that declares a parameter is filled with.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclaredCarton {
    pub label: String,
}

/// An untagged enum that declares a parameter: a member that is the parameter, and one of named
/// fields.
#[model_schema(default_types(Cargo = DeclaredCarton))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DeclaredEither<Cargo> {
    Counted { count: u32 },
    Held(Cargo),
}

/// [`DeclaredEither`] written with an argument and flattened beside a member of its holder's own.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclaredLanding {
    #[serde(flatten)]
    pub either: DeclaredEither<DeclaredCarton>,
    pub note: String,
}

/// [`DeclaredEither`] flattened at its holder's own parameter.
#[model_schema(default_types(Stowed = DeclaredCarton))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclaredJetty<Stowed> {
    #[serde(flatten)]
    pub either: DeclaredEither<Stowed>,
    pub note: String,
}

/// An untagged enum that declares a parameter, of two members of named fields, each closed to the
/// other's key where it is flattened.
#[model_schema(default_types(Cargo = String))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DeclaredPick<Cargo> {
    Carried { load: Cargo },
    Counted { count: u32 },
}

/// [`DeclaredPick`] written with an argument and flattened beside a member of its holder's own.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclaredSlipway {
    pub note: String,
    #[serde(flatten)]
    pub pick: DeclaredPick<u32>,
}

/// An untagged enum of two members of named fields, each closed to the other's key where it is
/// flattened.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DeclaredSkiff {
    Rowed { oars: u32 },
    Sailed { sail: String },
}

/// [`DeclaredSkiff`] under an alias.
#[model_schema()]
pub type DeclaredSkiffAlias = DeclaredSkiff;

/// [`DeclaredSkiffAlias`] flattened beside a member of its holder's own.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclaredDinghy {
    pub note: String,
    #[serde(flatten)]
    pub skiff: DeclaredSkiffAlias,
}

/// [`DeclaredPick`] under an alias that declares a parameter of its own.
#[model_schema(default_types(Stowed = String))]
pub type DeclaredPickOf<Stowed> = DeclaredPick<Stowed>;

/// [`DeclaredPickOf`] written with an argument and flattened beside a member of its holder's own.
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclaredGangway {
    pub note: String,
    #[serde(flatten)]
    pub pick: DeclaredPickOf<bool>,
}

fn bundle() -> String {
    [
        PREAMBLE.to_owned(),
        DeclaredPath::ts_definition(),
        DeclaredPath::zod_schema(),
        DeclaredBounded::ts_definition(),
        DeclaredBounded::zod_schema(),
        DeclaredLabel::ts_definition(),
        DeclaredLabel::zod_schema(),
        DeclaredNode::<String>::ts_definition(),
        DeclaredNode::<String>::zod_schema(),
        DeclaredHolder::<String>::ts_definition(),
        DeclaredHolder::<String>::zod_schema(),
        DeclaredChoice::<String>::ts_definition(),
        DeclaredChoice::<String>::zod_schema(),
        DeclaredPlainSlots::ts_definition(),
        DeclaredPlainSlots::zod_schema(),
        DeclaredSlots::<String>::ts_definition(),
        DeclaredSlots::<String>::zod_schema(),
        DeclaredChain::<String>::ts_definition(),
        DeclaredChain::<String>::zod_schema(),
        DeclaredMaps::<String>::ts_definition(),
        DeclaredMaps::<String>::zod_schema(),
        DeclaredAhead::<String>::ts_definition(),
        DeclaredAhead::<String>::zod_schema(),
        DeclaredBehind::<String>::ts_definition(),
        DeclaredBehind::<String>::zod_schema(),
        DeclaredPlain::ts_definition(),
        DeclaredPlain::zod_schema(),
        DeclaredExampled::ts_definition(),
        DeclaredExampled::zod_schema(),
        DeclaredExampledLabel::ts_definition(),
        DeclaredExampledLabel::zod_schema(),
        DeclaredExampledPair::<String>::ts_definition(),
        DeclaredExampledPair::<String>::zod_schema(),
        DeclaredLabelSlot::ts_definition(),
        DeclaredLabelSlot::zod_schema(),
        declared_label_ref_schema::Schema::ts_definition(),
        declared_label_ref_schema::Schema::zod_schema(),
    ]
    .into_iter()
    .chain(dated())
    .chain(checks())
    .collect::<Vec<_>>()
    .join("\n\n")
}

/// The payload as it was handed, which [`DeclaredYaw`] names.
const fn as_given(payload: serde_json::Value) -> serde_json::Value {
    payload
}

/// Refuses [`DeclaredLull::Calm`], which [`DeclaredLull`] names on itself.
fn not_calm_lull(value: &DeclaredLull) -> Result<(), Vec<String>> {
    if matches!(value, DeclaredLull::Calm) {
        Err(vec!["a calm lull is refused".to_owned()])
    } else {
        Ok(())
    }
}

/// Refuses [`DeclaredTack::Calm`], which [`DeclaredSway`] names.
fn not_calm(value: &DeclaredTack) -> Result<(), Vec<String>> {
    if matches!(value, DeclaredTack::Calm) {
        Err(vec!["a calm tack is refused".to_owned()])
    } else {
        Ok(())
    }
}

/// The externally tagged enums of [`HATCHES`] and [`YAWS`]: a bundle of their own, compiled with
/// `tsc --strict` before it is loaded.
fn hatches_bundle() -> String {
    [
        PREAMBLE.to_owned(),
        DeclaredHatch::ts_definition(),
        DeclaredHatch::zod_schema(),
        DeclaredLading::<String>::ts_definition(),
        DeclaredLading::<String>::zod_schema(),
        AS_GIVEN_OR_NULL.to_owned(),
        DeclaredTack::ts_definition(),
        DeclaredTack::zod_schema(),
        DeclaredYaw::ts_definition(),
        DeclaredYaw::zod_schema(),
        NOT_CALM.to_owned(),
        DeclaredSway::ts_definition(),
        DeclaredSway::zod_schema(),
        DeclaredLull::ts_definition(),
        DeclaredLull::zod_schema(),
        DeclaredDrift::ts_definition(),
        DeclaredDrift::zod_schema(),
        DeclaredMooring::ts_definition(),
        DeclaredMooring::zod_schema(),
        DeclaredBerth::ts_definition(),
        DeclaredBerth::zod_schema(),
        DeclaredQuay::<bool>::ts_definition(),
        DeclaredQuay::<bool>::zod_schema(),
        declared_hold_schema::Schema::ts_definition(),
        declared_hold_schema::Schema::zod_schema(),
        DeclaredPier::ts_definition(),
        DeclaredPier::zod_schema(),
        DeclaredWharf::ts_definition(),
        DeclaredWharf::zod_schema(),
        DeclaredMarina::<bool>::ts_definition(),
        DeclaredMarina::<bool>::zod_schema(),
        DeclaredCarton::ts_definition(),
        DeclaredCarton::zod_schema(),
        DeclaredEither::<DeclaredCarton>::ts_definition(),
        DeclaredEither::<DeclaredCarton>::zod_schema(),
        DeclaredLanding::ts_definition(),
        DeclaredLanding::zod_schema(),
        DeclaredJetty::<DeclaredCarton>::ts_definition(),
        DeclaredJetty::<DeclaredCarton>::zod_schema(),
        DeclaredPick::<String>::ts_definition(),
        DeclaredPick::<String>::zod_schema(),
        DeclaredSlipway::ts_definition(),
        DeclaredSlipway::zod_schema(),
        DeclaredSkiff::ts_definition(),
        DeclaredSkiff::zod_schema(),
        declared_skiff_alias_schema::Schema::ts_definition(),
        declared_skiff_alias_schema::Schema::zod_schema(),
        DeclaredDinghy::ts_definition(),
        DeclaredDinghy::zod_schema(),
        declared_pick_of_schema::Schema::ts_definition(),
        declared_pick_of_schema::Schema::zod_schema(),
        DeclaredGangway::ts_definition(),
        DeclaredGangway::zod_schema(),
    ]
    .join("\n\n")
}

/// [`DeclaredStamp`]: a bundle of its own, compiled with `tsc --strict` before it is loaded,
/// beside the consumer's own `ObjectId`.
#[cfg(feature = "mongodb")]
fn stamps_bundle() -> String {
    [
        PREAMBLE.to_owned(),
        "type ObjectId = { $oid: string };".to_owned(),
        DeclaredStamp::ts_definition(),
        DeclaredStamp::zod_schema(),
    ]
    .join("\n\n")
}

/// The types a check is written on, and the functions their checks name.
fn checks() -> Vec<String> {
    vec![
        checked::Parcel::ts_definition(),
        checked::Parcel::zod_schema(),
        checked::Manifest::ts_definition(),
        checked::Manifest::zod_schema(),
        checked::Route::ts_definition(),
        checked::Route::zod_schema(),
        checked::Stack::ts_definition(),
        checked::Stack::zod_schema(),
        checked::ClosedDock::ts_definition(),
        checked::ClosedDock::zod_schema(),
        checked::Shipment::ts_definition(),
        checked::Shipment::zod_schema(),
        checked::Transfer::ts_definition(),
        checked::Transfer::zod_schema(),
        checked::Handover::ts_definition(),
        checked::Handover::zod_schema(),
        checked::Reference::ts_definition(),
        checked::Reference::zod_schema(),
        checked::Grade::ts_definition(),
        checked::Grade::zod_schema(),
        checked::Code::ts_definition(),
        checked::Code::zod_schema(),
        checked::Crate::<String>::ts_definition(),
        checked::Crate::<String>::zod_schema(),
        checked::Batch::<String>::ts_definition(),
        checked::Batch::<String>::zod_schema(),
        checked::Ledger::<String>::ts_definition(),
        checked::Ledger::<String>::zod_schema(),
        checked::Pallet::ts_definition(),
        checked::Pallet::zod_schema(),
        checked::places::Package::ts_definition(),
        checked::places::Package::zod_schema(),
        checked::places::Docket::ts_definition(),
        checked::places::Docket::zod_schema(),
        checked::places::Bundle::ts_definition(),
        checked::places::Bundle::zod_schema(),
        checked::places::Lone::ts_definition(),
        checked::places::Lone::zod_schema(),
        checked::places::Strapped::ts_definition(),
        checked::places::Strapped::zod_schema(),
        checked::places::Crane::ts_definition(),
        checked::places::Crane::zod_schema(),
        checked::places::Hoist::ts_definition(),
        checked::places::Hoist::zod_schema(),
        checked::places::Freight::ts_definition(),
        checked::places::Freight::zod_schema(),
        checked::places::Seal::ts_definition(),
        checked::places::Seal::zod_schema(),
        checked::places::Haul::ts_definition(),
        checked::places::Haul::zod_schema(),
        checked::places::Lift::ts_definition(),
        checked::places::Lift::zod_schema(),
        checked::places::Load::ts_definition(),
        checked::places::Load::zod_schema(),
        checked::places::Mooring::ts_definition(),
        checked::places::Mooring::zod_schema(),
        checked::places::Measured::ts_definition(),
        checked::places::Measured::zod_schema(),
        checked::places::Origin::ts_definition(),
        checked::places::Origin::zod_schema(),
        checked::places::Departure::ts_definition(),
        checked::places::Departure::zod_schema(),
        CHECKS.to_owned(),
    ]
}

/// The types a `preprocess` is written on, the types that hold them and the functions they name:
/// a bundle of its own, several of its names being those of the types a check is written on.
fn corrections() -> String {
    [
        PREAMBLE.to_owned(),
        corrected::Parcel::ts_definition(),
        corrected::Parcel::zod_schema(),
        corrected::Tagged::ts_definition(),
        corrected::Tagged::zod_schema(),
        corrected::Pinned::ts_definition(),
        corrected::Pinned::zod_schema(),
        corrected::Carrier::ts_definition(),
        corrected::Carrier::zod_schema(),
        corrected::ClosedDock::ts_definition(),
        corrected::ClosedDock::zod_schema(),
        corrected::Consigned::ts_definition(),
        corrected::Consigned::zod_schema(),
        corrected::Crate::<String>::ts_definition(),
        corrected::Crate::<String>::zod_schema(),
        corrected::Crated::ts_definition(),
        corrected::Crated::zod_schema(),
        corrected::Delivery::ts_definition(),
        corrected::Delivery::zod_schema(),
        corrected::Either::ts_definition(),
        corrected::Either::zod_schema(),
        corrected::Grade::ts_definition(),
        corrected::Grade::zod_schema(),
        corrected::Handover::ts_definition(),
        corrected::Handover::zod_schema(),
        corrected::Label::ts_definition(),
        corrected::Label::zod_schema(),
        corrected::Manifest::ts_definition(),
        corrected::Manifest::zod_schema(),
        corrected::Node::ts_definition(),
        corrected::Node::zod_schema(),
        corrected::Reference::ts_definition(),
        corrected::Reference::zod_schema(),
        corrected::Routed::ts_definition(),
        corrected::Routed::zod_schema(),
        corrected::Shelf::ts_definition(),
        corrected::Shelf::zod_schema(),
        corrected::Shipment::ts_definition(),
        corrected::Shipment::zod_schema(),
        corrected::Slot::ts_definition(),
        corrected::Slot::zod_schema(),
        corrected::Stack::ts_definition(),
        corrected::Stack::zod_schema(),
        corrected::Transfer::ts_definition(),
        corrected::Transfer::zod_schema(),
        corrected::Unnamed::ts_definition(),
        corrected::Unnamed::zod_schema(),
        corrected::Weighed::ts_definition(),
        corrected::Weighed::zod_schema(),
        corrected::Wrapped::ts_definition(),
        corrected::Wrapped::zod_schema(),
        CORRECTIONS.to_owned(),
    ]
    .join("\n\n")
}

/// The types a key under `#[serde(default)]` is written on: a bundle of its own, compiled with
/// `tsc --strict` before it is loaded.
fn defaults() -> String {
    let written = [
        PREAMBLE.to_owned(),
        defaulted::Gangway::ts_definition(),
        defaulted::Gangway::zod_schema(),
        defaulted::Wharf::ts_definition(),
        defaulted::Wharf::zod_schema(),
        defaulted::Jetty::ts_definition(),
        defaulted::Jetty::zod_schema(),
        defaulted::Pontoon::ts_definition(),
        defaulted::Pontoon::zod_schema(),
        defaulted::Marina::ts_definition(),
        defaulted::Marina::zod_schema(),
        defaulted::Basin::<String>::ts_definition(),
        defaulted::Basin::<String>::zod_schema(),
        defaulted::Harbour::<String>::ts_definition(),
        defaulted::Harbour::<String>::zod_schema(),
        defaulted::Link::<String>::ts_definition(),
        defaulted::Link::<String>::zod_schema(),
        defaulted::Config::<String>::ts_definition(),
        defaulted::Config::<String>::zod_schema(),
        defaulted::Fleet::<String>::ts_definition(),
        defaulted::Fleet::<String>::zod_schema(),
        defaulted::Logbook::<String>::ts_definition(),
        defaulted::Logbook::<String>::zod_schema(),
        defaulted::Manifest::<String>::ts_definition(),
        defaulted::Manifest::<String>::zod_schema(),
        defaulted::Berthing::ts_definition(),
        defaulted::Berthing::zod_schema(),
        defaulted::ExternalBerthing::ts_definition(),
        defaulted::ExternalBerthing::zod_schema(),
        defaulted::AdjacentBerthing::ts_definition(),
        defaulted::AdjacentBerthing::zod_schema(),
        defaulted::UntaggedBerthing::ts_definition(),
        defaulted::UntaggedBerthing::zod_schema(),
        defaulted::Quay::ts_definition(),
        defaulted::Quay::zod_schema(),
        #[cfg(feature = "indexmap")]
        defaulted::IndexedFleet::<String>::ts_definition(),
        #[cfg(feature = "indexmap")]
        defaulted::IndexedFleet::<String>::zod_schema(),
        #[cfg(feature = "indexmap")]
        defaulted::IndexedLogbook::<String>::ts_definition(),
        #[cfg(feature = "indexmap")]
        defaulted::IndexedLogbook::<String>::zod_schema(),
    ];
    written.join("\n\n")
}

/// `value` written with the keys of each object in order, as the driver of the ranges writes it.
#[cfg(feature = "jsonschema")]
fn sorted_keys(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Object(object) => {
            let mut keys: Vec<&String> = object.keys().collect();
            keys.sort();
            let members: Vec<String> = keys
                .into_iter()
                .map(|key| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(key).unwrap(),
                        sorted_keys(&object[key])
                    )
                })
                .collect();
            format!("{{{}}}", members.join(","))
        }
        serde_json::Value::Array(items) => {
            let written: Vec<String> = items.iter().map(sorted_keys).collect();
            format!("[{}]", written.join(","))
        }
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_) => value.to_string(),
    }
}

/// The types the four std ranges are written on: a bundle of its own, compiled with `tsc --strict`
/// before it is loaded.
fn ranges() -> String {
    [
        PREAMBLE.to_owned(),
        ranged::Milestone::ts_definition(),
        ranged::Milestone::zod_schema(),
        ranged::Waymark::ts_definition(),
        ranged::Waymark::zod_schema(),
        ranged::Itinerary::<String>::ts_definition(),
        ranged::Itinerary::<String>::zod_schema(),
        ranged::Spanned::ts_definition(),
        ranged::Spanned::zod_schema(),
        ranged::Positioned::ts_definition(),
        ranged::Positioned::zod_schema(),
        ranged::Detour::ts_definition(),
        ranged::Detour::zod_schema(),
        ranged::Course::<u32>::ts_definition(),
        ranged::Course::<u32>::zod_schema(),
        ranged::Compass::ts_definition(),
        ranged::Compass::zod_schema(),
        ranged::Atlas::ts_definition(),
        ranged::Atlas::zod_schema(),
        ranged::Excursion::ts_definition(),
        ranged::Excursion::zod_schema(),
        ranged::Waypost::ts_definition(),
        ranged::Waypost::zod_schema(),
        ranged::Stride::<u32>::ts_definition(),
        ranged::Stride::<u32>::zod_schema(),
        ranged::Signposted::ts_definition(),
        ranged::Signposted::zod_schema(),
        ranged::Frontier::ts_definition(),
        ranged::Frontier::zod_schema(),
        #[cfg(feature = "chrono")]
        ranged::Rostered::ts_definition(),
        #[cfg(feature = "chrono")]
        ranged::Rostered::zod_schema(),
    ]
    .join("\n\n")
}

/// The models whose names hold a quote: a bundle of their own, compiled with `tsc --strict` before
/// it is loaded, each type after the ones it holds.
fn quoted_bundle() -> String {
    [
        PREAMBLE.to_owned(),
        quoted::Leveed::ts_definition(),
        quoted::Leveed::zod_schema(),
        quoted::Tolled::ts_definition(),
        quoted::Tolled::zod_schema(),
        quoted::Tariffed::ts_definition(),
        quoted::Tariffed::zod_schema(),
        quoted::Levied::ts_definition(),
        quoted::Levied::zod_schema(),
        quoted::Dutied::ts_definition(),
        quoted::Dutied::zod_schema(),
        quoted::Excised::ts_definition(),
        quoted::Excised::zod_schema(),
        quoted::Tithed::ts_definition(),
        quoted::Tithed::zod_schema(),
        quoted::Mulcted::ts_definition(),
        quoted::Mulcted::zod_schema(),
    ]
    .join("\n\n")
}

/// The models whose names and doc comments hold the end of a comment: a bundle of their own,
/// compiled with `tsc --strict` before it is loaded.
fn commented_bundle() -> String {
    [
        PREAMBLE.to_owned(),
        commented::Glossed::ts_definition(),
        commented::Glossed::zod_schema(),
        commented::Remarked::ts_definition(),
        commented::Remarked::zod_schema(),
        commented::Footnoted::ts_definition(),
        commented::Footnoted::zod_schema(),
        commented::Marginal::ts_definition(),
        commented::Marginal::zod_schema(),
        commented::Rubric::ts_definition(),
        commented::Rubric::zod_schema(),
        commented::Scholion::ts_definition(),
        commented::Scholion::zod_schema(),
        commented::marginalia_schema::Schema::ts_definition(),
        commented::marginalia_schema::Schema::zod_schema(),
    ]
    .join("\n\n")
}

/// The types a std `Result` is written on: a bundle of its own, compiled with `tsc --strict` before
/// it is loaded.
fn results() -> String {
    [
        PREAMBLE.to_owned(),
        resulted::Masthead::ts_definition(),
        resulted::Masthead::zod_schema(),
        resulted::Unparsed::ts_definition(),
        resulted::Unparsed::zod_schema(),
        resulted::Ballot::ts_definition(),
        resulted::Ballot::zod_schema(),
        resulted::Mailbag::<String>::ts_definition(),
        resulted::Mailbag::<String>::zod_schema(),
        resulted::Reply::ts_definition(),
        resulted::Reply::zod_schema(),
        resulted::Missive::ts_definition(),
        resulted::Missive::zod_schema(),
        resulted::Postbox::ts_definition(),
        resulted::Postbox::zod_schema(),
        resulted::Ruling::ts_definition(),
        resulted::Ruling::zod_schema(),
        resulted::settlement_schema::Schema::ts_definition(),
        resulted::settlement_schema::Schema::zod_schema(),
        resulted::Errand::ts_definition(),
        resulted::Errand::zod_schema(),
        resulted::Attempted::<u32>::ts_definition(),
        resulted::Attempted::<u32>::zod_schema(),
        resulted::Turn::<u32>::ts_definition(),
        resulted::Turn::<u32>::zod_schema(),
        resulted::Summary::ts_definition(),
        resulted::Summary::zod_schema(),
        resulted::Bureau::ts_definition(),
        resulted::Bureau::zod_schema(),
        resulted::Jacket::ts_definition(),
        resulted::Jacket::zod_schema(),
        resulted::Notebook::ts_definition(),
        resulted::Notebook::zod_schema(),
        resulted::Errata::ts_definition(),
        resulted::Errata::zod_schema(),
    ]
    .join("\n\n")
}

/// The types a set is written on: a bundle of its own, compiled with `tsc --strict` before it is
/// loaded.
fn sets() -> String {
    [
        PREAMBLE.to_owned(),
        sets::Point::ts_definition(),
        sets::Point::zod_schema(),
        sets::Ticket::ts_definition(),
        sets::Ticket::zod_schema(),
        sets::Folder::ts_definition(),
        sets::Folder::zod_schema(),
        sets::EveryList::ts_definition(),
        sets::EveryList::zod_schema(),
        sets::Nesting::ts_definition(),
        sets::Nesting::zod_schema(),
        sets::Lanyard::ts_definition(),
        sets::Lanyard::zod_schema(),
        sets::Pouch::ts_definition(),
        sets::Pouch::zod_schema(),
        sets::Shelving::ts_definition(),
        sets::Shelving::zod_schema(),
        sets::Sorting::ts_definition(),
        sets::Sorting::zod_schema(),
        sets::Coin::<String>::ts_definition(),
        sets::Coin::<String>::zod_schema(),
        sets::Wallet::<String>::ts_definition(),
        sets::Wallet::<String>::zod_schema(),
    ]
    .join("\n\n")
}

/// The types a map, or an optional struct, is flattened into: a bundle of its own, compiled with
/// `tsc --strict` before it is loaded.
fn flattened_maps() -> String {
    [
        PREAMBLE.to_owned(),
        flattened_maps::Thwart::ts_definition(),
        flattened_maps::Thwart::zod_schema(),
        flattened_maps::Gunwale::ts_definition(),
        flattened_maps::Gunwale::zod_schema(),
        flattened_maps::StrakeCode::ts_definition(),
        flattened_maps::StrakeCode::zod_schema(),
        flattened_maps::Strake::ts_definition(),
        flattened_maps::Strake::zod_schema(),
        flattened_maps::Coaming::ts_definition(),
        flattened_maps::Coaming::zod_schema(),
        flattened_maps::Transom::ts_definition(),
        flattened_maps::Transom::zod_schema(),
        flattened_maps::Bilge::ts_definition(),
        flattened_maps::Bilge::zod_schema(),
        flattened_maps::Locker::ts_definition(),
        flattened_maps::Locker::zod_schema(),
        flattened_maps::Scupper::ts_definition(),
        flattened_maps::Scupper::zod_schema(),
        flattened_maps::Hawse::ts_definition(),
        flattened_maps::Hawse::zod_schema(),
        flattened_maps::Windlass::ts_definition(),
        flattened_maps::Windlass::zod_schema(),
        flattened_maps::Ratlines::ts_definition(),
        flattened_maps::Ratlines::zod_schema(),
        flattened_maps::Deadeye::ts_definition(),
        flattened_maps::Deadeye::zod_schema(),
        flattened_maps::sheave_tallies_schema::Schema::ts_definition(),
        flattened_maps::sheave_tallies_schema::Schema::zod_schema(),
        flattened_maps::Sheave::ts_definition(),
        flattened_maps::Sheave::zod_schema(),
        flattened_maps::spare_tallies_schema::Schema::ts_definition(),
        flattened_maps::spare_tallies_schema::Schema::zod_schema(),
        flattened_maps::Lazarette::ts_definition(),
        flattened_maps::Lazarette::zod_schema(),
        flattened_maps::PintleCounts::ts_definition(),
        flattened_maps::PintleCounts::zod_schema(),
        flattened_maps::Pintle::ts_definition(),
        flattened_maps::Pintle::zod_schema(),
        flattened_maps::Oarlock::<String>::ts_definition(),
        flattened_maps::Oarlock::<String>::zod_schema(),
        flattened_maps::Thole::ts_definition(),
        flattened_maps::Thole::zod_schema(),
        flattened_maps::Kedge::ts_definition(),
        flattened_maps::Kedge::zod_schema(),
        flattened_maps::Fender::ts_definition(),
        flattened_maps::Fender::zod_schema(),
        flattened_maps::Stanchion::ts_definition(),
        flattened_maps::Stanchion::zod_schema(),
        flattened_maps::Fairlead::ts_definition(),
        flattened_maps::Fairlead::zod_schema(),
        flattened_maps::Davit::ts_definition(),
        flattened_maps::Davit::zod_schema(),
        flattened_maps::Capstan::ts_definition(),
        flattened_maps::Capstan::zod_schema(),
        flattened_maps::Burgee::ts_definition(),
        flattened_maps::Burgee::zod_schema(),
        flattened_maps::Hawser::ts_definition(),
        flattened_maps::Hawser::zod_schema(),
    ]
    .join("\n\n")
}

/// The models of maps keyed by a value serde writes as text of its own, each declared ahead of the
/// one that names it: a bundle of their own, compiled with `tsc --strict` before it is loaded.
#[cfg(feature = "chrono")]
fn keyed_maps() -> String {
    [
        PREAMBLE.to_owned(),
        keyed_maps::Baggywrinkle::ts_definition(),
        keyed_maps::Baggywrinkle::zod_schema(),
        keyed_maps::Futtock::ts_definition(),
        keyed_maps::Futtock::zod_schema(),
        keyed_maps::Gudgeon::ts_definition(),
        keyed_maps::Gudgeon::zod_schema(),
        keyed_maps::Marlinspike::ts_definition(),
        keyed_maps::Marlinspike::zod_schema(),
        keyed_maps::Scuttle::ts_definition(),
        keyed_maps::Scuttle::zod_schema(),
        keyed_maps::Trunnel::ts_definition(),
        keyed_maps::Trunnel::zod_schema(),
    ]
    .join("\n\n")
}

/// The holders of an optional flattened member whose type the Kotlin emitter recorded no keys for:
/// a bundle of their own, compiled with `tsc --strict` before it is loaded.
fn unrecorded_options() -> String {
    [
        PREAMBLE.to_owned(),
        unrecorded::Bight::ts_definition(),
        unrecorded::Bight::zod_schema(),
        unrecorded::Fiddling::ts_definition(),
        unrecorded::Fiddling::zod_schema(),
        unrecorded::Crowned::ts_definition(),
        unrecorded::Crowned::zod_schema(),
        unrecorded::Hitched::ts_definition(),
        unrecorded::Hitched::zod_schema(),
        unrecorded::Snagging::ts_definition(),
        unrecorded::Snagging::zod_schema(),
        unrecorded::Bridled::ts_definition(),
        unrecorded::Bridled::zod_schema(),
        unrecorded::Bridling::ts_definition(),
        unrecorded::Bridling::zod_schema(),
        unrecorded::Slipped::ts_definition(),
        unrecorded::Slipped::zod_schema(),
        unrecorded::Slipping::ts_definition(),
        unrecorded::Slipping::zod_schema(),
    ]
    .join("\n\n")
}

/// The holders of an optional flattened member that names a function: a bundle of their own,
/// compiled with `tsc --strict` before it is loaded.
fn corrected_options_bundle() -> String {
    [
        PREAMBLE.to_owned(),
        AS_GIVEN_OR_NULL.to_owned(),
        corrected_options::Tholepin::ts_definition(),
        corrected_options::Tholepin::zod_schema(),
        corrected_options::Reeved::ts_definition(),
        corrected_options::Reeved::zod_schema(),
        corrected_options::Wormed::ts_definition(),
        corrected_options::Wormed::zod_schema(),
        corrected_options::Frapped::ts_definition(),
        corrected_options::Frapped::zod_schema(),
        corrected_options::Moused::ts_definition(),
        corrected_options::Moused::zod_schema(),
    ]
    .join("\n\n")
}

/// The holders of a flattened struct that reads a key taken around it: a bundle of their own,
/// compiled with `tsc --strict` before it is loaded.
fn shared_keys_bundle() -> String {
    [
        PREAMBLE.to_owned(),
        shared_keys::Earring::ts_definition(),
        shared_keys::Earring::zod_schema(),
        shared_keys::Spritsail::ts_definition(),
        shared_keys::Spritsail::zod_schema(),
        shared_keys::Bowline::ts_definition(),
        shared_keys::Bowline::zod_schema(),
        shared_keys::Gantline::ts_definition(),
        shared_keys::Gantline::zod_schema(),
        shared_keys::Clinching::ts_definition(),
        shared_keys::Clinching::zod_schema(),
        shared_keys::Garnet::ts_definition(),
        shared_keys::Garnet::zod_schema(),
        shared_keys::Parrel::ts_definition(),
        shared_keys::Parrel::zod_schema(),
        shared_keys::Gudgeon::ts_definition(),
        shared_keys::Gudgeon::zod_schema(),
        shared_keys::Rubrail::ts_definition(),
        shared_keys::Rubrail::zod_schema(),
        shared_keys::Toggled::ts_definition(),
        shared_keys::Toggled::zod_schema(),
        shared_keys::Halliard::ts_definition(),
        shared_keys::Halliard::zod_schema(),
        shared_keys::Snotter::ts_definition(),
        shared_keys::Snotter::zod_schema(),
        shared_keys::Bitts::ts_definition(),
        shared_keys::Bitts::zod_schema(),
    ]
    .join("\n\n")
}

/// The enums with a variant serde neither writes nor reads, or drops in one direction alone: a
/// bundle of their own, compiled with `tsc --strict` before it is loaded.
fn skipped_variants_bundle() -> String {
    [
        PREAMBLE.to_owned(),
        skipped_variants::Berthage::ts_definition(),
        skipped_variants::Berthage::zod_schema(),
        skipped_variants::Wharfage::ts_definition(),
        skipped_variants::Wharfage::zod_schema(),
        skipped_variants::Pilotage::ts_definition(),
        skipped_variants::Pilotage::zod_schema(),
        skipped_variants::Lighterage::ts_definition(),
        skipped_variants::Lighterage::zod_schema(),
        skipped_variants::Keelage::ts_definition(),
        skipped_variants::Keelage::zod_schema(),
        skipped_variants::Cartage::ts_definition(),
        skipped_variants::Cartage::zod_schema(),
        skipped_variants::Tonnage::ts_definition(),
        skipped_variants::Tonnage::zod_schema(),
        skipped_variants::Dockage::ts_definition(),
        skipped_variants::Dockage::zod_schema(),
    ]
    .join("\n\n")
}

/// The holders of a text serde reads in one form alone: a bundle of their own, compiled with
/// `tsc --strict` before it is loaded.
#[cfg(feature = "chrono")]
fn text_forms_bundle() -> String {
    [
        PREAMBLE.to_owned(),
        text_forms::Chronometer::ts_definition(),
        text_forms::Chronometer::zod_schema(),
        text_forms::LetterLog::ts_definition(),
        text_forms::LetterLog::zod_schema(),
        text_forms::LetterLines::ts_definition(),
        text_forms::LetterLines::zod_schema(),
        text_forms::DayLog::ts_definition(),
        text_forms::DayLog::zod_schema(),
        text_forms::DayLines::ts_definition(),
        text_forms::DayLines::zod_schema(),
        text_forms::TimeLog::ts_definition(),
        text_forms::TimeLog::zod_schema(),
        text_forms::TimeLines::ts_definition(),
        text_forms::TimeLines::zod_schema(),
        text_forms::StampLog::ts_definition(),
        text_forms::StampLog::zod_schema(),
        text_forms::StampLines::ts_definition(),
        text_forms::StampLines::zod_schema(),
        text_forms::InstantLog::ts_definition(),
        text_forms::InstantLog::zod_schema(),
        text_forms::InstantLines::ts_definition(),
        text_forms::InstantLines::zod_schema(),
    ]
    .join("\n\n")
}

/// The enums of [`untagged_millis::MILLIS_ROWS`]: a bundle of their own, compiled with `tsc
/// --strict` before it is loaded.
#[cfg(feature = "chrono")]
fn untagged_millis_bundle() -> String {
    [
        PREAMBLE.to_owned(),
        untagged_millis::MillisOrFlag::ts_definition(),
        untagged_millis::MillisOrFlag::zod_schema(),
        untagged_millis::MillisNamed::ts_definition(),
        untagged_millis::MillisNamed::zod_schema(),
        untagged_millis::MillisMaybe::ts_definition(),
        untagged_millis::MillisMaybe::zod_schema(),
        untagged_millis::MillisLater::ts_definition(),
        untagged_millis::MillisLater::zod_schema(),
    ]
    .join("\n\n")
}

/// The models of [`wire_forms::WIRE_FORMS`]: a bundle of their own, compiled with `tsc --strict`
/// before it is loaded, beside the consumer's own `ObjectId`.
#[cfg(feature = "chrono")]
fn wire_forms_bundle() -> String {
    let mut parts = vec![
        PREAMBLE.to_owned(),
        wire_forms::ConveyedFlag::ts_definition(),
        wire_forms::ConveyedFlag::zod_schema(),
        wire_forms::ConveyedLetter::ts_definition(),
        wire_forms::ConveyedLetter::zod_schema(),
        wire_forms::ConveyedText::ts_definition(),
        wire_forms::ConveyedText::zod_schema(),
        wire_forms::ConveyedU8::ts_definition(),
        wire_forms::ConveyedU8::zod_schema(),
        wire_forms::ConveyedU16::ts_definition(),
        wire_forms::ConveyedU16::zod_schema(),
        wire_forms::ConveyedU32::ts_definition(),
        wire_forms::ConveyedU32::zod_schema(),
        wire_forms::ConveyedI8::ts_definition(),
        wire_forms::ConveyedI8::zod_schema(),
        wire_forms::ConveyedI16::ts_definition(),
        wire_forms::ConveyedI16::zod_schema(),
        wire_forms::ConveyedI32::ts_definition(),
        wire_forms::ConveyedI32::zod_schema(),
        wire_forms::ConveyedI64::ts_definition(),
        wire_forms::ConveyedI64::zod_schema(),
        wire_forms::ConveyedIsize::ts_definition(),
        wire_forms::ConveyedIsize::zod_schema(),
        wire_forms::ConveyedU64::<u64>::ts_definition(),
        wire_forms::ConveyedU64::<u64>::zod_schema(),
        wire_forms::ConveyedUsize::<usize>::ts_definition(),
        wire_forms::ConveyedUsize::<usize>::zod_schema(),
        wire_forms::ConveyedF32::ts_definition(),
        wire_forms::ConveyedF32::zod_schema(),
        wire_forms::ConveyedF64::ts_definition(),
        wire_forms::ConveyedF64::zod_schema(),
        wire_forms::ConveyedLiteralCount::ts_definition(),
        wire_forms::ConveyedLiteralCount::zod_schema(),
        wire_forms::ConveyedLiteralFlag::ts_definition(),
        wire_forms::ConveyedLiteralFlag::zod_schema(),
        wire_forms::ConveyedLiteralText::ts_definition(),
        wire_forms::ConveyedLiteralText::zod_schema(),
        wire_forms::ConveyedDay::ts_definition(),
        wire_forms::ConveyedDay::zod_schema(),
        wire_forms::ConveyedHour::ts_definition(),
        wire_forms::ConveyedHour::zod_schema(),
        wire_forms::ConveyedStamp::ts_definition(),
        wire_forms::ConveyedStamp::zod_schema(),
        wire_forms::ConveyedInstant::ts_definition(),
        wire_forms::ConveyedInstant::zod_schema(),
        wire_forms::ConveyedMillis::ts_definition(),
        wire_forms::ConveyedMillis::zod_schema(),
        wire_forms::ConveyedSlot::ts_definition(),
        wire_forms::ConveyedSlot::zod_schema(),
        wire_forms::ConveyedSlotted::ts_definition(),
        wire_forms::ConveyedSlotted::zod_schema(),
    ];
    #[cfg(feature = "mongodb")]
    parts.extend([
        "type ObjectId = { $oid: string };".to_owned(),
        wire_forms::ConveyedObjectId::ts_definition(),
        wire_forms::ConveyedObjectId::zod_schema(),
    ]);
    parts.join("\n\n")
}

/// The models of [`digit_strings`]: a bundle of their own, compiled with `tsc --strict` and loaded
/// under `node`.
fn digit_strings_bundle() -> String {
    #[cfg(not(any(feature = "swift", feature = "kotlin")))]
    let wide = [
        digit_strings::DigitWide::ts_definition(),
        digit_strings::DigitWide::zod_schema(),
    ];
    #[cfg(any(feature = "swift", feature = "kotlin"))]
    let wide: [String; 0] = [];
    [
        PREAMBLE.to_owned(),
        digit_strings::DigitTally::ts_definition(),
        digit_strings::DigitTally::zod_schema(),
        digit_strings::DigitId::ts_definition(),
        digit_strings::DigitId::zod_schema(),
        digit_strings::DigitEvent::ts_definition(),
        digit_strings::DigitEvent::zod_schema(),
        digit_strings::DigitBounded::ts_definition(),
        digit_strings::DigitBounded::zod_schema(),
        digit_strings::DigitHolder::<i64>::ts_definition(),
        digit_strings::DigitHolder::<i64>::zod_schema(),
        digit_strings::DigitKept::ts_definition(),
        digit_strings::DigitKept::zod_schema(),
        digit_strings::DigitPair::ts_definition(),
        digit_strings::DigitPair::zod_schema(),
        digit_strings::DigitTagged::ts_definition(),
        digit_strings::DigitTagged::zod_schema(),
        digit_strings::DigitAdjacent::ts_definition(),
        digit_strings::DigitAdjacent::zod_schema(),
        digit_strings::DigitUntagged::ts_definition(),
        digit_strings::DigitUntagged::zod_schema(),
    ]
    .into_iter()
    .chain(wide)
    .collect::<Vec<_>>()
    .join("\n\n")
}

/// The JavaScript value a row of [`wire_forms::WIRE_FORMS`] hands the Zod schema of its model:
/// a `Date` for a `DateTime<Tz>`, read from the text handed, a `bigint` for an `i64`, and the
/// JSON handed for any other.
#[cfg(feature = "chrono")]
fn wire_form_handed(row: &wire_forms::WireFormRow) -> String {
    let value = if row.held_by == "ConveyedInstant" {
        format!("new Date({})", row.handed)
    } else if row.held_by == "ConveyedI64" {
        format!("{}n", row.handed)
    } else {
        row.handed.to_owned()
    };
    let key = if row.handed.starts_with('"') {
        row.handed.to_owned()
    } else {
        format!("\"{}\"", row.handed)
    };
    match wire_forms::places(row.held_by) {
        wire_forms::Places::Keyed => {
            format!("{{ keyed: {{ [{key}]: true }}, value: {value}, values: [{value}] }}")
        }
        wire_forms::Places::Listed => format!("{{ value: {value}, values: [{value}] }}"),
        wire_forms::Places::Lone => format!("{{ value: {value} }}"),
    }
}

/// The schema the model `held_by` names publishes: its `$SchemaDefault` where it declares a
/// parameter.
#[cfg(feature = "chrono")]
fn wire_form_schema(held_by: &str) -> String {
    if matches!(held_by, "ConveyedU64" | "ConveyedUsize") {
        format!("{held_by}$SchemaDefault")
    } else {
        format!("{held_by}$Schema")
    }
}

/// What serde's read of `json` as a `T` prints, as the driver of the `Result`s prints the same
/// payload: the value, with the keys of each object in order, or the refusal, with each message
/// of `validate()` where the type judges a model it holds.
#[cfg(feature = "jsonschema")]
fn result_read<T>(name: &str, json: &str, judged: bool) -> (bool, String)
where
    T: DeserializeOwned,
{
    match serde_json::from_str::<T>(json) {
        Ok(_) => {
            let value: serde_json::Value = serde_json::from_str(json).unwrap();
            (true, format!("{name}: read {}", sorted_keys(&value)))
        }
        Err(refused) => {
            let mut said = vec![format!("{name}: refused")];
            if judged {
                let mut messages: Vec<String> = refused
                    .to_string()
                    .split("; ")
                    .map(|message| format!("    {message}"))
                    .collect();
                messages.sort();
                said.extend(messages);
            }
            (false, said.join("\n"))
        }
    }
}

/// Whether serde reads `json` as a `T`.
#[cfg(feature = "jsonschema")]
fn serde_reads<T>(json: &str) -> bool
where
    T: DeserializeOwned,
{
    serde_json::from_str::<T>(json).is_ok()
}

/// The models the payloads of the ranges are read as.
#[cfg(feature = "jsonschema")]
fn ranged_models() -> Vec<RangedModel> {
    vec![
        (
            "Compass",
            serde_reads::<ranged::Compass>,
            ranged::Compass::json_schema,
        ),
        (
            "Course",
            serde_reads::<ranged::Course<u32>>,
            ranged::Course::<u32>::json_schema,
        ),
        (
            "Detour",
            serde_reads::<ranged::Detour>,
            ranged::Detour::json_schema,
        ),
        (
            "Excursion",
            serde_reads::<ranged::Excursion>,
            ranged::Excursion::json_schema,
        ),
        (
            "Waypost",
            serde_reads::<ranged::Waypost>,
            ranged::Waypost::json_schema,
        ),
        (
            "Frontier",
            serde_reads::<ranged::Frontier>,
            ranged::Frontier::json_schema,
        ),
        (
            "Positioned",
            serde_reads::<ranged::Positioned>,
            ranged::Positioned::json_schema,
        ),
        #[cfg(feature = "chrono")]
        (
            "Rostered",
            serde_reads::<ranged::Rostered>,
            ranged::Rostered::json_schema,
        ),
        (
            "Signposted",
            serde_reads::<ranged::Signposted>,
            ranged::Signposted::json_schema,
        ),
        (
            "Spanned",
            serde_reads::<ranged::Spanned>,
            ranged::Spanned::json_schema,
        ),
        (
            "Stride",
            serde_reads::<ranged::Stride<u32>>,
            ranged::Stride::<u32>::json_schema,
        ),
    ]
}

/// The models the payloads of a `Result` are read as.
#[cfg(feature = "jsonschema")]
fn resulted_models() -> Vec<ResultedModel> {
    vec![
        (
            "Attempted",
            false,
            result_read::<resulted::Attempted<u32>>,
            resulted::Attempted::<u32>::json_schema,
        ),
        (
            "Bureau",
            false,
            result_read::<resulted::Bureau>,
            resulted::Bureau::json_schema,
        ),
        (
            "Errand",
            false,
            result_read::<resulted::Errand>,
            resulted::Errand::json_schema,
        ),
        (
            "Errata",
            true,
            result_read::<resulted::Errata>,
            resulted::Errata::json_schema,
        ),
        (
            "Missive",
            true,
            result_read::<resulted::Missive>,
            resulted::Missive::json_schema,
        ),
        (
            "Notebook",
            true,
            result_read::<resulted::Notebook>,
            resulted::Notebook::json_schema,
        ),
        (
            "Postbox",
            false,
            result_read::<resulted::Postbox>,
            resulted::Postbox::json_schema,
        ),
        (
            "Reply",
            false,
            result_read::<resulted::Reply>,
            resulted::Reply::json_schema,
        ),
        (
            "Summary",
            false,
            result_read::<resulted::Summary>,
            resulted::Summary::json_schema,
        ),
        (
            "Turn",
            false,
            result_read::<resulted::Turn<u32>>,
            resulted::Turn::<u32>::json_schema,
        ),
    ]
}

/// The types that hold a date, which the bundle declares where `chrono` is on.
#[cfg(feature = "chrono")]
fn dated() -> Vec<String> {
    vec![
        DeclaredMark::ts_definition(),
        DeclaredMark::zod_schema(),
        DeclaredStamped::ts_definition(),
        DeclaredStamped::zod_schema(),
    ]
}

#[cfg(not(feature = "chrono"))]
const fn dated() -> Vec<String> {
    Vec::new()
}

/// Runs the compiler over the project file `project` names in `at`, and answers whether it
/// accepted it and everything it reported, or `None` where no compiler could be started.
fn compiled(at: &Path, project: &str) -> Option<(bool, String)> {
    let named_compiler = env::var(COMPILER_VAR).ok();
    let compiler = named_compiler.clone().unwrap_or_else(|| "tsc".to_owned());
    let run = Command::new(&compiler)
        .args(["--pretty", "false", "-p", project])
        .current_dir(at)
        .output();
    let Ok(reported) = run else {
        assert!(
            named_compiler.is_none(),
            "{COMPILER_VAR} names `{compiler}`, and no compiler could be started there: {}",
            run.unwrap_err()
        );
        return None;
    };
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&reported.stdout),
        String::from_utf8_lossy(&reported.stderr)
    );
    Some((reported.status.success(), said))
}

/// The `node_modules` that holds `zod`, or `None` where the variable names none. One it names
/// without `zod` in it is a failure: somebody said where it was.
fn node_modules() -> Option<PathBuf> {
    let at = PathBuf::from(env::var(MODULES_VAR).ok()?).join("node_modules");
    assert!(
        at.join("zod").is_dir(),
        "{MODULES_VAR} names a directory whose `node_modules` holds no `zod`: {}",
        at.display()
    );
    Some(at)
}

/// Said on the process's own stderr, which `cargo test` does not capture: a stand-down is a pass
/// that proved nothing, so it has to show on a run where everything passed.
fn stand_down() {
    STOOD_DOWN.call_once(|| {
        let notice = format!(
            "\ntixschema: no TypeScript compiler, no `zod` or no `node` is reachable, so the \
             bundle of types that reach themselves was NOT compiled or loaded.\n  Put `tsc` and \
             `node` on PATH or name them in {COMPILER_VAR} and {NODE_VAR}, name in {MODULES_VAR} \
             a directory whose `node_modules` holds `zod`, and run `just typecheck-ts`, which \
             refuses to stand down.\n\n"
        );
        drop(stderr().write_all(notice.as_bytes()));
    });
}

/// The first package in a directory of its own for the check `named`, compiled with its
/// declarations, or `None` where the compiler or `zod` is not reachable.
fn built(named: &str) -> Option<PathBuf> {
    built_from(named, &bundle())
}

/// The package `built` answers, over the generated text `source`.
fn built_from(named: &str, source: &str) -> Option<PathBuf> {
    let Some(modules) = node_modules() else {
        stand_down();
        return None;
    };
    let at = temp_dir().join(format!("tixschema-declarations-{named}-{run}", run = id()));
    if at.exists() {
        fs::remove_dir_all(&at).unwrap();
    }
    fs::create_dir_all(at.join("src")).unwrap();
    symlink(&modules, at.join("node_modules")).unwrap();
    fs::write(at.join("package.json"), PACKAGE).unwrap();
    fs::write(at.join("tsconfig.json"), PROJECT).unwrap();
    fs::write(at.join("src/index.ts"), source).unwrap();

    let Some((emitted, said)) = compiled(&at, "tsconfig.json") else {
        fs::remove_dir_all(&at).unwrap();
        stand_down();
        return None;
    };
    assert!(emitted, "the bundle does not compile:\n{said}");
    Some(at)
}

#[test]
fn type_check_a_recursive_generic_keeps_its_type_in_another_package() {
    let Some(at) = built("consumer") else {
        return;
    };
    fs::write(at.join("tsconfig.consumer.json"), CONSUMER_PROJECT).unwrap();
    fs::write(at.join("consumer.ts"), CONSUMER).unwrap();
    let declarations = fs::read_to_string(at.join("lib/index.d.ts")).unwrap();
    let (accepted, refused) = compiled(&at, "tsconfig.consumer.json").unwrap();
    fs::remove_dir_all(&at).unwrap();
    assert!(
        accepted,
        "a package reading the declarations loses a type:\n{refused}\n\nThe declarations:\n\
         {declarations}"
    );
}

/// What `driver` prints once the bundle built for the check `named` is loaded under `node`, with
/// what it wrote to its stderr, or `None` where the bundle or the runtime is not reachable.
fn loaded(named: &str, driver: &str) -> Option<(String, String)> {
    loaded_from(named, &bundle(), driver)
}

/// What `loaded` answers, over the generated text `source`.
fn loaded_from(named: &str, source: &str, driver: &str) -> Option<(String, String)> {
    let at = built_from(named, source)?;
    fs::write(at.join("run.mjs"), driver).unwrap();
    let named_node = env::var(NODE_VAR).ok();
    let node = named_node.clone().unwrap_or_else(|| "node".to_owned());
    let run = Command::new(&node).arg("run.mjs").current_dir(&at).output();
    fs::remove_dir_all(&at).unwrap();
    let Ok(ran) = run else {
        assert!(
            named_node.is_none(),
            "{NODE_VAR} names `{node}`, and no runtime could be started there: {}",
            run.unwrap_err()
        );
        stand_down();
        return None;
    };
    Some((
        String::from_utf8_lossy(&ran.stdout).trim().to_owned(),
        String::from_utf8_lossy(&ran.stderr).into_owned(),
    ))
}

#[test]
fn type_check_a_generic_that_holds_an_optional_one_of_itself_loads_and_parses() {
    let Some((printed, failed)) = loaded("optional", OPTIONAL_DRIVER) else {
        return;
    };
    assert_eq!(
        printed, OPTIONAL_LOADED,
        "the bundle does not load and parse under node:\n{failed}"
    );
}

#[test]
fn type_check_a_tuple_struct_that_reaches_itself_loads_and_parses() {
    let Some((printed, failed)) = loaded("loaded", DRIVER) else {
        return;
    };
    assert_eq!(
        printed, LOADED,
        "the bundle does not load and parse under node:\n{failed}"
    );
}

#[test]
fn type_check_a_bound_on_a_field_typed_with_a_brand_loads_and_parses() {
    let Some((printed, failed)) = loaded("bounded", BOUNDED_DRIVER) else {
        return;
    };
    assert_eq!(
        printed, BOUNDED,
        "the bundle does not load and parse under node:\n{failed}"
    );
}

#[test]
fn type_check_every_schema_holds_its_examples_once_loaded() {
    let aliased: DeclaredLabelRef = DeclaredLabel("a".to_owned());
    assert_eq!(aliased.0, "a");

    let Some((printed, failed)) = loaded("examples", EXAMPLES_DRIVER) else {
        return;
    };
    assert_eq!(
        printed, EXAMPLES,
        "the bundle does not load and hold its examples under node:\n{failed}"
    );
}

/// `z.coerce.date()` alone reads `null` and `false` as the first instant of 1970, ahead of the
/// `null` and the boolean a union lists after it.
#[cfg(feature = "chrono")]
#[test]
fn type_check_a_date_refuses_what_is_no_date_in_every_position_that_holds_one() {
    let Some((printed, failed)) = loaded("dated", DATED_DRIVER) else {
        return;
    };
    assert_eq!(
        printed, DATED,
        "the bundle does not load and parse under node:\n{failed}"
    );
}

/// Each case of the shared table, of the shapes and of the places serde writes a member at is
/// parsed by the schema its type publishes, and the two sides are compared as one text.
#[test]
fn type_check_a_check_answers_under_zod_what_validate_answers() {
    let mut table = checked::parcel_cases();
    table.push(checked::manifest_case());
    let mut shapes = checked::shape_cases();
    shapes.extend(checked::places::place_cases());
    let answered = format!("{}{}", checked::report(&table), checked::report(&shapes));
    assert_eq!(
        answered,
        format!(
            "{}{}{}",
            checked::TABLE,
            checked::SHAPES,
            checked::places::PLACES
        )
    );

    let of_the_table = table.iter().map(|case| {
        let schema = if case.name == "case 6" {
            "Manifest"
        } else {
            "Parcel"
        };
        (case.name, schema, case.payload.as_str())
    });
    let of_the_shapes = shapes.iter().map(|case| {
        let schema = case.name.split(',').next().unwrap_or(case.name);
        (case.name, schema, case.payload.as_str())
    });
    let cases: Vec<(&str, &str, &str)> = of_the_table.chain(of_the_shapes).collect();
    let driver = CHECKS_DRIVER.replace("CASES", &serde_json::to_string(&cases).unwrap());
    let Some((printed, failed)) = loaded("checks", &driver) else {
        return;
    };
    assert_eq!(
        printed,
        answered.trim_end(),
        "the schemas do not answer what `validate()` answers:\n{failed}"
    );
}

/// Each row of the sample, each position another type reads a preprocessed type in and each
/// shape the key is written on is parsed by the schema its type publishes, and the two sides are
/// compared as one text.
#[test]
fn type_check_a_preprocess_answers_under_zod_what_a_rust_read_answers() {
    let sample = corrected::sample_report();
    assert_eq!(sample, corrected::SAMPLE);
    let positions = corrected::positions();
    let read: String = positions
        .iter()
        .map(|(_name, _schema, _payload, printed)| printed.as_str())
        .collect();
    assert_eq!(read, corrected::POSITIONS);

    let of_the_sample = corrected::SAMPLE_ROWS.iter().map(|&(row, payload)| {
        let schema = if row == "row 5" { "Manifest" } else { "Parcel" };
        (row, schema, payload)
    });
    let of_the_positions = positions
        .iter()
        .map(|&(name, schema, payload, _)| (name, schema, payload));
    let cases: Vec<(&str, &str, &str)> = of_the_sample.chain(of_the_positions).collect();
    let driver = CORRECTIONS_DRIVER.replace("CASES", &serde_json::to_string(&cases).unwrap());
    let Some((printed, failed)) = loaded_from("corrections", &corrections(), &driver) else {
        return;
    };
    assert_eq!(
        printed,
        format!("{sample}{read}").trim_end(),
        "the schemas do not read what a Rust read reads:\n{failed}"
    );
}

/// A key under `#[serde(default)]` the payload leaves out is parsed as the default serde reads,
/// held to the member's checks as a payload is, and a key the payload holds is read as before.
#[test]
fn type_check_a_default_answers_under_zod_what_a_serde_read_answers() {
    assert_eq!(defaulted::report(), defaulted::REPORT);
    let (every_default, both_held) = defaulted::read_values();
    for (name, read) in [
        (
            "every default at once",
            serde_json::to_string(&every_default),
        ),
        ("row 5: the key held", serde_json::to_string(&both_held)),
    ] {
        let line = format!("{name}: read {}\n", read.unwrap());
        assert!(defaulted::REPORT.contains(&line), "{line}");
    }
    let (marina, _) = defaulted::unwritten_defaults();
    assert_eq!(
        serde_json::from_str::<defaulted::Marina>(r#"{"label":"m"}"#).unwrap(),
        marina
    );
    let driver =
        DEFAULTS_DRIVER.replace("CASES", &serde_json::to_string(&defaulted::CASES).unwrap());
    let Some((printed, failed)) = loaded_from("defaults", &defaults(), &driver) else {
        return;
    };
    assert_eq!(
        printed,
        defaulted::REPORT.trim_end(),
        "the schemas do not read what a serde read reads:\n{failed}"
    );
}

/// A member of a variant the payload leaves out is parsed under each tagging as the default serde
/// reads, held to the member's checks as a payload is, and a key the payload holds is read as
/// before. A struct that holds the untagged enum reads its member's default too.
#[test]
fn type_check_a_default_of_a_variant_answers_under_zod_what_a_serde_read_answers() {
    assert_eq!(defaulted::variant_report(), defaulted::VARIANT_REPORT);
    let driver = VARIANTS_DRIVER.replace(
        "CASES",
        &serde_json::to_string(&defaulted::VARIANT_CASES).unwrap(),
    );
    let Some((printed, failed)) = loaded_from("variants", &defaults(), &driver) else {
        return;
    };
    assert_eq!(
        printed,
        defaulted::VARIANT_REPORT.trim_end(),
        "the schemas do not read what a serde read reads:\n{failed}"
    );
}

/// A list, a set or a map of a struct that declares a parameter, left out of the payload, is
/// parsed at the declared default as the empty collection serde reads, and a key held is read as
/// before. A key whose default has no JSON stays required.
#[test]
fn type_check_an_empty_collection_answers_under_zod_what_a_serde_read_answers() {
    assert_eq!(defaulted::filled_report(), defaulted::filled_expected());
    let (_, harbours) = defaulted::unwritten_defaults();
    for ((name, payload), harbour) in defaulted::REQUIRED.iter().zip(harbours) {
        assert_eq!(
            serde_json::from_str::<defaulted::Harbour<String>>(payload).unwrap(),
            harbour,
            "{name}"
        );
    }
    let driver = FILLED_DRIVER
        .replace(
            "FILLED",
            &serde_json::to_string(&defaulted::filled_cases()).unwrap(),
        )
        .replace(
            "REQUIRED",
            &serde_json::to_string(&defaulted::REQUIRED).unwrap(),
        );
    let Some((printed, failed)) = loaded_from("filled", &defaults(), &driver) else {
        return;
    };
    assert_eq!(
        format!("{printed}\n"),
        format!(
            "{}{}",
            defaulted::filled_expected(),
            defaulted::REQUIRED_REPORT
        ),
        "the schemas do not read what a serde read reads:\n{failed}"
    );
}

/// Zod hands the function of a member the default of its key, where a serde read calls no
/// function for a key the payload leaves out: the two answer `"spare"` and `"  spare  "`.
#[test]
fn type_check_a_default_is_handed_to_the_function_of_its_member_under_zod() {
    for ((_, payload), read) in defaulted::PREPROCESSED
        .iter()
        .zip(defaulted::preprocessed_values())
    {
        assert_eq!(
            serde_json::from_str::<defaulted::Bollard>(payload).unwrap(),
            read
        );
    }
    let source = [
        PREAMBLE.to_owned(),
        DOCKED.to_owned(),
        defaulted::Bollard::ts_definition(),
        defaulted::Bollard::zod_schema(),
    ]
    .join("\n\n");
    let driver = DOCKED_DRIVER.replace(
        "CASES",
        &serde_json::to_string(&defaulted::PREPROCESSED).unwrap(),
    );
    let Some((printed, failed)) = loaded_from("docked", &source, &driver) else {
        return;
    };
    assert_eq!(
        format!("{printed}\n"),
        defaulted::PREPROCESSED_REPORT.replace("\"  spare  \"", "\"spare\""),
        "the bundle does not load and parse under node:\n{failed}"
    );
}

/// The two places a function is handed something else under zod than in a Rust read. A serde
/// read answers a key the payload does not hold itself, and calls no function for it; and it
/// hands the function of a flattened type the keys its holder does not take.
#[test]
fn type_check_a_function_is_handed_under_zod_what_a_rust_read_does_not_hand_it() {
    assert_eq!(
        serde_json::from_str::<corrected::Unnamed>("{}")
            .unwrap_err()
            .to_string(),
        "missing field `label` at line 1 column 2"
    );
    let pinned: corrected::Pinned = serde_json::from_str(r#"{"dock":"north","tag":"t"}"#).unwrap();
    assert_eq!(
        serde_json::to_string(&pinned).unwrap(),
        r#"{"dock":"north","keys":["tag"],"tag":"t"}"#
    );
    let Some((printed, failed)) = loaded_from("unheld", &corrections(), UNHELD_DRIVER) else {
        return;
    };
    assert_eq!(
        printed, UNHELD,
        "the bundle does not load and parse under node:\n{failed}"
    );
}

#[test]
fn a_union_around_a_factorys_call_is_built_over_a_type_parameter() {
    let zod = DeclaredChain::<String>::zod_schema();
    for member in [
        "  get next() { return (<Reached$ extends SomeType>(reached$: Reached$) => \
         z.union([z.null().transform(() => undefined), reached$, \
         z.undefined()]).prefault(undefined))(DeclaredChain$SchemaFactory(idType)); },",
        "  get many() { return (<Reached$ extends SomeType>(reached$: Reached$) => \
         z.union([z.null().transform(() => undefined), z.array(reached$), \
         z.undefined()]).prefault(undefined))(DeclaredChain$SchemaFactory(idType)); },",
        "  get named() { return (<Reached$ extends SomeType>(reached$: Reached$) => \
         z.union([z.record(z.string(), reached$), \
         z.null()]))(DeclaredChain$SchemaSelf(idType)); },",
    ] {
        assert!(zod.contains(member), "missing `{member}` in: {zod}");
    }
}

#[test]
fn a_map_of_the_type_itself_is_read_through_the_self_view() {
    let zod = DeclaredMaps::<String>::zod_schema();
    for written in [
        "function DeclaredMaps$SchemaSelf<IdType extends ZodType>(\n  idType: IdType,\n): \
         ZodType<DeclaredMaps<IdType[\"_zod\"][\"output\"]>>;",
        "  get held() { return z.record(z.string(), DeclaredMaps$SchemaSelf(idType)); },",
        "  get maybe() { return (<Reached$ extends SomeType>(reached$: Reached$) => \
         z.union([z.null().transform(() => undefined), z.record(z.string(), reached$), \
         z.undefined()]).prefault(undefined))(DeclaredMaps$SchemaSelf(idType)); },",
        "  get rows() { return z.array(z.record(z.string(), DeclaredMaps$SchemaSelf(idType))); },",
    ] {
        assert!(zod.contains(written), "missing `{written}` in: {zod}");
    }
    let node = DeclaredNode::<String>::zod_schema();
    assert!(
        node.contains("  get children() { return z.array(DeclaredNode$SchemaFactory(idType)); },")
            && !node.contains("$SchemaSelf"),
        "a list of the type itself keeps the factory's own type. Got: {node}"
    );
}

fn with_its_code<CodeType>(value: &CodedCrate<CodeType>) -> Result<(), Vec<String>> {
    if value.codes.is_empty() {
        return Err(vec!["'codes': a crate holds at least one code".to_owned()]);
    }
    Ok(())
}

#[test]
fn type_check_the_check_of_a_generic_type_that_holds_its_parameter_compiles() {
    let source = [
        PREAMBLE.to_owned(),
        WITH_ITS_CODE.to_owned(),
        CodedCrate::<String>::ts_definition(),
        CodedCrate::<String>::zod_schema(),
    ]
    .join("\n\n");
    let Some(at) = built_from("generic-check", &source) else {
        return;
    };
    fs::remove_dir_all(at).unwrap();
}

/// Every payload serde writes of a range is parsed by the Zod schema of its type under `node` and
/// admitted by the type's JSON Schema, and every payload serde refuses is refused by both: a key
/// no range writes, a bound left out, a bound of another type, and a bound that breaks its own.
#[cfg(feature = "jsonschema")]
#[test]
fn type_check_a_range_parses_under_zod_what_serde_writes() {
    let mut payloads = ranged::read_payloads();
    payloads.extend(ranged::refused_payloads());
    payloads.extend(ranged::judged_payloads());
    let mut expected = Vec::new();
    for payload in &payloads {
        let (read, document) = ranged_models()
            .into_iter()
            .find(|(named, _, _)| *named == payload.read_as)
            .map(|(_, reads, document)| (reads(&payload.json), document()))
            .unwrap();
        let value: serde_json::Value = serde_json::from_str(&payload.json).unwrap();
        assert_eq!(
            documents::admits(&document, &value),
            read,
            "the JSON Schema of {} and serde disagree on {}: {}",
            payload.read_as,
            payload.name,
            payload.json
        );
        expected.push(if read {
            format!("{}: read {}", payload.name, sorted_keys(&value))
        } else if payload.undeclared_key {
            format!("{}: refused for a key it does not declare", payload.name)
        } else {
            format!("{}: refused", payload.name)
        });
    }
    let listed: Vec<(&str, &str, &str)> = payloads
        .iter()
        .map(|payload| (payload.name, payload.read_as, payload.json.as_str()))
        .collect();
    let driver = RANGES_DRIVER.replace("PAYLOADS", &serde_json::to_string(&listed).unwrap());
    let Some((printed, failed)) = loaded_from("ranges", &ranges(), &driver) else {
        return;
    };
    assert_eq!(
        printed,
        expected.join("\n"),
        "the schemas do not read what a serde read reads:\n{failed}"
    );
}

/// Every payload serde writes of a `Result` is parsed by the Zod schema of its type under `node`
/// and admitted by the type's JSON Schema, and every payload serde refuses is refused by both: an
/// object of both keys, of neither and of another key, a value under an arm of `()`, an arm of
/// another type, and a model in an arm that breaks its bounds, each issue under its path.
#[cfg(feature = "jsonschema")]
#[test]
fn type_check_a_result_parses_under_zod_what_serde_writes() {
    let mut payloads = resulted::read_payloads();
    payloads.extend(resulted::refused_payloads());
    payloads.extend(resulted::judged_payloads());
    let mut expected = Vec::new();
    let mut listed: Vec<(&str, &str, &str, bool)> = Vec::new();
    for payload in &payloads {
        let (_, judged, read, document) = resulted_models()
            .into_iter()
            .find(|(named, _, _, _)| *named == payload.read_as)
            .unwrap();
        let (reads, printed) = read(payload.name, &payload.json, judged);
        let value: serde_json::Value = serde_json::from_str(&payload.json).unwrap();
        assert_eq!(
            documents::admits(&document(), &value),
            reads,
            "the JSON Schema of {} and serde disagree on {}: {}",
            payload.read_as,
            payload.name,
            payload.json
        );
        expected.push(printed);
        listed.push((payload.name, payload.read_as, payload.json.as_str(), judged));
    }
    let driver = RESULTS_DRIVER.replace("PAYLOADS", &serde_json::to_string(&listed).unwrap());
    let Some((printed, failed)) = loaded_from("results", &results(), &driver) else {
        return;
    };
    assert_eq!(
        printed,
        expected.join("\n"),
        "the schemas do not read what a serde read reads:\n{failed}"
    );
}

/// The JSON Schema of the model of [`UNITS`] named `read_as`.
#[cfg(feature = "jsonschema")]
fn unit_document(read_as: &str) -> serde_json::Value {
    if read_as == "DeclaredNothing" {
        DeclaredNothing::json_schema()
    } else {
        DeclaredUnits::json_schema()
    }
}

/// `()` is read under Zod as serde reads it, `null` in every place it is held and no empty array,
/// and the JSON Schema of each model admits what serde reads.
#[test]
fn type_check_a_unit_parses_under_zod_what_serde_reads() {
    let mut expected = Vec::new();
    for (name, read_as, payload) in UNITS {
        let reads = if read_as == "DeclaredNothing" {
            serde_json::from_str::<DeclaredNothing>(payload).is_ok()
        } else {
            serde_json::from_str::<DeclaredUnits>(payload).is_ok()
        };
        #[cfg(feature = "jsonschema")]
        assert_eq!(
            documents::admits(
                &unit_document(read_as),
                &serde_json::from_str(payload).unwrap()
            ),
            reads,
            "the JSON Schema of {read_as} and serde disagree on {name}: {payload}"
        );
        expected.push(format!(
            "{name}: {}",
            if reads { "read" } else { "refused" }
        ));
    }
    assert_eq!(
        expected
            .iter()
            .filter(|line| line.ends_with(": read"))
            .count(),
        2,
        "serde reads `null` alone: {expected:?}"
    );
    let source = [
        PREAMBLE.to_owned(),
        DeclaredNothing::ts_definition(),
        DeclaredNothing::zod_schema(),
        DeclaredUnits::ts_definition(),
        DeclaredUnits::zod_schema(),
    ]
    .join("\n\n");
    let driver = UNITS_DRIVER.replace("PAYLOADS", &serde_json::to_string(&UNITS).unwrap());
    let Some((printed, failed)) = loaded_from("units", &source, &driver) else {
        return;
    };
    assert_eq!(
        printed,
        expected.join("\n"),
        "the schemas do not read what a serde read reads:\n{failed}"
    );
}

/// A map whose values are `()` is described as an object of `null` values on every surface, and
/// each surface reads what serde reads.
#[test]
#[expect(
    clippy::zero_sized_map_values,
    reason = "a map of `()` is the shape under test"
)]
fn type_check_a_map_of_units_parses_under_zod_what_serde_reads() {
    use alloc::collections::BTreeMap;

    #[model_schema()]
    #[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
    pub struct ProbeUnitsKeyed {
        pub keyed: BTreeMap<String, ()>,
    }

    const WRITTEN: &str = r#"{"keyed":{"a":null}}"#;
    const LISTED: &str = r#"{"keyed":{"a":[]}}"#;

    let value = ProbeUnitsKeyed {
        keyed: BTreeMap::from([("a".to_owned(), ())]),
    };
    assert_eq!(
        serde_json::to_string(&value).unwrap(),
        WRITTEN,
        "serde's write"
    );
    assert_eq!(
        serde_json::from_str::<ProbeUnitsKeyed>(WRITTEN).unwrap(),
        value,
        "serde's read of its write"
    );
    let refused = serde_json::from_str::<ProbeUnitsKeyed>(LISTED)
        .unwrap_err()
        .to_string();
    assert!(
        refused.starts_with("invalid type: sequence, expected unit"),
        "serde's refusal of an empty array: {refused}"
    );

    let ts = ProbeUnitsKeyed::ts_definition();
    assert!(
        ts.contains("keyed: Partial<Record<string, null>>"),
        "the TypeScript text:\n{ts}"
    );
    let zod = ProbeUnitsKeyed::zod_schema();
    assert!(
        zod.contains("keyed: z.record(z.string(), z.null()),"),
        "the Zod text:\n{zod}"
    );
    #[cfg(feature = "jsonschema")]
    {
        let document = ProbeUnitsKeyed::json_schema();
        assert_eq!(
            document,
            serde_json::json!({
                "additionalProperties": false,
                "properties": {
                    "keyed": {"additionalProperties": {"type": "null"}, "type": "object"}
                },
                "required": ["keyed"],
                "type": "object"
            }),
            "the JSON Schema"
        );
        for (payload, admitted) in [(WRITTEN, true), (LISTED, false)] {
            assert_eq!(
                documents::admits(&document, &serde_json::from_str(payload).unwrap()),
                admitted,
                "the JSON Schema and serde disagree on {payload}"
            );
        }
    }

    let payloads = [
        ("written by serde", "ProbeUnitsKeyed", WRITTEN),
        ("an empty array", "ProbeUnitsKeyed", LISTED),
    ];
    let source = [PREAMBLE.to_owned(), ts, zod].join("\n\n");
    let driver = UNITS_DRIVER.replace("PAYLOADS", &serde_json::to_string(&payloads).unwrap());
    let Some((printed, failed)) = loaded_from("map-of-units", &source, &driver) else {
        return;
    };
    assert_eq!(
        printed, "written by serde: read\nan empty array: refused",
        "the Zod schema does not read what a serde read reads:\n{failed}"
    );
}

/// Every payload that repeats an item of a set is refused by the Zod schema of its type under
/// `node`, each set under its path, in every place a set is held, where Rust's read keeps one item
/// of it.
#[test]
fn type_check_a_set_refuses_a_repeat_under_zod() {
    sets::assert_rust_keeps_one_item();
    let listed: Vec<(&str, String, &str)> = sets::PAYLOADS
        .iter()
        .map(|payload| {
            let binding = if sets::declares_a_parameter(payload.read_as) {
                "SchemaDefault"
            } else {
                "Schema"
            };
            (
                payload.name,
                format!("{}${binding}", payload.read_as),
                payload.json,
            )
        })
        .collect();
    let driver = SETS_DRIVER.replace("PAYLOADS", &serde_json::to_string(&listed).unwrap());
    let Some((printed, failed)) = loaded_from("sets", &sets(), &driver) else {
        return;
    };
    assert_eq!(
        format!("{printed}\n"),
        sets::PRINTED,
        "the schemas do not refuse a repeat:\n{failed}"
    );
}

/// Each payload of a `char` and of a chrono date, time and date and time is read by the Zod schema
/// of its type under `node` exactly where [`text_forms::TEXT_FORMS`] states, which the Kotlin read
/// is held to.
#[cfg(feature = "chrono")]
#[test]
fn type_check_a_text_of_one_form_is_read_under_zod_where_the_table_states() {
    let taken: Vec<bool> = text_forms::TEXT_FORMS.iter().map(|row| row.zod).collect();
    let listed: Vec<(String, &str)> = text_forms::TEXT_FORMS
        .iter()
        .map(|row| (format!("{}$Schema", row.read_as), row.json))
        .collect();
    let driver = TAKEN_DRIVER.replace("PAYLOADS", &serde_json::to_string(&listed).unwrap());
    let Some((printed, failed)) = loaded_from("text-forms", &text_forms_bundle(), &driver) else {
        return;
    };
    assert_eq!(
        printed,
        serde_json::to_string(&taken).unwrap(),
        "the schemas do not read what the table states:\n{failed}"
    );
}

/// The Zod schema of each enum of [`untagged_millis::MILLIS_ROWS`] parses each payload under `node`
/// to what the table states, the epoch milliseconds serde writes among them. Its output is typed
/// `ZodType` of the enum's TypeScript type, which `tsc --strict` holds it to as the bundle
/// compiles.
#[cfg(feature = "chrono")]
#[test]
fn type_check_an_untagged_member_under_as_number_is_parsed_as_the_number_serde_writes() {
    let listed: Vec<(String, &str)> = untagged_millis::MILLIS_ROWS
        .iter()
        .map(|(read_as, json, written, _)| {
            assert_eq!(
                untagged_millis::serde_written(read_as, json).as_deref(),
                *written,
                "serde answers {json} read as {read_as} otherwise"
            );
            (format!("{read_as}$Schema"), *json)
        })
        .collect();
    let expected: Vec<Option<&str>> = untagged_millis::MILLIS_ROWS
        .iter()
        .map(|(_, _, _, zod)| *zod)
        .collect();
    let driver = format!(
        "import * as lib from \"./lib/index.js\";\n\n\
         console.log(JSON.stringify({}.map(([schema, payload]) => {{\n  \
         const read = lib[schema].safeParse(JSON.parse(payload));\n  \
         return read.success ? JSON.stringify(read.data) : null;\n\
         }})));\n",
        serde_json::to_string(&listed).unwrap()
    );
    let Some((printed, failed)) =
        loaded_from("untagged-millis", &untagged_millis_bundle(), &driver)
    else {
        return;
    };
    let parsed: Vec<Option<String>> = serde_json::from_str(&printed).unwrap_or_default();
    assert_eq!(
        parsed.iter().map(Option::as_deref).collect::<Vec<_>>(),
        expected,
        "the Zod schemas parse a payload otherwise than the table states:\n{failed}"
    );
}

/// Each value of [`wire_forms::WIRE_FORMS`] is sent by the Zod schema of its model under `node`
/// in the form the table states, which is what the clients over every transport send, as the
/// check's own output with each `bigint` written as its digits, and the schema reads every
/// payload any client sends but a number a JavaScript `number` cannot hold, which it refuses.
#[cfg(feature = "chrono")]
#[test]
fn type_check_each_value_is_sent_under_zod_in_the_form_every_target_reads() {
    let handed = wire_forms::WIRE_FORMS
        .iter()
        .filter(|row| {
            !matches!(
                row.typescript,
                wire_forms::Sends::Unheld | wire_forms::Sends::Unemitted
            )
        })
        .fold(String::new(), |mut written, row| {
            let _ = writeln!(
                written,
                "  sent({}, {}),",
                wire_form_schema(row.held_by),
                wire_form_handed(row)
            );
            written
        });
    let expected: Vec<String> = wire_forms::WIRE_FORMS
        .iter()
        .filter(|row| {
            !matches!(
                row.typescript,
                wire_forms::Sends::Unheld | wire_forms::Sends::Unemitted
            )
        })
        .map(|row| {
            row.payload(row.typescript)
                .unwrap_or_else(|| "refused".to_owned())
        })
        .collect();
    let read: Vec<(String, String, bool)> = wire_forms::WIRE_FORMS
        .iter()
        .flat_map(|row| {
            let held = row.typescript != wire_forms::Sends::Unheld;
            row.sent()
                .into_iter()
                .map(move |sent| (wire_form_schema(row.held_by), sent, held))
        })
        .collect();
    let driver = format!(
        "import * as lib from \"./lib/index.js\";\n\n\
         {wire}\n\
         const sent = (schema, handed) => {{\n  \
         const read = schema.safeParse(wireForm(handed));\n  \
         return read.success ? JSON.stringify(wireForm(read.data)) : \"refused\";\n\
         }};\n\
         const {{ {schemas} }} = lib;\n\
         console.log(JSON.stringify([\n{handed}]));\n\
         console.log(JSON.stringify({read}.map(([schema, payload]) => \
         lib[schema].safeParse(JSON.parse(payload)).success)));\n",
        wire = WIRE_FORM,
        schemas = wire_forms::WIRE_FORMS
            .iter()
            .map(|row| wire_form_schema(row.held_by))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .join(", "),
        read = serde_json::to_string(
            &read
                .iter()
                .map(|(schema, sent, _)| (schema, sent))
                .collect::<Vec<_>>()
        )
        .unwrap(),
    );
    let Some((printed, failed)) = loaded_from("wire-forms", &wire_forms_bundle(), &driver) else {
        return;
    };
    let mut lines = printed.lines();
    let sent: Vec<String> = serde_json::from_str(lines.next().unwrap_or("[]")).unwrap();
    assert_eq!(
        sent, expected,
        "the Zod schemas do not send each value as the table states:\n{failed}"
    );
    let readable: Vec<bool> = serde_json::from_str(lines.next().unwrap_or("[]")).unwrap();
    let misread: Vec<&(String, String, bool)> = read
        .iter()
        .zip(&readable)
        .filter_map(|(sent_by_a_client, taken)| {
            (*taken != sent_by_a_client.2).then_some(sent_by_a_client)
        })
        .collect();
    assert!(
        misread.is_empty() && readable.len() == read.len(),
        "the Zod schemas read a payload a client sends otherwise than the table states, a \
         value a JavaScript number cannot hold being refused: {misread:?}\n{failed}"
    );
}

/// The Zod schema under `node` reads each `u64` and `i64` a member writes itself from the string
/// of its digits into a `bigint`, exactly, and refuses one beyond its type or its bound in the
/// sentence `validate()` writes, and what it reads is written back by a client as serde wrote it.
#[test]
fn type_check_a_64_bit_integer_is_read_as_a_bigint_from_its_digits_under_zod() {
    assert_eq!(
        serde_json::to_string(
            &serde_json::from_str::<digit_strings::DigitTally>(DIGIT_TALLY).unwrap()
        )
        .unwrap(),
        DIGIT_TALLY,
        "serde writes the tally the driver reads"
    );
    let payloads = [
        ("DigitTally$Schema", DIGIT_TALLY.to_owned()),
        (
            "DigitTally$Schema",
            DIGIT_TALLY.replace(r#""signed":"-1152921504606846976""#, r#""signed":-1"#),
        ),
        ("DigitId$Schema", r#""9223372036854775808""#.to_owned()),
        ("DigitId$Schema", r#""1.5""#.to_owned()),
        ("DigitEvent$Schema", r#"{"Counted":"5"}"#.to_owned()),
        (
            "DigitBounded$Schema",
            r#"{"level":"6","start":"1"}"#.to_owned(),
        ),
        ("DigitBounded$Schema", r#"{"level":"-5"}"#.to_owned()),
    ];
    let driver = DIGIT_STRINGS_DRIVER
        .replace("PAYLOADS", &serde_json::to_string(&payloads).unwrap())
        .replace("WIRE_FORM", WIRE_FORM)
        .replace("TALLY", &serde_json::to_string(DIGIT_TALLY).unwrap());
    let Some((printed, failed)) = loaded_from("digit-strings", &digit_strings_bundle(), &driver)
    else {
        return;
    };
    assert_eq!(
        printed,
        [
            r#"{"boxed":"-1152921504606846976n","grid":["1n","1152921504606846976n"],"keyed":{"a":"-7n"},"listed":["0n","9223372036854775807n"],"maybe":"-9223372036854775808n","nested":[["3n"],null],"paired":["9n","x"],"set":["2n","5n"],"signed":"-1152921504606846976n"}"#,
            "Invalid input: expected string, received number",
            "Too big: expected bigint to be <=9223372036854775807",
            "Invalid string: must match pattern /^-?(0|[1-9][0-9]*)$/",
            r#"{"Counted":"5n"}"#,
            "too large: maximum is 5, got 6",
            r#"{"level":"-5n","start":"7n"}"#,
            DIGIT_TALLY,
        ]
        .join("\n"),
        "the Zod schemas do not read each payload as stated:\n{failed}"
    );
}

/// The Zod schema of each enum under `node` refuses the payload of a variant serde neither writes
/// nor reads, as serde refuses it, and takes the payload of every variant serde writes, and of a
/// variant serde drops in one direction alone.
#[test]
fn type_check_a_variant_serde_neither_writes_nor_reads_is_refused_under_zod() {
    for row in &skipped_variants::ROWS {
        assert_eq!(
            (row.read)(row.json),
            row.serde,
            "serde's read of {} as {}",
            row.json,
            row.read_as
        );
    }
    assert!(
        skipped_variants::unread_writes()
            .iter()
            .rev()
            .skip(1)
            .all(|said| said.ends_with("cannot be serialized")),
        "serde writes a variant it skips"
    );
    let taken: Vec<bool> = skipped_variants::ROWS.iter().map(|row| row.zod).collect();
    let listed: Vec<(String, &str)> = skipped_variants::ROWS
        .iter()
        .map(|row| (format!("{}$Schema", row.read_as), row.json))
        .collect();
    let driver = TAKEN_DRIVER.replace("PAYLOADS", &serde_json::to_string(&listed).unwrap());
    let Some((printed, failed)) =
        loaded_from("skipped-variants", &skipped_variants_bundle(), &driver)
    else {
        return;
    };
    assert_eq!(
        printed,
        serde_json::to_string(&taken).unwrap(),
        "the schemas do not take what the table states:\n{failed}"
    );
}

/// Every payload serde reads with a map flattened into its object is read by the Zod schema of its
/// type under `node`, and one with a count of the wrong type is refused under the count's key:
/// serde hands a flattened map every key of its object but the tag an internally tagged enum reads
/// ahead of the variant. An optional map with such a count is read, as serde reads it as `None`.
#[test]
fn type_check_a_flattened_map_reads_under_zod_what_serde_reads() {
    flattened_maps::assert_serde_reads_what_zod_reads();
    let listed: Vec<(&str, String, &str)> = flattened_maps::PAYLOADS
        .iter()
        .map(|payload| {
            (
                payload.name,
                format!("{}$Schema", payload.read_as),
                payload.json,
            )
        })
        .collect();
    let driver =
        FLATTENED_MAPS_DRIVER.replace("PAYLOADS", &serde_json::to_string(&listed).unwrap());
    let Some((printed, failed)) = loaded_from("flattened-maps", &flattened_maps(), &driver) else {
        return;
    };
    assert_eq!(
        format!("{printed}\n"),
        flattened_maps::PRINTED,
        "the schemas do not read what serde reads:\n{failed}"
    );
}

/// Every payload of a map keyed by a value serde writes as text of its own is read by the Zod
/// schema of its model under `node` as serde reads it, but where serde parts from Zod: the text of
/// a number key Zod reads as the number, and an enum's content ahead of its tag.
#[cfg(feature = "chrono")]
#[test]
fn type_check_a_map_keyed_by_text_reads_under_zod_what_serde_reads() {
    let listed: Vec<(&str, String, &str)> = keyed_maps::PAYLOADS
        .iter()
        .map(|payload| {
            (
                payload.name,
                format!("{}$Schema", payload.read_as),
                payload.json,
            )
        })
        .collect();
    let driver =
        FLATTENED_MAPS_DRIVER.replace("PAYLOADS", &serde_json::to_string(&listed).unwrap());
    let Some((printed, failed)) = loaded_from("keyed-maps", &keyed_maps(), &driver) else {
        return;
    };
    assert_eq!(
        format!("{printed}\n"),
        keyed_maps::PRINTED,
        "the schemas do not read what they are stated to read:\n{failed}"
    );
    keyed_maps::assert_serde_reads_what_zod_reads();
}

/// An optional flattened struct is read by the Zod schema of its holder under `node` where it is
/// absent and where its key reads, and refused where its key does not read, a payload serde reads
/// as `None`, dropping the key.
#[test]
fn type_check_an_optional_flattened_struct_refuses_under_zod_a_key_serde_drops() {
    flattened_maps::assert_zod_refuses_each_key_serde_drops();
    let listed: Vec<(&str, &str, &str)> = flattened_maps::OPTIONAL_STRUCT_PAYLOADS
        .iter()
        .map(|&(name, json, _)| (name, "Hawser$Schema", json))
        .collect();
    let driver =
        FLATTENED_MAPS_DRIVER.replace("PAYLOADS", &serde_json::to_string(&listed).unwrap());
    let Some((printed, failed)) = loaded_from("optional-struct", &flattened_maps(), &driver) else {
        return;
    };
    assert_eq!(
        format!("{printed}\n"),
        flattened_maps::OPTIONAL_STRUCT_PRINTED,
        "the schema does not read what it is stated to read:\n{failed}"
    );
}

/// An optional flattened member whose type the Kotlin emitter recorded no struct's keys for is
/// taken by TypeScript and by the Zod schema of its holder under `node` where it is absent and
/// where its type reads, and refused where a key of its type does not read, a payload serde reads
/// as `None`, dropping the key.
#[test]
fn type_check_an_optional_flattened_type_is_refused_where_a_key_of_it_does_not_read() {
    let taken: Vec<bool> = unrecorded::UNRECORDED
        .iter()
        .map(|row| row.taken_by_zod)
        .collect();
    let listed: Vec<(String, &str)> = unrecorded::UNRECORDED
        .iter()
        .map(|row| (format!("{}$Schema", row.holder), row.json))
        .collect();
    let driver = TAKEN_DRIVER.replace("PAYLOADS", &serde_json::to_string(&listed).unwrap());
    let Some((printed, failed)) = loaded_from("unrecorded", &unrecorded_options(), &driver) else {
        return;
    };
    assert_eq!(
        printed,
        serde_json::to_string(&taken).unwrap(),
        "the schemas do not take what they are stated to take:\n{failed}"
    );
    let Some(at) = built_from("unrecorded-payloads", &unrecorded_options()) else {
        return;
    };
    let mut payloads = String::from(
        "import type { Bridling, Fiddling, Slipping, Snagging } from \"./lib/index.js\";\n",
    );
    for (position, row) in unrecorded::UNRECORDED.iter().enumerate() {
        let _ = writeln!(
            payloads,
            "export const row{position}: {} = {};",
            row.holder, row.json
        );
    }
    fs::write(at.join("tsconfig.consumer.json"), CONSUMER_PROJECT).unwrap();
    fs::write(at.join("consumer.ts"), &payloads).unwrap();
    let (_, said) = compiled(&at, "tsconfig.consumer.json").unwrap();
    fs::remove_dir_all(&at).unwrap();
    // A row's line is its position after the import.
    let typed: Vec<bool> = (0..taken.len())
        .map(|row| !said.contains(&format!("consumer.ts({},", row + 2)))
        .collect();
    assert_eq!(
        typed, taken,
        "TypeScript does not take what it is stated to take:\n{said}\n{payloads}"
    );
}

/// An optional flattened member that names a function is read by the Zod schema of its holder
/// under `node` from what the function answers, and is absent where that read fails and the object
/// holds none of its type's keys, and the payload refused where it holds one. TypeScript, which
/// calls no function, takes the type's keys whatever the function answers.
#[test]
fn type_check_an_optional_flattened_type_that_names_a_function_is_read_from_its_answer() {
    let listed: Vec<(String, &str)> = corrected_options::CORRECTED
        .iter()
        .map(|row| (format!("{}$Schema", row.holder), row.json))
        .collect();
    let driver = READ_DRIVER.replace("PAYLOADS", &serde_json::to_string(&listed).unwrap());
    let Some((printed, failed)) =
        loaded_from("corrected-options", &corrected_options_bundle(), &driver)
    else {
        return;
    };
    let zod: Vec<&str> = corrected_options::CORRECTED
        .iter()
        .map(|row| row.zod)
        .collect();
    assert_eq!(
        printed,
        zod.join("\n"),
        "the schemas do not read what they are stated to read:\n{failed}"
    );
    let Some(at) = built_from("corrected-options-payloads", &corrected_options_bundle()) else {
        return;
    };
    let mut payloads =
        String::from("import type { Moused, Reeved, Wormed } from \"./lib/index.js\";\n");
    for (position, row) in corrected_options::CORRECTED.iter().enumerate() {
        let _ = writeln!(
            payloads,
            "export const row{position}: {} = {};",
            row.holder, row.json
        );
    }
    fs::write(at.join("tsconfig.consumer.json"), CONSUMER_PROJECT).unwrap();
    fs::write(at.join("consumer.ts"), &payloads).unwrap();
    let (_, said) = compiled(&at, "tsconfig.consumer.json").unwrap();
    fs::remove_dir_all(&at).unwrap();
    // A row's line is its position after the import.
    let typed: Vec<bool> = (0..corrected_options::CORRECTED.len())
        .map(|row| !said.contains(&format!("consumer.ts({},", row + 2)))
        .collect();
    let stated: Vec<bool> = corrected_options::CORRECTED
        .iter()
        .map(|row| row.typed)
        .collect();
    assert_eq!(
        typed, stated,
        "TypeScript does not take what it is stated to take:\n{said}\n{payloads}"
    );
}

/// A flattened struct is read by the Zod schema of its holder under `node` with a key a member of
/// its holder's own, or a struct flattened ahead of it, reads too, an optional one where that key
/// is there, and TypeScript takes the payload, where serde refuses it or reads `None`.
#[test]
fn type_check_a_key_read_by_a_struct_and_the_type_around_it_is_read_under_zod() {
    let listed: Vec<(String, &str)> = shared_keys::SHARED_KEYS
        .iter()
        .map(|row| (format!("{}$Schema", row.holder), row.json))
        .collect();
    let driver = READ_DRIVER.replace("PAYLOADS", &serde_json::to_string(&listed).unwrap());
    let Some((printed, failed)) = loaded_from("shared-keys", &shared_keys_bundle(), &driver) else {
        return;
    };
    let zod: Vec<&str> = shared_keys::SHARED_KEYS.iter().map(|row| row.zod).collect();
    assert_eq!(
        printed,
        zod.join("\n"),
        "the schemas do not read what they are stated to read:\n{failed}"
    );
    let Some(at) = built_from("shared-keys-payloads", &shared_keys_bundle()) else {
        return;
    };
    let mut payloads = String::from(
        "import type { Bitts, Bowline, Clinching, Gudgeon, Halliard, Parrel, Rubrail } from \
         \"./lib/index.js\";\n",
    );
    for (position, row) in shared_keys::SHARED_KEYS.iter().enumerate() {
        let _ = writeln!(
            payloads,
            "export const row{position}: {} = {};",
            row.holder, row.json
        );
    }
    fs::write(at.join("tsconfig.consumer.json"), CONSUMER_PROJECT).unwrap();
    fs::write(at.join("consumer.ts"), &payloads).unwrap();
    let (passed, said) = compiled(&at, "tsconfig.consumer.json").unwrap();
    fs::remove_dir_all(&at).unwrap();
    assert!(
        passed,
        "TypeScript does not take each payload:\n{said}\n{payloads}"
    );
}

/// The TypeScript type of an externally tagged enum takes one variant's entry and the name of a
/// variant that holds none, and refuses an object of two variants' entries, as serde and the Zod
/// schema under `node` refuse it.
#[test]
fn type_check_an_externally_tagged_enum_refuses_an_object_of_two_variants_entries() {
    let stated: Vec<bool> = HATCHES.iter().map(|&(_, _, _, taken)| taken).collect();
    let read: Vec<bool> = HATCHES
        .iter()
        .map(|&(_, typed, json, _)| {
            if typed == "DeclaredHatch" {
                serde_json::from_str::<DeclaredHatch>(json).is_ok()
            } else {
                serde_json::from_str::<DeclaredLading<String>>(json).is_ok()
            }
        })
        .collect();
    assert_eq!(read, stated, "serde does not take what the rows state");
    let listed: Vec<(&str, &str)> = HATCHES
        .iter()
        .map(|&(schema, _, json, _)| (schema, json))
        .collect();
    let driver = TAKEN_DRIVER.replace("PAYLOADS", &serde_json::to_string(&listed).unwrap());
    let Some((printed, failed)) = loaded_from("hatches", &hatches_bundle(), &driver) else {
        return;
    };
    assert_eq!(
        printed,
        serde_json::to_string(&stated).unwrap(),
        "the schemas do not take what the rows state:\n{failed}"
    );
    let Some(at) = built_from("hatches-payloads", &hatches_bundle()) else {
        return;
    };
    let mut payloads =
        String::from("import type { DeclaredHatch, DeclaredLading } from \"./lib/index.js\";\n");
    for (position, &(_, typed, json, _)) in HATCHES.iter().enumerate() {
        let _ = writeln!(payloads, "export const row{position}: {typed} = {json};");
    }
    fs::write(at.join("tsconfig.consumer.json"), CONSUMER_PROJECT).unwrap();
    fs::write(at.join("consumer.ts"), &payloads).unwrap();
    let (_, said) = compiled(&at, "tsconfig.consumer.json").unwrap();
    fs::remove_dir_all(&at).unwrap();
    // A row's line is its position after the import.
    let typed: Vec<bool> = (0..HATCHES.len())
        .map(|row| !said.contains(&format!("consumer.ts({},", row + 2)))
        .collect();
    assert_eq!(
        typed, stated,
        "TypeScript does not take what the rows state:\n{said}\n{payloads}"
    );
}

/// An externally tagged enum flattened under a member that names a function, a `preprocess` or a
/// `check`, is taken by the Zod schema of its holder under `node` as serde writes and reads it:
/// each variant's entry, a unit variant's over `null` among them, and not a unit variant's over
/// any other value. A check, the member's or the enum's own, is handed the enum's value, the unit
/// variant's bare name, and refuses it as the Rust check does.
#[test]
fn type_check_a_flattened_enum_under_a_function_takes_a_unit_variants_entry_over_null() {
    let checked = || {
        YAWS.iter()
            .map(|&(json, taken)| taken && !json.contains("Calm"))
    };
    let stated: Vec<bool> = YAWS
        .iter()
        .map(|&(_, taken)| taken)
        .chain(checked())
        .chain(checked())
        .collect();
    let read: Vec<bool> = YAWS
        .iter()
        .map(|&(json, _)| serde_json::from_str::<DeclaredYaw>(json).is_ok())
        .chain(
            YAWS.iter()
                .map(|&(json, _)| serde_json::from_str::<DeclaredSway>(json).is_ok()),
        )
        .chain(
            YAWS.iter()
                .map(|&(json, _)| serde_json::from_str::<DeclaredDrift>(json).is_ok()),
        )
        .collect();
    assert_eq!(read, stated, "serde does not take what the rows state");
    let listed: Vec<(&str, &str)> = YAWS
        .iter()
        .map(|&(json, _)| ("DeclaredYaw$Schema", json))
        .chain(YAWS.iter().map(|&(json, _)| ("DeclaredSway$Schema", json)))
        .chain(YAWS.iter().map(|&(json, _)| ("DeclaredDrift$Schema", json)))
        .collect();
    let driver = TAKEN_DRIVER.replace("PAYLOADS", &serde_json::to_string(&listed).unwrap());
    let Some((printed, failed)) = loaded_from("yaws", &hatches_bundle(), &driver) else {
        return;
    };
    assert_eq!(
        printed,
        serde_json::to_string(&stated).unwrap(),
        "the schema does not take what the rows state:\n{failed}"
    );
}

/// The models whose names on the wire and doc comments hold the end of a comment compile under
/// `tsc --strict`, each comment running on past it, and the Zod schema of each reads what serde
/// writes of it as serde writes it.
#[test]
fn type_check_a_name_or_a_doc_that_ends_a_comment_is_read_under_zod_as_serde_writes_it() {
    let bundle = commented_bundle();
    for kept in [
        " * a*\\/b",
        " * A member doc that holds *\\/ the end of a comment, beside a bound.",
        " * An item doc that holds *\\/ the end of a comment.",
        "  * A variant doc that holds *\\/ the end of a comment.",
        " * p*\\/q",
        " * t*\\/u",
        " * An alias whose doc holds *\\/ the end of a comment.",
    ] {
        assert!(
            bundle.contains(kept),
            "the comment does not hold `{kept}`:\n{bundle}"
        );
    }
    let listed = commented::written();
    let expected: Vec<serde_json::Value> = listed
        .iter()
        .map(|(_, written)| serde_json::from_str(written).unwrap())
        .collect();
    let driver = QUOTED_DRIVER.replace("PAYLOADS", &serde_json::to_string(&listed).unwrap());
    let Some((printed, failed)) = loaded_from("commented", &bundle, &driver) else {
        return;
    };
    let read: Vec<serde_json::Value> = printed
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(
        read, expected,
        "Zod does not read what serde writes:\n{failed}"
    );
}

/// An externally tagged enum in the lone slot of a variant under a tag alone, under a function or
/// not, is taken by the Zod schema of the enum that holds it under `node` and by its TypeScript type
/// under `tsc --strict` as serde writes and reads it: each variant's entry beside the tag, a unit
/// variant's over `null` among them.
#[test]
fn type_check_a_slot_beside_the_tag_takes_a_unit_variants_entry_over_null() {
    let stated: Vec<bool> = MOORINGS.iter().map(|&(_, taken)| taken).collect();
    let read: Vec<bool> = MOORINGS
        .iter()
        .map(|&(json, _)| serde_json::from_str::<DeclaredMooring>(json).is_ok())
        .collect();
    assert_eq!(read, stated, "serde does not take what the rows state");
    let listed: Vec<(&str, &str)> = MOORINGS
        .iter()
        .map(|&(json, _)| ("DeclaredMooring$Schema", json))
        .collect();
    let driver = TAKEN_DRIVER.replace("PAYLOADS", &serde_json::to_string(&listed).unwrap());
    let Some((printed, failed)) = loaded_from("moorings", &hatches_bundle(), &driver) else {
        return;
    };
    assert_eq!(
        printed,
        serde_json::to_string(&stated).unwrap(),
        "the schema does not take what the rows state:\n{failed}"
    );
    let Some(at) = built_from("moorings-payloads", &hatches_bundle()) else {
        return;
    };
    let mut payloads = String::from("import type { DeclaredMooring } from \"./lib/index.js\";\n");
    for (position, &(json, _)) in MOORINGS.iter().enumerate() {
        let _ = writeln!(
            payloads,
            "export const row{position}: DeclaredMooring = {json};"
        );
    }
    fs::write(at.join("tsconfig.consumer.json"), CONSUMER_PROJECT).unwrap();
    fs::write(at.join("consumer.ts"), &payloads).unwrap();
    let (_, said) = compiled(&at, "tsconfig.consumer.json").unwrap();
    fs::remove_dir_all(&at).unwrap();
    // A row's line is its position after the import.
    let typed: Vec<bool> = (0..MOORINGS.len())
        .map(|row| !said.contains(&format!("consumer.ts({},", row + 2)))
        .collect();
    assert_eq!(
        typed, stated,
        "TypeScript does not take what the rows state:\n{said}\n{payloads}"
    );
}

/// An `ObjectId` in the lone slot of a variant under a tag alone is taken by the Zod schema of the
/// enum under `node` and by its TypeScript type under `tsc --strict` as serde writes and reads it:
/// its `$oid` key beside the tag, and not the tag alone.
#[cfg(feature = "mongodb")]
#[test]
fn type_check_an_object_id_beside_the_tag_is_taken_as_serde_writes_it() {
    let held = DeclaredStamp::Held(ObjectId::parse_str("507f1f77bcf86cd799439011").unwrap());
    assert_eq!(
        serde_json::to_string(&held).unwrap(),
        STAMPS[0].0,
        "serde no longer writes an ObjectId beside the tag as the first row"
    );
    let stated: Vec<bool> = STAMPS.iter().map(|&(_, taken)| taken).collect();
    let read: Vec<bool> = STAMPS
        .iter()
        .map(|&(json, _)| serde_json::from_str::<DeclaredStamp>(json).is_ok())
        .collect();
    assert_eq!(read, stated, "serde does not take what the rows state");
    let listed: Vec<(&str, &str)> = STAMPS
        .iter()
        .map(|&(json, _)| ("DeclaredStamp$Schema", json))
        .collect();
    let driver = TAKEN_DRIVER.replace("PAYLOADS", &serde_json::to_string(&listed).unwrap());
    let Some((printed, failed)) = loaded_from("stamps", &stamps_bundle(), &driver) else {
        return;
    };
    assert_eq!(
        printed,
        serde_json::to_string(&stated).unwrap(),
        "the schema does not take what the rows state:\n{failed}"
    );
    let Some(at) = built_from("stamps-payloads", &stamps_bundle()) else {
        return;
    };
    let mut payloads = String::from("import type { DeclaredStamp } from \"./lib/index.js\";\n");
    for (position, &(json, _)) in STAMPS.iter().enumerate() {
        let _ = writeln!(
            payloads,
            "export const row{position}: DeclaredStamp = {json};"
        );
    }
    fs::write(at.join("tsconfig.consumer.json"), CONSUMER_PROJECT).unwrap();
    fs::write(at.join("consumer.ts"), &payloads).unwrap();
    let (_, said) = compiled(&at, "tsconfig.consumer.json").unwrap();
    fs::remove_dir_all(&at).unwrap();
    // A row's line is its position after the import.
    let typed: Vec<bool> = (0..STAMPS.len())
        .map(|row| !said.contains(&format!("consumer.ts({},", row + 2)))
        .collect();
    assert_eq!(
        typed, stated,
        "TypeScript does not take what the rows state:\n{said}\n{payloads}"
    );
}

/// An externally tagged enum that declares a parameter, written with an argument, flattened and in
/// the lone slot of a variant under a tag alone, is taken by the Zod schema of its holder under
/// `node` and by its TypeScript type under `tsc --strict` as serde writes and reads it: each
/// variant's entry at the argument, a unit variant's over `null` among them, directly, at the
/// holder's own parameter, flattened and beside the tag, and through an alias that declares a
/// parameter of its own.
#[test]
fn type_check_an_enum_written_with_an_argument_takes_each_entry_at_the_argument() {
    let stated: Vec<bool> = LADINGS.iter().map(|&(_, _, _, taken)| taken).collect();
    let read: Vec<bool> = LADINGS
        .iter()
        .map(|&(_, typed, json, _)| match typed {
            "DeclaredBerth" => serde_json::from_str::<DeclaredBerth>(json).is_ok(),
            "DeclaredPier" => serde_json::from_str::<DeclaredPier>(json).is_ok(),
            "DeclaredWharf" => serde_json::from_str::<DeclaredWharf>(json).is_ok(),
            "DeclaredMarina<boolean>" => serde_json::from_str::<DeclaredMarina<bool>>(json).is_ok(),
            _ => serde_json::from_str::<DeclaredQuay<bool>>(json).is_ok(),
        })
        .collect();
    assert_eq!(read, stated, "serde does not take what the rows state");
    let listed: Vec<(&str, &str)> = LADINGS
        .iter()
        .map(|&(schema, _, json, _)| (schema, json))
        .collect();
    let driver = TAKEN_DRIVER.replace("PAYLOADS", &serde_json::to_string(&listed).unwrap());
    let Some((printed, failed)) = loaded_from("ladings", &hatches_bundle(), &driver) else {
        return;
    };
    assert_eq!(
        printed,
        serde_json::to_string(&stated).unwrap(),
        "the schemas do not take what the rows state:\n{failed}"
    );
    let Some(at) = built_from("ladings-payloads", &hatches_bundle()) else {
        return;
    };
    let mut payloads = String::from(
        "import type { DeclaredBerth, DeclaredMarina, DeclaredPier, DeclaredQuay, DeclaredWharf } \
         from \"./lib/index.js\";\n",
    );
    for (position, &(_, typed, json, _)) in LADINGS.iter().enumerate() {
        let _ = writeln!(payloads, "export const row{position}: {typed} = {json};");
    }
    fs::write(at.join("tsconfig.consumer.json"), CONSUMER_PROJECT).unwrap();
    fs::write(at.join("consumer.ts"), &payloads).unwrap();
    let (_, said) = compiled(&at, "tsconfig.consumer.json").unwrap();
    fs::remove_dir_all(&at).unwrap();
    // A row's line is its position after the import.
    let typed: Vec<bool> = (0..LADINGS.len())
        .map(|row| !said.contains(&format!("consumer.ts({},", row + 2)))
        .collect();
    assert_eq!(
        typed, stated,
        "TypeScript does not take what the rows state:\n{said}\n{payloads}"
    );
}

/// An untagged enum that declares a parameter, written with an argument and flattened, is taken by
/// the Zod schema of its holder under `node` and by its TypeScript type under `tsc --strict` as
/// serde writes and reads it: each member at the argument, directly and at the holder's own
/// parameter, and each member of named fields closed to the other's key.
#[test]
fn type_check_an_untagged_enum_written_with_an_argument_takes_each_member_at_the_argument() {
    let stated: Vec<bool> = LANDINGS.iter().map(|&(_, _, _, taken)| taken).collect();
    let read: Vec<bool> = LANDINGS
        .iter()
        .map(|&(_, typed, json, _)| match typed {
            "DeclaredLanding" => serde_json::from_str::<DeclaredLanding>(json).is_ok(),
            "DeclaredSlipway" => serde_json::from_str::<DeclaredSlipway>(json).is_ok(),
            _ => serde_json::from_str::<DeclaredJetty<DeclaredCarton>>(json).is_ok(),
        })
        .collect();
    assert_eq!(read, stated, "serde does not take what the rows state");
    let listed: Vec<(&str, &str)> = LANDINGS
        .iter()
        .map(|&(schema, _, json, _)| (schema, json))
        .collect();
    let driver = TAKEN_DRIVER.replace("PAYLOADS", &serde_json::to_string(&listed).unwrap());
    let Some((printed, failed)) = loaded_from("landings", &hatches_bundle(), &driver) else {
        return;
    };
    assert_eq!(
        printed,
        serde_json::to_string(&stated).unwrap(),
        "the schemas do not take what the rows state:\n{failed}"
    );
    let Some(at) = built_from("landings-payloads", &hatches_bundle()) else {
        return;
    };
    let mut payloads = String::from(
        "import type { DeclaredCarton, DeclaredJetty, DeclaredLanding, DeclaredSlipway } from \
         \"./lib/index.js\";\n",
    );
    for (position, &(_, typed, json, _)) in LANDINGS.iter().enumerate() {
        let _ = writeln!(payloads, "export const row{position}: {typed} = {json};");
    }
    fs::write(at.join("tsconfig.consumer.json"), CONSUMER_PROJECT).unwrap();
    fs::write(at.join("consumer.ts"), &payloads).unwrap();
    let (_, said) = compiled(&at, "tsconfig.consumer.json").unwrap();
    fs::remove_dir_all(&at).unwrap();
    // A row's line is its position after the import.
    let typed: Vec<bool> = (0..LANDINGS.len())
        .map(|row| !said.contains(&format!("consumer.ts({},", row + 2)))
        .collect();
    assert_eq!(
        typed, stated,
        "TypeScript does not take what the rows state:\n{said}\n{payloads}"
    );
}

/// Whether serde writes `json` back as it reads it, as `T`: a payload serde writes.
fn serde_writes<T>(json: &str) -> bool
where
    T: DeserializeOwned + Serialize,
{
    serde_json::from_str::<T>(json).is_ok_and(|read| {
        serde_json::to_value(read).ok() == serde_json::from_str::<serde_json::Value>(json).ok()
    })
}

/// An untagged enum flattened through an alias is taken by the Zod schema of its holder under
/// `node` and by its TypeScript type under `tsc --strict` as the enum named directly is: each
/// member serde writes, and not the keys of two members, which serde reads as the first member
/// and never writes.
#[test]
fn type_check_an_untagged_enum_flattened_through_an_alias_takes_each_member_serde_writes() {
    let stated: Vec<bool> = ALIASED_PICKS
        .iter()
        .map(|&(_, _, _, taken)| taken)
        .collect();
    let written: Vec<bool> = ALIASED_PICKS
        .iter()
        .map(|&(_, typed, json, _)| match typed {
            "DeclaredDinghy" => serde_writes::<DeclaredDinghy>(json),
            _ => serde_writes::<DeclaredGangway>(json),
        })
        .collect();
    assert_eq!(written, stated, "serde does not write what the rows state");
    let listed: Vec<(&str, &str)> = ALIASED_PICKS
        .iter()
        .map(|&(schema, _, json, _)| (schema, json))
        .collect();
    let driver = TAKEN_DRIVER.replace("PAYLOADS", &serde_json::to_string(&listed).unwrap());
    let Some((printed, failed)) = loaded_from("aliased-picks", &hatches_bundle(), &driver) else {
        return;
    };
    assert_eq!(
        printed,
        serde_json::to_string(&stated).unwrap(),
        "the schemas do not take what the rows state:\n{failed}"
    );
    let Some(at) = built_from("aliased-picks-payloads", &hatches_bundle()) else {
        return;
    };
    let mut payloads =
        String::from("import type { DeclaredDinghy, DeclaredGangway } from \"./lib/index.js\";\n");
    for (position, &(_, typed, json, _)) in ALIASED_PICKS.iter().enumerate() {
        let _ = writeln!(payloads, "export const row{position}: {typed} = {json};");
    }
    fs::write(at.join("tsconfig.consumer.json"), CONSUMER_PROJECT).unwrap();
    fs::write(at.join("consumer.ts"), &payloads).unwrap();
    let (_, said) = compiled(&at, "tsconfig.consumer.json").unwrap();
    fs::remove_dir_all(&at).unwrap();
    // A row's line is its position after the import.
    let typed: Vec<bool> = (0..ALIASED_PICKS.len())
        .map(|row| !said.contains(&format!("consumer.ts({},", row + 2)))
        .collect();
    assert_eq!(
        typed, stated,
        "TypeScript does not take what the rows state:\n{said}\n{payloads}"
    );
}

/// The models whose names hold a `$`, a quote, a double quote and a backslash compile under
/// `tsc --strict`, and the Zod schema of each reads every payload serde writes as serde writes it:
/// a struct's members, its own tag key and its name, an enum's variants under each tagging, its tag
/// key and its content key, a plain enum's values, and a struct and an externally tagged enum
/// flattened beside a member of their holder's own. A payload under an alias is read as serde
/// writes it back.
#[test]
fn type_check_a_name_that_holds_a_quote_is_read_under_zod_as_serde_writes_it() {
    let mut listed: Vec<(&str, String)> = quoted::WRITTEN
        .iter()
        .map(|&(read_as, json)| (read_as, quoted::serde_written(read_as, json)))
        .collect();
    for (read_as, json, back) in quoted::UNDER_AN_ALIAS {
        assert_eq!(
            quoted::serde_written(read_as, json),
            back,
            "serde no longer reads {json} as {back}"
        );
        listed.push((read_as, back.to_owned()));
    }
    let expected: Vec<serde_json::Value> = listed
        .iter()
        .map(|(_, written)| serde_json::from_str(written).unwrap())
        .collect();
    let driver = QUOTED_DRIVER.replace("PAYLOADS", &serde_json::to_string(&listed).unwrap());
    let Some((printed, failed)) = loaded_from("quoted", &quoted_bundle(), &driver) else {
        return;
    };
    let read: Vec<serde_json::Value> = printed
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(
        read, expected,
        "Zod does not read what serde writes:\n{failed}"
    );
}

/// A literal is a rule under Zod as it is in Rust: each payload of the table is read and refused
/// as serde reads and refuses it, each issue in the sentence `validate()` reports.
#[test]
fn type_check_a_literal_refuses_under_zod_what_serde_refuses() {
    let source = [
        PREAMBLE.to_owned(),
        literals::AdjacentAssay::ts_definition(),
        literals::AdjacentAssay::zod_schema(),
        literals::Assay::ts_definition(),
        literals::Assay::zod_schema(),
        literals::Bullion::ts_definition(),
        literals::Bullion::zod_schema(),
        literals::Hallmark::ts_definition(),
        literals::Hallmark::zod_schema(),
        literals::Ingot::ts_definition(),
        literals::Ingot::zod_schema(),
        literals::TaggedAssay::ts_definition(),
        literals::TaggedAssay::zod_schema(),
        literals::Touchmark::ts_definition(),
        literals::Touchmark::zod_schema(),
    ]
    .join("\n\n");
    let listed: Vec<(&str, &str, &str, bool)> = literals::LITERAL_READS
        .iter()
        .map(|row| {
            assert_eq!(
                literals::serde_read(row.held_by, row.payload).is_ok(),
                matches!(row.answered, literals::Answered::Read),
                "serde no longer reads {} as the table says",
                row.name
            );
            (
                row.name,
                row.held_by,
                row.payload,
                literals::names_each_violation(row),
            )
        })
        .collect();
    let driver = LITERALS_DRIVER.replace("PAYLOADS", &serde_json::to_string(&listed).unwrap());
    let Some((printed, failed)) = loaded_from("literals", &source, &driver) else {
        return;
    };
    assert_eq!(
        printed,
        literals::literal_report(literals::names_each_violation).trim_end(),
        "the Zod schemas do not refuse what a serde read refuses:\n{failed}"
    );
}

/// An alias of an `Option` held outside a field is read under Zod as serde writes it, `null` as
/// `null`, and each payload serde writes is a value of its TypeScript type under `tsc --strict`.
#[test]
fn type_check_an_alias_of_an_option_held_outside_a_field_is_read_with_null() {
    let source = [
        PREAMBLE.to_owned(),
        option_aliases::plummet_schema::Schema::ts_definition(),
        option_aliases::plummet_schema::Schema::zod_schema(),
        option_aliases::relead_schema::Schema::ts_definition(),
        option_aliases::relead_schema::Schema::zod_schema(),
        option_aliases::haply_schema::Schema::ts_definition(),
        option_aliases::haply_schema::Schema::zod_schema(),
        option_aliases::Swung::<u32>::ts_definition(),
        option_aliases::Swung::<u32>::zod_schema(),
        option_aliases::Leadsman::ts_definition(),
        option_aliases::Leadsman::zod_schema(),
        option_aliases::Chanced::ts_definition(),
        option_aliases::Chanced::zod_schema(),
        option_aliases::Handlead::ts_definition(),
        option_aliases::Handlead::zod_schema(),
        option_aliases::Reckoned::ts_definition(),
        option_aliases::Reckoned::zod_schema(),
        option_aliases::Deepsea::ts_definition(),
        option_aliases::Deepsea::zod_schema(),
        option_aliases::Sinker::ts_definition(),
        option_aliases::Sinker::zod_schema(),
        option_aliases::Heaved::ts_definition(),
        option_aliases::Heaved::zod_schema(),
    ]
    .join("\n\n");
    for (read_as, json) in option_aliases::LEADS {
        assert_eq!(
            option_aliases::serde_wrote(read_as, json).as_deref(),
            Some(json),
            "serde does not write back {json} read as {read_as}"
        );
    }
    let driver = VALUES_DRIVER.replace(
        "PAYLOADS",
        &serde_json::to_string(&option_aliases::LEADS).unwrap(),
    );
    let Some((printed, failed)) = loaded_from("option-aliases", &source, &driver) else {
        return;
    };
    let read: Vec<Option<serde_json::Value>> = printed
        .lines()
        .map(|line| serde_json::from_str(line).ok())
        .collect();
    let written: Vec<Option<serde_json::Value>> = option_aliases::LEADS
        .iter()
        .map(|&(_, json)| serde_json::from_str(json).ok())
        .collect();
    assert_eq!(
        read, written,
        "Zod does not read what serde writes:\n{printed}\n{failed}"
    );
    let Some(at) = built_from("option-aliases-payloads", &source) else {
        return;
    };
    let mut payloads = String::from(
        "import type { Chanced, Deepsea, Handlead, Heaved, Leadsman, Reckoned, Sinker } from \
         \"./lib/index.js\";\n",
    );
    for (position, &(read_as, json)) in option_aliases::LEADS.iter().enumerate() {
        let _ = writeln!(payloads, "export const row{position}: {read_as} = {json};");
    }
    fs::write(at.join("tsconfig.consumer.json"), CONSUMER_PROJECT).unwrap();
    fs::write(at.join("consumer.ts"), &payloads).unwrap();
    let (typed, said) = compiled(&at, "tsconfig.consumer.json").unwrap();
    fs::remove_dir_all(&at).unwrap();
    assert!(
        typed,
        "TypeScript refuses a payload serde writes:\n{said}\n{payloads}"
    );
}

/// Each payload serde reads as an untagged enum with a newtype variant over an `Option` is a value
/// of the enum's TypeScript type under `tsc --strict`, `null` among them, and the Zod schema reads
/// it as the value serde writes back.
#[test]
fn type_check_an_untagged_variant_over_an_option_holds_null() {
    let source = [
        PREAMBLE.to_owned(),
        null_models::Fathom::ts_definition(),
        null_models::Fathom::zod_schema(),
        null_models::Kedged::ts_definition(),
        null_models::Kedged::zod_schema(),
        null_models::Drogue::ts_definition(),
        null_models::Drogue::zod_schema(),
        null_models::Drogued::ts_definition(),
        null_models::Drogued::zod_schema(),
        null_models::sounded_schema::Schema::ts_definition(),
        null_models::sounded_schema::Schema::zod_schema(),
        null_models::Plumb::ts_definition(),
        null_models::Plumb::zod_schema(),
    ]
    .join("\n\n");
    let read: Vec<(&str, &str)> = null_models::NULL_ROWS
        .iter()
        .filter(|row| {
            assert_eq!(
                null_models::serde_told(row.read_as, row.json),
                row.told(str::to_owned),
                "serde answers {} read as {} otherwise",
                row.json,
                row.read_as
            );
            row.taken.is_some()
        })
        .map(|row| (row.read_as, row.json))
        .collect();
    let driver = VALUES_DRIVER.replace("PAYLOADS", &serde_json::to_string(&read).unwrap());
    let Some((printed, failed)) = loaded_from("untagged-nulls", &source, &driver) else {
        return;
    };
    let values: Vec<Option<serde_json::Value>> = printed
        .lines()
        .map(|line| serde_json::from_str(line).ok())
        .collect();
    let written: Vec<Option<serde_json::Value>> = read
        .iter()
        .map(|&(_, json)| serde_json::from_str(json).ok())
        .collect();
    assert_eq!(
        values, written,
        "Zod does not read what serde writes back:\n{printed}\n{failed}"
    );
    let Some(at) = built_from("untagged-nulls-payloads", &source) else {
        return;
    };
    let mut payloads =
        String::from("import type { Drogued, Fathom, Kedged, Plumb } from \"./lib/index.js\";\n");
    for (position, &(read_as, json)) in read.iter().enumerate() {
        let _ = writeln!(payloads, "export const row{position}: {read_as} = {json};");
    }
    fs::write(at.join("tsconfig.consumer.json"), CONSUMER_PROJECT).unwrap();
    fs::write(at.join("consumer.ts"), &payloads).unwrap();
    let (typed, said) = compiled(&at, "tsconfig.consumer.json").unwrap();
    fs::remove_dir_all(&at).unwrap();
    assert!(
        typed,
        "TypeScript refuses a payload serde reads:\n{said}\n{payloads}"
    );
}
