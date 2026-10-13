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
```

## Code Architecture

### Macro Processing Flow

1. **Entry Point** ([lib.rs](src/lib.rs))
   - Exports two proc macros: `#[model_schema]` and `#[model_schema_prop]`
   - `model_schema` is the main macro for structs/enums/type aliases
   - `model_schema_prop` is for field-level customization

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
   - `recovering_decode.rs`: Generates what `#[model_schema(decode_with)]` adds to a type: `from_value_with` and `from_value_piped`, `from_bson_with` and `from_bson_piped` under `bson`, the walker of each, and the callback's and the resolvers' types in `{type}_schema`. `recovering_decode/enums.rs` holds the walkers of an enum, and the names its typed paths read a variant by (`written_variants`, `tag_arms`). Under `mongodb` three more files emit into the same module and onto the same type: `recovering_decode/query.rs` the query types (`Filter`, `ArmFilter`, `Update`, `ArmUpdate`, the path kinds, `nest`), `recovering_decode/fields.rs` the type's own typed paths (`MongoFields`, `MONGO_FIELDS`, one function per field, and one per arm of an enum under a tag), and `recovering_decode/operations.rs` the operations (`find_one`, `find`, `count`, `insert_one`, the updates and the deletes, with `Read` and `OperationError`)
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
- Complex: `SiblingType` (custom types), `Map` (HashMap), `Tuple`
- Special: `ObjectId` (MongoDB, feature-gated), `StringLiteral`, `TypeParam`, `Unknown`

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
- `mongodb`: Enables MongoDB ObjectId type support; turns on `bson`. On a type that opts in with `decode_with` it also generates the typed field paths (`MONGO_FIELDS`, one function per field, `nest` below a nested type, and `<tag>__<value>()` for each arm of an enum under a tag), the filters and updates they build, and the operations over a `mongodb::Collection<bson::Document>` of the type's rows. A consuming crate that declares such a type lists the `mongodb` driver, built for the major version of `bson` it lists
- `bson`: BSON support, turned on by `mongodb`; turns on `serde`. Generates `from_bson_with` and `from_bson_piped` on types that opt in with `decode_with`
- `typescript`: Enables TypeScript type generation
- `chrono`: Enables chrono date/time type support (`NaiveDate`, `NaiveTime`, `NaiveDateTime`, `DateTime<Tz>`)
- `indexmap`: Enables indexmap type support: `IndexMap` as a map, `IndexSet` as a list. It adds no dependency: the generated code names the types of the crate that declares them. With the feature off a type written `IndexMap<_, _>` or `IndexSet<_>` is refused at `exec_model_schema` with the feature named (`indexmap_feature_errors`), unless the crate declared a type of its own under the name above
- `dart`: Enables Dart type generation with a JSON codec, and the Dart HTTP client
- `swift`: Enables Swift type generation with a `Codable` codec
- `kotlin`: Enables Kotlin type generation with `kotlinx.serialization` annotations. A consuming Kotlin build declares two dependencies: the runtime library `org.jetbrains.kotlinx:kotlinx-serialization-json` and the Kotlin Gradle plugin `kotlin("plugin.serialization")`

