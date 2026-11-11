### Enums

#### Plain Enums

An enum whose variants are all unit variants and which names no `tag` and no `untagged` generates a TypeScript string union and `z.enum()` in Zod, matching the bare variant name Serde writes for it. Serde reads the variant from that name, or from an object of one key over `null`, `{"status":{"Active":null}}`, which the schema and every client read as `Active`; a value that is neither a string nor an object of one key, `1`, `[]`, `true`, `{}` or an object of two keys, is refused as the shape of an externally tagged enum's value is, `undeserializable-payload` naming no field, an object of one key over a value other than `null`, `{"Active":1}`, is refused naming the field under the variant, `status.Active`, and a name no variant takes naming the field, `status`. Either attribute changes what Serde writes, and the surfaces follow it rather than the string union. Naming a `tag` moves the variant name under the tag key, into an object, so the same all-unit variants are described as the tagged objects below. Naming `untagged` drops the name altogether -- Serde writes every unit variant as a bare `null` -- and the declaration is refused per variant, the way any unit variant in an untagged enum is refused under "Untagged Enums" below.

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

Under a `tag` with no `content`, serde's derive refuses the declaration, under `#[serde(skip)]` or not, as it refuses every tuple variant of no slot or of several there, and the refusal says so:

```text
model_schema: variant `Empty`: `Empty()` is a tuple variant with no field, and serde's derive refuses a tuple variant of no slot or of several under `#[serde(tag = "kind")]` with no `content`, under `#[serde(skip)]` or not: `#[serde(tag = "...")] cannot be used with tuple variants`. Write `Empty` for a unit variant.
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
    Pair(String, u32),
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
    value: z.tuple([z.string(), z.number().int().min(0).max(4294967295)]),
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

An enum with data-carrying variants that names neither `tag` nor `content` is externally tagged, which is Serde's default: the variant name is the sole key of an object holding the content, and a unit variant is that name as a bare string. The generated surfaces describe that form -- a JSON Schema `oneOf`, a TypeScript union, and a Zod `z.union([...])`. There is no field every member shares, so the Zod schema is a plain union rather than a `z.discriminatedUnion`, behind a check that refuses a value that is neither a string nor an object of one key with serde's words, ``expected value``, which serde classifies as text that is not the document: a service answers it `undeserializable-payload`, naming no field, as [What is no document](ws-rpc.md) states, and so does each client's read. A string that names a variant that carries data, `{"peal":"Swung"}` for a `Swung { arc: u32 }`, and one that names the arm of a `Result` that carries a value, `{"tally":"Ok"}`, is refused under the variant's own step, `peal.Swung` and `tally.Ok`, as serde refuses a unit variant where a struct or a newtype variant is expected, on every surface. An unknown string keeps the enum's place.

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

Generated TypeScript, each object member saying it carries no other variant's key, so an object of two variants' keys, which serde and Zod refuse, does not type-check either. Every object member declares every variant's key, so a member is told apart by its key holding a value, `value.Fields !== undefined`, where `"Fields" in value` holds of each:

```typescript
export type External = "Bare" | {
  "Fields": {
    a: string;
    b: boolean;
  };
  "Pair"?: never;
  "Single"?: never;
} | {
  "Pair": [number, number];
  "Fields"?: never;
  "Single"?: never;
} | {
  "Single": string;
  "Fields"?: never;
  "Pair"?: never;
};
```

Generated Zod:

```typescript
export const External$Schema: ZodType<External> = z.unknown().check((ctx) => { if (ctx.value !== undefined && typeof ctx.value !== "string" && (typeof ctx.value !== "object" || ctx.value === null || Array.isArray(ctx.value) || Object.keys(ctx.value).length !== 1)) ctx.issues.push({ code: "custom", message: "expected value", input: ctx.value, params: { undeserializable: true } }); }).pipe(z.union([
  z.literal("Bare"),
  z.strictObject({
    "Fields": z.strictObject({ a: z.string(), b: z.boolean() }),
  }),
  z.strictObject({
    "Pair": z.tuple([z.number().int().min(0).max(4294967295), z.number().int().min(0).max(4294967295)]),
  }),
  z.strictObject({
    "Single": z.string(),
  }),
]).superRefine((_read, ctx) => {
  const held = ctx.value;
  const carrying = [["Fields", "struct variant"], ["Pair", "tuple variant"], ["Single", "newtype variant"]];
  const keys = typeof held === "object" && held !== null && !Array.isArray(held) ? Object.keys(held) : [];
  const at = ctx.issues.findIndex((issue) => issue.code === "invalid_union" && (issue.path ?? []).length === 0);
  const refused = ctx.issues[at];
  const named = typeof held === "string" ? carrying.find(([name]) => name === held) : undefined;
  const arm = keys.length === 1 && refused?.code === "invalid_union" ? refused.errors.find((issues) => issues.some((issue) => issue.path[0] === keys[0])) : undefined;
  if (named !== undefined && refused?.code === "invalid_union") ctx.issues.splice(at, 1, { code: "custom", message: "invalid type: unit variant, expected " + named[1], input: undefined, path: [named[0]], params: { variant: true } });
  else if (arm !== undefined) ctx.issues.splice(at, 1, ...arm.map((issue) => ({ ...issue, input: undefined })));
}, { when: () => true })).meta({
  examples: []
});
```

