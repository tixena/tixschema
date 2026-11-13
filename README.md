# TixSchema

A Rust procedural macro library that generates TypeScript type definitions and Zod v4 validation schemas from Rust structs and enums, and whole [service definitions](docs/guide/services.md#services) -- messages, clients and dispatchers, in both languages, and [Model Context Protocol servers](docs/guide/mcp.md#the-mcp-transport) -- from Rust traits, ensuring type safety and consistency between Rust backends and TypeScript frontends.

## Installation

### Rust Dependencies

Add the following to your `Cargo.toml`:

```toml
[dependencies]
tixschema = "0.4"
serde = { version = "1.0", features = ["derive"] }
```

A crate also lists `serde_json = "1.0"` where the generated code names it:

- a type lists `enable = ["jsonschema"]`;
- a type lists `enable = ["decode_with"]`, in a build with `serde` on;
- the crate invokes a service's dispatcher, client or server macro;
- a type names a `preprocess`;
- a type or a member writes `#[serde(default)]`, in a build with `serde` and any of `zod`, `dart`, `swift` and `kotlin` on;
- a type's doc comment holds a `rust example` block, in a build with `zod` on;
- a member is written as a bare `Value` or `Map<K, V>`.

A crate that invokes a service's dispatcher, client or server macro also lists `serde_path_to_error = "0.1"`, which the generated readers use to name the path of a value serde refuses. A crate that invokes a dispatcher or server macro lists `tracing = "0.1"` beside it, and one that invokes the `amqp_rpc` server macro lists `lapin`, `tokio` and `futures` as well: each macro's doc names the crates its expansion calls.

A crate that declares any `pattern` also names `regex`, because the generated check calls it directly.

### Frontend Dependencies (Zod v4)

**TypeScript 5.7 or later** reads the generated types: a `Vec<u8>` is `Uint8Array<ArrayBuffer>`, which earlier releases do not parse.

**Important:** This crate requires Zod 4.3.2 or later. Zod v3 is not supported. An earlier Zod 4 release refuses every payload of a variant that holds a struct under `#[serde(tag = "...")]`: the schema intersects the tag's strict object with the struct's, and before 4.3.2 each side refuses the other's keys.

```bash
npm install zod@^4.3.2
# or
yarn add zod@^4.3.2
```

### Kotlin Dependencies

**Kotlin 2.0.20 or later** compiles the generated types: the valid class of a model type is `@ConsistentCopyVisibility`, which makes its `copy()` as private as its constructor, and Kotlin declares it from 2.0.20.

## Quick Start

```rust
use tixschema::model_schema;
use serde::{Deserialize, Serialize};

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    pub age: u32,
    pub is_active: bool,
}
```

This generates:
- `User::ts_definition()` -- Returns a TypeScript type definition and Zod schema as a string
- `User::json_schema()` -- Returns a JSON schema (requires `enable = ["jsonschema"]`)

Generated TypeScript:

```typescript
export type User = {
  id: string;
  name: string;
  email: string;
  age: number;
  is_active: boolean;
};

export const User$Schema: ZodType<User> = z.strictObject({
  id: z.string(),
  name: z.string(),
  email: z.string(),
  age: z.number().int().min(0).max(4294967295),
  is_active: z.boolean(),
}).meta({
  examples: []
});
```

## Documentation

The guide is one file per topic, under `docs/guide/`, in the order a crate meets them:

- [Models](docs/guide/models.md): Structs, published names, unknown keys, optional fields, defaults, collections and maps, numbers, bytes, ranges, results and the unit type.
- [Enums](docs/guide/enums.md): Every tagging serde writes and how each surface describes it.
- [Flattened members](docs/guide/flatten.md): `#[serde(flatten)]`: the shapes that read back, and the build errors that name the fix for the others.
- [Untagged enums](docs/guide/untagged-enums.md): `#[serde(untagged)]`: the variant order serde reads in, and what it refuses.
- [Nested, recursive and aliased types](docs/guide/nested-recursive-and-aliased-types.md): Types that hold each other, types that hold themselves, and type aliases.
- [Type parameters and brands](docs/guide/generics-and-brands.md): Generic types, `default_types`, and branded newtypes.
- [Services](docs/guide/services.md): `#[service_schema]`: messages, errors, faults, validation, what a service generates, and the `amqp_rpc` walkthrough.
- [The `http_rest` transport](docs/guide/http-rest.md): Routes, bodies, headers, streams, multipart and the OpenAPI document.
- [The `mcp` transport](docs/guide/mcp.md): Serving a service to an MCP host and calling one.
- [The `ws_rpc` transport](docs/guide/ws-rpc.md): A service over a WebSocket.
- [The form each client sends](docs/guide/client-forms.md): What each client writes on the wire for each type.
- [Field validation](docs/guide/validation.md): `#[model_schema_prop(...)]` bounds, `validate()`, and where a constraint is checked.
- [Preprocess and check](docs/guide/preprocess-and-check.md): Correcting a value before it is read, and judging it after.
- [Upgrading](docs/guide/upgrading.md): What a crate that upgrades meets, each rule with what a build or a read answers.
- [Checks](docs/guide/checks.md): `check`: a rule written once in Rust and run on every surface.
- [`validate()` and the read in Dart](docs/guide/dart.md): What the Dart read refuses, and `preprocess` and `check` in Dart.
- [`validate()` and the read in Kotlin](docs/guide/kotlin.md): What the Kotlin read refuses, and `preprocess` and `check` in Kotlin.
- [`validate()` and the read in Swift](docs/guide/swift.md): What the Swift read refuses, and `preprocess` and `check` in Swift.
- [Literal values and type names](docs/guide/literals-and-type-names.md): `literal` and `as`.
- [Preprocessing](docs/guide/preprocess.md): `preprocess`: the function a read calls first, on every surface.
- [Optional and nullable keys](docs/guide/optional-and-nullable-keys.md): `ts_optional` and `nullable`.
- [Compiler-validated examples](docs/guide/examples.md): Examples the compiler checks against the type they describe.
- [MongoDB `ObjectId`](docs/guide/object-id.md): `ObjectId` on every surface.
- [Chrono dates and times](docs/guide/chrono.md): Dates and times, and `as_number`.
- [Recovering decode](docs/guide/recovering-decode.md): `decode_with`: reading stored or received values that may not match their model.
- [arky frontmatter](docs/guide/arky.md): Converting to and from arky's frontmatter `Value`.
- [MongoDB operations and typed filters](docs/guide/mongodb-operations.md): Reading and writing a model's own rows through typed filters, updates and operations.
- [Feature flags and generation](docs/guide/features.md): Every feature, generating the TypeScript files, and wiring the frontend.
- [Serde attributes](docs/guide/serde-attributes.md): Which serde attributes each surface reads.
- [Troubleshooting](docs/guide/troubleshooting.md): Notes, the errors the macros answer and what each asks, and Zod v4.
