### Literal Values

`model_schema_prop(literal = "value")` holds a `String` field to one value. The TypeScript type is the literal, the Zod schema `z.literal("value")` and the JSON Schema a `const`, and the field is a rule in Rust and in every client: the Rust field remains a `String`, and [`validate()`](validation.md#the-validate-method) refuses any other value, so the type's read refuses it too.

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
export type ActionDraft = {
  type: "Generate";
  value: "document";
};

export type Action = Readonly<ActionDraft> & $brand<"Action">;

export const Action$Schema = z.discriminatedUnion("type", [
  z.strictObject({
    type: z.literal("Generate"),
    value: z.literal("document", { error: (issue) => `not the literal "document", got ${String(issue.input)}` }).check(z.superRefine((_, ctx) => {
  for (const [at, issue] of ctx.issues.entries()) if (issue.code === "invalid_value") ctx.issues[at] = { ...issue, continue: true };
}, { when: (payload) => typeof payload.value === "string" })),
  }),
]).meta({
  examples: []
});
```

`literal` also accepts a boolean or a numeric value, provided the field's own Rust type can carry it: `literal = true`/`literal = false` on a `bool` field, `literal = 214` on any numeric field. Every surface renders the literal in the field's own kind, so the kind written must match the field's declared type — `literal = true` on a `String` field, or `literal = "x"` on a `bool` field, is a compile error naming both sides. A numeric literal must be a value of the field's number: `literal = 300` on a `u8`, `literal = 1.5` on an integer and a literal an `f32` does not write back are compile errors, since the literal would refuse every value the field holds.

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
export type AdminUserDraft = {
  id: string;
  isSystemAdmin: true;
  email: string;
};

export type AdminUser = Readonly<AdminUserDraft> & $brand<"AdminUser">;

export const AdminUser$Schema = z.strictObject({
  id: z.string(),
  isSystemAdmin: z.literal(true, { error: (issue) => `not the literal true, got ${String(issue.input)}` }).check(z.superRefine((_, ctx) => {
  for (const [at, issue] of ctx.issues.entries()) if (issue.code === "invalid_value") ctx.issues[at] = { ...issue, continue: true };
}, { when: (payload) => typeof payload.value === "boolean" })),
  email: z.string(),
}).meta({
  examples: []
});
```

**A literal is a rule.** `validate()` refuses a value other than the literal, as it refuses a value that breaks a bound, and a serde read of the type refuses the payload with the same sentence, the literal written as the Zod schema writes it:

```rust
#[model_schema()]
#[derive(Debug, Serialize, Deserialize)]
pub struct Fixed {
    #[model_schema_prop(literal = 214)]
    pub value: u8,
}

let seven = FixedDraft { value: 7 };
assert_eq!(seven.validate().unwrap_err(), vec!["'value': not the literal 214, got 7".to_owned()]);
assert!(serde_json::from_str::<Fixed>(r#"{"value":7}"#).is_err());
assert_eq!(serde_json::from_str::<Fixed>(r#"{"value":214}"#).unwrap().value, 214);
```

- **Every position holds it**: a struct's field, a variant's member under every tagging, a slot of a tuple struct and of a tuple variant, and a member read through `Option`, `Box`, `Rc`, `Arc`, `Cow` or a reference. An `Option` is held to it where it holds a value, and `None` stays valid. A literal on a member read through `RefCell`, `Cell`, `Mutex` or `RwLock` is refused at expansion: none implements `Deref`, so the validator cannot reach the value.
- **A literal on a list holds each item**, as the TypeScript type `Array<"x">` and the Zod schema `z.array(z.literal("x"))` say: `'tags.1': not the literal "x", got y`.
- **A member of an `#[serde(untagged)]` enum is held to it as the member is read**, so a literal chooses the variant, as Zod's union chooses it: a payload whose literal is not one variant's is tried against the next.
- **A [recovering decode](recovering-decode.md#recovering-decode-decode_with)** lists a broken literal as one `Invalid` at the value that breaks it, each item of a list at its index.
- **The Dart, Swift and Kotlin clients** hold it too: their [`validate()`](dart.md#validate-in-dart) refuses another value with the same sentence, and so does each client's read. A numeric literal is compared in the field's own type: an integer field as an integer.
- **A literal on an `i64` or a `u64` is the `bigint`**, as the field is: `214n` in TypeScript and `{"type": "integer", "const": 214}` in the JSON Schema. See [A `u64` and an `i64` are a `bigint` in TypeScript](models.md#a-u64-and-an-i64-are-a-bigint-in-typescript).
- **The Zod schema** reports the sentence as the issue of the literal, at the value's path, in every place it reads one, a union among them: a variant of an externally tagged or an untagged enum, and an `Option` of a list. A value of the literal's own type that is not the literal leaves its issue no abort, as a broken bound leaves its own, so a union reports the issue where the value is: `{"Struck":{"tool":"hammer"}}` is refused with `'Struck.tool': not the literal "die", got hammer`. The refinement the schema writes after `z.literal` is what does it, and the schema's TypeScript type, its input and its output stay those of `z.literal`.

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

An enum's own check does not reach a field that carries a literal ([A Check Written on the Enum](enums.md#a-check-written-on-the-enum)).

**Recommended: Single-value enums over literal strings.** A single-value enum provides type safety in Rust -- the field can only hold the correct value at compile time, whereas a `String` with a literal annotation can be built in Rust with any string, which `validate()` and the read then refuse.

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
        value: String,  // Any string can be built, and `validate()` refuses all but "document"
    },
}
```

Both approaches describe the same payload on every surface. The single-value enum is preferred because:

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
