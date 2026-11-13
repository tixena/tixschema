## Feature Flags

The crate uses optional features to control code generation and dependencies. Each feature is enabled or disabled on its own, with three exceptions: `mongodb` turns on `bson`, and `bson` and `arky` turn on `serde`.

| Feature | Default | Description |
|---------|---------|-------------|
| `serde` | Yes | Serde attribute parsing (`rename`, `rename_all`, `tag`, etc.) |
| `zod` | Yes | Zod v4 schema generation alongside TypeScript types |
| `typescript` | Yes | TypeScript type generation via `ts_definition()` method |
| `mongodb` | No | MongoDB support: the ObjectId type, with validation, and on types whose [`enable` list](recovering-decode.md#opting-in) holds `"mongodb"` the [typed field paths, filters, updates and operations](mongodb-operations.md#mongodb-operations-and-typed-filters) over a collection of their rows; turns on `bson` |
| `bson` | No | BSON support: `from_bson_with` and `from_bson_piped` on every type whose [`enable` list](recovering-decode.md#opting-in) holds `"decode_with"`; turns on `serde` |
| `chrono` | No | Chrono date/time type support (`NaiveDate`, `NaiveTime`, `NaiveDateTime`, `DateTime<Tz>`) |
| `indexmap` | No | `indexmap` type support: `IndexMap` as a map and `IndexSet` as a list. See [The order of a map's keys](models.md#the-order-of-a-maps-keys) |
| `dart` | No | Dart type generation via `dart_definition()`, with a JSON `fromJson`/`toJson` codec whose read [calls each `preprocess`](preprocess.md#a-preprocess-in-the-dart-client) and [refuses what `validate()` refuses](dart.md#the-dart-read-refuses-what-validate-refuses), and a [`validate()`](dart.md#validate-in-dart) on every type |
| `swift` | No | Swift type generation with a `Codable` codec, and a [`validate()`](swift.md#validate-in-the-swift-client) on every type, which [the read of each calls](swift.md#a-swift-read-refuses-what-validate-refuses) |
| `arky` | No | `TryFrom` conversions to and from arky's frontmatter `Value` on types whose [`enable` list](recovering-decode.md#opting-in) holds `"arky"`. See [arky Frontmatter Conversions](arky.md#arky-frontmatter-conversions-arky); turns on `serde` |
| `kotlin` | No | Kotlin type generation via `kotlin_definition()`, with `kotlinx.serialization` annotations, a [`validate()`](kotlin.md#validate-in-the-kotlin-client) on every type and [a read that refuses what it refuses](kotlin.md#the-read-in-the-kotlin-client). A consuming Kotlin build declares two runtime dependencies the emitted code names -- `org.jetbrains.kotlinx:kotlinx-serialization-json` for the JSON codec and `org.jetbrains.kotlinx:kotlinx-coroutines-core` for the transport's own reply correlation and heartbeat -- plus the Kotlin Gradle plugin `kotlin("plugin.serialization")`, on Kotlin 2.0.20 or later (a model's valid class is `@ConsistentCopyVisibility`) |

Common configurations:

```toml
# Default (serde + zod + typescript)
tixschema = "0.4"

# All features including optional ones
tixschema = { version = "0.4", features = ["serde", "zod", "typescript", "mongodb", "bson", "chrono", "indexmap", "dart", "swift", "kotlin", "arky"] }

# Minimal (TypeScript only, no Zod)
tixschema = { version = "0.4", default-features = false, features = ["typescript"] }

# TypeScript + Zod with serde: the default `web` set, written out
tixschema = { version = "0.4", default-features = false, features = ["serde", "zod", "typescript"] }
```

CI tests every combination of the feature sets (`web`, `mobile`, `mongo`) via `cargo-hack`; `just test` runs every combination of the plain features locally.

The features say what tixschema can generate, and a type's [`enable` list](recovering-decode.md#opting-in) says what it generates for that type: the recovering decode for `"decode_with"`, from BSON as well under `bson`, the MongoDB layer for `"mongodb"`, under `mongodb`, and the conversions to arky's frontmatter `Value` for `"arky"`, under `arky`. A crate whose types write the list writes [the initializers](recovering-decode.md#the-support-libraries) at its root, `tixschema::decode_with_foundation!();`, and `tixschema::mongodb_foundation!();` where a type lists `"mongodb"`. `mongodb_foundation!()` writes nothing in a build without `mongodb`, and `decode_with_foundation!()` nothing in a build without `serde`, so both calls stay whatever features the crate builds with.

### A member named after a word the client language reserves

A Rust field or variant may carry a name Dart, Swift or Kotlin keeps for itself: `class` and `is` are ordinary Rust field names, `r#in` and `r#for` are legal ones, and a variant named `Default` becomes the member `default` wherever the client lower-cases it. Each emitter writes such a member in the form its language accepts, and the key on the wire stays the one serde writes:

```rust
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reserved {
    pub class: String,
    pub r#in: String,
}
```

| Target | The member | The key |
|--------|-----------|---------|
| Dart | `final String class_; final String in_;` -- Dart has no escape for a reserved word, so the member takes a trailing underscore: `Reserved(class_: ..., in_: ...)` | `json['class']`, `json['in']` |
| Swift | ``public let `class`: String; public let `in`: String;`` and ``case `class`; case `in`;`` in `CodingKeys` | `class`, `in` |
| Kotlin | ``val `class`: String, val `in`: String`` | `class`, `in`, with no `@SerialName` |

The words are each language's own: Dart's [reserved words](https://dart.dev/language/keywords), Swift's [keywords](https://docs.swift.org/swift-book/documentation/the-swift-programming-language/lexicalstructure/#Keywords-and-Punctuation) reserved in a declaration, a statement or an expression, and Kotlin's [hard keywords](https://kotlinlang.org/docs/keyword-reference.html#hard-keywords). A word a language reserves in one context only -- `type` in Swift, `value` in Kotlin, `get` in Dart -- names a member as it is. Dart also moves a field named `dynamic`, the type every emitted `fromJson` and `toJson` is written with, a field and a plain enum's member named `validate`, the method every emitted type has, and a plain enum's member named `index`, `values`, `wireValue` or `fromJsonUnjudged`, each of which a Dart enum or the emitted one already holds. Where the moved name is one the type writes itself (`class` beside `class_`), it takes a second underscore.

A plain enum's members are lower-cased by Dart and Swift, so the same rule reaches a variant: `Default` is `default_` in Dart and `` `default` `` in Swift. Kotlin keeps a variant's spelling, which no keyword matches.

Swift also moves a field named `validate`, the [method every emitted type has](swift.md#validate-in-the-swift-client): the property is `validate_`, and `CodingKeys` keeps the key, `case validate_ = "validate"`.

A service's clients follow the same rule for an operation, an argument and a path placeholder. An argument Rust needs a raw identifier for binds the placeholder written without it, `r#in` and `{in}`:

```rust
#[service_schema_op(http(
    method = "POST",
    path = "/items/{in}",
    header_in("x-tenant" = default),
))]
async fn r#for(
    &self,
    ctx: &Ctx,
    r#in: String,
    r#final: String,
    default: Option<String>,
) -> Result<Found, FindError>;
```

| Target | The call |
|--------|----------|
| TypeScript | `client.for({ in: ..., final: ... }, default_)` -- a method and a property take any word, an argument takes a trailing underscore |
| Dart | `client.for_(ForRequest(in_: ..., final_: ...), default_)` |
| Swift | ``client.`for`(ForRequest(`in`: ..., final: ...), `default`: ...)`` |
| Kotlin | ``client.`for`(ForRequest(`in` = ..., final = ...), default)`` |

The operation's wire name (`for`), the URL (`/items/...`) and the keys (`final`) are what they are for any other name.

An argument may also be named after a local the generated code writes around it -- `path`, `headers`, `body`, `request` -- and no client or dispatcher reads it as its own. The Rust transports hold a header or part argument in a local of their own. TypeScript and Dart, which take such an argument by position, move its name (`path_`); Swift keeps the label a caller writes and reads the argument by another name (`path path_: String`); Kotlin keeps the parameter and copies it before it writes its own local. Nothing a caller writes changes.

`just test-emitted` runs the emitted clients under their own toolchains. Its Kotlin leg compiles
against the serialization compiler plugin and the `kotlinx-serialization-json`,
`kotlinx-serialization-core` and `kotlinx-coroutines-core` jars, which `just kotlin-libs` installs
into `~/.local/share/tixschema/kotlin-libs`. Set `TIXSCHEMA_KOTLIN_LIBS` to use another directory.

## Generating TypeScript Files

Create a utility function to generate TypeScript files with all your types:

```rust
use std::fs;

pub enum MyEntities {}

impl MyEntities {
    pub fn get_entities() -> (String, Vec<String>) {
        (
            "Generated Types".to_string(),
            vec![
                User::ts_definition(),
                UserStatus::ts_definition(),
                PaymentMethod::ts_definition(),
                Address::ts_definition(),
            ],
        )
    }
}

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

// Run generation as a test
#[test]
fn generate_typescript() {
    generate_ts_schemas("../frontend/src/types/generated.ts").unwrap();
}
```

The bundle declares everything it names except what that header brings in. `z` is used as a value only, so the header may bind it however the consuming codebase does (`const z = zod;` works). These types are named bare and are the header's to supply:

| Type | Named where |
|------|-------------|
| `ZodType` | every schema's annotation, and a factory's parameters |
| `$brand` | a branded newtype's type |
| `SomeType` | an optional or `nullable` field that holds a generic type |
| `ObjectId` | an `ObjectId` field, under the `mongodb` feature: the consuming codebase declares it, as `{ $oid: string }` or as the type its own driver gives it |

### Alternatives

There are several ways to run the generation:

- **Test-based generation** (shown above): Run `cargo test generate_typescript` to produce the file. Simple and works well for most projects.
- **Binary crate**: Create a small binary that imports your types and calls `ts_definition()`. Run it with `cargo run --bin generate_types`.
- **`just` command**: Wrap `cargo test --test generation` in a justfile target for a convenient `just generate-ts` command.

**Note:** `build.rs` is not recommended for proc-macro libraries since the types are not available during the build script phase.

## Integration with Frontend

1. Run your TypeScript generation: `cargo test generate_typescript`
2. The generated file will include all your types and schemas
3. Import and use in your TypeScript/JavaScript code:

```typescript
import { z } from "zod";
import { User, User$Schema } from './types/generated';

// Runtime validation
const userData = User$Schema.parse(apiResponse);

// Type-safe usage
const user: User = {
  id: "123",
  name: "John Doe",
  email: "john@example.com",
  age: 30,
  is_active: true
};
```

An `http_rest` service answers its own OpenAPI 3.1 document: see [OpenAPI](http-rest.md#openapi).