**Feature sets**: `web` (the default: `serde`, `zod`, `jsonschema`, `typescript`), `mobile` (`serde`, `dart`, `swift`, `kotlin`) and `mongo` (`mongodb`, `chrono`, `indexmap`). CI tests the powerset of the sets (`just test-sets`); `just test` runs the powerset of the plain features locally

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
`BTreeMap`, `IndexMap` under the `indexmap` feature, and `Map`, which is `serde_json`'s, read by its
last name as `Value` is. The field definition, the constraint walk, the walk `validate()` makes to
what a member holds, the recovering decode's walker and its rewriting of an alias, and the MongoDB
paths all ask it, as they ask `is_sequence_wrapper` for a list, which takes `IndexSet` under the
same feature. `IndexMap`, `IndexSet` and `Map` are names of another crate's, so the two functions
ask `is_declared_ident` for the three: a type the crate declared under one of them above keeps the
name, with the feature on or off.
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
- `HashMap<String, T>` → `Partial<Record<string, T>>`
- Custom types → Reference by the name the referenced type publishes (its ident, or its `name` override)
- A unit struct (`pub struct Ping;`, no braces) → `Record<string, never>` (unchanged); the wire is `{}` both ways: the macro replaces the author's own `Serialize`/`Deserialize` derive with impls writing and reading `{}` (serde's own derive writes and reads `null`, refusing `{}`). Kotlin renders `@Serializable object Ping` rather than a `class`; Dart and Swift render their own no-data type. Declared above a service as `Result<Ping, E>`, it reads like a `()` success on every client — a braced `struct Ping {}` is a different shape and is unaffected
- `ObjectId` → `ObjectId` (with $oid validation)
- `DateTime<Tz>` → `Date` (Zod `z.union([z.date(), z.string(), z.number()]).pipe(z.coerce.date())`, which refuses `null` and booleans that `z.coerce.date()` alone reads as 1970, ahead of the `null` arm and the boolean member a union lists after the date; `type_check_a_date_refuses_what_is_no_date_in_every_position_that_holds_one` in `tests/generic_types_tests/declarations.rs` loads the schema under `node` and holds each input to it); with `#[model_schema_prop(as_number)]` → `number` (inline epoch-ms coercer)
- `NaiveTime` → `string` (Zod `z.iso.time()` wrapped in an inline preprocessor that also accepts millis-since-start-of-day)
- `#[serde(flatten)]` field → TypeScript intersection (`A & B`), Zod `.and(...)`. A source writing more than one key set — a registered choice, or one reached through an `Option` — multiplies instead: `zod_merged_joins` builds one `.and(...)` chain per key set, `zod_merged_statements` binds the object to `{Type}$OwnSchema` and joins the copies where the object is a struct's own published binding, and `zod_merged_object` writes the object out once per copy where it is an enum variant's fragment of a larger literal. Either way the item contributes exactly one member to whatever union encloses it
- `#[serde(untagged)]` enum → TypeScript union (`A | B`), Zod `z.union([...])`, JSON Schema `anyOf`
- Type parameter → TypeScript parameter (`X<IdType>`), Zod `X$SchemaFactory(idType)` (memoized, one cache level per parameter) plus `X$SchemaDefault` — that same factory called at the item's own declared `default_types`, checks included — JSON Schema the document of whatever fills it — `default_types(...)` where the type stands alone, the reference site's arguments where a field embeds it. A lifetime is dropped on every surface, a borrowed value writing what its owned form writes; a const renders as an array length only, and is refused where it is handed to a written type as an argument

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

All validation constraints generate checks in **Zod (frontend), JSON Schema, and Rust**. In Rust the check runs in `validate()`, which the type's own `Deserialize` calls on the value it read, so a serde read refuses what `validate()` refuses ("A read refuses what `validate()` refuses", below), and in the recovering decode of a type declared with `decode_with`, which lists each violation as an issue. While the members are still being read it runs in one position only: a member of an untagged enum, where the check decides which variant the payload is:

