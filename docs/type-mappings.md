# Type Mappings

How each Rust type, serde attribute and published name is written on every surface, and the form each client sends.

### 1. The Name a Type Publishes Under

A type publishes under the Rust ident it is declared with, spelled exactly as written. Nothing is read off that spelling — no suffix is taken off it and no part of it is rewritten.

`#[model_schema(name = "...")]` is the only publishing override, and it moves the name on every surface at once: the `export type` line, the Zod consts, the `$defs` key a self-referential document hoists under, and every reference another type writes.

```rust
#[model_schema()]
pub struct User { ... }                             // → TypeScript: User

#[model_schema()]
pub struct UserData { ... }                         // → TypeScript: UserData

#[model_schema(name = "User")]
pub struct UserData { ... }                         // → TypeScript: User
```

A renamed item also publishes its Rust ident as an alias of the moved name — `export type UserData = User;`, `export const UserData$Schema = User$Schema;` — which is what lets a type declared *above* it, with only the ident to name it by, still resolve. `ident_schema_module_name` names the generated Rust module from the ident for the same reason: an override never moves it.

`compute_item_export_name` is the one seam a declared item's published name is read from, and `compute_alias_export_name` the alias counterpart — an alias has no surface name of its own and takes the `Type` suffix without an override.

Two declarations cannot publish under one name: the emitted types, schemas and definitions are one flat namespace, so `published_name_collision_errors` refuses the second at `exec_model_schema` — the ungated seam, so the verdict is the same in every feature combination — naming both declarations. `claim_published_name` holds the claims, keyed the opposite way to the `ALIAS_INFO` registry so an ident reclaiming its own name is never a collision.

### 3. HashMap Key Restriction

A map key must be one serde can write as a JSON object key. `String` is the open case: any string
is a key. `bool`, the numeric types, `char`, and the chrono date/time types are accepted too —
serde stringifies each one (`7` → `"7"`, `true` → `"true"`) — so the map describes as an open
object. A plain `#[model_schema()]` enum key narrows the object to its members. A key serde does
not stringify — a struct, tuple, `Vec`/array, `Option`, nested map, or `ObjectId` — is refused at
expansion:

