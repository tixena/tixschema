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

In the JSON Schema, a named type used two or more times in one document is written once under `$defs` and referenced with `$ref` at each use, a brand over a `String` among them; a type used once is written in place. `UserWithAddress::json_schema()` writes `Address` once under `$defs`, and `address` and the items of `backup_addresses` reference it. A recursive type is always a definition. The key is the type's published name, and its fillings for a generic type; a second type published under that name, with another body, is defined under the name followed by a fingerprint of its body, such as `Revision.a38e709c45a1e062`, and a field never references a definition of another type's name. Each tool's `inputSchema` and `outputSchema` holds its own `$defs` by the same rule, and `openapi()` holds its definitions under `components.schemas`. The form an MCP operation asks with writes every type in place at each use: an elicitation form holds a primitive under each property, which a `$ref` is not.

A referenced type must be declared at item scope — in a module or at crate level, not inside a function body. Each type publishes its schema in a module beside itself, and a type that references one reaches that module through `use super::*`, which a module nested in a function body is not part of. The referencing type is unconstrained: it may sit inside a function body as long as everything it names does not. See [Function-Local Types](troubleshooting.md#function-local-types) for the error a violation produces.

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
    Integer(i32),
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
    value: z.number().int().min(-2147483648).max(2147483647),
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

Under the `swift` feature, an alias of a list, a map or an `Option` is declared above every type whose field names it, as [the Swift client](swift.md#validate-in-the-swift-client) states.

An alias of an `Option` is named by a field alone, but under `nullable`. Its own type describes the `Option` of a field, `T | undefined`, whose `None` leaves the key out. Everywhere else serde writes `None` as `null`: an item of a list, a value of a map, a position of a tuple, a type argument, an arm of a `Result`, a slot of a tuple struct and the slot of a variant. There the holder writes the type the alias names in place, as an `Option` written there is written:

```rust
#[model_schema()]
pub type Plummet = Option<u32>;

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Handlead {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kept: Plummet,
    pub listed: Vec<Plummet>,
}
```

```typescript
export type PlummetType = number | undefined;

export type Handlead = {
  kept?: PlummetType;
  listed: Array<number | null>;
};
```

Zod writes `listed: z.array(z.nullable(z.number().int().min(0).max(4294967295)))`, which reads the `[null]` serde writes as `[null]`, and the JSON Schema describes the same `null`. An alias of such an alias, and one that declares a type parameter, are written in place the same way.

A field typed with an alias of an `Option` declared above is held to every rule a field written `Option<T>` is held to. With no attribute that leaves its key out it stops the build, as serde writes its `None` as `null`, which `T | undefined` does not describe. Flattened, it builds with none, as a flattened `Option<T>` does, and each client reads and writes it as a flattened `Option` of the type it names: `null` where none of that type's keys is there, and no key for its `None`. Under `skip_serializing_if = "Option::is_none"` alone it builds, and serde, the schemas and each client read its absent key as `None`. `nullable` and `ts_optional` are taken on it, and under `nullable` the field is written in place, `kept: number | null`, its key required: the alias's own type describes a key left out. An alias declared below the field, or in another crate, is not known as an `Option` there: a field of it under `skip_serializing_if` alone is refused as a key serde requires, and one with no attribute builds, described as the alias's own type.

The Kotlin client declares no `typealias` for an alias of an `Option` that declares a type parameter, as kotlinc refuses `typealias HaplyType<T> = T?`: a holder writes the type it names in place, `String?` for `Haply<String>`. An alias that declares none keeps its `typealias`, `typealias PlummetType = UInt?`, and a member of it whose key may be absent is declared with no second `?`, `val kept: PlummetType = null`. An alias of an `Option` declared below its holder is named as it is written, which kotlinc refuses where it declares a type parameter, and reports as a redundant `?` where the key may be absent.

An alias takes `name`, and `default_types` where it declares a type parameter. `pattern`, `minLength` and `maxLength` are refused on it at expansion: an alias is another name for the type it names, not a type of its own, so a check written on one has nothing to belong to and nothing to enforce it. A value that carries a rule is a branded newtype, which takes all three -- see [Branded Newtype Validation Constraints](generics-and-brands.md#branded-newtype-validation-constraints) -- and a rule that holds a single field is written on that field, in `#[model_schema_prop(...)]`.

The Zod record of an alias of a map whose values are a type declared below the alias is written with its values behind `z.lazy`, `z.record(z.string(), z.lazy(() => Cell$Schema))`, so a model that holds the alias and is declared after it loads: `pub type Sheet = HashMap<String, Cell>;` above `pub enum Cell { Nested(Sheet), .. }`.
