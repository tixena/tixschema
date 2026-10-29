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

A change is held to the gate, [.github/workflows/ci.yml](../.github/workflows/ci.yml), on every push and pull request to `master`. It is `just all`, two lines with every feature on:

1. `cargo clippy --all-targets --all-features -- -D warnings`
2. `cargo test --all-features`
3. Discord notification with build status

The full run, [.github/workflows/full.yml](../.github/workflows/full.yml), is started every three days and by hand. No change waits for it:

1. `just check` (cargo check + clippy)
2. `just lint-sets` (clippy over every combination of the feature sets, in this package and in `bson3/`)
3. `cargo test --verbose` (basic tests)
4. `just test-sets` (every combination of the `web`, `mobile` and `mongo` feature sets via cargo-hack, in this package and in `bson3/`)
5. `just typecheck-ts` (the emitted TypeScript through a real `tsc --strict`, and a second package compiled against the declarations emitted for it)
6. `just test-slow` (the emitted Node, Dart, Swift and Kotlin clients run under their own toolchains, through `test-emitted`, with the tests that wait on a real clock)
7. `just audit` (`cargo audit` over a lock file generated for each package, this one and `bson3/`)
8. Discord notification with build status

The gate builds one feature selection of one package. What only the full run builds: a build with `serde` and no schema surface (the `mobile` and `mongo` sets), a build without `serde`, and `bson3/`, the package that tests against version 3 of the `bson` library. A break there is found at the next full run.

Both workflows install every toolchain the emitted-client groups need: Node with `zod`, `ws` and `typescript`, Dart, and the Kotlin jars; Swift, Kotlin and a JRE come with the runner image. Inside a plain `cargo test` those groups stand down and pass when their toolchain is missing, so only `just typecheck-ts` and `just test-emitted` prove anything: both refuse to stand down, and both are in the full run, `test-emitted` through `test-slow`.

The MongoDB operations' live checks stand down the same way where `TIXSCHEMA_MONGODB_URI` names no server, and CI names none: `just test-mongodb` is the recipe that refuses to, and no step of either workflow runs it.

**Before pushing**, run `just all`.