```rust
// ✅ Supported — string, and any key serde stringifies (bool/numeric/char/chrono)
pub struct Config {
    pub settings: HashMap<String, String>,
    pub counts: HashMap<u32, String>,
}

// ❌ NOT supported - refused at expansion
#[model_schema()]
pub struct KeyStruct {
    pub value: String,
}

#[model_schema()]
pub struct BadMapKey {
    pub m: HashMap<KeyStruct, u32>,
}
// error: field `m`: a map key must be a plain `#[model_schema()]` enum, whose members
// become the object's keys — `KeyStruct` resolves to a type with no `enum_members()`
```

`is_map_wrapper` in `src/field_type.rs` is the one seam that says which names are a map: `HashMap`,
`BTreeMap`, and `IndexMap` under the `indexmap` feature. The field definition, the constraint walk,
the walk `validate()` makes to what a member holds, the recovering decode's walker and its
rewriting of an alias, and the MongoDB paths all ask it, as they ask `is_sequence_wrapper` for a
list, which takes `IndexSet` under the same feature. `IndexMap` and `IndexSet` are names of another
crate's, so the two functions ask `is_declared_ident` for both: a type the crate declared under one
of them above keeps the name, with the feature on or off. `serde_json::Map` is no map: a member
that holds one is refused (section 4).
`is_declared_ident` reads the published names, the one registry every build writes. A key is held
to the rules of this section whichever of those maps it is written in.

On the two nominal surfaces a key renders in the form serde **writes** it wherever the key's own
value form is not a TypeScript property key, and under the name it was written with wherever it is.
`typescript_map_key_typename` and `zod_map_record_call` are the one pair of seams that decide, both
reading `FieldDef::map_key_wire`, so the surfaces cannot drift:

| key reached | TypeScript | Zod |
|-------------|------------|-----|
| `String`, `char`, `PathBuf`, type parameter | own type (`string`, …) | own schema |
| plain enum | own type (`MetricSlot`) | `z.partialRecord(MetricSlot$Schema, V)` |
| the integer types | `number` | `z.record(z.number().int(), V)` |
| `NaiveDate` / `NaiveTime` / `NaiveDateTime` | `string` | own schema |
| `bool` | `"true" \| "false"` | `z.partialRecord(z.enum(["true", "false"]), V)` |
| `DateTime<Tz>` | `string` | `z.record(z.iso.datetime({ offset: true }), V)` |

The last two rows are the ones whose value form spells no property key: `boolean` and `Date` are
rejected by `Record`'s own constraint, `z.boolean()` rejects the string serde wrote, and
the date schema accepts it only to rewrite every key into a locale-dependent rendering. A brand or
alias over one of those loses its name at a key position for the same reason — the only binding it
publishes is that value-shaped schema — while a brand or alias over any other row keeps its name
exactly as before. The constructor moves with the key because `z.record` over an enumerated key
demands every member: a key whose published Zod binding enumerates — a plain enum, and any brand or
alias over one, `.brand()` being type-level only — is written with `z.partialRecord`, which admits
the subset a `HashMap` holds while still refusing a key outside the enumeration.

The JSON Schema describes a map whose key serde writes as text of its own (`MapKeyPath::Unnarrowed`)
at every position, flattened (section 4) or under a key, as it describes a map keyed by a `String`,
its values in `additionalProperties`, and names its keys in `propertyNames` (`text_key_names`): a
`char` and a chrono date or time by their own descriptions, an integer by `INTEGER_KEY_PATTERN`
(`^-?[0-9]+(\.0+)?$`), a float by `NUMBER_KEY_PATTERN`, a `bool` by `BOOLEAN_KEY_PATTERN`
(`^(true|false)$`), and a named key by its own description, which `named_key_text` reads, where the
document is built, as the pattern of a number's or a `bool`'s text where it describes one.
`text_key_map_json_schema_value` writes it, for `map_json_schema_value` and
`build_nested_map_member_item`. Where serde and Zod's record part on the text of a number key, the
pattern follows Zod: a leading zero, `-0` and a fraction of zeros on an integer key, which serde
refuses. It states no range, as `{"type": "integer"}` states none for a value. A tuple as a map's
value is described as the fixed array it is in a field, and `()` as `null` (`build_map_member_item`,
the one dispatch of a map's value and a tuple's position). `tests/collection_tests/keyed_maps.rs`
holds the models and the payloads, `keyed_map_documents.rs` beside it the JSON Schema of each held
to serde and to Zod, and `tests/generic_types_tests/declarations.rs` parses each payload under node.

### 4. Type Mappings (Rust → TypeScript)

- `String` → `string`
- `bool` → `boolean`
- Numeric types → `number`. Zod holds each integer to its type's range, `z.number().int().min(0).max(255)` for a `u8`: `FieldDefType::integer_range` answers it, and `zod_number_type` writes it where no declared `minimum` or `maximum` narrows that side. `usize` takes `.min(0)` alone, and `isize` `.int()` alone, which refuses an integer beyond `Number.MAX_SAFE_INTEGER`; a `u64` and an `i64` are a `bigint` (below). `integer_range` is compiled under `zod` and `dart`, its two readers. An `f32` is `z.number().gt(-B).lt(B)`, `B` being `F32_READ_BOUND`, `3.4028235677973366e38`, the size below which serde reads a finite `f32`. The JSON Schema states no range
- A `u64` and an `i64` → `bigint`, written on the JSON wire as the number serde writes. `README.md`'s "A `u64` and an `i64` are a `bigint` in TypeScript" is the consumer's view; this is what a change has to keep.
  - **TypeScript writes `bigint` wherever the type is the value**: a member, an item, a tuple's position, a type argument, a `default_types` filling, an alias's own type, a range's bound, a `Result`'s arm and a brand's slot (`typescript_base`), and `214n` for a literal (`FieldDef::bigint_literal`). A map's key is a `number`: `map_key_wire` answers `MapKeyWire::Signed64` and `MapKeyWire::Unsigned64`, which `typescript_map_key_typename` writes `number` and `zod_map_record_call` `z.record(z.number().int(), V)`, with `.min(0)` for a `u64`, so the Zod schema refuses a key beyond `2^53 - 1` in size.
  - **Each TypeScript declaration that writes such a map carries `INEXACT_KEY_NOTE`** (`src/utils.rs`), a JSDoc `@deprecated` tag, which an editor shows where the declaration is read (suggestion `6385`, `6387` for a method) and `tsc --strict` prints nothing for. No declaration is added, so two emitted types share none. `FieldDef::writes_an_inexact_key` and `slot_writes_an_inexact_key` say whether a type writes such a map at any depth, walking what `typescript_base` and `typescript_slot_typename` write, and a name the registry records with `AliasInfo::inexact_key` (`record_inexact_key`): an alias, a brand or a tuple struct declared above whose value writes one. `inexact_key_noted` closes a member's JSDoc on the tag (`write_field_type_and_schema`, `write_named_variant_fields`), and so do the key of a variant of slots under its name (`render_external_variant`), the content key under a tag and a content key (`content_jsdoc`), a tuple struct, an alias and a brand, and the name each republishes itself under (`noted_ident_reexport_ts`). In the service code `message::method_jsdoc` writes it on a client's and an implementation's method whose message, header or part writes one (`sends_an_inexact_key`), and `result::result_arm` on the `value` or `error` of a result and an outcome, on lines of its own: a comment on the line of another member documents that member.
  - **Zod reads a `bigint`, or a `number` that holds the integer exactly, and answers the `bigint`.** `zod_bigint_type` writes `z.codec(z.union([z.bigint(), z.number().int()]), z.bigint()…, { decode: (value) => BigInt(value), encode: (value) => value })`: the type's range (`FieldDefType::bigint_range`) on the `bigint` where no declared bound narrows that side, and each declared `minimum` and `maximum` as a `bigint` bound in `zod_error_arg`'s sentence. A literal is a `z.codec` from the read, refined to the literal in the sentence of `Bound::Literal`, to `z.literal(214n)`.
  - **The JSON Schema describes `{"type": "integer"}`**, as every integer, and a literal as `{"type": "integer", "const": 214}` (`bigint_literal_const`, `build_bigint_literal_field_schema`). `apply_model_schema_prop_meta` keeps a `U64` or an `I64` under a literal, where every other numeric type becomes a `NumberLiteral`; `LiteralValue::Integer` holds the literal exactly, and `held_literal` checks that the field holds it.
  - **The clients hold the number**: Dart an `int`, Swift an `Int64` and Kotlin a `Long`, read and written as the number serde writes. Swift and Kotlin refuse a `u64`.
  - **The TypeScript service code reads and writes each integer exactly.** `message::read_fn` writes the reader, `JSON.parse` with a reviver that rebuilds an integer a `number` does not hold exactly from its `context.source`, as a `bigint`: `{prefix}Read` (`client.rs`, a reply header), `{prefix}RequestRead` (`service.rs`, a request header), `{prefix}HttpRead` (`http_client.rs`, a body and a fault), `{prefix}HttpRequestRead` (`http_service.rs`, a body and a reply header) and `{prefix}WsRead` (`ws_client.rs`, `ws_service.rs` and `ws_server.rs`, a frame and a header). `message::wire_fn` writes the writer, each `bigint` through `JSON.rawJSON`, read off `JSON` through `Reflect.get`, since the TypeScript declarations name no such function, and, in a runtime with none, a `bigint` a `number` holds exactly as that number and a `RangeError` for any other: `{prefix}Sent`, `{prefix}HttpSent`, `{prefix}Answered`, `{prefix}HttpAnswered` and `{prefix}WsSent`. `message::checked_fn` writes `{prefix}Checked` and `{prefix}HttpChecked`, which parse a message or a header with its schema and write what it answers, the writer's error answered as an issue, so a client answers it as a message that failed validation. A path, a query, a header and a text part of a `u64` or an `i64` are read by `{prefix}HttpCoerceBigInt` (`message::decode_ts_expr`, `parse::held_as_a_bigint`), and `decode_expr` in `http_rest.rs` reads the text as an `i64`, then a `u64`, so a `u64` is read whole. `zod_repeat_check` compares a set's items with each `bigint` written as its digits.
  - **A transport of the transport-agnostic client and dispatcher carries JavaScript values**: what it is handed holds each `bigint` as `JSON.rawJSON`, and a transport that reads its text with a plain `JSON.parse` rounds an integer beyond `2^53 - 1`. The README shows the reader to use.
  - **MongoDB stores the integer serde writes**, an `Int64`. The recovering decode reads the number, and the string of its digits a stored value may hold: `value_digits_settled` and `bson_digits_settled`, in the `decode_with` library, rewrite into the number each digit string an `Invalid` issue found at an `I64` or a `U64`, where every such string of the issue holds a value of its type, and an entry point reads the value again once, so a stored value of digits reads with no callback.
  - **The tests.** `tests/wide_integer_tests/` (serde's write and read in every position, the text of every surface, a literal, a bound, a map's key, an alias declared below, a `u64` in a build without `swift` and `kotlin`, and in `keyed_models.rs` and `keyed.rs` the note over a map keyed by one in each position and the README's example), `tests/generic_types_tests/declarations.rs` (`type_check_a_64_bit_integer_is_read_as_a_bigint_under_zod`, `type_check_a_64_bit_integer_in_each_place_its_holder_reaches_is_read_as_a_bigint_under_zod`, and `type_check_a_map_keyed_by_a_64_bit_integer_is_marked_where_an_editor_reads_it`, which runs `deprecated_reads.mjs` beside it, the TypeScript language service out of the `typescript` in `TIXSCHEMA_NODE_MODULES`), `tests/service_schema_typescript_tests/keyed.rs` with `a_map_keyed_by_a_64_bit_integer_is_marked_where_an_editor_reads_it_on_every_surface` in `type_check.rs` (the note in the service code), `tests/wire_forms_tests/models.rs` (`ConveyedI64`, and `ConveyedU64` at `u64::MAX`), `tests/service_schema_emitted_client_tests/wide_integer_models.rs` with `run_{dart,swift,kotlin}_wide_integers.rs`, `run_node_wide_integers.rs` (each TypeScript client and dispatcher, a bare `i64`, a reply read by a plain `JSON.parse`, a runtime with no `JSON.rawJSON`), `bare_wide_integer_rows.rs` with `run_{dart,swift,kotlin}_bare_wide_integers.rs`, `tests/service_wide_integer_tests.rs` (the Rust AMQP and `http_rest` dispatchers and clients, and a `u64` at `u64::MAX` through `http_rest` in a build without `swift` and `kotlin`), `tests/service_schema_dispatch_tests/path_bound_beside_query_service.rs` (a query), and `tests/recovering_decode_tests/wide_integers.rs` and `tests/recovering_decode_bson_tests/wide_integers.rs` (both majors).
- `i128`, `u128`, `f16`, `f128` → refused, in every build, at any depth of a member, an alias and a `default_types` filling: `undescribable_primitive_errors` at `exec_model_schema`, through `ranges::undescribable_primitive_reached` over `written_positions` (the positions `any_json_positions` reads), naming the member and the type, and the item is emitted as its refusal alone. A member serde neither writes nor reads, a type parameter and a type the crate declared above under the name are not refused. `tests/foundation_refusals/undescribable_primitive.rs` holds the error.
- `Option<T>` → `T | undefined` (Zod: `z.union([z.null().transform(() => undefined), T, z.undefined()]).prefault(undefined)`, accepting an explicit `null` and coercing it to `undefined`; JSON Schema `anyOf: [T, {"type": "null"}]`, key left out of `required`); with `#[model_schema_prop(ts_optional)]` the TypeScript key becomes optional instead: `field?: T`; with `#[model_schema_prop(nullable)]` all three surfaces render `T | null` with the key **required** instead: Zod `z.union([T, z.null()])`, JSON Schema `anyOf: [T, {"type": "null"}]` with the key in `required`. Outside a field (an item of a list, a value of a map, a position of a tuple, a type argument, an arm of a `Result`, a slot) serde writes `None` as `null`, and an `Option` is `T | null`, Zod `z.nullable(T)`: `typescript_slot_typename` and `zod_slot_type`
- An alias of an `Option` (`pub type Sounded = Option<u32>;`) publishes the field form, `export type SoundedType = number | undefined;`, and a field typed with it names it. Outside a field a holder writes the type the alias names in place, in the nullable form an `Option` written there takes: `Array<number | null>` and `z.array(z.nullable(...))` for `Vec<Sounded>`, where the alias's binding would read `[null]` as `[undefined]`. `record_option_alias_item`, at `exec_model_schema` ahead of every shape and in every build, records in `OPTION_ALIASES` in `src/utils.rs` (`record_option_alias`, `option_alias`) the type it names as text beside its own type parameters (`OptionTarget`), for an alias of an `Option` and an alias of such an alias, and none for an alias that reaches itself through such aliases, a cycle the compiler refuses, or for any other item under the name. `FieldDef::option_alias_target` parses it back and fills the parameters with the arguments written (`fill_type_parameters`), and `written_in_place` stacks the holder's own array levels over it. `typescript_base` and `zod_array_base` read it below an array level, `typescript_slot_typename` and `zod_slot_type` at any level, `zod_factory_calls` through `zod_slot_factory_calls`, and `contains_type_reference`, `reaches_a_type_declared_later` and `names_a_sibling_binding` through the target. A tuple struct of one slot over such an alias builds its own expression, annotated `ZodType<{Name}>` (`tuple_struct_republishes_slot`). The JSON Schema describes `null` there already. `tests/semantic_types_tests/option_alias_models.rs` holds every position and `LEADS`, what serde writes; `option_aliases.rs` beside it pins the text and the JSON Schema, and `tests/generic_types_tests/declarations.rs` reads each payload under node and types it under `tsc --strict`, `NULL_ROWS` of `tests/untagged_tests/null_models.rs` among them
  - **A field of one is held to every rule of a field written `Option<T>`**, through
    `FieldDef::stands_for_an_option`, in every build: `check_optional_field_serialization` and
    `check_omitted_key_is_readable` (handed it by `field_guard_errors`' callers),
    `validate_nullable_flag` and `validate_ts_optional_flag`, `reach` in
    `src/features/serde/defaults.rs`, the `#[serde(default)]` `hook_member` in
    `src/model_schema/judged_read.rs` hangs beside the hook of such a member (serde reads no absent
    key through a `deserialize_with`), and the absence the recovering decode reads (`walked_field`,
    `keyed_content`). Under `nullable` the field is written in place, `nullable_in_place` keeping
    its docs, as the alias's binding describes a key left out. An alias declared below is not in the
    registry: a field of it keeps the refusal of a key serde requires. `Shoaled`, `Lineman` and
    `Plumbline` in `option_alias_models.rs` hold the three shapes, and
    `tests/recovering_decode_tests/option_aliases.rs` the decode
  - **Kotlin writes an alias of an `Option` that declares a type parameter in place**, as kotlinc
    refuses `typealias HaplyType<T> = T?`: `kotlin_field_def`
    (`FieldDef::with_parameterized_option_aliases_in_place`) builds the def of a member, a brand's
    value, an alias's target, a declared default and a service message the Kotlin emitter reads, and
    `alias_kotlin_tokens` writes no source for such an alias. An alias that declares none keeps its
    `typealias`, and `marked_nullable_at` writes no second `?` (and no second `.nullable`) over it,
    in `kotlin_base`, `kotlin_typename`, `listed_serializer` and `read.rs`'s `annotated_base` and
    `annotated_typename`: `val kept: PlummetType = null`. Swift declares `PlummetType?`, a nested
    optional swiftc takes with no diagnostic, and Dart a class the `?` is not redundant over.
    `run_option_aliases.rs` runs every row under each toolchain, `Deepsea` apart in Dart and Kotlin
    (a struct of one slot read as an array)
  - **A flattened field of one is read and written as the `Option` it names, in each client.**
    `FieldDef::written_in_place` answers the field with its type written as that `Option`:
    `collect_swift_fields` and `collect_kotlin_fields` read the member through it, the alias
    being a `typealias` of the `Option` there, and `collect_dart_fields` keeps the alias's
    class as the member's type and reads and writes its `value` as that `Option`
    (`DartField::read_as`, `wire_def`): `null` where none of the keys of the type it names is
    there, and no key for its `None`. `Bottomed`, `Groundage` and `Grounding` in
    `option_alias_models.rs` hold a flattened `Option` and a flattened alias of one under no
    attribute that drops a key
- `Vec<T>` → `Array<T>`
- `HashSet<T>`, `BTreeSet<T>` and `IndexSet<T>` (under `indexmap`) → the array a `Vec<T>` writes, its items distinct. `is_set_wrapper` in `src/field_type.rs`, beside `is_sequence_wrapper`, is the one list of sets, and `collection_element_field` records each array level written as one in `FieldDef::set_levels`, numbered as `array_lengths` numbers its levels, so the fact survives every merge into array levels. The JSON Schema writes `"uniqueItems": true` at that level (`arrayed_json_schema_value`, `arrayed_json_schema_fragment`), and Zod `zod_repeat_check` after the level's count, which compares the items it parsed by their `JSON.stringify` and pushes its issue with `continue: true`. The `validate()` of each client answers a repeat at every set level, a member's own set ahead of its count: `own_repeat_elements` and `listed_at` in Dart, comparing the JSON its codec writes for each item; `own_repeat_rules` and `held_walk` under `Walk::Reported` in Swift, comparing the `JSONEncoder` texts with sorted keys; `own_repeat_statements` from `Rules::member` and `held_statements` in Kotlin, comparing the values. A brand over a set answers it too, and no bound a variant of an untagged enum is chosen by holds it. Swift's `reaches` asks `Walk::Rules`, which writes no repeat, so the read of a set is what it was; a tuple that holds a set is read unjudged, its `validate()` answering the repeat. The sentence is `REPEATED_ITEM` in `src/bound_message.rs`, written by `zod_repeat_check`, `dart_repeated`, `swift_repeated` and `kotlin_repeated`. Rust's read keeps one item of a repeat, and its `validate()` answers nothing of one; TypeScript writes `Array<T>`. `tests/set_tests/models.rs` holds the models and `PRINTED`, the table Zod and each client print, and `assert_rust_keeps_one_item`, what Rust's read answers of the same payloads
- `HashMap<String, T>` → `Partial<Record<string, T>>`
- Custom types → Reference by the name the referenced type publishes (its ident, or its `name` override)
- A unit struct (`pub struct Ping;`, no braces) → `Record<string, never>` (unchanged); the wire is `{}` both ways: the macro replaces the author's own `Serialize`/`Deserialize` derive with impls writing and reading `{}` (serde's own derive writes and reads `null`, refusing `{}`). Kotlin renders a serializable `object Ping` rather than a `class`; Dart and Swift render their own no-data type. Declared above a service as `Result<Ping, E>`, it reads like a `()` success on every client — a braced `struct Ping {}` is a different shape and is unaffected
- `ObjectId` → `ObjectId` (with $oid validation): Zod a `z.strictObject` of `$oid` and the JSON
  Schema an object of `$oid` with `additionalProperties: false`, refusing a key beside `$oid` as
  serde refuses it wherever the `ObjectId` is held as a value. A flattened one is joined as a
  flattened struct is, and so is one in the lone slot of a variant under a tag alone. The clients
  name `ObjectId` and the consuming project declares it, and README.md's "MongoDB ObjectId
  Support" section shows the Dart, Swift and Kotlin types to declare: each refuses a key beside
  `$oid` and a `$oid` that is not 24 hexadecimal digits in lower case, and the Swift one is public
  and `Sendable`. `tests/service_schema_emitted_client_tests/object_id_models.rs` holds an
  `ObjectId` in each position and what serde, the schemas and each client take of each payload
  (`rows`), and `Waxing`, which flattens an `Option` of its type parameter, filled at an
  `ObjectId`, a struct, one the Kotlin client reads by hand, an enum under a tag alone, a struct
  declared below it and through a type that hands the parameter on;
  `tests/generic_types_tests/declarations.rs` holds Zod under node after `tsc --strict`
  and the JSON Schema to it, and `run_dart_object_ids.rs`, `run_swift_object_ids.rs` and
  `run_kotlin_object_ids.rs` compile the README's blocks (`object_id_runs.rs`) beside the
  emitted types, warnings as errors, and read and write each payload against serde, each client's
  answer in a column of its own (`Told`). The tagged-slot runs read the README's types too.
