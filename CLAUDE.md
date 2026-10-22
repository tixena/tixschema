# CLAUDE.md

**Note**: This project uses [bd (beads)](https://github.com/steveyegge/beads) for issue tracking. Use `bd` commands instead of markdown TODOs. See [AGENTS.md](AGENTS.md) for workflow details.

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**tixschema** is a Rust procedural macro library that generates TypeScript type definitions and Zod v4 validation schemas from Rust structs and enums. This ensures type safety and consistency between Rust backends and TypeScript frontends in Tixena applications.

## Task Management System

**MANDATORY: Use `bd` (beads) for ALL task tracking.**

- ✅ **DO**: Use `bd` commands for issue/task tracking (`bd create`, `bd ready`, `bd update`, `bd close`)
- ❌ **DO NOT**: Use TodoWrite tool, markdown TODO lists, or any other task tracking system
- 📖 **Reference**: See [AGENTS.md](AGENTS.md) for complete bd workflow and commands

**Quick bd commands:**
```bash
bd ready --json           # Check for available work
bd create "Task" -t task -p 2 --json
bd update <id> --status in_progress --json
bd close <id> --reason "Completed" --json
```

## Essential Commands

### Testing
```bash
# Quick test with default features (recommended for rapid iteration)
just quick
# or
cargo test

# Comprehensive test - every combination of the plain features (run before a release)
just test

# Test specific feature combinations
just test-minimal          # No features
just test-default          # Default features only
just test-named-features   # Key combinations

# Run a specific test by name
just test-name <TEST_NAME>
# or
cargo test <TEST_NAME>

# Run the emitted clients under their own toolchains; refuses to stand down
just test-emitted

# Lint, then run, the tests that wait on a real clock, through test-emitted
just test-slow

# Install the Kotlin jars test-emitted compiles against, into ~/.local/share/tixschema/kotlin-libs
just kotlin-libs

# Run the MongoDB operations against the server TIXSCHEMA_MONGODB_URI names; refuses to stand down
just test-mongodb
```

### Code Quality
```bash
# Check code (runs cargo check + clippy with warnings as errors)
just check

# Format code
just fmt

# Check all feature combinations without running tests
just check-all

# Audit the dependencies this package and bson3/ resolve, each from a lock file generated for it (needs cargo-audit)
just audit
```

### Build & Documentation
```bash
# Build the project
cargo build

# Build and open documentation
just docs-open

# Build docs only
just docs
```

### The Gate (what a change is held to, here and in CI)
```bash
just all
# Runs, with every feature on: one clippy lint, one build, one test run
```

### The Full Run (every few days)
```bash
just all-named     # both packages: default and all-features lints, the format check, the named feature lists
just lint-sets     # clippy over every combination of the feature sets
just test-sets     # tests over every combination of the feature sets
just test          # tests over every combination of the plain features
just test-slow     # the tests that wait on a real clock, with the emitted clients
```

No test the gate runs waits on a real clock: no sleep, no real timer, no wall-clock window. A
test that must compiles only under `cfg(slow_tests)`, which `unexpected_cfgs` declares in both
manifests and only `just test-slow` sets (`--config 'build.rustflags=["--cfg", "slow_tests"]'`);
a compiler cfg, so `--all-features` never turns it on. Every such test is a WebSocket run of the
emitted clients (heartbeats, fixed waits, and polls with a deadline), and the helpers and
imports only they use carry the same cfg. A wait for an event the script itself observes resolves
on that event and holds no deadline, as the attachment runs of `run_node_ws_headers.rs` and
`run_dart_ws_headers.rs` do: a script whose event never comes ends with no output, and the test
fails.

## Code Architecture

### Macro Processing Flow

1. **Entry Point** ([lib.rs](src/lib.rs))
   - Exports two proc macros: `#[model_schema]` and `#[model_schema_prop]`
   - `model_schema` is the main macro for structs/enums/type aliases
   - `model_schema_prop` is for field-level customization
   - Exports the two initializers a crate calls once at its root, `decode_with_foundation!()` and `mongodb_foundation!()`, which [foundation.rs](src/foundation.rs) expands: the support libraries of the recovering decode ("Recovering Decode", below)

2. **Macro Execution** ([model_schema.rs](src/model_schema.rs))
   - `exec_model_schema()` routes to `process_struct()`, `process_enum()`, `process_type_alias()`, or `process_branded_newtype()`
   - Parses Serde attributes when `serde` feature enabled
   - Extracts example code from doc comments (every ` ```rust example` fence)
   - Generates methods: `ts_definition()`, optionally `json_schema()`, optionally `schema_examples()` with `schema_example()`, and optionally `validate()`
   - `process_branded_newtype()`: handles `#[serde(transparent)]` structs serde writes as the value of one field -- a single-slot tuple struct, or a struct with named fields -- generating Zod brand schemas or `unique symbol` branded TypeScript types

3. **Type Analysis** ([field_type.rs](src/field_type.rs))
   - `FieldDef`: Core data structure representing a field's type, optionality, docs, etc.
   - `FieldDefType`: Enum categorizing Rust types (primitives, collections, custom types, ObjectId, etc.)
   - `get_field_def()`: Recursively analyzes Rust types and builds FieldDef trees
   - Handles: Option<T>, Vec<T>, HashMap<String, T>, nested types, generics

4. **Feature Modules** ([features/](src/features/))
   - `serde.rs`: Parses Serde attributes (`rename`, `rename_all`, `tag`, etc.), and says what serde writes on the way to a member (`member_steps`, `variant_steps`, over `Tagging`)
   - `zod.rs`: Generates Zod v4 schema strings (`z.string()`, `z.union()`, etc.), embeds examples in `.meta()`
   - `jsonschema.rs`: Generates JSON schema objects
   - `object_id.rs`: MongoDB ObjectId type detection and schema generation
   - `recovering_decode.rs`: Generates what `#[model_schema(enable = ["decode_with"])]` adds to a type: `from_value_with` and `from_value_piped`, `from_bson_with` and `from_bson_piped` under `bson`, and the walker of each. It also writes the two support libraries the initializers put at a crate root, `__tixschema_decode_with` (the callback's and the resolvers' types, what one walker hands another, and under `bson` the BSON pieces) and `__tixschema_mongodb` (under `mongodb`, the query types and what the operations answer), and the re-export of both, name by name, in each `{type}_schema`. `recovering_decode/enums.rs` holds the walkers of an enum, and the names its typed paths read a variant by (`written_variants`, `tag_arms`). Under `mongodb` three more files emit, for a type that lists `"mongodb"`: `recovering_decode/query.rs` the query types into the MongoDB library (`Filter`, `ArmFilter`, `Update`, `ArmUpdate`, the path kinds, `nest`), `recovering_decode/fields.rs` the type's own typed paths into its module and onto the type (`MongoFields`, `MONGO_FIELDS`, one function per field, and one per arm of an enum under a tag), and `recovering_decode/operations.rs` the operations onto the type (`find_one`, `find`, `count`, `insert_one`, the updates and the deletes) and `Read` and `OperationError` into the library
   - `model_schema_prop.rs`: Parses field-level customization attributes (`pattern`, `minLength`, `maxLength`, `minimum`, `maximum`, `literal`, `as`, `preprocess`, `check`)

5. **Code Generation** ([generation/](src/generation/))
   - `typescript.rs`: Generates TypeScript type definitions and Zod schemas
   - Combines FieldDef trees into complete type/schema strings
   - Handles discriminated unions, generics, nested types
   - Every `zod_schema()` writes `examples` into the schema's `.meta()`: `[]` as text where the type writes no block, `schema_examples()` serialized where it writes one or more

6. **Example Extraction** ([utils.rs](src/utils.rs))
   - `extract_examples_tokens()`: Parses every ` ```rust example` code fence from doc comments
   - Returns each block's tokens in the order written, an empty block skipped
   - Each block's code is inserted, in a scope of its own, into the generated `schema_examples()` method
   - Compiler validates example code at compile time

7. **Service Schema Accessors** ([features/service_schema.rs](src/features/service_schema.rs))
   - `ts_http_service()`: the TypeScript route table and request dispatcher for `http_rest`, mirroring the Rust `dispatch`/`ROUTES` pair
   - `ts_ws_server()`: the TypeScript connection-accepting WebSocket server for Node, wrapping the existing single-socket dispatcher attachment
   - `dart_definition()`: every Dart type a service publishes -- its messages, fault type, and one sealed `{Service}{Operation}Result` pair per reply operation
   - `swift_http_client()`/`swift_ws_client()`: the Swift `http_rest` and `ws_rpc` clients -- one method per operation answering `Result<Success, Failure>`, a one-way method throwing the shared `{Service}Refusal`
   - `kotlin_http_client()`/`kotlin_ws_client()`: the Kotlin `http_rest` and `ws_rpc` clients -- one method per operation answering the sealed `{Service}{Operation}Result`; the WebSocket one also carries the mini server, `attach{Service}WsDispatcher(frames, handlers, onFault)`, and its `share`
   - The module itself is gated on `serde` alone. `typescript` gates only the `ts_*` accessors (and `zod` narrows further, to the client and dispatcher seam); `dart`, `swift` and `kotlin` each gate their own accessors independently, under `serde` plus that language's own feature -- none of the four needs any of the others on

### Key Data Structures

**FieldDef** (central type representation):
```rust
pub struct FieldDef {
    pub name: String,                    // Field name (respects Serde rename)
    pub docs: String,                    // Rust doc comments → JSDoc
    pub field_type: FieldDefType,        // The actual type category
    pub array_depth: u8,                 // Vec<T> → Array<T>, one level per Vec/slice/set written
    pub nullable_levels: Vec<u8>,        // Which array levels were written as Option
    pub array_lengths: Vec<(u8, usize)>, // Length of each level written as [T; N]
    pub model_schema_prop_meta: ...,     // Field-level overrides
}
```

Levels count from the innermost value outward, so `Vec<Option<T>>` records level 0 — a `null`
among the array's items, rendered `Array<T | null>` — while `Option<Vec<T>>` records level 1, a
`null` in place of the whole array, rendered `Array<T> | undefined` in a field and
`Array<T> | null` in a slot. `is_optional()` asks the question of the outermost level, the only
one whose rendering depends on where the field sits.

`array_lengths` numbers levels the same way: `[[T; 2]; 3]` records `(0, 2)` and `(1, 3)`. Only the
validating surfaces spend it — `minItems`/`maxItems` in the JSON schema, `.length(N)` in Zod —
while TypeScript stays `Array<T>`. A length the expansion cannot read (const generic, `const`
item, computed expression) records nothing and describes as an unbounded array.

**FieldDefType** (type categories):
- Primitives: `Boolean`, `String`, `U8-U64`, `I8-I64`, `F32`, `F64`, `Usize`, `Isize`
- Complex: `SiblingType` (custom types), `Map` (HashMap), `Tuple`, `Range` (a std range, its `RangeKind` and its bound), `Result` (a std `Result`, its `Ok` and its `Err`)
- Special: `ObjectId` (MongoDB, feature-gated), `StringLiteral`, `TypeParam`, `Unknown`

`Unknown` is a type no surface can name: one written as no path (a function pointer, a trait
object, a macro in type position), a path with parenthesized arguments, and, in Dart alone, an
opaque type parameter (`opaque_parameters`). Every surface describes it as any value. A
`serde_json::Value` is never one: it is refused before any field is read (section 4).

`TypeParam` is one of the enclosing item's own type parameters, which `erase_type_parameters`
rewrites a written name into. The three surfaces answer for it differently: TypeScript renders the
name it was written with, JSON Schema describes it as `{}`, and Zod composes the argument the
enclosing factory binds for it (`idType` for `IdType`). A surface with no factory to bind an
argument — an alias, a branded newtype — calls `with_opaque_type_parameters` first and writes
`z.unknown()`.

### Feature Flag System

The crate uses optional features for minimal dependencies:

- `serde`: Enables Serde attribute parsing and field renaming
- `zod`: Enables Zod schema generation (v4 syntax)
- `jsonschema`: Enables `json_schema()` method generation
- `mongodb`: Enables MongoDB ObjectId type support; turns on `bson`. On a type that lists `"mongodb"` in `enable = ["decode_with", "mongodb"]` it also generates the typed field paths (`MONGO_FIELDS`, one function per field, `nest` below a nested type, and `<tag>__<value>()` for each arm of an enum under a tag), the filters and updates they build, and the operations over a `mongodb::Collection<bson::Document>` of the type's rows. A consuming crate that declares such a type lists the `mongodb` driver, built for the major version of `bson` it lists, and writes `tixschema::mongodb_foundation!();` at its root beside `tixschema::decode_with_foundation!();`
- `bson`: BSON support, turned on by `mongodb`; turns on `serde`. Generates `from_bson_with` and `from_bson_piped` on every type that lists `"decode_with"`, whether it lists `"mongodb"` or not, and the BSON pieces of the `decode_with` library. A crate that declares a flagged type names `bson`, and lists it, in every build with this feature on; it needs no `mongodb` unless a type lists `"mongodb"`
- `typescript`: Enables TypeScript type generation
- `chrono`: Enables chrono date/time type support (`NaiveDate`, `NaiveTime`, `NaiveDateTime`, `DateTime<Tz>`)
- `indexmap`: Enables indexmap type support: `IndexMap` as a map, `IndexSet` as a list. It adds no dependency: the generated code names the types of the crate that declares them. With the feature off a type written `IndexMap<_, _>` or `IndexSet<_>` is refused at `exec_model_schema` with the feature named (`indexmap_feature_errors`), unless the crate declared a type of its own under the name above
- `dart`: Enables Dart type generation with a JSON codec whose read calls each `preprocess` and refuses what `validate()` refuses, a `validate()` on every type, and the Dart HTTP client
- `swift`: Enables Swift type generation with a `Codable` codec, and `validate()` on every emitted type: the bounds and the checks, run as the Rust `validate()` runs them. `init(from:)` of each type calls it on the value it read and refuses what it refuses. The read of a type calls the functions a `preprocess` names, on the type and on a member, ahead of the value it builds
- `archie`: turns on `serde`. On a type that lists `"archie"` in its `enable` list it writes `TryFrom<&T> for archie_core::parser::ast::Value`, through `archie_core::value::to_value`, and `TryFrom<&archie_core::parser::ast::Value> for T`, through `archie_core::value::from_value`, both with `archie_core::value::ValueError`; a type with parameters bounds each impl on the type itself (`where T<..>: Serialize`, `where T<..>: DeserializeOwned`). `src/features/archie.rs` writes them, and `exec_model_schema` appends them after every shape. The entry is read in every build and adds nothing without the feature; `archie_guard_errors` refuses it on a type alias in every build. The recovering decode reads archie's `Value` through `from_value_with`, after `Into<serde_json::Value>`: the entry adds no source of its own. A consuming crate lists `archie-core` under the name `archie_core`; tixschema's tests take it as a dev-dependency (`tests/archie_tests.rs`)
- `kotlin`: Enables Kotlin type generation with `kotlinx.serialization` annotations, a `validate()` on every type that runs the bounds and calls the checks, and a serializer of each type that throws what `validate()` answers of the value it read. A consuming Kotlin build declares two dependencies: the runtime library `org.jetbrains.kotlinx:kotlinx-serialization-json` and the Kotlin Gradle plugin `kotlin("plugin.serialization")`

**Feature sets**: `web` (the default: `serde`, `zod`, `jsonschema`, `typescript`), `mobile` (`serde`, `dart`, `swift`, `kotlin`) and `mongo` (`mongodb`, `chrono`, `indexmap`, `archie`). CI tests the powerset of the sets (`just test-sets`); `just test` runs the powerset of the plain features locally

**Default configuration**: `serde`, `zod`, `jsonschema`, `typescript` (the `mongodb`, `bson`, `chrono`, `indexmap`, `dart`, `swift` and `kotlin` features are opt-in)

## Critical Development Rules

### 1. The Name a Type Publishes Under

A type publishes under the Rust ident it is declared with, spelled exactly as written. Nothing is read off that spelling — no suffix is taken off it and no part of it is rewritten.

`#[model_schema(name = "...")]` is the only publishing override, and it moves the name on every surface at once: the `export type` line, the Zod consts, the `$defs` key a self-referential document hoists under, and every reference another type writes.

```rust
#[model_schema()]
pub struct User { ... }                             // → TypeScript: User

#[model_schema()]
pub struct UserData { ... }                         // → TypeScript: UserData

#[model_schema(name = "User")]
pub struct UserData { ... }                         // → TypeScript: User
```

A renamed item also publishes its Rust ident as an alias of the moved name — `export type UserData = User;`, `export const UserData$Schema = User$Schema;` — which is what lets a type declared *above* it, with only the ident to name it by, still resolve. `ident_schema_module_name` names the generated Rust module from the ident for the same reason: an override never moves it.

`compute_item_export_name` is the one seam a declared item's published name is read from, and `compute_alias_export_name` the alias counterpart — an alias has no surface name of its own and takes the `Type` suffix without an override.

Two declarations cannot publish under one name: the emitted types, schemas and definitions are one flat namespace, so `published_name_collision_errors` refuses the second at `exec_model_schema` — the ungated seam, so the verdict is the same in every feature combination — naming both declarations. `claim_published_name` holds the claims, keyed the opposite way to the `ALIAS_INFO` registry so an ident reclaiming its own name is never a collision.

### 2. Required Derives and Imports

```rust
use tixschema::model_schema;
use serde::{Deserialize, Serialize};

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MyType { ... }
```

### 3. HashMap Key Restriction

A map key must be one serde can write as a JSON object key. `String` is the open case: any string
is a key. `bool`, the numeric types, `char`, and the chrono date/time types are accepted too —
serde stringifies each one (`7` → `"7"`, `true` → `"true"`) — so the map describes as an open
object. A plain `#[model_schema()]` enum key narrows the object to its members. A key serde does
not stringify — a struct, tuple, `Vec`/array, `Option`, nested map, or `ObjectId` — is refused at
expansion:

```rust
// ✅ Supported — string, and any key serde stringifies (bool/numeric/char/chrono)
pub struct Config {
    pub settings: HashMap<String, String>,
    pub counts: HashMap<u32, String>,
}

// ❌ NOT supported - refused at expansion
#[model_schema()]
pub struct KeyStruct {
    pub value: String,
}

#[model_schema()]
pub struct BadMapKey {
    pub m: HashMap<KeyStruct, u32>,
}
// error: field `m`: a map key must be a plain `#[model_schema()]` enum, whose members
// become the object's keys — `KeyStruct` resolves to a type with no `enum_members()`
```

`is_map_wrapper` in `src/field_type.rs` is the one seam that says which names are a map: `HashMap`,
`BTreeMap`, and `IndexMap` under the `indexmap` feature. The field definition, the constraint walk,
the walk `validate()` makes to what a member holds, the recovering decode's walker and its
rewriting of an alias, and the MongoDB paths all ask it, as they ask `is_sequence_wrapper` for a
list, which takes `IndexSet` under the same feature. `IndexMap` and `IndexSet` are names of another
crate's, so the two functions ask `is_declared_ident` for both: a type the crate declared under one
of them above keeps the name, with the feature on or off. `serde_json::Map` is no map: a member
that holds one is refused (section 4).
`is_declared_ident` reads the published names, the one registry every build writes. A key is held
to the rules of this section whichever of those maps it is written in.

On the two nominal surfaces a key renders in the form serde **writes** it wherever the key's own
value form is not a TypeScript property key, and under the name it was written with wherever it is.
`typescript_map_key_typename` and `zod_map_record_call` are the one pair of seams that decide, both
reading `FieldDef::map_key_wire`, so the surfaces cannot drift:

| key reached | TypeScript | Zod |
|-------------|------------|-----|
| `String`, `char`, `PathBuf`, type parameter | own type (`string`, …) | own schema |
| plain enum | own type (`MetricSlot`) | `z.partialRecord(MetricSlot$Schema, V)` |
| the integer types | `number` | `z.record(z.number().int(), V)` |
| `NaiveDate` / `NaiveTime` / `NaiveDateTime` | `string` | own schema |
| `bool` | `"true" \| "false"` | `z.partialRecord(z.enum(["true", "false"]), V)` |
| `DateTime<Tz>` | `string` | `z.record(z.iso.datetime({ offset: true }), V)` |

The last two rows are the ones whose value form spells no property key: `boolean` and `Date` are
rejected by `Record`'s own constraint, `z.boolean()` rejects the string serde wrote, and
the date schema accepts it only to rewrite every key into a locale-dependent rendering. A brand or
alias over one of those loses its name at a key position for the same reason — the only binding it
publishes is that value-shaped schema — while a brand or alias over any other row keeps its name
exactly as before. The constructor moves with the key because `z.record` over an enumerated key
demands every member: a key whose published Zod binding enumerates — a plain enum, and any brand or
alias over one, `.brand()` being type-level only — is written with `z.partialRecord`, which admits
the subset a `HashMap` holds while still refusing a key outside the enumeration. JSON Schema is
unaffected at every position: a stringified key describes as `{"type": "object",
"additionalProperties": true}` whatever its wire form.

### 4. Type Mappings (Rust → TypeScript)

- `String` → `string`
- `bool` → `boolean`
- Numeric types → `number`
- `Option<T>` → `T | undefined` (Zod: `z.union([z.null().transform(() => undefined), T, z.undefined()]).prefault(undefined)`, accepting an explicit `null` and coercing it to `undefined`; JSON Schema `anyOf: [T, {"type": "null"}]`, key left out of `required`); with `#[model_schema_prop(ts_optional)]` the TypeScript key becomes optional instead: `field?: T`; with `#[model_schema_prop(nullable)]` all three surfaces render `T | null` with the key **required** instead: Zod `z.union([T, z.null()])`, JSON Schema `anyOf: [T, {"type": "null"}]` with the key in `required`
- `Vec<T>` → `Array<T>`
- `HashSet<T>`, `BTreeSet<T>` and `IndexSet<T>` (under `indexmap`) → the array a `Vec<T>` writes, its items distinct. `is_set_wrapper` in `src/field_type.rs`, beside `is_sequence_wrapper`, is the one list of sets, and `collection_element_field` records each array level written as one in `FieldDef::set_levels`, numbered as `array_lengths` numbers its levels, so the fact survives every merge into array levels. The JSON Schema writes `"uniqueItems": true` at that level (`arrayed_json_schema_value`, `arrayed_json_schema_fragment`), and Zod `zod_repeat_check` after the level's count, which compares the items it parsed by their `JSON.stringify` and pushes its issue with `continue: true`. The `validate()` of each client answers a repeat at every set level, a member's own set ahead of its count: `own_repeat_elements` and `listed_at` in Dart, comparing the JSON its codec writes for each item; `own_repeat_rules` and `held_walk` under `Walk::Reported` in Swift, comparing the `JSONEncoder` texts with sorted keys; `own_repeat_statements` from `Rules::member` and `held_statements` in Kotlin, comparing the values. A brand over a set answers it too, and no bound a variant of an untagged enum is chosen by holds it. Swift's `reaches` asks `Walk::Rules`, which writes no repeat, so the read of a set is what it was; a tuple that holds a set is read unjudged, its `validate()` answering the repeat. The sentence is `REPEATED_ITEM` in `src/bound_message.rs`, written by `zod_repeat_check`, `dart_repeated`, `swift_repeated` and `kotlin_repeated`. Rust's read keeps one item of a repeat, and its `validate()` answers nothing of one; TypeScript writes `Array<T>`. `tests/set_tests/models.rs` holds the models and `PRINTED`, the table Zod and each client print, and `assert_rust_keeps_one_item`, what Rust's read answers of the same payloads
- `HashMap<String, T>` → `Partial<Record<string, T>>`
- Custom types → Reference by the name the referenced type publishes (its ident, or its `name` override)
- A unit struct (`pub struct Ping;`, no braces) → `Record<string, never>` (unchanged); the wire is `{}` both ways: the macro replaces the author's own `Serialize`/`Deserialize` derive with impls writing and reading `{}` (serde's own derive writes and reads `null`, refusing `{}`). Kotlin renders a serializable `object Ping` rather than a `class`; Dart and Swift render their own no-data type. Declared above a service as `Result<Ping, E>`, it reads like a `()` success on every client — a braced `struct Ping {}` is a different shape and is unaffected
- `ObjectId` → `ObjectId` (with $oid validation)
- `DateTime<Tz>` → `Date` (Zod `z.union([z.date(), z.string(), z.number()]).pipe(z.coerce.date())`, which refuses `null` and booleans that `z.coerce.date()` alone reads as 1970, ahead of the `null` arm and the boolean member a union lists after the date; `type_check_a_date_refuses_what_is_no_date_in_every_position_that_holds_one` in `tests/generic_types_tests/declarations.rs` loads the schema under `node` and holds each input to it); with `#[model_schema_prop(as_number)]` → `number` (inline epoch-ms coercer)
- `NaiveTime` → `string` (Zod `z.iso.time()` wrapped in an inline preprocessor that also accepts millis-since-start-of-day)
- `Range<T>` and `RangeInclusive<T>` → `{ start: T; end: T }`, `RangeFrom<T>` → `{ start: T }`, `RangeTo<T>` → `{ end: T }` (Zod `z.strictObject({ … })`, JSON Schema an object of those members, each `required`, with `additionalProperties: false`), each bound rendered as its type renders anywhere else. `written_std_range` in `src/field_type.rs` is the one seam that says a path is one: one of the four last names with one type argument, written in full under `core::ops` or `std::ops` (`names_std_ops`), or under any other path but a name `is_declared_ident` holds, a `#[model_schema()]` type of the crate's own declared above. A range written by any path but the full one earns the check `src/model_schema/ranges.rs` writes beside the holder in every build, which hands the std range back as the type the path names and fails with E0308 where that is another type. Each client writes a type of its own per range: a Dart record with named members, a Swift struct `{Holder}{Field}Range` (`{Holder}{Field}Argument{N}Range` in a type argument) that declares the parameters its bound reaches, and a Kotlin `@Serializable` data class over its bound's type, one per kind of range an item writes (`KOTLIN_RANGES`). Each refuses an object that holds a key the range does not write, as serde's own read of a range does: the Dart record's `switch` takes it only where every key is a member, and the Swift struct reads its keys through `SwiftSchemaAnyKey` first and throws `typeMismatch`, as Swift's own read of an enum does. `validate()` walks a range's bounds under their keys (`ConstraintWrap::Range`), the judged read reads a model in one unjudged (`range_built_back`), the recovering decode reads one whole (`Expected::Range`), and under `mongodb` a range field is a `Field`. A range as a map key and a bound key on a range field are refused in every build, and a range under `#[serde(flatten)]` in every build with `serde`
- `Result<T, E>` → `{ Ok: T } | { Err: E }` (Zod `z.union([z.strictObject({ Ok: T }), z.strictObject({ Err: E })])`, JSON Schema a `oneOf` of the two objects, each with its one key `required` and `additionalProperties: false`), each arm rendered as its type renders anywhere else, and an arm of `()` (`FieldDef::is_unit`) as `null` on every surface, where `()` anywhere else keeps its own description. `written_std_result` in `src/field_type.rs` is the one seam that says a path is one: `Result` with two type arguments, written in full under `core::result` or `std::result` (`names_std_result`), or under any other path but a name `is_declared_ident` holds; a `Result<T>` of one keeps its reading. A `Result` written by any path but the full one earns the check `is_std_result` that `src/model_schema/ranges.rs` writes beside the holder, which fails with E0308 where the name is another type. Each client writes a type of its own: a Dart sealed class `DartResult{N}` with an `Ok` and an `Err` subclass, one per shape of `()` arms an item writes (`DART_RESULTS`), a Swift enum `{Holder}{Field}Result` of the cases `ok` and `err` that declares the parameters its arms reach, and a Kotlin sealed interface `KotlinResult{N}` with a class or an object per arm and a serializer that reads the object key by key (`KOTLIN_RESULTS`). Each refuses an object of both keys, of neither or of another key, as serde does. `validate()` walks the arm the value holds under its key (`HeldEnd::Arms`, the walk that branches as `HeldEnd::Positions` does), the judged read reads a model in either arm unjudged, built back arm by arm (`walked_read`, `result_read`, which several models of one member hand `MemberHook::leaves`), the recovering decode walks into the arm (`Step::Arms`) and lists an object that names neither arm or both as one `Invalid` at the field, expected `Variants(["Ok", "Err"])`, and under `mongodb` a `Result` field is a `Field`. A `Result` as a map key and a bound key on a `Result` field are refused in every build, and a `Result` under `#[serde(flatten)]` in every build with `serde`
- `#[serde(flatten)]` field → TypeScript intersection (`A & B`), Zod `.and(...)`. A source writing more than one key set — a registered choice, or one reached through an `Option` — multiplies instead: `zod_merged_joins` builds one `.and(...)` chain per key set, `zod_merged_statements` binds the object to `{Type}$OwnSchema` and joins the copies where the object is a struct's own published binding, and `zod_merged_object` writes the object out once per copy where it is an enum variant's fragment of a larger literal. Either way the item contributes exactly one member to whatever union encloses it. A map under `#[serde(flatten)]` beside any other member that writes a key is refused at `exec_model_schema`, ahead of every shape, in every build with `serde`, on a struct and on a variant of an enum under every tagging: serde writes the map's entries at one level with that key, and no TypeScript type describes both. `flattened_map_beside_errors` in `src/model_schema/ranges.rs` answers one error per such map, spanned on it and naming a member beside it; a member under `skip` writes no key. A map flattened alone beside the tag serde writes at its level is refused there too, naming the tag: a struct's own `#[serde(tag = "...")]` (`struct_tag`) and an enum's `tag` with no `content` (`Tagging::Internal`), which `flattened_map_errors` in `src/model_schema.rs` hands `flattened_map_beside_errors` as its `tag`. A variant under a `content` key, under its name or untagged writes no tag beside the map, and builds. The lone slot of a newtype variant under a tag alone is written beside the tag as well, and one that writes a map is refused by `tagged_slot_map_error` in `src/model_schema/ranges.rs`, which `internally_tagged_guard_errors` asks of every slot `tagged_content` answers `TaggedContent::Flattened` for, in every build with `serde`, spanned on the slot and naming the variant, the slot's type and the tag: ``type `Filled`: variant `Counted`, field `0`: `Tally` writes a map, which cannot sit beside the tag `kind` ``. A member writes a map where its type, through `Option` and the transparent wrappers, is a map, or names a type declared above that `writes_an_open_map` recorded (`record_open_map` and `is_open_map` in `src/utils.rs`, kept in every build with `serde`): a struct whose one member on the wire is a flattened map, a brand or a struct of one slot over a map, an alias of one, and an `#[serde(untagged)]` enum with a variant serde writes as one (one slot, or one member on the wire that is flattened). A generic type of those shapes written as its own type parameter is recorded with that parameter's position in `OPEN_MAP_PARAMETERS` (`open_map_parameter_positions`, which reads a parameter through `Option`, the transparent wrappers, what `forwarded_parameter_position` reads and a type `OPEN_MAP_PARAMETERS` holds), a struct with a tag of its own apart, and `def_writes_a_map` reads it as a map where the argument at that position writes one: `Wrap<HashMap<String, i32>>` beside a member is refused with the message of a map. A type parameter flattened beside a key is recorded in `FLATTENED_BESIDE_PARAMETERS` (`flattened_beside_parameters_of`, a `FlattenedParameter` naming the type that flattens it, the parameter and the member or tag beside it; the lone slot of a variant under a tag alone is read as flattened beside the tag), and so is a parameter the item hands on to such a type at that position (`struct Bag<U> { letter: Letter<U> }` records `U`). `filled_map_errors`, beside `flattened_map_beside_errors` in `flattened_map_errors`, walks every type the item writes, at any depth, and each `default_types` entry against the item's own record, and refuses each argument that writes a map at a recorded position, spanned on the argument: ``type `Post`: field `letter`: `Letter` flattens its type parameter `T` beside `id`, and `HashMap<String, i32>` fills it with a map``. The type that flattens its parameter, and every filling that is no map, build. A type declared below, or in another crate, is not looked into. A flattened map is therefore the only key set of its object, and is joined as a record of every key (`FieldDef::zod_merged_schema`). The JSON Schema merges a flattened map as its own description where its key is read as a string or a plain enum (`flattened_map_json_schema_value`), and as no key for any other key. The merge (`merged_tree` and `map_handing` in `src/features/jsonschema.rs`) reads as a map a leaf of a source `MergedSource::map` marks, a map keyed by a plain enum among them, and a leaf whose `additionalProperties` is not `false`, a struct with a flattened map and an alias of a map among them: the finished object's `additionalProperties` is the map's values. An object that merges no map is written as it was. serde reads an `Option` of such a map, the field's own or an alias's, as `None` wherever an entry does not read as the map, and every entry stays in the object, so its absence holds every key with any value: `flattened_as_entries` in `src/model_schema.rs` says which source is read as entries, through `def_writes_a_map`, the reading of the refusal, Zod joins `ZOD_DROPPED_ENTRIES` in place of the bare object where `MergedOperand::entries` holds (`zod_merged_joins`), and the JSON Schema writes `Branches::Dropped`, `additionalProperties: true`, in place of `Branches::Absent` where `MergedSource::entries` holds (`entries_left`). An `Option` of a struct, or of an enum that writes no map, keeps its two key sets. TypeScript, Zod and the JSON Schema therefore refuse a payload whose keys do not read as that source, which serde reads as `None`, dropping the keys, as `README.md` states and `Hawser` in `tests/flatten_tests/maps.rs` holds. `tests/flatten_tests/maps.rs` holds the models of a map flattened alone in each place, an optional one among them, an optional generic brand filled at a map and an optional untagged enum with a map among its variants, and what serde reads of each payload, `tests/flatten_tests/map_schemas.rs` the Zod text, `tests/flatten_tests/map_documents.rs` the JSON Schema of each model held to what serde reads, `src/model_schema/ranges/tests.rs` each refusal and each shape that builds, and `tests/generic_types_tests/declarations.rs` parses each payload under node
- `#[serde(untagged)]` enum → TypeScript union (`A | B`), Zod `z.union([...])`, JSON Schema `anyOf`
- A member of a struct or of an enum's variant under `#[serde(default)]` → the TypeScript member unchanged, Zod `T.prefault(<json>)`, JSON Schema `"default": <json>` with the key left out of `required`, and each client reading the JSON of the default where the key is left out (section 13)
- Type parameter → TypeScript parameter (`X<IdType>`), Zod `X$SchemaFactory(idType)` (memoized, one cache level per parameter) plus `X$SchemaDefault` — that same factory called at the item's own declared `default_types`, checks included — JSON Schema the document of whatever fills it — `default_types(...)` where the type stands alone, the reference site's arguments where a field embeds it. A lifetime is dropped on every surface, a borrowed value writing what its owned form writes; a const renders as an array length only, and is refused where it is handed to a written type as an argument
- `serde_json::Value` and `serde_json::Map` → refused, in every build, by the path written. No
  target describes either, and none keeps code for them.
  - **The full path is refused at expansion**: `serde_json::Value`, `::serde_json::Value` and
    `serde_json::Map<…>`, a path whose segments are exactly `serde_json` then the name. A compile
    error naming the place, ``model_schema: type `Holding`: field `v` reaches
    `serde_json::Value`, which holds any JSON value, so no target can describe it.`` and the
    rewrite to make. `any_json_errors` asks it at `exec_model_schema`, ahead of every shape, of
    each type `any_json_positions` lists (a field, a slot, a variant's member, the type an alias
    names and each `default_types` filling), through `ranges::any_json_named`, and refuses through
    `UndescribableStd::AnyJson`, the arm beside the std types `undescribable_std_rejection`
    refuses (`OsString`, the cells and locks, `LinkedList`). Those are asked at each position
    under a schema surface; this one is asked once, ungated, because no build may describe it. A
    module of the crate's own named `serde_json` is not told apart from the crate: the owner's
    decision.
  - **A bare name is asked of the compiler.** A `Value` with no arguments and a `Map` with
    arguments, written as one segment, may be a type declared below the item or in another
    crate, which no expansion sees. `any_json_checks`, gathered by `asked_of_the_compiler` beside
    `ranges::identity_checks`, writes one `ranges::any_json_check` beside the item per spelling a
    position writes: `Probe<T>` with an inherent `const ANY_JSON: bool = true` on
    `Probe<::serde_json::Value>` (on `Probe<::serde_json::Map<::std::string::String,
    ::serde_json::Value>>` for `Map`) and a `Fallback` trait's `false` for every other `T`, then
    `::core::assert!(!Probe::<written>::ANY_JSON, refusal)`, spanned on the written type, which
    fails with E0080 and the refusal's message only where the name is `serde_json`'s type. The
    first assert reads the inherent const, so no `dead_code` fires; nothing is allowed.
    `Probe` and `Fallback` are numbered where the written type names them (`unclaimed_parameter`).
    A crate that writes a bare name lists `serde_json`, which the check names.
    `ranges::any_json_bare` keeps no check for a name `is_declared_ident` answers for, one of the
    item's own type parameters, or the item's own name, and none for a `Map<…>` whose arguments
    name what a check outside the item cannot (one of its parameters, `Self`, a lifetime other
    than `'static`). `tests/any_json_refusals.rs` holds a bare `Value` brought in with
    `use serde_json::Value;` to the E0080, and the crate's own `Value` declared below an alias
    that names it to a build with warnings denied; `tests/shadowed_map_tests/named_value.rs`
    lints that shape under this crate's own table.
  - **A `#[service_schema]` operation is held to both**: `any_json_refusals` in
    `src/service_schema/parse.rs`, gathered by `service_refusals`, refuses the full path in each
    type `json_positions` lists (the one message the operation names, each `header_in` and `part`,
    and both arms of the reply, a tuple that carries `header_out` or `error_header_out` taken
    apart), and `any_json_checks` there writes the check of each bare name beside the trait, in the
    expansion that passes every refusal, keeping the context and a declared name. Both word the
    error through `undescribable_std_message`: ``service_schema: operation `fetch`: the request
    reaches `serde_json::Value`, ...``. The message declared from several arguments is a
    `#[model_schema()]` struct, refused or checked by that expansion. tixschema's own APIs that
    take or answer a `serde_json::Value` (a `preprocess` function, `from_value_with`,
    `from_value_piped`, `schema_examples()`, the archie conversions) are untouched

### 5. Zod v4 Requirements

**⚠️ IMPORTANT: This crate requires Zod v4 for full functionality.**

Frontend dependencies:
```bash
npm install zod@^4.0.0
```

Generated schemas use Zod v4's modern syntax:

```typescript
// ✅ Generated (Zod v4 compatible)
export const User$Schema = z.strictObject({
  id: z.string(),
  name: z.string(),
  email: z.union([z.null().transform(() => undefined), z.string(), z.undefined()]).prefault(undefined),      // Modern v4 syntax, null coerced to undefined
  age: z.union([z.null().transform(() => undefined), z.number().int(), z.undefined()]).prefault(undefined),  // Works with JSON schema generation
}).meta({
  examples: []
});

// ❌ OLD FORMAT (no longer generated)
export const User$Schema = z.strictObject({
  id: z.string(),
  name: z.string(),
  email: z.string().optional(),
  age: z.number().int().optional(),
}).transform(args => Object.assign(args, {
  email: args.email,
  age: args.age
}));
```

**Benefits of Zod v4**:
- **JSON Schema Generation**: Can generate JSON schemas from Zod schemas
- **Cleaner Code**: No transform functions needed
- **Better Performance**: No runtime transform overhead

### 6. Serde Attribute Support

The macro respects Serde attributes when the `serde` feature is enabled:

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfile {
    pub user_id: String,  // → userId in TypeScript
    #[serde(rename = "emailAddress")]
    pub email: String,    // → emailAddress in TypeScript
}
```

A struct's own `#[serde(tag = "...")]` writes the struct's serde name under that key, and every
surface describes it. `struct_tag` in `src/features/serde.rs` answers the key and the name (the
ident, or the container's `rename`); `with_struct_tag` puts the pair ahead of the struct's members
as a required string literal and refuses a field that writes the same key, which serde would write
twice. Dart writes it into `toJson`, Swift encodes it under `SwiftSchemaTagCodingKeys`, and Kotlin
holds it in an `@EncodeDefault` property outside the constructor; all three read a payload with
the key or without it, as serde does.

### 7. Field Validation Attributes (`#[model_schema_prop(...)]`)

All validation constraints generate checks in **Zod, JSON Schema, Rust, Dart, Swift and Kotlin**. In Rust the check runs in `validate()`, which the type's own `Deserialize` calls on the value it read, so a serde read refuses what `validate()` refuses ("A read refuses what `validate()` refuses", below), and in the recovering decode of a type whose `enable` list holds `"decode_with"`, which lists the violations of each value as one issue. While the members are still being read it runs in one position only: a member of an untagged enum, where the check decides which variant the payload is. In Dart, Swift and Kotlin the check runs in the `validate()` every emitted type carries, which the type's read calls on the value it read: `fromJson`, `init(from:)` and `{Type}Serializer`.

A function a `preprocess` names is called by the read alone of each language, ahead of the value, so the read that judges calls it too: `model_schema_read_unjudged` in Rust, `z.preprocess(fn, ...)` in the Zod schema, `fromJsonUnjudged` in Dart, `init(swiftSchemaUnjudged:)` in Swift and `{Type}Unjudged` in Kotlin. A function a `check` names is called by `validate()` in the four that have one, and by one `.check(...)` on the Zod schema. Rust calls the snake case of a name, and every other language the name as written:

| Attribute | Field Type | Zod | JSON Schema | Rust | Dart | Swift | Kotlin |
|-----------|------------|-----|-------------|------------|------|-------|--------|
| `pattern = "regex"` | `String` | `.check(z.regex(/regex/))` | `"pattern"` | `validate()`, which the type's read runs; the member's own hook too in an untagged enum | `validate()`: `RegExp("...").hasMatch(value)` | `validate()`: `value.range(of: "...", options: .regularExpression) == nil` | `validate()`: `Regex("...").containsMatchIn(value)` |
| `minLength = N` | `String` | `.min(N)` | `"minLength"` | `validate()`, which the type's read runs; the member's own hook too in an untagged enum | `validate()`: `value.runes.length` | `validate()`: `value.unicodeScalars.count` | `validate()`: `value.codePointCount(0, value.length)` |
| `maxLength = N` | `String` | `.max(N)` | `"maxLength"` | `validate()`, which the type's read runs; the member's own hook too in an untagged enum | `validate()`: `value.runes.length` | `validate()`: `value.unicodeScalars.count` | `validate()`: `value.codePointCount(0, value.length)` |
| `minLength = N` | a list | `.min(N)` on the array | `"minItems"` | `validate()`, which the type's read runs; the member's own hook too in an untagged enum | `validate()`: `value.length` | `validate()`: `value.count` | `validate()`: `value.size` |
| `maxLength = N` | a list | `.max(N)` on the array | `"maxItems"` | `validate()`, which the type's read runs; the member's own hook too in an untagged enum | `validate()`: `value.length` | `validate()`: `value.count` | `validate()`: `value.size` |
| `minimum = N` | numeric | `.min(N)` | `"minimum"` | `validate()`, which the type's read runs; the member's own hook too in an untagged enum | `validate()`: `value < N` | `validate()`: `value < N` | `validate()`: `value < N`, in the number's own type |
| `maximum = N` | numeric | `.max(N)` | `"maximum"` | `validate()`, which the type's read runs; the member's own hook too in an untagged enum | `validate()`: `value > N` | `validate()`: `value > N` | `validate()`: `value > N`, in the number's own type |
| `literal = "val"` | `String` | `z.literal("val")` | `{"type": "string", "const": "val"}` | — | — (a `String`) | — (a `String`) | — (a `String`) |
| `literal = true` | `bool` | `z.literal(true)` | `{"type": "boolean", "const": true}` | — | — (a `bool`) | — (a `Bool`) | — (a `Boolean`) |
| `literal = 214` | numeric | `z.literal(214)` | `{"type": "number", "const": 214}` | — | — (a `double`) | — (a `Double`) | — (a `Double`) |
| `preprocess = ["fn"]` | any, and a type | `z.preprocess(fn, ...)` | — | every read of the type calls `fn` in snake case on the payload, a `serde_json::Value`, ahead of the value's read | in `fromJsonUnjudged`: `fn` on the payload, a `dynamic`, ahead of the value's read | in `init(swiftSchemaUnjudged:)`: `fn` on the payload, an `Any`, ahead of the value's read | in the read of `{Type}Unjudged`: `fn` on the payload, a `JsonElement`, ahead of the value's read |
| `check = ["fn"]` | any, and a type | `.check((ctx) => { ... fn(own.value, own); ... })` | — | `validate()` calls `fn` in snake case, in every build, and the type's read refuses with its messages; an issue of the recovering decode | `validate()` calls `fn`, and `fromJson` throws `FormatException` with its messages | `validate()` calls `fn`, and `init(from:)` throws `DecodingError.dataCorrupted` with its messages | `validate()` calls `fn`, and `{Type}Serializer` throws `SerializationException` with its messages |
| `ts_optional` | `Option<T>` | — | — | — (TypeScript-only) | — | — | — |
| `as_number` | `DateTime<Tz>` | inline `z.preprocess(..., z.number())` | — | serde `with = chrono::serde::ts_milliseconds` injected | an `int` | an `Int64`, which the emitted codec reads from an ISO 8601 string and writes as one | a `Long` |
| `nullable` | `Option<T>` | `z.union([T, z.null()])`, key required | `anyOf: [T, {"type":"null"}]`, key required | guard: refuses a key-dropping serde attr | `T?`, the one spelling of an `Option` | the key always encoded, `nil` included | no `= null` default on the property |

`README.md`'s "Preprocess and check" section is the one place that states `preprocess` and `check`
for the five languages side by side. It names the section of each language for the detail and
repeats none of it. What a change has to keep:

- **Each cell of its table is text the test of that language holds.**
  `tests/preprocess_and_check_tests/readme.rs` reads the section beside the source of each test:
  a signature is one the test compiles, an error is one its run asserts, and the name of a read
  is one the text of the emitter is pinned to. No test holds the TypeScript error of a name with
  no function, `error TS2304: Cannot find name 'trimmed'.`: it is what `tsc --strict` reports.
- **The 13 lines are one text in the test of each language.** `tests/check_tests/models.rs`
  holds them in `TABLE`, which the Rust `validate()` and the Zod schema are compared with, and
  `run_dart_validate.rs`, `run_swift_validate.rs` and `run_kotlin_validate.rs` each hold a copy
  their run is compared with. The section shows that text.
- **The parcel the section declares is compiled.** `tests/preprocess_and_check_tests/models.rs`
  declares it, its members in the order of their names, and the two reads the section shows are
  what a run answers.
- **A limit of a client is stated in the client's own section.** The section says that limits
  exist and names them in one sentence.
- **The key is a last resort in every place the documents introduce it**: the section, the
  section of the key, and the section of each client.
- **The upgrade list states what a consumer sees**, each item with the text of its error.

**A key written on a field measures the field's own value.** `Option` and the transparent wrappers
(`Box`, `Rc`, `Arc`, `Cow`, and every reference) hold one value and are read through;
a list is a value of its own and is not. So on a list `minLength` and `maxLength` count its items,
whatever they are, and `pattern`, `minimum` and `maximum` are refused. `README.md`'s "List Length"
section is the consumer's view; this is what a change has to keep.

- `constrained_shape` ends on `ConstraintLeaf::List` at an array, a slice or a std sequence
  (`names_a_list`), under whatever wrappers it walked, and reads nothing the list holds.
  `FieldDef::is_list` is the same question asked of the def every surface renders.
- `apply_model_schema_prop_meta` moves a list's two lengths out of `min_length`/`max_length` into
  `ModelSchemaPropMeta::min_items`/`max_items`, so no reader of a string's length finds them. Zod
  writes them on the outermost array (`zod_item_count_checks`), the JSON Schema as `minItems` and
  `maxItems` beside a fixed array's own (`array_length_json_schema_bounds`, the longer minimum and
  the shorter maximum), and `apply_constraint_docs` in the words a string's length is stated in.
- `list_validator` and `list_deserializer` write `validate_{stem}_value` and `deserialize_{stem}`
  over any list: the list is the type parameter `L`, held to `for<'held> &'held L: IntoIterator`
  and counted by iterating it, and each transparent wrapper is a parameter named by what it derefs
  to. No type a list holds is written into the schema module, where one of the item's own
  parameters, `Self` and a borrowed lifetime name nothing. A bound on the iterator itself
  (`ExactSizeIterator`) is one the compiler cannot infer `L` under.
- `BoundKind::Scalar` is what a list is not measured by, and `check_bound_kind` refuses it in the
  list's own words, with `item_rule_advice` saying where a rule for each item goes: a brand carries
  a `pattern`, and a range for each number of a list has no place. `check_fixed_shape_constraints`
  asks nothing more of a list, whose items may be of any type.
- A list's own count does not stand in for what its items' type declares: `named_member_validate_body`
  runs both, where every other member's own check is run in place of its type's.
- The count is reported in a string's sentence (`Bound::MinItems`, `Bound::MaxItems`), and Zod reads
  the items off the issue behind `Array.isArray`.
- A brand over a list (`held_as_a_list`: the list itself, or behind `Box`/`Rc`/`Arc`/`Cow`) takes the
  two lengths on the type. `branded_value_inner` hands them to the inner's def as its count, so Zod
  and the JSON Schema write them where a field's list carries them, and `branded_list_validation`
  emits the list's validator under the brand's name, with no `Display` bound. `pattern`
  on such a brand is refused by `branded_constraint_inner_error`.

A length or a pattern on a field typed with a type declared elsewhere reaches a brand over a
string. `constrained_shape` ends on `ConstraintLeaf::Named`, the field's validator takes the leaf
itself, and `named_leaf_rendering` reads its text through `model_schema_bound_text`, which
`branded_bound_text_method` publishes on a brand whose inner is a string or a path. An inherent
method is found ahead of a trait's, so the block-local `UnbrandedText` answers only for an alias of
a string or a path, and its `#[diagnostic::on_unimplemented]` is the refusal every other type
gets, at the field. Zod writes the checks as one `.check(…)` on the named schema
(`zod_named_checks`), and JSON Schema narrows it inside an `allOf`
(`named_field_json_schema_value`). `check_fixed_shape_constraints` refuses a range on any named
type and a length on one written with arguments, in every build.

A bound of the wrong kind is refused there as well, by `check_bound_kind`, in every build: a length
or a pattern on a numeric field, a range on a string, a `bool`, a `char` or a field of a type every
surface describes as unknown (`FieldDefType::Unknown`), and a pattern or a
range on a list, under `Option` and any transparent wrapper, naming the field and only the keys of
the wrong kind. The refusal of a range says what the field writes, and only a string's names the
keys that bound it. `FieldDef::unmeasured_bound` is the one question the refusal
and `apply_constraint_docs` are both written from, so the TypeScript comment states no bound the
refusal turned away.

`validate()` reads through every reference, whatever it borrows, as `get_field_def` does for the
schemas: a borrowed `str` is held to its bound as a `String` is, a `&'a String` and a `&'a u32` as
their owned types are, and a borrowed list is counted as an owned one is. `constrained_shape` reads
each reference as `ConstraintWrap::Borrowed`, at any depth, and the walk reads through it with
`&**`. A field read through a reference publishes its validator and no `deserialize_{field}` hook
(`ends_on_a_borrow`, which asks whether any wrapper of the walk is a reference), and
`generate_field_validation` hangs nothing on one, nor on a member its author reads through a
`deserialize_with` or a `with` of their own (`has_serde_read_hook`), serde admitting one reader per
field. A borrowed member of an untagged enum and one read by its author's reader are therefore the
constrained members the read does not check: each is read with no bound written, and `validate()`
answers for the bound. A reference to a type a bound is refused on is
refused the same way: a length on a `&'a bool` is the E0277 a length on a `bool` is
(`tests/bound_text_refusals.rs`), and `blocking_interior_mutability_wrapper` retraces the walk
through a reference, so a bound on a `&'a RefCell<String>` is refused at expansion, naming the
wrapper.

A slot takes the five keys, of a tuple variant and of a tuple struct, and is checked exactly where a
named member in the same position is. `checked_member` is the one seam that names a member for its
helpers: a slot goes by its declaration index, behind its variant where it has one
(`validate_{variant}_{index}_value`, `deserialize_{variant}_{index}`) and alone in a struct
(`validate_{index}_value`, `deserialize_{index}`), a stem no field's can meet, an ident never
starting with a digit. A variant's slot is read off the binding its arm makes
(`positional_member_binding`, `bound_member`), and a struct's off `self` by its index
(`slot_access_expr`).

- The lone slot of an untagged newtype variant is hung with the hook, and `validate()` reports it
  under no name, as it reports what the slot's own type declares. A slot with a check of its own
  and a type that declares one runs both (`slot_validate_body`).
- A slot of a tagged variant is checked by `validate()`, which the enum's read runs on the
  value it read, and by no hook as the slot is read. It is reported where serde writes it, as
  what its type holds is: under the variant's name or the content key, and under its
  position among several slots (`'Code': …`, `'Pair.1': …`, `'data.1': …`). Its validator is
  recorded under the `Validator` gate, and the recovering decode reads it back through
  `slot_validator`: `Slot::validator` for a variant of several slots,
  `WalkedVariant::lone_bound` for a variant of one, each an `Invalid` at the slot's own path.
- A slot of a tuple struct is checked where a named field of a struct is: by `validate()`,
  which the struct's read runs on the value it read, and by no hook as the slot is read. It is
  reported under its position among the slots serde reads, and under no name where it is the
  struct's one slot, as what its type holds is. `collect_tuple_slots` hands each
  slot's validator and `validate()` fragment to `process_tuple_struct` in `TupleSlots`. The
  recovering decode reads the validator
  back through `slot_validator` with no variant: `Slot::validator` for a struct of several slots,
  and for a struct of one, which serde writes as the slot's value, the validator
  `struct_recovering_decode` hands `held_methods`. A list, an `Option` or a model in that slot is
  walked with it as a field's value is, and a plain value is held to it by `lone_slot_bound`,
  which reads the struct whole with its own reader, so a hook on the slot has run before the
  bound is checked.
- A `#[serde(transparent)]` struct of one slot is a brand, and `branded_slot_prop_errors` still
  refuses a `#[model_schema_prop]` on its slot.
- The JSON Schema writes a slot's bounds through `bounded_slot_json_schema` and
  `bounded_slots_json_schema`, a variant's and a tuple struct's alike; Zod reads them off the
  slot's `FieldDef` as it does for any member.

A string check written on an enum, `#[model_schema(minLength = N, maxLength = N, pattern = "...")]`,
is the same check written on each field the enum reaches, enforced exactly where that field's own
is in that position, under every tagging. `member_prop_meta` is the one seam: it reads a member's
own `#[model_schema_prop]` and holds it to the enum's arguments as well, so every surface and gate
reads the merged meta as it reads a member's own. `process_field` asks it through
`FieldContext::enum_args`, which a struct's field leaves `None`, and
`collect_untagged_variant_members` asks it directly.

- `written_as_text` says what is reached: a field written as a string or a path itself, under
  `Option`, an ownership wrapper or a reference. A list and its items, a field typed with
  a named type (a brand included), a number, a map, a tuple and a type parameter are not. It is
  `constrained_shape`'s walk held to `ConstraintLeaf::Str` and `ConstraintLeaf::Path`, written
  apart because it is asked in a build without `serde`;
  `the_reach_of_an_enums_check_is_the_constraint_walks` holds the two together. A member that
  carries a `literal` is not reached: the enum's check never sits beside a literal, which takes no
  other rule. `reached_by_enums_check` is the one question, `written_as_text` and no `literal`:
  `member_prop_meta` asks it before it holds a member to the enum's check, and
  `enum_check_reaches_nothing` asks it of every field, so the refusal and the surfaces cannot
  drift.
- A field with a check of its own is held to both: the longer `minLength`, the shorter `maxLength`,
  and the enum's `pattern` in `ModelSchemaPropMeta::enum_pattern` beside the field's own, which
  `patterns()` yields in that order. Zod writes one `.check(z.regex(…))` each, `both_patterns_json`
  states the two in an `allOf`, and the validator runs one check each. A named leaf is never
  reached, so its three readers (`zod_named_checks`, `named_field_json_schema_value`,
  `named_leaf_rendering`) read `pattern` alone.
- `enum_string_check_refusal_output` answers at `process_item`, ahead of every enum shape and in
  every build, so an enum's `pattern` is judged in every build as a field's is. It refuses a
  `pattern` no surface reads, a check that reaches nothing (`enum_check_reaches_nothing`: no
  variant written with no data and no field `reached_by_enums_check` answers true for), and each
  name of a unit variant that breaks the check (`unit_variant_name_errors`). The refused enum is
  re-emitted without its members' `#[model_schema_prop]` (`without_member_keys`, through
  `refused_item_output`, which every refusal raised before the members are walked goes through),
  so the refusal is the only error rustc reports for it.
