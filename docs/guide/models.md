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

#### A Draft and a Valid Type

One declaration makes two types. The type you write is emitted as `{Name}Draft`, with the fields, derives and attributes you wrote: code builds it and changes it. `{Name}` is the valid type, which only the draft's `validate()` and a read of the type make, so a function that takes `&UserProfile` gets a value that passed every rule the type declares. Every `#[model_schema()]` struct and enum has the pair; a [brand](generics-and-brands.md#a-brand-is-sealed) has none.

```rust
let draft = UserProfileDraft {
    user_id: "u1".to_owned(),
    name: "Ada".to_owned(),
    age: 36,
    score: 1.5,
    is_verified: true,
};

// `validate()` takes the draft and answers the valid value, or every rule it breaks.
let profile: UserProfile = draft.clone().validate().unwrap();
assert_eq!(profile.age, 36);

// The draft lists the same rules without being consumed.
assert!(draft.validation_issues().is_empty());

// A valid value turns back into its draft, which is open again.
let mut again: UserProfileDraft = profile.into();
again.age = 37;
```

The valid type reads like the draft: `profile.age` and `&*profile` (a `match &*payment { PaymentDraft::Card { .. } => .. }` over an enum) work through `Deref`. It has no `DerefMut`, so `profile.age = 1` and `&mut profile.age` do not compile, and its tuple constructor is closed, so `UserProfile(draft)` does not compile in the declaring module or outside it. `#[derive(Default)]` stays on the draft only, because a default would build a valid value no check saw.

The valid type writes what its draft writes, and `serde_json::from_str::<UserProfile>(..)`, a MongoDB read and a dispatcher's read return a valid value and refuse a broken one with the messages `validate()` gives. The surfaces keep the declared name: TypeScript, Zod, JSON Schema, Dart, Swift, Kotlin, MCP and OpenAPI all publish `UserProfile`. A member that holds a model names the valid type (`pub profile: UserProfile`), so a draft is built from values that already passed. `Self` in a member names the valid type too. A serde message that names the type, such as `data did not match any variant of untagged enum` or `the enum variant X::Y cannot be serialized`, names `UserProfile` where the read or the write goes through the valid type, and `UserProfileDraft` where it goes through the draft itself.

A declaration whose `{Name}Draft` is the name of another type in the module stops the build with the compiler's duplicate-name error (`E0428`). The type the valid wrapper is made of is generic and hidden: the compiler prints a valid value's type as `UserProfile<UserProfileDraft>`, and a type declared in a function body is wrapped as one declared beside it.

#### The Draft and the Valid Type in TypeScript and Zod