- `DateTime<Tz>` → `Date` (Zod `z.union([z.date(), z.string(), z.number()]).pipe(z.coerce.date())`, which refuses `null` and booleans that `z.coerce.date()` alone reads as 1970, ahead of the `null` arm and the boolean member a union lists after the date; `type_check_a_date_refuses_what_is_no_date_in_every_position_that_holds_one` in `tests/generic_types_tests/declarations.rs` loads the schema under `node` and holds each input to it); with `#[model_schema_prop(as_number)]` → `number` (inline epoch-ms coercer)
- `NaiveTime` → `string` (Zod `z.iso.time()` wrapped in an inline preprocessor that also accepts millis-since-start-of-day, and writes the seconds `HH:MM` leaves out)
- `NaiveDateTime` → `string` (Zod `z.iso.datetime({ local: true })` wrapped in an inline preprocessor that drops a `Z` and writes the seconds `T…HH:MM` leaves out, so a client sends the form serde reads: section 14)
- A chrono type's JSON Schema is the one form every other surface describes, from one pair of seams
  in `src/model_schema.rs`: `chrono_json_schema_text` says what the text of a type is held to
  (`ChronoText`), and `chrono_json_schema_item` answers `{"type": "integer"}` for a `DateTime<Tz>`
  under `as_number` (`FieldDef::has_as_number`) and that text for any other. A field, a slot, a
  map's value and an untagged member ask the item, and a map's key and a brand's inner the text
  alone. A `NaiveDate` is `{"type": "string", "format": "date"}` and a `DateTime<Tz>` `{"type":
  "string", "format": "date-time"}`. A `NaiveTime` and a `NaiveDateTime` carry no zone, which RFC
  3339's `time` and `date-time` formats require and serde refuses, so each is held to the `pattern`
  of the text serde writes, whole: `TIME_SENT`, and `DATE`, `T` and `TIME_SENT`, of
  `src/features/text_forms.rs`, spelled through `js_spelling` with `[0-9]` for `\d`
  (`whole_text_pattern`). A brand over one layers its own `pattern` in an `allOf`
  (`BasePattern::Stated`). `tests/std_range_tests/documents.rs` evaluates the `date`, `date-time`
  and `time` formats as RFC 3339 states them, and `tests/chrono_tests/tests.rs` holds each document
  to what serde writes and refuses