- A unit variant is one `variant_wire_kind` publishes as a unit. `variant_wire_names` lists the name
  serde writes for it, then the name it reads and each `alias` where they differ, and
  `checks_broken_by` measures each as the validator measures a value. An untagged enum writes such
  a variant as `null`, under no name, and refuses it on its own. The name of a variant that carries
  data is not measured.

A `check = ["name", ...]` names functions of the consumer's own, on a field and, in
`#[model_schema(...)]`, on a type. `README.md`'s "Checks" section is the consumer's view; this is
what a change has to keep.

- **One reader.** `function_names` in `src/features/model_schema_prop.rs` reads the key for both
  attributes, in every build, into `FunctionName { name, written_at }`, and raises the five refusals,
  each spanned on what was written: no array, an element that is no string literal, a name that is
  no identifier, no name, a name twice. `apply_arg` and `parse_prop_key` both call it, and the key
  is in `KNOWN_ARGS` and `KNOWN_KEYS`. `checks::alias_refusal_output` refuses it on a type alias at
  `process_item`, and `branded_slot_prop_errors` on a brand's slot, as every key written there.
- **Every call is written beside the type**, in the type's own `impl`, by `checks::call` in
  `src/model_schema/checks.rs`: the snake case of the name (`to_snake_case`) as an ident located at
  the string literal, with the macro's own hygiene, so a missing function (E0425) and a bound the
  type does not declare (E0277) are reported at the attribute. No call is written into
  `{type}_schema`: a type declared inside a function body names a function no path from another
  module reaches.
- **The glob does not reach a call.** `validate()` of an item that names no check holds
  `use {type}_schema::*;` and is emitted token for token as before. Where the item names one
  (`ItemChecks::named`, which `names_a_check` answers for a member's bound from the registry
  `record_item_checks` writes at `exec_model_schema`), `validate_method` writes no such import and
  `bound_body` wraps each bound in a block that imports the module for itself. A check named after
  a helper of the module (`validateLabelValue`) therefore calls the consumer's function.
- **The order.** A member's bounds and what its type declares, then that member's checks in the
  order written (`then_checks`), member by member, and the type's own last
  (`ItemChecks::own_calls`). Every one runs.
- **A member's check takes the member's own value.** `checks::reach` walks `Option`
  (`CheckStep::Present`), the ownership wrappers and a reference (`CheckStep::Held`) and stops at
  whatever is under them, a list and a map included; `the_reach_of_a_check_is_the_reach_of_a_bound`
  holds it to the wrappers `constrained_shape` reads. It is reported through
  `MemberPlace::reported`, which `member_place` answers for a bound's member as well: the steps
  serde writes on the way to the member, joined by dots, and nothing where it writes none.
  `nested_under` writes the name.
