## Compiler-Validated Examples

You can provide compiler-validated example values for your types. Every generated Zod schema holds them in its `.meta()` under `examples`: a list with one item per example block, in the order written, and an empty list where the type writes none. Examples are written in doc comment code blocks and fully type-checked at compile time. An example expression builds the type's [draft](models.md#a-draft-and-a-valid-type) or its valid type: a struct or an enum writes `UserDraft { .. }` or `ShapeDraft::Circle { .. }`, a constant such as `SessionRole::Auditor`, `.into()` on a draft that declares no rule, into a binding that names the valid type (`let label: Label = LabelDraft { .. }.into(); label`; a bare `.into()` or `Default::default()` has no type to build and is `E0283`, so name it), or a draft followed by `.validate()?`. A brand has no draft, so its example builds the brand itself.

A block may use `?` on any `Result` whose error implements `Debug` (the `Vec<String>` of `new` and `validate()`, a `ParseIntError`, a `bson::oid::Error`). A `?` that fails makes the type's `schema_examples()` panic with `example N of `Name` failed:` and the error's `Debug` text, and makes the type's example test fail where the type has a `validate()`. A block that holds an `Option` turns it into an error with `.ok_or("..")?`:

```rust
/// A correlation id
/// ```rust example
/// CorrelationId::new("20e690df-a407-4468-adda-76d0b84913f5")?
/// ```
#[model_schema(minLength = 1)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CorrelationId(String);
```

```rust
/// User profile
/// ```rust example
/// UserDraft {
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
  age: z.number().int().min(0).max(4294967295),
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
/// UserDraft {
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
/// DataTypeDraft::Numeric
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
/// ShapeDraft::Circle { radius: 2 }
/// ```
/// ```rust example
/// ShapeDraft::Square { side: 3 }
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
const ShapeDraft$RawSchema = z.discriminatedUnion("kind", [z.strictObject({
  kind: z.literal("Circle"),
  radius: z.number().int().min(0).max(4294967295),
}), z.strictObject({
  kind: z.literal("Square"),
  side: z.number().int().min(0).max(4294967295),
})]).superRefine((_read, ctx) => { ctx.issues.splice(0, ctx.issues.length, ...ctx.issues.map((issue) => { const buffered = ((issue) => issue.code === "custom" && issue.params?.variant === true ? { ...issue, path: [] } : issue.code === "custom" && issue.params?.undeserializable === true && issue.message === "expected value" ? { ...issue, message: ((held) => held === undefined ? "invalid type: expected string or map" : typeof held === "object" && held !== null && !Array.isArray(held) ? "invalid value: map, expected map with a single key" : "invalid type: " + ((held) => typeof held === "number" ? (Number.isInteger(held) ? "integer `" : "floating point `") + held + "`" : Object.prototype.toString.call(held) === "[object Number]" ? "floating point `" + Number(held) + "`" : typeof held === "string" ? "string " + JSON.stringify(held) : typeof held === "boolean" ? "boolean `" + held + "`" : Array.isArray(held) ? "sequence" : held === null ? "unit value" : "map")(held) + ", expected string or map")(issue.input), params: undefined, path: [] } : undefined)(issue); if (buffered !== undefined) return buffered; const steps = issue.path ?? []; const last = steps[steps.length - 1]; let above = ctx.value; for (const step of steps.slice(0, -1)) above = typeof above === "object" && above !== null ? Object.getOwnPropertyDescriptor(above, step)?.value : undefined; const absent = last !== undefined && !(typeof above === "object" && above !== null && Object.prototype.hasOwnProperty.call(above, last)); const refused = ["invalid_type", "invalid_union", "invalid_value"].includes(issue.code) && !(issue.code === "invalid_value" && issue.inst?._zod.def.error !== undefined); const width = ["too_big", "too_small"].includes(issue.code) && issue.inst?._zod.def.error === undefined; if (absent && steps.length > 1 && refused) return { ...issue, path: [last] }; return width || (!absent && refused) ? { ...issue, path: [] } : issue; })); }, { when: () => true });

export const ShapeDraft$Schema: ZodType<ShapeDraft> = ShapeDraft$RawSchema;

