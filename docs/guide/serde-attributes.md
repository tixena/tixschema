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
- `#[serde(skip_serializing_if = "...")]` -- the key is left out of the payload when the predicate fires, and serde reads the missing key as the member's `default`: Zod writes `roles: z.array(z.string()).prefault([]),`, the TypeScript member is `roles: Array<string>;`, and the JSON Schema states the `"default"` and leaves the key out of `required`. An `Option` field keeps `T | undefined` unless [`ts_optional`](optional-and-nullable-keys.md#optional-typescript-keys-ts_optional) asks otherwise
- `#[serde(skip)]` -- the key is written into no payload and read out of none, so no surface describes the member at all: no TypeScript member, no Zod key, and neither a `properties` nor a `required` entry. No rule of what a surface describes is held to its type: a `u64` under `swift` or `kotlin`, a std type with no schema, a map keyed by a value serde writes as no string and a JSON value all build there. On a tuple-struct or tuple-variant slot it takes the slot out of the described tuple, which shortens the arity -- and a variant declaring one slot becomes a unit variant, which is what serde writes for it. On a variant, the variant is described on no surface and read and written by no client: see [A Variant Serde Neither Writes Nor Reads](enums.md#a-variant-serde-neither-writes-nor-reads)
- `#[serde(skip_serializing)]` -- the write half of `skip`: the key is left out of every payload while a supplied one is still read, and every surface answers as it does for `skip_serializing_if`
- `#[serde(skip_deserializing)]` -- the read half: the key is written into every payload while a supplied one is discarded, so the member keeps a required key
- `#[serde(default)]` and `#[serde(default = "...")]`, on a field or on the struct -- a key the payload leaves out is read as the default on every surface, which is handed the default as JSON: see [A Key Read as Its Default](models.md#a-key-read-as-its-default-serdedefault)
- `#[serde(alias = "...")]` -- read by the [recovering decode](recovering-decode.md#recovering-decode-decode_with), by the [Kotlin client's read](kotlin.md#the-read-in-the-kotlin-client), by the [Dart client's read](dart.md#the-dart-read-refuses-what-validate-refuses) and by the [Swift client's read](swift.md#a-swift-read-refuses-what-validate-refuses): a key stored under an alias counts as its field, and a tag stored as one names its variant. No schema surface describes an alias
- `#[serde(deserialize_with = "...")]`, `#[serde(serialize_with = "...")]` and `#[serde(with = "...")]` -- the field's own functions, which the [recovering decode](recovering-decode.md#recovering-decode-decode_with) reads a value through and writes it back through, as serde's derive does

The three `skip` spellings are three different wires, and [Optional Fields](models.md#optional-fields) reads each one in both directions, positional slots included.

The TypeScript type and the Zod schema write a key, a tag, a variant's name, a plain enum's value and a `literal` as an escaped string literal, so a name holding a `"` or a `\` describes exactly what serde writes: `#[serde(rename = "say \"hi\"")]` is the key `"say \"hi\""`, and `#[serde(rename = "C:\\Path")]` the variant `"C:\\Path"`.

Without tixschema's `serde` feature no `#[serde(...)]` attribute is read, and TypeScript and Zod write each member of a struct or of a variant in camelCase: `#[serde(rename = "mail")] pub email_address: String` is `emailAddress` on both, where serde writes `mail`, and `user_id` under `#[serde(rename_all = "kebab-case")]` is `userId`, where serde writes `user-id`. An enum with data is written as one object per variant, the variant's name under `type` and its data under `value`, a unit variant with no `value`, and Zod reads it with `z.discriminatedUnion("type", [...])`: `pub enum Shape { Square { side_length: u32 }, Circle(f64), Point }` is `{ type: "Square"; value: { sideLength: number; }; } | { type: "Circle"; value: number; } | { type: "Point"; }`, where serde writes `{"Square":{"side_length":3}}`, `{"Circle":1.5}` and `"Point"`. A plain enum is the union of its variants' names, `"Calm" | "Gale"`, as serde writes it.

### A Struct's Own Tag

`#[serde(tag = "...")]` on a struct makes serde write the struct's name under that key, ahead of its fields. Every surface describes the key as a required member whose only value is that name -- the struct's ident, or its `#[serde(rename = "...")]`. serde cannot refuse a key no member declares beside the tag (`#[serde(deny_unknown_fields)]` there refuses the tag itself), so such a struct is `nonstrict`, and every surface reads past such a key:

```rust
#[model_schema(nonstrict)]
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

const Tagged$RawSchema = z.object({
  kind: z.literal("Tagged"),
  name: z.string(),
});
```

The JSON Schema lists `kind` in `required` as `{"type": "string", "const": "Tagged"}`. The Dart, Swift and Kotlin types write the key. They read a payload whose key holds `"Tagged"`, and refuse one without the key or with another value under it, as TypeScript and Zod refuse it: see [The Dart read refuses what `validate()` refuses](dart.md#the-dart-read-refuses-what-validate-refuses), [A Swift read refuses what `validate()` refuses](swift.md#a-swift-read-refuses-what-validate-refuses) and [The read in the Kotlin client](kotlin.md#the-read-in-the-kotlin-client). serde reads the struct either way, whatever value the key holds, so the schemas describe what the struct writes. Without `nonstrict` the struct stops the build:

```text
error: model_schema: type `Tagged` is under a tag of its own, `kind`, and serde cannot refuse a key no member declares beside it: `#[serde(deny_unknown_fields)]` there refuses the tag itself. Mark `Tagged` `#[model_schema(nonstrict)]`, so every surface reads past such a key as serde does
```

A struct that flattens a tagged one is written with the key in its own object, which is what its intersection describes. A field of the struct that writes the same key is refused, since serde would write the key twice.