- **`validate()` in every build.** With `serde` and a schema surface it is the method the bounds
  are in. Without `serde`, and with `serde` and no surface, `validate_method` and
  `checks_validate_impl` publish it with the checks alone, and only for an item that names one.
- **Every shape.** `process_struct`, `process_tuple_struct`, a unit struct, the three tagged enum
  processors and the untagged one (a member's check in its variant's arm, through
  `build_member_check_arms`, and the enum's own after the `match`), a plain enum, and a brand
  (`branded_validate_method`: the brand's bound, then its checks, under no path). A generic type's
  calls are in its generic `impl`, so its function is written once over every filling.
- **Zod.** `zod_check_wrap` in `src/field_type.rs` is the one writer of the wrap. A field's is
  written by `FieldDef::zod_checked_base`: after the bounds, inside the member of the `Option`
  union and of the `nullable` one, on the outermost array after its count, and inside
  `z.preprocess`, `zod_base` wrapping the checked base. A field that is one factory call hands the
  checked call into the union's arrow (`zod_handed_calls`). A member with a check is written around
  (`is_written_around`), so a flattened one is joined whole and never multiplied. A type's is
  written by `zod_published_binding` from `PublishedBinding::checks`, ahead of `.meta` and inside
  the builder of a generic item; a plain enum and a brand write it ahead of their own
  `.meta({ description })`, a brand ahead of `.brand()` too. The wrap's text holds `"\n})"`, so
  `build_branded_delegate_items` puts the examples at the first one after `.meta({\n  description: `.
- **The wrap copies each issue with `continue: true`.** Zod skips a later check after an issue
  without the flag, where Rust runs every one, and `tsc --strict` refuses to set the flag on a
  pushed issue (TS2540).
- **The clients call each check.** The Dart, Swift and Kotlin emitters write a call of the
  function, under the name as written, in the `validate()` every emitted type carries: see
  "`validate()` on the Dart types", "The Swift `validate()`" and "The Kotlin client's
  `validate()`" below. The JSON Schema and the TypeScript type write what they wrote, and no
  expected output of a type without the key changes.
- **The documents' example is a doctest.** `src/check_example.md` is included in
  `model_schema_prop`'s rustdoc under `serde` and a schema surface, and
  `tests/check_tests/readme.rs` holds the README to it. `tests/check_tests/models.rs` is compiled
  by `tests/check_tests.rs` and by `tests/generic_types_tests/declarations.rs`, which parses each
  case under node and compares the two sides as one text. This crate's lints order a struct's
  members by name, so the compiled `Parcel` declares `label` last and reports it after
  `item_code_list`; the doctest declares the members as the documents do.

Multiple constraints on one field are combined. Multiple `preprocess` functions nest:
`z.preprocess(fn1, z.preprocess(fn2, innerSchema))`.

`literal` takes a string, boolean, integer or float literal. The kind written must match what the
field's Rust type can carry — a boolean literal only on a `bool` field, a numeric literal only on a
numeric field (`u8`–`f64`), a string literal only on a `String` field — or the attribute is a
compile error naming both the literal's kind and the field's declared type. A numeric literal is
stored as `f64` and rendered without a trailing `.0` on TypeScript and Zod; the JSON Schema `const`
is always written under `"type": "number"`, never `"integer"`.

A `literal` takes no other rule: it admits one value, so a rule beside it is always met or never
met. `check_literal_alone`, called from `model_schema_prop_guard_errors` right after
`check_literal_kind_match`, refuses a literal written beside `minLength`, `maxLength`, `pattern`,
`minimum`, `maximum` or `check`, in every build, naming the field, the literal and each rule
(`written_constraint_keys`, read before a list's lengths move into `min_items`/`max_items`, then
`check`). `as`, `preprocess`, `ts_optional` and `nullable` hold the value to no rule and stay
allowed beside it. `README.md`'s "Literal Values" section shows the refusal, and
`a_literal_beside_a_rule_is_refused` holds the README to it.

`ts_optional` is a bare flag (no value): it renders an `Option<T>` field as the optional TypeScript key `field?: T` instead of the default `field: T | undefined`. Zod and JSON Schema output are unchanged. It is only valid on `Option<T>` fields (non-`Option` is a compile error) and composes with `as = Type`. The member must also have a key for the flag to make optional, and `validate_ts_optional_flag` refuses both positions where it has none, in every build: a positional slot writes no key at all, and one a serde attribute takes off the wire in both directions (`skip`, or `skip_serializing` and `skip_deserializing` together) is described on no surface. An attribute that drops the key one way only (`skip_serializing_if`, `skip_serializing`) leaves the member standing, and the flag still decides its spelling.

`as_number` is a bare flag (no value): it renders a `DateTime<Tz>` field as a `number` (epoch milliseconds) with an inline self-contained Zod coercer, instead of the default native `Date`. It is only valid on `DateTime<Tz>` fields (anything else is a compile error) and is honored on a tuple-variant `DateTime<Tz>` enum payload. With the `serde` feature on, it also injects `#[serde(with = "chrono::serde::ts_milliseconds")]` (`ts_milliseconds_option` for `Option<DateTime<Tz>>`) so Rust writes the same epoch-milliseconds number every other surface describes, unless the field already carries its own `with`/`deserialize_with` — a consumer's own `chrono` dependency needs its `serde` feature enabled for that module to exist.

`nullable` is a bare flag (no value): on an `Option<T>` field it renders `T | null` with the key **required** on TypeScript, Zod and JSON Schema, instead of the default coercing `T | undefined` with the key left optional. It is only valid on `Option<T>` fields (non-`Option` is a compile error), is refused together with `ts_optional` (the two disagree about the key), and composes with `preprocess` — the preprocess wrap goes around the whole nullable union. With the `serde` feature on, it is also refused together with a key-dropping serde attribute (`skip_serializing_if`, `skip_serializing`, `skip`) — the flag declares the key always written, so dropping it would let serde write a payload the generated schema does not admit. The mirror guard, `check_optional_field_serialization`, requires that same attribute on a bare `Option<T>` field carrying no `nullable`.

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct User {
    #[model_schema_prop(minLength = 3, maxLength = 50, pattern = "^[a-z0-9_]+$")]
    pub username: String,

    #[model_schema_prop(minimum = 0, maximum = 120)]
    pub age: u32,

    #[model_schema_prop(preprocess = ["epoch_to_date"])]
    pub date_value: NaiveDate,
}
```

#### Generated `validate()` method

When any field has constraints, the macro generates a `validate(&self) -> Result<(), Vec<String>>` method that aggregates all per-field errors. It judges a value built or changed in code, and the type's own `Deserialize` calls it on the value it read:

```rust
let result = my_instance.validate();
match result {
    Ok(()) => println!("Valid"),
    Err(errors) => println!("Errors: {:?}", errors),
}
```

A message names a value by its path in the payload, in single quotes: every step serde writes on
the way to it, joined by dots (`'parcelList.0.itemLabel': too short: minimum length is 1, got 0`).
A step is a key, a position in an array, or nothing where serde writes the value itself. It is
the path an issue of the Zod schema holds, and the one the recovering decode lists. `README.md`'s
"The path a message names" section is the consumer's view; this is what a change has to keep.

| Member | serde writes it | Reported under |
|---|---|---|
| a named field of a struct | under its key | `'itemLabel'` |
| a slot of a tuple struct of several slots | at its position in the array | `'1'`, its position among the slots serde reads |
| the one slot of a struct | as the value itself | no step |
| a named member of a variant, no tag attribute | `{"Loose":{"freightLabel":…}}` | `'Loose.freightLabel'` |
| the one slot of a variant, no tag attribute | `{"Boxed":…}` | `'Boxed'` |
| a slot of a variant of several, no tag attribute | `{"Split":[…]}` | `'Split.1'` |
| a named member, `tag` and `content` | `{"kind":"Loose","held":{"haulLabel":…}}` | `'held.haulLabel'` |
| the one slot, `tag` and `content` | `{"kind":"Boxed","held":…}` | `'held'` |
| a slot of several, `tag` and `content` | `{"kind":"Split","held":[…]}` | `'held.1'` |
| a named member, `tag` alone | beside the tag | `'liftLabel'` |
| the one slot, `tag` alone | its own keys beside the tag | no step |
| a member of an `untagged` variant | as the value itself, or under its key | no step, or the key |

- **Two functions say what serde writes on the way to a member**, in `src/features/serde.rs`,
  and nothing else computes a step. `variant_steps` answers what is written ahead of a variant's
  members, read through `Tagging`: the variant's written name (its own `rename`, else the enum's
  `rename_all`) with no tag attribute, the content key under `tag` and `content`, and nothing
  under `tag` alone and for an untagged enum. `member_steps` answers each member's steps below
  that: a named member under its key (`parse_written_renames`, or the casing it is handed), a
  flattened one under none, the only slot under none, and any other slot under its position
  among the slots serde reads. A slot serde does not read has no position.
- `validate()` and the recovering decode both call them. `steps_of_members` and
  `steps_of_variants` hand each container's steps to `FieldContext::member_steps`, read before
  the walk takes a member's attributes off it, and `member_place` joins a member's steps into
  `MemberPlace::reported`, which `checked_member` holds in `CheckedMember::reported`. A bound, a
  check and the walk below a member are all written from that one place, a flattened member's
  and an untagged variant's among them. `read_members` and `variant_members` in
  `recovering_decode/checks.rs` call the same two, and keep the members serde reads.
- `rename_direction_guard_errors` refuses a `rename` whose two sides differ in every build that
  describes a type, so the key a surface is named by (`get_final_field_name`) is the key
  `member_steps` reads.
- Without the `serde` feature no attribute is read: a variant is under its own name and a member
  is as it is declared, the steps serde's defaults write.
- `CheckedMember::label` and `CheckedMember::stem` keep the Rust ident: a refusal at build time
  and a helper (`validate_{field}_value`, `validate_{variant}_{index}_value`) name the member as
  its author wrote it, a slot by its place among the ones declared.
- A read hook that answers a brand's refusal under a name (`named_refusal`, from
  `read_unjudged_below` and `named_read_hook`) is handed `MemberPlace::reported`, so a read and
  `validate()` name one path. An `Option` adds no step.
- `tests/check_tests/places.rs`, a module of `models.rs`, holds one type per row of the table,
  with a bound on the member and a model held in it. `tests/check_tests/table.rs` holds
  `validate()` to `PLACES`, and `tests/generic_types_tests/declarations.rs` parses each payload
  under node and holds the Zod schema to the same text.
- The index and the key are written by the nested walk alone. `walk_wraps` binds one index per
  list level (`index_binding`) and one entry per map level (`entry_binding`), `held_at` lists
  them after the member's key as `PlaceStep`s, innermost last, and `named_at` writes the name.
  `nested_leaf` reads a key (`key_binding`) only where a report is written under it, so an entry
  that reports nothing costs no text. A member's own bound stops at a list and reads no map
  (`constrained_shape`), so a list's count is reported at the list, with no index.
- The validator of a member is the one sentence both `validate()` and its `deserialize_{field}`
  hook answer with, so the hook names the same key.

`validate()` runs the `validate()` of every value of a declared type a member holds, and
`README.md`'s "A model held in a map, in a slot or in a tuple" section is the consumer's view:

- `held_shape` reads the member's type against its def: through `Option`, the transparent
  wrappers, every reference (`ConstraintWrap::Borrowed`) and each level of a list, as
  `nested_shape` does, into a map's values (`ConstraintWrap::Map`, which
  `is_map_wrapper` decides) and into each position of a tuple written as a type
  (`HeldEnd::Positions`). `held_walk` writes the walk and `nested_leaf` the call. A walk that
  ends on no declared type is `None` and adds no token: a primitive, one of the item's own
  parameters and a set of either are none.
- A slot of a tuple struct and of a tagged variant is walked as a named member is
  (`slot_nested_validate_body`), read and reported through `member_place`, under the steps serde
  writes on the way to it.
- A named member, a flattened one and an untagged variant's slot fall back on
  `reachable_nested_shape` (`member_held_shape`), the walk they had before a map and a tuple were
  read. A set, a deque and a heap are described as a declared type is, so a field written
  `HashSet<String>` is walked to its strings, and its type publishes a `validate()` that reports
  nothing. `named_read_hook` is hung by `reachable_nested_shape` alone: on no map and no tuple,
  on no member read through a reference (`named_below`, as `ends_on_a_borrow` says for a
  member's own bound), and on no member the type's unjudged read reaches (`member_read_hook`).
- A map's key is the text serde writes for it. `written_key_items` declares `MapKey` in the walk's
  block: its `Display` hands the key's own `Serialize` the `Formatter`, serde's serializer for
  which (in serde from 1.0.108) writes a string, a number, a `bool`, a `char`, a unit variant's
  written name and a newtype's inner. `written_key` answers `None` where serde writes no text,
  and where the key's type has no `Serialize` in reach (`UnwrittenKey`, reached for `&MapKey`
  alone), a type parameter that declares none among them: `validate()` is written under the
  bounds the type declares. `keyed_under` then leaves the key out of the name. Nothing here
  names `serde_json`.
- `nested_under` composes: a name is written into a report one run of steps at a time, innermost
  first, which is what lets a key be left out.

The macro also generates into the type's schema module:
- `validate_{field}_value(&FieldType) -> Result<(), String>` — pure static validator per field
- `deserialize_{variant}_{field}(D) -> Result<FieldType, E>` — serde hook that calls the static validator, generated only for a member of an untagged enum, the one position where a member's check runs as the member is read, and hung on no member its author reads through a `deserialize_with` or a `with` of their own
- A tuple variant's slot writes its declaration index where a named member writes `{field}`: `validate_other_0_value` and `deserialize_other_0` for the lone slot of `Other`

#### The Kotlin client's `validate()`

Every Kotlin type `src/features/kotlin.rs` writes carries `fun validate(): List<String>`, written
by `src/features/kotlin/validate.rs`: a data class, a unit struct's `object`, a brand's value
class, a tuple struct, a sealed interface with each variant, and a plain enum. It answers what
Rust's `validate()` answers for the same value. `README.md`'s "`validate()` in the Kotlin client"
section is the consumer's view; this is what a change has to keep.

- **The emitter reads what the web surfaces read.** `kotlin_schema_dispatch` takes `KotlinArgs`,
  which `kotlin_suffix_tokens` builds: the `name` override, the type's `check` names,
  `default_types`, and a brand's `minLength`, `maxLength` and `pattern`. `KotlinArgs::member_meta`
  is `member_prop_meta`, so an enum's own string check is each reached member's. `patterns_of`
  reads `enum_pattern` under the gate that field is compiled under.
- **`Rules` is the one writer, and its order is Rust's.** `Rules::member` writes a member's bounds
  (`bound_statements`), then what the value it holds reports (`held_statements`), then its checks
  (`check_statements`). A member with a bound of its own is not walked, a list apart, which runs
  its count and then each item: `named_member_validate_body` is the Rust rule. `Rules::own` writes
  the type's checks last, `Rules::brand` a brand's own bounds and no walk, and `Rules::written` the
  method. A type with no statement writes `= emptyList()`.
- **The sentence is `kotlin_violation`'s**, in `src/bound_message.rs`, written from
  `Bound::stated()`. `kotlin_reports_each_bound_in_the_sentence_rust_reports_it_in` holds it to
  `rust_violation`, per bound.
- **The units.** A string's length is `codePointCount`, in Unicode code points, and a list's is
  `size`. `Number` compares a range in the Kotlin type the field is held as, and quotes a float
  back through `kotlinSchemaShown`, as Rust's `Display` writes one. `jvm_pattern` writes `$` as
  `\z`, `\v` as `\x0B` and a named group as a plain one, which is what the `regex` crate reads in
  the three; the message states the pattern as declared.
- **Every function a type calls is private to the type's own body** (`Helper`): `kotlinSchemaUnder`,
  `kotlinSchemaShown`, the `String.kotlinSchemaText()` a declared type is asked through, and the
  `Any?.validate()` a held type with no `validate()` answers with. The emitted types are joined
  into one file, where two file-scope declarations of one signature do not compile, and tixschema
  ships no runtime library.
- **A declared type is asked for its text, and the language picks.** A brand over a string or a
  path carries `kotlinSchemaText()` (`Rules::text_member`), and a brand over a brand or over an
  alias that answers one answers through it. A holder writes `held.kotlinSchemaText()`
  (`text_read`): a member is found ahead of the holder's own extension of `String`, so a brand
  declared above or below the holder and an alias of a string all build, and no expansion is
  refused. `NOTED` is read for four things alone: whether the declared type a brand holds was
  written above and answers its text, whether a held type is known to carry `validate()`,
  which spares its holder the fallback, and the two the read asks: whether the type has an
  unjudged serializer a holder can name, and whether it carries `kotlinSchemaBound()`.
- **A slot and a variant's member are named by the steps serde writes** (`Tagging`,
  `variant_body`, `slot_rules`): the variant's wire name where no tag is written, the content key
  under a tag and a content key, and nothing under a tag alone and under no tag; a slot's position
  where there are several, and nothing for a lone slot.
- **A map's key is `key_step`'s**: the key itself for a string, a number and a `Boolean`, the text
  its serializer writes for a declared type, and a type parameter's only where
  `serialized_parameters` finds `Serialize` declared for it, as Rust writes it.
- **One wrapper is written per tuple shape per item.** `tuple_wrapper_typename` queues a wrapper
  class the first time the item asks for a shape, and answers its name to every later ask of that
  shape (`KOTLIN_TUPLES`, cleared with the queue in `kotlin_module_tokens`). A shape is the
  class's body without its name: the type of each position and the serializer each is written
  and read with, so a walked and an unwalked read are two shapes where a position holds a model
  the walked one reads unjudged. The walk reads a position as `slot{N}` off the value, and names
  no wrapper.
- **A brand with a parameter** runs its bounds behind `if (value is {Default})`, the Kotlin type
  of the parameter's declared default (`declared_default`). A default written with arguments
  writes no bound: the JVM keeps no argument to tell the filling by.
- **The tests.** `tests/kotlin_tests/validate.rs` pins the text in every build with `kotlin`, a
  build with no schema surface included: Rust holds no bound there and Kotlin holds every one.
  `tests/service_schema_emitted_client_tests/run_kotlin_validate.rs` compiles the emitted types
  with `-Werror`, runs them, and holds each row to what Rust's `validate()` answers, or, for a
  slot and a variant's member, to the name written in the row. `src/features/kotlin/tests.rs`
  holds the README's block to what the emitter writes for the documents' parcel.

#### The Kotlin client's read

The serializer of a Kotlin type throws for a payload whose value breaks a bound or a check.
`src/features/kotlin/read.rs` writes it. `README.md`'s "The read in the Kotlin client" section is
the consumer's view; this is what a change has to keep.

- **A type has two serializers, a plain enum apart.** `{Type}Unjudged` reads and writes the type
  and judges nothing. `{Type}Serializer` (`judged`) is the type's own, named by
  `@Serializable(with = ...)`: it reads through the first and throws
  `SerializationException(validate().joinToString("; "))`. A type with parameters writes both as
  classes over one serializer per parameter.
- **The plugin's serializer is kept where the plugin takes the annotation.** A class, a unit
  struct's `object` and a brand's value class carry `@KeepGeneratedSerializer` (`kept`,
  `kept_inline`), and their `{Type}Unjudged` delegates to `generatedSerializer()` (`generated`).
  The serializer the emitter writes for a struct with a flattened member, a tuple struct and an
  enum is the unjudged one, under `written_head`.
- **A struct's member serde never writes is read and has no `put`.** `KotlinField::never_written`
  is `has_serde_skip_serializing`: `skip_serializing`, never `skip_serializing_if`. The plugin
  writes every property, so `struct_kotlin_tokens` hands a struct with such a member to
  `flatten_merging_serializer`, as it hands one with a flattened member, and `own_field_plan`
  answers no `serialize` statement for the member. A struct with neither keeps the plugin's
  serializer and its text. A member under `skip_serializing_if` is written wherever it holds a
  value: no client calls the Rust function, and the README states the limit.
- **Every flattened member is read at its place, in the order declared**, as serde reads it
  (`flatten_plan`): from `JsonObject(obj - setOf(...) - {member}Keys ...)`, the object without the
  keys the holder's own members take and those each member flattened ahead of it took, and from
  `obj` where nothing was taken. An optional one looks for its keys in that object alone
  (`flatten_struct_field_plan`). A struct takes the keys `flattened_struct_keys` answers, unless
  `preprocess::takes_no_key` answers for it: a member that names a function, and a type `HANDED`
  does not note as handing its object to none, so a type the emitter has not written takes none.
  `struct_kotlin_tokens` records in `KOTLIN_TAKEN_KEYS`, for every struct it writes, the keys
  `keys_taken_when_flattened` in `src/features/serde.rs` answers: the name and each alias of every
  member serde reads, one serde never writes among them, and neither the struct's own tag nor a
  member under `skip_deserializing`; none for a struct with a flattened member of its own, which
  serde reads as a map. The descriptor of a serializer the emitter writes declares no element,
  and the plugin's names the tag and a member serde never reads, and no alias. A struct that
  names a function on itself takes none, and is recorded with the keys its read reads, which an
  optional holder looks for. A struct the emitter has not written is looked for by the
  `descriptor.elementNames` of its serializer. A map is the one member of its object (section 4).
- **A flattened map's key is written and read through the key's own serializer**
  (`flatten_map_field_plan`), where the key's Kotlin type (`kotlin_typename`) is no `String`: it
  is written as `output.json.encodeToJsonElement(<key>, k).jsonPrimitive.content` and read over
  `entries.associate` through `input.json.decodeFromJsonElement(<key>, JsonPrimitive(k))`, the
  serializer `kotlin_serializer_expr` answers, so a key that serializer refuses refuses the entry
  and reads an optional map as `null`. A plain enum's key is the name serde writes, its `rename`
  and the enum's `rename_all` among them. A `String` key, a `char` and a date among them, is
  written and read as it was. A number or a `bool` key is read from its text, where serde reads
  none. `tests/kotlin_tests/flattened_models.rs` holds `Cathead`, `Hawsepipe` and `Snubbing` and
  serde's reads of `CHOCKED` and `SNUBBED`, and `run_kotlin_flattened.rs` holds the Kotlin read
  of each to serde's.
- **An optional flattened map is `null` where serde reads `None`** (`flatten_map_field_plan`):
  written through `value.{member}?.forEach`, so `null` writes no entry, and read inside
  `runCatching { ... }.getOrNull()`, so an entry that throws as the map's value reads the map as
  `null`, as serde's untagged read of an `Option` reads `None`. The decode of an entry throws
  `SerializationException`, `IndexOutOfBoundsException` for a list or an object where a text or
  a number is read, and an enum's read `IllegalStateException`, so the read catches whatever is
  thrown, as the untagged enum's `recoverCatching` chain does. A map that is not optional is
  read as it was.
  `tests/kotlin_tests/flattened_models.rs` holds `ProbeLocker` and serde's reads of `LOCKER`, and
  `run_kotlin_flattened.rs` holds the Kotlin read of each to serde's.
- **A flattened externally tagged enum takes one entry** (`flatten_enum_field_plan`): the first
  of that object, in its order, under a name in the `setOf(...)` written from
  `variant_names_when_flattened` in `src/features/serde.rs`, which Dart reads too. The holder
  hands the enum the object of that one entry, throws `SerializationException` where there is
  none and reads `null` for an optional enum, and every member after it is read from the object
  without `{member}Entry.key`. `enum_kotlin_tokens` records the names in `KOTLIN_VARIANT_NAMES`,
  and `flattened_variant_names` reads them: an enum under a tag key, an untagged one, one that
  names a function on itself or on its member, and one the emitter has not written are read as a
  struct is, and take no key. The enum's own read takes no variant under an `alias`, so an entry
  under one is taken and refused: the README states both limits.
- **A flattened enum's unit variant is written as its name over `null`** (`flattened_merge`), as
  serde writes it, where the enum writes a bare name. `enum_kotlin_tokens` records in
  `KOTLIN_BARE_NAMED` each externally tagged enum with a unit variant, whatever function it
  names, and `writes_a_bare_name` reads it: the holder merges what such an enum writes through
  `when (val written = ...)`, a `JsonPrimitive` as `put(written.content, JsonNull)` and an object
  entry by entry, optional and under a function too. Every other flattened member merges its
  object as it did, so an enum the emitter has not written throws on a unit variant: the README
  states the limit. `run_kotlin_flattened.rs` writes each variant of `Tacking` against serde.
- **An enum under a tag alone is written by hand as well.** The plugin refuses
  `@KeepGeneratedSerializer` on a sealed interface, so `dispatched_tagged_enum_kotlin_source`
  writes all three taggings, `Tagging::Unwrapped` being the tag alone there: the tag is taken off
  the object and the rest is read as the variant's own subclass, a lone slot under the `value` key
  that subclass writes. The subclass carries its tag in `@SerialName`, and a tag no variant has
  throws `SerializationException`.
- **A model is read unjudged exactly where `validate()` asks it, and nowhere else.** A model read
  unjudged where no `validate()` reaches it is a broken value handed back. `validate::asks_the_held`
  is the condition `Rules::member` walks under, asked of a scratch `Rules`, and
  `holds_an_unjudged` follows the walk of `held_statements`. `plugin_read` writes the annotation
  where the plugin reads, on the property and on the type inside a list, a nullable and a map,
  and `member_serializer` composes the serializer where the emitter writes the read. What a brand
  holds, a type argument and a member with a bound of its own on a brand are read by the type's
  own serializer.
- **A holder names only a serializer the emitter has written.** `reads_unjudged` answers for a
  type written above. A type declared below its holder, an alias and a type of the consumer's are
  read by their own serializer and judge themselves as they are read: an annotation names a class,
  and no fallback stands in for one. Rust reads a model declared below unjudged, so the two differ
  there, and `run_kotlin_read.rs` pins the Kotlin text.
- **A tuple a `validate()` walks reads its positions unjudged, by its walked wrapper wherever the
  emitter reads it.** `tuple_wrapper_typename` takes `walked`: `annotated_base` and
  `unjudged_serializer` ask for the wrapper of a walked tuple, at any depth of the member, and
  `kotlin_base` for every other. One wrapper is written per tuple shape per item, and a walked and
  an unwalked read are two shapes where a position holds a model, so a member is declared with the
  wrapper its read names. `plugin_read` declares what the plugin reads. Where the emitter writes
  the read through `member_serializer`, `read::declared_typename` declares the member: a member of
  a struct with a flattened member, a slot of a tuple struct and a member a serializer of its own
  reads (`read_whole`). The wrapper of a walked tuple declares each position with
  `read::position_typename`, the type `position_serializer` reads. A serializer that reads a tuple
  its holder has declared names the wrapper of that declaration (`read::declared_serializer`): the
  lone value of a tagged variant, a variant of several slots among them, is read by the type
  `variant_subclass` declared it with.
- **An untagged enum takes the first variant whose bounds hold.** `Rules::choosing` collects, per
  variant, each member's own bound and `kotlinSchemaBound()` of each brand it holds
  (`Asked::Bound`), and writes them into the variant's own `kotlinSchemaBound()`, which the chain
  of `untagged_enum_kotlin_source` asks after the variant decodes. `Rules::brand` writes the
  method on a brand with a bound. A check and what a held model reports choose nothing: the
  enum's own serializer refuses them after the choice, as serde's read does. `Helper::Unbounded`
  answers for a type the emitter has not written.
- **A name serde reads beside the one it writes is read too.** `aliases::written_on` reads the
  `#[serde(alias)]` names of a variant and of a field, and `KotlinField::aliases` holds a member's,
  none for a flattened member or one serde never reads. `dispatched_tagged_variant` writes each
  variant's arm under its written name and each alias, under every tagging. `aliases::keys_named`
  reads a member's alias as its wire name in the object the member is read from, and refuses two
  names of one member as serde refuses a duplicate field: in the `{Type}Unjudged` `kept_unjudged`
  writes for a class the plugin reads, a `JsonTransformingSerializer` over the plugin's, after the
  type's own functions and ahead of `preprocess::keyed`; in the object `flatten_merging_serializer`
  reads every member from; and in the object a tagged or an untagged enum hands a variant's class
  (`members_renamed`). A plain enum that names no function is read by
  `aliases::plain_enum_serializer`, a `{Type}Unjudged` written by hand that reads each variant under
  its name and each alias, and carries no opt-in. One that names a function keeps
  `read::kept_plain`, and `aliases::text_named` reads the alias after the functions. The write keeps
  the written name. A struct records in `KOTLIN_TAKEN_KEYS` the keys `keys_taken_when_flattened`
  answers, its aliases among them, which its descriptor's `elementNames` do not hold. `@JsonNames`
  is not used: it is experimental, and the build compiles under `-Werror` with no opt-in to it.
- **A serializer the emitter writes binds a parameter to `Any`** (`COMPOSED`), which
  `KSerializer<T>.nullable` requires, and one that delegates to the plugin's binds none. So
  `unjudged_serializer` composes a model written with a nullable argument through its own
  serializer.
- **The tests.** `src/features/kotlin/tests.rs` pins the text of each shape, a type written above
  and one written below in one test: the registries are thread-local, and a unit test has its
  own. `tests/kotlin_tests/read.rs` pins what the macro's own path writes, in every build with
  `kotlin`. `run_kotlin_validate.rs` reads each row with `{Type}Unjudged` and, in a run of its
  own, with the type's serializer, held to what `validate()` answered.
  `run_kotlin_read.rs` holds the README's rows to what the Rust read answers of the same payload,
  and both clients to their fault. `tests/kotlin_tests/flattened_models.rs` holds the models of a
  member flattened after a struct and beside a member of its holder's own, a struct under its own
  tag and one with a member serde never reads among them, and of an externally tagged enum
  flattened beside other keys, `tests/kotlin_tests/tests.rs` pins their reads, and
  `run_kotlin_flattened.rs` reads each under the toolchain, against serde, and writes back the
  structs, a variant's entry under an alias among them (`UNDER_AN_ALIAS`).
  `tests/kotlin_tests/aliased_models.rs` holds the models of a variant and a member under an
  alias, under each tagging, in a plain enum, a struct the plugin reads, one read by hand and one
  flattened, and pins what serde reads of each payload, which the Dart client is held to as well.
  `tests/kotlin_tests/aliased.rs` pins their Kotlin text, and `run_kotlin_aliased.rs` reads and
  writes back each payload under the toolchain, against serde.

#### The Kotlin client's `preprocess`

The Kotlin read calls each function a `preprocess` names, under the name the attribute writes:
`fun trimmed(payload: JsonElement): JsonElement`, on a field and on a type alike.
`src/features/kotlin/preprocess.rs` writes each call, inside the unjudged read, so both
serializers of a type make it. `README.md`'s "`preprocess` in the Kotlin client" section is the
consumer's view; this is what a change has to keep. The key is a last resort, and the documents
say so.

- **The emitter is handed the type's functions.** `KotlinArgs::preprocess` is the type's own,
  from `kotlin_suffix_tokens`, and a member's are in the meta `KotlinArgs::member_meta` answers
  (`preprocess::of`). `preprocess::corrected` is the one writer of a call: each function over the
  answer of the one before, in the order written.
- **The order of one read is Rust's**: the type's functions on the whole payload, each member's
  on the member's own, the value, then `validate()` in the type's own serializer. Nothing is
  called on a write.
- **A type's functions are in `{Type}Unjudged`.** Where the plugin writes the serializer,
  `kept_unjudged` hands `read::transforming` the correction: a `JsonTransformingSerializer` over
  `generatedSerializer()`, in place of the delegation `read::generated` writes. A serializer the
  emitter writes calls the functions on `input.decodeJsonElement()`: the flatten-merging one, a
  tuple struct's, a tagged enum's, and an untagged enum's ahead of the choice of a variant. A
  type that names no function is written token for token as before.
- **A brand with a parameter is read through `read::transforming_inline`.** kotlinc hands a
  value class with a parameter unboxed to the `serialize` a `JsonTransformingSerializer` inherits,
  and the type's own serializer then throws `ClassCastException` on a write. So that one keeps
  the delegation and overrides `deserialize`.
- **A plain enum that names a function carries `{Type}Unjudged` as its one serializer**
  (`read::kept_plain`). `note_plain_enum` still says no holder names it: a holder reads the enum
  by its own serializer, which is that one.
- **A member the plugin reads is read through a serializer of its own.** `Plugin::declared`
  queues `object {Owner}{Member}Preprocess : JsonTransformingSerializer<T>(...)`
  (`read::corrected_member`) and answers the annotation that names it. It reads through
  `read::read_whole`: `member_serializer`, the serializer the member has without the key, so a
  model is still judged once. A tuple is asked for its name once there. `kotlin_module_tokens`
  writes the queue after the type's source (`preprocess::serializers`).
- **A member that reaches a type parameter is corrected at its key** (`Plugin::keyed`,
  `preprocess::keyed`). The plugin takes a serializer on a property only where its constructor
  takes no parameter, or one per argument of the property's own type, so none can be handed the
  parameter's serializer. `{Type}Unjudged` corrects such a member of a class, and the enum's
  serializer the object it hands a variant's class (`variant_keyed`). The lone slot of a variant
  under a tag alone is corrected the same way, under the `value` key its class reads.
- **A member the emitter reads is corrected in place**: `own_field_plan`, a slot of a tuple
  struct, a slot of a variant read by the enum's serializer, a slot of an untagged variant as the
  variant is tried, and a position of a tuple's wrapper (`tuple_wrapper_typename`).
- **No function is called for a key the payload does not hold.** The plugin asks a property's
  serializer only for a key that is there, and `own_field_plan` corrects inside `?.let`. A `null`
  is handed to the function: the serializer of a nullable member is a
  `JsonTransformingSerializer<T?>`.
- **A flattened member a function reads is handed what serde hands it**, as every flattened
  member is ("The Kotlin client's read", above), and takes none of its keys
  (`preprocess::takes_no_key`). `HANDED` notes, per type the emitter wrote, whether it hands its
  object to a function, its own or one of a member it flattens: a type known to hand it to none
  takes its own keys.
- **A flattened map is read from every key of the object** (`flatten_map_field_plan`): it is
  the one member of its object (section 4), and the enum's serializer takes the tag off the
  object before it hands it a variant's class. A function the map names is handed the whole
  object. An optional map is read from what the function answers as the Rust read of a
  corrected payload reads it: `null` where it answers `JsonNull`, and refused where an entry
  does not read as the map's value (`CorrectedLocker`, `EmptiedLocker`).
- **The path of a refusal below a function starts at the value the function answered**: a
  member and a type that name one are each read from a JSON value of their own. A list or an
  object where a text or a number is read is thrown by `kotlinx.serialization` 1.11.0 as
  `IndexOutOfBoundsException`, as it is by every read the emitter writes itself.
  `run_kotlin_preprocess.rs` pins both, and that each client answers its fault for the second.
- **The tests.** `src/features/kotlin/tests/preprocess.rs` pins the text of each shape, and the
  README's block to what the emitter writes for the documents' parcel.
  `tests/kotlin_tests/preprocess.rs` pins what the macro's own path writes, in every build with
  `kotlin`. `tests/service_schema_emitted_client_tests/run_kotlin_preprocess.rs` compiles the
  emitted types with `-Werror`, runs them, and holds each row to what the Rust read prints of
  the same payload: the README's sample, every position a type is held in, every shape, and
  every place a member is read. Rows whose words are Kotlin's own are pinned beside them.

#### A read refuses what `validate()` refuses

A type's read is its own unjudged read, followed by its `validate()`. A holder reads the models it
holds unjudged, so the whole value is judged once, by the type the caller asked for. `README.md`'s
"A read refuses what `validate()` refuses" section is the consumer's view; this is what a change
has to keep.

- `src/model_schema/judged_read.rs` is the seam. `of_struct` and `of_enum` are handed the item and
  whether it publishes `validate()`, from `struct_output_with_unit_impls`, `branded_read`,
  `enum_output_with_decode` and `process_plain_enum`. An item that derives no `Deserialize`, or
  publishes no `validate()` and names no `preprocess` ("A payload corrected before it is read",
  below), gains nothing and is emitted as it was written, token for token.
- serde's derive runs on a copy of the item, `ModelSchemaRemote`, declared inside
  `model_schema_read_unjudged` under `#[serde(remote = "Type")]` (`remote_definition`). `remote`
  is not written on the item itself: the derive would then write inherent `deserialize` and
  `serialize` functions on the type, where `Type::deserialize(..)` would answer the unjudged read
  and clippy's `same_name_method` fires inside the derive's output in the consumer's crate.
  `Deserialize` is taken off the item's derive list (`leave_the_read_to_the_macro`), and the
  item's serde attributes with it where it derives no `Serialize` either. `Serialize` stays the
  derive's, untouched.
- The copy keeps the item's `serde` and `cfg` attributes and no other (`kept_by_the_copy`), is
  written with the macro's own spans (`generated`), and names the type wherever the item writes
  `Self` (`naming_the_type`).
- The written `impl Deserialize` and the function carry the bounds serde's derive writes, which
  `read_bounds` in `src/features/serde/read_bounds.rs` reads off the item: a container, a field
  and a variant `bound`; `Self: Default` for a container `default`; `Deserialize<'de>` for each
  parameter a read field names, none for `PhantomData`, a skipped field, a field or a variant
  read through `with` or `deserialize_with`, and a borrowing `Cow`; `Default` for a bare
  `default` and for a skipped field nothing names a function for; an associated type by its
  path; and the lifetimes `'de` outlives, `'static` among them reading the type at `'static`. A
  bound wider than the derive's refuses a type that builds without it. The copy is given the
  bounds as an explicit `bound(deserialize = ...)` wherever the item's container writes none,
  the derive holding the copy, and not the type, to `Default` for a container `default`.
- A holder's hook is recorded as its members are walked (`hook_member`, from `member_read_hook`
  for a field and a slot and from `read_untagged_member_unjudged`), and written when the item's
  read is: `MEMBER_HOOKS` holds each as text, a token outliving no expansion, and
  `exec_model_schema` clears it. `reached` reads the member's type: a declared type itself, or
  under `Option`, `Vec`, `Box` and a map's value, in any nesting. A hook is recorded only where
  `validate()` walks the same wrappers (`held_shape`), which is what judges what the hook leaves
  unjudged, and never for a member serde does not read, one read through a hook of its
  author's, or a type that names a lifetime.
- A member written with `Self` or with the item's own type parameters (`Vec<Entry<IdType>>`) is
  hooked as any other: a model filled with the holder's parameter is judged once, with its
  holder, and what fills a parameter itself (`Vec<Held>`) is no model `validate()` walks.
  `HookScope::written` writes `Self` as the item (`naming_the_type`) and makes the hook generic
  over exactly the item's parameters the member's type names, in the order declared, each
  inferred at serde's call from the return type. Every bound sits in its `where` clause, one
  predicate per bounded type and each bound once (`merged`): the item's own bounds and the
  read's predicates (`ReadBounds::predicates`) whose parameters are all the hook's. The type the
  model is read as declares only the parameters the model names, under the item's own bounds on
  them. A bound that ties such a parameter to another of the item's or to one of its lifetimes,
  a read at `'static`, a const parameter, and a parameter named behind a path
  (`IdType::Out`, `<IdType as Trait>`) leave the member to its own `Deserialize`: the hook is
  not written, and `unjudged_hooks` hangs nothing on the member.
- The hooks are functions local to `model_schema_read_unjudged`: `read_unjudged_{stem}`, each
  with a local `Unjudged` newtype whose `Deserialize` calls the leaf's
  `model_schema_read_unjudged`, behind the local `ReadUnjudged` trait. An inherent function is
  found ahead of a trait's, so a type with no unjudged read of its own is read as serde reads
  it. A refusal raised below the member, by a type that judges itself as it is read, is answered
  under the name `validate()` reports the field under (`named_refusal`) where `named_below` says
  the named reader did so, and an outermost `Option` under a key is given `#[serde(default)]` by
  the rule of `needs_injected_default`, the container writing none.
- A brand takes the same read. `branded_read` hands `of_struct` every brand, and one that
  publishes `validate()` (a bound, a count of a list, a check) or names a `preprocess` gains
  `model_schema_read_unjudged` and the written `Deserialize`. No hook is hung on a brand's slot,
  and a brand with none of the four is emitted as it was written. A holder's hook therefore
  reads a brand unjudged, and the holder's `validate()` names it: `'listed.1': too short: ...`.
- `BrandBound` holds what a brand's own bound adds: the validator in the schema module, the call
  `validate()` makes, and `model_schema_validate_bound`, a method of the brand's own that
  answers for the bound alone (`judged_read::BOUND_ALONE`). It has two readers. A member of an
  untagged enum is held to it as the member is read (`HookPlace::choosing`,
  `MemberHook::held_leaf`), behind the local `ChosenByShape` trait, which answers `Ok(())` for a
  reference to any type with no such method: the bound of a brand chooses the variant, as it did
  where the brand refused in a read of its own, and nothing of a model chooses one. And the read
  a recovering decode lists a bounded brand from holds the value to it (`bounded_listed_read`,
  through `brand_recovering_decode`), so the walker lists a broken bound as the refusal of the
  value. A check a brand names chooses no variant: the enum's `validate()` refuses with it.
- `named_read_hook` stays for a named field `reachable_nested_shape` walks and `reached` does
  not, a declared type in an `Arc` and in a fixed array among them. A model or a brand there,
  in a tuple and filling a type parameter judges itself as it is read, and ends the read at the
  first one that breaks a rule.
- A unit struct's `Deserialize` is the macro's already (`unit_struct_serde_impls`), and
  `unit_struct_read` gives it `model_schema_read_unjudged` and the `validate()` call where the
  struct has a check.
- `remote`, `from`, `try_from` and `into` (`serde_conversion`) on an item whose read the macro
  writes are refused by `conversion_refusal`, in every build.
- `model_schema_read_unjudged` is the type's own name. Each name the function declares beside
  the item's own types is numbered where the item writes it (`unclaimed_parameter`): the
  reader's type parameter `D`, the copy `ModelSchemaRemote`, the traits `ReadUnjudged` and
  `ChosenByShape`, and the `Unjudged` of a hook where the member's type writes that name.
- A service's dispatcher reads a request through `unjudged_{operation}`, which
  `message_readers` in `src/service_schema/support.rs` writes into the service module, and calls
  the validator next. The AMQP and WebSocket arm hands it a `serde_json::Deserializer` over the
  payload and ends it, as `from_slice` does, and the HTTP dispatcher the value it carried.
- `tests/validation_tests/refusing_read.rs` holds each shape, the bounds and the README's
  sample; `tests/branded_newtype_tests/read.rs` a brand under each inner, held in each place and
  in an untagged enum; `tests/check_tests/read.rs` the read of a type that names a check, in
  every build; `tests/recovering_decode_bson_tests/refused_read.rs` a BSON value under each
  major; `tests/judged_once_tests/` a model held under the holder's own parameter or `Self`, in
  each wrapper and beside each bound, its checks counted on the thread that runs them.

#### A payload corrected before it is read

A `preprocess = ["name", ...]` names functions of the consumer's own, on a field and, in
`#[model_schema(...)]`, on a type: each takes a payload and answers the payload the value is read
from. Rust calls the snake case of the name, `fn trimmed(payload: serde_json::Value) ->
serde_json::Value`, the Zod schema the name as written, `function trimmed(payload: unknown):
unknown`, and each client's read the name as written: `dynamic trimmed(dynamic payload)` in Dart
("A `preprocess` in the Dart read", below), `func trimmed(_ payload: Any) -> Any` in Swift ("The
Swift `preprocess`") and `fun trimmed(payload: JsonElement): JsonElement` in Kotlin ("The Kotlin
client's `preprocess`"). The documents call the key a last resort, and a change
keeps them saying so: a function runs on every read, on a payload no schema describes. `README.md`'s "Preprocessing" section is
the consumer's view; this is what a change has to keep.

- **One reader, the one of `check`.** `function_names` reads the key for both attributes, in
  every build, with the five refusals of a `check` under the key's own words
  (`PREPROCESS_TAKES`). `apply_arg` and `parse_prop_key` both call it, and the key is in
  `KNOWN_ARGS` and `KNOWN_KEYS`. `preprocess::alias_refusal_output` refuses it on a type alias at
  `process_item`, and `branded_slot_prop_errors` on a brand's slot: a brand takes it on the type.
- **`src/model_schema/preprocess.rs` holds what the item names.** `record`, called from
  `record_item_functions` at `exec_model_schema`, writes the type's functions and each member's,
  by variant and index, and `named()` is what the read and the recovering decode read. An item
  that derives no `Deserialize` records nothing, and a member serde never reads neither: nothing
  is called for either, and nothing is refused.
- **Every call is written beside the type**, by `preprocess::corrected`: the snake case of the
  name as an ident located at the string literal (`checks::located`), each call over the answer
  of the one before, so a missing function is E0425 at the attribute. No call is written into
  `{type}_schema`.
- **The read is the judged read's.** `judged` in `judged_read.rs` answers for an item that
  publishes `validate()` or names a function, and `written_read` calls `validate()` only where
  one is published. Where the type names functions of its own, `unjudged_read` writes
  `model_schema_read_unjudged` as the payload read into a `serde_json::Value`, the calls, and
  `model_schema_read_corrected` over what they answer, which is the copy's read; the refusal of
  a corrected payload is told through `told_by`. The order of one read is the type's functions,
  each member's as it is read, the value, then `validate()`.
- **A member's functions are called by a function local to the read, hung on the copy.**
  `hooked` writes one per member, of three kinds. Alone, `read_corrected_{stem}`, generic over
  what it reads. Where the member already has `read_unjudged_{stem}`, that one function corrects
  the payload first (`MemberHook::function`). Where the copy keeps a `deserialize_with` or a
  `with` of tixschema's own, an untagged member's bound hook and the one `as_number` injects,
  `corrected_hook` reads the corrected payload through that function and takes the attribute's
  place on the copy (`HungOn::replaces`): under the type the member is written with, or, for a
  counted list, under the header `list_hook_header` shares with `list_deserializer`. A key the
  payload does not hold is serde's to answer, and no function is called for it, where the Zod
  schema calls the field's function with `undefined`: the documents state both, and
  `type_check_a_function_is_handed_under_zod_what_a_rust_read_does_not_hand_it` holds them.
- **Every name a read declares is its own.** `Corrects` holds `payload`, `corrected` and the
  reader's parameter, each numbered where a function is called under it, and both outer
  functions take the reader under that name: a hook nested in them is a `fn` item, which cannot
  name a parameter of the function around it.
- **The refusals are raised at `exec_model_schema`, in every build.** `preprocess::refusals`,
  through `preprocess_refusal_output`: a member read through a `deserialize_with` or a `with` of
  its author's (`read_hook`), a member that borrows, and a type that holds one
  (`read_bounds::borrows`). The item is emitted stripped of its member keys
  (`without_member_keys`). `conversion_refusal` refuses `remote`, `from`, `try_from` and `into`
  in the words of a type that names a `preprocess`.
- **A unit struct and a brand.** `unit_struct_correction` writes the two functions of a unit
  struct around the visitor `unit_struct_read` already writes, and `branded_read` gives a brand
  the judged read where it names a function, as where it publishes `validate()`.
- **The recovering decode corrects once.** `Walker::corrects_own` is read off `named().own`. An
  entry point calls `decode_with_{source}_corrected` on the value as it takes it, reads through
  `decode_with_read_corrected` and walks through `decode_with_{source}_corrected_issues`,
  `_named` and `_fields`; the three public names stay, for a holder, each correcting a copy
  first (`corrected_entries`). Under BSON the value crosses through `bson::Deserializer::new` and
  back through `bson::Serializer::new`, and a value that cannot cross is one `Undescribed` with
  no callback. A member that names functions is a leaf read whole through
  `preprocess::corrected_reader`, with nothing written back (`Whole::corrected`), so its
  function is called by the read and by the listing.
  `a_type_that_names_a_preprocess_adds_the_walk_of_a_corrected_value` pins the names in order.
- **One writer of the Zod wrap.** `zod_preprocess_wrap` in `src/field_type.rs` writes a field's
  and a type's, the first name outermost. `zod_published_binding` wraps what `zod_check_wrap`
  answers (`PublishedBinding::preprocess`), inside the `.meta` and inside a generic's builder;
  `branded_zod_expression` wraps ahead of `.brand()`, and
  `generate_plain_enum_zod_schema_method` ahead of the `.meta` a plain enum writes itself.
- **A holder reads the binding.** `record_zod_binding_preprocess` notes which bindings call a
  function (`ZOD_PREPROCESSED`), an alias of one included, and `flattened_zod_branches` and
  `zod_member_leaves` write no member of such a type in its place: a flattened choice that names
  one is joined by its binding, the one schema that calls the function.
- **The tests.** `src/model_schema/preprocess/tests.rs` holds the refusals and the emitted read;
  `tests/pattern_preprocess_tests/` the models, the reads of every shape (`read.rs`), the chained
  hooks (`chained.rs`), the Zod text (`zod.rs`) and the README (`readme.rs`);
  `tests/recovering_decode_tests/preprocessed.rs` and
  `tests/recovering_decode_bson_tests/preprocessed.rs` the recovering decode, the second under
  each `bson` major; `tests/service_schema_dispatch_tests/corrected_request_service.rs` a
  dispatched request. `tests/generic_types_tests/declarations.rs` compiles the models in a
  bundle of their own (`corrections`) and holds what node prints to what the Rust read prints,
  from one table (`SAMPLE`, `POSITIONS`).

`validate()` is written in the type's own `impl`, beside the schema delegates
(`assemble_schema_output`, `assemble_branded_output`), so a type that declares a parameter has it
at every filling. `README.md`'s "A type that declares a parameter" section is the consumer's view;
this is what a change has to keep.

- A holder's `value.validate()` finds the method whatever fills the type it holds. The block-local
  fallback (`unpublished_validate_fallback`) answers only for a type that publishes none, and it
  answers `Ok(())` without a word: a `validate()` moved to an `impl` of one filling, or under a
  bound the holder's parameter lacks, is a check lost in every generic holder.
- A struct's and an enum's body builds under the type's own bounds: a bound is written on a
  string, a number, a list counted whole or a named type, and one on a bare parameter is refused.
- A generic brand's string checks are its declared default's. Its `validate()` hands the value
  to `validate_at_declared_default`, which `embedded_validate_at_declared_default` emits beside
  `type_identity`: where the two types are one it reads the value as the declared default
  through a pointer cast and runs `validate_value`, and at any other filling it answers `Ok(())`,
  so nothing asks `Display` of the parameter. `TypeId` does not tell two lifetimes apart, so the
  function declares `'held` for its borrow and `at_one_lifetime` writes the default with every
  lifetime it writes replaced by that one: the value outlives the borrow, and is never read at
  a lifetime longer than its own. It answers the result and no reference. A brand over a list
  counts its items at every filling and takes no such function.
- The macro adds no bound to a brand's declaration, and a brand's read asks of a parameter what
  serde's derive asks (`read_bounds`). Nothing asks `Display` of a parameter, on the read or in
  `validate()`.
- A consumer's own `validate` on one filling is a duplicate definition (E0592).
  `struct_validate_method`'s doc comment holds the run.

#### `validate()` on the Dart types

Every type `src/features/dart.rs` writes has `List<String> validate()`, which runs what the Rust
`validate()` of the type runs, in its order. `README.md`'s "`validate()` in Dart" section is the
consumer's view; this is what a change has to keep.

- **The dispatch is handed what it reads of the attribute.** `dart_suffix_tokens` builds a
  `DartTypeArgs`: the name, the type's `check`, a brand's or an enum's `minLength`, `maxLength` and
  `pattern`, `default_types`, and `member_meta`, which is `member_prop_meta` with the enum's
  arguments for a member of an enum. `ModelSchemaArgs` stays private to `model_schema.rs`. Every
  member's `FieldDef` takes its meta through `field_def_with_prop_meta`, so an enum's string check
  reaches in Dart the fields `written_as_text` says it reaches.
- **One sentence source.** `dart_violation` in `src/bound_message.rs` writes the Dart string from
  `Bound::stated()`, as `rust_violation` and `zod_error_arg` do, and
  `the_three_renderings_of_a_bound_hold_one_sentence` holds the three to it. `dart_escaped` writes
  a text between two quotes of a Dart string, which reads `$` as an interpolation.
- **A member** is `member_elements`: its own bounds (`bound_elements`), then what the value it
  holds reports where the member writes no bound or is a list (`held_elements`), then its checks
  (`check_elements`). That is the order `named_member_validate_body` and `then_checks` give Rust.
  A type's own checks come last (`own_check_elements`).
- **A check is called under the name the attribute writes**, a function of the consumer's that
  answers its messages as a list: `List<String> withoutDuplicates(List<String> value)` for a
  field's, which takes the field's own value and names no place,
  `List<String> withinParcel(Parcel value)` for a type's, which is handed `this` and leads each
  message with a place, and `List<String> withACode<CodeType>(Crate<CodeType> value)` for a type
  with a parameter, written once. A missing function is the Dart compiler's to report.
- **A length** is `runes.length` on a string, its Unicode code points, and `length` on a list. A
  member typed with a named type is measured on `_boundText(...)`, and a `double` is quoted back
  through `_number`, a whole one without `.0`. `_under` writes a place in front of a message, as
  `nested_under` does in Rust. `validate_members` writes each helper into the type that calls it
  and into no other: two emitted types share nothing.
- **What a value holds** is walked over its `FieldDef`: an optional (`present`, an
  `if (x case final held?)`), each level of a list and of a set (`item_elements`, the index a
  step of the place), a map's values (the key's text from `dart_map_key_text`), a tuple's
  positions (`slot_elements`) and a declared type, whose `validate()` is called with no knowledge
  of what it declares. A type parameter is not reached. A binder is named by its depth
  (`binder`): `held`, `index` and `entry`, then `held1`.
- **A place is the steps serde writes on the way to the value** (`Step`): a key, a position in an
  array, or nothing. `tagged_variant_steps` answers a variant's: the content key, nothing beside a
  tag, and otherwise the name the variant is written under. A tuple struct that declares one slot,
  an untagged variant and a `#[serde(flatten)]` member add no step, and a slot of several is its
  position on the wire.
- **A brand** (`brand_validate`) runs the type's string check on `value` under no place, then its
  checks, and no `validate()` of what it wraps. Over a bare parameter it measures the declared
  default (`declared_default`) behind `if (value case final String held)`. An alias runs its
  target's `validate()` where the target is a declared type written as itself.
- **A sealed enum** declares `List<String> validate();` on its base (`VALIDATE_DECLARED`), and
  `variant_validate` writes each variant's override, the enum's checks last, each handed `this`.
  A plain enum runs its checks.
- **`validate` is a taken name** in `DART_FIELD_NAMES_TAKEN` and `DART_ENUM_NAMES_TAKEN`.
- **`toJson` is untouched.** `validate()` is the last member of each class, after its codec.
  `the_codec_of_each_type_is_the_text_it_was` in `tests/dart_tests/validate.rs` holds every piece
  of the codec's text, in its order: the two reads, and `toJson` as it was before any type had
  `validate()`.
- **The tests.** `tests/dart_tests/validate.rs` pins the text of each shape and calls no Rust
  `validate()`, so it holds in a build with no schema surface. `tests/dart_tests/bounded_brands.rs`
  pins a brand's string check, which such a build refuses.
  `tests/service_schema_emitted_client_tests/run_dart_validate.rs` runs the emitted source under
  the Dart SDK: a value under a named field, in a list, in a map and in an optional is compared
  with what Rust's `validate()` answers of it, a slot and a variant's member are written out, and
  the README's section is held to the text. `dart analyze` passes every shape whose codec it
  passed before. Each value is read with `fromJsonUnjudged`: `fromJson` refuses a broken one.

#### The Dart read refuses what `validate()` refuses

`fromJson` of every type `src/features/dart.rs` writes is its own unjudged read followed by its
`validate()`, and throws a `FormatException` whose message is the list joined by `; `, as the Rust
read joins it. `README.md`'s "The Dart read refuses what `validate()` refuses" section is the
consumer's view; this is what a change has to keep.

- **One writer of the pair.** `read_pair` writes `fromJson` over `fromJsonUnjudged` for a class, a
  wrapper (a brand, a tuple struct, an alias), a variant with named members and the base of a
  sealed enum. `judged_body` writes the judgement, which the static `fromJson` of a plain enum
  takes too. A generic type takes its converters in both reads.
- **A declared type is read unjudged exactly where the holder's `validate()` runs it.** `Read` is
  threaded through `dart_decode_expr` and `dart_decode_at`, and `Read::of_member` asks
  `walks_what_it_holds`, the question `member_elements` asks: a member's own bound stands in for
  what its type declares, a list's count apart. A value read unjudged where no `validate()`
  walks is a rule no read runs, so each of these is read judged and ends the read with its own
  messages: a converter handed for a type argument, the key of a map (`dart_map_key_decode`),
  what a brand wraps, an alias of anything but a declared type itself, and a member under a
  bound of its own. A flattened member is read unjudged whatever it writes, as `field_elements`
  walks it.
- **`dart_json_decode` stays judged**, so the clients' text is what it was: a reply is read
  through `fromJson` inside the `try` each client already writes. The HTTP client answers
  `undeserializablePayload` and the WebSocket client `failedValidation`, the fault each answers
  for a reply it cannot read.
- **A tagged enum** dispatches in the base's unjudged read: to `fromJsonUnjudged` of a variant
  with named members, and to the constructor of a variant that holds a value or none, its
  payload read as a member is.
- **An untagged enum takes the variant serde takes.** `variant_bounds` writes `_bounds()` ahead
  of a variant's `validate()`, and the attempt takes the variant where it answers nothing.
  `Asked` is what one walk is asked: `Rules` for `validate()`, and `Bound` for the member's own
  bounds under no place, then `_heldBound(...)` of each declared type the walk reaches, with no
  check. `_heldBound` calls `validateBound()` through `dynamic` and answers nothing for a value
  with no such method. `brand_validate` writes that method on a brand that carries a bound: the
  bound alone. So a brand's bound chooses a variant whether the brand is declared above the
  enum, below it or in another file, and nothing a check or a model reports does. A variant
  with nothing to ask keeps `try { return ...; } catch (_) {}`.
- **A plain enum** keeps its `fromJson` as it was where it names no check, and judges where it
  names one. Both carry `fromJsonUnjudged`, the name a holder reads every declared type by.
- **`validate()` stays the last member of a class**: `validateBound()` and `_bounds()` are
  written ahead of it.
- **A member serde never writes is read and has no entry in `toJson`.** `DartField::never_written`
  is `has_serde_skip_serializing`: `skip_serializing`, never `skip_serializing_if`.
  `class_body_parts` writes no entry for it, a variant with named members sharing it. A member
  under `skip_serializing_if` is written wherever it holds a value: no client calls the Rust
  function, and the README states the limit.
- **Every flattened member is read at its place, in the order declared**, as serde reads it
  (`constructor_arguments`): from the object without the tag, the keys the holder's own members
  take and the entries each member flattened ahead of it took. That is what a struct with a
  flattened member of its own and a function a member names are handed, and a struct that reads
  a key a struct ahead of it took is refused, as serde refuses it. A map is the one member of its
  object (section 4), and is handed every key but the tag of a variant under a tag key. What a
  member takes is `taken_by`, the `Taken` recorded for its type in `DART_TAKEN_KEYS`, and nothing
  for a member that names a function:
  - `Taken::Keys` of a struct, which `struct_dart_tokens` records from
    `keys_taken_when_flattened` in `src/features/serde.rs`, which the Swift and Kotlin emitters
    read too: serde's own list of the name and each alias of every member it reads. `without`
    removes them.
  - `Taken::FirstOf` of an externally tagged enum, which `enum_dart_tokens` records from
    `variant_names_when_flattened` in `src/features/serde.rs`: the name each variant serde reads
    is read under and each alias, serde's `VARIANTS`. serde's
    `FlatMapDeserializer::deserialize_enum` takes the first entry, in the object's order, under one
    of them, wherever it is. `first_of` hands the enum an object of that one entry, and of none
    where no entry is, and `without_first` removes it from the object a `jsonDecode` answers in
    the order written, for the members after the enum: each a `switch` of one arm whose variable,
    `whole` or `rest`, calls no function. The entry depends on what is left to the enum, so the
    keys taken ahead of it are removed first and the keys taken after it from what
    `without_first` answers (`source`, `removed`); a holder with no flattened enum is written as
    it was.
  - Nothing, for an enum under a tag key, an untagged one and one that names a function on
    itself. serde reads an enum under a tag alone and an untagged one as a map, which takes no
    key; it reads one under a tag and a content key as a struct of the two, which takes both,
    and the README states the limit.

  A struct that names a `preprocess` on itself is read by serde as a value, and one with a
  flattened member of its own as a map: each records none. A type the emitter has not written
  takes none, so a member after a struct declared below its holder is handed the struct's keys,
  and the README states the limit. `toJson` writes each flattened member at its place: a key two
  of them hold is written once, with the last value, where serde writes it twice.
- **A name serde reads beside the one it writes is read too.** `read_aliases` reads the
  `#[serde(alias)]` names of a field and of a variant, none for one serde never reads, and
  `DartField::aliases` holds a member's. A tagged enum takes a variant under its written name or
  any alias (`any_of`, `case 'Berth' || 'Pier':`), under every tagging, and an externally tagged
  variant read under an alias too reads its value from the entry it took, `map[tag]`
  (`VariantClass::aliased`); a plain enum's `switch` takes each alias the same way. Where a member
  of a class has an alias, `class_reads` reads the members from `named`, the object copied with
  each alias moved to the member's wire name in its place (`named_by_aliases`), and refuses a
  member under two of its names with `FormatException('duplicate field ...')`, as serde refuses
  it: a member flattened after them is handed `named` without their keys. A type that names no
  alias is written as it was, and `toJson` writes the written name.
- **`fromJsonUnjudged` is a taken name of a plain enum** (`DART_ENUM_NAMES_TAKEN`), which holds
  it as a static function. It is not in `DART_FIELD_NAMES_TAKEN`: Dart takes a field and a
  constructor under one name, so such a field is emitted as it was.
- **The tests.** `tests/dart_tests/read.rs` pins the text of each read, and
  `src/features/dart/tests.rs` that of a field named `fromJsonUnjudged`, which no integration
  test can declare, the name being no snake case.
  `tests/service_schema_emitted_client_tests/run_dart_read.rs` runs the reads under the Dart
  SDK: the sample beside what the Rust read answers, every case of `run_dart_validate.rs` read
  with `fromJson` and held to what its `validate()` answers, an untagged enum beside the variant
  serde takes, a reply that breaks a rule through both clients, and the README's section.
  `tests/dart_tests/flattened.rs` pins the read of a flattened member at each place, over the
  models of `tests/dart_tests/flattened_models.rs`, which also pin what serde reads of each, and
  `run_dart_flattened.rs` reads and writes each back under the Dart SDK, against serde.
  `tests/dart_tests/aliased.rs` pins the read of a variant and a member under an alias, over the
  models of `tests/kotlin_tests/aliased_models.rs` and of
  `tests/dart_tests/flattened_aliased_models.rs`, which flattens each tagged enum of the first
  beside a member and pins what serde reads of it, and `run_dart_aliased.rs` reads and writes
  each payload back under the Dart SDK, against serde. A unit variant's entry over `null` is the
  one payload it holds apart (`UNIT_ENTRY`): the Dart read of an enum has no case for that entry,
  under any of the variant's names.

#### A `preprocess` in the Dart read

`fromJsonUnjudged` of every type `src/features/dart.rs` writes calls the functions the type and
its members name, under the names the attribute writes, each a function of the consumer's own:
`dynamic trimmed(dynamic payload)`. `fromJson` calls that read, so both correct the payload, and
`validate()` judges what the functions answered. The order of one read is Rust's: the type's
functions, each member's as it is read, the value, then `validate()`. `README.md`'s "A
`preprocess` in the Dart client" section is the consumer's view, which calls the key a last
resort; this is what a change has to keep.

- **The dispatch is handed the type's functions** in `DartTypeArgs::preprocess`, and a member's
  are in the meta `member_meta` answers. A type that names none, on itself or on a member, is
  written as it was, a holder of a flattened member apart (below).
- **A type's functions are called by `unjudged_read`**, the one writer of the body of every
  unjudged read: a class, a wrapper (a brand, a tuple struct), the base of a sealed enum and a
  plain enum. Where the type names functions the read takes its payload as `payload` and holds
  what they answer as `json`, cast as the read types it (`final json = withoutFragile(payload)
  as Map<String, dynamic>;`), and a read of no member holds it under no name. `read_pair` writes
  `fromJson` as it was: it calls no function. A sealed enum calls them in its base alone
  (`Corrects::of_a_variant`), and a plain enum that names one and no check delegates `fromJson`
  to `fromJsonUnjudged`.
- **A member's functions are called by `member_decode`**, around the payload its own read is
  written over: `constructor_arguments` for a named member, the tuple arm of `dart_decode_at` for
  a slot, and the arm of a variant that holds one value. `corrected` folds the calls, the first
  name innermost, so it is the first one called.
- **No function is called for a key the payload does not hold.** A named member and a slot
  under a content key are read from `json.containsKey('key') ? f(json['key']) : null`, so an
  absent key is read as it was, and a `null` the payload holds is handed over. A slot of a tuple
  and the value under a variant's name are held by every payload that is read.
- **One read calls each function once.** A read that names its payload more than once, an
  optional and a tuple, holds the corrected payload under `corrected`, the variable of a `switch`
  of one arm; any other read is written around the call itself.
- **A flattened member is read from the keys serde hands it** (`without`, `without_first`,
  `first_of`): the object without the keys of the members beside it and the entries the members
  flattened ahead of it took, and without the tag where the holder is a variant under a tag key
  (`constructor_arguments`' `tag_key`), and an externally tagged enum the one entry of it under
  a variant's name. A function the member names, and one the type it holds names, is handed
  those keys. A holder with no member of its own hands its first flattened member `json` as it
  was, an externally tagged enum apart. A struct's own
  tag is not taken out: serde hands it on too. The slot of a variant under a
  tag key is read from the object without the tag where the slot names a function, and from
  `json` as it was where it names none, so a type such a slot holds is handed the tag, where
  Rust hands it none.
- **Every name a read declares is its own.** `dart_schema_dispatch` records the name of each
  function the item's reads call (`called_by`, `DART_CALLED`), and `declared` numbers a name
  where one is called under it (`json2`): `json`, `payload`, `map`, `tag` and `read`, which
  `Corrects` holds, and the converter of a type parameter (`from_json_argument`), in the
  signature of both reads and where a value behind the parameter is read. A member of the type
  still hides a function of its name, as it hides a check's, and the documents say so.
- **The payload a type's function takes is typed as its read types it**: a
  `Map<String, dynamic>` for a class and an enum under a tag key, a `String` for a plain enum,
  `dynamic` for a wrapper and any other enum. A payload of another kind is refused ahead of the
  function, where Rust hands it over.
- **The tests.** `tests/dart_tests/preprocess.rs` pins the text of each shape.
  `tests/service_schema_emitted_client_tests/run_dart_preprocess.rs` compiles the models of
  `tests/pattern_preprocess_tests/models.rs` and runs them under the Dart SDK: the sample and the
  shared table of positions print what the Rust read prints, two rows apart (`NOT_READ`), with
  the keys a flattened type is handed, the calls of one read counted, a function named after a
  name of the read, a reply through both clients, and the README's section. Its module is
  declared last in the binary: its models take names the modules above it declare, and an
  expansion reads a type by its name.

### 8. Branded Newtypes

Single-field tuple structs with `#[serde(transparent)]` are treated as branded/opaque types, and so is any other `#[serde(transparent)]` struct, with named fields or with several slots, over the one field serde writes and reads it as: `brand_field` in `src/features/serde.rs` is the one seam every surface asks, the web three and Dart, Swift and Kotlin alike, and `transparent_field` beside it is the rule of serde's derive in a `Direction` for which field that is. A struct that derives `Serialize` alone is described over the field serde writes (not under `skip` or `skip_serializing`, which `has_serde_skip_serializing` reads, and not `PhantomData`), and any other over the field it reads (not skipped for reading, no `default`, not `PhantomData`). A struct that derives both, whose two derives take two different fields, is refused at `exec_model_schema`, in every build, with a message naming both (`transparent_direction_errors`, which `serde_attribute_guard_errors` joins to the rename guard): one schema cannot describe two wire forms, as with a rename whose two directions differ. A tuple struct of one slot is a brand over that slot whatever its attributes say. `brand_slot` answers the field with its place among the struct's fields, and every reader of the value on the Rust side (`validate()`, `Display`, the text a bound on a holder's field measures) reaches a slot by that place, through `brand_member`, so `struct Tagged(#[serde(skip)] PhantomData<u8>, pub String)` is read at `self.1`. The brand publishes under its Rust ident unless `#[model_schema(name = "...")]` names another, and that name reaches the surface twice: as the exported type and as the brand tag the values carry.

A non-generic brand publishes a `$RawSchema`/`$Schema` const pair:

```rust
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CorrelationId(pub String);
```

With `zod` + `typescript` features:
```typescript
export type CorrelationId = string & $brand<"CorrelationId">;
const CorrelationId$RawSchema = z.string().brand<"CorrelationId">().meta({
  description: "CorrelationId",
  examples: [],
});

export const CorrelationId$Schema: typeof CorrelationId$RawSchema = CorrelationId$RawSchema;
```

A brand with a type parameter publishes `X$SchemaFactory` plus `X$SchemaDefault` instead, exactly
like any other generic item — never a plain `const`:

```rust
#[model_schema(default_types(IdType = String))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UserId<IdType>(pub IdType);
```

```typescript
export type UserId<IdType> = IdType & $brand<"UserId">;
const buildUserId$Schema = <IdType extends ZodType>(
  idType: IdType,
) =>
  idType.meta({
  description: "UserId",
  examples: [],
}).brand<"UserId">();

type UserId$SchemaOf<IdType extends ZodType> = ReturnType<
  typeof buildUserId$Schema<IdType>
>;

const UserId$SchemaFactoryCache = new WeakMap<ZodType, UserId$SchemaOf<ZodType>>();

export function UserId$SchemaFactory<IdType extends ZodType>(
  idType: IdType,
): UserId$SchemaOf<IdType>;
export function UserId$SchemaFactory(
  idType: ZodType,
): UserId$SchemaOf<ZodType> {
  const hit = UserId$SchemaFactoryCache.get(idType);
  if (hit) return hit;

  const schema = buildUserId$Schema(idType);
  UserId$SchemaFactoryCache.set(idType, schema);
  return schema;
}

const UserId$RawSchemaDefault = UserId$SchemaFactory(z.string());

export const UserId$SchemaDefault: typeof UserId$RawSchemaDefault = UserId$RawSchemaDefault;
```

With `typescript` only (no `zod`):
```typescript
declare const __brand_UserId: unique symbol;
export type UserId<IdType> = IdType & { readonly [__brand_UserId]: true };
```

Rules:
- Generic parameter names are preserved exactly (`IdType` stays `IdType` in TypeScript)
- A brand with no type parameter publishes the `$RawSchema`/`$Schema` const pair; a brand with a
  type parameter publishes `$SchemaFactory`/`$SchemaDefault` instead, the default binding its
  factory call to `{Name}$RawSchemaDefault` first — the parameter's inner composes the factory's
  bound argument (`idType`), never the opaque `z.unknown()`
- Either binding is annotated `typeof` the raw `const` beneath it, never a Zod class: `.brand()`
  narrows at the value position, and the class an inner renders to is zod's to decide
  (the `DateTime<Tz>` schema is a `ZodPipe`, `z.iso.date()` a `ZodISODate`, a brand over a brand a
  `$ZodBranded` of a `$ZodBranded`). Reading the annotation off the value is the one spelling that
  stays true of all of them
- Serde transparent serialization works normally — the newtype is invisible in JSON
- A brand that carries a bound or names a check is read as a struct or an enum is (section 7, "A
  read refuses what `validate()` refuses"): its own unjudged read, then its `validate()`. No hook
  is hung on its slot, and a type that holds the brand reads it unjudged and names it at its path
- The slot takes no `#[model_schema_prop(...)]`: a brand publishes its inner's own schema with a
  `.brand()` written onto it, so no key written there reaches any surface, and one written there is
  refused at `exec_model_schema` — the ungated seam, so the verdict is the same in every feature
  combination — with the item re-emitted stripped of it. The three checks a brand does carry are
  written on the type: `#[model_schema(pattern = "...", minLength = N, maxLength = N)]`. Over a
  list the two lengths count its items and `pattern` is refused (section 7). The
  field a `#[serde(transparent)]` struct with *named* fields or several slots is the value of is
  that slot, and is held to the same. Every other field of such a struct, one serde neither reads
  nor writes (a skipped one, a `PhantomData` marker), is refused a key as well, in every build
  and with the item re-emitted without it, under a message of its own
  (`UNREAD_BRAND_FIELD_PROP_MESSAGE`): `branded_slot_prop_errors` reads every field of a struct
  `brand_field` answers for
- The three checks are type-level arguments on a brand and on an enum, which hands them to the
  fields it reaches (section 7) and publishes no check of its own. A struct that is no brand and a
  type alias are refused them at `process_item`, in every build (`string_check_refusal_output`,
  `alias_string_check_refusal_output`)
- An item whose published expression *is* another item's binding — an alias of a brand, a one-slot
  tuple struct over one, and the `$SchemaDefault` of either where it declares a parameter — is
  annotated `typeof {Name}$RawSchema` rather than `ZodType<{Name}>`, so the brand's narrowing
  survives the republish. An item that builds an expression of its own keeps `ZodType<{Name}>`

### 9. Generated Rust Writes Standard Names by Their Full Path

What the macros emit lands in a module the consumer wrote, where a type of theirs may be named
`Vec`, `String`, `Ok` or `Send`. Nothing inside a `quote!` writes a name the standard prelude
supplies bare: it is `::std::vec::Vec`, `::core::option::Option::Some`,
`::core::result::Result::Ok`, `::core::marker::Send`, `::core::ops::FnOnce`. Every such path starts
with `::`: a module of the consumer's may be named `core` or `std`, and a path written
`core::option::Option` resolves through that module. A path serde reads out of an attribute's text
follows the same rule (`"::core::option::Option::is_none"`). A name inside `#[derive(...)]` stays
bare: a derive resolves in the macro namespace, where no type of the consumer's reaches.

`src/emitted_names_tests.rs` reads the crate's own sources and fails on a bare name, and on a
`core` or `std` path that does not start with `::`; `tests/shadowed_core_module_tests.rs` builds
model types and a service beside a module of each name. A test that
declares a model type named `Vec` or `String` needs a test binary of its own
(`tests/shadowed_vec_tests.rs`): the registry is keyed by ident for the whole crate, so a field
written `Vec` in any later expansion would be read as that type.

### 10. The Swift `validate()`

Every Swift type the emitter writes has `public func validate() -> [String]`, which runs what the
Rust `validate()` of the type runs, in its order and in its words. `README.md`'s "`validate()` in
the Swift client" section is the consumer's view; this is what a change to the emitter has to keep.

- **The emitter is handed what it reads.** `swift_suffix_tokens` in `src/model_schema.rs` builds a
  `SwiftTypeArgs`: the `name` override, the type's `check` names, a brand's or an enum's
  `minLength`, `maxLength` and `pattern`, the declared `default_types`, and `member_meta`, which is
  `member_prop_meta` with the enum's arguments where the item is an enum. Every member's bounds
  are read through it (`member_def`), so an enum's string check reaches in Swift the fields it
  reaches in Rust.
- **`src/features/swift/validate.rs` writes the method.** `member_rules` is one member: its bounds
  (`bound_rules`), what the value it holds reports (`held_walk`), then the checks it names
  (`check_rules`). A bound of the member's own stands in place of the walk, but on a list and on
  the slot of an untagged variant (`Checked::runs_both`), which is `named_member_validate_body`'s
  and `slot_validate_body`'s rule. `own_checks` are the type's, last. `method` writes the method
  and the helpers its statements call, and `[]` for a type with no rule.
- **One sentence source.** `swift_violation` in `src/bound_message.rs` is written from
  `Bound::stated()`, as `rust_violation` is, and
  `the_swift_sentence_is_the_one_rust_writes_for_each_bound` holds the two together.
- **The unit of a length is the Unicode code point** (`unicodeScalars.count`), and a list is
  counted by `count`. Rust measures a string with `len()`, its bytes: a test of a non-ASCII string
  pins this client's own text.
- **A range is written where a value can break it.** `Number::least` and `Number::most` answer no
  literal for a bound outside the range of the field's integer type: `x < 0` on an unsigned
  integer is a warning in Swift. A float is quoted back through `swiftSchemaPlain`, which drops
  the `.0` Rust does not write.
- **A pattern is respelled for Foundation.** `swift_pattern` writes `$` as `\z` and `\v` as
  `\u000B`, which is what makes the engine read the portable pattern as the `regex` crate does. The
  message quotes the pattern as written.
- **A place is the steps serde writes.** A named member goes by `SwiftField::wire_name`, a
  flattened one by none, a slot of a tuple by its position (`slot_rules`), the one slot of a tuple
  struct by none (`tuple_struct_rules`), and a variant's payload by the variant's name with no tag
  attribute, by the content key under one, and by none under a tag alone and in an untagged enum
  (`resolve_variant_payload`'s `step`). `held_walk` adds the index of an item and the key of an
  entry, a map being walked by its sorted keys.
- **Nothing is shared between two emitted types, and nothing is declared beside them.**
  `swiftSchemaUnder` and `swiftSchemaPlain` are private static functions written into each type
  whose statements call them (`method`). A name a second emitted type would declare too does not
  compile once the types are joined into one file.
- **A named type is measured by what the language finds.** A brand over a string or a path
  declares `swiftSchemaBoundText` (`brand_text_member`), a brand over a brand answering through the
  one it holds, and a bound on a field typed with any named type reads that member, declared above
  or below. The registry (`note`, `Text`) decides one thing: an alias of a string the emitter has
  written is measured as the string. No bound is refused as the type is expanded: a type with no
  such member is the Swift compiler's to refuse.
- **A holder calls `validate()` on every named type it holds** but one the registry has noted as
  running no rule (`validates`): a type declared below is called, the holder having no way to tell
  it from one with a rule.
- **A local of the method is never read as a property.** `property_read` reads a property named
  `errors`, `held`, `index` or `key` through `self` (`TAKEN_BY_VALIDATE`), and `swift_property`
  declares a field named `validate` as `validate_`, with its key kept. `swift_field_member` is
  the same seam, so a service client reads the property under the name it was declared with.

### 11. The Swift read

Every Swift type the emitter writes has two reads: `init(swiftSchemaUnjudged:)` is the type's own,
and `init(from:)` runs it and then `validate()`, throwing `DecodingError.dataCorrupted` with every
message joined by `; ` under the decoder's own coding path. `README.md`'s "A Swift read refuses
what `validate()` refuses" section is the consumer's view; this is what a change to the emitter has
to keep.

- **`src/features/swift/read.rs` writes both.** `both` takes the statements of the type's own read
  and writes the pair, `JUDGED` being one text for every type. A struct, the struct written for a
  variant's members and for a tuple, a brand's wrapper, a tuple struct and an enum under every
  tagging call it. A plain enum that names no check keeps the `init(from:)` Swift synthesizes, and
  its unjudged initializer is that read.
- **A member is read unjudged exactly where `validate()` walks it.** `validate::walks` is the one
  question: no bound of the member's own stands in place of the walk (`walked`, which
  `member_rules` and `bound_alone` ask too) and `held_walk` ends on a value whose `validate()` it
  runs (`reaches`). `read::unjudged` then takes the member apart as `held_walk` does: an `Option`,
  each level of a list, a map's values, the struct written for a tuple, a named type. A step added
  to one is added to the other: a value the read leaves unjudged and the walk does not reach is
  judged by nothing.
- **Any other value is read through `Decodable` and judges itself**: one behind a type parameter,
  one in a field with a bound of its own, and the value a brand wraps, a brand's `validate()`
  running no `validate()` of what it holds.
- **A held type is handed a decoder of its own** (`Source`): `superDecoder(forKey:)` under a key,
  `superDecoder()` in a list, and the holder's decoder for a flattened member, the tuple of a
  tuple struct and a payload beside a tag.
- **An absent key is what it was** (`Presence`). `superDecoder(forKey:)` answers a decoder over
  `null` for an absent key, where `decode` answers `keyNotFound`, so `Refused` writes
  `_ = try container.decodeNil(forKey:)` ahead of the read of a held type. `Nil` is an optional
  member's: `contains`, then `decodeNil`. `decode_statement` is told whether Swift synthesized the
  read before one was written (`synthesized`): a synthesized read took the absent key of every
  optional member as `nil`, a `nullable` one included, and a written one refused that key. A
  member under `#[serde(default)]` is read as its default where `contains` answers no key
  (section 13).
- **Every struct writes its memberwise initializer**, public, the struct written for a tuple
  included: Swift synthesizes none beside a written `init(from:)`.
- **`encode(to:)` does not move.** `struct_body_content` writes it where it wrote it before
  (`needs_encode`), and Swift synthesizes it everywhere else.
- **A member serde never writes is read and encoded under no key.** `SwiftField::never_written`
  is `has_serde_skip_serializing`: `skip_serializing`, never `skip_serializing_if`. Swift's own
  `encode(to:)` encodes every property, so `needs_encode` holds for a struct with such a member,
  a variant's payload sharing `struct_body_content`, and the written one encodes no statement for
  it. Where no member is encoded under a key, it opens the object with
  `_ = encoder.container(keyedBy: CodingKeys.self)`. A member under `skip_serializing_if` is
  encoded wherever it holds a value: no client calls the Rust function, and the README states the
  limit.
- **The container of a struct's keys is declared where a statement uses it.**
  `struct_body_content` writes `let container` ahead of the read's statements and
  `var container` ahead of `encode(to:)`'s only where `names_container` finds a statement that
  reads or writes through it, so a struct and a variant's payload whose every member is flattened
  declare neither, which `-warnings-as-errors` would refuse unused.
- **Nothing is shared between two emitted types.** `SwiftSchemaAnyKey`, the key type a map is read
  entry by entry under, is private to each type whose read names it (`any_key`). `Locals` numbers
  what one initializer declares (`listed`, `items`, `keyed`, `entries`, `entry`, `number`), and a
  property is assigned through `self.` alone, so one named after a local is never read as it. A
  written `encode(to:)` reads each property through `self.` too (`encode_statement`), so a member
  named after one of its locals (`container`, `tagged`, `held`, a `wire{Member}`) or its
  parameter, `encoder`, is never read as either. `run_swift_flattened.rs` runs a member named
  `container`, of its holder's own and flattened, against serde.
- **A map under a key Swift reads through a wrapper** (`MapKeys::Wrapped`) is read by the wrapper,
  which gains an `init(swiftSchemaUnjudged:)` where its values are walked.
- **A flattened map is read entry by entry** (`flattened_map`, from `member_reads`,
  `read::read_entries`), under no key the types around the holder took, a `where` clause over
  `taken` that `names_taken` counts, so the holder takes those keys: the tag of a variant under a
  tag key among them. A map is the one member of its object (section 4), so the holder's own
  members and the members flattened ahead of it take no key of it. A map under a key Swift wraps
  is read by its wrapper, through the `init(swiftSchemaUnjudged:swiftSchemaWithout:)`
  `flattened_wrapper_read` writes in an extension of the wrapper for the flattened member alone.
  `run_swift_flattened.rs` runs a map alone in each shape, and `tests/swift_tests/flattened.rs`
  pins its text. What a struct takes is `preprocess::keys_taken_by`: the keys
  `struct_swift_tokens` records for its type (`preprocess::note_keys`), serde's own list from
  `keys_taken_when_flattened`, and none for a member that names a function. A struct that names a
  `preprocess` on itself is read by serde as a value, and one with a flattened member of its own
  as a map: each records none. A type the emitter has not written takes none, and the README
  states the limit. `encode(to:)` writes each flattened member at its place, through its shape
  (`written_from`): a key two of them hold is written once, with the last value, where serde
  writes it twice. A flattened `Option` is written under `if let held` (`encode_statement`): the
  keys of the value it holds, and nothing for `nil`, where `Optional`'s own `encode(to:)` writes
  one value over its holder's keys. A holder with no member written under a key opens its object
  all the same. `tests/swift_tests/flattened_options.rs` pins the text, and
  `run_swift_flattened_options.rs` runs it against serde.
- **A flattened `Option` is read as `nil` where the object it is handed holds none of its type's
  keys, and a key there that does not read refuses the payload**, as TypeScript and Zod refuse
  it and Kotlin's `flatten_struct_field_plan` reads it. serde reads `None` there,
  `FlatMapDeserializer::deserialize_option` answering `None` for any error of the type's read.
  `flattened_option`, from `present_statement`, reads the type under the `Option` as a flattened
  member of that type is read, unjudged where `validate()` walks it and handed the keys taken
  where it takes them, behind `if {test} { ... } else { self.{member} = nil }`. The test is
  `preprocess::holds_one_of`: one of the keys `preprocess::keys_taken_by` answers for the type, in
  `decoder.container(keyedBy: SwiftSchemaAnyKey.self).allKeys`, less the `Handed` keys where the
  member is withheld, so the key type is the one `read::both` writes into the holder. A type
  `keys_taken_by` answers no key for keeps `do { ... } catch { self.{member} = nil }`, which reads
  `nil` wherever its read throws: a struct declared below its holder or in another crate, one with
  a flattened member of its own or that names a function on itself, an enum, an alias and a type
  parameter. A model it holds that breaks its rule still refuses the payload through `validate()`,
  as the Rust read refuses it. A member that names a function is read from what its functions
  answer, and refused where its type does not read the answer, as the Rust read refuses it, and a
  flattened map, optional or not, by `flattened_map`. `tests/swift_tests/flattened_options.rs`
  pins the text, and `run_swift_flattened_options.rs` reads each payload against serde, the rows
  serde reads as `None` and the Swift read refuses held to the refusal it prints.
- **Every flattened member is handed the keys its holder withholds, the first one included.**
  `member_reads` hands each a `Handed`: the `taken + [...]` of the holder's own keys and those
  each member flattened ahead took, and `withheld`, whether any key can be withheld: the holder
  has a member of its own, a member ahead took keys, or the types around the holder hand some. A
  type that takes keys (`preprocess::beside`) and a member that names a function read that
  expression. Where `withheld` holds, `present_statement` reads any other flattened member through
  `corrected_field` with no function: `SwiftSchemaPayload(from:without:)`, then the type, a model
  `validate()` walks through `SwiftSchemaRead{Member}`, unjudged. A struct that reads a key of
  the holder's own members or one a struct ahead took is then refused, as serde refuses it. A
  holder of no member of its own is handed keys where the types around it hand some (`around`,
  which `struct_declaration` takes) or where one of its reads names `taken`, so `handed_reads`
  writes its reads twice there, the second time with every member withheld from; one whose
  reads name none, and that nothing around hands keys, reads its members from its decoder.
- **What a variant under a tag alone holds is handed the object without the tag**, as serde
  hands it. `EnumTagAttrs::tag_alone` is the one question. `named_payload` hands
  `struct_declaration` `around` under it, so the struct written for a variant whose every member
  is flattened withholds keys from each, takes the keys itself and is read by `read::beside` with
  `taken + [tag]`; a struct with a member of its own is written as it was. `slot_payload` reads a
  slot whose type takes no keys (`preprocess::beside`) through `VariantPayload::corrected` with no
  function: `corrected_member` reads it from `SwiftSchemaPayload(from:without:)`, a model
  `validate()` walks through `SwiftSchemaRead{Case}`, unjudged. A slot whose type takes keys is
  read through its taking initializer, as before. `tests/swift_tests/flattened.rs` pins the text
  of `Grafting` and `Pointing`, and `run_swift_flattened.rs` runs both against serde.
- **An untagged enum takes the variant serde takes.** `validate::bound_alone` is what chooses: a
  member's own bounds, and `held_walk` under `Walk::Bound`, which asks each brand the member
  holds for `swiftSchemaValidateBound()`, the member the wrapper of a bounded brand declares
  (`bound_member`). The enum writes one `swiftSchemaBound{Variant}` per variant that has such a
  rule, and one `swiftSchemaRead{Variant}` where a slot is read in several statements. A check and
  a model choose nothing. The registry says which name is a bounded brand (`note_bounded`), so one
  declared below the enum chooses nothing.
- **A name the emitter has not written is read as a model**, as `validate()` calls it: a type
  declared below its holder, and a type of the consumer's own, which declares
  `init(swiftSchemaUnjudged:)` beside `validate()`.
- **A std sequence is the array serde writes.** `arrayed` in `src/features/swift.rs` reads a
  `VecDeque`, a `BinaryHeap`, a `HashSet`, a `BTreeSet` and an `IndexSet` as one array level, as
  `Vec` is read, through a sequence of sequences. `member_def` and the target of an alias call
  it, so the declaration, the read, the write and `validate()` read one def, and `swift_resolve`
  calls it for what no member is collected into: a type argument, a map's value, a tuple's
  position and a service's message. `tests/swift_tests/sequences.rs` holds the text of each
  shape to the text of its twin written with `Vec`, and `run_swift_sequences.rs` reads and writes
  each under the toolchain.
- **The tests.** `src/features/swift/read/tests.rs` holds each `Source` and `Presence`, and the
  README's block to what the emitter writes for the documents' parcel;
  `tests/swift_tests/read.rs` pins the text of each shape;
  `tests/service_schema_emitted_client_tests/run_swift_read.rs` compiles the emitted types with
  `-warnings-as-errors`, holds each read to the Rust `validate()` of the value and to the Rust
  read of the payload, an untagged enum to the variant serde takes, and each client to its fault.
  `an_absent_key_and_a_null_are_read_as_they_were` passed on the emitter before a read was
  written. The `validate()` runs read a payload through the unjudged initializer, and build in
  code the three values no read hands back. `tests/swift_tests/flattened.rs` pins the read of a
  flattened map alone in each shape, of a member that names a function at each place, and of
  each other member flattened after a struct, over the models of
  `tests/swift_tests/flattened_models.rs`, which also pin what serde reads of each, and
  `run_swift_flattened.rs` reads and writes each back under the toolchain, against serde, a holder
  whose every member is flattened among them.

### 12. The Swift `preprocess`

The read of a Swift type calls the functions the type and its members name, each on a payload,
ahead of the value read from it. A function is called under the name the attribute writes,
`func trimmed(_ payload: Any) -> Any`, on the Swift values of a JSON value: a `[String: Any]`, an
`[Any]`, a `String`, a `Bool`, an `Int64` for a number one holds, a `Double` for any other, and an
`NSNull`. The documents call the key a last resort, and a change keeps them saying so.
`README.md`'s "`preprocess` in the Swift client" section is the consumer's view; this is what a
change to the emitter has to keep.

- **The emitter is handed the type's functions.** `swift_suffix_tokens` puts `args.preprocess` in
  `SwiftTypeArgs::preprocess`, and a member's are in the meta `member_def` reads
  (`preprocess::functions`). Nothing reads the item's derives: the Swift type is read whether or
  not the Rust one derives `Deserialize`.
- **`src/features/swift/preprocess.rs` holds what is written.** `PAYLOAD` is the private
  `SwiftSchemaPayload`, written into each type that names a function and into each that reads a
  member from its object without some keys, a flattened member or what a variant under a tag
  alone holds, and into no other (`read::both` adds it where the statements or `Written::own`
  name it). It reads any JSON value from a `Decoder`, reads a type back from one through
  `JSONSerialization` and a `JSONDecoder` of its own, and writes the coding path of the value
  into each `DecodingError` of that read. An answer `JSONSerialization.isValidJSONObject` refuses
  is a `dataCorrupted`, never an exception.
- **A member's functions are called in the read the member had.** `preprocess::member` is the one
  writer, called from `decode_statement` (a field), `tuple_shape` (a slot of a tuple) and
  `VariantPayload::read` (the one slot of a variant). `Source::payload` answers the payload and
  the coding path of each source: `decode(SwiftSchemaPayload.self, forKey:)` under a key, which
  refuses an absent key as a plain `decode` does, a decoder of its own for a slot, and the
  decoder itself. `Source::absent_as_nil` asks an optional member for its key first, so no
  function is called for a key the payload does not hold. The value is then read as it was read:
  as its declared type, as its wire type with the conversion it had (`Plain::Wire`), or, where
  `validate()` walks it, through a private type written for it (`SwiftSchemaRead{Member}`),
  whose `init(from:)` holds the statements of `read::unjudged`. A model read from a corrected
  payload is therefore still read with no judgement.
- **A type's functions are called by its own read.** With `Written::own`, `read::both` writes the
  statements into the private `init(swiftSchemaCorrected:)`, and `init(swiftSchemaUnjudged:)`
  hands the functions the payload, then reads `SwiftSchemaRead` from what they answer, a private
  type whose `init(from:)` calls that initializer. Every caller of `both` passes the type's own
  functions, so every shape takes the key: a struct, a unit struct, a brand's wrapper, a tuple
  struct, an enum under each tagging and a plain one, which then writes its read as one that
  names a check does. `JUDGED` does not move: `init(from:)` judges what the functions answered.
- **A type written inside another names its owner.** In a private type nested in `Parcel`, `Self`
  is the nested type, so `both` writes each member's type through `naming_its_owner`.
- **A function is handed the keys no type around its own took**, as the Rust read hands them.
  `Source::Beside` is the decoder of a type that reads keys of its own beside the value: a
  flattened member (`decode_statement`, with the wire names of the struct's own members and the
  keys each struct flattened ahead of it took), a
  payload under a tag alone (the tag) and an untagged one. It holds the keys as a Swift
  expression, `taken + ["dock"]` (`taken_with`). A type whose statements name `taken`
  (`names_taken`) has its own read twice: `init(swiftSchemaUnjudged:swiftSchemaWithout:)`, and
  `init(swiftSchemaUnjudged:)`, which calls it with no key. A type that names a function on
  itself always has both, and removes the keys from the payload it hands its functions
  (`SwiftSchemaPayload(from:without:)`); the statements of its `init(swiftSchemaCorrected:)` then
  read `let taken: [String] = []`.
- **The registry says which type takes the keys.** `preprocess::note` records each written type
  whose reads hold the taking one (`read::takes_keys`), an alias of one included, and
  `preprocess::beside` is what a holder asks. A flattened member, the slot of a variant read
  from the enum's decoder and the value of a brand are read through that initializer where the
  type takes keys, and through the read they had everywhere else, so a type that names no
  function and reads none is written as it was, a flattened member its holder withholds keys from
  and the slot of a variant under a tag alone apart. A struct written for a variant's members
  answers through `VariantPayload::takes_keys`. A type declared below its holder is not known
  to take keys, and is read from the holder's whole object, or, flattened where its holder
  withholds keys and as the slot of a variant under a tag alone, from the payload without them.
- **An untagged enum hands its readers the keys.** A `swiftSchemaRead{Variant}` whose statements
  name `taken` takes it as a second parameter, and `Written::beside` hands `both` the readers,
  which name the payload type and the key type the enum declares ahead of them.
- **Names.** `swiftSchemaCorrected`, `swiftSchemaWithout`, `SwiftSchemaPayload`, `SwiftSchemaRead`
  and `SwiftSchemaRead{Member}` are the ones this adds. A local is `{member}Payload`,
  `{member}Decoder` for a slot, and `wire{Member}` where the member is converted.
- **The tests.** `src/features/swift/preprocess/tests.rs` holds the call, each source and the
  registry; `tests/swift_tests/preprocess.rs` pins the text of each shape, and that a type with
  no function is written as it was;
  `tests/service_schema_emitted_client_tests/run_swift_preprocess.rs` compiles the emitted types
  with `-warnings-as-errors` and holds what they read to `SAMPLE` and `POSITIONS` of
  `tests/pattern_preprocess_tests/models.rs`, the table the Rust read and the Zod schemas are
  held to, to the Rust read of each payload of its own models, and each client to its reply.

### 13. A Key Read as Its Default

A key under `#[serde(default)]`, written on a field of a struct or of an enum's variant, with a
function, or on the struct, that the payload leaves out is read as its default on every surface.
The value of a default is known to Rust alone, so each function that answers a text writes it with
serde when it builds the text.
`README.md`'s "A Key Read as Its Default" section is the consumer's view; this is what a change
has to keep.

- **One seam.** `SerdeDefaults::of` in `src/features/serde/defaults.rs` answers, for a struct
  with named fields, which members serde reads as a default, in the order of the fields, and the
  tokens that build the JSON of each: `<T as Default>::default()`, the function `default = "…"`
  names, or the member of the struct's own default, written through the field's
  `serialize_with`, its `with` or the hook `as_number` hangs on it. It reads the attributes serde's
  derive reads, so it answers the same before `process_struct` walks the fields and after.
- **What has no JSON** on a struct, which keeps its key required on every surface but Rust and
  builds: a struct that does not derive both `Serialize` and `Deserialize`, or names `from`,
  `try_from`, `into` or `remote`; a `transparent` struct; a member under `skip`,
  `skip_serializing`, `skip_serializing_if` or `skip_deserializing`; a flattened member; an
  `Option`, which keeps what it had; a type that names a parameter of the struct, a lifetime or
  `Self`; and the struct's own default on a struct that declares a parameter. A list, a set or a
  map under the bare `default` is read as its default whatever its type names, unless it names a
  `serialize_with` or a `with`: `empty_collection` writes `[]` or `{}` for a type whose last name
  `is_sequence_wrapper` or `is_map_wrapper` accepts, a literal `literals()` builds with no
  parameter in scope.
- **An enum.** `SerdeDefaults::of_enum` answers the same for each variant with named fields,
  keyed by variant and member, with the exclusions `of` makes for a struct's. The index runs
  across the whole enum, so each placeholder is unique within the item, and `SerdeDefaults::at`
  and `index` take the variant, `None` for a struct. A variant has no container default: serde
  reads `default` on a variant's field alone. Each enum processor sets `FieldDef::serde_default`
  on a variant's member from it, as `collect_struct_fields` does for a struct's, and the enum's
  schema module declares the holder and the `values_impl`, as `struct_output_with_unit_impls`
  does. Dart's and Kotlin's `variant_payload` and Swift's `named_payload` hand a variant's fields
  the enum's defaults for that variant.
- **No placeholder reaches the text of another item.** It is replaced only by the function that
  answers the item's own text. `FieldDef::zod_read_whole` asks `holds_a_placeholder` of the
  spellings `zod_union_members` and `flatten_variants` record, and `flattened_zod_branches` and
  `zod_member_leaves` ask it where they ask `zod_names_a_preprocess`: a holder, flattening the
  enum or holding it, joins it by its published binding, as it joins a type that names a
  `preprocess` (`ZOD_PREPROCESSED`). The registry still records the members, so the refusal of a
  flattened member that is no object reads them as before.
- **A variant's member with no JSON is refused.** `default_refusals`, beside `of_enum` and written
  from the same `reach` `of_enum` reads so the two cannot drift, answers one error per member under a
  field-level `default` that no surface but Rust can read as serde does, spanned on the field and
  opened by ``model_schema: type `{enum}`: variant `{variant}`, field `{field}`:``: a type that
  names a parameter of the enum, a lifetime or `Self` (`names_any`), other than a list, a set or a
  map under the bare `default` with no `serialize_with` or `with` (`empty_collection`); a flattened
  member; a member under `skip_serializing` or `skip_serializing_if` that is not an `Option`; and
  every such member of an enum that derives `Deserialize` and not `Serialize`, or names `into` or
  `remote`. `serde_attribute_guard_errors` joins them to `rename_direction_guard_errors`, and
  `exec_model_schema` raises the two through one `refused_item_output`, every refusal of the item
  in one expansion, under the rename guard's gate: `serde` with any
  of `typescript`, `zod`, `jsonschema`, `dart`, `swift` and `kotlin`. An `Option`, a member under
  `skip` or `skip_deserializing`, and an enum that derives no `Deserialize` or reads itself
  through `from` or `try_from` are refused nothing: no surface reads them otherwise than serde
  does. A struct is refused none of the four: its key stays as "What has no JSON" says, and it
  builds. `README.md`'s "What does not build on a variant's member" holds the four messages.
- **The value is built where the type is declared.** Each module that answers a text declares
  `SerdeDefaults` (`SerdeDefaults::holder`), and `values_impl` writes
  `impl {module}::SerdeDefaults { pub fn literals() }` beside the struct or the enum, where every
  name its author wrote resolves, inside a function body too: no path from the module reaches a
  type declared in one, and an `impl` written beside the type reaches the module.
- **The text holds a placeholder** (`placeholder`), which `substituted` replaces with the JSON
  when the function that answers the text runs, escaped for the literal it sits in: a Dart
  `'…'`, a Swift or Kotlin `"…"`. Zod writes the JSON as an expression, and the JSON Schema puts
  the value itself into the member's object.
- **Each surface.** Zod: `FieldDef::serde_default`, which `zod_type` writes as
  `.prefault(<json>)` around the whole member schema, and `generate_zod_schema_method`, whose
  `zod_schema()` replaces the placeholder. JSON Schema: `build_field_schema`, `key_is_required`.
  TypeScript writes nothing new. Dart: `DartField::default_at`, `constructor_arguments`. Swift:
  `SwiftField::default_at`, `decode_statement` and `default_statement`, with
  `preprocess::defaulted` reading the JSON through `SwiftSchemaPayload` with the read the member
  has. Kotlin: `KotlinField::default_at`, `property_declaration`, which writes
  `= Json.decodeFromString(<member_serializer>, "…")`, and `own_field_plan` for a struct with a
  flattened member. A default is read with the member's own read, unjudged where `validate()`
  walks it, so `validate()` judges it as it judges a payload. A member whose type names a
  parameter of the class (`KotlinField::empty_default`) is `= emptyList()` or `= emptyMap()`
  instead: an initializer reaches no serializer of the class's parameters.
- **A type with no default is emitted token for token as before**: each module keeps
  `#source.to_owned()`, and no `SerdeDefaults` is declared.
- **No function a `preprocess` names is called for a default**, but by Zod, where `.prefault`
  hands the default to the whole member schema.
- **The tests.** `tests/serde_default_tests/` holds the models, the table every surface is held
  to (`CASES`, `REPORT`), the structs that declare a parameter, read at `String` (`FILLED`,
  `INDEXED` under `indexmap`) and refused a key they require (`REQUIRED`), a member that names a
  `preprocess` (`PREPROCESSED`), the JSON Schema, the Zod text, the README's section, and the
  text of a type with no default on every surface, which the emitter before defaults were read
  writes too; `tests/generic_types_tests/declarations.rs` compiles the Zod schemas with
  `tsc --strict` and parses the tables under node; `run_dart_serde_default.rs`,
  `run_swift_serde_default.rs` and `run_kotlin_serde_default.rs` run the tables under each
  client's toolchain, `Fleet` and `IndexedFleet` among them.
  `Berthing` under each of the four taggings is read in the same places, with the key and
  without it, beside a holder of its untagged form whose Zod text holds no placeholder. Each
  refusal of a variant's member is a `trybuild` case whose `.stderr` holds its message spanned on
  the field, and an enum with two refused members reports both.

### Generic Types and Zod Factories

A Zod schema is a runtime value and TypeScript generics do not exist at runtime, so a generic
struct, tuple struct or enum has no one schema to publish. It exports `X$SchemaFactory` instead —
a function taking one required schema argument per parameter — while a type that binds no type
parameter keeps exporting `X$Schema` exactly as before. `zod_binding_suffix` is the one seam that
decides which, and `zod_published_binding` the one seam that writes either.

Beside the factory, a generic type also exports `X$SchemaDefault` — the factory called at the
type's own declared `default_types`, so a consumer who wants the ordinary filling shares the memo
rather than building a second schema for it. `zod_default_block` builds the const, through
`default_zod_rendering`, the same `get_field_def` path every field renders through; where a
declared default names another generic item at exactly the arguments that item calls its own,
the rendered argument folds onto that item's `$SchemaDefault` instead of reconstructing a factory
call the memo would not share with it — see `record_zod_default_arguments`. `zod_binding_reexport`
covers both bindings wherever a renamed item re-exports its factory.

`X$SchemaDefault` is a module-scope `const`, and any argument `default_zod_rendering` renders as a
reference to another item — the fold above, or an ordinary (non-folding) call to that item's own
`X$SchemaFactory` — names one more module-scope `const`. One macro invocation sees one type, so
nothing here can know whether that `const` is written above this one in the generated module or
below it; `default_zod_rendering` defers every such reference through `deferred_zod_operand`, the
same `z.lazy(() => …)` wrap `.and(...)` already relies on for a flattened base. A self-contained
expression like `z.string()` names no sibling `const` and is left eager. The fold's own gate
(`array_depth == 0 && !is_optional()`, a *direct* sibling) only scopes the fold — a default
wrapped in a `Vec`/`Option`/`Map`/`Tuple` has no bare binding to fold onto — but deferral is not
bounded by it: `FieldDef::names_a_sibling_binding` walks the whole rendered tree, so
`Tagged$SchemaFactory` inside `z.array(Tagged$SchemaFactory(z.string()))` defers exactly as a bare
`Tagged$SchemaFactory(z.string())` does, the `z.lazy` wrapping the whole expression rather than the
factory call alone. The deferral is also what ends a cycle between two declared defaults —
`CycleLeader`'s naming `CycleFollower` and `CycleFollower`'s naming `CycleLeader` back: neither can
have registered the other's arguments yet when it expands, so neither side folds, and both calls
read the other's factory behind `z.lazy` rather than at the top of a `const` initializer — the
module loads regardless of which of the two names the consuming project's entity list writes
first.

Each factory memoizes on the identity of its arguments, one `WeakMap` level per parameter
(`zod_cache_type` writes the nesting, `zod_factory_body` the walk), so two calls with the same
argument objects return the identical schema and no two argument lists collide.

A `WeakMap` fixes both of its own parameters at construction, so its value type cannot depend on
its key type and the store is written at the widened `ZodType` spelling throughout. What recovers
the precise types is an overload, which `zod_factory_declaration` emits: the signature a caller
sees names real type parameters, the implementation beneath it takes the widened ones the store is
keyed at, and the read is therefore already the implementation's return type. Nothing emitted is
`as`, `any` or `unknown`; nothing a caller passes is written to; and there is no preamble — a
generated module is the types in it and nothing shared between them.

Every parameter is a real TypeScript type parameter (`<IdType extends ZodType>`), never a bare
`ZodType` annotation — `ZodType` defaults its own parameters, so an argument annotated with it
infers every field as `unknown`.

A generic type may reach itself, directly or through a second type reaching back. JSON Schema
hoists it into `$defs` once and points a `$ref` at that definition; TypeScript writes the name
inside itself with its arguments; Zod calls the factory again with the argument the outer call was
handed, the memo cache being what ends the recursion — the schema is cached before the recursive
call is made. A builder that defers a reference — a member behind a getter, an operand behind
`z.lazy` — reads its type back under `interface X$SchemaOf<…> extends ReturnType<…> {}` where every
other generic item keeps the `type` alias: `zod_schema_of` decides, off `defers_a_reference`. A
declaration file writes a type that reaches itself only by name, and writes an alias as the
structure it resolves to, with `any` where that structure recurs; the interface is what a second
package, reading the `.d.ts`, parses the recursive member through.
`tests/generic_types_tests/declarations.rs` compiles that second package for real.

A tuple struct has no key to write a getter on. `tuple_slot_zod` writes a slot that reaches the
struct itself, or a type declared below it, behind `z.lazy` through `deferred_zod_operand`; read
as it stands, the slot names a binding still being built and the module throws as it is imported.
For a struct with no parameter that is the whole of it, the binding being annotated
`ZodType<X>`. A generic one cannot read its own factory's type inside the tuple, whose slot types
are read as the tuple is built: `zod_self_view` writes `X$SchemaSelf`, an overload declared to
return `ZodType<X<T["_zod"]["output"]>>` over an implementation that hands back the factory's schema, and
the slot calls that behind `z.lazy`. The same test file loads the bundle under `node` and parses
through both. Which of the item's two bindings a self-reference names is read off the store
`record_zod_factory` writes, and that is written at `exec_model_schema` ahead of every shape rather
than where the binding is finally spelled: the fields are rendered before then, so an answer stored
on the item's own registry entry would be read before the item had put one there.

`write_field_type_and_schema` defers a member whose type reaches the item being defined, and one
that reaches a type declared *below* it — `reaches_a_type_declared_later`. The rule applies whether
or not the referenced type declares a parameter of its own: a zero-argument sibling not yet
registered is exactly as much a forward reference as a generic one, since its `X$Schema` is read
eagerly at the top of the referencing item's own module-scope const initializer. The second clause
is what ends a cycle spanning two types: every cycle contains at least one reference pointing
forward, since declaration positions cannot strictly decrease all the way round one, and what is
left once those are deferred cannot cycle. The deferral is a getter rather than `z.lazy`, which at
an operand
position collapses the factory's inferred return type to `any`.

A union written around a factory's call -- the `Option` wrap, or the `nullable` one -- is checked
against what each member takes and answers: `.prefault(undefined)` types its argument by the
union's input, and `z.union` over a `z.record` of the call resolves the record's own output. Inside
a cycle that is the type still being inferred, which `tsc --strict` refuses (TS7023, TS2615).
`zod_union_around` therefore hands every factory call in through a generic arrow
(`<Reached$ extends SomeType>(reached$: Reached$) => …`) and builds the union over the type
parameter, so the check is made once against the parameter and the member only instantiates it.
`FieldDef::zod_factory_calls` lists the calls, outermost only and once each, by the same renderer
that writes them; the names end in `$`, which no argument `zod_factory_argument` binds can. The
bound is the one `z.union` and `z.array` are themselves declared with -- `ZodType` there reads the
schema's output and brings the cycle back. It is written bare (`ZOD_SCHEMA_BOUND`, `SomeType`) and
the self view writes `T["_zod"]["output"]`: a consumer may bind `z` as a constant
(`const z = zod;`), which names no type, so nothing emitted writes `z.` where a type is read.
`SomeType` is the consumer's to bring into scope, as `ZodType` and `$brand` are
(`import type { SomeType } from "zod/v4/core";`). The bundle
`tests/generic_types_tests/declarations.rs` compiles binds `z` as a constant and imports both.

A map of the item itself is read through the self view as well. `z.record` resolves its own type
off its value as it is written, and inside the builder the item's own factory answers the
builder's return type, still being inferred: `tsc --strict` refuses a record of it nested in
another call (TS2345) and resolves a second record's output to `unknown` after a first (TS2322).
`maps_of_itself_through_the_self_view`, called from `zod_factory_block` under `typescript`,
rewrites the item's own factory call to `X$SchemaSelf(…)` wherever it is written inside a
`z.record`/`z.partialRecord`, and hands a union's arrow the view where the arrow writes its
argument inside one. `zod_factory_block` emits the view for whatever builder calls it, a tuple
struct's included. A list or an `Option` of the item, and a reference to another generic item,
keep the factory's call. Without `typescript` there is no `X<…>` to declare the view with, and
nothing is rewritten.

### Declaring a Default Type per Parameter

JSON Schema has no type parameters, so a generic item's document is built from one concrete
filling, and `default_types(IdType = String, DateType = f64)` is where an author names it — one
`Parameter = Type` pair per type parameter, in any order, beside every other item-level argument.

`default_types_guard_errors` reads the declaration against the item's own parameters at
`exec_model_schema`, the one seam every expanded shape is dispatched from, so a struct, a tuple
struct, a branded newtype, an enum and an alias are all answered the same way and before any of
them is split off. It refuses in both directions:

- an entry naming nothing the item declares — refused in **every** feature configuration, spanned
  on the entry, since a misspelled parameter fills nothing while the one it was meant for keeps no
  default;
- a type parameter with no entry — refused only under `#[cfg(feature = "jsonschema")]`, spanned on
  the parameter, because nothing else reads the default;
- `default_types` on an item with no type parameter, and an empty `default_types()`, both being a
  declaration with nothing to declare;
- an entry filled at a type the field-definition dispatch has no arm for — refused under
  `#[cfg(any(feature = "zod", feature = "jsonschema"))]`, the two surfaces that read a declared
  filling, spanned on the filling. That dispatch takes a name it does not recognise for another
  `#[model_schema]` item, which is right for a sibling declared below and gibberish for `i128`: the
  emission names an `i128_schema` module nothing publishes. Tokens cannot separate the two, so
  `is_undescribable_primitive` refuses only what provably cannot become a sibling — the primitive
  names the language reserves that the dispatch answers for nowhere
  (`i128`, `u128`, `f16`, `f128`). `char` is not among them: it has an arm of its own, and a
  filling at it describes as the one-character string serde writes.

A const parameter earns its own refusal, from `const_parameter_argument_errors` at the same seam
and gated on `typescript`/`jsonschema`: a const handed to a written type as an *argument* stands
where a type is read, so the JSON side would call into a module named after it and the TypeScript
side would write a name its declaration binds nothing for. A const written as an array length is
untouched — that is the one position it renders in, as an unbounded array.

There is no fallback filling. A guessed one produces a document that silently rejects valid
payloads, which is what the declaration exists to prevent.

### String Constraints on a Generic Brand's Declared Default

A branded newtype's own `pattern`/`minLength`/`maxLength` normally have no inner to measure when
the inner is one of the brand's own bare type parameters — a parameter is the opaque value on
every surface, and `z.unknown()` carries no `.min`/`.max`. `branded_constraint_inner_error` reads
the declared default for that parameter instead of refusing outright: `declared_default_field`
resolves the same `default_types` entry `zod_default_block` reads, and `non_string_inner_shape`
asks of it the same question asked of a concrete argument — string-shaped, the checks compose onto
the default; not string-shaped, the refusal names the default rather than the parameter, through
`declared_default_constraint_message`. An entry the declaration left out falls back to `String`,
the same fallback `schema_example_value_type` uses for the identical gap, so the guard alone
requires nothing; only `jsonschema`'s own separate requirement (above) forces every parameter to
declare one.

The checks never reach the factory's own parameter — `branded_zod_inner` renders a bare-parameter
inner with no check appended regardless of genericness, since a caller filling the parameter with
something other than the default (an `ObjectId` schema, say) must not inherit bounds meant for it.
They land once, on `$SchemaDefault`'s argument for that one parameter: `zod_default_block` and
`zod_factory_block` both take a `ZodDefaultInputs` bundling `default_types` with the optional
`(parameter, checks)` pair a branded newtype's own expansion supplies — `None` for every ordinary
generic struct, tuple struct, enum and alias: an enum's own string check reaches its fields and
never a type parameter, and the others carry none. The JSON surface needs no separate change: a bare parameter's document is already the runtime
argument bound to the parameter's declared filling (`json_argument_value`, defaulting through
`declared_filling_json_schema_value`), which `branded_layered_over` narrows with the brand's own
bounds through the same `allOf` a named inner is narrowed through — never the inert `{}` a
parameter with no default at all would describe as.

The serde read holds the same line, being the brand's unjudged read followed by its `validate()`
(section 7, "A read refuses what `validate()` refuses"). `validate()` is at every filling and asks
which one it holds: `validate_at_declared_default`, which `build_branded_validation` emits beside
`type_identity`, compares `type_identity::<T>()` against the inner type with each parameter's
declared default substituted in (`substitute_declared_defaults`, reading
`declared_default_syn_type`), and runs the checks on the value read as that default only where the
two types are one, so the read and `validate()` reach the same one filling `$SchemaDefault` does.
`type_identity` is emitted into the schema module by `embedded_type_identity`, a copy of
`typeid::of` (see `THIRD-PARTY-NOTICES`), so a consumer adds no dependency for it.

### Recovering Decode

`#[model_schema(enable = ["decode_with"])]` gives a type `from_value_with`, and `from_bson_with`
as well under `bson`: serde reads the value with the type's read alone, a walker lists every issue
in it, and a callback supplied with the call rejects the record or fixes it once.
`from_value_piped` and `from_bson_piped` are the same reads over resolvers run in order, each
answering one issue. `enable = ["decode_with", "mongodb"]` adds, under `mongodb`, the type's typed
MongoDB paths, the filters and updates they build, and the operations that read and write its
rows; without the `mongodb` feature the entry adds nothing. What every flagged type shares is a
support library its crate writes once. `README.md`'s "Recovering Decode" and "MongoDB Operations and Typed Filters"
sections are the consumer's view; this is what a change to the emitter has to keep. "The flag"
below is the entry `"decode_with"`, and a flagged type is one that lists it.

- **The `enable` list is read by the parser, in every build.** `enable_arg` in `src/model_schema.rs`
  answers `Enabled { decode_with, mongodb }` into `ModelSchemaArgs::enable`, and refuses, spanned on
  what is written: `"mongodb"` without `"decode_with"`, an entry it does not read, an entry written
  twice, an empty list, and any form but a list of string literals (`ENABLE_TAKES`). `apply_arg`
  refuses the bare `decode_with`, in each of its three spellings, with the `enable` form to write,
  and `KNOWN_ARGS` lists `enable` in its place. A refusal is `ModelSchemaArgs::arg_rejection`,
  raised at `exec_model_schema` ahead of every shape under the prefix ``model_schema: type
  `{name}`:``, and the item expands with nothing of the recovering decode.
  `the_bare_decode_with_flag_is_refused_with_the_enable_list_it_moved_to` and the `enable_refuses_*`
  tests in `src/model_schema/tests.rs` hold each message.
- **What each entry generates, and under which feature.** `"decode_with"` is the recovering
  decode: `from_value_with`, `from_value_piped`, their walkers, the type's own module items and the
  re-export of the `decode_with` library, and under `bson` the BSON source as well.
  `Source::read_by()` takes no argument and answers `Source::GENERATED`, which holds `Bson` under
  `bson` alone, so every flagged type is walked from BSON in a build with `bson`, and
  `bson_issue_method` writes its BSON check issues. `"mongodb"` adds, under `mongodb` and nothing
  without it, the typed paths (`struct_paths` and `enum_paths` are called only for such a type,
  `TypedPaths::default()` otherwise), the operations, which `bson_entry_methods` writes only where
  `Source::entry_methods` hands it `written.mongodb`, and the re-export of the MongoDB library and
  its probe. `Written::mongodb` carries the entry through the walk. The expansion test in
  `recovering_decode/tests.rs` holds a type that lists `"decode_with"` alone, under `mongodb`, to
  `from_bson_with`, `from_bson_piped` and the BSON walkers, and to no name of `OPERATIONS`, no
  `MONGO_FIELDS` and no `MongoFields`.
- **The two initializers write the support libraries, once per crate.** `src/foundation.rs` refuses
  any argument (`refused_arguments`) and hands each macro to `decode_with_foundation` and
  `mongodb_foundation` in `recovering_decode.rs`. The first writes the module
  `__tixschema_decode_with` (`DECODE_WITH_LIBRARY`): `decode_with_library`, which is `path_items`,
  `callback_items`, `handoff_items`, `taken_items` and `bound_items`, and under
  `#[cfg(feature = "bson")]` the three BSON pieces (`bson_bound_items`, `bson_leaf_items`, and
  `bson_path_helpers` as an inherent `impl crate::__tixschema_decode_with::Path`). It names no
  `bson` path in a build without `bson`. So a crate whose types list `"decode_with"` alone names
  `bson`, and lists it, wherever tixschema's `bson` feature is on, and never needs `mongodb`. The
  second writes `__tixschema_mongodb` (`MONGODB_LIBRARY`) under `mongodb`: `mongodb_library`, which
  is `query_items` and `operation_items`. It reads the first library through
  `use crate::__tixschema_decode_with::*;`, so it is written beside it. `MONGODB_LIBRARY`,
  `MONGODB_LIBRARY_NAMES`, `mongodb_reexport`, `mongodb_foundation_probe` and the
  `mongodb_foundation` that writes the module are under `#[cfg(feature = "mongodb")]`, and the
  empty `mongodb_foundation` answers every build without it. The first writes nothing without
  `serde`, so each call stays in a crate whatever it builds with, and no consumer writes a line
  for the BSON pieces. Each writes beside its module a private marker trait implemented for `()`,
  `__TixschemaDecodeWithFoundation` and `__TixschemaMongodbFoundation`, whose one method the probes
  look for. `each_initializer_writes_its_module_and_its_marker` and
  `the_two_libraries_share_no_name` hold them.
- **Each `{type}_schema` module re-exports the libraries, name by name.** `library_reexport`
  writes `pub use crate::__tixschema_decode_with::{...};` from `DECODE_WITH_LIBRARY_NAMES`, whose
  four BSON pieces (`bson_bound`, `bson_leaf`, `bson_remaining`, `same_bracket`) are each under
  `#[cfg(feature = "bson")]`, and, for a type that lists `"mongodb"`, the same from
  `MONGODB_LIBRARY_NAMES` under `mongodb`. A glob does not
  serve: the module reads the author's scope through `use super::*`, and a second glob holding a
  name the first holds makes it ambiguous (E0659), where a name written out shadows the glob as an
  item the module declares does. So every `{type}_schema::Name` path a consumer writes resolves,
  and `reading_the_authors_scope` and `ADDED_TYPE_NAMES` read the module as before. The items
  written for one type alone (`paths.module_items`, `read_whole_items`, `Reach::module_items`)
  stay in its module and name the libraries' items through the re-export.
  `a_type_reexports_its_crates_libraries_by_name` holds it.
- **A crate without its initializer fails with the line to add.** `foundation_probes` writes, beside
  each flagged type, one `foundation_probe` for the `decode_with` library and, for a type that lists
  `"mongodb"` under `mongodb`, one for the second. A probe is an anonymous `const` that brings
  `crate::*` into scope and calls the marker's method on `&()`: with the initializer, the crate
  root's marker answers `()`, which the block-local `FoundationWritten` is implemented for; without
  it, the block-local `Fallback` on `&()` answers `Missing`, and the
  `#[diagnostic::on_unimplemented]` on `FoundationWritten` is the E0277, its message, label and note
  naming the line to write and where. The probe is order-independent: the initializer may be
  anywhere at the crate root, after the `mod` lines included.
  `a_type_probes_for_the_libraries_it_reexports` holds the text.
- **The flag is judged at `exec_model_schema`.** `decode_with_guard_errors` runs there, ungated, so
  the refusals -- a type alias, a type that declares a lifetime, a field that borrows -- are the
  same in every feature combination and are raised before any shape is split off. The emitter
  itself runs under `serde`, from `struct_decode_with` and `enum_decode_with`, and reads the item
  as it is emitted, so a read hook tixschema hangs on a field is one the walker reads through.
- **Every read the flag makes is the type's read alone.** A plain read of a type ends on its
  `validate()` (section 7, "A read refuses what `validate()` refuses"), and a refusal there would
  hide the list. `listed_read` in `judged_read.rs` writes `decode_with_read_unjudged` on every
  flagged type, once, after the first source's methods, and the entry points, `read_whole`,
  `lone_slot_bound` and the enum walkers read `Self` through it. Where a schema surface is on it
  calls the type's `model_schema_read_unjudged` behind the `ReadUnjudged` fallback. With none on,
  a holder that names no check publishes no `validate()` and its own read judges what it holds,
  so the function is serde's derive on a copy of the item, each model below read through its own
  `decode_with_read_unjudged`. A generic type's `impl` asks what that read asks beside
  `Deserialize` of a parameter (`read_under`), at every lifetime, and the function repeats none
  of it: a bound asked by the `impl` and by the function is one the compiler cannot choose
  between (E0283). A list held to a count of its own is read whole through `listed_reader`. A
  type that names a `preprocess` on itself is read through `decode_with_read_corrected`, from a
  value its functions corrected once (section 7, "A payload corrected before it is read").
- **A walker is generic over the issue type.** Each walker method takes a type parameter for the
  issue it builds and an `IssueFromParts` constructor, a function pointer over standard types, so
  one type's walker builds another type's issues at their full path. Within one crate the flagged
  types share one `Issue`, the library's, and every `{type}_schema::Issue` is that type. Across
  crates each crate holds its own library, and what crosses is standard types alone:
  `ExpectedToken` and `IssueFromParts` are aliases of standard types, and `issue_from_parts`
  turns the parts into the `Issue` of the crate whose walker called it. So a type of one crate
  holding a type of another builds and lists the held type's issues at their full path, as two
  types of one crate do.
- **A field's type is walked by what tixschema has seen of it.** `record_declaration` notes at
  `exec_model_schema`, in every build with `serde`, whether a name is a struct or an enum or an
  alias, and what a parameterless alias names. A model type seen above is called as it stands, so
  one with no flag fails the build (E0599), as before. Any other type goes through
  `Walker::asked`: the call is written in a block with `{type}_schema::ReadWhole` in scope, so a
  type with a walker of its own answers with it (an inherent function wins over a trait's) and
  one with none is read whole. `<()>::decode_with_read_whole()` beside the call keeps the import
  and the trait's methods in use where the type answers itself; without it a consumer that denies
  warnings fails on `unused_imports` and `dead_code`. The trait is added to the module only where
  a walker asks for it, and answers for the sources the type's own walkers read.
- **A type that lists `"mongodb"` may hold one that lists `"decode_with"` alone.** Under `bson`
  every flagged type has the BSON walker its holder calls, and the holder's typed paths name no
  held type's module, so nothing asks what a held type lists and nothing is refused for it.
- **What holds the libraries as a consumer meets them.**
  - `tests/foundation_refusals.rs` is the `trybuild` harness. Each file of
    `tests/foundation_refusals/` is a crate of its own, held to the `.stderr` beside it, written in
    the build with every plain feature on, the one the harness is gated on: a missing initializer,
    each refusal of the `enable` list, an initializer handed an argument, and a type that lists
    `"mongodb"` with a map keyed by a number. The initializer
    written twice, `tests/foundation_refusals/written_twice/main.rs`, is checked by cargo's JSON
    messages instead: an `E0428` names each item it writes, and no error but `E0428` and `E0119`
    is reported. rustc 1.99 reports the second marker's `impl` as `E0119` and 1.100 does not.
  - `tests/crates_apart/` holds two crates, path dev-dependencies of this package, each writing its
    own libraries: `stored_customers` declares a stored `Customer`, and `invoice_lookups`, whose
    types list `"decode_with"` alone, lists no `mongodb` in its manifest and builds under
    `deny(warnings)` where tixschema has `mongodb` on. `tests/crates_apart_tests.rs` reads a
    `Customer` inside a type of its own and lists an issue in it at its full path, from JSON and
    from BSON, and reads the types of `invoice_lookups` through that crate's library.
  - `tests/support_library_tests.rs` holds one library per crate: three types re-exporting it and
    one resolver serving the reads of all three, what a type that lists `"mongodb"` gains under each
    feature and a type that lists `"decode_with"` alone does not, the BSON decode of a type that
    lists `"decode_with"` alone (`a_type_that_lists_decode_with_alone_decodes_bson`), the issue of
    such a type held by one that lists `"mongodb"`, at its full path
    (`a_stored_holder_lists_the_issue_of_a_held_type_that_lists_decode_with_alone`), and the
    author's own `Path` and `Filter` beside the library's.
  - `examples/strict_consumer_models`, a library with models in a public and a private module, and
    `examples/strict_consumer_app`, a binary with models of its own, each write both initializers
    and are linted by this package's own clippy table, pedantic and restriction included. Both
    require the `mongodb` feature.
- **An alias seen above is walked as the type it names.** `aliases::Reach` writes the field's type
  out again in a clone of the item the walker is then run on: a standard type by its full path,
  and every other type under an alias in `{type}_schema::decode_with_at{N}` that reaches it from
  the field's own type (`<<Counts as IntoIterator>::Item as EntryOf>::Value`) and keeps its last
  name, which is what the walker reads a type by. Nothing the alias was written with has to
  resolve beside the field. A `const _: fn(Written) -> WrittenOut = |held| held;` beside the type
  holds the two to one type.
- **A field's bound is the walker's to list.** `generate_field_validation` records each validator
  it publishes under the `Validator` gate (`record_field_validator`, by schema module and helper
  stem, with the name the validator writes in front of a violation, an `Option`), `walked_field`
  reads it back (`recorded_validator`, into `FieldValidator::named`), and `Walker::bounded` writes
  one `{source}_bound` call beside the read of each value the bound reaches: the validator run on
  what serde read, one `Invalid` per value, its violations joined by `; `, that name taken off
  each sentence. The name is the steps serde writes on the way to the field, read from the
  registry and computed nowhere else, so the two cannot drift, and the validator of a value
  serde writes itself, the one slot of a struct and of a variant under a tag alone, writes none:
  its sentence is listed as it is.
  `recovering_bound_check` builds the closure from the walk's own type at
  that position. A list is that position: the arm
  that walks its items hands them no validator, and holds the list read whole (`read_as_a_list`)
  to its count, one `Invalid` at the list. The registry is cleared per item at `exec_model_schema`, and holds nothing in a
  build with no schema surface, where no validator is published.
- **A check is the read's to list, after serde.** `recovering_decode/checks.rs` emits
  `decode_with_check_messages(&self)` in an `impl` of its own beside the walkers, under the bounds
  the type's author wrote (`RecoveringDecode::check_messages`, `checks::type_impl`): the members'
  messages, the type's own, then those of every model it holds, each under the place serde writes it
  (`member_steps` and `variant_steps`, the two functions `validate()` names a member by). A held
  type is asked through the block-local `Unchecked` trait, so one with no such method answers
  nothing, and a map's key is written through `WrittenKey`. The checks are read back from the
  registry `record_item_checks` and `record_member_checks` write, by schema module and helper stem.
  `entry_methods` and `bson_entry_methods` splice the call after each read serde accepts, in the two
  shared fragments, and `decode_with_value_check_issues` and, under `bson`,
  `decode_with_bson_check_issues`, which `bson_issue_method` writes for every flagged type, turn
  each message into an `Invalid`, `expected: Unknown`, at the message's lead read against the raw
  value. The three names are in the list
  `every_added_method_but_the_entry_point_carries_the_flags_name` pins.
- **Every name the flag adds is its own.** Each walker method carries the flag's name
  (`decode_with_value_issues`, `decode_with_bson_fields`, `decode_with_value_named`), and a
  method's own type parameters take a name the item does not write (`unclaimed_parameter`): `D`,
  `F` and `I`, and under `mongodb` `U`, numbered where the item writes one. The parameters a struct
  of paths declares beside the type's own are named the same way: `Root`, and `Whole` and `Value`
  on a type serde writes as one value. An entry point and an operation are the name they are
  called by (`from_bson_piped`, `find_one`), and the typed paths carry the feature's
  (`MONGO_FIELDS`, `mongo_write_{field}`, `mongo_read_row`).
  `every_added_method_but_the_entry_point_carries_the_flags_name` pins the whole list in order:
  a function of the author's own under one of those names is a duplicate definition (E0592). The
  libraries' modules and markers are named for tixschema (`__tixschema_decode_with`,
  `__TixschemaDecodeWithFoundation`), and a probe's names are local to its anonymous `const`.
- **Both entry points of a source share its two reads.** `entry_methods` and `bson_entry_methods`
  build the first read and the second read once, as token fragments, and splice them into the
  callback entry point and into the piped one, so neither read can change for one and not the other.
  `resolver_items` writes `Resolution`, `Resolver` and `unsettled`, the pipe, beside `Verdict`, in
  the `decode_with` library. `Resolver` is `&dyn Fn + Sync`: a read that waits for MongoDB holds its
  resolvers, and without the bound its future is not `Send`, which clippy's `future_not_send`
  reports at every flagged type of a consumer that denies it.
- **What `mongodb` adds is emitted under `#[cfg(feature = "mongodb")]`, for a type that lists
  `"mongodb"`, and nothing of it in any other build or for any other type.** `query.rs` writes the
  query types into the MongoDB library (`query_items`, spliced by `mongodb_library`).
  `fields.rs` writes `MongoFields` into the type's own module with one function per field, and
  onto the type `MONGO_FIELDS` and one `mongo_write_{field}` per hooked field (`struct_paths`,
  `enum_paths`, called only where `Written::mongodb` holds). `operations.rs` writes
  `OperationError` and `Read` into the library (`error_items`, `read_items`, through
  `operation_items`) and the operations onto the type, beside the BSON entry points and under
  their bounds (`methods`, which `bson_entry_methods` calls only where it is handed the entry).
  Each is reached through a `cfg`'d pair in `recovering_decode.rs`, whose other half answers
  nothing, so a type without the flag expands as before, a type that lists `"decode_with"` alone
  gains no operation and no path under any feature, and a type that lists `"mongodb"` in a build
  without `mongodb` gains nothing more than the BSON read every flagged type has under `bson`.
  The query types' items the typed paths of a type call across modules are `pub(crate)`
  (`Field::held_path`, `Filter::held`).
- **A type's paths hold its own keys, and nothing of another type's.** `MongoFields` holds a
  marker and no path. Each named member is a `const fn` of it (`of_functions`) that answers one
  path kind at a key written as a literal, relative to the type: `"total"`, and `"list.0"` where
  the type's own shape puts two keys. A `MongoPath` is that one key. So the size of what is
  generated for a type does not follow what the type holds, and no function builds a tree:
  `tests/mongodb_query_tests/wide.rs` holds four levels of six fields each to clippy's
  `large_stack_frames`. Each binary declares that module outside `cfg(test)`, where the lint
  reports; inside it clippy reports nothing for the same types. A tuple of slots stays a tuple of
  paths, a position being a member access.
- **`nest` is the one way below a nested type, and two standard traits are what crosses.** A
  path kind carries the model `nest` reaches as its last parameter, `N`: the field's type read
  through `Option`, the list and the wrappers `through_wrappers` reads, which `reached_through`
  writes only where it is not the value an operator takes. `nest` takes a filter over the rows
  of `N` or an update of them by the bound `Nests<Root, N, Kind>`, which the MongoDB library
  implements twice over the two markers a filter and an update carry (`AsFilter`,
  `AsUpdate`), and writes
  each key under the path's own (`nested_filter`, `nested_update`, `nested_key`). No expansion
  names another type's module: a flattened model and a variant that holds one at the enum's own
  level are a `Nested<Root, N>` at the key they are written under.
- **Every `impl` of a struct of paths is written beside the type, and the schema module and
  `mongo_members` name no type, trait or bound of the author's.** A module written inside a
  function body reaches none of that function's items, so the two modules hold declarations
  only, under the parameters `Own::bare` writes: the row type, then the type's own with no bound
  but `?Sized`, no default and no `where` clause.
  - `of_functions` declares a struct of functions in its module and writes its one `impl` beside
    the type (`TypedPaths::beside`), under `Own::writing`: every member's function, the
    `is_{variant}` asks, and the arm functions of an enum under a tag.
  - A tuple's slots keep their positions. `of_slots` declares a trait beside the struct, with one
    associated type per slot (`Slot0`, `Slot1`, ..): `MongoSlots` for `MongoFields`, and for a
    struct of `mongo_members` `{Struct}Slots`, under a name the item does not write. The struct
    reads each slot as `<() as Trait<..>>::SlotN` and declares `where (): Trait<..>`, and the
    `impl` of the trait for `()` beside the type fills each slot under the author's bounds
    (`Own::declared`), so a slot whose type needs a bound to be well formed builds. The
    `MongoFields` `Own::keeping` writes repeats the predicate of the `Members` it holds.
  - `Own::keeping` and `Own::whole` keep `built`, `Deref` and `regex` in the module: they name
    only the module's own items, under the bare parameters.
  - A hooked path writes through `mongo_write_{field}`, a function of the type's own that calls
    the hook as the attribute writes it. In an `impl` beside the type `Self` is the struct of
    paths, so `Own::naming_itself` writes the type's own name wherever its author wrote `Self`:
    in the value of a path, `children: Vec<Self>` on `Node` being a `Field<Root, Vec<Node>>`,
    and in a bound of the type's own. `mongo_write_{field}` is in the type's own `impl` and
    keeps `Self` as written.
  - `tests/mongodb_query_tests/function_body.rs` declares each shape inside a test function, a
    holder of a type declared there under `not(feature = "jsonschema")`, whose JSON Schema
    names the held type's module.
- **A type serde writes as one value is one path.** `written_as_one_value`, recorded as
  `Declared::OneValue`: its `MongoFields<Root, .., Whole>` holds the path at the empty key and
  dereferences to it, the path type being a parameter the type's own `impl` writes, so its module
  names no type of its author's and builds inside a function body. Held by another type it is a
  plain kind over the value, a `Field` or an `OptionalField`.
- **A member is a model only where its walker is called.** `Own::seen_model` reaches a struct or
  an enum `#[model_schema]` was written on above (`declared`), never the type itself, one
  declared below, an alias or another crate's type: each of those is one whole value, and so is a
  field under a hook or one serde never reads back, where no walker is called either. `nest`
  reaches below a whole value as it does below a model.
- **An `is_{variant}` keeps its name.** `enum_paths` claims the names of the asks first, and the
  function of a variant that meets one is numbered by `unclaimed`. A struct of paths carries no
  inherent function but the members' own, the asks and, on an enum under a tag key, the arm
  functions, so a field named `built`, `plain`, `key` or `nest` builds. `MongoFields::built` is
  on the struct of a one-value type, and of a tuple that keeps a marker beside its slots
  (`Own::keeping`), neither of which has a field function.
- **An enum under a tag key has one filter per arm, and a name two functions would take is
  refused.** `tag_arms` in `recovering_decode/enums.rs` is the one seam: for an enum
  `Tagging::of` reads as `Internal` or `Adjacent` it answers the tag key and one `TagArm` per
  variant serde writes, with the value serde writes for it and the name of its function,
  `<tag>__<value>`, each side through `snake_form`. `written_variants` beside it is where a
  variant's written name and the stem of its `is_{stem}` are read, for every enum: `fields.rs`
  computes neither.
  - `Emitter::arms` writes the functions into the `impl` of the struct of paths **beside the
    type**, with its other functions, so the model an arm holds is named as its author wrote it
    and an enum declared inside a function builds. `Own::naming_itself` writes `Self`
    as the enum there. Each is a `const fn` that answers `ArmFilter<Root, M>`.
  - `ArmFilter` (`query.rs`, in the MongoDB library) holds the tag, the value, the arm's
    place and the conditions, and no document until it is turned into one. The place is
    `MongoPath::ROOT` under a tag written beside the fields and the content key under
    `content`. Its `and` takes a filter over the rows of `M` by the two standard traits and
    writes it at the place through `nested_filter`. It carries `Into<bson::Document>` and
    `AsRef<PhantomData<Root>>` itself, and `Filter` converts from it.
  - `ArmFilter::update` and `ArmFilter::tag_on_insert` answer `ArmUpdate<Root, M>` (`query.rs`),
    an update of the rows of the enum. `update` and `ArmUpdate::and` take an update of the rows
    of `M` by the two standard traits and write it at the place through `nested_update`, and
    `update` leaves the arm filter's conditions behind. `tag_on_insert`, on both, adds the tag
    to `$setOnInsert` under the tag key, at the enum's own level and never at the place, and
    only where it is called. An arm update carries `Into<bson::Document>` and
    `AsRef<PhantomData<fn(Root) -> Root>>`, that `AsRef` alone, and `Update` converts from it.
    Every merge is `Update::merged_update`, a private function of `Update` that `Update::and`
    calls too: `an_update_and_an_arm_update_merge_through_one_function` holds the loop to being
    written once.
  - `M` is what `nest` reaches below a path over the variant's one slot (`arm_model`, through
    `taken` and `through_wrappers`), and `()` for any other variant and for a map. No registry
    is asked, so a model declared above, one declared below and one of another crate are one
    case.
  - `Arm`, in the enum's own module, has one unit arm per variant that has a function, under
    the variant's ident, with no serde attribute. Nothing emitted reads it.
  - `tag_arm_refusals` refuses what would let two functions of the struct take one name: `__`
    in a variant's Rust name, two values with one snake case, and an arm function that meets an
    `is_{variant}`. `decode_with_guard_errors` raises them under `serde` for every enum that
    lists `"decode_with"`, so the verdict is the same with the `mongodb` feature on and off and
    with the entry `"mongodb"` listed or not, and an enum without `"decode_with"` is refused
    nothing.
  - A name that is no identifier, under a tag key whose snake case starts with a digit, is
    written as no function and no arm (`function_ident`).
- **A type that lists `"mongodb"` holds no map whose key the `bson` library refuses.** serde
  writes a key of a number or a `bool` as text into JSON only, and the `bson` library refuses to
  write one and reads a stored one back as text, which the key's type refuses. Measured under
  both majors: a `char`, a `chrono` date or time, a plain enum and a brand over a `String` are
  stored as their text and read back, and are not refused. `mongodb_map_key_refusals`, which
  `decode_with_guard_errors` chains where `args.enable.mongodb` holds, raises one error per such
  map, in every build, spanned on the member's ident (a slot's type) and naming the member, behind
  its variant in an enum, and the key as written. A type that lists `"mongodb"` and keys a map by
  its own type parameter is not refused, by decision: no expansion sees what fills the parameter,
  and the README states the error a write and a read of such a row answer at run time.
  - `bson_refused_keys` finds each map (`is_map_wrapper`) a member serde writes or reads holds,
    behind `Option`, a list, a tuple, a transparent wrapper, a map's value, both arms of a std
    `Result` (`written_std_result`, the `Ok` first) and the bound of a std range
    (`written_std_range`), and each model declared above that holds one (`RefusedKey::Held`).
    The two seams leave a type of the crate's own declared above under a std name to
    `held_model_maps`, the model it is. A member or a variant under `skip`
    (`absent_from_wire`) is read past, and so is a member or a variant written and read by its
    author's functions (`written_and_read_by_the_author`, in every build: `with`, or
    `serialize_with` with `deserialize_with`), for a map and a held model alike, so an enum whose
    only such map is in such a variant is not recorded as a holder. One under either of the two
    alone is not.
  - A held model is looked into through `BSON_REFUSED_HOLDERS`, each struct, enum and alias that
    holds such a map at any depth to a description of the first one. It is written in every build
    at `exec_model_schema` (`record_bson_refused_holder_item`), whatever the model's `enable`
    list: for a struct and an enum from the walk `bson_refused_maps` makes, the one
    `mongodb_map_key_refusals` makes, and for an alias from `bson_refused_keys` over the type it
    names, described by `RefusedKey::described`. The refusal names the holder's member, the model
    or the alias, and that map. A member that holds the item itself is read past, its maps being
    the item's own. A model or an alias declared below, or of another crate, is not looked into.
  - `is_bson_refused_key` answers for a key, read under `Box`, `Rc`, `Arc` and `Cow`: one of
    `BSON_REFUSED_KEY_TYPES`, or a name `BSON_REFUSED_KEYS` holds. That registry is written in
    every build at `exec_model_schema` (`record_bson_refused_key_item`), for a brand or an alias
    over such a key, so a chain forwards its target's answer. A brand or an alias whose value
    reaches one of its own type parameters is written instead into `FORWARDED_KEY_PARAMETERS`,
    with that parameter's position among its type parameters (`forwarded_parameter_position`),
    and a key written with its name is answered for the argument at that position: `Ordinal<u32>`
    is refused, `Ordinal<String>` and a bare `Ordinal` are not. The value reaches the parameter
    written bare, under `Box`, `Rc`, `Arc` and `Cow` (`Boxed<T>(pub Box<T>)`,
    `type Held<T> = Box<T>`), and through a brand or an alias recorded there, at the position it
    forwards (`Rank<T>(pub Ordinal<T>)`). A name declared below, or of another crate, is not
    refused, and a brand reached through one declared below forwards nothing.
  - A held model or alias is read at the arguments it is written with (`held_model_maps`). A walk
    asks one thing of each key (`KeyAsked`): `Refused`, or `Parameter(at)`, the holder's own type
    parameter at that position. `FORWARDED_KEY_PARAMETERS` also holds, for each struct, enum and
    alias that keys a map by its own type parameters, the position of each such parameter and the
    first map it keys, written in every build at `exec_model_schema`
    (`record_forwarded_key_maps_item`): `first_held_map` asks `Parameter(at)` of a copy of the
    item whose members, or whose alias's value, spell each parameter as its
    `parameter_placeholder` (`with_parameters_placed`), through `bson_refused_maps` for a struct
    and an enum and `bson_refused_keys` for an alias, the walks `record_bson_refused_holder_item`
    makes. `filled_in` writes a holder's arguments in their place in one pass, so the map is named
    at what fills each parameter on the way: `ByKey<u32>` over
    `type ByKey<KeyType> = HashMap<KeyType, String>`, and `Rekeyed<u32>` over
    `type Rekeyed<K> = Keyed<K>`. One name holds one record, so an item whose value reaches one of
    its parameters (`forwarded_parameter_position`) keeps the position
    `record_bson_refused_key_item` writes and records no map. A held model whose argument at such
    a position is the key a walk asks for, and that holds no such map of its own
    (`BSON_REFUSED_HOLDERS`), is one `RefusedKey::Held` named as written (`Keyed<u32>`). A model
    `is_declared_ident` answers for has each type argument walked as the member's own type, and
    each map found there is a `RefusedKey::Held` named as written. Every argument is read as one
    the model writes into its row: a map an argument holds is refused even where the model holds
    that argument only in a `PhantomData` or in a member serde never writes, an assumption the
    owner reviews. A model or an alias declared below, or of another crate, is not read at its
    arguments.
  - `tests/foundation_refusals/number_keyed_map_with_mongodb.rs` holds the README's `Roster` to
    the error, `held_number_keyed_map_with_mongodb.rs` beside it a held model's, the unit tests
    in `src/model_schema/tests.rs` each position, each key, each held model, each held alias,
    each generic model and alias read at its arguments and each member and variant read past, and
    `tests/mongodb_operation_tests/readme/operations.rs` the README's text: the declaration and
    the error, the keys stored as their text, a type that lists `"decode_with"` alone, and a map
    keyed by the type's own parameter, with the error each major answers at run time.