| Attribute | Field Type | Zod | JSON Schema | Rust |
|-----------|------------|-----|-------------|------------|
| `pattern = "regex"` | `String` | `.check(z.regex(/regex/))` | `"pattern"` | `validate()`, which the type's read runs; the member's own hook too in an untagged enum |
| `minLength = N` | `String` | `.min(N)` | `"minLength"` | `validate()`, which the type's read runs; the member's own hook too in an untagged enum |
| `maxLength = N` | `String` | `.max(N)` | `"maxLength"` | `validate()`, which the type's read runs; the member's own hook too in an untagged enum |
| `minLength = N` | a list | `.min(N)` on the array | `"minItems"` | `validate()`, which the type's read runs; the member's own hook too in an untagged enum |
| `maxLength = N` | a list | `.max(N)` on the array | `"maxItems"` | `validate()`, which the type's read runs; the member's own hook too in an untagged enum |
| `minimum = N` | numeric | `.min(N)` | `"minimum"` | `validate()`, which the type's read runs; the member's own hook too in an untagged enum |
| `maximum = N` | numeric | `.max(N)` | `"maximum"` | `validate()`, which the type's read runs; the member's own hook too in an untagged enum |
| `literal = "val"` | `String` | `z.literal("val")` | `{"type": "string", "const": "val"}` | — |
| `literal = true` | `bool` | `z.literal(true)` | `{"type": "boolean", "const": true}` | — |
| `literal = 214` | numeric | `z.literal(214)` | `{"type": "number", "const": 214}` | — |
| `preprocess = ["fn"]` | any, and a type | `z.preprocess(fn, ...)` | — | every read of the type calls `fn` in snake case on the payload, a `serde_json::Value`, ahead of the value's read |
| `check = ["fn"]` | any, and a type | `.check((ctx) => { ... fn(own.value, own); ... })` | — | `validate()` calls `fn` in snake case, in every build, and the type's read refuses with its messages; an issue of the recovering decode |
| `ts_optional` | `Option<T>` | — | — | — (TypeScript-only) |
| `as_number` | `DateTime<Tz>` | inline `z.preprocess(..., z.number())` | — | serde `with = chrono::serde::ts_milliseconds` injected |
| `nullable` | `Option<T>` | `z.union([T, z.null()])`, key required | `anyOf: [T, {"type":"null"}]`, key required | guard: refuses a key-dropping serde attr |

**A key written on a field measures the field's own value.** `Option` and the transparent wrappers
(`Box`, `Rc`, `Arc`, `Cow`, the borrow of a `str` or of a list) hold one value and are read through;
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
  emits the list's validator and hook under the brand's names, with no `Display` bound. `pattern`
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
or a pattern on a numeric field, a range on a string field, and a pattern or a range on a list,
under `Option` and any transparent wrapper, naming the field and only the keys of the wrong kind. `FieldDef::unmeasured_bound` is the one question the refusal
and `apply_constraint_docs` are both written from, so the TypeScript comment states no bound the
refusal turned away.

A borrowed `str` is held to its bound by `validate()`, as a `String` is, and a borrowed list is
counted as an owned one is. They are the two references `constrained_shape` reads through
(`ConstraintWrap::Borrowed`, which `names_str` and `names_a_list` decide): `validate()` reaches the
`str` through `&'a str` at any depth, and a reference to anything else has no shape and no
validator. A borrowed field publishes its validator and no `deserialize_{field}`
hook (`ends_on_a_borrow`), and `generate_field_validation` hangs nothing on one. A borrowed `str`
member of an untagged enum is therefore the one constrained member the read does not check: it is
read as it is with no bound written, and `validate()` answers for the bound.

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
- A slot of a tagged variant is checked by `validate()` alone and reported where serde writes
  it, as what its type holds is: under the variant's name or the content key, and under its
  position among several slots (`'Code': …`, `'Pair.1': …`, `'data.1': …`). Its validator is
  recorded under the `Validator` gate, and the recovering decode reads it back through
  `slot_validator`: `Slot::validator` for a variant of several slots,
  `WalkedVariant::lone_bound` for a variant of one, each an `Invalid` at the slot's own path.
- A slot of a tuple struct is checked where a named field of a struct is: by `validate()` alone,
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
  `Option`, an ownership wrapper or the borrow of a `str`. A list and its items, a field typed with
  a named type (a brand included), a number, a map, a tuple and a type parameter are not. It is
  `constrained_shape`'s walk held to `ConstraintLeaf::Str` and `ConstraintLeaf::Path`, written
  apart because it is asked in a build without `serde`;
  `the_reach_of_an_enums_check_is_the_constraint_walks` holds the two together.
- A field with a check of its own is held to both: the longer `minLength`, the shorter `maxLength`,
  and the enum's `pattern` in `ModelSchemaPropMeta::enum_pattern` beside the field's own, which
  `patterns()` yields in that order. Zod writes one `.check(z.regex(…))` each, `both_patterns_json`
  states the two in an `allOf`, and the validator runs one check each. A named leaf is never
  reached, so its three readers (`zod_named_checks`, `named_field_json_schema_value`,
  `named_leaf_rendering`) read `pattern` alone.
