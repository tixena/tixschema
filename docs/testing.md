# Testing

How to add a type mapping, a feature, a test or an example, and what CI runs.

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
- The Zod schema of **every** type holds `examples` in its `.meta()`, on the valid type's `$Schema` and not on its draft's, and none writes an `example` key: `examples: []` where the type writes no block, and the serialized `schema_examples()` where it writes one or more
- Each example is serialized using Serde, so it matches your API's JSON format

```typescript
// no block
export const Plain$Schema = PlainDraft$Schema.readonly().brand<"Plain">().meta(PlainDraft$Schema.meta() ?? {}).meta({
  examples: []
});

// two blocks
export const SeveralExamples$Schema = SeveralExamplesDraft$Schema.readonly().brand<"SeveralExamples">().meta(SeveralExamplesDraft$Schema.meta() ?? {}).meta({
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

1. Add variant to `FieldDefType` in [src/field_type.rs](../src/field_type.rs)
2. Update `get_field_def()` to detect the new type
3. Add TypeScript generation logic in [src/model_schema.rs](../src/model_schema.rs)
4. Add Zod generation logic in [src/features/zod.rs](../src/features/zod.rs) (if `zod` feature)
5. Add tests in appropriate test file (e.g., `tests/primitive_types_tests.rs`)

### Adding a New Feature

1. Add feature to `Cargo.toml` `[features]` section
2. Create feature module in [src/features/](../src/features/)
3. Add `#[cfg(feature = "...")]` guards in affected code
4. Update `Features` struct in [src/features.rs](../src/features.rs)
5. Add tests that verify behavior with/without the feature

### Adding Tests

Tests are organized by category in [tests/](../tests/):

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

### Flattened Members

`src/model_schema/flatten_order/tests.rs` holds the decision table of flattened members to every
sequence of member classes, and each member kind to its class. `tests/flatten_matrix_part*_tests.rs`
hold every shape of flattened members a plain-serde measure found reading back that
`#[model_schema]` declares, eight binaries so cargo builds them side by side, over the helpers of
`tests/flatten_matrix/harness.rs`: each row is written, read back equal, and read with a stray key
first as the measure states, but for the unit variant of a strict enum under a tag alone, which
the macro reads refusing the stray key serde's own derive read past. They were generated once from
that measure and are edited as any other test. `tests/flatten_order_refusals/` holds what rustc
prints for each refusal, a member declared below its holder or in another crate among them, whose
refusal the `const` beside the holder prints, and
`service_schema_emitted_client_tests/run_flatten_order.rs` reads one holder of each accepted class
on every client.

### Validation in Every Build

[tests/every_build_validation_tests.rs](../tests/every_build_validation_tests.rs) holds every build
to the same validation. `models.rs` declares one constrained model of each shape: a named struct,
a tuple struct, a brand, an alias of the brand held by a field, an enum of each tagging (external,
internal, adjacent, untagged), a generic struct read at a filling other than its declared default,
and, with `serde`, a type that lists `enable = ["decode_with"]` and a `#[service_schema()]` trait
with no transport. `validate.rs` asserts each model's `validate()` in every build. `read.rs`
asserts, with `serde`, the read that refuses a broken rule, `from_value_with` handing its callback
the `Invalid` issue, and `{service}_schema::validated_{operation}` refusing a broken request. Each
expected text is what a `web` build answers.

`serde` is the binary's only feature gate, on `read.rs` and on the decode and service declarations,
which need it. Without `serde` no `#[serde(...)]` attribute is read: `validate()` names a field as
Rust spells it and every enum as externally tagged, so `validate.rs` holds those names in one
`#[cfg(feature = "serde")]` / `#[cfg(not(feature = "serde"))]` pair of constants. A new shape, or a
new place a rule is held, gets a model and a test here, under no other gate. `just check-mix` runs
the binary with no feature and under `web`, `mobile`, `mongo` and `mobile,mongo`.

### Testing the `mcp` Transport

Every MCP answer a test reads is held to the official schema of revision 2026-07-28, vendored as
[tests/mcp_declaration_tests/mcp-schema-2026-07-28.json](../tests/mcp_declaration_tests/mcp-schema-2026-07-28.json),
and every task message to the tasks extension's schema beside it, through the `jsonschema`
dev-dependency (`schema_errors` in `tests/mcp_dispatch_tests/support.rs`). A new message is checked
there by its `$defs` name before any test compares its bytes.

- `tests/mcp_declaration_tests.rs`: `mcp_tools()`, `wrapped_{operation}()` and `MCP_NAMES`, each
  tool held to the official `Tool` definition, and `needs_wrapper()`, `mcp_form_value()` and
  `mcp_form_flat()` on every kind of model type.
- `tests/mcp_dispatch_tests.rs`: the Rust dispatcher and client, one file per feature under
  `tests/mcp_dispatch_tests/` (tools, resources, prompts, elicitation, the writer, subscriptions,
  tasks, skills, authorization over HTTP).
- `tests/mcp_client_tests.rs`: a crate that declares the service and a crate that calls it, apart.
- `tests/mcp_fault_cases/`: the one table of the faults every server answers. Both
  `tests/mcp_dispatch_tests.rs` and `tests/service_schema_emitted_client_tests.rs` include it by
  `#[path]`, so the Rust, TypeScript, Dart and Kotlin servers are held to the same rows. A new fault
  rule is a row here, never a copy in one run.