- **A path is under the name serde writes.** `parse_written_renames` reads `rename`, `rename_all`
  and `rename_all_fields` for their `serialize` side in every build. A list whose two sides differ
  is refused by every build that describes the type, a mobile client included:
  `rename_direction_guard_errors` runs under `serde` with any of `typescript`, `zod`,
  `jsonschema`, `dart`, `swift` and `kotlin`, and a build with none of the six takes the list.
  `the_rename_guard_is_compiled_in_every_build_that_describes_a_type` reads those gates off the
  sources, no local build being of the clients alone.
- **Two standard traits join filters and updates across modules.** A `Filter<Root>` is
  `Into<bson::Document>` and `AsRef<PhantomData<Root>>`, and an `Update<Root>` is
  `Into<bson::Document>` and `AsRef<PhantomData<fn(Root) -> Root>>`. `Filter::and`, `Filter::or`,
  `ArmFilter::and`, `ArmFilter::update`, `Update::and`, `ArmUpdate::and`, `ModelList::elem_match`,
  `ModelList::pull`, `nest` and every operation take a filter or an update by those bounds, never
  as the module's own type: that is what lets a filter of another module over the same rows join
  and be taken, a filter over a nested model be nested, and what refuses an update where a filter
  is asked. `a_filter_and_an_update_each_carry_a_marker_of_their_own` pins each bound.
