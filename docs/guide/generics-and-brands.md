### Type Parameters

A struct, a tuple struct, an enum, an alias and a branded newtype can each name their own type parameters, and every surface reads such a name under one rule.

**TypeScript binds the parameter for real.** It is a type surface, and the declaration it emits carries the parameter list: `export type Wrapper<T> = { id: T }` is a generic type, and a use site fills `T` in. The one position where it does not is a map's key, which states `string` on all three surfaces — see [Collections and Maps](models.md#collections-and-maps).

**JSON Schema writes the document of whatever fills the parameter.** JSON Schema has no type parameters at all, so a document exists only at one filling: the type `default_types` declares where the document stands on its own, and the reference site's arguments where a field embeds it. The shape around the parameter -- an array, a map's keys, a tuple's arity -- is described either way.

**An argument is written as serde writes a value outside a field.** serde writes the `None` of an `Option` that fills a parameter as `null` wherever the parameter is held, under the key of a field typed with it among them, which `skip_serializing_if = "Option::is_none"` cannot be written on. So `Wrapper<Option<u32>>` is `Wrapper<number | null>` in TypeScript, and its factory is handed `z.nullable(z.number().int().min(0).max(4294967295))`, which reads `null` as `null`. A default declared at an `Option` is written into `X$SchemaDefault` and its annotation the same way.

**Zod publishes a factory rather than a schema -- and a default beside it.** A Zod schema is a runtime value and TypeScript generics do not exist at runtime, so there is no one value a generic type could publish: the caller has to say what fills each parameter before anything can validate. A generic type therefore exports `X$SchemaFactory`, a function taking one required schema argument per parameter, and a field written with a parameter composes the argument bound for it. It also exports `X$SchemaDefault`: the factory called at the type's own declared `default_types`, memoized like any other call, so a consumer who wants the ordinary filling never constructs the argument list by hand.

**A type that lists `enable = ["jsonschema"]` needs a declared default type for every type parameter.** JSON Schema has no type parameters, so its document has to be built from one concrete filling, and `default_types` is where that filling is named — see [Declaring the default type](#declaring-the-default-type) below.

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
#[model_schema(enable = ["jsonschema"], default_types(IdType = String))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DocumentId<IdType>(pub IdType);

#[model_schema(enable = ["jsonschema"], default_types(IdType = String, DateType = f64))]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EcmDocument<IdType, DateType> {
    pub document_id: DocumentId<IdType>,
    pub created_at: DateType,
}
```

The pairs may be written in any order, and the argument sits beside every other item-level argument -- `name`, `pattern`, `minLength`, `maxLength`, `no_display` -- changing none of them. A lifetime and a const parameter name no type, so neither takes an entry.

A lifetime is dropped from every emitted surface, neither TypeScript nor JSON Schema having one and a borrowed value writing exactly what its owned form writes. A const renders in one position only -- an array length, which [describes as an unbounded array](models.md#collections-and-maps) -- so a const handed to a written type as an *argument* is refused, spanned on the argument: an argument list is read as a list of types, and the const standing in one would have the JSON document call into a module named after it and the TypeScript declaration write a name it binds nothing for. A const no written type carries is untouched.

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
| A type parameter with no entry | Refused only on a type that lists `enable = ["jsonschema"]`, spanned on the parameter. Nothing else reads the default, so the same item compiles untouched without that entry. |
| `default_types` on an item declaring no type parameter | Refused, there being nothing for a filling to fill. |
| An entry filled at a type the macro cannot describe -- `i128`, `u128`, `f16`, `f128` | Refused where `zod` is on and on a type that lists `enable = ["jsonschema"]`, spanned on the filling -- those are the two surfaces that read a declared filling. A name the type dispatch has no arm for is read as another `#[model_schema]` item, which is right for a sibling declared below and gibberish for a primitive: the JSON emission would name an `i128_schema` module nothing publishes, and the zod `$SchemaDefault` an `i128$Schema` binding no generated module exports. Only the reserved primitive names the dispatch answers for nowhere are refused, so a forward-referenced sibling keeps compiling -- and so does a filling at `char`, which describes as the one-character string serde writes for it. |

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

