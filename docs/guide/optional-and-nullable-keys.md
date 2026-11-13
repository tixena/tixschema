### Optional TypeScript Keys (`ts_optional`)

The bare `ts_optional` flag asks for the optional key (`field?: T` rather than `field: T | undefined`) on the author's word. It is the only thing that writes that spelling.

**Two positions have no key for it to make optional, and writing it on either is a compile error.** A positional slot writes no key at all, in every build, so the flag names a spelling the tuple line has no room for. With the `serde` feature on, a member a serde attribute takes off the wire in *both* directions (`skip`, or `skip_serializing` and `skip_deserializing` together) is described on no surface, so there is no line for the flag to write its key on. An attribute that drops the key one way only (`skip_serializing_if`, `skip_serializing`) leaves the member standing and the flag decides its spelling as usual.

With the `serde` feature on, an `Option<T>` field must carry a key-dropping attribute anyway: its declared shape is `T | undefined`, so its key has to be dropped for a `None`, not written as `null` the way serde writes it absent an attribute saying otherwise — the guard refuses the declaration and names the attribute to add (or, if the wire really does carry `null` here, points at [`nullable`](#nullable-object-keys-nullable) instead). The example below is a build with the feature **off**, where no attribute is read, no such guard runs, and TypeScript writes each member in camelCase:

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
export type ProfileDraft = {
  name: string;
  nickname?: string;
  nickHandle: string | undefined;
};

export type Profile = Readonly<ProfileDraft> & $brand<"Profile">;
```

`nick_handle` is the same field without the flag, so the flag is the whole of the difference between those two lines.

This is a TypeScript-only knob -- the Zod schema and JSON Schema are unchanged (the field is already optional in both, flagged or not). The flag is only valid on `Option<T>` fields, an alias of an `Option` declared above among them; applying it to a non-`Option` field is a compile error. It composes with `as = Type`, which names the type the field already renders.

### Nullable Object Keys (`nullable`)

The bare `nullable` flag is for the `Option<T>` field whose empty state genuinely *is* `null` on the wire — the key is always written and `null` is its value, not the absence of one. Where the default coerces an incoming `null` away to `undefined` and leaves the key optional, `nullable` renders the field as `T | null` with the key **required** on all three surfaces:

```rust
#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentMetadata {
    pub name: String,
    #[model_schema_prop(nullable)]
    pub template_id: Option<String>,
}
```

Generated TypeScript:

```typescript
export type DocumentMetadataDraft = {
  name: string;
  templateId: string | null;
};

export type DocumentMetadata = Readonly<DocumentMetadataDraft> & $brand<"DocumentMetadata">;
```

Generated Zod: `templateId: z.union([z.string(), z.null()])` -- no `.prefault`, no coercing arm. Parsing `{ name: "invoice.pdf", templateId: null }` succeeds; parsing `{ name: "invoice.pdf" }` fails, because the flag declares the key required.

Generated JSON Schema: `"templateId": { "anyOf": [{ "type": "string" }, { "type": "null" }] }`, with `templateId` in `required`.

`nullable` is only valid on `Option<T>` fields, an alias of an `Option` declared above among them, which the field then writes in place as the `Option` it names; applying it to a non-`Option` field is a compile error. It is refused together with `ts_optional` -- the two disagree about the key (`nullable` keeps it, `ts_optional` drops it), and writing both would spell `field?: T | null`, a third state neither flag models. It composes with `preprocess`: the preprocess wrap goes around the whole nullable union, coercing the raw value before it is checked against `T | null`.

With the `serde` feature on, `nullable` is also refused together with any key-dropping serde attribute -- `skip_serializing_if`, `skip_serializing`, or `skip` -- because the flag declares the key always written and one of those would drop it instead, leaving serde write a payload the generated schema does not admit. The guard names the field and the flag. Deserialization needs no such guard: a `nullable` field reads both `{"templateId":null}` and `{"templateId":"invoice.pdf"}` the same way it always did, `{}` included, since nothing about `nullable` changes what serde accepts on the way in.