- **The operations hand the driver documents, and read each row themselves.** The collection is
  `mongodb::Collection<bson::Document>`: a row is read through `from_bson_piped`
  (`mongo_read_row`) and written through `bson::Serializer::new` (`mongo_written_row`), never by
  the driver's serde, and many rows are read with the cursor's own `advance` and
  `deserialize_current`, so a consumer lists no `futures`. `WriteError` is
  `<bson::Serializer as serde::Serializer>::Error`, one path under both majors. `OperationError`
  converts from neither error, and the generated operations and every example build each member
  by name.
- **A read answers `Read`, never a future of its own.** `find_one`, `find`, `count` and the two
  that take resolvers answer `{type}_schema::Read<'c, T, O>`, which holds the function that reads
  one row, so the module names no type of its author's. It takes the five options, and
  `IntoFuture` is implemented once per answer (`Vec<T>`, `Option<T>`, `u64`), each handing the
  driver's method the options that method has. What is awaited is a boxed `Send` future.
  `insert_one` writes the row when it is called and answers a boxed `Send` future too: an
  `async fn` over `&self` is `Send` only where the type is `Sync`.
- **The lists of what the flag adds are pinned.** `DECODE_WITH_LIBRARY_NAMES` is every name the
  `decode_with` library makes public in the build, the four BSON pieces under their own `cfg`, and
  `MONGODB_LIBRARY_NAMES`, under `mongodb`, every name the MongoDB library makes public: each is
  what a `{type}_schema` module re-exports, and
  `the_decode_with_library_names_are_every_name_it_makes_public` and
  `the_mongodb_library_names_are_every_name_it_makes_public` hold each list to the items its library
  declares. `ADDED_TYPE_NAMES`, with `ADDED_TYPE_COUNT`, is every type name the schema module of a
  type that lists both entries holds, the ones it re-exports and the ones written for it alone, the
  ones `mongodb` adds under their own `cfg`: `reading_the_authors_scope` reads past those names, and
  `every_type_the_flag_adds_is_a_name_read_past` holds the list to the libraries and to what the
  module of a struct, that of a tuple struct and that of an enum under a tag declare.
  `every_added_method_but_the_entry_point_carries_the_flags_name` is every method the flag adds to
  the type, and `OPERATIONS` in `recovering_decode/tests.rs` is what
  `mongodb` adds to it. A name a library, a module or a type gains joins its list in the same
  change.
