# TixSchema

A Rust procedural macro library that generates TypeScript type definitions and Zod v4 validation schemas from Rust structs and enums, and whole [service definitions](#services) -- messages, clients and dispatchers, in both languages -- from Rust traits, ensuring type safety and consistency between Rust backends and TypeScript frontends.

## Installation

### Rust Dependencies

Add the following to your `Cargo.toml`:

```toml
[dependencies]
tixschema = <path or crate_id or repo>  # eg: { git = "https://github.com/tixena/tixschema.git" }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

A crate that declares any `pattern` also names `regex`, because the generated check calls it directly.

### Frontend Dependencies (Zod v4)

**Important:** This crate requires Zod v4 for full functionality, especially JSON schema generation. Zod v3 is not supported.

```bash
npm install zod@^4.0.0
# or
yarn add zod@^4.0.0
```

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
- `User::json_schema()` -- Returns a JSON schema (requires `jsonschema` feature)

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
  age: z.number().int(),
  is_active: z.boolean(),
}).meta({
  examples: []
});
```

## Usage

### Structs

Any Rust struct annotated with `#[model_schema()]` generates a corresponding TypeScript type and Zod schema. Primitive types map as follows: `String` to `string`, `bool` to `boolean`, all numeric types to `number` (integers get `.int()`), `Option<T>` to `T` under a key the omission attribute decides ([Optional Fields](#optional-fields)), `Vec<T>` to `Array<T>`, and `HashMap<String, T>` to `Partial<Record<string, T>>`.

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct UserProfile {
    pub user_id: String,
    pub name: String,
    pub age: u32,
    pub score: f64,
    pub is_verified: bool,
}
```

#### Unit Structs

A unit struct -- `pub struct Ping;`, no braces, no fields -- crosses the wire as `{}`. Serde's own derive would write and read it as `null`, disagreeing with every other surface, which already describes an empty struct as an object; the macro replaces the author's own `Serialize`/`Deserialize` derive with impls that write `{}` and read it back (ignoring unknown keys, refusing `null`), keeping every other derive and attribute exactly as written:

```rust
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ping;

assert_eq!(serde_json::to_string(&Ping).unwrap(), "{}");
serde_json::from_str::<Ping>("{}").unwrap();
serde_json::from_str::<Ping>("null").unwrap_err();
```

TypeScript, Zod and JSON Schema describe it exactly as a braced empty struct already does: `Record<string, never>`, `z.strictObject({})`, `{"type": "object", "additionalProperties": false, "properties": {}, "required": []}`. Dart, Swift and Kotlin each publish their own no-data type -- Dart a `const` class, Swift an empty `Codable` struct, and Kotlin a serializable `object Ping` rather than a `class` (a braced `struct Ping {}` still publishes a `class`, since it is a different shape). Declared above a service as an operation's success, `Result<Ping, E>`, a unit struct reads like a `()` success on every client -- the limit is a registry's own: the struct must be declared in the same crate, above the service, or the client sees the field-position type instead (and the call still succeeds, since both sides still agree the wire is `{}`).

### Published Names (`name`)

A type publishes under the Rust ident it is declared with, spelled exactly as written. Nothing is read off that spelling -- no suffix is taken off it and no part of it is rewritten -- so `UserData` publishes as `UserData` on all three surfaces.

`#[model_schema(name = "...")]` is the one way to publish under something else. It moves the name everywhere at once: the `export type` line, the Zod consts, the `$defs` key a self-referential document hoists under, and every reference another type writes.

```rust
#[model_schema(name = "ContextValue")]
#[derive(Serialize, Deserialize)]
pub struct ContextValueData {
    pub label: String,
}

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Holder {
    pub value: ContextValueData,
}
```

Generated TypeScript (`JSDoc` blocks elided):

```typescript
export type ContextValue = {
  label: string;
};

export type ContextValueData = ContextValue;

export type Holder = {
  value: ContextValue;
};
```

The `export type ContextValueData = ContextValue;` line is the one place the Rust ident is written on purpose. A type declared *above* the renamed one has only that ident to name it by -- an override is not recoverable from it -- so both spellings answer, and the alias carries the one type. The Zod surface publishes the same pair, `export const ContextValueData$Schema = ContextValue$Schema;`, and the Rust module the schema lives in is named from the ident for the same reason: an override never moves it.

A type alias is the one shape with no surface name of its own, and publishes under a `Type` suffix -- `pub type Slug = String;` as `SlugType` -- unless `name` says otherwise. See [Type Aliases](#type-aliases).

Two declarations cannot publish under one name. The emitted types, schemas and definitions are one flat namespace, so a second declaration reaching a name already taken is refused at expansion, in every feature configuration, naming both declarations:

```text
error: model_schema: type `Beta` publishes as `Shared`, which type `Alpha` already publishes as
       -- one name cannot carry two declarations, whose types, schemas and definitions would
       overwrite each other. Give one of them a `#[model_schema(name = "...")]` of its own
```

### Optional Fields

`Option<T>` fields validate as `z.union([z.null().transform(() => undefined), type, z.undefined()]).prefault(undefined)` in Zod v4 and are left out of the JSON Schema's `required` list, whose `anyOf: [type, { "type": "null" }]` describes the same two spellings serde reads into `None`. The `.prefault(undefined)` makes the field default to `undefined` when omitted from the input, and the leading `z.null()` arm coerces an explicit `null` to the same `undefined`.

TypeScript writes every `Option<T>` field as `field: T | undefined`. The one thing that writes the other spelling, `field?: T`, is [`#[model_schema_prop(ts_optional)]`](#ts_optional) on the author's word. A serde attribute that drops the key decides what reaches the wire — the JSON Schema's `required` list and the Zod schema both read it — and never which of the two TypeScript spellings is written. [`#[model_schema_prop(nullable)]`](#nullable-object-keys-nullable) writes a third spelling, `field: T | null`, for the field whose `None` reaches the wire as an explicit `null` rather than an absent key.

Each flavor is pinned to the one shape its declared type describes, and the `serde` feature enforces the pin. A bare `Option<T>` field is a compile error unless it carries a key-dropping attribute (`skip_serializing_if`, `skip_serializing`, or `skip`) — its declared shape is `T | undefined`, so a `None` has to leave the key out, not reach the wire as `null`. `nullable` is refused the opposite way: pairing it with a key-dropping attribute is a compile error too, since the flag declares the key always written. Either violation names the field and the attribute to add or remove. Deserialization does not enforce either pin: `{}` and `{"a":null}` both read into `None` under both flavors, so an optional key accepts an explicit `null` on input even on a field whose own serializer never writes one.

A bare `#[serde(skip)]` is not that. serde writes the key into no payload *and* throws it away out of every payload that supplies one, so there is nothing on the wire for any surface to describe: TypeScript writes no member, Zod no key, and the JSON Schema neither a `properties` entry nor a `required` one. `#[serde(skip_serializing, skip_deserializing)]` is the same wire spelled out and is answered the same way. Note what this costs on the way in — a `z.strictObject` and an `additionalProperties: false` both *reject* a payload carrying that key, while serde accepts such a payload and discards the value. The schemas describe the payload serde writes, and that key appears in none of them; a member under an optional key would instead claim the key is sometimes written, and would describe a value nothing ever reads.

`#[serde(skip_deserializing)]` on its own drops neither surface's key: serde writes it in every payload, so the member keeps a required key, and the value a payload supplies under it is discarded on the way in.

A positional slot answers by that same wire, read in the place a tuple writes rather than under a key. A slot carrying a bare `#[serde(skip)]` is absent from the array serde writes *and* from every array serde reads, and the slots behind it move up: `struct Pair(#[serde(skip)] Option<String>, String)` holding `(Some("s"), "x")` writes `["x"]`, reads that back, and refuses `["s","x"]`. So the described tuple is the slots that remain — `[string]` in TypeScript, `z.tuple([z.string()])` in Zod, and one `prefixItems` entry under `minItems`/`maxItems` of 1. Dropping every slot leaves `[]`, which is the array serde writes there.

A slot dropped from only one of the two directions has no such description and is refused where it is declared. `skip_serializing` alone writes `["x"]` and reads only `["s","x"]`; `skip_deserializing` alone writes `["s","x"]` and reads only `["x"]`. A named field in that position is still describable — the key is absent from one payload and present in the other, which an optional key covers — but a slot is written by its place, so the two payloads differ in their arity and no fixed-arity tuple describes both.

A struct declaring a single slot keeps it whatever these attributes say: serde writes and reads a newtype struct's only slot regardless, so nothing there is dropped.

A tuple *variant*'s slots read the same way, with one difference at the lone-slot arity. `enum E { One(#[serde(skip)] String, u32) }` holding `("s", 7)` writes `{"One":[7]}`, reads that back, and refuses `{"One":["s",7]}`, so the described tuple shrinks exactly as a struct's does — under the content key when one is named, and to `{"One":[]}` when every slot is dropped. Where a variant differs is that it has its own name to fall back on: a variant declaring one slot and dropping it is written as a unit variant, `"One"` externally, `{"type":"One"}` under a tag and `null` untagged, and each surface describes it as the unit it has become. Because serde does read a variant slot's attributes at every arity, the one-directional halves are refused there at every arity too — `One(#[serde(skip_serializing)] String)` writes `"One"` and reads only `{"One":"s"}`.

Which key a field writes is read off the attribute in every build, `serde` feature or not — one declaration describes one wire under every toggle. What the feature buys is the renaming, the tagging and the guards.

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct UserWithOptionals {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
}
```

Generated TypeScript:

```typescript
export type UserWithOptionals = {
  id: string;
  name: string;
  email: string | undefined;
  phone: string | undefined;
  avatar_url: string | undefined;
};
```

### A Key Read as Its Default (`#[serde(default)]`)

serde reads a key the payload leaves out as a default where a field of a struct, or a named member of an enum's variant, writes `#[serde(default)]` or `#[serde(default = "path")]`, or where the struct writes either one on itself. Every surface reads that key as the same default. The value of a default is known to Rust alone: the `Default` of the field's type, the value the named function answers, or the member of the struct's own default. So `zod_schema()`, `json_schema()`, `dart_definition()`, `swift_definition()` and `kotlin_definition()` write each default with serde when they build their text, and write the JSON into the text. The field's `serialize_with` or `with` writes the default where the field names one, as serde writes the field. A crate that declares such a type lists `serde_json`, as [Rust Dependencies](#rust-dependencies) shows.

```rust
pub fn north_dock() -> String {
    "north".to_owned()
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Wharf {
    #[serde(default)]
    pub dock: String,
    #[serde(default)]
    pub gangway: Gangway,
    #[serde(default = "north_dock")]
    pub gate: String,
    pub label: String,
}
```

`Gangway` is a `#[model_schema()]` struct of the crate's own, whose `Default` is `{"label":"main","width":2}`. serde reads `{"label":"w"}` as `{"dock":"","gangway":{"label":"main","width":2},"gate":"north","label":"w"}`, and so does each surface:

| Surface | What reads a key the payload leaves out |
|---------|-----------------------------------------|
| Zod | `dock: z.string().prefault("")`: the parse is handed the default as a payload, so the checks of the member judge it |
| TypeScript | nothing changes: the type a parse answers holds every member, `dock: string;` |
| JSON Schema | the key is not in `required`, and the member states `"default": ""` |
| Rust | serde's own read, then `validate()` |
| Dart | `json.containsKey('dock') ? (json['dock'] as String) : (jsonDecode('""') as String)`, in the read the member already has |
| Swift | `if container.contains(.dock) { ... } else { ... }`: the default is read from the JSON with the read the member already has |
| Kotlin | `val dock: String = Json.decodeFromString(serializer<String>(), "\"\"")`: the plugin evaluates the default of the property for a key the payload leaves out |

- **A key the payload holds is read as it was**, on every surface.
- **A default is judged as a payload is.** The default is read with the read of the member, and then `validate()` judges the value. So a default that breaks a bound of the member is refused with the member named, on every surface: `#[model_schema_prop(minLength = 1)] #[serde(default)] pub dock: String` refuses `{"label":"p"}` with `'dock': too short: minimum length is 1, got 0`.
- **A function a `preprocess` names is not called for a default**, in Rust, Dart, Swift and Kotlin. The Zod schema calls it with the default: `.prefault` is written around the whole schema of the member, so the function is handed the default.
- **The Dart file imports `dart:convert`**, for `jsonDecode`. **The Swift file imports `Foundation`**, for `JSONSerialization`. The Kotlin file imports the `json` package, as [the read in the Kotlin client](#the-read-in-the-kotlin-client) shows.
- **The Kotlin client writes a member with a default only where its value is another.** `kotlinx.serialization` with the default `Json` (`encodeDefaults = false`) writes `Jetty()` as `{}`, which serde reads as the same value. A `Json { encodeDefaults = true }` writes each member, as serde writes it.
- **An `Option` keeps what it had.** `#[serde(default)]` on an `Option` changes nothing on any surface: [Optional Fields](#optional-fields) describes it.
- **A member whose key serde leaves out of what it writes keeps the reading [Optional Fields](#optional-fields) describes.** That is a member under `skip_serializing` or `skip_serializing_if`: each surface already reads its key as optional.

**On a struct, a default with no JSON keeps the key required, and the struct builds.** serde still reads such a key as its default, but no surface other than Rust is handed the default, so each one requires the key as it did before. That is the case where:

- the struct does not derive both `Serialize` and `Deserialize`, or reads or writes itself through `from`, `try_from`, `into` or `remote`;
- the type of the field names a type parameter of the struct, a lifetime or `Self`, and the field is not a list, a set or a map under the bare `#[serde(default)]`. The default of `T`, and the `Vec<T>` a function answers, hold one value for each type that fills `T`, so the text has no one value to write. A field of the same struct whose type names no parameter is read as its default;
- the field is a list, a set or a map whose type names a parameter, and serde writes it through a `serialize_with` or a `with` of the field's: only that function knows the JSON of the empty collection. Without one, a list, a set or a map under the bare `#[serde(default)]` is read as its default on every surface: the default is the empty collection at every filling, so `Vec<T>`, `VecDeque<T>`, `BinaryHeap<T>`, `HashSet<T>` and `BTreeSet<T>` are read as `[]`, and `HashMap<K, T>` and `BTreeMap<K, T>` as `{}` (`IndexSet` and `IndexMap` too, under the [`indexmap` feature](#feature-flags)). The Kotlin client writes that default as `emptyList()` or `emptyMap()`, a property's initializer having no serializer of the parameter to read `[]` with;
- the default is the struct's own, and the struct declares a parameter;
- the field is `#[serde(flatten)]`.

A `Serialize` that refuses to write a default makes the function that builds the text panic, as a refused example makes `schema_examples()` panic.

#### A member of an enum's variant

A named member of a variant under `#[serde(default)]` or `#[serde(default = "path")]` is read as its default on every surface, as a struct's member is, whatever the enum's tagging: no tag attribute, `tag`, `tag` with `content`, or `untagged`. A variant has no default of its own: serde reads `default` on a variant's field alone.

```rust
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Berthing {
    Docked {
        #[serde(default)]
        dock: String,
        pier: u32,
    },
    Adrift,
}
```

serde reads `{"kind":"Docked","pier":3}` as `{"kind":"Docked","dock":"","pier":3}`, and so does each surface. Each tagging writes the payload that leaves `dock` out in its own form, and each surface reads `dock` as `""` in every one of them:

| The enum writes | A payload that leaves `dock` out |
|-----------------|----------------------------------|
| no tag attribute | `{"Docked":{"pier":3}}` |
| `#[serde(tag = "kind")]` | `{"kind":"Docked","pier":3}` |
| `#[serde(tag = "kind", content = "data")]` | `{"kind":"Docked","data":{"pier":3}}` |
| `#[serde(untagged)]` | `{"pier":3}` |

- **Each surface writes the member as it writes a struct's**, in the variant's own object: the Zod member takes `.prefault("")`, the JSON Schema leaves the key out of the variant's `required` and states `"default": ""`, and the Dart, Swift and Kotlin clients read the default where the payload leaves the key out.
- **A key the payload holds is read as it was**, on every surface.
- **A default is judged as a payload is.** A default that breaks a bound of the member is refused with the member named, on every surface.
- **A type that holds or flattens an untagged enum with such a member joins the enum's own schema in its Zod text.** The enum's schema is the one text the default is written into.

#### What does not build on a variant's member

A named member of a variant under `#[serde(default)]` or `#[serde(default = "path")]` that the other surfaces cannot read as serde reads it is refused where the enum is declared, and the build stops. No key of a variant is kept required in silence. The refusal points at the member, and its message opens with the enum, the variant and the member, ``model_schema: type `Berthing`: variant `Docked`, field `dock`:``, followed by the reason. Every refused member of the enum is reported in one build. It is raised in every build that describes the type, a mobile client included: the `serde` feature with any of `typescript`, `zod`, `jsonschema`, `dart`, `swift` and `kotlin`.

| The member | Why no surface but Rust can read the default | The message, after its opening |
|------------|-----------------------------------------------|--------------------------------|
| Its type names a type parameter of the enum, a lifetime or `Self`, and it is not a list, a set or a map under the bare `#[serde(default)]` with no `serialize_with` or `with` | The default holds one value for each type that fills the parameter, so the text has no one JSON to write | `` `#[serde(default)]` is read only by Rust here: the type names the type parameter `T`, whose default has one JSON for each type that fills it. Give the field a type that names no parameter, or remove the default. `` |
| It is `#[serde(flatten)]` | serde reads each missing key of the flattened type from that type's own default, which no other surface describes | `` `#[serde(default)]` on a flattened field is read only by Rust. Remove the default, or the flatten. `` |
| It is under `skip_serializing` or `skip_serializing_if`, and it is not an `Option` | Each surface reads its key as optional, and answers no value where serde reads the default | `` `#[serde(default)]` beside `skip_serializing_if` is read only by Rust on a field that is not an `Option`. Make the field an `Option`, or remove one of the two. `` |
| The enum derives `Deserialize` and not `Serialize`, or names `into` or `remote` | The JSON of the default is written with serde, through the type the surfaces describe, and this enum gives no such type | `` `#[serde(default)]` needs the enum to derive `Serialize`, with no `into` or `remote`, so the default can be written for the other targets. `` |

The first message names what the type names: a const parameter, whose default has one JSON for each value that fills it; a lifetime, or `Self`, which name nothing where the JSON of the default is built, and for `Self` the fix is to write the enum's name in its place. The third names the key the member is written under, `skip_serializing` or `skip_serializing_if`.

These build as they did, since no surface reads the member otherwise than serde does:

- **An `Option`.** Every surface reads the missing key as absent, which is serde's default, `None`.
- **A member under `skip` or `skip_deserializing`.** serde never reads its key, so no missing key is read as the default.
- **An enum that derives no `Deserialize`, or reads itself through `from` or `try_from`.** serde never reads the fields of the enum.
- **A list, a set or a map under the bare `#[serde(default)]` whose type names a parameter**, with no `serialize_with` or `with`: it is read as `[]` or `{}` on every surface, as a struct's is.

**A struct's member keeps the struct's rules.** On a struct, the same four cases build, and each surface reads the key as the section says above: required, or optional under `skip_serializing` or `skip_serializing_if`.

### Collections and Maps

`Vec<T>` becomes `Array<T>`, and so does every other std wrapper serde writes as a JSON array of its element: `VecDeque<T>`, `BinaryHeap<T>`, `HashSet<T>`, and `BTreeSet<T>`. Under the [`indexmap` feature](#feature-flags), `indexmap::IndexSet<T>` is one more. Each is that array on the wire, so each is typed and validated as that array — the element decides what the array holds. A set is that array with its items distinct: see [A set holds each item once](#a-set-holds-each-item-once). Nesting is kept at whatever depth it is written: a `Vec<Vec<T>>` (or `HashSet<Vec<T>>`, or any other mix of those wrappers) writes an array of arrays, so it becomes `Array<Array<T>>`, `z.array(z.array(...))`, and a JSON schema whose `items` is itself an array.

`LinkedList<T>` is the one std sequence left out. It writes that same array, so the covered spellings already describe every wire form it has, and a type written with one is refused at compile time with the rewrite named: use `Vec<T>`, or `VecDeque<T>` where values are pushed at both ends.

A fixed-size array `[T; N]` is that array too, with its length described. serde writes exactly `N` items and reads one back only at that length, so the two validating surfaces say so: the JSON schema pins the level with `"minItems": N` and `"maxItems": N`, and Zod appends `.length(N)`. The bound belongs to the level it was written at, so `Vec<[T; 3]>` is an unbounded array of 3-element arrays and `[Vec<T>; 3]` is a 3-element array of unbounded ones. TypeScript stays `Array<T>`: the fixed-length form its type system has is the N-element tuple, which has to be written out element by element and stops being readable long before `N` stops being legal. A length the macro cannot read — a const generic parameter, a `const` item, any computed expression — describes as an unbounded array, the macro running before there is a value to ask for; a slice `[T]` has no length to describe at all.

```rust
use std::collections::HashMap;

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct UserWithCollections {
    pub id: String,
    pub tags: Vec<String>,
    pub scores: Vec<u32>,
    pub metadata: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<HashMap<String, String>>,
}
```

`HashMap<K, V>` and `BTreeMap<K, V>` are both read as maps, and so is `indexmap::IndexMap<K, V>`, under the [`indexmap` feature](#feature-flags): [The order of a map's keys](#the-order-of-a-maps-keys) says what sets the three apart. `serde_json::Map` is refused, as [A JSON value is no member](#a-json-value-is-no-member) says. The key decides what the map describes as. A `String` key is the open case: any string is a key, so the JSON schema puts the value's schema under `"additionalProperties"`, TypeScript writes `Partial<Record<string, V>>`, and Zod writes `z.record(z.string(), V)`.

A key written as a plain `#[model_schema()]` enum is the narrowed case: the enum's members become the object's properties and nothing else is allowed, so the JSON schema lists each member with the value's schema and closes the object with `"additionalProperties": false`, while TypeScript and Zod name the enum itself as the key type — `Partial<Record<Slot, V>>` and `z.partialRecord(Slot$Schema, V)`. Zod's constructor is `z.partialRecord` because `z.record` over a key whose schema enumerates its members demands every one of them, while a `HashMap` holds any subset and serde writes the subset; a key outside the enumeration is refused either way. The members are named as serde writes them, so a `#[serde(rename_all = "...")]` on the enum renames the properties too. This holds wherever the enum is declared relative to the type that writes the map, and through a `#[model_schema()]` alias of one.

```rust
use std::collections::HashMap;

#[model_schema()]
#[derive(Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Slot {
    Primary,
    Secondary,
}

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Schedule {
    // {"type": "object", "properties": {"Primary": {...}, "Secondary": {...}},
    //  "additionalProperties": false}
    pub slots: HashMap<Slot, String>,
}
```

A `#[serde(transparent)]` branded newtype over a string — a brand over `String` or `PathBuf`, or over another such brand — is the open case wearing a name. serde writes the brand as the bare string its inner is, which is exactly what a JSON object key is, so the map is an object keyed by arbitrary strings. TypeScript and Zod keep the brand's own spelling as the key type the way they keep an enum's — `Partial<Record<CorrelationId, V>>` and `z.record(CorrelationId$Schema, V)` — while the JSON schema is the open object, having no brand to say.

```rust
use std::collections::HashMap;

#[model_schema()]
#[derive(Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct CorrelationId(String);

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Traces {
    // {"type": "object", "additionalProperties": {"type": "string"}}, serialized {"abc": "..."}
    pub spans: HashMap<CorrelationId, String>,
}
```

A brand adds a name to its inner's wire and nothing else, so it keys a map however its inner keys one — at every link of a chain of brands. A brand over a **plain enum** is the open case on the structural surface: serde writes the variant name, which is a bare string, and the brand publishes no `enum_members()` of its own for a schema to close the object over, so it opens the object under its own name rather than closing it over members nothing can supply. Its Zod binding is a different matter — `.brand()` is type-level only, so what it publishes is the enum's own `z.enum`, which enumerates, and the map is written `z.partialRecord(SlotBrand$Schema, V)`. A brand over a value serde **stringifies** — a number, a `char`, a chrono `Naive*` type — describes exactly as that bare inner describes: its values, and its keys by the text the inner is written as, while TypeScript and Zod keep the brand's name as the key type. A brand over a `bool` or a `DateTime<Tz>` is the one place that name has nowhere to land: `boolean` and `Date` spell no TypeScript property key, and the only binding such a brand publishes is the value-shaped schema Zod refuses or rewrites in key position — so the key renders in the form serde writes it, exactly as the bare inner does, and `HashMap<Enabled, V>` is `Partial<Record<"true" | "false", V>>` and `z.partialRecord(z.enum(["true", "false"]), V)`. Only a brand over something serde writes as no key at all — a struct, a container, a tuple, a nested map, an `ObjectId` — is refused, and so is a brand over an inner this expansion has not classified yet: what serde writes for such an inner is exactly what cannot be told.

A `#[model_schema()]` **alias** answers the same way, its target standing in for it because a type path resolves straight through an alias. An alias of `String` or `PathBuf`, of a string-shaped brand, or of another such alias is written as that bare string, so it keys a map exactly as a `String` does — under the alias's own exported name on the nominal surfaces and as the open object on the structural one. An alias of a plain enum enumerates its members, as it always has.

```rust
use std::collections::HashMap;

#[model_schema()]
type SlotKey = String;

#[model_schema()]
#[derive(Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct Tick(u32);

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Samples {
    // {"type": "object", "additionalProperties": {"type": "number"}}, serialized {"abc": 1.0}
    // TypeScript `Partial<Record<SlotKeyType, number>>`, Zod `z.record(SlotKeyType$Schema, ...)`
    pub by_slot: HashMap<SlotKey, f64>,
    // {"type": "object", "additionalProperties": {"type": "number"},
    //  "propertyNames": {"type": "string", "pattern": "^-?[0-9]+(\\.0+)?$"}}, serialized {"7": 1.0}
    // TypeScript `Partial<Record<Tick, number>>`, Zod `z.record(Tick$Schema, ...)`
    pub by_tick: HashMap<Tick, f64>,
}
```

A key path is the one spelling that can name an enum, an alias or a brand, so a key path the expansion can prove serde writes as no key at all — a struct, a brand over one or over a container, a tagged or untagged enum, or a `#[model_schema()]` alias of any of them — is refused where it is written, at whatever depth the map sits at, naming the type as the author spelled it. A key path the expansion has not seen yet, one declared after the type that writes the map or one from another crate, cannot be ruled out and is emitted as an enumerating key; a key that turns out to have no members surfaces as an `E0599` at the key type instead.

The remaining refusals are all one rule: a JSON object key is a string, and `serde_json` raises `key must be a string` — refusing the whole map at serialization, with no fallback form — for every key whose own wire form is not one. Each of these is therefore refused rather than described, there being no object to describe:

- a **sequence-wrapped** key — a `Vec`, a `[T; N]`, one of the sets — which writes a JSON array;
- a **tuple** key, which writes a JSON array;
- a **nested map** key and an **`ObjectId`** key, which write JSON objects;
- an **`Option`** key, whose `Some` writes what its inner writes but whose `None` writes nothing a key can be — so a single `None` fails the whole map at runtime, and a schema for the `Some` half alone would describe a contract the type does not keep.

All of them are covered under [Compilation Errors](#compilation-errors), with the exact diagnostic each produces.

Every other key is neither open nor enumerable, and none of them is refused: serde stringifies each one into a key for you. serde writes the object with the key's string form for its keys: `7` becomes `"7"`, `true` becomes `"true"`, a chrono `NaiveDate` its ISO rendering. The JSON schema describes such a map at every position, [flattened](#intersection-types-serdeflatten) or under a key, as it describes a map keyed by a `String`, its values in `additionalProperties`, and names its keys by that text in `propertyNames`:

| key | `propertyNames` |
|-----|-----------------|
| `char` | `{"type": "string", "minLength": 1, "maxLength": 1}` |
| `NaiveDate` / `NaiveTime` / `NaiveDateTime` / `DateTime<Tz>` | the type's own description, `{"type": "string", "format": "date"}` for a `NaiveDate` |
| `u8`–`i64`, `usize`, `isize` | `{"type": "string", "pattern": "^-?[0-9]+(\\.0+)?$"}` |
| `f32`, `f64` | `{"type": "string", "pattern": "^-?[0-9]+(\\.[0-9]+)?$"}` |
| `bool` | `{"type": "string", "pattern": "^(true\|false)$"}` |
| a brand or an alias over one of them | its own description, a number's and a `bool`'s read as the pattern of its text |

`HashMap<u32, u8>` refuses `{"7": "x"}` and `{"x": 1}`, as serde and Zod refuse them. Where serde and Zod part on the text of a number key, the pattern follows Zod: it admits a leading zero, `-0` and a fraction of zeros on an integer key, which Zod reads as the integer and serde refuses. It states no range, as `{"type": "integer"}` states none for a value. The rule the refusals apply is refuse-what-serde-refuses, never refuse-what-is-not-a-`String`.

On the two nominal surfaces such a key keeps its own type wherever that type spells a TypeScript property key, and renders in the form serde **writes** it wherever it does not:

| key | TypeScript | Zod |
|-----|------------|-----|
| `u8`–`i64`, `usize`, `isize` | `number` | `z.record(z.number().int(), V)` |
| `NaiveDate` / `NaiveTime` / `NaiveDateTime` | `string` | the type's own schema |
| `bool` | `"true" \| "false"` | `z.partialRecord(z.enum(["true", "false"]), V)` |
| `DateTime<Tz>` | `string` | `z.record(z.iso.datetime({ offset: true }), V)` |

So a `HashMap<u32, String>` is `Partial<Record<number, string>>` and `z.record(z.number().int(), z.string())`, and a `HashMap<bool, String>` is `Partial<Record<"true" | "false", string>>` and `z.partialRecord(z.enum(["true", "false"]), z.string())`. `Record<K, V>` is declared `K extends keyof any`, which `boolean` and `Date` do not satisfy, and in key position `z.boolean()` rejects the `"true"` serde wrote while the date schema accepts the timestamp only to rewrite every key into a locale-dependent `Date.prototype.toString()` rendering — neither surface can carry the value form, so both carry the wire form instead. The constructor moves with the key because `z.record` over an enumerated key demands every one of its members; `z.partialRecord` (zod ≥ 4.0.0) admits a map holding one of them, or none. A brand or alias over one of those keys renders exactly as the bare key does, its name having nothing to stand in.

A key written as one of the enclosing item's own type parameters is the open case, read off that same rule. The expansion cannot see which type the instantiation will supply, but serde has already settled what any of them may write: an instantiation whose key writes as a string keys the map, and one whose key does not fails the whole map at serialization with `key must be a string` — so string keys hold for every instantiation that serializes at all. All three surfaces say exactly that and nothing more: `{"type": "object", "additionalProperties": V}`, `z.record(z.string(), V)`, and `Partial<Record<string, V>>` — the trio a `String` key earns. The value side stays described, being no parameter's business, so `HashMap<KeyType, ValueType>` on `Holder<KeyType, ValueType>` is `Partial<Record<string, ValueType>>`.

The key position is the one place TypeScript stops rendering a parameter as itself — see [Type Parameters](#type-parameters) for what it does everywhere else — and the declaration is what forces it. `Record<K, T>` is declared `K extends keyof any`, so a declaration handing it a parameter it binds without bounding does not type-check at all, and the consumer pasting the emitted `.ts` gets the error before writing a value. Bounding the parameter would move the failure rather than fix it: the bound propagates to every name written over the item, including the erased fillings a binding annotated for the opaque value supplies. What the member gives up is naming the parameter in its key; what it gains is the guarantee serde already makes about every filling that reaches the wire.

#### The order of a map's keys

A JSON object is written with its keys in an order. The three maps describe as the same object on every surface, and each does its own thing with that order in Rust:

| The map | The order of its keys |
|---|---|
| `HashMap<K, V>` | none |
| `BTreeMap<K, V>` | sorted |
| `IndexMap<K, V>` | as read |

`indexmap::IndexMap<K, V>` and the list beside it, `indexmap::IndexSet<T>`, are read under the `indexmap` feature. The feature adds no dependency to tixschema: the generated code names the types of the crate that declares them, so that crate lists `indexmap` itself, with its `serde` feature.

```toml
tixschema = { version = "0.1.0", features = ["indexmap"] }
indexmap = { version = "2", features = ["serde"] }
```

```rust
use indexmap::{IndexMap, IndexSet};

#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub data: IndexMap<String, String>,
    pub labels: IndexSet<String>,
}

let read: Document =
    serde_json::from_str(r#"{"data":{"zeta":"1","alpha":"2"},"labels":["b","a"]}"#)?;
assert_eq!(
    serde_json::to_string(&read)?,
    r#"{"data":{"zeta":"1","alpha":"2"},"labels":["b","a"]}"#
);
```

```typescript
export type Document = {
  data: Partial<Record<string, string>>;
  labels: Array<string>;
};
```

Every surface writes for each of the two what it writes for its std twin: for `IndexMap<K, V>` what a `HashMap<K, V>` gives, and for `IndexSet<T>` what a `HashSet<T>` gives. That holds for TypeScript, Zod, the JSON Schema, Dart, Swift and Kotlin, and under whatever holds the field: an `Option`, a `Vec`, another map. A key is held to the rules above, so `IndexMap<Slot, V>` over a plain enum is `Partial<Record<Slot, V>>` and `z.partialRecord(Slot$Schema, V)`. A hasher written as a further argument, `IndexMap<K, V, S>` or `IndexSet<T, S>`, says nothing about the wire. A `minLength` or a `maxLength` written on an `IndexSet` field counts its items, as on any list ([List Length](#list-length-minlength-maxlength)).

The order is the Rust type's. A schema describes an object and states no order, and each generated client holds the map in its language's own: a Dart `Map` and a Kotlin `Map` keep the order of the document. A Swift client holds every map as a dictionary, which has no order, and `JSONEncoder` writes the keys of every object in hash order: a Swift client neither keeps nor writes the order of an `IndexMap`.

The [recovering decode](#recovering-decode-decode_with) walks each of these maps entry by entry, each issue at its key's path, and an `IndexSet` item by item. Its second read is serde's, so an `IndexMap` field is filled in the order of the source: a `bson` document keeps the order the row stored, and a `serde_json::Value` holds its keys in the order of the crate's `serde_json`, as read where its `preserve_order` feature is on and sorted where it is off. Under `mongodb` a map has no typed path, whichever of the three it is, and an `IndexSet` field is a `ListField`.

A `#[model_schema()]` type of the crate's own named `IndexMap` or `IndexSet`, declared above the type that holds it, keeps its name: a field written with that name is that type, with the `indexmap` feature on or off. With the feature off, any other type written `IndexMap<K, V>` or `IndexSet<T>` is refused with the feature named. See [`IndexMap` or `IndexSet` Without the Feature](#indexmap-or-indexset-without-the-feature).

#### A set holds each item once

`HashSet<T>`, `BTreeSet<T>` and, under the `indexmap` feature, `IndexSet<T>` are arrays whose items are distinct. serde reads a payload that repeats an item and keeps the item once: `{"tags":["a","a"]}` is read as the set of the one `"a"`, and a `minLength` written on the set counts what is left. Every other surface refuses the repeat:

| Surface | `{"tags":["a","a"]}`, read as a `HashSet<String>` field |
|---|---|
| Rust | reads `{"a"}` |
| JSON Schema | refuses it: the set's array writes `"uniqueItems": true` |
| Zod | refuses it with `repeats an item: a set holds each item once`, at the path `["tags"]` |
| Dart, Swift and Kotlin | `validate()` answers `'tags': repeats an item: a set holds each item once`, so the read refuses it |
| TypeScript | `Array<string>`, as a list writes it |

- **Two items are one where they write one JSON text.** Zod compares the items it parsed, an object's keys in the order its schema declares them. Dart compares the JSON its codec writes for each item, Swift the JSON `JSONEncoder` writes with sorted keys, and Kotlin the values themselves, the emitted types comparing by value. So two objects that write their members in another order are one item on every surface.
- **The rule holds at every depth**: a set in a list, in an `Option`, as a map's value, in a slot, as a variant's member, under a brand, and a list in a set. Each repeat is reported under the place of its set: `'grid.1'` for the second set of a `Vec<HashSet<u32>>`, `'keyed.k'` for the set a map holds under `k`.
- **Zod checks a repeat after the set's count, and a client's `validate()` ahead of it.**
- **A set chooses no variant of an untagged enum by a repeat.** serde reads the payload, so each surface chooses the variant serde chooses, and then refuses the repeat under that variant's place.
- **Rust's `validate()` answers nothing of a repeat**: a Rust set never holds one, so each client agrees with Rust on every value Rust holds.

### A JSON value is no member

A member that holds `serde_json::Value` or `serde_json::Map` stops the build. Either holds any JSON value, and no target can describe one: a schema would admit anything, and a client would read the value as a type of its language's that holds anything, or as none. Written by its full path, `serde_json::Value`, `::serde_json::Value` or `serde_json::Map<K, V>`, the type is refused by a compile error at expansion, in every build, one for each place the item writes it:

- a field, a slot of a tuple struct, and a member or a slot of a variant, flattened or not;
- the value under an `Option`, a list, a map's key or value, a tuple, a `Result` and a type argument, at any depth: `Option<Vec<HashMap<String, Value>>>`, `(u32, Value)`, `Wrapper<Value>`;
- the type an alias names, the value a brand holds, and a `default_types` filling.

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Holding {
    pub v: serde_json::Value,
}
```

```text
error: model_schema: type `Holding`: field `v` reaches `serde_json::Value`, which holds any JSON value, so no target can describe it. Declare the shape that value takes as a type of its own, and write that type in its place.
```

The path is read as written: a module of the crate's own named `serde_json` is read as the crate.

A `Value` or a `Map<K, V>` written by its bare name, as `use serde_json::{Map, Value};` brings it into scope, may be `serde_json`'s type or a type of the crate's own, declared anywhere, and only the compiler can tell. So the macro writes a check beside the item, which the compiler evaluates: where the name is `serde_json`'s type, the build stops with the same message, as error `E0080`, spanned on the type as written; where it is another type, the check holds and the item builds. The check names `::serde_json`, so a crate that writes a bare `Value` or `Map<K, V>` in a member lists `serde_json` among its dependencies, whichever type the name is.

```rust
use serde_json::Value;

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Reading {
    pub data: Value,
}
```

```text
error[E0080]: evaluation panicked: model_schema: type `Reading`: field `data` reaches `serde_json::Value`, which holds any JSON value, so no target can describe it. Declare the shape that value takes as a type of its own, and write that type in its place.
```

The build reports, beside it, what the rest of the expansion names for a type of the crate's own, such as a `value_schema` module nothing declares.

No check is written for a name that is one of the item's own type parameters, for the item itself, for a type the crate declares with `#[model_schema()]` under either name above the item, for a `Value<T>` written with arguments or a `Map` written with none, which are no `serde_json` type, and for a `Map<K, V>` whose arguments name one of the item's own type parameters, `Self` or a lifetime other than `'static`, which a check beside the item cannot name.

A [service](#services) operation that takes or answers either type stops the build the same way, at each type it puts on the wire, in every build with the `serde` feature, spanned on the type as written: by its full path at expansion, and by its bare name at a check the compiler evaluates beside the trait. The error names the operation and the position: `the request`, the one message the operation takes; `the reply` and `the fault`, the two arms of its `Result`; ``request header `name` ``, ``reply header `name` `` and ``fault header `name` ``, a `header_in`, `header_out` and `error_header_out` binding; and ``request part `name` ``, a `part` binding. The message declared from several arguments is a `#[model_schema()]` struct, so an argument there is refused as a field of it.

```rust
#[service_schema()]
pub trait RawService<Ctx> {
    async fn fetch(&self, ctx: &Ctx, req: serde_json::Value) -> Result<Balance, FetchError>;
}
```

```text
error: service_schema: operation `fetch`: the request reaches `serde_json::Value`, which holds any JSON value, so no target can describe it. Declare the shape that value takes as a type of its own, and write that type in its place.
```

A context named `Value` or `Map` is the service's context, and a type the crate declares with `#[model_schema()]` under either name, above the service, is that type.

The functions tixschema writes that take or answer a `serde_json::Value` are unchanged: the function a [`preprocess`](#preprocessing-preprocess) names, `from_value_with` and `from_value_piped`, `schema_examples()`, and the archie conversions.

### Pointers and Borrowed Values

`Box<T>`, `Rc<T>`, `Arc<T>` and `Cow<'_, T>` describe as `T`. serde writes each of them as the value it holds, with nothing of its own around it, so a field written under one is the field its inner type is — on every surface, and wherever the wrapper was written: `Box<Option<T>>` is the `Option<T>` field, optional; `Vec<Rc<T>>` is the `Vec<T>` field; `Box<str>`, `Rc<str>` and `Cow<'_, str>` are `String` fields, `str` being what a `String` writes.

This is what lets a type hold itself: `Option<Box<Self>>` is the self-reference, deferred in the generated schema the way any self-reference is. An `Rc` or `Arc` field needs serde's `rc` feature, which is where serde's impls for those two live.

`RefCell<T>`, `Cell<T>`, `Mutex<T>` and `RwLock<T>` describe as `T` on every surface the same way: serde writes each as the value it guards, returning a serialization error rather than panicking when the guard cannot be taken (an already-mutably-borrowed `RefCell`, a poisoned `Mutex`/`RwLock`) — the same fallible path a schema does not describe for any other type. What sets them apart from `Box`/`Rc`/`Arc`/`Cow` is `Deref`: none of the four implements it, so `#[model_schema_prop(pattern = ..., minLength = ..., maxLength = ..., minimum = ..., maximum = ...)]` cannot reach through one to the value it guards, and is refused at expansion, naming the wrapper, wherever it is combined with one.

```rust
use std::borrow::Cow;
use std::sync::Arc;

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct TreeNode {
    pub label: Cow<'static, str>,
    pub shared_tags: Arc<[String]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<Box<TreeNode>>,
}
```

### Ranges

`Range<T>`, `RangeInclusive<T>`, `RangeFrom<T>` and `RangeTo<T>` describe as the object serde writes for each: `Range` and `RangeInclusive` as `{"start": T, "end": T}`, `RangeFrom` as `{"start": T}` and `RangeTo` as `{"end": T}`. Every member is required, and an object that holds a key no range writes is refused, as serde refuses it. serde also reads the array `[8, 13]` as a `Range`; the surfaces describe the object it writes.

```rust
use core::ops::{Range, RangeFrom, RangeInclusive, RangeTo};

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Spanned {
    pub closed: RangeInclusive<u32>,
    pub from: RangeFrom<u32>,
    pub span: Range<u32>,
    pub to: RangeTo<u32>,
}
```

```typescript
export type Spanned = {
  closed: { start: number; end: number };
  from: { start: number };
  span: { start: number; end: number };
  to: { end: number };
};
```

```typescript
const Spanned$RawSchema = z.strictObject({
  closed: z.strictObject({ start: z.number().int(), end: z.number().int() }),
  from: z.strictObject({ start: z.number().int() }),
  span: z.strictObject({ start: z.number().int(), end: z.number().int() }),
  to: z.strictObject({ end: z.number().int() }),
});
```

The JSON Schema of each range is an object whose `properties` are its members, every one of them `required`, with `additionalProperties: false`. A bound is described as its type is anywhere else: a date, an `ObjectId`, a brand, a model, or one of the type's own parameters, `Range<Bound>` being `{ start: Bound; end: Bound }` and the factory's argument in Zod. A range is that object wherever it is held: in an `Option`, a list, a map's value, a tuple's position and a type argument.

Dart, Swift and Kotlin each read and write the object through a type of their own, written for the field that holds the range: Dart a record with named members, `({int start, int end})`, Swift a struct named after the type and the field, `SpannedSpanRange`, and Kotlin a `@Serializable` data class. Each refuses an object that holds a key the range does not write, as serde does. None of them is the language's own range: Swift's `Range` writes `[8,13]`.

`validate()` walks a range of a type that runs rules, a model or a brand with a bound, and reports each bound under its key: a `span: Range<Waymark>`, `Waymark` a brand under `minLength = 2`, holding `"a".."b"` reports `'span.start': too short: minimum length is 2, got 1` and `'span.end': too short: minimum length is 2, got 1`, in Rust, Dart, Swift and Kotlin. The read reads a model held in a range unjudged, so the whole value is judged once. The [recovering decode](#recovering-decode-decode_with) reads a range as one whole value at its key, and under `mongodb` a range field is a `Field` of that value.

A range is read by its last name. A `#[model_schema()]` type of the crate's own named `Range`, `RangeInclusive`, `RangeFrom` or `RangeTo`, declared above the type that holds it, keeps its name, and `core::ops::Range` or `std::ops::Range` written in full is the std range. A range written by its bare name earns a check beside the type that holds it, which fails the build with `E0308` where the name is another type. A range is refused as a map key, serde writing it as an object, and under `#[serde(flatten)]`, where serde writes its bounds among the members of the object that holds it. `pattern`, `minLength`, `maxLength`, `minimum` and `maximum` are refused on a range field: a bound on what a range holds goes on the type of its bounds, a branded newtype.

### Results

`Result<T, E>` describes as the object serde writes for it, the externally tagged form of an enum of two variants: `{"Ok": T}` or `{"Err": E}`. An arm of `()` is `null`, serde writing `Ok(())` as `{"Ok": null}`.

```rust
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Reply {
    pub outcome: Result<u32, String>,
    pub stored: Result<(), String>,
}
```

```typescript
export type Reply = {
  outcome: { Ok: number } | { Err: string };
  stored: { Ok: null } | { Err: string };
};
```

```typescript
const Reply$RawSchema = z.strictObject({
  outcome: z.union([z.strictObject({ Ok: z.number().int() }), z.strictObject({ Err: z.string() })]),
  stored: z.union([z.strictObject({ Ok: z.null() }), z.strictObject({ Err: z.string() })]),
});
```

The JSON Schema of a `Result` is a `oneOf` of the two objects, each with its one key `required` and `additionalProperties: false`. An arm is described as its type is anywhere else, and an arm of `()` as `{"type": "null"}`, as [`()`](#the-unit-) is wherever it is held. An object of both keys, of neither or of another key is refused, as serde refuses it. A `Result` is that object wherever it is held: in an `Option`, a list, a map's value, a tuple's position, a type argument, the arm of another `Result`, a brand, an alias and a variant's slot.

Dart, Swift and Kotlin each read and write the object through a type of their own, written for the field that holds the `Result`: Dart a sealed class, `DartResult{N}`, with an `Ok` and an `Err` subclass, Swift an enum of two cases named after the type and the field, `ReplyOutcomeResult`, and Kotlin a sealed interface, `KotlinResult{N}`, with a class per arm and a serializer of its own. `{N}` numbers the shapes an item writes, by which of the arms hold `()`. An arm of `()` is a case, a subclass or an object that holds no value. Each refuses an object of both keys, of neither or of another key, as serde does. None of them is named `Result`: Swift has its own, and Kotlin imports `kotlin.Result` into every file.

`validate()` walks the arm the value holds, and reports under the key serde writes the arm under: a `frontmatter: Option<Result<Masthead, Unparsed>>`, `Masthead` holding a `title` under `minLength = 1`, whose `Ok` holds an empty title reports `'frontmatter.Ok.title': too short: minimum length is 1, got 0`, in Rust, Dart, Swift and Kotlin. The read reads a model in either arm unjudged, so the whole value is judged once, and each message is listed once. The [recovering decode](#recovering-decode-decode_with) walks into the arm, and lists an issue inside it at its full path, `frontmatter.Ok.title`. An object of both keys, of neither or of another key is one issue at the field. Under `mongodb` a `Result` field is a `Field` of the whole value.

A `Result` is read by its last name with two type arguments: a `Result<T>` of one, an alias of the crate's own, is read as it was. A `#[model_schema()]` type of the crate's own named `Result`, declared above the type that holds it, keeps its name, and `core::result::Result` or `std::result::Result` written in full is the std `Result`. A `Result` written by its bare name earns a check beside the type that holds it, which fails the build with `E0308` where the name is another type. A `Result` is refused as a map key, serde writing it as an object, and under `#[serde(flatten)]`, where serde writes the key of its arm among the members of the object that holds it. `pattern`, `minLength`, `maxLength`, `minimum` and `maximum` are refused on a `Result` field: a bound on what an arm holds goes on the arm's own type. A brand over a `Result` is declared with `no_display`, a `Result` writing no text.

### The Unit `()`

`()` describes as the `null` serde writes for it, wherever it is held: a member, a list's item, a tuple's position, a type argument, an arm of a `Result`, and the slot of a [brand](#branded-newtypes) or of a struct of one slot. TypeScript writes `null`, Zod `z.null()` and the JSON Schema `{"type": "null"}`. serde refuses the empty array: `invalid type: sequence, expected unit`. A brand over `()` is `null & $brand<"Name">`, the type Zod's `.brand()` gives `z.null()`.

```rust
#[model_schema(no_display)]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct UnitBrand(());

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct UnitHolder {
    pub branded: UnitBrand,
    pub unit: (),
}
```

```typescript
export type UnitBrand = null & $brand<"UnitBrand">;

export type UnitHolder = {
  branded: UnitBrand;
  unit: null;
};
```

Flattened, serde writes `()` as no key, and beside the tag of a newtype variant under a `tag` with no `content` it writes the variant as the tag alone; it reads both back. The TypeScript type and the Zod schema intersect the object with `null` there, which admits no value, so the build stops where the `()` is written in place or reached through a type declared above, and `json_schema()` panics where it is reached through a type declared below. An `#[serde(untagged)]` enum declared above with a member over `()` stops the build too, flattened and beside a tag: serde writes that member as no key, or as the tag alone, and does not read it back as that member. Declared below, it makes `json_schema()` panic, flattened and beside a tag, and so does an alias, a [brand](#branded-newtypes) or a struct of one slot over it, written by its name: the enum's schema module records that it has such a member, and the merge reads the record. Flattened, a member the enum's own expansion does not see as `()` -- its own type parameter filled at `()`, or a type declared below the enum that serde writes as `()` -- is still read as no key. Write `()` under a key of its own, or write a unit variant. See [Intersection Types](#intersection-types-serdeflatten) and [Internally Tagged Enums](#internally-tagged-enums-tag-with-no-content).

### Enums

#### Plain Enums

An enum whose variants are all unit variants and which names no `tag` and no `untagged` generates a TypeScript string union and `z.enum()` in Zod, matching the bare variant name Serde writes for it. Either attribute changes what Serde writes, and the surfaces follow it rather than the string union. Naming a `tag` moves the variant name under the tag key, into an object, so the same all-unit variants are described as the tagged objects below. Naming `untagged` drops the name altogether -- Serde writes every unit variant as a bare `null` -- and the declaration is refused per variant, the way any unit variant in an untagged enum is refused under "Untagged Enums" below.

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserStatus {
    Active,
    Inactive,
    Pending,
    Suspended,
}
```

#### Discriminated Unions (Tagged Enums)

Enums with `#[serde(tag = "...")]` generate TypeScript discriminated unions.

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum PaymentMethod {
    CreditCard {
        card_number: String,
        expiry_date: String,
        cvv: String,
    },
    BankTransfer {
        account_number: String,
        routing_number: String,
    },
    PayPal {
        email: String,
    },
}
```

#### Tuple Variants

Discriminated unions also support tuple variants. Single-element tuples are flattened to a `value` field, and multi-element tuples generate a TypeScript tuple type.

A tuple variant with no field, `Empty()`, is refused on every enum: serde writes it with an empty `[]` payload, which no surface describes. Write `Empty` for a unit variant, or give it a field:

```text
model_schema: variant `Empty`: `Empty()` is a tuple variant with no field, which serde writes with an empty `[]` payload, while every surface describes it as the unit variant `Empty`, so a value of it never crosses. Write `Empty` for a unit variant, or give it a field.
```

```rust
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "value")]
pub enum FixedValue {
    // Unit variant (no data)
    Empty,
    // Single-element tuple: { type: "Text", value: string }
    Text(String),
    // Multi-element tuple: { type: "Pair", value: [string, number] }
    Pair(String, i64),
    // Named struct variant (existing behavior)
    Named { field_a: String, field_b: bool },
}
```

Generated TypeScript:

```typescript
export type FixedValue = {
  type: "Empty";
} | {
  type: "Text";
  value: string;
} | {
  type: "Pair";
  value: [string, number];
} | {
  type: "Named";
  field_a: string;
  field_b: boolean;
};
```

Generated Zod:

```typescript
export const FixedValue$Schema: ZodType<FixedValue> = z.discriminatedUnion("type", [
  z.strictObject({
    type: z.literal("Empty"),
  }),
  z.strictObject({
    type: z.literal("Text"),
    value: z.string(),
  }),
  z.strictObject({
    type: z.literal("Pair"),
    value: z.tuple([z.string(), z.number().int()]),
  }),
  z.strictObject({
    type: z.literal("Named"),
    field_a: z.string(),
    field_b: z.boolean(),
  }),
]).meta({
  examples: []
});
```

You can customize the tag and content field names using Serde's `tag` and `content` attributes:

```rust
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "kind", content = "data")]
pub enum Event {
    Text(String),
    Number(i64),
}
```

This generates `kind` as the discriminator and `data` as the value field instead of the defaults (`type` and `value`).

#### Externally Tagged Enums (No Tagging Attributes)

An enum with data-carrying variants that names neither `tag` nor `content` is externally tagged, which is Serde's default: the variant name is the sole key of an object holding the content, and a unit variant is that name as a bare string. The generated surfaces describe that form -- a JSON Schema `oneOf`, a TypeScript union, and a Zod `z.union([...])`. There is no field every member shares, so the Zod schema is a plain union rather than a `z.discriminatedUnion`.

```rust
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum External {
    Bare,
    Fields { a: String, b: bool },
    Pair(u32, u32),
    Single(String),
}
```

Serialized by Serde:

```json
"Bare"
{ "Fields": { "a": "a", "b": true } }
{ "Pair": [1, 2] }
{ "Single": "a" }
```

Generated TypeScript:

```typescript
export type External = "Bare" | {
  "Fields": {
    a: string;
    b: boolean;
  };
} | {
  "Pair": [number, number];
} | {
  "Single": string;
};
```

Generated Zod:

```typescript
export const External$Schema: ZodType<External> = z.union([
  z.literal("Bare"),
  z.strictObject({
    "Fields": z.strictObject({ a: z.string(), b: z.boolean() }),
  }),
  z.strictObject({
    "Pair": z.tuple([z.number().int(), z.number().int()]),
  }),
  z.strictObject({
    "Single": z.string(),
  }),
]).meta({
  examples: []
});
```

Add `#[serde(tag = "...", content = "...")]` to get the adjacently tagged `{ type, value }` form documented above instead.

#### Internally Tagged Enums (`tag` With No `content`)

An enum that names `tag` but no `content` is internally tagged: there is no key for a variant's data, so Serde writes it as members of the object the tag is written in. A struct variant's fields sit beside the tag, and so do the members of a newtype variant's inner type -- which the surfaces describe as an intersection, the same composition `#[serde(flatten)]` uses.

A variant carrying nothing is the tag alone, `{"type":"Bare"}`, and an enum whose variants are *all* unit variants is written that way throughout -- one object per variant, each holding only the tag. That is the shape of a service's error code, and it is described as the tagged object union it is written as, never as the bare string union the same variants would publish with no `tag` named.

```rust
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TagPayload {
    pub a: String,
    pub b: bool,
}

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum Internal {
    Bare,
    Fields { a: String, b: bool },
    Wrapped(TagPayload),
}
```

Serialized by Serde:

```json
{ "type": "Bare" }
{ "type": "Fields", "a": "a", "b": true }
{ "type": "Wrapped", "a": "a", "b": true }
```

Generated TypeScript:

```typescript
export type Internal = {
  type: "Bare";
} | {
  type: "Fields";
  a: string;
  b: boolean;
} | {
  type: "Wrapped";
} & TagPayload;
```

Generated Zod -- a flattened member is an intersection, which has no shape of its own to read the discriminator out of, so a union holding one is a plain `z.union` rather than a `z.discriminatedUnion`:

```typescript
export const Internal$Schema: ZodType<Internal> = z.union([
  z.strictObject({ type: z.literal("Bare") }),
  z.strictObject({ type: z.literal("Fields"), a: z.string(), b: z.boolean() }),
  z.strictObject({ type: z.literal("Wrapped") }).and(z.lazy(() => TagPayload$Schema)),
]).meta({
  examples: []
});
```

The content joins the member under `z.lazy` for the reason a flattened base does: the inner type is a `const` of its own and nothing orders one type's schema against another's, so reading the name while the union's `const` initializes would fail for any inner type declared below -- and a cycle running through a variant's content and a `#[serde(flatten)]` base has no order that puts each above the other. Deferred, every module loads whatever order it is assembled in.

Only a value Serde writes as an object has members to put beside the tag. A newtype variant wrapping a string, a number, a boolean, a sequence, an `Option` or a tuple of one element or more is one Serde refuses to serialize at run time (`cannot serialize tagged newtype variant ... containing a string`), and a multi-element tuple variant is one Serde's own derive refuses outright. A newtype variant wrapping [`()`](#the-unit-) Serde writes as the tag alone and reads back, and the TypeScript type and the Zod schema, which intersect the tag's object with `null`, admit no value of it. `#[model_schema()]` rejects all of those at expansion rather than describing a value no surface reads as Serde does; name a `content` key so the value gets an object of its own, or wrap it in a struct whose fields can sit beside the tag. `Held(())` answers:

```text
error: model_schema: variant `Held` of internally tagged enum `ProbeUnitTagged` wraps `()`, which serde writes as `null`: beside the tag serde writes the variant as the tag alone, and reads it back, while the TypeScript type and the Zod schema of the variant intersect the tag's object with `null`, which admits no value. Name a `content` key so the value gets an object of its own, or write `Held` as a unit variant, which serde writes as the tag alone as well.
```

A name is not a promise of an object either. A newtype variant wrapping a plain `#[model_schema()]` enum is rejected at expansion the same way: a plain enum writes its own variant name, which Serde puts beside the tag as a key holding null and which a schema closed around the tag rejects. So is a newtype variant wrapping any other type declared above the enum that Serde writes as no object, in every build with `serde`: a [brand](#branded-newtypes) or a struct of one slot over text, a number, a `bool` or a list, any other tuple struct, an alias of one, and an `#[serde(untagged)]` enum, or an alias of one, with a member Serde writes as no object, refused with that member's branch. A member Serde writes as a variant's name, a plain enum's or an externally tagged enum's unit variant's, is refused with its branch in the words of a plain enum: Serde writes that name beside the tag as a key holding `null` (`{"kind":"Held","Calm":null}`), and reads it back. Serde refuses every write and every read of a brand over text and of a tuple struct there (`cannot serialize tagged newtype variant ProbeReading::Held containing a string`). A brand over a struct, and an untagged enum whose every member is an object, are not refused. With `#[serde(transparent)] pub struct ProbeId(String);` declared above it, `#[serde(tag = "kind")] pub enum ProbeReading { Held(ProbeId) }` answers:

```text
error: model_schema: variant `Held` of internally tagged enum `ProbeReading` wraps `ProbeId`, which is not written as an object — its schema describes a `string`, which cannot sit beside the tag: `#[serde(tag = "kind")]` with no `content` writes the variant's data as members of the object the tag is written in, and only a value serde writes as an object has members to put there. Name a `content` key so the value gets an object of its own, or wrap it in a struct whose fields can sit beside the tag. This refusal holds in every build with serde, and only a type declared above reaches it: one declared below, or in another crate, is not refused here, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.
```

A value Serde writes as a plain enum's variant name by way of a type declared above the enum is refused with the error of the plain enum itself, in every build with `serde`, and named as written: a [brand](#branded-newtypes) or a struct of one slot over the plain enum, a generic type declared above whose value is its own type parameter, filled at the plain enum, and a type whose value is one of those. Serde writes each beside the tag as the key naming the variant, holding `null` (`{"kind":"Held","Calm":null}`), and reads it back. One that writes an `Option` of the plain enum on the way (`pub struct ProbeMaybeMoodSlot(Option<ProbeMood>);`, `ProbeWrap<Option<ProbeMood>>`) is refused with the error of an `Option`, below: Serde refuses every write of it there. With `pub enum ProbeMood { Calm, Tense }` and `#[model_schema(no_display)] pub struct ProbeMoodSlot(ProbeMood);` declared above it, `#[serde(tag = "kind")] pub enum ProbeM2 { Held(ProbeMoodSlot) }` answers:

```text
error: model_schema: variant `Held` of internally tagged enum `ProbeM2` wraps `ProbeMoodSlot`, which serde does not write as an object: a plain enum writes its own variant name, so nothing of it joins the tag — serde writes that name as a key holding null, which a schema closed around the tag rejects. Name a `content` key so the value gets an object of its own, or wrap it in a struct whose fields can sit beside the tag. This refusal holds in every build with serde, and only a plain enum declared above reaches it: one declared below, or in another crate, is not refused here, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.
```

and, with `pub struct ProbeWrap<T>(T);` declared above it, `#[serde(tag = "kind")] pub enum ProbeM4 { Held(ProbeWrap<ProbeMood>) }` answers:

```text
error: model_schema: variant `Held` of internally tagged enum `ProbeM4` wraps `ProbeWrap<ProbeMood>`, which serde does not write as an object: a plain enum writes its own variant name, so nothing of it joins the tag — serde writes that name as a key holding null, which a schema closed around the tag rejects. Name a `content` key so the value gets an object of its own, or wrap it in a struct whose fields can sit beside the tag. This refusal holds in every build with serde, and only a plain enum declared above reaches it: one declared below, or in another crate, is not refused here, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.
```

An `Option` behind a name is refused the same way, in every build with `serde`: a newtype variant wrapping a struct of one slot or a [brand](#branded-newtypes) over an `Option`, or an alias of one, declared above the enum. Serde refuses every write of it, of `Some` and of `None` alike (`cannot serialize tagged newtype variant ProbeOptional::Held containing an optional`), though it reads `{"kind":"Held","note":"n"}` back. The same type flattened into a struct, or into a variant's members, is not refused: Serde writes a flattened `None` as no key, and reads it back. An `Option` of a value Serde writes as no object, `pub struct ProbeMaybeCountSlot(Option<u32>);` or an `Option` of an untagged enum with a member written as text, is refused beside the tag with the error of an `Option` too: Serde's tag refuses the `Option` before it reads what it holds. Flattened, it is refused with the branch Serde writes as no object. A type declared below the enum, or in another crate, makes `json_schema()` panic: the merge reads a `null` among the choices of the content as an `Option`'s or as `()`'s, and refuses it ahead of every other choice. With `pub struct ProbeFields { pub note: String }` and `pub struct ProbeMaybe(Option<ProbeFields>);` declared above it, `#[serde(tag = "kind")] pub enum ProbeOptional { Held(ProbeMaybe) }` answers:

```text
error: model_schema: variant `Held` of internally tagged enum `ProbeOptional` wraps `ProbeMaybe`, which serde writes as an optional, which cannot sit beside the tag: `#[serde(tag = "kind")]` with no `content` writes the variant's data as members of the object the tag is written in, and only a value serde writes as an object has members to put there. serde refuses this one at run time — `cannot serialize tagged newtype variant ProbeOptional::Held containing an optional`. Name a `content` key so the value gets an object of its own, or wrap it in a struct whose fields can sit beside the tag. This refusal holds in every build with serde, and only a type declared above reaches it: one declared below, or in another crate, is not refused here, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.
```

A generic type declared above the enum whose value is its own type parameter -- a [brand](#branded-newtypes), a struct of one slot or an alias over the parameter, under `Box` or through another such type -- is read at what fills that parameter, in every build with `serde`. Filled at text, a number, a `bool`, a list, or a type declared above that Serde writes as no object, it is refused with the error of a value written as no object; filled at an `Option`, or at a type declared above that Serde writes as one, it is refused with the error of an `Option`. Serde refuses every write of one filled at text, a number, a `bool`, a list or an `Option` (`cannot serialize tagged newtype variant ProbeFilledText::Held containing a string`), and every read of one filled at text. Filled at a struct, at a type declared below, or at the enum's own type parameter, it builds. With `pub struct ProbeWrap<T>(T);` declared above it, `#[serde(tag = "kind")] pub enum ProbeFilledText { Held(ProbeWrap<String>) }` answers:

```text
error: model_schema: variant `Held` of internally tagged enum `ProbeFilledText` wraps `ProbeWrap<String>`, which is not written as an object — its schema describes a `string`, which cannot sit beside the tag: `#[serde(tag = "kind")]` with no `content` writes the variant's data as members of the object the tag is written in, and only a value serde writes as an object has members to put there. Name a `content` key so the value gets an object of its own, or wrap it in a struct whose fields can sit beside the tag. This refusal holds in every build with serde, and only a type declared above reaches it: one declared below, or in another crate, is not refused here, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.
```

and `#[serde(tag = "kind")] pub enum ProbeFilledOptional { Held(ProbeWrap<Option<ProbeFields>>) }` answers:

```text
error: model_schema: variant `Held` of internally tagged enum `ProbeFilledOptional` wraps `ProbeWrap<Option<ProbeFields>>`, which serde writes as an optional, which cannot sit beside the tag: `#[serde(tag = "kind")]` with no `content` writes the variant's data as members of the object the tag is written in, and only a value serde writes as an object has members to put there. serde refuses this one at run time — `cannot serialize tagged newtype variant ProbeFilledOptional::Held containing an optional`. Name a `content` key so the value gets an object of its own, or wrap it in a struct whose fields can sit beside the tag. This refusal holds in every build with serde, and only a type declared above reaches it: one declared below, or in another crate, is not refused here, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.
```

A type declared above the enum whose value is such a generic type, filled -- a struct of one slot, a [brand](#branded-newtypes) or an alias over it -- is read at that filling as well, in every build with `serde`. `pub struct ProbeTextFilled(ProbeWrap<String>);` is written as text and refused with the error of a value written as no object, and `pub struct ProbeMaybeFilled(ProbeWrap<Option<ProbeFields>>);` is written as an `Option` and refused with the error of an `Option`, as the type over the filling itself is. Serde refuses every write and every read of the first there (`cannot serialize tagged newtype variant ProbeHeldTextFilled::Held containing a string`), and every write of the second. Filled at a struct, or at a type declared below, it builds. With `pub struct ProbeTextFilled(ProbeWrap<String>);` declared above it, `#[serde(tag = "kind")] pub enum ProbeHeldTextFilled { Held(ProbeTextFilled) }` answers:

```text
error: model_schema: variant `Held` of internally tagged enum `ProbeHeldTextFilled` wraps `ProbeTextFilled`, which is not written as an object — its schema describes a `string`, which cannot sit beside the tag: `#[serde(tag = "kind")]` with no `content` writes the variant's data as members of the object the tag is written in, and only a value serde writes as an object has members to put there. Name a `content` key so the value gets an object of its own, or wrap it in a struct whose fields can sit beside the tag. This refusal holds in every build with serde, and only a type declared above reaches it: one declared below, or in another crate, is not refused here, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.
```

An `#[serde(untagged)]` enum declared above whose member names such a generic type, filled, is read at that filling as well, in every build with `serde`: `Text(ProbeWrap<String>)` is a member written as text, as `Text(String)` is, and the enum is refused with that member's branch. Serde refuses every write of that member there (`cannot serialize tagged newtype variant ProbeHeldWrapMember::Held containing a string`). Filled at a struct, or at a type declared below, the member builds. With `#[serde(untagged)] pub enum ProbeWrapMember { Obj { a: String }, Text(ProbeWrap<String>) }` declared above it, `#[serde(tag = "kind")] pub enum ProbeHeldWrapMember { Held(ProbeWrapMember) }` answers:

```text
error: model_schema: variant `Held` of internally tagged enum `ProbeHeldWrapMember` wraps `ProbeWrapMember`, which writes a union member that is not an object — its branch 2 describes a `string`, which cannot sit beside the tag: `#[serde(tag = "kind")]` with no `content` writes the variant's data as members of the object the tag is written in, and only a value serde writes as an object has members to put there. Name a `content` key so the value gets an object of its own, or wrap it in a struct whose fields can sit beside the tag. This refusal holds in every build with serde, and only a type declared above reaches it: one declared below, or in another crate, is not refused here, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.
```

An `#[serde(untagged)]` enum declared above whose member is its own type parameter -- itself, under `Box` or an `Option`, in such a generic type, or handed on to another such enum -- is read at what fills that parameter, in every build with `serde`, and so is an alias of it, filled. Held as `ProbeOwnMember<String>`, the member `Held(X)` is written as text, as `Held(String)` is, and the enum is refused with that member's branch, named with its arguments. Serde refuses every write of that member there (`cannot serialize tagged newtype variant ProbeHeldOwnMember::Held containing a string`). Filled at a struct, at the holder's own type parameter, or declared below, it builds. With `#[serde(untagged)] pub enum ProbeOwnMember<X> { Obj { a: String }, Held(X) }` declared above it, `#[serde(tag = "kind")] pub enum ProbeHeldOwnMember { Held(ProbeOwnMember<String>) }` answers:

```text
error: model_schema: variant `Held` of internally tagged enum `ProbeHeldOwnMember` wraps `ProbeOwnMember<String>`, which writes a union member that is not an object — its branch 2 describes a `string`, which cannot sit beside the tag: `#[serde(tag = "kind")]` with no `content` writes the variant's data as members of the object the tag is written in, and only a value serde writes as an object has members to put there. Name a `content` key so the value gets an object of its own, or wrap it in a struct whose fields can sit beside the tag. This refusal holds in every build with serde, and only a type declared above reaches it: one declared below, or in another crate, is not refused here, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.
```

A struct of one slot or a [brand](#branded-newtypes) declared above whose value is an `#[serde(untagged)]` enum with a member Serde writes as no object is read as that enum, in every build with `serde`, as an alias of it is: over the enum itself, through such a generic type (`ProbeWrap<ProbeTextMember>`), or over the enum filled at its own parameter (`ProbeOwnMember<String>`), it is refused with that member's branch, and named as written. Serde refuses every write of that member there (`cannot serialize tagged newtype variant ProbeHeldSlotText::Held containing a string`). Over an enum whose every member is an object, or one declared below, it builds. With `#[serde(untagged)] pub enum ProbeTextMember { Obj { a: String }, Text(String) }` and `#[model_schema(no_display)] pub struct ProbeSlotText(ProbeTextMember);` declared above it, `#[serde(tag = "kind")] pub enum ProbeHeldSlotText { Held(ProbeSlotText) }` answers:

```text
error: model_schema: variant `Held` of internally tagged enum `ProbeHeldSlotText` wraps `ProbeSlotText`, which writes a union member that is not an object — its branch 2 describes a `string`, which cannot sit beside the tag: `#[serde(tag = "kind")]` with no `content` writes the variant's data as members of the object the tag is written in, and only a value serde writes as an object has members to put there. Name a `content` key so the value gets an object of its own, or wrap it in a struct whose fields can sit beside the tag. This refusal holds in every build with serde, and only a type declared above reaches it: one declared below, or in another crate, is not refused here, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.
```

A generic alias, struct of one slot or [brand](#branded-newtypes) declared above whose value is an `#[serde(untagged)]` enum whose member is its own type parameter, filled at the holder's own type parameter (`pub type ProbeOwnAliasOf<T> = ProbeOwnMember<T>;`), is read at what fills the holder, in every build with `serde`, and so are an alias of one, a generic type filled at one, and an untagged enum with a member over one: held as `ProbeOwnAliasOf<String>`, it is refused as `ProbeOwnMember<String>` is, with that member's branch, and named with its arguments. Serde refuses every write of that member there (`cannot serialize tagged newtype variant ProbeHeldOwnAliasOf::Held containing a string`). Filled at a struct, at the holder's own type parameter, or over an enum declared below, it builds. With `ProbeOwnMember<X>` and `#[model_schema(default_types(T = String))] pub type ProbeOwnAliasOf<T> = ProbeOwnMember<T>;` declared above it, `#[serde(tag = "kind")] pub enum ProbeHeldOwnAliasOf { Held(ProbeOwnAliasOf<String>) }` answers:

```text
error: model_schema: variant `Held` of internally tagged enum `ProbeHeldOwnAliasOf` wraps `ProbeOwnAliasOf<String>`, which writes a union member that is not an object — its branch 2 describes a `string`, which cannot sit beside the tag: `#[serde(tag = "kind")]` with no `content` writes the variant's data as members of the object the tag is written in, and only a value serde writes as an object has members to put there. Name a `content` key so the value gets an object of its own, or wrap it in a struct whose fields can sit beside the tag. This refusal holds in every build with serde, and only a type declared above reaches it: one declared below, or in another crate, is not refused here, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.
```

A type declared below the enum, or in another crate, is not looked into: the expansion of the enum cannot see what it writes, so one that turns out not to be written as an object is caught when `json_schema()` runs, with a diagnostic naming the variant, the inner type and the same remedy. The TypeScript and Zod intersections for that case are a known gap: they are written, and only the JSON Schema surface refuses them. One that writes an `Option` is refused by no surface: the JSON Schema reads its `None` as the tag alone, `{"kind":"Held"}`.

A named type Serde writes as a map has no members to name either. A newtype variant whose value writes a map stops the build, in every build with `serde`: Serde writes the tag and the map's entries at one level, as it does for [a map flattened beside a tag](#intersection-types-serdeflatten), and no TypeScript type describes both. `{ kind: "Counted"; } & Partial<Record<string, number>>` refuses `{"kind":"Counted","north":2}`, which Serde reads. A value writes a map where its type, through `Box`, is declared above the enum as a struct whose one member is a flattened map, a [brand](#branded-newtypes) or a struct of one slot over a map, an alias of a map, or an `#[serde(untagged)]` enum with a variant that writes one, or is a generic type of those shapes filled at a map. A variant that holds the enum's own type parameter, `Counted(T)` or `Counted(Wrap<T>)`, builds, and the build stops where a map fills that parameter, naming the tag. Name a `content` key so the map gets an object of its own, or write the map under a key of its own. The same value under an enum written under its variant's name, under a `content` key or untagged writes no tag beside the map, and builds. A type declared below the enum, or in another crate, is not looked into: the expansion of the enum cannot see what it writes. With `pub struct Tally { #[serde(flatten)] counts: BTreeMap<String, u8> }` declared above it, `#[serde(tag = "kind")] pub enum Filled { Counted(Tally) }` answers:

```text
error: model_schema: type `Filled`: variant `Counted`, field `0`: `Tally` writes a map, which cannot sit beside the tag `kind` — serde writes the tag and the map's entries at one level, and no TypeScript type describes both. Name a `content` key so the map gets an object of its own, or write the map under a key of its own.
```

A variant under `#[serde(skip)]` is refused none of these: Serde neither writes nor reads it, so nothing it holds sits beside the tag, and `#[serde(tag = "kind")] pub enum Skipped { Open { note: String }, #[serde(skip)] Hidden(String) }` builds. Serde writes `Skipped::Open` as `{"kind":"Open","note":"a"}` and reads it back, and answers `unknown variant` for `{"kind":"Hidden"}`. A tuple variant of several slots is refused all the same, as Serde's derive refuses it skipped or not. A variant under `skip_serializing` alone, which Serde still reads beside the tag, or under `skip_deserializing` alone, which it still writes there, is refused as any other variant. The surfaces still describe a variant under `skip`, with what it holds, as they describe every variant.

#### A Check Written on the Enum

`minLength`, `maxLength` and `pattern` written in an enum's own `#[model_schema(...)]`, each alone or together, are the check of every string the enum carries. A free-text arm is held to a length with no attribute on the arm and no brand declared for it:

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub enum KnownKind {
    #[serde(rename = "application/pdf")]
    Pdf,
}

#[model_schema(minLength = 1, maxLength = 20)]
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContentKind {
    Known(KnownKind),
    Other(String),
}

ContentKind::Other(String::new()).validate();
// Err(["too short: minimum length is 1, got 0"])
ContentKind::Other("a".repeat(21)).validate();
// Err(["too long: maximum length is 20, got 21"])
```

`"text/csv"` reads as `Other` and `"application/pdf"` as `Known`, while `""` and a value of 21 characters match no variant. The Zod schema is `z.union([KnownKind$Schema, z.string().min(1, ...).max(20, ...)])`, the second member of the JSON Schema's `anyOf` is `{ "type": "string", "minLength": 1, "maxLength": 20 }`, and the TypeScript type is still `KnownKind | string`. The wire does not change.

The enum's check is the same check written in `#[model_schema_prop(...)]` on each field it reaches, and it is enforced exactly where that field's own check is, on every enum Serde writes: plain, externally, internally and adjacently tagged, and `#[serde(untagged)]`.

- **Reached:** a field of any variant, a named member or a slot, written as a string or a path itself -- `String`, `&str`, `PathBuf` -- under `Option`, the transparent wrappers `Box`, `Rc`, `Arc` and `Cow`, and any reference.
- **Not reached:** the items of a list, so a field written `Vec<String>` is held to nothing; a field typed with a named type, a brand included, which carries its own check; a number, a `bool`, a map and a tuple; a field typed with one of the enum's own type parameters; and a field that carries a [`literal`](#literal-values), which takes no other rule.
- **A member of an untagged enum** is checked by the read, which is what chooses the variant, and by `validate()`, the Zod schema and the JSON Schema.
- **A member of a tagged enum** is checked by `validate()`, which the enum's own read runs on the value it read, and by the Zod schema, the JSON Schema and the [recovering decode](#recovering-decode-decode_with). See [Where a constraint is checked](#where-a-constraint-is-checked).

`validate()` reports a field at [the path serde writes it at](#the-path-a-message-names), as it does for [a check written on the member](#a-slot-of-a-tuple-variant): below the variant's name or the content key where serde writes one, and under no name for an untagged newtype variant's slot. A validator is published only where a schema surface is on (`typescript`, `zod` or `jsonschema`), so in a build with none the enum's check holds a field to nothing, as the field's own does there.

A field that writes a check of its own under the enum's is held to both. `minLength` is the larger of the two and `maxLength` the smaller. Two patterns are both checked: the Zod schema writes two `.check(z.regex(...))`, the JSON Schema states both in an `allOf`, and `validate()` reports each one the value fails.

```rust
#[model_schema(minLength = 1, maxLength = 5, pattern = "^[a-z]+$")]
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Tag {
    Plain { word: String },
    Prefixed {
        #[model_schema_prop(minLength = 3, pattern = "^ab")]
        word: String,
    },
}

Tag::Plain { word: "abcdef".to_string() }.validate();
// Err(["'word': too long: maximum length is 5, got 6"])
Tag::Prefixed { word: "XY".to_string() }.validate();
// Err(["'word': too short: minimum length is 3, got 2", "'word': does not match pattern '^ab'", "'word': does not match pattern '^[a-z]+$'"])
```

The JSON Schema describes `word` of `Prefixed` as `{ "type": "string", "minLength": 3, "maxLength": 5, "allOf": [{ "pattern": "^ab" }, { "pattern": "^[a-z]+$" }] }`.

**A unit variant's name is checked at expansion.** Each name Serde writes or reads for a variant written with no data -- its name after `rename` and `rename_all`, in each direction where the two differ, and each `alias` -- is measured as a field's value is. A name that breaks the check is a compile error on the variant, in every feature configuration, naming the variant, the name and the check:

```rust
#[model_schema(minLength = 2, maxLength = 3)]
#[derive(Serialize, Deserialize)]
pub enum Size {
    S,
    Xl,
    Xxxxl,
}
```

```text
error: model_schema: type `Size`: variant `S` is written `S`, which breaks `minLength = 2` written on the enum
error: model_schema: type `Size`: variant `Xxxxl` is written `Xxxxl`, which breaks `maxLength = 3` written on the enum
```

The name of a variant that carries data is not checked, and an enum whose names all fit publishes exactly what it publishes with no check written.

**A check that reaches nothing is refused.** An enum with no unit variant and no field written as a string or a path that carries no `literal` leaves the check nothing to hold, so `#[model_schema(minLength = 1)]` on `enum Shape { Circle { r: u32 } }` is a compile error rather than a check dropped in silence, and so is the same check on `enum Shape { Fixed { #[model_schema_prop(literal = "x")] word: String } }`:

```text
error: model_schema: type `Shape`: `minLength` reaches nothing here: an enum's check holds each variant written with no data and each field written as a string or a path that carries no `literal`, and `Shape` has neither
```

### Intersection Types (`#[serde(flatten)]`)

A struct field marked `#[serde(flatten)]` is lifted into the parent type as a TypeScript intersection (`A & B`) and a Zod `.and()` chain, instead of a nested object. This is the idiomatic way to compose a common set of fields with a discriminated union.

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataElementSampleValueEntry {
    pub data_element_id: String,
    #[serde(flatten)]
    pub variant: DataElementSampleValueVariant,
}

#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(tag = "dataType")]
pub enum DataElementSampleValueVariant {
    Alphanumeric {
        #[serde(rename = "sampleValues")]
        sample_values: Vec<String>,
    },
    Numeric {
        #[serde(rename = "sampleValues")]
        sample_values: Vec<i64>,
    },
}
```

Generated TypeScript:

```typescript
export type DataElementSampleValueEntry = {
  dataElementId: string;
} & DataElementSampleValueVariant;
```

Generated Zod:

```typescript
export const DataElementSampleValueEntry$Schema: ZodType<DataElementSampleValueEntry> =
  z.strictObject({
    dataElementId: z.string(),
  }).and(z.lazy(() => DataElementSampleValueVariant$Schema)).meta({
    examples: []
  });
```

Notes:

- The flattened base joins the intersection under `z.lazy`. A base is a `const` of its own and nothing orders one type's schema against another's, so reading the name while the intersection's `const` initializes would fail for any base declared below -- and for two bases that flatten each other, no order puts each above the other. Deferred, every module loads whatever order it is assembled in, and a pair that does flatten each other fails when asked to validate rather than at import.
- Multiple `#[serde(flatten)]` fields chain: `{ ... } & BasePart & ExtraPart` in TypeScript and `z.strictObject({ ... }).and(z.lazy(() => BasePart$Schema)).and(z.lazy(() => ExtraPart$Schema))` in Zod.
- A struct whose only field is flattened becomes a plain alias (e.g. `export type FlattenOnly = DataElementSampleValueVariant;`).
- **A map is flattened alone.** A `#[serde(flatten)]` member that writes a map, beside any other member that writes a key, stops the build, on a struct and on a variant of an enum under every tagging, in every build with `serde`: serde writes the map's entries at one level with that member's key, and no TypeScript type describes both. `{ note: string } & Partial<Record<string, number>>` refuses `{ note: "a", north: 2 }`, which serde reads. A map is written under a key of its own instead. A member that writes a map is a map, an `Option` of one and a `Box` of one, and a type declared above the holder that serde writes as one: a struct whose one member is a flattened map, a [brand](#branded-newtypes) or a struct of one slot over a map, an alias of a map, and an `#[serde(untagged)]` enum with a variant that writes one. A generic type of those shapes over its own type parameter writes a map where a map fills that parameter. A member under `skip` writes no key and stands beside no map, and a variant under `#[serde(skip)]`, which serde neither writes nor reads, holds no map beside anything. A type declared below the holder, or in another crate, is not looked into: the expansion of the holder cannot see what it writes. `pub struct Stocktake { #[serde(flatten)] pub counts: BTreeMap<String, u8>, pub note: String }` answers:

  ```text
  error: model_schema: type `Stocktake`: field `counts`: `#[serde(flatten)]` cannot hold a map beside other members — serde writes the map's entries at one level with `note`, and no TypeScript type describes both. Write the map under a key of its own.
  ```

- **A map is not flattened beside a tag.** A `#[serde(flatten)]` member that writes a map, alone in a struct with its own `#[serde(tag = "...")]` or in a variant of an enum under a `tag` with no `content`, stops the build as well, in every build with `serde`: serde writes the tag and the map's entries at one level, and no TypeScript type describes both. `{ kind: "Open"; } & Partial<Record<string, number>>` refuses `{"kind":"Open","north":2}`, which serde reads. A struct's own tag is worse off still: serde hands it back to the map on a read, so it cannot read back a map of numbers it wrote, and it writes the tag twice for a map of text. A map is written under a key of its own instead, and in a variant a `content` key gives the map an object of its own. A variant written under its name, or untagged, writes no tag beside the map, and builds. A newtype variant under a tag alone whose value writes a map is refused the same way: see [Internally Tagged Enums](#internally-tagged-enums-tag-with-no-content). `#[serde(tag = "kind")] pub enum Keelson { Open { #[serde(flatten)] counts: BTreeMap<String, u8> } }` answers:

  ```text
  error: model_schema: type `Keelson`: variant `Open`, field `counts`: `#[serde(flatten)]` cannot hold a map beside the tag `kind` — serde writes the tag and the map's entries at one level, and no TypeScript type describes both. Write the map under a key of its own.
  ```

- **A map that reaches the member another way is refused as well**, in every build with `serde`. Each of these writes the map's entries at one level with `id`, and both TypeScript and Zod refuse what serde reads:
  - An untagged enum with a map among its variants, `#[serde(untagged)] pub enum Scores { Captioned(Captioned), Tallied(HashMap<String, i32>) }`, flattened in `pub struct Scoreboard { pub id: String, #[serde(flatten)] pub scores: Scores }`. An enum written under its variant's name or under a `content` key writes the map under a key of its own, and builds:

    ```text
    error: model_schema: type `Scoreboard`: field `scores`: `#[serde(flatten)]` cannot hold a map beside other members — serde writes the map's entries at one level with `id`, and no TypeScript type describes both. Write the map under a key of its own.
    ```

  - A generic type serde writes as its own argument, filled at a map: a brand `#[serde(transparent)] pub struct Wrap<T>(T);` in `pub struct Crate { #[serde(flatten)] pub body: Wrap<HashMap<String, i32>>, pub id: String }`, and in the same way an alias `type Held<T> = T;`, a struct whose one member is its flattened parameter and an untagged enum whose variant holds it. `Wrap<HashMap<String, i32>>` flattened alone builds:

    ```text
    error: model_schema: type `Crate`: field `body`: `#[serde(flatten)]` cannot hold a map beside other members — serde writes the map's entries at one level with `id`, and no TypeScript type describes both. Write the map under a key of its own.
    ```

  - A type parameter flattened beside a member, `pub struct Letter<T> { #[serde(flatten)] pub body: T, pub id: String }`, where a map fills it: `Letter<T>` builds, and so does every filling that is no map, `Letter<Plain>` among them. The build stops where a map fills `T`, at any depth of a member (`Vec<Letter<HashMap<String, i32>>>`), in an alias (`type Mailbag = Letter<HashMap<String, i32>>;`), in a `default_types` entry of `Letter` itself, and in a type that hands its own parameter on (`pub struct Bag<U> { pub letter: Letter<U> }`, filled as `Bag<HashMap<String, i32>>`). A parameter flattened alone beside a tag is refused the same way, naming the tag, and so is one the lone slot of a variant under a tag alone writes. The error is spanned on the map. `pub struct Post { pub letter: Letter<HashMap<String, i32>> }` answers:

    ```text
    error: model_schema: type `Post`: field `letter`: `Letter` flattens its type parameter `T` beside `id`, and `HashMap<String, i32>` fills it with a map — serde writes the map's entries at one level with `id`, and no TypeScript type describes both. Write the map under a key of its own.
    ```

  A generic type declared below the place that fills it, or in another crate, is not looked into: the expansion that fills it cannot see what it flattens.

- **A map keyed by a number or a `bool` is not read below a flattened member.** serde reads what a `#[serde(flatten)]` member holds with every key as text, and reads no number and no `bool` from text: `pub struct Soundings { #[serde(flatten)] pub rest: BTreeMap<isize, u8> }` writes `{"7":2}` and refuses it on the way back (`invalid type: string "7", expected isize`), and it refuses a struct flattened with such a map under a key of its own, at any depth, the same way; an `Option` of either reads it back as `None`. A `#[serde(flatten)]` member that holds such a map, at any depth, stops the build, on a struct and on a variant of an enum, in every build with `serde`, naming the member and the key: the map itself, under an `Option`, a list, a `Box` or another map's value, a type declared above that holds one (a struct, an enum, a [brand](#branded-newtypes) or an alias), and a generic type filled at one. The key is a number or a `bool`, under `Box`, `Rc`, `Arc` or `Cow`, or a brand or an alias over one declared above, as for a type that [lists `"mongodb"`](#mongodb-operations-and-typed-filters). A key that is one of the holder's own type parameters, a member serde does not read (`skip_deserializing`), a member its author reads through a `deserialize_with` or a `with` of their own, and a type declared below the holder are not refused. A map keyed by text, a `char`, a date or a plain enum is read from each key's text, and builds. `pub struct Soundings { #[serde(flatten)] pub rest: BTreeMap<isize, u8> }` answers:

  ```text
  error: model_schema: type `Soundings`: field `rest`: `#[serde(flatten)]` cannot hold a map keyed by `isize` — serde reads every key below a flattened member as text, and reads no number and no `bool` from text, so no entry of that map reads back. Key the map by a `String`, or write the field under a key of its own.
  ```

  A type parameter read below a flattened member is held to the same rule where it is filled: `pub struct Rows<T> { #[serde(flatten)] pub rest: BTreeMap<T, u8> }` reads `T` there as the key of a map, and `pub struct Spread<T> { #[serde(flatten)] pub rest: T }` reads `T` there. Both build, and the build stops where a number or a `bool` fills `Rows`, or a type that holds a map keyed by one fills `Spread`: at any depth of a member, flattened or not (serde reads no entry of `Rows<u32>` wherever it reads one), in an alias, in a `default_types` entry of the type itself, and in a type that hands its own parameter on. The error is spanned on the filling. `pub struct Dock { pub rows: Rows<u32> }` answers:

  ```text
  error: model_schema: type `Dock`: field `rows`: `Rows` reads its type parameter `T` as the key of a map below `#[serde(flatten)]`, and `u32` fills it — serde reads every key below a flattened member as text, and reads no number and no `bool` from text, so no entry of that map reads back. Key the map by a `String`, or write the member under a key of its own.
  ```

- **A map keyed by a number or a `bool` is not read in an untagged enum, nor in an enum under a `tag` with no `content`.** serde reads such an enum from a copy of the payload it buffers, where every key is text, as it reads a flattened member: `#[serde(untagged)] pub enum Leadlines { Lines(BTreeMap<u32, u8>) }` writes `{"7":2}` and refuses it on the way back (`data did not match any variant of untagged enum Leadlines`), and `#[serde(tag = "kind")] pub enum Plumbed { Named { depths: BTreeMap<u32, u8> } }` writes `{"kind":"Named","depths":{"7":2}}` and refuses it (`invalid type: string "7", expected u32`). A member of a variant of such an enum that holds such a map stops the build, in every build with `serde`, naming the variant, the member and the key, with the reach of a flattened member: the map at any depth, a type declared above that holds one, a generic type filled at one, and a type parameter the enum reads there, where a number, a `bool` or a map keyed by one fills it. A key that is one of the enum's own type parameters, a member or a variant serde does not read (`skip_deserializing`), a member its author reads through a `deserialize_with` or a `with` of their own, and a type declared below the enum are not refused. An enum under its name and an enum under a `tag` and a `content` key build: serde reads the first with no copy, and the second where the tag comes ahead of the content, as serde writes it. `#[serde(untagged)] pub enum Leadlines { Lines(BTreeMap<u32, u8>) }` answers:

  ```text
  error: model_schema: type `Leadlines`: variant `Lines`, field `0`: an untagged enum cannot hold a map keyed by `u32` — serde reads an untagged enum from a copy of the payload it buffers, where every key is text, and reads no number and no `bool` from text, so no entry of that map reads back. Key the map by a `String`.
  ```

  and `#[serde(tag = "kind")] pub enum Plumbed { Named { depths: BTreeMap<u32, u8> } }` answers:

  ```text
  error: model_schema: type `Plumbed`: variant `Named`, field `depths`: an enum under a tag alone cannot hold a map keyed by `u32` — serde reads an enum under a tag alone from a copy of the payload it buffers, where every key is text, and reads no number and no `bool` from text, so no entry of that map reads back. Key the map by a `String`, or name a `content` key: serde writes the tag ahead of the content, and reads the content that follows it without a copy.
  ```

- A flattened map is handed every key of its object, so in **Zod** its record takes every key.
- In the **JSON Schema** a flattened map's values are the object's `additionalProperties`: `{"type": "object", "properties": {}, "required": [], "additionalProperties": {"type": "integer"}}` reads `{"north":2}` and refuses `"north": "x"`. A struct that flattens a map of its own, and an alias of a map, merge as the map does. A map keyed by a plain enum adds each of its members as an optional key, and every other key serde hands it is refused, as serde refuses it. A map keyed by a `char`, a date or a time, or by a type declared above that writes one, names the keys it reads by that key's own description, in `propertyNames`: `pub struct Ratlines { #[serde(flatten)] pub turns: BTreeMap<char, u8> }` is `{"type": "object", "properties": {}, "required": [], "additionalProperties": {"type": "integer"}, "propertyNames": {"type": "string", "minLength": 1, "maxLength": 1}}`, which reads `{"n":2}` and refuses `{"nn":2}`, as serde does, and a `NaiveDate` key is `{"type": "string", "format": "date"}`.
- **JSON Schema** stays strict: rather than `allOf` (which cannot faithfully compose a tagged union under `additionalProperties: false`), the base properties are distributed into each branch of the flattened union, keeping every branch closed. Both spellings of a union are distributed into: a discriminated enum's `oneOf` and an untagged enum's `anyOf`. Plain-struct flattens merge into a single closed object; multiple flattened unions form a cross-product.
- Each flattened union keeps the spelling it was written under. A discriminated enum's members are exclusive, so its branches stay a `oneOf`; an untagged enum is first-match-wins and its members may overlap (one member's keys a subset of another's, the difference optional), so its branches are an `anyOf` -- under `oneOf` the document would reject exactly what Serde writes for the narrower member, which matches both branches. An object flattening one of each nests the wrappers, the untagged `anyOf` sitting inside each branch of the discriminated `oneOf`.
- A flattened source reached through an `Option` is a choice rather than a key set. Serde writes its members beside the object's own for a `Some` and writes the object alone for a `None`, so the **JSON Schema** offers both under an `anyOf`: one branch with the source's members merged in, one naming the object's own keys alone. Folding the members into a single object would require keys the `None` payload never carries, and dropping them from `required` would admit a base written in part -- neither is a payload Serde produces. A union reached through an `Option` keeps its own spelling inside the branch and gains the absence outside it.
- **Zod** cannot name a choice as an operand, so it multiplies instead. An intersection recognizes exactly the keys its operands name; `z.discriminatedUnion` propagates its members' keys and is an operand like any other, while a plain `z.union` names none -- each of its branches would be asked to validate the whole payload alone and reject the keys the container and the sibling branches carry, which is every payload the object writes. So a flattened untagged enum, and a flattened source reached through an `Option`, are written as a union *of* intersections: one branch per combination, with the object's own keys bound once to `{Type}$OwnSchema` and read by every branch. A union nested inside a union contributes its leaves, the nesting writing no key of its own. With one combination there is nothing to choose between, so the object's own keys stay where they were.
- **An `Option` of a map is no choice between two key sets.** serde reads a flattened `Option` of a map as `None` wherever an entry it hands the map does not read as the map's value type, and drops every entry with it, so it reads every object its holder reads. `pub struct Locker { #[serde(default, flatten, skip_serializing_if = "Option::is_none")] pub counts: Option<BTreeMap<String, u8>> }` reads `{"north":2}` with `counts: Some({"north": 2})`, and `{"north":"x"}` with `counts: None`, which it writes back as `{}`. The absence of such a map is therefore every key, with any value. In **Zod** it is joined after the branch that reads the map, and reads no entry: `Locker$OwnSchema.and(z.record(z.string(), z.unknown()).transform(() => ({})))`, so `{"north":"x"}` reads `{}`, as serde reads it. In the **JSON Schema** its branch admits every other key, `"additionalProperties": true`. That holds for a map under any key, for a type declared above the holder that serde writes as a map (an alias of a map, a [brand](#branded-newtypes) over one, a struct whose one member is a flattened map), reached through the field's own `Option` or through an alias of an `Option`, on a struct and in a variant under its name, under a `content` key or untagged. The TypeScript type is the one it was: serde writes `None` as no entry.
- **An `Option` of a struct or an enum is read as absent where its keys do not read.** serde reads a flattened `Option` of a struct or an enum as `None` wherever the keys it hands the source do not read as the source, and drops those keys with it, with no error. `pub struct Hawser { #[serde(default, flatten, skip_serializing_if = "Option::is_none")] pub burgee: Option<Burgee> }`, where `Burgee` is `pub struct Burgee { pub label: String }`, reads `{"label":"a"}` with `burgee: Some(Burgee { label: "a" })`, and `{"label":3}` with `burgee: None`, which it writes back as `{}`. TypeScript, **Zod** and the **JSON Schema** refuse `{"label":3}`: each offers the source's keys as the source reads them, or none of them. So a client that runs these checks cannot send a key that does not read, and a server that reads one from a client that does not run them loses it with no error.
- The same multiplication runs where an enum's own struct variant flattens such a source, on every tagging. A variant's object is a fragment of a larger literal with no name to bind beside it, so the combinations are written where the object stands: one `z.strictObject({ ... }).and(...)` per key set, joined in a `z.union`. The variant still contributes exactly one member to the enum's union -- that member is a union itself. Externally tagged, it sits under the variant-name key; adjacently tagged, under the content key, leaving the enclosing `z.discriminatedUnion` intact because the tag stays in the member's own object; internally tagged, it *is* the member, and the enclosing union drops to a plain `z.union` for the reason any flattened member forces it to; untagged, it is the member as well. Two separately written `z.strictObject`s over the same keys validate identically, so writing the object once per combination costs generated text and nothing else. TypeScript needs no equivalent -- it distributes an intersection over a union on its own -- and JSON Schema already multiplies these branches at run time.
- That multiplication needs the union's members, which reach the merge through the registry -- so **an untagged enum must be declared above the object that flattens it**. Declared below, it has not expanded yet and the merge names it as one operand, the spelling that rejects every payload the object writes. Nothing at the merge tells such a source apart from a plain struct declared below, which is a spelling that works, so this is a declaration-order requirement rather than a diagnostic. It sits beside the reason the operands are deferred at all: a base is a `const` of its own, and nothing orders one type's schema against another's.
- Only a value Serde writes as an object can be flattened -- Serde refuses the rest at run time (`can only flatten structs and maps`). Flattening a plain `#[model_schema()]` enum, or an alias of one, is rejected at expansion in every build with `serde`, a build of the Dart, Swift and Kotlin clients alone included, and so is a variant that holds one beside the tag `#[serde(tag = "...")]` writes: Serde writes it as the key naming its variant, holding `null`, which no schema and no client reads. Only an enum declared above the type that flattens it is seen there. Any other type declared above that Serde does not write as an object is rejected at expansion in every build with `serde` as well, in a struct and in a variant's members, naming the field's type and the remedy (write the field as a named member): a [brand](#branded-newtypes) or a struct of one slot over text, a number, a `bool` or a list, a tuple struct of any other number of slots, an alias of one of those, and an `#[serde(untagged)]` enum, or an alias of one, with a member Serde does not write as an object, which is refused with its branch named. With `#[serde(transparent)] pub struct RecordId(String);` declared above it, `pub struct Carded { #[serde(flatten)] pub id: RecordId, pub name: String }` answers:

  ```text
  error: model_schema: `Carded`: `#[serde(flatten)]` of `RecordId` is not written as an object — its schema describes a `string`, which has no members to merge, and what serde writes for it does not join the object being written; write the field as a named member so the value gets a key of its own. This refusal holds in every build with serde, and only a type declared above reaches it: one declared below, or in another crate, is not refused here, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.
  ```

  A generic type declared above whose value is its own type parameter -- a [brand](#branded-newtypes), a struct of one slot or an alias over the parameter, under `Box` or through another such type -- is read at what fills that parameter, flattened directly or under an `Option`, and so is a type declared above whose value is one, filled. Filled at text, a number, a `bool`, a list, or a type declared above that Serde writes as no object, it is refused with the error of a value written as no object, naming the type with its arguments; filled at an `Option` of one of those, it is refused with that `Option`'s branch, as a struct of one slot over the `Option` is. Serde refuses every write and every read of one filled at text, a number or a list (`can only flatten structs and maps (got a string)`), and every write of a `Some` of one filled at an `Option` of text. Filled at a struct or at an `Option` of one, at a type declared below, or at the holder's own type parameter, it builds. With `pub struct ProbeWrap<T>(T);` declared above it, `pub struct ProbeFlatText { pub id: String, #[serde(flatten)] pub wrap: ProbeWrap<String> }` answers:

  ```text
  error: model_schema: `ProbeFlatText`: `#[serde(flatten)]` of `ProbeWrap<String>` is not written as an object — its schema describes a `string`, which has no members to merge, and what serde writes for it does not join the object being written; write the field as a named member so the value gets a key of its own. This refusal holds in every build with serde, and only a type declared above reaches it: one declared below, or in another crate, is not refused here, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.
  ```

  An `#[serde(untagged)]` enum declared above whose member names such a generic type, filled, is read at that filling too: `Text(ProbeWrap<String>)` is a member written as text, as `Text(String)` is, and the enum is refused with that member's branch, flattened and [held beside a tag](#internally-tagged-enums-tag-with-no-content). Serde refuses every write of that member flattened (`can only flatten structs and maps (got a string)`). Filled at a struct, or at a type declared below, the member builds. With `pub struct ProbeWrap<T>(T);` and `#[serde(untagged)] pub enum ProbeWrapMember { Obj { a: String }, Text(ProbeWrap<String>) }` declared above it, `pub struct ProbeFlatWrapMember { pub id: String, #[serde(flatten)] pub wrap: ProbeWrapMember }` answers:

  ```text
  error: model_schema: `ProbeFlatWrapMember`: `#[serde(flatten)]` of `ProbeWrapMember` writes a union member that is not an object — its branch 2 describes a `string`, which has no members to merge, and what serde writes for that member does not join the object being written; write the field as a named member so the value gets a key of its own. This refusal holds in every build with serde, and only a type declared above reaches it: one declared below, or in another crate, is not refused here, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.
  ```

  An `#[serde(untagged)]` enum declared above whose member is its own type parameter -- itself, under `Box` or an `Option`, in such a generic type, or handed on to another such enum -- is read at what fills that parameter, and so is an alias of it, filled: flattened as `ProbeOwnMember<String>`, the member `Held(X)` is written as text, as `Held(String)` is, and the enum is refused with that member's branch, named with its arguments, flattened and [held beside a tag](#internally-tagged-enums-tag-with-no-content). Serde refuses every write of that member flattened (`can only flatten structs and maps (got a string)`). Filled at a struct, at the holder's own type parameter, or declared below, it builds. With `#[serde(untagged)] pub enum ProbeOwnMember<X> { Obj { a: String }, Held(X) }` declared above it, `pub struct ProbeFlatOwnMember { pub id: String, #[serde(flatten)] pub wrap: ProbeOwnMember<String> }` answers:

  ```text
  error: model_schema: `ProbeFlatOwnMember`: `#[serde(flatten)]` of `ProbeOwnMember<String>` writes a union member that is not an object — its branch 2 describes a `string`, which has no members to merge, and what serde writes for that member does not join the object being written; write the field as a named member so the value gets a key of its own. This refusal holds in every build with serde, and only a type declared above reaches it: one declared below, or in another crate, is not refused here, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.
  ```

  A struct of one slot or a [brand](#branded-newtypes) declared above whose value is an `#[serde(untagged)]` enum with a member Serde writes as no object is read as that enum, as an alias of it is: over the enum itself, through such a generic type (`ProbeWrap<ProbeTextMember>`), or over the enum filled at its own parameter (`ProbeOwnMember<String>`), it is refused with that member's branch, and named as written, flattened and [held beside a tag](#internally-tagged-enums-tag-with-no-content). Serde refuses every write of that member flattened (`can only flatten structs and maps (got a string)`). Over an enum whose every member is an object, or one declared below, it builds. With `#[serde(untagged)] pub enum ProbeTextMember { Obj { a: String }, Text(String) }` and `#[model_schema(no_display)] pub struct ProbeSlotText(ProbeTextMember);` declared above it, `pub struct ProbeFlatSlotText { pub id: String, #[serde(flatten)] pub wrap: ProbeSlotText }` answers:

  ```text
  error: model_schema: `ProbeFlatSlotText`: `#[serde(flatten)]` of `ProbeSlotText` writes a union member that is not an object — its branch 2 describes a `string`, which has no members to merge, and what serde writes for that member does not join the object being written; write the field as a named member so the value gets a key of its own. This refusal holds in every build with serde, and only a type declared above reaches it: one declared below, or in another crate, is not refused here, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.
  ```

  A generic alias, struct of one slot or [brand](#branded-newtypes) declared above whose value is an `#[serde(untagged)]` enum whose member is its own type parameter, filled at the holder's own type parameter (`pub type ProbeOwnAliasOf<T> = ProbeOwnMember<T>;`), is read at what fills the holder, and so are an alias of one, a generic type filled at one, and an untagged enum with a member over one: flattened as `ProbeOwnAliasOf<String>`, it is refused as `ProbeOwnMember<String>` is, with that member's branch, and named with its arguments, flattened and [held beside a tag](#internally-tagged-enums-tag-with-no-content). Serde refuses every write of that member flattened (`can only flatten structs and maps (got a string)`). Filled at a struct, at the holder's own type parameter, or over an enum declared below, it builds. With `ProbeOwnMember<X>` and `#[model_schema(default_types(T = String))] pub type ProbeOwnAliasOf<T> = ProbeOwnMember<T>;` declared above it, `pub struct ProbeFlatOwnAliasOf { pub id: String, #[serde(flatten)] pub wrap: ProbeOwnAliasOf<String> }` answers:

  ```text
  error: model_schema: `ProbeFlatOwnAliasOf`: `#[serde(flatten)]` of `ProbeOwnAliasOf<String>` writes a union member that is not an object — its branch 2 describes a `string`, which has no members to merge, and what serde writes for that member does not join the object being written; write the field as a named member so the value gets a key of its own. This refusal holds in every build with serde, and only a type declared above reaches it: one declared below, or in another crate, is not refused here, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.
  ```

  A value Serde writes as a plain enum's variant name by way of a type declared above is refused with the error of the plain enum itself, in every build with `serde`, flattened into a struct or into a variant's members, and named as written: a [brand](#branded-newtypes) or a struct of one slot over the plain enum, a generic type declared above whose value is its own type parameter, filled at the plain enum, a type whose value is one of those, and an `Option` of any of them or in any of them (`pub struct ProbeMaybeMoodSlot(Option<ProbeMood>);`, `ProbeWrap<Option<ProbeMood>>`). Serde writes each as the key naming the variant, holding `null` (`{"id":"i","Calm":null}`), and reads it back, as it writes and reads the plain enum flattened; a `None` writes no key. With `pub enum ProbeMood { Calm, Tense }` and `#[model_schema(no_display)] pub struct ProbeMoodSlot(ProbeMood);` declared above it, `pub struct ProbeM1 { pub id: String, #[serde(flatten)] pub m: ProbeMoodSlot }` answers:

  ```text
  error: model_schema: field `m` of `ProbeM1` carries `#[serde(flatten)]` over `ProbeMoodSlot`, which serde does not write as an object: a plain enum writes its own variant name, so it contributes no members to the object being written — serde writes that name as a key holding null, which a schema closed around the remaining fields rejects. Write the field as a named member so the value gets a key of its own, or flatten a type serde writes as an object. This refusal holds in every build with serde, and only a plain enum declared above reaches it: one declared below, or in another crate, is not refused here, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.
  ```

  and, with `pub struct ProbeWrap<T>(T);` declared above it, `pub struct ProbeM3 { pub id: String, #[serde(flatten)] pub m: ProbeWrap<ProbeMood> }` answers:

  ```text
  error: model_schema: field `m` of `ProbeM3` carries `#[serde(flatten)]` over `ProbeWrap<ProbeMood>`, which serde does not write as an object: a plain enum writes its own variant name, so it contributes no members to the object being written — serde writes that name as a key holding null, which a schema closed around the remaining fields rejects. Write the field as a named member so the value gets a key of its own, or flatten a type serde writes as an object. This refusal holds in every build with serde, and only a plain enum declared above reaches it: one declared below, or in another crate, is not refused here, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.
  ```

  A value written in place that Serde writes as no object -- text, a number, a `bool`, a list, a tuple or [`()`](#the-unit-), under an `Option` or not -- is refused at expansion too, in every build with `serde`: Serde refuses every write of the first five flattened (`can only flatten structs and maps (got a tuple)`), and writes `()` as no key, which the TypeScript type and the Zod schema, intersecting the object with `null`, do not admit. `pub struct Host { pub id: String, #[serde(flatten)] pub pair: (String, u32) }` answers:

  ```text
  error: model_schema: `Host`: `#[serde(flatten)]` of `(String, u32)` is not written as an object — its schema describes a `array`, which has no members to merge, and what serde writes for it does not join the object being written; write the field as a named member so the value gets a key of its own. This refusal holds in every build with serde.
  ```

  An `#[serde(untagged)]` enum declared above with a member Serde writes as a variant's name, a plain enum's or an externally tagged enum's unit variant's, is refused with that member's branch in the words of a plain enum: Serde writes the name as a key holding `null`, and reads it back.

  A type declared below the one that flattens it, or in another crate, is caught when `json_schema()` runs, in the words of the refusal of its kind, and a flattened union is checked there member by member.

### Untagged Enums (`#[serde(untagged)]`)

An enum marked `#[serde(untagged)]` serializes as just its content, with no discriminator field. It generates a TypeScript union (`A | B`), a Zod `z.union([...])`, and a JSON Schema `anyOf`. Newtype (`S(T)`) and named-struct (`{ a: A }`) variants are supported.

```rust
/// ISO-8601 date string; branded newtype carrying the regex pattern.
#[model_schema(pattern = r"^\d{4}-\d{2}-\d{2}$")]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct DateString(pub String);

/// A date sample value: an ISO date string OR an epoch number.
#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum DateValue {
    N(i64),
    S(DateString),
}
```

Generated TypeScript:

```typescript
export type DateValue = number | DateString;
```

Generated Zod:

```typescript
const DateValue$RawSchema = z.union([z.number().int(), DateString$Schema]);
export const DateValue$Schema: ZodType<DateValue> = DateValue$RawSchema.meta({
  examples: []
});
```

Generated JSON Schema:

```json
{ "anyOf": [
  { "type": "integer" },
  { "type": "string", "pattern": "^[0-9]{4}-[0-9]{2}-[0-9]{2}$" }
] }
```

The `\d` the brand was declared with reaches the schema as `[0-9]`, the members it stands for, so the JSON Schema and the Rust validator accept the one set of strings -- see [What a `pattern` may contain](#what-a-pattern-may-contain).

Named-struct variants render each member as a closed object:

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum NamedUnion {
    A { x: String },
    B { y: i64 },
}
```

```typescript
// TypeScript
export type NamedUnion = { x: string } | { y: number };
// Zod
z.union([z.strictObject({ x: z.string(), }), z.strictObject({ y: z.number().int(), })])
// JSON Schema: { "anyOf": [ <object>, <object> ] }  — each branch has additionalProperties: false
```

Untagged enums compose with `#[serde(flatten)]`: a flattened variant carrying `Vec<DateValue>` renders `sampleValues: z.array(DateValue$Schema)` (TypeScript `Array<DateValue>`), and the JSON-schema `items` for that field is the `DateValue` `anyOf`.

A member of an untagged variant carries `#[model_schema_prop(...)]` exactly as the same field written in a tagged variant does: the constraint reaches the Zod schema, the JSON Schema and the Rust side alike -- the same per-member validator and the same [`validate()` accessor](#the-validate-method) -- and every guard the attribute earns is reported at the member. One thing is the untagged member's alone: it is also hung with the `deserialize_with` hook, because an untagged read picks the variant by which one accepts the payload, so the constraint decides what the value *is* and not merely whether it is admissible -- all but a member read through a reference and a member its author reads through a `deserialize_with` or a `with` of their own, each of which is read with no bound written and chooses no variant (see [what is checked while the members are read](#what-is-checked-while-the-members-are-read)). Everywhere else the members are read first and the value is judged once it is whole; see [where a constraint is checked](#where-a-constraint-is-checked).

What differs is the position, and it costs the read its diagnosis. Serde tries an untagged enum's variants in declaration order, and a member whose bound fails takes its variant out of the candidate set rather than ending the read -- which is exactly what the same declaration means under `anyOf` and under `z.union`. So a value the bound rejects lands on the next variant that accepts it, and when none does, serde's derived `Deserialize` has already discarded each candidate's own error and answers with one sentence of its own:

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum SoleConstrained {
    Slug {
        #[model_schema_prop(minLength = 2)]
        slug: String,
    },
}

let refused = serde_json::from_str::<SoleConstrained>(r#"{"slug":"A"}"#).unwrap_err();
assert_eq!(refused.to_string(), "data did not match any variant of untagged enum SoleConstrained");
```

The tagged twin of that declaration is refused with `'slug': too short: minimum length is 2, got 1`, because its tag names the variant before its members are read, and its read then judges the value. To learn which bound refused an untagged value, ask a surface that answers per member rather than per variant: validate the payload against the generated Zod schema or JSON Schema for a diagnosable verdict, or -- once the value is in hand -- call the enum's `validate()`, or the schema module's `validate_{variant}_{member}_value` directly.

The slot of a newtype variant carries the same attribute, which is how a free-text arm refuses some text with no brand declared for it:

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub enum KnownMimeType {
    #[serde(rename = "application/pdf")]
    Pdf,
}

#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum MimeType {
    Known(KnownMimeType),
    Other(#[model_schema_prop(minLength = 1, maxLength = 127, pattern = "^[a-z]+/[a-z0-9.+-]+$")] String),
}
```

The arm still writes and reads a bare string, and its TypeScript type is still `KnownMimeType | string`. `"text/csv"` reads as `Other` and `"application/pdf"` as `Known`; `""`, a value of 128 characters and `"Text/CSV"` match no variant, because the hook hung on the slot takes `Other` out as it does for a named member. The Zod schema writes the three checks on the arm's member of the union, `z.union([KnownMimeType$Schema, z.string().min(1, ...).max(127, ...).check(z.regex(...))])`, and the JSON Schema on its member of the `anyOf`: `{ "type": "string", "minLength": 1, "maxLength": 127, "pattern": "^[a-z]+/[a-z0-9.+-]+$" }`. A numeric arm takes `minimum` and `maximum` the same way. [A slot of a tuple variant](#a-slot-of-a-tuple-variant) says how `validate()` reports the slot. A length or a pattern that holds every string the enum carries is written once, on the enum: see [A Check Written on the Enum](#a-check-written-on-the-enum).

A member holding a map or a tuple describes as the struct field written from the same type does, on every surface. A map is dispatched on the classification its key earns -- enumerated properties for a key whose members the expansion knows, `additionalProperties` for an open one -- and a key no surface can write is refused at the member, at whatever depth the map sits at. A tuple is the fixed-arity array Serde writes, `prefixItems` and the arity bounds included. A member the parser could not classify at all is the one shape that stays permissive: it carries no type name to narrow with, so it admits any value, exactly as the same field does.

**Unsupported variants:** unit variants and multi-field tuple variants in an untagged enum produce a compile-time error.

### Nested Types

Types annotated with `#[model_schema()]` can reference each other. The TypeScript output uses the type's name (without any suffix) as the reference.

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Address {
    pub street: String,
    pub city: String,
    pub zip_code: String,
}

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct UserWithAddress {
    pub id: String,
    pub name: String,
    pub address: Address,
    pub backup_addresses: Vec<Address>,
}
```

A referenced type must be declared at item scope — in a module or at crate level, not inside a function body. Each type publishes its schema in a module beside itself, and a type that references one reaches that module through `use super::*`, which a module nested in a function body is not part of. The referencing type is unconstrained: it may sit inside a function body as long as everything it names does not. See [Function-Local Types](#function-local-types) for the error a violation produces.

### Recursive Types

The library supports recursive and self-referential types. In the generated Zod schema, recursive fields use JavaScript getter syntax to defer the reference and avoid "use before declaration" errors.

The self-reference can be written either with the type's own name (`Vec<TreeNode>`) or with the `Self` keyword (`Vec<Self>`); both produce identical output.

```rust
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TreeNode {
    pub val: String,
    pub children: Vec<TreeNode>,
}
```

Generated TypeScript:

```typescript
export type TreeNode = {
  val: string;
  children: Array<TreeNode>;
};
```

Generated Zod:

```typescript
const TreeNode$RawSchema = z.strictObject({
  val: z.string(),
  get children() { return z.array(TreeNode$Schema); },
});

export const TreeNode$Schema: ZodType<TreeNode> = TreeNode$RawSchema.meta({
  examples: []
});
```

Recursive enums are also supported. Only the variants that contain a self-reference use getter syntax; non-recursive variants use normal property syntax:

```rust
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "value")]
pub enum DynamicValue {
    #[serde(rename = "string")]
    String(String),
    #[serde(rename = "integer")]
    Integer(i64),
    #[serde(rename = "array")]
    Array(Vec<DynamicValue>),
    #[serde(rename = "object")]
    Object(HashMap<String, DynamicValue>),
}
```

Generated Zod:

```typescript
export const DynamicValue$Schema: ZodType<DynamicValue> = z.discriminatedUnion("type", [
  z.strictObject({
    type: z.literal("string"),
    value: z.string(),
  }),
  z.strictObject({
    type: z.literal("integer"),
    value: z.number().int(),
  }),
  z.strictObject({
    type: z.literal("array"),
    get value() { return z.array(DynamicValue$Schema); },
  }),
  z.strictObject({
    type: z.literal("object"),
    get value() { return z.record(z.string(), DynamicValue$Schema); },
  }),
]).meta({
  examples: []
});
```

A tuple struct has no key to write a getter on, so a slot that reaches the struct itself, or a type declared below it, is read behind `z.lazy` instead:

```rust
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NestsInASlot(pub String, pub Vec<Self>);
```

```typescript
const NestsInASlot$RawSchema = z.tuple([z.string(), z.lazy(() => z.array(NestsInASlot$Schema))]);
```

### Type Aliases

The `#[model_schema()]` macro supports `type` alias statements, creating semantic type aliases that appear in the generated TypeScript output. Use the `name` argument to control the generated TypeScript name.

```rust
use tixschema::model_schema;

#[model_schema(name = "DocumentId")]
pub type DocumentId = String;

#[model_schema(name = "Revision")]
pub type Revision = i64;

#[model_schema(name = "Tags")]
pub type Tags = Vec<String>;
```

Generated TypeScript:

```typescript
export type DocumentId = string;

export type Revision = number;

export type Tags = Array<string>;
```

Type aliases are referenced by name when used as struct fields:

```rust
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DocumentRecord {
    pub document_id: DocumentId,
    pub revision: Revision,
}
```

Generated TypeScript:

```typescript
export type DocumentRecord = {
  document_id: DocumentId;
  revision: Revision;
};
```

An alias takes `name`, and `default_types` where it declares a type parameter. `pattern`, `minLength` and `maxLength` are refused on it at expansion: an alias is another name for the type it names, not a type of its own, so a check written on one has nothing to belong to and nothing to enforce it. A value that carries a rule is a branded newtype, which takes all three -- see [Branded Newtype Validation Constraints](#branded-newtype-validation-constraints) -- and a rule that holds a single field is written on that field, in `#[model_schema_prop(...)]`.

### Type Parameters

A struct, a tuple struct, an enum, an alias and a branded newtype can each name their own type parameters, and every surface reads such a name under one rule.

**TypeScript binds the parameter for real.** It is a type surface, and the declaration it emits carries the parameter list: `export type Wrapper<T> = { id: T }` is a generic type, and a use site fills `T` in. The one position where it does not is a map's key, which states `string` on all three surfaces — see [Collections and Maps](#collections-and-maps).

**JSON Schema writes the document of whatever fills the parameter.** JSON Schema has no type parameters at all, so a document exists only at one filling: the type `default_types` declares where the document stands on its own, and the reference site's arguments where a field embeds it. The shape around the parameter -- an array, a map's keys, a tuple's arity -- is described either way.

**Zod publishes a factory rather than a schema -- and a default beside it.** A Zod schema is a runtime value and TypeScript generics do not exist at runtime, so there is no one value a generic type could publish: the caller has to say what fills each parameter before anything can validate. A generic type therefore exports `X$SchemaFactory`, a function taking one required schema argument per parameter, and a field written with a parameter composes the argument bound for it. It also exports `X$SchemaDefault`: the factory called at the type's own declared `default_types`, memoized like any other call, so a consumer who wants the ordinary filling never constructs the argument list by hand.

**With the `jsonschema` feature on, every type parameter needs a declared default type.** JSON Schema has no type parameters, so its document has to be built from one concrete filling, and `default_types` is where that filling is named — see [Declaring the default type](#declaring-the-default-type) below.

```rust
#[model_schema(default_types(IdType = String))]
pub struct Wrapper<IdType> {
    pub children: Vec<IdType>,
    pub id: IdType,
    pub name: String,
}
```

```typescript
export type Wrapper<IdType> = {
  children: Array<IdType>;
  id: IdType;
  name: string;
};

const buildWrapper$Schema = <IdType extends ZodType>(
  idType: IdType,
) =>
  z.strictObject({
  children: z.array(idType),
  id: idType,
  name: z.string(),
});

type Wrapper$SchemaOf<IdType extends ZodType> = ReturnType<
  typeof buildWrapper$Schema<IdType>
>;

const Wrapper$SchemaFactoryCache = new WeakMap<ZodType, Wrapper$SchemaOf<ZodType>>();

export function Wrapper$SchemaFactory<IdType extends ZodType>(
  idType: IdType,
): Wrapper$SchemaOf<IdType>;
export function Wrapper$SchemaFactory(
  idType: ZodType,
): Wrapper$SchemaOf<ZodType> {
  const hit = Wrapper$SchemaFactoryCache.get(idType);
  if (hit) return hit;

  const schema = buildWrapper$Schema(idType).meta({
    examples: []
  });
  Wrapper$SchemaFactoryCache.set(idType, schema);
  return schema;
}

export const Wrapper$SchemaDefault: ZodType<Wrapper<string>> = Wrapper$SchemaFactory(z.string());
```

Every parameter is a real TypeScript type parameter, never a bare `ZodType` annotation: `ZodType` defaults its own parameters, so an argument annotated with it would infer every field it validates as `unknown` and the caller would learn nothing from the schema handed back. Arguments are required -- a default would let a call site say nothing about a filling and still be handed a schema, which is exactly the silent mis-validation the factory exists to prevent.

Each factory memoizes on the *identity* of the arguments it was handed, one cache level per parameter. Two calls with the same argument objects return the identical schema, and no two argument lists collide -- a change in the first argument and a change in the last each key a different level:

```typescript
const wireDocument = EcmDocument$SchemaFactory(z.string(), z.number());
const storedDocument = EcmDocument$SchemaFactory(objectIdSchema, z.date());

EcmDocument$SchemaFactory(z.string(), z.number()) === wireDocument;  // false -- fresh arguments, new key
```

That is why `$SchemaDefault` is written as a call through the factory rather than composed inline: a declared default that itself names another generic item -- `default_types(IdType = DocumentId<String>)` -- reads back `DocumentId`'s own `$SchemaDefault` rather than reconstructing `DocumentId$SchemaFactory(z.string())` from scratch. The two calls would key different cache entries even though they mean the same filling, so reading the sibling's binding back is what keeps `EcmDocument$SchemaDefault` sharing the one `DocumentId` schema everything else that asks for the ordinary filling shares too.

That reference is deferred, exactly like a flattened base's is: `DocumentId$SchemaDefault` is a module-scope `const`, a generated module concatenates one type's output after another in whatever order the consuming project's entity list produces, and one macro invocation sees one type -- so nothing here can know whether `DocumentId`'s `const` is written above `EcmDocument`'s in the emitted module or below it.

```typescript
export const EcmDocument$SchemaDefault: ZodType<EcmDocument<DocumentId<string>, number>> =
  EcmDocument$SchemaFactory(z.lazy(() => DocumentId$SchemaDefault), z.number());
```

The same deferral covers an argument that does not fold -- a declared default naming a sibling at arguments other than that sibling's own default still calls its factory, behind the identical `z.lazy(() => …)` -- which is what keeps a cycle between two declared defaults from throwing at import: neither side can have registered the other's arguments yet when it is expanded, so neither folds, and both read the other's factory lazily rather than at the top of a `const` initializer.

A generic type that also flattens keeps the deferred read of its base, so declaration order stays irrelevant: `.and(z.lazy(() => Envelope$Schema))` composes inside the factory unchanged.

#### A generic type may reach itself

A generic type that holds itself -- directly, or through a second type that holds it back -- is described on every surface. JSON Schema hoists it into `$defs` once and points a `$ref` back at that one definition, which stands because every reference around the cycle carries the same filling. TypeScript writes the name inside itself, arguments and all. Zod calls the factory again with the argument the outer call was handed, and the memo cache is what makes that terminate: the schema is in the cache before the recursive call is made, so it comes back rather than being rebuilt.

```rust
#[model_schema(default_types(IdType = String))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node<IdType> {
    pub children: Vec<Self>,
    pub id: IdType,
}
```

```typescript
export type Node<IdType> = {
  children: Array<Node<IdType>>;
  id: IdType;
};

const buildNode$Schema = <IdType extends ZodType>(
  idType: IdType,
) =>
  z.strictObject({
  get children() { return z.array(Node$SchemaFactory(idType)); },
  id: idType,
});

interface Node$SchemaOf<IdType extends ZodType> extends ReturnType<
  typeof buildNode$Schema<IdType>
> {}
```

The member is written as a getter so that the call is made after the factory has reached its cache rather than while its object is still being built. A type whose builder defers a reference reads its schema type back under an `interface` rather than the `type` alias every other generic type keeps: a declaration file (`.d.ts`) writes a type that reaches itself only by name, and an alias is written there as the structure it resolves to, with `any` at the point the structure recurs. Under the interface the member is declared `z.ZodArray<Node$SchemaOf<IdType>>`, so a second package reading the declarations parses `children` as `Node<…>` all the way down.

A generic tuple struct that holds itself has no getter to defer behind, and the types of a tuple's slots are read as the tuple is built, so its builder cannot read its own factory's type there. It reaches itself through a function of its own, `X$SchemaSelf`, declared to return what the factory's schema parses -- `ZodType<X<IdType["_zod"]["output"]>>` -- and called behind `z.lazy`. The factory keeps its own precise return type. Where a cycle spans two types, the reference the getter is written on is the one pointing *forward* -- at a type declared below -- because that is the reference no cycle can be built without: if every reference in a cycle named something already declared, declaration positions would have to decrease all the way round. Deferring those leaves nothing that can cycle, and every reference pointing back at a type already declared is written as it stands.

An `Option` of a generic type is written as a union, and a union is checked against what each of its members takes and answers. For the schema a factory returns that is the very type still being worked out wherever the member belongs to a cycle, and the TypeScript compiler refuses it as a type that references itself. So wherever a union is written around a factory's call -- an `Option` field, or one marked `nullable` -- the call is handed in through a generic arrow, and the union is built over the arrow's type parameter:

```rust
#[model_schema(default_types(IdType = String))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chain<IdType> {
    pub id: IdType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<Box<Self>>,
}
```

```typescript
  get next() { return (<Reached$ extends SomeType>(reached$: Reached$) => z.union([z.null().transform(() => undefined), reached$, z.undefined()]).prefault(undefined))(Chain$SchemaFactory(idType)); },
```

It parses exactly what the plain union parses: an absent key, a `null` and an `undefined` all answer `undefined`, under a key that is there. An optional list or map of the type is built inside the arrow the same way (`z.array(reached$)`, `z.record(z.string(), reached$)`), and a member that reaches two generic types takes one parameter for each.

A map of the type itself goes one step further. A map's own type is read off what it holds as soon as the map is written, and for a generic type holding itself that is the builder's own return type, which is still being worked out. So inside a map the type is reached through the function a tuple struct reaches itself through, `X$SchemaSelf`, declared to return what the factory's schema parses:

```typescript
  get by_key() { return z.record(z.string(), Chain$SchemaSelf(idType)); },
```

Where the map is optional, that call is what the arrow is handed. A list or an `Option` of the type keeps the factory's own call, and with it the factory's precise type.

#### Declaring the default type

JSON Schema has no type parameters. A generic type's document has to be built from one concrete filling, and nothing in the declaration says which — so `default_types` says it, one `Parameter = Type` pair per type parameter:

```rust
#[model_schema(default_types(IdType = String, DateType = f64))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EcmDocument<IdType, DateType> {
    pub document_id: DocumentId<IdType>,
    pub created_at: DateType,
}
```

The pairs may be written in any order, and the argument sits beside every other item-level argument -- `name`, `pattern`, `minLength`, `maxLength`, `no_display` -- changing none of them. A lifetime and a const parameter name no type, so neither takes an entry.

A lifetime is dropped from every emitted surface, neither TypeScript nor JSON Schema having one and a borrowed value writing exactly what its owned form writes. A const renders in one position only -- an array length, which [describes as an unbounded array](#collections-and-maps) -- so a const handed to a written type as an *argument* is refused, spanned on the argument: an argument list is read as a list of types, and the const standing in one would have the JSON document call into a module named after it and the TypeScript declaration write a name it binds nothing for. A const no written type carries is untouched.

The declared filling is what `json_schema()` writes the document at. Beside it, a generic type's schema module publishes `json_schema_with`, which takes one document per parameter, positionally, in the order the item declares them:

```rust
let wire = EcmDocument::<String, f64>::json_schema();
assert_eq!(wire["properties"]["createdAt"], serde_json::json!({ "type": "number" }));

let stored = ecm_document_schema::Schema::json_schema_with(&[
    serde_json::json!({ "type": "object", "properties": { "$oid": { "type": "string" } } }),
    serde_json::json!({ "type": "string", "format": "date-time" }),
]);
```

A field naming a generic type reaches that same entry point with the arguments written at the field, so what it embeds is the document its own filling describes -- a `StoredFolder` holding `EcmDocument<ObjectId, DateTime<Utc>>` embeds the `$oid` object and a date-time string, where a `WireFolder` holding `EcmDocument<String, f64>` embeds a string and a number. A parameter forwarded into such a reference carries whatever filled the item forwarding it.

Zod reads the same declaration for `EcmDocument$SchemaDefault` -- see [Type Parameters](#type-parameters) above -- so the JSON document a standalone type describes and the Zod schema its ordinary filling validates against are built from the one statement of what that filling is, never two.

The declaration is read in both directions, and each refusal points at what earned it:

| Written | Verdict |
|---------|---------|
| An entry naming something the item does not declare | Refused in **every** feature configuration, spanned on the entry. A misspelled parameter fills nothing, and the parameter it was meant for would be left with no default at all. |
| A type parameter with no entry | Refused only where `jsonschema` is on, spanned on the parameter. Nothing else reads the default, so the same item compiles untouched without that feature. |
| `default_types` on an item declaring no type parameter | Refused, there being nothing for a filling to fill. |
| An entry filled at a type the macro cannot describe -- `i128`, `u128`, `f16`, `f128` | Refused where `zod` or `jsonschema` is on, spanned on the filling -- those are the two surfaces that read a declared filling. A name the type dispatch has no arm for is read as another `#[model_schema]` item, which is right for a sibling declared below and gibberish for a primitive: the JSON emission would name an `i128_schema` module nothing publishes, and the zod `$SchemaDefault` an `i128$Schema` binding no generated module exports. Only the reserved primitive names the dispatch answers for nowhere are refused, so a forward-referenced sibling keeps compiling -- and so does a filling at `char`, which describes as the one-character string serde writes for it. |

There is deliberately no fallback. Guessing a filling produces a document that silently rejects valid payloads, which is the failure the declaration exists to prevent.

#### The module preamble

Each factory declares its own store, so a generated module is the types in it and nothing else -- there is no preamble to emit ahead of them, and nothing shared between one type and the next.

A `WeakMap` fixes both of its own parameters at construction, so its value type cannot depend on its key type; a store declared beside the factory is therefore keyed and valued at the widened `ZodType` spelling. What recovers the precise types is an **overload**: the signature a caller sees names real type parameters, and the implementation beneath it takes the widened ones the store is written at. The read is then already the implementation's return type, so there is nothing to cast.

```typescript
const Wrapper$SchemaFactoryCache = new WeakMap<ZodType, Wrapper$SchemaOf<ZodType>>();

export function Wrapper$SchemaFactory<IdType extends ZodType>(
  idType: IdType,
): Wrapper$SchemaOf<IdType>;
export function Wrapper$SchemaFactory(
  idType: ZodType,
): Wrapper$SchemaOf<ZodType> { ... }
```

A type declaring N parameters writes N nested levels, each keyed by one argument, and the levels below the first are built where they are first needed:

```typescript
const Quintet$SchemaFactoryCache = new WeakMap<ZodType, WeakMap<ZodType, WeakMap<ZodType, WeakMap<ZodType, WeakMap<ZodType, Quintet$SchemaOf<ZodType, ZodType, ZodType, ZodType, ZodType>>>>>>();
```

Nothing in the emitted output is `as`, `any`, or `unknown`, and nothing a caller passes is written to. The store holds the arguments weakly, so a schema nobody else holds is collected rather than pinned for the life of the module.

#### An alias is a factory too

Every generic item publishes a factory, an alias among them, so a parameter inside one is the argument that factory binds for it:

```rust
#[model_schema(default_types(T = String))]
pub type Wrapper<T> = Vec<T>;
```

```typescript
export type WrapperType<T> = Array<T>;

const buildWrapperType$Schema = <T extends ZodType>(
  t: T,
) =>
  z.array(t);
```

Only the item's *own* parameters are read this way. A name the expansion cannot resolve because the type lives elsewhere keeps its `Name$Schema` reference, since that type publishes the binding.

One consequence is worth stating outright: a parameter is a value only where a factory binds one for it, and a JSON document is written at one filling rather than for the type as declared — so no `model_schema_prop` bound may be spelled against a type parameter, and a *field* typed with one is refused for the same reason, under `Option` and the transparent wrappers too. A list of the parameter is a list: `minLength` and `maxLength` on a `Vec<IdType>` [count its items](#list-length-minlength-maxlength). A branded newtype's own `pattern`/`minLength`/`maxLength` are the one exception: applied directly to a bare-parameter inner, they read the parameter's *declared default* rather than the parameter itself — see [Branded Newtype Validation Constraints](#branded-newtype-validation-constraints).

```rust
#[model_schema(default_types(IdType = String))]
pub struct Constrained<IdType> {
    #[model_schema_prop(minLength = 3)] // refused, and so is it on `Option<IdType>`
    pub id: IdType,
}
```

The value's type is whatever the instantiation supplies, so nothing here holds it to anything: Zod hands the check to a schema it has no bound on, the JSON schema describes the value as the permissive empty one, the generated validator emits no check for it, and serde reads the payload back untouched. Constrain the argument instead — declare the type the instantiation supplies as a branded newtype carrying the bound — or drop the key. The JSDoc says nothing about a bound the refusal turns away either, the sentence being written only where something holds the value to it.

### Branded Newtypes

`#[serde(transparent)]` tuple structs with a single public field generate branded TypeScript types. The newtype is invisible in JSON serialization but carries a distinct type identity in TypeScript, preventing accidental mixing of different ID types.

A `#[serde(transparent)]` struct with named fields is the same brand, over the one field serde reads it as -- the one that is not skipped for reading, carries no `default` and is not written `PhantomData`. serde writes `struct Slug { text: String }` as the string alone, exactly as it writes `struct Slug(String)`, so the two publish the same thing on every surface, and everything this section says of the slot holds for that field: the `Display` requirement and `no_display`, the checks written on the type, and no `#[model_schema_prop(...)]` on the field itself. A key on any other field of the struct, one serde neither reads nor writes (a skipped field, a `PhantomData` marker), is refused too.

A `#[serde(transparent)]` tuple struct of several slots is the same brand, over the one slot serde reads it as, by the same rule, wherever that slot sits: serde writes `struct Slug(#[serde(skip)] PhantomData<u8>, String)` as the string alone, so it publishes what `struct Slug(String)` publishes, and its checks and its `Display` read the second slot.

Which field that is follows what the struct derives. serde's `Deserialize` derive takes the field described above, and its `Serialize` derive takes the one that is not under `skip` or `skip_serializing` and not written `PhantomData`. A struct that derives `Serialize` alone is a brand over the field serde writes, and any other struct over the field serde reads. A struct that derives both, where serde writes one field and reads another, is refused at expansion with a message naming both, since no one schema describes both payloads:

```text
error: model_schema: type `Split`: `#[serde(transparent)]` writes the struct as the value of field `count` and reads it as the value of field `text`, so the payload serde writes is not one serde reads. A schema and an emitted client describe one value, so neither can be written for the pair. Make one field the value in both directions, and write `#[serde(skip)]` on every other field.
```

```rust
use tixschema::model_schema;
use serde::{Deserialize, Serialize};

// Generic branded newtype (good for parameterized IDs)
#[model_schema(default_types(IdType = String))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UserId<IdType>(pub IdType);

// Non-generic branded newtype
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CorrelationId(pub String);
```

Generated TypeScript (with `zod` feature):

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

export type CorrelationId = string & $brand<"CorrelationId">;
const CorrelationId$RawSchema = z.string().brand<"CorrelationId">().meta({
  description: "CorrelationId",
  examples: [],
});

export const CorrelationId$Schema: typeof CorrelationId$RawSchema = CorrelationId$RawSchema;
```

Generated TypeScript (without `zod` feature):

```typescript
declare const __brand_UserId: unique symbol;
export type UserId<IdType> = IdType & { readonly [__brand_UserId]: true };

declare const __brand_CorrelationId: unique symbol;
export type CorrelationId = string & { readonly [__brand_CorrelationId]: true };
```

Notes:

- The brand is published under the Rust ident, spelled exactly as written, unless `#[model_schema(name = "...")]` names another -- see [Published Names](#published-names-name). The name reaches the surface twice: as the exported type and as the brand tag the values carry.
- Generic parameter names (e.g., `IdType`) are preserved exactly in the TypeScript type.
- A generic brand publishes a factory, as every other generic item does, so the brand and its inner's shape land on the argument the caller supplied rather than on a value pinned at expansion. A parameter reaches it as the factory's own argument, under the rule in [Type Parameters](#type-parameters); a brand written over that parameter still describes what its inner writes, so `TagList<T>(pub Vec<T>)` is an array on every surface — `Array<T>`, `z.array(t)`, `{"type": "array", "items": {}}` — and not the bare parameter.
- The description is written before the brand in a factory and after it in a `const`. Inside a factory the receiver is the parameter the caller filled, and Zod's `.meta()` returns `this` — which TypeScript resolves back to that bare parameter, dropping the marker `.brand<"Name">()` had just added. Both orders build the same schema.
- Serde transparent serialization works normally -- the wrapper is invisible in JSON.
- An item whose published expression *is* another item's binding -- an alias of a brand, a one-slot tuple struct over one, and the `$SchemaDefault` of either where it declares a parameter -- is annotated `typeof {Name}$RawSchema` rather than `ZodType<{Name}>`, so the brand's narrowing survives the republish; `.brand()` narrows at the value position, which a restated `ZodType<{Name}>` discards. An item that builds an expression of its own keeps `ZodType<{Name}>`.
- The slot takes no `#[model_schema_prop(...)]`. A brand publishes its inner's own schema with a `.brand()` written onto it, so no key written on the slot -- `pattern`, `as`, `preprocess`, `literal`, `ts_optional`, `nullable`, any of them -- reaches any surface, and one written there is refused with a message naming the spelling that does work. The three checks a brand carries are written on the type itself: `#[model_schema(pattern = "...", minLength = N, maxLength = N)]`, under [Branded Newtype Validation Constraints](#branded-newtype-validation-constraints). The same holds for the field a `#[serde(transparent)]` struct with named fields or several slots is the value of. A key on any other field of that struct is refused too: serde neither reads nor writes it, so no key written there reaches any surface.
- Use branded newtypes for opaque IDs and phantom types to prevent passing the wrong ID type across domain boundaries.

#### A Named Inner Must Carry `#[model_schema()]`

**Error:** `cannot find module or crate <type>_schema in this scope` (`E0433`), reported at the inner type.

A brand carries its inner's wire form and adds a name to it, so it describes as that inner describes — by reference, through the schema module the inner's own `#[model_schema()]` publishes. A named inner declared without the attribute publishes no such module, and the reference names one that was never emitted. This is the same requirement a field naming that type lives under, reported the same way and at the same place:

```rust
// Wrong: `Foreign` carries no `#[model_schema()]`, so no `foreign_schema` module exists
#[derive(Serialize, Deserialize)]
pub struct Foreign(pub String);

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct HoldsForeign {
    pub id: Foreign, // E0433: cannot find module or crate `foreign_schema` in this scope
}

#[model_schema(no_display)]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Wrapped(pub Foreign); // E0433: the same module, named in brand position
```

The remedy is either of two:

- **Annotate the inner.** Put `#[model_schema()]` on `Foreign`, and both spellings resolve — the brand describing as whatever `Foreign` describes as.
- **Brand the wire form instead.** Where the inner is a foreign scalar this crate cannot annotate -- `uuid::Uuid`, `rust_decimal::Decimal` -- brand the type it is written as: `pub struct Wrapped(pub String)`, with `pattern` on the brand where the format is worth pinning.

A type this crate describes must be one it can read. The alternative -- letting a name it cannot resolve fall back to `{"type": "string"}` -- describes a struct as a string whenever the inner turns out to be one, which is the failure the brand's schema path exists to rule out.

#### The `Display` Requirement and `no_display`

Every branded newtype gets a `Display` impl that delegates to its inner value, so **the inner type must implement `Display`**. An inner type that does not is reported at the inner field, naming the trait:

```text
error[E0277]: `Vec<String>` doesn't implement `std::fmt::Display`
  --> src/lib.rs:7:21
   |
 7 | pub struct Tags(pub Vec<String>);
   |                     ^^^^^^^^^^^ the trait `std::fmt::Display` is not implemented for `Vec<String>`
```

Pass `no_display` for brands over an inner type that has none — a container, or a `PathBuf`. The brand then gets no `Display` impl and carries no such requirement; its schema and its serde transparency are unchanged. String constraints are a separate matter: `pattern`, `minLength`, and `maxLength` validate through `to_string()` for every inner but a path, so a constrained brand over one of the others needs a `Display` inner whether or not it passes `no_display`. A list is the other exception: it takes the two lengths as the count of its items and needs no `Display`. A map and a tuple cannot carry them at all (see [Branded Newtype Validation Constraints](#branded-newtype-validation-constraints)).

```rust
use tixschema::model_schema;
use serde::{Deserialize, Serialize};

#[model_schema(no_display)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Tags(pub Vec<String>);
```

A generic brand carries the requirement as a `Display` bound on its `Display` impl, so a non-`Display` type argument (e.g. `DocumentId<Vec<String>>`) is rejected where the brand is displayed rather than where it is declared.

#### Branded Newtype Validation Constraints

You can add `pattern`, `minLength`, and `maxLength` constraints directly on the `#[model_schema()]` attribute for branded newtypes. Constraints are enforced in three places: the generated Zod schema, serde deserialization, and a `validate()` method on the type. A brand is read as a struct or an enum is: its own read, then `validate()`, so a read [refuses what `validate()` refuses](#a-read-refuses-what-validate-refuses), with every message, and a type that holds the brand reads it with no judgement of its own and names it at its full path.

The three checks are written on a branded newtype, and on [an enum](#a-check-written-on-the-enum), which holds every string it carries to them. On a struct that is no brand and on a type alias they are refused at expansion, and a check on a field is written in `#[model_schema_prop(...)]`.

A bound written here holds every value of the brand. One that should hold a single field is written on that field instead, and the brand keeps none: see [A field typed with a branded string](#a-field-typed-with-a-branded-string).

**The inner type has to be one whose schema is a string, or a list** — `String`, `PathBuf`, `ObjectId`, a chrono date/time type, a named type whose own schema is one of those, or a list written on the brand itself. A numeric, boolean, map, tuple or opaque inner is rejected at expansion time, because the three constraints are string checks and each surface would read them differently: Zod's `.min`/`.max` become bounds on the value itself, JSON Schema ignores `minLength`/`maxLength`/`pattern` outside `"type": "string"`, and `validate()` measures the inner's `Display` rendering.

**A brand over a list takes `minLength` and `maxLength`, which count its items**, as they do on [a list field](#list-length-minlength-maxlength):

```rust
#[model_schema(no_display, minLength = 1)]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct NameConfig(pub Vec<Part>);

NameConfig(Vec::new()).validate();
// Err(["too short: minimum length is 1, got 0"])
```

`serde_json::from_str::<NameConfig>("[]")` is refused with the same sentence, the Zod schema is `z.array(Part$Schema).min(1, { error: ... }).brand<"NameConfig">()`, and the JSON Schema writes `"minItems": 1` on the array. The list is written itself or behind `Box`, `Rc`, `Arc` or `Cow`. The brand may declare type parameters (`struct Batch<T>(pub Vec<T>)`), and the count holds whatever fills them. Such a brand passes `no_display`, a list having no `Display` to delegate to. `pattern` on a brand over a list is refused: a list has no text, and a rule for each item goes on the item's own type.

**A named inner is judged by what the named type publishes**, not by the fact that it is a name. The brand appends its checks to that type's own schema binding — `Inner$Schema.min(3, ...)` — so a name whose schema is an object, a union, a `z.enum`, a number or `z.unknown()` is rejected exactly as the same shape spelled directly is, and the refusal names both the brand and the inner. A name whose schema is an array is rejected too: only a list written on the brand itself is counted.

That answer comes from the type's own expansion, so it is only available once that expansion has run: a brand written **above** the type it names, or over a type this crate never expands at all, is admitted with the emission it has always had. Declaration order is not a diagnostic — moving a declaration must not turn a compiling program into a rejected one — so keep a constrained brand below the type it constrains if you want the check.

**One of the brand's own type parameters is different: it reads its declared default instead of being refused.** A bare parameter is not itself a type the checks could measure, but `#[model_schema]` already requires (for `jsonschema`) or defaults (elsewhere) a concrete filling for every parameter — see [Declaring the default type](#declaring-the-default-type) — and that filling is exactly as concrete as a named type's own schema. So the guard asks the same question of the declared default it asks of a named inner: a string-shaped default carries the checks, a non-string one is refused in its place. The checks land once, on `$SchemaDefault`'s argument for that parameter; the factory itself stays unconstrained, so a caller filling the parameter with something else — an `ObjectId` schema, say — is never held to bounds meant for the default:

```rust
// Accepted: `IdType`'s declared default is `String`, so the checks land there.
#[model_schema(minLength = 24, maxLength = 24, pattern = "^[a-f0-9]{24}$", default_types(IdType = String))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DocumentId<IdType>(pub IdType);

// Rejected: `IdType`'s declared default is `u32`, which cannot carry them.
#[model_schema(minLength = 3, default_types(IdType = u32))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GenericCount<IdType>(pub IdType);
```

```typescript
const buildDocumentId$Schema = <IdType extends ZodType>(idType: IdType) =>
  idType.brand<"DocumentId">();

// The factory itself carries no check — a non-default filling is not held to it.
export const DocumentId$SchemaFactory = <IdType extends ZodType>(idType: IdType) => { /* ... */ };

// $SchemaDefault composes the factory with the constrained default argument.
export const DocumentId$SchemaDefault = DocumentId$SchemaFactory(
  z.string().min(24, { error: (issue) => `too short: minimum length is 24, got ${String(issue.input).length}` }).max(24, { error: (issue) => `too long: maximum length is 24, got ${String(issue.input).length}` }).check(z.regex(/^[a-f0-9]{24}$/, { error: "does not match pattern '^[a-f0-9]{24}$'" })),
);
```

`validate()` is written in the brand's own `impl<IdType> DocumentId<IdType>`, beside the schema delegates and under no bound the declaration does not write, so every filling has it and a type that holds the brand at any filling runs it. The string checks run where the filling is the declared default and at no other: the method asks which filling it holds. A `validate` of your own on one filling [does not build](#a-type-that-declares-a-parameter).

The serde read follows the same rule, being the brand's own read followed by `validate()`: it refuses a value of the declared default that breaks a check, and reads any other filling as it is, the same as `$SchemaFactory`. Neither asks `Display` of the parameter, so the brand is a type, and is read, at a filling that has none, and a holder's parameter needs no bound of the brand's to hold it:

```rust
#[model_schema(minLength = 2, default_types(CodeType = String))]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Code<CodeType>(pub CodeType);

#[model_schema(default_types(CodeType = String))]
#[derive(Serialize, Deserialize)]
pub struct Bin<CodeType> {
    pub code: Code<CodeType>,
    pub codes: Vec<Code<CodeType>>,
}

// A holder written by hand, over a parameter with no bound.
pub struct Plain<Held> {
    pub held: Code<Held>,
}

impl<Held> Plain<Held> {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        self.held.validate()
    }
}
```

```text
Code<String>("a").validate():                 Err(["too short: minimum length is 2, got 1"])
Code<String>("ab").validate():                Ok(())
Code<u32>(7).validate():                      Ok(())
Bin<String> { code: "a", codes: ["ab", "b"] }: Err(["'code': too short: minimum length is 2, got 1", "'codes.1': too short: minimum length is 2, got 1"])
Bin<u32> { code: 7, codes: [1] }:              Ok(())
Plain<&str> { held: "a" }:                    Ok(())
Plain<String> { held: "a" }:                  Err(["too short: minimum length is 2, got 1"])
```

An entry `default_types` leaves out falls back to `String`, the same fallback a parameter with no declared filling gets everywhere else the declaration is read — so a constrained bare-parameter brand with no `default_types` at all still compiles. Only the `jsonschema` feature actually requires a declared entry for every parameter, through the unrelated requirement in the table above.

**A name written over one of those parameters is rejected too**, whether or not the named type has expanded yet. `Tagged<T>` is not a type the declaration fixed — the instantiation fixes it — so the checks land on whatever the caller supplies: Zod appends them to a schema the call site decides the shape of, the one JSON document written for every instantiation still holds the `{}` a parameter describes as, and `validate()` measures the inner's `Display` rendering, which rejects `TaggedSlug(Tagged(7))` for having one decimal digit rather than for its value. This is the one place the registry's silence is not read as consent, and it is what keeps declaration order out of the diagnostic: the same two declarations written in the other order are rejected through the registry, so admitting this order would decide one program two ways.

```rust
// Rejected: the checks would measure whatever the instantiation supplies for `T`.
#[model_schema(minLength = 3, default_types(T = String))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TaggedSlug<T>(pub Tagged<T>);

#[model_schema(default_types(T = String))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Tagged<T>(pub T);
```

Brand a string-typed inner instead. The bound belongs on a declaration that writes a string — a `String` brand of its own, which the parameterised type can then be instantiated with (`Tagged<BoundedSlug>`), so the check sits where every surface can read it.

**The inner type also has to implement `Display`,** since validation runs against `to_string()`. That holds whether or not the brand passes `no_display`: the flag drops the `Display` impl, not the requirement.

A path inner is the one exception, and needs no `Display`: its checks read the path's `to_string_lossy` rendering — the string serde writes for it — exactly as a constrained path field's do. Such a brand still passes `no_display`, since the impl a brand gets by default has no inner `Display` to delegate to:

```rust
#[model_schema(no_display, minLength = 3, pattern = "^/[a-z]+$")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AssetPath(pub std::path::PathBuf);
```

That holds for every spelling of a path: a transparent wrapper writes nothing of its own on the wire and derefs to what it holds, so `Arc<Path>`, `Box<Path>`, `Cow<'static, Path>`, and `Rc<Path>` are branded and bounded exactly as `PathBuf` is.

```rust
#[model_schema(pattern = "^[a-z0-9_]+$", minLength = 3, maxLength = 50)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SlugId(pub String);
```

Generated Zod:

```typescript
const SlugId$RawSchema = z.string().min(3, { error: (issue) => `too short: minimum length is 3, got ${String(issue.input).length}` }).max(50, { error: (issue) => `too long: maximum length is 50, got ${String(issue.input).length}` }).check(z.regex(/^[a-z0-9_]+$/, { error: "does not match pattern '^[a-z0-9_]+$'" })).brand<"SlugId">().meta({
  description: "SlugId",
  examples: [],
});

export const SlugId$Schema: typeof SlugId$RawSchema = SlugId$RawSchema;
```

Serde deserialization validates automatically:

```rust
// Rejects values that violate constraints
let result: Result<SlugId, _> = serde_json::from_str("\"ab\"");
assert!(result.is_err()); // too short (min 3)

let result: Result<SlugId, _> = serde_json::from_str("\"UPPERCASE\"");
assert!(result.is_err()); // pattern mismatch

// Accepts valid values
let result: Result<SlugId, _> = serde_json::from_str("\"hello_world\"");
assert!(result.is_ok());
```

The `validate()` method checks constraints on instances constructed directly in code:

```rust
let slug = SlugId("hello_world".to_string());
assert!(slug.validate().is_ok());

let bad = SlugId("ab".to_string());
match bad.validate() {
    Ok(()) => unreachable!(),
    Err(errors) => println!("{:?}", errors), // ["too short: minimum length is 3, got 2"]
}
```

A brand's report names no field where a struct field's is named and quoted (`'username': ...`): a newtype has one value and no field name to quote, so whatever holds it supplies the name.

You can use any combination of the three constraints:

```rust
// Pattern only -- e.g., ObjectId hex string
#[model_schema(pattern = "^[0-9a-fA-F]{24}$")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ObjectIdStr(pub String);

// Length only -- no pattern
#[model_schema(minLength = 1, maxLength = 255)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NonEmptyString(pub String);
```

An inner type whose schema is not a string is rejected at the field:

```rust
// This will NOT compile:
#[model_schema(pattern = "^[0-9]+$")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BadNum(pub u64);
```

```text
error: model_schema: branded newtype `BadNum` applies string constraints (pattern, minLength,
       maxLength) to a numeric inner type, which cannot carry them: ...
 --> src/lib.rs:4:19
  |
4 | pub struct BadNum(pub u64);
  |                   ^^^
```

An inner type the macro cannot resolve because it lives elsewhere — another brand, a user type — is
admitted here and checked for `Display` instead, so a non-`Display` one is still reported at the
field:

```rust
#[model_schema(no_display)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Tags(pub Vec<String>);

// This will NOT compile either: `Tags` has no `Display` for `validate()` to render.
#[model_schema(pattern = "^[a-z]+$")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TagsBrand(pub Tags);
```

```text
error[E0277]: `Tags` doesn't implement `std::fmt::Display`
  --> src/lib.rs:12:26
   |
12 | pub struct TagsBrand(pub Tags);
   |                          ^^^^ unsatisfied trait bound
```

#### Doc Comments and Examples on Branded Newtypes

Branded newtypes support doc comments (for Zod `.meta({ description })`) and compiler-validated examples, just like structs and enums:

```rust
/// Generic document identifier.
///
/// - `DocumentId<String>` for API/HTTP layer
/// - `DocumentId<ObjectId>` for `MongoDB` layer
///
/// ```rust example
/// DocumentId("64de3d95ff45b119e5b53a7e".to_string())
/// ```
#[model_schema(default_types(IdType = String))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DocumentId<IdType>(pub IdType);
```

Generated Zod:

```typescript
const buildDocumentId$Schema = <IdType extends ZodType>(
  idType: IdType,
) =>
  idType.meta({
  description: "Generic document identifier.\n- `DocumentId<String>` for API/HTTP layer\n- `DocumentId<ObjectId>` for `MongoDB` layer",
  examples: ["64de3d95ff45b119e5b53a7e"],
}).brand<"DocumentId">();

type DocumentId$SchemaOf<IdType extends ZodType> = ReturnType<
  typeof buildDocumentId$Schema<IdType>
>;

const DocumentId$SchemaFactoryCache = new WeakMap<ZodType, DocumentId$SchemaOf<ZodType>>();

export function DocumentId$SchemaFactory<IdType extends ZodType>(
  idType: IdType,
): DocumentId$SchemaOf<IdType>;
export function DocumentId$SchemaFactory(
  idType: ZodType,
): DocumentId$SchemaOf<ZodType> {
  const hit = DocumentId$SchemaFactoryCache.get(idType);
  if (hit) return hit;

  const schema = buildDocumentId$Schema(idType);
  DocumentId$SchemaFactoryCache.set(idType, schema);
  return schema;
}

const DocumentId$RawSchemaDefault = DocumentId$SchemaFactory(z.string());

export const DocumentId$SchemaDefault: typeof DocumentId$RawSchemaDefault = DocumentId$RawSchemaDefault;
```

### Services

`#[service_schema()]` declares a service as a trait: one trait, one context, and one message in and one message out per operation. Failing to implement an operation is a compile error, which is the whole reason the construct is a trait rather than a table of handlers.

**Services need the `serde` feature.** A service's messages derive `Serialize` and `Deserialize`, a dispatcher reads payloads through `serde_json`, and the camelCase and kebab-case keys it puts on the wire come from serde attributes the macro writes. A build without the feature is refused at the declaration rather than left to publish TypeScript that names Rust fields the wire never carries:

```text
service_schema: a service needs tixschema's `serde` feature, and this build does not have it
       without it the TypeScript a service publishes names Rust fields rather than the camelCase
       and kebab-case keys its own dispatcher writes, so the two halves disagree about the wire
       add `features = ["serde"]` to the tixschema dependency in Cargo.toml
```

**Only the `typescript` feature publishes TypeScript at all.** `<Service>Schema::ts_definition()`
and every other `ts_*` accessor live behind it, independently of `dart`, `swift` and `kotlin`. A
build with `serde` and `dart` alone publishes `dart_definition()`, `dart_http_client()` and
`dart_ws_client()` with no TypeScript in sight, and the reverse holds too — `serde` and `typescript`
with none of the mobile features publishes TypeScript alone. Each language earns its own accessors
under `serde` plus that language's own feature; none of the four needs any of the others.

**A service that publishes TypeScript needs the `zod` feature too.** A message validates when it is
constructed, in both directions: the generated client parses what it is about to send before it
reaches a transport, and the generated dispatcher parses what arrived before it enters an
implementation, so an implementation may assume its message is valid. On the TypeScript side both
checks are the same parse, against the `<Message>$Schema` const `#[model_schema()]` publishes — and
only a build with the `zod` feature publishes one.

So a build with `typescript` on and `zod` off publishes the service's **types** and none of the
three seam artifacts: no `<Service>Schema::ts_client()`, no `<Service>Schema::ts_http_client()`, and
no `<Service>Schema::ts_service()`. The types are published as they always are, because they
describe what the Rust dispatcher and the Rust client put on the wire, and that half validates in
either build. The three that are withheld are withheld rather than emitted without their check: a
client that forwards whatever it is handed and a dispatcher that narrows an unread payload with `as`
both compile and both read exactly like the checked ones, while the Rust half of the same service
goes on validating — two halves of one service disagreeing about what they accept, with nothing to
say so.

A bundle that names either one in such a build is refused where it names it, which is the one place
the choice of features can still be acted on:

```text
error[E0599]: no associated function or constant named `ts_client` found for struct `UsageServiceSchema` in the current scope
   |
23 | #[service_schema()]
   | ------------------- associated function or constant `ts_client` not found for this struct
...
34 |     let _ = UsageServiceSchema::ts_client();
   |                                 ^^^^^^^^^ associated function or constant not found in `UsageServiceSchema`
```

**A service is declared at module scope, never inside a function body.** The module the macro
generates opens with `use super::*;`, which is how the client and the per-operation validators
reach the trait and the message types declared beside it. A module written inside a function body
has the enclosing *module* as its parent rather than the function, so `super` from there reaches
past every name that function declared. `#[model_schema()]` carries the same requirement, for the
same reason.

The macro cannot refuse the placement: an attribute macro is handed the annotated item's own tokens
and nothing about the scope it was written in, and a trait written inside a function is the same
tokens as one written beside it. A function-scoped declaration earns the compiler's own resolution
errors instead — one for the trait, and one for each type an operation named:

```text
error[E0405]: cannot find trait `UsageService` in module `super`
error[E0425]: cannot find type `BalanceRequest` in this scope
error[E0425]: cannot find type `BalanceResponse` in this scope
error[E0425]: cannot find type `BalanceError` in this scope
```

One consequence is worth knowing before it costs anyone an afternoon: rustdoc wraps a doctest with
no explicit `fn main` in one, so a doctest that declares a service writes `fn main() {}` of its own.

#### The Shape of a Service

A service names a **context** type parameter, and every operation takes it as its first argument after `&self`. An operation receives one message and answers `Result<Success, Error>`.

```rust
use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

#[model_schema()]
#[derive(Deserialize, Serialize)]
pub struct AvailableBalanceRequest {
    pub organization_id: String,
}

#[model_schema()]
#[derive(Deserialize, Serialize)]
pub struct AvailableBalanceResponse {
    pub available_credits: u32,
    pub is_post_paid: bool,
}

#[model_schema()]
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum BalanceError {
    DbError,
    InsufficientBalance { shortfall: u32 },
}

#[service_schema()]
pub trait UsageService<Ctx> {
    /// No reply, so no return type and no error arm.
    #[service_schema_op(one_way)]
    async fn apply_bundle(&self, ctx: &Ctx, req: AvailableBalanceRequest);

    /// Carried on the wire as `usage-generation-request` rather than `can-generate`.
    #[service_schema_op(message = "usage-generation-request")]
    async fn can_generate(
        &self,
        ctx: &Ctx,
        req: AvailableBalanceRequest,
    ) -> Result<AvailableBalanceResponse, BalanceError>;

    /// Several arguments after the context: the macro declares the message.
    async fn expire_credit(
        &self,
        ctx: &Ctx,
        organization_id: String,
        credit_id: String,
    ) -> Result<AvailableBalanceResponse, BalanceError>;

    /// One argument after the context: that argument already is the message.
    async fn get_available_balance(
        &self,
        ctx: &Ctx,
        req: AvailableBalanceRequest,
    ) -> Result<AvailableBalanceResponse, BalanceError>;

    /// None at all, so the macro declares an empty message.
    async fn sweep(&self, ctx: &Ctx) -> Result<AvailableBalanceResponse, BalanceError>;
}
```

The context is whatever an implementation needs and no caller may see -- a logger, a correlation id. The implementation chooses the type, and it reaches no message, no schema and no wire:

```rust
pub struct UsageContext {
    pub logger_name: String,
}

pub struct UsageBackEnd;

impl UsageService<UsageContext> for UsageBackEnd {
    async fn apply_bundle(&self, ctx: &UsageContext, req: AvailableBalanceRequest) {
        // ...
    }

    // ... and the other four, or this does not compile.
}
```

The trait is emitted as declared, except that `async fn` is desugared to `-> impl Future<Output = ...> + Send`, the desugaring the compiler's own `async_fn_in_trait` warning recommends. A trait with `async fn` is not dyn compatible, so the generated dispatcher is generic over the implementing type rather than taking `&dyn UsageService`.

#### Service Messages

Every operation receives exactly one message. How many arguments it takes after the context decides only who declares that message:

| arguments after the context | the incoming message is | who declares it |
| --- | --- | --- |
| exactly one | that argument, as-is | you, with `#[model_schema()]`, like any other type |
| more than one | a message declared from the argument list | the macro, named `<Operation>Request` |
| none | an empty message | the macro, named `<Operation>Request`, with no fields |

**One argument is the normal case and the one to reach for.** The argument *is* the message: nothing is synthesized, nothing is renamed, and the type is one you wrote and can reuse, version and reference from elsewhere.

**The multi-argument form costs something, and the cost is on the wire.** Each argument's name becomes a field name on the declared message, so renaming a parameter -- an invisible refactor in Rust -- moves a key on the wire, and no compiler will flag it. Reach for it where the input is one identifier and a named type would be ceremony; reach for a declared type everywhere else. Every generated message repeats this in its own rustdoc and in the JSDoc it publishes.

`expire_credit` takes two arguments, so `ExpireCreditRequest` is declared from them, in declaration order and with camelCase keys:

```typescript
export type ExpireCreditRequest = {
  organizationId: string;
  creditId: string;
};
```

```typescript
const ExpireCreditRequest$RawSchema = z.strictObject({
  organizationId: z.string(),
  creditId: z.string(),
});

export const ExpireCreditRequest$Schema: ZodType<ExpireCreditRequest> = ExpireCreditRequest$RawSchema.meta({
  examples: []
});
```

`sweep` takes none, so its message is empty rather than absent. An operation that later needs a field gains one here, instead of changing from carrying no payload to carrying one and breaking every caller:

```typescript
export type SweepRequest = Record<string, never>;

const SweepRequest$RawSchema = z.strictObject({
});

export const SweepRequest$Schema: ZodType<SweepRequest> = SweepRequest$RawSchema.meta({
  examples: []
});
```

`get_available_balance` takes one argument, so nothing is declared for it at all -- `AvailableBalanceRequest` is the message, exactly as you wrote it.

#### Operation Names

An operation's name is written once, in Rust, and spelled per language by convention. Nothing is repeated and no attribute is involved:

| where | spelling | derived as |
| --- | --- | --- |
| Rust | `get_available_balance` | the trait method as declared |
| TypeScript | `getAvailableBalance` | camelCase, what a TypeScript caller expects to type |
| the wire | `get-available-balance` | kebab-case of the method name |

`#[service_schema_op(message = "...")]` moves the **wire name only**. Rust still calls the operation by its method name and TypeScript by the camelCased one:

```rust
#[service_schema_op(message = "usage-generation-request")]
async fn can_generate(
    &self,
    ctx: &Ctx,
    req: AvailableBalanceRequest,
) -> Result<AvailableBalanceResponse, BalanceError>;
```

It exists for one reason: services already ship wire names nobody would derive from the method. An operation a caller knows as `canGenerate` may dispatch today as `usage-generation-request`, the two sharing no substring, so without the override no such service could adopt this construct without changing bytes. A greenfield operation writes no attribute at all. The override replaces the derived name rather than adding a second one -- after it, nothing answers to `can-generate`.

#### Service Errors

An operation's `Result` names **one** type in its error position. That type is normally an enum, which is a union, so "an operation's errors" is one `#[model_schema()]` type with several variants rather than several types.

What is per-operation is *which* type is named, and two operations on one service may name entirely unrelated ones. A service that genuinely has one error surface pays nothing for this -- it writes the same type in every signature. What it buys is that a read operation is never handed a write-conflict variant to match on, and a caller's `switch` over an operation's errors is exhaustive over exactly what that operation can return.

A variant may carry data of its own, which the tag sits beside:

```typescript
export type BalanceError = {
  errorCode: "db-error";
} | {
  errorCode: "insufficient-balance";
  shortfall: number;
};
```

#### One-Way Operations

An operation marked `#[service_schema_op(one_way)]` expects no reply, and therefore declares no return type:

```rust
#[service_schema_op(one_way)]
async fn apply_bundle(&self, ctx: &Ctx, req: AvailableBalanceRequest);
```

The flag and the return type must agree, and the macro checks both directions, so a forgotten `Result` cannot become a silent fire-and-forget. A missing return type with no flag is refused naming both choices:

```text
service_schema: operation `apply_bundle` has no return type
       add `#[service_schema_op(one_way)]` if it expects no reply,
       or give it a `Result<Success, Error>` return
```

And the flag with a return type is refused too:

```text
service_schema: operation `apply_bundle` is marked `one_way` but returns a value
       a one-way operation produces no reply
```

There is no error arm on a one-way operation, because there is no reply to carry one. An operation that needs to report failure is a request-and-reply operation declared wrong. In TypeScript a one-way method answers `Promise<void>`; in Rust it answers nothing beyond the send.

The one failure it can still report is its own. A message that fails its own validation never reaches the transport, and the fault naming the key has nowhere to go but out of band: in Rust it is `Err(ServiceFault)`, the failure arm the method already has, and in TypeScript `Promise<void>` has no failure arm at all, so it is **thrown**. What is thrown is published rather than left to be discovered from the body -- `<Service>Refusal`, an `Error` carrying the fault -- and the method's own `JSDoc` names it:

```typescript
export type UsageServiceRefusal = Error & { fault: UsageServiceFault };
```

Silently dropping a refused message was the alternative and is worse: a fault is a defect meant to page a human, and a dropped one pages nobody.

#### Service Validation

A message is validated when it is constructed, in both directions, by the validator `#[model_schema()]` already generates for it -- `validate()` on the Rust side, the Zod schema on the TypeScript side. Nothing new is written for a service. A generated Rust dispatcher reads a request with the message's read alone, `model_schema_read_unjudged`, through the `unjudged_<operation>` function the service's module publishes for each operation, and calls `validate()` next, so a request that breaks a bound or a check is a `failed-validation` fault naming its field, with every message in its detail.

Inbound, the dispatcher deserializes the payload into the operation's message and validates it *before* the operation body is entered. **A service implementation may therefore assume its incoming message is valid, because an invalid one never reaches it.**

Outbound, the generated client validates the message it is about to send, so a malformed request fails at the caller with the field named rather than as a remote error a round trip later -- and the transport is never touched:

```typescript
    async expireCredit(req) {
      const validated = ExpireCreditRequest$Schema.safeParse(req);
      if (!validated.success) {
        return {
          ok: false,
          error: {
            isServiceFault: true,
            fault: usageServiceOutboundFault("expire-credit", validated.error.issues),
          },
        };
      }
      const { answered } = await transport.request<UsageServiceExpireCreditResult>("expire-credit", validated.data, []);
      if (answered.ok) {
        const parsed = AvailableBalanceResponse$Schema.safeParse(answered.value);
        if (!parsed.success) {
          return {
            ok: false,
            error: {
              isServiceFault: true,
              fault: usageServiceReplyFault("expire-credit", parsed.error.issues),
            },
          };
        }
        return { ok: true, value: parsed.data };
      }
      const error = answered.error;
      if (typeof error === "object" && error !== null && "isServiceFault" in error) {
        return { ok: false, error };
      }
      const declared = BalanceError$Schema.safeParse(error);
      if (!declared.success) {
        return {
          ok: false,
          error: {
            isServiceFault: true,
            fault: usageServiceReplyFault("expire-credit", declared.error.issues),
          },
        };
      }
      return { ok: false, error: declared.data };
    },
```

A one-way method runs the same check and has nowhere to put the result of it, so it throws the refusal the section above names -- still before the transport is reached:

```typescript
    async applyBundle(req) {
      const validated = AvailableBalanceRequest$Schema.safeParse(req);
      if (!validated.success) {
        throw usageServiceRefused(usageServiceOutboundFault("apply-bundle", validated.error.issues));
      }
      await transport.notify("apply-bundle", validated.data, []);
    },
```

A replying method checks what comes back as well, in the tail of `expireCredit` above. A success is parsed against the operation's declared success and a failed reply against its declared error before the caller reads either: a fault the far side already sealed passes through, a reply that parses is returned as parsed, and one that does not -- a declared field missing, an undeclared key, another shape altogether -- answers a `failed-validation` fault naming the first offending key rather than a value typed as that error. A bundle therefore publishes the Zod schema of every declared error type beside its type.

#### Service Faults

A payload that will not deserialize, a message that fails validation, an operation name nothing recognises, a handler that panicked and a call the transport could not carry are none of the things an operation declared it could fail at. Folding them into a service's domain errors would put transport concerns in every signature, so they come back as a **fault**: a defect rather than a condition, meant to be logged at error level and to page a human.

A result therefore keeps two arms, with the fault riding inside the failure arm behind a literal a caller can narrow on:

```typescript
export type UsageServiceGetAvailableBalanceResult =
  | { ok: true; value: AvailableBalanceResponse }
  | { ok: false; error: BalanceError | { isServiceFault: true; fault: UsageServiceFault } };
```

A caller that only handles the declared errors still compiles; one that wants to distinguish narrows on `isServiceFault` and then on whatever the error type discriminates on, with no `default` arm to write:

```typescript
if (result.ok) {
  render(result.value.availableCredits);
} else if ("isServiceFault" in result.error) {
  reportUnexpected(result.error.fault);
} else {
  switch (result.error.errorCode) {
    case "db-error":
      return retryLater();
    case "insufficient-balance":
      return showUpgradePrompt();
  }
}
```

**No service implementation can answer with a fault.** In Rust an operation's signature admits only its own error type, so there is nowhere to put one. In TypeScript the interface an implementation satisfies is typed against an *outcome*, which has no fault member in it:

```typescript
export type UsageServiceGetAvailableBalanceOutcome =
  | { ok: true; value: AvailableBalanceResponse }
  | { ok: false; error: BalanceError };
```

**And the fault type itself refuses to be written.** An outcome with no fault member stops a fault being *returned*; it does not stop one being *built*, and a plain structural object type is writable by anyone who can name it. So the fault's fields publish under a name of their own and the name a caller reads is those fields under a brand keyed on a symbol the bundle declares and exports nowhere:

```typescript
export type UsageServiceFaultFields = {
  detail: string;
  field: string | undefined;
  kind: UsageServiceFaultKind;
  operation: string;
};

declare const usageServiceFaultSeal: unique symbol;

export type UsageServiceFault = UsageServiceFaultFields & {
  readonly [usageServiceFaultSeal]: true;
};
```

The fields still come from the Rust declaration, so nothing about the wire moved and nothing a caller reads changed -- narrow on `isServiceFault`, then read `kind`, `detail`, `field` and `operation`, or switch over `kind` exhaustively. The brand is a type and never a value: `declare const` emits nothing, and a fault carries no extra key.

What stops compiling is writing one. An object literal under a `UsageServiceFault` annotation cannot carry the branded property, and neither can a structurally-equal value assigned into a `UsageServiceFault` position -- a module outside the bundle cannot name the symbol, and a module inside has no value to write. What it does **not** stop is a deliberate type assertion: `built as UsageServiceFault` compiles, as does anything laundered through `any` or `unknown`, and TypeScript has no construct that refuses those. The generated constructors mint through exactly that assertion, from the fields type, which is the one direction an assertion is unambiguously sound in. So the seal makes fabricating a fault a deliberate, greppable act rather than something an annotated literal does silently -- it is not the `E0451` Rust gives, because TypeScript has none to give.

Exactly two generated places build one: the dispatcher, when an incoming message cannot be turned into a valid request or names an operation nothing recognises, and the client, when the message *it* is about to send fails its own validation or the transport reports that the call never landed. In Rust a client call answers `Result<Success, CallError<Error>>`, `CallError` being `Operation(E)` for the error the operation declared and `Fault(ServiceFault)` for a defect that reached the caller.

Which kind a refused payload is reported under is `serde_json`'s own classification of the refusal, not the shape of the sentence it wrote. `undeserializable-payload` means the bytes are not a document at all -- not JSON, or a document that ends early -- which is a sender whose serialization is broken, and such a refusal names no field because nothing was read far enough for a key to be what went wrong. `failed-validation` means the bytes read as a document and did not match the message: a field carrying the wrong type of value, a key that is missing, a value a bound refuses. That is a value someone supplied. The byte offset serde appends is dropped, it being a position inside an encoding the caller never saw.

The TypeScript dispatcher raises only the second of the two, and that is the same line rather than a different one: it is handed a payload somebody already read out of the bytes, so by the time it runs the bytes *were* a document and the only thing left to be wrong is what the document said. Bytes that are no document at all fail before it, in whatever turns them into the value it takes. So one defect reaches a call site under one kind whichever language served it.

The field is named wherever the refusal named one: a validator's report names it in single quotes, and serde names it in backticks for a missing or unknown key. A bound declared on a field's *type* rather than on the field -- a constrained brand -- is the validator's to report as every other is: the brand's own report names nothing, the brand being the value rather than a member of anything, and the message holding it writes the field's wire key into the report, so `'organizationId': too short: minimum length is 3, got 2` is what a caller reads, one quoted run holding the whole name, beside every other message of the request. A type mismatch is the one refusal left carrying no field: serde says what it expected and not where, and inventing a name for it would send a caller to the wrong key.

#### What a Service Generates

On the Rust side, beside the trait, in a module named for the service (`usage_service_schema`):

- `ServiceFault` and `ServiceFaultKind` -- readable by anyone, constructible by nothing outside the module. `ServiceFault` is the alias; the struct is declared as `<Service>FaultFields`, which is the name its *fields* publish under in TypeScript, `<Service>Fault` there being the sealed type written over them.
- `CallError<E>` -- `Operation(E)` or `Fault(ServiceFault)`.
- `<Operation>Message`, `unjudged_<operation>` and `validated_<operation>` -- the name each operation's message is republished under, the read of that message alone, which a dispatcher reads a request with, and the validator that message runs. Which of the two answers a check gives depends on the message's concrete type, so both halves of a service ask this one function rather than answering for themselves, from wherever they were expanded.
- `Answered<T, E>` -- the envelope a request-and-reply operation answers in, written by a dispatcher and read back by a client.
- `ExpireCreditRequest` and `SweepRequest` -- the messages declared for the operations that named none, each an ordinary `#[model_schema()]` type.

Beside the trait, the `macro_rules!` each transport the service asked for contributes -- three for `amqp_rpc`, two for `http_rest`, two for `ws_rpc`; see [Transports](#transports) -- and nothing else transport-shaped. A service that asks for no transport is emitted none of them.

#### The `amqp_rpc` walkthrough

`#[service_schema(transports = ["amqp_rpc"])]` says which transports the service wants. Each one contributes `#[macro_export]` macros named `{service_snake_case}_{transport}_dispatcher` and `{service_snake_case}_{transport}_client`, and `amqp_rpc` a third, `{service_snake_case}_{transport}_server`, all emitted at the trait's own scope, whose bodies hold that transport's halves as *tokens*: nothing inside any of them is compiled where the service is declared.

Two macros rather than one, and a third for `amqp_rpc`, because the halves of a service usually live in different crates -- a crate that calls the service can see the contract but has no business seeing the server's backend, a server crate has no use for a client, and a crate that only wants `dispatch` -- a hand-rolled adapter, or a test with no broker in reach -- has no business seeing `lapin`, `tokio` or `futures` either. `amqp_rpc` earns the third because one consumer loop over `lapin` serves every service that places it; `http_rest` and `ws_rpc` have no such fixed loop, a server there being the hosting application's own router or socket handler. Each is invoked and placed by the half that wants it, none drags in another, and a crate that wants more than one places more than one.

```rust,ignore
// In the crate that declares the service: nothing below is built here.
#[service_schema(transports = ["amqp_rpc"])]
pub trait UsageService<Ctx> { /* ... */ }
```

Each invocation goes in a module of its own **file**, and those `mod` declarations go above the crate's `use` items. A `macro_rules!` body is linted under the levels of the crate that *invokes* it, and this is the placement that measures clean: an inline `mod amqp_transport { ... }` earns `clippy::inline_modules`, a `mod` written below a `use` earns `clippy::arbitrary_source_item_ordering`, and reaching the author's own types through `use declaring_crate::*;` earns `clippy::wildcard_imports`.

```text
// In the crate that serves it -- src/lib.rs
mod amqp_transport;

// src/amqp_transport.rs
declaring_crate::usage_service_amqp_rpc_dispatcher!();
```

```text
// In a crate that calls it -- src/lib.rs: no backend, no bus library.
mod amqp_client;

// src/amqp_client.rs
use declaring_crate::{AvailableBalanceRequest, AvailableBalanceResponse};

declaring_crate::usage_service_amqp_rpc_client!();
```

```text
// In the crate that serves it for real, over a real `lapin::Channel` -- src/lib.rs
mod amqp;

// src/amqp.rs
declaring_crate::usage_service_amqp_rpc_server!();
```

Each macro takes no arguments and emits bare items rather than a module of its own, so the caller names the module and two transports in one crate cannot collide. The dispatcher emits `IncomingMessage` -- built with `IncomingMessage::new(operation, payload, headers)` by whichever crate owns the bus and read through `operation()`, `payload()` and, where the service claims at least one `header_in` binding, `headers()`; `headers` carries the request's own headers table, and a service claiming none is left with the earlier two-field shape and no accessor for a field nothing would read -- `Reply`, the handle a transport implements to answer one message, with `send(value, headers)` and `fault`, and `dispatch(svc, ctx, message, reply)`, generic over the implementing type. Beside them go the panic guard an arm calls its implementation behind and the reader that classifies a refused payload, both emitted only where the service declares an operation for them to serve. The client emits:

- `Transport` -- the seam a client is bound to, with `notify(operation, payload, headers)` and `request(operation, payload, headers)`, `request` answering the encoded reply *and* the headers it carried back. Both answer a `Result`, whose failure arm carries in words what stopped a call from travelling; the client turns it into a fault of kind `transport-failure`.
- `UsageServiceClient` -- one method per operation, over any `Transport`. Each takes that operation's arguments -- one more per `header_in` binding, in declaration order -- and no context: a context is what an implementation needs, and a caller has nothing to hand one to. Every method answers a `Result`, and every method's generated documentation says under an `# Errors` heading what its failure arm can hold.
- The mirror that reads a fault back off the wire and the reader that turns one envelope into three outcomes, emitted only where the service declares an operation that answers. A service declaring only one-way operations is emitted neither, there being no reply for either to read.

**`header_in`/`header_out`/`error_header_out` ride the headers channel, not the payload.** `#[service_schema_op(http(header_in("name" = parameter), header_out("name")))]` binds an ordinary argument beside the message to a named header on the way in, and names the tuple element after the response on the way out; `error_header_out("name")` names the tuple element after the *declared error* the same way, read and written only when the error is the reply. On `amqp_rpc` that channel is the message's own basic-properties headers table: the dispatcher decodes each `header_in` value (JSON-encoded) off the incoming headers before calling the implementation, naming the header rather than a payload field if a value will not decode; a `header_out`-bound success type and an `error_header_out`-bound error type are each a tuple, and the arm sends the value alone while writing every element after it into the reply's own headers. The client mirrors both directions, encoding each `header_in` argument and decoding `request`'s returned headers back into whichever declared tuple its outcome answers. An operation naming no `http(...)` group carries an empty headers list either way -- the channel exists on every operation, but nothing reads or writes it there.

The server emits everything the dispatcher does, plus the pieces that turn a real delivery into a call on an implementation:

- `Context { pub logger: tracing::Span }` -- what the service is handed beside every message.
- `ReplyHandle` -- implements the dispatcher's own `Reply` over a `lapin::Channel`: `fault` and `send` both publish through the wire framing below; `send`'s headers are written into the outgoing `lapin::BasicProperties`' own headers table, JSON-encoded, beside whatever the transport merges in; a handle whose delivery carried no `reply_to` publishes nothing, and a publish the channel refuses is logged at error and dropped rather than propagated.
- **A declared error and a fault reply both carry the header `is_error` = `true`, an AMQP boolean field; a success reply carries none.** The payload is unchanged -- this rides the same headers table `header_out` writes into, as a boolean rather than the JSON text a `header_out` value is. `header_out("is_error")` is refused at the declaration on a service that asks for `amqp_rpc`, so a bound value never collides with it. A crate that places only `{service}_amqp_rpc_dispatcher!()` builds no `ReplyHandle` and does its own framing over `Reply` by hand, so it is untouched by this.
- **A reply's body is the payload alone, and everything else is AMQP metadata.** The body is the value, the declared error or the fault, exactly as `dispatch` wrote it -- `null` for a unit success -- with nothing added and nothing removed. Which of the three it is rides in the AMQP `type` property (`response`, `error` or `fault`), the correlation id in `correlation_id`, and `is_error` and every `header_out` value in the headers table. A transport that hands the reply to a generated client rebuilds `{ ok, value | error }` from the `type` property and the body.
- `reply`, `framed_fault` and `outgoing_headers` -- the wire framing a reply and a fault are built through: `reply` answers the `type` property and the body for one answer.
- `serve_until(channel: &lapin::Channel, queue: &str, service: &S, prefetch: NonZeroU16, shutdown: F) -> Result<Stopped, lapin::Error>` -- the consumer loop itself: declares the queue, tells the broker this consumer holds `prefetch` unacknowledged deliveries, and dispatches up to that many at once, acknowledging each when its own dispatch finishes, in whatever order they finish. The prefetch has no default: `basic.qos` reads `0` as unlimited, which no bounded loop can honour, and a fixed one would park deliveries behind a busy consumer. The deliveries run concurrently on the task awaiting `serve_until`, not in parallel, so a handler that blocks its thread holds up every other; a service that wants threads runs more than one loop, each spawned on a task of its own. Once `shutdown` completes the consumer is cancelled at once, so the broker stops pushing, no delivery is started, and every delivery already started is dispatched and acknowledged before `serve_until` returns `Stopped::ShutdownRequested`; a consumer the broker closes is drained the same way before it returns `Stopped::ConsumerClosed`. The queue is a runtime `&str`; the span the loop opens is `tracing::info_span!("amqp_service", queue, operation = operation.as_str())`. Every delivery's headers table is read into `IncomingMessage` regardless of whether the service claims any of them, `header_in` being a per-operation question the loop itself does not answer.
- `serve_deliveries(deliveries, limit, shutdown, stop, handle) -> Stopped` -- the loop `serve_until` runs, generic over the stream of deliveries, the shutdown, the action that stops the consumer and the handler one delivery is given to, so it runs with no broker at all: in a test, over `futures::stream::iter`.

A crate that invokes the server macro names all six of `lapin`, `tokio`, `futures`, `serde`, `serde_json` and `tracing` in its own manifest, the emitted consumer loop calling every one of them.

Paths inside a `macro_rules!` body resolve where the macro is *invoked*, so the two kinds are spelled apart. Everything tixschema generated is reached through `$crate::`, which resolves in the crate that declared the service; every runtime crate is reached through a leading `::` and resolves in the invoking crate, **which is therefore the one that names `serde`, `serde_json` and `tracing` in its own manifest**. That is the point of the shape: a crate that only declares services names serde alone. A crate that places only a client names `serde` and `serde_json` and not `tracing`, nothing in a client catching a panic; the `use` in the client module above is what resolves the message types the author declared, which the expansion spells exactly as they were written.

A crate that declares a service *and* places one of its halves invokes that macro by name rather than by path -- `usage_service_amqp_rpc_dispatcher!()` -- after the declaration and inside it or a module below it, `#[macro_use]` carrying it out of a submodule. Rust refuses an absolute path to a `macro_export` macro that a macro produced, from within the crate that produced it -- `crate::usage_service_amqp_rpc_dispatcher!()` and `use crate::usage_service_amqp_rpc_dispatcher;` both earn a deny-by-default future-incompatibility:

```text
error: macro-expanded `macro_export` macros from the current crate cannot be referred to by absolute paths
   = note: `#[deny(macro_expanded_macro_exports_accessed_by_absolute_paths)]` (part of `#[deny(future_incompatible)]`) on by default
```

Another crate reaches either macro by path as above.

`#[macro_export]` places each macro at the declaring crate's root whatever module it was written in, so each name has to be unique across that crate, and `$crate` reads from that same root. A service declared at the crate root needs nothing further; one declared in a submodule re-exports at the crate root its generated module and its trait, which is all either half reaches by name -- every message a dispatcher deserializes, and every message a client builds, is reached as `$crate::{service}_schema::<Operation>Message`. A message the *author* named is spelled as they wrote it where a client method takes it, and resolves in the module the macro was invoked in.

Both of those the declaring crate checks for itself. A service that asked for at least one transport resolves its generated module and its trait against the crate root at the declaration, so leaving either unreachable fails **that** crate's own `cargo build` rather than every crate that goes on to invoke a macro. The caret sits on the trait, with the attribute macro named as the origin:

```text
error[E0433]: cannot find `usage_service_schema` in `crate`
  --> src/services/usage.rs:15:11
   |
14 | #[service_schema(transports = ["amqp_rpc"])]
   | -------------------------------------------- in this attribute macro expansion
15 | pub trait UsageService<Ctx> {
   |           ^^^^^^^^^^^^ unresolved import

error[E0405]: cannot find trait `UsageService` in the crate root
```

Each is earned on its own, so re-exporting one leaves the other standing rather than repeating both. Write them as **`pub use`**, not `use`: a private import at the root satisfies an invocation inside the declaring crate -- which is what this crate's own test harnesses rely on -- while a crate invoking the macro from outside earns ``error[E0603]: module `usage_service_schema` is private``. A service that asked for no transport publishes no macro, is checked against no root, and compiles below the crate root with nothing re-exported.

On the TypeScript side, three artifacts, reached through a unit struct named `<Service>Schema`:

```rust
pub fn get_entities() -> (String, Vec<String>) {
    (
        "service entities".to_string(),
        vec![
            UsageServiceSchema::ts_definition(),
            UsageServiceSchema::ts_client(),
            UsageServiceSchema::ts_service(),
        ],
    )
}
```

`ts_definition()` answers with every message the macro declared for the service, the fault type and the kind it reports, and one result type per operation that answers -- **so a generated message needs no registration line of its own.** The author never wrote those types and has no reason to know their names; a forgotten line would leave a Rust-only message and a client unable to call the operation.

`ts_client()` answers with the transport seam, the client type and the factory that binds one:

```typescript
export type UsageServiceClient = {
  /**
   * Sends `apply-bundle` on `UsageService`, which expects no reply.
   *
   * @throws {UsageServiceRefusal} when the message fails its own schema. The operation
   * answers `Promise<void>`, so there is no failure arm to put the fault in; the transport
   * is still never reached.
   */
  applyBundle(req: AvailableBalanceRequest): Promise<void>;
  /** Calls `usage-generation-request` on `UsageService` and waits for the answer. */
  canGenerate(req: AvailableBalanceRequest): Promise<UsageServiceCanGenerateResult>;
  /** Calls `expire-credit` on `UsageService` and waits for the answer. */
  expireCredit(req: ExpireCreditRequest): Promise<UsageServiceExpireCreditResult>;
  /** Calls `get-available-balance` on `UsageService` and waits for the answer. */
  getAvailableBalance(req: AvailableBalanceRequest): Promise<UsageServiceGetAvailableBalanceResult>;
  /** Calls `sweep` on `UsageService` and waits for the answer. */
  sweep(req: SweepRequest): Promise<UsageServiceSweepResult>;
};
```

`ts_service()` answers with the interface an implementation satisfies in full, the outcome types it answers with, and the dispatcher factory:

```typescript
export interface UsageServiceImpl<Ctx> {
  /** Handles `apply-bundle` on `UsageService`, which expects no reply. */
  applyBundle(ctx: Ctx, req: AvailableBalanceRequest): Promise<void>;
  /** Handles `usage-generation-request` on `UsageService` and answers it. */
  canGenerate(ctx: Ctx, req: AvailableBalanceRequest): Promise<UsageServiceCanGenerateOutcome>;
  /** Handles `expire-credit` on `UsageService` and answers it. */
  expireCredit(ctx: Ctx, req: ExpireCreditRequest): Promise<UsageServiceExpireCreditOutcome>;
  /** Handles `get-available-balance` on `UsageService` and answers it. */
  getAvailableBalance(ctx: Ctx, req: AvailableBalanceRequest): Promise<UsageServiceGetAvailableBalanceOutcome>;
  /** Handles `sweep` on `UsageService` and answers it. */
  sweep(ctx: Ctx, req: SweepRequest): Promise<UsageServiceSweepOutcome>;
}
```

```typescript
export function createUsageServiceDispatcher<Ctx>(
  impl: UsageServiceImpl<Ctx>,
): (
  ctx: Ctx,
  operation: string,
  payload: unknown,
  headers?: ReadonlyArray<readonly [string, string]>,
  parts?: ReadonlyArray<readonly [string, unknown]>,
) => Promise<UsageServiceDispatched | undefined> {
```

Every member is required, so an implementation missing one is refused where it reaches `createUsageServiceDispatcher`. Every emitted name carries the service -- `UsageServiceFault`, `UsageServiceGetAvailableBalanceResult`, `UsageServiceClient`, and the fault's own brand symbol `usageServiceFaultSeal` -- because TypeScript has no per-service scope and a bundle is one flat file. Rust needs no such prefix, the generated module being the scope TypeScript lacks.

**Headers cross the TypeScript seam both ways, each value JSON-encoded.** The client's transport seam carries them beside the payload, and `request` answers the reply's own headers beside the envelope:

```typescript
export type UsageServiceTransport = {
  notify(
    operation: string,
    payload: unknown,
    headers: ReadonlyArray<readonly [string, string]>,
  ): Promise<void>;
  request<Answered>(
    operation: string,
    payload: unknown,
    headers: ReadonlyArray<readonly [string, string]>,
  ): Promise<{ answered: Answered; headers: ReadonlyArray<readonly [string, string]> }>;
};
```

The header text is the JSON encoding the Rust `amqp_rpc` transport writes into the AMQP headers table, so an AMQP transport hands the pairs over as they are. A transport written against the earlier seam -- `request` answering the reply alone -- answers `{ answered, headers }` now, with an empty list where the reply carried none. A client method takes one argument per `header_in` binding after the message and sends each JSON-encoded, an optional one holding `undefined` as `null`, which is what the Rust client writes for a `None`. A `header_out` or `error_header_out` reply is rejoined into the tuple the operation declared: the body off the envelope, each header off the reply's headers, checked against the element's own schema. An absent optional header reads as `null`, the value its tuple slot holds; a required one that is absent, is not JSON, or does not match its schema answers a `failed-validation` fault naming the header.

The dispatcher reads each `header_in` value off the same JSON text, checked against the argument's own schema, and refuses a missing required one or a malformed one through the same framed fault a bad payload gets. It answers `UsageServiceDispatched` -- `{ answered, headers }`, the envelope that goes on the wire and the headers written beside it -- or `undefined` for a one-way operation that ran. A header tuple is split there: the body answers as the envelope's `value` or `error`, and each header element is written under its name, one holding `null` written nowhere. A transport that drove the dispatcher before writes `dispatched.answered` where it wrote the whole answer, and `dispatched.headers` into its own headers channel.

Every reader ignores `value` for a `Result<(), E>` operation, and for a `Result<T, E>` operation whose declared success `T` is a unit struct declared above the service. `ts_client()`'s result type reads `{ ok: true; value: undefined }`, and `ts_service()`'s outcome type reads `{ ok: true }` with no `value` member at all.

#### Transports

tixschema generates no transport of its own -- what it emits is the seam either side of one -- with one exception: the `amqp_rpc` transport's server macro also emits a real consumer loop over `lapin`, since AMQP's own request-and-reply shape is fixed enough that the loop is the same for every service that places it. The seam below is what every transport, `amqp_rpc` included, is built on:

- **The operation name arrives from the transport**, beside the payload and never inside it, so no message type has to reserve a key for routing. `dispatch` is handed the name and matches on it; a gRPC method name or an HTTP path serves the same role on those transports.
- **The encoding lives behind `Reply`.** `Reply::send(value, headers)` is handed a value rather than bytes, because a transport merges its own fields -- a correlation id, an error flag -- into the object before serializing it, and neither is reachable behind an encoded buffer.
- **Queues, retries, timeouts, acknowledgement, connection handling and reconnection belong to the caller.** `dispatch` returns nothing, so the adapter that called it still holds the delivery when the arm finishes, and acknowledges there -- for every message, including a one-way one. That placement is what lets the handle carry replying and nothing else: a path that never replies names nothing about replying.
- **A call that never landed has a place to be reported.** `Transport::notify` and `Transport::request` both answer a `Result`, and `Err` carries whatever the transport wants to say about a message that did not go out or a reply that is not coming. Without it a transport that hit its own deadline could only panic or hang, and a caller would be left holding a call that never completes and no fault to report. Whether there is a deadline, and how long it is, stays the transport's decision -- the arm is where the answer is reported, not where it is decided.
- **A header-shaped channel rides beside the payload on both legs.** `notify`/`request` take a `headers: Vec<(String, String)>` beside the payload -- one JSON-encoded entry per `header_in` binding, empty for an operation that claims none -- and `request`'s answer carries the reply's own headers back the same way, which is where a `header_out` value is read from. On `amqp_rpc` this is the AMQP message's own basic-properties headers table; another transport maps it onto whatever header-shaped channel it has.

A request-and-reply arm answers exactly once, through `send` or `fault`, whichever way it goes. A one-way arm that reached its implementation calls neither; one whose payload was refused before it got there answers with a fault, that being the only thing it can say about a message it never ran.

#### The `http_rest` Transport

`#[service_schema(transports = ["http_rest"])]` asks for a real REST surface instead of (or beside) `amqp_rpc`: owner-declared methods, paths, statuses and header bindings, served through whatever HTTP stack the hosting application already uses. It contributes **two** macros rather than three -- `{service_snake_case}_http_rest_dispatcher!()` and `{service_snake_case}_http_rest_client!()` -- because there is no third: `amqp_rpc`'s server macro exists because AMQP's own request-and-reply shape is fixed enough that one consumer loop over `lapin` serves every service that places it, and no such fixed loop exists for HTTP. What an HTTP server looks like is the hosting application's own router, so `http_rest` stops at the dispatcher and leaves registering routes on a real listener to a small hand-written adapter -- see "Adapters carry no tixschema runtime crate" below.

There is no envelope on this transport. `amqp_rpc` answers every reply through `{ ok, value?, error? }`; `http_rest` answers a success with the declared `ok_status` and the success type serialized directly as the body, a declared error with its mapped status and the error type itself as the body, and a fault -- a defect no operation declared -- through the fault catch-all described below. The status line carries what the `ok` flag carries on the bus.

**The `http(...)` grammar.** Every grammar arm at once, on one operation:

```rust,ignore
#[service_schema(transports = ["http_rest"])]
pub trait DocumentService<Ctx> {
    #[service_schema_op(http(
        method = "GET",
        path = "/documents/{document_id}/versions/{version_id}",
        ok_status = 200,
        header_in("range" = byte_range),
        header_out("etag"),
        error_status(NotFound = 404, VersionGone = 410),
    ))]
    async fn get_version(
        &self,
        ctx: &Ctx,
        req: GetVersionRequest,
        byte_range: Option<String>,
    ) -> Result<(VersionResponse, String), GetVersionError>;
}
```

`method` is one of `"GET"`, `"POST"`, `"PUT"`, `"DELETE"`, `"PATCH"`; `path` is a template walked left to right, `{field}` placeholders included; `ok_status` is the success status (default 200, or 204 where the reply carries nothing -- see below); `error_status(Variant = code, ...)` maps the operation's own declared error variants to the statuses they answer at; `header_in`/`header_out`/`error_header_out` bind request and response headers (covered on its own below); `body` picks the body kind (`"json"`, the default; `"bytes"`; `"multipart"`; or `"stream"` -- covered on its own below); and `part("name" = parameter)` claims one multipart file part, only meaningful under `body = "multipart"`. Writing an argument this grammar does not recognise is refused naming the arguments it does:

```text
service_schema: unknown `http` argument
       the arguments are `method`, `path`, `ok_status`, `error_status`, `header_in`, `header_out`, `error_header_out`, `part` and `body`
```

**An operation naming no `http(...)` group at all** still gets one, defaulted rather than left unhandled: `POST /{wire-name}`, the same `ok_status` default every annotated operation gets (200, or 204 for nothing to serialize), no header bindings, and every declared error answered at one fixed status, `422`, rather than a per-variant table -- there being no annotation to read one from. So an internal service pays no attribute cost and still answers real HTTP:

```rust,ignore
    // Names no `http(...)` group: defaults to `POST /sweep-documents`, 200 on success,
    // every declared error of `SweepError` at 422.
    async fn sweep_documents(&self, ctx: &Ctx) -> Result<SweepReport, SweepError>;
```

**Path placeholders and the query string.** A `{field}` placeholder binds a same-named field on the operation's own message; naming one the message does not have is refused on the placeholder:

```text
service_schema: operation `get_widget`'s path names `{item_id}`, and its message has no field named `item_id`
              a path placeholder binds a same-named field on the message
```

Every client fills a placeholder the way the Rust client does, through the value's own codec: a string as itself, a number or boolean as its text, and a newtype such as `MediaId("abc123")` as its wire value, `/media/abc123`. That includes a field of an author's own named message, whose type the macro never sees: the Dart, Kotlin and Swift clients render it at runtime through a generated helper.

`GET` and `DELETE` carry no request body, so a required field the path did not bind has nowhere left to go and is refused on the field rather than silently dropped:

```text
service_schema: operation `get_widget`'s field `filter` is required and is bound by no path placeholder
              `GET` carries no body, so a required field must appear in the path
```

An *optional* field a bodyless method leaves unbound becomes a query parameter instead, read out of the parsed query string on the way in and appended to it on the way out. A path placeholder, a query parameter and a `header_in` value are all read the same way, off the operation argument's own declared type: `true`/`false` become a JSON boolean, anything that parses as a number becomes a JSON number, anything else stays a JSON string, and a `Vec<T>` splits its raw text on `,` and coerces each piece the same way -- the dispatcher and the TypeScript client apply this judgement identically, from one shared rule, so the two sides of the wire cannot read a query string two different ways.

A single-argument message -- the whole message already being the author's own type, or the argument list collapsed to one -- gives this macro no fields to read off the query string, so a bodyless method refuses it outright, naming what to write instead:

```text
service_schema: operation `download` takes its whole message as the one argument `req` on a `GET`
              a `GET` carries no body, and the path does not bind `req`; this macro cannot see its fields to read them off the query string
              declare each field as its own argument, so the message is generated and every field is read by its type
```

The one exception is a scalar bound whole by the path's one placeholder of its own name -- `get(&self, ctx: &Ctx, id: String)` on `/documents/{id}` stays legal, its value traveling in the path rather than the query.

**`header_in`/`header_out` on `http_rest` are the same grammar already introduced above for `amqp_rpc`'s headers-table channel, realized over real HTTP request and response headers instead.** `header_in("range" = byte_range)` binds the request's own `range` header to an ordinary argument beside the message; naming a parameter the operation does not take is refused on the parameter:

```text
service_schema: operation `get_widget` binds header "range" to a parameter named `byte_range`, and `get_widget` takes no argument by that name
              `header_in` binds one ordinary argument beside the message; name it in the signature, or remove the binding
```

`header_out("etag")` makes the operation's success type a tuple: the body first, then one element per declared `header_out` entry, in declaration order, each written out as a response header and read back the same way by the client. A tuple success type with no `header_out` to explain it is refused naming the requirement, and so is a `header_out` whose count does not match the tuple's own arity:

```text
service_schema: operation `get_widget` returns a tuple success type and declares no `header_out`
              name what each element after the first is with `header_out("name")`, or return the type directly
```

`error_header_out("name")` is the same tuple-splitting rule applied to the *declared error* instead of the success type: the error first, then one element per declared `error_header_out` entry, in declaration order, each written out as a response header when the error is the reply and read back the same way by the client. A tuple error type with no `error_header_out` to explain it, an `error_header_out` whose count does not match the tuple's own arity, and `error_header_out` on a `one_way` operation (which has no declared error to carry one in) are each refused naming the requirement:

```text
service_schema: operation `check_range` returns a tuple error type and declares no `error_header_out`
              name what each element after the first is with `error_header_out("name")`, or return the type directly
```

**Header names and values are checked, not trusted.** Across `header_in`, `header_out` and `error_header_out` together, on one operation: a header name must be a legal HTTP token (letters, digits and `` !#$%&'*+-.^_`|~ ``, RFC 9110's own `tchar` set); a name the transport already writes itself -- `content-type`, `content-length`, and `content-range` in `header_in` or `header_out` on a `body = "stream"` operation -- cannot be claimed by any of the three directives; and within any one of `header_in`, `header_out` or `error_header_out`, the same name cannot be declared twice -- the same name declared in two *different* lists, an echoed `x-request-id` read by `header_in` and written back by `header_out`, is legal. All three are refused at compile time, on the operation:

```text
service_schema: operation `get_widget` declares the header name "content-type", which `json_response`, and a `body = "bytes"` or `body = "stream"` reply's own content type, writes itself
              name a header this transport does not already control
```

A header *value* is checked at runtime instead, on every HTTP writer: `header_in`, `header_out` and `error_header_out` values are each rejected -- a fault on the server, a refusal before sending on the client -- unless every byte is visible ASCII (`0x21`-`0x7E`), a space, or a tab, which rules out a value carrying a line break or another control character before it ever reaches the wire.

Every REST client reads a `header_out` or `error_header_out` element back the way the Rust client does. An absent optional element reads as the language's own null. An absent required one answers the `undeserializable-payload` fault `a declared response header was missing`. A declared type -- a newtype such as `MediaId`, or an enum -- is read as a JSON string through its own codec, as the Rust client reads it. A present one that does not decode as its declared type -- a number that does not parse, a boolean other than `true`/`false`, a value its type's codec refuses, any piece of a list -- answers the same fault with `a response header did not match its declared type`, never an exception or a default value.

**Statuses.** `ok_status` and `error_status` are owner-chosen, exactly as `amqp_rpc`'s own message contract stays HTTP-free -- neither lives on a type, only on the operation. `error_status`'s completeness against the operation's own declared error type is checked unconditionally, by a `match` over exactly the declared arms and no wildcard: a variant the table leaves out is refused with rustc's own exhaustiveness check, naming it, and the refusal is spanned on the trait's own `#[service_schema]` attribute rather than invented by this crate:

```text
error[E0004]: non-exhaustive patterns: `&WidgetError::Gone` not covered
```

An operation whose reply carries nothing to serialize -- `one_way`, or a declared reply of `()` -- answers a bodyless status: 204 by default, or whatever `ok_status` the owner wrote (202 Accepted for queued work, say). Nothing about the *shape* of that case is special: the dispatcher decodes, validates and calls the implementation exactly as any other arm does, and only the answer has nothing in it.

**Body kinds.** `body = "json"` is the default: the message is the whole body on a bodied method, and the success/error types are the whole JSON body coming back. `body = "bytes"` says the response is not JSON: the reply's success type must be exactly `(Vec<u8>, String)` -- the raw bytes and their content type -- plus one further tuple element per declared `header_out` entry, in that order. A reply that still claims a JSON success type is refused naming the required shape:

```text
service_schema: `body = "bytes"` requires a success type of `(Vec<u8>, String)`
              the operation's signature still claims a JSON success type - answer `Result<(Vec<u8>, String), Error>`, the bytes and their content type
```

`body = "stream"` answers an undrained body instead of a materialized one: the reply's success type must name `StreamedAnswer` -- the seam type `#[service_schema]` publishes beside the trait wherever any operation needs it -- again plus one tuple element per `header_out` entry:

```rust,ignore
async fn get_content(
    &self,
    ctx: &Ctx,
    document_id: String,
    byte_range: Option<String>,
) -> Result<content_service_schema::StreamedAnswer, ContentError>;
```

`StreamedAnswer::Full { source, content_type }` answers the declared `ok_status` with the whole body; `StreamedAnswer::Partial { source, content_range, content_type }` answers `206` with `content-range` set to what the handler built (`bytes {start}-{end}/{total}`, RFC 9110 section 14.4), the declared headers (if any) riding beside it rather than being replaced by it. Both write `content_type` as the `content-type` header, checked like any other header value, as a `body = "bytes"` reply writes the second element of its pair; every client reads it back into the answer. A declared error never gets a `content-range` from the transport, so `error_header_out("content-range")` is how a `416` answers `bytes */<size>`, the complete length a client reads to learn the object's size (RFC 9110 section 15.5.17). `source` is a `Box<dyn BodySource + Send>` -- `BodySource::pull(&mut self, buf) -> io::Result<usize>` pulled one chunk at a time, blanket-implemented for every `std::io::Read` so a `File` or any other reader an author already has satisfies it for free, and named through no runtime crate beyond `std::io`. The dispatcher never buffers a streamed body whole; it hands `OutgoingBody::Stream(source)` to the adapter, which drains it onto the wire a chunk at a time.

`body = "multipart"` reads the *request* as named parts instead of one JSON object -- it says nothing about the response, whose success and error types stay ordinary JSON, `header_out` included, exactly like `body = "json"`. A scalar field is read straight off the same-named part, the same way a bodyless method's field is read off the query string; a `part("name" = parameter)` binding claims one file part and hands it through as the same `BodySource` handle a streamed response pulls from, undecoded -- a message field never carries one, `BodySource` publishing no `Deserialize` for it to fall back to. `GET` and `DELETE` carry no request body for parts to travel in, so `body = "multipart"` on either is refused naming the method:

```text
service_schema: operation `upload_document` declares `body = "multipart"` on a `GET`
              `GET` carries no request body for parts to travel in; write `POST`, `PUT` or `PATCH`
```

and a `part(...)` naming a parameter the operation does not take is refused exactly as an unclaimed `header_in` is:

```text
service_schema: operation `upload_document` binds part "file" to a parameter named `attachment`, and `upload_document` takes no argument by that name
              `part` binds one ordinary argument beside the message; name it in the signature, or remove the binding
```

A single-argument message under `body = "multipart"` is refused the same way, struct or lone scalar alike -- this macro cannot see its fields to send or read as parts:

```text
service_schema: operation `upload` takes its whole message as the one argument `req` under `body = "multipart"`
              a multipart request carries its fields as text parts, and this macro cannot see `req`'s fields to write or read them
              declare each field as its own argument beside the `part(...)` bindings
```

Verified end to end, no server involved: a multipart upload dispatched by hand decodes its scalar parts into the generated message and hands the file part through as an undrained `BodySource`, draining to exactly the bytes it was built with, before `validate()` runs; a streamed download answers `200` with the whole body pulled across more than one `BodySource::pull()` call for a body larger than the reader's own chunk cap, and a `range` header answers `206` with `content-range` and the sliced body, the declared `header_out` riding beside either arm.

**What the dispatcher macro emits.** `IncomingRequest` -- one plain-terms request, built with `IncomingRequest::new(method, path, query, headers, body)` by whichever adapter owns the socket, and read back through `method()`, `path()`, `query()` (the query string, unparsed, `dispatch` reading it itself), `header(name)` (case-insensitive) and `headers()`. `OutgoingResponse` -- `status()`, `headers()`, `body()` (or, on a service with at least one streamed operation, `into_body()` answering an `OutgoingBody` of `Bytes(Vec<u8>)` or `Stream(Box<dyn BodySource + Send>)` instead of a bare `body()`, since materializing every answer into bytes is exactly what a streamed body must not do). `Route`/`ROUTES` -- every operation's method, path template (`{field}` placeholders written exactly as declared), `ok_status()` and `error_statuses()`, for an adapter to iterate when it registers one handler per route. `FaultHandler` -- the fault catch-all, covered next -- and `dispatch(service, ctx, request, [parts,] handler) -> impl Future<Output = OutgoingResponse>`, generic over the implementing type (a trait with `async` methods is not `dyn`-compatible), which matches the method and path itself, decodes path, query, headers and body into the operation's own arguments, validates, calls the implementation behind the same panic guard every transport uses, and answers through `FaultHandler` for anything the operation itself did not declare. A service with at least one `body = "multipart"` operation gives `dispatch` one further argument, `parts: Vec<(String, IncomingPart)>` -- `IncomingPart::Text(String)` or `IncomingPart::File(Box<dyn BodySource + Send>)`, one entry per part the adapter already split off the wire.

**What the client macro emits.** `OutgoingRequest` -- `method()`, `path()` (placeholders already filled in), `query()`, `headers()`, `body()`, plus `into_parts()` on a service with a multipart operation, answering the `Vec<(String, OutgoingPart)>` a `body = "multipart"` method built (`OutgoingPart::Text`/`OutgoingPart::File`, the file variant carrying an owned `BodySource` handle). `IncomingResponse` -- `status()`, `header(name)`, `body()` (or `into_body()` answering `IncomingBody::Bytes`/`IncomingBody::Stream` where the service streams). `Transport` -- what an adapter implements: one method, `send(&self, request: OutgoingRequest) -> impl Future<Output = Result<IncomingResponse, String>> + Send`, sending a whole request and answering with a whole response or with what stopped it in words. `{Service}Client<T: Transport>` -- `new(transport)`, `transport()`, and one method per operation, over any `T: Transport + Sync`, each building the request from the operation's own validated message (path, query, headers filled from it) and decoding the answer by status exactly as the dispatcher answers it: the declared `ok_status` into the success type (or tuple, `header_out` elements read back off response headers; or the bytes and content type for `body = "bytes"`; or a `StreamedAnswer` pulling from the response body for `body = "stream"`, `Full` at `200`, `Partial` with `content-range` at `206`, both with their `content-type`), a mapped status into the declared error, and anything else -- including a fixed fault status this build did not expect at that code -- into a fault.

**Placement follows the same doctrine as `amqp_rpc`'s three macros** -- see "Where each goes, and why placement is fussy" above: each invocation in a module of its own file, the `mod` declaration above the crate's `use` items, invoked by name (not by path) from inside the declaring crate:

```text
// the crate that serves it -- src/lib.rs
mod http_transport;

// src/http_transport.rs
declaring_crate::document_service_http_rest_dispatcher!();
```

```text
// a crate that calls it -- src/lib.rs
mod http_client;

// src/http_client.rs
use declaring_crate::{GetVersionRequest, VersionResponse};

declaring_crate::document_service_http_rest_client!();
```

The invoking crate names its own subset of the runtime crates the macro's body reaches through `::`: the dispatcher's crate names `serde`, `serde_json` and `tracing` (the panic guard logs through it); the client's crate names `serde` and `serde_json` and not `tracing` -- nothing in a client catches a panic.

**The fault catch-all.** A payload that will not deserialize or fails `validate()`, a request no route answers to, and a handler that panicked are none of the things an operation declared it could fail at, so they answer through `FaultHandler` rather than through any declared error:

```rust,ignore
pub trait FaultHandler {
    fn on_fault(&self, fault: &ServiceFault) -> OutgoingResponse {
        // the provided default
    }
}

pub struct DefaultFaultHandler;
impl FaultHandler for DefaultFaultHandler {}
```

The provided default answers 404 for `ServiceFaultKind::UnknownOperation`, 500 for `ServiceFaultKind::HandlerPanic`, and 400 for everything else (an undeserializable payload, a validation failure), each as the fault itself serialized under `json_response`. `dispatch` takes a `&H: FaultHandler` argument, so an owner installs their own by implementing the trait on a type of their own and handing it to `dispatch` in place of `DefaultFaultHandler`; overriding `on_fault` replaces the default for every kind at once, `fault.kind()` being how an override tells one kind from another:

```rust,ignore
struct RecordingFaultHandler;

impl FaultHandler for RecordingFaultHandler {
    fn on_fault(&self, fault: &ServiceFault) -> OutgoingResponse {
        let status = match fault.kind() {
            ServiceFaultKind::UnknownOperation => 490,
            ServiceFaultKind::FailedValidation | ServiceFaultKind::UndeserializablePayload => 491,
            ServiceFaultKind::HandlerPanic => 492,
            ServiceFaultKind::TransportFailure => 493,
        };
        OutgoingResponse::new(
            status,
            vec![("x-fault-kind".to_owned(), format!("{}", fault.kind()))],
            format!("recorded: {}", fault.detail()).into_bytes(),
        )
    }
}
```

A `dispatch` built on `RecordingFaultHandler` answers a request to an unmatched route with status 490 and an `x-fault-kind: unknown operation` header instead of the default's 404 -- nothing about the numbers above is the contract; the seam is.

**Adapters carry no tixschema runtime crate.** `IncomingRequest`, `OutgoingResponse`, `Route`/`ROUTES`, `FaultHandler` and `dispatch` on one side, and `OutgoingRequest`, `IncomingResponse` and `Transport` on the other, are plain terms: a method, a path, headers and a body in, a status, headers and a body out. Nothing generated here names Axum, reqwest or fetch. A server adapter is a small hand-written loop, living with the hosting application, that iterates `ROUTES` to register one handler per route on its own router, and inside each handler builds an `IncomingRequest` from whatever its framework handed it, calls `dispatch`, and writes the answered `OutgoingResponse` back exactly as its own framework answers a request -- the same shape this crate's own dual-transport test harness proves by joining a client straight back to a dispatcher with no socket at all:

```rust,ignore
impl<H: FaultHandler + Sync> http_rest_client::Transport for HttpLoop<'_, H> {
    async fn send(
        &self,
        request: http_rest_client::OutgoingRequest,
    ) -> Result<http_rest_client::IncomingResponse, String> {
        let incoming = http_rest_transport::IncomingRequest::new(
            request.method().to_owned(),
            request.path().to_owned(),
            request.query().to_owned(),
            request.headers().to_vec(),
            request.body().to_vec(),
        );
        let answered = http_rest_transport::dispatch(self.service, &(), &incoming, &self.handler).await;
        Ok(http_rest_client::IncomingResponse::new(
            answered.status(),
            answered.headers().to_vec(),
            answered.body().to_vec(),
        ))
    }
}
```

A client adapter is the mirror: a small hand-written `Transport` implementation over a real HTTP stack (reqwest in Rust; a service-agnostic `fetch` helper in TypeScript; a service-agnostic `send` implementation in Dart), living with whichever codebase calls the service, doing exactly one job -- carry the plain-terms request across a real connection and hand the plain-terms response back. Timeouts, cancellation, retries, connection handling, mutual TLS and authentication are that adapter's own concerns, never the generated seam's, exactly as `amqp_rpc`'s own adapter split already works: the generator emits the seam, the application implements it against its own libraries, and a workspace that wants to share one adapter across services keeps it in its own crate under its own name -- never a `tixschema`-branded runtime dependency.

**The TypeScript REST server.** `<Service>Schema::ts_http_service()` emits the route table, the plain-terms request and response shapes, a fault handler with the Rust defaults, and `create{Service}HttpDispatcher(impl, onFault?)` -- the TypeScript twin of `ROUTES`, `IncomingRequest`/`OutgoingResponse`, `FaultHandler` and `dispatch` above, doing its own method and path matching, placeholder and query coercion, and message assembly rather than taking an already-matched operation name. It names no framework: the adapter that binds it to a real listener is the hosting application's, exactly as the client adapter above is.

Each bound `header_in` and `part(...)` arrives at the TypeScript implementation as its own argument after the message, in declaration order, the way the Rust trait method already receives it -- `create{Service}Dispatcher` looks it up, decodes it and refuses a missing required one through the same framed fault a bad payload gets, before the implementation is ever called, so an argument read there is as trustworthy as the message is. The REST server coerces each `header_in` value from its header text the way a query parameter is coerced, and hands it to the dispatcher JSON-encoded; a `header_out` or `error_header_out` value comes back from the dispatcher JSON-encoded and is written as header text again. `get_version`'s `byte_range` above reaches its TypeScript implementation this way:

```typescript
// not compiled here
export interface DocumentServiceImpl<Ctx> {
  getVersion(
    ctx: Ctx,
    req: GetVersionRequest,
    byteRange: string | undefined,
  ): Promise<DocumentServiceGetVersionOutcome>;
}
```

The REST server itself keeps no presence check of its own: it passes each `header_in` value it found and, on a service with at least one multipart operation, the request's own `parts` through to the dispatcher, exactly as it passes the assembled message.

<!-- read from tests/service_schema_emitted_client_tests/run_node_http_service.rs, the Node `http` adapter driving the emitted dispatcher -->
```typescript
const dispatch = createConversationClientServiceHttpDispatcher(impl);
const server = createServer(async (req, res) => {
  const chunks = [];
  for await (const chunk of req) chunks.push(chunk);
  const url = new URL(req.url ?? "/", "http://localhost");
  const answered = await dispatch(
    {},
    {
      body: new Uint8Array(Buffer.concat(chunks)),
      headers: Object.entries(req.headers).map(([name, value]) => [name, Array.isArray(value) ? value.join(",") : String(value ?? "")]),
      method: req.method ?? "GET",
      path: url.pathname,
      query: url.search.startsWith("?") ? url.search.slice(1) : url.search,
    },
  );
  res.writeHead(answered.status, Object.fromEntries(answered.headers));
  res.end(Buffer.from(answered.body));
});
```

The same dispatcher runs unchanged behind another web framework -- Express, as a catch-all handler that leaves the matching to the dispatcher:

```typescript
app.use(express.raw({ limit: "8mb", type: () => true }));
app.all(/.*/, async (req, res) => {
  writeResponse(res, await dispatch({ requestId: ++requestCounter }, toRequest(req)));
});
```

Run against seven requests -- a matched call, a declared error, a handler that throws, a one-way call, and three that match no route (a wrong path, a wrong method on a known path, an empty placeholder segment) -- both adapters answered identically: 200 with the success value as the bare JSON body, 404 with the declared error's own body, 500 with a `handler-panic` fault, 204 with nothing, and 404 with an `unknown-operation` fault for each of the last three. What decides each status:

| what answers | who decides it |
| --- | --- |
| the success status | the operation's own `ok_status`, else 204 for nothing to serialize and 200 otherwise |
| a declared error's status | `error_status(Variant = code)`, else 422 for an operation that declares no table |
| a fault's status | 404 unknown route, 400 refused payload, 500 handler threw -- replaceable through the dispatcher's own `onFault` argument |

**The TypeScript, Dart, Swift and Kotlin clients.** `<Service>Schema::ts_http_client()` publishes the `http_rest` half beside `ts_client()`'s AMQP-shaped one: a service-agnostic `{Service}HttpTransport` seam (`send(request): Promise<response>`, both the request and the response carrying `method`/`path`/`query`/`headers`/`body` as plain strings, plus `parts` on the request where the service declares a multipart operation and `bodyStream: ReadableStream<Uint8Array>` on the response where the service declares a streamed operation -- the platform's own stream type, never a naming of `fetch`), the `{Service}HttpClient` interface, and `create{Service}HttpClient(transport)`. It needs the `zod` feature exactly as `ts_client()` and `ts_service()` do -- outbound validation before a byte goes out is what a `safeParse` against the message's own `$Schema` gives it, and a build without Zod cannot write that check truthfully, so it publishes none of the three rather than one without it. What comes back is parsed the same way: a success body and a declared error body are each read through their own `$Schema` rather than cast, and a body that is no JSON or not the declared type answers the `undeserializable-payload` fault, as every other REST client answers a body that will not decode. `<Service>Schema::dart_http_client()` is the Dart sibling -- the same seam and per-operation client, over the `dart` feature's own generated types and JSON codec rather than Zod, needing no separate outbound check because a Dart message is a real class with `required` constructor parameters and cannot be built malformed in the first place. A message, success or declared error that is not a generated class -- a `String`, an `i64`, a `Vec<T>`, a plain enum -- crosses both Dart clients as its own JSON value, the way a model field of that type does. Unlike the TypeScript seam, the Dart request and response records carry `parts` and `bodyStream` on every service whatever it declares: Dart records are structural, so one shape is what lets one implementation satisfy every service's interface. Where TypeScript answers a reply with an `{ ok, value | error }` union, Dart answers a returned `{Service}{Operation}Result` instead: `<Service>Schema::dart_definition()` publishes every message, fault type and result pair a service needs beside `dart_http_client()`, one sealed class per reply operation carrying three members -- `{Service}{Operation}ResultOk(value)` for the success, `{Service}{Operation}ResultOperation(error)` for the declared error, `{Service}{Operation}ResultFault(fault)` for a fault the operation never declared -- so a reply method answers `Future<{Service}{Operation}Result>` and a caller reads the outcome through an exhaustive `switch` rather than a catch block:

```dart
// not compiled here
final result = await client.window(WindowRequest(conversation_id: '652f1a3b4c5d6e7f8a9b0c1d'));
switch (result) {
  case ConversationClientServiceWindowResultOk(:final value):
    print(value.items);
  case ConversationClientServiceWindowResultOperation(:final error):
    print('declared: $error');
  case ConversationClientServiceWindowResultFault(:final fault):
    print('fault ${fault.kind} in ${fault.operation}');
}
```

A one-way method still answers `Future<void>` and throws the fault-only `{Service}HttpRefusal` -- Dart's own idiom for a `Future`, mirroring exactly how its own one-way AMQP methods already throw, and having no reply arm to carry a fault through instead.

`<Service>Schema::swift_http_client()` is the Swift sibling, over the `swift` feature's own generated `Codable` types -- needing no separate outbound check for the same reason Dart needs none, a Swift message being a `Codable` struct with non-optional stored properties for every required field. A reply method answers Swift's own `Result<Success, Failure>` rather than a returned pair, `Failure` being `{Service}{Operation}Failure`, an enum of `.declared(Error)` for the operation's own declared error and `.fault({Service}Fault)` for one it never declared:

```swift
// adapted from tests/service_schema_emitted_client_tests/run_swift.rs, the http_rest group's own recording transport
let client = ConversationClientServiceHttpClient(transport: recorder)
let outcome = await client.window(WindowRequest(conversationId: "652f1a3b4c5d6e7f8a9b0c1d", limit: 10))
if case .failure(.declared(let error)) = outcome {
  report(error)
}
```

A one-way method still `throws` the fault-only `{Service}Refusal`, shared with `swift_ws_client()`'s own one-way methods rather than named apart from them.

`<Service>Schema::kotlin_http_client()` is the Kotlin sibling, over the `kotlin` feature's own `kotlinx.serialization` types. A reply method answers the same sealed `{Service}{Operation}Result` `kotlin_ws_client()` answers with -- one `sealed interface` per reply operation with `Ok`/`Declared`/`Fault` members, narrowed with `is` exactly as the Dart pair is narrowed with `switch`:

```kotlin
// adapted from tests/service_schema_emitted_client_tests/run_kotlin.rs, the http_rest group's own recording transport
val client = ConversationClientServiceHttpClient(transport)
val outcome = client.window(WindowRequest(conversationId = "652f1a3b4c5d6e7f8a9b0c1d", limit = 10u))
if (outcome is ConversationClientServiceWindowResult.Declared) {
  report(outcome.error)
}
```

A one-way method still throws the fault-only `{Service}Refusal`.

All four non-Rust backends cover `body = "json"`, `body = "bytes"`, `body = "stream"` and `body = "multipart"` in full. A streamed operation's TypeScript client answers `{ contentRange: string | undefined; contentType: string; body: ReadableStream<Uint8Array> }` -- `contentRange` left `undefined` at the operation's own `ok_status`, read back off the response ahead of naming the body and set to the range text at `206`, `contentType` read back as a `bytes` reply's is -- off the seam's own `bodyStream` field; its Dart client answers the same fields as a `({String? contentRange, String contentType, Stream<List<int>> body})` record off `bodyStream` there too; its Swift client answers them as `(contentRange: String?, contentType: String, body: AsyncThrowingStream<Data, Error>)` off `bodyStream` there too; its Kotlin client answers them wrapped in an aux `{Result}Streamed(contentRange, contentType, body: Flow<ByteArray>)` type off `bodyStream` there too. A declared `header_out` composes onto either answer exactly as it does for `json` and `bytes`. A multipart operation's TypeScript client builds `parts` from the message's own fields and the declared `part` bindings; its Dart client builds the same list, the file handles crossing as `dynamic` through the same path an unknown type already renders by; its Swift client builds the same list, the file handles crossing as `any Sendable`; its Kotlin client builds the same list, the file handles crossing as `Any?`.

#### The `ws_rpc` Transport

`#[service_schema(transports = ["ws_rpc"])]` serves a service over one WebSocket as JSON text frames -- the same request, notify and reply terms `amqp_rpc` already answers through, carried over a socket instead of a queue. A service that only answers calls asks for it alone; a service the server calls instead -- pushing an event to whichever connection wants it -- asks for it too, notify travelling either direction over the same wire. The pair below is what a browser dashboard and its backend actually share: the browser calls `Ledger`, and the server pushes `LedgerEvents` back down the same connection.

```rust,ignore
/// What the browser or the Flutter app calls on the server.
#[service_schema(transports = ["ws_rpc"])]
pub trait Ledger<Ctx> {
    async fn create_transaction(
        &self,
        ctx: &Ctx,
        req: CreateTransactionRequest,
    ) -> Result<Transaction, CreateError>;

    async fn list_transactions(
        &self,
        ctx: &Ctx,
        req: ListTransactionsRequest,
    ) -> Result<TransactionList, ListError>;

    /// The app tells the server it showed a transaction; nothing comes back.
    #[service_schema_op(one_way)]
    async fn mark_seen(&self, ctx: &Ctx, req: MarkSeen);
}

/// What the server calls on the browser or the Flutter app.
#[service_schema(transports = ["ws_rpc"])]
pub trait LedgerEvents<Ctx> {
    #[service_schema_op(one_way)]
    async fn transaction_posted(&self, ctx: &Ctx, ev: TransactionPosted);
}
```

**The wire.** One JSON object per text frame, `kind` naming which:

| key | present on | value |
| --- | --- | --- |
| `kind` | every frame | `"request"`, `"notify"`, `"reply"`, `"ping"` or `"pong"` |
| `id` | `request`, `reply` | the id the caller's own request counter minted; a `reply` carries back whatever `id` its `request` named |
| `service` | `request`, `notify`, `reply` | the trait's own name, exactly as declared -- carried beside the payload rather than folded into it, since more than one service's frames can share one socket and no message type has to reserve a key for routing |
| `operation` | `request`, `notify` | the operation's own wire name |
| `payload` | `request`, `notify` | the operation's message, JSON-encoded |
| `headers` | `request`/`notify` claiming a `header_in`, `reply` claiming a `header_out` | one JSON-encoded entry per bound header -- the same encoding `header_in`/`header_out` already ride on `amqp_rpc`'s own headers table -- left off entirely where none are bound |
| `ok` | `reply` | `true` for a success, `false` for a declared error or a fault |
| `value` | `reply`, `ok: true` | the success value, or `null` for a unit success or a one-way operation answered through `request` |
| `error` | `reply`, `ok: false` | the operation's own declared error, or `{ isServiceFault: true, fault: <ServiceFault> }` |

`ping` and `pong` carry no field beside `kind`. A `ping` is sent by whichever side wants to probe liveness -- the generated TypeScript and Dart transports run their own heartbeat on the calling side; a hand-rolled interval does the same on the serving side, there being no `FrameSession` there to run one through -- and is answered with a `pong` by whichever side reads it: `answer` on the dispatcher, `FrameSession::deliver` on the client, and each generated transport's own socket listener.

**What the dispatcher macro emits.** `{service}_ws_rpc_dispatcher!()` emits everything every transport's dispatcher publishes -- `IncomingMessage`, `Reply`, `dispatch` and the panic guard and refused-payload reader, described under [Transports](#transports) above -- plus this transport's own pieces: `SERVICE`, the constant this dispatcher's own `answer` matches a frame's own `service` field against, so a frame belonging to another service on the same connection is left alone -- the client macro publishes an identical copy of its own, which `FrameSession::deliver` matches the same way; `Frame`, the enum one inbound frame decodes into -- `Request`, `Notify` and `Ping` on this side, a `Reply` or a `Pong` decoding to `Frame::Ignored` rather than a refusal, since it belongs to the client macro's own half of the wire; `FrameReply`, the `Reply` a request frame is answered through, its `send` published only where the service declares at least one operation that answers -- a one-way-only service's `FrameReply` publishes `fault` alone; and `answer(text, &svc, &ctx) -> Option<String>`, the one call an adapter makes per inbound text frame: a request naming this service is dispatched and its reply frame returned, a notify naming this service is dispatched and nothing returned, a ping is answered with a pong, and anything else -- another service's frame, a reply, text this transport does not recognise -- answers `None`.

**What the client macro emits.** `{service}_ws_rpc_client!()` emits everything every transport's client publishes -- `Transport`, `{Service}Client`, the fault mirror and the envelope readers, described under [Transports](#transports) above -- plus its own copy of the frame codec (`Frame` here reading `Reply`, `Ping` and `Pong` alone, a `Request` or a `Notify` decoding to `Frame::Ignored`), `ping_frame` -- published by the client macro alone, a dispatcher answering a ping rather than ever sending one -- and `pong_frame`, published by both; `request_frame` and `notify_frame`; and two ways to send them: `FrameWriter`, a one-way `Transport` over any function that puts a text frame on the wire, whose `request` always answers `Err` -- a writer keeps no correlation map to resolve one against; and `FrameSession`, a request-and-reply `Transport` over one socket in `core` and `std` alone, a correlation map keyed on an id from an atomic counter and settled by `deliver`, `close` failing everything still waiting once the socket goes away. A caller's own adapter shrinks to one send function plus one call to `deliver` per inbound frame.

**Placement follows the same doctrine as `amqp_rpc`'s three macros and `http_rest`'s two -- see "Where each goes, and why placement is fussy" above:** each invocation in a module of its own file, the `mod` declaration above the crate's `use` items, invoked by name (not by path) from inside the declaring crate:

```text
// the crate that serves it -- src/lib.rs
mod ledger_transport;

// src/ledger_transport.rs
declaring_crate::ledger_ws_rpc_dispatcher!();
```

```text
// a crate that calls it -- src/lib.rs
mod ledger_client;

// src/ledger_client.rs
use declaring_crate::{
    CreateError, CreateTransactionRequest, ListError, ListTransactionsRequest, MarkSeen,
    Transaction, TransactionList,
};

declaring_crate::ledger_ws_rpc_client!();
```

The dispatcher's crate names `serde`, `serde_json` and `tracing` in its own manifest, the panic guard logging through it; the client's crate names `serde` and `serde_json` and not `tracing` -- nothing in a client catches a panic -- and needs no runtime crate beyond those two for `FrameSession`'s own correlation map either, `core` and `std` being enough for a mutex and a waker.

**Refusals.** `amqp_rpc` and `ws_rpc` both answer every reply inside `{ ok, value, error }`, `ws_rpc` over its own frame rather than a queue message; `http_rest` does not, which is exactly why `body = "stream"` and `part(...)`-claimed file parts are `http_rest`-only grammar, and only meaningful on an operation that also carries an `http(...)` binding. A service that asks for `http_rest` beside `ws_rpc` (or `amqp_rpc`) on such an operation is refused at the declaration, naming the operation and the transport: a `body = "stream"` operation's `StreamedAnswer` and a `part(...)`-claimed file part are both a `Box<dyn BodySource + Send>`, publishing neither `Serialize` nor `Deserialize`, and neither has a carrier inside the envelope the other transport answers with:

```rust,ignore
#[service_schema(transports = ["ws_rpc", "http_rest"])]
pub trait ContentService<Ctx> {
    #[service_schema_op(http(
        method = "GET",
        path = "/documents/{document_id}/content",
        body = "stream",
        error_status(NotFound = 404),
    ))]
    async fn get_content(
        &self,
        ctx: &Ctx,
        document_id: String,
    ) -> Result<content_service_schema::StreamedAnswer, ContentError>;
}
```

```text
service_schema: operation `get_content` declares a streamed body, and this service also declares `ws_rpc`
       a streamed body has no carrier inside the `{ ok, value, error }` envelope those transports answer with - drop them from `transports`, or drop `body = "stream"` from this operation
```

A `part(...)` binding under `body = "multipart"` is refused the same way:

```rust,ignore
#[service_schema(transports = ["ws_rpc", "http_rest"])]
pub trait UploadService<Ctx> {
    #[service_schema_op(http(
        method = "POST",
        path = "/documents",
        body = "multipart",
        part("file" = attachment),
        error_status(TooLarge = 413),
    ))]
    async fn upload_document(
        &self,
        ctx: &Ctx,
        title: String,
        attachment: Box<dyn upload_service_schema::BodySource + Send>,
    ) -> Result<UploadResponse, UploadError>;
}
```

```text
service_schema: operation `upload_document` declares a multipart file part, and this service also declares `ws_rpc`
       a file part has no carrier inside the `{ ok, value, error }` envelope those transports answer with - drop them from `transports`, or drop the `part(...)` binding and `body = "multipart"` from this operation
```

Both are checked at the declaration, ahead of every transport's own macros, so the mismatch never gets as far as a dispatcher whose `Reply::send` would not compile.

**The TypeScript client and dispatcher attachment.** `<Service>Schema::ts_ws_client()` publishes the socket half beside `ts_client()`'s AMQP-shaped one: a `{Service}WsSocket` seam naming the four members every platform `WebSocket` already has (`send`, `close`, `addEventListener`/`removeEventListener` for `"message"` and `"close"`), so `new WebSocket(url)` plugs into `create{Service}WsTransport(socket, options)` with nothing written in between -- nothing generated here names `WebSocket` itself. `options.heartbeat` defaults to `{ intervalMs: 30_000, timeoutMs: 10_000 }`; `heartbeat: false` turns the probe off entirely. The returned transport owns that heartbeat and a per-socket correlation map, and checks every reply against the operation's own declared success or error schema before a caller sees it -- a mismatch folds into a `failed-validation` fault naming the first offending key, a unit success (having no schema to check) normalizes to `{ ok: true, value: undefined }` whatever `value` the reply carried, and a header tuple is checked on its body alone -- and settles every request still waiting with a `transport-failure` fault on a missed `pong` or a closed socket, so no caller hangs. Binding is per socket: two request-and-reply services sharing one connection each call `create{Service}WsTransport(socket)` on their own, and each runs its own heartbeat.

```typescript
import { createProbeServiceClient, createProbeServiceWsTransport } from "./bundle";

declare const socket: WebSocket;

export async function read(): Promise<string> {
  const client = createProbeServiceClient(createProbeServiceWsTransport(socket));
  const answered = await client.getBalance({ organization_id: "acme" });
  if (answered.ok) {
    return `${answered.value.credits}`;
  }
  return "failed";
}
```

Headers ride the frame's own `headers` object both ways: the transport writes the seam's JSON-encoded pairs into a `request` or `notify` frame as the JSON values they encode, leaving the key off where there are none, and hands a `reply` frame's `headers` back beside the envelope -- what the Rust `headers_table` and `headers_of` do.

`<Service>Schema::ts_ws_service()` publishes the other half: `attach{Service}WsDispatcher(socket, ctx, impl, onFault)` reads every `notify` and `request` frame naming the service off `socket`, drives `impl` through the generated dispatcher with the frame's own `headers`, and answers a `request` with a `reply` frame carrying the headers the dispatcher answered -- `{ ok: true, value: null }` where the dispatcher answered nothing, which is what lets a `request` naming a one-way operation still get an answer instead of leaving its caller waiting. `onFault` is required rather than optional: a `notify` that fails inside the dispatcher has nobody waiting on a reply to carry the fault, so it has nowhere else to go.

```typescript
import {
  attachProbeServiceWsDispatcher,
  type ProbeServiceExpireCreditOutcome,
  type ProbeServiceFaultKind,
  type ProbeServiceGetBalanceOutcome,
  type ProbeServiceSettleOutcome,
  type ProbeServiceSweepOutcome,
} from "./bundle";

type ProbeContext = { loggerName: string };

declare const socket: WebSocket;

attachProbeServiceWsDispatcher<ProbeContext>(socket, { loggerName: "probe" }, {
  async applyBundle(ctx, req): Promise<void> {
    void `${ctx.loggerName}:${req.organizationId}:${req.bundleId}`;
  },
  async expireCredit(ctx, req): Promise<ProbeServiceExpireCreditOutcome> {
    void `${ctx.loggerName}:${req.organizationId}:${req.creditId}`;
    return { ok: false, error: { errorCode: "conflict" } };
  },
  async getBalance(ctx, req): Promise<ProbeServiceGetBalanceOutcome> {
    void `${ctx.loggerName}:${req.organization_id}`;
    return { ok: true, value: { credits: 1 } };
  },
  async settle(ctx, req): Promise<ProbeServiceSettleOutcome> {
    void `${ctx.loggerName}:${req.organization_id}`;
    return { ok: true, value: { applied: true } };
  },
  async sweep(ctx, req): Promise<ProbeServiceSweepOutcome> {
    void `${ctx.loggerName}:${Object.keys(req).length}`;
    return { ok: false, error: { errorCode: "db-error" } };
  },
}, (fault) => {
  const kind: ProbeServiceFaultKind = fault.kind;
  void kind;
});
```

Both need the `zod` feature exactly as `ts_client()` and `ts_service()` do -- checking an inbound reply against a declared schema is what a `safeParse` against the message's own `$Schema` gives them, and a build without Zod cannot write that check truthfully, so a build with no Zod surface publishes neither.

**The Node WebSocket server.** `<Service>Schema::ts_ws_server()` publishes `create{Service}WsServer(impl, contextFor, options)`, which accepts connections rather than attaching to one socket somebody else already opened. `accept(socket)` builds a context per connection through the caller's own synchronous `contextFor`, wraps the existing `attach{Service}WsDispatcher` around a write-guarded view of the socket, and returns a `{Service}WsConnection` carrying an `AbortSignal` aborted when the connection closes, a `close()`, and `share(attach)` for handing the same guarded socket and signal to a second service's own attachment:

<!-- read from tests/service_schema_emitted_client_tests/run_node_ws_server.rs, the server accepting connections from the `ws` package -->
```typescript
const server = createConversationClientServiceWsServer(
  impl,
  (connection) => ({ n: ++n, signal: connection.signal }),
  { onFault: () => {} },
);
const wss = new WebSocketServer({ port: 0 });
wss.on("connection", (socket) => server.accept(socket));
```

`options.heartbeat` defaults to the client's own `{ intervalMs: 30_000, timeoutMs: 10_000 }`: the idle timer arms only once no frame of any kind has arrived for `intervalMs`, a `ping` then goes out, and the connection closes if no `pong` answers within `timeoutMs` -- a busy connection is never probed. The accepted-socket type requires an `error` listener the single-socket seam does not carry, because the `ws` package throws an unhandled `error` event out of `emit`; `options.onSocketError` is called, and the connection closed, when one fires. A second service reaches the same socket through the connection's own hook rather than through a second listener:

<!-- read from tests/service_schema_emitted_client_tests/run_node_ws_server.rs, scenario 8 -->
```typescript
wss.on("connection", (socket) => {
  connection = server.accept(socket);
  connection.share(attachInventoryServiceStub);
});
```

Each service answers only the frames naming it, exactly one `pong` answers one `ping` because the accepting server owns the heartbeat, and `share` throws once the connection has closed.

**The Dart client.** `<Service>Schema::dart_ws_client()` is the Dart sibling, over the `dart` feature's own generated types and JSON codec rather than Zod -- needing no separate outbound check, a Dart message being a real class with `required` constructor parameters that cannot be built malformed in the first place. `{Service}WsTransport` takes the sink and stream a `WebSocketChannel` already exposes (`StreamSink<dynamic>`/`Stream<dynamic>`) rather than naming `web_socket_channel` or `dart:io` itself, plus an optional `{Service}WsHeartbeat` (`{Service}WsHeartbeat.off()` turning liveness checking off, the same thing `heartbeat: false` does in TypeScript). `WebSocketChannel.stream` is single-subscription, so the transport is the only thing that ever calls `.listen` on it; an attachment for a second, browser-implemented service sharing the same connection reaches the transport's own `frames` instead -- a record pairing `inbound` (every frame the transport did not itself correlate to a pending request) with `send`, structurally identical for every service, so `attach{Service}WsDispatcher` composes over it without naming the calling service's transport class at all.

`{Service}WsClient` answers a returned `{Service}{Operation}Result` rather than throwing, exactly as `dart_http_client()` now does: a reply method answers `Future<{Service}{Operation}Result>` from the same sealed pair `<Service>Schema::dart_definition()` publishes, its `Ok`/`Operation`/`Fault` members carrying the success, the declared error and an undeclared fault respectively. A one-way method still answers `Future<void>` and throws the fault-only `{Service}WsRefusal`, having no reply arm to carry a fault through. A reply that will not decode answers the pair's `Fault` member carrying `failedValidation` rather than `undeserializablePayload` -- `ws_rpc` has no status line to draw that distinction with, so a reply failing the check the transport was always going to make against the declared type reads the same as any other failed validation. `{Service}Handlers` carries one handler per declared operation, and `attach{Service}WsDispatcher(frames, ctx, handlers, onFault: ...)` is the dispatcher attachment's own mirror: a decode failure, an unknown operation, or a handler throwing anything but its own declared error reaches `onFault` rather than vanishing, and a request left waiting on any of them is answered with the fault instead of hanging.

A `header_in` binding is one more argument after `req`, a `None` value still sent as JSON `null` rather than left out. A `header_out`/`error_header_out` tuple is rejoined on the way back: an absent optional element reads `null`, a missing or undecodable required one answers the pair's `Fault` member. `{Service}Handlers` takes the same `header_in` arguments, and `attach{Service}WsDispatcher` splits a header-tuple answer the same way it reads one -- a `null` element not written.

```dart
// not compiled here
final transport = LedgerWsTransport(sink: channel.sink, stream: channel.stream);
final ledger = LedgerWsClient(transport);
final created = await ledger.createTransaction(
  CreateTransactionRequest(account_id: 'acc-1', amount_cents: 1250, memo: 'coffee'),
);

attachLedgerEventsWsDispatcher(
  transport.frames,
  screen,
  LedgerEventsHandlers(
    transactionPosted: (ctx, ev) async => ctx.render(ev.transaction),
  ),
  onFault: (fault) => report(fault),
);
```

**The Swift client.** `<Service>Schema::swift_ws_client()` is the Swift sibling: `{Service}WsTransport(socket:, options:)` is an `actor` that owns the socket -- a `{Service}WsSocket` protocol of four members (`send`, `close`, `onMessage`, `onClose`), none naming a networking type, so an app's own `URLSessionWebSocketTask` wrapper satisfies it without an adapter -- correlates a `request` to its reply, answers an inbound `ping` with one `pong`, and runs its own heartbeat (`options.heartbeat` defaulting to `.init(intervalMs: 30_000, timeoutMs: 10_000)`, `nil` turning it off). A reply method answers `Result<Success, Failure>` exactly as `swift_http_client()`'s own methods do, rather than a `{Service}{Operation}Result` pair -- Swift's own `Result` already carries the two arms. `{Service}WsClient` is a `typealias` of `{Service}WsTransport`, so a caller constructs one and calls its methods directly, mirroring `{Service}HttpClient`'s own shape rather than wrapping a second object around it. A one-way method still `throws` the shared `{Service}Refusal`, having no reply arm to carry a fault through.

```swift
// adapted from tests/service_schema_emitted_client_tests/run_swift.rs, the ws_rpc group's own driver
let transport = ConversationClientServiceWsTransport(socket: socket, options: .init(heartbeat: nil))
let outcome = await transport.window(WindowRequest(conversationId: "652f1a3b4c5d6e7f8a9b0c1d", limit: nil))
if case .failure(.fault(let fault)) = outcome {
  report(fault)
}
```

A method takes one argument per `header_in` binding after the message, exactly as `swift_http_client()`'s own methods do; an `Optional` holding `nil` is sent as JSON `null` rather than left out. A `header_out` or `error_header_out` reply answers the declared tuple: an absent optional element reads `nil`, and a missing required one answers a `failedValidation` fault.

**The Kotlin client and mini server.** `<Service>Schema::kotlin_ws_client()` publishes `{Service}WsTransport(socket, options)` under the same preferred ownership shape: it correlates a `request` to its reply through a `CompletableDeferred`, answers an inbound `ping` with one `pong`, and runs its own heartbeat on a `CoroutineScope` it owns -- closing the transport cancels that scope, and with it every dispatcher reading the transport's own `frames`. `{Service}WsClient(transport)` wraps it and answers a reply method with the same sealed `{Service}{Operation}Result` `kotlin_http_client()` publishes; a one-way method still throws the fault-only `{Service}WsRefusal`, having no reply arm to carry a fault through. Where the service declares a `header_in`, a client method takes one parameter per binding after `req`, sent in the frame's own `headers` (`None` as JSON `null`); a `header_out`/`error_header_out` reply is rejoined into the `{Result}Value`/`{Result}Declared` class `kotlin_http_client()` publishes, an absent optional element reading `null` and a missing or undecodable required one answering a `failed-validation` fault naming the header.

Kotlin is the one target that also emits a server-side surface for this transport: `{Service}WsFrames` (`inbound`, `send`, `scope`) is the structural seam `attach{Service}WsDispatcher(frames, handlers, onFault)` -- the mini server -- dispatches over rather than the socket itself, so a second service sharing the same connection reaches the same frames without naming the first service's transport class at all. A handler takes the same `header_in` parameters the client sends and, for a header tuple, answers the same `{Result}Value`/`{Result}Declared` class; the attachment writes its `value`/`error` and each header under its own name in the reply's `headers`, a `null` element left out. The attachment's own `share(attach)` hands a second service's `attach*WsDispatcher` the transport's `send` and `scope`, and `detach()` tears down every service it shared the socket with along with itself:

```kotlin
// adapted from tests/service_schema_emitted_client_tests/run_kotlin.rs, the mini-server group's own driver
val serverTransport = ConversationClientServiceWsTransport(serverSocket, ConversationClientServiceWsOptions(heartbeat = null))
val attachment = attachConversationClientServiceWsDispatcher(serverTransport.frames, Handlers, onFault = { faults.add(it) })

attachment.share { send, scope ->
    val pulseFrames = PulseClientServiceWsFrames(serverTransport.frames.inbound, send, scope)
    attachPulseClientServiceWsDispatcher(pulseFrames, PulseHandlers, onFault = {})
    ({ pulseDetachRan = true })
}
```

**What a developer types.** Five seats share the pair declared above, each typing only what its own role needs:

1. **The crate that declares them** -- the two traits shown at the top of this section, and nothing transport-shaped beside the macro invocations every other seat places. `ServiceFault` and the rest of what [What a Service Generates](#what-a-service-generates) lists are generated once, from the declaration alone.
2. **The crate that serves `Ledger` and pushes `LedgerEvents`** -- invokes both macros, `ledger_ws_rpc_dispatcher!()` to answer inbound calls and `ledger_events_ws_rpc_client!()` to push over the same connection, and reads every frame off the socket through `answer`, forwarding whatever it returns back onto the wire. The reference loop, over axum, is `ledger_socket` in `tests/service_schema_ws_rpc_live_tests/tests.rs`:

   ```rust,ignore
   async fn ledger_socket(
       ws: WebSocketUpgrade,
       State(back_end): State<Arc<LedgerBackEnd>>,
   ) -> impl IntoResponse {
       ws.on_upgrade(move |socket| async move {
           use axum::extract::ws::Message;

           let (mut sink, mut stream) = socket.split();
           let (out, mut outbox) = mpsc::channel::<String>(64);
           tokio::spawn(async move {
               while let Some(text) = outbox.recv().await {
                   let _: Result<(), _> = sink.send(Message::Text(text.into())).await;
               }
           });
           let push = out.clone();
           let session = Session {
               user_id: "u-1".to_owned(),
               events: LedgerEventsClient::new(ledger_events_ws::FrameWriter::new(move |text| {
                   let sender = push.clone();
                   async move {
                       sender
                           .send(text)
                           .await
                           .map_err(|refused| refused.to_string())
                   }
               })),
           };
           while let Some(Ok(message)) = stream.next().await {
               let Message::Text(text) = message else {
                   continue;
               };
               if let Some(reply) = ledger_ws::answer(&text, &*back_end, &session).await {
                   let _: Result<(), _> = out.send(reply).await;
               }
           }
       })
   }
   ```

3. **A crate that calls `Ledger` and receives `LedgerEvents` pushes** -- invokes the other two macros, `ledger_ws_rpc_client!()` and `ledger_events_ws_rpc_dispatcher!()`, and drives them with a matching loop. The reference, over tokio-tungstenite, is `connect` in the same file:

   ```rust,ignore
   async fn connect(
       url: &str,
   ) -> (
       LedgerClient<FrameSession>,
       mpsc::Receiver<TransactionPosted>,
   ) {
       let (socket, _response) = connect_async(url).await.unwrap();
       let (mut sink, mut stream) = socket.split();
       let (out, mut outbox) = mpsc::channel::<String>(64);
       tokio::spawn(async move {
           while let Some(text) = outbox.recv().await {
               let _: Result<(), _> = sink.send(Message::Text(text.into())).await;
           }
       });

       let session = FrameSession::new(move |text| {
           let sender = out.clone();
           async move {
               sender
                   .send(text)
                   .await
                   .map_err(|refused| refused.to_string())
           }
       });
       let ledger = LedgerClient::new(session.clone());
       let reader = session;
       let (posted_out, posted_in) = mpsc::channel(16);
       let screen = Screen(posted_out);
       tokio::spawn(async move {
           while let Some(Ok(message)) = stream.next().await {
               let Message::Text(text) = message else {
                   continue;
               };
               reader.deliver(&text).await; // replies to my requests, and pongs
               screen_ws::answer(&text, &screen, &()).await; // pushes from the server
           }
           reader.close("the socket closed");
       });

       (ledger, posted_in)
   }
   ```

   Both loops read every frame off the socket and act on text frames alone (`let Message::Text(text) = message else { continue };`); a binary frame is left unread, there being no binary framing this transport defines.
4. **The browser** -- `createLedgerWsTransport(socket)` to call `Ledger`, `attachLedgerEventsWsDispatcher(socket, ctx, impl, onFault)` to answer `LedgerEvents` pushes on the same socket, exactly as shown above.
5. **The Flutter app** -- `LedgerWsTransport(sink: ..., stream: ...)` to call `Ledger`, `attachLedgerEventsWsDispatcher(transport.frames, ctx, handlers, onFault: ...)` to answer `LedgerEvents` pushes on the same connection, exactly as shown above.

There is no subscription operation, no topic and no broker: a push travels exactly as far as the one connection it was written against, over the `LedgerEventsClient` the serving crate built from that connection's own `FrameWriter` -- `Session` above holds one `events` client per connection, not a registry of every connection there is. "Tell this user" is nowhere generated, because it is the application's own question to answer: a registry from a user id to the connections currently open for it, built and kept by whatever code accepts the upgrade, exactly as `Session.user_id` above stands in for one connection's own identity. `ws_rpc` gives that registry a wire to push over once it has decided who to tell; deciding who that is stays the application's.

## Field Validation (`model_schema_prop`)

Use `#[model_schema_prop(...)]` on individual fields to add validation constraints, name [checks](#checks-check) of your own, override types, or correct a payload before it is read ([`preprocess`](#preprocessing-preprocess)). A constraint written once is held in five languages: the Zod schema checks it in TypeScript, a generated `validate()` method checks it in Rust, and the `validate()` of each emitted [Dart](#validate-in-dart), [Swift](#validate-in-the-swift-client) and [Kotlin](#validate-in-the-kotlin-client) type checks it there. The JSON Schema states it. In Rust, Dart, Swift and Kotlin the read of a type runs its `validate()` on the value it read, and refuses a payload that breaks a rule. [Preprocess and check](#preprocess-and-check) states the two keys that name functions of your own, for the five languages side by side.

### String Constraints (minLength, maxLength, pattern)

```rust
use tixschema::{model_schema, model_schema_prop};
use serde::{Deserialize, Serialize};

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct UserProfile {
    #[model_schema_prop(minLength = 3, maxLength = 30, pattern = "^[a-z0-9_]+$")]
    pub username: String,

    #[model_schema_prop(maxLength = 200)]
    pub bio: String,

    #[model_schema_prop(pattern = "^[0-9a-fA-F]{24}$")]
    pub external_id: String,
}
```

Generated Zod for `username` -- each check carrying the sentence the Rust validator reports for the same bound, so the two languages answer a bad value with the same words:

```text
z.string()
  .min(3, { error: (issue) => `too short: minimum length is 3, got ${String(issue.input).length}` })
  .max(30, { error: (issue) => `too long: maximum length is 30, got ${String(issue.input).length}` })
  .check(z.regex(/^[a-z0-9_]+$/, { error: "does not match pattern '^[a-z0-9_]+$'" }))
```

Generated JSON Schema for `username`: `{ "type": "string", "minLength": 3, "maxLength": 30, "pattern": "^[a-z0-9_]+$" }`

The TypeScript type is unchanged -- still just `string`.

A value breaking more than one of a field's bounds is reported once per bound on both sides, in the order the bounds were declared: `validate()` collects every violation rather than stopping at the first, and every check on the Zod schema runs. So `"A!"` in the field above reads `'username': too short: minimum length is 3, got 2; 'username': does not match pattern '^[a-z0-9_]+$'` from a Rust service and from a TypeScript one alike.

A type whose schema this crate writes whole carries none of the five constraints, and writing one on
such a field is a compile error naming the keys and the type: `ObjectId` writes a `{"$oid": "..."}`
object rather than a string, and the chrono types (`NaiveDate`, `NaiveTime`, `NaiveDateTime`,
`DateTime<Tz>`) write their own ISO spellings, which no surface reads a length, a pattern or a range
beside. Carry the value in a `String` field if it needs one.

A `PathBuf` field carries the same three constraints, as does the `Path` borrow behind a wrapper (`Box<Path>`, `Cow<'_, Path>`, `Arc<Path>`, `Rc<Path>`): serde writes a path as a JSON string, which is what the three surfaces render a constrained string for. The checks measure that string -- the path's `to_string_lossy` rendering, which is the exact wire value for every path serde can write, a path that is not UTF-8 being one serde refuses to serialize at all.

A borrowed string carries them too: a `&'a str` or `&'static str` field, bare or under the wrappers a `String` is reached through (`Option<&'a str>`). serde writes it as the JSON string a `String` writes, so the Zod schema and the JSON Schema are the ones a `String` field gets, and `validate()` holds the borrowed value to the bound in the same words.

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Greeting<'text> {
    #[model_schema_prop(minLength = 3)]
    pub name: &'text str,
}

Greeting { name: "al" }.validate();
// Err(["'name': too short: minimum length is 3, got 2"])
```

`validate()` reads through every reference, whatever it borrows, as the Zod schema and the JSON Schema do. A bound on a `&'a String`, a `&'a u32`, a `&'a [String]`, an `Option<&'a String>` or a `&'a Option<String>` field holds the value it borrows, and `validate()` answers it in the words the owned field gets. A bound refused on the owned type is refused on a reference to it: a length on a `&'a bool` fails to build with the error a length on a `bool` gives, and a bound on a `&'a RefCell<String>` is refused at expansion, naming `RefCell`.

#### A field typed with a branded string

A field typed with a [branded newtype](#branded-newtypes) over a string carries the same three constraints, and the bound holds that field alone. The brand keeps none, so another field of the same type reads any value:

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct FullPath(pub String);

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct GroupTag {
    #[model_schema_prop(minLength = 1)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_field: Option<FullPath>,
    pub source: FullPath,
}
```

- `validate()` reports `'sort_field': too short: minimum length is 1, got 0` for `Some(FullPath(String::new()))`, and nothing for an empty `source`.
- The Zod schema writes the checks onto the brand's own schema, so the parsed value keeps its brand: `FullPath$Schema.check(z.minLength(1, { error: ... }))`.
- The JSON Schema narrows the brand's document from inside an `allOf`: `{ "allOf": [{ "type": "string" }, { "minLength": 1 }] }`.

The bound reaches the brand through the wrappers it reaches a `String` through -- `Option`, `Box`/`Rc`/`Arc`/`Cow` -- and whether the brand is declared above the field's type or below it. On a list of the brand it counts the list, as on [every list](#list-length-minlength-maxlength). The validator measures the text the brand publishes for it: `model_schema_bound_text()`, a method `#[model_schema]` adds to every brand whose inner is a `String`, a `str`, a `PathBuf` or a `Path`, directly or behind `Box`/`Rc`/`Arc`/`Cow`. A `#[model_schema]` alias of a `String` or a `PathBuf` that takes no parameter is measured as the type it names.

A length or a pattern on a field typed with anything else is a compile error at the field, because no surface has a string to measure there: a struct, an enum, a brand over a number, a `bool`, a `char`.

```text
error[E0277]: model_schema: field `latest`: `minLength` cannot apply to `Version`
   |
14 |     pub latest: Version,
   |                 ^^^^^^^ this type holds no string for the bound to measure
   |
   = note: a length or a pattern is measured on a `String`, on a path, or on a `#[serde(transparent)]` brand over one of them that `#[model_schema]` was written on: declare the field as one of those, or drop the bound
```

Two more placements are refused as the field is expanded: a length or a pattern on a type written with arguments (`UserId<String>`), which writes whatever its arguments make it, and `minimum` or `maximum` on a field typed with any type declared elsewhere, a range being spelled against a number this crate writes itself.

#### What a `pattern` may contain

One `pattern` string reaches three readers: the generated Rust validator builds it with `regex::Regex::new`, the Zod schema splices it between `/` delimiters as a regex literal, and the JSON Schema `pattern` keyword is an ECMA-262 regex. A pattern is accepted only if all three read it, and read it the same way; otherwise the derive fails at expansion with the construct named.

**Engine baseline: ES2018.** The emitted Zod literals and JSON Schema patterns are written for an ECMA-262 engine at ES2018 or newer, and the guard's admissions are decided against that version rather than against whichever JavaScript runtime happens to be installed where the derive runs -- the schema is read wherever it is loaded, not where it was generated. ES2018 is what the crate's own output already assumes: it emits `(?<name>...)` named capture groups, which are ES2018. Nothing it emits needs anything newer.

Zod literals are spliced without flags, so the JavaScript side is always non-Unicode mode.

Some constructs are **translated** on the way out, to a spelling that means the same thing to all three readers. The translation is what every surface receives, the Rust validator included, so the three never validate different sets:

| You write | Every surface receives | Why |
|---|---|---|
| `(?P<name>...)` | `(?<name>...)` | One group under two spellings; the `regex` crate reads both, JavaScript reads only the second. |
| `\d` | `[0-9]` | The `regex` crate reads `\d` as the Unicode digit class, a flagless literal as ASCII -- so `^\d+$` would accept `١` in Rust and reject it in the browser. |
| `\w` | `[0-9A-Za-z_]` | Same divergence: the `regex` crate counts `é` and `α` as word characters, a flagless literal does not. |
| `\s` | `[\t\n\v\f\r ]` | The two whitespace sets are not even nested -- the `regex` crate spaces U+0085 and JavaScript does not, JavaScript spaces U+FEFF and the `regex` crate does not -- leaving the ASCII run as the only common ground. |

A class you already wrote out is left exactly as you wrote it, byte for byte.

Some constructs are **refused**, because no spelling makes the readers agree:

- `.`, and the negated classes `\D`, `\W`, `\S`. A flagless JavaScript literal matches one UTF-16 code unit where the `regex` crate matches one character, so a single character outside the Basic Multilingual Plane -- an emoji, say -- fills `^.$` in Rust and never in JavaScript. Writing the members out settles *which* characters are named and cannot settle how many code units one of them is. Name the characters you mean instead.
- `(?i:...)`, `(?m:...)` and `(?s:...)`, and their negated and combined forms. These are ECMA-262 regular expression modifiers, which post-date the ES2018 baseline; an engine at the baseline throws a `SyntaxError` where the schema loads. Recent runtimes do parse them, which is exactly why the baseline is a recorded decision rather than a measurement.
- `(?i)`, `(?x:...)`, `(?U:...)`, `(?u:...)`, `(?R:...)` -- inline flag directives and flags ECMA-262 never had.
- `\A`, `\z`, `\b{start}`, `\<`, `\>` -- anchors and boundaries JavaScript reads as escaped letters. Use `^` and `$`.
- `\p{...}`, `\pL`, `\P{...}` and POSIX `[:alpha:]` -- Unicode and POSIX classes a flagless literal reads as ordinary characters.
- `&&`, `--`, `~~` class operators and a class nested inside a class -- set operations JavaScript reads as class members.
- `\x{...}`, `\u{...}`, `\U...` and octal escapes -- code point escapes JavaScript reads differently or not at all. Write the character.
- An unescaped `]` opening a class, as in `[]-a]`. This one is refused rather than escaped because escaping it changes the meaning: `[]-a]` is the three members `]`, `-` and `a`, and `[\]-a]` is a range.

#### A `pattern` has to turn some value away

A pattern every string satisfies is refused too, for a different reason: it is a constraint that constrains nothing. `""`, `^`, `$`, `|`, `a*` and `^a*` all match at some position of every string, so the generated validator would reject no value and the Zod and JSON schemas would publish a check every payload passes -- an attribute that reads as a guarantee and is not one. The derive fails at expansion, naming the field or the brand.

The verdict is read off the parsed pattern rather than off its text, so the same shape written any way gets the same answer -- and the near misses keep theirs:

| Pattern | Verdict |
|---|---|
| `""`, `^`, `$`, `\|`, `()`, `(^)` | Refused -- matches at a position every string has |
| `a*`, `a?`, `a\|`, `^a*`, `a*$` | Refused -- the repeated part may run zero times, or the alternative may be skipped |
| `^$` | Accepted -- both ends at one position, which only the empty string has |
| `^a*$` | Accepted -- both ends pinned around a run of `a` |
| `\b`, `\B` | Constraining -- the empty string holds no word boundary -- and refused by the next rule |
| `\b\w+`, `\Ba` | Accepted -- a boundary with something beside it |
| `a+`, `^[a-z]+$` | Accepted -- a character has to be there |

#### A `pattern` cannot be one assertion and nothing else

`\b` and `\B` are refused for a third reason, and it is not about what they accept. The generated Rust validator builds the pattern with `regex::Regex::new`, and `clippy::trivial_regex` -- which a consumer denying `clippy::nursery` gets -- calls a lone assertion "unlikely to be useful as it is" and reports it against the `#[model_schema]` attribute, where the author has no edit to make. For the shapes that lint does name a replacement for -- `^abc`, `abc$`, `^abc$`, `^$`, `abc` -- this crate emits the `str` call instead of a regex, so they compile clean; for a lone boundary it names none, so there is nothing to put in the regex's place.

Write the boundary beside what has to sit next to it: `\b\w+` rather than `\b`, `\B\w` rather than `\B`. That keeps the regex, keeps the boundary check, and is no longer trivial to the lint. Only the lone assertion is refused -- `\ba`, `^\bfoo` and every other pattern with a boundary somewhere in it are untouched.

### Numeric Constraints (minimum, maximum)

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Product {
    #[model_schema_prop(minimum = 0, maximum = 120)]
    pub age_restriction: u32,

    #[model_schema_prop(minimum = 0.0)]
    pub price: f64,
}
```

### List Length (minLength, maxLength)

A key written on a field measures the field's own value. On a list, `minLength` and `maxLength` count its items, and no key written on a list reaches the items:

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    pub value: String,
}

#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    #[model_schema_prop(minLength = 1, maxLength = 3)]
    pub name_config: Vec<Part>,
}

Document { name_config: Vec::new() }.validate();
// Err(["'nameConfig': too short: minimum length is 1, got 0"])
```

A list of four parts answers `'nameConfig': too long: maximum length is 3, got 4`, and a list of one to three passes whatever its items hold. Each key works alone.

- **Zod** writes the count on the array: `z.array(Part$Schema).min(1, { error: ... }).max(3, { error: ... })`, each check reporting the sentence `validate()` reports.
- **JSON Schema** writes `"minItems": 1, "maxItems": 3` on the array, which are the keywords a validator reads there.
- **`validate()`** reports the list once, under the field's key. On a member of an `#[serde(untagged)]` enum the read counts it too, and the [recovering decode](#recovering-decode-decode_with) lists one `Invalid` at the list. See [Where a constraint is checked](#where-a-constraint-is-checked).
- **TypeScript** stays `Array<Part>`, with `Minimum length: 1` and `Maximum length: 3` in the member's comment.

Every list takes the two keys: `Vec`, `VecDeque`, `HashSet`, `BTreeSet`, `BinaryHeap`, a slice (`Box<[T]>`, `&'a [T]`), a fixed-size array `[T; N]`, and `IndexSet<T>` under the `indexmap` feature, whatever `T` is -- a model, a string, a number, a map, one of the item's own type parameters, or the item itself. On `[T; N]` the JSON Schema states the declared length and the bound together. `Option` and the transparent wrappers (`Box`, `Rc`, `Arc`, `Cow`) are read through, each holding one value: an `Option<Vec<T>>` that is `None` passes, and `Some(vec![])` is counted. A list is a value of its own and is not read through, so on a `Vec<Vec<String>>` the keys count the outer list and say nothing of the inner ones.

`pattern`, `minimum` and `maximum` on a list are compile errors, a list having no text and no number. A rule for each item goes on the item's own type, a [branded newtype](#branded-newtype-validation-constraints) that carries it, and `validate()` runs it on each item beside the list's own count, reporting each item under [its index](#the-path-a-message-names):

```rust
#[model_schema(pattern = "^[a-z]+$", minLength = 1, maxLength = 3)]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Tag(pub String);

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Article {
    #[model_schema_prop(minLength = 1)]
    pub tags: Vec<Tag>, // at least one tag, each held by `Tag`
}
```

A brand carries `pattern`, `minLength` and `maxLength` and no range, so a range for each number of a list has no place to be written.

The validator and the hook a list publishes, `validate_{field}_value(&list)` and `deserialize_{field}`, are written over any list: they name no type the list holds, so a `Vec<T>` of the item's own parameter and a `Vec<Self>` get them as a `Vec<String>` does.

**What changed for a declaration that compiled.** A key written on a list used to be carried to each of its items. `minLength` or `maxLength` on a list of strings, of paths or of brands still compiles and now counts the items: `#[model_schema_prop(minLength = 3)] pub tags: Vec<String>` held each tag to three characters, and holds the list to three tags. The compiler does not report that change. `pattern`, `minimum` or `maximum` on a list no longer compiles.

### A bound of the wrong kind

A length or a pattern is measured on a string, a range on a number, and a length alone on a list. A key written on a field whose value it does not measure is a compile error as the field is expanded, in every feature configuration, naming the field and the keys:

| Keys | Field | What happens |
|---|---|---|
| `minLength`, `maxLength`, `pattern` | a number: `u8` to `u64`, `i8` to `i64`, `usize`, `isize`, `f32`, `f64` | Refused |
| `minimum`, `maximum` | a string: `String`, `str`, `PathBuf`, `Path` | Refused |
| `minimum`, `maximum` | a `bool`, a `char`, and a value every surface describes as unknown | Refused |
| `pattern`, `minimum`, `maximum` | a list | Refused |
| `minLength`, `maxLength`, `pattern` | a string, one read through a reference included | Enforced by Zod, the JSON Schema and `validate()` |
| `minimum`, `maximum` | a number | Enforced by Zod, the JSON Schema and `validate()` |
| `minLength`, `maxLength` | a list | Enforced by Zod, the JSON Schema and `validate()`: they [count its items](#list-length-minlength-maxlength) |

`Option` and a transparent wrapper do not change which kind a field is: `Option<u32>` and `Box<u32>` are numbers, `Option<String>`, `Box<str>`, `Cow<'_, str>` and `&'a str` are strings, and `Option<bool>` and `Box<char>` are refused a range as `bool` and `char` are. A list is a list whatever it holds: `Vec<u32>`, `[u32; 4]` and `Option<Vec<String>>` are lists.

```text
error: model_schema: field `age_restriction`: `minLength` cannot apply to a `u32` field — a length or a pattern is measured on a string, and this field writes a number: the constraint would reach neither Zod, nor the JSON schema, nor the generated validator. Bound a number with `minimum` or `maximum`, or drop it.
```

```text
error: model_schema: field `username`: `minimum` cannot apply to a `String` field — a range is measured on a number, and this field writes a string: the constraint would reach neither Zod, nor the JSON schema, nor the generated validator. Bound a string with `minLength`, `maxLength` or `pattern`, or drop it.
```

```text
error: model_schema: field `tags`: `pattern` cannot apply to a `Vec<String>` field — a key measures the field's own value, and this field writes a list, which has no text for a pattern and no number for a range. `minLength` and `maxLength` count the list's items, and no key written here reaches them. A rule for each item goes on the item's own type: declare the item as a `#[serde(transparent)]` brand that carries the `pattern`.
```

The refusal names the keys of the wrong kind and no other: `minimum` written beside a refused `minLength` on a number is not named, and neither is a `minLength` written beside a refused `pattern` on a list.

### The `validate()` Method

When any field carries a constraint, or the type or one of its fields names a [check](#checks-check), the macro generates a `validate(&self) -> Result<(), Vec<String>>` method. It is what checks a field's constraints. A serde read of the type calls it on the value it read and [refuses what it refuses](#a-read-refuses-what-validate-refuses), and you call it on an instance built or changed in Rust.

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Registration {
    #[model_schema_prop(minLength = 3, maxLength = 30)]
    pub username: String,

    #[model_schema_prop(minimum = 0, maximum = 120)]
    pub age: u32,
}

let reg = Registration { username: "ab".to_string(), age: 150 };

match reg.validate() {
    Ok(()) => println!("valid"),
    Err(errors) => {
        for e in &errors {
            println!("Error: {e}");
        }
        // "'username': too short: minimum length is 3, got 2"
        // "'age': too large: maximum is 120, got 150"
    }
}
```

The macro also generates into the type's schema module:
- `validate_{field}_value(&FieldType) -> Result<(), Vec<String>>` -- pure static validator per field, answering every bound the value broke in the order they were declared
- `deserialize_{field}(D) -> Result<FieldType, E>` -- serde hook that calls the static validator; a field read through a reference gets none

A constrained field of an enum variant is named for its variant too -- `validate_{variant}_{field}_value` and `deserialize_{variant}_{field}`, with `{variant}` in `snake_case`. One schema module holds every variant's helpers, and a field name is unique only within the variant that declares it, so two variants naming one field carry their own constraints.

An enum whose members carry constraints publishes the same `validate(&self) -> Result<(), Vec<String>>`, under every tagging serde offers (externally, internally and adjacently tagged, and `#[serde(untagged)]`). A value holds one variant at a time, so the check runs that variant's members and no other's, collecting every violation in the words a struct's field is answered in:

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Action {
    Delete {
        #[model_schema_prop(minLength = 5)]
        note: String,
    },
    Upload {
        #[model_schema_prop(minLength = 3)]
        note: String,
    },
}

let deleting = Action::Delete { note: "abc".to_string() };
assert_eq!(
    deleting.validate().unwrap_err(),
    vec!["'note': too short: minimum length is 5, got 3".to_string()],
);
// The same value read as an `Upload` is held to that variant's own minimum, and passes.
assert!(Action::Upload { note: "abc".to_string() }.validate().is_ok());
```

Parity with structs runs both ways: an enum no member of which carries a constraint publishes no `validate()`, exactly as a constraint-free struct publishes none.

#### A type that declares a parameter

`validate()` is written in the type's own `impl`, so a struct, a tuple struct and an enum that declare a type parameter have it at every filling, and a type that holds one at any filling runs it:

```rust
#[model_schema(default_types(CodeType = String))]
#[derive(Serialize, Deserialize)]
pub struct Crate<CodeType> {
    pub codes: Vec<CodeType>,
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Depot {
    pub numbered: Crate<u32>,
    pub named: Crate<String>,
}

Crate::<u32> { codes: vec![], label: String::new() }.validate();
// Err(["'label': too short: minimum length is 1, got 0"])

Depot {
    numbered: Crate { codes: vec![], label: String::new() },
    named: Crate { codes: vec![], label: String::new() },
}
.validate();
// Err(["'numbered.label': too short: minimum length is 1, got 0", "'named.label': too short: minimum length is 1, got 0"])
```

A [branded newtype](#branded-newtype-validation-constraints) that declares a parameter has `validate()` at every filling as well, and runs its string checks where the filling is its declared default.

**`validate()` reads nothing behind a type parameter.** The `validate()` of a `Crate<CodeType>` runs nothing on its `codes`, whatever fills `CodeType`, a type that has a `validate()` of its own included. A read of the same payload is refused: what fills the parameter is read by its own `Deserialize`, and judges itself [as it is read](#a-read-refuses-what-validate-refuses), with its own messages and no place of the holder in front of them:

```rust
let outer = Crate {
    codes: vec![Crate::<u32> { codes: vec![], label: String::new() }],
    label: "outer".to_string(),
};
outer.validate();
// Ok(())

serde_json::from_str::<Crate<Crate<u32>>>(r#"{"codes":[{"codes":[],"label":""}],"label":"outer"}"#);
// Err: 'label': too short: minimum length is 1, got 0 at line 1 column 34
```

**A `validate` of your own on one filling does not build.** A type has one inherent function under a name, and the generated one is there at every filling:

```rust,compile_fail
impl Crate<u32> {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        if self.codes.contains(&0) {
            return Err(vec!["'codes': no code is zero".to_owned()]);
        }
        Ok(())
    }
}
```

```text
error[E0592]: duplicate definitions with name `validate`
```

#### The path a message names

Each message opens with where the broken value sits in the payload, in single quotes: every step serde writes on the way to the value, joined by dots. A step is the key of a member, the name or the content key serde writes a variant under, a position in an array, the index of an item of a list or the key of an entry of a map. Where serde writes the value itself, there is no step. The path is the one an issue of the Zod schema of the same declaration holds, and the one an issue of the [recovering decode](#recovering-decode-decode_with) holds.

- **A member** is named by the key serde writes it under. `rename_all` on a struct, `rename_all_fields` on an enum and a variant's own `rename_all` each move the name, and a field's own `rename` wins over the three. A type that renames nothing names each field as it was declared.
- **An item of a list** is named by its index, counted from zero, at each level of the list. A `grid: Vec<Vec<Package>>` answers `'grid.0.1.itemLabel': too short: minimum length is 1, got 0`, and a `code_list: Vec<Code>` answers `'codeList.1': too short: minimum length is 2, got 1`. A count written on the list itself is reported at the list, with no index: a `parcel_list` under `minLength = 1` answers `'parcelList': too short: minimum length is 1, got 0` when it is empty.
- **An entry of a map** is named by its key, as serde writes it. A `parcels_by_dock: BTreeMap<String, Parcel>` answers `'parcelsByDock.north.itemLabel': too short: minimum length is 1, got 0`. A key that is no string is the text serde writes for it: `'byNumber.7.itemLabel'` for a `u32`, `'byFlag.true.itemLabel'` for a `bool`, and for a plain enum the name serde writes the variant under, `'byDock.north.itemLabel'`. A map of lists and a list of maps name the key and the index in the order the payload nests them: `'listsByDock.north.1.itemLabel'` and `'docksInOrder.1.north.itemLabel'`. See [A model held in a map, in a slot or in a tuple](#a-model-held-in-a-map-in-a-slot-or-in-a-tuple).
- **A slot among several** is named by its position in the array serde writes, `'0'`, and so is a position of a tuple written as a field's type: `'deckPair.1.itemLabel'`. A slot under `#[serde(skip)]` is in no array, so the slots after it move up.
- **The one slot of a struct** is the value serde writes for the struct, and adds no step. `Sealed(Parcel)` answers `'itemLabel': too short: minimum length is 1, got 0` for the payload `{"itemLabel":""}`, a field `sealed_load: Sealed` answers `'sealedLoad.itemLabel': ...`, and a field `code: Lone`, over a `Lone(String)` held to `minLength = 1`, answers `'code': too short: minimum length is 1, got 0` for the payload `{"code":""}`.
- **A member of a variant** is named below what serde writes ahead of it:

  | The enum is written | serde writes | A named member | The one slot | A slot among several |
  |---|---|---|---|---|
  | with no tag attribute | `{"Loose":{"freightLabel":""}}` | `'Loose.freightLabel'` | `'Boxed.itemLabel'` | `'Split.1.itemLabel'` |
  | under `tag = "kind", content = "held"` | `{"kind":"Loose","held":{"haulLabel":""}}` | `'held.haulLabel'` | `'held.itemLabel'` | `'held.1.itemLabel'` |
  | under `tag = "kind"` | `{"kind":"Loose","liftLabel":""}` | `'liftLabel'` | `'itemLabel'` | |
  | under `untagged` | `{"loadLabel":"","loadParcels":[]}` | `'loadLabel'` | `'itemLabel'` | |

  The one slot holds a `Parcel` in each row, and the variant's step is the name serde writes it under: the variant's own `rename`, or the enum's `rename_all`. A slot that holds a string is named by the variant alone, `'Stamped': too short: minimum length is 1, got 0` for the payload `{"Stamped":""}`.
- **A `#[serde(flatten)]` member and an `Option`** write no key and add no step.

```rust
#[model_schema(minLength = 2)]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Code(pub String);

#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Package {
    #[model_schema_prop(minLength = 1)]
    pub item_label: String,
    #[serde(rename = "codes")]
    #[model_schema_prop(minLength = 1)]
    pub listed_item_codes: Vec<String>,
    pub parcel_code: Code,
}

#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Base {
    #[model_schema_prop(minLength = 1)]
    pub base_name: String,
}

#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub first_parcel: Package,
    pub parcel_list: Vec<Package>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spare_parcel: Option<Package>,
    #[serde(flatten)]
    pub starting_base: Base,
}

let broken = || Package {
    item_label: String::new(),
    listed_item_codes: Vec::new(),
    parcel_code: Code("a".to_string()),
};
let manifest = Manifest {
    first_parcel: broken(),
    parcel_list: vec![broken(), broken()],
    spare_parcel: Some(broken()),
    starting_base: Base { base_name: String::new() },
};
println!("{}", manifest.validate().unwrap_err().join("\n"));
```

```text
'firstParcel.itemLabel': too short: minimum length is 1, got 0
'firstParcel.codes': too short: minimum length is 1, got 0
'firstParcel.parcelCode': too short: minimum length is 2, got 1
'parcelList.0.itemLabel': too short: minimum length is 1, got 0
'parcelList.0.codes': too short: minimum length is 1, got 0
'parcelList.0.parcelCode': too short: minimum length is 2, got 1
'parcelList.1.itemLabel': too short: minimum length is 1, got 0
'parcelList.1.codes': too short: minimum length is 1, got 0
'parcelList.1.parcelCode': too short: minimum length is 2, got 1
'spareParcel.itemLabel': too short: minimum length is 1, got 0
'spareParcel.codes': too short: minimum length is 1, got 0
'spareParcel.parcelCode': too short: minimum length is 2, got 1
'baseName': too short: minimum length is 1, got 0
```

What a program is written with keeps its Rust name: the helpers of `item_label` are `validate_item_label_value` and `deserialize_item_label`, and a refusal at build time names the field as its author wrote it.

#### A slot of a tuple variant

A slot of a tuple variant takes the five constraints as a named member of the variant does, with one slot or several, and the variant's wire and TypeScript type do not change. The Zod schema and the JSON Schema write the checks on the slot's own member, and the Rust side enforces them where it enforces a named member's in the same position:

| The slot of | The read | `validate()` reports a violation |
|---|---|---|
| a newtype variant of an `#[serde(untagged)]` enum, such as the `Other(String)` arm of [`MimeType`](#untagged-enums-serdeuntagged) | checks it, which is what chooses the variant | under no name: `too short: minimum length is 1, got 0` |
| a tuple variant of a tagged enum | reads it, and the enum's read refuses the value once it is whole | at [the path serde writes the slot at](#the-path-a-message-names): `'Code': ...` for the one slot of a variant, and `'Pair.0': ...` and `'Pair.1': ...` for two slots |

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub enum Entry {
    Code(#[model_schema_prop(pattern = "^[A-Z]{3}$")] String),
    Pair(
        #[model_schema_prop(minLength = 2)] String,
        #[model_schema_prop(minimum = 1)] u32,
    ),
}

Entry::Code("abc".to_string()).validate();
// Err(["'Code': does not match pattern '^[A-Z]{3}$'"])
Entry::Pair("a".to_string(), 0).validate();
// Err(["'Pair.0': too short: minimum length is 2, got 1", "'Pair.1': too small: minimum is 1, got 0"])
MimeType::Other(String::new()).validate();
// Err(["too short: minimum length is 1, got 0", "does not match pattern '^[a-z]+/[a-z0-9.+-]+$'"])
```

serde refuses `{"Code":"abc"}` and `{"Pair":["a",0]}` with those messages, the two of the pair joined by `; `. It reads `{"Code":"ABC"}` and `{"Pair":["ab",1]}` as it did, and `Entry::Code("ABC".to_string())` still writes `{"Code":"ABC"}`. The helpers are named by the slot's place among the ones its variant declares, `validate_{variant}_{index}_value` and `deserialize_{variant}_{index}`: `validate_code_0_value` and `validate_pair_1_value` here, `validate_other_0_value` and `deserialize_other_0` for `MimeType`. An untagged slot whose type is a brand that declares a bound of its own runs both, the slot's check first.

#### A slot of a tuple struct

A slot of a tuple struct takes the five constraints as a named field of a struct does, and the struct's wire and TypeScript type do not change. `validate()` reports a violation under the slot's position in the array serde writes, the struct's read refuses the value with it, and the Zod schema and the JSON Schema write the checks on the slot's member of the tuple:

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Pair(
    #[model_schema_prop(minLength = 1)] pub String,
    #[model_schema_prop(minimum = 1)] pub u32,
);

Pair(String::new(), 0).validate();
// Err(["'0': too short: minimum length is 1, got 0", "'1': too small: minimum is 1, got 0"])
```

serde refuses `["",0]` with those two messages joined by `; `, reads `["a",1]` as it did, and `Pair("a".to_string(), 1)` still writes `["a",1]`. The Zod schema is `z.tuple([z.string().min(1, { error: ... }), z.number().int().min(1, { error: ... })])`, the JSON Schema writes `{ "type": "string", "minLength": 1 }` and `{ "type": "integer", "minimum": 1.0 }` under `prefixItems`, and the TypeScript type stays `[string, number]`. The helpers are named by the slot's place among the ones declared, `validate_{index}_value` and `deserialize_{index}`: `validate_0_value` and `validate_1_value` here. A slot typed as a list is [counted](#list-length-minlength-maxlength), as a field is.

A struct of one slot is written as that slot's value, so its check is reported under no name: a `Lone(String)` held to `minLength = 1` answers `too short: minimum length is 1, got 0`, and under the key of a field that holds it. A `#[serde(transparent)]` struct of one slot is a [branded newtype](#branded-newtype-validation-constraints), whose checks are written on the type: a `#[model_schema_prop]` on its slot is refused.

#### Constraints under `Option` and wrappers

A constraint describes the field's own value. `validate()` therefore reaches through everything that holds one value -- an `Option`, a transparent wrapper (`Box`, `Rc`, `Arc`, `Cow`), and every reference, whatever it borrows -- and checks the value under them, which is the same place the Zod, TypeScript and JSON Schema surfaces put the bound. A list is a value of its own: the walk stops at it, and [its length](#list-length-minlength-maxlength) is what a key written on it measures.

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Article {
    #[model_schema_prop(minLength = 3)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,   // checked inside the Some; a None is nothing to check

    #[model_schema_prop(minLength = 3)]
    pub tags: Vec<String>,          // at least three tags, each of any length
}
```

### Where a constraint is checked

As the value is read, and by `validate()`.

#### A read refuses what `validate()` refuses

A type that publishes [`validate()`](#the-validate-method) and derives `Deserialize` is read in two steps: serde reads its members, and `validate()` judges the value read. `#[model_schema()]` writes that `Deserialize` itself, so every serde read answers an error for a payload whose value breaks a bound or a [check](#checks-check): `serde_json::from_str`, `serde_json::from_value`, a BSON document under either major of the `bson` library, and any other format. No object is handed back. The error holds what `validate()` answers of the value read, every message, joined by `; `:

```rust
#[model_schema(check = ["withinParcel"])]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parcel {
    pub fragile_item_codes: Vec<String>,
    #[model_schema_prop(minLength = 1)]
    pub item_label: String,
}

#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub boxed_parcel: Box<Parcel>,
    pub first_parcel: Parcel,
    pub parcel_list: Vec<Parcel>,
    pub parcels_by_dock: BTreeMap<String, Parcel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spare_parcel: Option<Parcel>,
}

serde_json::from_str::<Parcel>(r#"{"fragileItemCodes":["x","y"],"itemLabel":""}"#);
// Err: 'itemLabel': too short: minimum length is 1, got 0; 'fragileItemCodes.1': y is outside of itemCodeList
```

A type that holds another reads it with no judgement of its own, and the whole value is judged once, by the type the read was asked for. A manifest that holds that parcel in each of its five places is refused with every message of every parcel, each at its full path. The error is one line, broken here after each `; `:

```text
'boxedParcel.itemLabel': too short: minimum length is 1, got 0;
'boxedParcel.fragileItemCodes.1': y is outside of itemCodeList;
'firstParcel.itemLabel': too short: minimum length is 1, got 0;
'firstParcel.fragileItemCodes.1': y is outside of itemCodeList;
'parcelList.1.itemLabel': too short: minimum length is 1, got 0;
'parcelList.1.fragileItemCodes.1': y is outside of itemCodeList;
'parcelsByDock.north.itemLabel': too short: minimum length is 1, got 0;
'parcelsByDock.north.fragileItemCodes.1': y is outside of itemCodeList;
'spareParcel.itemLabel': too short: minimum length is 1, got 0;
'spareParcel.fragileItemCodes.1': y is outside of itemCodeList
```

The messages come in the order `validate()` reports them: member by member as declared, a type's own check after its members. The text is the same from a string, from a `serde_json::Value` and from a BSON document. What each library writes around it is its own: `serde_json` adds no position to a refusal of the value it was asked for, and version 3 of the `bson` library writes it inside a sentence of its own, `BSON error. Kind: A deserialization-related error occurred. Message: ... .`

- **A refusal serde raises itself is serde's, unchanged.** A parcel holding `"itemLabel": 5` is refused with `invalid type: integer `5`, expected a string at line 1 column 39`, and with the same sentence at its own position where a manifest holds it: no value was read for `validate()` to judge.
- **A payload that keeps every rule** is read to the value it was read to before, and written back to the same text. Writing judges nothing: `Serialize` is still serde's derive, and a broken value built in code is written as it is.
- **A type that publishes no `validate()`** is left as it was written, with serde's own `Deserialize`, unless it names a [`preprocess`](#preprocessing-preprocess), on itself or on a member: its `Deserialize` is written too, and calls each function ahead of the read. A type that derives `Serialize` alone is left as it was written.

**Where a held model is judged.** A holder reads a model unjudged where `validate()` walks it: held itself, or under `Option`, `Vec`, `Box` and as a map's value, in any nesting of those, in a named field, a slot or a `#[serde(flatten)]` member. That holds where the member's type is written with the holder's own type parameters, `entries: Vec<Entry<IdType>>` in a `Ledger<IdType>`, and where it is written with `Self`, `children: Vec<Self>`. A model held any other way judges itself as it is read, and the payload is refused there too, with that model's own messages, the first broken model ending the read:

| Held | Judged |
|---|---|
| in a field, an `Option`, a `Vec`, a `Box`, a map's value, whether its type is written with a concrete filling, with the holder's own type parameter (`Vec<Entry<IdType>>`) or as `Self` | once, with its holder, each message at its full path |
| in a tuple, a fixed array, a set, an `Rc`, an `Arc` | by itself, as it is read |
| filling one of the holder's own type parameters, as what fills `Held` in `held: Vec<Held>` | by itself, as it is read |
| in a member whose type names a lifetime | by itself, as it is read |
| in a member written with a type parameter that a bound of the holder ties to another of its parameters or to a lifetime (`where IdType: From<Held>`), or with one of a holder read at `'static` | by itself, as it is read |
| in a field read through a `deserialize_with` or a `with` of your own | by whatever that function calls |
| by a type with no `#[model_schema()]` | by itself, as it is read: two broken parcels in a `Vec<Parcel>` of such a type answer the first one's messages, `... at line 1 column 58` |

**A branded newtype is read the same way.** A [brand](#branded-newtypes) that carries a `pattern`, a `minLength` or a `maxLength`, or names a [check](#checks-check), publishes `validate()`, and its read is its own read followed by `validate()`. A type that holds the brand reads it with no judgement, in the places of the table above, and names each broken one at its full path:

```rust
#[model_schema(minLength = 2, check = ["labelInUpperCase"])]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Label(pub String);

fn label_in_upper_case(value: &Label) -> Result<(), Vec<String>> {
    if value.0.chars().any(char::is_lowercase) {
        return Err(vec!["a label is upper case".to_owned()]);
    }
    Ok(())
}

#[model_schema(no_display, minLength = 3)]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Dir(pub PathBuf);

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Listing {
    pub listed: Vec<Dir>,
}

serde_json::from_str::<Label>(r#""a""#);
// Err: too short: minimum length is 2, got 1; a label is upper case

serde_json::from_str::<Listing>(r#"{"listed":["/a","/etc","/b"]}"#);
// Err: 'listed.0': too short: minimum length is 3, got 2; 'listed.2': too short: minimum length is 3, got 2
```

A brand that carries no bound and names no check publishes no `validate()`, and keeps the `Deserialize` serde's derive writes it. One position reads a brand held to its bound while the members are still being read: [a member of an `#[serde(untagged)]` enum](#what-is-checked-while-the-members-are-read).

**What `validate()` is for.** A value built in code, and a value changed after it was read:

```rust
let mut parcel: Parcel = serde_json::from_str(payload)?;
parcel.fragile_item_codes.push("y".to_string());
parcel.validate();
// Err(["'fragileItemCodes.1': y is outside of itemCodeList"])
```

**What the macro writes.** `#[derive(Deserialize)]` is taken off the item, and serde's derive runs on a private copy of it, declared inside a function the type gains: `model_schema_read_unjudged`, the read alone, public so that a holder in another module can call it. The `Deserialize` of the type calls it and then `validate()`. The copy keeps the item's `serde` and `cfg` attributes and no other, so an attribute of another macro written on the item or on a member is not on the copy.

- `model_schema_read_unjudged` is a name of the type's own: a function of yours under that name on the same type is a duplicate definition (E0592). A type whose [`enable` list](#opting-in) holds `"decode_with"` gains `decode_with_read_unjudged` beside it.
- A brand that carries a `pattern`, a `minLength` or a `maxLength` gains `model_schema_validate_bound` beside `validate()`: what the bound alone answers of the value, the checks the brand names apart. A member of an `#[serde(untagged)]` enum is held to it as it is read, and the recovering decode lists the brand from a read held to it. It is a name of the type's own as well.
- The `Deserialize` is written under the bounds serde's derive writes. A container `#[serde(bound = ...)]` is used as written. Without one, each type parameter a read field names is asked `Deserialize<'de>`, a parameter used only in `PhantomData`, in a skipped field or in a field read through `with` or `deserialize_with` is asked nothing to read, a `default` asks `Default` of what it builds, and `'de` outlives each lifetime serde borrows for: a `&'a str`, a `&'a [u8]`, and what a `#[serde(borrow)]` names. A type that borrows for `'static` is read at `'static`.
- A unit struct with a check gains the `validate()` call in the `Deserialize` the macro already writes for it.
- `#[serde(remote = ...)]`, `from`, `try_from` and `into` on a type that publishes `validate()` and derives `Deserialize` are refused, the macro having no place to put a judgement behind a conversion it does not write:

```text
error: model_schema: type `Sealed`: `#[serde(try_from = ...)]` puts a conversion of its author's between the type and what serde reads or writes for it, and `Sealed` publishes `validate()`, so `#[model_schema()]` writes its `Deserialize`: the read of its members, then `validate()`. It cannot put that judgement behind a conversion it does not write. Take `try_from` off the type and convert where the value is read, or take off the bounds and the checks that make the type publish `validate()`.
```

#### What is checked while the members are read

A field still generates both helpers into its schema module: `validate_{field}_value()`, which `validate()` calls, and `deserialize_{field}`, a serde hook an author may hang on a field of their own accord. A field read through a reference generates the validator alone. Two positions are checked while the members are read, ahead of the type's `validate()`, and both have to be:

- **A member of an `#[serde(untagged)]` enum**, a struct variant's named member and a newtype variant's slot alike, where whether the member is admissible is what chooses which variant the payload is. The check is part of reading the value rather than part of judging it -- exactly as it is under `anyOf` and `z.union` on the two schema surfaces the same type publishes -- and `validate()` cannot stand in for it, since by the time it runs the variant has already been chosen. A wrapped member's hook deserializes the member's own declared type and then runs the same walk `validate()` runs over it; the two differ in that `validate()` answers with every violation in the instance while a `Deserializer` answers with one line, so the read stops at the first value that broke a bound and hands serde that value's violations joined by `; `. A member read through a reference, a borrowed `&str` or a borrowed list among them, and a member its author reads through a `deserialize_with` or a `with` of their own, serde admitting one reader per field, are the constrained members that choose no variant: each is read with no bound written, the first variant whose shape fits taking the payload, and the enum's read then refuses the value where it breaks the bound.
- **A constrained brand held by a member of an `#[serde(untagged)]` enum**, where the same reasoning applies: the member is held to the brand's `pattern`, `minLength` and `maxLength` as it is read, so a value the brand's bound refuses takes that variant out of the running. A [check](#checks-check) the brand names chooses no variant, as no check does: the variant is chosen first, and the enum's read then refuses the value with the check's messages. Held anywhere else, a brand is read with no judgement, as a model is, and the holder's `validate()` judges it.

The [recovering decode](#recovering-decode-decode_with) lists what a plain read refuses. `from_value_with` and `from_value_piped`, `from_bson_with` and `from_bson_piped` read with the type's read alone, run each field's validator on the value serde read, and the validator of each slot of a tuple struct and of a tagged tuple variant, and list the violations of each value as one issue, which is what lets a callback or a resolver repair a stored value. The [MongoDB reads](#rows-that-do-not-read-as-the-type) of a type that lists `"mongodb"` read each row the same way, so a row that breaks a bound is refused until a resolver repairs it. See [What each shape reports](#what-each-shape-reports). A [service's dispatcher](#service-validation) reads a request with the read alone as well and calls `validate()` next, so a broken bound is a `failed-validation` fault with its field.

A field whose key may be left out keeps that reading. Where a member is read through a hook -- an untagged variant's, and a member whose model is read unjudged -- an `Option` written outermost (under any number of transparent wrappers) is given `#[serde(default)]` alongside it, since a `deserialize_with` otherwise turns a missing key into an error; a field that writes its own `default` keeps the one it wrote. Off the hook there is nothing to put back and no `default` is written, so a required key that goes missing is still the error it always was.

#### A bound the field's own type declares

A field carrying no `model_schema_prop` of its own may still hold a type that declares one -- a constrained brand, or a nested `#[model_schema()]` type. The enclosing `validate()` runs whatever validator that type published and writes its report under the field that held it:

```rust
#[model_schema(minLength = 3)]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Slug(pub String);

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Enrolment {
    pub slug: Slug,
}
```

```rust
Enrolment { slug: Slug("a".to_string()) }.validate();
// Err(["'slug': too short: minimum length is 3, got 1"])
```

The brand's own report says `too short: ...` and names nothing, the brand being the value rather than a field of anything -- so the field's key is the message's to supply, written first and in single quotes, which is where a reader of these reports already looks for one: `'slug': too short: ...`. A nested type's report already names its own member, and the field's key is written *into* that name rather than in front of it: `'holds.name': too short: ...`. One quoted run carries the whole path, so a reader taking the first one gets the member that was actually wrong rather than the hop above it -- and it is the string the TypeScript schema published from the same declaration reports for the same payload.

A `#[serde(flatten)]` hop writes no key, so it contributes no segment: a bound under a flattened `claims` inside `account` reads `'account.jti': too short: ...`, which is what the wire carries and what TypeScript names.

This reaches through `Option`, `Box`/`Cow`/`Rc`/`Arc`, every reference and every level of a list, a fixed array included, to each item a list holds -- beside the count a key written on the list holds it to, and with the item's [index](#the-path-a-message-names) written after the field's key at each level -- and applies to a struct's field, to a named member of an enum variant under any tagging, and to the lone slot of an `#[serde(untagged)]` newtype variant. That last one contributes no segment either, for the same reason a flattened hop contributes none: what an untagged newtype member writes on the wire *is* the inner value, so a violation beneath it is already one of the enclosing object's own keys. So a union of newtype members publishes a `validate()` that dispatches to whichever variant the value holds, and a field holding such a union reports the path the payload spells -- `'account.jti': too short: ...` whether the account is a union or a plain struct.

#### A model held in a map, in a slot or in a tuple

`validate()` runs what each of these holds as well, and names it by the place the payload holds it at:

| Held | Reported under |
|---|---|
| as a value of a `HashMap`, of a `BTreeMap` and, under the [`indexmap` feature](#feature-flags), of an `IndexMap` | the entry's key, as serde writes it |
| in a slot of a tuple struct, and of a tuple variant of a tagged enum | [the path serde writes the slot at](#the-path-a-message-names), as the check [written on the slot itself](#a-slot-of-a-tuple-struct) is: its position among several slots, nothing for the one slot of a struct, and the variant's name or the content key ahead of either |
| in a tuple written as a field's type, `(String, Parcel)` | its position in the tuple |

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parcel {
    #[model_schema_prop(minLength = 1)]
    pub item_label: String,
}

#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Terminal {
    pub first_parcel: Parcel,
    pub parcels_by_dock: BTreeMap<String, Parcel>,
}

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Consignment(#[model_schema_prop(minLength = 1)] pub String, pub Parcel);

let unlabelled = || Parcel { item_label: String::new() };

Terminal {
    first_parcel: Parcel { item_label: "A".to_string() },
    parcels_by_dock: BTreeMap::from([("north".to_string(), unlabelled())]),
}
.validate();
// Err(["'parcelsByDock.north.itemLabel': too short: minimum length is 1, got 0"])

Consignment(String::new(), unlabelled()).validate();
// Err(["'0': too short: minimum length is 1, got 0", "'1.itemLabel': too short: minimum length is 1, got 0"])
```

The walk goes on below each of the three as it does below a field: through an `Option`, the transparent wrappers, every reference and every level of a list, and from a map into a list or another map. A model held behind a reference -- `&'a Parcel`, `Option<&'a Parcel>`, `&'a [Parcel]` or `&'a Option<Parcel>` -- is walked as the owned model is and named by the same path: a `first_parcel: &'a Parcel` answers `'firstParcel.itemLabel': too short: minimum length is 1, got 0`. serde reads no reference to a model, so no read hook is written for such a member. The entries of a map are reported in the order the map yields them: sorted for a `BTreeMap`, as inserted for an `IndexMap`, and in no fixed order for a `HashMap`. An empty map reports nothing. A map, a slot and a tuple that hold no declared type add nothing to `validate()`, and a type that holds nothing else publishes none.

**A key is the text serde writes for it**, read through the key's own `Serialize` with the serializer serde has kept for a `Formatter` since 1.0.108: a string as it is, a number and a `bool` as serde stringifies them, a plain enum under the name serde writes for the variant, a [brand](#branded-newtypes) as its inner value, and a `chrono::NaiveDateTime` as serde writes it, `2026-10-13T04:17:00`, where the type prints `2026-10-13 04:17:00`. A map written as the whole value -- a `#[serde(flatten)]` map, and the lone slot of an `#[serde(untagged)]` variant -- names each entry by its key alone, `'north.itemLabel': ...`.

`validate()` is written under the bounds the type declares, so a key typed with one of the type's own parameters is written only where the type declares `Serialize` for it. `struct Rack<Key: Ord + Serialize>` over a `by_key: BTreeMap<Key, Parcel>` answers `'byKey.north.itemLabel': ...`, and `struct Shelf<Key: Ord>` runs each parcel as well and names the field alone, `'byKey.itemLabel': ...`.

A brand's rules and a nested `#[model_schema()]` type's bound are both the holder's `validate()` to report. The holder reads each with no judgement of its own, and its read then runs `validate()` over the whole value, which is also what refuses a message built in Rust, where no read ever ran, as every outbound message a generated client sends is.

### Preprocess and check

Two keys name functions of your own. [`preprocess`](#preprocessing-preprocess) names functions that correct a payload before a value is read from it. [`check`](#checks-check) names functions that judge a value. Each key is written on a type, in `#[model_schema(...)]`, and on a field, in `#[model_schema_prop(...)]`. A name is written once, in the attribute, and you write one function under it in each language the crate builds for. Rust calls the snake case of the name, and TypeScript, Dart, Swift and Kotlin call the name as written.

**`preprocess` is a last resort.** It is not encouraged. It is for a payload that cannot be read otherwise. A function of yours runs on every read of the type, on a payload no schema describes, and you write it again in each language that reads the type.

This section states the two keys for Rust, TypeScript with Zod, Dart, Swift and Kotlin side by side. The detail of each language is in the section of that language, which this section names.

```rust
#[model_schema(check = ["withinParcel"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Parcel {
    pub fragile_item_codes: Vec<String>,
    #[model_schema_prop(check = ["withoutDuplicates"])]
    pub item_code_list: Vec<String>,
    #[model_schema_prop(minLength = 1, preprocess = ["trimmed"])]
    pub label: String,
}
```

`trimmed` corrects the payload of `label`, `withoutDuplicates` judges `item_code_list`, and `withinParcel` judges the whole parcel. A Rust read of the parcel calls `trimmed`, builds the value, and judges it with the bound and the two checks:

```rust
serde_json::from_str::<Parcel>(r#"{"fragileItemCodes":["x"],"itemCodeList":["x"],"label":"  A  "}"#);
// Ok: `label` is "A"

serde_json::from_str::<Parcel>(r#"{"fragileItemCodes":["y"],"itemCodeList":["x","x"],"label":"   "}"#);
// Err: 'itemCodeList': duplicate item code x; 'label': too short: minimum length is 1, got 0; 'fragileItemCodes.0': y is outside of itemCodeList
```

#### What each language calls

| | A `preprocess` function is called in | The function a `preprocess` names | A `check` function is called in | The function a `check` names | A refusal is | A name with no function is |
|---|---|---|---|---|---|---|
| Rust | `model_schema_read_unjudged`, the read alone, which the `Deserialize` of the type reads through | `fn trimmed(payload: serde_json::Value) -> serde_json::Value` | `validate()`, which the `Deserialize` of the type calls on the value it read | `fn without_duplicates(value: &[String]) -> Result<(), Vec<String>>` on a field, `fn within_parcel(value: &Parcel) -> Result<(), Vec<String>>` on a type | the error of the serde read, which holds the messages joined by `; ` | ``error[E0425]: cannot find function `within_parcel` in this scope`` |
| TypeScript with Zod | one `z.preprocess(trimmed, ...)` per name, around the schema | `function trimmed(payload: unknown): unknown` | one `.check((ctx) => { ... })` per name, on the schema | `function withoutDuplicates(value: Array<string>, ctx: ParsePayload<Array<string>>): void` on a field, `function withinParcel(value: Parcel, ctx: ParsePayload<Parcel>): void` on a type | a `ZodError`, which `parse` throws and `safeParse` answers under `error`, with one issue per broken rule | `error TS2304: Cannot find name 'withinParcel'.` |
| Dart | `fromJsonUnjudged`, the read alone, which `fromJson` reads through | `dynamic trimmed(dynamic payload)` | `validate()`, which `fromJson` calls on the value it read | `List<String> withoutDuplicates(List<String> value)` on a field, `List<String> withinParcel(Parcel value)` on a type | a `FormatException`, whose `message` holds the messages joined by `; ` | `Error: Method not found: 'trimmed'.` for a `preprocess`, `Error: The method 'withinParcel' isn't defined for the type 'Parcel'.` for a `check` |
| Swift | `init(swiftSchemaUnjudged:)`, the read alone, which `init(from:)` reads through | `func trimmed(_ payload: Any) -> Any` | `validate()`, which `init(from:)` calls on the value it read | `func withoutDuplicates(_ value: [String]) -> [String]` on a field, `func withinParcel(_ value: Parcel) -> [String]` on a type | `DecodingError.dataCorrupted`, whose `debugDescription` holds the messages joined by `; ` | `error: cannot find 'withinParcel' in scope` |
| Kotlin | `{Type}Unjudged`, the serializer that reads alone, which `{Type}Serializer` reads through | `fun trimmed(payload: JsonElement): JsonElement` | `validate()`, which `{Type}Serializer` calls on the value it read | `fun withoutDuplicates(value: List<String>): List<String>` on a field, `fun withinParcel(value: Parcel): List<String>` on a type | a `SerializationException`, whose message holds the messages joined by `; ` | `error: unresolved reference 'withinParcel'.` |

- **A function a `preprocess` names takes a payload and answers a payload**, on a field and on a type alike. A field's function takes the payload of the field, and a type's function takes the whole payload.
- **A function a `check` names takes a value and answers its messages.** A field's function takes the field's own value, and a type's function takes the whole value.
- **A name with no function is one error for the two keys**, in every language but Dart, which reports each in its own words.
- **`ParsePayload` is zod's own type**, which `zod/v4/core` exports.

Where each row is stated in full:

| | The functions of a `preprocess` | The functions of a `check` and `validate()` | The refusal |
|---|---|---|---|
| Rust | [Preprocessing](#preprocessing-preprocess) | [Checks](#checks-check), [The `validate()` Method](#the-validate-method) | [A read refuses what `validate()` refuses](#a-read-refuses-what-validate-refuses) |
| TypeScript with Zod | [The order of one read](#the-order-of-one-read) | [Where a check runs](#where-a-check-runs) | [The messages](#the-messages) |
| Dart | [A `preprocess` in the Dart client](#a-preprocess-in-the-dart-client) | [`validate()` in Dart](#validate-in-dart) | [The Dart read refuses what `validate()` refuses](#the-dart-read-refuses-what-validate-refuses) |
| Swift | [`preprocess` in the Swift client](#preprocess-in-the-swift-client) | [`validate()` in the Swift client](#validate-in-the-swift-client) | [A Swift read refuses what `validate()` refuses](#a-swift-read-refuses-what-validate-refuses) |
| Kotlin | [`preprocess` in the Kotlin client](#preprocess-in-the-kotlin-client) | [`validate()` in the Kotlin client](#validate-in-the-kotlin-client) | [The read in the Kotlin client](#the-read-in-the-kotlin-client) |

**Each client has limits of its own**, and the section of the client states them: the typed payload of a type's function in Dart, a bounded brand declared below an `#[serde(untagged)]` enum and a type declared below the type that reads it from its own decoder in Swift, and a model declared below its holder, the read of a plain enum and a member that reaches a type parameter in Kotlin.

#### The six shared cases

Six values run the bound and the two checks of the parcel. Cases 1 to 5 are parcels, and case 6 is a `Manifest` that holds the parcel of case 4 under `parcel`:

| Case | The payload |
|---|---|
| 1 | `{"label":"A","itemCodeList":["x"],"fragileItemCodes":["x"]}` |
| 2 | `{"label":"","itemCodeList":["x"],"fragileItemCodes":["x"]}` |
| 3 | `{"label":"A","itemCodeList":["x","x"],"fragileItemCodes":["x"]}` |
| 4 | `{"label":"A","itemCodeList":["x"],"fragileItemCodes":["x","y"]}` |
| 5 | `{"label":"","itemCodeList":["x","x"],"fragileItemCodes":["y"]}` |
| 6 | `{"parcel":{"label":"A","itemCodeList":["x"],"fragileItemCodes":["x","y"]}}` |

Every language prints the same 13 lines for them:

```text
case 1: passes
case 2: refused
    'label': too short: minimum length is 1, got 0
case 3: refused
    'itemCodeList': duplicate item code x
case 4: refused
    'fragileItemCodes.1': y is outside of itemCodeList
case 5: refused
    'itemCodeList': duplicate item code x
    'label': too short: minimum length is 1, got 0
    'fragileItemCodes.0': y is outside of itemCodeList
case 6: refused
    'parcel.fragileItemCodes.1': y is outside of itemCodeList
```

Rust, Dart, Swift and Kotlin print them from the `validate()` of each value, and TypeScript from the issues the Zod schema answers for each payload. This repository holds the five to the lines with the parcel of [`validate()` in Dart](#validate-in-dart), which declares the members of the parcel above in the same order, with the bound and the two checks and no `preprocess`. The Rust `validate()` of the parcel above answers the same lines. A member's lines come in the order the parcel declares its members.

#### What holds in every language

- **One name, written once.** The attribute holds the name, Rust calls its snake case, `withinParcel` as `within_parcel`, and every other language calls the name as written. A name is an identifier: ASCII letters, digits and `_`, not led by a digit.
- **The order of one read is the same.** The type's `preprocess` functions on the whole payload, each field's as that field is read, then the value is built, then its bounds and its checks judge it. A bound and a check judge what the functions answered.
- **Several functions of one key run in the order written.** Each `preprocess` function is called over the answer of the one before, and every `check` runs: none stops another, and none is stopped by a broken bound.
- **A message is led by the path of the value in the payload**, in single quotes: the keys serde writes and the index of each item, joined by dots, `'parcel.fragileItemCodes.1'`. A field's `check` names no path, and its messages are written under the field. A type's `check` leads each message with the path of what it refuses.
- **A `preprocess` is not called for a key the payload does not hold**, in Rust, Dart, Swift and Kotlin. The Zod schema calls the field's function for such a key, with `undefined`, or with the default of a key under `#[serde(default)]` ([A Key Read as Its Default](#a-key-read-as-its-default-serdedefault)). A `null` the payload holds is a payload, and every language hands it to the function.
- **`validate()` is on every type the Dart, Swift and Kotlin emitters write.** In Rust a type has it where a field carries a bound, where the type or a field names a `check`, and where it holds a type that has one. It judges a value built or changed in code.
- **A read refuses what `validate()` refuses**, in Rust, Dart, Swift and Kotlin: the read of a type is its own read alone, followed by its `validate()`, and no value is handed back for a payload that breaks a rule. The refusal holds every message, joined by `; `.
- **The whole value is judged once.** A type that holds another reads it with no judgement, and the type the read was asked for judges the whole value, each message at its full path. A value behind a type parameter is not reached by the `validate()` of its holder: it judges itself as it is read, with its own messages.
- **The Dart, Swift and Kotlin service clients answer their fault for a reply that breaks a rule**, with the messages in the fault's detail: `undeserializable-payload` from an `http_rest` client, and `failed-validation` from a `ws_rpc` client. Dart spells the two `undeserializablePayload` and `failedValidation`.

#### What an upgrade asks of your crate

Eighteen rules meet a crate that upgrades. Each is stated with what a build or a read answers:

- **A map under `#[serde(flatten)]` beside another member that writes a key stops the build**, on a struct and on a variant of an enum: `pub struct Stocktake { #[serde(flatten)] pub counts: BTreeMap<String, u8>, pub note: String }` answers ``error: model_schema: type `Stocktake`: field `counts`: `#[serde(flatten)]` cannot hold a map beside other members — serde writes the map's entries at one level with `note`, and no TypeScript type describes both. Write the map under a key of its own.`` Write the map under a key of its own, `pub counts: BTreeMap<String, u8>`. See [Intersection Types](#intersection-types-serdeflatten).
- **A map keyed by a number or a `bool` below a `#[serde(flatten)]` member stops the build**, at any depth, on a struct and on a variant of an enum, as does a type declared above that holds one there, and a type parameter read there filled with a number, a `bool` or a map keyed by one: serde reads no entry of such a map back. `pub struct Soundings { #[serde(flatten)] pub rest: BTreeMap<isize, u8> }` answers ``error: model_schema: type `Soundings`: field `rest`: `#[serde(flatten)]` cannot hold a map keyed by `isize` — serde reads every key below a flattened member as text, and reads no number and no `bool` from text, so no entry of that map reads back. Key the map by a `String`, or write the field under a key of its own.`` Key the map by a `String`, or write it under a key of its own, `pub rest: BTreeMap<isize, u8>`. See [Intersection Types](#intersection-types-serdeflatten).
- **A map keyed by a number or a `bool` in an untagged enum, or in an enum under a `tag` with no `content`, stops the build**, at any depth of a member of a variant, as does a type declared above that holds one there, and a type parameter read there filled with a number, a `bool` or a map keyed by one: serde reads such an enum from a copy of the payload it buffers, and reads no entry of such a map back. `#[serde(untagged)] pub enum Leadlines { Lines(BTreeMap<u32, u8>) }` answers ``error: model_schema: type `Leadlines`: variant `Lines`, field `0`: an untagged enum cannot hold a map keyed by `u32` — serde reads an untagged enum from a copy of the payload it buffers, where every key is text, and reads no number and no `bool` from text, so no entry of that map reads back. Key the map by a `String`.`` Key the map by a `String`, or, in an enum under a `tag`, name a `content` key. See [Intersection Types](#intersection-types-serdeflatten).
- **A map keyed by a `char`, a date, a number or a `bool` describes its values in the JSON Schema**, at every position, and names its keys by the text serde writes them as, in `propertyNames`: `HashMap<u32, u8>` is `{"type": "object", "additionalProperties": {"type": "integer"}, "propertyNames": {"type": "string", "pattern": "^-?[0-9]+(\\.0+)?$"}}`, where it was `{"type": "object", "additionalProperties": true}`, and refuses `{"7": "x"}`, as serde and Zod refuse it. A tuple as a map's value is described as the fixed array it is anywhere else, where the build stopped (`a tuple is not supported as a map value`), and a [brand](#branded-newtypes) over a `char` as `{"type": "string", "minLength": 1, "maxLength": 1}`, where it was `{"type": "string"}`. See [Collections and Maps](#collections-and-maps).
- **A map under `#[serde(flatten)]` alone beside a tag stops the build**, in a struct with its own `#[serde(tag = "...")]` and in a variant of an enum under a `tag` with no `content`: `#[serde(tag = "kind")] pub enum Keelson { Open { #[serde(flatten)] counts: BTreeMap<String, u8> } }` answers ``error: model_schema: type `Keelson`: variant `Open`, field `counts`: `#[serde(flatten)]` cannot hold a map beside the tag `kind` — serde writes the tag and the map's entries at one level, and no TypeScript type describes both. Write the map under a key of its own.`` Write the map under a key of its own, or, in a variant, name a `content` key. See [Intersection Types](#intersection-types-serdeflatten).
- **A newtype variant under a `tag` with no `content` whose value writes a map stops the build**: a struct whose one member is a flattened map, a brand or a struct of one slot over a map, an alias of a map and an untagged enum with a variant that writes one, declared above the enum. With `pub struct Tally { #[serde(flatten)] counts: BTreeMap<String, u8> }`, `#[serde(tag = "kind")] pub enum Filled { Counted(Tally) }` answers ``error: model_schema: type `Filled`: variant `Counted`, field `0`: `Tally` writes a map, which cannot sit beside the tag `kind` — serde writes the tag and the map's entries at one level, and no TypeScript type describes both. Name a `content` key so the map gets an object of its own, or write the map under a key of its own.`` A variant that holds the enum's own type parameter is refused where a map fills it. Name a `content` key, or write the map under a key of its own. See [Internally Tagged Enums](#internally-tagged-enums-tag-with-no-content).
- **A map that reaches a flattened member through an untagged enum, a generic type or a type parameter stops the build** beside another member that writes a key, as a map written there does: an `#[serde(untagged)]` enum with a map among its variants, and a brand, an alias or a struct over its own type parameter filled at a map, are refused with the error of a map written there, and a type parameter flattened beside a member is refused where a map fills it: with `Letter<T>` flattening `T` beside `id`, `pub struct Post { pub letter: Letter<HashMap<String, i32>> }` answers ``error: model_schema: type `Post`: field `letter`: `Letter` flattens its type parameter `T` beside `id`, and `HashMap<String, i32>` fills it with a map — serde writes the map's entries at one level with `id`, and no TypeScript type describes both. Write the map under a key of its own.`` Write the map under a key of its own. See [Intersection Types](#intersection-types-serdeflatten).
- **A member that holds `serde_json::Value` or `serde_json::Map` stops the build**, at any depth: a `pub v: serde_json::Value` on `Holding` answers ``error: model_schema: type `Holding`: field `v` reaches `serde_json::Value`, which holds any JSON value, so no target can describe it. Declare the shape that value takes as a type of its own, and write that type in its place.`` A bare `Value` or `Map<K, V>` that names `serde_json`'s type stops it at a check the compiler evaluates beside the item, with `error[E0080]: evaluation panicked:` and the same message, and a crate that writes either bare name in a member lists `serde_json` among its dependencies, which the check names. A service operation whose message, reply, fault, header or part reaches one stops the build too, the error naming the operation and the position. See [A JSON value is no member](#a-json-value-is-no-member).
- **A plain enum under `#[serde(flatten)]` stops the build with `serde` and no schema surface**, as it stops one with a surface: a build of the Dart, Swift and Kotlin clients alone took it, and no client wrote or read it as serde does. With `pub enum Weather { Calm, Gale }` declared above it, `pub struct Logged { pub note: String, #[serde(flatten)] pub weather: Weather }` answers ``error: model_schema: field `weather` of `Logged` carries `#[serde(flatten)]` over `Weather`, which serde does not write as an object: a plain enum writes its own variant name, so it contributes no members to the object being written — serde writes that name as a key holding null, which a schema closed around the remaining fields rejects. Write the field as a named member so the value gets a key of its own, or flatten a type serde writes as an object. This refusal holds in every build with serde, and only a plain enum declared above reaches it: one declared below, or in another crate, is not refused here, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.`` An alias of a plain enum is refused the same way, and so is a variant that holds one beside the tag `#[serde(tag = "...")]` writes. So are a [brand](#branded-newtypes) or a struct of one slot over a plain enum, a generic type declared above filled at one (`ProbeWrap<ProbeMood>` over `pub struct ProbeWrap<T>(T);`), a type whose value is one of those, and, flattened, an `Option` of any of them or in any of them, each named as written: ``carries `#[serde(flatten)]` over `ProbeWrap<ProbeMood>`, which serde does not write as an object``. They were refused with the error of a value Serde writes as no object, which said what Serde writes does not join the object, or cannot sit beside the tag; Serde writes each as the key naming the variant, holding `null`, and reads it back. A newtype variant beside the tag over one that writes an `Option` of a plain enum on the way is refused with the error of an `Option`. See [Intersection Types](#intersection-types-serdeflatten) and [Internally Tagged Enums](#internally-tagged-enums-tag-with-no-content).
- **A flattened type serde writes as no object stops the build with `serde` and no schema surface**, as it stops one with Zod on: a build of the Dart, Swift and Kotlin clients alone, and one with TypeScript or the JSON Schema alone, took it, and serde refuses every read of a brand over text and of a tuple struct flattened. A brand or a struct of one slot over text, a number, a `bool` or a list, a tuple struct, an alias of one, and an `#[serde(untagged)]` enum with such a member, declared above the type that flattens it, are refused, and so are a generic brand, struct of one slot or alias declared above whose value is its own type parameter, filled at one of those, and a type declared above whose value is one: `#[serde(flatten)] pub wrap: ProbeWrap<String>` over `pub struct ProbeWrap<T>(T);` is refused. With `#[serde(transparent)] pub struct RecordId(String);` declared above it, `pub struct Carded { #[serde(flatten)] pub id: RecordId, pub name: String }` answers ``error: model_schema: `Carded`: `#[serde(flatten)]` of `RecordId` is not written as an object — its schema describes a `string`, which has no members to merge, and what serde writes for it does not join the object being written; write the field as a named member so the value gets a key of its own. This refusal holds in every build with serde, and only a type declared above reaches it: one declared below, or in another crate, is not refused here, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.`` An untagged enum is refused with the branch of the member named. A member Serde writes as a plain enum's variant name is a key holding `null` in the object, which Serde reads back: with `pub enum Mood { Calm, Tense }` and `#[serde(untagged)] pub enum Feeling { Mood(Mood), Named { name: String } }` declared above it, Serde writes `{"Calm":null,"id":"i"}` for `pub struct Felt { #[serde(flatten)] pub feeling: Feeling, pub id: String }`, which answers ``error: model_schema: `Felt`: `#[serde(flatten)]` of `Feeling` writes a union member as a plain enum's variant name — its branch 1 describes a `string`, and serde writes that name as a key holding null, which a schema closed around the remaining fields rejects; write the field as a named member so the value gets a key of its own. This refusal holds in every build with serde, and only a plain enum declared above reaches it: one declared below, or in another crate, is not refused here, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.`` A member that names a generic type of those shapes, filled, is read at its filling: `Text(ProbeWrap<String>)` is refused as `Text(String)` is. A member that is the enum's own type parameter is read at the enum's argument: `ProbeOwnMember<String>`, with the member `Held(X)`, is refused as an enum with `Held(String)` is, and named with its arguments. A brand or a struct of one slot over such an enum is refused as an alias of it is: `ProbeSlotText`, over `ProbeTextMember` with the member `Text(String)`, is refused with that member's branch. A generic alias, struct of one slot or brand whose value is `ProbeOwnMember<T>`, filled at its own parameter, is read at what fills it: `ProbeOwnAliasOf<String>`, over `pub type ProbeOwnAliasOf<T> = ProbeOwnMember<T>;`, is refused as `ProbeOwnMember<String>` is, and named with its arguments. Write the field as a named member. See [Intersection Types](#intersection-types-serdeflatten).
- **A newtype variant under a `tag` with no `content` whose value serde writes as no object stops the build**, in every build with `serde`: a brand or a struct of one slot over text, a number, a `bool` or a list, a tuple struct, an alias of one, and an `#[serde(untagged)]` enum with such a member, declared above the enum, and a generic brand, struct of one slot or alias declared above whose value is its own type parameter, filled at one of those, and a type declared above whose value is one: `Held(ProbeWrap<String>)` over `pub struct ProbeWrap<T>(T);`, `Held(ProbeTextFilled)` over `pub struct ProbeTextFilled(ProbeWrap<String>);`, `Held(ProbeWrapMember)` over an untagged `ProbeWrapMember` with the member `Text(ProbeWrap<String>)`, `Held(ProbeOwnMember<String>)` over an untagged `ProbeOwnMember<X>` with the member `Held(X)`, `Held(ProbeSlotText)` over `pub struct ProbeSlotText(ProbeTextMember);` with the member `Text(String)`, and `Held(ProbeOwnAliasOf<String>)` over `pub type ProbeOwnAliasOf<T> = ProbeOwnMember<T>;`, are refused. serde refuses every write and every read of a brand over text and of a tuple struct there. With `#[serde(transparent)] pub struct ProbeId(String);` declared above it, `#[serde(tag = "kind")] pub enum ProbeReading { Held(ProbeId) }` answers ``error: model_schema: variant `Held` of internally tagged enum `ProbeReading` wraps `ProbeId`, which is not written as an object — its schema describes a `string`, which cannot sit beside the tag: `#[serde(tag = "kind")]` with no `content` writes the variant's data as members of the object the tag is written in, and only a value serde writes as an object has members to put there. Name a `content` key so the value gets an object of its own, or wrap it in a struct whose fields can sit beside the tag. This refusal holds in every build with serde, and only a type declared above reaches it: one declared below, or in another crate, is not refused here, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.`` Name a `content` key so the value gets an object of its own, or wrap it in a struct whose fields can sit beside the tag. See [Internally Tagged Enums](#internally-tagged-enums-tag-with-no-content).
- **A newtype variant under a `tag` with no `content` whose value serde writes as an `Option` stops the build**, in every build with `serde`: a struct of one slot or a brand over an `Option`, and an alias of one, declared above the enum, and a generic brand, struct of one slot or alias declared above whose value is its own type parameter, filled at an `Option` or at one of those, and a type declared above whose value is one: `Held(ProbeWrap<Option<ProbeFields>>)`, and `Held(ProbeMaybeFilled)` over `pub struct ProbeMaybeFilled(ProbeWrap<Option<ProbeFields>>);`, are refused. serde refuses every write of it, of `Some` and of `None` alike. With `pub struct ProbeMaybe(Option<ProbeFields>);` declared above it, `#[serde(tag = "kind")] pub enum ProbeOptional { Held(ProbeMaybe) }` answers ``error: model_schema: variant `Held` of internally tagged enum `ProbeOptional` wraps `ProbeMaybe`, which serde writes as an optional, which cannot sit beside the tag: `#[serde(tag = "kind")]` with no `content` writes the variant's data as members of the object the tag is written in, and only a value serde writes as an object has members to put there. serde refuses this one at run time — `cannot serialize tagged newtype variant ProbeOptional::Held containing an optional`. Name a `content` key so the value gets an object of its own, or wrap it in a struct whose fields can sit beside the tag. This refusal holds in every build with serde, and only a type declared above reaches it: one declared below, or in another crate, is not refused here, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.`` Name a `content` key so the value gets an object of its own, or wrap it in a struct whose fields can sit beside the tag. An `Option` of a value Serde writes as no object, `pub struct ProbeMaybeCountSlot(Option<u32>);`, is refused with this error, where it was refused with the error of the branch Serde writes as no object, and one declared below the enum makes `json_schema()` panic, where no surface refused it. See [Internally Tagged Enums](#internally-tagged-enums-tag-with-no-content).
- **`()` is described as `null`**: TypeScript writes `null` where it wrote `[]`, Zod `z.null()` where it wrote `z.tuple([])`, and the JSON Schema `{"type": "null"}` where it wrote an array of no item, wherever `()` is held. Serde writes `()` as `null` and refuses `[]` (`invalid type: sequence, expected unit`), so a payload a client built from the old description is refused by a Rust read. A brand over `()` is `null & $brand<"Name">`. See [The Unit `()`](#the-unit-).
- **A flattened value written in place that Serde writes as no object stops the build**, in every build with `serde`: text, a number, a `bool`, a list, a tuple or `()`, under an `Option` or not, built, and Serde refuses every write of the first five flattened (`can only flatten structs and maps (got a tuple)`). `pub struct Host { pub id: String, #[serde(flatten)] pub pair: (String, u32) }` answers ``error: model_schema: `Host`: `#[serde(flatten)]` of `(String, u32)` is not written as an object — its schema describes a `array`, which has no members to merge, and what serde writes for it does not join the object being written; write the field as a named member so the value gets a key of its own. This refusal holds in every build with serde.`` Write the field as a named member. See [Intersection Types](#intersection-types-serdeflatten).
- **Every language the crate builds for calls the function a `preprocess` names**, on a field and on a type. A build stops until a function under that name is in scope in that language:

  | | The build answers |
  |---|---|
  | Rust | ``error[E0425]: cannot find function `without_fragile` in this scope`` |
  | TypeScript with Zod | `error TS2304: Cannot find name 'trimmed'.` |
  | Dart | `Error: Method not found: 'trimmed'.` |
  | Swift | `error: cannot find 'trimmed' in scope` |
  | Kotlin | `error: unresolved reference 'trimmed'.` |

  Rust calls the snake case of the name, so `preprocess = ["withoutFragile"]` asks for `fn without_fragile(payload: serde_json::Value) -> serde_json::Value`.
- **A read refuses a payload that breaks a bound or a check**, in Rust and in the Dart, Swift and Kotlin clients. A Rust read answers the error of its serde library: `serde_json::from_str::<Parcel>` answers `'label': too short: minimum length is 1, got 0` for a label the bound refuses. Dart throws a `FormatException`, Swift `DecodingError.dataCorrupted`, and Kotlin a `SerializationException`, each with the same text. To read a value that breaks a rule, call the read alone: `model_schema_read_unjudged`, `fromJsonUnjudged`, `init(swiftSchemaUnjudged:)`, or the serializer `{Type}Unjudged`.
- **`validate()` is there at every filling of a type that declares a parameter.** A `validate` of your own on one filling does not build: ``error[E0592]: duplicate definitions with name `validate` ``. See [A type that declares a parameter](#a-type-that-declares-a-parameter).
- **A message of `validate()` names a value by its path in the payload**: `'parcelList.0.itemLabel': too short: minimum length is 1, got 0` for the first parcel of a list under the key `parcelList`. Code that reads a message for the name of a field reads the key serde writes, the index of each item and the key of each entry. See [The path a message names](#the-path-a-message-names).

### Checks (`check`)

`check = ["name", ...]` names functions of your own that judge a value: a rule that reads one field against another, or one no bound spells. It is written on a type, in `#[model_schema(...)]`, and on a field, in `#[model_schema_prop(...)]`. A name is written once, and every language calls a function under it: Rust the snake case of the name, `withinParcel` as `within_parcel`, and TypeScript, Dart, Swift and Kotlin the name as written. [Preprocess and check](#preprocess-and-check) states the functions of the five side by side.

```rust
use serde::{Deserialize, Serialize};
use serde_json::json;
use tixschema::{model_schema, model_schema_prop};

tixschema::decode_with_foundation!();

#[model_schema(enable = ["decode_with"], check = ["withinParcel"])]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parcel {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
    #[model_schema_prop(check = ["withoutDuplicates"])]
    pub item_code_list: Vec<String>,
    pub fragile_item_codes: Vec<String>,
}

#[model_schema(enable = ["decode_with"])]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub parcel: Parcel,
}

// A field's check takes the field's own value, and names no path.
fn without_duplicates(value: &[String]) -> Result<(), Vec<String>> {
    for (index, code) in value.iter().enumerate() {
        if value[..index].contains(code) {
            return Err(vec![format!("duplicate item code {code}")]);
        }
    }
    Ok(())
}

// A type's check takes the whole value, and leads each message with the path of what it refuses.
fn within_parcel(value: &Parcel) -> Result<(), Vec<String>> {
    let outside: Vec<String> = value
        .fragile_item_codes
        .iter()
        .enumerate()
        .filter(|(_, code)| !value.item_code_list.contains(code))
        .map(|(index, code)| {
            format!("'fragileItemCodes.{index}': {code} is outside of itemCodeList")
        })
        .collect();
    if outside.is_empty() { Ok(()) } else { Err(outside) }
}

fn main() {
    let parcel = |label: &str, codes: &[&str], fragile: &[&str]| Parcel {
        label: label.to_string(),
        item_code_list: codes.iter().map(|code| code.to_string()).collect(),
        fragile_item_codes: fragile.iter().map(|code| code.to_string()).collect(),
    };
    let lines = |answered: Result<(), Vec<String>>| answered.err().unwrap_or_default();

    assert!(parcel("A", &["x"], &["x"]).validate().is_ok());
    assert_eq!(
        lines(parcel("", &["x"], &["x"]).validate()),
        ["'label': too short: minimum length is 1, got 0"]
    );
    assert_eq!(
        lines(parcel("A", &["x", "x"], &["x"]).validate()),
        ["'itemCodeList': duplicate item code x"]
    );
    assert_eq!(
        lines(parcel("A", &["x"], &["x", "y"]).validate()),
        ["'fragileItemCodes.1': y is outside of itemCodeList"]
    );
    // Every bound and every check runs, and none stops another.
    assert_eq!(
        lines(parcel("", &["x", "x"], &["y"]).validate()),
        [
            "'label': too short: minimum length is 1, got 0",
            "'itemCodeList': duplicate item code x",
            "'fragileItemCodes.0': y is outside of itemCodeList",
        ]
    );
    // A holder runs the `validate()` of what it holds, under the key it holds it at.
    let manifest = Manifest { parcel: parcel("A", &["x"], &["x", "y"]) };
    assert_eq!(
        lines(manifest.validate()),
        ["'parcel.fragileItemCodes.1': y is outside of itemCodeList"]
    );

    // The recovering read lists a broken check as an issue, at the place its message names.
    let read = Parcel::from_value_with(
        json!({ "label": "A", "itemCodeList": ["x"], "fragileItemCodes": ["x", "y"] }),
        |_, issues| {
            assert_eq!(
                format!("{:?}", issues[0]),
                r#"Invalid { path: Path([Key("fragileItemCodes"), Index(1)]), expected: Unknown, found: String("y"), reason: "y is outside of itemCodeList" }"#
            );
            parcel_schema::Verdict::Reject
        },
    );
    assert!(read.is_err());
}
```

#### The functions you write

A Rust function is in scope where the type is declared, a type declared inside a function body included. A TypeScript function is imported by the file the generated schemas are written into, as a [`preprocess`](#preprocessing-preprocess) function is.

| | Rust | TypeScript |
|---|---|---|
| A field's check | `fn without_duplicates(value: &[String]) -> Result<(), Vec<String>>` | `function withoutDuplicates(value: Array<string>, ctx: ParsePayload<Array<string>>): void` |
| A type's check | `fn within_parcel(value: &Parcel) -> Result<(), Vec<String>>` | `function withinParcel(value: Parcel, ctx: ParsePayload<Parcel>): void` |

`ParsePayload` is zod's own type, `z.core.ParsePayload`, which `zod/v4/core` exports. The two functions of the example in TypeScript:

```ts
import type { ParsePayload } from "zod/v4/core";

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
```

- **A field's check takes the field's own value**, read as a [bound reads it](#constraints-under-option-and-wrappers): under an `Option` it is called only where a value is held, under `Box`, `Rc`, `Arc`, `Cow` and a reference it takes what they hold, and a list or a map is taken whole. The call passes a reference, so `&[String]` and `&Vec<String>` both take a `Vec<String>`, and `&str` takes a `String`.
- **A type's check takes the whole value**, `&Parcel`, and an enum's takes the enum, whichever variant it holds.
- **A type with a parameter writes its function once, over every filling**: `fn with_a_code<CodeType>(value: &Crate<CodeType>) -> Result<(), Vec<String>>`, and `function withACode<CodeType>(value: Crate<CodeType>, ctx: ParsePayload<Crate<CodeType>>): void`.
- **The check of a [branded newtype](#branded-newtypes)** takes the brand in Rust, `&Code`, and the value the brand is written over in TypeScript, a `string` for a brand over a `String`: the Zod schema writes the check ahead of `.brand()`.

#### The messages

Rust answers `Err` with one message per broken rule, and TypeScript pushes one issue per broken rule to `ctx.issues`: `{ code: "custom", message, path, input: value }`.

- **A field's check names no path.** `validate()` writes each message under the name a bound of that field is reported under, [the path serde writes the field at](#the-path-a-message-names): its key, a slot's position among several, the variant's name or the content key ahead of a variant's member, and nothing for a `#[serde(flatten)]` member and for a slot serde writes as the value itself. A message that leads with a path below the field's value is written into that name: a check of `item_code_list` answering `'1': ...` is reported as `'itemCodeList.1': ...`. In TypeScript the issue's `path` is `[]`, or the path below the field's value, `[1]`, and zod writes the field's own key ahead of it.
- **A type's check leads each message with the path of what it refuses**, in single quotes, written with the keys of the payload and the index of each item, joined by dots: `'fragileItemCodes.1': y is outside of itemCodeList`. In TypeScript that is `path: ["fragileItemCodes", 1]`. tixschema cannot check a path you write: a key under `rename_all` is the renamed one, as in every generated message. A message with no lead is about the value itself.

#### Where a check runs

**In `validate()`, in every build.** A type that names a check, on itself or on a field, publishes [`validate()`](#the-validate-method) whether or not a field of it carries a bound, with the `serde` feature off and with no schema surface on as well: a function you wrote is called wherever the type is declared. Each member's bounds run first and its checks after them, member by member in the order declared, and the type's checks run last. Several checks of one key run in the order written, and every one runs: none stops another, and none is stopped by a broken bound.

Every shape runs its checks: a struct with named fields, a tuple struct (a slot's check where [a slot's bound](#a-slot-of-a-tuple-struct) is, reported where that bound is), a unit struct, an enum under every tagging and a plain one (a member's check runs for the variant the value holds, the enum's own after it), a branded newtype (after the brand's own string checks, with no path), and a type with a parameter at every filling. A type that holds another runs the `validate()` of what it holds, as it does [for a bound](#a-bound-the-fields-own-type-declares), so `Manifest` reports `'parcel.fragileItemCodes.1': y is outside of itemCodeList`.

**In the Zod schema**, as one `.check(...)` per name:

```ts
.check((ctx) => {
  const own = { value: ctx.value, issues: ctx.issues.slice(0, 0) };
  withinParcel(own.value, own);
  for (const issue of own.issues) ctx.issues.push({ ...issue, continue: true });
})
```

A field's check is written on the schema of the value the Rust function takes: after the field's bounds, inside the union an `Option` writes, on the array of a list after its count, and inside a `z.preprocess` wrap. A type's check is written on the type's own expression, ahead of `.brand()` on a brand and of the `.meta({ ... })` every binding carries, and inside the builder of a type with a parameter, so every filling of its factory carries it. Each issue is copied with `continue: true`: zod skips a later check once an issue without that flag is listed, and the Rust side runs every one.

**In the [recovering decode](#recovering-decode-decode_with)** of a type whose `enable` list holds `"decode_with"`. serde builds the value, the checks run on it, and each message is one `Issue::Invalid` of the read: `path` is what the message leads with, read against the raw value, `expected` is `Unknown`, `found` is the raw value at that path, and `reason` is the text after the lead. See [What each shape reports](#what-each-shape-reports). A check of a model a holder holds is listed at its full path, the holder's place in front, and so it is below a generic holder and a recursive one: a `Truck<CodeType>` holding `load: Crate<CodeType>` lists a broken check of the crate at `load.fragileItemCodes[1]`, and a `Carton` holding `cartons: Vec<Self>` lists one of its first inner carton at `cartons[0].fragileItemCodes[1]`.

**In a serde read** of a type that derives `Deserialize`. The read runs `validate()` on the value it read, checks included, and [refuses the payload](#a-read-refuses-what-validate-refuses) with what it answers. The hook an `#[serde(untagged)]` member is read through runs no check: a check chooses no variant.

**In the Dart `validate()`** every emitted Dart type has, under the name as written. See [`validate()` in Dart](#validate-in-dart).

**In the `validate()` of the Kotlin client.** Every Kotlin type carries one, which runs the bounds and calls each check under the name written, and the serializer of the type calls it on the value it read. See [`validate()` in the Kotlin client](#validate-in-the-kotlin-client) and [The read in the Kotlin client](#the-read-in-the-kotlin-client).

**In the Swift client**, in the [`validate()`](#validate-in-the-swift-client) every emitted type has: a member's checks after its bounds and after what the value it holds reports, the type's own last, each a call of the Swift function of that name.

**Nowhere else.** The JSON Schema calls no function, and a document is written as it is without the key. The Zod schema of a type that flattens an `#[serde(untagged)]` enum writes that enum's variants out one by one, with each member's check and without the check written on the enum itself, which the holder's `validate()` does run.

#### What is refused

The key is read in every build, and each refusal is spanned on what was written:

| Written | The refusal says |
|---|---|
| `check = "withinParcel"` | takes an array of string literals, each naming a function: `check = ["withinParcel"]` |
| `check = [withinParcel]` | takes string literals: each element is a string literal that names a function |
| `check = ["within-parcel"]` | names `within-parcel`, and a name is an identifier: ASCII letters, digits and `_`, not led by a digit. The name is written as a function's name in every language |
| `check = []` | names no function: write one name at least, or drop the key |
| `check = ["a", "a"]` | names `a` twice: the name is written twice, and a function runs once |

A `check` on a type alias is refused, an alias having no `impl` of its own to call a function from: write it on the type the alias names, or on a field that holds the alias. A `check` on the slot of a [branded newtype](#branded-newtypes) is refused as every key written there is: a brand's check is written on the type.

**A name with no function in scope does not build**, and the error is reported at the name:

```rust,compile_fail
#[model_schema(check = ["withinParcel"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    pub label: String,
}
```

```text
error[E0425]: cannot find function `within_parcel` in this scope
  |
4 | #[model_schema(check = ["withinParcel"])]
  |                         ^^^^^^^^^^^^^^ help: try calling `within_parcel` as a method: `self.within_parcel()`
```

The `help` is the compiler's own. What resolves the error is a function `within_parcel` in scope where `Parcel` is declared.

**A function that asks more of a type parameter than the type declares does not build** until the bound is written on the type, the call being made at every filling:

```rust,compile_fail
#[model_schema(default_types(CodeType = String), check = ["withoutRepeats"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Crate<CodeType> {
    pub codes: Vec<CodeType>,
}

fn without_repeats<CodeType>(value: &Crate<CodeType>) -> Result<(), Vec<String>>
where
    CodeType: PartialEq,
{
    for (index, code) in value.codes.iter().enumerate() {
        if value.codes[..index].contains(code) {
            return Err(vec![format!("'codes.{index}': a code is listed once")]);
        }
    }
    Ok(())
}
```

```text
error[E0277]: can't compare `CodeType` with `CodeType`
   |
 4 | #[model_schema(default_types(CodeType = String), check = ["withoutRepeats"])]
   |                                                           ^^^^^^^^^^^^^^^^ no implementation for `CodeType == CodeType`
   |
help: consider restricting type parameter `CodeType` with trait `PartialEq`
   |
 6 | pub struct Crate<CodeType: std::cmp::PartialEq> {
   |                          +++++++++++++++++++++
```

### `validate()` in Dart

With the `dart` feature, every Dart type `dart_definition()` writes has `List<String> validate()`: a class, the wrapper of a [branded newtype](#branded-newtypes) and of a type alias, a tuple struct, a plain enum, and a sealed enum with each of its variants. It runs what the Rust [`validate()`](#the-validate-method) of the same type runs, in the same order, and answers the messages as a list, empty for a value that keeps every rule. A type with no rule answers `const []`.

```rust
#[model_schema(check = ["withinParcel"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Parcel {
    pub fragile_item_codes: Vec<String>,
    #[model_schema_prop(check = ["withoutDuplicates"])]
    pub item_code_list: Vec<String>,
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Manifest {
    pub parcel: Parcel,
}
```

`parcel_dart::dart_definition()` writes the class with its constructor, its fields, [its two reads](#the-dart-read-refuses-what-validate-refuses) and `toJson`, and after them:

```dart
List<String> validate() => [
  for (final violation in withoutDuplicates(item_code_list)) _under('itemCodeList', violation),
  if (label.runes.length < 1) "'label': too short: minimum length is 1, got ${label.runes.length}",
  ...withinParcel(this),
];

static String _under(String place, String violation) {
  if (violation.startsWith("'")) {
    final end = violation.indexOf("'", 1);
    if (end > 0) return "'$place.${violation.substring(1, end)}'${violation.substring(end + 1)}";
  }
  return "'$place': $violation";
}
```

`Manifest` holds a parcel, and runs it under the key it holds it at:

```dart
List<String> validate() => [
  for (final violation in parcel.validate()) _under('parcel', violation),
];
```

#### The functions you write in Dart

A check is called under the name the attribute writes, `withinParcel`, from inside the emitted class. The function is yours, in scope in the file the emitted definitions are written into. It takes the value and answers its messages, an empty list for a value it passes:

| | Dart |
|---|---|
| A field's check | `List<String> withoutDuplicates(List<String> value)` |
| A type's check | `List<String> withinParcel(Parcel value)` |
| The check of a type with a parameter, written once | `List<String> withACode<CodeType>(Crate<CodeType> value)` |

```dart
List<String> withoutDuplicates(List<String> value) {
  final seen = <String>{};
  for (final code in value) {
    if (!seen.add(code)) return ['duplicate item code $code'];
  }
  return const [];
}

List<String> withinParcel(Parcel value) => [
      for (final (index, code) in value.fragile_item_codes.indexed)
        if (!value.item_code_list.contains(code)) "'fragileItemCodes.$index': $code is outside of itemCodeList",
    ];
```

- **A field's check takes the field's own value** and names no place: `validate()` writes each message under the field's key. Under an `Option` it is called where a value is held.
- **A type's check takes the whole value** and leads each message with the place of what it refuses, in single quotes, as [the Rust function does](#the-messages). The check of an enum takes the enum, and each variant hands itself over. The check of a brand takes the wrapper.
- **A member of the type hides a function of its name** inside the class: a check named `label` on a type with a field `label` does not compile. Name the check after the rule.

With those two functions, the `validate()` of six values, each read with `fromJsonUnjudged` from the payload serde writes for it, answers what the Rust `validate()` answers of the same values:

```text
case 1: passes
case 2: refused
    'label': too short: minimum length is 1, got 0
case 3: refused
    'itemCodeList': duplicate item code x
case 4: refused
    'fragileItemCodes.1': y is outside of itemCodeList
case 5: refused
    'itemCodeList': duplicate item code x
    'label': too short: minimum length is 1, got 0
    'fragileItemCodes.0': y is outside of itemCodeList
case 6: refused
    'parcel.fragileItemCodes.1': y is outside of itemCodeList
```

**A name with no function does not compile.** The emitted class calls it, and the Dart compiler reports the call:

```text
Error: The method 'withinParcel' isn't defined for the type 'Parcel'.
```

#### What Dart checks

| Written in Rust | Dart writes |
|---|---|
| `minLength`, `maxLength` on a string | `value.runes.length`, its Unicode code points: `"é"` and a character outside the Basic Multilingual Plane each count one |
| `minLength`, `maxLength` on a list | `value.length`, its items |
| `minimum`, `maximum` | `value < N`, `value > N` |
| `pattern`, and the second pattern [an enum's own check](#a-check-written-on-the-enum) adds | `RegExp("...").hasMatch(value)`, a match anywhere in the string |
| `check` on a field and on a type | a call of the function under that name |
| a `HashSet`, a `BTreeSet` or an `IndexSet`, with no key | `_repeats(value)`, ahead of the set's count: two items are one where the JSON the codec writes for each is one text, and the message is [`repeats an item: a set holds each item once`](#a-set-holds-each-item-once) |

- **The sentence is the one Rust and Zod write**, from the same source. A whole number is quoted back without `.0`: `too large: maximum is 3, got 4` for a `double` holding `4.0`.
- **The order is Rust's**: each member as declared, its bounds, then what the value it holds reports, then its checks, and the type's own checks last. The repeat of a set, which Rust never reports, runs ahead of the set's count.
- **An optional member is measured where it holds a value.** A member typed with a branded string is measured on the text the wrapper holds.
- **A holder runs the `validate()` of what it holds**, through an optional, each level of a list, a map's values and the positions of a tuple, and writes the place in front: `'parcelList.1.fragileItemCodes.1'`, `'parcelsByDock.north.label'`. An entry of a map is named by the text its key is written as, in the order the map holds its keys. A value behind one of the type's own parameters is not reached, as in Rust.
- **A brand runs its own string check on the value it wraps**, under no place. A brand over one of its own parameters holds its declared default and no other filling: `if (value case final String held) ...`. A brand over a list counts its items at every filling.
- **An enum's own string check** reaches in Dart the fields it reaches in Rust.

A message names a value by the steps serde writes on the way to it: a key, a position in an array, or nothing where serde writes the value itself.

| Member | serde writes it | The name |
|---|---|---|
| a named field of a struct | under its key | `'itemLabel'` |
| a slot of a tuple struct of several slots | at its position in the array | `'1'` |
| the one slot of a struct serde writes as the value itself | as the value itself | none: the sentence alone |
| a named member of a variant, no tag attribute | `{"Loose":{"freightLabel":...}}` | `'Loose.freightLabel'` |
| the one slot of a variant, no tag attribute | `{"Boxed":...}` | `'Boxed'`, and `'Boxed.label'` for a model it holds |
| a slot of a variant of several, no tag attribute | `{"Split":[...]}` | `'Split.1'` |
| a named member, tag and content key | `{"kind":"Loose","held":{...}}` | `'held.haulLabel'` |
| the one slot, tag and content key | `{"kind":"Boxed","held":...}` | `'held'` |
| a slot of several, tag and content key | `{"kind":"Split","held":[...]}` | `'held.1'` |
| a named member, tag alone | beside the tag | `'liftLabel'` |
| the one slot, tag alone | its own keys beside the tag | none |
| a member of an untagged variant | as the value itself, or under its key | none for the slot, its key for a named member |

The step of a variant is the name serde writes it under: its own `rename`, or the enum's `rename_all` over its name. A `#[serde(flatten)]` member writes no key and adds no step.

#### The Dart read refuses what `validate()` refuses

`fromJson` of every emitted type is its own unjudged read followed by its `validate()`, as [the Rust read](#a-read-refuses-what-validate-refuses) is. A payload whose value breaks a bound or a check is refused with a `FormatException`, and no object is handed back. Its `message` holds what `validate()` answers of the value read, every message, joined by `; `. Each class carries the two constructors:

```dart
factory Parcel.fromJson(Map<String, dynamic> json) {
  final read = Parcel.fromJsonUnjudged(json);
  final violations = read.validate();
  if (violations.isNotEmpty) throw FormatException(violations.join('; '));
  return read;
}

factory Parcel.fromJsonUnjudged(Map<String, dynamic> json) => Parcel(
  fragile_item_codes: (json['fragileItemCodes'] as List<dynamic>).map((e) => e as String).toList(),
  item_code_list: (json['itemCodeList'] as List<dynamic>).map((e) => e as String).toList(),
  label: json['label'] as String,
);
```

A type that holds another reads it with `fromJsonUnjudged`, so the whole value is judged once, by the type the read was asked for:

```dart
factory Manifest.fromJsonUnjudged(Map<String, dynamic> json) => Manifest(
  parcel: Parcel.fromJsonUnjudged(json['parcel']),
);
```

A terminal holds a parcel in a field, in a list, as a map's value and in an optional:

```rust
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Terminal {
    pub first_parcel: Parcel,
    pub lists_by_dock: BTreeMap<String, Vec<Parcel>>,
    pub parcel_list: Vec<Parcel>,
    pub parcels_by_dock: BTreeMap<String, Parcel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare_parcel: Option<Parcel>,
}
```

Five reads, each printing whether it was read and, where it was refused, the `message` of the exception broken after each `; `. A terminal with four broken parcels is refused once, with every message at its full place, and the Rust read answers the same four for the same payload:

```text
Parcel alone: refused (FormatException)
    'fragileItemCodes.1': y is outside of itemCodeList
Terminal, four broken parcels: refused (FormatException)
    'firstParcel.label': too short: minimum length is 1, got 0
    'parcelList.1.fragileItemCodes.1': y is outside of itemCodeList
    'parcelsByDock.north.label': too short: minimum length is 1, got 0
    'spareParcel.fragileItemCodes.1': y is outside of itemCodeList
List<Terminal>, the second broken: refused (FormatException)
    'firstParcel.label': too short: minimum length is 1, got 0
    'parcelList.1.fragileItemCodes.1': y is outside of itemCodeList
    'parcelsByDock.north.label': too short: minimum length is 1, got 0
    'spareParcel.fragileItemCodes.1': y is outside of itemCodeList
Crate<Parcel>, the second code broken: refused (FormatException)
    'fragileItemCodes.1': y is outside of itemCodeList
Terminal with no spareParcel key, all good: read
```

- **`fromJsonUnjudged` is the read alone.** It is public, because a holder in another file calls it. Call it yourself for the value and its messages together: `Parcel.fromJsonUnjudged(json).validate()`.
- **Every refusal the read writes itself is a `FormatException`**, the exception it throws for a value that breaks a rule, so a caller that catches it catches each. With `enum Drizzling { Calm, Gust { knots: u32 }, Held(String) }`, `"Gale"` and `{"Gale":{}}` are refused with `Unknown Drizzling: Gale`, and `{"Calm":1}` with `Drizzling: Calm takes no value`. A tag no variant has is refused in the same words under every tagging, and so is a name no variant of a plain enum has. A payload no variant of an untagged enum `Misting` reads is refused with `No variant of Misting matched`, a range of a key it does not write with `Unknown key of a Range: (start, end, step)`, and a `Result` of other than one of its two keys with `Not a Result: (Ok, Err)`, as serde refuses each. An externally tagged enum is read from an object of one key, and a tuple from an array of its length, as serde and Zod read each: with `enum Fogging { Boxed(String), Split(String, u32) }`, `{"Boxed":"a","Split":["b",1]}` is refused with `more than one key for Fogging`, `{}` with `empty object for Fogging`, `{"Split":["b",1,2]}` with `invalid length 3, expected 2` and `{"Split":["b"]}` with `invalid length 1, expected 2`, as serde and Zod refuse each. The array of a tuple struct, of a member's tuple and of a tuple a tuple holds is refused in the same words wherever its length is another. A fixed array `[T; N]` is read from an array of `N` items alone, at each level: with `struct Clenching { pair: [u8; 2] }`, `{"pair":[1,2,3]}` is refused with `invalid length 3, expected 2` and `{"pair":[1]}` with `invalid length 1, expected 2`, and a `[[u8; 2]; 3]` refuses an inner array of another length in the same words, as serde and Zod refuse each. A payload Dart cannot take apart is refused by the Dart language with an error of its own, which is no `FormatException`: the `TypeError` of a cast among them, for a text where a number is read. Either way no value was read for `validate()` to judge.
- **A value `validate()` does not reach judges itself as it is read**, and the first one that breaks a rule ends the read with its own messages, which name no place of the holder: a value behind one of the holder's own type parameters, read by the function the caller hands in (`Crate<Parcel>.fromJson(json, (e) => Parcel.fromJson(e))`), the key of a map, what a brand wraps, and a member whose own bound stands in for what its type declares.
- **A sealed enum judges in its base.** The unjudged read of the base chooses the variant by the tag, and a variant with named members carries the two constructors as a class does.
- **An `#[serde(untagged)]` enum takes the variant serde takes.** Its unjudged read takes the first variant whose payload decodes and whose bounds hold: each member's own, and the bound a brand it holds carries. A brand with a `pattern`, a `minLength` or a `maxLength` answers for that bound alone in `validateBound()`. A check, and what a model it holds reports, choose no variant: the variant is taken, and `fromJson` then refuses the value.
- **A plain enum judges where it names a check**, and carries `fromJsonUnjudged` in every case.
- **`toJson` judges nothing**, and is the text it was: a broken value built in code is written as it is.
- **A struct's member under `#[serde(skip_serializing)]` is read, and `toJson` writes no key for it**, as serde writes none, and spreads nothing of a flattened one. A flattened one that is no `Option` is read as any flattened member is, its `default` aside, as serde reads it: a payload without a key its struct requires, or with an entry its map does not read, is refused. A member under `skip_serializing_if` is written wherever it holds a value: `toJson` cannot call the Rust function, so under any function but `Option::is_none`, `Vec::is_empty` say, it writes the key where serde leaves it out.
- **A `#[serde(flatten)]` member is read at its place, in the order declared**, as serde reads it: from the object without the keys the holder's own members take and the entries each member flattened ahead of it took. A struct with a `#[serde(flatten)]` member of its own and the function a flattened member names are each handed that object, and a struct that reads a key a struct flattened ahead of it took is refused, as serde refuses it with `missing field`. A struct takes the keys it reads. A struct that names a [`preprocess`](#a-preprocess-in-the-dart-client) on itself, a flattened member that names one and a struct with a `#[serde(flatten)]` member of its own take no key, as serde reads each, so the function of a member flattened after one is handed its keys as well. A struct declared below its holder, or in another crate, is not known to take any: a member flattened after it is read from its keys as well, where serde reads it without them. A map is the [one member of its object](#intersection-types-serdeflatten), and is read from every key of it, but the tag of an enum under a tag key. An optional map writes no entry where it holds no value, and is `null` wherever an entry does not read as its key and value, as serde reads it: `#[serde(default, flatten, skip_serializing_if = "Option::is_none")] counts: Option<BTreeMap<String, u8>>` reads `{"north":2}` as that map, `{}` as an empty one, and `{"north":"x"}` as `null`, which is written back as `{}`, in a variant under its name, under a `content` key or untagged too, and keyed by a plain enum it reads a key that names no variant as `null`. A model it holds is read with no judgement, so one that breaks its rule still refuses the payload, as serde refuses it. Under a [`preprocess`](#a-preprocess-in-the-dart-client) it is read from what the function answers, and an entry there that does not read refuses the payload, as serde refuses it. `toJson` writes each flattened member at its place, and a key two of them hold is written once, with the last value, where serde writes it twice.
- **A flattened `Option` of a struct is `null` where the object it is handed holds none of the struct's keys, and a key there that does not read refuses the payload**, as TypeScript and Zod refuse it, where serde reads `None` and drops the key with no error. With `struct Lapping { note: String, #[serde(flatten, default, skip_serializing_if = "Option::is_none")] tabling: Option<Tabling> }` over a `Tabling` that reads `label`, `{"note":"n"}` is read with `tabling` `null` and `{"label":"north","note":"n"}` with the `Tabling` it holds, and `{"label":5,"note":"n"}` is refused with the `TypeError` of its cast, where serde reads `None`. A struct of two keys is refused where one of them is there and the other is not. The keys are those serde reads the struct under, each member's name and each alias, and they are looked for in the object the member is handed, without the keys its holder's own members and the members flattened ahead of it take, and without the tag of a variant under a tag key. An optional enum under a tag key is looked for the same way, by its tag and its content key: it is `null` where neither is there, and is read where one is, so a tag or a content that does not read refuses the payload, as TypeScript and Zod refuse it, where serde reads `None`. With `#[serde(tag = "kind")] enum Grommet { Calm, Round { turns: u32 } }` flattened beside `note: String`, `{"kind":"Round","note":"n","turns":2}` reads `Round`, `{"note":"n"}` reads `null`, and so does `{"note":"n","turns":2}`, a variant's member with no tag, as serde reads it, and `{"kind":"Round","note":"n","turns":"x"}` is refused with the `TypeError` of its cast and `{"kind":"Gale","note":"n"}` with `FormatException: Unknown Grommet: Gale`. Under `#[serde(tag = "kind", content = "body")]`, `{"body":{"turns":2},"note":"n"}`, a content with no tag, is refused as well. The enum takes neither key off the object a member flattened after it is handed. A type whose keys are not known is read as `null` wherever its read throws, as serde reads `None`: a struct declared below its holder or in another crate, a struct with a `#[serde(flatten)]` member of its own, a struct with no member serde reads, a type that names a [`preprocess`](#a-preprocess-in-the-dart-client) on itself, an untagged enum, an enum under a tag key declared below its holder or in another crate, an alias and a type parameter. A model the holder's `validate()` walks is read with no judgement where it reads, so one that breaks its rule still refuses the payload, as serde refuses it. A flattened `Option` of any type but a map that names a `preprocess` is read from what its functions answer, as the Zod schema of its holder reads it, where serde parts from Zod: where the type does not read the answer, `null` among it, the member is `null` where the object its functions are handed holds none of its struct's keys, or no key at all for a type whose keys are not known, an enum among them, and the payload is refused where it holds one. serde refuses an answer its type does not read, and reads `null` as `None`. An externally tagged enum takes one entry, below, and an optional map is `null` wherever an entry does not read, above.
- **A flattened externally tagged enum takes one entry**, as serde reads it: the first, in the object's order, under the name a variant is read under, its `rename` or the enum's `rename_all` included, or an alias of one, wherever it is in the object, and the enum is read from that entry. With `enum Hatching { Bay { label: String }, Dock { label: String } }` flattened beside `note: String`, `{"north":2,"Bay":{"label":"n"},"note":"a"}` reads `Bay` and the note `a`, and a payload with no such entry is refused, as serde refuses it. An optional enum, `#[serde(default, flatten, skip_serializing_if = "Option::is_none")] hatching: Option<Hatching>`, reads `null` from a payload with no such entry, `{"north":2,"note":"a"}` among them, as serde reads `None`, and an entry that does not read, `{"Bay":{"label":5},"note":"a"}`, refuses the payload, as TypeScript and Zod refuse it, where serde reads `None` and drops the entry with no error. Flattened ahead of a member that names a function, `{"Bay":{"label":"n"},"Dock":{"label":"s"},"north":2}` reads `Bay`, and the function is handed `Dock` and `north`. The enum is written as serde writes it, its variant's entry beside the holder's members, and a unit variant as its name over `null`: with `Calm` among `Hatching`'s variants, the holder writes `{"note":"a","Calm":null}`, the enum optional or under a `preprocess` as well, and reads `Calm` from that entry in either order. The entry over any other value is refused, as the Zod schema, which takes `null` alone, refuses it: serde refuses it too, but for an empty object, which it reads as `Calm`. An enum under a tag key alone and an untagged enum take no key, as serde reads each, and so does an enum that names a `preprocess` on itself. An enum under a tag and a content key is not known to take any: the function of a member flattened after it is handed the tag and the content, where serde hands it neither. An enum declared below its holder, or in another crate, is not known to take an entry: it is read from the object it is handed, which is refused where it holds any other key, where serde reads the entry: with `struct Overcast { note: String, #[serde(flatten)] stratus: Hazing }` declared above `enum Hazing { Bay { label: String } }`, `{"note":"a","Bay":{"label":"n"},"north":2}` is refused with `more than one key for Hazing`. The write of the unit variant of such an enum throws a `TypeError`.
- **An externally tagged enum reads a unit variant from its entry over `null` too**, as serde reads it, where the Zod schema of the enum takes the name alone: `Hatching.fromJson(jsonDecode('{"Calm":null}'))` reads `Calm`, beside `"Calm"`, and writes it back `"Calm"`. The entry over any other value is refused, as serde refuses it.
- **An enum under a tag and a content key reads a unit variant from an object with no content key**, as TypeScript and Zod read it: with `#[serde(tag = "kind", content = "held")] enum Becalming { Breeze(bool), Lull }`, `Becalming.fromJson(jsonDecode('{"kind":"Lull"}'))` reads `Lull`, and `{"kind":"Lull","held":5}` is refused with `FormatException: Becalming: Lull takes no value`, as serde refuses it. `{"kind":"Lull","held":null}` is refused with the same exception, as TypeScript and Zod refuse it, where serde reads `Lull`. serde writes the variant as its tag alone, so every payload serde writes is read.
- **A struct under its own `#[serde(tag = "...")]` reads a payload whose tag is the struct's name, and refuses any other**, as TypeScript and the generated Zod schema refuse it, whose tag is the literal of that name. serde reads the struct whatever the tag holds, and where it is absent, and writes the name back. With `#[serde(tag = "kind")] struct Flag { label: String }`, `{"kind":"Flag","label":"n"}` reads, `{"label":"n"}` is refused with ``FormatException: missing field `kind` ``, `{"kind":"Other","label":"n"}` with `FormatException: expected tag "Flag", found "Other"`, and `{"kind":5,"label":"n"}` with `FormatException: expected tag "Flag", found 5`, where serde reads each as `Flag { label: "n" }`. A struct with a `#[serde(flatten)]` member reads its tag the same way, and so does a struct flattened into a holder, from the object the holder hands it, so the holder refuses the same payloads. `toJson` writes the tag ahead of the members, as serde writes it.
- **A name serde writes is read and written as serde writes it, whatever it holds.** A member's, an alias's, a struct's own tag key and name, an enum's variant, tag key and content key, and a plain enum's value are each written between the quotes of a Dart string, a `$`, a `'` and a `\` escaped: with `#[serde(tag = "ki$nd", rename = "Price's$Tag\\")] struct Tariffed { #[serde(rename = "back\\slash")] backslash: String, #[serde(rename = "cost$usd", alias = "cost's")] cost: String, #[serde(rename = "say \"hi\"")] quoted: String }`, `{"ki$nd":"Price's$Tag\\","back\\slash":"b","cost$usd":"c","say \"hi\"":"s"}` is read and written back as serde writes it.
- **A name serde reads beside the one it writes is read too.** A variant is read under each of its `#[serde(alias = "...")]` names, under every tagging, flattened, and in a plain enum, and a member under each of its aliases, in a struct, flattened or not, and in a variant. Each is written under the name serde writes. With `#[serde(alias = "Quay")] Dock { #[serde(alias = "name")] label: String }`, `{"Quay":{"name":"s"}}` reads `Dock` and is written back `{"Dock":{"label":"s"}}`. A member under two of its names, `{"Dock":{"label":"s","name":"t"}}`, is refused with ``FormatException: duplicate field `label` ``, as serde refuses it. A flattened struct takes each alias of its members off its holder's object, as serde does. A class with a member under an alias reads its members from a copy of the payload, each alias read as the name it stands for, and a type that names no alias is written as it was.
- **A field named `fromJsonUnjudged` keeps its name**, Dart taking a field and a constructor under one. A member of a plain enum under that name is `fromJsonUnjudged_`, the enum having the function, and keeps the value serde writes.
- **The service clients read a reply through `fromJson`**, so a reply that breaks a rule is the fault each client answers for a reply it cannot read, with the messages in its `detail`: `undeserializablePayload` from the HTTP client, and `failedValidation` from the WebSocket client.

#### What stays yours

- **`validate()` is for a value built in code**, before it is sent, and for a value changed after it was read: [`fromJson` judges what it reads](#the-dart-read-refuses-what-validate-refuses). It calls no [`preprocess`](#a-preprocess-in-the-dart-client): a read does, ahead of it.
- **A class of your own that a model holds** declares `List<String> validate()` and the constructor `fromJsonUnjudged` as the emitted ones do: a holder reads it and calls it with no knowledge of what the held type declares, whether that type is declared above the holder or below it. [`ObjectId`](#mongodb-objectid-support) is the one exception, read through its `fromJson` and asked nothing.
- **A field named `validate`** is the member `validate_`, and keeps its key, as [a reserved word does](#a-member-named-after-a-word-the-client-language-reserves).
- **Every build writes the same Dart.** The Dart `validate()` holds each bound and calls each check with a schema surface on and with none. One declaration differs: a brand's own `minLength`, `maxLength` or `pattern` is refused by a build with no `typescript`, `zod` or `jsonschema`, so no wrapper carries one there.

### `validate()` in the Kotlin client

Every type `kotlin_definition()` writes carries `fun validate(): List<String>`: a data class, the `object` of a unit struct, the value class of a brand, a tuple struct, a sealed interface with each of its variants, and a plain enum. It answers the messages Rust's [`validate()`](#the-validate-method) answers for the same value, in the same order, and an empty list for a value that keeps every rule. A type with no rule answers `emptyList()`.

The serializer of a type calls it on the value it read: [the read refuses what `validate()` refuses](#the-read-in-the-kotlin-client). You call it yourself on a value built in code, and on one changed after it was read.

This is what the `Parcel` and the `Manifest` of [Checks](#checks-check) emit. The emitter writes each type on one line, and each line break below stands for one space of it:

```kotlin
@OptIn(ExperimentalSerializationApi::class) @Serializable(with = ParcelSerializer::class) @KeepGeneratedSerializer
data class Parcel(val label: String, val itemCodeList: List<String>, val fragileItemCodes: List<String>) {
  fun validate(): List<String> {
    val violations = mutableListOf<String>();
    if (label.codePointCount(0, label.length) < 1) violations.add("'label': too short: minimum length is 1, got ${label.codePointCount(0, label.length)}");
    violations.addAll(withoutDuplicates(itemCodeList).map { kotlinSchemaUnder("itemCodeList", it) });
    violations.addAll(withinParcel(this));
    return violations
  };
  private fun kotlinSchemaUnder(place: String, violation: String): String {
    val end = violation.indexOf('\'', 1);
    return if (violation.startsWith("'") && end > 0) { "'$place.${violation.substring(1, end)}'${violation.substring(end + 1)}" } else { "'$place': $violation" }
  }
}
@OptIn(ExperimentalSerializationApi::class) object ParcelUnjudged : KSerializer<Parcel> by Parcel.generatedSerializer()
object ParcelSerializer : KSerializer<Parcel>, () -> KSerializer<Parcel> {
  override fun invoke(): KSerializer<Parcel> = ParcelUnjudged;
  override val descriptor: SerialDescriptor = ParcelUnjudged.descriptor;
  override fun serialize(encoder: Encoder, value: Parcel) = ParcelUnjudged.serialize(encoder, value);
  override fun deserialize(decoder: Decoder): Parcel {
    val read = ParcelUnjudged.deserialize(decoder);
    val violations = read.validate();
    if (violations.isNotEmpty()) throw SerializationException(violations.joinToString("; "));
    return read
  }
}

@OptIn(ExperimentalSerializationApi::class) @Serializable(with = ManifestSerializer::class) @KeepGeneratedSerializer
data class Manifest(@Serializable(with = ParcelUnjudged::class) val parcel: Parcel) {
  fun validate(): List<String> {
    val violations = mutableListOf<String>();
    violations.addAll(parcel.validate().map { kotlinSchemaUnder("parcel", it) });
    return violations
  };
  private fun kotlinSchemaUnder(place: String, violation: String): String {
    val end = violation.indexOf('\'', 1);
    return if (violation.startsWith("'") && end > 0) { "'$place.${violation.substring(1, end)}'${violation.substring(end + 1)}" } else { "'$place': $violation" }
  }
}
@OptIn(ExperimentalSerializationApi::class) object ManifestUnjudged : KSerializer<Manifest> by Manifest.generatedSerializer()
object ManifestSerializer : KSerializer<Manifest>, () -> KSerializer<Manifest> {
  override fun invoke(): KSerializer<Manifest> = ManifestUnjudged;
  override val descriptor: SerialDescriptor = ManifestUnjudged.descriptor;
  override fun serialize(encoder: Encoder, value: Manifest) = ManifestUnjudged.serialize(encoder, value);
  override fun deserialize(decoder: Decoder): Manifest {
    val read = ManifestUnjudged.deserialize(decoder);
    val violations = read.validate();
    if (violations.isNotEmpty()) throw SerializationException(violations.joinToString("; "));
    return read
  }
}
```

The `validate()` of the parcel `{"label":"","itemCodeList":["x","x"],"fragileItemCodes":["y"]}` answers the three lines Rust answers, and that of a manifest that holds a parcel with `"y"` outside its codes answers the fourth:

```text
'label': too short: minimum length is 1, got 0
'itemCodeList': duplicate item code x
'fragileItemCodes.0': y is outside of itemCodeList
'parcel.fragileItemCodes.1': y is outside of itemCodeList
```

- **The order is Rust's.** Each member in the order declared: its bounds, then what the value it holds reports, then the checks it names. The type's own checks run last. Every rule runs, and none stops another.
- **A member with a bound of its own** is not asked for what its type declares, a list apart: a list runs its count and then each item, as in Rust.
- **Every function a type calls is in the type's own body.** `kotlinSchemaUnder` writes a holder's key into a message, and is written into each type that names a place. The emitted types are joined into one file, so no function is written beside a type, and tixschema ships no runtime library. The two serializers of a type are the only declarations beside it, each under the type's own name.
- **A member named `validate` builds and keeps its key**, a property and a method being two names to Kotlin. A member named `violations` is read as `this.violations`.
- **Every build with the `kotlin` feature writes the same `validate()`.** The emitter reads the declaration itself, so a build with no schema surface, where Rust holds no bound, still writes each bound a field declares into the Kotlin client.

#### The bounds

| Key | On | Kotlin writes |
|---|---|---|
| `minLength`, `maxLength` | a string | `value.codePointCount(0, value.length)`: Unicode code points |
| `minLength`, `maxLength` | a list | `value.size` |
| `minimum`, `maximum` | a number | `value < 1u`, `value > 120.0`: a comparison in the number's own type |
| `pattern`, and the second pattern an enum writes | a string | `Regex("...").containsMatchIn(value)`: a match anywhere |
| none | a `HashSet`, a `BTreeSet` or an `IndexSet` | `value.toSet().size != value.size`, ahead of the set's count: two equal items repeat one, and the message is [`repeats an item: a set holds each item once`](#a-set-holds-each-item-once) |

- **A length is counted in Unicode code points**, so `"é"` and a character outside the Basic Multilingual Plane each count one. The Rust validator counts the bytes of the UTF-8 text (`len()`) and the Zod schema counts UTF-16 code units (`length`), so the three agree on ASCII text and can differ on any other.
- **A nullable member is measured only where it holds a value**: `spareLabel?.let { held -> ... }`.
- **A number is compared in its own type.** An unsigned one is compared with an unsigned literal (`1u`), a `Float` with a `Float` (`0.5f`), a `Double` with a `Double`. A bound that is no value of the type, `minimum = 0.5` on a `u32`, is compared as a `Double`.
- **A float is quoted back as Rust writes it.** Kotlin prints `150.0` and `2.5E8` where Rust prints `150` and `250000000`, so a type that bounds a float carries `kotlinSchemaShown`, which writes a whole float without `.0` and any float without an exponent.
- **A pattern is written for the JVM, and stated as you wrote it.** The JVM reads `$` as the end of the text or the place before a last line break, `\v` as any vertical space, and refuses a named group whose name holds `_`. The emitted regex writes `$` as `\z`, `\v` as `\x0B` and a named group as a plain one, which is what the `regex` crate reads in the three. The message still states the pattern of the declaration: `does not match pattern '^[a-z]+$'`.

#### What a holder runs

A holder calls the `validate()` of each value of a declared type it holds, and writes each message under the place the payload holds the value at:

```kotlin
violations.addAll(firstParcel.validate().map { kotlinSchemaUnder("firstParcel", it) });
parcelList.forEachIndexed { index, held -> violations.addAll(held.validate().map { kotlinSchemaUnder("parcelList.$index", it) }) };
spareParcel?.let { held -> violations.addAll(held.validate().map { kotlinSchemaUnder("spareParcel", it) }) };
for ((key, held) in parcelsByDock) { violations.addAll(held.validate().map { kotlinSchemaUnder("parcelsByDock.$key", it) }) }
```

- **A list** names each item by its index at each level, and **a tuple** written as a field's type each position: `'parcelGrid.0.1.label'`, `'deckPair.1.label'`.
- **A map** is walked in the order it holds its entries, and names each by its key as serde writes it. A key that is a string, a number or a `Boolean` is written as it is. A key of a declared type is written as its serializer writes it, so a plain enum goes by the name serde writes the variant under: `'byWing.north.label'`.
- **A `#[serde(flatten)]` member** writes no key, and what it reports is written under no name. A flattened map names each entry by its key alone.
- **A value behind a type parameter is not reached**, as in Rust. A key typed with a type parameter is written where the Rust type declares `Serialize` for the parameter, as Kotlin prints it, and left out where the type declares none.
- **A type the emitter has not written** when it writes the holder has no `validate()` the holder can count on: a type of your own you supply to Kotlin, an alias of a string or of a list, a type declared below. The holder then carries `private fun Any?.validate(): List<String> = emptyList()` in its own body. A member is found ahead of an extension, so a type that has `validate()` answers with its own.

#### A bound on a field typed with a declared type

A length or a pattern on [a field typed with a branded string](#a-field-typed-with-a-branded-string) is run on the string the type holds. The value class of a brand over a string or a path carries `fun kotlinSchemaText(): String = value`, and a brand over such a brand answers through the brand it holds. The holder asks the field for its text, and carries the answer of a string in its own body:

```kotlin
if (slug.kotlinSchemaText().codePointCount(0, slug.kotlinSchemaText().length) < 3) violations.add("'slug': too short: minimum length is 3, got ${slug.kotlinSchemaText().codePointCount(0, slug.kotlinSchemaText().length)}");
private fun String.kotlinSchemaText(): String = this
```

A member is found ahead of an extension, so a brand answers with its own text and an alias of a string with the string. The brand and the alias may be declared above the holder or below it. A field typed with a type that holds no string does not compile in Kotlin, at the holder: `error: candidate 'fun String.kotlinSchemaText(): String' is inapplicable because of a receiver type mismatch.` A brand over a brand answers through the brand it holds where that brand is declared above it.

Rust writes no text for a brand over a brand, and refuses a string bound on a field typed with one. Kotlin measures it.

#### A brand

The value class of a brand runs the brand's own `minLength`, `maxLength` and `pattern` over `value`, then the checks the brand names, and names no place: `too short: minimum length is 2, got 1`. A brand over a list counts its items. A brand with such a bound carries `fun kotlinSchemaBound(): List<String>` beside `validate()`: what the bounds alone answer, the checks apart, which is what [an untagged enum chooses a variant by](#the-read-in-the-kotlin-client).

A brand that holds one of its type parameters bare runs its bounds where the value is of the type declared for the parameter, and at no other filling: `if (value is String) { ... }`. Where the declared type is itself written with arguments, `default_types(WrapType = StrictDocumentId<String>)`, the JVM keeps no argument to tell the filling by, and the value class runs no bound.

#### The name of a slot and of a variant's member

A message names a value by the steps serde writes on the way to it: a key, a position in an array, or nothing where serde writes the value itself.

| Member | serde writes it | The name |
|---|---|---|
| a slot of a tuple struct of several slots | at its position in the array | `'1'` |
| the one slot of a tuple struct | as the value itself | none: the sentence alone, or the holder's key |
| a named member of a variant, no tag attribute | `{"Loose":{"freightLabel":…}}` | `'Loose.freightLabel'` |
| the one slot of a variant, no tag attribute | `{"Boxed":…}` | `'Boxed'`, and `'Boxed.label'` for a model it holds |
| a slot of a variant of several, no tag attribute | `{"Split":[…]}` | `'Split.1'` |
| a named member, tag and content key | `{"kind":"Loose","held":{…}}` | `'held.haulLabel'` |
| the one slot, tag and content key | `{"kind":"Boxed","held":…}` | `'held'` |
| a slot of several, tag and content key | `{"kind":"Split","held":[…]}` | `'held.1'` |
| a named member, tag alone | beside the tag | `'liftLabel'` |
| the one slot, tag alone | its own keys beside the tag | none |
| a member of an `#[serde(untagged)]` variant | as the value itself, or under its key | none for the slot, its key for a named member |

The variant's step is the name serde writes it under: the variant's own `rename`, or the enum's `rename_all` applied to its ident. Each variant of a sealed interface overrides `validate()` with the rules of its own members, and the enum's own checks run last in each. A string check written on the enum reaches in Kotlin the fields [it reaches in Rust](#a-check-written-on-the-enum).

#### The checks

A check is called under the name the attribute writes, on the value Rust hands its function, and answers the messages as a list: empty where the value keeps the rule. A top-level function of the package, or one the file imports, is in scope.

| | Kotlin |
|---|---|
| A field's check | `fun withoutDuplicates(value: List<String>): List<String>` |
| A type's check | `fun withinParcel(value: Parcel): List<String>` |
| The check of a type with a parameter, written once | `fun <CodeType> withACode(value: Crate<CodeType>): List<String>` |

```kotlin
fun withoutDuplicates(value: List<String>): List<String> {
    val seen = mutableSetOf<String>()
    for (code in value) {
        if (!seen.add(code)) return listOf("duplicate item code $code")
    }
    return emptyList()
}

fun withinParcel(value: Parcel): List<String> =
    value.fragileItemCodes.withIndex()
        .filter { it.value !in value.itemCodeList }
        .map { "'fragileItemCodes.${it.index}': ${it.value} is outside of itemCodeList" }
```

A field's check names no path, and `validate()` writes its messages under the field. A type's check leads each message with the path of what it refuses, as [in Rust](#the-messages). The check of a nullable member is called only where the member holds a value, and the check of a brand takes the value class.

**A name with no Kotlin function does not compile**, and the Kotlin compiler says so at the call:

```text
error: unresolved reference 'withinParcel'.
```

#### The read in the Kotlin client

The serializer of a type throws for a payload whose value breaks a bound or a check, and hands no object back. `Json.decodeFromString<Parcel>(payload)`, `serializer<Parcel>()` and a property typed `Parcel` in a class of your own all read through `ParcelSerializer`, the type's own serializer: it reads the value and calls `validate()` on it. What it throws is `SerializationException`, the exception of `kotlinx.serialization`, and its message is what `validate()` answers of the value read, every message, joined by `; `:

```kotlin
Json.decodeFromString<Parcel>("""{"label":"","itemCodeList":["x"],"fragileItemCodes":["x","y"]}""")
// SerializationException: 'label': too short: minimum length is 1, got 0; 'fragileItemCodes.1': y is outside of itemCodeList
```

**Every type has two serializers**, a plain enum apart:

| | What it reads | Who names it |
|---|---|---|
| `{Type}Unjudged` | the payload, with no judgement | a holder, for each model it holds, and you, to read a value that breaks a rule |
| `{Type}Serializer` | through the first, and then throws what `validate()` answers | the type: `@Serializable(with = ParcelSerializer::class)` |

Both write the same payload, and writing judges nothing: a broken value built in code is written as it is. A type with parameters writes the two as classes over one serializer per parameter, `CrateUnjudged(serializer<String>())`. The type's own serializer is a function too: called, `ParcelSerializer()` answers `ParcelUnjudged`, which is how a read tixschema writes reaches the unjudged serializer of a type it holds but cannot name, one declared below it.

- **A class, the `object` of a unit struct and the value class of a brand keep the serializer the plugin writes**, with `@KeepGeneratedSerializer`, and `{Type}Unjudged` is that serializer under a name: `object ParcelUnjudged : KSerializer<Parcel> by Parcel.generatedSerializer()`. A struct under its own `#[serde(tag = "...")]` holds the tag in a property the plugin reads and writes after the struct's members, so its `{Type}Unjudged` wraps the plugin's serializer, refuses a payload whose tag is not the struct's name (below), and writes the tag ahead of the members, as serde writes it: `#[serde(tag = "kind")] struct Flag { label: String }` is written `{"kind":"Flag","label":"n"}`, alone and flattened into a holder. The annotation is an experimental one of `kotlinx.serialization`, so each such type carries `@OptIn(ExperimentalSerializationApi::class)`. This repository compiles the emitted source with `kotlinx.serialization` 1.11.0, the version `just kotlin-libs` installs.
- **A struct with a `#[serde(flatten)]` member, a struct with a member serde never writes, a tuple struct and an enum under every tagging** have a serializer tixschema writes, and it is the unjudged one. So does the class of a variant with a `#[serde(flatten)]` member or a member serde never writes, `{Enum}{Variant}Unjudged`, its one serializer: the enum's own serializer judges the variant.
- **Every refusal of a serializer tixschema writes is a `SerializationException`**, the exception the plugin's own serializers throw, so a caller that catches it catches each. Its message says what the read expected and found: ``missing field `held` `` for a key serde requires, `expected an object, found [1]` for a value of another kind, `expected an object of one key, found {}` for an externally tagged enum read from an object of no entry or of several, `unknown tag Gale` for a tag no variant has, and `invalid length 3, expected 2` for the array of a tuple, or of a fixed array, of another length, a longer one among them, as serde refuses each.
- **A `char`, a `NaiveDate`, a `NaiveTime` and a `NaiveDateTime`, which Kotlin holds as a `String`, are read in their one form wherever they are held**: a field, an item of a list, a key and a value of a map, a type argument, a range's bound, an arm of a `Result`, a position of a tuple, a variant's value, the key of a flattened map, the value of a brand and the type an alias names. Each is read through a serializer of the type's own, `object KotlinText1 : KSerializer<String>`, which the property names, `val letter: @Serializable(with = KotlinText1::class) String`, so the property keeps its type, and which throws `SerializationException` for any other text. A `char` is one code point, as serde reads it: `"ab"` and `""` are refused, the first with `SerializationException: expected a character, found "ab"`, and `"😀"` is read. A date, a time and a date and time are read where the generated Zod schema reads them, `z.iso.date()`, `z.iso.time()` and `z.iso.datetime({ local: true })`: `"x"` is refused with `SerializationException: expected a date, found "x"`, and with `expected a time` and `expected a date and time` where it is read as the other two. Where serde and Zod part, the read follows Zod: `"2024-1-6"`, a year before 0000 or after 9999, `"+12024-01-06"`, which serde writes with its sign, and the leap second `"23:59:60"`, all of which serde reads, are refused, and `"2024-01-05T11:20:30Z"` and `"2024-01-05T11:20"`, which serde refuses, are read as a `NaiveDateTime`. A `DateTime<Tz>` without `as_number` is read as any text: Zod reads any text a JavaScript `Date` reads, which no Kotlin check follows, and serde reads an RFC 3339 date and time. A read tixschema writes, of a struct with a `#[serde(flatten)]` member or a member serde never writes, of a tuple and of a variant's value, reads as any text an alias declared below its holder, and a `char` or a date that fills the type parameter of an alias, or of a generic type declared below its holder. The generated service clients and the WebSocket dispatcher read a request, a reply and a header that is itself a `char` or a date, or a list or a map of one, in its one form as well: a reply of another is answered with the client's fault, `undeserializable-payload` from the `http_rest` client and `failed-validation` from the `ws_rpc` one, and a header of another with the dispatcher's `failed-validation`, the refusal in its `detail`.
- **A fixed array `[T; N]` is read from an array of `N` items alone, at each level**, as serde and the generated Zod schema (`z.array(...).length(N)`) read it, wherever it is held: a field, beside a `#[serde(flatten)]` member, an item of a list, a map's value, under an `Option`, a position of a tuple, a variant's value, a type argument, the fixed array of a type parameter and the type an alias names. Each level is read through a class of the type's own, `class KotlinArray1<Item>(item: KSerializer<Item>) : KSerializer<List<Item>>`, which the property names, `val pair: @Serializable(with = KotlinArray1::class) List<UByte>`, so the property keeps its type, and which throws `SerializationException` for an array of another length: with `struct Clenching { pair: [u8; 2] }`, `{"pair":[1,2,3]}` is refused with `SerializationException: invalid length 3, expected 2` and `{"pair":[1]}` with `invalid length 1, expected 2`, and a `[[u8; 2]; 3]` refuses an inner array of another length in the same words, through the type's own serializer and its unjudged one. A list, `Vec<u8>`, is read at any length. A read tixschema writes, of a struct with a `#[serde(flatten)]` member or a member serde never writes, of a tuple and of a variant's value, reads at any length a fixed array an alias declared below its holder names, and one that fills the type parameter of an alias, or of a generic type declared below its holder, as it reads any text there.
- **A struct under its own `#[serde(tag = "...")]` reads a payload whose tag is the struct's name, and refuses any other**, as TypeScript and the generated Zod schema refuse it, whose tag is the literal of that name. serde reads the struct whatever the tag holds, and where it is absent, and writes the name back. With `#[serde(tag = "kind")] struct Flag { label: String }`, `{"kind":"Flag","label":"n"}` reads, `{"label":"n"}` is refused with ``SerializationException: missing field `kind` ``, `{"kind":"Other","label":"n"}` with `SerializationException: expected tag "Flag", found "Other"`, and `{"kind":5,"label":"n"}` with `SerializationException: expected tag "Flag", found 5`, where serde reads each as `Flag { label: "n" }`. A struct with a `#[serde(flatten)]` member reads its tag the same way, and so does a struct flattened into a holder, from the object the holder hands it, so the holder refuses the same payloads.
- **A member of a struct or of an enum's variant under `#[serde(skip_serializing)]` is read, and written into no object**, as serde writes it into none, and a flattened one merges nothing into its holder's. A flattened one that is no `Option` is read as any flattened member is, its `default` aside, as serde reads it: a payload without a key its struct requires, or with an entry its map does not read, is refused. The plugin writes every property, so a struct that holds such a member is written by the serializer tixschema writes, and so is the class of a variant that holds one, under every tagging. A member under `skip_serializing_if` is written wherever it holds a value: the serializer cannot call the Rust function, so under any function but `Option::is_none`, `Vec::is_empty` say, it writes the key where serde leaves it out.
- **The serializer tixschema writes for a struct or a variant's class refuses a key no member reads**, as TypeScript, Zod and the plugin's own serializer refuse it, where serde reads past it: with `struct Furling { label: String, #[serde(default, skip_serializing)] spare: Option<u32> }`, `{"label":"north","other":1}` is refused with `SerializationException: Encountered an unknown key 'other'`. A `Json { ignoreUnknownKeys = true }` reads past the key, as it does in a class the plugin reads. The keys the members read are each member's name and alias, the struct's own tag, every key a flattened struct tixschema wrote reads, its own tag and a member under `#[serde(skip_deserializing)]` among them, and the one entry a flattened externally tagged enum takes: a second variant's entry is refused. A payload serde writes holds no other key. A model held at any depth inside a flattened struct whose keys tixschema knows, or inside the entry of a flattened externally tagged enum, refuses such a key too, as it does held under a key: the member is read under the `Json` the read was asked with, from the keys it reads alone. With `struct Thimbling { held: Ringbolt, label: String, rail: Pinrail }` flattened beside `note: String`, over `Pinrail { ringbolt: Ringbolt }` and `Ringbolt { label: String }`, `{"note":"a","held":{"label":"x"},"label":"n","rail":{"ringbolt":{"label":"y","other":1}}}` is refused with the `SerializationException` of the plugin's serializer, whose message opens `Encountered an unknown key 'other' at path: $`. Beside a flattened map, and beside a flattened member whose keys tixschema does not know, no key is refused, and every model inside such a member reads past one, where TypeScript and Zod refuse one none of the members reads: a struct with a `#[serde(flatten)]` member of its own, a type or a member that names a [`preprocess`](#preprocess-in-the-kotlin-client), an enum under a tag key or untagged, and a type declared below its holder, in another crate, an alias or a type parameter. The serializer of an enum under a tag key refuses a key beside its tag and its content, as TypeScript and Zod refuse it: under `#[serde(tag = "kind", content = "load")]`, `{"kind":"Gust","load":{"speed":1},"other":1}` and `{"kind":"Lull","other":1}` are refused, and under `#[serde(tag = "kind")]` a unit variant, `{"kind":"Lull","other":1}`, as a variant of members is by the class that reads it. A lenient `Json` reads past each.
- **A `#[serde(flatten)]` member is read at its place, in the order declared**, as serde reads it: from the object without the keys the holder's own members take and those each member flattened ahead of it took. A struct that reads a key a member of its holder's own, or a struct flattened ahead of it, took is refused, as serde refuses it with `missing field`, and an optional one looks for its keys in that object alone: it is `null` where none of them is there, and is read where one is, a key that does not read refusing the payload, as TypeScript and Zod refuse it, where serde reads `None`. A struct takes the keys serde reads it from: the name and each alias of every member it reads, one serde never writes among them. Its own `#[serde(tag = "...")]` and a member under `#[serde(skip_deserializing)]` are not among them, so a member flattened after it is handed both, as serde hands them, and an optional one whose tag alone is in the object is `null`. A struct that names a [`preprocess`](#preprocess-in-the-kotlin-client) on itself, a flattened member that names one and a struct with a `#[serde(flatten)]` member of its own take no key, as serde reads each. An optional struct with a `#[serde(flatten)]` member of its own has no key to be looked for by: it is read wherever its read succeeds and is `null` wherever that read throws, as serde reads `None`. With `struct Crowning { note: String, #[serde(flatten)] tabling: Tabling }` over a `Tabling` that reads `label`, `#[serde(flatten, default, skip_serializing_if = "Option::is_none")] crowning: Option<Crowning>` reads `{"label":"north","note":"n"}` as that `Crowning`, written back as `{"note":"n","label":"north"}`, and reads `{}`, `{"note":"n"}` and `{"label":5,"note":"n"}` as `null`, written back as `{}`. A struct declared below its holder, or in another crate, is not known to take any: a member flattened after it is read from its keys as well, where serde reads it without them, and an optional one is looked for by the properties its serializer declares, its tag and a member serde never reads among them and no alias. Where its serializer declares none, as the serializer tixschema writes for a struct with a `#[serde(flatten)]` member of its own declares none, the optional one whose member names no `preprocess` has no key to be looked for by, and is read wherever its read succeeds and is `null` wherever that read throws, as serde reads `None`: with `Crowning` declared below its holder, `{"label":"north","note":"n"}` reads that `Crowning`, and `{"note":"n"}` reads `null`, where TypeScript and Zod refuse it. Where the read throws for a value that reads and breaks a rule, the payload is refused with the rule's message, as serde, TypeScript and Zod refuse it: the read is tried again through the unjudged serializer the type's own answers when called, and the refusal stands where that read succeeds. With `struct Gromet { #[serde(flatten)] earing: Earing, #[model_schema_prop(minLength = 2)] note: String }` declared below its holder, `{"label":"north","note":"n"}` is refused with `SerializationException: 'note': too short: minimum length is 2, got 1`, and `{"label":5,"note":"nn"}` reads `null`. A type of your own is read as before. An optional untagged enum is read the same way, and so is an enum under a tag key declared below its holder or that names a [`preprocess`](#preprocess-in-the-kotlin-client) on itself, and an alias or a type parameter that names a type whose serializer declares no property. An optional one whose member names a [`preprocess`](#preprocess-in-the-kotlin-client) is read as the Zod schema of its holder reads it: its functions are called first, on the object the member is handed, and the type is read from what they answer. Where that read throws, it is `null` where that object holds none of the type's keys, or no key at all where those keys are not known, and the payload is refused where the object holds one. With `struct Reeved { note: String, #[model_schema_prop(preprocess = ["asGiven"])] #[serde(flatten, default, skip_serializing_if = "Option::is_none")] tholepin: Option<Tholepin> }` over a `Tholepin` that reads `label`, and `fun asGiven(payload: JsonElement): JsonElement = payload`, `{"label":"north","note":"n"}` reads that `Tholepin`, `{"note":"n"}` reads `null`, where serde refuses it with `missing field`, and `{"label":5,"note":"n"}` is refused. Under a function that answers `JsonNull`, `{"label":"north","note":"n"}` is refused, as Zod refuses it, where serde reads `None`. A map is the [one member of its object](#intersection-types-serdeflatten), and is read from every key of it. A key Kotlin holds as no `String`, a plain enum or a [brand](#branded-newtypes) among them, is written as the text its own serializer writes and read back through that serializer: with `enum Chock { Port, Starboard }`, `#[serde(flatten)] lines: BTreeMap<Chock, u8>` reads `{"Port":2}` and writes it back as it was, and refuses `{"Bow":2}` with `SerializationException`, as serde refuses it, a variant under its `rename` or the enum's `rename_all` read under the name serde reads. A map keyed by a number or a `bool` [does not build](#intersection-types-serdeflatten) flattened, and a `char` or a date key is read in its one form, as every `char` and date is (above). An optional map writes no entry where it holds no value, and is `null` wherever an entry does not read as its key and value, as serde reads it: `#[serde(default, flatten, skip_serializing_if = "Option::is_none")] counts: Option<BTreeMap<String, u8>>` reads `{"north":2}` as that map, `{}` as an empty one, and `{"north":"x"}` as `null`, which is written back as `{}`, and keyed by `Chock` it reads `{"Bow":2}` as `null`. Under a [`preprocess`](#preprocess-in-the-kotlin-client) it is read from what the function answers, as serde reads it: `null` is `null`, and an entry that does not read as its value is refused.
- **A variant reads and writes a `#[serde(flatten)]` member at its own level**, as serde does: with no tag attribute, under a tag and a content key, untagged, and under a tag alone, where a struct is flattened beside the tag and a map [stops the build](#intersection-types-serdeflatten). The variant is read, as a struct with such a member is, from the object its enum hands it: its entry, its content, the object without the tag, or the whole payload. It is written into that object. Over a `Tabling` that reads `label`, `enum Gaffing { Counted { #[serde(flatten)] counts: BTreeMap<String, u8> }, Labelled { #[serde(flatten)] tabling: Tabling, note: String } }` reads `{"Counted":{"north":2}}` as the count `north` and writes it back as it was, and reads `{"Labelled":{"note":"a","label":"n"}}` as the label `n` and the note `a`, written back `{"Labelled":{"label":"n","note":"a"}}`. A member of such a variant is read under each of its aliases and through each of its [`preprocess`](#preprocess-in-the-kotlin-client) functions, as a struct's is.
- **A flattened externally tagged enum takes one entry**, as serde reads it: the first, in the object's order, under the name a variant is read under, its `rename` or the enum's `rename_all` included, or an alias of one, and every member flattened after it is handed the object without that entry. With `enum Hatching { Bay { label: String }, Dock { label: String } }` flattened beside `note: String`, `{"note":"a","Bay":{"label":"n"}}` reads `Bay` and the note `a`, and `{"note":"a","Bay":{"label":"n"},"Dock":{"label":"s"}}` is refused, as Zod refuses it, where serde reads `Bay` and reads past `Dock` (above). Flattened ahead of a member that names a function, `{"Bay":{"label":"n"},"Dock":{"label":"s"},"north":2}` reads `Bay`, and the function is handed `Dock` and `north`. A payload with no such entry is refused with `SerializationException`, and an optional enum reads `null` from it. The enum is written as serde writes it, its variant's entry beside the holder's members, and a unit variant as its name over `null`: with `Calm` among `Hatching`'s variants, the holder writes `{"note":"a","Calm":null}`, the enum optional or under a `preprocess` as well. An enum under a tag key alone and an untagged enum take no key, as serde reads each, and so does an enum that names a `preprocess` on itself or on its member. An enum under a tag and a content key is not known to take any: a member flattened after it is handed the tag and the content, where serde hands it neither. An optional enum under a tag key, whose member names no `preprocess`, is looked for by its tag and its content key in the object it is handed: it is `null` where neither is there, and is read where one is, so a tag or a content that does not read refuses the payload, as TypeScript and Zod refuse it, where serde reads `None`. With `#[serde(tag = "kind")] enum Hitched { Bay { label: String } }` flattened beside `note: String`, `{"kind":"Bay","label":"n","note":"a"}` reads `Bay`, `{"kind":"Bay","note":"a"}` is refused, and `{"note":"a"}` reads `null`, and so does `{"label":"n","note":"a"}`, a variant's member with no tag, as serde reads it, where TypeScript and Zod refuse it. Under `#[serde(tag = "kind", content = "held")]`, `{"held":{"label":"n"},"note":"a"}`, a content with no tag, is refused as well. The refusal is what the enum's read throws, a `SerializationException`: under a tag and a content key, ``missing field `kind` `` for a content with no tag and ``missing field `held` `` for a tag with no content. An optional untagged enum, and one under a tag key declared below its holder or that names a `preprocess` on itself, has no key to be looked for by: it is read wherever its read succeeds and is `null` wherever that read throws, as serde reads `None`. Where the member of an optional enum under a tag key, or untagged, names a `preprocess`, it is read from what the functions answer, as an optional struct that names one is, and has no key to be looked for by: with `asGiven`, `{"kind":"Bay","label":"n","note":"a"}` reads `Bay`, `{"note":"a"}` reads `null`, and `{"kind":"Bay","note":"a"}` and `{"label":"n","note":"a"}` are refused, as Zod reads each, where serde refuses all three. An enum declared below its holder, or in another crate, is not known to take an entry: it is read from the object it is handed, and is refused where that object holds any other key, and the write of its unit variant throws `IllegalArgumentException`.
- **An externally tagged enum reads a unit variant from its entry over `null` alone**, as serde reads it: with `Calm` among `Hatching`'s variants, `{"Calm":null}` reads `Calm`, beside `"Calm"`, and is written back `"Calm"`. The entry over any other value, `{"Calm":1}`, is refused with `SerializationException: Hatching: Calm takes no value`, as serde refuses it. Flattened, the enum is read from that entry, so a holder of an optional `Hatching` refuses `{"Calm":1,"note":"a"}` with the same exception, as TypeScript and Zod refuse it, where serde reads `None`.
- **An enum under a tag and a content key reads a unit variant from an object with no content key**, as TypeScript and Zod read it: with `#[serde(tag = "kind", content = "held")] enum Becalming { Breeze(bool), Lull }`, `{"kind":"Lull"}` reads `Lull`, and `{"kind":"Lull","held":5}` is refused with `SerializationException: Becalming: Lull takes no value`, as serde refuses it. `{"kind":"Lull","held":null}` is refused with the same exception, as TypeScript and Zod refuse it, where serde reads `Lull`. serde writes the variant as its tag alone, so every payload serde writes is read.
- **A name serde reads beside the one it writes is read too.** A variant is read under each of its `#[serde(alias = "...")]` names, under every tagging, flattened, and in a plain enum, and a member under each of its aliases, in a struct, flattened or not, and in a variant. Each is written under the name serde writes. With `#[serde(alias = "Quay")] Dock { #[serde(alias = "name")] label: String }`, `{"Quay":{"name":"s"}}` reads `Dock` and is written back `{"Dock":{"label":"s"}}`. A member under two of its names, `{"Dock":{"label":"s","name":"t"}}`, is refused with ``SerializationException: duplicate field `label` ``, as serde refuses it. A flattened struct takes each alias of its members off its holder's object, as serde does. A class the plugin reads, with a member under an alias, has a `{Type}Unjudged` that hands the plugin's serializer the payload with each alias read as the name it stands for, `object ParcelUnjudged : JsonTransformingSerializer<Parcel>(Parcel.generatedSerializer())`, and a plain enum with a variant under an alias has a `{Type}Unjudged` written by hand, its one serializer.
- **An enum under a tag alone is read by a serializer tixschema writes as well.** The plugin refuses `@KeepGeneratedSerializer` on a sealed interface, so the enum carries no `@JsonClassDiscriminator`: its unjudged serializer reads the tag key, takes it off the object and reads the rest as the variant, and writes the variant with the tag ahead of its members. A tag no variant has, and a payload with no tag, throw `SerializationException`.
- **A plain enum keeps the one serializer the plugin writes**, and its read judges nothing. One that names a check is refused by the read of a type that holds it, and by your own call of `validate()`. One that names a [`preprocess`](#preprocess-in-the-kotlin-client), or reads a variant under an alias, is read by `{Type}Unjudged`, the one serializer it then has.
- **A refusal the decoder raises itself is its own, unchanged**: a missing key, a value of another type. No value was read for `validate()` to judge.

**The file the definitions are written into imports what they name.** The two serializers of every type name `KSerializer` and `SerializationException` of `kotlinx.serialization`, `SerialDescriptor` of its `descriptors` package, and `Encoder` and `Decoder` of its `encoding` package. A serializer tixschema writes names the `json` package as well, and `builtins` where it composes the serializer of a list, a map or a nullable:

```kotlin
import kotlinx.serialization.*
import kotlinx.serialization.builtins.*
import kotlinx.serialization.descriptors.*
import kotlinx.serialization.encoding.*
import kotlinx.serialization.json.*
```

**A holder reads what it holds unjudged, and the whole value is judged once**, by the type the read was asked for. It names the unjudged serializer of each model where the model is written, on the property and inside a list, a nullable and a map:

```kotlin
data class Manifest(
  @Serializable(with = ParcelUnjudged::class) val firstParcel: Parcel,
  val parcelList: List<@Serializable(with = ParcelUnjudged::class) Parcel>,
  val parcelsByDock: Map<String, @Serializable(with = ParcelUnjudged::class) Parcel>,
  @Serializable(with = ParcelUnjudged::class) val spareParcel: Parcel? = null,
)
```

A manifest that holds a broken parcel in each of the four places is refused once, with every message at its full place. The message is one line, broken here after each `; `, and it is the text the [Rust read](#a-read-refuses-what-validate-refuses) answers of the same payload:

```text
'firstParcel.label': too short: minimum length is 1, got 0;
'parcelList.1.fragileItemCodes.1': y is outside of itemCodeList;
'parcelsByDock.north.label': too short: minimum length is 1, got 0;
'spareParcel.fragileItemCodes.1': y is outside of itemCodeList
```

A model is read unjudged where the holder's `validate()` asks it for what it reports, and nowhere else. Held any other way it is read by its own serializer and judges itself as it is read: the payload is refused there too, by the first broken model, with that model's own messages and no place in front of them.

| Held | Judged |
|---|---|
| in a field, a nullable, a list, a map's values, a tuple written as a field's type, a `#[serde(flatten)]` member, a slot of a tuple struct or of a variant | once, with its holder, each message at its full place |
| behind one of the holder's own type parameters | by itself: `Crate<Parcel>` with a broken parcel answers `'fragileItemCodes.1': y is outside of itemCodeList` |
| by a brand, and by a member that writes a length or a pattern of its own on a brand | by itself: `too short: minimum length is 2, got 1`, where Rust writes the member's key in front, `'code': too short: ...` |
| where its type is declared below its holder, is an alias, or is a type of your own | by itself: the emitter has written no `{Type}Unjudged` under that name when it writes the holder |

Rust reads a model declared below its holder unjudged, so the two differ there: a holder that breaks a bound of its own and holds such a model that breaks one answers both messages in Rust, and the model's alone in Kotlin. Declare the model above the type that holds it, and the two answer the same text.

**An `#[serde(untagged)]` enum takes the variant serde takes.** Its unjudged serializer tries each variant in the order declared and takes the first whose payload decodes and whose bounds hold: each member's own, and the `minLength`, `maxLength` and `pattern` of each brand the variant holds. A [check](#checks-check) and what a held model reports choose no variant: the variant is chosen first, and the enum's own serializer then refuses the value. A variant with such a bound, and the value class of a brand with one, carry `fun kotlinSchemaBound(): List<String>` for that: what the bounds alone answer.

```rust
#[model_schema(pattern = "^[A-Z]+$", check = ["unreserved"])]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Tag(pub String);

#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum Mark {
    Brief(#[model_schema_prop(maxLength = 3)] String),
    Coded(Tag),
    Plain(String),
}
```

| Payload | Read |
|---|---|
| `"ab"` | `MarkBrief` |
| `"ABCD"` | `MarkCoded`: past the bound of the first variant |
| `"abcd"` | `MarkPlain`: past that bound, and outside the pattern of the brand |
| `"NONE"` | refused with `a tag is not NONE`: `MarkCoded` is chosen, and the check of the brand refuses it |

A payload no variant takes throws `SerializationException: No variant of Mark matched`.

**The generated clients answer their fault for a reply that breaks a rule**, with the messages in the fault's detail: `undeserializable-payload` from the `http_rest` client, and `failed-validation` from the `ws_rpc` one.

**To read a value that breaks a rule**, name the unjudged serializer, and call `validate()` for what is wrong with it:

```kotlin
val parcel = Json.decodeFromString(ParcelUnjudged, payload)
parcel.validate()
// ['label': too short: minimum length is 1, got 0, 'fragileItemCodes.1': y is outside of itemCodeList]
```

#### `preprocess` in the Kotlin client

The Kotlin read calls each function a [`preprocess`](#preprocessing-preprocess) names, under the name the attribute writes, on the payload the Rust read hands its own function. The payload is a `JsonElement` of `kotlinx.serialization`, and the function answers the payload the value is then read from: a payload it has nothing to correct in, as it is. A top-level function of the package, or one the file imports, is in scope. **The key is a last resort** here as it is in Rust: a function of yours runs on every read of the type, on a payload no schema describes.

| | Kotlin |
|---|---|
| A field's function | `fun trimmed(payload: JsonElement): JsonElement` |
| A type's function | `fun withoutFragile(payload: JsonElement): JsonElement` |

The two functions of the [example](#preprocessing-preprocess) in Kotlin:

```kotlin
fun trimmed(payload: JsonElement): JsonElement =
    if (payload is JsonPrimitive && payload.isString) JsonPrimitive(payload.content.trim()) else payload

fun withoutFragile(payload: JsonElement): JsonElement =
    if (payload is JsonObject && "fragileItemCodes" !in payload) {
        JsonObject(payload + ("fragileItemCodes" to JsonArray(emptyList())))
    } else {
        payload
    }
```

This is what the `Parcel` of that example emits. The emitter writes it on one line, and each line break below stands for one space of it:

```kotlin
@OptIn(ExperimentalSerializationApi::class) @Serializable(with = ParcelSerializer::class) @KeepGeneratedSerializer
data class Parcel(@Serializable(with = ParcelLabelPreprocess::class) val label: String, val itemCodeList: List<String>, val fragileItemCodes: List<String>) {
  fun validate(): List<String> {
    val violations = mutableListOf<String>();
    if (label.codePointCount(0, label.length) < 1) violations.add("'label': too short: minimum length is 1, got ${label.codePointCount(0, label.length)}");
    return violations
  }
}
@OptIn(ExperimentalSerializationApi::class) object ParcelUnjudged : JsonTransformingSerializer<Parcel>(Parcel.generatedSerializer()) {
  override fun transformDeserialize(element: JsonElement): JsonElement = withoutFragile(element)
}
object ParcelSerializer : KSerializer<Parcel>, () -> KSerializer<Parcel> {
  override fun invoke(): KSerializer<Parcel> = ParcelUnjudged;
  override val descriptor: SerialDescriptor = ParcelUnjudged.descriptor;
  override fun serialize(encoder: Encoder, value: Parcel) = ParcelUnjudged.serialize(encoder, value);
  override fun deserialize(decoder: Decoder): Parcel {
    val read = ParcelUnjudged.deserialize(decoder);
    val violations = read.validate();
    if (violations.isNotEmpty()) throw SerializationException(violations.joinToString("; "));
    return read
  }
}
object ParcelLabelPreprocess : JsonTransformingSerializer<String>(serializer<String>()) {
  override fun transformDeserialize(element: JsonElement): JsonElement = trimmed(element)
}
```

The serializer of the type answers each row of the [sample](#preprocessing-preprocess) as the Rust read answers it:

| # | Payload | The Kotlin read |
|---|---|---|
| 1 | `{"label":"  A  ","itemCodeList":["x"],"fragileItemCodes":["x"]}` | read; `label` is `"A"` |
| 2 | `{"label":"   ","itemCodeList":["x"],"fragileItemCodes":["x"]}` | refused: `'label': too short: minimum length is 1, got 0` |
| 3 | `{"label":"A","itemCodeList":["x"]}` | read; `fragileItemCodes` is `[]` |
| 5 | row 1 inside `Manifest`, under `parcel` | read; `parcel.label` is `"A"` |

- **The order of one read is Rust's.** The type's functions on the whole payload, each field's as that field is read, the value, then `validate()`: a bound and a [check](#the-checks) judge what the functions answered, which is why row 2 is refused. Several functions run in the order written, each over the answer of the one before: `preprocess = ["unwrapped", "withoutFragile"]` calls `withoutFragile(unwrapped(element))`.
- **A type's functions are called by `{Type}Unjudged`**, so [both serializers of the type](#the-read-in-the-kotlin-client) read the corrected payload, and so does a type that holds it, wherever it holds it: in a field, a nullable, a list, a map's value, a `#[serde(flatten)]` member, a variant under every tagging, and behind a type parameter. Where the plugin writes the serializer, of a class, of the `object` of a unit struct and of the value class of a brand, `{Type}Unjudged` is a `JsonTransformingSerializer` over it, as above. The serializer tixschema writes for a struct with a `#[serde(flatten)]` member, for a tuple struct and for an enum under every tagging calls the functions on the payload it decodes, and an `#[serde(untagged)]` enum does so once, ahead of the choice of a variant.
- **A brand's functions take the payload of the value the brand is written over.** The value class of a brand with a parameter keeps the delegation to the plugin's serializer and reads through it in a `deserialize` of its own: the Kotlin compiler hands such a value class unboxed to the `serialize` a `JsonTransformingSerializer` inherits, and the write then throws `ClassCastException`.
- **A plain enum that names a function has `{Type}Unjudged` as its one serializer**, over the plugin's: `@Serializable(with = GradeUnjudged::class) @KeepGeneratedSerializer enum class Grade`. Its read judges nothing, as before.
- **A field's functions are called by a serializer of the field's own**, `{Type}{Field}Preprocess`, which the property names, in a class and in a variant's class alike. It reads through the serializer the field has without the key: the plugin's, the unjudged one of a model the holder's `validate()` asks, a nullable one for an `Option`, so the corrected payload is read, and judged, exactly as the payload was.
- **A field whose type reaches a type parameter has no serializer of its own.** The plugin takes one on a property only where its constructor takes no parameter, or one per argument of the property's own type, so it cannot be handed the serializer of the parameter. `{Type}Unjudged` calls that field's functions at its key in the object, after the type's own, and an enum's serializer does the same for such a member of a variant.
- **Where tixschema writes the read, a field's functions are called in place**: a member of a struct with a `#[serde(flatten)]` member, a slot of a tuple struct, and the slot of a variant. `val dock = input.json.decodeFromJsonElement(serializer<String>(), trimmed(obj.getValue("dock")))`.
- **A key the payload does not hold calls no function**, as in Rust: `kotlinx.serialization` asks a property's serializer only for a key the payload holds, and a property with a default keeps it. A `null` the payload holds is a payload, and is handed to the function.
- **A `#[serde(flatten)]` member a function reads is handed the keys no member ahead of it took**, as the Rust read hands them, and as [every flattened member is](#the-read-in-the-kotlin-client): `{"dock":"north","tag":"t"}` hands the function of the flattened type `{"tag":"t"}`. A member flattened ahead of it, of a type that names no function, takes its own keys too, as serde's read of it does, and one that names a function takes none. An optional one is read from what its functions answer, and where that read throws is `null` or refuses the payload as the Zod schema of its holder does, as [the read](#the-read-in-the-kotlin-client) states.
- **Writing calls no function.** `{Type}Unjudged` and the serializer of a preprocessed field write a value as it is written without the key, and a value built in code is never read, so nothing corrects it.
- **The path of a refusal below a function starts at the value the function answered.** A field that names a function, and a type that names one, are each read from a JSON value of their own, so a payload the read refuses, where the function left it as it was, is named inside that value and not from the payload the read was handed: `{"label":5}` is refused with `String literal for value of key 'primitive' should be quoted at path: $.primitive`, where a field with no function is named at `$.label`. A list or an object where the field reads a text or a number is thrown by `kotlinx.serialization` 1.11.0 as an `IndexOutOfBoundsException`, which is no `SerializationException`, as it is by every read tixschema writes itself; the generated clients answer their fault for it, as they do for any refusal of a reply.
- **The definitions name two more types of the `json` package**, `JsonTransformingSerializer` and `JsonElement`, which the [imports of the read](#the-read-in-the-kotlin-client) already bring in.

**A name with no Kotlin function does not compile**, and the Kotlin compiler says so at the call:

```text
error: unresolved reference 'trimmed'.
```


### `validate()` in the Swift client

Every Swift type `swift_definition()` writes has `public func validate() -> [String]`: a struct, a tuple struct, a unit struct, the wrapper of a [branded newtype](#branded-newtypes), an enum under every tagging and a plain one, a type with a parameter at every filling, and the struct written for a variant's named members or for a tuple. It answers one message per broken rule, and an empty list for a value that keeps every one. It judges a value built or changed in code, and `init(from:)` of the type calls it on the value it read, so [a read refuses what it refuses](#a-swift-read-refuses-what-validate-refuses).

```swift
let parcel = Parcel(label: "", itemCodeList: ["x", "x"], fragileItemCodes: [])
let violations = parcel.validate()
// ["'label': too short: minimum length is 1, got 0", "'itemCodeList': duplicate item code x"]
```

A type with no rule, which holds no type that has one, declares `public func validate() -> [String] { [] }`.

**What runs** is what the Rust [`validate()`](#the-validate-method) of the type runs, in its order and in its words: a member's bounds, then what the value it holds reports, then the checks it names, member by member as declared, and the type's own checks last. A bound written on a member stands in place of the bounds its type declares, but on a list, which is counted and walked.

| Written in Rust | What Swift runs |
|---|---|
| `minLength`, `maxLength` on a string | `value.unicodeScalars.count`: the string is counted in Unicode code points |
| `minLength`, `maxLength` on a list | `value.count` |
| `minimum`, `maximum` | `value < N`, `value > N` |
| `pattern`, and an enum's own beside a field's | `value.range(of: "...", options: .regularExpression) == nil`: a match anywhere in the string |
| `check = ["withinParcel"]` | `withinParcel(value)`, the Swift function of that name |
| a `HashSet`, a `BTreeSet` or an `IndexSet`, with no key | `Parcel.swiftSchemaRepeats(value)`, ahead of the set's count: two items are one where `JSONEncoder` with sorted keys writes one text for both, and the message is [`repeats an item: a set holds each item once`](#a-set-holds-each-item-once) |

- **The unit of a length is the Unicode code point**: `"é"` counts 1, `"e\u{301}"` 2, and a character outside the Basic Multilingual Plane 1. The Rust `validate()` measures a string with `len()`, its bytes in UTF-8, so the two agree on ASCII text and differ beyond it.
- **A whole float is quoted back without `.0`**, as Rust writes it: `2.0` reads `got 2`.
- **An `Option` is measured where it holds a value.**
- **A range no value of the type can break is not written**: `minimum = 0` on a `u32` adds no comparison, which Swift would warn of.
- **A pattern is read by Foundation's regular expressions**, so a type that carries one needs `import Foundation`. Two constructs are written another way for that engine to read the pattern as the `regex` crate does: `$` as `\z`, since it otherwise also matches ahead of a line terminator that ends the string, and `\v` as `\u000B`, since it is otherwise every vertical space. The message quotes the pattern as you wrote it.
- **A branded newtype runs its own bounds on the value it wraps**, under no path. One that declares a parameter runs them where the value is of its declared default's Swift type (`value as? String`), and at no other filling. A `String`, a `PathBuf` and a `char` are all a Swift `String`.
- **A bound on a field typed with a brand** measures the text the brand's wrapper answers with `swiftSchemaBoundText`, a member every brand over a string or a path has; a brand over a brand answers through the brand it holds. The brand is declared above the field's type or below it. A bound on a field typed with an alias of a string measures the string.

**A type that holds another** runs its `validate()` and writes each message under the place it holds it at:

```swift
errors += firstParcel.validate().map { Manifest.swiftSchemaUnder("firstParcel", $0) }
for (index, held) in parcelList.enumerated() { errors += held.validate().map { Manifest.swiftSchemaUnder("parcelList.\(index)", $0) } }
for (key, held) in parcelsByDock.sorted(by: { $0.key < $1.key }) { errors += held.validate().map { Manifest.swiftSchemaUnder("parcelsByDock.\(key)", $0) } }
if let held = spareParcel { errors += held.validate().map { Manifest.swiftSchemaUnder("spareParcel", $0) } }
```

A Swift dictionary keeps no order, so a map is walked by its sorted keys. A value behind a type parameter is not reached, as in Rust. `swiftSchemaUnder` is a private function written into each type that names a place: nothing is shared between two emitted types, and nothing is declared beside them.

**The path a message names** is the steps serde writes on the way to the value, joined by dots: a key, a position in an array, or nothing where serde writes the value itself.

| Member | serde writes it | The name |
|---|---|---|
| a named field of a struct | under its key | `'itemLabel'` |
| a slot of a tuple struct of several slots | at its position in the array | `'1'` |
| the one slot of a struct serde writes as the value itself | as the value itself | none: the sentence alone, or the holder's key alone |
| a named member of a variant, no tag attribute | `{"Loose":{"freightLabel":…}}` | `'Loose.freightLabel'` |
| the one slot of a variant, no tag attribute | `{"Boxed":…}` | `'Boxed'`, and `'Boxed.itemLabel'` for a model it holds |
| a slot of a variant of several, no tag attribute | `{"Split":[…]}` | `'Split.1'` |
| a named member, tag and content key | `{"kind":"Loose","held":{…}}` | `'held.haulLabel'` |
| the one slot, tag and content key | `{"kind":"Boxed","held":…}` | `'held'` |
| a slot of several, tag and content key | `{"kind":"Split","held":[…]}` | `'held.1'` |
| a named member, tag alone | beside the tag | `'liftLabel'` |
| the one slot, tag alone | its own keys beside the tag | none |
| a member of an untagged variant | as the value itself, or under its key | none for a slot, its key for a named member |

**The functions you write** are any functions of the module the generated types are compiled in. A check is called under the name the attribute writes:

| | Swift |
|---|---|
| A field's check | `func withoutDuplicates(_ value: [String]) -> [String]` |
| A type's check | `func withinParcel(_ value: Parcel) -> [String]` |
| The check of a type with a parameter, written once | `func withACode<CodeType>(_ value: Crate<CodeType>) -> [String]` |

```swift
func withoutDuplicates(_ value: [String]) -> [String] {
  var seen = Set<String>()
  for code in value where !seen.insert(code).inserted {
    return ["duplicate item code \(code)"]
  }
  return []
}

func withinParcel(_ value: Parcel) -> [String] {
  value.fragileItemCodes.enumerated()
    .filter { !value.itemCodeList.contains($0.element) }
    .map { "'fragileItemCodes.\($0.offset)': \($0.element) is outside of itemCodeList" }
}
```

A function answers one message per broken rule, and an empty list for none. A field's check takes the field's own value, under an `Option` only where one is held, and names no path: `validate()` writes each message under the field. A type's check takes the whole value and leads each message with the path of what it refuses, as [in Rust](#the-messages). A name with no function does not build:

```text
error: cannot find 'withinParcel' in scope
```

**What you declare.**

- **A field named `validate`** is declared `validate_`, the method having the name, and keeps its key: `case validate_ = "validate"`.
- **A type tixschema has not written** is one a holder cannot tell from a model declared below it, so it calls its `validate()` and reads it through its `init(swiftSchemaUnjudged:)`: a type of your own that a generated type holds declares `func validate() -> [String] { [] }` and `init(swiftSchemaUnjudged decoder: Decoder) throws { try self.init(from: decoder) }`. An alias of a type that has none (`type LaterTitle = String;`) is declared above the type that holds it. Declared below, the Swift compiler answers `value of type 'LaterTitle' (aka 'String') has no member 'validate'`.
- **An alias of a string that a field bounds** is declared above that field's type. Declared below, it is a name the emitter reads as a brand, and the Swift compiler answers `value of type 'LaterTitle' (aka 'String') has no member 'swiftSchemaBoundText'`. The same error answers a length or a pattern on a field typed with a struct or an enum, which holds no string to measure.

#### A Swift read refuses what `validate()` refuses

`init(from:)` of every type `swift_definition()` writes reads the type and then calls `validate()` on the value it read, as [the Rust read](#a-read-refuses-what-validate-refuses) does. A payload whose value breaks a bound or a check is thrown as `DecodingError.dataCorrupted`, and no value is handed back:

```swift
do {
  let manifest = try JSONDecoder().decode(Manifest.self, from: payload)
  carry(manifest)
} catch DecodingError.dataCorrupted(let context) {
  print(context.debugDescription)
}
```

The `debugDescription` is the list `validate()` answers of the value read, every message, joined by `; `. A manifest that holds a broken parcel in a field, a list, a map and an optional is refused once, each message at its full path. The error is one line, broken here after each `; `:

```text
'firstParcel.label': too short: minimum length is 1, got 0;
'parcelList.1.fragileItemCodes.1': y is outside of itemCodeList;
'parcelsByDock.north.label': too short: minimum length is 1, got 0;
'spareParcel.fragileItemCodes.1': y is outside of itemCodeList
```

The text holds no coding path. `codingPath` is the decoder's own, as for every decoding error: empty for the value the read was asked for, and `1` for the second manifest of a `[Manifest]`.

**What the emitter writes.** Every struct and every enum has two initializers that read it:

```swift
public init(swiftSchemaUnjudged decoder: Decoder) throws {
  let container = try decoder.container(keyedBy: CodingKeys.self)
  self.label = try container.decode(String.self, forKey: .label)
  self.itemCodeList = try container.decode([String].self, forKey: .itemCodeList)
  self.fragileItemCodes = try container.decode([String].self, forKey: .fragileItemCodes)
}

public init(from decoder: Decoder) throws {
  try self.init(swiftSchemaUnjudged: decoder)
  let violations = validate()
  if !violations.isEmpty {
    throw DecodingError.dataCorrupted(DecodingError.Context(codingPath: decoder.codingPath, debugDescription: violations.joined(separator: "; ")))
  }
}
```

- **`init(swiftSchemaUnjudged:)` is the read alone.** It builds the value from the same payload and judges nothing. It is what a type that holds another reads it with, and what reads a value that is to be repaired: call it from an `init(from:)` of your own, which is where a `Decoder` is handed out.
- **Every struct declares its memberwise initializer**, `public init(label: String, itemCodeList: [String], fragileItemCodes: [String])` here: the one Swift synthesizes is gone once a struct writes an `init(from:)`. A value built with it is judged when you call `validate()`.
- **`encode(to:)` is what it was**, the one Swift synthesizes wherever it was. Writing judges nothing.
- **A struct's member under `#[serde(skip_serializing)]` is read, and encoded under no key**, as serde writes none, and a flattened one encodes nothing into its holder. A flattened one that is no `Option` is read as any flattened member is, its `default` aside, as serde reads it: a payload without a key its struct requires, or with an entry its map does not read, is refused. A struct that holds a member under `skip_serializing` has an `encode(to:)` tixschema writes in place of Swift's, which would encode every property. A member under `skip_serializing_if` is encoded wherever it holds a value: `encode(to:)` cannot call the Rust function, so under any function but `Option::is_none`, `Vec::is_empty` say, it encodes the key where serde leaves it out.
- **A plain enum that names no check** keeps the `init(from:)` Swift synthesizes, and its `init(swiftSchemaUnjudged:)` is that read.

**A type that holds another reads it with no judgement**, handed a decoder of its own, so the whole value is judged once, by the type the read was asked for:

```swift
_ = try container.decodeNil(forKey: .firstParcel)
self.firstParcel = try Parcel(swiftSchemaUnjudged: container.superDecoder(forKey: .firstParcel))

var listed = try container.nestedUnkeyedContainer(forKey: .parcelList)
var items: [Parcel] = []
while !listed.isAtEnd { items.append(try Parcel(swiftSchemaUnjudged: listed.superDecoder())) }
self.parcelList = items

let keyed = try container.nestedContainer(keyedBy: SwiftSchemaAnyKey.self, forKey: .parcelsByDock)
var entries: [String: Parcel] = [:]
for entry in keyed.allKeys { entries[entry.stringValue] = try Parcel(swiftSchemaUnjudged: keyed.superDecoder(forKey: entry)) }
self.parcelsByDock = entries

if container.contains(.spareParcel), try !container.decodeNil(forKey: .spareParcel) { self.spareParcel = try Parcel(swiftSchemaUnjudged: container.superDecoder(forKey: .spareParcel)) } else { self.spareParcel = nil }
```

`SwiftSchemaAnyKey` is a private key type written into each type that reads a map entry by entry. A value is read with no judgement exactly where the `validate()` of its holder runs its `validate()`. Anywhere else it is read through `Decodable` and judges itself: the first broken one ends the read with its own messages, under the coding path it sits at.

| Held | Judged |
|---|---|
| in a field, an optional, a list, a map's value, a tuple, a flattened member or the payload of a variant, in any nesting of those | once, with its holder, each message at its full path |
| under one of the holder's type parameters | by itself, as it is read: a `Crate<Parcel>` whose second parcel is broken is refused with `'fragileItemCodes.1': y is outside of itemCodeList`, under the coding path `codes`, `1` |
| in a field that carries a bound of its own, a list apart | by itself, as it is read: the field's bound stands in place of what its type declares in `validate()`, so the holder judges nothing of the type |
| as the value a brand wraps | by itself, as it is read |

- **An absent key and a `null` are read as they were.** A key a struct requires is still `keyNotFound` where it is absent, which is what the first statement above is for: `superDecoder(forKey:)` answers a decoder over `null` for an absent key. An optional member is still `nil` for an absent key and for a `null`. A key under `#[serde(default)]` is read as its default where it is absent ([A Key Read as Its Default](#a-key-read-as-its-default-serdedefault)).
- **An `#[serde(untagged)]` enum takes the variant serde takes.** Its `init(swiftSchemaUnjudged:)` takes the first variant whose payload reads and whose own bounds hold: a bound written on a member, and the `pattern`, `minLength` and `maxLength` of a [brand](#branded-newtypes) the member holds, which the brand's wrapper answers for alone with `swiftSchemaValidateBound()`. A check and a model the variant holds choose nothing: the variant is taken, and `init(from:)` then refuses the value. A brand declared below the enum is not known to carry a bound where the enum is written, and chooses nothing either: declare it above the enum.
- **An externally tagged enum is read from an object of one key, or a unit variant from its name**, as serde reads it. With `enum Hatching { Bay { label: String }, Calm, Dock { label: String } }`, `{"Bay":{"label":"n"}}` reads `Bay` and `"Calm"` reads `Calm`. An object of several keys, `{"Bay":{"label":"n"},"Dock":{"label":"s"}}` or `{"Bay":{"label":"n"},"north":2}`, is refused with `DecodingError.dataCorrupted` and `more than one key for Hatching`, and `{}` with `empty object for Hatching`, as serde and the generated Zod schema refuse each.
- **A fixed array `[T; N]` and a tuple are read from an array of their length alone**, as serde and the generated Zod schema read each. With `struct Clenching { pair: [u8; 2] }`, `{"pair":[1,2,3]}` is refused with `DecodingError.dataCorrupted` and `invalid length 3, expected 2`, and `{"pair":[1]}` with `invalid length 1, expected 2`, at the coding path of the array, `pair`, by `init(from:)` and by `init(swiftSchemaUnjudged:)`. A `[[u8; 2]; 3]` refuses an inner array of another length in the same words, at the path of that array, and the outer one at its own. A level written as `[T; N]` is read item by item wherever it is held: in a field, an optional, a map's value, a tuple's slot, a variant's payload, a tuple struct of one slot, a brand, a member a [`preprocess`](#preprocess-in-the-swift-client) corrects, an alias declared above its holder and a type argument. A fixed array that fills a type parameter is held in a wrapper, `{Holder}{Field}Argument{N}Fixed`, whose `value` is the array, as the type it fills reads it through `Decodable`. An alias declared below its holder, or in another crate, and one that declares a parameter, are read through `Decodable`, which takes an array of any length. With `struct Thawing(String, u32)`, `["b",1,2]` is refused with `invalid length 3, expected 2` and `["b"]` with `invalid length 1, expected 2`, and so is the array of a member's tuple, of a tuple a tuple holds and of a variant of several slots: the tuple is held to the length its decoder knows of the array, which `JSONDecoder` knows. An `#[serde(untagged)]` enum takes the variant serde takes: with `enum Grappling { Pair([u8; 2]), Spread(Vec<u8>) }`, `[1,2,3]` reads `Spread`.
- **An enum under a tag and a content key reads a unit variant from an object with no content key**, as TypeScript and Zod read it: with `#[serde(tag = "kind", content = "held")] enum Becalming { Breeze(bool), Lull }`, `{"kind":"Lull"}` reads `Lull`, and `{"kind":"Lull","held":5}` is refused with `DecodingError.dataCorrupted` and `Becalming: Lull takes no value`, as serde refuses it. `{"kind":"Lull","held":null}` is refused with the same error, as TypeScript and Zod refuse it, where serde reads `Lull`. serde writes the variant as its tag alone, so every payload serde writes is read.
- **A struct under its own `#[serde(tag = "...")]` reads a payload whose tag is the struct's name, and refuses any other**, as TypeScript and the generated Zod schema refuse it, whose tag is the literal of that name. serde reads the struct whatever the tag holds, and where it is absent, and writes the name back. With `#[serde(tag = "kind")] struct Flag { label: String }`, `{"kind":"Flag","label":"n"}` reads, `{"label":"n"}` is refused with `DecodingError.dataCorrupted` and ``missing field `kind` ``, `{"kind":"Other","label":"n"}` with `expected tag "Flag", found "Other"`, and `{"kind":5,"label":"n"}` with `expected tag "Flag", found 5`, where serde reads each as `Flag { label: "n" }`. The value found is quoted as JSON. A struct that names a [`preprocess`](#preprocess-in-the-swift-client) on itself reads the tag of what its functions answer. A struct with a `#[serde(flatten)]` member reads its tag the same way, and so does a struct flattened into a holder, from the object the holder hands it, so the holder refuses the same payloads. `encode(to:)` writes the tag with the struct's name, as serde writes it.
- **A `#[serde(flatten)]` member is read at its place, in the order declared**, as serde reads it. Every flattened member, the first one included, is handed the object without the keys the holder's own members take, those each struct flattened ahead of it took and those a holder of that holder took: the function a flattened member names is handed it, a model the holder's `validate()` walks is read from it unjudged, and a struct that reads one of those keys is refused, as serde refuses it with `missing field`. With `struct Whipping { #[serde(flatten)] flag: Pennant, label: String }` over a `Pennant` that reads `label`, `{"label":"n"}` is refused, the key being the holder's own. A struct with a `#[serde(flatten)]` member of its own, declared above its holder and naming no [`preprocess`](#preprocess-in-the-swift-client) on itself, hands that object on to the members it flattens and reads its own members from the whole object, so it reads a key a member of its holder's own or a struct flattened ahead of it took, as TypeScript and the generated Zod schema read it, where serde refuses the payload with `missing field`: with `struct Spritsail { #[serde(flatten)] earring: Earring, label: String }` over an `Earring` that reads `tag`, flattened in `struct Bowline { label: String, #[serde(flatten)] spritsail: Spritsail }`, `{"label":"n","tag":"t"}` reads `n` into both labels and `t` into the tag. A payload whose keys collide with none of those reads as it did before. A struct takes the keys it reads. A struct that names a [`preprocess`](#preprocess-in-the-swift-client) on itself, a flattened member that names one and a struct with a `#[serde(flatten)]` member of its own take no key, as serde reads each. A struct declared below its holder, or in another crate, is not known to take any: a member flattened after it is read from its keys as well, where serde reads it without them. A map is the [one member of its object](#intersection-types-serdeflatten), and is read entry by entry, without the keys the types around its holder took, the tag of an enum under a tag key among them. A map under a key Swift wraps is read by its wrapper, through an `init(swiftSchemaUnjudged:swiftSchemaWithout:)` tixschema writes for it in an extension. An optional map is `nil` wherever an entry does not read as its key and value, as serde reads it: `#[serde(default, flatten, skip_serializing_if = "Option::is_none")] counts: Option<BTreeMap<String, u8>>` reads `{"north":2}` as that map, `{}` as an empty one, and `{"north":"x"}` as `nil`, which is written back as `{}`, in a variant under its name, under a `content` key or untagged too, and keyed by a plain enum it reads a key that names no variant as `nil`. A model it holds is read with no judgement, so one that breaks its rule still refuses the payload, as serde refuses it. `encode(to:)` writes each flattened member at its place, a map under a key Swift wraps through its wrapper and a map of dates as the text serde writes, and a key two of them hold is written once, with the last value, where serde writes it twice. A flattened `Option` writes the keys of the value it holds beside its holder's, and nothing for `nil`, as serde writes nothing for `None`.
- **A flattened externally tagged enum takes one entry**, as serde reads it: the one under the name a variant is read under, its `rename` or the enum's `rename_all` included, or an alias of one, wherever it is in the object the enum is handed, and the enum is read from that entry. Every member flattened after it is handed the object without that entry. With `enum Hatching { Bay { label: String }, Calm, Dock { label: String } }` flattened in `struct Stowing { note: String, #[serde(flatten)] hatching: Hatching }`, `{"north":2,"Bay":{"label":"n"},"note":"a"}` reads `Bay` and the note `a`, and `{"Calm":null,"note":"a"}` reads `Calm`: a unit variant's entry is read over `null` alone, as TypeScript and Zod read it, where serde reads `{"Calm":{}}` too. Flattened ahead of a member that names a function, `{"Bay":{"label":"n"},"north":2}` reads `Bay`, and the function is handed `north`. A payload with no such entry is refused with `DecodingError.dataCorrupted` and `no variant of enum Hatching found in flattened data`, as serde refuses it, and an optional enum reads `nil` from it; an entry the enum does not read refuses the payload, as TypeScript and Zod refuse it, where serde reads `None`. A model the holder's `validate()` walks is read from the entry with no judgement, and judged once, with its holder. The enum is written as serde writes it, its variant's entry beside the holder's members, and a unit variant as its name over `null`: `Stowing(note: "a", hatching: .calm)` writes `{"note":"a","Calm":null}`, the enum optional or under a `preprocess` of its member as well. Under a `preprocess` of its member the enum is read by its own read, which refuses that entry with `unknown tag Calm`, as the Zod schema refuses it. An enum under a tag key alone and an untagged enum take no key, as serde reads each, and so does an enum that names a [`preprocess`](#preprocess-in-the-swift-client) on itself or on its member. An enum under a tag and a content key is not known to take any: a member flattened after it is handed the tag and the content, where serde hands it neither. An enum declared below its holder, or in another crate, is not known to take an entry: it is read from the object it is handed, and is refused where that object holds any other key, as TypeScript and the generated Zod schema refuse the payload, where serde reads past the key: with `Hatching` declared below `Stowing`, `{"note":"a","Bay":{"label":"n"},"north":2}` is refused with `more than one key for Hatching`, and `Stowing(note: "a", hatching: .calm)` writes `{"note":"a"}`, with no entry for its unit variant, where serde writes `{"note":"a","Calm":null}`.
- **A name serde writes is read and written as serde writes it, whatever it holds.** A member's, an alias's, a struct's own tag key and name, an enum's variant, tag key and content key, and a plain enum's value are each written between the quotes of a Swift string, a `"` and a `\` escaped: with `#[serde(tag = "ki$nd", rename = "Price's$Tag\\")] struct Tariffed { #[serde(rename = "back\\slash")] backslash: String, #[serde(rename = "cost$usd", alias = "cost's")] cost: String, #[serde(rename = "say \"hi\"")] quoted: String }`, `CodingKeys` declares `case quoted = "say \"hi\""`, and `{"ki$nd":"Price's$Tag\\","back\\slash":"b","cost$usd":"c","say \"hi\"":"s"}` is read and written back as serde writes it.
- **A name serde reads beside the one it writes is read too.** A variant is read under each of its `#[serde(alias = "...")]` names, under every tagging, flattened, and in a plain enum, and a member under each of its aliases, in a struct, flattened or not, and in a variant. Each is written under the name serde writes. With `#[serde(alias = "Quay")] Dock { #[serde(alias = "name")] label: String }`, `{"Quay":{"name":"s"}}` reads `Dock` and is written back `{"Dock":{"label":"s"}}`. A member under two of its names, `{"Dock":{"label":"s","name":"t"}}`, is refused with `DecodingError.dataCorrupted` and ``duplicate field `label` ``, as serde refuses it. A flattened struct takes each alias of its members off its holder's object, and a member flattened after a member of its holder's own under an alias is handed the object without each of that member's names, as serde does. A struct reads a member under an alias from a container of every key, `let aliased = try decoder.container(keyedBy: SwiftSchemaAnyKey.self)`, under the one name the payload holds it under, and a plain enum with a variant under an alias declares its own `public init?(rawValue: String)`, which takes each alias, so every read of its raw value takes them, the key of a map among them. A type that names no alias is written as it was. An externally tagged enum read alone refuses a unit variant's entry over `null` under each of its names: with `#[serde(alias = "Still")] Calm`, `{"Still":null}` is refused with `unknown tag Still` as `{"Calm":null}` is, as TypeScript and the generated Zod schema refuse both, where serde reads `Calm`.
- **A payload with entries under two of a flattened enum's names is refused**, where serde reads the first entry in the object's order. Swift keeps no order of an object's keys, so it cannot tell which entry is the first. `{"note":"a","Bay":{"label":"n"},"Dock":{"label":"s"}}` is refused with `DecodingError.dataCorrupted` and `more than one variant of enum Hatching found in flattened data: Bay, Dock` at the holder's coding path, the names in the order the enum declares its variants, whatever order the payload holds them in, and so is a variant's entry beside an entry under its alias, the enum optional too. TypeScript refuses that payload as well, `error TS2322: Type '{ note: string; Bay: { label: string; }; Dock: { label: string; }; }' is not assignable to type 'Stowing'.`, and so does the generated Zod schema, with `Invalid input`.
- **A flattened `Option` of any type but a map is read as `nil` where the object it is handed holds none of that type's keys, and a key there that does not read refuses the payload**, as TypeScript and Zod refuse it, where serde reads `None` and drops the key with no error. With `struct Flaking { #[serde(flatten, default, skip_serializing_if = "Option::is_none")] tabling: Option<Tabling> }` over a `Tabling` that reads `label`, `{}` is read with `tabling` `nil` and `{"label":"north"}` with the `Tabling` it holds, and `{"label":5}` is refused, where serde reads `None`. A struct of two keys is refused where one of them is there and the other is not. The keys are those serde reads the type under, each member's name and each alias, and they are looked for in the object the member is handed, without the keys its holder's own members and the members flattened ahead of it take. An externally tagged enum is `nil` where the object holds no entry under one of the names its variants are read under, an enum under a tag key alone where it holds no tag, and an enum under a tag and a content key where it holds neither, and each is refused where one of those is there and the enum does not read it: with `#[serde(tag = "kind", content = "body")] enum Becket { Calm, Round { turns: u32 } }` flattened beside `note`, `{"note":"n"}` reads `nil`, and `{"kind":"Gale","note":"n"}` and `{"body":{"turns":2},"note":"n"}` are refused, where serde reads `None`. A type whose keys are not known is read as `nil` wherever its read throws, as serde reads `None`: a struct declared below its holder or in another crate, a struct that reads no key of its holder's (one with a `#[serde(flatten)]` member of its own, one that names a [`preprocess`](#preprocess-in-the-swift-client) on itself, one with no member serde reads), an untagged enum, an enum declared below its holder or in another crate, an enum that names a [`preprocess`](#preprocess-in-the-swift-client) on itself, an alias and a type parameter. A model the holder's `validate()` walks is read with no judgement where it reads, so one that breaks its rule still refuses the payload, as serde refuses it. A flattened `Option` that names a [`preprocess`](#preprocess-in-the-swift-client) is read as the Zod schema of its holder reads it: its functions are called first, on the object the member is handed, and the type is read from what they answer. Where that read throws, an answer of `null` among it, it is `nil` where that object holds none of a struct's keys, or no key at all for a type whose keys are not known, an enum among them, and the payload is refused where the object holds one. With `struct Reeved { note: String, #[model_schema_prop(preprocess = ["asGiven"])] #[serde(flatten, default, skip_serializing_if = "Option::is_none")] tholepin: Option<Tholepin> }` over a `Tholepin` that reads `label`, and `func asGiven(_ payload: Any) -> Any { payload }`, `{"label":"north","note":"n"}` reads that `Tholepin`, `{"note":"n"}` reads `nil`, where serde refuses it with `missing field`, and `{"label":5,"note":"n"}` is refused. Under a function that answers `NSNull()`, `{"label":"north","note":"n"}` is refused, as Zod refuses it, where serde reads `None`.
- **What a variant under a tag alone holds is read from the object without the tag**, as serde reads it: serde takes the tag off the object and hands the variant the rest. The type a variant holds as its one slot, and each member a variant flattens, beside a member of its own or alone, are read from that object: a type that takes the keys the types around it took through `init(swiftSchemaUnjudged:swiftSchemaWithout:)`, the tag among those keys, and any other through [`SwiftSchemaPayload`](#preprocess-in-the-swift-client), from the object without them. With `#[serde(tag = "kind")] enum Grafting { Held { #[serde(flatten)] flag: Kinded }, Slot(Kinded) }` over a `Kinded` that reads `kind`, `{"kind":"Held"}` and `{"kind":"Slot"}` are each refused with `missing field`, as serde refuses them. A model the enum's `validate()` walks is still read with no judgement.
- **The service clients answer the fault they had** for a reply that breaks a rule, the thrown error reaching the handling they have for a reply that does not read: `swift_http_client()` answers `undeserializable-payload` and `swift_ws_client()` answers `failed-validation`, each with the messages in its `detail`.

### Literal Values

You can constrain a `String` field to a specific literal value using `model_schema_prop(literal = "value")`. This generates `z.literal("value")` in the Zod schema and a literal type in TypeScript, while the Rust field remains a `String`.

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    Generate {
        #[model_schema_prop(literal = "document")]
        value: String,
    },
}
```

Generated:

```typescript
export type Action = {
  type: "generate";
  value: "document";
};

export const Action$Schema = z.discriminatedUnion("type", [
  z.strictObject({
    type: z.literal("generate"),
    value: z.literal("document"),
  }),
]).meta({
  examples: []
});
```

`literal` also accepts a boolean or a numeric value, provided the field's own Rust type can carry it: `literal = true`/`literal = false` on a `bool` field, `literal = 214` on any numeric field. Every surface renders the literal in the field's own kind, so the kind written must match the field's declared type — `literal = true` on a `String` field, or `literal = "x"` on a `bool` field, is a compile error naming both sides.

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminUser {
    pub id: String,
    #[model_schema_prop(literal = true)]
    pub is_system_admin: bool,
    pub email: String,
}
```

Generated:

```typescript
export type AdminUser = {
  id: string;
  isSystemAdmin: true;
  email: string;
};

export const AdminUser$Schema = z.strictObject({
  id: z.string(),
  isSystemAdmin: z.literal(true),
  email: z.string(),
}).meta({
  examples: []
});
```

A literal takes no other rule. `minLength`, `maxLength`, `pattern`, `minimum`, `maximum` or `check` written beside it is a compile error in every build, naming the field, the literal and each rule: a literal admits one value, so a rule beside it is always met or never met. `as`, `preprocess`, `ts_optional` and `nullable` hold the value to no rule, and are written beside a literal as on any field.

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Envelope {
    #[model_schema_prop(literal = "v1", minLength = 3)]
    pub version: String,
}
```

```text
error: model_schema: field `version`: `literal = "v1"` takes no other rule, and `minLength` is written beside it — a literal admits one value, so the rule is always met or never met. Drop `minLength`, or drop the literal.
```

An enum's own check does not reach a field that carries a literal ([A Check Written on the Enum](#a-check-written-on-the-enum)).

**Recommended: Single-value enums over literal strings.** While `model_schema_prop(literal = ...)` works, a single-value enum provides type safety in Rust -- the field can only hold the correct value at compile time, whereas a `String` with a literal annotation can hold any string in Rust (the constraint only applies in the generated TypeScript/Zod output).

```rust
// Correct: single-value enum
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub enum DocumentLiteralValue {
    #[serde(rename = "document")]
    Document,
}

#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Action {
    Generate {
        value: DocumentLiteralValue,  // Type-safe in Rust
    },
}

// Wrong: not recommended
#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ActionAlt {
    Generate {
        #[model_schema_prop(literal = "document")]
        value: String,  // Any string value accepted in Rust
    },
}
```

Both approaches produce identical TypeScript and Zod output. The single-value enum is preferred because:

- **Type safety in Rust** -- impossible to construct with a wrong value
- **Self-documenting** -- the enum name makes the intent clear
- **Pattern matching** -- match on `DocumentLiteralValue::Document` instead of checking string equality
- **Naming convention**: Use the `<Something>LiteralValue` naming pattern (e.g., `DocumentLiteralValue`)

### Type Names (`as`)

Use `as` to name the type a field renders. The target must be the type the field already renders --
either the field's own type, or the value under its wrappers, so `as = String` is written on a
`String`, an `Option<String>` and a `Vec<String>` alike:

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct ApiConfig {
    pub id: String,
    #[model_schema_prop(as = String)]
    pub metric_type: String,
    #[model_schema_prop(as = String, minLength = 1)]
    pub tags: Vec<String>, // at least one tag: a length on a list counts its items
    pub enabled: bool,
}
```

Naming any other type is a compile error. The key cannot override the emitted type: all three
surfaces are written from the field's declared type because that is the type serde reads and writes,
and a `serialize_with` names a function whose output the expansion cannot see -- so a target that
rendered differently would describe a payload serde never produces. `as` also cannot be written
beside `preprocess`; the two have no defined order.

### Preprocessing (`preprocess`)

`preprocess = ["name", ...]` names functions of your own that correct a payload before a value is read from it: a label written with spaces at its ends, a key an older writer left out. It is written on a type, in `#[model_schema(...)]`, and on a field, in `#[model_schema_prop(...)]`. A name is written once, and every read calls a function under it: Rust the snake case of the name, `withoutFragile` as `without_fragile`, and the Zod schema, [the Dart client](#a-preprocess-in-the-dart-client), [the Swift client](#preprocess-in-the-swift-client) and [the Kotlin client](#preprocess-in-the-kotlin-client) the name as written. [Preprocess and check](#preprocess-and-check) states the functions of the five side by side.

**The key is a last resort.** A function of yours runs on every read of the type, on a payload no schema describes. Reach for it where the payload cannot be corrected where it is written, and where a serde `default`, an `Option` or an `alias` does not say what you mean.

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::{model_schema, model_schema_prop};

#[model_schema(preprocess = ["withoutFragile"])]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parcel {
    #[model_schema_prop(minLength = 1, preprocess = ["trimmed"])]
    pub label: String,
    pub item_code_list: Vec<String>,
    pub fragile_item_codes: Vec<String>,
}

#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub parcel: Parcel,
}

// A field's function takes the payload of the field, and answers the payload the field is read from.
fn trimmed(payload: Value) -> Value {
    match payload {
        Value::String(text) => Value::String(text.trim().to_owned()),
        other => other,
    }
}

// A type's function takes the whole payload, ahead of every field's.
fn without_fragile(mut payload: Value) -> Value {
    if let Some(object) = payload.as_object_mut() {
        object
            .entry("fragileItemCodes")
            .or_insert_with(|| Value::Array(Vec::new()));
    }
    payload
}

fn main() {
    // Row 1: the label is read without the spaces at its ends.
    let parcel: Parcel = serde_json::from_str(
        r#"{"label":"  A  ","itemCodeList":["x"],"fragileItemCodes":["x"]}"#,
    )
    .unwrap();
    assert_eq!(parcel.label, "A");

    // Row 2: the bound judges the label the function answered.
    let refused = serde_json::from_str::<Parcel>(
        r#"{"label":"   ","itemCodeList":["x"],"fragileItemCodes":["x"]}"#,
    )
    .unwrap_err();
    assert_eq!(
        refused.to_string(),
        "'label': too short: minimum length is 1, got 0"
    );

    // Row 3: the type's function gives the payload the key it leaves out.
    let parcel: Parcel = serde_json::from_str(r#"{"label":"A","itemCodeList":["x"]}"#).unwrap();
    assert!(parcel.fragile_item_codes.is_empty());

    // Row 4: the same payload as a BSON document.
    let stored = bson::doc! { "label": "A", "itemCodeList": ["x"] };
    let parcel: Parcel = bson::from_document(stored).unwrap();
    assert!(parcel.fragile_item_codes.is_empty());

    // Row 5: a type that holds a parcel reads it through the parcel's functions.
    let manifest: Manifest = serde_json::from_str(
        r#"{"parcel":{"label":"  A  ","itemCodeList":["x"],"fragileItemCodes":["x"]}}"#,
    )
    .unwrap();
    assert_eq!(manifest.parcel.label, "A");

    // A key the payload does not hold is read as serde reads it: no function is called for it.
    let missing = serde_json::from_str::<Parcel>(r#"{"itemCodeList":["x"]}"#).unwrap_err();
    assert_eq!(missing.to_string(), "missing field `label`");

    // A value built in code is never read, so nothing corrects it.
    let built = Parcel {
        label: "  A  ".to_owned(),
        item_code_list: Vec::new(),
        fragile_item_codes: Vec::new(),
    };
    assert_eq!(built.label, "  A  ");
}
```

| # | Payload | A Rust read | The Zod schema |
|---|---|---|---|
| 1 | `{"label":"  A  ","itemCodeList":["x"],"fragileItemCodes":["x"]}` | read; `label` is `"A"` | the same |
| 2 | `{"label":"   ","itemCodeList":["x"],"fragileItemCodes":["x"]}` | refused: `'label': too short: minimum length is 1, got 0` | the same line |
| 3 | `{"label":"A","itemCodeList":["x"]}` | read; `fragile_item_codes` is `[]` | read; `fragileItemCodes` is `[]` |
| 4 | row 3 as a BSON document | read, the same value | not applicable |
| 5 | row 1 inside `Manifest`, under `parcel` | read; `parcel.label` is `"A"` | the same |

Row 4 is read by the `bson` library's own read of a document, `bson::from_document` in version 2 and `bson::deserialize_from_document` in version 3.

#### The functions you write

A Rust function is in scope where the type is declared, a type declared inside a function body included. A TypeScript function is imported by the file the generated schemas are written into.

| | Rust | TypeScript |
|---|---|---|
| A field's function | `fn trimmed(payload: serde_json::Value) -> serde_json::Value` | `function trimmed(payload: unknown): unknown` |
| A type's function | `fn without_fragile(payload: serde_json::Value) -> serde_json::Value` | `function withoutFragile(payload: unknown): unknown` |

The two functions of the example in TypeScript:

```ts
export function trimmed(payload: unknown): unknown {
  return typeof payload === "string" ? payload.trim() : payload;
}

export function withoutFragile(payload: unknown): unknown {
  if (typeof payload !== "object" || payload === null || Array.isArray(payload)) {
    return payload;
  }
  return "fragileItemCodes" in payload ? payload : { ...payload, fragileItemCodes: [] };
}
```

- **A field's function takes the payload of the field**, and a type's takes the whole payload. Each answers the payload the value is then read from, and answers a payload it has nothing to correct in as it is: the read takes it or refuses it in its own words.
- **The payload is a `serde_json::Value`** whatever it was read from, a JSON text, a `serde_json::Value` or a BSON document, so one function serves every read and a crate that names a `preprocess` lists `serde_json`. It is untyped: `{"label": 5}` reaches `trimmed` as the number it is, and the read refuses what the function leaves, ``invalid type: integer `5`, expected a string``.
- **Several functions run in the order written**, each over the answer of the one before: `preprocess = ["unwrapped", "withoutFragile"]` calls `without_fragile(unwrapped(payload))`.
- **A type with a parameter writes its function once**: the payload has no type, so one function serves every filling.
- **The function of a [branded newtype](#branded-newtypes)** is written on the type, and takes the payload of the value the brand is written over.

#### The order of one read

1. The type's functions, on the whole payload.
2. Each field's functions, as that field is read.
3. serde builds the value.
4. [`validate()`](#the-validate-method) judges it, where the type publishes one: a bound and a [check](#checks-check) judge what the functions answered, which is why row 2 is refused.

The Zod schema keeps the same order. A type's wrap is around its own expression and its `.check(...)`, a field's around the field's schema and its bounds:

```ts
const Parcel$RawSchema = z.preprocess(withoutFragile, z.strictObject({
  label: z.preprocess(trimmed, z.string().min(1, { error: (issue) => `too short: minimum length is 1, got ${String(issue.input).length}` })),
  itemCodeList: z.array(z.string()),
  fragileItemCodes: z.array(z.string()),
}));
```

Several names nest in the order written, the first outermost, so it is the first one called. A type's wrap is inside `.brand()` on a brand, inside the `.meta({ ... })` every binding carries, and inside the builder of a type with a parameter, so every filling of its factory carries it.

#### Where a function is called

**In every serde read** of a type that derives `Deserialize`, in every build: `serde_json::from_str`, `serde_json::from_value`, a BSON document under either major of the `bson` library, and any other format that describes itself. `#[model_schema()]` writes the `Deserialize` of a type that names a `preprocess`, on itself or on a field, as it writes that of a type that [publishes `validate()`](#a-read-refuses-what-validate-refuses), whether or not the type publishes one. Every shape takes the key on the type: a struct with named fields, a tuple struct, a unit struct, an enum under every tagging and a plain one, a branded newtype, and a type with a parameter at every filling.

A type that holds another reads it through that type's functions wherever it holds it: in a field, an `Option`, a list, a map's value, a `#[serde(flatten)]` member, and in a variant under every tagging. A type that reaches itself is corrected at every level. A function of a flattened type is handed the keys the holder's own members do not take, where the Zod schema hands it the holder's whole object.

**In the [recovering decode](#recovering-decode-decode_with)** of a type whose `enable` list holds `"decode_with"`. `from_value_with`, `from_value_piped`, `from_bson_with` and `from_bson_piped` call the type's functions once, on the value as they take it, so the callback and the resolvers are handed the corrected value, the one their repairs are read from, and no issue is listed for what a function corrected. A type held by another is corrected where it is walked. A field that names a function is read whole through it by the read and by the listing of issues, each calling it once: a payload the read refuses once corrected is one `Invalid` at the field, and so is a bound the corrected value breaks. The value the callback is handed holds the field as it was written.

**In a [MongoDB operation](#mongodb-operations-and-typed-filters)** that reads a row, each row being read through `from_bson_piped`, and in a [service's dispatcher](#service-validation), which reads a request through the functions of its message.

**In the Zod schema**, as one `z.preprocess(name, ...)` per name.

**In [the Dart client](#a-preprocess-in-the-dart-client)**, in the read of every type `dart_definition()` writes, under the `dart` feature.

**In the Swift client**, in every read of a type `swift_definition()` writes: [`preprocess` in the Swift client](#preprocess-in-the-swift-client) is what it calls and what it is handed.

**In the read of the Kotlin client**, by the serializers of each type: see [`preprocess` in the Kotlin client](#preprocess-in-the-kotlin-client).

**Nowhere else.** Writing calls no function, `Serialize` being serde's derive, and neither does `validate()`: a value built in code is never read, so nothing corrects it. The JSON Schema describes the value the type is read to and calls no function.

#### A key the payload does not hold

A Rust read answers a key the payload does not hold as serde does, and calls no function for it: a `#[serde(default)]` or an `Option` supplies the value, and any other field is refused with ``missing field `label` ``. The Zod schema calls the field's function for such a key, with `undefined`, or with the default of a key under `#[serde(default)]` ([A Key Read as Its Default](#a-key-read-as-its-default-serdedefault)).

So a field's function that turns `undefined` into a value makes the Zod schema read a payload the Rust read refuses. Where a key may be left out, say so on the field, with `#[serde(default)]` or an `Option`, or give the payload the key in the type's function, which both sides call on the whole payload, as `withoutFragile` does. A `null` the payload holds is a payload: both sides hand it to the field's function.

#### A payload that is not JSON

A function takes a `serde_json::Value`, so what is read crosses to one through serde, and what the function answers is read through serde again. A type's function makes the whole payload cross, and a field's makes that field's payload cross and no other. The format has to describe itself: the crossing asks it what each value is.

Under BSON, measured against versions 2 and 3 of the `bson` library:

| The document holds | The function is handed | The read answers |
|---|---|---|
| an `ObjectId` | `{"$oid": "0123456789abcdef01234567"}` | the same `ObjectId`, where the function leaves it as it is |
| a date | `{"$date": "2023-11-14T22:13:20Z"}` | the same date |
| an `Int32`, an `Int64`, a `Double`, a text, a list | the JSON value of the same content | the same value |
| a `Double` that is `NaN` or infinite | `null` | what the field reads `null` as: an `f64` is refused, `invalid type: null, expected f64` |
| a generic binary, a `Decimal128` | nothing: the crossing is refused | `invalid type: byte array, expected any valid JSON value` |

A type that holds a value of the last row keeps `preprocess` off itself and off that field. Version 3 of the `bson` library writes each refusal inside a sentence of its own, `BSON error. Kind: A deserialization-related error occurred. Message: ... .`

In the recovering decode the document the callback is handed has crossed back to BSON: an `ObjectId` and a date are the BSON values they were, and an `Int32` is an `Int64`. A document that cannot cross is answered `Unrecovered` with one `Undescribed` issue holding the refusal, and the callback is not called.

#### What is refused

The key is read in every build, and each refusal is spanned on what was written:

| Written | The refusal says |
|---|---|
| `preprocess = "trimmed"` | takes an array of string literals, each naming a function: `preprocess = ["trimmed"]` |
| `preprocess = [trimmed]` | takes string literals: each element is a string literal that names a function |
| `preprocess = ["(v) => v"]` | names `(v) => v`, and a name is an identifier: ASCII letters, digits and `_`, not led by a digit. The name is written as a function's name in every language |
| `preprocess = []` | names no function: write one name at least, or drop the key |
| `preprocess = ["trimmed", "trimmed"]` | names `trimmed` twice: the name is written twice, and a function runs once |

A type that derives `Deserialize` is refused the key in five more places, each naming what to write instead:

- **Beside a `deserialize_with` or a `with` of your own on the same field.** serde reads a field through one function, and the macro's own is the one that calls yours: call the function from your own reader, or take the attribute off.
- **On a field that borrows from the payload**, a `&'a str` or a `#[serde(borrow)]`, and on a type that holds one. What a function answers is a payload of the read's own, which nothing read from it can borrow from.
- **On a type alias**, which has no read of its own: write it on the type the alias names, or on a field that holds the alias.
- **On the slot of a [branded newtype](#branded-newtypes)**, as every key written there is: a brand's `preprocess` is written on the type.
- **Beside [`as`](#type-names-as)** on the same field.

`#[serde(remote = ...)]`, `from`, `try_from` and `into` are refused on a type that names a `preprocess`, as they are on one that [publishes `validate()`](#a-read-refuses-what-validate-refuses): the macro writes the read, and cannot put it behind a conversion it does not write. A type that derives no `Deserialize` is never read: it calls no function and is refused none of these.

**A name with no function in scope does not build**, and the error is reported at the name:

```rust,compile_fail
#[model_schema(preprocess = ["withoutFragile"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    pub label: String,
}
```

```text
error[E0425]: cannot find function `without_fragile` in this scope
  |
4 | #[model_schema(preprocess = ["withoutFragile"])]
  |                              ^^^^^^^^^^^^^^^^ not found in this scope
```

What resolves the error is a function `without_fragile` in scope where `Parcel` is declared.

**What the macro writes.** The type's `Deserialize` is written as [that of a type that publishes `validate()`](#a-read-refuses-what-validate-refuses) is: serde's derive runs on a private copy of the item inside `model_schema_read_unjudged`, and a field's functions are called by a function local to that read, hung on the copy's member. A type that names a `preprocess` on itself gains `model_schema_read_corrected` as well, the read of a payload its functions have already corrected, and one declared with `decode_with` gains `decode_with_read_corrected`, `decode_with_value_corrected` and the walk of a corrected value under `decode_with_value_corrected_issues`, `_named` and `_fields`, with the `bson` twin of each under the `bson` feature. Each is a name of the type's own: a function of yours under one of them on the same type is a duplicate definition (E0592).

#### A `preprocess` in the Dart client

With the `dart` feature, `fromJsonUnjudged` of every type `dart_definition()` writes calls the functions the type and its members name, each under the name the attribute writes. [`fromJson` is that read followed by `validate()`](#the-dart-read-refuses-what-validate-refuses), so both reads correct the payload, in [the order of a Rust read](#the-order-of-one-read): the type's functions, each member's as it is read, the value, then `validate()`. The parcel of the example is read by:

```dart
factory Parcel.fromJsonUnjudged(Map<String, dynamic> payload) {
  final json = withoutFragile(payload) as Map<String, dynamic>;
  return Parcel(
    label: (json.containsKey('label') ? trimmed(json['label']) : null) as String,
    item_code_list: (json['itemCodeList'] as List<dynamic>).map((e) => e as String).toList(),
    fragile_item_codes: (json['fragileItemCodes'] as List<dynamic>).map((e) => e as String).toList(),
  );
}
```

The functions are yours, in scope in the file the emitted definitions are written into. Each takes a payload and answers the payload the value is read from:

| | Dart |
|---|---|
| A field's function | `dynamic trimmed(dynamic payload)` |
| A type's function | `dynamic withoutFragile(dynamic payload)` |

```dart
dynamic trimmed(dynamic payload) => payload is String ? payload.trim() : payload;

dynamic withoutFragile(dynamic payload) =>
    payload is Map<String, dynamic> ? {'fragileItemCodes': <dynamic>[], ...payload} : payload;
```

With those two functions, `Parcel.fromJson` and `Manifest.fromJson` answer for the rows of [the table above](#preprocessing-preprocess) what the Rust read answers. Row 4 is a BSON document, which no Dart client reads:

| # | A Dart read |
|---|---|
| 1 | read; `label` is `"A"` |
| 2 | refused with a `FormatException`: `'label': too short: minimum length is 1, got 0` |
| 3 | read; `fragile_item_codes` is `[]` |
| 5 | read; `parcel.label` is `"A"` |

- **The key stays a last resort.** Each name is one more function you write and keep in step, in every language you emit.
- **The payload is what `jsonDecode` holds**: a `Map<String, dynamic>`, a `List<dynamic>`, a `String`, a number, a `bool` or `null`. A function answers `dynamic`, and the read casts what it answers.
- **A field's function is called where the payload holds the key**, a `null` it holds included. A key the payload does not hold calls no function and is read as it was: `{"itemCodeList":["x"],"fragileItemCodes":["x"]}` is refused with `type 'Null' is not a subtype of type 'String' in type cast`, where the Rust read answers ``missing field `label` ``. A key under `#[serde(default)]` that the payload does not hold is read as its default, and no function is called for it. A slot's function is called with what its position holds.
- **Several functions run in the order written**, each over the answer of the one before: `preprocess = ["unwrapped", "withoutFragile"]` calls `withoutFragile(unwrapped(payload))`. One read calls each function once.
- **A type's function takes the payload its read takes**, typed as the read types it: the object of a class and of an enum under a tag key, the text of a plain enum, and the payload as it is for a brand, a tuple struct and any other enum. A payload of another kind does not reach it: a text handed to the read of a class is refused with a `TypeError` ahead of the function, where the Rust read hands the function any payload.
- **A type that holds another reads it through that type's own read**, so it is corrected wherever it is held: in a field, an optional, a list, a map's value, a variant and a `#[serde(flatten)]` member. A holder that names no function and flattens no member is written as it was.
- **A flattened member is read from the entries the members beside it and the members flattened ahead of it do not take**, as serde hands them: `Tagged.fromJsonUnjudged(({...json}..remove('dock')))`. A function it names, and a function of the type it holds, is handed those keys. Under a tag key, a type flattened into a variant and the function of a variant's one slot are handed the object without the tag as well. A type that one slot holds is read from the object with its tag, where the Rust read hands its function the object without it.
- **A flattened `Option` of any type but a map is read from what its functions answer as the Zod schema of its holder reads it**, where serde parts from Zod: where the type does not read the answer, `null` among it, the member is `null` where the object its functions are handed holds none of its struct's keys, or no key at all for a type whose keys are not known, an enum among them, and the payload is refused where it holds one, with the error of that read, as [the Dart read](#the-dart-read-refuses-what-validate-refuses) states. With `struct Reeved { note: String, #[model_schema_prop(preprocess = ["asGiven"])] #[serde(flatten, default, skip_serializing_if = "Option::is_none")] tholepin: Option<Tholepin> }` over a `Tholepin` that reads `label`, `{"note":"n"}` reads `tholepin` as `null`, and `{"label":5,"note":"n"}` is refused. serde refuses an answer its type does not read, the first as well, and reads `null` as `None`.
- **A sealed enum calls its functions in its base**, once, ahead of the dispatch on the tag, and an `#[serde(untagged)]` one ahead of every attempt. A plain enum calls them on the text, in `fromJsonUnjudged`.
- **The service clients read a reply through `fromJson`**, so a reply is corrected as any other payload is.
- **A name with no function does not compile**, and the Dart compiler reports the call: `Error: Method not found: 'trimmed'.`
- **A member of the type hides a function of its name** inside the class, as it hides a [check](#the-functions-you-write-in-dart): a `preprocess` named `label` on a type with a field `label` is reported as a name with no function. The names a read declares itself hide none: `json`, `payload`, `map`, `tag`, `read` and the converter it takes for a type parameter, `codeTypeFromJson`, are numbered where a function is called under one, `json2`.

#### `preprocess` in the Swift client

Every read of a type `swift_definition()` writes calls the functions the type and its members name: `init(from:)`, [`init(swiftSchemaUnjudged:)`](#a-swift-read-refuses-what-validate-refuses), and the service clients, which read a reply through `init(from:)`. Every shape takes the key on the type: a struct, a tuple struct, a unit struct, the wrapper of a [branded newtype](#branded-newtypes), an enum under every tagging and a plain one, and a type with a parameter at every filling. A member takes it in a struct, in the named members and the slots of a variant, and in the slots of a tuple struct.

**The key is a last resort in Swift as it is in Rust.** A function of yours runs on every read of the type, on a payload no type describes, and you write it again in each language that reads the type.

The two functions of the example in Swift, each under the name the attribute writes:

```swift
func trimmed(_ payload: Any) -> Any {
  guard let text = payload as? String else { return payload }
  return text.trimmingCharacters(in: .whitespaces)
}

func withoutFragile(_ payload: Any) -> Any {
  guard var members = payload as? [String: Any], members["fragileItemCodes"] == nil else { return payload }
  members["fragileItemCodes"] = [String]()
  return members
}
```

| | Swift |
|---|---|
| A field's function | `func trimmed(_ payload: Any) -> Any` |
| A type's function | `func withoutFragile(_ payload: Any) -> Any` |

A function is any function of the module the generated types are compiled in, and a type that names one needs `import Foundation`. A name with no function does not build:

```text
error: cannot find 'trimmed' in scope
```

- **The payload arrives as the Swift values of a JSON value**: an object as a `[String: Any]`, a list as an `[Any]`, a text as a `String`, `true` and `false` as a `Bool`, a number an `Int64` holds as an `Int64`, any other number as a `Double`, and `null` as an `NSNull`. `5`, `2.0` and `1e3` are each an `Int64`. It is untyped, as in Rust: `{"label": 5}` reaches `trimmed` as the `Int64` it is.
- **A function answers a payload in the same values**, and answers a payload it has nothing to correct in as it is. An answer that is no JSON value, a `Date` or a value of a type of your own, is thrown as `DecodingError.dataCorrupted`, `a preprocess answered a value that is not JSON`.
- **A function is not called for a key the payload does not hold.** The key is read as it is with no function named: `keyNotFound` for a member the type requires, the default for a member under `#[serde(default)]`, `nil` for an optional one. A `null` the payload holds is a payload, and the function is handed an `NSNull`.
- **A flattened `Option` of any type but a map is read from what its functions answer as the Zod schema of its holder reads it**, where serde parts from Zod: where the type does not read the answer, `null` among it, the member is `nil` where the object its functions are handed holds none of its struct's keys, or no key at all for a type whose keys are not known, an enum among them, and the payload is refused where it holds one, as [the Swift read](#a-swift-read-refuses-what-validate-refuses) states. serde refuses an answer its type does not read, and reads `null` as `None`.
- **Several functions run in the order written**, each over the answer of the one before: `preprocess = ["unwrapped", "withoutFragile"]` calls `withoutFragile(unwrapped(payload))`.
- **A type with a parameter writes its function once**, the payload having no type.
- **The order of one read is [the Rust read's](#the-order-of-one-read)**: the type's functions on the whole payload, each member's as that member is read, then the value is built, and `init(from:)` calls `validate()` on it. A bound and a check judge what the functions answered.

The Swift read answers the sample as the Rust read does:

| # | Payload | A Swift read |
|---|---|---|
| 1 | `{"label":"  A  ","itemCodeList":["x"],"fragileItemCodes":["x"]}` | read; `label` is `"A"` |
| 2 | `{"label":"   ","itemCodeList":["x"],"fragileItemCodes":["x"]}` | thrown, `DecodingError.dataCorrupted`: `'label': too short: minimum length is 1, got 0` |
| 3 | `{"label":"A","itemCodeList":["x"]}` | read; `fragileItemCodes` is `[]` |
| 5 | row 1 inside `Manifest`, under `parcel` | read; `parcel.label` is `"A"` |

**What the emitter writes** for `Parcel`, beside its properties, its `CodingKeys`, its memberwise initializer and its `init(from:)`:

```swift
private init(swiftSchemaCorrected decoder: Decoder) throws {
  let container = try decoder.container(keyedBy: CodingKeys.self)
  self.fragileItemCodes = try container.decode([String].self, forKey: .fragileItemCodes)
  self.itemCodeList = try container.decode([String].self, forKey: .itemCodeList)
  let labelPayload = try container.decode(SwiftSchemaPayload.self, forKey: .label).value
  self.label = try SwiftSchemaPayload.read(String.self, from: trimmed(labelPayload), at: container.codingPath + [CodingKeys.label])
}

public init(swiftSchemaUnjudged decoder: Decoder, swiftSchemaWithout taken: [String]) throws {
  self = try SwiftSchemaPayload.read(SwiftSchemaRead.self, from: withoutFragile(try SwiftSchemaPayload(from: decoder, without: taken).value), at: decoder.codingPath).value
}

public init(swiftSchemaUnjudged decoder: Decoder) throws {
  try self.init(swiftSchemaUnjudged: decoder, swiftSchemaWithout: [])
}
```

- **`SwiftSchemaPayload` is a private type written into each type that names a function**, on itself or on a member, into each type that reads a member from its object without some of its keys, a flattened member or what a variant under a tag alone holds, and into a struct under its own tag, which quotes a tag of another value through it: nothing is shared between two emitted types. It reads any JSON value from a `Decoder`, and reads a type back from what a function answers, through `JSONSerialization` and a `JSONDecoder` of its own.
- **A member's function is called in the type's own read**, on the payload of the member's key, and the member is read from what it answers.
- **A type's function is called by `init(swiftSchemaUnjudged:)`**, which reads the value from what it answers through the private `init(swiftSchemaCorrected:)`, the read of a payload the functions have already corrected. `SwiftSchemaRead` is the private type that hands that initializer a decoder.
- **A type that holds another reads it through that type's functions** wherever it holds it: in a field, an optional, a list, a map's value, a tuple, a flattened member and the payload of a variant. A model the `validate()` of its holder walks is read with no judgement where its member names a function too, through a private type written for the member, `SwiftSchemaReadFirstParcel` for `firstParcel`, so the whole value is still judged once.
- **A payload the read refuses once corrected is reported at the coding path of the value**, under the path of whatever holds the type: `label` for a parcel read alone, and `1`, `parcel`, `label` for the parcel of the second manifest of a list.
- **A function is handed the keys no type around its own took**, as [the Rust read hands them](#where-a-function-is-called). The function of a type flattened into a holder, and the function a flattened member names, is not handed the keys of the holder's own members, those a struct flattened ahead of it took, the entry an externally tagged enum flattened ahead of it took, nor those of a holder of that holder, and the function of a type a variant holds under a tag alone is not handed the tag. `init(swiftSchemaUnjudged:swiftSchemaWithout:)` is the read that takes those keys. It is written for a type that names a function on itself, and for every type that reads one from its own decoder: a holder that flattens it, an enum that holds it under a tag alone or with no tag, and a brand over it. The type is declared above the type that reads it: declared below, or held under a type parameter, it is read from the whole object of its holder, flattened from that object without the keys of the holder's own members, those a struct flattened ahead of it took and those of a holder of that holder, and as the one slot of a variant under a tag alone from that object without the tag.

### Optional TypeScript Keys (`ts_optional`)

The bare `ts_optional` flag asks for the optional key (`field?: T` rather than `field: T | undefined`) on the author's word. It is the only thing that writes that spelling.

**Two positions have no key for it to make optional, and writing it on either is a compile error, in every build.** A positional slot writes no key at all, so the flag names a spelling the tuple line has no room for. A member a serde attribute takes off the wire in *both* directions (`skip`, or `skip_serializing` and `skip_deserializing` together) is described on no surface, so there is no line for the flag to write its key on. An attribute that drops the key one way only (`skip_serializing_if`, `skip_serializing`) leaves the member standing and the flag decides its spelling as usual.

With the `serde` feature on, an `Option<T>` field must carry a key-dropping attribute anyway: its declared shape is `T | undefined`, so its key has to be dropped for a `None`, not written as `null` the way serde writes it absent an attribute saying otherwise — the guard refuses the declaration and names the attribute to add (or, if the wire really does carry `null` here, points at [`nullable`](#nullable-object-keys-nullable) instead). The example below is a build with the feature **off**, where no attribute is read and no such guard runs:

```rust
#[model_schema()]
pub struct Profile {
    pub name: String,
    #[model_schema_prop(ts_optional)]
    pub nickname: Option<String>,
    pub nick_handle: Option<String>,
}
```

Generated TypeScript:

```typescript
export type Profile = {
  name: string;
  nickname?: string;
  nick_handle: string | undefined;
};
```

`nick_handle` is the same field without the flag, so the flag is the whole of the difference between those two lines.

This is a TypeScript-only knob -- the Zod schema and JSON Schema are unchanged (the field is already optional in both, flagged or not). The flag is only valid on `Option<T>` fields; applying it to a non-`Option` field is a compile error. It composes with `as = Type`, which names the type the field already renders.

### Nullable Object Keys (`nullable`)

The bare `nullable` flag is for the `Option<T>` field whose empty state genuinely *is* `null` on the wire — the key is always written and `null` is its value, not the absence of one. Where the default coerces an incoming `null` away to `undefined` and leaves the key optional, `nullable` renders the field as `T | null` with the key **required** on all three surfaces:

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub name: String,
    #[model_schema_prop(nullable)]
    pub template_id: Option<String>,
}
```

Generated TypeScript:

```typescript
export type DocumentMetadata = {
  name: string;
  templateId: string | null;
};
```

Generated Zod: `templateId: z.union([z.string(), z.null()])` -- no `.prefault`, no coercing arm. Parsing `{ name: "invoice.pdf", templateId: null }` succeeds; parsing `{ name: "invoice.pdf" }` fails, because the flag declares the key required.

Generated JSON Schema: `"templateId": { "anyOf": [{ "type": "string" }, { "type": "null" }] }`, with `templateId` in `required`.

`nullable` is only valid on `Option<T>` fields; applying it to a non-`Option` field is a compile error. It is refused together with `ts_optional` -- the two disagree about the key (`nullable` keeps it, `ts_optional` drops it), and writing both would spell `field?: T | null`, a third state neither flag models. It composes with `preprocess`: the preprocess wrap goes around the whole nullable union, coercing the raw value before it is checked against `T | null`.

With the `serde` feature on, `nullable` is also refused together with any key-dropping serde attribute -- `skip_serializing_if`, `skip_serializing`, or `skip` -- because the flag declares the key always written and one of those would drop it instead, leaving serde write a payload the generated schema does not admit. The guard names the field and the flag. Deserialization needs no such guard: a `nullable` field reads both `{"templateId":null}` and `{"templateId":"invoice.pdf"}` the same way it always did, `{}` included, since nothing about `nullable` changes what serde accepts on the way in.

## Compiler-Validated Examples

You can provide compiler-validated example values for your types. Every generated Zod schema holds them in its `.meta()` under `examples`: a list with one item per example block, in the order written, and an empty list where the type writes none. Examples are written in doc comment code blocks and fully type-checked at compile time.

```rust
/// User profile
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

Generated Zod:

```typescript
export const User$Schema: ZodType<User> = z.strictObject({
  name: z.string(),
  email: z.string(),
  age: z.number().int(),
}).meta({
  examples: [{"age":25,"email":"john@example.com","name":"John Doe"}]
});
```

You can include arbitrary setup code before the example value:

```rust
/// User with tags
/// ```rust example
/// let user_id = "usr_123".to_string();
/// let tags = vec!["admin".to_string(), "active".to_string()];
/// User {
///     id: user_id,
///     tags,
///     age: 30,
/// }
/// ```
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub id: String,
    pub tags: Vec<String>,
    pub age: u32,
}
```

Enum examples work similarly:

```rust
/// Data type enumeration
/// ```rust example
/// DataType::Numeric
/// ```
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DataType {
    Alphanumeric,
    Numeric,
    Date,
}
```

A type can write several blocks, one value each, for example one per variant of an enum. Every block is built and type-checked on its own, so a name one block binds is not in scope in the next:

```rust
/// A shape
/// ```rust example
/// Shape::Circle { radius: 2 }
/// ```
/// ```rust example
/// Shape::Square { side: 3 }
/// ```
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "kind")]
pub enum Shape {
    Circle { radius: u32 },
    Square { side: u32 },
}
```

Generated Zod:

```typescript
const Shape$RawSchema = z.discriminatedUnion("kind", [z.strictObject({
  kind: z.literal("Circle"),
  radius: z.number().int(),
}), z.strictObject({
  kind: z.literal("Square"),
  side: z.number().int(),
})]);

export const Shape$Schema: ZodType<Shape> = Shape$RawSchema.meta({
  examples: [{"kind":"Circle","radius":2},{"kind":"Square","side":3}]
});
```

A type that writes no block publishes the empty list:

```rust
#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Plain {
    pub value: u32,
}
```

```typescript
const Plain$RawSchema = z.strictObject({
  value: z.number().int(),
});

export const Plain$Schema: ZodType<Plain> = Plain$RawSchema.meta({
  examples: []
});
```

The same values are read in Rust. `Shape::schema_examples()` answers every block's value, a `Vec<serde_json::Value>` in the order written, and `Shape::schema_example()` answers the first. A type that writes no block has neither function.

Key points:

- Examples are **optional** -- a type that writes none publishes `examples: []`, and has no `schema_examples()` and no `schema_example()`.
- Use the exact syntax: ` ```rust example` (note the space and `example` keyword).
- Every block is published, in the order written. An empty block is skipped.
- An example is written as `serde_json` holds it. An object's keys are in the order serde wrote them where the crate's `serde_json` has `preserve_order` on, which the `bson` library turns on, and sorted where it is off.
- Examples respect Serde attributes (field renaming, etc.).
- Being Rust source, an example block is dropped from the JSDoc comment above the generated `export type`, whatever the type is declared as. On a struct or an enum it reaches the Zod `examples` list. A type alias builds no example: its block reaches no generated surface, and the alias publishes `examples: []`.
- A brand and a plain enum hold `examples` inside the `.meta()` that holds their `description`. A generic type holds it on the schema its factory memoizes, so `$SchemaDefault` and every other call of the factory carry it.
- A generic struct or enum has each example built at one instantiation, with every type parameter filled at `String`. A lifetime elides there and needs no filling, but a **const parameter takes none** -- `String` names a type, a const is a value, and no value is the one every example would be written at. So a struct or an enum that declares a const parameter and writes an example is refused, naming the const; drop the example or the const parameter. Nothing is owed where no example is written, on a type alias, or with the `zod` feature off, `zod` being the only surface that reads one.
- The example code is executed at compile time and serialized to JSON.
- Wrong types produce compile errors, ensuring examples stay in sync with your types.

## MongoDB ObjectId Support

Enable the `mongodb` feature for first-class MongoDB ObjectId support with proper serialization and validation.

```rust
use tixschema::model_schema;
use serde::{Deserialize, Serialize};
use mongodb::bson::oid::ObjectId;

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Document {
    pub id: ObjectId,
    pub title: String,
    pub author_id: ObjectId,
    pub tags: Vec<ObjectId>,
    pub metadata: HashMap<String, ObjectId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<ObjectId>,
    pub related_docs: HashMap<String, Vec<ObjectId>>,
}
```

Generated TypeScript:

```typescript
export type Document = {
  id: ObjectId;
  title: string;
  author_id: ObjectId;
  tags: Array<ObjectId>;
  metadata: Partial<Record<string, ObjectId>>;
  parent_id: ObjectId | undefined;
  related_docs: Partial<Record<string, Array<ObjectId>>>;
};

export const Document$Schema = z.strictObject({
  id: z.object({ $oid: z.string().regex(/^[a-f0-9]{24}$/, { message: "Invalid ObjectId" }) }),
  title: z.string(),
  author_id: z.object({ $oid: z.string().regex(/^[a-f0-9]{24}$/, { message: "Invalid ObjectId" }) }),
  tags: z.array(z.object({ $oid: z.string().regex(/^[a-f0-9]{24}$/, { message: "Invalid ObjectId" }) })),
  metadata: z.record(z.string(), z.object({ $oid: z.string().regex(/^[a-f0-9]{24}$/, { message: "Invalid ObjectId" }) })),
  parent_id: z.union([z.null().transform(() => undefined), z.object({ $oid: z.string().regex(/^[a-f0-9]{24}$/, { message: "Invalid ObjectId" }) }), z.undefined()]).prefault(undefined),
  related_docs: z.record(z.string(), z.array(z.object({ $oid: z.string().regex(/^[a-f0-9]{24}$/, { message: "Invalid ObjectId" }) }))),
}).meta({
  examples: []
});
```

ObjectIds serialize to MongoDB's standard JSON format:

```json
{
  "id": { "$oid": "507f1f77bcf86cd799439011" },
  "title": "My Document",
  "author_id": { "$oid": "507f1f77bcf86cd799439012" },
  "tags": [
    { "$oid": "507f1f77bcf86cd799439013" },
    { "$oid": "507f1f77bcf86cd799439014" }
  ]
}
```

Key details:

- Uses `{ "$oid": "hex_string" }` format matching MongoDB's native serialization.
- Validates 24-character hexadecimal ObjectId strings via regex.
- Supports ObjectIds in arrays, HashMaps, optional fields, and deeply nested structures.
- The MongoDB crate is a dev-dependency only -- zero production overhead.

Every non-Rust target treats `ObjectId` as a bare reference rather than publishing it -- `ts_definition()` writes `ObjectId` unresolved (see "Missing TypeScript Types" below), and `dart_definition()`, `swift_definition()` and `kotlin_definition()` do the same. The consuming project supplies a one-line type in its own language matching the `{ "$oid": "hex_string" }` wire form:

```dart
class ObjectId {
  final String oid;
  ObjectId(this.oid);
  factory ObjectId.fromJson(Map<String, dynamic> json) => ObjectId(json['\$oid'] as String);
  Map<String, dynamic> toJson() => {'\$oid': oid};
}
```

```swift
struct ObjectId: Codable {
  let oid: String
  enum CodingKeys: String, CodingKey { case oid = "$oid" }
}
```

```kotlin
@Serializable
data class ObjectId(@SerialName("\$oid") val oid: String)
```

## Chrono Date/Time Types

Enable the `chrono` feature for chrono date/time type support. All chrono types map to `string` in TypeScript, with appropriate Zod validation for the specific date/time format.

```toml
tixschema = { features = ["chrono"] }
```

A consumer enabling `chrono` on tixschema must also enable `serde` on its own `chrono` dependency — `as_number` (below) injects a `chrono::serde::ts_milliseconds` hook that only exists there.

Supported types and mappings:

| Rust Type | TypeScript | Zod Schema | JSON Schema Format |
|-----------|------------|------------|--------------------|
| `NaiveDate` | `string` | `z.iso.date()` | `"date"` |
| `NaiveTime` | `string` | `z.preprocess(<millis arrow>, z.iso.time())` | `"time"` |
| `NaiveDateTime` | `string` | `z.iso.datetime({ local: true })` | `"date-time"` |
| `DateTime<Tz>` (default) | `Date` | `z.union([z.date(), z.string(), z.number()]).pipe(z.coerce.date())` | `"date-time"` |
| `DateTime<Tz>` + `#[model_schema_prop(as_number)]` | `number` | `z.preprocess(<epoch arrow>, z.number())` | `"date-time"` |

`DateTime<Tz>` renders as a native TypeScript `Date` by default, which is what MongoDB needs to expire a BSON `Date` via TTL. A `Date`, an ISO string and epoch milliseconds are coerced to one; anything else is refused before the coercion, since `z.coerce.date()` alone reads `null` as `1970-01-01T00:00:00.000Z` and `true` as one millisecond later. An `Option<DateTime<Tz>>` still reads `null` as `undefined`, through the `z.null()` arm every `Option` has. A `nullable` one reads `null` as `null`, a `null` among the items of a `Vec<Option<DateTime<Tz>>>` stays `null`, and in an untagged enum an input that is no date reaches the member written after the date. An object that is no `Date` is refused even where `new Date` would convert it, as it would a dayjs or a moment value: hand the schema the `Date` itself. The bare `as_number` flag opts a single `DateTime<Tz>` field into an epoch-milliseconds `number` instead, validated by a self-contained inline coercer (no imported helper). `as_number` is only valid on a `DateTime<Tz>` field — using it elsewhere is a compile error.

On the Rust side, `as_number` also injects `#[serde(with = "chrono::serde::ts_milliseconds")]` (or `ts_milliseconds_option` for an `Option<DateTime<Tz>>`), so the field serializes to, and deserializes from, the same epoch-milliseconds number every surface describes — no hand-written serde attribute needed. Where the field already carries its own `with` or `deserialize_with`, the injection is held back and the author's hook runs instead.

`NaiveTime` stays a TypeScript `string`, but its Zod schema also accepts millis-since-start-of-day, converting them to an `HH:MM:SS` string before validation.

Example:

```rust
use tixschema::{model_schema, model_schema_prop};
use serde::{Deserialize, Serialize};
use chrono::{NaiveDate, NaiveTime, NaiveDateTime, DateTime, Utc};

#[model_schema()]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Event {
    pub name: String,
    pub date: NaiveDate,
    pub time: NaiveTime,
    pub local_datetime: NaiveDateTime,
    pub created_at: DateTime<Utc>,
    #[model_schema_prop(as_number)]
    pub epoch_ms: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}
```

Generated TypeScript:

```typescript
export type Event = {
  name: string;
  date: string;
  time: string;
  local_datetime: string;
  created_at: Date;
  epoch_ms: number;
  updated_at: Date | undefined;
};
```

Generated Zod:

```typescript
export const Event$Schema: ZodType<Event> = z.strictObject({
  name: z.string(),
  date: z.iso.date(),
  time: z.preprocess((arg) => { if (typeof arg === "number") { const s = Math.floor(arg / 1000); const hh = String(Math.floor(s / 3600)).padStart(2, "0"); const mm = String(Math.floor((s % 3600) / 60)).padStart(2, "0"); const ss = String(s % 60).padStart(2, "0"); return `${hh}:${mm}:${ss}`; } return arg; }, z.iso.time()),
  local_datetime: z.iso.datetime({ local: true }),
  created_at: z.union([z.date(), z.string(), z.number()]).pipe(z.coerce.date()),
  epoch_ms: z.preprocess((arg) => { if (arg instanceof Date) return arg.getTime(); if (typeof arg === "string") return Date.parse(arg); return arg; }, z.number()),
  updated_at: z.union([z.null().transform(() => undefined), z.union([z.date(), z.string(), z.number()]).pipe(z.coerce.date()), z.undefined()]).prefault(undefined),
}).meta({
  examples: []
});
```

Chrono types also work in collections (`Vec<NaiveDate>` generates `z.array(z.iso.date())`) and in enums as tuple variant elements (`as_number` is honored on a tuple-variant `DateTime<Tz>` payload).

## Recovering Decode (`decode_with`)

A type that opts in gets methods that read a value the caller already holds: `from_value_with` and `from_value_piped` read a `serde_json::Value`, and, in a build with tixschema's `bson` feature on, `from_bson_with` and `from_bson_piped` read a `bson::Document`. Serde reads the value with the type's read alone, ahead of the `validate()` [a plain read of the type ends on](#a-read-refuses-what-validate-refuses), so a broken bound and a broken check are issues to list and never a refusal that hides them. When serde reads it, the value is held in the form the type writes and it breaks no bound a field declares, the decoded value comes back and nothing else happens. When serde refuses it, when it carries a key the type does not declare, when a value reads but is held in another form than its field writes, or when a value reads but breaks a bound its field declares, the raw value and every issue found in it are handed over, once. A `with` method hands them to a callback supplied with that call, which rejects the record or fixes the raw value. A `piped` method hands them to [resolvers](#resolvers) supplied with that call, small functions that each answer one issue at a time. Either way serde then gets one more read, the same way.

The entry `"decode_with"` adds these methods. The types the callback and the resolvers work with are the crate's [support library](#the-support-libraries), which the crate writes once, at its root, and every flagged type's module re-exports. A generated dispatcher reads a request with the read alone and then calls `validate()`, as [Service Validation](#service-validation) says, and in a build without the `mongodb` feature nothing tixschema generates calls them. With `mongodb` on, a type that lists `"mongodb"` also reads and writes its own rows in a collection, and those reads go through `from_bson_piped`: see [MongoDB Operations and Typed Filters](#mongodb-operations-and-typed-filters).

### Opting in

Write the `enable` list among the type's `#[model_schema(...)]` arguments. It takes three entries, `"decode_with"`, `"mongodb"` and `"archie"`:

- `#[model_schema(enable = ["decode_with"])]` gives the type the recovering decode of a JSON value, and, in a build with tixschema's `bson` feature on, of a BSON document. A type that is read and never stored lists this, whether it is read from a request or from a BSON document.
- `#[model_schema(enable = ["decode_with", "mongodb"])]` gives it, under tixschema's `mongodb` feature, the [MongoDB layer](#mongodb-operations-and-typed-filters) as well. A type that is stored lists both. In a build without the `mongodb` feature the entry adds nothing and is accepted, so the declaration builds whatever features tixschema is built with.
- `#[model_schema(enable = ["archie"])]` gives it, under tixschema's `archie` feature, the [conversions to and from archie's frontmatter `Value`](#archie-frontmatter-conversions-archie). It needs no other entry, and is written beside them in any order.

A type without the list gains nothing: its expansion is what it was, with no method and no type added. This section calls a type that lists `"decode_with"` a flagged type, and the entry the flag. The crate that declares a flagged type writes [the initializers](#the-support-libraries) once, at its root.

```rust
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Stored: the BSON decode and the MongoDB layer.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Customer {
    pub name: String,
    pub open_invoices: u32,
}

/// Read and never stored: the recovering decode, from BSON as well under `bson`, and no MongoDB code.
#[model_schema(enable = ["decode_with"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceLookup {
    pub number: String,
}
```

What each entry generates on a type, by the features tixschema is built with:

| Entry | tixschema built with | Generates, per type |
|-------|----------------------|---------------------|
| `"decode_with"` | `serde` | `from_value_with`, `from_value_piped` and their walkers, the type's own items in its `{type}_schema` module, and the re-export of the `decode_with` library there |
| `"decode_with"` | `bson`, which `mongodb` turns on | The row above, and `from_bson_with`, `from_bson_piped` and the BSON walkers, and the BSON pieces of the `decode_with` library among the names the module re-exports. No operation, no `MONGO_FIELDS` and no typed path |
| `"mongodb"` | `mongodb` | The operations `find_one`, `find_one_with`, `find`, `find_with`, `count`, `insert_one`, `update_one`, `update_many`, `delete_one` and `delete_many`, `MONGO_FIELDS`, `MongoFields`, the typed paths, and the re-export of the MongoDB library: the query types and what the operations answer |

The BSON decode follows the `bson` feature alone: every flagged type has it in a build with `bson` on, whether it lists `"mongodb"` or not, so a type read from a BSON document and never stored lists `"decode_with"` alone. A type without `"mongodb"` gets none of the last row, under any feature, and a type that lists `"mongodb"` gets nothing of it in a build without the `mongodb` feature.

On a type `Record` that lists `"decode_with"`, in a build with `bson` on, the list generates these methods:

```rust
impl Record {
    pub fn from_value_with<F>(
        mut value: serde_json::Value,
        decide: F,
    ) -> core::result::Result<Self, record_schema::Unrecovered<serde_json::Value>>
    where
        F: FnOnce(&mut serde_json::Value, &[record_schema::Issue<serde_json::Value>]) -> record_schema::Verdict,
    { /* ... */ }

    pub fn from_value_piped(
        mut value: serde_json::Value,
        resolvers: &[record_schema::Resolver<'_, serde_json::Value, serde_json::Value>],
    ) -> core::result::Result<Self, record_schema::Unrecovered<serde_json::Value>>
    { /* ... */ }

    pub fn from_bson_with<F>(
        document: bson::Document,
        decide: F,
    ) -> core::result::Result<Self, record_schema::Unrecovered<bson::Bson>>
    where
        F: FnOnce(&mut bson::Document, &[record_schema::Issue<bson::Bson>]) -> record_schema::Verdict,
    { /* ... */ }

    pub fn from_bson_piped(
        document: bson::Document,
        resolvers: &[record_schema::Resolver<'_, bson::Document, bson::Bson>],
    ) -> core::result::Result<Self, record_schema::Unrecovered<bson::Bson>>
    { /* ... */ }
}
```

- The callback is the method's own type parameter, named `F` unless the type's declaration already writes that name, with its bound in a `where` clause. It is an `FnOnce`, so the compiler holds it to one run per read.
- The resolvers of a `piped` method are a slice, in the order they are to run: `&[&first, &second]`. An empty slice, `&[]`, is a read any issue refuses.
- `record_schema` is the module tixschema writes for `Record`. The callback's types and the resolvers' are the crate's support library, which `record_schema` re-exports name by name: `record_schema::Verdict` and `record_schema::Issue` are the crate's one `Verdict` and one `Issue` ([One library per crate](#one-library-per-crate)).
- `from_value_with` and `from_value_piped` are generated in a build with tixschema's `serde` feature on, and `from_bson_with` and `from_bson_piped` in a build with its `bson` feature on, which `mongodb` turns on, whether the type lists `"mongodb"` or not.

The list is read in every build. Each of these is refused, spanned on what is written, with a message that says what to write instead. The error opens with the type it is written on, ``model_schema: type `Record`:``, and goes on with the message:

| Written | The message |
|---------|-------------|
| `decode_with`, `decode_with = true` or `decode_with = false`, the flag as it was written before the list | ``tixschema: `decode_with` is now an entry of the `enable` list: write `#[model_schema(enable = ["decode_with"])]` `` |
| `enable = ["mongodb"]` | ``tixschema: `"mongodb"` requires `"decode_with"`: write `enable = ["decode_with", "mongodb"]` `` |
| `enable = ["decode_wth"]`, an entry the list does not read | ``tixschema: unknown entry `"decode_wth"`; the entries are `"archie"`, `"decode_with"` and `"mongodb"` `` |
| `enable = ["decode_with", "decode_with"]` | ``tixschema: `"decode_with"` is written twice`` |
| `enable = []` | ``tixschema: the list is empty; remove `enable` `` |
| `enable = "decode_with"`, `enable = [decode_with]`, or any other form but a list of string literals | `` `model_schema` argument `enable` takes a list of string literals, written `enable = ["decode_with"]` or `enable = ["decode_with", "mongodb"]` `` |

`"mongodb"` without `"decode_with"` is refused because the MongoDB layer reads every row through the recovering decode, which the first entry generates.

### What your crate lists

The generated code names `serde_json`, and, in a build with tixschema's `bson` feature on, the code of every flagged type and the library `decode_with_foundation!()` writes name the `bson` library, for `from_bson_with`. So the crate that declares a flagged type lists `serde_json` in its own manifest beside `serde`, and lists `bson` too wherever tixschema is built with `bson`:

```toml
serde_json = "1"
# for from_bson_with, at the major version your MongoDB driver uses
bson = "2.15"
```

List the `bson` library at the major version your MongoDB driver uses, so the `Document` the driver hands you is the `bson::Document` the method takes. The generated code calls only what both major versions of the library have, and this repository's own tests build it against each, beside a MongoDB driver built for that version: `bson = "2.15"` in the crate's own `tests/`, and `bson = { version = "3.1", features = ["serde"] }`, with version 3's `serde` feature turned on, in `bson3/`, a package of its own.

With tixschema's `mongodb` feature on, the generated code names the `mongodb` driver too, in every crate where a type lists `"mongodb"`, for the operations the type gains. [The driver your crate lists](#the-driver-your-crate-lists) has the lines for each major version of `bson`.

A crate whose types list `"decode_with"` alone, and that writes `decode_with_foundation!()` alone, names no `mongodb` path and lists no driver, whatever features tixschema is built with. It names `bson` wherever tixschema's `bson` feature is on, and so lists it there. That includes a workspace where another crate turns tixschema's `mongodb` feature on, which Cargo then turns on, `bson` with it, for every crate of the build.

### The support libraries

What a callback, a resolver and a walker work with is the same for every flagged type, so a crate holds it once. Two initializers, each called once at the crate root, write it:

- `tixschema::decode_with_foundation!();` writes the module `__tixschema_decode_with`: `Issue`, `Expected`, `Path` and `Segment`, `Verdict`, `Resolution`, `Resolver`, `Unrecovered`, `unsettled`, and what one type's walker hands another's. Under `bson` it holds as well what reads a BSON value and the helpers `set_in_document` and `remove_from_document` of `Path`, and it names no `bson` path in a build without `bson`. It writes nothing in a build without tixschema's `serde` feature.
- `tixschema::mongodb_foundation!();` writes the module `__tixschema_mongodb` under tixschema's `mongodb` feature: the query types (`Filter`, `Update`, `ArmFilter`, `ArmUpdate`, the path kinds) and what the operations answer (`Read`, `OperationError`, `WriteError`). It reads the names of the first library, so it is written beside `decode_with_foundation!()`. In a build without `mongodb` it writes nothing, so the call stays in a crate whatever features the crate builds with.

A crate whose types list `"decode_with"` writes the first, and a crate where one lists `"mongodb"` writes both. Each is written once per crate, at the crate root (`src/lib.rs` or `src/main.rs`), on its own line, anywhere outside a function, after the `mod` lines as well as before them:

```rust
// src/lib.rs
tixschema::decode_with_foundation!();
tixschema::mongodb_foundation!();

pub mod billing;
```

- **Neither takes an argument.** Anything written between the parentheses is refused: ``tixschema: `decode_with_foundation!` takes no arguments: write `tixschema::decode_with_foundation!();` at the crate root``, and the same for `mongodb_foundation!`.
- **Each is written once.** Written twice, an initializer declares its module and its marker twice, which rustc refuses (`E0428`, `` the name `__tixschema_decode_with` is defined multiple times ``).
- **Each crate has its own.** A library, a binary, each test binary, each example and each doctest is a crate, and writes the initializers at its own root. A doctest has a root of its own only where it writes its own `fn main`: one that does not is wrapped in a `fn main`, and an initializer inside a function writes no module at the crate root.
- **Every `{type}_schema` module re-exports its crate's libraries, name by name**: the `decode_with` library always, its BSON pieces included under `bson`, and the MongoDB library where the type lists `"mongodb"`, under `mongodb`. So every path written against a type's module resolves, `record_schema::Verdict::Reject`, `invoice_schema::Issue`, `invoice_schema::Filter<Invoice>`, from the crate and from another one. A name written out shadows the one the module reads from your scope: in a module of yours that writes `use std::path::Path;` or declares a struct `Filter` of its own beside a flagged type, `Path` and `Filter` stay yours, and `record_schema::Path` and `invoice_schema::Filter` are the library's.

Nothing tixschema emits into your crate carries `#[allow]`, `#[expect]` or `#[doc(hidden)]`, so your lint levels reach all of it, with two exceptions, both on the libraries:

- **Each re-export in a `{type}_schema` module carries `#[expect(clippy::pub_use, reason = "...")]`.** The re-export is a `pub use`, which clippy's `pub_use`, a lint of its restriction group, reports wherever that group is denied. It is what keeps every `{type}_schema::Name` path reachable from your crate and from another, so tixschema writes the expectation, with its reason, rather than leaving the lint to fire at every flagged type. Neither `pub_use` nor an unmet expectation is reported, in `cargo build`, in clippy at the lint's default level, or in clippy with the restriction group denied.
- **Each library module, `__tixschema_decode_with` and `__tixschema_mongodb`, carries `#[doc(hidden)]`**, so the library is not published in your crate's documentation as a module of its own. Its items are documented where you name them, under each `{type}_schema` module that re-exports them.

### A crate without its initializer

Each flagged type checks that its crate wrote the libraries it re-exports. In a crate that did not, the build fails at each such type with an error that names the line to add and where to add it. With no `decode_with_foundation!()`, beside rustc's own error for the re-export it cannot resolve (`E0432`, `` could not find `__tixschema_decode_with` in the crate root ``), each flagged type reports an `E0277` whose message, label and note are:

```text
message: tixschema: this crate has `#[model_schema(enable = ["decode_with"])]` types but no `tixschema::decode_with_foundation!();`
label:   this type needs the decode_with support library, which `tixschema::decode_with_foundation!();` writes once per crate
note:    Fix: add the line `tixschema::decode_with_foundation!();` at the crate root (src/lib.rs or src/main.rs), on its own line, anywhere outside a function. Add it once per crate; nothing else is needed.
```

With `decode_with_foundation!()` and no `mongodb_foundation!()`, in a build with `mongodb` on, each type that lists `"mongodb"` reports:

```text
message: tixschema: this crate has `#[model_schema(enable = ["decode_with", "mongodb"])]` types but no `tixschema::mongodb_foundation!();`
label:   this type needs the MongoDB support library, which `tixschema::mongodb_foundation!();` writes once per crate
note:    Fix: add the line `tixschema::mongodb_foundation!();` at the crate root (src/lib.rs or src/main.rs), on its own line, anywhere outside a function. Add it once per crate; nothing else is needed.
```

The fix is the line the note gives. The check is the same wherever the initializer is written at the crate root, so a crate whose `mod` lines come first and whose initializers come last builds.

### One library per crate

A crate holds its libraries once, and every flagged type of it shares them:

- **Every path written against a type's module compiles**: `invoice_schema::Verdict::Reject`, `invoice_schema::Issue`, `invoice_schema::OperationError`.
- **Within one crate, `invoice_schema::Issue` and `customer_schema::Issue` are one type**, and so is every other name of the libraries: `Verdict`, `Resolution`, `Resolver` and `OperationError` among them. A type of another crate has its own crate's.
- **One resolver serves every flagged type of its crate.** A resolver written against `invoice_schema::Issue` is handed to the `piped` read of any flagged type the crate declares, and to `find_one_with` and `find_with` of any that lists `"mongodb"`.
- **Two impls written for two modules' types are one impl written twice**, which rustc refuses. Keep one of them:

```rust
// two impls of one crate, each written for one module's `OperationError`
impl From<invoice_schema::OperationError> for StoreError {
    fn from(failed: invoice_schema::OperationError) -> Self { Self::Stored(failed.to_string()) }
}
impl From<customer_schema::OperationError> for StoreError {
    fn from(failed: customer_schema::OperationError) -> Self { Self::Stored(failed.to_string()) }
}
// error[E0119]: conflicting implementations of trait `From<OperationError>` for type `StoreError`
```

- **A crate with no stored type names no `mongodb` path, and names `bson` only where tixschema's `bson` feature is on.** Its types list `"decode_with"` alone and it writes `decode_with_foundation!()` alone. Built without `bson`, its manifest lists neither library:

```rust
// src/lib.rs of a crate whose Cargo.toml lists tixschema, serde, serde_json, bson where tixschema
// has `bson` on, and no mongodb
tixschema::decode_with_foundation!();

#[tixschema::model_schema(enable = ["decode_with"])]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct InvoiceLookup {
    pub number: String,
}
```

- **A doctest that declares a flagged type writes its own `fn main`**, with the initializers at its crate root beside it.

### One call, one chance

A read is one call. What it finds is handed over once, to a callback or to resolvers, and the value is then read one more time.

1. Serde reads the value, with the type's read alone, and the walker walks it. When serde reads it and the walk finds no issue, the decoded value is returned, and neither the callback nor any resolver is called.
2. Otherwise every issue the walk found is handed over, with the raw value, mutable. A record serde reads still gets there when it carries a key its type does not declare, a value held in another form than its field writes, or a value that breaks a bound its field declares.
3. `from_value_with` and `from_bson_with` call their callback once, with the whole list. `Verdict::Reject` fails the read with `Unrecovered`, carrying that list. `Verdict::Fixed` goes on to the second read.
4. `from_value_piped` and `from_bson_piped` run their resolvers in the order given, each over one issue at a time. A resolver answers an issue `Settled`, it repaired the raw value; `Rejected`, the issue is known and is not to be repaired; or `NotTouched`, the issue is not its own. An issue answered `Settled` or `Rejected` is handed to no later resolver, and one left `NotTouched` goes on to the next. Once the last resolver has run, any issue that is not `Settled` fails the read with `Unrecovered`, carrying only those issues, in the order the walk found them. With every issue settled, the second read follows.
5. The second read: serde reads the value as it was left, the same way, and the walker walks it again. A read with no issue returns the value. Anything else fails the read with `Unrecovered`, carrying the second walk's list. Neither the callback nor a resolver is called again: an issue a repair uncovers fails the read.

The result is the decoded value or the failure. Nothing reports whether the callback or a resolver ran, or what it changed: a caller that wants to know records it inside its own callback or resolver.

### The issues

The callback's types and the resolvers' are named through the flagged type's own module, `record_schema` for `Record`, which re-exports them from the crate's [support library](#the-support-libraries). An `Issue<V>` is generic over the raw value: `serde_json::Value` from `from_value_with` and `from_value_piped`, `bson::Bson` from `from_bson_with` and `from_bson_piped`. One callback receives every issue in the whole record as the outer type's own `Issue`, an issue inside a nested type included, at its full path, and so does each resolver, one issue at a time.

| `Issue` member | What it says | Example |
|----------------|--------------|---------|
| `Invalid { path, expected, found, reason }` | The value is there, and either serde refuses it for its field, or serde reads it and it breaks a `#[model_schema_prop]` bound its field declares, or a [check](#checks-check) refuses it. `reason` is serde's own message for the first, in the `bson` library's wording where the value is BSON, the bound's own sentence for the second, and the check's message after its path for the third | A version holding `"number": "2"` where `number` is an `i32`: `Invalid` at `versions[1].number`, expected `I32`. A `humanName` holding `""` under `minLength = 1`: `Invalid` at `humanName`, reason `too short: minimum length is 1, got 0` |
| `Missing { path, expected }` | The type requires the key, and it is not there | A record with no `name` where `name` is a `String`: `Missing` at `name`, expected `String` |
| `Unknown { path, found }` | The key is there, and the type declares no field by that name | A record holding `"legacyField": true`: `Unknown` at `legacyField` |
| `Mistyped { path, expected, found }` | serde reads the value, but it is held in another form than its field writes: another JSON shape, or another BSON type | An id held as the text `"6a7cc592ca0574e6efdfe217"` where an `ObjectId` writes `{ "$oid": "..." }`: `Mistyped` at `recordId`, expected `ObjectId` |
| `NoVariant { path, found, variants }` | An untagged enum none of whose variants reads the value. `variants` holds each variant's own list of issues, in the order declared | `{ "country": "one", "digits": "555" }` read as an untagged enum of `Email`, `Phone` and `Versioned`: one `NoVariant`, holding a list for each of the three |
| `Undescribed { reason }` | serde refuses the value and the walk found nothing that says why. `reason` is serde's own message | `{ "Circle": 5 }` read as an enum whose `Circle` variant holds named fields |

`Undescribed` is added when everything the walk listed is an `Unknown` or a `Mistyped`, or it listed nothing, and serde still refuses the value. A type whose `Deserialize` is written by hand earns it, and so does an undeclared key under `#[serde(deny_unknown_fields)]`, listed beside its `Unknown`.

`Expected` describes a field's type as tixschema classifies it:

- one member per plain category: `Boolean`, `Char`, `String`, `I8` to `I64`, `Isize`, `U8` to `U64`, `Usize`, `F32`, `F64`, `DateTime`, `NaiveDate`, `NaiveDateTime`, `NaiveTime`, `ObjectId`, `Unknown`, and the three literals `BooleanLiteral(bool)`, `NumberLiteral(f64)` and `StringLiteral(&'static str)`;
- `Model(&'static str)`, a `#[model_schema]` type by its Rust name as the field's type writes it, and `TypeParam(&'static str)`, one of the type's own type parameters by its name;
- `Array(Box<Expected>)`, `Optional(Box<Expected>)` and `Map(Box<Expected>)`, each carrying what it holds, a map the type of its values, and `Tuple(Vec<Expected>)`, carrying its positions in order: `Array(Model("Version"))`, `Optional(ObjectId)`, `Tuple([String, U32])`;
- `Range(&'static str, Box<Expected>)`, a std range by the name it is declared under in `core::ops` and the type of its bounds: `Range("RangeInclusive", Box::new(U32))`;
- `Variants(&'static [&'static str])`, the tag values a tagged enum accepts, for a tag naming none of them, and the two arms of a `Result`, `Variants(["Ok", "Err"])`, for an object that names neither or both.

`Path` is where an issue sits: `Path(pub Vec<Segment>)`, a `Segment` being `Key(String)` or `Index(usize)`. It displays as `versions[1].number`, and the empty path is the value itself. It carries the helpers a callback fixes the raw value through, each answering whether it did so:

- `set_in_value(&self, root: &mut serde_json::Value, value: serde_json::Value) -> bool` puts `value` at the path, inserting the last key when it is absent. The empty path replaces the whole value. A parent that is not there, or an index past the end of its list, answers `false`.
- `remove_from_value(&self, root: &mut serde_json::Value) -> bool` removes the key or the item at the path.
- `set_in_document(&self, root: &mut bson::Document, value: bson::Bson) -> bool` and `remove_from_document(&self, root: &mut bson::Document) -> bool` do the same inside a BSON document, and are generated with the `bson` feature. A path into a document starts at a key, so the empty path sets and removes nothing there.

`Verdict` is the callback's answer, `Reject` or `Fixed`. `Resolution` is a resolver's answer for one issue, `Settled`, `Rejected` or `NotTouched`. `Resolver<'r, D, V>` is the type a `piped` method takes each resolver as, `&'r (dyn Fn(&mut D, &Issue<V>) -> Resolution + Sync)`: `D` is the raw value, `serde_json::Value` or `bson::Document`, and `V` what an issue holds. `unsettled(raw, issues, resolvers)` is the pipe a `piped` method runs, a public function of the module: it runs the resolvers in order and answers the issues none settled.

`Unrecovered<V>` is the failed read, with its issues in `pub issues: Vec<Issue<V>>`. It implements `Debug`, `Display`, one line per issue with its path first, and `std::error::Error`, so `?` carries a failed read into the caller's own error type.

Each of these types is `#[non_exhaustive]`: inside the crate that declares the flagged type a `match` may list every member, and a callback written in another crate needs a wildcard arm.

### What each shape reports

The walker reads each type in the form serde writes it. A type held in another form than the one its walker walks, a struct as something other than an object or a tuple struct as something other than an array, is read whole with the type's own reader, and what is listed for it is serde's verdict: `Invalid` where serde refuses it, and `Mistyped` where serde reads it in a form the type does not write. A struct held as text is `Invalid`; one held as an array of its fields in order, which serde reads, is `Mistyped`.

**A struct with named fields** is an object, each field looked up under the name serde reads it under, after every `rename` and `rename_all`. A renaming written as a list counts by its `deserialize` side: every build that describes the type, a mobile client included, refuses a list whose two sides name two keys, and a build with no schema surface and no client takes it.

- A key that is there is read with the function serde's derive calls for the field: the field type's own `Deserialize`, or the field's `deserialize_with` function or `with` module. A refusal is `Invalid`.
- A key that is absent is `Missing`, unless serde reads its absence: an `Option` field, a `#[serde(default)]` field or container, or a `#[serde(skip_deserializing)]` field.
- A key stored under one of a field's `#[serde(alias)]` names counts as that field, and an issue with it is listed at the key it is stored under.
- A key the type does not declare is `Unknown`. A key the type writes and never reads back, under `skip_deserializing`, is one it declares, under the name serde writes it under.
- A field whose type is another flagged type is walked by that type's walker at the extended path, through a list, a map, an `Option` or a `Box`: `versions[1].number`, `byName.first.number`, `latest.draft`. An optional one is walked when its key is there and does not hold `null`.
- A value that reads is written back the way its field writes it, through the field's `serialize_with` function or `with` module when it has one, and compared with what is held. A mismatch is `Mistyped`.
- A `#[model_schema_prop]` bound on a field is no part of the read the recovering decode makes, the type's read alone, so serde reads a value that breaks it. The walker runs the field's validator on what serde read, and lists one `Invalid` at the value, with the bound's sentence as its `reason`: `humanName` holding `""` under `minLength = 1` lists `Invalid` at `humanName`, reason `too short: minimum length is 1, got 0`. A value that breaks two bounds lists one issue at its path, its reasons joined by `; `, a list that breaks its count is one issue at the list (`tags`, whatever its items are), and a field of a tagged enum's variant is listed the same way. So is a slot of a tagged tuple variant, at the slot's own path: `Pair[1]` for the second slot of an externally tagged `Pair`, and the key the variant's value sits under for a lone slot. And so is a slot of a tuple struct: `[1]` for the second slot of a `struct Pair(String, u32)` read as the value itself, `pair[1]` where a field `pair` holds it, and the path the struct sits at for the one slot of a single-slot struct. A callback or a resolver repairs the value at the issue's path. A validator is published only where a schema surface is on (`typescript`, `zod` or `jsonschema`), so a build with none lists nothing.
- A [check](#checks-check) the type or one of its fields names takes the typed value, which the walker never holds, so the checks run once serde has read the value: after the first read where serde accepts it, and after the read that follows a `Fixed` verdict or the resolvers. Each message is an `Invalid` of its own, expected `Unknown`. With `Parcel` of [the example](#checks-check), `{ "label": "A", "itemCodeList": ["x"], "fragileItemCodes": ["x", "y"] }` lists `Invalid` at `fragileItemCodes[1]`, reason `y is outside of itemCodeList`, and the same parcel held by `Manifest` lists it at `parcel.fragileItemCodes[1]`. The path is what the message leads with, read against the raw value: a segment that is a number where the raw value holds a list is an index, and every other segment is a key. A message with no lead is at the value itself, and `found` is `null` where the path leads nowhere. A member's check is listed where serde writes the member under each tagging of an enum, and the checks of a flagged type held in a field, an `Option`, a list, a map or a `Box` are listed under the key, and the index or the map key, it is held at. A payload serde refuses runs no check, no value having been built: a callback that fixes it meets the checks on the second read, and a `Fixed` verdict that leaves a check broken fails the read with `Unrecovered`, as it does for a bound. A check is listed whether or not a schema surface is on, where a bound is listed only with one. Nothing is listed for a member serde never reads back, nor for what a field read through a hook of your own holds. A path is told apart by its dots, so a map key that holds one is read as two segments.

Read as a struct holding `name: String` and `versions: Vec<Version>`, `{ "name": "Loan", "versions": [{ "number": 1 }, { "number": "2" }], "legacyField": true }` lists `Invalid` at `versions[1].number`, expected `I32`, and `Unknown` at `legacyField`.

**A plain value inside a list or a map** is read on its own, at its own path, with its own expected type. Read as a struct holding `tags: Vec<ObjectId>`, `owners: HashMap<String, ObjectId>` and `counts: Vec<i32>`, an id held as text is `Mistyped` at `tags[1]` and at `owners.alice`, each expecting `ObjectId`, and a number held as text is `Invalid` at `counts[1]`, expecting `I32`. A field holding something other than the list or the map it is written as is one `Invalid` at the field. An optional plain value is read whole at its field, expecting `Optional(ObjectId)`.

**A tuple struct and a tuple field** are walked by position, each position a `Segment::Index`. Read as `struct Pair(String, u32)`, `["a", "x"]` lists `Invalid` at `[1]`, expected `U32`, and `["a"]` lists `Missing` at `[1]`. A position the type does not declare is `Unknown`: serde refuses a JSON array longer than the tuple, so `["a", 1, true]` lists `Unknown` at `[2]` beside an `Undescribed`, and serde reads past the position in a BSON list, where the `Unknown` is the only issue. A field `spot: (String, u32)` holding `[5, 1]` lists `Invalid` at `spot[0]`, expected `String`.

**A brand and a single-slot tuple struct** are read as the one value they hold, at the path they sit at, and the issue names what they hold. With `id` a brand over `String`, a struct holding `"id": 5` lists `Invalid` at `id`, expected `String`. The read a brand is listed from holds the value to the brand's own bound, wherever a schema surface writes one: a brand declared `minLength = 3` lists `"ab"` as `Invalid`, and a [check](#checks-check) the brand names is listed beside it, from the value read. Over another flagged type it is walked as that type, whatever shape that type has: with `pinned` a brand over `Version`, `"pinned": { "number": "3" }` lists `Invalid` at `pinned.number`, and with `owner` a brand over a brand over `String`, `"owner": 5` lists `Invalid` at `owner`, expected `String`.

**A std `Result`** is walked as the externally tagged enum serde writes it as: an object whose one key, `Ok` or `Err`, names the arm, and what the arm holds is walked under that key. With `frontmatter: Option<Result<Headline, Garbled>>`, `"frontmatter": { "Ok": { "title": "" } }` lists `Invalid` at `frontmatter.Ok.title`, where `title` is under `minLength = 1`, and `"outcome": { "Ok": "seven" }` lists `Invalid` at `outcome.Ok`, expected `U32`. An object of both keys, of neither or of another key, and a value that is no object, is one `Invalid` at the field, expected `Variants(["Ok", "Err"])`. A `Result` an arm of which holds a value of one of the type's own parameters is read whole.

**A unit struct** is the `{}` tixschema makes it write, in which no key is its own: read as `struct Ping;`, `{ "x": 1 }` lists `Unknown` at `x`.

**A `#[serde(transparent)]` struct**, with a named field or with slots, is walked as the value of the one field serde reads it as, and no key is looked up under that field's name. Read as `struct Slug { text: String }` under `transparent`, `5` lists `Invalid` at the value itself, expected `String`. A slot serde does not read is no part of the value: with `tags` a `struct Tags(Vec<String>, #[serde(skip)] u8)` under `transparent`, `"tags": ["a", 7]` lists `Invalid` at `tags[1]`, expected `String`.

**A plain enum** is one value, read with its own reader. Read as `enum Status { Draft, Published }`, `"Archived"` lists `Invalid` at the value itself, expected `Model("Status")`, and `{ "Draft": null }`, which serde reads and the enum does not write, lists `Mistyped`. `from_bson_with` is generated on a plain enum as on every flagged type, and every call to it reaches the callback, a document never being the bare name the enum writes.

**A tagged enum** has its tag read first, and the variant the tag names is walked where that form writes it. The examples read a struct holding one enum of each form as `outline`, `fill` and `stroke`.

- Externally tagged, the enum is a unit variant's name as text, or an object whose one key names a variant over what it holds. A key naming no variant is `Invalid` at the enum's own path, holding the whole value: `"outline": { "Hexagon": {} }` lists it at `outline`, expected `Variants(["Circle", "Empty", "Label"])`. An issue inside a variant sits under the variant's key: `outline.Circle.radius`.
- Internally tagged, the tag is read from its key, and the variant's fields are walked in the same object. A tag naming no variant is `Invalid` at the tag's key: `"fill": { "color": "red", "kind": "Striped" }` lists it at `fill.kind`, expected `Variants(["Clear", "Solid", "Versioned"])`. An absent tag is `Missing` at that key, and an issue inside a variant sits beside the tag: `fill.number`. A variant that holds one flagged type hands that type the object without the tag's entry, which is what serde hands it.
- Adjacently tagged, the tag is read from one key and what the variant holds is walked under the other: `"stroke": { "data": {}, "kind": "Dashed" }` lists `Missing` at `stroke.data.gap`.
- A tag naming no variant that serde reads all the same, as the variant marked `#[serde(other)]`, is `Mistyped` at the tag.
- The content of a variant with named fields, held as something other than an object, is the one place the walk lists nothing: `{ "Circle": 5 }` carries serde's refusal alone, as an `Undescribed`.

**An untagged enum** is walked as the variant serde reads the value as, and that variant alone. Where serde reads it as none, the one issue is a `NoVariant` at the enum's path, holding each variant's own list in the order declared. A constrained member is read through the hook a schema surface hangs on it, so a value its bound refuses takes that variant out.

**A variant's `alias`** is a tag serde reads the variant under, so a tag or a key stored as one names that variant: nothing is listed for the alias, an issue inside the variant is listed under the key it is stored under, `Round.radius` for a `Circle` stored as `Round`, and `Variants` lists each variant's name and then its aliases. **A variant serde never reads**, under `skip_deserializing` or `skip`, is no variant to the walker: a tag naming it is `Invalid`, `Variants` leaves it out, and an untagged enum holds no list for it in its `NoVariant`.

**A `#[serde(flatten)]` field** has its keys among the keys of the object that holds it, so its issues sit at that object's path, never under the name of the field. A flattened type is handed what serde hands it: that object without the keys the outer type's own fields are read under, and without the entries serde took for the flattened fields declared before it. A struct with named fields, a plain or externally tagged enum and an adjacently tagged enum take the entries serde reads for them; a struct that itself flattens a field, and an internally tagged or untagged enum are read from what is left and take nothing, so a type flattened after one is handed its keys as well. A map is the [one member of its object](#intersection-types-serdeflatten). The examples read `struct Sheet`, which declares `title: String` and flattens an `audit: Audit` holding `createdBy` and `revision`, an `extra: Option<Extra>` holding `note` and `weight`, and a `paint: Paint`, an enum internally tagged under `kind`.

- A flattened flagged struct is walked by its own fields walker, and the keys it declares count as declared: `"createdBy": 7` lists `Invalid` at `createdBy`, and a key neither `Sheet` nor a type it flattens declares is `Unknown` once, `legacy` at `legacy`.
- A flattened `Option` of a flagged struct is read by serde as absent wherever serde does not read the struct from the object. With none of its keys there, nothing is listed. With `note` there and `weight` not, the object holds what the field would not write, and the one issue is a `Mistyped` at the object's own path, expected `Optional(Model("Extra"))`. Where serde reads it, it lists what its own walk finds. A flattened `Option` of an enum is absent when what names its variant is absent: the tag's key for an internally or adjacently tagged enum, every variant's key for an externally tagged one, and, for an untagged enum, whenever serde reads none of its variants from the object. The object's keys are then the outer type's to judge, so one nothing declares is `Unknown` as anywhere else.
- A flattened tagged enum is walked under its own keys in that object: a `kind` naming no variant is `Invalid` at `kind`, and an absent one is `Missing` there. An externally tagged enum is walked under the key naming its variant, and an adjacently tagged one under its tag and content keys.
- A flattened untagged enum whose variants are objects with keys of their own is walked as the variant serde reads from what it is handed, and is one `NoVariant` at the object's path, holding what it was handed, where serde reads none.
- A flattened map takes every key of its object, and each value is walked at its key: read as a struct that flattens a `HashMap<String, i32>`, `{ "a": "x", "b": 2 }` lists `Invalid` at `a`, and no key is `Unknown`.
- A flattened type parameter is read whole, with its own reader, from the keys nothing else declares: one issue at the object's path, expected `TypeParam("T")`. A flattened field its author's hook reads is read the same way, through the hook. Where either reads an `Option`, in what fills the parameter or behind the hook, serde reads it as absent wherever what it holds is refused, so nothing is listed for it: read as a `Letter<Option<Plain>>`, which declares `id` and flattens its parameter, `{ "id": "i" }` lists nothing, and neither does `{ "id": "i", "text": 5 }`, which serde reads with the `Option` absent.
- A flattened type that itself takes every key it is handed, an untagged enum with a map among its variants, takes none of the outer type's own. Read as `struct Scoreboard`, which declares `id: String` and flattens `enum Scores { Captioned(Captioned), Tallied(HashMap<String, i32>) }`, `{ "a": 1, "id": "i" }` lists nothing. A key the outer type and a type it flattens both declare is the outer type's: the flattened type is handed an object without it.
- Beside a struct and a map, serde flattens a single-slot struct or a brand as the value it holds, and a plain enum as the key naming its variant, holding `null`. Each is walked as that, and the keys serde reads for it are its own: a single-slot struct over a map, over an `Option` of a flagged type, over a type parameter or over a JSON value is walked as a flattened field of that value is, and a plain enum, which builds flattened only where it is declared below the type that flattens it or in another crate, as an externally tagged enum whose variants hold nothing. A single-slot struct whose slot carries a hook of the author's is read whole with the struct's own reader, which runs the hook, and every key is its own; where that slot is an `Option`, nothing is listed for it. A single-slot struct over an `ObjectId`, which serde writes as an object, takes every key, and nothing is listed for it. Read as a struct that flattens `struct Extras(HashMap<String, i32>)`, `{ "a": "x" }` lists `Invalid` at `a`, and no key is `Unknown`. serde refuses to flatten a tuple struct and a single-slot struct over text, a list or a tuple, which build flattened only where they are declared below the type that flattens them or in another crate: nothing is listed for one, no key is its own, and the read carries serde's refusal as an `Undescribed`.
- Some flattened fields are walked by nothing: an `Option` of a map, an `ObjectId`, which serde writes as an object and so flattens, and a second field that takes the keys nothing else declares, after the first one. Every key then counts as declared, so none is `Unknown`, and serde's verdict is the read's: a value serde refuses there is one `Undescribed`.
- A flattened field of a variant is walked in the object the variant's fields sit in, wherever the enum's form puts that object.

**A generic type** gets its methods on `impl`s of their own, which bound every type parameter, and the type itself, `serde::de::DeserializeOwned`, and `serde::Serialize` as well for `from_bson_with`. A type that itself bounds a parameter `Serialize` or `DeserializeOwned` has that bound written a second time on these `impl`s, which clippy reports at the declaration as `trait_duplication_in_bounds` (a `nursery` lint), as it already does for the `Serialize` bound serde's own derive repeats; leave the bound off the type and neither is reported, serde's derive and these `impl`s each adding what they need. A value of a parameter's type is read whole, with that parameter's own reader, and so is a list, a map, an `Option` or a tuple that holds one: read as `Page<Version>`, where `Page<T>` declares `items: Vec<T>`, a bad number inside one item is a single `Invalid` at `items`, expected `Array(TypeParam("T"))`. From a JSON value such a value is never `Mistyped`, nothing there being bound to write it back. From a BSON document it is: an id filling a parameter and stored as text is `Mistyped` at its field.

### Nested types

Every model type a flagged type's fields reach lists `"decode_with"` too, whatever its shape. Where the model type is declared above the flagged one and has no flag, the build fails: the flagged type's walker calls the nested type's walker, so the error names the method the flag would have generated:

```rust
tixschema::decode_with_foundation!();

#[model_schema()]
#[derive(Deserialize, Serialize)]
pub struct Version {
    pub number: i32,
}

#[model_schema(enable = ["decode_with"])]
#[derive(Deserialize, Serialize)]
pub struct Record {
    pub versions: Vec<Version>,
}
```

```text
error[E0599]: no associated function or constant named `decode_with_value_issues` found for struct `Version` in the current scope
```

The error is reported at the `#[model_schema(...)]` of the type that names the unflagged one. A type flattened without the flag earns the same error twice, naming `decode_with_value_named` and `decode_with_value_fields`, and, in a build with `bson` on, each of these errors comes a second time, for `decode_with_bson_issues`, `decode_with_bson_named` and `decode_with_bson_fields`.

**A type that lists `"mongodb"` may hold a model type that lists `"decode_with"` alone**, declared above it, below it or in another crate. Its BSON walk calls the held type's BSON walker, which every flagged type has under `bson`, and lists an issue inside the held type at its full path. With `Record` listing both entries and holding `current: Version`, and `Version` listing `"decode_with"` alone with `minimum = 1` on its `number`, `from_bson_with` lists `{ "current": { "number": 0 } }` as one `Invalid` at `current.number`, in a build with a schema surface on.

- **A model type filling a type parameter needs no flag.** It is read as one value, so `Page<Plain>` reads with no flag on `Plain`.
- **A field typed with an alias of a flagged model type is walked**, the alias being that type: with `#[model_schema()] type EntryAlias = Entry;`, a field `featured: EntryAlias` lists an issue at `featured.name`.
- **A field typed with an alias of a list or a map is walked as that list or map**, where `#[model_schema]` is written on the alias and the alias is declared above the type that names it. With `#[model_schema()] pub type Versions = Vec<Version>;` and a field `all: Versions`, `{ "all": [{ "number": 1 }, { "number": "2" }] }` lists `Invalid` at `all[1].number`, and with `#[model_schema()] pub type Counts = HashMap<String, i32>;` and a field `counts: Counts`, `{ "counts": { "a": "x", "b": 2 } }` lists `Invalid` at `counts.a`, expected `I32`. The field publishes under the alias's name on every schema surface, as it does without the flag. The alias may be declared in another module: what it holds is reached from the field's own type, so nothing the alias is written with has to be in scope beside the field. An alias that takes a type parameter, and one declared below the type that names it, are read whole, as the next point says.
- **A field of a type tixschema has not seen is read whole.** A name `#[model_schema]` was not written on above the flagged type may be a `type` alias with no `#[model_schema]`, a type from another crate, or a model type declared below, and tixschema cannot tell which. So the walker asks the type for its walker, and where the type has none it reads the value whole, with the type's own `Deserialize`: a value serde refuses is one `Invalid` at the field, expected `Unknown`, and flattened, the field takes every key nothing else declares. With `pub type Properties = HashMap<String, u8>;` and a field `properties: Properties`, `{ "properties": { "a": 5 } }` reads with no issue and `{ "properties": 5 }` lists `Invalid` at `properties`. A flagged model type declared below is walked by its own walker all the same. An unflagged one declared below is read whole, where one declared above fails the build. A field `timeout: Duration`, `Duration` being `std::time::Duration`, is read whole too; the `duration_schema` error the same field earns wherever a schema surface is on is no part of the flag and is unchanged.

### What is refused

The entry `"decode_with"` is refused where the methods cannot be written, with a `compile_error!`, in every feature combination. No other shape is refused. What the `enable` list itself refuses is in [Opting in](#opting-in).

**On a type alias.** An alias is another name for a type that already exists, so the methods would be that type's. For `pub type DecodeVersions = Vec<DecodeVersion>;`:

```text
model_schema: type `DecodeVersions`: `enable = ["decode_with"]` cannot be written on a type alias: `DecodeVersions` is another name for `Vec<DecodeVersion>`, and the methods would have to be added to that type. List it on the model types the alias reaches.
```

**On a type that declares a lifetime.** The methods own the value they read, and a decoded value cannot borrow from it. For `pub struct DecodeNamed<'a> { pub name: &'a str }`, and for an enum declaring `'a` alike:

```text
model_schema: type `DecodeNamed`: `enable = ["decode_with"]` cannot be written on a type that borrows: `from_value_with` owns the value it reads, so the decoded `DecodeNamed<'a>` cannot borrow from it for `'a`.
```

**On a type with a field that borrows.** A reference written in a field borrows from the owned value as a declared lifetime does. For `pub struct DecodeLabelled { pub label: &'static str }`:

```text
model_schema: type `DecodeLabelled`: `enable = ["decode_with"]` cannot be written on a type with a field that borrows: `label` is `&'static str`, and `from_value_with` owns the value it reads, so nothing decoded from it can be borrowed for `'static`. Write the field as an owned type.
```

The message names the first such field serde reads, a reference held inside an `Option` or a `Vec` included, a tuple position by its number and an enum's field behind its variant: `Named.label`. A field that names a lifetime and borrows nothing from the value is not refused: a `Cow<'static, str>`, which serde reads as an owned value, and a reference in a field serde never reads, under `skip` or `skip_deserializing`.

### Reading from MongoDB

- **A row's `_id` is a key like any other.** Read into a type that does not declare `_id`, it is `Unknown`, and every such read goes to the callback or the resolvers. A type read from MongoDB declares `_id`, or the query projects it out with `_id: 0`, or a resolver settles the key by taking it out, as [Rows that do not read as the type](#rows-that-do-not-read-as-the-type) shows.
- **A value stored as another BSON type than its field writes is `Mistyped`**, serde reading it all the same: a MongoDB query for the field's own type does not match such a value. Numbers count as one type, `Int32`, `Int64`, `Double` and `Decimal128`, strings and symbols as one, and every other BSON type matches only itself. An id stored as its hex text where the field is an `ObjectId` is `Mistyped`; a number stored as an `Int64` where the field writes an `Int32` is no issue.
- **A `chrono` `DateTime<Tz>` field writes text, or a number under `as_number`**, so a row holding a BSON date in it is `Invalid`, serde refusing it.
- **A reader on an older model** sends every record a newer model wrote with an added field to the callback, the added key being one its type does not declare.
- **The `reason` of an issue is serde's refusal or a bound's sentence.** For a value serde refuses it is the `bson` library's own wording, which differs between its two major versions. For a value that breaks a bound it is the bound's own sentence, the same under both. A callback or a resolver matches on the member, the path, `expected` and `found`, never on `reason`.
- **A value that breaks two bounds is one `Invalid` issue at its path**, its `reason` every violation in the order `validate()` reports them, joined by `; `. One repair of the value at that path answers it.
- **With tixschema's `mongodb` feature on, a type that lists `"mongodb"` reads its own rows from a collection**, each through `from_bson_piped`: [MongoDB Operations and Typed Filters](#mongodb-operations-and-typed-filters).

### Cost

Every read walks the record, one that decodes cleanly included: the walk is what finds a key the type does not declare, a value held in another form than its field writes and a value that breaks a bound its field declares. Each plain value is read a second time and written back, to compare it with what is held.

`from_bson_with` copies the document once for each read of it by serde, `bson::Deserializer::new` taking the value it reads by value: once for the read that decodes it, and once more for the read of what the callback left after it answers `Fixed`. The walk borrows the document. Whatever the walk reads with serde on its own, each plain value first of all, it copies as it reads it, for the same reason.

Measured on one machine in a release build, on the `Record` below holding two versions, a `from_value_with` read that calls no callback took about 2.7 times what `serde_json::from_value` takes to read the same value, and a `from_bson_with` read about 3.3 times what the `bson` library's own read takes to read the same document, `bson::from_document` in version 2 and `bson::deserialize_from_document` in version 3. Each read was handed its own copy of the value and consumed it. Plain serde reading the JSON value through a reference, `Record::deserialize(&value)`, leaves the value with its caller and took less than half of what `serde_json::from_value` takes: `from_value_with` took about six times that.

A type that flattens another flagged type beside keys of its own copies the entries its own fields did not take, once each time it is walked, and hands that copy to the first type it flattens: those entries are what serde hands a flattened type. A type with no key of its own hands over the object it is read from, and copies nothing. A flattened type declared after another is handed what serde left once it had read the earlier ones: one more copy each time an earlier one took entries, and the same object each time it took none. What an earlier one takes is asked of its own reader, which is called once more for it with a deserializer that reads nothing. An internally tagged variant that holds one flagged type copies the object without the tag's entry. Where a flattened value read through a hook, or of a parameter's type, is refused, its reader is called once more, with that same deserializer, to ask whether it reads an `Option`.

### A complete callback

A `Record` with an inner `Version`, read from the MongoDB rows older writers left. Both list `"decode_with"` and `"mongodb"`, as stored types do, and the crate that declares them writes both [initializers](#the-support-libraries) at its root. `from_bson_with` is what `"decode_with"` generates in a build with `bson` on. The example needs tixschema's `mongodb` and `chrono` features.

```rust
use bson::oid::ObjectId;
use bson::{Bson, Document};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub number: i32,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    #[serde(rename = "recordId")]
    pub id: ObjectId,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub versions: Vec<Version>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}
```

The callback, a function over `Record` and its inner `Version`. An issue inside a version reaches it as `record_schema::Issue`, at its full path:

```rust
use record_schema::{Expected, Issue, Verdict};

/// Repairs what older writers left in a `Record` row, inner versions included, and rejects
/// anything else.
fn repair_record(row: &mut Document, issues: &[Issue<Bson>]) -> Verdict {
    for issue in issues {
        if let Issue::Mistyped {
            path,
            expected: Expected::ObjectId,
            found: Bson::String(hex),
        } = issue
        {
            // the record's id, stored as text by rows written before it was an `ObjectId`
            let Ok(id) = ObjectId::parse_str(hex) else {
                return Verdict::Reject;
            };
            path.set_in_document(row, Bson::ObjectId(id));
        } else if let Issue::Invalid {
            path,
            expected: Expected::DateTime,
            found: Bson::DateTime(date),
            reason: _reason,
        } = issue
        {
            // the creation date, stored as a BSON date by a driver that writes dates natively
            let Some(created) = DateTime::<Utc>::from_timestamp_millis(date.timestamp_millis())
            else {
                return Verdict::Reject;
            };
            path.set_in_document(row, Bson::String(created.to_rfc3339()));
        } else if let Issue::Invalid {
            path,
            expected: Expected::DateTime,
            found: Bson::Int64(millis),
            reason: _reason,
        } = issue
        {
            // the creation date, stored as epoch milliseconds
            let Some(created) = DateTime::<Utc>::from_timestamp_millis(*millis) else {
                return Verdict::Reject;
            };
            path.set_in_document(row, Bson::String(created.to_rfc3339()));
        } else if let Issue::Invalid {
            path,
            expected: Expected::I32,
            found: Bson::String(text),
            reason: _reason,
        } = issue
        {
            // a version's number, stored as text: the path is `versions[1].number`
            let Ok(number) = text.parse::<i32>() else {
                return Verdict::Reject;
            };
            path.set_in_document(row, Bson::Int32(number));
        } else if let Issue::Unknown {
            path,
            found: _found,
        } = issue
        {
            // a key no type declares, on the record or on one of its versions
            path.remove_from_document(row);
        } else {
            return Verdict::Reject;
        }
    }
    Verdict::Fixed
}
```

A stored row with problems at three levels: the record's id stored as text and its creation date as a BSON date, a version's number stored as text beside a key no version declares, and the row's own `_id`, which `Record` does not declare. Shown as MongoDB Extended JSON:

```json
{
  "_id": { "$oid": "6a7cc592ca0574e6efdfe299" },
  "recordId": "6a7cc592ca0574e6efdfe217",
  "name": "Loan",
  "createdAt": { "$date": "2025-10-04T17:46:40Z" },
  "versions": [{ "number": 1 }, { "number": "2", "draft": true }]
}
```

Read with the callback, `row` being that row as a `bson::Document`:

```rust
let record = Record::from_bson_with(row, repair_record)?;
```

The callback runs once and is handed five issues, `Found` being the held value as `bson::Bson` prints it. It can read the row's `_id` from the raw document it is handed, `row.get_object_id("_id")`, to record which row it repaired.

| Member | Path | Expected | Found |
|--------|------|----------|-------|
| `Mistyped` | `recordId` | `ObjectId` | `String("6a7cc592ca0574e6efdfe217")` |
| `Invalid` | `createdAt` | `DateTime` | `DateTime(2025-10-04 17:46:40.0 +00:00:00)` |
| `Invalid` | `versions[1].number` | `I32` | `String("2")` |
| `Unknown` | `versions[1].draft` | | `Boolean(true)` |
| `Unknown` | `_id` | | `ObjectId("6a7cc592ca0574e6efdfe299")` |

It answers `Fixed`, the second read finds no issue, and the call returns this `Record`, each version shown by its `number`:

| Field | Value |
|-------|-------|
| `id` | `6a7cc592ca0574e6efdfe217` |
| `name` | `Loan` |
| `created_at` | `2025-10-04T17:46:40+00:00` |
| `versions` | `[1, 2]` |
| `note` | `None` |

### Resolvers

`from_bson_piped` and `from_value_piped` take resolvers where `from_bson_with` and `from_value_with` take one callback. A resolver is a function over the raw value and one issue, and it answers that issue alone: `Settled`, `Rejected` or `NotTouched`. Each resolver knows one repair, so a read is handed the ones its rows need, in the order they are to run.

A resolver is written against `Issue`, which a crate holds once: within one crate, `invoice_schema::Issue` and `order_schema::Issue` are one type, so one resolver serves the reads of every flagged type the crate declares ([One library per crate](#one-library-per-crate)). A type of another crate has its own crate's `Issue`. An issue inside a nested type reaches the outer type's resolvers as the outer type's `Issue`, at its full path, so a resolver for `Invoice` repairs what is wrong inside its `Customer`.

An `Invoice` that holds a `Customer`, both read from the rows older writers left. Both list `"decode_with"` and `"mongodb"`, as stored types do. `from_bson_piped` is what `"decode_with"` generates in a build with `bson` on, and the example needs tixschema's `bson` feature.

```rust
use bson::{Bson, Document, doc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Customer {
    pub name: String,
    pub open_invoices: u32,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Invoice {
    pub customer: Customer,
    pub number: String,
}
```

Two resolvers. Each looks for the one issue it repairs and answers `NotTouched` for any other:

```rust
use invoice_schema::{Expected, Issue, Resolution};

/// A customer an older writer stored as JSON text: parsed into a document.
fn customer_as_text(raw: &mut Document, issue: &Issue<Bson>) -> Resolution {
    let Issue::Invalid {
        path,
        expected: Expected::Model("Customer"),
        found: Bson::String(text),
        reason: _reason,
    } = issue
    else {
        return Resolution::NotTouched;
    };
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text) else {
        return Resolution::Rejected;
    };
    // Each major version of the library names its own function for this; both have the serializer.
    let Ok(Bson::Document(customer)) = parsed.serialize(bson::Serializer::new()) else {
        return Resolution::Rejected;
    };
    if path.set_in_document(raw, Bson::Document(customer)) {
        Resolution::Settled
    } else {
        Resolution::Rejected
    }
}

/// A whole number an older writer stored as text; a negative count is refused.
fn whole_number_as_text(raw: &mut Document, issue: &Issue<Bson>) -> Resolution {
    let Issue::Invalid {
        path,
        expected: Expected::U32,
        found: Bson::String(text),
        reason: _reason,
    } = issue
    else {
        return Resolution::NotTouched;
    };
    let settled = text
        .trim()
        .parse::<i64>()
        .is_ok_and(|number| number >= 0_i64 && path.set_in_document(raw, Bson::Int64(number)));
    if settled {
        Resolution::Settled
    } else {
        Resolution::Rejected
    }
}
```

Five stored rows, and the read of each with both resolvers:

```rust
/// Five rows older writers left.
fn stored_rows() -> [Document; 5] {
    [
        // a count stored as text
        doc! {
            "customer": { "name": "Acme", "openInvoices": "3" },
            "number": "INV-0042",
        },
        // a count stored as text that is no count
        doc! {
            "customer": { "name": "Acme", "openInvoices": "-3" },
            "number": "INV-0042",
        },
        // a key no type declares
        doc! {
            "customer": { "name": "Acme", "openInvoices": 3 },
            "legacy": true,
            "number": "INV-0042",
        },
        // a customer stored as text, beside a key no type declares
        doc! {
            "customer": "{\"name\":\"Acme\",\"openInvoices\":3}",
            "legacy": true,
            "number": "INV-0042",
        },
        // a customer stored as text, whose count is text too
        doc! {
            "customer": "{\"name\":\"Acme\",\"openInvoices\":\"3\"}",
            "number": "INV-0042",
        },
    ]
}

/// One stored row read with both resolvers, told in one line.
fn told(row: Document) -> String {
    match Invoice::from_bson_piped(row, &[&customer_as_text, &whole_number_as_text]) {
        Ok(invoice) => format!("read as {invoice:?}"),
        Err(refused) => format!("refused: {refused}"),
    }
}
```

The five rows told in order, under version 2 of the `bson` library:

```text
read as Invoice { customer: Customer { name: "Acme", open_invoices: 3 }, number: "INV-0042" }
refused: customer.openInvoices: invalid: expected U32, found String("-3"): invalid type: string "-3", expected u32
refused: legacy: unknown: found Boolean(true)
refused: legacy: unknown: found Boolean(true)
refused: customer.openInvoices: invalid: expected U32, found String("3"): invalid type: string "3", expected u32
```

1. The first row reads. `customer_as_text` leaves its one issue `NotTouched`, `whole_number_as_text` settles it, and the second read finds no issue.
2. The second is refused by the resolver that knows its issue: `whole_number_as_text` answers `Rejected` for a count below zero.
3. The third is refused over an issue no resolver knows. Both leave the key `legacy` `NotTouched`.
4. The fourth is refused over that same issue, and that one alone: `customer_as_text` settled the customer stored as text, and a settled issue is not among the ones a refused read holds.
5. The fifth is refused by the second read. `customer_as_text` settles the customer, every issue of the first read is then settled, and the read of what the resolvers left finds the count inside that customer held as text. No resolver is handed that issue: the resolvers run once.

The second and the fifth line end in the `bson` library's own wording, which its two major versions write differently. Under version 3 the same two lines read:

```text
refused: customer.openInvoices: invalid: expected U32, found String("-3"): BSON error. Kind: A deserialization-related error occurred. Message: invalid type: string "-3", expected u32.
refused: customer.openInvoices: invalid: expected U32, found String("3"): BSON error. Kind: A deserialization-related error occurred. Message: invalid type: string "3", expected u32.
```

What the pipe holds every read to:

- **Resolvers run in the order of the slice.** Each one is handed, one at a time and in the order the read found them, the issues every resolver before it left `NotTouched`.
- **`Settled` says the resolver repaired the raw value.** Nothing checks the repair but the second read, which lists the issue again where the value still does not read.
- **`Rejected` fails the read.** The issue is known and is not to be repaired. No later resolver is handed it, and the failed read holds it.
- **An issue every resolver leaves `NotTouched` fails the read** as well, and the failed read holds it.
- **A failed read holds the issues no resolver settled**, in the order they were found, as `Unrecovered`. With no resolver at all, `&[]`, that is every issue.
- **A resolver is `Sync`.** It is a function, or a closure that holds what it needs, taken by reference: `&[&customer_as_text, &counted]`. A closure that counts in a `Cell` or a `RefCell` is refused (`E0277`, `` `Cell<u32>` cannot be shared between threads safely``), and one that counts in an atomic or behind a `Mutex` is taken. It is what lets a read that waits for MongoDB hold its resolvers and still move to another thread.

`from_value_piped` is the same read of a `serde_json::Value`. Its resolvers take `&mut serde_json::Value` and `&Issue<serde_json::Value>`, and repair through `set_in_value` and `remove_from_value`.

## archie Frontmatter Conversions (`archie`)

With tixschema's `archie` feature on, a type declared with `#[model_schema(enable = ["archie"])]` converts to archie's frontmatter `Value`, and is read back from one. Both go through serde, so a `Value` holds what serde writes for the type:

```rust
#[model_schema(enable = ["archie"])]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_on: Option<String>,
    pub tags: Vec<String>,
    #[model_schema_prop(minLength = 1)]
    pub title: String,
}

let value: Value = (&note).try_into()?;  // { "tags": ["parser"], "title": "Notes" }
let note = Note::try_from(&value)?;
```

- **The write** is `TryFrom<&Note> for archie_core::parser::ast::Value`, through `archie_core::value::to_value`. It fails only where a member's own `Serialize` returns an error, or where a map key is not text, a number or a boolean.
- **The read** is `TryFrom<&Value> for Note`, through `archie_core::value::from_value`. It is the type's own read, so it refuses what [`validate()`](#a-read-refuses-what-validate-refuses) refuses: a `title` written `""` is refused with `'title': too short: minimum length is 1, got 0`. A date, a time and a date-time the frontmatter holds are read as the text chrono reads.
- **Both errors** are `archie_core::value::ValueError`.
- **A type with parameters** converts at any filling. Each impl is bounded on the type itself: the write on `Tagged<Id>: Serialize`, the read on `Tagged<Id>: DeserializeOwned`.
- **The recovering decode needs no entry of its own.** archie's `Value` turns into a `serde_json::Value`, and `from_value_with` reads it: `Note::from_value_with((&value).into(), decide)`, for a type that also lists `"decode_with"`.
- **A type alias is refused**, in every build: `` `enable = ["archie"]` cannot be written on a type alias: `Short` is another name for `String`, and the conversions would have to be written for that type. List it on the model types the alias reaches.``
- **What your crate lists:** `archie-core`, under the name `archie_core`, and `serde`. Without the `archie` feature, the entry adds nothing.

## MongoDB Operations and Typed Filters

With tixschema's `mongodb` feature on, a type declared with [`#[model_schema(enable = ["decode_with", "mongodb"])]`](#opting-in) reads and writes its own rows in a MongoDB collection, and names its own fields in the filters and updates it does so with:

- **Typed paths.** `MONGO_FIELDS`, a const on the type, answers one path per key serde writes for it, each from a function named after the field. A path builds a filter or an update from a value of its field's own type, so a field that does not exist and a value of another type are compile errors. `nest` writes what is asked of a nested model's own paths under the key that holds the model.
- **Operations.** `find_one`, `find`, `count`, `insert_one`, `update_one`, `update_many`, `delete_one` and `delete_many`, over a `mongodb::Collection<bson::Document>`. Every row a read is handed goes through the recovering decode, so a row that does not read as the type is reported by its `_id` and its issues, and `find_one_with` and `find_with` take the [resolvers](#resolvers) that repair one.

The entry `"mongodb"` is what turns both on, under tixschema's `mongodb` feature, and it needs `"decode_with"` beside it. A type whose list holds `"decode_with"` alone has neither in any build. In a build without the `mongodb` feature the entry adds nothing and is accepted: a type that lists it has the recovering decode `"decode_with"` gives it, from JSON, and from BSON under `bson`, and no operation, no `MONGO_FIELDS` and no typed path. A type that is never stored lists `"decode_with"` alone, whether it is read from BSON or not, and none of this section's code is generated for it.

The query types and what the operations answer are the crate's MongoDB [support library](#the-support-libraries), which `mongodb_foundation!()` writes once under the `mongodb` feature, and which the `<type>_schema` module of every type that lists `"mongodb"` re-exports name by name, beside the issue types. So `invoice_schema::Filter`, `invoice_schema::OperationError` and `invoice_schema::Read` are the crate's one `Filter`, `OperationError` and `Read`, and `customer_schema` names the same three. A compiler error names such a type where the library declares it, `__tixschema_mongodb::Filter<Invoice>`. What each type writes for itself is its typed paths, `MongoFields` in its own module and `MONGO_FIELDS` on the type, and its operations, on the type. A filter and an update still belong to the rows of one type, which their first parameter names: a `Filter<Invoice>` is over the rows of `Invoice`, and is refused where a filter over the rows of `Customer` is asked. `nest` takes a filter or an update built from a nested model's own paths, and answers one over the outer type's rows ([Filters and updates](#filters-and-updates)).

No line of this section is the answer of a MongoDB server. A document shown is the one a call writes, as `bson::Document` displays it, and an answer is the driver's own result type.

### The driver your crate lists

The code generated for a type that lists `"mongodb"`, and the library `mongodb_foundation!()` writes, name the `mongodb` driver, so a crate that declares such a type lists the driver beside `bson`, `serde` and `serde_json`, whether or not it calls an operation. Without it the build fails with ``error[E0433]: cannot find module or crate `mongodb` in this scope``. A crate whose types list `"decode_with"` alone lists no driver, and lists `bson` wherever tixschema's `bson` feature is on, as [What your crate lists](#what-your-crate-lists) says.

The crate's `bson` is the major version its driver is built for. The driver's default features build it for version 2 of the `bson` library:

```toml
mongodb = "3.9"
bson = "2.15"
```

With its default features off and `bson-3` on, it is built for version 3:

```toml
mongodb = { version = "3.9", default-features = false, features = ["bson-3", "compat-3-3-0", "rustls-tls", "dns-resolver"] }
bson = { version = "3.1", features = ["serde"] }
```

`compat-3-3-0`, `rustls-tls` and `dns-resolver` are part of the driver's defaults, written out because turning the defaults off turns them off too. The crate's own `tests/` build against the driver at its default features and `bson` 2.15, and `bson3/`, this repository's package for version 3, against the second pair.

A `bson` of the other major version fails the build at each type that lists `"mongodb"`, the generated operations handing the driver a `Document` it does not take:

```text
error[E0308]: mismatched types
 --> src/lib.rs:8:1
  |
8 | #[model_schema(enable = ["decode_with", "mongodb"])]
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  | |
  | expected `mongodb::bson::Document`, found `bson::Document`
  | arguments to this method are incorrect
  |
note: there are multiple different versions of crate `bson` in the dependency graph
```

The note goes on to name the two versions, each at its place in the Cargo registry of the machine that compiled it.

### The model of the examples

The examples of this section share one model: an `Invoice`, and the types it holds. Every one of them lists `"decode_with"` and `"mongodb"`, and each is declared above the type that holds it.

```rust
use std::collections::HashMap;

use bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

tixschema::decode_with_foundation!();
tixschema::mongodb_foundation!();

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    pub city: String,
    pub postal_code: String,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Customer {
    pub address: Address,
    pub name: String,
    pub open_invoices: u32,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum InvoiceStatus {
    Draft,
    Paid,
    PastDue,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct LineItem {
    pub price: f64,
    pub quantity: u32,
    pub sku: String,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Payment {
    Card { last4: String },
    Cash,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Invoice {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub billing: Option<Address>,
    pub customer: Customer,
    pub details: HashMap<String, String>,
    pub items: Vec<LineItem>,
    pub number: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paid_at: Option<String>,
    pub payment: Payment,
    pub status: InvoiceStatus,
    pub tags: Vec<String>,
    pub total: f64,
}
```

### The paths of a type

`Invoice::MONGO_FIELDS` is an `invoice_schema::MongoFields<Invoice>`. It holds no path: it has one function per field of `Invoice`, under the field's Rust name, and each answers the path of the key serde writes that field under. The type of the field decides the kind of the path:

| Function | The kind it answers |
|----------|---------------------|
| `id()` | `Field<Invoice, ObjectId>` |
| `billing()` | `OptionalModel<Invoice, Address>` |
| `customer()` | `Model<Invoice, Customer>` |
| `details` | none: a map's keys are data |
| `items()` | `ModelList<Invoice, LineItem>` |
| `number()` | `Field<Invoice, String>` |
| `paid_at()` | `OptionalField<Invoice, String>` |
| `payment()` | `Model<Invoice, Payment>` |
| `status()` | `Field<Invoice, InvoiceStatus>` |
| `tags()` | `ListField<Invoice, String>` |
| `total()` | `Field<Invoice, f64>` |

Each kind is a type of `invoice_schema`. The first parameter is the row a filter is over, and the second the value an operator takes. A path is one key, the one its own type writes: the paths of `Invoice` hold no key of `Customer`, and those of `Customer` none of `Address`.

**`nest` reaches a nested model.** `Customer` has paths of its own, `Customer::MONGO_FIELDS`. `nest`, on the path that holds a customer, takes a filter or an update built from those and answers the same filter or update over the rows of `Invoice`, each key written under the path's own. `paths.customer().nest(customer.address().nest(address.city().eq("Moca".to_owned())?))` asks for a city three types down, and the key it writes is `customer.address.city`.

A path builds a `Filter<Invoice>` or an `Update<Invoice>`, the two documents an operation takes. Building one asks nothing of MongoDB:

```rust
use bson::doc;

use invoice_schema::{Filter, Update, WriteError};

/// Unpaid invoices of one city that are past due or above an amount.
fn to_chase(city: &str, above: f64) -> Result<Filter<Invoice>, WriteError> {
    let (paths, customer) = (Invoice::MONGO_FIELDS, Customer::MONGO_FIELDS);
    let in_city = Address::MONGO_FIELDS.city().eq(city.to_owned())?;
    Ok(paths
        .status()
        .eq(InvoiceStatus::PastDue)?
        .or(paths.total().gt(above)?)
        .and(paths.customer().nest(customer.address().nest(in_city)))
        .and(paths.paid_at().exists(false)))
}

/// What paying an invoice changes.
fn paid(at: &str) -> Result<Update<Invoice>, WriteError> {
    let (paths, customer) = (Invoice::MONGO_FIELDS, Customer::MONGO_FIELDS);
    Ok(paths
        .status()
        .set(InvoiceStatus::Paid)?
        .and(paths.paid_at().set(at.to_owned())?)
        .and(paths.customer().nest(customer.open_invoices().set(0)?))
        .and(paths.tags().push("settled".to_owned())?))
}
```

| Written | The document it builds |
|---------|------------------------|
| `to_chase("Moca", 1000.5)?` | `{ "$and": [{ "$or": [{ "status": { "$eq": "past-due" } }, { "total": { "$gt": 1000.5 } }] }, { "customer.address.city": { "$eq": "Moca" } }, { "paidAt": { "$exists": false } }] }` |
| `paid("2026-10-07T17:40:00Z")?` | `{ "$set": { "status": "paid", "paidAt": "2026-10-07T17:40:00Z", "customer.openInvoices": 0 }, "$push": { "tags": "settled" } }` |

- **A value is of the field's own type, taken by value.** `paths.status().eq` takes an `InvoiceStatus`, and the text serde writes for one is refused: `paths.status().eq("past-due")` is ``error[E0308]: mismatched types``, ``expected `InvoiceStatus`, found `&str` ``. `paths.total().gt` takes an `f64`.
- **A field the type does not declare is a function that does not exist.** `paths.amount()` is ``error[E0599]: no method named `amount` found for struct `invoice_schema::MongoFields<Root>` in the current scope``.
- **A value is written the way its field is stored**, by the type's own `Serialize` through `bson::Serializer::new()`: `InvoiceStatus::PastDue` as the text `past-due`, a `u32` as a 64-bit integer, an `ObjectId` as one.
- **A key is the name serde writes**: `paid_at()` is the key `paidAt`, and `open_invoices()` nested under `customer()` is `customer.openInvoices`.
- **An operator that writes a value answers a `Result`**, since a value may be one BSON cannot hold; the error is `WriteError`, the `bson` library's own serialization error, `bson::ser::Error` in version 2 and `bson::error::Error` in version 3. An operator that writes none, such as `exists`, `unset`, `size`, `regex` and `is_<variant>`, answers the filter or the update itself. `nest` answers what it is handed, a filter or an update.
- **A path is a value, and `MONGO_FIELDS` a const.** Each function is a `const fn`, so a `const` item holds a path: `const TOTAL: invoice_schema::Field<Invoice, f64> = Invoice::MONGO_FIELDS.total();`. `MongoFields` holds no path and a path holds one key, so neither grows with what the type holds.

### Operators

Every operator, once, with the document it writes. `paths` is `Invoice::MONGO_FIELDS`, and `customer`, `address`, `item` and `payment` are `Customer::MONGO_FIELDS`, `Address::MONGO_FIELDS`, `LineItem::MONGO_FIELDS` and `Payment::MONGO_FIELDS`.

**A value every row holds, a `Field`.** The comparisons, membership in a list of values, and the two operators that set it. `regex` is there where the value is a `String`, and takes MongoDB's options as text:

| Written | The document it builds |
|---------|------------------------|
| `paths.number().eq("INV-0042".to_owned())?` | `{ "number": { "$eq": "INV-0042" } }` |
| `paths.number().ne("INV-0042".to_owned())?` | `{ "number": { "$ne": "INV-0042" } }` |
| `paths.total().gt(1000.5)?` | `{ "total": { "$gt": 1000.5 } }` |
| `paths.total().gte(1000.5)?` | `{ "total": { "$gte": 1000.5 } }` |
| `paths.total().lt(1000.5)?` | `{ "total": { "$lt": 1000.5 } }` |
| `paths.total().lte(1000.5)?` | `{ "total": { "$lte": 1000.5 } }` |
| `paths.number().is_in(["INV-0042".to_owned(), "INV-0043".to_owned()])?` | `{ "number": { "$in": ["INV-0042", "INV-0043"] } }` |
| `paths.number().not_in(["INV-0042".to_owned()])?` | `{ "number": { "$nin": ["INV-0042"] } }` |
| `paths.number().regex("^INV-", "i")` | `{ "number": { "$regex": "^INV-", "$options": "i" } }` |
| `paths.total().set(99.5)?` | `{ "$set": { "total": 99.5 } }` |
| `paths.number().set_on_insert("INV-0099".to_owned())?` | `{ "$setOnInsert": { "number": "INV-0099" } }` |

**A value a row may leave out, an `OptionalField`.** Every operator of a `Field`, and two of its own: `exists`, which takes whether the key is to be there, and `unset`:

| Written | The document it builds |
|---------|------------------------|
| `paths.paid_at().exists(false)` | `{ "paidAt": { "$exists": false } }` |
| `paths.paid_at().unset()` | `{ "$unset": { "paidAt": "" } }` |
| `paths.paid_at().lt("2026-10-01".to_owned())?` | `{ "paidAt": { "$lt": "2026-10-01" } }` |

`unset` is on a path a row may leave out and on no other: `paths.number().unset()` is ``error[E0599]: no method named `unset` found for struct `__tixschema_mongodb::Field<Root, V, N>` in the current scope``.

**A type serde writes as one value**, here the plain enum `InvoiceStatus`. Held by another type it is a `Field`, with every operator of one:

| Written | The document it builds |
|---------|------------------------|
| `paths.status().eq(InvoiceStatus::PastDue)?` | `{ "status": { "$eq": "past-due" } }` |
| `paths.status().is_in([InvoiceStatus::Draft, InvoiceStatus::PastDue])?` | `{ "status": { "$in": ["draft", "past-due"] } }` |
| `paths.status().set(InvoiceStatus::Paid)?` | `{ "$set": { "status": "paid" } }` |

**A list of plain values, a `ListField`.** A list is matched through its elements: `contains` asks for one value, `contains_any` and `contains_none` for several, and `elem_match` holds one element to every condition of an `Element`, which `element()` starts and `eq`, `ne`, `gt`, `gte`, `lt` and `lte` add to. `size` takes how many elements the list holds. `push` and `pull` change one element, and `set` and `set_on_insert` take the whole list:

| Written | The document it builds |
|---------|------------------------|
| `paths.tags().contains("export".to_owned())?` | `{ "tags": { "$eq": "export" } }` |
| `paths.tags().contains_any(["export".to_owned(), "priority".to_owned()])?` | `{ "tags": { "$in": ["export", "priority"] } }` |
| `paths.tags().contains_none(["void".to_owned()])?` | `{ "tags": { "$nin": ["void"] } }` |
| `paths.tags().size(0)` | `{ "tags": { "$size": 0 } }` |
| `paths.tags().elem_match(paths.tags().element().gte("a".to_owned())?.lt("n".to_owned())?)` | `{ "tags": { "$elemMatch": { "$gte": "a", "$lt": "n" } } }` |
| `paths.tags().push("reviewed".to_owned())?` | `{ "$push": { "tags": "reviewed" } }` |
| `paths.tags().pull("draft".to_owned())?` | `{ "$pull": { "tags": "draft" } }` |
| `paths.tags().set(["new".to_owned()])?` | `{ "$set": { "tags": ["new"] } }` |
| `paths.tags().set_on_insert(["new".to_owned()])?` | `{ "$setOnInsert": { "tags": ["new"] } }` |

A comparison of the list itself is no operator: `paths.tags().gt("export".to_owned())` does not compile (`E0599`).

**A nested model, a `Model`.** `nest` writes what is asked of the model's own paths under its key, a filter and an update alike. The model is one whole value as well: `eq`, `ne`, `is_in`, `not_in`, `set` and `set_on_insert` take the model itself. `moca()` is an `Address`:

```rust
/// The address the examples write.
fn moca() -> Address {
    Address {
        city: "Moca".to_owned(),
        postal_code: "56000".to_owned(),
    }
}
```

| Written | The document it builds |
|---------|------------------------|
| `paths.customer().nest(customer.name().eq("Acme".to_owned())?)` | `{ "customer.name": { "$eq": "Acme" } }` |
| `paths.customer().nest(customer.address().nest(address.city().eq("Moca".to_owned())?))` | `{ "customer.address.city": { "$eq": "Moca" } }` |
| `paths.customer().nest(customer.name().set("John".to_owned())?)` | `{ "$set": { "customer.name": "John" } }` |
| `paths.customer().nest(customer.address().eq(moca())?)` | `{ "customer.address": { "$eq": { "city": "Moca", "postalCode": "56000" } } }` |
| `paths.customer().nest(customer.address().ne(moca())?)` | `{ "customer.address": { "$ne": { "city": "Moca", "postalCode": "56000" } } }` |
| `paths.customer().nest(customer.address().is_in([moca()])?)` | `{ "customer.address": { "$in": [{ "city": "Moca", "postalCode": "56000" }] } }` |
| `paths.customer().nest(customer.address().not_in([moca()])?)` | `{ "customer.address": { "$nin": [{ "city": "Moca", "postalCode": "56000" }] } }` |
| `paths.customer().nest(customer.address().set(moca())?)` | `{ "$set": { "customer.address": { "city": "Moca", "postalCode": "56000" } } }` |
| `paths.customer().nest(customer.address().set_on_insert(moca())?)` | `{ "$setOnInsert": { "customer.address": { "city": "Moca", "postalCode": "56000" } } }` |

**A nested model a row may leave out, an `OptionalModel`.** Every operator of a `Model`, `nest` among them, and `exists` and `unset`:

| Written | The document it builds |
|---------|------------------------|
| `paths.billing().nest(address.city().eq("Moca".to_owned())?)` | `{ "billing.city": { "$eq": "Moca" } }` |
| `paths.billing().exists(true)` | `{ "billing": { "$exists": true } }` |
| `paths.billing().unset()` | `{ "$unset": { "billing": "" } }` |
| `paths.billing().set(moca())?` | `{ "$set": { "billing": { "city": "Moca", "postalCode": "56000" } } }` |

**A list of nested models, a `ModelList`.** What `nest` writes under it matches where any element does. `elem_match` holds one element to a filter over the rows of the element's own type, and `pull` takes out every element such a filter matches. `size`, `push` and `set` are those of a list. `bolts()` is a `LineItem`:

```rust
/// The line item the examples write.
fn bolts() -> LineItem {
    LineItem {
        price: 4.5,
        quantity: 9,
        sku: "C-3".to_owned(),
    }
}
```

| Written | The document it builds |
|---------|------------------------|
| `paths.items().nest(item.price().gt(100.5)?)` | `{ "items.price": { "$gt": 100.5 } }` |
| `paths.items().elem_match(item.quantity().gte(2)?.and(item.sku().eq("B-7".to_owned())?))` | `{ "items": { "$elemMatch": { "$and": [{ "quantity": { "$gte": 2 } }, { "sku": { "$eq": "B-7" } }] } } }` |
| `paths.items().size(2)` | `{ "items": { "$size": 2 } }` |
| `paths.items().push(bolts())?` | `{ "$push": { "items": { "price": 4.5, "quantity": 9, "sku": "C-3" } } }` |
| `paths.items().pull(item.quantity().lt(1)?)` | `{ "$pull": { "items": { "quantity": { "$lt": 1 } } } }` |
| `paths.items().set([bolts()])?` | `{ "$set": { "items": [{ "price": 4.5, "quantity": 9, "sku": "C-3" }] } }` |

**An enum under a tag**, here `Payment`. `is_<variant>()` asks which variant a row holds, and what a variant holds is reached through a function named after the variant in snake case. Both are functions of the enum's own paths, which `nest` writes under the key that holds the enum. The enum is one whole value as any nested model is:

| Written | The document it builds |
|---------|------------------------|
| `paths.payment().nest(payment.is_card())` | `{ "payment.kind": { "$eq": "card" } }` |
| `paths.payment().nest(payment.is_cash())` | `{ "payment.kind": { "$eq": "cash" } }` |
| `paths.payment().nest(payment.card().last4().eq("4242".to_owned())?)` | `{ "payment.last4": { "$eq": "4242" } }` |
| `paths.payment().set(Payment::Cash)?` | `{ "$set": { "payment": { "kind": "cash" } } }` |

**The arms of an enum under a tag.** `<tag>__<value>()` is a function of the enum's own paths, one per variant serde writes, and it answers the rows of that arm. Its name is the tag key, `__`, and the value serde writes for the variant, each in snake case. What it answers is an arm filter, an `ArmFilter<Root, M>`: a filter over the rows of the enum that holds the arm's tag from the start, and whose `and` takes a filter over the rows of `M`, the model the arm holds, and over no other. So a field of an arm is asked only together with that arm's tag, and a row of another arm that writes the same key is not matched. The function is a `const fn`, so a `const` item holds an arm filter, and each use of the `const` is a new value:

```rust
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Courier {
    pub name: String,
    pub region: String,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum Transport {
    #[serde(rename = "by-courier")]
    ByCourier(Courier),
    Collected,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Booking {
    pub depot_code: String,
    #[serde(flatten)]
    pub transport: Transport,
}

/// The bookings a courier carries, held in a `const`.
const BY_COURIER: transport_schema::ArmFilter<Transport, Courier> =
    Transport::MONGO_FIELDS.type__by_courier();

/// The bookings one courier carries out of one depot.
fn couriered(depot: &str, courier: &str) -> Result<booking_schema::Filter<Booking>, WriteError> {
    let booking = Booking::MONGO_FIELDS;
    let by_name = BY_COURIER.and(Courier::MONGO_FIELDS.name().eq(courier.to_owned())?);
    Ok(booking
        .transport()
        .nest(by_name)
        .and(booking.depot_code().eq(depot.to_owned())?))
}
```

| Written | The document it builds |
|---------|------------------------|
| `couriered("SDQ-1", "Acme Freight")?` | `{ "$and": [{ "type": { "$eq": "by-courier" } }, { "name": { "$eq": "Acme Freight" } }, { "depotCode": { "$eq": "SDQ-1" } }] }` |
| `Booking::count(&collection, couriered("SDQ-1", "Acme Freight").map_err(booking_schema::OperationError::Unwritable)?).await?` | the count of those rows |

`collection` is a `mongodb::Collection<bson::Document>` of bookings. `Booking` flattens the enum, so `nest` writes the tag and the condition among the row's own keys. Where a row holds the enum under a key, `nest` writes both under that key. With the model in another crate nothing changes: `Courier::MONGO_FIELDS` is that crate's, and `and` takes its filter by the two standard traits ([Filters and updates](#filters-and-updates)).

An arm filter is a filter over the rows of its enum. It turns into the document of its tag alone, `and` adds each condition to one `$and`, and `Filter::from` makes it the module's own `Filter`, which has `negated`:

| Written | The document it builds |
|---------|------------------------|
| `BY_COURIER` | `{ "type": { "$eq": "by-courier" } }` |
| `BY_COURIER.and(Courier::MONGO_FIELDS.name().eq("Acme".to_owned())?)` | `{ "$and": [{ "type": { "$eq": "by-courier" } }, { "name": { "$eq": "Acme" } }] }` |
| `transport_schema::Filter::from(BY_COURIER).negated()` | `{ "$nor": [{ "type": { "$eq": "by-courier" } }] }` |

The model `M` an arm is over follows from what its variant holds:

| The variant holds | `M` |
|-------------------|-----|
| One slot serde writes | The slot's type, read through `Option`, a list and the wrappers `nest` reads through: what `nest` reaches below a path over that slot. `ByCourier(Courier)` is over `Courier`, and an arm that holds a `Box<Courier>` is over `Courier` too. A map is `()` |
| Fields written in the variant, several slots, nothing, or one slot serde never writes | `()` |

The snake case of a tag key and of a value is its words in lower case, joined by one `_`. A word ends at a character that is no ASCII letter or digit, before a capital that follows a small letter or a digit, and before a capital a small letter follows where a capital comes before it: `docType` is `doc_type`, `HTTPServer` is `http_server`, `v2Beta` is `v2_beta`, and `@type` is `type`.

| The enum | The variant | Its arm function |
|----------|-------------|------------------|
| `#[serde(tag = "type")]` | `#[serde(rename = "by-courier")] ByCourier(Courier)` | `type__by_courier()` |
| `#[serde(tag = "type")]` | `Collected` | `type__collected()` |
| `#[serde(tag = "docType", content = "body", rename_all = "kebab-case")]` | `PastDue(Notice)` | `doc_type__past_due()` |

**An arm whose `M` is `()` answers its tag alone**: no model is there for its `and` to be over. A condition on fields written in the variant joins through the plain `and`, here with `payment` the paths of `Payment`:

| Written | The document it builds |
|---------|------------------------|
| `payment.kind__cash()` | `{ "kind": { "$eq": "cash" } }` |
| `paths.payment().nest(payment.kind__cash())` | `{ "payment.kind": { "$eq": "cash" } }` |
| `payment.card().last4().eq("4242".to_owned())?.and(payment.kind__card())` | `{ "$and": [{ "last4": { "$eq": "4242" } }, { "kind": { "$eq": "card" } }] }` |

**`and` on an arm filter refuses a filter over another type.** With `Wagon` a flagged struct that declares a `number`, `BY_COURIER.and(Wagon::MONGO_FIELDS.number().eq("W-9".to_owned())?)` is ``error[E0277]: the trait bound `__tixschema_mongodb::Filter<Wagon>: AsRef<PhantomData<Courier>>` is not satisfied``.

**An arm takes an update of the model it holds.** `update` on an arm filter takes an update of the rows of `M`, the model the arm holds, and of no other, and answers an arm update, an `ArmUpdate<Root, M>`: an update of the rows of the enum, with each key of the model written where the arm holds it. `and` on an arm update merges one more update of `M`, as `and` on an update does. `tag_on_insert` adds the arm's tag to `$setOnInsert`, which an upsert writes where it inserts the row. The tag is written only where it is asked for: an update of a row that is there needs none.

```rust
/// A booking's courier renamed, with the arm's tag written where an upsert inserts the row.
fn recouriered(courier: &str) -> Result<booking_schema::Update<Booking>, WriteError> {
    let renamed = BY_COURIER
        .update(Courier::MONGO_FIELDS.name().set(courier.to_owned())?)
        .tag_on_insert();
    Ok(Booking::MONGO_FIELDS.transport().nest(renamed))
}
```

| Written | The document it builds |
|---------|------------------------|
| `recouriered("Acme Freight")?` | `{ "$set": { "name": "Acme Freight" }, "$setOnInsert": { "type": "by-courier" } }` |
| `Booking::update_one(&collection, couriered("SDQ-1", "Acme")?, recouriered("Acme Freight")?).await?` | the driver's `UpdateResult` |
| `collection.update_one(couriered("SDQ-1", "Acme")?.into_document(), recouriered("Acme Freight")?.into_document()).upsert(true).await?` | the same, as an upsert: the driver's own call |

`Booking` flattens the enum, so `nest` writes the model's key and the tag among the row's own keys. Where a row holds the enum under a key, `nest` writes every key under it, the tag included. The model may be of another crate, or declared below the enum: no function of the variant takes its update there, and the arm does.

**An update carries no condition.** `update` leaves behind the conditions `and` added to the arm filter: an update selects no row, and the filter the operation takes beside it does, as `couriered` does above. No operation upserts. An upsert is the driver's own call ([A query the operations do not cover](#a-query-the-operations-do-not-cover)), which `into_document()` hands the filter and the update.

The tag is written at the enum's own level under either tagging, and what the arm holds is written under the content key where the enum has one. `Update::from` makes an arm update the module's own `Update`. `courier` and `notice` are `Courier::MONGO_FIELDS` and `Notice::MONGO_FIELDS`, and `paper` is the paths of the enum of the last row of the table of arm functions above, whose `PastDue(Notice)` holds a `Notice` that declares a `code`:

| Written | The document it builds |
|---------|------------------------|
| `BY_COURIER.update(courier.name().set("Renamed".to_owned())?)` | `{ "$set": { "name": "Renamed" } }` |
| `BY_COURIER.update(courier.name().set("Renamed".to_owned())?).and(courier.region().set_on_insert("north".to_owned())?).tag_on_insert()` | `{ "$set": { "name": "Renamed" }, "$setOnInsert": { "region": "north", "type": "by-courier" } }` |
| `paper.doc_type__past_due().update(notice.code().set("N-8".to_owned())?).tag_on_insert()` | `{ "$set": { "body.code": "N-8" }, "$setOnInsert": { "docType": "past-due" } }` |
| `transport_schema::Update::from(BY_COURIER.tag_on_insert())` | `{ "$setOnInsert": { "type": "by-courier" } }` |

**An arm whose `M` is `()` writes its tag all the same.** No update is over `()`, so `update` has nothing to take there, and `tag_on_insert` is on the arm filter itself. What it answers joins a plain update of the fields written in the variant:

| Written | The document it builds |
|---------|------------------------|
| `payment.kind__cash().tag_on_insert()` | `{ "$setOnInsert": { "kind": "cash" } }` |
| `payment.card().last4().set("4242".to_owned())?.and(payment.kind__card().tag_on_insert())` | `{ "$set": { "last4": "4242" }, "$setOnInsert": { "kind": "card" } }` |

**`update` on an arm filter takes an update of the arm's model and nothing else.** Handed a filter over the rows of that model, `BY_COURIER.update(Courier::MONGO_FIELDS.name().eq("Acme".to_owned())?)` is ``error[E0277]: the trait bound `__tixschema_mongodb::Filter<Courier>: AsRef<PhantomData<fn(Courier) -> Courier>>` is not satisfied``. Handed an update of another type, with `Wagon` a flagged struct that declares a `number`, `BY_COURIER.update(Wagon::MONGO_FIELDS.number().set("W-1".to_owned())?)` is ``error[E0277]: the trait bound `__tixschema_mongodb::Update<Wagon>: AsRef<PhantomData<fn(Courier) -> Courier>>` is not satisfied``. An arm update is no filter: it carries an update's marker and no other, so `BY_COURIER.and(BY_COURIER.tag_on_insert())` does not compile (`E0277`).

### Filters and updates

`and` and `or` join two filters over the same rows, and `negated` turns one around. `and` on an update merges two updates of the same rows: the keys of each operator are put together, and a key both write keeps the later value. `Filter::raw` and `Update::raw` take a document written by hand, for what has no typed form:

| Written | The document it builds |
|---------|------------------------|
| `paths.total().gt(1000.5)?.and(paths.paid_at().exists(false))` | `{ "$and": [{ "total": { "$gt": 1000.5 } }, { "paidAt": { "$exists": false } }] }` |
| `paths.total().gt(1000.5)?.or(paths.paid_at().exists(false))` | `{ "$or": [{ "total": { "$gt": 1000.5 } }, { "paidAt": { "$exists": false } }] }` |
| `paths.paid_at().exists(true).negated()` | `{ "$nor": [{ "paidAt": { "$exists": true } }] }` |
| `Filter::<Invoice>::raw(doc! { "details.costCenter": "CC-7" })` | `{ "details.costCenter": "CC-7" }` |
| `paths.total().set(99.5)?.and(paths.paid_at().unset())` | `{ "$set": { "total": 99.5 }, "$unset": { "paidAt": "" } }` |
| `Update::<Invoice>::raw(doc! { "$inc": { "total": 12.5 } })` | `{ "$inc": { "total": 12.5 } }` |

- **`nest` takes a filter or an update over the nested model, whichever module it comes from, and answers one of the module it was called in.** `address.city().eq(..)` is an `address_schema::Filter<Address>`. `customer.address().nest(..)` makes it a `customer_schema::Filter<Customer>`, and `paths.customer().nest(..)` an `invoice_schema::Filter<Invoice>`, which `to_chase` above joins with `and`. An update goes the same way, and `nest` tells the two apart by what it is handed: nothing is imported for it.
- **`nest` joins two keys with a dot, and leaves an empty one out with its dot.** `customer` and `address.city` are `customer.address.city`. A model serde flattens into the type that holds it has no key there, so its keys are written as they are. The one path of a type serde writes as one value is the empty key, so under `status` it is `status`.
- **A filter is nested whole.** Each of its keys is written under the path's, inside `$and`, `$or` and `$nor` too. Any other key that starts with `$`, which a `Filter::raw` document may write at its top, is kept as it is written. In an update, each key of each operator is written under the path's, and its value is kept.
- **A filter joins a filter over the same rows, whichever module it comes from.** What `and` and `or` ask of their argument is that it turn into a `bson::Document` and carry the row type, which every module's `Filter` does through two standard traits, `Into<bson::Document>` and `AsRef<PhantomData<Root>>`. `nest` asks the same of what it is handed, over the nested model. The answer is of the type `and` was called on.
- **An arm filter goes where a filter over its enum goes.** A `transport_schema::ArmFilter<Transport, Courier>` turns into a `bson::Document` and carries the row type `Transport` through the same two traits, so `nest` on a path that holds a `Transport` takes one, and so do `and`, `or`, `elem_match` on a list of the enum, and every operation. It is no `Filter` itself: `transport_schema::Filter::from(arm)` makes it the module's own `Filter<Transport>`, which starts a join and has `negated`. Its own `and` asks the two traits of its argument over the model the arm holds ([The arms of an enum under a tag](#operators)).
- **An arm update goes where an update of its enum goes.** A `transport_schema::ArmUpdate<Transport, Courier>` turns into a `bson::Document` and carries `AsRef<PhantomData<fn(Transport) -> Transport>>`, an update's marker over the rows of the enum, and no other. So `nest` on a path that holds a `Transport` takes one, and so do `and` on an update of the enum, `update_one` and `update_many`, and nothing that asks for a filter does. `transport_schema::Update::from(arm)` makes it the module's own `Update<Transport>`. Its own `and`, and `update` on the arm filter, ask the two traits of their argument over the model the arm holds ([The arms of an enum under a tag](#operators)).
- **A filter over another type's rows is refused.** Handed `Order::MONGO_FIELDS.placed().eq(true)?`, `and` on a filter over `Invoice` is ``error[E0277]: the trait bound `__tixschema_mongodb::Filter<Order>: AsRef<PhantomData<Invoice>>` is not satisfied``, and `nest` on `paths.customer()`, which holds a `Customer`, is ``error[E0277]: the trait bound `__tixschema_mongodb::Filter<Order>: Nests<Invoice, Customer, _>` is not satisfied``.
- **An update is refused where a filter is asked, and a filter where an update is.** An update carries `AsRef<PhantomData<fn(Root) -> Root>>` in place of a filter's marker. `paths.total().gt(1000.0)?.and(paths.total().set(0.0)?)` is ``error[E0277]: the trait bound `__tixschema_mongodb::Update<Invoice>: AsRef<PhantomData<Invoice>>` is not satisfied``, and `paths.total().set(0.0)?.and(paths.total().gt(1000.0)?)` is ``error[E0277]: the trait bound `__tixschema_mongodb::Filter<Invoice>: AsRef<PhantomData<fn(Invoice) -> Invoice>>` is not satisfied``.
- **A document is not a filter until `Filter::raw` makes it one.** `and` handed a `doc! { .. }` as it stands is ``error[E0277]: the trait bound `bson::Document: AsRef<PhantomData<Invoice>>` is not satisfied``. `raw` gives the document the row type of what it joins, and nothing checks what it holds: its keys and its values are the caller's.
- **`into_document()` hands over the document** of a filter or an update, and so does `bson::Document::from`, for a call made on the driver itself.

A map has no path, its keys being data, and an operator the typed paths do not write has no method. Both are written by hand and joined to the typed ones:

```rust
/// Invoices of one cost center that are no draft. `details` is a map: its keys are data.
fn of_cost_center(code: &str) -> Result<Filter<Invoice>, WriteError> {
    let paths = Invoice::MONGO_FIELDS;
    Ok(paths
        .status()
        .ne(InvoiceStatus::Draft)?
        .and(Filter::raw(doc! { "details.costCenter": code })))
}

/// A surcharge: `$inc`, an operator the typed paths do not write, beside one they do.
fn surcharged(by: f64) -> Result<Update<Invoice>, WriteError> {
    let paths = Invoice::MONGO_FIELDS;
    Ok(paths
        .tags()
        .push("surcharged".to_owned())?
        .and(Update::raw(doc! { "$inc": { "total": by } })))
}
```

| Written | The document it builds |
|---------|------------------------|
| `of_cost_center("CC-7")?` | `{ "$and": [{ "status": { "$ne": "draft" } }, { "details.costCenter": "CC-7" }] }` |
| `surcharged(12.5)?` | `{ "$push": { "tags": "surcharged" }, "$inc": { "total": 12.5 } }` |

### What each field becomes

A function is there for every key serde writes, and the kind it answers follows from how serde writes the field:

| The field | What its function answers |
|-----------|---------------------------|
| A plain value `T` | `Field<Root, T>` |
| `Option<T>` | `OptionalField<Root, T>` |
| `Vec<T>`, `VecDeque<T>`, `HashSet<T>`, `BTreeSet<T>`, `BinaryHeap<T>`, `LinkedList<T>`, `[T; N]`, `[T]`, and `IndexSet<T>` under the `indexmap` feature | `ListField<Root, T>` |
| A struct or an enum `M` declared with `#[model_schema(enable = ["decode_with", "mongodb"])]` above the type, which serde writes under keys | `Model<Root, M>`, whose `nest` reaches `M`'s own paths |
| `Option<M>` | `OptionalModel<Root, M>` |
| A list of `M` | `ModelList<Root, M>` |
| A flagged type serde writes as one value: a brand, a plain enum, a unit struct | `Field<Root, M>`, and `OptionalField<Root, M>` in an `Option`. A list of one is a `ListField<Root, M>` |
| `Box`, `Rc`, `Arc`, `Cow`, `Cell`, `Mutex`, `RefCell` or `RwLock` around any of these | The kind of what it holds, with a third parameter: `Model<Root, Box<Customer>, Customer>`. An operator over the whole value takes the type as written, `Box<Customer>`, and `nest` a filter over what it holds, `Customer` |
| `#[serde(flatten)]` of `M`, or of `Option<M>` | `Nested<Root, M>` at the empty key: `nest` writes `M`'s keys as they are, among those of the type that flattens it |
| A flattened field of any other type | No function: it has no key of its own |
| A map: bare, in an `Option`, behind one of those wrappers, flattened | No function |
| A type tixschema has not seen as a flagged model: another crate's, one declared below, a `type` alias, the type itself | One whole value: `Field`, `OptionalField` or `ListField` over it, whose `nest` reaches its paths |
| A member typed with one of the type's own parameters | One whole value of what fills it |
| `Option<Vec<T>>` | `OptionalField<Root, Vec<T>>`: one whole list a row may leave out |
| A list of lists | `ListField<Root, Vec<T>>`: each element is one whole list |
| A tuple | `Field<Root, (A, B)>`: one whole value |
| A std range: `Range<T>`, `RangeInclusive<T>`, `RangeFrom<T>`, `RangeTo<T>` | `Field<Root, Range<T>>`: one whole value |
| A std `Result<T, E>` | `Field<Root, Result<T, E>>`: one whole value |
| A field serde never writes, under `skip` or `skip_serializing` | No function |
| A field with a hook that writes it: `with`, `serialize_with`, the one `as_number` adds | `Field`, or `OptionalField` for an `Option`, whose values are written through the hook |
| A field serde writes and never reads back, under `skip_deserializing`, or reads through `deserialize_with` alone | One whole value, of the kind its `Option` or its list is |

What a flagged type's own paths are follows from the shape serde writes the type in:

| The flagged type | Its paths |
|------------------|-----------|
| A struct with named fields | One function per field, under the field's Rust name |
| A tuple struct with several slots | A tuple of paths, each slot's key its position: with `Point` a `struct Point(f64, f64)`, `Point::MONGO_FIELDS.0` writes the key `0`, and nested under a field `origin`, `origin.0`. A slot serde never writes keeps its place as `()` |
| An internally tagged enum | `is_<variant>()` is `$eq` on the tag's key, and `<tag>__<value>()` answers the arm filter of each variant. A variant's fields sit beside the tag: a function named after the variant answers a struct with one function per field. A variant that holds one flagged model answers a `Nested` at the empty key |
| An adjacently tagged enum | The same `is_<variant>()` and `<tag>__<value>()`. What a variant holds is under the content key: its fields, one value, or a tuple of positions, and each condition of its arm filter |
| An externally tagged enum | What a variant holds is under the name serde writes the variant as. `is_<variant>()` is `$exists` on that key, and `$eq` with the name for a variant that holds nothing |
| An untagged enum | One function per variant that holds a value, its paths at the enum's own level. No `is_<variant>()`: nothing stored names the variant |
| A brand, a single-slot tuple struct, a `#[serde(transparent)]` struct, a plain enum, a unit struct | One path at the empty key, which `MONGO_FIELDS` dereferences to, with the operators of a `Field`, and `regex` where the one value is a `String` |

- **A type serde writes as one value is one path.** `InvoiceStatus::MONGO_FIELDS.eq(InvoiceStatus::Paid)?` is a filter over an `InvoiceStatus`, at the empty key, which `nest` writes at the key that holds one. Held by another type the value is a plain one, a `Field` or an `OptionalField`, so every comparison is a plain call: `paths.status().eq(InvoiceStatus::PastDue)`. `regex` on one written as text is asked of the type's own path: with `callsign` a field typed with `Callsign`, a brand over a `String`, `paths.callsign().nest(Callsign::MONGO_FIELDS.regex("^mav", "i"))`. A list of one is a `ListField`, a list of plain values matched through `contains` and `element()`. A filter over the rows of such a type has no key to write its operator under, so `elem_match` on that list takes an `Element` and refuses one (``error[E0308]: `?` operator has incompatible types``).
- **A path is under the name serde writes.** `rename` on a field, `rename_all` on the type, `rename` and `rename_all` on a variant and `rename_all_fields` on an enum all count. A renaming written as a list counts by its `serialize` side, `rename(serialize = "ref", deserialize = "ref")` being the key `ref`. A list whose two sides name two keys is refused by every build that describes the type, a mobile client included; a build with no schema surface and no client takes it, and the path is the key that is written.
- **A function of an enum is named after its variant, in snake case**: a variant `OverLand` is the function `over_land()` and the method `is_over_land()`. A variant serde never writes has neither.
- **`is_<variant>()` keeps its name.** With variants `Card` and `IsCard`, `is_card()` asks for `Card` and `is_is_card()` for `IsCard`, and what `IsCard` holds is reached through `is_card2()`: the function of a variant that meets a taken name is numbered, as those of two variants with one snake-case name are.
- **An arm function is named `<tag>__<value>()`**: the tag key and the value serde writes for the variant, each in snake case, with `__` between them. An enum serde writes under a tag key has one per variant serde writes, whatever the variant holds. An externally tagged, an untagged and a plain enum have none, since they store no tag key. Beside the functions, the schema module of the enum declares `Arm`, an enum with one unit arm per variant that has a function: `transport_schema::Arm::ByCourier`.
- **No two functions of an enum's paths take one name.** `__` is in the name of an arm function and of no other, and what would break that is refused where the `enable` list is read, whether the enum lists `"mongodb"` or not, with the `mongodb` feature on and off. A variant whose Rust name holds `__` is ``model_schema: type `Transport`: variant `By__Courier` holds `__` in its name. Under a tag and `decode_with`, `__` parts the tag from the value in the function of each arm, `type__<value>()`. Rename the variant``. Two values that read as one in snake case are ``model_schema: type `Transport`: variants `ByCourier` and `Couriered` are written `by-courier` and `by_courier` under the tag `type`, and both read `by_courier` in snake case, so `type__by_courier()` would answer for both. Rename one of the two values``. An arm function that meets an `is_<variant>()`, which takes a tag key that reads `is` and a variant whose name starts with `_`, is ``model_schema: type `Transport`: the arm function `is__parcel()` of variant `Parcel` and `is__parcel()`, which asks for variant `_Parcel`, would be one function. Rename the variant `_Parcel` ``. An enum that lists `"decode_with"` alone has no arm function and is refused all the same, and an enum with no `enable` list is refused none of the three.
- **A generic type has its paths at each filling**: with `Wrapper<T>` declaring `inner: T` and `label: String`, `Wrapper::<u32>::MONGO_FIELDS.inner()` is a `Field<Wrapper<u32>, u32>`. The arms of a generic enum are at each filling too.

**A field with a serde hook is compared the way it is stored.** A `chrono` date writes text, so a date kept as a BSON date carries the `bson` library's hook, and a filter on it has to write a BSON date too. The path writes its values through the field's own hook, here bound as `date_hook`:

```rust
// version 2 of the `bson` library
use bson::serde_helpers::chrono_datetime_as_bson_datetime as date_hook;
// version 3
use bson::serde_helpers::datetime::FromChrono04DateTime as date_hook;
```

```rust
use window_schema::{Filter, WriteError};

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Window {
    #[serde(with = "date_hook")]
    pub closes_at: DateTime<Utc>,
    pub opens_at: DateTime<Utc>,
}

/// Windows that close before an instant, and windows that open before it.
fn closing_before(instant: DateTime<Utc>) -> Result<[Filter<Window>; 2], WriteError> {
    let paths = Window::MONGO_FIELDS;
    Ok([
        paths.closes_at().lt(instant)?,
        paths.opens_at().lt(instant)?,
    ])
}
```

For the instant `2026-10-07T17:40:00Z`, the two filters, the hooked date as a BSON date and the plain one as text:

```text
{ "closesAt": { "$lt": DateTime("2026-10-07 17:40:00.0 +00:00:00") } }
{ "opensAt": { "$lt": "2026-10-07T17:40:00Z" } }
```

Each hook lives behind the `bson` library's `chrono-0_4` feature. The path of the attribute is called as it is written, type arguments included, so a hook written `::serde_with::As::<Option<FromChrono04DateTime>>` on an `Option` of a date is the one its path writes through. A hook on a field that holds a list writes the whole list, so that field is one whole value, a `Field<Root, Vec<T>>`.

### A type tixschema has not seen

A field typed with a flagged model, one that lists `"mongodb"` or `"decode_with"` alone, is a `Model`, an `OptionalModel` or a `ModelList` only where tixschema has already expanded the model: a struct or an enum `#[model_schema]` is written on **above** the type that holds it, in the same crate. A type of another crate, one declared below, a plain `type` alias and the type itself are held as one whole value, a `Field`, an `OptionalField` or a `ListField` over the type, with the operators of that kind over the whole value. This is the reason every model of the section's examples is declared above `Invoice`.

`nest` reaches the paths below such a member as it does below a model: a `Field` and a `ListField` have it too.

```rust
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Shipment {
    pub carrier: Carrier,
    pub code: String,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Carrier {
    pub name: String,
}

/// Shipments of one carrier. `Carrier` is declared below `Shipment`, so `carrier` is one whole
/// value there, and `nest` reaches the paths below it as it does below any model.
fn carried_by(name: &str) -> Result<shipment_schema::Filter<Shipment>, WriteError> {
    let shipment = Shipment::MONGO_FIELDS;
    let named = Carrier::MONGO_FIELDS.name().eq(name.to_owned())?;
    Ok(shipment
        .code()
        .regex("^S-", "")
        .and(shipment.carrier().nest(named)))
}
```

| Written | The document it builds |
|---------|------------------------|
| `carried_by("Acme Freight")?` | `{ "$and": [{ "code": { "$regex": "^S-", "$options": "" } }, { "carrier.name": { "$eq": "Acme Freight" } }] }` |

`Shipment::MONGO_FIELDS.carrier()` is a `Field<Shipment, Carrier>` there. `named` is a `carrier_schema::Filter<Carrier>`, and what `nest` answers is a `shipment_schema::Filter<Shipment>` under the key `carrier`.

- **A kind that wraps a model names the model as a third parameter.** `parent: Option<Box<Category>>` on `Category` is an `OptionalField<Category, Box<Category>, Category>`: `set` takes a `Box<Category>`, and `nest` a filter over the rows of `Category`. Where the field wraps nothing the third parameter is the second, so `Field<Invoice, f64>` names the kind whole.
- **`MongoPath` is one key**, as the type that declares the path writes it: `MongoPath::at("total")`, and `MongoPath::ROOT` for the empty key. `key()` reads it. A key holds a dot where the type's own shape puts two keys, `list.0` being the first slot of a tuple variant under the content key `list`. The module's `Path` is another type: where an issue of the recovering decode sits.
- **A path can be built by hand** through the constructors the generated ones are built with, `Field::plain(MongoPath::at("total"))` and its twins on `OptionalField` and `ListField`. Such a path writes whatever key it is given: nothing checks it against the type.

### The operations

Each operation is a function on a type that lists `"mongodb"`, over a `mongodb::Collection<bson::Document>`. The collection is one of documents, never of the type: a row is read through the recovering decode and written through the type's own `Serialize`, not by the driver's serde.

| Operation | What it sends | What it answers |
|-----------|---------------|-----------------|
| `Invoice::find_one(collection, filter)` | The driver's `find_one` | `Option<Invoice>`: the first row the filter matches, read as the type, and `None` where it matches none |
| `Invoice::find_one_with(collection, filter, resolvers)` | The same | The same, with the resolvers run over the row's issues |
| `Invoice::find(collection, filter)` | The driver's `find` | `Vec<Invoice>`: every row the filter matches |
| `Invoice::find_with(collection, filter, resolvers)` | The same | The same, with the resolvers run over each row's issues |
| `Invoice::count(collection, filter)` | The driver's `count_documents` | `u64`: how many rows the filter matches. No row is read |
| `invoice.insert_one(collection)` | The driver's `insert_one`, with the value written as a document | The driver's `mongodb::results::InsertOneResult` |
| `Invoice::update_one(collection, filter, update)` | The driver's `update_one` | The driver's `mongodb::results::UpdateResult` |
| `Invoice::update_many(collection, filter, update)` | The driver's `update_many` | The same |
| `Invoice::delete_one(collection, filter)` | The driver's `delete_one` | The driver's `mongodb::results::DeleteResult` |
| `Invoice::delete_many(collection, filter)` | The driver's `delete_many` | The same |

Every one of them fails with the crate's one [`OperationError`](#operationerror). Each bound to what it answers, in a function that compiles:

```rust
use bson::{Bson, Document, doc};
use mongodb::Collection;
use mongodb::options::{Collation, Hint};
use mongodb::results::{DeleteResult, InsertOneResult, UpdateResult};
use tokio::task::JoinHandle;

use invoice_schema::{Expected, Filter, Issue, OperationError, Read, Resolution, Resolver};

/// Every operation, each bound to what it answers.
async fn every_operation(
    invoices: &Collection<Document>,
    invoice: &Invoice,
) -> Result<(), OperationError> {
    let paths = Invoice::MONGO_FIELDS;
    let drafts = || {
        paths
            .status()
            .eq(InvoiceStatus::Draft)
            .map_err(OperationError::Unwritable)
    };
    let voided = || {
        paths
            .total()
            .set(0.0_f64)
            .map_err(OperationError::Unwritable)
    };
    let resolvers: [Resolver<'_, Document, Bson>; 1] = [&whole_number_as_text];

    let _stored: InsertOneResult = invoice.insert_one(invoices).await?;
    let _first: Option<Invoice> = Invoice::find_one(invoices, drafts()?).await?;
    let _repaired: Option<Invoice> =
        Invoice::find_one_with(invoices, drafts()?, &resolvers).await?;
    let _every: Vec<Invoice> = Invoice::find(invoices, drafts()?).await?;
    let _every_repaired: Vec<Invoice> = Invoice::find_with(invoices, drafts()?, &resolvers).await?;
    let _how_many: u64 = Invoice::count(invoices, drafts()?).await?;
    let _changed: UpdateResult = Invoice::update_one(invoices, drafts()?, voided()?).await?;
    let _all_changed: UpdateResult = Invoice::update_many(invoices, drafts()?, voided()?).await?;
    let _deleted: DeleteResult = Invoice::delete_one(invoices, drafts()?).await?;
    let _all_deleted: DeleteResult = Invoice::delete_many(invoices, drafts()?).await?;
    Ok(())
}
```

`whole_number_as_text` is the resolver of [Resolvers](#resolvers), written there against another `Invoice` and here against this one: the same text, over `invoice_schema::Issue`.

- **A filter is any filter over the type's rows**, whichever module's `Filter` it is. A filter over another type's rows and an update are refused, by the bound a filter joins another under.
- **A filter and an update cannot change places.** `Invoice::update_one(invoices, paths.status().set(InvoiceStatus::Paid).map_err(OperationError::Unwritable)?, paths.number().eq("INV-0042".to_owned()).map_err(OperationError::Unwritable)?)` does not compile: ``error[E0277]: the trait bound `__tixschema_mongodb::Update<Invoice>: AsRef<PhantomData<Invoice>>` is not satisfied`` for the update handed as the filter, and ``error[E0277]: the trait bound `__tixschema_mongodb::Filter<Invoice>: AsRef<PhantomData<fn(Invoice) -> Invoice>>` is not satisfied`` for the filter handed as the update.
- **A read of many rows fails as a whole.** The rows are read one at a time as the driver hands them over, and the first that does not read ends the read with that row's `_id` and its issues. No row is answered.
- **`insert_one` writes the row when it is called**, before anything is awaited. A value BSON cannot hold, and a type serde writes as anything but a document, a plain enum or a brand, answer `OperationError::Unwritable` without MongoDB being asked anything. What is awaited holds no borrow of the value.
- **The updates and the deletes answer what the driver answers**: `UpdateResult` with its `matched_count`, `modified_count` and `upserted_id`, `DeleteResult` with its `deleted_count`, and `InsertOneResult` with the `inserted_id` of the row.
- **Every operation can be moved to another thread.** What each answers is `Send`, the resolvers of a read being `Sync`. A read of many rows asks that the row type be `Send`, the rows read so far being held while the next is awaited.

### Read options

`find_one`, `find`, `count` and the two that take resolvers do not answer a future. They answer a `<type>_schema::Read<'c, T, O>`, a read that asks nothing of MongoDB until it is awaited, and that takes the options it is to be run under first. `T` is the row type, and `O` what the read answers: `Vec<T>`, `Option<T>` or `u64`.

```rust
/// The largest unpaid invoices of one customer, largest first, and how many it has unpaid.
async fn largest_unpaid(
    invoices: &Collection<Document>,
    customer: &str,
    at_most: i64,
) -> Result<(Vec<Invoice>, u64), OperationError> {
    let paths = Invoice::MONGO_FIELDS;
    let unpaid = || {
        Customer::MONGO_FIELDS
            .name()
            .eq(customer.to_owned())
            .map(|named| paths.customer().nest(named))
            .map(|of_customer| of_customer.and(paths.paid_at().exists(false)))
            .map_err(OperationError::Unwritable)
    };
    let largest = Invoice::find_with(invoices, unpaid()?, &[&whole_number_as_text])
        .sort(doc! { "total": -1_i32 })
        .limit(at_most)
        .await?;
    let how_many = Invoice::count(invoices, unpaid()?).await?;
    Ok((largest, how_many))
}

/// One page of invoices in the order of their numbers: a read under every option it takes,
/// which asks nothing of MongoDB until it is awaited.
fn page(invoices: &Collection<Document>, at: u64) -> Read<'_, Invoice, Vec<Invoice>> {
    Invoice::find(invoices, Filter::raw(doc! {}))
        .sort(doc! { "number": 1_i32 })
        .skip(at * 20)
        .limit(20)
        .hint(Hint::Name("number_1".to_owned()))
        .collation(Collation::builder().locale("en").build())
}
```

| Option | Takes | Sent with |
|--------|-------|-----------|
| `sort` | A `bson::Document`, each key with `1` or `-1` | `find` and `find_one` |
| `limit` | An `i64`, as the driver's `find` takes it: `0` is every row | `find`, as given. A count is held to the same number: it is sent the limit's absolute value, and none for `0` |
| `skip` | A `u64`, the rows to pass over | `find`, `find_one` and `count` |
| `hint` | A `mongodb::options::Hint`, the index to use | `find`, `find_one` and `count` |
| `collation` | A `mongodb::options::Collation` | `find`, `find_one` and `count` |

- **An option the driver's own method does not have is not sent**: `limit` on a read of one row, which answers the first row whatever the limit, and `sort` on a count, which is the same in any order.
- **A sort is a document written by hand**, its keys the ones serde writes. There is no projection: a read answers the whole type.
- **What `unpaid` hands `nest` in `largest_unpaid` is a `customer_schema::Filter<Customer>`**, a filter over a customer. What `nest` answers is an `invoice_schema::Filter<Invoice>`, which `find_with` and `count` take.
- **An option is handed to the driver as it is given.** What a collation folds, and which hint MongoDB takes or refuses, are MongoDB's own to answer.
- **A read is `#[must_use]`.** One that is never awaited asks nothing, and the compiler says so: ``unused `__tixschema_mongodb::Read` that must be used``, with the note `a read asks nothing of MongoDB until it is awaited`.
- **A read is turned into a future by `.await`**, and is not one before. It implements `IntoFuture`, not `Future`, so something that takes a future is handed `.into_future()`, and a read handed over as it stands is ``error[E0277]: `__tixschema_mongodb::Read<'_, Invoice, Vec<Invoice>>` is not a future``:

```rust
/// A count handed to something that takes a future, which a read is not until it is turned
/// into one.
fn counted_elsewhere(
    invoices: &'static Collection<Document>,
) -> JoinHandle<Result<u64, OperationError>> {
    tokio::spawn(Invoice::count(invoices, Filter::raw(doc! {})).into_future())
}
```

### Rows that do not read as the type

`find_one` and `find` take no resolver, so any issue in a row refuses it: a key the type does not declare, a value held in another form, a value serde refuses, and, in a build with a schema surface on, a value that breaks a bound its field declares. `find_one_with` and `find_with` take the resolvers that repair such a row, and run them over each row's issues as [`from_bson_piped`](#resolvers) does, once.

A row older writers left, an invoice whose customer holds `"openInvoices": "3"`, is refused by the plain read with `OperationError::Unreadable`. Under version 2 of the `bson` library it displays as:

```text
the row ObjectId("6a7cc592ca0574e6efdfe217") does not read as expected: customer.openInvoices: invalid: expected U32, found String("3"): invalid type: string "3", expected u32
```

and under version 3, whose own wording the end of the line is:

```text
the row ObjectId("6a7cc592ca0574e6efdfe217") does not read as expected: customer.openInvoices: invalid: expected U32, found String("3"): BSON error. Kind: A deserialization-related error occurred. Message: invalid type: string "3", expected u32.
```

Read with `&[&whole_number_as_text]`, the same row reads.

**A type that declares no `_id` is refused on every stored row.** MongoDB stores every row under an `_id`, and to a type that does not declare one it is a key like any other, `Unknown` at `_id`. `Customer` declares none. A stored customer read by `Customer::find_one` fails with:

```text
the row ObjectId("6a7cc592ca0574e6efdfe217") does not read as expected: _id: unknown: found ObjectId("6a7cc592ca0574e6efdfe217")
```

A type whose rows are stored declares the `_id`, as `Invoice` does with `#[serde(rename = "_id")]`, or reads through a resolver that settles the key by taking it out:

```rust
/// The `_id` MongoDB stores every row under, which `Customer` does not declare: taken out.
fn id_dropped(
    raw: &mut Document,
    issue: &customer_schema::Issue<Bson>,
) -> customer_schema::Resolution {
    use customer_schema::{Issue, Resolution};

    let Issue::Unknown {
        path,
        found: _found,
    } = issue
    else {
        return Resolution::NotTouched;
    };
    if path.to_string() == "_id" && path.remove_from_document(raw) {
        Resolution::Settled
    } else {
        Resolution::NotTouched
    }
}

/// A customer by name, from a collection whose rows are customers.
async fn customer_named(
    customers: &Collection<Document>,
    name: &str,
) -> Result<Option<Customer>, customer_schema::OperationError> {
    let named = Customer::MONGO_FIELDS
        .name()
        .eq(name.to_owned())
        .map_err(customer_schema::OperationError::Unwritable)?;
    Customer::find_one_with(customers, named, &[&id_dropped]).await
}
```

With that resolver the same row reads as:

```text
Customer { address: Address { city: "Moca", postal_code: "56000" }, name: "Acme", open_invoices: 3 }
```

`insert_one` on such a type sends a row with no `_id`. MongoDB stores every row under one, so the stored row is one this type's own `find_one` refuses.

### `OperationError`

Every operation fails with `OperationError`, which the crate's MongoDB [support library](#the-support-libraries) declares once and the `<type>_schema` module of every type that lists `"mongodb"` re-exports:

| Member | When | It displays as |
|--------|------|----------------|
| `Database(mongodb::error::Error)` | MongoDB refused the operation, or could not be reached | `MongoDB refused the operation or could not be reached: `, then the driver's own error |
| `Unreadable { row, issues }` | A row did not read as the type | `the row `, the row's `_id`, ` does not read as expected: `, then one line per issue |
| `Unwritable(WriteError)` | A value of a filter, an update or an insert could not be written as BSON | `a value could not be written as BSON: `, then the `bson` library's own error |

- **`row` is the row's `_id` as `bson::Bson` displays it**, which keeps its BSON type in the text, so that the text `3` and the number `3` are told apart:

| The row's `_id` | `row` |
|-----------------|-------|
| an `ObjectId` | `ObjectId("6a7cc592ca0574e6efdfe217")` |
| the text `abc` | `"abc"` |
| the text `3` | `"3"` |
| the number `3` | `3` |
| no `_id` | `without an _id` |

- **`issues` holds the issues no resolver settled**, as the crate's `Issue<bson::Bson>`, in the order they were found. The line of a value serde refuses ends in the `bson` library's own wording, which differs between its major versions, and the line of a value that breaks a bound ends in the bound's own sentence, the same under both: match on the member, the path, `expected` and `found`.
- **`?` converts neither error into it.** A filter or an update the `bson` library could not write is turned by name, `.map_err(OperationError::Unwritable)`, and a call made on the driver itself by `.map_err(OperationError::Database)`, as the examples of this section do. With `?` alone, either is ``error[E0277]: `?` couldn't convert the error to `OperationError` ``.
- **It is an error like any other**: `Debug`, `Display`, `std::error::Error`, `Send`, `Sync` and `'static`. `source()` is the driver's error for `Database` and the `bson` library's for `Unwritable`, and `Unreadable` has none. It is `#[non_exhaustive]`, as each type the flag adds is.

### What a row stores

A row is written by the type's own `Serialize`, and a value of a filter or an update the same way, so what a filter writes is what the field stores. Two cases stand apart: a member whose serde form depends on the format, whose type reads and writes as JSON as before, and a map whose key is not text, whose type does not build.

**A member whose serde form depends on the format has two stored forms.** The generated reads and writes go through `bson::Serializer::new()` and `bson::Deserializer::new(..)`, which call themselves human-readable. A typed `Collection<T>` of the driver writes through the `bson` library's raw serializer, which does not. `std::net::IpAddr` is such a member: no schema surface describes it, so the example is one of a build with tixschema's `mongo` features alone.

```rust
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Host {
    pub address: IpAddr,
}
```

`insert_one` stores a host at `127.0.0.1` as:

```text
{ "address": "127.0.0.1" }
```

The same value written through a typed `Collection<Host>` is stored as `{ "address": { "V4": [127, 0, 0, 1] } }`, and a row in that form is refused by the generated read, as an `Invalid` at `address`. Under version 2 of the `bson` library, and then under version 3:

```text
the row without an _id does not read as expected: address: invalid: expected Unknown, found Document({"V4": Array([Int32(127), Int32(0), Int32(0), Int32(1)])}): invalid type: map, expected IP address
the row without an _id does not read as expected: address: invalid: expected Unknown, found Document({"V4": Array([Int32(127), Int32(0), Int32(0), Int32(1)])}): BSON error. Kind: A deserialization-related error occurred. Message: invalid type: map, expected IP address.
```

`ObjectId`, `bson::DateTime` and `bson::Uuid` are stored in one form by both serializers.

**A map whose key is not text does not build.** tixschema takes a map keyed by a number or a `bool`, because serde writes such a key as text into JSON, and into JSON only. BSON document keys are always text: the `bson` library refuses to write such a key, and hands a stored one back to the key's type as text, which the type refuses. So a type that lists `"mongodb"` and holds such a map is refused where it is declared, in every build:

```rust
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Roster {
    pub by_number: HashMap<u32, String>,
}
```

```text
error: model_schema: type `Roster`: field `by_number`: a map keyed by `u32` cannot be stored in MongoDB — BSON document keys are always text, and the `bson` library refuses to write this key and to read it back. Key the map by a `String`, or do not list "mongodb" in `enable`.
```

- **The keys refused** are the integer types, `f32`, `f64` and `bool`, and a brand or an alias declared above over one of them, each also under `Box`, `Rc`, `Arc` and `Cow`. A generic brand or alias declared above whose value is its own type parameter, `#[serde(transparent)] pub struct Ordinal<IdType>(pub IdType);` or `pub type Slot<T> = T;`, is refused where that parameter is filled at one of them: `Ordinal<u32>` and `Slot<bool>` are refused, and `Ordinal<String>` is not. So is one whose value reaches its parameter under `Box`, `Rc`, `Arc` or `Cow`, `#[serde(transparent)] pub struct Boxed<T>(pub Box<T>);` or `pub type Held<T> = Box<T>;`, or through such a brand or alias declared above it, `#[serde(transparent)] pub struct Rank<T>(pub Ordinal<T>);`: `Boxed<u32>`, `Held<u32>` and `Rank<u32>` are refused, and `Rank<String>` is not. A brand reached through one declared below it is not refused. The error names the field and the key as written, once for each such map the field holds.
- **The map is found in every member serde writes or reads**, of a struct and of each variant of an enum, behind an `Option`, a list, a tuple, a wrapper, another map's value, either arm of a `Result` and the bound of a range: a field written `Option<Vec<BTreeMap<bool, u32>>>` or `Result<HashMap<u32, String>, String>` is refused as `by_number` is. A type of the crate's own declared above under the name `Result`, `Range`, `RangeInclusive`, `RangeFrom` or `RangeTo` is read as the model it is, and the std type written in full under `std` or `core`, `std::result::Result` or `core::ops::Range`, is read as the std type. A member under `#[serde(skip)]` is not, and neither is a member or a variant written and read by functions of its author's, under `#[serde(with = "...")]` or under both `serialize_with` and `deserialize_with`: what those functions store is theirs to answer for. A member or a variant under one of the two alone is refused, serde still writing or reading the map itself.
- **A model or an alias declared above is looked into.** A member that holds a `#[model_schema]` struct, enum or alias declared above the type, behind the same `Option`, list, tuple, wrapper, map's value, arm of a `Result` and bound of a range, is refused where that model or alias holds such a map, at any depth and whatever the model's own `enable` list. The error names the member, the model or the alias, and the first such map it holds. The members and the variants the bullet above reads past are read past here too, in the type and in the model. A model or an alias declared below the type, or of another crate, is not looked into.
- **A model or an alias declared above is read at the arguments it is held with.** A model that keys a map by one of its own type parameters, `pub struct Keyed<KeyType> { pub by_key: HashMap<KeyType, String> }`, is refused where that parameter is filled at a refused key: a member `keyed: Keyed<u32>` is refused, and `Keyed<String>` is not. An alias that keys a map by one of its own type parameters, `pub type ByKey<KeyType> = HashMap<KeyType, String>;`, is refused the same way: a member `by_key: ByKey<u32>` is refused, and `ByKey<String>` is not. A model or an alias that hands its parameter on to such a model is refused the same way, at what fills it: `pub type Rekeyed<K> = Keyed<K>;` held as `Rekeyed<u32>` is refused. A map an argument holds is refused as one the member holds itself: `Wrapper<HashMap<u32, String>>` is refused, and `Wrapper<HashMap<String, String>>` is not. The error names the member, the model or the alias as written, and the map:

  ```text
  error: model_schema: type `Arena`: field `keyed`: `Keyed<u32>` holds a map that cannot be stored in MongoDB (field `by_key`, a map keyed by `u32`) — BSON document keys are always text, and the `bson` library refuses to write this key and to read it back. Key that map by a `String`, or do not list "mongodb" in `enable`.
  ```

  Every argument is read as one the model writes into its row: a map an argument holds is refused even where the model holds that argument only in a `PhantomData`, or only in a member serde neither writes nor reads. A model or an alias declared below the type, or of another crate, is not read at its arguments.
- **A type that lists `"mongodb"` and keys a map by its own type parameter builds, whatever fills the parameter.** The filling is written where the type is used, which no expansion sees, so nothing is refused:

  ```rust
  #[model_schema(enable = ["decode_with", "mongodb"], default_types(KeyType = String))]
  #[derive(Debug, Deserialize, Serialize)]
  pub struct Keyed<KeyType: Hash + Eq> {
      pub by_key: HashMap<KeyType, String>,
  }
  ```

  `Keyed<String>` is stored and read back: a value that maps `"8"` to `"eight"` is stored as `{ "by_key": { "8": "eight" } }`. `Keyed<u32>` builds as well, and so does a filling at any other integer type or at `bool`, and each fails at run time: every write of a value whose map holds an entry, `insert_one` among them, and every read of a row whose map holds one, `find_one` and `find` among them. A value whose map is empty is stored and read back. Under version 2 of the `bson` library, the write of a `Keyed<u32>` that maps `8` to `"eight"`, and then the read of the row above as `Keyed<u32>`:

  ```text
  a value could not be written as BSON: Invalid map key type: 8
  the row without an _id does not read as expected: undescribed: invalid type: string "8", expected u32
  ```

  Under version 3:

  ```text
  a value could not be written as BSON: BSON error. Kind: A serialization error occurred. Message: invalid document key type: int64.
  the row without an _id does not read as expected: undescribed: BSON error. Kind: A deserialization-related error occurred. Message: invalid type: string "8", expected u32.
  ```
- **A key serde writes as text is stored as that text and read back**: a `String`, a `char`, a `chrono` date or time, a plain enum, and a brand over a `String`. A key the expansion cannot see into, a type of another crate or a brand declared below the type, is not refused.
- **A type that lists `"decode_with"` alone builds with such a map** and reads it from JSON. Its `from_bson_piped` refuses a stored row that holds the map, with one `Undescribed` issue.

### A query the operations do not cover

The operations cover a filter, an update and the five read options. An aggregation, an upsert, a find-one-and-update and a projection are calls on the driver itself, on the same `Collection<bson::Document>`:

- a typed filter or update is handed to the driver as a document, through `into_document()`;
- each row the driver answers is read with `from_bson_piped` or `from_bson_with`, into a flagged type;
- a row is read as the driver's cursor hands it over, with `advance` and `deserialize_current`, so reading many rows asks no `futures` crate of the caller, as the generated `find` asks none.

An aggregation answers rows that are no rows of `Invoice`, so they are read as a type of their own. `"decode_with"` gives it `from_bson_piped`, and `Billed` lists `"mongodb"` as well, so that `billed_schema` re-exports `OperationError`:

```rust
/// What one customer was billed: no row of `Invoice`, so no read of `Invoice` answers it.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Billed {
    #[serde(rename = "_id")]
    pub customer: String,
    pub total: f64,
}

/// What each customer was billed, by an aggregation run on the driver itself. Each row it
/// answers is read as the operations read one, and fails as they fail.
async fn billed(
    invoices: &Collection<Document>,
) -> Result<Vec<Billed>, billed_schema::OperationError> {
    use billed_schema::OperationError;

    let pipeline = [
        doc! { "$group": { "_id": "$customer.name", "total": { "$sum": "$total" } } },
        doc! { "$sort": { "_id": 1_i32 } },
    ];
    let mut cursor = invoices
        .aggregate(pipeline)
        .await
        .map_err(OperationError::Database)?;
    let mut rows = Vec::new();
    while cursor.advance().await.map_err(OperationError::Database)? {
        let row = cursor
            .deserialize_current()
            .map_err(OperationError::Database)?;
        let id = row
            .get("_id")
            .map_or_else(|| "without an _id".to_owned(), ToString::to_string);
        let read =
            Billed::from_bson_piped(row, &[]).map_err(|refused| OperationError::Unreadable {
                row: id,
                issues: refused.issues,
            })?;
        rows.push(read);
    }
    Ok(rows)
}
```

The function fails with `billed_schema::OperationError`, the crate's one `OperationError`, built as the generated reads build it: `Database` by name for each call on the driver, and `Unreadable` from the row's `_id` and the issues of the refused read.

### Limits

- **A type that holds a type declared in the same function body does not build with the `jsonschema` feature on.** Its JSON Schema names the held type's schema module, which no module beside the holder reaches: ``error[E0433]: cannot find module or crate `inner_schema` in this scope``. Declare the held type at module level in such a build, as [Function-Local Types](#function-local-types) says.
- **An operation is a function of the type itself, under its own name.** A type that lists `"mongodb"` and already has a function of its own called `find_one`, `find_one_with`, `find`, `find_with`, `count`, `insert_one`, `update_one`, `update_many`, `delete_one` or `delete_many` builds without the `mongodb` feature and stops building with it. For a type with a `count` of its own: ``error[E0592]: duplicate definitions with name `count` ``.
- **A slot of a tuple struct under `skip_serializing_if` moves the positions after it.** The paths are written for a value that holds every slot. With `pair` a field typed with `Pair`, a tuple struct of an `Option<u32>` under `skip_serializing_if` and then a `String`, `Pair::MONGO_FIELDS.1` nested under `paths.pair()` writes the key `pair.1`, and a value whose first slot is `None` is stored as `{ "pair": ["x"] }`, its text at position `0`.

### This repository's checks against a server

The checks that run the operations against a real collection are in `tests/mongodb_operation_tests/`: `live.rs`, and this section's own examples beside it. Both major versions of the `bson` library compile them.

- **They read `TIXSCHEMA_MONGODB_URI`**, a MongoDB connection string. Each check works in a collection of its own in the database `tixschema_live`, named after the check, the `bson` major and the process, and drops it when it is done.
- **Without the variable every one of them stands down**, and says so once on stderr: a `cargo test` that passes with no server named has run none of them. With the variable set, a value that is no address and a server that does not answer are failures.
- **`just test-mongodb` refuses to stand down.** It stops with `No MongoDB server: set TIXSCHEMA_MONGODB_URI to one's address, as in mongodb://127.0.0.1:27017.` where the variable is not set, and runs the checks under both major versions where it is.
- **CI does not run them**: no MongoDB server comes with the CI runner. What `cargo test` proves with no server is what needs none: the documents the paths write, the read of a row once the driver has handed it over, and a row refused before it is sent.

## Feature Flags

The crate uses optional features to control code generation and dependencies. Each feature is enabled or disabled on its own, with three exceptions: `mongodb` turns on `bson`, and `bson` and `archie` turn on `serde`.

| Feature | Default | Description |
|---------|---------|-------------|
| `serde` | Yes | Serde attribute parsing (`rename`, `rename_all`, `tag`, etc.) |
| `zod` | Yes | Zod v4 schema generation alongside TypeScript types |
| `jsonschema` | Yes | JSON Schema generation via `json_schema()` method |
| `typescript` | Yes | TypeScript type generation via `ts_definition()` method |
| `mongodb` | No | MongoDB support: the ObjectId type, with validation, and on types whose [`enable` list](#opting-in) holds `"mongodb"` the [typed field paths, filters, updates and operations](#mongodb-operations-and-typed-filters) over a collection of their rows; turns on `bson` |
| `bson` | No | BSON support: `from_bson_with` and `from_bson_piped` on every type whose [`enable` list](#opting-in) holds `"decode_with"`; turns on `serde` |
| `chrono` | No | Chrono date/time type support (`NaiveDate`, `NaiveTime`, `NaiveDateTime`, `DateTime<Tz>`) |
| `indexmap` | No | `indexmap` type support: `IndexMap` as a map and `IndexSet` as a list. See [The order of a map's keys](#the-order-of-a-maps-keys) |
| `dart` | No | Dart type generation via `dart_definition()`, with a JSON `fromJson`/`toJson` codec whose read [calls each `preprocess`](#a-preprocess-in-the-dart-client) and [refuses what `validate()` refuses](#the-dart-read-refuses-what-validate-refuses), and a [`validate()`](#validate-in-dart) on every type |
| `swift` | No | Swift type generation with a `Codable` codec, and a [`validate()`](#validate-in-the-swift-client) on every type, which [the read of each calls](#a-swift-read-refuses-what-validate-refuses) |
| `archie` | No | `TryFrom` conversions to and from archie's frontmatter `Value` on types whose [`enable` list](#opting-in) holds `"archie"`. See [archie Frontmatter Conversions](#archie-frontmatter-conversions-archie); turns on `serde` |
| `kotlin` | No | Kotlin type generation via `kotlin_definition()`, with `kotlinx.serialization` annotations, a [`validate()`](#validate-in-the-kotlin-client) on every type and [a read that refuses what it refuses](#the-read-in-the-kotlin-client). A consuming Kotlin build declares two runtime dependencies the emitted code names -- `org.jetbrains.kotlinx:kotlinx-serialization-json` for the JSON codec and `org.jetbrains.kotlinx:kotlinx-coroutines-core` for the transport's own reply correlation and heartbeat -- plus the Kotlin Gradle plugin `kotlin("plugin.serialization")` |

Common configurations:

```toml
# Default (serde + zod + jsonschema + typescript)
tixschema = "0.1.0"

# All features including optional ones
tixschema = { features = ["serde", "zod", "jsonschema", "typescript", "mongodb", "bson", "chrono", "indexmap", "dart", "swift", "kotlin", "archie"] }

# Minimal (TypeScript only, no Zod or JSON Schema)
tixschema = { default-features = false, features = ["typescript"] }

# TypeScript + Zod without JSON Schema
tixschema = { default-features = false, features = ["serde", "zod", "typescript"] }
```

CI tests every combination of the feature sets (`web`, `mobile`, `mongo`) via `cargo-hack`; `just test` runs every combination of the plain features locally.

The features say what tixschema can generate, and a type's [`enable` list](#opting-in) says what it generates for that type: the recovering decode for `"decode_with"`, from BSON as well under `bson`, the MongoDB layer for `"mongodb"`, under `mongodb`, and the conversions to archie's frontmatter `Value` for `"archie"`, under `archie`. A crate whose types write the list writes [the initializers](#the-support-libraries) at its root, `tixschema::decode_with_foundation!();`, and `tixschema::mongodb_foundation!();` where a type lists `"mongodb"`. `mongodb_foundation!()` writes nothing in a build without `mongodb`, and `decode_with_foundation!()` nothing in a build without `serde`, so both calls stay whatever features the crate builds with.

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

Swift also moves a field named `validate`, the [method every emitted type has](#validate-in-the-swift-client): the property is `validate_`, and `CodingKeys` keeps the key, `case validate_ = "validate"`.

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

You can also generate JSON schemas from the Zod schemas for API documentation or OpenAPI specs:

```typescript
import { generateSchema } from '@zod-schema/json-schema';
import { User$Schema } from './types/generated';

const jsonSchema = generateSchema(User$Schema);
```

## Serde Attribute Support

The macro respects Serde attributes when the `serde` feature is enabled:

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfile {
    pub user_id: String,       // becomes userId in TypeScript
    pub first_name: String,    // becomes firstName in TypeScript
    pub last_name: String,     // becomes lastName in TypeScript
    #[serde(rename = "emailAddress")]
    pub email: String,         // becomes emailAddress in TypeScript
    pub created_at: String,    // becomes createdAt in TypeScript
}
```

Supported Serde attributes:

- `#[serde(rename = "...")]` -- rename individual fields
- `#[serde(rename_all = "camelCase")]` -- rename all fields with a naming convention
- `#[serde(tag = "...")]` -- internally tagged enums: the variant's data is written beside the discriminator
- `#[serde(tag = "...")]` on a struct -- the struct's name is written under that key, and every surface describes the key: see [A Struct's Own Tag](#a-structs-own-tag)
- `#[serde(tag = "...", content = "...")]` -- adjacently tagged enums
- `#[serde(untagged)]` -- untagged enums generate a union (`A | B`) / Zod `z.union([...])` / JSON Schema `anyOf`
- `#[serde(flatten)]` -- flatten a field into the parent as an intersection type (`A & B`) / Zod `.and(...)`
- `#[serde(transparent)]` -- transparent wrappers (used for branded newtypes)
- `#[serde(skip_serializing_if = "...")]` -- the key is left out of the payload when the predicate fires: `roles: z.array(z.string()).optional(),` in Zod and no `required` entry in the JSON Schema. On a field that is not an `Option` the TypeScript member takes the optional key too, `roles?: Array<string>;`, there being no second spelling for it; an `Option` field keeps `T | undefined` unless [`ts_optional`](#ts_optional) asks otherwise
- `#[serde(skip)]` -- the key is written into no payload and read out of none, so no surface describes the member at all: no TypeScript member, no Zod key, and neither a `properties` nor a `required` entry. On a tuple-struct or tuple-variant slot it takes the slot out of the described tuple, which shortens the arity -- and a variant declaring one slot becomes a unit variant, which is what serde writes for it
- `#[serde(skip_serializing)]` -- the write half of `skip`: the key is left out of every payload while a supplied one is still read, and every surface answers as it does for `skip_serializing_if`
- `#[serde(skip_deserializing)]` -- the read half: the key is written into every payload while a supplied one is discarded, so the member keeps a required key
- `#[serde(default)]` and `#[serde(default = "...")]`, on a field or on the struct -- a key the payload leaves out is read as the default on every surface, which is handed the default as JSON: see [A Key Read as Its Default](#a-key-read-as-its-default-serdedefault)
- `#[serde(alias = "...")]` -- read by the [recovering decode](#recovering-decode-decode_with), by the [Kotlin client's read](#the-read-in-the-kotlin-client), by the [Dart client's read](#the-dart-read-refuses-what-validate-refuses) and by the [Swift client's read](#a-swift-read-refuses-what-validate-refuses): a key stored under an alias counts as its field, and a tag stored as one names its variant. No schema surface describes an alias
- `#[serde(deserialize_with = "...")]`, `#[serde(serialize_with = "...")]` and `#[serde(with = "...")]` -- the field's own functions, which the [recovering decode](#recovering-decode-decode_with) reads a value through and writes it back through, as serde's derive does

The three `skip` spellings are three different wires, and [Optional Fields](#optional-fields) reads each one in both directions, positional slots included.

If the `serde` feature is disabled but serde attributes are present, you will see compile-time warnings and field names will not be transformed.

### A Struct's Own Tag

`#[serde(tag = "...")]` on a struct makes serde write the struct's name under that key, ahead of its fields. Every surface describes the key as a required member whose only value is that name -- the struct's ident, or its `#[serde(rename = "...")]`:

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind")]
pub struct Tagged {
    pub name: String,
}
```

```typescript
export type Tagged = {
  kind: "Tagged";
  name: string;
};

const Tagged$RawSchema = z.strictObject({
  kind: z.literal("Tagged"),
  name: z.string(),
});
```

The JSON Schema lists `kind` in `required` as `{"type": "string", "const": "Tagged"}`. The Dart, Swift and Kotlin types write the key. They read a payload whose key holds `"Tagged"`, and refuse one without the key or with another value under it, as TypeScript and Zod refuse it: see [The Dart read refuses what `validate()` refuses](#the-dart-read-refuses-what-validate-refuses), [A Swift read refuses what `validate()` refuses](#a-swift-read-refuses-what-validate-refuses) and [The read in the Kotlin client](#the-read-in-the-kotlin-client). serde reads the struct either way, whatever value the key holds, so the schemas describe what the struct writes. A struct that flattens a tagged one is written with the key in its own object, which is what its intersection describes. A field of the struct that writes the same key is refused, since serde would write the key twice.

## Important Notes

1. **Naming Convention**: A type publishes under the Rust ident it is declared with, spelled exactly as written. `#[model_schema(name = "...")]` is the only way to publish under another name, and it moves that name on every surface at once. Two declarations reaching one published name are refused at expansion. See [Published Names](#published-names-name).

2. **Type References**: Nested types reference each other by the name the referenced type publishes -- its Rust ident, or its `name` override.

3. **Map Keys**: `HashMap`, `BTreeMap` and, under the `indexmap` feature, `IndexMap` are the maps that are supported, and the key decides what the map describes as: a `String` key gives an object open to any string key, and a plain `#[model_schema()]` enum key gives one closed to the enum's members. A brand and a `#[model_schema()]` alias answer by their inner and their target, keeping their own name on the TypeScript and Zod surfaces. A key path proved to be one serde writes as no key at all, and any sequence-wrapped key, is refused at expansion; every other key describes as an open object. See [Collections and Maps](#collections-and-maps).

4. **Array Types**: `Vec<T>` becomes `Array<T>` in TypeScript, one `Array<...>` per level written — `Vec<Vec<T>>` is `Array<Array<T>>`.

5. **Optional Fields**: `Option<T>` becomes `field: T | undefined` in TypeScript unless `#[model_schema_prop(ts_optional)]` asks for `field?: T`, and `z.union([z.null().transform(() => undefined), type, z.undefined()]).prefault(undefined)` in Zod v4 either way.

6. **Complex Nesting**: The crate supports deeply nested structures including `HashMap<String, Vec<HashMap<String, T>>>` and similar patterns.

7. **Doc Comments**: Rust doc comments on types and fields are carried through to the generated TypeScript as JSDoc comments.

8. **Built-in Type Mappings**: Besides the primitives, `String` and `std::path::PathBuf` both map to `string` (`z.string()`). MongoDB `ObjectId` and the `chrono` date/time types are supported behind their respective feature flags. A member that holds `serde_json::Value` or `serde_json::Map` stops the build, and so does a service operation that takes or answers one: see [A JSON value is no member](#a-json-value-is-no-member).

## Error Handling & Troubleshooting

### Feature-Related Errors

#### ObjectId Errors

**Error:** `cannot find type 'ObjectId' in this scope`

**Cause:** Using `ObjectId` without the `mongodb` feature enabled.

**Solutions:**
```toml
# Option 1: Enable the mongodb feature
tixschema = { features = ["mongodb"] }

# Option 2: Use full path in your code
# use mongodb::bson::oid::ObjectId;
```

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
tixschema = { features = ["indexmap"] }
```

A crate that declares a `#[model_schema()]` type of its own named `IndexMap` or `IndexSet`, above the type that holds it, is not refused: the name is that type.

#### JSON Schema Method Missing

**Error:** `no function or associated item named 'json_schema' found`

**Cause:** Calling `.json_schema()` without the `jsonschema` feature enabled.

**Solution:**
```toml
tixschema = { features = ["jsonschema"] }
```

#### Zod Schema Missing

**Symptom:** Generated TypeScript contains only types, no Zod schemas.

**Cause:** `zod` feature is disabled.

**Solution:**
```toml
tixschema = { features = ["zod"] }
```

#### Serde Attributes Ignored

**Symptom:** Field names not transformed (e.g., `user_id` instead of `userId`).

**Cause:** `serde` feature is disabled.

**Solution:**
```toml
tixschema = { features = ["serde"] }
```

### Compilation Errors

#### Map Keys Without Enumerable Members

**Error:** *a map key must be a plain `#[model_schema()]` enum, whose members become the object's keys — `<type>` resolves to a type with no `enum_members()`*, reported at the field; or `no associated function or constant named enum_members found for <type>` (`E0599`), reported at the map's key type.

A map key written as a type path must be something serde writes into a key: a plain `#[model_schema()]` enum, whose members become the object's keys; a `#[serde(transparent)]` brand or a `#[model_schema()]` alias whose wire form is one serde writes into a key; or a `String`. [Collections and Maps](#collections-and-maps) sets out what each of those describes as, and every key that is none of them is covered there too — this entry is about the two diagnostics a type path earns.

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

The keys serde *does* stringify for you are untouched: numbers, `bool`, `char` and the chrono types describe the map's values and name its keys by the text serde writes them as, as [Collections and Maps](#collections-and-maps) sets out. The rule is refuse-what-serde-refuses, never refuse-what-is-not-a-`String`. A tuple or a map in the map's *value* is untouched too: only the key has to be a string.

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

A referenced type must be declared at item scope. A type declared inside a function body publishes its schema module inside that same body, where the `use super::*` of the referencing type's own module never reaches it:

```rust
// Wrong: `Inner` is declared inside the function body
#[test]
fn builds_the_schema() {
    #[model_schema()]
    pub struct Inner {
        pub id: String,
    }

    #[model_schema()]
    pub struct Outer {
        pub inner: Inner, // E0433: cannot find module or crate `inner_schema` in this scope
    }
}

// Correct: `Inner` is declared at item scope; `Outer` may stay in the function body
#[model_schema()]
pub struct Inner {
    pub id: String,
}

#[test]
fn builds_the_schema() {
    #[model_schema()]
    pub struct Outer {
        pub inner: Inner,
    }
}
```

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

With the `jsonschema` feature on, a field naming a `#[model_schema]` type reaches that type's generated `{type}_schema` module by its bare name, so the module has to be in scope where the referencing type is declared. Import it beside the type:

```rust
pub mod version {
    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Version {
        pub number: i32,
    }
}

pub mod record {
    use crate::version::{Version, version_schema};

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Record {
        pub versions: Vec<Version>,
    }
}
```

rustc's help line suggests `cargo add version_schema`; there is no such crate. The module is generated next to `Version`, in the module that declares it.

With the `mongodb` feature on, a `#[model_schema(enable = ["decode_with", "mongodb"])]` type asks for the same import in every build, `jsonschema` or not, for each type that lists `"mongodb"` one of its fields is typed with: its typed MongoDB paths name that type's own. The error is then ``cannot find `version_schema` in `super` `` (`E0433`), reported at the field's type, and the same import resolves it. [Limits](#limits) has it beside the other limits of the paths.

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

The same refusal answers `rename_all` and `rename_all_fields`, and a list written on a variant or on the type. It is raised in every build that describes the type, a mobile client included: the `serde` feature with any of `typescript`, `zod`, `jsonschema`, `dart`, `swift` and `kotlin`. A build with none of the six takes the list: the [recovering decode](#recovering-decode-decode_with) looks a value up under the `deserialize` side, and a [MongoDB path](#mongodb-operations-and-typed-filters) is under the `serialize` side.

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

#[cfg(feature = "jsonschema")]
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
