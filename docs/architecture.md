# Architecture

How the macros turn a declaration into each surface, the data structures they share, the feature flags, and where each part of the source lives.

## Code Architecture

### Macro Processing Flow

1. **Entry Point** ([lib.rs](../src/lib.rs))
   - Exports two proc macros: `#[model_schema]` and `#[model_schema_prop]`
   - `model_schema` is the main macro for structs/enums/type aliases
   - `model_schema_prop` is for field-level customization
   - Exports the two initializers a crate calls once at its root, `decode_with_foundation!()` and `mongodb_foundation!()`, which [foundation.rs](../src/foundation.rs) expands: the support libraries of the recovering decode ("Recovering Decode", below)

2. **Macro Execution** ([model_schema.rs](../src/model_schema.rs))
   - `exec_model_schema()` routes to `process_struct()`, `process_enum()`, `process_type_alias()`, or `process_branded_newtype()`
   - Parses Serde attributes when `serde` feature enabled
   - Extracts example code from doc comments (every ` ```rust example` fence)
   - Generates methods: `ts_definition()`, optionally `json_schema()`, optionally `schema_examples()` with `schema_example()`, and optionally `validate()`
   - `process_branded_newtype()`: handles `#[serde(transparent)]` structs serde writes as the value of one field -- a single-slot tuple struct, or a struct with named fields -- generating Zod brand schemas or `unique symbol` branded TypeScript types

3. **Type Analysis** ([field_type.rs](../src/field_type.rs))
   - `FieldDef`: Core data structure representing a field's type, optionality, docs, etc.
   - `FieldDefType`: Enum categorizing Rust types (primitives, collections, custom types, ObjectId, etc.)
   - `get_field_def()`: Recursively analyzes Rust types and builds FieldDef trees
   - Handles: Option<T>, Vec<T>, HashMap<String, T>, nested types, generics

4. **Feature Modules** ([features/](../src/features/))
   - `serde.rs`: Parses Serde attributes (`rename`, `rename_all`, `tag`, etc.), and says what serde writes on the way to a member (`member_steps`, `variant_steps`, over `Tagging`)
   - `zod.rs`: Generates Zod v4 schema strings (`z.string()`, `z.union()`, etc.), embeds examples in `.meta()`
   - `jsonschema.rs`: Generates JSON schema objects
   - `object_id.rs`: MongoDB ObjectId type detection and schema generation
   - `recovering_decode.rs`: Generates what `#[model_schema(enable = ["decode_with"])]` adds to a type: `from_value_with` and `from_value_piped`, `from_bson_with` and `from_bson_piped` under `bson`, and the walker of each. It also writes the two support libraries the initializers put at a crate root, `__tixschema_decode_with` (the callback's and the resolvers' types, what one walker hands another, and under `bson` the BSON pieces) and `__tixschema_mongodb` (under `mongodb`, the query types and what the operations answer), and the re-export of both, name by name, in each `{type}_schema`. `recovering_decode/enums.rs` holds the walkers of an enum, and the names its typed paths read a variant by (`written_variants`, `tag_arms`). Under `mongodb` three more files emit, for a type that lists `"mongodb"`: `recovering_decode/query.rs` the query types into the MongoDB library (`Filter`, `ArmFilter`, `Update`, `ArmUpdate`, the path kinds, `nest`), `recovering_decode/fields.rs` the type's own typed paths into its module and onto the type (`MongoFields`, `MONGO_FIELDS`, one function per field, and one per arm of an enum under a tag), and `recovering_decode/operations.rs` the operations onto the type (`find_one`, `find`, `count`, `insert_one`, the updates and the deletes) and `Read` and `OperationError` into the library
   - `model_schema_prop.rs`: Parses field-level customization attributes (`pattern`, `minLength`, `maxLength`, `minimum`, `maximum`, `literal`, `as`, `preprocess`, `check`)

5. **Code Generation** ([src/model_schema.rs](../src/model_schema.rs))
   - `typescript.rs`: Generates TypeScript type definitions and Zod schemas
   - Combines FieldDef trees into complete type/schema strings
   - Handles discriminated unions, generics, nested types
   - Every `zod_schema()` writes `examples` into the schema's `.meta()`: `[]` as text where the type writes no block, `schema_examples()` serialized where it writes one or more

6. **Example Extraction** ([utils.rs](../src/utils.rs))
   - `extract_examples_tokens()`: Parses every ` ```rust example` code fence from doc comments
   - Returns each block's tokens in the order written, an empty block skipped
   - Each block's code is inserted, in a scope of its own, into the generated `schema_examples()` method
   - Compiler validates example code at compile time

7. **Service Schema Accessors** ([features/service_schema.rs](../src/features/service_schema.rs))
   - `ts_http_service()`: the TypeScript route table and request dispatcher for `http_rest`, mirroring the Rust `dispatch`/`ROUTES` pair
   - `ts_ws_server()`: the TypeScript connection-accepting WebSocket server for Node, wrapping the existing single-socket dispatcher attachment
   - `ts_http_client()`'s seam carries `body` as a `Uint8Array`, a JSON body as its UTF-8 text, as
     the Dart, Swift and Kotlin seams carry bytes; a `bytes` reply is answered as the
     `Uint8Array` its result declares, which the transport-agnostic and `ws_rpc` clients parse
     from the pair of the bytes and their content type the TypeScript dispatcher answers
     (`message::success_body`)
   - An operation only `http_rest` carries, a `body = "stream"` one and one with a `part(...)` file
     part, is `OperationDef::carried_by_http_alone`, the question the Rust refusal of `ws_rpc` and
     `amqp_rpc` reads. Every `ws_rpc` and transport-agnostic emitter reads
     `ServiceDef::enveloped()`, the service without such operations, so no client of those
     transports has a method, no reply table an entry and no dispatcher a handler for one, and
     `left_to_http_rest` writes the sentence each states that in. `create{Service}Dispatcher`,
     `attach{Service}WsDispatcher` and `create{Service}WsServer` take `Omit<{Service}Impl<Ctx>,
     ...>` (`service::enveloped_impl`), and `create{Service}HttpDispatcher` answers such an
     operation in a `switch` of its own ahead of that dispatcher (`dispatch_stmt`). The `http_rest`
     clients keep it. `run_http_alone.rs` compiles the `ws_rpc` client of each language of such a
     service under its toolchain, and
     `a_service_with_an_operation_only_http_rest_carries_type_checks_against_the_real_zod` the
     TypeScript surfaces under `tsc --strict`
   - `dart_definition()`: every Dart type a service publishes -- its messages, fault type, and one sealed `{Service}{Operation}Result` pair per reply operation
   - `swift_http_client()`/`swift_ws_client()`: the Swift `http_rest` and `ws_rpc` clients -- one method per operation answering `Result<Success, Failure>`, a one-way method throwing the shared `{Service}Refusal`
   - `kotlin_http_client()`/`kotlin_ws_client()`: the Kotlin `http_rest` and `ws_rpc` clients -- one method per operation answering the sealed `{Service}{Operation}Result`; the WebSocket one also carries the mini server, `attach{Service}WsDispatcher(frames, handlers, onFault)`, and its `share`
   - The module itself is gated on `serde` alone. `typescript` gates only the `ts_*` accessors (and `zod` narrows further, to the client and dispatcher seam); `dart`, `swift` and `kotlin` each gate their own accessors independently, under `serde` plus that language's own feature -- none of the four needs any of the others on
   - Every name the TypeScript service code writes between quotes -- a path's literal piece, a
     message name, a part name, a header name and the service's own name -- is written by
     `ts_quoted` in `src/utils.rs`, the one writer of a TypeScript string literal, which escapes a
     `"`, a `\`, a line terminator and every other control character. Each emitter binds a message
     name once as that literal (`wire`). A name written into a comment goes through
     `message::commented`, which spells a line terminator as its escape (`on_one_line` in
     `src/features/service_schema.rs`) and hands the rest to `ts_commented` in `src/utils.rs`,
     which spells `*/` as `*\/`. A name with none of those characters is written as it was.
     `a_service_whose_names_hold_a_quote_type_checks_on_every_surface` in
     `tests/service_schema_typescript_tests/type_check.rs` compiles every surface of such a service
     under `tsc --strict`, and `run_node_quoted.rs` calls one through the `http_rest` and `ws_rpc`
     clients and dispatchers under node, each request read by the Rust dispatcher too
   - The Dart, Swift and Kotlin `http_rest` and `ws_rpc` clients write a message name and a path
     into the `///` line over a method through `on_one_line` too: a line comment of the three ends
     at `\n` and `\r` alone, and a `*/` in it closes nothing. An `http_rest` client writes a
     path's literal piece between quotes through the writer of its language's other names
     (`quoted`, `swift_quoted`, `kotlin_quoted`). `run_quoted_service.rs` compiles and calls each
     client of a service whose message name holds a line break, and the Rust dispatcher of
     `run_node_quoted.rs`, whose service declares the same operation, routes the path they carry
   - A brand, a struct of one slot or an alias declared above its service is read from the text of a
     path, a query, a header and a text part as the scalar it wraps: `record_wire_scalar_of` in
     `src/utils.rs`, called by `record_wire_scalar_item` in every build with `serde`, records the
     `WireScalarKind` (text, `bool`, float, integer, `i64`, `u64`) beside the wire-scalar record, and
     `wire_scalar_kind` is the one reader. `scalar_kind` answers from it for the Rust `decode_expr` and
     the TypeScript `decode_ts_expr`, `held_as_a_bigint` and `wide_integer_range` for the `bigint` read
     and its range, `is_integer_scalar` for the whole-number check of a TypeScript `header_out`, and
     `dart_header_out_decode`, `swift_header_out_decode` and `kotlin_header_out_decode` read a
     `header_out` of a number brand from the JSON number its text is, where they read a brand over text
     from the JSON string. A named type declared below its service is recorded by no one and stays
     text. `tests/service_number_brand_tests.rs` holds the Rust client and dispatcher in a path, a
     query, a header, a `header_out` and a text part, and `run_node_number_brands.rs` the TypeScript
     clients and dispatchers beside the Rust dispatchers, and the Dart, Swift and Kotlin clients
   - The `http_rest` TypeScript dispatcher answers a declared error that takes one status whatever
     it holds at that status, written as a number (`fixed_error_status`, `error_status_of` in
     `src/features/service_schema/http_service.rs`): 422 where the operation declares no
     `error_status` table, as the Rust dispatcher does, and the one status an untagged enum's table
     maps every variant to. Only a table a switch reads is a closure called with the error. A
     streamed operation's outcome in `ts_service()` names the record its result names
     (`stream_success_ts_type`). `a_declared_error_of_one_status_type_checks_at_the_http_dispatcher`
     in `tests/service_schema_typescript_tests/type_check.rs` compiles each shape under
     `tsc --strict`

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

`array_lengths` numbers levels the same way: `[[T; 2]; 3]` records `(0, 2)` and `(1, 3)`. The
validating surfaces spend it — `minItems`/`maxItems` in the JSON schema, `.length(N)` in Zod — and
so do the Dart, the Swift and the Kotlin reads, which refuse an array of another length
(`dart_decode_at`, `read::counted`, and in Kotlin a class of its length,
`src/features/kotlin/arrays.rs`), while TypeScript stays `Array<T>`. A length the expansion cannot
read (const generic, `const` item, computed expression) records nothing and describes as an
unbounded array.

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
│   │   └── ranges.rs             # The identity check of a std range or a std Result written by its bare name; a flattened one, and one as a map key where no surface runs, refused; a map flattened beside another member refused, and one keyed by a number below a flattened member or in an enum serde buffers; serde_json's Value and Map refused by their full path and checked by their bare name
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
│   │   ├── text_forms.rs         # The one form of a char and a chrono text, which each client reads and sends
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
│   │   │   ├── aliases.rs        # The names serde reads a variant and a member under beside the one it writes
│   │   │   ├── forms.rs          # A char and a chrono text read and written in their one form
│   │   │   ├── preprocess.rs     # preprocess in the Swift read: the payload type, the calls, the keys taken
│   │   │   ├── read.rs           # The two reads of every Swift type: its own, and the one that judges
│   │   │   └── validate.rs       # validate() of every Swift type: bounds, checks, holders
│   │   └── model_schema_prop.rs  # Field attribute parsing
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
│   ├── wire_forms_tests.rs               # The form each client sends: serde and the JSON Schema held to WIRE_FORMS
│   ├── wire_forms_tests/                 # WIRE_FORMS and its models, which every client's run sends
│   ├── skipped_variant_tests.rs          # A variant serde neither writes nor reads: every surface, and the models the client runs read
│   ├── skipped_variant_tests/            # The models and the payloads every surface and client is held to
│   ├── wide_integer_tests.rs             # A u64 and an i64 as the JSON number serde writes: serde's write and read in every position, and every surface
│   ├── wide_integer_tests/               # The models
│   ├── service_wide_integer_tests.rs     # A bare u64 and i64 through the Rust dispatchers and clients, as JSON numbers
│   ├── service_wide_integer_tests/       # The service, its transports and its clients
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
│   ├── collection_tests/                 # The tests, and the maps keyed by text of their own with the payloads every surface is held to
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
│   ├── foundation_refusals/              # The cases: a missing initializer, the enable refusals, a held type missing "mongodb", a map keyed by a number under "mongodb", a flattened map keyed by a number, one in an enum serde buffers, an enum of no variant, and an i128
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