- **The emitted code names only what both major versions of the `bson` library have.** Every BSON
  value is read through `bson::Deserializer::new` and written through `bson::Serializer::new`.
  `tests/recovering_decode_bson_tests/` is compiled once per major, each time beside a MongoDB
  driver built for that major: by `tests/recovering_decode_bson2_tests.rs` here, which binds the
  name `bson` to version 2 with an `extern crate`, and by
  `bson3/tests/recovering_decode_bson3_tests.rs` in `bson3/`, a package of its own whose `bson` is
  version 3.
- **`bson3/` mirrors this package's manifest.** One package holds one build of the driver, and one
  build serves one major, so the version 3 tests cannot sit in `tests/`. The shared sources read
  `cfg(feature = "...")` and are linted in whichever package compiles them, so `bson3/Cargo.toml`
  declares every feature under the same name, each turning on `tixschema/<name>`, and carries the
  lint tables entry for entry. It resolves into a lock file of its own, so it also pins every
  advisory floor this manifest pins, the dependencies under the comment `# Transitive dep floors`,
  at that version or above. `bson3/tests/mirrored_manifest_tests.rs` fails when any of the three
  drifts. Every `just` recipe that tests, lints, checks, formats or audits this package runs the
  same line over `bson3/Cargo.toml`.
