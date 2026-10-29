# Branded Newtypes and Generic Types

Branded newtypes, generic types and Zod factories, declared defaults, and the string constraints of a generic brand.

### 8. Branded Newtypes

Single-field tuple structs with `#[serde(transparent)]` are treated as branded/opaque types, and so is any other `#[serde(transparent)]` struct, with named fields or with several slots, over the one field serde writes and reads it as: `brand_field` in `src/features/serde.rs` is the one seam every surface asks, the web three and Dart, Swift and Kotlin alike, and `transparent_field` beside it is the rule of serde's derive in a `Direction` for which field that is. A struct that derives `Serialize` alone is described over the field serde writes (not under `skip` or `skip_serializing`, which `has_serde_skip_serializing` reads, and not `PhantomData`), and any other over the field it reads (not skipped for reading, no `default`, not `PhantomData`). A struct that derives both, whose two derives take two different fields, is refused at `exec_model_schema`, in every build, with a message naming both (`transparent_direction_errors`, which `serde_attribute_guard_errors` joins to the rename guard): one schema cannot describe two wire forms, as with a rename whose two directions differ. A tuple struct of one slot is a brand over that slot whatever its attributes say. `brand_slot` answers the field with its place among the struct's fields, and every reader of the value on the Rust side (`validate()`, `Display`, the text a bound on a holder's field measures) reaches a slot by that place, through `brand_member`, so `struct Tagged(#[serde(skip)] PhantomData<u8>, pub String)` is read at `self.1`. The brand publishes under its Rust ident unless `#[model_schema(name = "...")]` names another, and that name reaches the surface twice: as the exported type and as the brand tag the values carry.

A non-generic brand publishes a `$RawSchema`/`$Schema` const pair:

```rust
#[model_schema()]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CorrelationId(pub String);
```

With `zod` + `typescript` features:
```typescript
export type CorrelationId = string & $brand<"CorrelationId">;
const CorrelationId$RawSchema = z.string().brand<"CorrelationId">().meta({
  description: "CorrelationId",
  examples: [],
});

export const CorrelationId$Schema: typeof CorrelationId$RawSchema = CorrelationId$RawSchema;
```

A brand with a type parameter publishes `X$SchemaFactory` plus `X$SchemaDefault` instead, exactly
like any other generic item — never a plain `const`:

```rust
#[model_schema(default_types(IdType = String))]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UserId<IdType>(pub IdType);
```

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
```

With `typescript` only (no `zod`):
```typescript
declare const __brand_UserId: unique symbol;
export type UserId<IdType> = IdType & { readonly [__brand_UserId]: true };
```

Rules:
- Generic parameter names are preserved exactly (`IdType` stays `IdType` in TypeScript)
- A brand with no type parameter publishes the `$RawSchema`/`$Schema` const pair; a brand with a
  type parameter publishes `$SchemaFactory`/`$SchemaDefault` instead, the default binding its
  factory call to `{Name}$RawSchemaDefault` first — the parameter's inner composes the factory's
  bound argument (`idType`), never the opaque `z.unknown()`
- Either binding is annotated `typeof` the raw `const` beneath it, never a Zod class: `.brand()`
  narrows at the value position, and the class an inner renders to is zod's to decide
  (the `DateTime<Tz>` schema is a `ZodPipe`, `z.iso.date()` a `ZodISODate`, a brand over a brand a
  `$ZodBranded` of a `$ZodBranded`). Reading the annotation off the value is the one spelling that
  stays true of all of them
- Serde transparent serialization works normally — the newtype is invisible in JSON
- The JSON Schema of a brand over a `char` is the one-character string a `char` is, `{"type":
  "string", "minLength": 1, "maxLength": 1}`: `branded_json_inner` hands a `char` to the slot
  dispatch, as it hands a named type, and the brand's own checks narrow it in an `allOf`
- A brand that carries a bound or names a check is read as a struct or an enum is (section 7, "A
  read refuses what `validate()` refuses"): its own unjudged read, then its `validate()`. No hook
  is hung on its slot, and a type that holds the brand reads it unjudged and names it at its path
- The slot takes no `#[model_schema_prop(...)]`: a brand publishes its inner's own schema with a
  `.brand()` written onto it, so no key written there reaches any surface, and one written there is
  refused at `exec_model_schema` — the ungated seam, so the verdict is the same in every feature
  combination — with the item re-emitted stripped of it. The three checks a brand does carry are
  written on the type: `#[model_schema(pattern = "...", minLength = N, maxLength = N)]`. Over a
  list the two lengths count its items and `pattern` is refused (section 7). The
  field a `#[serde(transparent)]` struct with *named* fields or several slots is the value of is
  that slot, and is held to the same. Every other field of such a struct, one serde neither reads
  nor writes (a skipped one, a `PhantomData` marker), is refused a key as well, in every build
  and with the item re-emitted without it, under a message of its own
  (`UNREAD_BRAND_FIELD_PROP_MESSAGE`): `branded_slot_prop_errors` reads every field of a struct
  `brand_field` answers for
- The three checks are type-level arguments on a brand and on an enum, which hands them to the
  fields it reaches (section 7) and publishes no check of its own. A struct that is no brand and a
  type alias are refused them at `process_item`, in every build (`string_check_refusal_output`,
  `alias_string_check_refusal_output`)
- An item whose published expression *is* another item's binding — an alias of a brand, a one-slot
  tuple struct over one, and the `$SchemaDefault` of either where it declares a parameter — is
  annotated `typeof {Name}$RawSchema` rather than `ZodType<{Name}>`, so the brand's narrowing
  survives the republish. An item that builds an expression of its own keeps `ZodType<{Name}>`

### Generic Types and Zod Factories

A Zod schema is a runtime value and TypeScript generics do not exist at runtime, so a generic
struct, tuple struct or enum has no one schema to publish. It exports `X$SchemaFactory` instead —
a function taking one required schema argument per parameter — while a type that binds no type
parameter keeps exporting `X$Schema` exactly as before. `zod_binding_suffix` is the one seam that
decides which, and `zod_published_binding` the one seam that writes either.

Beside the factory, a generic type also exports `X$SchemaDefault` — the factory called at the
type's own declared `default_types`, so a consumer who wants the ordinary filling shares the memo
rather than building a second schema for it. `zod_default_block` builds the const, through
`default_zod_rendering`, the same `get_field_def` path every field renders through; where a
declared default names another generic item at exactly the arguments that item calls its own,
the rendered argument folds onto that item's `$SchemaDefault` instead of reconstructing a factory
call the memo would not share with it — see `record_zod_default_arguments`. `zod_binding_reexport`
covers both bindings wherever a renamed item re-exports its factory.

`X$SchemaDefault` is a module-scope `const`, and any argument `default_zod_rendering` renders as a
reference to another item — the fold above, or an ordinary (non-folding) call to that item's own
`X$SchemaFactory` — names one more module-scope `const`. One macro invocation sees one type, so
nothing here can know whether that `const` is written above this one in the generated module or
below it; `default_zod_rendering` defers every such reference through `deferred_zod_operand`, the
same `z.lazy(() => …)` wrap `.and(...)` already relies on for a flattened base. A self-contained
expression like `z.string()` names no sibling `const` and is left eager. The fold's own gate
(`array_depth == 0 && !is_optional()`, a *direct* sibling) only scopes the fold — a default
wrapped in a `Vec`/`Option`/`Map`/`Tuple` has no bare binding to fold onto — but deferral is not
bounded by it: `FieldDef::names_a_sibling_binding` walks the whole rendered tree, so
`Tagged$SchemaFactory` inside `z.array(Tagged$SchemaFactory(z.string()))` defers exactly as a bare
`Tagged$SchemaFactory(z.string())` does, the `z.lazy` wrapping the whole expression rather than the
factory call alone. The deferral is also what ends a cycle between two declared defaults —
`CycleLeader`'s naming `CycleFollower` and `CycleFollower`'s naming `CycleLeader` back: neither can
have registered the other's arguments yet when it expands, so neither side folds, and both calls
read the other's factory behind `z.lazy` rather than at the top of a `const` initializer — the
module loads regardless of which of the two names the consuming project's entity list writes
first.

Each factory memoizes on the identity of its arguments, one `WeakMap` level per parameter
(`zod_cache_type` writes the nesting, `zod_factory_body` the walk), so two calls with the same
argument objects return the identical schema and no two argument lists collide.

A `WeakMap` fixes both of its own parameters at construction, so its value type cannot depend on
its key type and the store is written at the widened `ZodType` spelling throughout. What recovers
the precise types is an overload, which `zod_factory_declaration` emits: the signature a caller
sees names real type parameters, the implementation beneath it takes the widened ones the store is
keyed at, and the read is therefore already the implementation's return type. Nothing emitted is
`as`, `any` or `unknown`; nothing a caller passes is written to; and there is no preamble — a
generated module is the types in it and nothing shared between them.

Every parameter is a real TypeScript type parameter (`<IdType extends ZodType>`), never a bare
`ZodType` annotation — `ZodType` defaults its own parameters, so an argument annotated with it
infers every field as `unknown`.

A generic type may reach itself, directly or through a second type reaching back. JSON Schema
hoists it into `$defs` once and points a `$ref` at that definition; TypeScript writes the name
inside itself with its arguments; Zod calls the factory again with the argument the outer call was
handed, the memo cache being what ends the recursion — the schema is cached before the recursive
call is made. A builder that defers a reference — a member behind a getter, an operand behind
`z.lazy` — reads its type back under `interface X$SchemaOf<…> extends ReturnType<…> {}` where every
other generic item keeps the `type` alias: `zod_schema_of` decides, off `defers_a_reference`. A
declaration file writes a type that reaches itself only by name, and writes an alias as the
structure it resolves to, with `any` where that structure recurs; the interface is what a second
package, reading the `.d.ts`, parses the recursive member through.
`tests/generic_types_tests/declarations.rs` compiles that second package for real.

A tuple struct has no key to write a getter on. `tuple_slot_zod` writes a slot that reaches the
struct itself, or a type declared below it, behind `z.lazy` through `deferred_zod_operand`; read
as it stands, the slot names a binding still being built and the module throws as it is imported.
For a struct with no parameter that is the whole of it, the binding being annotated
`ZodType<X>`. A generic one cannot read its own factory's type inside the tuple, whose slot types
are read as the tuple is built: `zod_self_view` writes `X$SchemaSelf`, an overload declared to
return `ZodType<X<T["_zod"]["output"]>>` over an implementation that hands back the factory's schema, and
the slot calls that behind `z.lazy`. The same test file loads the bundle under `node` and parses
through both. Which of the item's two bindings a self-reference names is read off the store
`record_zod_factory` writes, and that is written at `exec_model_schema` ahead of every shape rather
than where the binding is finally spelled: the fields are rendered before then, so an answer stored
on the item's own registry entry would be read before the item had put one there.

`write_field_type_and_schema` defers a member whose type reaches the item being defined, and one
that reaches a type declared *below* it — `reaches_a_type_declared_later`. The rule applies whether
or not the referenced type declares a parameter of its own: a zero-argument sibling not yet
registered is exactly as much a forward reference as a generic one, since its `X$Schema` is read
eagerly at the top of the referencing item's own module-scope const initializer. The second clause
is what ends a cycle spanning two types: every cycle contains at least one reference pointing
forward, since declaration positions cannot strictly decrease all the way round one, and what is
left once those are deferred cannot cycle. The deferral is a getter rather than `z.lazy`, which at
an operand
position collapses the factory's inferred return type to `any`.

A union written around a factory's call -- the `Option` wrap, or the `nullable` one -- is checked
against what each member takes and answers: `.prefault(undefined)` types its argument by the
union's input, and `z.union` over a `z.record` of the call resolves the record's own output. Inside
a cycle that is the type still being inferred, which `tsc --strict` refuses (TS7023, TS2615).
`zod_union_around` therefore hands every factory call in through a generic arrow
(`<Reached$ extends SomeType>(reached$: Reached$) => …`) and builds the union over the type
parameter, so the check is made once against the parameter and the member only instantiates it.
`FieldDef::zod_factory_calls` lists the calls, outermost only and once each, by the same renderer
that writes them; the names end in `$`, which no argument `zod_factory_argument` binds can. The
bound is the one `z.union` and `z.array` are themselves declared with -- `ZodType` there reads the
schema's output and brings the cycle back. It is written bare (`ZOD_SCHEMA_BOUND`, `SomeType`) and
the self view writes `T["_zod"]["output"]`: a consumer may bind `z` as a constant
(`const z = zod;`), which names no type, so nothing emitted writes `z.` where a type is read.
`SomeType` is the consumer's to bring into scope, as `ZodType` and `$brand` are
(`import type { SomeType } from "zod/v4/core";`). The bundle
`tests/generic_types_tests/declarations.rs` compiles binds `z` as a constant and imports both.

A map of the item itself is read through the self view as well. `z.record` resolves its own type
off its value as it is written, and inside the builder the item's own factory answers the
builder's return type, still being inferred: `tsc --strict` refuses a record of it nested in
another call (TS2345) and resolves a second record's output to `unknown` after a first (TS2322).
`maps_of_itself_through_the_self_view`, called from `zod_factory_block` under `typescript`,
rewrites the item's own factory call to `X$SchemaSelf(…)` wherever it is written inside a
`z.record`/`z.partialRecord`, and hands a union's arrow the view where the arrow writes its
argument inside one. `zod_factory_block` emits the view for whatever builder calls it, a tuple
struct's included. A list or an `Option` of the item, and a reference to another generic item,
keep the factory's call. Without `typescript` there is no `X<…>` to declare the view with, and
nothing is rewritten.

### Declaring a Default Type per Parameter

JSON Schema has no type parameters, so a generic item's document is built from one concrete
filling, and `default_types(IdType = String, DateType = f64)` is where an author names it — one
`Parameter = Type` pair per type parameter, in any order, beside every other item-level argument.

`default_types_guard_errors` reads the declaration against the item's own parameters at
`exec_model_schema`, the one seam every expanded shape is dispatched from, so a struct, a tuple
struct, a branded newtype, an enum and an alias are all answered the same way and before any of
them is split off. It refuses in both directions:

- an entry naming nothing the item declares — refused in **every** feature configuration, spanned
  on the entry, since a misspelled parameter fills nothing while the one it was meant for keeps no
  default;
- a type parameter with no entry — refused only under `#[cfg(feature = "jsonschema")]`, spanned on
  the parameter, because nothing else reads the default;
- `default_types` on an item with no type parameter, and an empty `default_types()`, both being a
  declaration with nothing to declare;
- an entry filled at a type the field-definition dispatch has no arm for — refused under
  `#[cfg(any(feature = "zod", feature = "jsonschema"))]`, the two surfaces that read a declared
  filling, spanned on the filling. That dispatch takes a name it does not recognise for another
  `#[model_schema]` item, which is right for a sibling declared below and gibberish for `i128`: the
  emission names an `i128_schema` module nothing publishes. Tokens cannot separate the two, so
  `is_undescribable_primitive` refuses only what provably cannot become a sibling — the primitive
  names the language reserves that the dispatch answers for nowhere
  (`i128`, `u128`, `f16`, `f128`). `char` is not among them: it has an arm of its own, and a
  filling at it describes as the one-character string serde writes.

A const parameter earns its own refusal, from `const_parameter_argument_errors` at the same seam
and compiled in every build that describes a type, the Dart, Swift and Kotlin clients alone
included: a const handed to a written type as an *argument* stands where a type is read, so the JSON side would call into a module named after it and the TypeScript
side would write a name its declaration binds nothing for. A const written as an array length is
untouched — that is the one position it renders in, as an unbounded array.

There is no fallback filling. A guessed one produces a document that silently rejects valid
payloads, which is what the declaration exists to prevent.

### String Constraints on a Generic Brand's Declared Default

A branded newtype's own `pattern`/`minLength`/`maxLength` normally have no inner to measure when
the inner is one of the brand's own bare type parameters — a parameter is the opaque value on
every surface, and `z.unknown()` carries no `.min`/`.max`. `branded_constraint_inner_error` reads
the declared default for that parameter instead of refusing outright: `declared_default_field`
resolves the same `default_types` entry `zod_default_block` reads, and `non_string_inner_shape`
asks of it the same question asked of a concrete argument — string-shaped, the checks compose onto
the default; not string-shaped, the refusal names the default rather than the parameter, through
`declared_default_constraint_message`. An entry the declaration left out falls back to `String`,
the same fallback `schema_example_value_type` uses for the identical gap, so the guard alone
requires nothing; only `jsonschema`'s own separate requirement (above) forces every parameter to
declare one.

The checks never reach the factory's own parameter — `branded_zod_inner` renders a bare-parameter
inner with no check appended regardless of genericness, since a caller filling the parameter with
something other than the default (an `ObjectId` schema, say) must not inherit bounds meant for it.
They land once, on `$SchemaDefault`'s argument for that one parameter: `zod_default_block` and
`zod_factory_block` both take a `ZodDefaultInputs` bundling `default_types` with the optional
`(parameter, checks)` pair a branded newtype's own expansion supplies — `None` for every ordinary
generic struct, tuple struct, enum and alias: an enum's own string check reaches its fields and
never a type parameter, and the others carry none. The JSON surface needs no separate change: a bare parameter's document is already the runtime
argument bound to the parameter's declared filling (`json_argument_value`, defaulting through
`declared_filling_json_schema_value`), which `branded_layered_over` narrows with the brand's own
bounds through the same `allOf` a named inner is narrowed through — never the inert `{}` a
parameter with no default at all would describe as.

The serde read holds the same line, being the brand's unjudged read followed by its `validate()`
(section 7, "A read refuses what `validate()` refuses"). `validate()` is at every filling and asks
which one it holds: `validate_at_declared_default`, which `build_branded_validation` emits beside
`type_identity`, compares `type_identity::<T>()` against the inner type with each parameter's
declared default substituted in (`substitute_declared_defaults`, reading
`declared_default_syn_type`), and runs the checks on the value read as that default only where the
two types are one, so the read and `validate()` reach the same one filling `$SchemaDefault` does.
`type_identity` is emitted into the schema module by `embedded_type_identity`, a copy of
`typeid::of` (see `THIRD-PARTY-NOTICES`), so a consumer adds no dependency for it.