- `enum_string_check_refusal_output` answers at `process_item`, ahead of every enum shape and in
  every build, so an enum's `pattern` is judged in every build as a field's is. It refuses a
  `pattern` no surface reads, a check that reaches nothing (`enum_check_reaches_nothing`: no
  variant written with no data and no reached field), and each name of a unit variant that breaks
  the check (`unit_variant_name_errors`).
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
- **No other surface moves.** The JSON Schema, TypeScript, Dart, Swift and Kotlin write what they
  wrote, and no expected output of a type without the key changes.
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
  wrappers and each level of a list, into a map's values (`ConstraintWrap::Map`, which
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
  and on no member the type's unjudged read reaches (`member_read_hook`).
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
- `deserialize_{variant}_{field}(D) -> Result<FieldType, E>` — serde hook that calls the static validator, generated only for a member of an untagged enum, the one position where a member's check runs as the member is read
- A tuple variant's slot writes its declaration index where a named member writes `{field}`: `validate_other_0_value` and `deserialize_other_0` for the lone slot of `Other`

#### A read refuses what `validate()` refuses

A type's read is its own unjudged read, followed by its `validate()`. A holder reads the models it
holds unjudged, so the whole value is judged once, by the type the caller asked for. `README.md`'s
"A read refuses what `validate()` refuses" section is the consumer's view; this is what a change
has to keep.

- `src/model_schema/judged_read.rs` is the seam. `of_struct` and `of_enum` are handed the item and
  whether it publishes `validate()`, from `struct_output_with_unit_impls`,
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
  author's, or a type that names a lifetime, `Self` or one of the item's parameters.
- The hooks are functions local to `model_schema_read_unjudged`: `read_unjudged_{stem}`, each
  with a local `Unjudged` newtype whose `Deserialize` calls the leaf's
  `model_schema_read_unjudged`, behind the local `ReadUnjudged` trait. An inherent function is
  found ahead of a trait's, so a type with no unjudged read of its own is read as serde reads
  it. A refusal a brand below raises is answered under the name `validate()` reports the field
  under (`named_refusal`) where `named_below` says the named reader did so, and an outermost
  `Option` under a key is given
  `#[serde(default)]` by the rule of `needs_injected_default`, the container writing none.
- `named_read_hook` stays for a named field `reachable_nested_shape` walks and `reached` does
  not, a declared type in an `Arc` and in a fixed array among them. A model there, in a tuple
  and under a type parameter judges itself as it is read, and ends the read at the first one
  that breaks a rule.
- A unit struct's `Deserialize` is the macro's already (`unit_struct_serde_impls`), and
  `unit_struct_read` gives it `model_schema_read_unjudged` and the `validate()` call where the
  struct has a check.
- `remote`, `from`, `try_from` and `into` (`serde_conversion`) on an item whose read the macro
  writes are refused by `conversion_refusal`, in every build.
- `model_schema_read_unjudged` is the type's own name. Each name the function declares beside
  the item's own types is numbered where the item writes it (`unclaimed_parameter`): the
  reader's type parameter `D`, the copy `ModelSchemaRemote`, the trait `ReadUnjudged`, and the
  `Unjudged` of a hook where the member's type writes that name.
- A service's dispatcher reads a request through `unjudged_{operation}`, which
  `message_readers` in `src/service_schema/support.rs` writes into the service module, and calls
  the validator next. The AMQP and WebSocket arm hands it a `serde_json::Deserializer` over the
  payload and ends it, as `from_slice` does, and the HTTP dispatcher the value it carried.
- `tests/validation_tests/refusing_read.rs` holds each shape, the bounds and the README's
  sample; `tests/check_tests/read.rs` the read of a type that names a check, in every build;
  `tests/recovering_decode_bson_tests/refused_read.rs` a BSON document under each major.

#### A payload corrected before it is read