One consequence is worth stating outright: a parameter is a value only where a factory binds one for it, and a JSON document is written at one filling rather than for the type as declared — so no `model_schema_prop` bound may be spelled against a type parameter, and a *field* typed with one is refused for the same reason, under `Option` and the transparent wrappers too. A list of the parameter is a list: `minLength` and `maxLength` on a `Vec<IdType>` [count its items](validation.md#list-length-minlength-maxlength). A branded newtype's own `pattern`/`minLength`/`maxLength` are the one exception: applied directly to a bare-parameter inner, they read the parameter's *declared default* rather than the parameter itself — see [Branded Newtype Validation Constraints](#branded-newtype-validation-constraints).

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

A brand exists with tixschema's `serde` feature, and its rules then hold whatever the other features. Without the feature no `#[serde(...)]` attribute is read, so a `#[serde(transparent)]` struct is a struct like any other, and a `pattern`, `minLength` or `maxLength` written on it is refused, naming the feature:

```text
error: model_schema: type `Username`: `minLength` is supported only on a branded newtype: a `#[serde(transparent)]` struct serde writes as the value of one field. A branded newtype is read with tixschema's `serde` feature, which this build has off. On a field, write it in `#[model_schema_prop(...)]`.
```

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

- The brand is published under the Rust ident, spelled exactly as written, unless `#[model_schema(name = "...")]` names another -- see [Published Names](models.md#published-names-name). The name reaches the surface twice: as the exported type and as the brand tag the values carry.
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

You can add `pattern`, `minLength`, and `maxLength` constraints directly on the `#[model_schema()]` attribute for branded newtypes. Constraints are enforced in three places: the generated Zod schema, serde deserialization, and a `validate()` method on the type. A brand is read as a struct or an enum is: its own read, then `validate()`, so a read [refuses what `validate()` refuses](validation.md#a-read-refuses-what-validate-refuses), with every message, and a type that holds the brand reads it with no judgement of its own and names it at its full path.

The three checks are written on a branded newtype, and on [an enum](enums.md#a-check-written-on-the-enum), which holds every string it carries to them. On a struct that is no brand and on a type alias they are refused at expansion, and a check on a field is written in `#[model_schema_prop(...)]`.

A bound written here holds every value of the brand. One that should hold a single field is written on that field instead, and the brand keeps none: see [A field typed with a branded string](validation.md#a-field-typed-with-a-branded-string).

**The inner type has to be one whose schema is a string, or a list** — `String`, `PathBuf`, `ObjectId`, a chrono date/time type, a named type whose own schema is one of those, or a list written on the brand itself. A numeric, boolean, map, tuple or opaque inner is rejected at expansion time, because the three constraints are string checks and each surface would read them differently: Zod's `.min`/`.max` become bounds on the value itself, JSON Schema ignores `minLength`/`maxLength`/`pattern` outside `"type": "string"`, and `validate()` measures the inner's `Display` rendering.

**A brand over a list takes `minLength` and `maxLength`, which count its items**, as they do on [a list field](validation.md#list-length-minlength-maxlength):

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

**One of the brand's own type parameters is different: it reads its declared default instead of being refused.** A bare parameter is not itself a type the checks could measure, but `#[model_schema]` already requires (on a type that lists `"jsonschema"`) or defaults (elsewhere) a concrete filling for every parameter — see [Declaring the default type](#declaring-the-default-type) — and that filling is exactly as concrete as a named type's own schema. So the guard asks the same question of the declared default it asks of a named inner: a string-shaped default carries the checks, a non-string one is refused in its place. The checks land once, on `$SchemaDefault`'s argument for that parameter; the factory itself stays unconstrained, so a caller filling the parameter with something else — an `ObjectId` schema, say — is never held to bounds meant for the default:

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

`validate()` is written in the brand's own `impl<IdType> DocumentId<IdType>`, beside the schema delegates and under no bound the declaration does not write, so every filling has it and a type that holds the brand at any filling runs it. The string checks run where the filling is the declared default and at no other: the method asks which filling it holds. A `validate` of your own on one filling [does not build](validation.md#a-type-that-declares-a-parameter).

A brand's checks are asked of the value where Rust can measure it. When the inner is the parameter, that is the declared default: a default that is a path is measured as a path and asks no `Display`, a string default is measured by its text, and the parameter itself is never asked for `Display`. When the inner names no parameter (a `String` or a `PathBuf` beside a `PhantomData<Marker>` parameter), the brand is held to its checks at every filling. An entry `default_types` leaves out is the standard `String`, even beside a type of yours named `String`. With a string check on a generic brand, a declared default must be one the macro can see every lifetime of: a standard type, or a type `#[model_schema]` was written on above the brand and is written with all its lifetimes (`Tag<'static>`, `Tag<'_>`). A plain alias, a type of another crate, a type declared below the brand, a type written without a lifetime its declaration has, and an alias that hides one are refused at the `default_types` entry, with the type to write instead.

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

An entry `default_types` leaves out falls back to `String`, the same fallback a parameter with no declared filling gets everywhere else the declaration is read — so a constrained bare-parameter brand with no `default_types` at all still compiles. Only a type that lists `enable = ["jsonschema"]` requires a declared default type for every parameter, through the unrelated requirement in the table above.

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