The check after the union refuses an object of one key by the refusal of the member that key names, at its own place, as serde names a value inside the variant it reads: `{"Fields":{"a":"a","b":1}}` is refused at `Fields.b`, where the union alone answers one `invalid_union` issue at the enum. An object whose key no variant names, `{"Gone":1}`, keeps the union's issue, and a string that names a variant that carries data, `"Fields"`, is refused under that variant's step, `Fields`, as a unit variant where a struct variant is expected. A `Result` is read through the same check.

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
  z.strictObject({ type: z.literal("Wrapped") }).and(z.lazy(() => TagPayload$Schema).superRefine((_read, ctx) => { … }, { when: () => true })),
]).meta({
  examples: []
});
```

The content joins the member under `z.lazy` for the reason a flattened base does: the inner type is a `const` of its own and nothing orders one type's schema against another's, so reading the name while the union's `const` initializes would fail for any inner type declared below -- and a cycle running through a variant's content and a `#[serde(flatten)]` base has no order that puts each above the other. Deferred, every module loads whatever order it is assembled in. It joins through the check a [flattened](flatten.md#intersection-types-serdeflatten) member joins its holder through: serde reads the content from the keys the tag's object buffers, and names the enum for a value of the wrong type inside it, and so does the schema.

An externally tagged enum held by a newtype variant is written beside the tag as Serde writes it [flattened](flatten.md#intersection-types-serdeflatten): each variant as its entry, a unit variant as its name over `null`. Its own type and schema take a unit variant as its bare name, so the member is joined by each variant's entry instead, as a flattened one is. With `pub enum Palleted { Loose, Stacked { layers: u32 } }` declared above `#[serde(tag = "kind")] pub enum Docketed { External(Palleted) }`, Serde writes `Docketed::External(Palleted::Loose)` as `{"kind":"External","Loose":null}`, which the TypeScript type, `{ kind: "External"; } & ({ "Loose": null; "Stacked"?: never; } | { "Stacked": { layers: number; }; "Loose"?: never; })`, the Zod schema and the JSON Schema read, and which the Dart, Swift and Kotlin clients read and write back as Serde writes it. The tag alone, `{"kind":"External"}`, and the entries of two variants are refused, as Serde refuses them. An alias of such an enum declared above is joined the same way, held there or flattened, and so are a brand and a tuple struct of one slot over it. An enum that declares a type parameter, written with an argument, `External(Choice<u32>)`, is joined the same way, each variant's entry written at the argument, and so is an alias of one. A variant that holds the enum's own type parameter is refused where such an enum fills it: see [Intersection Types](flatten.md#intersection-types-serdeflatten).

An `ObjectId` held by a newtype variant is written beside the tag as its `$oid` key, as a [flattened one](flatten.md#intersection-types-serdeflatten) is written beside the object's own members: `{"kind":"Held","$oid":"507f1f77bcf86cd799439011"}` for `#[serde(tag = "kind")] pub enum Stamping { Held(ObjectId) }`, which every surface describes and the Dart, Swift and Kotlin clients read and write back as Serde writes it.

Only a value Serde writes as an object has members to put beside the tag. A newtype variant wrapping a string, a `char`, a number, a boolean, a sequence (a `Vec`, a slice, a set, a `VecDeque` or a `BinaryHeap`), an `Option`, a tuple of one element or more or a fixed-size array is one Serde refuses to serialize at run time, and the refusal quotes Serde's own words for it: `cannot serialize tagged newtype variant Lettered::Held containing a char` for `#[serde(tag = "kind")] pub enum Lettered { Held(char) }`, and `containing a tuple` for a fixed-size array, which Serde writes as a tuple. A multi-element tuple variant is one Serde's own derive refuses outright. A newtype variant wrapping [`()`](models.md#the-unit-) Serde writes as the tag alone and reads back, and the TypeScript type and the Zod schema, which intersect the tag's object with `null`, admit no value of it. `#[model_schema()]` rejects all of those at expansion rather than describing a value no surface reads as Serde does; name a `content` key so the value gets an object of its own, or wrap it in a struct whose fields can sit beside the tag. `Held(())` answers:

```text
error: model_schema: variant `Held` of internally tagged enum `ProbeUnitTagged` wraps `()`, which serde writes as `null`: beside the tag serde writes the variant as the tag alone, and reads it back, while the TypeScript type and the Zod schema of the variant intersect the tag's object with `null`, which admits no value. Name a `content` key so the value gets an object of its own, or write `Held` as a unit variant, which serde writes as the tag alone as well.
```

A name is not a promise of an object either. A newtype variant wrapping a plain `#[model_schema()]` enum is rejected at expansion the same way: a plain enum writes its own variant name, which Serde puts beside the tag as a key holding null and which a schema closed around the tag rejects. So is a newtype variant wrapping any other type declared above the enum that Serde writes as no object, in every build with `serde`: a [brand](generics-and-brands.md#branded-newtypes) or a struct of one slot over text, a number, a `bool` or a list, any other tuple struct, an alias of one, and an `#[serde(untagged)]` enum, or an alias of one, with a member Serde writes as no object, refused with that member's branch. A member Serde writes as a variant's name, a plain enum's or an externally tagged enum's unit variant's, is refused with its branch in the words of a plain enum: Serde writes that name beside the tag as a key holding `null` (`{"kind":"Held","Calm":null}`), and reads it back. Serde refuses every write and every read of a brand over text and of a tuple struct there (`cannot serialize tagged newtype variant ProbeReading::Held containing a string` for the brand below, and `containing a tuple struct` for a tuple struct). A brand over a struct, and an untagged enum whose every member is an object, are not refused. With `#[serde(transparent)] pub struct ProbeId(String);` declared above it, `#[serde(tag = "kind")] pub enum ProbeReading { Held(ProbeId) }` answers:

```text
error: model_schema: variant `Held` of internally tagged enum `ProbeReading` wraps `ProbeId`, which is not written as an object — its schema describes a `string`, which cannot sit beside the tag: `#[serde(tag = "kind")]` with no `content` writes the variant's data as members of the object the tag is written in, and only a value serde writes as an object has members to put there. Name a `content` key so the value gets an object of its own, or wrap it in a struct whose fields can sit beside the tag. This refusal holds in every build with serde, and only a type declared above reaches it: one declared below, or in another crate, is refused by the `const` beside the type that holds it, and one no `#[model_schema]` expanded is not refused, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.
```

A value Serde writes as a plain enum's variant name by way of a type declared above the enum is refused with the error of the plain enum itself, in every build with `serde`, and named as written: a [brand](generics-and-brands.md#branded-newtypes) or a struct of one slot over the plain enum, a generic type declared above whose value is its own type parameter, filled at the plain enum, and a type whose value is one of those. Serde writes each beside the tag as the key naming the variant, holding `null` (`{"kind":"Held","Calm":null}`), and reads it back. One that writes an `Option` of the plain enum on the way (`pub struct ProbeMaybeMoodSlot(Option<ProbeMood>);`, `ProbeWrap<Option<ProbeMood>>`) is refused with the error of an `Option`, below: Serde refuses every write of it there. With `pub enum ProbeMood { Calm, Tense }` and `#[model_schema(no_display)] pub struct ProbeMoodSlot(ProbeMood);` declared above it, `#[serde(tag = "kind")] pub enum ProbeM2 { Held(ProbeMoodSlot) }` answers:

```text
error: model_schema: variant `Held` of internally tagged enum `ProbeM2` wraps `ProbeMoodSlot`, which serde writes beside the tag as a plain enum: as a key named after the variant, holding null. TypeScript, Zod and the clients describe a plain enum as the variant's name, so none of them reads that key beside the tag. Name a `content` key so the value gets an object of its own, or wrap it in a struct whose fields can sit beside the tag. This refusal holds in every build with serde, and only a plain enum declared above reaches it: one declared below, or in another crate, is refused by the `const` beside the type that holds it, and one no `#[model_schema]` expanded is not refused, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.
```

and, with `pub struct ProbeWrap<T>(T);` declared above it, `#[serde(tag = "kind")] pub enum ProbeM4 { Held(ProbeWrap<ProbeMood>) }` answers:

```text
error: model_schema: variant `Held` of internally tagged enum `ProbeM4` wraps `ProbeWrap<ProbeMood>`, which serde writes beside the tag as a plain enum: as a key named after the variant, holding null. TypeScript, Zod and the clients describe a plain enum as the variant's name, so none of them reads that key beside the tag. Name a `content` key so the value gets an object of its own, or wrap it in a struct whose fields can sit beside the tag. This refusal holds in every build with serde, and only a plain enum declared above reaches it: one declared below, or in another crate, is refused by the `const` beside the type that holds it, and one no `#[model_schema]` expanded is not refused, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.
```

An `Option` behind a name is refused the same way, in every build with `serde`: a newtype variant wrapping a struct of one slot or a [brand](generics-and-brands.md#branded-newtypes) over an `Option`, or an alias of one, declared above the enum. Serde refuses every write of it, of `Some` and of `None` alike (`cannot serialize tagged newtype variant ProbeOptional::Held containing an optional`), though it reads `{"kind":"Held","note":"n"}` back. The same type flattened into a struct, or into a variant's members, is not refused: Serde writes a flattened `None` as no key, and reads it back. An `Option` of a value Serde writes as no object, `pub struct ProbeMaybeCountSlot(Option<u32>);` or an `Option` of an untagged enum with a member written as text, is refused beside the tag with the error of an `Option` too: Serde's tag refuses the `Option` before it reads what it holds. Flattened, it is refused with the branch Serde writes as no object. A type declared below the enum, or in another crate, makes `json_schema()` panic: the merge reads a `null` among the choices of the content as an `Option`'s or as `()`'s, and refuses it ahead of every other choice. With `pub struct ProbeFields { pub note: String }` and `pub struct ProbeMaybe(Option<ProbeFields>);` declared above it, `#[serde(tag = "kind")] pub enum ProbeOptional { Held(ProbeMaybe) }` answers:

```text
error: model_schema: variant `Held` of internally tagged enum `ProbeOptional` wraps `ProbeMaybe`, which serde writes as an optional, which cannot sit beside the tag: `#[serde(tag = "kind")]` with no `content` writes the variant's data as members of the object the tag is written in, and only a value serde writes as an object has members to put there. serde refuses this one at run time — `cannot serialize tagged newtype variant ProbeOptional::Held containing an optional`. Name a `content` key so the value gets an object of its own, or wrap it in a struct whose fields can sit beside the tag. This refusal holds in every build with serde, and only a type declared above reaches it: one declared below, or in another crate, is not refused here, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.
```

A generic type declared above the enum whose value is its own type parameter -- a [brand](generics-and-brands.md#branded-newtypes), a struct of one slot or an alias over the parameter, under `Box` or through another such type -- is read at what fills that parameter, in every build with `serde`. Filled at text, a number, a `bool`, a list, or a type declared above that Serde writes as no object, it is refused with the error of a value written as no object; filled at an `Option`, or at a type declared above that Serde writes as one, it is refused with the error of an `Option`. Serde refuses every write of one filled at text, a number, a `bool`, a list or an `Option` (`cannot serialize tagged newtype variant ProbeFilledText::Held containing a string`), and every read of one filled at text. Filled at a struct, at a type declared below, or at the enum's own type parameter, it builds. With `pub struct ProbeWrap<T>(T);` declared above it, `#[serde(tag = "kind")] pub enum ProbeFilledText { Held(ProbeWrap<String>) }` answers:

```text
error: model_schema: variant `Held` of internally tagged enum `ProbeFilledText` wraps `ProbeWrap<String>`, which is not written as an object — its schema describes a `string`, which cannot sit beside the tag: `#[serde(tag = "kind")]` with no `content` writes the variant's data as members of the object the tag is written in, and only a value serde writes as an object has members to put there. Name a `content` key so the value gets an object of its own, or wrap it in a struct whose fields can sit beside the tag. This refusal holds in every build with serde, and only a type declared above reaches it: one declared below, or in another crate, is refused by the `const` beside the type that holds it, and one no `#[model_schema]` expanded is not refused, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.
```

and `#[serde(tag = "kind")] pub enum ProbeFilledOptional { Held(ProbeWrap<Option<ProbeFields>>) }` answers:

```text
error: model_schema: variant `Held` of internally tagged enum `ProbeFilledOptional` wraps `ProbeWrap<Option<ProbeFields>>`, which serde writes as an optional, which cannot sit beside the tag: `#[serde(tag = "kind")]` with no `content` writes the variant's data as members of the object the tag is written in, and only a value serde writes as an object has members to put there. serde refuses this one at run time — `cannot serialize tagged newtype variant ProbeFilledOptional::Held containing an optional`. Name a `content` key so the value gets an object of its own, or wrap it in a struct whose fields can sit beside the tag. This refusal holds in every build with serde, and only a type declared above reaches it: one declared below, or in another crate, is not refused here, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.
```

A type declared above the enum whose value is such a generic type, filled -- a struct of one slot, a [brand](generics-and-brands.md#branded-newtypes) or an alias over it -- is read at that filling as well, in every build with `serde`. `pub struct ProbeTextFilled(ProbeWrap<String>);` is written as text and refused with the error of a value written as no object, and `pub struct ProbeMaybeFilled(ProbeWrap<Option<ProbeFields>>);` is written as an `Option` and refused with the error of an `Option`, as the type over the filling itself is. Serde refuses every write and every read of the first there (`cannot serialize tagged newtype variant ProbeHeldTextFilled::Held containing a string`), and every write of the second. Filled at a struct, or at a type declared below, it builds. With `pub struct ProbeTextFilled(ProbeWrap<String>);` declared above it, `#[serde(tag = "kind")] pub enum ProbeHeldTextFilled { Held(ProbeTextFilled) }` answers:

```text
error: model_schema: variant `Held` of internally tagged enum `ProbeHeldTextFilled` wraps `ProbeTextFilled`, which is not written as an object — its schema describes a `string`, which cannot sit beside the tag: `#[serde(tag = "kind")]` with no `content` writes the variant's data as members of the object the tag is written in, and only a value serde writes as an object has members to put there. Name a `content` key so the value gets an object of its own, or wrap it in a struct whose fields can sit beside the tag. This refusal holds in every build with serde, and only a type declared above reaches it: one declared below, or in another crate, is refused by the `const` beside the type that holds it, and one no `#[model_schema]` expanded is not refused, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.
```

An `#[serde(untagged)]` enum declared above whose member names such a generic type, filled, is read at that filling as well, in every build with `serde`: `Text(ProbeWrap<String>)` is a member written as text, as `Text(String)` is, and the enum is refused with that member's branch. Serde refuses every write of that member there (`cannot serialize tagged newtype variant ProbeHeldWrapMember::Held containing a string`). Filled at a struct, or at a type declared below, the member builds. With `#[serde(untagged)] pub enum ProbeWrapMember { Obj { a: String }, Text(ProbeWrap<String>) }` declared above it, `#[serde(tag = "kind")] pub enum ProbeHeldWrapMember { Held(ProbeWrapMember) }` answers:

```text
error: model_schema: variant `Held` of internally tagged enum `ProbeHeldWrapMember` wraps `ProbeWrapMember`, which writes a union member that is not an object — its branch 2 describes a `string`, which cannot sit beside the tag: `#[serde(tag = "kind")]` with no `content` writes the variant's data as members of the object the tag is written in, and only a value serde writes as an object has members to put there. Name a `content` key so the value gets an object of its own, or wrap it in a struct whose fields can sit beside the tag. This refusal holds in every build with serde, and only a type declared above reaches it: one declared below, or in another crate, is refused by the `const` beside the type that holds it, and one no `#[model_schema]` expanded is not refused, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.
```

An `#[serde(untagged)]` enum declared above whose member is its own type parameter -- itself, under `Box` or an `Option`, in such a generic type, or handed on to another such enum -- is read at what fills that parameter, in every build with `serde`, and so is an alias of it, filled. Held as `ProbeOwnMember<String>`, the member `Held(X)` is written as text, as `Held(String)` is, and the enum is refused with that member's branch, named with its arguments. Serde refuses every write of that member there (`cannot serialize tagged newtype variant ProbeHeldOwnMember::Held containing a string`). Filled at a struct, at the holder's own type parameter, or declared below, it builds. With `#[serde(untagged)] pub enum ProbeOwnMember<X> { Obj { a: String }, Held(X) }` declared above it, `#[serde(tag = "kind")] pub enum ProbeHeldOwnMember { Held(ProbeOwnMember<String>) }` answers:

```text
error: model_schema: variant `Held` of internally tagged enum `ProbeHeldOwnMember` wraps `ProbeOwnMember<String>`, which writes a union member that is not an object — its branch 2 describes a `string`, which cannot sit beside the tag: `#[serde(tag = "kind")]` with no `content` writes the variant's data as members of the object the tag is written in, and only a value serde writes as an object has members to put there. Name a `content` key so the value gets an object of its own, or wrap it in a struct whose fields can sit beside the tag. This refusal holds in every build with serde, and only a type declared above reaches it: one declared below, or in another crate, is refused by the `const` beside the type that holds it, and one no `#[model_schema]` expanded is not refused, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.
```

A struct of one slot or a [brand](generics-and-brands.md#branded-newtypes) declared above whose value is an `#[serde(untagged)]` enum with a member Serde writes as no object is read as that enum, in every build with `serde`, as an alias of it is: over the enum itself, through such a generic type (`ProbeWrap<ProbeTextMember>`), or over the enum filled at its own parameter (`ProbeOwnMember<String>`), it is refused with that member's branch, and named as written. Serde refuses every write of that member there (`cannot serialize tagged newtype variant ProbeHeldSlotText::Held containing a string`). Over an enum whose every member is an object, or one declared below, it builds. With `#[serde(untagged)] pub enum ProbeTextMember { Obj { a: String }, Text(String) }` and `#[model_schema(no_display)] pub struct ProbeSlotText(ProbeTextMember);` declared above it, `#[serde(tag = "kind")] pub enum ProbeHeldSlotText { Held(ProbeSlotText) }` answers:

```text
error: model_schema: variant `Held` of internally tagged enum `ProbeHeldSlotText` wraps `ProbeSlotText`, which writes a union member that is not an object — its branch 2 describes a `string`, which cannot sit beside the tag: `#[serde(tag = "kind")]` with no `content` writes the variant's data as members of the object the tag is written in, and only a value serde writes as an object has members to put there. Name a `content` key so the value gets an object of its own, or wrap it in a struct whose fields can sit beside the tag. This refusal holds in every build with serde, and only a type declared above reaches it: one declared below, or in another crate, is refused by the `const` beside the type that holds it, and one no `#[model_schema]` expanded is not refused, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.
```

A generic alias, struct of one slot or [brand](generics-and-brands.md#branded-newtypes) declared above whose value is an `#[serde(untagged)]` enum whose member is its own type parameter, filled at the holder's own type parameter (`pub type ProbeOwnAliasOf<T> = ProbeOwnMember<T>;`), is read at what fills the holder, in every build with `serde`, and so are an alias of one, a generic type filled at one, and an untagged enum with a member over one: held as `ProbeOwnAliasOf<String>`, it is refused as `ProbeOwnMember<String>` is, with that member's branch, and named with its arguments. Serde refuses every write of that member there (`cannot serialize tagged newtype variant ProbeHeldOwnAliasOf::Held containing a string`). Filled at a struct, at the holder's own type parameter, or over an enum declared below, it builds. With `ProbeOwnMember<X>` and `#[model_schema(default_types(T = String))] pub type ProbeOwnAliasOf<T> = ProbeOwnMember<T>;` declared above it, `#[serde(tag = "kind")] pub enum ProbeHeldOwnAliasOf { Held(ProbeOwnAliasOf<String>) }` answers:

```text
error: model_schema: variant `Held` of internally tagged enum `ProbeHeldOwnAliasOf` wraps `ProbeOwnAliasOf<String>`, which writes a union member that is not an object — its branch 2 describes a `string`, which cannot sit beside the tag: `#[serde(tag = "kind")]` with no `content` writes the variant's data as members of the object the tag is written in, and only a value serde writes as an object has members to put there. Name a `content` key so the value gets an object of its own, or wrap it in a struct whose fields can sit beside the tag. This refusal holds in every build with serde, and only a type declared above reaches it: one declared below, or in another crate, is refused by the `const` beside the type that holds it, and one no `#[model_schema]` expanded is not refused, and where a schema surface is on it keeps its TypeScript and Zod intersection, and is refused by the JSON surface when the merge runs.
```

A type declared below the enum, or in another crate, is not looked into: the expansion of the enum cannot see what it writes, so one that turns out not to be written as an object is caught when `json_schema()` runs, with a diagnostic naming the variant, the inner type and the same remedy. The TypeScript and Zod intersections for that case are a known gap: they are written, and only the JSON Schema surface refuses them. One that writes an `Option` is refused by no surface: the JSON Schema reads its `None` as the tag alone, `{"kind":"Held"}`.

A named type Serde writes as a map has no members to name either. A newtype variant whose value writes a map stops the build, in every build with `serde`: Serde writes the tag and the map's entries at one level, as it does for [a map flattened beside a tag](flatten.md#intersection-types-serdeflatten), and no TypeScript type describes both. `{ kind: "Counted"; } & Partial<Record<string, number>>` refuses `{"kind":"Counted","north":2}`, which Serde reads. A value writes a map where its type, through `Box`, is declared above the enum as a struct whose one member is a flattened map, a [brand](generics-and-brands.md#branded-newtypes) or a struct of one slot over a map, an alias of a map, or an `#[serde(untagged)]` enum with a variant that writes one, or is a generic type of those shapes filled at a map. A variant that holds the enum's own type parameter, `Counted(T)` or `Counted(Wrap<T>)`, builds, and the build stops where a map fills that parameter, naming the tag. Name a `content` key so the map gets an object of its own, or write the map under a key of its own. The same value under an enum written under its variant's name, under a `content` key or untagged writes no tag beside the map, and builds. A type declared below the enum, or in another crate, is not looked into: the expansion of the enum cannot see what it writes. With `pub struct Tally { #[serde(flatten)] counts: BTreeMap<String, u8> }` declared above it, `#[serde(tag = "kind")] pub enum Filled { Counted(Tally) }` answers:

```text
error: model_schema: type `Filled`: variant `Counted`, field `0`: `Tally` writes a map, which cannot sit beside the tag `kind` — serde writes the tag and the map's entries at one level, and no TypeScript type describes both. Name a `content` key so the map gets an object of its own, or write the map under a key of its own.
```

A variant under `#[serde(skip)]` is refused none of these: Serde neither writes nor reads it, so nothing it holds sits beside the tag, and `#[serde(tag = "kind")] pub enum Skipped { Open { note: String }, #[serde(skip)] Hidden(String) }` builds. Serde writes `Skipped::Open` as `{"kind":"Open","note":"a"}` and reads it back, and answers `unknown variant` for `{"kind":"Hidden"}`. A tuple variant of several slots is refused all the same, as Serde's derive refuses it skipped or not. A variant under `skip_serializing` alone, which Serde still reads beside the tag, or under `skip_deserializing` alone, which it still writes there, is refused as any other variant. No surface describes a variant under `skip`: see [A Variant Serde Neither Writes Nor Reads](#a-variant-serde-neither-writes-nor-reads).

#### A Variant Serde Neither Writes Nor Reads

A variant under `#[serde(skip)]`, or under `skip_serializing` and `skip_deserializing` together, is written into no payload and read out of none, under every tagging: serde answers `the enum variant Skipped::Closed cannot be serialized` for a write of it, and refuses its payload, with `unknown variant` under a name or a tag and `data did not match any variant` untagged. So it is described as a [member under `skip`](models.md#optional-fields) is, on no surface. TypeScript writes no member of the union for it, Zod no branch, the JSON Schema no branch of its `oneOf` or `anyOf`, `{Enum}$Variant` answers `""` for its name, and a plain enum's `enum_members()` leaves it out. The Dart, Swift and Kotlin clients write no class and no case for it, so each refuses its payload, as serde does.

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Skipped {
    Open { note: String },
    #[serde(skip)]
    Closed { reason: String },
}
```

```typescript
export type Skipped = {
  kind: "Open";
  note: string;
};

const Skipped$RawSchema = z.discriminatedUnion("kind", [z.strictObject({
  kind: z.literal("Open"),
  note: z.string(),
})]);
```

The schema refuses `{"kind":"Closed","reason":"r"}`, as serde does. Whether an enum is a plain one is read off every variant it declares: `enum Cartage { Light, Heavy, #[serde(skip)] Bulk(String) }` is described as the externally tagged union of `"Light"` and `"Heavy"`, the payloads serde writes for it.

What serde does not write or read is held to no rule of what it writes or reads. A skipped variant builds where the same variant written would be refused: a tuple variant of no slot, `Nothing()`, but under a tag alone, where Serde's derive refuses every tuple variant of no slot or of several, skipped or not; a member that holds a `u64` or a `usize` under `swift` or `kotlin`, a `LinkedList` or an `OsString`, a map keyed by a value serde writes as no string, or a JSON value, as a member serde neither writes nor reads does; a variant under a `content` key that declares one slot and takes it off the wire; a member that is a bare `Option` with no `skip_serializing_if`; a unit variant whose name breaks the enum's own `pattern`; a unit variant and a tuple variant of several slots of an untagged enum; and what it holds beside a tag (see [Internally Tagged Enums](#internally-tagged-enums-tag-with-no-content)). A holder that flattens the enum is refused for nothing such a variant holds. Its members are still held to their own `#[model_schema_prop]` keys, and `validate()` still checks a value of it built in Rust. A variant under `skip_serializing` alone, which serde reads, or under `skip_deserializing` alone, which serde writes, is described and held to every rule as any other variant. serde reads no member of a variant under `skip` or `skip_deserializing`, and neither does the read the macro writes, so such a member's type need not be one serde reads: a model that derives `Serialize` alone, or a `std::sync::OnceLock`, builds there as it does under serde's own derive.

An enum whose every variant serde neither writes nor reads has no value to describe, and does not build:

```text
error: model_schema: type `Becalmed`: every variant is under `#[serde(skip)]`, or under `skip_serializing` and `skip_deserializing` together, so serde writes no value of it and reads none, and no surface can describe it. Let serde write or read one of its variants.
```

Nor does an enum that declares no variant, under a tag or not:

```text
error: model_schema: type `Becalm`: declares no variant, so serde writes no value of it and reads none, and no surface can describe it. Declare a variant serde writes or reads.
```

#### A Braced Variant of No Member

A braced variant serde reads no member of, one that declares none or one whose every member serde neither writes nor reads, is written as an empty object under every tagging: `{"Off":{}}` with no tag attribute, `{"kind":"Off"}` under a tag alone and `{"kind":"Off","body":{}}` under a tag and a content key. It refuses a key in that object, and so does every other variant of a strict enum that holds it:

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub enum Signal {
    Off {},
    On { level: u8 },
}
```

serde, Zod, the JSON Schema and the Dart, Kotlin and Swift reads refuse `{"Off":{"stray":1}}` and `{"On":{"level":1,"stray":1}}`, and read `{"Off":{}}` and `{"On":{"level":1}}`; serde names the key, ``unknown field `stray`, there are no fields``. The macro reads the variant through a read it writes, hung on it with `#[serde(deserialize_with = "...")]`, which builds a member serde skips from its `#[serde(default = "...")]`, else from `Default`. Kotlin declares the variant as a plain `class`, which kotlinc takes with no property. Marked `#[model_schema(nonstrict)]`, the enum reads past the key on every surface.

#### A Variant Under `#[serde(untagged)]`

serde writes a variant under `#[serde(untagged)]` as its content alone, under no name, inside an enum it writes with names too: `Loose("x")` below is written `"x"`. No surface describes such a variant, so an enum with no `untagged` of its own does not build with one, with no tag attribute, under a `tag`, and under a `tag` and a `content` key, a plain enum among them:

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub enum Mixed {
    Named { label: String },
    #[serde(untagged)]
    Loose(String),
}
```

```text
error: model_schema: type `Mixed`: variant `Loose` is under `#[serde(untagged)]`, so serde writes it with no name, as its content alone, inside an enum it writes with names, and no surface can describe it. Take `#[serde(untagged)]` off the variant, or put it on the whole enum.
```

Every such variant of the enum is reported. One written ahead of a variant that is not under `#[serde(untagged)]` is refused in the words of serde's derive, which refuses it, skipped or not:

```text
error: model_schema: type `Ahead`: variant `Loose` is under `#[serde(untagged)]` ahead of a variant that is not, which serde's derive refuses: `all variants with the #[serde(untagged)] attribute must be placed at the end of the enum`. Take `#[serde(untagged)]` off the variant, or put it on the whole enum.
```

An enum under `#[serde(untagged)]` as a whole is described as before, with the attribute on a variant as well. A variant under `#[serde(untagged)]` that serde neither writes nor reads is refused only where it stands ahead of a variant that is not. Under a `tag` with no `content`, a tuple variant of no slot or of several is refused as [Tuple Variants](#tuple-variants) states, untagged or not.

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
- **Not reached:** the items of a list, so a field written `Vec<String>` is held to nothing; a field typed with a named type, a brand included, which carries its own check; a number, a `bool`, a map and a tuple; a field typed with one of the enum's own type parameters; and a field that carries a [`literal`](literals-and-type-names.md#literal-values), which takes no other rule.
- **A member of an untagged enum** is checked by the read, which is what chooses the variant, and by `validate()`, the Zod schema and the JSON Schema.
- **A member of a tagged enum** is checked by `validate()`, which the enum's own read runs on the value it read, and by the Zod schema, the JSON Schema and the [recovering decode](recovering-decode.md#recovering-decode-decode_with). See [Where a constraint is checked](validation.md#where-a-constraint-is-checked).

`validate()` reports a field at [the path serde writes it at](validation.md#the-path-a-message-names), as it does for [a check written on the member](validation.md#a-slot-of-a-tuple-variant): below the variant's name or the content key where serde writes one, and under no name for an untagged newtype variant's slot. A validator is published in every build.

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