A `preprocess = ["name", ...]` names functions of the consumer's own, on a field and, in
`#[model_schema(...)]`, on a type: each takes a payload and answers the payload the value is read
from. Rust calls the snake case of the name, `fn trimmed(payload: serde_json::Value) ->
serde_json::Value`, and the Zod schema the name as written, `function trimmed(payload: unknown):
unknown`. The documents call the key a last resort, and a change keeps them saying so: a function
runs on every read, on a payload no schema describes. `README.md`'s "Preprocessing" section is
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
  the judged read where it names a function.
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
- The macro adds no bound to a brand's declaration. The `Display` the read hook renders the value
  through stays on the hook and in the `#[serde(bound(deserialize = ...))]`
  `inject_branded_serde_attrs` writes.
- A consumer's own `validate` on one filling is a duplicate definition (E0592).
  `struct_validate_method`'s doc comment holds the run.

### 8. Branded Newtypes

Single-field tuple structs with `#[serde(transparent)]` are treated as branded/opaque types, and so is a `#[serde(transparent)]` struct with named fields, over the one field serde reads it as: `brand_field` in `src/features/serde.rs` is the one seam every surface asks, the web three and Dart, Swift and Kotlin alike, and `transparent_field` beside it is serde's own rule for which named field that is (read, no `default`, not `PhantomData`). The brand publishes under its Rust ident unless `#[model_schema(name = "...")]` names another, and that name reaches the surface twice: as the exported type and as the brand tag the values carry.

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
- The slot takes no `#[model_schema_prop(...)]`: a brand publishes its inner's own schema with a
  `.brand()` written onto it, so no key written there reaches any surface, and one written there is
  refused at `exec_model_schema` — the ungated seam, so the verdict is the same in every feature
  combination — with the item re-emitted stripped of it. The three checks a brand does carry are
  written on the type: `#[model_schema(pattern = "...", minLength = N, maxLength = N)]`. Over a
  list the two lengths count its items and `pattern` is refused (section 7). The
  field a `#[serde(transparent)]` struct with *named* fields is the value of is that slot, and is
  held to the same
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

The serde read holds the same line: `build_branded_validation`'s generic `deserialize_value` hook
compares `type_identity::<T>()` against the inner type with each parameter's declared default
substituted in (`substitute_declared_defaults`, reading `declared_default_syn_type`), and only
runs the checks when they are equal, so the read reaches the same one filling `$SchemaDefault` and
`validate()` do. `type_identity` is emitted into the schema module by `embedded_type_identity`, a
copy of `typeid::of` (see `THIRD-PARTY-NOTICES`), so a consumer adds no dependency for it.
`validate()` is at every filling and asks the same question: `validate_at_declared_default`, this
crate's own beside `type_identity`, runs the checks on the value read as the declared default
where the two types are one (section 7, "Generated `validate()` method").

### Recovering Decode

`#[model_schema(decode_with)]` gives a type `from_value_with`, and `from_bson_with` under `bson`:
serde reads the value with the type's read alone, a walker lists every issue in it, and a
callback supplied with the call rejects the record or fixes it once. `from_value_piped` and `from_bson_piped` are the same
reads over resolvers run in order, each answering one issue. Under `mongodb` the type also gets
its typed MongoDB paths, the filters and updates they build, and the operations that read and
write its rows. `README.md`'s "Recovering Decode" and "MongoDB Operations and Typed Filters"
sections are the consumer's view; this is what a change to the emitter has to keep.

- **The flag is read at `exec_model_schema`.** `decode_with_guard_errors` runs there, ungated, so
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
  one type's walker builds another type's issues at their full path. Two flagged types share no
  declaration: each `{type}_schema` module declares the same aliases of standard types
  (`ExpectedToken`, `IssueFromParts`), and `issue_from_parts` turns the parts into that module's
  own `Issue`.