- `Range<T>` and `RangeInclusive<T>` → `{ start: T; end: T }`, `RangeFrom<T>` → `{ start: T }`, `RangeTo<T>` → `{ end: T }` (Zod `z.strictObject({ … })`, JSON Schema an object of those members, each `required`, with `additionalProperties: false`), each bound rendered as its type renders anywhere else. `written_std_range` in `src/field_type.rs` is the one seam that says a path is one: one of the four last names with one type argument, written in full under `core::ops` or `std::ops` (`names_std_ops`), or under any other path but a name `is_declared_ident` holds, a `#[model_schema()]` type of the crate's own declared above. A range written by any path but the full one earns the check `src/model_schema/ranges.rs` writes beside the holder in every build, which hands the std range back as the type the path names and fails with E0308 where that is another type. Each client writes a type of its own per range: a Dart record with named members, a Swift struct `{Holder}{Field}Range` (`{Holder}{Field}Argument{N}Range` in a type argument) that declares the parameters its bound reaches, and a Kotlin `@Serializable` data class over its bound's type, one per kind of range an item writes (`KOTLIN_RANGES`). Each refuses an object that holds a key the range does not write, as serde's own read of a range does: the Dart record's `switch` takes it only where every key is a member, and the Swift struct reads its keys through `SwiftSchemaAnyKey` first and throws `typeMismatch`, as Swift's own read of an enum does. `validate()` walks a range's bounds under their keys (`ConstraintWrap::Range`), the judged read reads a model in one unjudged (`range_built_back`), the recovering decode reads one whole (`Expected::Range`), and under `mongodb` a range field is a `Field`. A range as a map key and a bound key on a range field are refused in every build, and a range under `#[serde(flatten)]` and in the lone slot of a variant under a tag alone (`TaggedContent::WrittenBeside`, worded by what serde writes beside the tag) in every build with `serde`
- `Result<T, E>` → `{ Ok: T } | { Err: E }` (Zod `z.union([z.strictObject({ Ok: T }), z.strictObject({ Err: E })])`, JSON Schema a `oneOf` of the two objects, each with its one key `required` and `additionalProperties: false`), each arm rendered as its type renders anywhere else, an arm of `()` as `null`, as `()` is everywhere. `written_std_result` in `src/field_type.rs` is the one seam that says a path is one: `Result` with two type arguments, written in full under `core::result` or `std::result` (`names_std_result`), or under any other path but a name `is_declared_ident` holds; a `Result<T>` of one keeps its reading. A `Result` written by any path but the full one earns the check `is_std_result` that `src/model_schema/ranges.rs` writes beside the holder, which fails with E0308 where the name is another type. Each client writes a type of its own: a Dart sealed class `DartResult{N}` with an `Ok` and an `Err` subclass, one per shape of `()` arms an item writes (`DART_RESULTS`), a Swift enum `{Holder}{Field}Result` of the cases `ok` and `err` that declares the parameters its arms reach, and a Kotlin sealed interface `KotlinResult{N}` with a class or an object per arm and a serializer that reads the object key by key (`KOTLIN_RESULTS`). Each refuses an object of both keys, of neither or of another key, as serde does. `validate()` walks the arm the value holds under its key (`HeldEnd::Arms`, the walk that branches as `HeldEnd::Positions` does), the judged read reads a model in either arm unjudged, built back arm by arm (`walked_read`, `result_read`, which several models of one member hand `MemberHook::leaves`), the recovering decode walks into the arm (`Step::Arms`) and lists an object that names neither arm or both as one `Invalid` at the field, expected `Variants(["Ok", "Err"])`, and under `mongodb` a `Result` field is a `Field`. A `Result` as a map key and a bound key on a `Result` field are refused in every build, and a `Result` under `#[serde(flatten)]` and in the lone slot of a variant under a tag alone in every build with `serde`
- `()` → `null` (Zod `z.null()`, JSON Schema `{"type": "null"}`), the `null` serde writes for it, wherever it is held: a member, a list's item, a tuple's position, a type argument, an arm of a `Result` and a brand's slot (`null & $brand<"Name">`). serde refuses `[]` for it. `FieldDef::is_unit` names it, and the `FieldDefType::Tuple` arm of `typescript_base`, `zod_array_base` and `tuple_json_schema_value` is the one place each surface renders it, the arms of a `Result` read through it. A tuple struct of no slot, and a tuple variant of no slot serde writes, keep the `[]` serde writes for them. `written_non_object_wire` and `scalar_json_type_keyword` answer `null` for `()`, and a leaf written as it carries `WrittenAs::Unit`: flattened, and in the lone slot of a variant under a tag alone, it is refused (below), in words that say serde writes no key, or the tag alone (`TAGGED_UNIT_WRITTEN`, `TaggedContent::Unit` for `()` written in place). A flattened union member over `()` is closed by `FLATTENED_UNIT_MEMBER_SCOPE`. A union declared below is refused by the JSON Schema merge when `json_schema()` runs (`refuse_unit_member`, in the words of the expansion's refusal), by the record its schema module's `Schema` publishes, `WRITES_A_UNIT_MEMBER` (`UNIT_MEMBER_RECORD`, written by `unit_member_record_item`), which an alias, a brand and a struct of one slot over such a union, written by its name, forward; the holder reads it behind a block-local trait that answers `false` for every other type (`unit_member_record`, carried as `MergedSource::unit_member`). A member the union's own expansion does not see as `()`, its type parameter filled at `()` or a type declared below it, sets no record, and the merge reads its `null` as the absence an `Option` offers; `tests/flatten_tests/tests.rs` holds both. A map key `()` is refused as the `null` serde writes (`WRITTEN_AS_NULL`). `tests/branded_newtype_tests/unit_value.rs` pins each surface, and `type_check_a_unit_parses_under_zod_what_serde_reads` holds the Zod schema and the JSON Schema to serde under node. Each client reads `()` from `null` alone and writes it as `null`, refusing any other value in serde's words (`invalid type: [], expected unit`, the value quoted as JSON): Dart in `unit_decode`, with an absent key that holds `()`, or a brand, an alias or an untagged enum over it declared above, refused by `keyed_decode` (`reads_a_unit`, `DART_READS_A_UNIT`); Swift through the struct `unit_shape` writes, `{hint}Tuple`, over a single-value container, with `DecodingError.dataCorrupted`; Kotlin through the `object` `unit_typename` declares once per item, `KotlinUnit{N}`, beside its serializer, with `SerializationException`. A tuple struct of no slot keeps `[]`: Dart reads it through `Held::Slots`, Swift's `Wrapped::Tuple` through `tuple_shape`, and Kotlin writes it as a `class`, kotlinc refusing a `data class` of no property. `tests/service_schema_emitted_client_tests/unit_models.rs` holds the models and what serde reads and refuses, and `run_dart_units.rs`, `run_swift_units.rs` and `run_kotlin_units.rs` hold each client to it.
- A slot of a tuple variant under `skip` → on no surface and in no client, as serde neither writes nor reads it. The web surfaces describe the slots that remain (`push_described_field` leaves the slot out of `field_defs`), and each client's `variant_payload` (Swift's `resolve_variant_payload`) reads the variant's kind through `variant_wire_kind`, so a variant that declares one slot and drops it is a unit there too, and the value of a variant of several slots through `written_slots` in `src/features/serde.rs`: the array of the slots serde writes, each at its place among them, one slot where one is left, and an array of `()` of length 0 where none is, which a client reads and writes as the `[]` serde writes and tells from the `null` of `()`; Swift writes the struct of a tuple of no slot for it (`tuple_shape`). `tests/service_schema_emitted_client_tests/skipped_slot_models.rs` holds the payloads, and `run_skipped_slots.rs` holds each client's read and write of each to serde's and Zod's under its toolchain, and its text to no declaration of a dropped slot
- `#[serde(flatten)]` field → TypeScript intersection (`A & B`), Zod `.and(...)`. A source writing more than one key set — a registered choice, or one reached through an `Option` — multiplies instead: `zod_merged_joins` builds one `.and(...)` chain per key set, `zod_merged_statements` binds the object to `{Type}$OwnSchema` and joins the copies where the object is a struct's own published binding, and `zod_merged_object` writes the object out once per copy where it is an enum variant's fragment of a larger literal. Either way the item contributes exactly one member to whatever union encloses it. A map under `#[serde(flatten)]` beside any other member that writes a key is refused at `exec_model_schema`, ahead of every shape, in every build with `serde`, on a struct and on a variant of an enum under every tagging: serde writes the map's entries at one level with that key, and no TypeScript type describes both. A variant under `skip`, which serde neither writes nor reads, is read past by this refusal and by each refusal beside a tag below: `flattening_levels` hands no such variant on, and `internally_tagged_guard_errors` refuses nothing of its slot, a tuple variant of several slots apart, which serde's derive refuses under a tag alone, skipped or not, as `empty_tuple_variant_errors` refuses one of no slot there, in serde's words. `flattened_map_beside_errors` in `src/model_schema/ranges.rs` answers one error per such map, spanned on it and naming a member beside it; a member under `skip` writes no key. A map flattened alone beside the tag serde writes at its level is refused there too, naming the tag: a struct's own `#[serde(tag = "...")]` (`struct_tag`) and an enum's `tag` with no `content` (`Tagging::Internal`), which `flattened_map_errors` in `src/model_schema.rs` hands `flattened_map_beside_errors` as its `tag`. A variant under a `content` key, under its name or untagged writes no tag beside the map, and builds. The lone slot of a newtype variant under a tag alone is written beside the tag as well, and one that writes a map is refused by `tagged_slot_map_error` in `src/model_schema/ranges.rs`, which `internally_tagged_guard_errors` asks of every slot `tagged_content` answers `TaggedContent::Flattened` for, in every build with `serde`, spanned on the slot and naming the variant, the slot's type and the tag: ``type `Filled`: variant `Counted`, field `0`: `Tally` writes a map, which cannot sit beside the tag `kind` ``. A member writes a map where its type, through `Option` and the transparent wrappers, is a map, or names a type declared above that `writes_an_open_map` recorded (`record_open_map` and `is_open_map` in `src/utils.rs`, kept in every build with `serde`): a struct whose one member on the wire is a flattened map, a brand or a struct of one slot over a map, an alias of one, and an `#[serde(untagged)]` enum with a variant serde writes as one (one slot, or one member on the wire that is flattened). A generic type of those shapes written as its own type parameter is recorded with that parameter's position in `OPEN_MAP_PARAMETERS` (`open_map_parameter_positions`, which reads a parameter through `Option`, the transparent wrappers, what `forwarded_parameter_position` reads and a type `OPEN_MAP_PARAMETERS` holds), a struct with a tag of its own apart, and `def_writes_a_map` reads it as a map where the argument at that position writes one: `Wrap<HashMap<String, i32>>` beside a member is refused with the message of a map. A flattened type parameter is recorded in `FLATTENED_PARAMETERS` (`flattened_parameters_of`, a `FlattenedParameter` naming the type that flattens it, the parameter and the member or tag beside it, `None` where it is flattened alone; the lone slot of a variant under a tag alone is read as flattened beside the tag), one record per parameter, one beside a key ahead of one alone, and so is a parameter the item hands on to such a type at that position (`struct Bag<U> { letter: Letter<U> }` records `U`). `refused_fillings` walks every type the item writes, at any depth, and each `default_types` entry against the item's own record, and hands each argument at a recorded position to a refusal: `filled_map_errors`, beside `flattened_map_beside_errors` in `flattened_map_errors`, refuses each that writes a map beside a key, spanned on the argument: ``type `Post`: field `letter`: `Letter` flattens its type parameter `T` beside `id`, and `HashMap<String, i32>` fills it with a map``. `filled_entry_errors` beside it refuses each that serde writes a variant of there as an entry of its name over `null` (`written_as_an_entry` in `src/model_schema.rs`, which reads `filled_wire_leaves` and `filled_union_members` for `WrittenAs::UnitVariantName` and `WrittenAs::PlainEnumName`): an externally tagged enum with a unit variant, under an `Option` or not, a brand or an alias over one, a plain enum, and an untagged enum with a member over one, at every recorded position, alone included, an `Option` in the lone slot of a variant under a tag alone apart (`FlattenedParameter::slot`, `writes_an_optional`), which `filled_non_object_errors` refuses. The holder's description is written once for every filling, and takes that variant's bare name. `filled_non_object_errors` beside it refuses each filling serde writes as no object, read by `refused_filling` in `src/model_schema.rs` as a member of the same type flattened is read, from the record every build with serde writes (`written_non_object_wire` for a value written in place, `written_non_object` and `flattened_filling_non_object` at a name's `value_filling`): text, a number, a `bool`, a list, a tuple and `()`, under an `Option` or not, and a brand, a struct of one slot, an alias or an untagged enum declared above that serde writes as one. Where the parameter is the lone slot of a variant under a tag alone (`FlattenedParameter::slot`, a `SlotBesideTag` naming the tag and the variant, kept across one parameter's records), it refuses an `Option` too, and a type declared above recorded as one (`RefusedFilling::Optional`), of which serde refuses every write there; an `Option` flattened anywhere else writes `None` as no key, and builds where it holds an object. Its message (`filled_non_object_message`, `filled_non_object_words`) is worded as the refusal of the same value flattened or held beside the tag, the slot's `Option` quoting serde's run-time refusal, and a filling the entry refusal answers for is left to it. `src/model_schema/ranges/tests.rs` holds each refusal and each filling that builds, a type parameter flattened in a model declared at a struct (`refused_flattening`), and `tests/flatten_tests/tests.rs` serde's refusal of each. The message advises naming the enum in place of the parameter where the value is an externally tagged enum, which every surface describes by its entries. The type that flattens its parameter, and every other filling, build. A type declared below, or in another crate, is not looked into. `a_type_parameter_flattened_or_beside_a_tag_is_refused_where_a_variant_written_as_an_entry_fills_it` in `src/model_schema/ranges/tests.rs` holds each shape, and `the_readme_states_the_refusal_of_a_filling_written_as_an_entry_as_the_build_answers_it` the README's quotes. A map keyed by a number or a `bool` at any depth of a member serde reads with every key as text is refused at `exec_model_schema` as well, in every build with `serde`: a member under `#[serde(flatten)]`, on a struct and on a variant's members, and every other member of a variant of an `#[serde(untagged)]` enum or of an enum under a tag alone, which serde reads from a copy of the payload it buffers (`text_read_members`, each with its `TextKeysRead`, which words the refusal). serde reads no number and no `bool` from text, so no entry of such a map reads back. An enum under its name, and one under a tag and a content key, which serde reads where the tag comes first, as it writes it, are not refused, and the lone slot of a variant under a tag alone that writes a map is left to its refusal beside the tag. `text_read_map_key_errors` in `src/model_schema/ranges.rs`, which `flattened_map_errors` calls beside `flattened_map_beside_errors`, answers one error per such member, spanned on it, naming the member and the first such map `bson_refused_keys` finds in its type (the walk the refusal under `"mongodb"` makes: through `Option`, lists, tuples, the transparent wrappers and a map's value, and into each model and alias declared above that holds one, `BSON_REFUSED_HOLDERS` and `FORWARDED_KEY_PARAMETERS`), its key one `is_bson_refused_key` answers for. A member or a variant serde does not read (`read_by_serde`: `skip_deserializing`, or a `deserialize_with` or a `with` of its author's), a key that is the holder's own type parameter, and a type declared below are not refused. A type parameter a type reads, in a value of its own, where serde reads every key as text is recorded in `TEXT_KEY_PARAMETERS` in `src/utils.rs` (`text_key_parameters_of`, a `TextKeyParameter` naming the type, the parameter, its `FlattenedRead`, `Key` where it keys a map there and `Below` where it is read there at all, and its `TextKeysRead`), and so is a parameter the item hands on to such a type; `filled_text_key_errors`, beside `filled_map_errors`, refuses each filling that is a number or a `bool` at a `Key`, or holds a map keyed by one at a `Below`, at any depth of every type the item writes but a member `text_read_map_key_errors` answers for whole, and in each `default_types` entry, spanned on the filling. `tests/foundation_refusals/flattened_number_keyed_map.rs` holds the error of a flattened member, `buffered_number_keyed_map.rs` beside it those of the two enums, and `src/model_schema/ranges/tests.rs` each shape. A flattened map is therefore the only key set of its object, and is joined as a record of every key (`FieldDef::zod_merged_schema`). The JSON Schema merges a flattened map as the description it has under a key (section 3, `flatten_merged_source`); a key no surface writes adds no key. A flattened `ObjectId` merges as its `$oid` object (`flattened_object_id`), the key serde writes among the object's own. An `ObjectId` in the lone slot of a newtype variant under a tag alone joins the tag's object the same way, on every surface: `tagged_content` answers `TaggedContent::Flattened` for it, as for a named type, so TypeScript intersects the tag's object with `ObjectId`, Zod with the `$oid` object, the JSON Schema merges it through `flatten_merged_source`, and each client reads it from the object without the tag and writes its `$oid` key beside the tag. `Stamping` in `tests/service_schema_emitted_client_tests/tagged_slot_models.rs` holds the clients to serde, `DeclaredStamp` in `tests/generic_types_tests/declarations.rs` TypeScript and Zod, and `TaggedOidHolder` in `tests/flatten_tests/tests.rs` the JSON Schema. The merge (`merged_tree` and `map_handing` in `src/features/jsonschema.rs`) reads as a map a leaf of a source `MergedSource::map` marks, a map keyed by a plain enum among them, and a leaf whose `additionalProperties` is not `false`, a struct with a flattened map and an alias of a map among them: the finished object's `additionalProperties` is the map's values, and its `propertyNames` the map's where it names some (`handed_beside`, `handed_out`). `tests/std_range_tests/documents.rs` evaluates `propertyNames`, `pattern` and the `date` format. An object that merges no map is written as it was. serde reads an `Option` of such a map, the field's own or an alias's, as `None` wherever an entry does not read as the map, and every entry stays in the object, so its absence holds every key with any value: `flattened_as_entries` in `src/model_schema.rs` says which source is read as entries, through `def_writes_a_map`, the reading of the refusal, Zod joins `ZOD_DROPPED_ENTRIES` in place of the bare object where `MergedOperand::entries` holds (`zod_merged_joins`), and the JSON Schema writes `Branches::Dropped`, `additionalProperties: true`, in place of `Branches::Absent` where `MergedSource::entries` holds (`entries_left`). An `Option` of a struct, or of an enum that writes no map, keeps its two key sets. TypeScript, Zod and the JSON Schema therefore refuse a payload whose keys do not read as that source, which serde reads as `None`, dropping the keys, as `README.md` states and `Hawser` in `tests/flatten_tests/maps.rs` holds. `tests/flatten_tests/maps.rs` holds the models of a map flattened alone in each place, an optional one among them, an optional generic brand filled at a map and an optional untagged enum with a map among its variants, and what serde reads of each payload, `tests/flatten_tests/map_schemas.rs` the Zod text, `tests/flatten_tests/map_documents.rs` the JSON Schema of each model held to what serde reads, `src/model_schema/ranges/tests.rs` each refusal and each shape that builds, and `tests/generic_types_tests/declarations.rs` parses each payload under node. A plain enum flattened into a struct or into a variant's members, an alias of one among them, and one held by the lone slot of a variant under a tag alone are refused in every build with `serde`, a build of the clients alone included: serde writes it as the key naming its variant over `null`, which no schema and no client reads. So is a value serde writes as a plain enum's variant name by way of a type declared above, named as written (`filled_spelling`): a brand or a struct of one slot over one, a generic type filled at one, a type whose value is one of those, and, flattened, an `Option` of any of them or in any of them. `written_as_a_plain_enum` is the one question: it reads `is_recorded_plain_enum` in `src/utils.rs` through `Option` and at a generic type's `value_filling`, and `record_item` writes the record in every build with `serde` (`written_as_a_variant_name`: an enum `Tagging::of` answers `Tagging::Plain` for, and a brand, a struct of one slot or an alias whose value `written_as_a_plain_enum` answers for). `flattened_field_guard_error`, from `collect_struct_fields` and `variant_flatten_guard_errors`, and the arm of `internally_tagged_guard_errors` (`TaggedSlot::plain_enum`) raise the refusal, its message closed by `FLATTENED_PLAIN_ENUM_SCOPE`; `flatten_edge_guard_error`, `flattened_name_non_object_wire` and `flattened_name_refused_branch` hold back the refusal of a value written as no object for it, whose words (what serde writes does not join the object, cannot sit beside the tag) are false of it. A union member serde writes as a variant's name, a plain enum's or an externally tagged enum's unit variant's, is recorded with `WireLeaf::written_as` (`WrittenAs::PlainEnumName`, `WrittenAs::UnitVariantName`, and `ZodUnionMember::written_as` beside it), which `non_object_wire`, `external_variant_wire_leaves` and `Surface::enumerated` set; `NonObject::Written` carries it to `flatten_edge_guard_error` and `tagged_non_object_message`, which word it through `WrittenWords`: serde writes the name as a key holding null, closed by `FLATTENED_PLAIN_ENUM_SCOPE` for a plain enum and `FLATTENED_NON_OBJECT_SCOPE` for a unit variant. `a_union_member_written_as_a_variant_name_is_refused_with_the_key_serde_writes_for_it` holds each. A slot under a tag alone whose value writes an `Option` on the way (`writes_an_optional`) is left to `tagged_optional_message`: serde refuses every write of it there. A plain enum declared below, or in another crate, is not refused. `the_flattened_plain_enum_refusal_is_compiled_in_every_build_with_serde` reads the gates off the sources, `a_flattened_plain_enum_declared_above_is_refused_in_every_build_with_serde` holds each refusal and the README's quote of it, and `a_flattened_value_serde_writes_as_a_plain_enum_is_refused_as_the_plain_enum_is`, `a_flattened_value_over_a_plain_enum_through_a_chain_or_an_option_is_refused_as_the_plain_enum_is` and `a_tagged_newtype_variant_over_a_value_serde_writes_as_a_plain_enum_is_refused_as_the_plain_enum_is` hold each shape over one and the README's quotes. Any other type declared above that serde writes as no object, flattened into a struct or into a variant's members, is refused in every build with `serde` too: a brand or a struct of one slot over text, a number, a `bool` or a list, any other tuple struct, an alias of one, and an `#[serde(untagged)]` enum, or an alias of one, with a member serde writes as no object, refused with that member's branch. `flatten_edge_guard_error`, from `collect_struct_fields` and `variant_flatten_guard_errors`, reads by name (`flattened_name_non_object`) `NON_OBJECT_WIRES` in `src/utils.rs` first (`written_non_object`; `record_non_object_wire`, `recorded_non_object_wire`, a `NonObjectWire` of the value's leaves and of each untagged member's), which `record_item` writes in every build with `serde` (`non_object_wire`), then what a Zod surface recorded (`surfaced_non_object`, off `ALIAS_INFO`). Both records are read by the same rules: `published_wire_leaves`, `named_wire_leaves`, `written_non_object_wire`, `flattened_name_refused_branch` and `flattened_name_non_object_wire` are handed the record they read as a `LeavesOf`, and `member_written_leaves` reads an untagged member as `zod_member_leaves` does, a generic type it names apart (below). The message is closed by `FLATTENED_NON_OBJECT_SCOPE`; a type declared below, or in another crate, is not refused, and is refused by the JSON Schema merge when `json_schema()` runs, which words a plain enum's schema (`{"type": "string", "enum": [...]}`), an externally tagged enum's unit variant below the choice the flatten edge offers (`refuse_variant_name`) and `()` (`refuse_unit`) by what serde writes. A flattened value written in place, which no name stands for (`written_in_place`: a list, a sequence, text, a number, a `bool`, a tuple or `()`, under an `Option` or not), is refused by `flatten_edge_guard_error` too, read by `written_non_object_wire` and closed by `FLATTENED_IN_PLACE_SCOPE`; `a_direct_flatten_of_a_value_written_in_place_is_refused_by_its_own_keyword` holds it. `the_record_every_build_with_serde_writes_proves_what_the_zod_surface_proves` holds the two records to one answer, `the_flattened_non_object_refusal_is_compiled_in_every_build_with_serde` reads the gates off the sources, and `a_flattened_type_serde_writes_as_no_object_declared_above_is_refused_in_every_build_with_serde` holds each refusal and the README's quote of it. The lone slot of a newtype variant under a tag alone that names such a type declared above is refused in every build with `serde` too, as serde refuses every write and every read of a brand over text and of a tuple struct there: `tagged_non_object_message`, from the `TaggedContent::Flattened` arm of `internally_tagged_guard_errors` after the map, the plain-enum and the `Option` refusals, reads `written_non_object` alone, so the verdict is the same with a schema surface and without, names the variant, the enum, the slot's type and what it writes, and is closed by `FLATTENED_NON_OBJECT_SCOPE`. A struct, a brand over one and an untagged enum of objects are not refused; a type declared below is refused by the JSON Schema merge when `json_schema()` runs. `the_tagged_non_object_refusal_is_compiled_in_every_build_with_serde` reads the gates off the sources, and `a_tagged_newtype_variant_over_a_type_serde_writes_as_no_object_is_refused_in_every_build_with_serde` holds each refusal, the slots left alone and the README's quote of it. A slot that names a type declared above that serde writes as an `Option`, a struct of one slot or a brand over one and an alias of one, is refused in every build with `serde` as well, as serde's tag refuses every write of it, of `Some` and of `None` alike: `NonObjectWire::optional`, which `non_object_wire` sets through `writes_an_optional` for a brand's, a struct of one slot's and an alias's value, the value an `Option` or a name recorded as one, and `tagged_optional_message`, asked ahead of `tagged_non_object_message`, as serde's tag refuses an `Option` before it reads what the `Option` holds, names the variant, the enum and the slot's type in the words `tagged_content` refuses an `Option` in (`refused_as("an optional")`), closed by `TAGGED_OPTIONAL_SCOPE`, the reach `FLATTENED_NON_OBJECT_SCOPE` states. `tagged_content` quotes serde's run-time refusal of a slot word for word: a `char` as `a char`, a fixed-size array, which it reads off the written type (`held_through_wrappers`), as `a tuple`, and every list, a std set among them (`FieldDef::is_list`), as `a sequence`; the `refused_beside_serde` tests in `src/model_schema/tests.rs` hold each quote to what `serde_json::to_string` answers for the same declaration. So an `Option` of a value written as no object is refused as an `Option` beside the tag (`a_tagged_optional_of_a_value_written_as_no_object_is_refused_as_an_optional`). The flatten refusal reads no such flag: serde writes a flattened `None` as no key and reads it back. A type declared below is refused by the JSON Schema merge when `json_schema()` runs: `MergeDiagnostic::beside_tag` marks the content of a newtype variant under a tag alone, and `branch_expansion` refuses a `null` among that content's choices (`refuse_tagged_null`), ahead of every other choice, in words true of an `Option`'s `null` and of `()`'s, where a flattened source reads it as the absence. `tests/tuple_variant_tests/tests.rs` holds it beside a flattened `Option` the merge reads as before. `the_tagged_optional_refusal_is_compiled_in_every_build_with_serde` reads the gates off the sources, and `a_tagged_newtype_variant_over_a_type_serde_writes_as_an_optional_is_refused_in_every_build_with_serde` holds each refusal, the shapes left alone and the README's quote of it. A generic type declared above whose value is its own type parameter, a brand, a struct of one slot or an alias over it under the transparent wrappers or through another such type, is read at what fills that parameter: `record_item` records the parameter's position in `VALUE_PARAMETERS` in `src/utils.rs` (`value_parameter_position`, `record_value_parameter`, `value_parameter`), and `TaggedSlot::read` hands both messages the filling, followed through each such type in turn (`value_filling`). An `Option` of the parameter records no position: the type is recorded as an `Option` at every filling. `tagged_non_object_message` reads the filling through `filled_non_object`, `written_non_object` for a name and the JSON type keyword of text, a number, a `bool`, a list or a tuple, and `tagged_optional_message` through `writes_an_optional`; each names the slot's type with its arguments as written (`filled_spelling`): ``wraps `ProbeWrap<String>` ``. A filling serde writes as an object, the enum's own parameter and a type declared below build. `the_tagged_filled_refusal_is_compiled_in_every_build_with_serde` reads the gates off the sources, `a_tagged_newtype_variant_over_a_generic_type_filled_at_no_object_is_refused_in_every_build_with_serde` and `a_tagged_newtype_variant_over_a_generic_type_filled_at_an_optional_is_refused_in_every_build_with_serde` hold each refusal beside the slot of the same shape no parameter fills and the README's quotes, and `a_tagged_newtype_variant_over_a_generic_type_filled_at_an_object_is_accepted` the shapes left alone. A type declared above whose value is such a generic type, filled, a brand, a struct of one slot or an alias over it, itself or under an `Option`, is recorded at that filling: `non_object_wire` reads its leaves through `filled_wire_leaves`, its members through `filled_union_members` and its `Option` flag through `writes_an_optional` of the `value_filling`, so `pub struct ProbeTextFilled(ProbeWrap<String>);` records a `string`, and both refusals of a tagged slot reach it as they reach the type over the filling itself. `without_its_option` and `under_an_option` take an `Option` off a def and put one around its leaves, as `published_wire_leaves` does. A flattened member typed with such a generic type, itself or under an `Option`, is read at its filling by `flatten_edge_guard_error`, any other type by its name (`flattened_name_non_object`): `flattened_filling_non_object` answers `filled_non_object` for a filling that is no `Option`, and for one that is, the branch `refused_branch` proves of its `filled_wire_leaves`, as a struct of one slot over that `Option` is refused. The message names the type with its arguments as written (`filled_spelling`, which reads through `Option` too): ``of `ProbeWrap<String>` ``. Only the record every build with serde writes is read at a filling; the record a Zod surface writes is not. A filling serde writes as an object or as an `Option` of one, a type declared below and the holder's own type parameter build. `the_filled_holder_and_flatten_refusals_are_compiled_in_every_build_with_serde` reads the gates off the sources, `a_tagged_newtype_variant_over_a_holder_of_a_generic_type_filled_at_no_object_is_refused_in_every_build_with_serde`, `a_flattened_generic_type_filled_at_no_object_is_refused_in_every_build_with_serde` and `a_flattened_generic_type_filled_at_an_option_of_no_object_is_refused_in_every_build_with_serde` hold each refusal beside the shape over the filling itself and the README's quotes, and `a_generic_type_filled_at_an_object_is_accepted_flattened_and_held` the shapes left alone. An `#[serde(untagged)]` enum declared above whose newtype member names such a generic type, itself or under an `Option`, is recorded with that member at its filling: `member_written_leaves`, from the untagged arm of `non_object_wire`, takes the member's `Option` off (`without_its_option`, `under_an_option`) and reads the rest at its `value_filling`, so `Text(ProbeWrap<String>)` records the leaf `Text(String)` records, and the flatten and the tagged refusals reach the enum with that member's branch. The record a Zod surface writes still reads the member by its name, so `flattened_name_non_object` reads `written_non_object` ahead of `surfaced_non_object`, and the branch named is the same in every build: `Count(Option<ProbeWrap<u32>>)` is refused at `2.1`, an `integer`, as `Count(Option<u32>)` is. A member filled at a struct, and one over a generic type declared below, build. `the_filled_untagged_member_refusals_are_compiled_in_every_build_with_serde` reads the gates off the sources, `an_untagged_member_over_a_generic_type_filled_at_no_object_is_refused_in_every_build_with_serde` and `an_untagged_member_over_a_generic_type_filled_through_a_chain_or_at_an_option_is_refused_as_its_filling` hold each refusal beside the enum with the member over the filling itself and the README's quotes, and `an_untagged_member_over_a_generic_type_filled_at_an_object_or_declared_below_is_accepted` the shapes left alone. An `#[serde(untagged)]` enum declared above whose newtype member is one of its own type parameters, itself, under `Box` or an `Option`, at a generic type's `value_filling`, or handed on to another such enum, is read at the argument that fills it: `union_parameters_of`, which `record_item` writes in every build with `serde` (`record_union_parameters`, `union_parameters`, `UNION_PARAMETERS` in `src/utils.rs`), records each such leaf of its members with the parameter's position (`UnionParameter`, walked by `member_parameter_leaves` as `member_written_leaves` walks the member), and `written_union_members` puts the leaves `member_written_leaves` reads of each argument the enum is named with in that leaf's place, in branch order, so `ProbeOwnMember<String>` with the member `Held(X)` records what `Held(String)` records, and the flatten and the tagged refusals reach it, and an alias of it filled there, with that member's branch. A union whose members its arguments fill is quoted with them (`union_spelling`, through `filled_spelling`): ``of `ProbeOwnMember<String>` ``. A filling that is an object, the holder's own type parameter, and an enum declared below build. `the_untagged_parameter_refusals_are_compiled_in_every_build_with_serde` reads the gates off the sources, `an_untagged_enum_filled_at_no_object_at_its_own_parameter_is_refused_in_every_build_with_serde` and `an_untagged_enum_filled_at_its_own_parameter_through_a_wrapper_a_holder_or_another_enum_is_refused_as_its_filling` hold each refusal beside the enum with the member over the filling itself and the README's quotes, and `an_untagged_enum_filled_at_an_object_at_its_own_parameter_or_declared_below_is_accepted` the shapes left alone. A brand, a struct of one slot or an alias declared above whose value is such an enum filled at its own type parameters, itself, through a generic type's `value_filling` or through another such holder, is recorded too: `union_parameters_of` reads its value through `filled_union_parameters`, which walks it as `filled_union_members` does, and `named_union_parameters`, which `member_parameter_leaves` shares, places each leaf `union_parameters` records of the union it names at the holder's parameter that fills it, so `ProbeOwnAliasOf<String>` over `pub type ProbeOwnAliasOf<T> = ProbeOwnMember<T>;` records what `ProbeOwnMember<String>` records, and the flatten and the tagged refusals reach it, and an untagged member over it, with that member's branch, quoted with its arguments. A filling that is an object, the holder's own type parameter, and an enum or a holder declared below build. `the_refusals_of_a_holder_of_an_untagged_enum_at_its_own_parameter_are_compiled_in_every_build_with_serde` reads the gates off the sources, `a_holder_of_an_untagged_enum_at_its_own_parameter_filled_at_no_object_is_refused_in_every_build_with_serde` and `a_holder_of_an_untagged_enum_at_its_own_parameter_through_another_type_is_refused_as_its_filling` hold each refusal beside the shape over the enum filled at the same value and the README's quotes, and `a_holder_of_an_untagged_enum_at_its_own_parameter_filled_at_an_object_or_declared_below_is_accepted` the shapes left alone. A brand or a struct of one slot declared above whose value is an `#[serde(untagged)]` enum, itself, through a generic type's `value_filling` or filled at its own parameter, is recorded with that enum's members, as an alias of it is: the struct arm of `non_object_wire` writes `NonObjectWire::members` through `filled_union_members`, so `written_union_members` reads them, and the flatten and the tagged refusals reach `ProbeSlotText(ProbeTextMember)`, a holder of it and an untagged member over it with the branch of the member serde writes as no object, the holder named as written. An enum whose every member is an object, and one declared below, build. `the_refusals_of_a_slot_over_an_untagged_enum_are_compiled_in_every_build_with_serde` reads the gates off the sources, `a_slot_over_an_untagged_enum_with_a_member_that_is_no_object_is_refused_in_every_build_with_serde` and `a_slot_over_an_untagged_enum_through_a_generic_type_or_held_by_one_is_refused_as_the_enum` hold each refusal beside the alias's and the README's quotes, and `a_slot_over_an_untagged_enum_of_objects_or_declared_below_is_accepted` the shapes left alone.
- `#[serde(untagged)]` enum → TypeScript union (`A | B`), Zod `z.union([...])`, JSON Schema `anyOf`. Flattened, it is joined by the members its expansion records (`record_zod_union_members`, `record_ts_union_members`). An enum that declares a type parameter records them with each parameter written as its `parameter_placeholder` (`placed_untagged_member`, on a copy of each variant's members `render_untagged_variant` renders again), and `FieldDef::zod_union_members` and `FieldDef::ts_union_members` fill each with the argument the reference writes at its position, through `placeholders_filled`, the holder's own parameter as the holder declares it; a reference with no argument at a position a member names is joined as the enum's own schema. An alias records the Zod and the TypeScript members through `alias_target_placed`, as it records an externally tagged enum's entries, so TypeScript closes the members of an alias as it closes the enum's. `tests/flatten_tests/tests.rs` (`UntaggedCarriedHolder`, `UntaggedPickedHolder`, `UntaggedPickedAliasHolder`, `InlineUntagAliasHolder`) holds the text, and `LANDINGS` and `ALIASED_PICKS` in `tests/generic_types_tests/declarations.rs` hold it under `tsc --strict` and `node`.
  A variant under `#[serde(untagged)]` in an enum with no `untagged` of its own, which serde
  writes with no name, is refused at `exec_model_schema`, in every build, ahead of every refusal
  that reads a variant beside the enum's tag: `untagged_variant_errors` in `src/model_schema.rs`,
  reading `carries_untagged` in `src/features/serde.rs`, one error per variant serde writes or
  reads, spanned on it. One written ahead of a variant that is not is refused in the words of
  serde's derive, which refuses it, skipped or not; under a tag alone a tuple variant of no slot or
  of several is left to the refusals of a tuple variant, serde's derive refusing it untagged or
  not. An enum untagged as a whole is untouched.
- An externally tagged enum → TypeScript union of each unit variant's name and one object per other variant, each object closing every other object variant's name with `"{other}"?: never` (`join_external_union`, through `sibling_key_exclusions` and `close_tagged_flatten_member`), so an object of two variants' entries, which serde and Zod refuse, does not type-check, and a member is told apart by `value.Bay !== undefined`, every object declaring every name. Flattened, the enum is joined by the entry of each variant, a unit variant's as its name over `null` (`record_external_flatten_operands`), each closed the same way, a member that names a `preprocess` or a `check` too (`FieldDef::flatten_variants_written_around`, `zod_merged_schema`), the functions written around the entries. The lone slot of a newtype variant under a tag alone that names such an enum is joined the same way (`entries_beside_the_tag`, through `variant_flatten_typescript` and `variant_flatten_zod`), as serde writes it beside the tag as it writes it flattened, and so is an alias of the enum, which `process_type_alias` records with the enum's entries (`record_flatten_variants`), and a brand and a struct of one slot over it, which `register_branded_newtype` and `process_tuple_struct` record the same way (`record_newtype_flatten_variants`). An enum that declares a type parameter records its entries with each parameter written as its `parameter_placeholder` (`FieldDef::place_type_parameters`, on a copy of each variant `record_external_flatten_operands` renders again), and `FieldDef::recorded_flatten_variants` fills each placeholder with the argument the reference writes at its position, as a reference to the enum writes it (`typescript_typename`, `zod_type`, through `placeholders_filled` in `src/utils.rs`), the holder's own parameter as the holder declares it; a reference with no argument at a position the entries name is joined as the enum's own schema. An alias that declares a parameter records the entries filled with its own placeholders, which its holder fills in turn. `tests/flatten_tests/tests.rs` (`ExtCarriedHolder`) and `tests/tuple_variant_tests/tests.rs` (`InternalOverCarried`) hold the text, `LADINGS` in `tests/generic_types_tests/declarations.rs` holds it under `tsc --strict` and `node`, and `Cradled` in `tagged_slot_models.rs` under each client. `tests/tuple_variant_tests/tests.rs` holds the slot's text and its JSON Schema, which `merged_object_value` already writes by the entries, against serde, and `DeclaredMooring` in `tests/generic_types_tests/declarations.rs` holds it under `tsc --strict` and `node`. Such a check is handed the enum's own value, a unit variant's bare name (`FlattenVariant::unit`, `zod_check_wrap_handing`), written in the call twice so the function's parameter types it, and `record_external_flatten_operands` writes the checks the enum names on itself on each entry, handed the same value. The absence of a flattened `Option` source closes the source's keys to `never` but for those the object's own members and its other sources write (`ts_merged_operand`, `Exclude<keyof X, ...>`), which a payload holds on either branch. `tests/generic_types_tests/declarations.rs` holds `HATCHES` under `tsc --strict` and `node`, and `YAWS` under `node`, for a `preprocess` and for a `check`
- A member of a struct or of an enum's variant under `#[serde(default)]` → the TypeScript member unchanged, Zod `T.prefault(<json>)`, JSON Schema `"default": <json>` with the key left out of `required`, and each client reading the JSON of the default where the key is left out (section 13)
- Type parameter → TypeScript parameter (`X<IdType>`), Zod `X$SchemaFactory(idType)` (memoized, one cache level per parameter) plus `X$SchemaDefault` — that same factory called at the item's own declared `default_types`, checks included — JSON Schema the document of whatever fills it — `default_types(...)` where the type stands alone, the reference site's arguments where a field embeds it. An argument, and a `default_types` filling in `$SchemaDefault` and its annotation, is written in the slot form (`typescript_slot_typename`, `zod_slot_type`): serde writes a parameter's `None` as `null` wherever the parameter is held, a field typed with it among them, so `X<Option<u32>>` is `X<number | null>` and `X$SchemaFactory(z.nullable(...))`, as the JSON Schema describes it. A lifetime is dropped on every surface, a borrowed value writing what its owned form writes; a const renders as an array length only, and is refused where it is handed to a written type as an argument
- `serde_json::Value` and `serde_json::Map` → refused, in every build, by the path written. No
  target describes either, and none keeps code for them.
  - **The full path is refused at expansion**: `serde_json::Value`, `::serde_json::Value` and
    `serde_json::Map<…>`, a path whose segments are exactly `serde_json` then the name. A compile
    error naming the place, ``model_schema: type `Holding`: field `v` reaches
    `serde_json::Value`, which holds any JSON value, so no target can describe it.`` and the
    rewrite to make. `any_json_errors` asks it at `exec_model_schema`, ahead of every shape, of
    each type `any_json_positions` lists (a field, a slot, a variant's member, the type an alias
    names and each `default_types` filling; a member serde neither writes nor reads, and a member
    of a variant it neither writes nor reads, is not listed), through
    `ranges::any_json_named`, and refuses through
    `UndescribableStd::AnyJson`, the arm beside the std types `undescribable_std_rejection`
    refuses (`OsString`, the cells and locks, `LinkedList`). Those are asked at each position
    under a schema surface; this one is asked once, ungated, because no build may describe it. A
    module of the crate's own named `serde_json` is not told apart from the crate: the owner's
    decision.
  - **A bare name is asked of the compiler.** A `Value` with no arguments and a `Map` with
    arguments, written as one segment, may be a type declared below the item or in another
    crate, which no expansion sees. `any_json_checks`, gathered by `asked_of_the_compiler` beside
    `ranges::identity_checks`, writes one `ranges::any_json_check` beside the item per spelling a
    position writes: `Probe<T>` with an inherent `const ANY_JSON: bool = true` on
    `Probe<::serde_json::Value>` (on `Probe<::serde_json::Map<::std::string::String,
    ::serde_json::Value>>` for `Map`) and a `Fallback` trait's `false` for every other `T`, then
    `::core::assert!(!Probe::<written>::ANY_JSON, refusal)`, spanned on the written type, which
    fails with E0080 and the refusal's message only where the name is `serde_json`'s type. The
    first assert reads the inherent const, so no `dead_code` fires; nothing is allowed.
    `Probe` and `Fallback` are numbered where the written type names them (`unclaimed_parameter`).
    A crate that writes a bare name lists `serde_json`, which the check names.
    `ranges::any_json_bare` keeps no check for a name `is_declared_ident` answers for, one of the
    item's own type parameters, or the item's own name, and none for a `Map<…>` whose arguments
    name what a check outside the item cannot (one of its parameters, `Self`, a lifetime other
    than `'static`). `tests/any_json_refusals.rs` holds a bare `Value` brought in with
    `use serde_json::Value;` to the E0080, and the crate's own `Value` declared below an alias
    that names it to a build with warnings denied; `tests/shadowed_map_tests/named_value.rs`
    lints that shape under this crate's own table.
  - **A `#[service_schema]` operation is held to both**: `any_json_refusals` in
    `src/service_schema/parse.rs`, gathered by `service_refusals`, refuses the full path in each
    type `json_positions` lists (the one message the operation names, each `header_in` and `part`,
    and both arms of the reply, a tuple that carries `header_out` or `error_header_out` taken
    apart), and `any_json_checks` there writes the check of each bare name beside the trait, in the
    expansion that passes every refusal, keeping the context and a declared name. Both word the
    error through `undescribable_std_message`: ``service_schema: operation `fetch`: the request
    reaches `serde_json::Value`, ...``. The message declared from several arguments is a
    `#[model_schema()]` struct, refused or checked by that expansion. tixschema's own APIs that
    take or answer a `serde_json::Value` (a `preprocess` function, `from_value_with`,
    `from_value_piped`, `schema_examples()`, the archie conversions) are untouched

### 5. Zod v4 Requirements

**⚠️ IMPORTANT: This crate requires Zod v4 for full functionality.**

Frontend dependencies:
```bash
npm install zod@^4.0.0
```

Generated schemas use Zod v4's modern syntax:

```typescript
// ✅ Generated (Zod v4 compatible)
export const User$Schema = z.strictObject({
  id: z.string(),
  name: z.string(),
  email: z.union([z.null().transform(() => undefined), z.string(), z.undefined()]).prefault(undefined),      // Modern v4 syntax, null coerced to undefined
  age: z.union([z.null().transform(() => undefined), z.number().int(), z.undefined()]).prefault(undefined),  // Works with JSON schema generation
}).meta({
  examples: []
});

// ❌ OLD FORMAT (no longer generated)
export const User$Schema = z.strictObject({
  id: z.string(),
  name: z.string(),
  email: z.string().optional(),
  age: z.number().int().optional(),
}).transform(args => Object.assign(args, {
  email: args.email,
  age: args.age
}));
```

**Benefits of Zod v4**:
- **JSON Schema Generation**: Can generate JSON schemas from Zod schemas
- **Cleaner Code**: No transform functions needed
- **Better Performance**: No runtime transform overhead

### 6. Serde Attribute Support

The macro respects Serde attributes when the `serde` feature is enabled:

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfile {
    pub user_id: String,  // → userId in TypeScript
    #[serde(rename = "emailAddress")]
    pub email: String,    // → emailAddress in TypeScript
}
```

A struct's own `#[serde(tag = "...")]` writes the struct's serde name under that key, and every
surface describes it. `struct_tag` in `src/features/serde.rs` answers the key and the name (the
ident, or the container's `rename`); `with_struct_tag` puts the pair ahead of the struct's members
as a required string literal and refuses a field that writes the same key, which serde would write
twice. Dart writes it into `toJson`, Swift encodes it under `SwiftSchemaTagCodingKeys`, and Kotlin
holds it in an `@EncodeDefault` property outside the constructor, which the plugin writes after the
members and `{Type}Unjudged` moves ahead of them (`read::tag_first`), where serde writes it. The
three read a payload whose key holds the struct's name, and refuse one without the key or with
another value under it, as TypeScript and Zod refuse it, where serde reads past the key ("The Dart
read refuses what `validate()` refuses", "The Swift read" and "The Kotlin client's read", below).

A variant serde neither writes nor reads, under `skip` or under `skip_serializing` and
`skip_deserializing` together, is described on no surface and read and written by no client, as a
member taken off the wire in both directions is (`push_described_field`). A variant under one of
the two alone is described as every variant is. `README.md`'s "A Variant Serde Neither Writes Nor
Reads" section is the consumer's view; this is what a change has to keep.

- **One question, in every build.** `variant_on_the_wire` in `src/features/serde.rs` reads it, as
  `parse_serde_key_omission` reads a member's.
- **The processors walk such a variant and describe nothing of it.** Each still walks its members
  for `validate()` and takes their keys off, and leaves the variant out of what it describes:
  `collect_plain_enum_options`, which `enum_members()` is written from,
  `collect_discriminated_variants`, the untagged walk (`collect_untagged_members`),
  `render_internal_variants`, the `$Variant` reader pairs (`plain_enum_reader_pairs`,
  `discriminated_variant_reader_pairs`), and the records a holder reads,
  `external_variant_wire_leaves` and the untagged branches of `non_object_wire` and
  `union_parameters_of`, which number a branch among the variants on the wire.
- **No guard of the wire reads it.** `FieldContext::on_the_wire` keeps `field_guard_errors` from
  asking its members `check_optional_field_serialization` and `check_omitted_key_is_readable`, and
  `check_variant_slot_wire_is_readable`, the flatten guards, `empty_tuple_variant_errors` but
  under a tag alone, where serde's derive refuses a tuple variant of no slot skipped or not,
  `adjacent_collapsed_slot_guard_errors`, the refusals `render_untagged_variant` raises, which the
  untagged walk does not call for it, and the enum's string check on a unit variant's name
  (`written_with_no_data`) read past it. A member's own `#[model_schema_prop]` keys are still
  judged.
- **No guard of a member's type reads it, nor a member serde neither writes nor reads.**
  `collect_field_guard_errors` asks the undescribable-std guard, the map-key guard, the Kotlin
  width guard and the flattened range and `Result` guards only of a member serde writes or reads
  in a container it writes or reads, `swift_scanned_fields` lists no other member for the Swift
  width guard, and `any_json_positions` none for the JSON value refusal: serde writes and reads
  none of such a member. A member under one of the two one-way skips is held to each.
- **Plain or not is read off every variant declared.** `is_plain_enum` is asked of the item as
  written, by `process_enum` and by each client, so an enum whose one variant holding data is
  skipped is described as the externally tagged union of its unit variants.
- **The clients write from `variants_on_the_wire`**, the item without such a variant, which
  `enum_dart_tokens`, `enum_swift_tokens` and `enum_kotlin_tokens` hand every writer they call.
- **An enum that declares variants and none on the wire is refused** at `exec_model_schema`, in
  every build, by `no_variant_on_the_wire_errors`, and so is an enum that declares no variant,
  under a tag or not, in words of its own (`tests/foundation_refusals/no_variant.rs` and
  `no_variant_tagged.rs`).
- **The tests.** `tests/skipped_variant_tests/models.rs` holds the models under every tagging and
  `ROWS`, what serde and Zod take of each payload; `tests/skipped_variant_tests/surfaces.rs` pins
  each surface and holds the JSON Schema to the rows; `tests/generic_types_tests/declarations.rs`
  holds Zod to them under `node`; `run_dart_skipped_variants.rs`, `run_swift_skipped_variants.rs`
  and `run_kotlin_skipped_variants.rs` read each row under the toolchain, against serde; and
  `src/model_schema/tests.rs` holds the refusal and each guard that reads past such a variant.

A name serde writes -- a member's key, a tag, a variant's name, a plain enum's value, a struct's own
tag name -- and a `literal` value are written into the TypeScript type and the Zod schema by
`ts_quoted` in `src/utils.rs`, the writer the service code's names go through too. A key that is
an identifier stays bare (`ts_member_key`), and `describe_item_lines` escapes a `description`
through the same `escape_js_double_quoted`. A name with no `"`, `\` or control character is written
as it was. `type_check_a_name_that_holds_a_quote_is_read_under_zod_as_serde_writes_it` in
`tests/generic_types_tests/declarations.rs` compiles the models of
`tests/dart_tests/quoted_models.rs` under `tsc --strict` and parses every payload serde writes.

A text the TypeScript surfaces write into a comment goes through `ts_commented` in `src/utils.rs`,
which spells `*/` as `*\/`, so the comment runs on: each line `item_jsdoc_body` writes (an item's
doc, a member's, a variant's, an alias's, and the name serde writes for a member or a variant with
none), the name `apply_constraint_docs` falls back to, a plain enum's variant doc, and every name
`message::commented` writes into the service code's comments. A text with no `*/` is written as it
was. `type_check_a_name_or_a_doc_that_ends_a_comment_is_read_under_zod_as_serde_writes_it`
compiles the models of `tests/generic_types_tests/commented_models.rs` under `tsc --strict` and
parses every payload serde writes of them.

### 13. A Key Read as Its Default

A key under `#[serde(default)]`, written on a field of a struct or of an enum's variant, with a
function, or on the struct, that the payload leaves out is read as its default on every surface.
The value of a default is known to Rust alone, so each function that answers a text writes it with
serde when it builds the text.
`README.md`'s "A Key Read as Its Default" section is the consumer's view; this is what a change
has to keep.

- **One seam.** `SerdeDefaults::of` in `src/features/serde/defaults.rs` answers, for a struct
  with named fields, which members serde reads as a default, in the order of the fields, and the
  tokens that build the JSON of each: `<T as Default>::default()`, the function `default = "…"`
  names, or the member of the struct's own default, written through the field's
  `serialize_with`, its `with` or the hook `as_number` hangs on it. It reads the attributes serde's
  derive reads, so it answers the same before `process_struct` walks the fields and after.
- **What has no JSON** on a struct, which keeps its key required on every surface but Rust and
  builds: a struct that does not derive both `Serialize` and `Deserialize`, or names `from`,
  `try_from`, `into` or `remote`; a `transparent` struct; a member under `skip`,
  `skip_serializing`, `skip_serializing_if` or `skip_deserializing`; a flattened member; an
  `Option`, which keeps what it had; a type that names a parameter of the struct, a lifetime or
  `Self`; and the struct's own default on a struct that declares a parameter. A list, a set or a
  map under the bare `default` is read as its default whatever its type names, unless it names a
  `serialize_with` or a `with`: `empty_collection` writes `[]` or `{}` for a type whose last name
  `is_sequence_wrapper` or `is_map_wrapper` accepts, a literal `literals()` builds with no
  parameter in scope.
- **An enum.** `SerdeDefaults::of_enum` answers the same for each variant with named fields,
  keyed by variant and member, with the exclusions `of` makes for a struct's. The index runs
  across the whole enum, so each placeholder is unique within the item, and `SerdeDefaults::at`
  and `index` take the variant, `None` for a struct. A variant has no container default: serde
  reads `default` on a variant's field alone. Each enum processor sets `FieldDef::serde_default`
  on a variant's member from it, as `collect_struct_fields` does for a struct's, and the enum's
  schema module declares the holder and the `values_impl`, as `struct_output_with_unit_impls`
  does. Dart's and Kotlin's `variant_payload` and Swift's `named_payload` hand a variant's fields
  the enum's defaults for that variant.
- **No placeholder reaches the text of another item.** It is replaced only by the function that
  answers the item's own text. `FieldDef::zod_read_whole` asks `holds_a_placeholder` of the
  spellings `zod_union_members` and `flatten_variants` record, and `flattened_zod_branches` and
  `zod_member_leaves` ask it where they ask `zod_names_a_preprocess`: a holder, flattening the
  enum or holding it, joins it by its published binding, as it joins a type that names a
  `preprocess` (`ZOD_PREPROCESSED`). The registry still records the members, so the refusal of a
  flattened member that is no object reads them as before.
- **A variant's member with no JSON is refused.** `default_refusals`, beside `of_enum` and written
  from the same `reach` `of_enum` reads so the two cannot drift, answers one error per member under a
  field-level `default` that no surface but Rust can read as serde does, spanned on the field and
  opened by ``model_schema: type `{enum}`: variant `{variant}`, field `{field}`:``: a type that
  names a parameter of the enum, a lifetime or `Self` (`names_any`), other than a list, a set or a
  map under the bare `default` with no `serialize_with` or `with` (`empty_collection`); a flattened
  member; a member under `skip_serializing` or `skip_serializing_if` that is not an `Option`; and
  every such member of an enum that derives `Deserialize` and not `Serialize`, or names `into` or
  `remote`. `serde_attribute_guard_errors` joins them to `rename_direction_guard_errors`, and
  `exec_model_schema` raises the two through one `refused_item_output`, every refusal of the item
  in one expansion, under the rename guard's gate: `serde` with any
  of `typescript`, `zod`, `jsonschema`, `dart`, `swift` and `kotlin`. An `Option`, a member under
  `skip` or `skip_deserializing`, and an enum that derives no `Deserialize` or reads itself
  through `from` or `try_from` are refused nothing: no surface reads them otherwise than serde
  does. A struct is refused none of the four: its key stays as "What has no JSON" says, and it
  builds. `README.md`'s "What does not build on a variant's member" holds the four messages.
- **The value is built where the type is declared.** Each module that answers a text declares
  `SerdeDefaults` (`SerdeDefaults::holder`), and `values_impl` writes
  `impl {module}::SerdeDefaults { pub fn literals() }` beside the struct or the enum, where every
  name its author wrote resolves, inside a function body too: no path from the module reaches a
  type declared in one, and an `impl` written beside the type reaches the module.
- **The text holds a placeholder** (`placeholder`), which `substituted` replaces with the JSON
  when the function that answers the text runs, escaped for the literal it sits in: a Dart
  `'…'`, a Swift or Kotlin `"…"`. Zod writes the JSON as an expression, and the JSON Schema puts
  the value itself into the member's object.
- **Each surface.** Zod: `FieldDef::serde_default`, which `zod_type` writes as
  `.prefault(<json>)` around the whole member schema, and `generate_zod_schema_method`, whose
  `zod_schema()` replaces the placeholder. JSON Schema: `build_field_schema`, `key_is_required`.
  TypeScript writes nothing new. Dart: `DartField::default_at`, `constructor_arguments`. Swift:
  `SwiftField::default_at`, `decode_statement` and `default_statement`, with
  `preprocess::defaulted` reading the JSON through `SwiftSchemaPayload` with the read the member
  has. Kotlin: `KotlinField::default_at`, `property_declaration`, which writes
  `= Json.decodeFromString(<member_serializer>, "…")`, and `own_field_plan` for a struct with a
  flattened member. A default is read with the member's own read, unjudged where `validate()`
  walks it, so `validate()` judges it as it judges a payload. A member whose type names a
  parameter of the class (`KotlinField::empty_default`) is `= emptyList()` or `= emptyMap()`
  instead: an initializer reaches no serializer of the class's parameters.
- **A type with no default is emitted token for token as before**: each module keeps
  `#source.to_owned()`, and no `SerdeDefaults` is declared.
- **No function a `preprocess` names is called for a default**, but by Zod, where `.prefault`
  hands the default to the whole member schema.
- **The tests.** `tests/serde_default_tests/` holds the models, the table every surface is held
  to (`CASES`, `REPORT`), the structs that declare a parameter, read at `String` (`FILLED`,
  `INDEXED` under `indexmap`) and refused a key they require (`REQUIRED`), a member that names a
  `preprocess` (`PREPROCESSED`), the JSON Schema, the Zod text, the README's section, and the
  text of a type with no default on every surface, which the emitter before defaults were read
  writes too; `tests/generic_types_tests/declarations.rs` compiles the Zod schemas with
  `tsc --strict` and parses the tables under node; `run_dart_serde_default.rs`,
  `run_swift_serde_default.rs` and `run_kotlin_serde_default.rs` run the tables under each
  client's toolchain, `Fleet` and `IndexedFleet` among them.
  `Berthing` under each of the four taggings is read in the same places, with the key and
  without it, beside a holder of its untagged form whose Zod text holds no placeholder. Each
  refusal of a variant's member is a `trybuild` case whose `.stderr` holds its message spanned on
  the field, and an enum with two refused members reports both.

### 14. The Form Each Client Sends

Every client sends each value in the one form every target reads: serde, Zod, the JSON Schema and
each other client. The Rust read and write do not change: the clients write the form at the
boundary. `README.md`'s "The Form Each Client Sends" section is the consumer's view; this is what a
change has to keep.

- **Two rules.** A value held in another form of the same value is sent in the form serde writes
  (a `NaiveDateTime`'s `Z` dropped, the seconds `HH:MM` leaves out written, a Dart `DateTime` sent
  in UTC). A value no form serde reads holds is refused before it is sent (an integer outside its
  type's range, an `f32` beyond `F32_READ_BOUND` in size, an `f64` NaN, a `char` of other than one
  code point, a date or time text no rewrite turns into its form). A text in a form serde reads
  beside the one it writes is refused where Zod refuses it (`2024-1-5`). A value already in the
  form is sent as it is.
- **`src/features/text_forms.rs` is the one source of the forms**, under `dart`, `swift` or
  `kotlin`: `TextForm`, its `read`, `sent`, `unsecond` and `unzoned` patterns, the Zod 4.6.5
  patterns `DATE`, `TIME`, `TIME_SENT` and `OFFSET`, and `expected`, the words of a refusal.
  `DATE` and `TIME_SENT` are compiled under `jsonschema` and `chrono` too: the JSON Schema holds a
  `NaiveTime` and a `NaiveDateTime` to them (section 4). Each
  client writes them in its own language: Dart inline (`src/features/dart/forms.rs`), Swift as
  private static functions of the type (`src/features/swift/forms.rs`), Kotlin as one serializer
  object per item and form (`src/features/kotlin/texts.rs`). `TextForm::sends` is the one Rust
  statement of what a client sends of a text, which the unit tests of the module hold.
- **TypeScript sends what the message's Zod check gives back**, over HTTP, the bus seam and the
  WebSocket transport, which carries the seam's messages: the integer ranges and the `f32` range
  of section 4, and the preprocessors of `NaiveTime` and `NaiveDateTime`, decide what is sent.
- **A value written as text is written in the form the Rust dispatcher reads**: a path
  placeholder, a query field, a `header_in` and a text part. `text_types` in
  `src/features/service_schema.rs` lists the types an operation writes as text, which each client
  writes the helpers of its forms for. TypeScript writes a header through its own Zod schema
  (`header_in_build_stmt`, the HTTP client's `header_issues`) and every text through
  `{prefix}HttpWireText`, a `Date` by `toISOString()`. Dart writes a value that is no `String` or
  `bool` through its codec (`dart_wire_text`), Swift through the form helpers or
  `{prefix}WireText` over `{prefix}DatedEncoder` (`swift_wire_text`, `bound_text`), and Kotlin
  through the serializer it reads the value with (`kotlin_wire_text`, `{prefix}HttpSentText`).
  Each refuses as it refuses a message: TypeScript and Swift answer the fault, Dart and Kotlin
  throw. A Swift `Date` is written and read as RFC 3339 text by the client's `DatedEncoder` and
  `DatedDecoder`, a `ws_rpc` header written through a container so the encoder's strategy applies
  (`header_in_type`), and a date `header_out` is read as that text by every client. The Rust
  and TypeScript `http_rest` dispatchers read a path placeholder percent-decoded, as they read a
  query value (`percent_decoded`, `{prefix}HttpPercentDecoded`), so the text every client
  percent-encodes there is the text the implementation is handed; an adapter hands
  `IncomingRequest::new` the path as the request carried it. The Kotlin `ws_rpc` attachment writes
  a reply's value, its declared error and each `header_out` through the serializer the client
  reads them with (`read_serializer`), and answers a text of another form as `handler-panic`, as
  the Dart attachment does.
- **What no client changes is reported, not changed**: an `i64`, `isize`, `u64` or `usize`
  beyond `2^53 - 1` in size, which a JavaScript number does not hold exactly. Years outside 0000
  to 9999 and leap seconds are out of scope.
- **The tests.** `tests/wire_forms_tests/models.rs` holds `WIRE_FORMS`: each value a caller hands
  a client, in a field, a list and, where section 3 allows, a map's key, what serde answers of it,
  and what each client sends (`Sends`). `tests/wire_forms_tests.rs` holds serde to the table and
  the JSON Schema to every payload a client sends, and to refusing a value in another form than
  serde writes; `tests/generic_types_tests/declarations.rs`
  sends each row through the Zod schemas under node; `run_dart_wire_forms.rs`,
  `run_swift_wire_forms.rs` and `run_kotlin_wire_forms.rs` send each row through each client's
  codec under its toolchain, and `run_node_wire_forms.rs` sends a message through the three
  TypeScript clients. `TEXT_FORMS` in `tests/kotlin_tests/text_forms_models.rs` holds what each
  client reads of a text, which `run_kotlin_text_forms.rs`, `run_dart_text_forms.rs` and
  `run_swift_text_forms.rs` run. `tests/service_schema_emitted_client_tests/text_position_rows.rs`
  hands each value of `WIRE_FORMS` to the `http_rest` client of each language in a path, a query,
  a header and a part (`run_node_text_positions.rs`, `run_dart_text_positions.rs`,
  `run_swift_text_positions.rs`, `run_kotlin_text_positions.rs`), and `ENCODED` a slash, a percent
  sign and a character beyond ASCII in a path, and the Rust dispatcher reads each request sent;
  `run_kotlin_service_forms.rs` holds the Kotlin attachment's replies of a date and a `char`;
  `sundial_service.rs` and `run_sundial.rs` send a date message, a `char` and a date header
  through the `http_rest` and `ws_rpc` clients of each language, which read the date reply and
  `header_out` the Rust dispatchers write.

## MongoDB ObjectId Support

The `mongodb` feature provides comprehensive MongoDB ObjectId type support with proper validation and serialization.

### Basic Usage

```rust
use mongodb::bson::oid::ObjectId;

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Document {
    pub id: ObjectId,                           // Scalar ObjectId
    pub author_id: ObjectId,
    pub tags: Vec<ObjectId>,                    // Array of ObjectIds
    pub metadata: HashMap<String, ObjectId>,    // HashMap with ObjectId values
    pub parent_id: Option<ObjectId>,            // Optional ObjectId
    pub related: HashMap<String, Vec<ObjectId>>, // Nested structures
}
```

### Generated TypeScript

```typescript
export type Document = {
  id: ObjectId;
  author_id: ObjectId;
  tags: Array<ObjectId>;
  metadata: Partial<Record<string, ObjectId>>;
  parent_id: ObjectId | undefined;
  related: Partial<Record<string, Array<ObjectId>>>;
};
```

### Generated Zod Schema

```typescript
export const Document$Schema = z.strictObject({
  id: z.strictObject({ $oid: z.string().regex(/^[a-f0-9]{24}$/, { message: "Invalid ObjectId" }) }),
  author_id: z.strictObject({ $oid: z.string().regex(/^[a-f0-9]{24}$/, { message: "Invalid ObjectId" }) }),
  tags: z.array(z.strictObject({ $oid: z.string().regex(/^[a-f0-9]{24}$/, { message: "Invalid ObjectId" }) })),
  metadata: z.record(z.string(), z.strictObject({ $oid: z.string().regex(/^[a-f0-9]{24}$/, { message: "Invalid ObjectId" }) })),
  parent_id: z.union([z.null().transform(() => undefined), z.strictObject({ $oid: z.string().regex(/^[a-f0-9]{24}$/, { message: "Invalid ObjectId" }) }), z.undefined()]).prefault(undefined),
  related: z.record(z.string(), z.array(z.strictObject({ $oid: z.string().regex(/^[a-f0-9]{24}$/, { message: "Invalid ObjectId" }) }))),
}).meta({
  examples: []
});
```

### JSON Serialization Format

ObjectIds serialize to MongoDB's standard JSON format:

```json
{
  "id": { "$oid": "507f1f77bcf86cd799439011" },
  "author_id": { "$oid": "507f1f77bcf86cd799439012" },
  "tags": [
    { "$oid": "507f1f77bcf86cd799439013" },
    { "$oid": "507f1f77bcf86cd799439014" }
  ],
  "metadata": {
    "template": { "$oid": "507f1f77bcf86cd799439015" }
  },
  "parent_id": { "$oid": "507f1f77bcf86cd799439016" },
  "related": {
    "references": [
      { "$oid": "507f1f77bcf86cd799439017" },
      { "$oid": "507f1f77bcf86cd799439018" }
    ]
  }
}
```

### Key Features

- **Validation**: Regex validation ensures 24-character hexadecimal strings
- **Type Safety**: Full TypeScript type checking for ObjectId fields
- **MongoDB Compatibility**: Matches MongoDB's native JSON format (`{ "$oid": "..." }`)
- **Zero Production Overhead**: MongoDB crate is a dev-dependency only
- **Comprehensive Testing**: Tested with real MongoDB library integration
