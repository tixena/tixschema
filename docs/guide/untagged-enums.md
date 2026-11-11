### Untagged Enums (`#[serde(untagged)]`)

An enum marked `#[serde(untagged)]` serializes as just its content, with no discriminator field. It generates a TypeScript union (`A | B`), a Zod `z.union([...])`, and a JSON Schema `anyOf`. Newtype (`S(T)`) and named-struct (`{ a: A }`) variants are supported.

serde tries the variants in order, so a variant every value of which an earlier one reads never reads back, and stops the build where serde's derive reads it: `#[model_schema(nonstrict)] #[serde(untagged)] pub enum Discount { Open { #[serde(default, skip_serializing_if = "Option::is_none")] note: Option<String> }, Percent { percent: u32 } }` answers ``Declare `Percent` before `Open`.`` A variant that reads only some of them, as one a bound or a pattern narrows does, builds: the narrower variant comes first, and a value the later one holds that the earlier one also reads comes back as the earlier one. `#[serde(untagged)] pub enum MediaType { Known(KnownKind), Other(String) }`, with `KnownKind` a plain enum of `"application/pdf"` and `"image/png"`, reads `"text/csv"` as `Other` and `"application/pdf"` as `Known`: `Other` stands for a value no `Known` variant names, so `Other("application/pdf")` is a value no caller builds.

This rule, and the refusal of a variant that reads every object ahead of a variant serde writes as an object, hold only an enum serde's derive reads. An enum whose `Deserialize` is written by hand, with no `Deserialize` in its derive list, reads in the order its own code picks, so neither applies to it, and it builds with its variants in any order. The Zod schema still tries the variants in the order they are declared, and an `f64` variant and an `i64` variant cannot both read back there in any order: `JSON.parse` reads `5` and `5.0` as the same number, which `z.number()` and the `i64` codec both accept, so Zod reads it as whichever variant is declared first.

```rust
/// ISO-8601 date string; branded newtype carrying the regex pattern.
#[model_schema(enable = ["jsonschema"], pattern = r"^\d{4}-\d{2}-\d{2}$")]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct DateString(pub String);

/// A date sample value: an ISO date string OR an epoch number.
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum DateValue {
    N(f64),
    S(DateString),
}
```

Generated TypeScript:

```typescript
export type DateValue = number | DateString;
```

Generated Zod:

```typescript
const DateValue$RawSchema = z.union([z.number(), DateString$Schema]);
export const DateValue$Schema: ZodType<DateValue> = DateValue$RawSchema.meta({
  examples: []
});
```

Generated JSON Schema:

```json
{ "anyOf": [
  { "type": "number" },
  { "type": "string", "pattern": "^[0-9]{4}-[0-9]{2}-[0-9]{2}$" }
] }
```

The `\d` the brand was declared with reaches the schema as `[0-9]`, the members it stands for, so the JSON Schema and the Rust validator accept the one set of strings -- see [What a `pattern` may contain](validation.md#what-a-pattern-may-contain).

Named-struct variants render each member as a closed object:

```rust
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum NamedUnion {
    A { x: String },
    B { y: u32 },
}
```

```typescript
// TypeScript
export type NamedUnion = { x: string } | { y: number };
// Zod
z.union([z.strictObject({ x: z.string(), }), z.strictObject({ y: z.number().int().min(0).max(4294967295), })])
// JSON Schema: { "anyOf": [ <object>, <object> ] }  — each branch has additionalProperties: false
```

Untagged enums compose with `#[serde(flatten)]`: a flattened variant carrying `Vec<DateValue>` renders `sampleValues: z.array(DateValue$Schema)` (TypeScript `Array<DateValue>`), and the JSON-schema `items` for that field is the `DateValue` `anyOf`.

A member of an untagged variant carries `#[model_schema_prop(...)]` exactly as the same field written in a tagged variant does: the constraint reaches the Zod schema, the JSON Schema and the Rust side alike -- the same per-member validator and the same [`validate()` accessor](validation.md#the-validate-method) -- and every guard the attribute earns is reported at the member. One thing is the untagged member's alone: it is also hung with the `deserialize_with` hook, because an untagged read picks the variant by which one accepts the payload, so the constraint decides what the value *is* and not merely whether it is admissible -- all but a member read through a reference and a member its author reads through a `deserialize_with` or a `with` of their own, each of which is read with no bound written and chooses no variant (see [what is checked while the members are read](validation.md#what-is-checked-while-the-members-are-read)). Everywhere else the members are read first and the value is judged once it is whole; see [where a constraint is checked](validation.md#where-a-constraint-is-checked).

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
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize)]
pub enum KnownMimeType {
    #[serde(rename = "application/pdf")]
    Pdf,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum MimeType {
    Known(KnownMimeType),
    Other(#[model_schema_prop(minLength = 1, maxLength = 127, pattern = "^[a-z]+/[a-z0-9.+-]+$")] String),
}
```

The arm still writes and reads a bare string, and its TypeScript type is still `KnownMimeType | string`. `"text/csv"` reads as `Other` and `"application/pdf"` as `Known`; `""`, a value of 128 characters and `"Text/CSV"` match no variant, because the hook hung on the slot takes `Other` out as it does for a named member. The Zod schema writes the three checks on the arm's member of the union, `z.union([KnownMimeType$Schema, z.string().min(1, ...).max(127, ...).check(z.regex(...))])`, and the JSON Schema on its member of the `anyOf`: `{ "type": "string", "minLength": 1, "maxLength": 127, "pattern": "^[a-z]+/[a-z0-9.+-]+$" }`. A numeric arm takes `minimum` and `maximum` the same way. [A slot of a tuple variant](validation.md#a-slot-of-a-tuple-variant) says how `validate()` reports the slot. A length or a pattern that holds every string the enum carries is written once, on the enum: see [A Check Written on the Enum](enums.md#a-check-written-on-the-enum).

A member holding a map or a tuple describes as the struct field written from the same type does, on every surface. A map is dispatched on the classification its key earns -- enumerated properties for a key whose members the expansion knows, `additionalProperties` for an open one -- and a key no surface can write is refused at the member, at whatever depth the map sits at. A tuple is the fixed-arity array Serde writes, `prefixItems` and the arity bounds included. A member the parser could not classify at all is the one shape that stays permissive: it carries no type name to narrow with, so it admits any value, exactly as the same field does.

**Unsupported variants:** unit variants and multi-field tuple variants in an untagged enum produce a compile-time error.