export const Shape$Schema = ShapeDraft$Schema.readonly().brand<"Shape">().meta(ShapeDraft$Schema.meta() ?? {}).meta({
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
const PlainDraft$RawSchema = z.strictObject({
  value: z.number().int().min(0).max(4294967295),
});

export const PlainDraft$Schema: ZodType<PlainDraft> = PlainDraft$RawSchema;

export const Plain$Schema = PlainDraft$Schema.readonly().brand<"Plain">().meta(PlainDraft$Schema.meta() ?? {}).meta({
  examples: []
});
```

The same values are read in Rust. `Shape::schema_examples()` answers every block's value, a `Vec<serde_json::Value>` in the order written, and `Shape::schema_example()` answers the first. A type that writes no block has neither function.

Key points:

- Examples are **optional** -- a type that writes none publishes `examples: []`, and has no `schema_examples()` and no `schema_example()`.
- Use the exact syntax: ` ```rust example` (note the space and `example` keyword).
- Every block is published, in the order written. An empty block is skipped.
- A `u64` or an `i64` in an example, and in a `#[serde(default)]`, is a bigint literal in the Zod source, `12n` or `9007199254740993n`, the way the bounds of its schema are written, so a value beyond `2^53 - 1` keeps its value. A `usize`, an `isize`, a date under `as_number` and every other number stay a number unless they lie beyond `2^53 - 1`, and a map's key stays a JSON string. A `#[serde(transparent)]` struct or an untagged enum's newtype variant over an `i64` writes a value within `2^53 - 1` as a number. `schema_examples()` and `schema_example()` answer plain `serde_json::Value`s.
- An example is written as `serde_json` holds it. An object's keys are in the order serde wrote them where the crate's `serde_json` has `preserve_order` on, which the `bson` library turns on, and sorted where it is off.
- Examples respect Serde attributes (field renaming, etc.).
- Being Rust source, an example block is dropped from the JSDoc comment above the generated `export type`, whatever the type is declared as. On a struct, an enum or a type alias it reaches the Zod `examples` list. A type alias has no `impl`, so its examples are `schema_examples()` and `schema_example()` of its schema module (`documented_alias_schema::Schema`). An alias that writes no block and names another published type publishes that type's examples, as does a one-slot tuple struct over one; any other alias without a block publishes `examples: []`. A `use` in a block is dropped whatever lines it spans.
- The JSON Schema document of a type holds the same values under the standard `examples` keyword, whether it is read from the type (`Label::json_schema()`) or from its schema module (`label_schema::Schema::json_schema()`), on a type that lists `enable = ["jsonschema"]`, whether or not `zod` is on. It is written for a struct, an enum, a brand, a one-slot tuple struct and an alias that writes a block. An alias, or a one-slot tuple struct, that writes no block and names another published type carries the `examples` of that type's own document, at its declared filling where it is generic. A type that has none writes no `examples` keyword. `schema_examples()` answers the values in a build with `zod` on, and on a type that lists `"jsonschema"`; `schema_example()` and the Zod list need `zod`.
- A struct or an enum that declares `validate()` and writes examples, in any build of the crate, gets a `#[cfg(test)]` module beside it (`label_schema_examples` for `Label`) whose test builds each example and runs `validate()` on it, so `cargo test` fails on an example that breaks a rule of its type, naming the type, the example's number and the rule: ``example 2 of `Label` breaks its rules: 'text': too short: minimum length is 1, got 0``. It runs in a library and a binary crate alike.
- A brand and a plain enum hold `examples` inside the `.meta()` that holds their `description`. A generic type holds it on the schema its factory memoizes, so `$SchemaDefault` and every other call of the factory carry it.
- A generic struct or enum has each example built at one instantiation, with every type parameter filled at `String`. A lifetime elides there and needs no filling, but a **const parameter takes none** -- `String` names a type, a const is a value, and no value is the one every example would be written at. So a struct, an enum or a type alias that declares a const parameter and writes an example is refused, naming the const; drop the example or the const parameter. Nothing is owed where no example is written, or on a type that does not list `"jsonschema"` in a build without `zod`, the two surfaces that read one.
- The example code is executed at compile time and serialized to JSON.
- Wrong types produce compile errors, ensuring examples stay in sync with your types.
