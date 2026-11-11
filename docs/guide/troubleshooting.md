## Important Notes

1. **Naming Convention**: A type publishes under the Rust ident it is declared with, spelled exactly as written. `#[model_schema(name = "...")]` is the only way to publish under another name, and it moves that name on every surface at once. Two declarations reaching one published name are refused at expansion. See [Published Names](models.md#published-names-name).

2. **Type References**: Nested types reference each other by the name the referenced type publishes -- its Rust ident, or its `name` override.

3. **Map Keys**: `HashMap`, `BTreeMap` and, under the `indexmap` feature, `IndexMap` are the maps that are supported, and the key decides what the map describes as: a `String` key gives an object open to any string key, and a plain `#[model_schema()]` enum key gives one closed to the enum's members. A brand and a `#[model_schema()]` alias answer by their inner and their target, keeping their own name on the TypeScript and Zod surfaces. A key path proved to be one serde writes as no key at all, and any sequence-wrapped key, is refused at expansion; every other key describes as an open object. See [Collections and Maps](models.md#collections-and-maps).

4. **Array Types**: `Vec<T>` becomes `Array<T>` in TypeScript, one `Array<...>` per level written — `Vec<Vec<T>>` is `Array<Array<T>>`.

5. **Optional Fields**: `Option<T>` becomes `field: T | undefined` in TypeScript unless `#[model_schema_prop(ts_optional)]` asks for `field?: T`, and `z.union([z.null().transform(() => undefined), type, z.undefined()]).prefault(undefined)` in Zod v4 either way.

6. **Complex Nesting**: The crate supports deeply nested structures including `HashMap<String, Vec<HashMap<String, T>>>` and similar patterns.

7. **Doc Comments**: Rust doc comments on types and fields are carried through to the generated TypeScript as JSDoc comments. A member with no doc comment is documented by the name serde writes for it. A `*/` in a doc comment or in such a name is written `*\/`, so the comment runs on past it.

8. **Built-in Type Mappings**: Besides the primitives, `String` and `std::path::PathBuf` both map to `string` (`z.string()`). MongoDB `ObjectId` and the `chrono` date/time types are supported behind their respective feature flags. A member that holds `serde_json::Value` or `serde_json::Map` stops the build, and so does a service operation that takes or answers one: see [A JSON value is no member](models.md#a-json-value-is-no-member).

## Error Handling & Troubleshooting

### Feature-Related Errors

#### `ObjectId` Without the Feature

**Error:**

```
error: `ObjectId` is read as a MongoDB ObjectId under tixschema's `mongodb` feature: add `features = ["mongodb"]` to the `tixschema` dependency
 --> src/main.rs:8:13
  |
8 |     pub id: ObjectId,
  |             ^^^^^^^^
```

**Cause:** A type written `ObjectId` with the `mongodb` feature off.

**Solution:**
```toml
tixschema = { version = "0.4", features = ["mongodb"] }
```

A crate that declares a `#[model_schema()]` type of its own named `ObjectId`, above the type that holds it, is not refused: the name is that type.

#### `NaiveDate`, `NaiveTime`, `NaiveDateTime` or `DateTime<Tz>` Without the Feature

**Error:**

```
error: `NaiveDate` is read as a date or a time under tixschema's `chrono` feature: add `features = ["chrono"]` to the `tixschema` dependency
 --> src/main.rs:8:14
  |
8 |     pub day: NaiveDate,
  |              ^^^^^^^^^
```

**Cause:** A type written `NaiveDate`, `NaiveTime`, `NaiveDateTime` or `DateTime<Tz>` with the `chrono` feature off. It is refused wherever the item writes it, as `IndexMap` is below.

**Solution:**
```toml
tixschema = { version = "0.4", features = ["chrono"] }
```

A crate that declares a `#[model_schema()]` type of its own under one of these names, above the type that holds it, is not refused: the name is that type.

#### `IndexMap` or `IndexSet` Without the Feature

**Error:**

```
error: `IndexMap` is read as a map under tixschema's `indexmap` feature: add `features = ["indexmap"]` to the `tixschema` dependency
 --> src/main.rs:8:15
  |
8 |     pub data: IndexMap<String, String>,
  |               ^^^^^^^^
```

**Cause:** A type written `IndexMap<K, V>` with the `indexmap` feature off. `IndexSet<T>` gets the same sentence, with `as a list`. It is refused wherever the item writes it: as a field's type, under an `Option` or a `Vec`, as a map's value, in an enum's variant, as the target of an alias.

**Solution:**
```toml
tixschema = { version = "0.4", features = ["indexmap"] }
```

A crate that declares a `#[model_schema()]` type of its own named `IndexMap` or `IndexSet`, above the type that holds it, is not refused: the name is that type.

#### `u64` or `usize` Under `swift` or `kotlin`

**Error:**

```text
error: model_schema: field `count`: `u64` has no Swift mapping; the Swift target refuses unsigned 64-bit and pointer-sized integers
error: model_schema: field `count`: `u64` has no Kotlin mapping; the Kotlin target refuses unsigned 64-bit and pointer-sized integers
```

**Cause:** A member that reaches a `u64` or a `usize` at any depth, under the `swift` or the `kotlin` feature. The Swift and the Kotlin targets each refuse the two widths.

**Solution:** Use `i64`, or `u32` where the range allows. A member serde neither writes nor reads, under `#[serde(skip)]` or under `skip_serializing` and `skip_deserializing` together, and every member of a variant serde neither writes nor reads, is not refused: serde writes none of it. A member under one of the two alone is.

#### JSON Schema Method Missing

**Error:** ``error[E0599]: no associated function or constant named `json_schema` found for struct `Address` in the current scope``, for `Address::json_schema()`.

**Cause:** Calling `.json_schema()` on a type that does not list `"jsonschema"` in its `enable` list. JSON Schema is asked for per type, not by a feature.

**Solution:**
```rust
#[model_schema(enable = ["jsonschema"])]
```

Every model type the type holds lists the entry too: the type's document calls each held type's `json_schema_within`, and a held type without the entry stops the build at the holder: ``error[E0599]: no associated function or constant named `json_schema_within` found for struct `address_schema::Schema` in the current scope``.

#### Zod Schema Missing

**Symptom:** Generated TypeScript contains only types, no Zod schemas.

**Cause:** `zod` feature is disabled.

**Solution:**
```toml
tixschema = { version = "0.4", features = ["zod"] }
```

#### Serde Attributes Ignored

**Symptom:** TypeScript and Zod write a member in camelCase where a `#[serde(rename = "...")]` or a `#[serde(rename_all = "...")]` names another key: `#[serde(rename = "mail")] pub email_address: String` is `emailAddress`, where serde writes `mail`.

**Cause:** tixschema's `serde` feature is off, so no `#[serde(...)]` attribute is read, and TypeScript and Zod write each member in camelCase.

**Solution:**
```toml
tixschema = { version = "0.4", features = ["serde"] }
```

### Compilation Errors

#### Map Keys Without Enumerable Members

**Error:** *a map key must be a plain `#[model_schema()]` enum, whose members become the object's keys — `<type>` resolves to a type with no `enum_members()`*, reported at the field; or `no associated function or constant named enum_members found for <type>` (`E0599`), reported at the map's key type.

A map key written as a type path must be something serde writes into a key: a plain `#[model_schema()]` enum, whose members become the object's keys; a `#[serde(transparent)]` brand or a `#[model_schema()]` alias whose wire form is one serde writes into a key; or a `String`. [Collections and Maps](models.md#collections-and-maps) sets out what each of those describes as, and every key that is none of them is covered there too — this entry is about the two diagnostics a type path earns.

A key the expansion has already seen and knows serde writes as no key at all — a struct, a brand over one or over a container, a tagged or untagged enum, or a `#[model_schema()]` alias of one of those — is named directly, at the field that writes the map:

```rust
#[model_schema()]
pub struct Doc {
    pub id: String,
}

// Wrong: `Doc` is a struct, so it has no members for the object's keys
#[model_schema()]
pub struct BadCounts {
    pub counts: HashMap<Doc, u32>, // reported at the field, naming `Doc`
}
```

An alias resolves through to what it names, so an alias of a struct is refused under the alias's own name, and a `#[model_schema()]` alias whose target *is* such a map is refused at the alias.

Items expand in the order they are written, though, so a key declared after the type that writes the map, like one from another crate, has not been seen yet and is emitted as an enumerating key. When such a key carries no `enum_members()`, the same requirement surfaces as an `E0599` at the key type:

```rust
// Wrong: `LateKey` has not expanded yet, so it is emitted as an enumerating key — and it
// resolves to `String`, which has no members to enumerate
#[model_schema()]
pub struct BadConfig {
    pub slots: HashMap<LateKey, String>, // E0599, reported at `LateKey`
}

#[model_schema()]
pub type LateKey = String;

// Correct: the key is a plain enum, and its members become the object's keys
#[model_schema()]
pub enum Slot {
    Primary,
    Secondary,
}

// Correct: the alias expands first, so the registry knows serde writes it as a bare string
#[model_schema()]
pub type EarlyKey = String;

#[model_schema()]
pub struct Config {
    pub slots: HashMap<Slot, String>,
    pub names: HashMap<EarlyKey, String>,
}
```

A plain enum key works wherever it is declared: it carries `enum_members()`, so the emitted call resolves whichever order the two items expand in, and declaration order decides only which of the two diagnostics a key *without* members earns. Every other supported spelling — an alias of a string, a brand — is known only through the registry, so it has to expand before the type that keys a map by it; until then the expansion cannot tell it from a plain enum declared later, and emits the enumerating key it would need to be.

#### Sequence-Wrapped Map Keys

**Error:** *a map key must be a value serde writes as a string ... this key is a sequence of `<type>`*, reported at the field.

A JSON object key is a string. A sequence writes a JSON array, so `serde_json` refuses to serialize a map keyed by one at all — `key must be a string`, raised at serialization time, with no object and no array-of-pairs fallback. There is no wire form for a schema to describe, so the spelling is refused rather than described:

```rust
// Wrong: the key writes an array, which serde will not use as an object key
#[model_schema()]
pub struct BadCounts {
    pub counts: HashMap<Vec<Slot>, u32>,
}

// Correct: the key is the element itself, whose members become the object's keys
#[model_schema()]
pub struct Counts {
    pub counts: HashMap<Slot, u32>,
}
```

Every sequence spelling earns this — `Vec`, `[T; N]`, and the sets — the wrapper being what serde writes as an array. The message names the element rather than the wrapper, the parser having already collapsed those spellings onto their array levels. A sequence in the map's *value* is untouched: only the key has to be a string.

#### Map Keys serde Refuses to Write

**Error:** *a map key must be a value serde writes as a string ... serde writes `<type>` as a JSON array | a JSON object, and refuses to serialize a map keyed by one at all*, reported at the field.

The same rule as above, reached by the key's own type rather than by a wrapper around it. A tuple writes a JSON array; a nested map and an `ObjectId` write JSON objects. `serde_json` uses none of them as an object key — it raises `key must be a string` and refuses the whole map — so there is no wire form for a schema to describe:

```rust
// Wrong: each key writes an array or an object, which serde will not use as an object key
#[model_schema()]
pub struct BadCounts {
    pub by_pair: HashMap<(Slot, Slot), u32>,
    pub by_map: HashMap<BTreeMap<String, u32>, u32>,
    pub by_oid: HashMap<ObjectId, u32>,
}

// Correct: key by a value serde writes as a string
#[model_schema()]
pub struct Counts {
    pub by_slot: HashMap<Slot, u32>,
    pub by_id: HashMap<String, u32>,
}
```

The keys serde *does* stringify for you are untouched: numbers, `bool`, `char` and the chrono types describe the map's values and name its keys by the text serde writes them as, as [Collections and Maps](models.md#collections-and-maps) sets out. The rule is refuse-what-serde-refuses, never refuse-what-is-not-a-`String`. A tuple or a map in the map's *value* is untouched too: only the key has to be a string.

#### `Option`-Wrapped Map Keys

**Error:** *a map key must be a value serde writes as a string ... this key is an `Option<T>`, whose `Some` serde writes as the bare `T` while a `None` has no string form at all and makes serde refuse the whole map; key it by `T`*, reported at the field.

An `Option` key is transparent for a `Some` — `HashMap<Option<Slot>, u32>` writes `{"Daily": 1}`, exactly what the bare-keyed map writes — and has no form at all for a `None`, which fails serialization with `key must be a string`. A schema describing only the `Some` half would validate documents the type can raise a runtime error on and say nothing about the half it cannot write, so the spelling is refused and the inner is named as the remedy:

```rust
// Wrong: a `None` key has no wire form, and one is enough to fail the whole map
#[model_schema()]
pub struct BadCounts {
    pub counts: HashMap<Option<Slot>, u32>,
}

// Correct: key by the inner, which is what every `Some` was already writing
#[model_schema()]
pub struct Counts {
    pub counts: HashMap<Slot, u32>,
}
```

An `Option` in the map's *value* is untouched: a map entry cannot be dropped the way an object key can, so a `None` value is written as `null` and described as such.

#### Function-Local Types

**Error:** `cannot find module or crate <type>_schema in this scope` (`E0433`), reported at the field's type.

A type that lists `enable = ["jsonschema"]` reaches each type it holds through that type's schema module, so a held type must be declared at item scope. A type declared inside a function body publishes its schema module inside that same body, where the `use super::*` of the referencing type's own module never reaches it:

```rust
// Wrong: `Inner` is declared inside the function body
#[test]
fn builds_the_schema() {
    #[model_schema(enable = ["jsonschema"])]
    pub struct Inner {
        pub id: String,
    }

    #[model_schema(enable = ["jsonschema"])]
    pub struct Outer {
        pub inner: Inner, // E0433: cannot find module or crate `inner_schema` in this scope
    }
}

// Correct: `Inner` is declared at item scope; `Outer` may stay in the function body
#[model_schema(enable = ["jsonschema"])]
pub struct Inner {
    pub id: String,
}

#[test]
fn builds_the_schema() {
    #[model_schema(enable = ["jsonschema"])]
    pub struct Outer {
        pub inner: Inner,
    }
}
```

With `#[model_schema()]` on both, and no `enable` list, the wrong form builds, under the default features, `mobile` and `mongo` alike.

#### Missing Derives

**Error:** Various compilation errors related to traits.

Always include required derives:
```rust
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MyType {
    // fields...
}
```

#### A Model Type Declared in Another Module

**Error:** `cannot find module or crate <type>_schema in this scope` (`E0433`), reported at the field's type.

On a type that lists `enable = ["jsonschema"]`, a field naming a `#[model_schema]` type reaches that type's generated `{type}_schema` module through the path it is written with: `crate::version::Version` needs no import. A bare name reaches the module by its bare name, so import it beside the type:

```rust
pub mod version {
    #[model_schema(enable = ["jsonschema"])]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Version {
        pub number: i32,
    }
}

pub mod record {
    use crate::version::{Version, version_schema};

    #[model_schema(enable = ["jsonschema"])]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Record {
        pub versions: Vec<Version>,
    }
}
```

rustc's help line suggests `cargo add version_schema`; there is no such crate. The module is generated next to `Version`, in the module that declares it, so a path through a re-export reaches no module.

The import is the JSON Schema's alone. The typed MongoDB paths name no other type's module, so a type that lists `"mongodb"` and not `"jsonschema"`, and holds a model declared in another module, builds with the model imported by itself, whatever the schema surfaces (`typescript`, `zod`) are.

#### A Renaming That Names Two Keys

**Error**, reported at the `rename`:

```text
error: model_schema: field `both_ways`: serde `rename` names `written` when serializing and `read_too` when deserializing, so the payload serde writes is not one serde reads. A schema and an emitted client describe one key per member, so neither can be written for the pair. Write both directions at one value, or use the single-value spelling `rename = "..."`.
```

A renaming written as a list gives serde one name to write and another to read. A schema and a client each hold one key for the member, so a party built from either cannot both read what Rust writes and write what Rust reads:

```rust
// Wrong: serde writes `both_ways` under `written`, and reads it only under `read_too`
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct SplitNames {
    #[serde(rename(serialize = "written", deserialize = "read_too"))]
    pub both_ways: u32,
    pub plain: u32,
}

// Correct: one name for the two directions, in either spelling
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct OneName {
    #[serde(rename = "written")]
    pub both_ways: u32,
    #[serde(rename(serialize = "shown", deserialize = "shown"))]
    pub plain: u32,
}
```

A list that writes one direction alone, `rename(serialize = "written")`, leaves the other at the name it had, which is two keys as well:

```text
error: model_schema: field `both_ways`: serde `rename` names `written` when serializing only, and leaves the other direction at the name it would otherwise use, so the payload serde writes is not one serde reads. A schema and an emitted client describe one key per member, so neither can be written for the pair. Write both directions at one value, or use the single-value spelling `rename = "..."`.
```

The same refusal answers `rename_all` and `rename_all_fields`, and a list written on a variant or on the type. It is raised in every build that describes the type, a mobile client included: the `serde` feature with any of `typescript`, `zod`, `dart`, `swift` and `kotlin`. A build with none of the five takes the list: the [recovering decode](recovering-decode.md#recovering-decode-decode_with) looks a value up under the `deserialize` side, and a [MongoDB path](mongodb-operations.md#mongodb-operations-and-typed-filters) is under the `serialize` side.

### Runtime Issues

#### Missing TypeScript Types

**Error:** `Cannot find name 'ObjectId'` in TypeScript.

**Solution:** Define the ObjectId type in your TypeScript project:
```typescript
// types/mongodb.ts
export interface ObjectId {
  $oid: string;
}
```

#### JSON Schema Validation Failures

**Problem:** Data doesn't validate against generated schemas.

Common causes:

1. **Field naming mismatch**: Check serde attributes and feature flags.
2. **Optional field handling**: Ensure consistent `Option<T>` usage.
3. **ObjectId format**: Must be `{ "$oid": "hex_string" }` format.

Debug by printing generated output:
```rust
println!("{}", MyType::ts_definition());

// `MyType` lists `enable = ["jsonschema"]`.
println!("{}", serde_json::to_string_pretty(&MyType::json_schema()).unwrap());
```

## Zod v4 Compatibility

This library generates Zod v4 compatible schemas exclusively. The key difference from Zod v3 is how optional fields are handled:

Zod v4 (generated by this library):
```typescript
export const User$Schema = z.strictObject({
  id: z.string(),
  name: z.string(),
  email: z.union([z.null().transform(() => undefined), z.string(), z.undefined()]).prefault(undefined),
  age: z.union([z.null().transform(() => undefined), z.number().int(), z.undefined()]).prefault(undefined),
}).meta({
  examples: []
});
```

Zod v3 style (not supported):
```typescript
// This format is NOT generated and will not work
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

Benefits of the Zod v4 approach:

- **JSON Schema generation**: Zod v4 can generate JSON schemas directly from the validation schemas.
- **Cleaner code**: No complex transform functions needed.
- **Better performance**: Eliminates runtime transform overhead.
- **Type safety**: Validates exactly the payloads the TypeScript member admits, absent key included.