- **Nothing emitted carries `#[allow]`, `#[expect]` or `#[doc(hidden)]`, with two exceptions.** A
  consumer's lint levels reach what the macro emits into their crate: every added type is
  `#[non_exhaustive]`, no added function takes `impl Trait` as a parameter, and a `Result` is
  written `core::result::Result`. The two exceptions, both on the support libraries, and why:
  - Each `pub use` of a library in a `{type}_schema` module carries
    `#[expect(clippy::pub_use, reason = REEXPORT_REASON)]` (`reexported`). This repository and
    `bson3/` deny clippy's whole restriction group, `pub_use` among it, and so may a consumer; the
    re-export is what keeps every `{type}_schema::Name` path reachable from the crate and from
    another, and an alias per type or a `pub(crate) use` in its place breaks a path written today
    or another crate's callback. A consumer crate built with `cargo build`, with clippy at the
    lint's default level and with clippy's restriction group denied reports neither `pub_use` nor
    an unfulfilled expectation.
  - Each library module, `__tixschema_decode_with` and `__tixschema_mongodb`, carries
    `#[doc(hidden)]`, so the library is not published in the consumer's documentation as a module
    of its own; its items are documented under each `{type}_schema` that re-exports them.
  `nothing_decode_with_emits_suppresses_a_lint_but_the_two_exceptions` in
  `src/model_schema/tests.rs` holds every expansion and both libraries to these two and no other:
  one `#[expect(clippy::pub_use, reason = ...)]` per re-export, and one `#[doc(hidden)]` per
  library module.