- **A field's type is walked by what tixschema has seen of it.** `record_declaration` notes at
  `exec_model_schema`, in every build with `serde`, whether a name is a struct or an enum or an
  alias, and what a parameterless alias names. A model type seen above is called as it stands, so
  one with no flag fails the build (E0599), as before. Any other type goes through
  `Walker::asked`: the call is written in a block with `{type}_schema::ReadWhole` in scope, so a
  type with a walker of its own answers with it (an inherent function wins over a trait's) and
  one with none is read whole. `<()>::decode_with_read_whole()` beside the call keeps the import
  and the trait's methods in use where the type answers itself; without it a consumer that denies
  warnings fails on `unused_imports` and `dead_code`. The trait is added to the module only where
  a walker asks for it.
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
  what serde read, one `Invalid` per violation, that name taken off the sentence. The name is
  the steps serde writes on the way to the field, read from the registry and computed nowhere
  else, so the two cannot drift, and the validator of a value serde writes itself, the one slot
  of a struct and of a variant under a tag alone, writes none: its sentence is listed as it is.
  `recovering_bound_check` builds the closure from the walk's own type at
  that position. A list is that position: the arm
  that walks its items hands them no validator, and holds the list read whole (`read_as_a_list`)
  to its count, one `Invalid` at the list. The registry is cleared per item at `exec_model_schema`, and holds nothing in a
  build with no schema surface, where no validator is published.
- **A check is the read's to list, after serde.** `recovering_decode/checks.rs` emits
  `decode_with_check_messages(&self)` in an `impl` of its own beside the walkers, under the bounds
  the type's author wrote (`RecoveringDecode::check_messages`, `checks::type_impl`): the members'
  messages, the type's own, then those of every model it holds, each under the place serde writes
  it (`member_steps` and `variant_steps`, the two functions `validate()` names a member by). A held type
  is asked through the block-local `Unchecked` trait, so one with no such method answers nothing,
  and a map's key is written through `WrittenKey`. The checks are read back from the registry
  `record_item_checks` and `record_member_checks` write, by schema module and helper stem.
  `entry_methods` and `bson_entry_methods` splice the call after each read serde accepts, in the
  two shared fragments, and `decode_with_value_check_issues` and `decode_with_bson_check_issues`
  turn each message into an `Invalid`, `expected: Unknown`, at the message's lead read against the
  raw value. The three names are in the list
  `every_added_method_but_the_entry_point_carries_the_flags_name` pins.
- **Every name the flag adds is its own.** Each walker method carries the flag's name
  (`decode_with_value_issues`, `decode_with_bson_fields`, `decode_with_value_named`), and a
  method's own type parameters take a name the item does not write (`unclaimed_parameter`): `D`,
  `F` and `I`, and under `mongodb` `U`, numbered where the item writes one. The parameters a struct
  of paths declares beside the type's own are named the same way: `Root`, and `Whole` and `Value`
  on a type serde writes as one value. An entry point and an operation are the name they are
  called by (`from_bson_piped`, `find_one`), and the typed paths carry the feature's
  (`MONGO_FIELDS`, `mongo_write_{field}`, `mongo_read_row`).
  `every_added_method_but_the_entry_point_carries_the_flags_name` pins the whole list in order: a
  function of the author's own under one of those names is a duplicate definition (E0592).
- **Both entry points of a source share its two reads.** `entry_methods` and `bson_entry_methods`
  build the first read and the second read once, as token fragments, and splice them into the
  callback entry point and into the piped one, so neither read can change for one and not the
  other. `resolver_items` writes `Resolution`, `Resolver` and `unsettled`, the pipe, beside
  `Verdict`. `Resolver` is `&dyn Fn + Sync`: a read that waits for MongoDB holds its resolvers, and
  without the bound its future is not `Send`, which clippy's `future_not_send` reports at every
  flagged type of a consumer that denies it.
- **What `mongodb` adds is emitted under `#[cfg(feature = "mongodb")]`, and nothing of it in any
  other build.** `query.rs` writes the query types into the module (`query_items`, spliced by
  `module_items`). `fields.rs` writes `MongoFields` into the module with one function per field,
  and onto the type `MONGO_FIELDS` and one `mongo_write_{field}` per hooked field
  (`struct_paths`, `enum_paths`). `operations.rs` writes `OperationError` and `Read` into the
  module (`error_items`, `read_items`) and the operations onto the type, beside the BSON entry
  points and under their bounds (`methods`, called from `bson_entry_methods`). Each is reached
  through a `cfg`'d pair in `recovering_decode.rs`, whose other half answers nothing, so a type
  without the flag expands as before and a flagged type in a build without `mongodb` gains no
  operation and no path.
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
  of `N` or an update of them by the bound `Nests<Root, N, Kind>`, which each module implements
  twice over the two markers a filter and an update carry (`AsFilter`, `AsUpdate`), and writes
  each key under the path's own (`nested_filter`, `nested_update`, `nested_key`). No expansion
  names another type's module: a flattened model and a variant that holds one at the enum's own
  level are a `Nested<Root, N>` at the key they are written under.
- **A function that calls a hook is written beside the type.** A hooked path writes through
  `mongo_write_{field}`, a function of the type's own that calls the hook as the attribute
  writes it. The schema module cannot name a type declared inside a function, so `of_functions`
  puts every function whose path calls one (`Built::calls_a_hook`) in an `impl` of the struct of
  paths beside the type (`TypedPaths::beside`), and the rest in the module. In that `impl`
  `Self` is the struct of paths, so `Own::scoped` at no depth writes the type's own name
  wherever its author wrote `Self` (`Own::naming_itself`): in the value of the path,
  `children: Vec<Self>` on `Node` being a `Field<Root, Vec<Node>>`, and in a bound of the
  type's own, which the arm functions of an enum under a tag are written under as well.
  `mongo_write_{field}` is in the type's own `impl` and keeps `Self` as written.
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
  - `Emitter::arms` writes the functions in an `impl` of the struct of paths **beside the
    type**, as a hooked path's function is, so the model an arm holds is named as its author
    wrote it and an enum declared inside a function builds. `Own::naming_itself` writes `Self`
    as the enum there. Each is a `const fn` that answers `ArmFilter<Root, M>`.
  - `ArmFilter` (`query.rs`, in every flagged type's module) holds the tag, the value, the arm's
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
    `is_{variant}`. `decode_with_guard_errors` raises them under `serde`, so the verdict is the
    same with `mongodb` on and off, and an enum without `decode_with` is refused nothing.
  - A name that is no identifier, under a tag key whose snake case starts with a digit, is
    written as no function and no arm (`function_ident`).
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
  converts from it through an `impl` bounded on that path; no `From` of the driver's error can
  stand beside that one, so `Database` is built by name.
- **A read answers `Read`, never a future of its own.** `find_one`, `find`, `count` and the two
  that take resolvers answer `{type}_schema::Read<'c, T, O>`, which holds the function that reads
  one row, so the module names no type of its author's. It takes the five options, and
  `IntoFuture` is implemented once per answer (`Vec<T>`, `Option<T>`, `u64`), each handing the
  driver's method the options that method has. What is awaited is a boxed `Send` future.
  `insert_one` writes the row when it is called and answers a boxed `Send` future too: an
  `async fn` over `&self` is `Send` only where the type is `Sync`.
- **The lists of what the flag adds are pinned.** `ADDED_TYPE_NAMES`, with `ADDED_TYPE_COUNT`, is
  every type name the flag puts into a schema module, the ones `mongodb` adds under their own
  `cfg`: `reading_the_authors_scope` reads past those names, and
  `every_type_the_flag_adds_is_a_name_read_past` holds the list to what the module of a struct
  and that of an enum under a tag declare.
  `OPERATIONS` in `recovering_decode/tests.rs` is what `mongodb` adds to the type. A type or a
  method the flag gains joins its list in the same change.
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
- **Nothing emitted carries `#[allow]`, `#[expect]` or `#[doc(hidden)]`.** A consumer's lint levels
  reach what the macro emits into their crate: every added type is `#[non_exhaustive]`, no added
  function takes `impl Trait` as a parameter, and a `Result` is written `core::result::Result`.
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
7. `just test-emitted` (the emitted Node, Dart, Swift and Kotlin clients run under their own toolchains)
8. `just audit` (`cargo audit` over a lock file generated for each package, this one and `bson3/`)
9. Discord notification with build status

The gate builds one feature selection of one package. What only the full run builds: a build with `serde` and no schema surface (the `mobile` and `mongo` sets), a build without `serde`, and `bson3/`, the package that tests against version 3 of the `bson` library. A break there is found at the next full run.

Both workflows install every toolchain the emitted-client groups need: Node with `zod`, `ws` and `typescript`, Dart, and the Kotlin jars; Swift, Kotlin and a JRE come with the runner image. Inside a plain `cargo test` those groups stand down and pass when their toolchain is missing, so only `just typecheck-ts` and `just test-emitted` prove anything: both refuse to stand down, and both are in the full run.

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
│   ├── model_schema.rs           # Main macro logic
│   ├── model_schema/
│   │   ├── checks.rs             # check: the calls validate() makes, the alias refusal
│   │   ├── judged_read.rs        # The Deserialize of a type that publishes validate() or names a preprocess
│   │   └── preprocess.rs         # preprocess: what an item names, the calls of a read, the refusals
│   ├── check_example.md          # The check example the rustdoc runs and the README shows
│   ├── preprocess_example.md     # The preprocess example the rustdoc runs and the README shows
│   ├── field_type.rs             # Type system (FieldDef, FieldDefType)
│   ├── utils.rs                  # Helpers (naming, docs, serde parsing)
│   ├── features/
│   │   ├── mod.rs                # Feature detection
│   │   ├── serde.rs              # Serde attribute parsing
│   │   ├── serde/
│   │   │   └── read_bounds.rs    # The bounds serde's derive writes on a Deserialize impl
│   │   ├── zod.rs                # Zod schema generation
│   │   ├── jsonschema.rs         # JSON schema generation
│   │   ├── object_id.rs          # ObjectId support
│   │   ├── recovering_decode.rs  # decode_with: the entry points, the walkers, the resolvers
│   │   ├── recovering_decode/
│   │   │   ├── enums.rs          # The walkers of an enum
│   │   │   ├── aliases.rs        # A field typed with an alias, walked as the type it names
│   │   │   ├── checks.rs         # decode_with_check_messages, and its messages as issues
│   │   │   ├── query.rs          # mongodb: Filter, Update and the path kinds
│   │   │   ├── fields.rs         # mongodb: MongoFields, MONGO_FIELDS, one function per field
│   │   │   └── operations.rs     # mongodb: OperationError, Read and the operations
│   │   └── model_schema_prop.rs  # Field attribute parsing
│   └── generation/
│       ├── mod.rs
│       └── typescript.rs         # TypeScript/Zod code generation
├── tests/
│   ├── basic_tests.rs
│   ├── pattern_preprocess_tests.rs       # pattern and preprocess: the Zod text, and every read of a preprocess
│   ├── pattern_preprocess_tests/         # The models, the reads, the chained hooks, the Zod text, the README
│   ├── check_tests.rs                    # check, as validate() runs it
│   ├── check_tests/                      # The models, the shared table, the members, the README
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
8. **A member named after a word Dart, Swift or Kotlin reserves** → Written in the form the language accepts, the wire key unchanged: `dart_member` adds a trailing underscore (Dart has no escape), `swift_member` and `kotlin_property_name` write backticks. Each list (`DART_RESERVED`, `SWIFT_RESERVED`, `KOTLIN_HARD_KEYWORDS`) holds only words the real compiler refused as a member name, so a name that compiled before is emitted as before. A longer identifier built from a member's name (`wireFor`, `forKeys`) reads it through `swift_bare`/`kotlin_bare`. The service clients name a method, a parameter and a request's member through the same seams (`dart_local`/`dart_field_member`, `swift_member`/`swift_field_member`, `kotlin_name`/`kotlin_property_name`), and every name the service code reads as text goes through `parse::written`, which drops a raw identifier's `r#`; `parse::field_ident` writes it back where a keyword-named field is read in Rust. The TypeScript emitters name a header or part argument through `message::parameter_name` and a placeholder's local through `message::local_name`, which add a trailing underscore to a word `tsc` refuses as a parameter or a local in a module (`REFUSED_AS_A_NAME`, each one measured)
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