TypeScript publishes the same pair. `ShipmentDraft` is the plain object type that was published as `Shipment`, and `Shipment` is the valid type: read-only, and branded with the `$brand` marker a [brand](generics-and-brands.md#a-brand-is-sealed) already carries, so the bundle imports `$brand` from `zod` as it does for a brand. Nothing but a `parse` returns one. A member that holds a model names the valid type, as in Rust (`label: LabelText`), so a draft is made of values that already passed:

```typescript
export type ShipmentDraft = {
  label: LabelText;
  parcelCount: number;
};

export type Shipment = Readonly<ShipmentDraft> & $brand<"Shipment">;
```

`Shipment` is the draft read-only and branded. A draft that admits `null` (a newtype over an `Option`, or over an alias of one) keeps the `null` beside the branded rest, written `(Readonly<NonNullable<XDraft>> & $brand<"X">) | null`, because `X$Schema.parse(null)` returns `null` and an intersection with `null` would be `never`; its schema is `XDraft$Schema.readonly().brand<"X">().nullable()`, which refuses what the draft refuses.

Zod publishes a schema for each. `ShipmentDraft$Schema` parses to a `ShipmentDraft`, and `Shipment$Schema` is the draft's schema frozen and branded, so `Shipment$Schema.parse(json)` has the type `Shipment`. The refusals of the two are the same ones, word for word, and so is the JSON Schema, which is not split: the wire is the same for both types.

```typescript
import { Shipment$Schema, ShipmentDraft$Schema, type Shipment, type ShipmentDraft } from "./models";

const draft: ShipmentDraft = ShipmentDraft$Schema.parse(json);
const shipment: Shipment = Shipment$Schema.parse(json);

function weigh(value: ShipmentDraft): number {
  return value.parcelCount;
}
weigh(shipment); // a Shipment is a ShipmentDraft: no cast.
```

A value written by hand is not a `Shipment`, and a member of one cannot be assigned. `tsc --strict` answers `const shipment: Shipment = { label, parcelCount: 3 }` with ``error TS2322: Type '{ label: ParcelLabel; parcelCount: number; }' is not assignable to type 'Parcel'.`` and ``Property '[$brand]' is missing in type '{ label: ParcelLabel; parcelCount: number; }' but required in type '$brand<"Parcel">'``, and `shipment.parcelCount = 1` with ``error TS2540: Cannot assign to 'parcelCount' because it is a read-only property.`` The schema freezes the value it returns as well, so the same assignment in a script throws ``TypeError: Cannot assign to read only property 'parcelCount' of object '#<Object>'`` under `"use strict"`. The freeze is shallow: each member that is a model was frozen by its own schema. A value that is its own bytes is the one it cannot freeze, because `Object.freeze` refuses a typed array with elements: a tuple struct of `Vec<u8>` and an untagged enum with a variant of one `Vec<u8>` parse to an unfrozen value of the same type.

TypeScript cannot stop an assertion: `value as Shipment` makes a `Shipment` out of anything, and no schema has seen it. That is the one way round the check, and it is a way a reader of the code can see. The same goes for a `Shipment` written in a `declare const`.

Every client and dispatcher types each message and result as the valid type, so `client.ship(...)` takes a `Shipment` and a handler is handed one. Make the argument with `Shipment$Schema.parse(draft)`. A function a `check` names takes the draft, the type the rules run on: `function withACode<CodeType>(value: CrateDraft<CodeType>, ctx: ParsePayload<CrateDraft<CodeType>>)`, where a `Crate<CodeType>` stops the build of the consumer with ``error TS2345: Argument of type 'CrateDraft<unknown>' is not assignable to parameter of type 'Crate<unknown>'.`` A struct that flattens a model holds its keys among its own, so it holds the model's draft: `{ label: string } & MoorageDraft`.

A type that declares a parameter publishes `WrapDraft<T>` and `Wrap<T>` of the same form, and two factories: `WrapDraft$SchemaFactory` answers the draft's schema as it always answered the type's, and `Wrap$SchemaFactory` answers a `ZodType<Wrap<...>>`, which names what the schema parses to, so `.shape` is not on it. `Wrap$SchemaDefault` and `WrapDraft$SchemaDefault` are each the factory at the declared defaults. A build with `typescript` and no `zod` writes the marker a brand writes there, `{ readonly [__brand_Shipment]: true }` after a `declare const __brand_Shipment: unique symbol;`.

#### Unit Structs

A unit struct -- `pub struct Ping;`, no braces, no fields -- crosses the wire as `{}`. Serde's own derive would write and read it as `null`, disagreeing with every other surface, which already describes an empty struct as an object; the macro replaces the author's own `Serialize`/`Deserialize` derive with impls that write `{}` and read it back (refusing a key, unless the type is [`nonstrict`](#unknown-keys-strict-reads-and-nonstrict), and refusing `null`), keeping every other derive and attribute exactly as written:

```rust
#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ping;

assert_eq!(serde_json::to_string(&PingDraft).unwrap(), "{}");
serde_json::from_str::<Ping>("{}").unwrap();
serde_json::from_str::<Ping>("null").unwrap_err();
```

TypeScript, Zod and JSON Schema describe it exactly as a braced empty struct already does: `Record<string, never>`, `z.strictObject({})`, `{"type": "object", "additionalProperties": false, "properties": {}, "required": []}`. Dart, Swift and Kotlin each publish their own no-data type -- Dart a `const` class, Swift an empty `Codable` struct, and Kotlin a serializable `object Ping` rather than a `class` (a braced `struct Ping {}` still publishes a `class`, since it is a different shape). Declared above a service as an operation's success, `Result<Ping, E>`, a unit struct reads like a `()` success on every client -- the limit is a registry's own: the struct must be declared in the same crate, above the service, or the client sees the field-position type instead (and the call still succeeds, since both sides still agree the wire is `{}`).

### A Member's Rustdoc in the JSON Schema

A named member's rustdoc is the `description` of its property in the JSON Schema: a struct field, and a member of a named enum variant under every tagging (external, internal, adjacent and untagged). `json_schema()`, `openapi()` and the tools of `mcp_tools()` all show it. The lines are joined with newlines, a `rust example` block is left out, and a member with no rustdoc has no `description`. A member typed with a named type keeps its `$ref` and gains the `description` beside it. TypeScript, Zod, Dart, Swift and Kotlin are unchanged.

```rust
#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct CommentArgs {
    /// Names from the registry who must see this in their pending queue.
    #[serde(default)]
    pub to: Vec<String>,
}
```

`CommentArgs::json_schema()["properties"]["to"]` is `{"default": [], "description": "Names from the registry who must see this in their pending queue.", "items": {"type": "string"}, "type": "array"}`.

### Published Names (`name`)

A type publishes under the Rust ident it is declared with, spelled exactly as written. Nothing is read off that spelling -- no suffix is taken off it and no part of it is rewritten -- so `UserData` publishes as `UserData` on all three surfaces.

`#[model_schema(name = "...")]` is the one way to publish under something else. It moves the name everywhere at once: the `export type` line, the Zod consts, the `$defs` key a document hoists it under, and every reference another type writes.

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
export type ContextValueDraft = {
  label: string;
};

export type ContextValue = Readonly<ContextValueDraft> & $brand<"ContextValue">;

export type ContextValueDataDraft = ContextValueDraft;

export type ContextValueData = ContextValue;

export type HolderDraft = {
  value: ContextValue;
};

export type Holder = Readonly<HolderDraft> & $brand<"Holder">;
```

The `export type ContextValueData = ContextValue;` line, and `export type ContextValueDataDraft = ContextValueDraft;` for the draft, are the one place the Rust ident is written on purpose. A type declared *above* the renamed one has only that ident to name it by -- an override is not recoverable from it -- so both spellings answer, and the alias carries the one type. The Zod surface publishes the same pairs, `export const ContextValueData$Schema = ContextValue$Schema;` and `export const ContextValueDataDraft$Schema = ContextValueDraft$Schema;`, and the Rust module the schema lives in is named from the ident for the same reason: an override never moves it.

A type alias is the one shape with no surface name of its own, and publishes under a `Type` suffix -- `pub type Slug = String;` as `SlugType` -- unless `name` says otherwise. See [Type Aliases](nested-recursive-and-aliased-types.md#type-aliases).

Two declarations cannot publish under one name. The emitted types, schemas and definitions are one flat namespace, so a second declaration reaching a name already taken is refused at expansion, in every feature configuration, naming both declarations:

```text
error: model_schema: type `Beta` publishes as `Shared`, which type `Alpha` already publishes as
       -- one name cannot carry two declarations, whose types, schemas and definitions would
       overwrite each other. Give one of them a `#[model_schema(name = "...")]` of its own
```

### Unknown Keys (strict reads and `nonstrict`)

A key no member declares is refused on every surface. TypeScript's type, Zod's `z.strictObject` and the JSON Schema's `"additionalProperties": false` describe a closed object, and the Rust read refuses what they refuse: `#[model_schema]` writes `#[serde(deny_unknown_fields)]` on every struct of named members and on every enum it expands, and on the message struct the `service_schema` macros generate for an operation's arguments. The developer writes nothing. The Rust dispatchers of `amqp_rpc`, `ws_rpc`, `http_rest` and `mcp` answer a request that holds such a key with `failed-validation`, and the handler is not called. With `pub struct Role { pub name: String, #[serde(default)] pub min_occurs: Option<u32> }` renamed to `minOccurs` on the wire, `{"entities":[{"name":"a","minOccur":1}]}` answers the field `entities.0.minOccur` and the detail ``'entities.0.minOccur': unknown field `minOccur`, expected `name` or `minOccurs` ``. A key serde reads under an alias is read; the key of a member under `#[serde(skip)]` is refused, as it is described by no surface. A unit struct refuses `{"a":1}` and reads `{}`, and so does the message of an operation that takes no argument: every dispatcher, the Rust ones of `amqp_rpc`, `ws_rpc`, `http_rest` and `mcp`, the TypeScript ones of `ws_rpc`, `http_rest` and `mcp`, and the Dart and Kotlin `ws_rpc` attachments and `mcp` servers, answers `{"stray":1}` with `failed-validation` and the field `stray`, and calls no handler. The TypeScript dispatchers and the Dart and Kotlin `ws_rpc` attachments name a key no member of any message declares by the path of the object that holds it and the key, as the Rust ones do: `{"rope":{"length":3,"stray":1},"strokes":2}` answers the field `rope.stray`. The TypeScript clients refuse such a message before they send it under the same field, and name such a key in a reply they read as `failed-validation`. The Kotlin, Dart and Swift reads of a strict type refuse such a key at the top level, in a list element, under `#[serde(flatten)]`, beside a tag alone and in an untagged enum. serde names the path of a key it refuses inside a type it buffers (a struct beside a `#[serde(flatten)]` member, an enum under a tag alone, an untagged enum) no deeper than the type, so the Rust dispatchers add the key serde names in its message to that path: `{"label":"a","stray":1}` read into a struct beside a flattened member answers the field `stray` and the detail ``'stray': unknown field `stray` ``.

`#[model_schema(nonstrict)]` opts one type out, named after Zod's `z.object`. A nonstrict type reads past a key no member declares on every surface: the Rust read has no `deny_unknown_fields`, Zod writes `z.object`, which strips the key as serde drops it, the JSON Schema writes `"additionalProperties": true` for that type's own object, and the Kotlin, Dart and Swift reads of that type read past it. TypeScript's type does not change. `nonstrict` beside a `#[serde(deny_unknown_fields)]` the developer wrote is refused at expansion, naming both. A type that already carries `#[serde(deny_unknown_fields)]` reads as it did.

The attribute stays off, and the Rust read of the type reads past the key, where serde could not refuse it without refusing the keys a member holds, or cannot refuse it at all:

- a type that flattens a member serde reads every key still left for: a map, an enum under a tag alone, an untagged enum, a struct that flattens a member of its own or is under its own tag, an `ObjectId`, a member a `preprocess` or a function of its author's reads, and a type parameter ([Order and strictness of flattened members](flatten.md#order-and-strictness-of-flattened-members));
- a type with a member under `#[serde(skip_deserializing)]` that serde still writes, whose own writes carry a key it would refuse, and a type that names a `preprocess`;
- a struct under its own `#[serde(tag = "...")]`, whose tag serde writes and does not read back;
- a tuple struct, a brand and a type alias, which hold no key.

The unit variant of a strict enum under a tag alone refuses a key beside the tag, which serde's own derive reads past: the macro hangs a read it writes on each such variant (`#[serde(deserialize_with = "...")]`), so `#[serde(tag = "kind")] pub enum Mark { Line { length: u32 }, Blank }` refuses `{"kind":"Blank","stray":1}` with ``unknown field `stray`, there are no fields`` and reads `{"kind":"Blank"}`. The Dart and Swift reads of that variant refuse the key as well.

A braced variant serde reads no member of, `Off {}`, or one whose every member serde neither reads nor writes, `Off { #[serde(skip)] since: u32 }`, is read the same way: the macro hangs a read it writes on the variant, which reads `{}`, refuses every key with ``unknown field `stray`, there are no fields`` and builds each member as serde's derive builds it, from the member's `#[serde(default = "...")]`, else from `Default`, under a `Default` bound of each such member whose type names a parameter of the enum. The enum is then closed, so its other variants refuse a key too: `pub enum Signal { Off {}, On { level: u8 } }` refuses `{"Off":{"stray":1}}` and `{"On":{"level":1,"stray":1}}` and reads `{"Off":{}}` and `{"On":{"level":1}}`, under a tag alone, under a tag and a content key, and untagged as well, as the Zod, JSON Schema, Dart, Kotlin and Swift reads do. Marked `nonstrict`, it reads past the key on every surface. A braced variant of no member read that serde never reads, under `#[serde(skip)]`, or reads through its author's `#[serde(deserialize_with = "...")]`, closes its enum the same way and gets no read of the macro's: the author's function decides what that variant takes.

A struct whose flattened members all take their own keys is closed, with a key of its own or none: `pub struct Bundle { #[serde(flatten)] pub stamp: Stamp, #[serde(flatten)] pub ink: Ink }` refuses `{"at":"noon","colour":"red","stray":1}`. A `nonstrict` holder of a member that refuses a key refuses it on every surface, as serde does. A `nonstrict` enum is judged variant by variant, as serde reads each variant: a variant that refuses a key, by a member it flattens or, under a tag alone or untagged, by the type a newtype variant holds, refuses it on every surface, and every other variant, and the object under a tag and a content key, reads it past on every surface. A struct not marked `nonstrict`, or a struct variant of an enum not marked `nonstrict`, that flattens a member which reads every key still left and refuses none, a map aside, stops the build where no other member it flattens refuses such a key, whatever another variant of the enum refuses: serde cannot refuse one beside it, so mark the struct or the enum `nonstrict`, and every surface reads past the key as serde does. So does one that flattens a member read through a `preprocess`, a `deserialize_with` or a `with` of its author's, whatever the member's type and wherever that type is declared: whether serde refuses such a key depends on what the function answers, which tixschema cannot see. A `nonstrict` struct that flattens such a member is a member that reads past a key to a holder that flattens it. A member whose type is declared below its holder, or in another crate, is held to take its own keys, and where its hidden constant says it reads every key still left, the build asks for the holder, or the enum of a struct variant, to be marked `nonstrict`. An enum one of whose variants flattens a member that reads every key still left cannot carry `deny_unknown_fields`, which serde reads for every variant at once, so where another of its levels would refuse a key (the object under a tag and a content key, another struct variant, the unit variant under a tag alone), the build stops: mark the enum `nonstrict`, or hold that variant's members in a struct of their own and write the variant as a newtype variant over it, a struct the build asks to be marked `nonstrict` where the member refuses no key. A braced struct of no member, `pub struct Hollow {}`, and one whose every member serde neither reads nor writes, `pub struct Skipped { #[serde(skip)] pub held: u32 }`, refuse `{"stray":1}` with ``unknown field `stray`, there are no fields`` and read `{}`: the macro takes `Deserialize` off the derive and writes the read, as it does for a unit struct, unless the type is `nonstrict`. So does the message the `service_schema` macros generate for an operation of no argument: the Rust dispatchers of `amqp_rpc`, `ws_rpc`, `http_rest` and `mcp` answer `{"stray":1}` with `failed-validation`, the field `stray` and the detail ``'stray': unknown field `stray`, there are no fields``, and call no handler, and the TypeScript, Dart and Kotlin dispatchers answer the same kind and field, each with a detail in its own words. The read the macro writes for a struct whose every member serde skips builds each member as serde's derive builds it, from the member's `#[serde(default = "...")]`, else from the container's `#[serde(default)]`, else from `Default`, so `{}` reads as `Skipped { held: 0 }`, and it asks the bounds that derive asks: `T: Default` of a `Skipped<T>` whose member of type `T` is built by `Default`, or what the container's `#[serde(bound(deserialize = "..."))]` writes. A braced struct of no member, or one whose every member serde skips, read through `#[serde(from = "...")]`, `try_from` or `remote`, stops the build, as the read the macro writes cannot be read through the conversion: mark it `nonstrict`, so serde reads it through the conversion and every other surface reads past the key. `into` is not refused.

A member written with the holder's own type parameter, `#[serde(flatten)] pub recipient: invoice_lookups::ParcelRecipient<IdType>`, is read at the type the holder's `default_types` fills the parameter with: under `default_types(IdType = String)`, the `const` beside the holder reads `ParcelRecipient<String>`. A type's keys are the same at every filling, since flattening a type parameter is refused, so that filling answers for all of them. Where the holder declares no entry for the parameter, the build stops and asks for one:

```text
error: model_schema: type `Handover` flattens `recipient` (`invoice_lookups::ParcelRecipient<IdType>`), declared below the type that flattens it, in another crate, or with no `#[model_schema]`: tixschema reads its keys from its hidden constant, which it names with `IdType` filled by the type `default_types` declares for it, and `Handover` declares none. Declare one, `#[model_schema(default_types(IdType = String))]` or any type the bounds of `IdType` accept.
```

Where serde would hand a strict member keys it does not own, the build stops and says the fix: declare the member last, mark it `nonstrict`, or change the shape ([Order and strictness of flattened members](flatten.md#order-and-strictness-of-flattened-members)).

The Dart, Kotlin and Swift reads close what they can name. The Dart read of a struct under its own tag reads past a key no member declares, as serde reads it. A strict holder of a member declared below it or in another crate, an enum's variant among them, refuses a key no member reads in each of them, the member's keys written when the text is built. A Swift read of the payload a holder hands a type (`SwiftSchemaPayload.read`), which a member that names a function is read from, refuses no key at its top level.

Those reads learn the keys of such a member only where its type is written with no type argument. A strict holder of a generic one declared below it or in another crate, `pub struct Haulage { pub label: String, #[serde(flatten)] pub haulier: Haulier<String> }` with `Haulier` declared below, refuses a stray key in Rust while the Dart, Kotlin and Swift reads read past it. A build with one of those clients on warns at the member, through a `#[deprecated]` constant the macro names, since a macro on stable Rust has no warning of its own; a crate built with `-D warnings` stops there:

```text
warning: use of deprecated constant `_::TIXSCHEMA_READS_PAST_A_STRAY_KEY`: type `Haulage` flattens `haulier` (`Haulier<String>`), a generic type declared below `Haulage` or in another crate: the Dart, Kotlin and Swift reads of `Haulage` cannot learn which keys it takes, so they read past a key no member declares, which the Rust read refuses. Avoid this shape where you can: declare `Haulier` above `Haulage`, write `haulier` under a key of its own, or mark `Haulage` `#[model_schema(nonstrict)]` so that every surface reads past the key.
```

A generic type declared above the holder earns no warning, whatever it holds.

Headers and query strings are not read as JSON objects, and this rule does not touch them. The MongoDB reads are unchanged: the recovering decode already refuses a key the type does not declare. Upgrading, a caller that sends a key no member declares is refused after it.

### Optional Fields

`Option<T>` fields validate as `z.union([z.null().transform(() => undefined), type, z.undefined()]).prefault(undefined)` in Zod v4 and are left out of the JSON Schema's `required` list, whose `anyOf: [type, { "type": "null" }]` describes the same two spellings serde reads into `None`. The `.prefault(undefined)` makes the field default to `undefined` when omitted from the input, and the leading `z.null()` arm coerces an explicit `null` to the same `undefined`.

TypeScript writes every `Option<T>` field as `field: T | undefined`. The one thing that writes the other spelling, `field?: T`, is [`#[model_schema_prop(ts_optional)]`](optional-and-nullable-keys.md#optional-typescript-keys-ts_optional) on the author's word. A serde attribute that drops the key of an `Option` decides what reaches the wire — the JSON Schema's `required` list and the Zod schema both read it — and never which of the two TypeScript spellings is written. [`#[model_schema_prop(nullable)]`](optional-and-nullable-keys.md#nullable-object-keys-nullable) writes a third spelling, `field: T | null`, for the field whose `None` reaches the wire as an explicit `null` rather than an absent key.

Each flavor is pinned to the one shape its declared type describes, and the `serde` feature enforces the pin. A bare `Option<T>` field is a compile error unless it carries a key-dropping attribute (`skip_serializing_if`, `skip_serializing`, or `skip`) — its declared shape is `T | undefined`, so a `None` has to leave the key out, not reach the wire as `null`. A [flattened](flatten.md#intersection-types-serdeflatten) one is not held to it: a flattened member writes no key of its own, so serde writes nothing for its `None`, whatever its attributes say, and reads the payload back. `nullable` is refused the opposite way: pairing it with a key-dropping attribute is a compile error too, since the flag declares the key always written. Either violation names the field and the attribute to add or remove. Deserialization does not enforce either pin: `{}` and `{"a":null}` both read into `None` under both flavors, so an optional key accepts an explicit `null` on input even on a field whose own serializer never writes one. The same holds for a `nullable` field: its key is required in the TypeScript type, the Zod schema and the JSON Schema, and the Rust read and each of the Swift, Dart and Kotlin reads take its absence as `None`, whether or not the struct also flattens a member.

A bare `#[serde(skip)]` is not that. serde writes the key into no payload *and* throws it away out of every payload that supplies one, so there is nothing on the wire for any surface to describe: TypeScript writes no member, Zod no key, and the JSON Schema neither a `properties` entry nor a `required` one. `#[serde(skip_serializing, skip_deserializing)]` is the same wire spelled out and is answered the same way. Note what this costs on the way in — the key is described by no schema, so a `z.strictObject`, an `additionalProperties: false` and the Rust read all *reject* a payload carrying it, as they reject any key no member declares. The schemas describe the payload serde writes, and that key appears in none of them; a member under an optional key would instead claim the key is sometimes written, and would describe a value nothing ever reads.

`#[serde(skip_deserializing)]` on its own drops neither surface's key: serde writes it in every payload, so the member keeps a required key, and the value a payload supplies under it is discarded on the way in.

A positional slot answers by that same wire, read in the place a tuple writes rather than under a key. A slot carrying a bare `#[serde(skip)]` is absent from the array serde writes *and* from every array serde reads, and the slots behind it move up: `struct Pair(#[serde(skip)] Option<String>, String)` holding `(Some("s"), "x")` writes `["x"]`, reads that back, and refuses `["s","x"]`. So the described tuple is the slots that remain — `[string]` in TypeScript, `z.tuple([z.string()])` in Zod, and one `prefixItems` entry under `minItems`/`maxItems` of 1. Dropping every slot leaves `[]`, which is the array serde writes there.

A slot dropped from only one of the two directions has no such description and is refused where it is declared. `skip_serializing` alone writes `["x"]` and reads only `["s","x"]`; `skip_deserializing` alone writes `["s","x"]` and reads only `["x"]`. A named field in that position is still describable — the key is absent from one payload and present in the other, which an optional key covers — but a slot is written by its place, so the two payloads differ in their arity and no fixed-arity tuple describes both.

A struct declaring a single slot keeps it whatever these attributes say: serde writes and reads a newtype struct's only slot regardless, so nothing there is dropped.

A tuple *variant*'s slots read the same way, with one difference at the lone-slot arity. `enum E { One(#[serde(skip)] String, u32) }` holding `("s", 7)` writes `{"One":[7]}`, reads that back, and refuses `{"One":["s",7]}`, so the described tuple shrinks exactly as a struct's does — under the content key when one is named, and to `{"One":[]}` when every slot is dropped. Where a variant differs is that it has its own name to fall back on: a variant declaring one slot and dropping it is written as a unit variant, `"One"` externally, `{"type":"One"}` under a tag and `null` untagged, and each surface describes it as the unit it has become. Because serde does read a variant slot's attributes at every arity, the one-directional halves are refused there at every arity too — `One(#[serde(skip_serializing)] String)` writes `"One"` and reads only `{"One":"s"}`.

The Dart, Swift and Kotlin clients declare, read and write the variant as those surfaces describe it. The value of `One` holds the one slot that remains: `{"One":[7]}` is read and written back as it was, and `{"One":["s",7]}` is refused, as serde refuses it; nothing of the dropped slot is declared, whatever its type. A variant whose every slot is dropped reads and writes `[]`, and a variant that declares one slot and drops it is a unit variant in each client too.

A whole variant reads as a member does. One under `#[serde(skip)]`, or under `skip_serializing` and `skip_deserializing` together, is written into no payload and read out of none, so no surface describes it and no client reads or writes it; one under either half alone stays, described as every variant is. [A Variant Serde Neither Writes Nor Reads](enums.md#a-variant-serde-neither-writes-nor-reads) has the rest.

Which key a field writes is read off the attribute with the `serde` feature on, as the renaming, the tagging and the guards are. With it off, tixschema reads no `#[serde(...)]` attribute: a member or a variant under `skip`, `skip_serializing`, `skip_serializing_if`, `skip_deserializing` or `default` is described as if the attribute were not written.

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
export type UserWithOptionalsDraft = {
  id: string;
  name: string;
  email: string | undefined;
  phone: string | undefined;
  avatar_url: string | undefined;
};

export type UserWithOptionals = Readonly<UserWithOptionalsDraft> & $brand<"UserWithOptionals">;
```

### A Key Read as Its Default (`#[serde(default)]`)

serde reads a key the payload leaves out as a default where a field of a struct, or a named member of an enum's variant, writes `#[serde(default)]` or `#[serde(default = "path")]`, or where the struct writes either one on itself. Every surface reads that key as the same default. The value of a default is known to Rust alone: the `Default` of the field's type, the value the named function answers, or the member of the struct's own default. So `zod_schema()`, `json_schema()`, `dart_definition()`, `swift_definition()` and `kotlin_definition()` write each default with serde when they build their text, and write the JSON into the text. The field's `serialize_with` or `with` writes the default where the field names one, as serde writes the field. A crate that declares such a type lists `serde_json`, as [Rust Dependencies](../../README.md#rust-dependencies) shows.

```rust
pub fn north_dock() -> String {
    "north".to_owned()
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Wharf {
    #[serde(default)]
    pub dock: String,
    #[serde(default = "main_gangway")]
    pub gangway: Gangway,
    #[serde(default = "north_dock")]
    pub gate: String,
    pub label: String,
}
```

`Gangway` is a `#[model_schema(enable = ["jsonschema"])]` struct of the crate's own, whose draft has a `Default` of `{"label":"main","width":2}` and which `main_gangway()` makes valid: a member typed with a model type names the function that builds the valid value, since the valid type has no `Default`. serde reads `{"label":"w"}` as `{"dock":"","gangway":{"label":"main","width":2},"gate":"north","label":"w"}`, and so does each surface:

| Surface | What reads a key the payload leaves out |
|---------|-----------------------------------------|
| Zod | `dock: z.string().prefault("")`: the parse is handed the default as a payload, so the checks of the member judge it |
| TypeScript | nothing changes: the type a parse answers holds every member, `dock: string;` |
| JSON Schema | the key is not in `required`, and the member states `"default": ""` |
| Rust | serde's own read, then `validate()` |
| Dart | `json.containsKey('dock') ? _readAt(json, 'dock', () => json['dock'] as String) : (jsonDecode('""') as String)`, in the read the member already has |
| Swift | `if container.contains(.dock) { ... } else { ... }`: the default is read from the JSON with the read the member already has |
| Kotlin | `val dock: String = Json.decodeFromString(serializer<String>(), "\"\"")`: the plugin evaluates the default of the property for a key the payload leaves out |

- **A key the payload holds is read as it was**, on every surface.
- **A default is judged as a payload is.** The default is read with the read of the member, and then `validate()` judges the value. So a default that breaks a bound of the member is refused with the member named, on every surface: `#[model_schema_prop(minLength = 1)] #[serde(default)] pub dock: String` refuses `{"label":"p"}` with `'dock': too short: minimum length is 1, got 0`.
- **A function a `preprocess` names is not called for a default**, in Rust, Dart, Swift and Kotlin. The Zod schema calls it with the default: `.prefault` is written around the whole schema of the member, so the function is handed the default.
- **The Dart file imports `dart:convert`**, for `jsonDecode`, and `dart:typed_data`, for `Uint8List`, where a type holds a `Vec<u8>`. **The Swift file imports `Foundation`**, for `JSONSerialization`. The Kotlin file imports the `json` package, as [the read in the Kotlin client](kotlin.md#the-read-in-the-kotlin-client) shows.
- **The Kotlin client writes a member with a default only where its value is another.** `kotlinx.serialization` with the default `Json` (`encodeDefaults = false`) writes `Jetty()` as `{}`, which serde reads as the same value. A `Json { encodeDefaults = true }` writes each member, as serde writes it.
- **An `Option` keeps what it had.** `#[serde(default)]` on an `Option` changes nothing on any surface: [Optional Fields](#optional-fields) describes it.
- **A struct member under `skip_serializing` or `skip_serializing_if` is read as its default too.** `#[serde(default, skip_serializing_if = "Vec::is_empty")]` on a `Vec<String>` reads `{"id":"1"}` as `roles: []` on every surface: Zod writes `.prefault([])`, the TypeScript type and the Dart, Swift and Kotlin members are not optional, the JSON Schema states `"default": []` and leaves the key out of `required`. The clients write the member every time under `skip_serializing_if`, since none calls the Rust function, and never under `skip_serializing`. So is one with no `default` of its own in a struct with a `#[serde(default)]` and no type parameter: it reads as the container default's value of the member. A member under `skip_serializing_if` with no `default` and no such container keeps the reading [Optional Fields](#optional-fields) describes: each surface reads its key as optional.

**On a struct, a default with no JSON keeps the key required, and the struct builds.** serde still reads such a key as its default, but no surface other than Rust is handed the default, so each one requires the key as it did before. That is the case where:

- the struct does not derive both `Serialize` and `Deserialize`, or reads or writes itself through `from`, `try_from`, `into` or `remote`;
- the type of the field names a type parameter of the struct, a lifetime or `Self`, and the field is not a list, a set or a map under the bare `#[serde(default)]`. The default of `T`, and the `Vec<T>` a function answers, hold one value for each type that fills `T`, so the text has no one value to write. A field of the same struct whose type names no parameter is read as its default;
- the field is a list, a set or a map whose type names a parameter, and serde writes it through a `serialize_with` or a `with` of the field's: only that function knows the JSON of the empty collection. Without one, a list, a set or a map under the bare `#[serde(default)]` is read as its default on every surface: the default is the empty collection at every filling, so `Vec<T>`, `VecDeque<T>`, `BinaryHeap<T>`, `HashSet<T>` and `BTreeSet<T>` are read as `[]`, and `HashMap<K, T>` and `BTreeMap<K, T>` as `{}` (`IndexSet` and `IndexMap` too, under the [`indexmap` feature](features.md#feature-flags)). The Kotlin client writes that default as `emptyList()` or `emptyMap()`, a property's initializer having no serializer of the parameter to read `[]` with;
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
- **A type that holds or flattens an untagged enum with such a member joins the enum's own schema in its Zod text.** The enum's schema is the one text the default is written into. A type that flattens the enum joins the members the enum publishes as `{Enum}$Members`, one branch each, so a payload that two members read is read as the first, as serde reads it.

#### What does not build on a variant's member

A named member of a variant under `#[serde(default)]` or `#[serde(default = "path")]` that the other surfaces cannot read as serde reads it is refused where the enum is declared, and the build stops. No key of a variant is kept required in silence. The refusal points at the member, and its message opens with the enum, the variant and the member, ``model_schema: type `Berthing`: variant `Docked`, field `dock`:``, followed by the reason. Every refused member of the enum is reported in one build. It is raised in every build that describes the type, a mobile client included: the `serde` feature with any of `typescript`, `zod`, `dart`, `swift` and `kotlin`.

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

`Vec<T>` becomes `Array<T>`, and so does every other std wrapper serde writes as a JSON array of its element: `VecDeque<T>`, `BinaryHeap<T>`, `HashSet<T>`, and `BTreeSet<T>`. Under the [`indexmap` feature](features.md#feature-flags), `indexmap::IndexSet<T>` is one more. Each is that array on the wire, so each is typed and validated as that array — the element decides what the array holds. A set is that array with its items distinct: see [A set holds each item once](#a-set-holds-each-item-once). Nesting is kept at whatever depth it is written: a `Vec<Vec<T>>` (or `HashSet<Vec<T>>`, or any other mix of those wrappers) writes an array of arrays, so it becomes `Array<Array<T>>`, `z.array(z.array(...))`, and a JSON schema whose `items` is itself an array.

`LinkedList<T>` is the one std sequence left out. It writes that same array, so the covered spellings already describe every wire form it has, and a type written with one is refused at compile time with the rewrite named: use `Vec<T>`, or `VecDeque<T>` where values are pushed at both ends. A member serde neither writes nor reads, and a member of a variant serde neither writes nor reads, is not refused: serde writes none of it.

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

`HashMap<K, V>` and `BTreeMap<K, V>` are both read as maps, and so is `indexmap::IndexMap<K, V>`, under the [`indexmap` feature](features.md#feature-flags): [The order of a map's keys](#the-order-of-a-maps-keys) says what sets the three apart. `serde_json::Map` is refused, as [A JSON value is no member](#a-json-value-is-no-member) says. The key decides what the map describes as. A `String` key is the open case: any string is a key, so the JSON schema puts the value's schema under `"additionalProperties"`, TypeScript writes `Partial<Record<string, V>>`, and Zod writes `z.record(z.string(), V)`.

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
    //  "propertyNames": {"type": "string", "pattern": "^(0|-?[1-9][0-9]*)$"}}, serialized {"7": 1.0}
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

All of them are covered under [Compilation Errors](troubleshooting.md#compilation-errors), with the exact diagnostic each produces.

Every other key is neither open nor enumerable, and none of them is refused: serde stringifies each one into a key for you. serde writes the object with the key's string form for its keys: `7` becomes `"7"`, `true` becomes `"true"`, a chrono `NaiveDate` its ISO rendering. The JSON schema describes such a map at every position, [flattened](flatten.md#intersection-types-serdeflatten) or under a key, as it describes a map keyed by a `String`, its values in `additionalProperties`, and names its keys by that text in `propertyNames`:

| key | `propertyNames` |
|-----|-----------------|
| `char` | `{"type": "string", "minLength": 1, "maxLength": 1}` |
| `NaiveDate` / `NaiveTime` / `NaiveDateTime` / `DateTime<Tz>` | the type's own description, `{"type": "string", "format": "date"}` for a `NaiveDate` |
| `u8`–`i64`, `usize`, `isize` | `{"type": "string", "pattern": "^(0\|-?[1-9][0-9]*)$"}` |
| `f32`, `f64` | `{"type": "string", "pattern": "^-?(0\|[1-9][0-9]*)(\\.[0-9]+)?([eE][+-]?[0-9]+)?$"}` |
| `bool` | `{"type": "string", "pattern": "^(true\|false)$"}` |
| a brand or an alias over one of them | its own description, a number's and a `bool`'s read as the pattern of its text |

`HashMap<u32, u8>` refuses `{"7": "x"}` and `{"x": 1}`, as serde and Zod refuse them. The pattern of a number key is serde's: an integer key admits no leading zero, which serde refuses as no number, and no `-0`, fraction or exponent, which serde reads as a float and refuses, and Zod refuses each as serde does. It states no range, as `{"type": "integer"}` states none for a value. The rule the refusals apply is refuse-what-serde-refuses, never refuse-what-is-not-a-`String`.

On the two nominal surfaces such a key keeps its own type wherever that type spells a TypeScript property key, and renders in the form serde **writes** it wherever it does not:

| key | TypeScript | Zod |
|-----|------------|-----|
| `u8`–`i64`, `usize`, `isize` | `number` | `z.unknown().check(...).pipe(z.record(z.number().int().min(0).max(255), V))` for a `u8`: each width within its own range, as [the form each client sends](client-forms.md#the-form-each-client-sends) states, each key read first as serde reads it; a `u64` or an `i64` key is `z.custom<number>((key) => typeof key === "string")`, so the record keeps each key's digits |
| `NaiveDate` / `NaiveTime` / `NaiveDateTime` | `string` | the type's own schema |
| `bool` | `"true" \| "false"` | `z.partialRecord(z.enum(["true", "false"]), V)` |
| `DateTime<Tz>` | `string` | `z.record(z.iso.datetime({ offset: true }), V)` |

So a `HashMap<u32, String>` is `Partial<Record<number, string>>` and `z.unknown().check(...).pipe(z.record(z.number().int().min(0).max(4294967295), z.string()))`, and a `HashMap<bool, String>` is `Partial<Record<"true" | "false", string>>` and `z.partialRecord(z.enum(["true", "false"]), z.string())`. `Record<K, V>` is declared `K extends keyof any`, which `boolean` and `Date` do not satisfy, and in key position `z.boolean()` rejects the `"true"` serde wrote while the date schema accepts the timestamp only to rewrite every key into a locale-dependent `Date.prototype.toString()` rendering — neither surface can carry the value form, so both carry the wire form instead. The constructor moves with the key because `z.record` over an enumerated key demands every one of its members; `z.partialRecord` (zod ≥ 4.0.0) admits a map holding one of them, or none. A brand or alias over one of those keys renders exactly as the bare key does, its name having nothing to stand in.

A key written as one of the enclosing item's own type parameters is the open case, read off that same rule. The expansion cannot see which type the instantiation will supply, but serde has already settled what any of them may write: an instantiation whose key writes as a string keys the map, and one whose key does not fails the whole map at serialization with `key must be a string` — so string keys hold for every instantiation that serializes at all. All three surfaces say exactly that and nothing more: `{"type": "object", "additionalProperties": V}`, `z.record(z.string(), V)`, and `Partial<Record<string, V>>` — the trio a `String` key earns. The value side stays described, being no parameter's business, so `HashMap<KeyType, ValueType>` on `Holder<KeyType, ValueType>` is `Partial<Record<string, ValueType>>`.

The key position is the one place TypeScript stops rendering a parameter as itself — see [Type Parameters](generics-and-brands.md#type-parameters) for what it does everywhere else — and the declaration is what forces it. `Record<K, T>` is declared `K extends keyof any`, so a declaration handing it a parameter it binds without bounding does not type-check at all, and the consumer pasting the emitted `.ts` gets the error before writing a value. Bounding the parameter would move the failure rather than fix it: the bound propagates to every name written over the item, including the erased fillings a binding annotated for the opaque value supplies. What the member gives up is naming the parameter in its key; what it gains is the guarantee serde already makes about every filling that reaches the wire.

#### The order of a map's keys

A JSON object is written with its keys in an order. The three maps describe as the same object on every surface, and each does its own thing with that order in Rust:

| The map | The order of its keys |
|---|---|
| `HashMap<K, V>` | none |
| `BTreeMap<K, V>` | sorted |
| `IndexMap<K, V>` | as read |

`indexmap::IndexMap<K, V>` and the list beside it, `indexmap::IndexSet<T>`, are read under the `indexmap` feature. The feature adds no dependency to tixschema: the generated code names the types of the crate that declares them, so that crate lists `indexmap` itself, with its `serde` feature.

```toml
tixschema = { version = "0.4", features = ["indexmap"] }
indexmap = { version = "2", features = ["serde"] }
```

```rust
use indexmap::{IndexMap, IndexSet};

#[model_schema(enable = ["jsonschema"])]
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
export type DocumentDraft = {
  data: Partial<Record<string, string>>;
  labels: Array<string>;
};

export type Document = Readonly<DocumentDraft> & $brand<"Document">;
```

Every surface writes for each of the two what it writes for its std twin: for `IndexMap<K, V>` what a `HashMap<K, V>` gives, and for `IndexSet<T>` what a `HashSet<T>` gives. That holds for TypeScript, Zod, the JSON Schema, Dart, Swift and Kotlin, and under whatever holds the field: an `Option`, a `Vec`, another map. A key is held to the rules above, so `IndexMap<Slot, V>` over a plain enum is `Partial<Record<Slot, V>>` and `z.partialRecord(Slot$Schema, V)`. A hasher written as a further argument, `IndexMap<K, V, S>` or `IndexSet<T, S>`, says nothing about the wire. A `minLength` or a `maxLength` written on an `IndexSet` field counts its items, as on any list ([List Length](validation.md#list-length-minlength-maxlength)).

The order is the Rust type's. A schema describes an object and states no order, and each generated client holds the map in its language's own: a Dart `Map` and a Kotlin `Map` keep the order of the document. A Swift client holds every map as a dictionary, which has no order, and `JSONEncoder` writes the keys of every object in hash order: a Swift client neither keeps nor writes the order of an `IndexMap`.

The [recovering decode](recovering-decode.md#recovering-decode-decode_with) walks each of these maps entry by entry, each issue at its key's path, and an `IndexSet` item by item. Its second read is serde's, so an `IndexMap` field is filled in the order of the source: a `bson` document keeps the order the row stored, and a `serde_json::Value` holds its keys in the order of the crate's `serde_json`, as read where its `preserve_order` feature is on and sorted where it is off. Under `mongodb` a map has no typed path, whichever of the three it is, and an `IndexSet` field is a `ListField`.

A `#[model_schema()]` type of the crate's own named `IndexMap` or `IndexSet`, declared above the type that holds it, keeps its name: a field written with that name is that type, with the `indexmap` feature on or off. With the feature off, any other type written `IndexMap<K, V>` or `IndexSet<T>` is refused with the feature named. See [`IndexMap` or `IndexSet` Without the Feature](troubleshooting.md#indexmap-or-indexset-without-the-feature).

#### A set holds each item once

`HashSet<T>`, `BTreeSet<T>` and, under the `indexmap` feature, `IndexSet<T>` are arrays whose items are distinct. serde reads a payload that repeats an item and keeps the item once: `{"tags":["a","a"]}` is read as the set of the one `"a"`, and a `minLength` written on the set counts what is left. Every other surface refuses the repeat:

| Surface | `{"tags":["a","a"]}`, read as a `HashSet<String>` field |
|---|---|
| Rust | reads `{"a"}` |
| JSON Schema | refuses it: the set's array writes `"uniqueItems": true` |
| Zod | refuses it with `repeats an item: a set holds each item once`, at the path `["tags"]` |
| Dart, Swift and Kotlin | `validate()` (`validationIssues()` in Dart) answers `'tags': repeats an item: a set holds each item once`, so the read refuses it |
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

A member serde neither writes nor reads, under `#[serde(skip)]` or under `skip_serializing` and `skip_deserializing` together, and every member of a variant serde neither writes nor reads, is not refused for a JSON value it holds: serde writes none of it. A member under one of the two alone is refused as any other.

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

A [service](services.md#services) operation that takes or answers either type stops the build the same way, at each type it puts on the wire, in every build with the `serde` feature, spanned on the type as written: by its full path at expansion, and by its bare name at a check the compiler evaluates beside the trait. The error names the operation and the position: `the request`, the one message the operation takes; `the reply` and `the fault`, the two arms of its `Result`; ``request header `name` ``, ``reply header `name` `` and ``fault header `name` ``, a `header_in`, `header_out` and `error_header_out` binding; and ``request part `name` ``, a `part` binding. The message declared from several arguments is a `#[model_schema()]` struct, so an argument there is refused as a field of it.

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

The functions tixschema writes that take or answer a `serde_json::Value` are unchanged: the function a [`preprocess`](preprocess.md#preprocessing-preprocess) names, `from_value_with` and `from_value_piped`, `schema_examples()`, and the arky conversions.

### `i128`, `u128`, `f16` and `f128` are no member

A member that holds an `i128`, a `u128`, an `f16` or an `f128` stops the build. No surface describes any of the four: no TypeScript, Zod, JSON Schema, Dart, Swift or Kotlin type is written for it. The type is refused by a compile error at expansion, in every build, one for each member and alias that writes it, at any depth: under an `Option`, a list, a map's key or value, a tuple and a type argument, and as the lone slot of a variant. A member serde neither writes nor reads is not refused, and neither is a name the item declares as a type parameter or the crate declares a type under above the item.

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct FieldWide {
    pub x: i128,
}
```

```text
error: model_schema: type `FieldWide`: field `x` reaches `i128`, which no surface describes: no TypeScript, Zod, JSON Schema, Dart, Swift or Kotlin type is written for it. Hold the value as an `i64` or a `u64`, or as a `#[model_schema]` brand over `String` that carries its digits.
```

### A `u64` and an `i64` are a `bigint` in TypeScript

A `u64` or an `i64` travels as the JSON number serde writes: `-7` is written `-7`. TypeScript holds it as a `bigint`, which holds every value of both types, where a JavaScript `number` holds an integer exactly only up to `2^53 - 1` in size:

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Tally {
    pub listed: Vec<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maybe: Option<i64>,
    pub signed: i64,
}
```

```json
{"listed":[0,9223372036854775807],"maybe":-9223372036854775808,"signed":-1152921504606846976}
```

| Target | `i64` | `u64` |
|---|---|---|
| TypeScript | `bigint` | `bigint` |
| Zod | `z.codec(z.union([z.bigint(), z.number().int()]), z.bigint().min(-9223372036854775808n).max(9223372036854775807n), { decode: (value) => BigInt(value), encode: (value) => value })` | the same with `.min(0n).max(18446744073709551615n)` |
| JSON Schema | `{"type": "integer"}` | `{"type": "integer"}` |
| Dart | `int`, on the Dart VM | `int`, on the Dart VM |
| Swift | `Int64` | refused, as every `u64` is: see [`u64` Under `swift` or `kotlin`](troubleshooting.md#u64-under-swift-or-kotlin) |
| Kotlin | `Long` | refused, as in Swift |

The Zod schema reads a `bigint`, and a `number` that holds the integer exactly, and answers the `bigint`: `JSON.parse('{"signed":1719624000000}')` reads as `{ signed: 1719624000000n }`. It refuses a `number` beyond `2^53 - 1` in size, which `JSON.parse` has already rounded: `9007199254740993` is read as `9007199254740992`, refused with `Too big: expected int to be <=9007199254740991`. Read such a payload with a reviver that rebuilds the integer from its source text, as the TypeScript service code does (below).

- **A bound holds the integer.** `minimum` and `maximum` are compared with the integer by `validate()` and by each client, and Zod writes them as `bigint` bounds in the sentence `validate()` writes: `#[model_schema_prop(maximum = 5)]` refuses `6` with `too large: maximum is 5, got 6`. A bound keeps the digits written, beyond `2^53` too.
- **A literal on an `i64` or a `u64` is the `bigint`**: `literal = 214` is `214n` in TypeScript, a `z.codec` to `z.literal(214n)` in Zod, and `{"type": "integer", "const": 214}` in the JSON Schema. An integer literal is held exactly, beyond `2^53` too.
- **The TypeScript service code reads and writes each integer exactly.** Each client and each dispatcher reads a JSON text through `JSON.parse` with a reviver that rebuilds each integer a `number` does not hold exactly from its source text, as a `bigint`, and writes each `bigint` through `JSON.rawJSON`, as the JSON number of its digits: `18446744073709551615` crosses as it is. A client checks the message, and each header, before it sends it. A runtime whose `JSON.parse` hands a reviver no source text reads such an integer as the rounded `number`, which the Zod schema refuses. A runtime with no `JSON.rawJSON` writes a `bigint` a `number` holds exactly as that number, and refuses any other: a client answers it as a message that failed validation, `9007199254740993 is written exactly only through JSON.rawJSON, which this runtime has not`, the transport-agnostic dispatcher rejects as a handler that throws, the `http_rest` dispatcher answers the fault of a handler that panicked, and the `mcp` server answers `-32603` with that fault.
- **A transport of the transport-agnostic client and dispatcher carries JavaScript values.** `request` and `notify` are handed the message with each `bigint` held as the `JSON.rawJSON` of its digits, which `JSON.stringify` writes exactly, and the dispatcher answers the reply the same way. A transport that reads a text with a plain `JSON.parse` reads an integer beyond `2^53 - 1` as the rounded `number`, which the Zod schema refuses. Read it as the generated code does:

```typescript
const read = (text: string): unknown =>
  JSON.parse(text, (_key, value: unknown, context?: { source?: string }) =>
    typeof value === "number" &&
    !Number.isSafeInteger(value) &&
    context?.source !== undefined &&
    /^-?(0|[1-9][0-9]*)$/.test(context.source)
      ? BigInt(context.source)
      : value,
  );
```

- **A text position holds the digits.** A path placeholder, a query parameter, a header and a text part hold the text of the number, which every client writes and every dispatcher reads exactly, a brand, a struct of one slot or an alias over the number declared above the service included. The Rust `http_rest` dispatcher reads a `u64` message, header and path placeholder up to `u64::MAX`.
- **A map's key is a `number`** in TypeScript and in `z.infer`: serde writes the key as the text of the number, and the Zod schema reads that text as serde reads it and keys the record by it, so a key beyond `2^53 - 1` in size keeps its digits. `{"counts":{"9223372036854775807":1}}` reads as an object whose `Object.entries` answer `[["9223372036854775807",1]]`; `counts[9223372036854775807]` reads the key `9223372036854776000`, the text of the nearest `number`, and answers `undefined`. The record's key schema is `z.custom<number>((key) => typeof key === "string")`, on which `z.toJSONSchema` throws, as it does on any `z.bigint()`. The TypeScript writes a `@deprecated` note over each declaration that writes such a map, so an editor strikes the name through where it is read and shows the note. `tsc --strict` prints nothing for it:

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Histogram {
    pub counts: BTreeMap<i64, u32>,
    pub labels: BTreeMap<u32, String>,
}
```

```typescript
/**
 * Histogram
 * 
 */
export type HistogramDraft = {
  /**
   * counts
   * 
   * @deprecated A map key above 2^53 - 1 (9007199254740991) in size is not held exactly as a number.
   */
  counts: Partial<Record<number, number>>;
  /**
   * labels
   * 
   */
  labels: Partial<Record<number, string>>;
};

export type Histogram = Readonly<HistogramDraft> & $brand<"Histogram">;
```

- **Where the note is written.** On a member whose type writes such a map at any depth, or names an alias, a brand or a tuple struct declared above it that carries the note. On the key a variant's slots are written under, its name or its content key. On a tuple struct, an alias and a brand whose value writes such a map, and on the name each republishes itself under. In the service code, on each method of a client and of an implementation whose message, header or part writes one, and on the `value` or the `error` of a result and of an outcome whose arm writes one. A type declared below the member that names it, or in another crate, is not looked into.
- **MongoDB stores the integer**, as serde writes it: a row an operation writes, `bson::to_document` of a model, and the value of a filter and of an update hold an `Int64`. See [What a row stores](mongodb-operations.md#what-a-row-stores).
- **The recovering decode** reads the number serde writes, and the string of its digits a stored value may hold, with no issue, and lists a string that holds no value of the type, a digit string beyond its range among them, as one `Invalid` where it is held.

### A `Vec<u8>` is a `Uint8Array` in TypeScript and a `Uint8List` in Dart

A `Vec<u8>` travels as the JSON array serde writes: `[137,80,78,71]`. TypeScript holds it as a `Uint8Array<ArrayBuffer>`, which `fetch`, `Blob`, `TextDecoder`, `crypto.subtle` and Node's `Buffer` take, and Dart as a `Uint8List`:

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageBytes {
    pub content_type: String,
    pub data: Vec<u8>,
}
```

```json
{"contentType":"image/png","data":[137,80,78,71]}
```

| Target | `Vec<u8>` |
|---|---|
| TypeScript | `Uint8Array<ArrayBuffer>` |
| Zod | `z.codec(z.union([z.instanceof(Uint8Array), z.array(z.number().int().min(0).max(255))]), z.custom<Uint8Array<ArrayBuffer>>((value) => value instanceof Uint8Array), { decode: (value) => Uint8Array.from(value), encode: (value) => Array.from(value) })` |
| JSON Schema | `{"type": "array", "items": {"type": "integer"}}`, as before |
| Dart | `Uint8List` |
| Swift | `[UInt8]`, as before |
| Kotlin | `List<UByte>`, as before |

The Zod schema reads the JSON array, or a `Uint8Array` itself, and answers the `Uint8Array<ArrayBuffer>`: `JSON.parse('{"data":[137,80,78,71]}')` reads as `{ data: Uint8Array(4) [137, 80, 78, 71] }`, and a value read once reads again to an equal value. An item that is no byte is refused before the `Uint8Array` is built, as `Uint8Array.from([256])` would build `[0]`: `[256]` and `[-1]` are refused at the item's index, with the sentence a `z.array(z.number().int().min(0).max(255))` reports, and `[1.5]` and `["1"]` are refused at the field. `Schema.encode` answers the plain array.

- **It needs TypeScript 5.7 or later.** Typed arrays are generic from 5.7, and the bare `Uint8Array` is `Uint8Array<ArrayBufferLike>`, which `Blob`, `fetch` and `crypto.subtle` refuse; `z.infer` of the schema is the published type.

- **A count holds the bytes.** `minLength` and `maxLength` on a `Vec<u8>` count the bytes of the `Uint8Array` and of the array, with the same sentences.
- **The TypeScript service code writes the JSON array.** The client and the dispatcher of every transport write a `Uint8Array` as `[137,80,78,71]`, where `JSON.stringify` alone writes `{"0":137,"1":80,"2":78,"3":71}`. A `body = "bytes"` reply is answered as a `Uint8Array` too.
- **Only a `Vec<u8>` is bytes.** A `[u8; N]`, a `[u8]` behind a reference or a `Box`, a `VecDeque<u8>`, a `BinaryHeap<u8>`, a set of `u8`, a `Vec<Option<u8>>`, a `Vec<u8>` under a `literal`, a `Vec` of any other number and a `Vec<T>` of a type parameter stay `Array<…>` in TypeScript and `List<…>` in Dart. A `Vec<Vec<u8>>` is `Array<Uint8Array<ArrayBuffer>>` and `List<Uint8List>`.
- **Dart reads and writes the same JSON.** `fromJson` reads `[137,80,78,71]` into a `Uint8List` and refuses `256` and `-1` with `expected an integer from 0 to 255, found 256`, and `toJson` writes the JSON array. `validate()` counts it as it counts a list.

### Pointers and Borrowed Values

`Box<T>`, `Rc<T>`, `Arc<T>` and `Cow<'_, T>` describe as `T`. serde writes each of them as the value it holds, with nothing of its own around it, so a field written under one is the field its inner type is — on every surface, and wherever the wrapper was written: `Box<Option<T>>` is the `Option<T>` field, optional; `Vec<Rc<T>>` is the `Vec<T>` field; `Box<str>`, `Rc<str>` and `Cow<'_, str>` are `String` fields, `str` being what a `String` writes.

This is what lets a type hold itself: `Option<Box<Self>>` is the self-reference, deferred in the generated schema the way any self-reference is. An `Rc` or `Arc` field needs serde's `rc` feature, which is where serde's impls for those two live.

`RefCell<T>`, `Cell<T>`, `Mutex<T>` and `RwLock<T>` describe as `T` on every surface the same way: serde writes each as the value it guards, returning a serialization error rather than panicking when the guard cannot be taken (an already-mutably-borrowed `RefCell`, a poisoned `Mutex`/`RwLock`) — the same fallible path a schema does not describe for any other type. What sets them apart from `Box`/`Rc`/`Arc`/`Cow` is `Deref`: none of the four implements it, so `#[model_schema_prop(pattern = ..., minLength = ..., maxLength = ..., minimum = ..., maximum = ...)]` and a [`literal`](literals-and-type-names.md#literal-values) cannot reach through one to the value it guards, and are refused at expansion, naming the wrapper, wherever they are combined with one.

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

#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Spanned {
    pub closed: RangeInclusive<u32>,
    pub from: RangeFrom<u32>,
    pub span: Range<u32>,
    pub to: RangeTo<u32>,
}
```

```typescript
export type SpannedDraft = {
  closed: { start: number; end: number };
  from: { start: number };
  span: { start: number; end: number };
  to: { end: number };
};

export type Spanned = Readonly<SpannedDraft> & $brand<"Spanned">;
```

```typescript
const SpannedDraft$RawSchema = z.strictObject({
  closed: z.strictObject({ start: z.number().int().min(0).max(4294967295), end: z.number().int().min(0).max(4294967295) }),
  from: z.strictObject({ start: z.number().int().min(0).max(4294967295) }),
  span: z.strictObject({ start: z.number().int().min(0).max(4294967295), end: z.number().int().min(0).max(4294967295) }),
  to: z.strictObject({ end: z.number().int().min(0).max(4294967295) }),
});
```

The JSON Schema of each range is an object whose `properties` are its members, every one of them `required`, with `additionalProperties: false`. A bound is described as its type is anywhere else: a date, an `ObjectId`, a brand, a model, or one of the type's own parameters, `Range<Bound>` being `{ start: Bound; end: Bound }` and the factory's argument in Zod. A range is that object wherever it is held: in an `Option`, a list, a map's value, a tuple's position and a type argument.

Dart, Swift and Kotlin each read and write the object through a type of their own, written for the field that holds the range: Dart a record with named members, `({int start, int end})`, Swift a struct named after the type and the field, `SpannedSpanRange`, and Kotlin a `@Serializable` data class. Each refuses an object that holds a key the range does not write, as serde does. None of them is the language's own range: Swift's `Range` writes `[8,13]`.

`validate()` walks a range of a type that runs rules, a model or a brand with a bound, and reports each bound under its key: a `span: Range<Waymark>`, `Waymark` a brand under `minLength = 2`, holding `"a".."b"` reports `'span.start': too short: minimum length is 2, got 1` and `'span.end': too short: minimum length is 2, got 1`, in Rust, Dart, Swift and Kotlin. The read reads a model held in a range unjudged, so the whole value is judged once. The [recovering decode](recovering-decode.md#recovering-decode-decode_with) reads a range as one whole value at its key, and under `mongodb` a range field is a `Field` of that value.

A range is read by its last name. A `#[model_schema()]` type of the crate's own named `Range`, `RangeInclusive`, `RangeFrom` or `RangeTo`, declared above the type that holds it, keeps its name, and `core::ops::Range` or `std::ops::Range` written in full is the std range. A range written by its bare name earns a check beside the type that holds it, which fails the build with `E0308` where the name is another type. A range is refused as a map key, serde writing it as an object, under `#[serde(flatten)]`, where serde writes its bounds among the members of the object that holds it, and in the lone slot of a variant under a `tag` with no `content`, where serde writes its bounds beside the tag. `pattern`, `minLength`, `maxLength`, `minimum` and `maximum` are refused on a range field: a bound on what a range holds goes on the type of its bounds, a branded newtype.

### Results

`Result<T, E>` describes as the object serde writes for it, the externally tagged form of an enum of two variants: `{"Ok": T}` or `{"Err": E}`. An arm of `()` is `null`, serde writing `Ok(())` as `{"Ok": null}`.

```rust
#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Reply {
    pub outcome: Result<u32, String>,
    pub stored: Result<(), String>,
}
```

```typescript
export type ReplyDraft = {
  outcome: { Ok: number } | { Err: string };
  stored: { Ok: null } | { Err: string };
};

export type Reply = Readonly<ReplyDraft> & $brand<"Reply">;
```

```typescript
const ReplyDraft$RawSchema = z.strictObject({
  outcome: z.unknown().check((ctx) => { if (ctx.value !== undefined && typeof ctx.value !== "string" && (typeof ctx.value !== "object" || ctx.value === null || Array.isArray(ctx.value) || Object.keys(ctx.value).length !== 1)) ctx.issues.push({ code: "custom", message: "expected value", input: ctx.value, params: { undeserializable: true } }); }).pipe(z.union([z.strictObject({ Ok: z.number().int().min(0).max(4294967295) }), z.strictObject({ Err: z.string() })]).superRefine((_read, ctx) => { const held = ctx.value; const carrying = [["Ok", "newtype variant"], ["Err", "newtype variant"]]; const keys = typeof held === "object" && held !== null && !Array.isArray(held) ? Object.keys(held) : []; const at = ctx.issues.findIndex((issue) => issue.code === "invalid_union" && (issue.path ?? []).length === 0); const refused = ctx.issues[at]; const named = typeof held === "string" ? carrying.find(([name]) => name === held) : undefined; const arm = keys.length === 1 && refused?.code === "invalid_union" ? refused.errors.find((issues) => issues.some((issue) => issue.path[0] === keys[0])) : undefined; if (named !== undefined && refused?.code === "invalid_union") ctx.issues.splice(at, 1, { code: "custom", message: "invalid type: unit variant, expected " + named[1], input: undefined, path: [named[0]], params: { variant: true } }); else if (arm !== undefined) ctx.issues.splice(at, 1, ...arm.map((issue) => ({ ...issue, input: undefined }))); }, { when: () => true })),
  stored: z.unknown().check((ctx) => { if (ctx.value !== undefined && typeof ctx.value !== "string" && (typeof ctx.value !== "object" || ctx.value === null || Array.isArray(ctx.value) || Object.keys(ctx.value).length !== 1)) ctx.issues.push({ code: "custom", message: "expected value", input: ctx.value, params: { undeserializable: true } }); }).pipe(z.union([z.strictObject({ Ok: z.null() }), z.strictObject({ Err: z.string() })]).superRefine((_read, ctx) => { const held = ctx.value; const carrying = [["Err", "newtype variant"]]; const keys = typeof held === "object" && held !== null && !Array.isArray(held) ? Object.keys(held) : []; const at = ctx.issues.findIndex((issue) => issue.code === "invalid_union" && (issue.path ?? []).length === 0); const refused = ctx.issues[at]; const named = typeof held === "string" ? carrying.find(([name]) => name === held) : undefined; const arm = keys.length === 1 && refused?.code === "invalid_union" ? refused.errors.find((issues) => issues.some((issue) => issue.path[0] === keys[0])) : undefined; if (named !== undefined && refused?.code === "invalid_union") ctx.issues.splice(at, 1, { code: "custom", message: "invalid type: unit variant, expected " + named[1], input: undefined, path: [named[0]], params: { variant: true } }); else if (arm !== undefined) ctx.issues.splice(at, 1, ...arm.map((issue) => ({ ...issue, input: undefined }))); }, { when: () => true })),
});
```

The JSON Schema of a `Result` is a `oneOf` of the two objects, each with its one key `required` and `additionalProperties: false`. An arm is described as its type is anywhere else, and an arm of `()` as `{"type": "null"}`, as [`()`](#the-unit-) is wherever it is held. An object of both keys, of neither or of another key is refused, as serde refuses it; Zod's check refuses a value that is neither a string nor an object of one key with serde's words, ``expected value``, which a service answers as `undeserializable-payload`, as the Rust dispatcher does. The check after the union refuses an object of one key by the refusal of the arm the key names, at the arm's own place, as serde names a value inside the arm it reads: `{"outcome":{"Ok":"x"},"stored":{"Ok":null}}` is refused at `outcome.Ok`, which a service names as the field, and `{"outcome":{"Maybe":1},"stored":{"Ok":null}}` at `outcome`. A `Result` is that object wherever it is held: in an `Option`, a list, a map's value, a tuple's position, a type argument, the arm of another `Result`, a brand, an alias and a variant's slot.

Dart, Swift and Kotlin each read and write the object through a type of their own, written for the field that holds the `Result`: Dart a sealed class, `DartResult{N}{Crate}`, with an `Ok` and an `Err` subclass, Swift an enum of two cases named after the type and the field, `ReplyOutcomeResult`, and Kotlin a sealed interface, `KotlinResult{N}`, with a class per arm and a serializer of its own. `{N}` numbers the shapes an item writes, by which of the arms hold `()`. An arm of `()` is a case, a subclass or an object that holds no value. Each refuses an object of both keys, of neither or of another key, as serde does. None of them is named `Result`: Swift has its own, and Kotlin imports `kotlin.Result` into every file.

`validate()` walks the arm the value holds, and reports under the key serde writes the arm under: a `frontmatter: Option<Result<Masthead, Unparsed>>`, `Masthead` holding a `title` under `minLength = 1`, whose `Ok` holds an empty title reports `'frontmatter.Ok.title': too short: minimum length is 1, got 0`, in Rust, Dart, Swift and Kotlin. The read reads a model in either arm unjudged, so the whole value is judged once, and each message is listed once. The [recovering decode](recovering-decode.md#recovering-decode-decode_with) walks into the arm, and lists an issue inside it at its full path, `frontmatter.Ok.title`. An object of both keys, of neither or of another key is one issue at the field. Under `mongodb` a `Result` field is a `Field` of the whole value.

A `Result` is read by its last name with two type arguments: a `Result<T>` of one, an alias of the crate's own, is read as it was. A `#[model_schema()]` type of the crate's own named `Result`, declared above the type that holds it, keeps its name, and `core::result::Result` or `std::result::Result` written in full is the std `Result`. A `Result` written by its bare name earns a check beside the type that holds it, which fails the build with `E0308` where the name is another type. A `Result` is refused as a map key, serde writing it as an object, under `#[serde(flatten)]`, where serde writes the key of its arm among the members of the object that holds it, and in the lone slot of a variant under a `tag` with no `content`, where serde writes the key of its arm beside the tag. `pattern`, `minLength`, `maxLength`, `minimum` and `maximum` are refused on a `Result` field: a bound on what an arm holds goes on the arm's own type. A brand over a `Result` is declared with `no_display`, a `Result` writing no text.

### The Unit `()`

`()` describes as the `null` serde writes for it, wherever it is held: a member, a list's item, a tuple's position, a type argument, an arm of a `Result`, and the slot of a [brand](generics-and-brands.md#branded-newtypes) or of a struct of one slot. TypeScript writes `null`, Zod `z.null()` and the JSON Schema `{"type": "null"}`. serde refuses the empty array: `invalid type: sequence, expected unit`. A brand over `()` is `null & $brand<"Name">`, the type Zod's `.brand()` gives `z.null()`.

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

export type UnitHolderDraft = {
  branded: UnitBrand;
  unit: null;
};

export type UnitHolder = Readonly<UnitHolderDraft> & $brand<"UnitHolder">;
```

The Dart, Swift and Kotlin clients read `()` from `null` alone and write it as `null`, wherever it is held, and refuse any other value as serde refuses it. See [The Dart read refuses what `validate()` refuses](dart.md#the-dart-read-refuses-what-validate-refuses), [A Swift read refuses what `validate()` refuses](swift.md#a-swift-read-refuses-what-validate-refuses) and [The read in the Kotlin client](kotlin.md#the-read-in-the-kotlin-client).

Flattened, serde writes `()` as no key, and beside the tag of a newtype variant under a `tag` with no `content` it writes the variant as the tag alone; it reads both back. The TypeScript type and the Zod schema intersect the object with `null` there, which admits no value, so the build stops where the `()` is written in place or reached through a type declared above, and `json_schema()` panics where it is reached through a type declared below. An `#[serde(untagged)]` enum declared above with a member over `()` stops the build too, flattened and beside a tag: serde writes that member as no key, or as the tag alone, and does not read it back as that member. Declared below, it makes `json_schema()` panic, flattened and beside a tag, and so does an alias, a [brand](generics-and-brands.md#branded-newtypes) or a struct of one slot over it, written by its name: the enum's schema module records that it has such a member, and the merge reads the record. Flattened, a member the enum's own expansion does not see as `()` -- its own type parameter filled at `()`, or a type declared below the enum that serde writes as `()` -- is still read as no key. Write `()` under a key of its own, or write a unit variant. See [Intersection Types](flatten.md#intersection-types-serdeflatten) and [Internally Tagged Enums](enums.md#internally-tagged-enums-tag-with-no-content).