- **The MongoDB suites are compiled once per `bson` major, and the live checks stand down.**
  `tests/mongodb_query_tests/` asserts the document each path writes, and
  `tests/mongodb_operation_tests/` the operations: `offline.rs` what needs no server, `live.rs`
  against a real collection. Each is compiled by a binary here, bound to version 2, and by one in
  `bson3/`. A live check reads `TIXSCHEMA_MONGODB_URI`; without it the check passes having run
  nothing, and says so once on the process's own stderr. `just test-mongodb` refuses to stand
  down, and sits outside `all` and `ci`: no MongoDB server comes with a clone or with the CI
  runner.
- **The README's examples are pinned.** Each is held as one text with the code that compiles, and
  what it prints is run: `tests/recovering_decode_bson_tests/readme.rs` (the callback) and
  `readme_resolvers.rs` (the resolvers), `tests/mongodb_operation_tests/readme.rs` with
  `readme/paths.rs`, `readme/operations.rs` and `readme/stored_forms.rs` (the model, every table
  of documents, the operations, what each failure is told as), and
  `tests/mongodb_query_tests/readme.rs` (a hooked date). A table row of a document is read off
  the call by `stringify!`, so the README's cell is the code. A line that ends in the `bson`
  library's own wording is shown under each major, and each binary holds the run to its own
  through its `BSON_MAJOR`. The README's operations run against a collection in
  `live_the_readme_examples_run_against_a_collection`, which stands down like every live check.

### Adding Examples to Types

To add examples to your types for inclusion in Zod schemas:

```rust
/// User profile description
/// ```rust example
/// User {
///     name: "John Doe".to_string(),
///     email: "john@example.com".to_string(),
///     age: 25,
/// }
/// ```
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub name: String,
    pub email: String,
    pub age: u32,
}
```

**Key Points:**
- Use the exact syntax: ` ```rust example` (note the space and `example` keyword)
- The example code must type-check at compile time
- The last expression in the block should evaluate to the type
- A type may write several blocks, one value each. Every block is built, in a scope of its own, and published in the order written; an empty block is skipped
- Examples are **optional** - a type that writes none gets no `schema_examples()` and no `schema_example()`, and publishes `examples: []`
- Examples respect Serde attributes (field names are serialized correctly)

**Generated Code:**
- A `schema_examples()` method is generated that returns `Vec<serde_json::Value>`, one value per block, and a `schema_example()` beside it that returns the first
- The Zod schema of **every** type holds `examples` in its `.meta()`, and none writes an `example` key: `examples: []` where the type writes no block, and the serialized `schema_examples()` where it writes one or more
- Each example is serialized using Serde, so it matches your API's JSON format

```typescript
// no block
export const Plain$Schema: ZodType<Plain> = Plain$RawSchema.meta({
  examples: []
});

// two blocks
export const SeveralExamples$Schema: ZodType<SeveralExamples> = SeveralExamples$RawSchema.meta({
  examples: [{"value":1},{"value":2}]
});
```

**What a change to the emitter has to keep:**
- `extract_examples_tokens` in `src/utils.rs` is the one reader of the blocks, and `schema_examples_methods` the one builder of the two functions: `item_schema_example_method` (a struct, a tuple struct), `enum_schema_example_method` (the five enum shapes) and `build_branded_schema_example` (a brand) all hand it their blocks
- `zod_examples_json` writes the list: the text `"[]"` where the type writes no block, so such a type names no `serde_json` and no function of its own, and `serde_json::to_string(&Self::schema_examples())` where it writes one
- `zod_schema_with_examples` writes the `zod_schema()` of every shape that publishes its binding through `zod_published_binding`: the delegate on a struct, a tuple struct, a unit struct and a tagged or untagged enum, and the module's own method on an alias, which has no `impl` to carry an example on and therefore always publishes `[]`. `zod_examples_injection` puts the list on the exported `const` ahead of its `;` where the item declares no parameter, and on the factory's memoized binding (`const schema = buildX$Schema(…).meta({ … });`) where it declares one, so `X$SchemaDefault` and every other call of the factory carry it
- A brand and a plain enum already write one `.meta({ description })`, and `build_branded_delegate_items` puts `examples` inside it, after the description. A generic brand writes that `.meta` in its builder, and its memoized binding takes no second one
- A renamed item's re-export and a generic item's `$SchemaDefault` publish another binding of the same schema and carry no `.meta` of their own
- The const-parameter refusal (`const_parameter_example_errors`) fires where any block is written

## Common Development Tasks

### Adding a New Type Mapping

1. Add variant to `FieldDefType` in [src/field_type.rs](src/field_type.rs)
2. Update `get_field_def()` to detect the new type
3. Add TypeScript generation logic in [src/generation/typescript.rs](src/generation/typescript.rs)
4. Add Zod generation logic in [src/features/zod.rs](src/features/zod.rs) (if `zod` feature)
5. Add tests in appropriate test file (e.g., `tests/primitive_types_tests.rs`)

### Adding a New Feature

1. Add feature to `Cargo.toml` `[features]` section
2. Create feature module in [src/features/](src/features/)
3. Add `#[cfg(feature = "...")]` guards in affected code
4. Update `Features` struct in [src/features/mod.rs](src/features/mod.rs)
5. Add tests that verify behavior with/without the feature

### Adding Tests

Tests are organized by category in [tests/](tests/):

- `basic_tests.rs`: Simple structs, basic types
- `collection_tests.rs`: Vec, HashMap, nested collections
- `enum_tests.rs`: Plain enums, discriminated unions
- `serde_tests.rs`: Serde attribute handling
- `mongodb_tests.rs` / `mongodb_real_tests.rs`: ObjectId support
- `edge_cases_tests.rs`: Complex scenarios, deep nesting
- `semantic_types_tests.rs`: Type aliases

**Test pattern**:
```rust
#[test]
fn test_my_feature() {
    #[model_schema()]
    #[derive(Serialize, Deserialize)]
    pub struct Test { ... }

    let ts = Test::ts_definition();
    assert!(ts.contains("expected output"));
}
```

### Running Single Tests During Development

```bash
# Run a specific test
cargo test test_basic_struct

# Run all tests in a module with output
cargo test basic_tests -- --nocapture

# Run tests matching a pattern
cargo test objectid
```

## TypeScript Generation Pattern

Standard pattern for generating TypeScript files:

```rust
// Define entities enum
pub enum MyEntities {}

impl MyEntities {
    pub fn get_entities() -> (String, Vec<String>) {
        (
            "Generated Types".to_string(),
            vec![
                User::ts_definition(),
                Address::ts_definition(),
                // Add all types here
            ],
        )
    }
}

// Generation function
pub fn generate_ts_schemas(target_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut file_contents = String::from(
        "import { z, type ZodType, type $brand } from \"zod\";\n\
         import type { SomeType } from \"zod/v4/core\";\n\n",
    );
    let (header, type_definitions) = MyEntities::get_entities();

    file_contents.push_str(&format!("/*\n * {}\n */\n\n", header));
    file_contents.push_str(&type_definitions.join("\n\n"));
    file_contents.push('\n');

    fs::write(target_path, file_contents)?;
    Ok(())
}

// Test that runs generation
#[test]
fn test_generate_typescript() {
    generate_ts_schemas("../frontend/src/types/generated.ts").unwrap();
}
```

## Generated Output Understanding

The macro transforms Rust types to TypeScript following these rules:

1. **Type Name Transformation**: None. A type publishes under the Rust ident it is declared with, and `#[model_schema(name = "...")]` is the only thing that moves it.
2. **Field Names**: Respect serde rename attributes (`#[serde(rename = "...")]`, `#[serde(rename_all = "...")]`)
3. **Optional Fields**: `Option<T>` becomes `T | undefined` in TypeScript and `z.union([z.null().transform(() => undefined), type, z.undefined()]).prefault(undefined)` in Zod, accepting both an absent key and an explicit `null`
4. **Arrays**: `Vec<T>` becomes `Array<T>` in TypeScript
5. **Maps**: `HashMap<String, T>` becomes `Partial<Record<string, T>>` in TypeScript
6. **Nested Types**: Reference other types by the name they publish (their ident, or their `name` override)
7. **MongoDB ObjectId**: `ObjectId` becomes `ObjectId` in TypeScript with proper JSON schema validation
8. **ObjectId Serialization**: Uses MongoDB format `{ "$oid": "hex_string" }`
9. **ObjectId Validation**: Includes regex validation for 24-character hexadecimal strings

## Testing Best Practices

### 1. Always Test TypeScript Generation

Include generation tests in your test suite to catch issues early:

```rust
#[test]
fn test_generate_typescript() {
    generate_ts_schemas("../frontend/src/types/generated.ts").unwrap();
}
```

### 2. Validate JSON Schemas

When using the `jsonschema` feature, test that generated schemas are valid:

```rust
#[cfg(feature = "jsonschema")]
#[test]
fn test_json_schema_validity() {
    let schema = MyType::json_schema();
    assert!(schema.get("type").is_some());
}
```

### 3. Test Serialization Roundtrips

Ensure serde serialization/deserialization works correctly:

```rust
#[test]
fn test_serde_roundtrip() {
    let original = MyType { /* ... */ };
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: MyType = serde_json::from_str(&json).unwrap();
    assert_eq!(original, deserialized);
}
```

### 4. Version Control Generated Files

Consider committing generated TypeScript files to version control for easier code review and frontend development without requiring Rust builds.

### 5. CI/CD Integration

Run generation tests in your CI pipeline to ensure frontend types stay in sync with backend changes (see [CI/CD Integration](#cicd-integration) section).

### 6. MongoDB ObjectId Testing

The crate includes comprehensive ObjectId tests with real MongoDB library integration (dev-only dependency). Test various ObjectId scenarios:

```rust
#[cfg(all(feature = "mongodb", test))]
#[test]
fn test_objectid_types() {
    #[model_schema()]
    #[derive(Serialize, Deserialize)]
    pub struct Test {
        pub id: ObjectId,
        pub tags: Vec<ObjectId>,
        pub metadata: HashMap<String, ObjectId>,
    }

    let ts = Test::ts_definition();
    assert!(ts.contains("ObjectId"));
}
```

### 7. Complex Structure Testing

Test deeply nested structures and edge cases to ensure macro robustness:

```rust
#[test]
fn test_complex_nested() {
    #[model_schema()]
    #[derive(Serialize, Deserialize)]
    pub struct Complex {
        pub nested: Vec<HashMap<String, Vec<Other>>>,
    }

    let ts = Complex::ts_definition();
    // Verify correct nesting
}
```

### 8. Production Safety

- MongoDB dependency is dev-only, ensuring zero production overhead
- Feature flags allow minimal dependency footprint
- All type analysis happens at compile time (no runtime cost)

## CI/CD Integration

A change is held to the gate, [.github/workflows/ci.yml](.github/workflows/ci.yml), on every push and pull request to `master`. It is `just all`, three lines with every feature on:

1. `cargo clippy --all-targets --all-features -- -D warnings`
2. `cargo build --all-features`
3. `cargo test --all-features`
4. Discord notification with build status

The full run, [.github/workflows/full.yml](.github/workflows/full.yml), is started every three days and by hand. No change waits for it:

1. `cargo build --verbose`
2. `just check` (cargo check + clippy)
3. `just lint-sets` (clippy over every combination of the feature sets, in this package and in `bson3/`)
4. `cargo test --verbose` (basic tests)
5. `just test-sets` (every combination of the `web`, `mobile` and `mongo` feature sets via cargo-hack, in this package and in `bson3/`)
6. `just typecheck-ts` (the emitted TypeScript through a real `tsc --strict`, and a second package compiled against the declarations emitted for it)
7. `just test-slow` (the emitted Node, Dart, Swift and Kotlin clients run under their own toolchains, through `test-emitted`, with the tests that wait on a real clock)
8. `just audit` (`cargo audit` over a lock file generated for each package, this one and `bson3/`)
9. Discord notification with build status

The gate builds one feature selection of one package. What only the full run builds: a build with `serde` and no schema surface (the `mobile` and `mongo` sets), a build without `serde`, and `bson3/`, the package that tests against version 3 of the `bson` library. A break there is found at the next full run.

Both workflows install every toolchain the emitted-client groups need: Node with `zod`, `ws` and `typescript`, Dart, and the Kotlin jars; Swift, Kotlin and a JRE come with the runner image. Inside a plain `cargo test` those groups stand down and pass when their toolchain is missing, so only `just typecheck-ts` and `just test-emitted` prove anything: both refuse to stand down, and both are in the full run, `test-emitted` through `test-slow`.

The MongoDB operations' live checks stand down the same way where `TIXSCHEMA_MONGODB_URI` names no server, and CI names none: `just test-mongodb` is the recipe that refuses to, and no step of either workflow runs it.

**Before pushing**, run `just all`.

## MongoDB ObjectId Support

The `mongodb` feature provides comprehensive MongoDB ObjectId type support with proper validation and serialization.

### Basic Usage

```rust
use mongodb::bson::oid::ObjectId;

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Document {
    pub id: ObjectId,                           // Scalar ObjectId
    pub author_id: ObjectId,
    pub tags: Vec<ObjectId>,                    // Array of ObjectIds
    pub metadata: HashMap<String, ObjectId>,    // HashMap with ObjectId values
    pub parent_id: Option<ObjectId>,            // Optional ObjectId
    pub related: HashMap<String, Vec<ObjectId>>, // Nested structures
}
```

### Generated TypeScript

```typescript
export type Document = {
  id: ObjectId;
  author_id: ObjectId;
  tags: Array<ObjectId>;
  metadata: Partial<Record<string, ObjectId>>;
  parent_id: ObjectId | undefined;
  related: Partial<Record<string, Array<ObjectId>>>;
};
```

### Generated Zod Schema

```typescript
export const Document$Schema = z.strictObject({
  id: z.object({ $oid: z.string().regex(/^[a-f0-9]{24}$/, { message: "Invalid ObjectId" }) }),
  author_id: z.object({ $oid: z.string().regex(/^[a-f0-9]{24}$/, { message: "Invalid ObjectId" }) }),
  tags: z.array(z.object({ $oid: z.string().regex(/^[a-f0-9]{24}$/, { message: "Invalid ObjectId" }) })),
  metadata: z.record(z.string(), z.object({ $oid: z.string().regex(/^[a-f0-9]{24}$/, { message: "Invalid ObjectId" }) })),
  parent_id: z.union([z.null().transform(() => undefined), z.object({ $oid: z.string().regex(/^[a-f0-9]{24}$/, { message: "Invalid ObjectId" }) }), z.undefined()]).prefault(undefined),
  related: z.record(z.string(), z.array(z.object({ $oid: z.string().regex(/^[a-f0-9]{24}$/, { message: "Invalid ObjectId" }) }))),
}).meta({
  examples: []
});
```

### JSON Serialization Format

ObjectIds serialize to MongoDB's standard JSON format:

```json
{
  "id": { "$oid": "507f1f77bcf86cd799439011" },
  "author_id": { "$oid": "507f1f77bcf86cd799439012" },
  "tags": [
    { "$oid": "507f1f77bcf86cd799439013" },
    { "$oid": "507f1f77bcf86cd799439014" }
  ],
  "metadata": {
    "template": { "$oid": "507f1f77bcf86cd799439015" }
  },
  "parent_id": { "$oid": "507f1f77bcf86cd799439016" },
  "related": {
    "references": [
      { "$oid": "507f1f77bcf86cd799439017" },
      { "$oid": "507f1f77bcf86cd799439018" }
    ]
  }
}
```

### Key Features

- **Validation**: Regex validation ensures 24-character hexadecimal strings
- **Type Safety**: Full TypeScript type checking for ObjectId fields
- **MongoDB Compatibility**: Matches MongoDB's native JSON format (`{ "$oid": "..." }`)
- **Zero Production Overhead**: MongoDB crate is a dev-dependency only
- **Comprehensive Testing**: Tested with real MongoDB library integration

## File Organization

```
tixschema/
├── src/
│   ├── lib.rs                    # Entry point, macro exports
│   ├── foundation.rs             # decode_with_foundation!() and mongodb_foundation!(): the support libraries, once per crate
│   ├── model_schema.rs           # Main macro logic
│   ├── model_schema/
│   │   ├── checks.rs             # check: the calls validate() makes, the alias refusal
│   │   ├── judged_read.rs        # The Deserialize of a type that publishes validate() or names a preprocess
│   │   ├── preprocess.rs         # preprocess: what an item names, the calls of a read, the refusals
│   │   └── ranges.rs             # The identity check of a std range or a std Result written by its bare name; a flattened one, and one as a map key where no surface runs, refused; a map flattened beside another member refused; serde_json's Value and Map refused by their full path and checked by their bare name
│   ├── check_example.md          # The check example the rustdoc runs and the README shows
│   ├── preprocess_example.md     # The preprocess example the rustdoc runs and the README shows
│   ├── field_type.rs             # Type system (FieldDef, FieldDefType)
│   ├── utils.rs                  # Helpers (naming, docs, serde parsing)
│   ├── features/
│   │   ├── mod.rs                # Feature detection
│   │   ├── serde.rs              # Serde attribute parsing
│   │   ├── serde/
│   │   │   ├── defaults.rs       # The members read as their default, and the JSON of each default
│   │   │   └── read_bounds.rs    # The bounds serde's derive writes on a Deserialize impl
│   │   ├── zod.rs                # Zod schema generation
│   │   ├── jsonschema.rs         # JSON schema generation
│   │   ├── object_id.rs          # ObjectId support
│   │   ├── recovering_decode.rs  # enable: the entry points, the walkers, the two support libraries, the probes
│   │   ├── recovering_decode/
│   │   │   ├── enums.rs          # The walkers of an enum
│   │   │   ├── aliases.rs        # A field typed with an alias, walked as the type it names
│   │   │   ├── checks.rs         # decode_with_check_messages, and its messages as issues
│   │   │   ├── query.rs          # mongodb: Filter, Update and the path kinds, in the MongoDB library
│   │   │   ├── fields.rs         # mongodb: MongoFields, MONGO_FIELDS, one function per field
│   │   │   └── operations.rs     # mongodb: the operations, and OperationError and Read in the library
│   │   ├── swift.rs              # Swift types and their Codable codec
│   │   ├── swift/
│   │   │   ├── preprocess.rs     # preprocess in the Swift read: the payload type, the calls, the keys taken
│   │   │   ├── read.rs           # The two reads of every Swift type: its own, and the one that judges
│   │   │   └── validate.rs       # validate() of every Swift type: bounds, checks, holders
│   │   └── model_schema_prop.rs  # Field attribute parsing
│   └── generation/
│       ├── mod.rs
│       └── typescript.rs         # TypeScript/Zod code generation
├── tests/
│   ├── basic_tests.rs
│   ├── pattern_preprocess_tests.rs       # pattern and preprocess: the Zod text, and every read of a preprocess
│   ├── pattern_preprocess_tests/         # The models, the reads, the chained hooks, the Zod text, the README; the Dart run compiles the models
│   ├── check_tests.rs                    # check, as validate() runs it
│   ├── check_tests/                      # The models, the shared table, the members, the README
│   ├── preprocess_and_check_tests.rs     # The README's section that states the two keys for the five languages
│   ├── preprocess_and_check_tests/       # The parcel the section declares, and the section held to the test of each language
│   ├── serde_default_tests.rs            # A key under #[serde(default)]: the serde read, the text of every surface
│   ├── serde_default_tests/              # The models and the table every surface is held to
│   ├── std_range_tests.rs                # The four std ranges: every surface, validate(), the read, the decode, the README
│   ├── std_range_tests/                  # The models and payloads every surface and client is held to
│   ├── declared_range_tests.rs           # A type of the crate's own named Range keeps its name
│   ├── std_range_refusals.rs             # A range named after another type fails at its identity check, E0308
│   ├── any_json_refusals.rs              # A bare Value that is serde_json's fails at its check, E0080; the crate's own Value builds
│   ├── std_result_tests.rs               # The std Result: every surface, validate(), the read, the decode, MongoDB, the README
│   ├── std_result_tests/                 # The models and payloads every surface and client is held to, and the reproduction
│   ├── declared_result_tests.rs          # A type of the crate's own named Result keeps its name
│   ├── std_result_refusals.rs            # A Result named after another type fails at its identity check, E0308
│   ├── bound_text_refusals.rs            # A length on a bool, owned or behind a reference, fails with E0277
│   ├── collection_tests.rs
│   ├── enum_tests.rs
│   ├── serde_tests.rs
│   ├── mongodb_tests.rs
│   ├── mongodb_real_tests.rs
│   ├── recovering_decode_tests.rs        # from_value_with, from_value_piped
│   ├── recovering_decode_bson2_tests.rs  # from_bson_with, from_bson_piped, against bson 2
│   ├── recovering_decode_bson_tests/     # The BSON suite both majors compile
│   ├── mongodb_query_bson2_tests.rs      # The typed paths, filters and updates, against bson 2
│   ├── mongodb_query_tests/              # The query suite both majors compile
│   ├── mongodb_operation_bson2_tests.rs  # The operations, against bson 2
│   ├── mongodb_operation_tests/          # The operation suite both majors compile: offline, live, readme
│   ├── support_library_tests.rs          # One library per crate, what "mongodb" adds per feature, the author's scope
│   ├── support_library_tests/            # The three parts: one library, stored types, the author's own names
│   ├── foundation_refusals.rs            # The trybuild harness: each case below fails with its .stderr
│   ├── foundation_refusals/              # The cases: a missing initializer, the enable refusals, a held type missing "mongodb", a map keyed by a number under "mongodb"
│   ├── crates_apart_tests.rs             # Types of the two crates apart, held and read from here
│   ├── crates_apart/
│   │   ├── stored_customers/             # A crate apart: a stored Customer, both initializers
│   │   └── invoice_lookups/              # A crate apart that lists no bson and no mongodb: "decode_with" types alone
│   ├── edge_cases_tests.rs
│   ├── semantic_types_tests.rs
│   └── ...
├── bson3/                        # A package of its own: bson 3, and a MongoDB driver built for it
│   ├── Cargo.toml                # The root's features and lint tables, mirrored
│   ├── src/lib.rs                # Empty: the package holds tests only
│   └── tests/
│       ├── mirrored_manifest_tests.rs        # Fails when the two manifests drift
│       ├── mongodb_operation_bson3_tests.rs  # The operations, against bson 3
│       ├── mongodb_query_bson3_tests.rs      # The typed paths, filters and updates, against bson 3
│       └── recovering_decode_bson3_tests.rs  # from_bson_with, from_bson_piped, against bson 3
├── examples/                     # A strict consumer, linted by this package's own clippy table
│   ├── strict_consumer_models/   # A library: models in a public and a private module, both initializers
│   └── strict_consumer_app.rs    # A binary: models of its own, both initializers
├── justfile                      # Task runner (commands)
├── Cargo.toml                    # Dependencies, features
└── README.md                     # User documentation
```

## Common Pitfalls

1. **Type name ambiguity** → Ensure Rust type names and TypeScript output names don't collide with built-in types
2. **Missing required derives** → Compilation errors
3. **Non-string HashMap keys** → Compilation errors
4. **Forgetting to add types to entities enum** → Types not included in generated output
5. **Using Zod v3** → Generated schemas use v4 syntax and won't work
6. **Testing without feature combinations** → May break in different feature configurations
7. **Declaring `u64`/`usize` under `swift` or `kotlin`** → Refused at expansion: neither target has a mapping for an unsigned 64-bit or pointer-sized integer. Use `i64`, or `u32` where the range allows
8. **A member named after a word Dart, Swift or Kotlin reserves** → Written in the form the language accepts, the wire key unchanged: `dart_member` adds a trailing underscore (Dart has no escape), `swift_member` and `kotlin_property_name` write backticks. `swift_property` also declares a field named `validate` as `validate_`, the method every emitted Swift type has. Each list (`DART_RESERVED`, `SWIFT_RESERVED`, `KOTLIN_HARD_KEYWORDS`) holds only words the real compiler refused as a member name, so a name that compiled before is emitted as before. A longer identifier built from a member's name (`wireFor`, `forKeys`) reads it through `swift_bare`/`kotlin_bare`. The service clients name a method, a parameter and a request's member through the same seams (`dart_local`/`dart_field_member`, `swift_member`/`swift_field_member`, `kotlin_name`/`kotlin_property_name`), and every name the service code reads as text goes through `parse::written`, which drops a raw identifier's `r#`; `parse::field_ident` writes it back where a keyword-named field is read in Rust. The TypeScript emitters name a header or part argument through `message::parameter_name` and a placeholder's local through `message::local_name`, which add a trailing underscore to a word `tsc` refuses as a parameter or a local in a module (`REFUSED_AS_A_NAME`, each one measured)
9. **An argument or placeholder named after a local the generated code writes around it** (`path`, `headers`, `body`, `request`, `message`) → Never read as that local, and never a change to what a caller writes. The Rust transports hold a header or part argument in `parse::argument_local` (`argument_{name}`), bound there by a dispatcher and moved there by a client's method ahead of its own locals, and a path placeholder in `placeholder_{name}`. Each client emitter holds a `TAKEN_BY_A_METHOD` list of the locals and parameters its methods write: TypeScript (`message::parameter_name`) and Dart (`dart_parameter`) move a positional argument's name, Swift keeps the label and moves the name the method reads it by (`swift_parameter_declared`), Kotlin keeps the parameter and copies it first (`held_copies`, `kotlin_held`). A local added to an emitted method goes into that emitter's list

## Debugging Tips

```rust
// Print generated TypeScript during tests
let ts = MyType::ts_definition();
println!("Generated:\n{}", ts);

// Print JSON schema
#[cfg(feature = "jsonschema")]
println!("{}", serde_json::to_string_pretty(&MyType::json_schema())?);

// Check which features are enabled
#[cfg(test)]
use crate::features::Features;
println!("Enabled: {:?}", Features::enabled_features());
```

## Performance Considerations

- Macro expansion happens at compile time (zero runtime overhead)
- Generated methods return `String` (consider caching if called frequently)
- Feature flags reduce dependencies and compilation time when not needed
- MongoDB ObjectId validation regex is compiled once by Zod

## Related Documentation

- [README.md](README.md) - User guide, examples, troubleshooting
- [justfile](justfile) - All available commands with descriptions