- `tests/service_schema_emitted_client_tests/`: the TypeScript, Dart and Kotlin servers and the five
  clients under their own toolchains (`just test-emitted`). Each `mcp_*_rows.rs` file sends the same
  requests to a server and to the Rust dispatcher, and asserts the same bytes. `mcp_streams.rs`
  holds the streams that record each `begin` and `write` with the handler still running, so an
  order is asserted with no clock. Every service there is one test binary, and a `macro_rules!` name
  is global to its crate, so a new service takes a name no other service of the binary uses.
- `tests/foundation_refusals/mcp_*.rs`: what rustc prints for a name two services of one endpoint
  share, a form that is not flat or not a model type, and a type that does not list `jsonschema`.
- `tests/mcp_skills/` and `tests/mcp_skills_refused/`: the skills folders a service reads, and the
  folders it refuses.
- `examples/strict_consumer_models/desk*.rs`: a module for each transport's dispatcher and client,
  and for the `amqp_rpc` server, each exported, linted by the package's own clippy table.
- `tests/crates_apart/sealed_parcel_desk/` and `tests/sealed_parcel_desk_tests.rs`: a crate that
  places the transports of a `non_exhaustive` service, and the test binary that builds what they
  seal from another crate. The root's `serde` feature turns the crate's `service` feature on.

### Running Single Tests During Development

```bash
# Run a specific test
cargo test test_basic_struct

# Run all tests in a module with output
cargo test basic_tests -- --nocapture

# Run tests matching a pattern
cargo test objectid
```

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

For a type that lists `enable = ["jsonschema"]`, test that generated schemas are valid:

```rust
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

Each workflow installs beta, which `rust-toolchain.toml` pins and the code is developed on, and runs every lint step and every test step on it. No step runs on the floor, `rust-version` in `Cargo.toml` (1.91.1): the tests' `arky` dev-dependency needs a newer rustc than the floor. The setup action is given `rustflags: ""`, so `RUSTFLAGS` stays unset: only a lint step, which passes `-D warnings` itself, fails on a warning.

A change is held to the gate, [.github/workflows/ci.yml](../.github/workflows/ci.yml), on every push and pull request to `master`. It is `just all`, two lines with every feature on, then `just check-mix`:

1. `cargo clippy --all-targets --all-features -- -D warnings`, on beta
2. `cargo test --all-features`, on beta
3. `just check-mix`, not part of `just all`, on beta, in three steps:
   - `cargo hack check --each-feature` compiles this package's library, no test, once with no feature, once with the default, once with every feature, and once with each feature by itself
   - the same line over `bson3/`, with `--tests`, compiles that package's tests, against version 3 of the `bson` library, and tixschema, a dev-dependency there, under the same features; no test runs
   - `tests/every_build_validation_tests.rs` runs with no feature and under `web`, `mobile`, `mongo` and `mobile,mongo`
4. Discord notification with build status

The full run, [.github/workflows/full.yml](../.github/workflows/full.yml), is started every three days and by hand. No change waits for it:

1. `just check` (cargo check of this package and of the tests of `bson3/`, then clippy)
2. `just lint-sets` (clippy over every combination of the feature sets, in this package and in `bson3/`, and over each output feature without `serde`, alone and in `typescript,zod` and `dart,swift,kotlin`)
3. `cargo test --verbose` (basic tests)
4. `just test-sets` (every combination of the `web`, `mobile` and `mongo` feature sets via cargo-hack, in this package and in `bson3/`)
5. `just typecheck-ts` (the emitted TypeScript through a real `tsc --strict`, and a second package compiled against the declarations emitted for it)
6. `just lint-slow`, then `just run-slow`, both on beta (the emitted Node, Dart, Swift and Kotlin clients run under their own toolchains, through `test-emitted`, with the tests that wait on a real clock)
7. `just audit` (`cargo audit` over a lock file generated for each package, this one and `bson3/`)
8. Discord notification with build status

The gate lints and tests one feature selection of one package. `just check-mix` compiles the library, and the tests of `bson3/`, the package that tests against version 3 of the `bson` library, under every feature by itself, so a declaration and its use that carry different features fail on the next push, and runs the every-build validation tests in five builds, so a rule that one of them stops holding fails there too. What only the full run reaches: the lints of each combination of the `web`, `mobile` and `mongo` sets, every other test in those combinations, the lints of `bson3/`, and the run of its tests. A break there is found at the next full run. A build without `serde` is compiled by `just check-mix`, and only the full run lints it, through `just lint-sets`.

`just check-all` builds what the first two lines of `just check-mix` build, over every combination of the features in place of each feature by itself: this package's library, then the tests of `bson3/` and tixschema. `just ci` runs it, and neither workflow does.

Both workflows install every toolchain the emitted-client groups need: Node with `zod`, `ws` and `typescript`, Dart, and the Kotlin jars; Swift, Kotlin and a JRE come with the runner image. Inside a plain `cargo test` those groups stand down and pass when their toolchain is missing, so only `just typecheck-ts` and `just test-emitted` prove anything: both refuse to stand down, and both are in the full run, `test-emitted` through `run-slow`.

The MongoDB operations' live checks stand down the same way where `TIXSCHEMA_MONGODB_URI` names no server, and CI names none: `just test-mongodb` is the recipe that refuses to, and no step of either workflow runs it.

**Before pushing**, run `just all`.
