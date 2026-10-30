# Field Validation

`#[model_schema_prop(...)]`, the generated `validate()`, and the read that refuses what `validate()` refuses.

### 7. Field Validation Attributes (`#[model_schema_prop(...)]`)

All validation constraints generate checks in **Zod, JSON Schema, Rust, Dart, Swift and Kotlin**. In Rust the check runs in `validate()`, which the type's own `Deserialize` calls on the value it read, so a serde read refuses what `validate()` refuses ("A read refuses what `validate()` refuses", below), and in the recovering decode of a type whose `enable` list holds `"decode_with"`, which lists the violations of each value as one issue. While the members are still being read it runs in one position only: a member of an untagged enum, where the check decides which variant the payload is. In Dart, Swift and Kotlin the check runs in the `validate()` every emitted type carries, which the type's read calls on the value it read: `fromJson`, `init(from:)` and `{Type}Serializer`.

A function a `preprocess` names is called by the read alone of each language, ahead of the value, so the read that judges calls it too: `model_schema_read_unjudged` in Rust, `z.preprocess(fn, ...)` in the Zod schema, `fromJsonUnjudged` in Dart, `init(swiftSchemaUnjudged:)` in Swift and `{Type}Unjudged` in Kotlin. A function a `check` names is called by `validate()` in the four that have one, and by one `.check(...)` on the Zod schema. Rust calls the snake case of a name, and every other language the name as written:

| Attribute | Field Type | Zod | JSON Schema | Rust | Dart | Swift | Kotlin |
|-----------|------------|-----|-------------|------------|------|-------|--------|
| `pattern = "regex"` | `String` | `.check(z.regex(/regex/))` | `"pattern"` | `validate()`, which the type's read runs; the member's own hook too in an untagged enum | `validate()`: `RegExp("...").hasMatch(value)` | `validate()`: `value.range(of: "...", options: .regularExpression) == nil` | `validate()`: `Regex("...").containsMatchIn(value)` |
| `minLength = N` | `String` | `.min(N)` | `"minLength"` | `validate()`, which the type's read runs; the member's own hook too in an untagged enum | `validate()`: `value.runes.length` | `validate()`: `value.unicodeScalars.count` | `validate()`: `value.codePointCount(0, value.length)` |
| `maxLength = N` | `String` | `.max(N)` | `"maxLength"` | `validate()`, which the type's read runs; the member's own hook too in an untagged enum | `validate()`: `value.runes.length` | `validate()`: `value.unicodeScalars.count` | `validate()`: `value.codePointCount(0, value.length)` |
| `minLength = N` | a list | `.min(N)` on the array | `"minItems"` | `validate()`, which the type's read runs; the member's own hook too in an untagged enum | `validate()`: `value.length` | `validate()`: `value.count` | `validate()`: `value.size` |
| `maxLength = N` | a list | `.max(N)` on the array | `"maxItems"` | `validate()`, which the type's read runs; the member's own hook too in an untagged enum | `validate()`: `value.length` | `validate()`: `value.count` | `validate()`: `value.size` |
| `minimum = N` | numeric | `.min(N)` | `"minimum"` | `validate()`, which the type's read runs; the member's own hook too in an untagged enum | `validate()`: `value < N` | `validate()`: `value < N` | `validate()`: `value < N`, in the number's own type |
| `maximum = N` | numeric | `.max(N)` | `"maximum"` | `validate()`, which the type's read runs; the member's own hook too in an untagged enum | `validate()`: `value > N` | `validate()`: `value > N` | `validate()`: `value > N`, in the number's own type |
| `literal = "val"` | `String`, and each item of a list | `z.literal("val", { error })` and a refinement that leaves its issue no abort | `{"type": "string", "const": "val"}` | `validate()`, which the type's read runs; the member's own hook too in an untagged enum | `validate()`: `value != 'val'` | `validate()`: `value != "val"` | `validate()`: `value != "val"` |
| `literal = true` | `bool`, and each item of a list | `z.literal(true, { error })` and a refinement that leaves its issue no abort | `{"type": "boolean", "const": true}` | `validate()`, which the type's read runs; the member's own hook too in an untagged enum | `validate()`: `!value` | `validate()`: `!value` | `validate()`: `!value` |
| `literal = 214` | numeric, and each item of a list | `z.literal(214, { error })` and a refinement that leaves its issue no abort; on an `i64` or a `u64` a `z.codec` to `z.literal(214n)` | `{"type": "number", "const": 214}`; on an `i64` or a `u64` `{"type": "integer", "const": 214}` | `validate()`, which the type's read runs; the member's own hook too in an untagged enum | `validate()`: `value != 214`, in the number's own type | `validate()`: `value != 214`, in the number's own type | `validate()`: `value.toUInt() != 214u`, in the number's own type |
| `preprocess = ["fn"]` | any, and a type | `z.preprocess(fn, ...)` | — | every read of the type calls `fn` in snake case on the payload, a `serde_json::Value`, ahead of the value's read | in `fromJsonUnjudged`: `fn` on the payload, a `dynamic`, ahead of the value's read | in `init(swiftSchemaUnjudged:)`: `fn` on the payload, an `Any`, ahead of the value's read | in the read of `{Type}Unjudged`: `fn` on the payload, a `JsonElement`, ahead of the value's read |
| `check = ["fn"]` | any, and a type | `.check((ctx) => { ... fn(own.value, own); ... })` | — | `validate()` calls `fn` in snake case, in every build, and the type's read refuses with its messages; an issue of the recovering decode | `validate()` calls `fn`, and `fromJson` throws `FormatException` with its messages | `validate()` calls `fn`, and `init(from:)` throws `DecodingError.dataCorrupted` with its messages | `validate()` calls `fn`, and `{Type}Serializer` throws `SerializationException` with its messages |
| `ts_optional` | `Option<T>` | — | — | — (TypeScript-only) | — | — | — |
| `as_number` | `DateTime<Tz>` | inline `z.preprocess(..., z.number().int())` | `{"type": "integer"}` | serde `with = chrono::serde::ts_milliseconds` injected | an `int` | an `Int64`, the number serde writes | a `Long` |
| `nullable` | `Option<T>` | `z.union([T, z.null()])`, key required | `anyOf: [T, {"type":"null"}]`, key required | guard: refuses a key-dropping serde attr | `T?`, the one spelling of an `Option` | the key always encoded, `nil` included | no `= null` default on the property |

`README.md`'s "Preprocess and check" section is the one place that states `preprocess` and `check`
for the five languages side by side. It names the section of each language for the detail and
repeats none of it. What a change has to keep:

- **Each cell of its table is text the test of that language holds.**
  `tests/preprocess_and_check_tests/readme.rs` reads the section beside the source of each test:
  a signature is one the test compiles, an error is one its run asserts, and the name of a read
  is one the text of the emitter is pinned to. No test holds the TypeScript error of a name with
  no function, `error TS2304: Cannot find name 'trimmed'.`: it is what `tsc --strict` reports.
- **The 13 lines are one text in the test of each language.** `tests/check_tests/models.rs`
  holds them in `TABLE`, which the Rust `validate()` and the Zod schema are compared with, and
  `run_dart_validate.rs`, `run_swift_validate.rs` and `run_kotlin_validate.rs` each hold a copy
  their run is compared with. The section shows that text.
- **The parcel the section declares is compiled.** `tests/preprocess_and_check_tests/models.rs`
  declares it, its members in the order of their names, and the two reads the section shows are
  what a run answers.
- **A limit of a client is stated in the client's own section.** The section says that limits
  exist and names them in one sentence.
- **The key is a last resort in every place the documents introduce it**: the section, the
  section of the key, and the section of each client.
- **The upgrade list states what a consumer sees**, each item with the text of its error.

**A key written on a field measures the field's own value.** `Option` and the transparent wrappers
(`Box`, `Rc`, `Arc`, `Cow`, and every reference) hold one value and are read through;
a list is a value of its own and is not. So on a list `minLength` and `maxLength` count its items,
whatever they are, and `pattern`, `minimum` and `maximum` are refused. `README.md`'s "List Length"
section is the consumer's view; this is what a change has to keep.

- `constrained_shape` ends on `ConstraintLeaf::List` at an array, a slice or a std sequence
  (`names_a_list`), under whatever wrappers it walked, and reads nothing the list holds.
  `FieldDef::is_list` is the same question asked of the def every surface renders.
- `apply_model_schema_prop_meta` moves a list's two lengths out of `min_length`/`max_length` into
  `ModelSchemaPropMeta::min_items`/`max_items`, so no reader of a string's length finds them. Zod
  writes them on the outermost array (`zod_item_count_checks`), the JSON Schema as `minItems` and
  `maxItems` beside a fixed array's own (`array_length_json_schema_bounds`, the longer minimum and
  the shorter maximum), and `apply_constraint_docs` in the words a string's length is stated in.
- A length on a field typed with a name is asked of the name, not read off the type: with no
  pattern written, `named_leaf_length_rendering` declares `BoundedLength` (a string, a path, a
  slice, an array, `Vec`, `VecDeque`, `BinaryHeap`, `BTreeSet`, `HashSet`) and `UnbrandedLength`
  beside it, and a brand's inherent `model_schema_bound_length` (`branded_bound_text_method`) is
  found ahead of them. `AliasInfo::counted` records what a brand or an alias counts, so a brand
  over a brand forwards (`counted_by`) and the guard (`branded_constraint_inner_error`) lets
  two lengths through to a name that counts items. A pattern keeps asking for the text.
  Zod writes `.check(z.minLength(..))` with `zod_named_error_arg`, whose sentence counts an array
  by its items; the JSON Schema writes `minLength` beside `minItems` (`named_field_json_schema_value`);
  a generic brand whose default is a list counts it at that filling alone, in the Zod default
  (`items`) and the JSON Schema (`branded_layered_over`). Dart counts with `_boundLength`, Swift with
  `swiftSchemaBoundLength` and Kotlin with `kotlinSchemaLength()`, which a brand carries and a
  string or a collection answers as an extension.
- `list_validator` and `list_deserializer` write `validate_{stem}_value` and `deserialize_{stem}`
  over any list: the list is the type parameter `L`, held to `for<'held> &'held L: IntoIterator`
  and counted by iterating it, and each transparent wrapper is a parameter named by what it derefs
  to. No type a list holds is written into the schema module, where one of the item's own
  parameters, `Self` and a borrowed lifetime name nothing. A bound on the iterator itself
  (`ExactSizeIterator`) is one the compiler cannot infer `L` under.
- `BoundKind::Scalar` is what a list is not measured by, and `check_bound_kind` refuses it in the
  list's own words, with `item_rule_advice` saying where a rule for each item goes: a brand carries
  a `pattern`, and a range for each number of a list has no place. `check_fixed_shape_constraints`
  asks nothing more of a list, whose items may be of any type.
- A member's own bound does not stand in for what its type declares: `named_member_validate_body`
  runs both, the type's message first and the member's own after, in Rust and in the Dart,
  Swift, Kotlin and Zod surfaces alike (Zod's `.check(...)` runs after the type's own). A list's
  own count and its items' type are two such bounds.
  The read of such a member takes the unjudged hook (`member_read_hook`), so the holder's read
  answers what its `validate()` answers.
- The count is reported in a string's sentence (`Bound::MinItems`, `Bound::MaxItems`), and Zod reads
  the items off the issue behind `Array.isArray`.
- A brand over a list (`held_as_a_list`: the list itself, or behind `Box`/`Rc`/`Arc`/`Cow`) takes the
  two lengths on the type. `branded_value_inner` hands them to the inner's def as its count, so Zod
  and the JSON Schema write them where a field's list carries them, and `branded_list_validation`
  emits the list's validator under the brand's name, with no `Display` bound. `pattern`
  on such a brand is refused by `branded_constraint_inner_error`.

A length or a pattern on a field typed with a type declared elsewhere reaches a brand over a
string. `constrained_shape` ends on `ConstraintLeaf::Named`, the field's validator takes the leaf
itself, and `named_leaf_rendering` reads its text through `model_schema_bound_text`, which
`branded_bound_text_method` publishes on a brand whose inner is a string or a path. An inherent
method is found ahead of a trait's, so the block-local `UnbrandedText` answers only for an alias of
a string or a path, and its `#[diagnostic::on_unimplemented]` is the refusal every other type
gets, at the field. Zod writes the checks as one `.check(…)` on the named schema
(`zod_named_checks`), and JSON Schema narrows it inside an `allOf`
(`named_field_json_schema_value`). `check_fixed_shape_constraints` refuses a range on any named
type and a length on one written with arguments, in every build.

A bound of the wrong kind is refused there as well, by `check_bound_kind`, in every build: a length
or a pattern on a numeric field, a range on a string, a `bool`, a `char` or a field of a type every
surface describes as unknown (`FieldDefType::Unknown`), and a pattern or a
range on a list, under `Option` and any transparent wrapper, naming the field and only the keys of
the wrong kind. The refusal of a range says what the field writes, and only a string's names the
keys that bound it. `FieldDef::unmeasured_bound` is the one question the refusal
and `apply_constraint_docs` are both written from, so the TypeScript comment states no bound the
refusal turned away.

`validate()` reads through every reference, whatever it borrows, as `get_field_def` does for the
schemas: a borrowed `str` is held to its bound as a `String` is, a `&'a String` and a `&'a u32` as
their owned types are, and a borrowed list is counted as an owned one is. `constrained_shape` reads
each reference as `ConstraintWrap::Borrowed`, at any depth, and the walk reads through it with
`&**`. A field read through a reference publishes its validator and no `deserialize_{field}` hook
(`ends_on_a_borrow`, which asks whether any wrapper of the walk is a reference), and
`generate_field_validation` hangs nothing on one, nor on a member its author reads through a
`deserialize_with` or a `with` of their own (`has_serde_read_hook`), serde admitting one reader per
field. A borrowed member of an untagged enum and one read by its author's reader are therefore the
constrained members the read does not check: each is read with no bound written, and `validate()`
answers for the bound. A reference to a type a bound is refused on is
refused the same way: a length on a `&'a bool` is the E0277 a length on a `bool` is
(`tests/bound_text_refusals.rs`), and `blocking_interior_mutability_wrapper` retraces the walk
through a reference, so a bound on a `&'a RefCell<String>` is refused at expansion, naming the
wrapper.

A slot takes the five keys, of a tuple variant and of a tuple struct, and is checked exactly where a
named member in the same position is. `checked_member` is the one seam that names a member for its
helpers: a slot goes by its declaration index, behind its variant where it has one
(`validate_{variant}_{index}_value`, `deserialize_{variant}_{index}`) and alone in a struct
(`validate_{index}_value`, `deserialize_{index}`), a stem no field's can meet, an ident never
starting with a digit. A variant's slot is read off the binding its arm makes
(`positional_member_binding`, `bound_member`), and a struct's off `self` by its index
(`slot_access_expr`).

- The lone slot of an untagged newtype variant is hung with the hook, and `validate()` reports it
  under no name, as it reports what the slot's own type declares. A slot with a check of its own
  and a type that declares one runs both (`slot_validate_body`).
- A slot of a tagged variant is checked by `validate()`, which the enum's read runs on the
  value it read, and by no hook as the slot is read. It is reported where serde writes it, as
  what its type holds is: under the variant's name or the content key, and under its
  position among several slots (`'Code': …`, `'Pair.1': …`, `'data.1': …`). Its validator is
  recorded under the `Validator` gate, and the recovering decode reads it back through
  `slot_validator`: `Slot::validator` for a variant of several slots,
  `WalkedVariant::lone_bound` for a variant of one, each an `Invalid` at the slot's own path.
- A slot of a tuple struct is checked where a named field of a struct is: by `validate()`,
  which the struct's read runs on the value it read, and by no hook as the slot is read. It is
  reported under its position among the slots serde reads, and under no name where it is the
  struct's one slot, as what its type holds is. `collect_tuple_slots` hands each
  slot's validator and `validate()` fragment to `process_tuple_struct` in `TupleSlots`. The
  recovering decode reads the validator
  back through `slot_validator` with no variant: `Slot::validator` for a struct of several slots,
  and for a struct of one, which serde writes as the slot's value, the validator
  `struct_recovering_decode` hands `held_methods`. A list, an `Option` or a model in that slot is
  walked with it as a field's value is, and a plain value is held to it by `lone_slot_bound`,
  which reads the struct whole with its own reader, so a hook on the slot has run before the
  bound is checked.
- A `#[serde(transparent)]` struct of one slot is a brand, and `branded_slot_prop_errors` still
  refuses a `#[model_schema_prop]` on its slot.
- The JSON Schema writes a slot's bounds through `bounded_slot_json_schema` and
  `bounded_slots_json_schema`, a variant's and a tuple struct's alike; Zod reads them off the
  slot's `FieldDef` as it does for any member.

A string check written on an enum, `#[model_schema(minLength = N, maxLength = N, pattern = "...")]`,
is the same check written on each field the enum reaches, enforced exactly where that field's own
is in that position, under every tagging. `member_prop_meta` is the one seam: it reads a member's
own `#[model_schema_prop]` and holds it to the enum's arguments as well, so every surface and gate
reads the merged meta as it reads a member's own. `process_field` asks it through
`FieldContext::enum_args`, which a struct's field leaves `None`, and
`collect_untagged_variant_members` asks it directly.

- `written_as_text` says what is reached: a field written as a string or a path itself, under
  `Option`, an ownership wrapper or a reference. A list and its items, a field typed with
  a named type (a brand included), a number, a map, a tuple and a type parameter are not. It is
  `constrained_shape`'s walk held to `ConstraintLeaf::Str` and `ConstraintLeaf::Path`, written
  apart because it is asked in a build without `serde`;
  `the_reach_of_an_enums_check_is_the_constraint_walks` holds the two together. A member that
  carries a `literal` is not reached: the enum's check never sits beside a literal, which takes no
  other rule. `reached_by_enums_check` is the one question, `written_as_text` and no `literal`:
  `member_prop_meta` asks it before it holds a member to the enum's check, and
  `enum_check_reaches_nothing` asks it of every field, so the refusal and the surfaces cannot
  drift.
- A field with a check of its own is held to both: the longer `minLength`, the shorter `maxLength`,
  and the enum's `pattern` in `ModelSchemaPropMeta::enum_pattern` beside the field's own, which
  `patterns()` yields in that order. Zod writes one `.check(z.regex(…))` each, `both_patterns_json`
  states the two in an `allOf`, and the validator runs one check each. A named leaf is never
  reached, so its three readers (`zod_named_checks`, `named_field_json_schema_value`,
  `named_leaf_rendering`) read `pattern` alone.
- `enum_string_check_refusal_output` answers at `process_item`, ahead of every enum shape and in
  every build, so an enum's `pattern` is judged in every build as a field's is. It refuses a
  `pattern` no surface reads, a check that reaches nothing (`enum_check_reaches_nothing`: no
  variant written with no data and no field `reached_by_enums_check` answers true for), and each
  name of a unit variant that breaks the check (`unit_variant_name_errors`). The refused enum is
  re-emitted without its members' `#[model_schema_prop]` (`without_member_keys`, through
  `refused_item_output`, which every refusal raised before the members are walked goes through),
  so the refusal is the only error rustc reports for it.
- A unit variant is one `variant_wire_kind` publishes as a unit. `variant_wire_names` lists the name
  serde writes for it, then the name it reads and each `alias` where they differ, and
  `checks_broken_by` measures each as the validator measures a value. An untagged enum writes such
  a variant as `null`, under no name, and refuses it on its own. The name of a variant that carries
  data is not measured.

A `check = ["name", ...]` names functions of the consumer's own, on a field and, in
`#[model_schema(...)]`, on a type. `README.md`'s "Checks" section is the consumer's view; this is
what a change has to keep.

- **One reader.** `function_names` in `src/features/model_schema_prop.rs` reads the key for both
  attributes, in every build, into `FunctionName { name, written_at }`, and raises the five refusals,
  each spanned on what was written: no array, an element that is no string literal, a name that is
  no identifier, no name, a name twice. `apply_arg` and `parse_prop_key` both call it, and the key
  is in `KNOWN_ARGS` and `KNOWN_KEYS`. `checks::alias_refusal_output` refuses it on a type alias at
  `process_item`, and `branded_slot_prop_errors` on a brand's slot, as every key written there.
- **Every call is written beside the type**, in the type's own `impl`, by `checks::call` in
  `src/model_schema/checks.rs`: the snake case of the name (`to_snake_case`) as an ident located at
  the string literal, with the macro's own hygiene, so a missing function (E0425) and a bound the
  type does not declare (E0277) are reported at the attribute. No call is written into
  `{type}_schema`: a type declared inside a function body names a function no path from another
  module reaches.
- **The glob does not reach a call.** `validate()` of an item that names no check holds
  `use {type}_schema::*;` and is emitted token for token as before. Where the item names one
  (`ItemChecks::named`, which `names_a_check` answers for a member's bound from the registry
  `record_item_checks` writes at `exec_model_schema`), `validate_method` writes no such import and
  `bound_body` wraps each bound in a block that imports the module for itself. A check named after
  a helper of the module (`validateLabelValue`) therefore calls the consumer's function.
- **The order.** A member's bounds and what its type declares, then that member's checks in the
  order written (`then_checks`), member by member, and the type's own last
  (`ItemChecks::own_calls`). Every one runs.
- **A member's check takes the member's own value.** `checks::reach` walks `Option`
  (`CheckStep::Present`), the ownership wrappers and a reference (`CheckStep::Held`) and stops at
  whatever is under them, a list and a map included; `the_reach_of_a_check_is_the_reach_of_a_bound`
  holds it to the wrappers `constrained_shape` reads. It is reported through
  `MemberPlace::reported`, which `member_place` answers for a bound's member as well: the steps
  serde writes on the way to the member, joined by dots, and nothing where it writes none.
  `nested_under` writes the name.
- **`validate()` in every build.** With `serde` and a schema surface it is the method the bounds
  are in. Without `serde`, and with `serde` and no surface, `validate_method` and
  `checks_validate_impl` publish it with the checks alone, and only for an item that names one.
- **Every shape.** `process_struct`, `process_tuple_struct`, a unit struct, the three tagged enum
  processors and the untagged one (a member's check in its variant's arm, through
  `build_member_check_arms`, and the enum's own after the `match`), a plain enum, and a brand
  (`branded_validate_method`: the brand's bound, then its checks, under no path). A generic type's
  calls are in its generic `impl`, so its function is written once over every filling.
- **Zod.** `zod_check_wrap` in `src/field_type.rs` is the one writer of the wrap. A field's is
  written by `FieldDef::zod_checked_base`: after the bounds, inside the member of the `Option`
  union and of the `nullable` one, on the outermost array after its count, and inside
  `z.preprocess`, `zod_base` wrapping the checked base. A field that is one factory call hands the
  checked call into the union's arrow (`zod_handed_calls`). A member with a check is written around
  (`is_written_around`), so a flattened one is joined whole and never multiplied. A type's is
  written by `zod_published_binding` from `PublishedBinding::checks`, ahead of `.meta` and inside
  the builder of a generic item; a plain enum and a brand write it ahead of their own
  `.meta({ description })`, a brand ahead of `.brand()` too. The wrap's text holds `"\n})"`, so
  `build_branded_delegate_items` puts the examples at the first one after `.meta({\n  description: `.
- **The wrap copies each issue with `continue: true`.** Zod skips a later check after an issue
  without the flag, where Rust runs every one, and `tsc --strict` refuses to set the flag on a
  pushed issue (TS2540).
- **The clients call each check.** The Dart, Swift and Kotlin emitters write a call of the
  function, under the name as written, in the `validate()` every emitted type carries: see
  "`validate()` on the Dart types", "The Swift `validate()`" and "The Kotlin client's
  `validate()`" below. The JSON Schema and the TypeScript type write what they wrote, and no
  expected output of a type without the key changes.
- **The documents' example is a doctest.** `src/check_example.md` is included in
  `model_schema_prop`'s rustdoc under `serde` and a schema surface, and
  `tests/check_tests/readme.rs` holds the README to it. `tests/check_tests/models.rs` is compiled
  by `tests/check_tests.rs` and by `tests/generic_types_tests/declarations.rs`, which parses each
  case under node and compares the two sides as one text. This crate's lints order a struct's
  members by name, so the compiled `Parcel` declares `label` last and reports it after
  `item_code_list`; the doctest declares the members as the documents do.

Multiple constraints on one field are combined. Multiple `preprocess` functions nest:
`z.preprocess(fn1, z.preprocess(fn2, innerSchema))`.

`literal` takes a string, boolean, integer or float literal. The kind written must match what the
field's Rust type can carry — a boolean literal only on a `bool` field, a numeric literal only on a
numeric field (`u8`–`f64`), a string literal only on a `String` field — or the attribute is a
compile error naming both the literal's kind and the field's declared type. An integer literal is
held as `LiteralValue::Integer`, exactly, and a float as `LiteralValue::Number`, an `f64`; each is
rendered without a trailing `.0` on TypeScript and Zod, and the JSON Schema `const` is written under
`"type": "number"`, never `"integer"`, but on an `i64` or a `u64`, `"integer"` (section 4).

A `literal` takes no other rule: it admits one value, so a rule beside it is always met or never
met. `check_literal_alone`, called from `model_schema_prop_guard_errors` right after
`check_literal_kind_match`, refuses a literal written beside `minLength`, `maxLength`, `pattern`,
`minimum`, `maximum` or `check`, in every build, naming the field, the literal and each rule
(`written_constraint_keys`, read before a list's lengths move into `min_items`/`max_items`, then
`check`). `as`, `preprocess`, `ts_optional` and `nullable` hold the value to no rule and stay
allowed beside it. `README.md`'s "Literal Values" section shows the refusal, and
`a_literal_beside_a_rule_is_refused` holds the README to it.

A `literal` is a rule, held where every bound is held: by `validate()`, by the read that runs it,
by the hook of an untagged member, by the recovering decode and by each client's `validate()` and
read. `README.md`'s "Literal Values" section is the consumer's view; this is what a change has to
keep.

- **One sentence.** `Bound::Literal` in `src/bound_message.rs` states `not the literal {written}`,
  the literal as the Zod schema writes it (`214`, `"fixed"`, `true`), and quotes the value back;
  `rust_violation`, `zod_error_arg`, `dart_violation`, `swift_violation` and `kotlin_violation`
  write it, and `VIOLATION_STEMS` holds its stem. `zod_error_arg` escapes the sentence for the
  template it is written into (`js_template_text`).
- **Rust.** `generate_field_validation` hands a member with a literal to
  `generate_literal_validation`: `literal_shape` reads the type through `Option`, the transparent
  wrappers, every reference and each level of a list, and `literal_broken` compares the leaf in its
  own type (`214u8`, `1.5f64`, `!*value`), a path on its `to_string_lossy` text. A literal on a list
  holds each item: its validator names nothing, `validate()` names each item by its index
  (`held_at`, `named_at`), its hook walks the items with no index (`unindexed_walk`), and
  `record_field_validator` records it as judging items, so the recovering decode hands it each item
  (`field_validator_judges_items`, `FieldValidator::items`) and no count of the list. A literal on
  a member reached through an interior-mutability wrapper is refused, as a bound there is
  (`interior_mutability_constraint_guard_error`).
- **A number the field holds no value of is refused.** `check_literal_held`, beside
  `check_literal_kind_match` in every build, refuses a numeric literal `held_literal` in
  `src/field_type.rs` answers `None` for: a fraction or a value out of range on an integer, a value
  an `f32` does not write back, and one no `f64` is.
- **Zod.** `FieldDef::zod_literal` writes `zod_error_arg(Bound::Literal)` on the `z.literal` of a
  member whose meta holds a literal, a struct's own tag apart, and after it a `z.superRefine` that
  runs, through its `when`, on a value of the literal's own JavaScript type alone, and copies the
  literal's issue with `continue: true`. An issue with no abort is what a union reports at the
  value's path, as it reports a broken bound's, so a variant of an externally tagged or an untagged
  enum and an `Option` of a list name a broken literal where it is. A value of another type keeps
  the abort, as a type mismatch does. The refinement changes no type: `.check` answers the schema
  it is written on.
- **The clients** hold the member in its own type and compare there: Dart's `literal_elements`
  from `own_bound_elements`, Swift's `literal_rule` from `bound_rules`, Kotlin's
  `literal_statements` from `bound_statements`, each through an optional and each level of a list,
  an item under its index. Each is a member's bound, so it chooses a variant of an untagged enum
  (`_bounds()`, `swiftSchemaBound{Variant}`, `kotlinSchemaBound()`).
- **The tests.** `tests/model_schema_prop_tests/literal_models.rs` holds `LITERAL_READS`, what each
  payload reads as. `literals.rs` holds serde's read to it, and
  `type_check_a_literal_refuses_under_zod_what_serde_refuses` in
  `tests/generic_types_tests/declarations.rs` the Zod schema under `node`;
  `run_dart_literals.rs`, `run_swift_literals.rs` and `run_kotlin_literals.rs` hold each client
  under its toolchain.

`ts_optional` is a bare flag (no value): it renders an `Option<T>` field as the optional TypeScript key `field?: T` instead of the default `field: T | undefined`. Zod and JSON Schema output are unchanged. It is only valid on `Option<T>` fields, an alias of an `Option` declared above among them (non-`Option` is a compile error), and composes with `as = Type`. The member must also have a key for the flag to make optional, and `validate_ts_optional_flag` refuses both positions where it has none, in every build: a positional slot writes no key at all, and one a serde attribute takes off the wire in both directions (`skip`, or `skip_serializing` and `skip_deserializing` together) is described on no surface. An attribute that drops the key one way only (`skip_serializing_if`, `skip_serializing`) leaves the member standing, and the flag still decides its spelling.

`as_number` is a bare flag (no value): it renders a `DateTime<Tz>` field as a `number` (epoch
milliseconds) with an inline self-contained Zod coercer, instead of the default native `Date`, and
as `{"type": "integer"}` in the JSON Schema. It is only valid on `DateTime<Tz>` fields (anything
else is a compile error) and is honored on a tuple-variant `DateTime<Tz>` enum payload. With the
`serde` feature on, it also injects `#[serde(with = "chrono::serde::ts_milliseconds")]`
(`ts_milliseconds_option` for `Option<DateTime<Utc>>`) so Rust writes the same epoch-milliseconds
number every other surface describes — a consumer's own `chrono` dependency needs its `serde`
feature enabled for that module to exist. What a change has to keep:

- **`as_number_serde_hooks` is the one writer of the hook**, called by `process_field` (a struct's
  field, a tuple struct's slot, a tagged variant's member) and by `collect_untagged_variant_members`
  (an untagged variant's member), after the member's def holds its meta. It hangs chrono's hook in
  each direction the author writes or reads the member through no function of their own: `with`
  where the author wrote neither, `deserialize_with = "...::deserialize"` beside the author's
  `serialize_with`, `serialize_with = "...::serialize"` beside the author's `deserialize_with`, and
  nothing beside the author's `with` or both. Where it hangs the read, a named `Option` with no
  `default` of its own also takes `#[serde(default)]`, as a bound's hook does
  (`needs_injected_default`), so an absent key is `None`. `serialized` in
  `src/features/serde/defaults.rs` writes a default through `ts_milliseconds::serialize` wherever
  the author wrote no writer.
- **`as_number_hook_refusal` refuses the key on a date chrono's hook reads no value of**, in every
  build with `serde` and `chrono`, from `model_schema_prop_guard_errors`: anything but a
  `DateTime<Utc>` or an `Option` of one (`names_a_utc_date`, `names_an_optional_utc_date`), unless
  the author writes and reads the member through functions of their own. The refusal names the type
  as written.
- **A refusal that names what serde writes reads a date under the key as an integer.**
  `with_as_number` hands a def the `as_number` written among a member's attributes, and
  `written_non_object_wire` answers `integer` and `tagged_content` `an integer` for a `DateTime`
  that `has_as_number()` answers for. The def is handed it by `flatten_edge_guard_error`,
  `internally_tagged_guard_errors`, `non_object_wire` (the slot of a struct of one slot and an
  untagged member, the record every build with `serde` writes) and `tuple_struct_surface` (the
  record a Zod surface writes), so a flattened date, the lone slot under a tag alone, and a type
  declared above over one are refused as serde refuses the integer. A bound on such a date is
  refused by `check_fixed_shape_constraints` in the words of the milliseconds.
- `tests/chrono_tests/untagged_millis_models.rs` holds `MILLIS_ROWS`, what serde, each client and
  Zod answer of a date under the key in an untagged member; `tests/chrono_tests/tests.rs` holds
  serde and the JSON Schema to it and the hooks per direction,
  `tests/generic_types_tests/declarations.rs` the Zod schema under `node`, and
  `run_untagged_millis.rs` each client.

`nullable` is a bare flag (no value): on an `Option<T>` field it renders `T | null` with the key **required** on TypeScript, Zod and JSON Schema, instead of the default coercing `T | undefined` with the key left optional. It is only valid on `Option<T>` fields, an alias of an `Option` declared above among them, written in place (non-`Option` is a compile error), is refused together with `ts_optional` (the two disagree about the key), and composes with `preprocess` — the preprocess wrap goes around the whole nullable union. With the `serde` feature on, it is also refused together with a key-dropping serde attribute (`skip_serializing_if`, `skip_serializing`, `skip`) — the flag declares the key always written, so dropping it would let serde write a payload the generated schema does not admit. The mirror guard, `check_optional_field_serialization`, requires that same attribute on a bare `Option<T>` field carrying no `nullable`.
A flattened one is not held to it (`is_flattened_field`): it writes no key of its own, so serde
writes nothing for its `None`, whatever its attributes say, and reads the payload back.

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct User {
    #[model_schema_prop(minLength = 3, maxLength = 50, pattern = "^[a-z0-9_]+$")]
    pub username: String,

    #[model_schema_prop(minimum = 0, maximum = 120)]
    pub age: u32,

    #[model_schema_prop(preprocess = ["epoch_to_date"])]
    pub date_value: NaiveDate,
}
```

#### Generated `validate()` method

When any field has constraints, the macro generates a `validate(&self) -> Result<(), Vec<String>>` method that aggregates all per-field errors. It judges a value built or changed in code, and the type's own `Deserialize` calls it on the value it read:

```rust
let result = my_instance.validate();
match result {
    Ok(()) => println!("Valid"),
    Err(errors) => println!("Errors: {:?}", errors),
}
```

A message names a value by its path in the payload, in single quotes: every step serde writes on
the way to it, joined by dots (`'parcelList.0.itemLabel': too short: minimum length is 1, got 0`).
A step is a key, a position in an array, or nothing where serde writes the value itself. It is
the path an issue of the Zod schema holds, and the one the recovering decode lists. `README.md`'s
"The path a message names" section is the consumer's view; this is what a change has to keep.

| Member | serde writes it | Reported under |
|---|---|---|
| a named field of a struct | under its key | `'itemLabel'` |
| a slot of a tuple struct of several slots | at its position in the array | `'1'`, its position among the slots serde reads |
| the one slot of a struct | as the value itself | no step |
| a named member of a variant, no tag attribute | `{"Loose":{"freightLabel":…}}` | `'Loose.freightLabel'` |
| the one slot of a variant, no tag attribute | `{"Boxed":…}` | `'Boxed'` |
| a slot of a variant of several, no tag attribute | `{"Split":[…]}` | `'Split.1'` |
| a named member, `tag` and `content` | `{"kind":"Loose","held":{"haulLabel":…}}` | `'held.haulLabel'` |
| the one slot, `tag` and `content` | `{"kind":"Boxed","held":…}` | `'held'` |
| a slot of several, `tag` and `content` | `{"kind":"Split","held":[…]}` | `'held.1'` |
| a named member, `tag` alone | beside the tag | `'liftLabel'` |
| the one slot, `tag` alone | its own keys beside the tag | no step |
| a member of an `untagged` variant | as the value itself, or under its key | no step, or the key |

- **Two functions say what serde writes on the way to a member**, in `src/features/serde.rs`,
  and nothing else computes a step. `variant_steps` answers what is written ahead of a variant's
  members, read through `Tagging`: the variant's written name (its own `rename`, else the enum's
  `rename_all`) with no tag attribute, the content key under `tag` and `content`, and nothing
  under `tag` alone and for an untagged enum. `member_steps` answers each member's steps below
  that: a named member under its key (`parse_written_renames`, or the casing it is handed), a
  flattened one under none, the only slot under none, and any other slot under its position
  among the slots serde reads. A slot serde does not read has no position.
- `validate()` and the recovering decode both call them. `steps_of_members` and
  `steps_of_variants` hand each container's steps to `FieldContext::member_steps`, read before
  the walk takes a member's attributes off it, and `member_place` joins a member's steps into
  `MemberPlace::reported`, which `checked_member` holds in `CheckedMember::reported`. A bound, a
  check and the walk below a member are all written from that one place, a flattened member's
  and an untagged variant's among them. `read_members` and `variant_members` in
  `recovering_decode/checks.rs` call the same two, and keep the members serde reads.
- `rename_direction_guard_errors` refuses a `rename` whose two sides differ in every build that
  describes a type, so the key a surface is named by (`get_final_field_name`) is the key
  `member_steps` reads.
- Without the `serde` feature no attribute is read: a variant is under its own name and a member
  is as it is declared, the steps serde's defaults write.
- `CheckedMember::label` and `CheckedMember::stem` keep the Rust ident: a refusal at build time
  and a helper (`validate_{field}_value`, `validate_{variant}_{index}_value`) name the member as
  its author wrote it, a slot by its place among the ones declared.
- A read hook that answers a brand's refusal under a name (`named_refusal`, from
  `read_unjudged_below` and `named_read_hook`) is handed `MemberPlace::reported`, so a read and
  `validate()` name one path. An `Option` adds no step.
- `tests/check_tests/places.rs`, a module of `models.rs`, holds one type per row of the table,
  with a bound on the member and a model held in it. `tests/check_tests/table.rs` holds
  `validate()` to `PLACES`, and `tests/generic_types_tests/declarations.rs` parses each payload
  under node and holds the Zod schema to the same text.
- The index and the key are written by the nested walk alone. `walk_wraps` binds one index per
  list level (`index_binding`) and one entry per map level (`entry_binding`), `held_at` lists
  them after the member's key as `PlaceStep`s, innermost last, and `named_at` writes the name.
  `nested_leaf` reads a key (`key_binding`) only where a report is written under it, so an entry
  that reports nothing costs no text. A member's own bound stops at a list and reads no map
  (`constrained_shape`), so a list's count is reported at the list, with no index.
- The validator of a member is the one sentence both `validate()` and its `deserialize_{field}`
  hook answer with, so the hook names the same key.

`validate()` runs the `validate()` of every value of a declared type a member holds, and
`README.md`'s "A model held in a map, in a slot or in a tuple" section is the consumer's view:

- `held_shape` reads the member's type against its def: through `Option`, the transparent
  wrappers, every reference (`ConstraintWrap::Borrowed`) and each level of a list, as
  `nested_shape` does, into a map's values (`ConstraintWrap::Map`, which
  `is_map_wrapper` decides) and into each position of a tuple written as a type
  (`HeldEnd::Positions`). `held_walk` writes the walk and `nested_leaf` the call. A walk that
  ends on no declared type is `None` and adds no token: a primitive, one of the item's own
  parameters and a set of either are none.
- A slot of a tuple struct and of a tagged variant is walked as a named member is
  (`slot_nested_validate_body`), read and reported through `member_place`, under the steps serde
  writes on the way to it.
- A named member, a flattened one and an untagged variant's slot fall back on
  `reachable_nested_shape` (`member_held_shape`), the walk they had before a map and a tuple were
  read. A set, a deque and a heap are described as a declared type is, so a field written
  `HashSet<String>` is walked to its strings, and its type publishes a `validate()` that reports
  nothing. `named_read_hook` is hung by `reachable_nested_shape` alone: on no map and no tuple,
  on no member read through a reference (`named_below`, as `ends_on_a_borrow` says for a
  member's own bound), and on no member the type's unjudged read reaches (`member_read_hook`).
- A map's key is the text serde writes for it. `written_key_items` declares `MapKey` in the walk's
  block: its `Display` hands the key's own `Serialize` the `Formatter`, serde's serializer for
  which (in serde from 1.0.108) writes a string, a number, a `bool`, a `char`, a unit variant's
  written name and a newtype's inner. `written_key` answers `None` where serde writes no text
  (`UnwrittenKey`, reached for `&MapKey` alone), and `keyed_under` then leaves the key out of the
  name. `validate()` is written under the bounds the type declares, so a map of declared types
  keyed by a type that names one of the item's own parameters is refused at expansion
  (`unwritable_map_key_errors`, asked ahead of every shape, in every build with `serde`) unless the
  item declares `Serialize` for that parameter, in its generics or in a `where` clause. The
  consumer writes `K: Serialize` on the type and `#[serde(bound(serialize = ""))]` beside it: the
  derive otherwise writes `K: Serialize` a second time, and clippy's `trait_duplication_in_bounds`
  refuses that. A map of values that are no declared type is not walked and is refused nothing.
  Nothing here names `serde_json`.
- `nested_under` composes: a name is written into a report one run of steps at a time, innermost
  first, which is what lets a key be left out.

The macro also generates into the type's schema module:
- `validate_{field}_value(&FieldType) -> Result<(), String>` — pure static validator per field
- `deserialize_{variant}_{field}(D) -> Result<FieldType, E>` — serde hook that calls the static validator, generated only for a member of an untagged enum, the one position where a member's check runs as the member is read, and hung on no member its author reads through a `deserialize_with` or a `with` of their own
- A tuple variant's slot writes its declaration index where a named member writes `{field}`: `validate_other_0_value` and `deserialize_other_0` for the lone slot of `Other`

#### A read refuses what `validate()` refuses

A type's read is its own unjudged read, followed by its `validate()`. A holder reads the models it
holds unjudged, so the whole value is judged once, by the type the caller asked for. `README.md`'s
"A read refuses what `validate()` refuses" section is the consumer's view; this is what a change
has to keep.

- `src/model_schema/judged_read.rs` is the seam. `of_struct` and `of_enum` are handed the item and
  whether it publishes `validate()`, from `struct_output_with_unit_impls`, `branded_read`,
  `enum_output_with_decode` and `process_plain_enum`. An item that derives no `Deserialize`, or
  publishes no `validate()` and names no `preprocess` ("A payload corrected before it is read",
  below), gains nothing and is emitted as it was written, token for token.
- serde's derive runs on a copy of the item, `ModelSchemaRemote`, declared inside
  `model_schema_read_unjudged` under `#[serde(remote = "Type")]` (`remote_definition`). `remote`
  is not written on the item itself: the derive would then write inherent `deserialize` and
  `serialize` functions on the type, where `Type::deserialize(..)` would answer the unjudged read
  and clippy's `same_name_method` fires inside the derive's output in the consumer's crate.
  `Deserialize` is taken off the item's derive list (`leave_the_read_to_the_macro`), and the
  item's serde attributes with it where it derives no `Serialize` either. `Serialize` stays the
  derive's, untouched.
- The copy keeps the item's `serde` and `cfg` attributes and no other (`kept_by_the_copy`), is
  written with the macro's own spans (`generated`), and names the type wherever the item writes
  `Self` (`naming_the_type`).
- The written `impl Deserialize` and the function carry the bounds serde's derive writes, which
  `read_bounds` in `src/features/serde/read_bounds.rs` reads off the item: a container, a field
  and a variant `bound`; `Self: Default` for a container `default`; `Deserialize<'de>` for each
  parameter a read field names, none for `PhantomData`, a skipped field, a field or a variant
  read through `with` or `deserialize_with`, and a borrowing `Cow`; `Default` for a bare
  `default` and for a skipped field nothing names a function for; an associated type by its
  path; and the lifetimes `'de` outlives, `'static` among them reading the type at `'static`. A
  bound wider than the derive's refuses a type that builds without it. The copy is given the
  bounds as an explicit `bound(deserialize = ...)` wherever the item's container writes none,
  the derive holding the copy, and not the type, to `Default` for a container `default`.
- The bounds are read off the item as its author wrote it (`authored_read_bounds`). The hook
  `hang_bound_hook` hangs on a constrained member of an untagged enum reads the member's type
  with its `Deserialize`, so it and the `default` beside it are recorded (`record_bound_hook`,
  `BOUND_HOOKS`, cleared with the member hooks) and taken off a copy before `read_bounds` reads
  it: a member over the enum's parameter asks `Deserialize<'de>` of it and no `Default`. The
  recovering decode reads its `read_predicates` the same way.
  `tests/untagged_tests/parameter_bounds.rs` holds the read,
  `a_bound_hook_on_a_member_that_names_a_parameter_keeps_the_bounds_of_serdes_derive` the
  text, and `tests/recovering_decode_tests/generic_types.rs` a filling with no `Default`.
- A holder's hook is recorded as its members are walked (`hook_member`, from `member_read_hook`
  for a field and a slot and from `read_untagged_member_unjudged`), and written when the item's
  read is: `MEMBER_HOOKS` holds each as text, a token outliving no expansion, and
  `exec_model_schema` clears it. `walked_read` reads the member's type along the wrappers `validate()` walks
  (`held_shape`): a declared type itself, or under `Option`, `Vec`, `Box`, `Rc`, `Arc`, a fixed
  array, a set, a deque, a heap, a map's value and each position of a tuple, in any nesting. An
  `Rc` and an `Arc` are read as what they hold and built back with `new`; a list but a `Vec` is
  read as a `Vec` and collected, which is how serde reads it. The recovering decode's
  `reached` reads `Option`, `Vec`, `Box` and a map's value alone. A hook is recorded only where
  `validate()` walks the same wrappers (`held_shape`), which is what judges what the hook leaves
  unjudged, and never for a member serde does not read, one read through a hook of its
  author's, or a type that names a lifetime. Nor is one hung on a member of a variant under
  `skip` or `skip_deserializing` (`variant_members_read`): a tagged variant's members are handed
  it as `FieldContext::container_read_back`, and an untagged variant's are given no hook and the
  `Validator` gate. serde's derive asks nothing of such a member's type, and neither does the read.
- A member written with `Self` or with the item's own type parameters (`Vec<Entry<IdType>>`) is
  hooked as any other: a model filled with the holder's parameter is judged once, with its
  holder, and what fills a parameter itself (`Vec<Held>`) is no model `validate()` walks.
  `HookScope::written` writes `Self` as the item (`naming_the_type`) and makes the hook generic
  over exactly the item's parameters the member's type names, in the order declared, each
  inferred at serde's call from the return type. Every bound sits in its `where` clause, one
  predicate per bounded type and each bound once (`merged`): the item's own bounds and the
  read's predicates (`ReadBounds::predicates`) whose parameters are all the hook's. The type the
  model is read as declares only the parameters the model names, under the item's own bounds on
  them. A bound that ties such a parameter to another of the item's or to one of its lifetimes,
  a read at `'static`, a const parameter, and a parameter named behind a path
  (`IdType::Out`, `<IdType as Trait>`) leave the member to its own `Deserialize`: the hook is
  not written, and `unjudged_hooks` hangs nothing on the member.
- The hooks are functions local to `model_schema_read_unjudged`: `read_unjudged_{stem}`, each
  with a local `Unjudged` newtype whose `Deserialize` calls the leaf's
  `model_schema_read_unjudged`, behind the local `ReadUnjudged` trait. An inherent function is
  found ahead of a trait's, so a type with no unjudged read of its own is read as serde reads
  it. A refusal raised below the member, by a type that judges itself as it is read, is answered
  under the name `validate()` reports the field under (`named_refusal`) where `named_below` says
  the named reader did so, and an outermost `Option` under a key is given `#[serde(default)]` by
  the rule of `needs_injected_default`, the container writing none.
- A brand takes the same read. `branded_read` hands `of_struct` every brand, and one that
  publishes `validate()` (a bound, a count of a list, a check) or names a `preprocess` gains
  `model_schema_read_unjudged` and the written `Deserialize`. No hook is hung on a brand's slot,
  and a brand with none of the four is emitted as it was written. A holder's hook therefore
  reads a brand unjudged, and the holder's `validate()` names it: `'listed.1': too short: ...`.
- `BrandBound` holds what a brand's own bound adds: the validator in the schema module, the call
  `validate()` makes, and `model_schema_validate_bound`, a method of the brand's own that
  answers for the bound alone (`judged_read::BOUND_ALONE`). It has two readers. A member of an
  untagged enum is held to it as the member is read (`HookPlace::choosing`,
  `MemberHook::held_leaf`), behind the local `ChosenByShape` trait, which answers `Ok(())` for a
  reference to any type with no such method: the bound of a brand chooses the variant, as it did
  where the brand refused in a read of its own, and nothing of a model chooses one. And the read
  a recovering decode lists a bounded brand from holds the value to it (`bounded_listed_read`,
  through `brand_recovering_decode`), so the walker lists a broken bound as the refusal of the
  value. A check a brand names chooses no variant: the enum's `validate()` refuses with it.
- A rule of any other model a member of an untagged enum holds, its checks included, and the
  bound on the slot of a struct of one slot, choose no variant, in serde's read and in the Dart,
  Swift and Kotlin reads, by the owner's decision: the first variant whose shape reads is taken,
  and the read then refuses the value. Zod's `z.union` and the JSON Schema's `anyOf` try the
  next variant there, and the README states the limit with its `Picker`.
  `tests/untagged_tests/held_rules.rs` holds serde's answers, the JSON Schema's and the README.
- `named_read_hook` stays for a named field `reachable_nested_shape` walks and `walked_read` does
  not. A model or a brand filling a type parameter judges itself as it is read, and ends the
  read at the first one that breaks a rule.
- A unit struct's `Deserialize` is the macro's already (`unit_struct_serde_impls`), and
  `unit_struct_read` gives it `model_schema_read_unjudged` and the `validate()` call where the
  struct has a check.
- `remote`, `from`, `try_from` and `into` (`serde_conversion`) on an item whose read the macro
  writes are refused by `conversion_refusal`, in every build.
- `model_schema_read_unjudged` is the type's own name. Each name the function declares beside
  the item's own types is numbered where the item writes it (`unclaimed_parameter`): the
  reader's type parameter `D`, the copy `ModelSchemaRemote`, the traits `ReadUnjudged` and
  `ChosenByShape`, and the `Unjudged` of a hook where the member's type writes that name.
- A service's dispatcher reads a request through `unjudged_{operation}`, which
  `message_readers` in `src/service_schema/support.rs` writes into the service module, and calls
  the validator next. The AMQP and WebSocket arm hands it a `serde_json::Deserializer` over the
  payload and ends it, as `from_slice` does, and the HTTP dispatcher the value it carried.
- `tests/validation_tests/refusing_read.rs` holds each shape, the bounds and the README's
  sample; `tests/branded_newtype_tests/read.rs` a brand under each inner, held in each place and
  in an untagged enum; `tests/check_tests/read.rs` the read of a type that names a check, in
  every build; `tests/recovering_decode_bson_tests/refused_read.rs` a BSON value under each
  major; `tests/judged_once_tests/` a model held under the holder's own parameter or `Self`, in
  each wrapper and beside each bound, its checks counted on the thread that runs them.

#### A payload corrected before it is read

A `preprocess = ["name", ...]` names functions of the consumer's own, on a field and, in
`#[model_schema(...)]`, on a type: each takes a payload and answers the payload the value is read
from. Rust calls the snake case of the name, `fn trimmed(payload: serde_json::Value) ->
serde_json::Value`, the Zod schema the name as written, `function trimmed(payload: unknown):
unknown`, and each client's read the name as written: `dynamic trimmed(dynamic payload)` in Dart
("A `preprocess` in the Dart read", below), `func trimmed(_ payload: Any) -> Any` in Swift ("The
Swift `preprocess`") and `fun trimmed(payload: JsonElement): JsonElement` in Kotlin ("The Kotlin
client's `preprocess`"). The documents call the key a last resort, and a change
keeps them saying so: a function runs on every read, on a payload no schema describes. `README.md`'s "Preprocessing" section is
the consumer's view; this is what a change has to keep.

- **One reader, the one of `check`.** `function_names` reads the key for both attributes, in
  every build, with the five refusals of a `check` under the key's own words
  (`PREPROCESS_TAKES`). `apply_arg` and `parse_prop_key` both call it, and the key is in
  `KNOWN_ARGS` and `KNOWN_KEYS`. `preprocess::alias_refusal_output` refuses it on a type alias at
  `process_item`, and `branded_slot_prop_errors` on a brand's slot: a brand takes it on the type.
- **`src/model_schema/preprocess.rs` holds what the item names.** `record`, called from
  `record_item_functions` at `exec_model_schema`, writes the type's functions and each member's,
  by variant and index, and `named()` is what the read and the recovering decode read. An item
  that derives no `Deserialize` records nothing, and a member serde never reads neither: nothing
  is called for either, and nothing is refused.
- **Every call is written beside the type**, by `preprocess::corrected`: the snake case of the
  name as an ident located at the string literal (`checks::located`), each call over the answer
  of the one before, so a missing function is E0425 at the attribute. No call is written into
  `{type}_schema`.
- **The read is the judged read's.** `judged` in `judged_read.rs` answers for an item that
  publishes `validate()` or names a function, and `written_read` calls `validate()` only where
  one is published. Where the type names functions of its own, `unjudged_read` writes
  `model_schema_read_unjudged` as the payload read into a `serde_json::Value`, the calls, and
  `model_schema_read_corrected` over what they answer, which is the copy's read; the refusal of
  a corrected payload is told through `told_by`. The order of one read is the type's functions,
  each member's as it is read, the value, then `validate()`.
- **A member's functions are called by a function local to the read, hung on the copy.**
  `hooked` writes one per member, of three kinds. Alone, `read_corrected_{stem}`, generic over
  what it reads. Where the member already has `read_unjudged_{stem}`, that one function corrects
  the payload first (`MemberHook::function`). Where the copy keeps a `deserialize_with` or a
  `with` of tixschema's own, an untagged member's bound hook and the one `as_number` injects,
  `corrected_hook` reads the corrected payload through that function and takes the attribute's
  place on the copy (`HungOn::replaces`): under the type the member is written with, or, for a
  counted list, under the header `list_hook_header` shares with `list_deserializer`. A key the
  payload does not hold is serde's to answer, and no function is called for it, where the Zod
  schema calls the field's function with `undefined`: the documents state both, and
  `type_check_a_function_is_handed_under_zod_what_a_rust_read_does_not_hand_it` holds them.
- **Every name a read declares is its own.** `Corrects` holds `payload`, `corrected` and the
  reader's parameter, each numbered where a function is called under it, and both outer
  functions take the reader under that name: a hook nested in them is a `fn` item, which cannot
  name a parameter of the function around it.
- **The refusals are raised at `exec_model_schema`, in every build.** `preprocess::refusals`,
  through `preprocess_refusal_output`: a member read through a `deserialize_with` or a `with` of
  its author's (`read_hook`), a member that borrows, and a type that holds one
  (`read_bounds::borrows`). The item is emitted stripped of its member keys
  (`without_member_keys`). `conversion_refusal` refuses `remote`, `from`, `try_from` and `into`
  in the words of a type that names a `preprocess`.
- **A unit struct and a brand.** `unit_struct_correction` writes the two functions of a unit
  struct around the visitor `unit_struct_read` already writes, and `branded_read` gives a brand
  the judged read where it names a function, as where it publishes `validate()`.
- **The recovering decode corrects once.** `Walker::corrects_own` is read off `named().own`. An
  entry point calls `decode_with_{source}_corrected` on the value as it takes it, reads through
  `decode_with_read_corrected` and walks through `decode_with_{source}_corrected_issues`,
  `_named` and `_fields`; the three public names stay, for a holder, each correcting a copy
  first (`corrected_entries`). Under BSON the value crosses through `bson::Deserializer::new` and
  back through `bson::Serializer::new`, and a value that cannot cross is one `Undescribed` with
  no callback. A member that names functions is a leaf read whole through
  `preprocess::corrected_reader`, with nothing written back (`Whole::corrected`), so its
  function is called by the read and by the listing.
  `a_type_that_names_a_preprocess_adds_the_walk_of_a_corrected_value` pins the names in order.
- **One writer of the Zod wrap.** `zod_preprocess_wrap` in `src/field_type.rs` writes a field's
  and a type's, the first name outermost. `zod_published_binding` wraps what `zod_check_wrap`
  answers (`PublishedBinding::preprocess`), inside the `.meta` and inside a generic's builder;
  `branded_zod_expression` wraps ahead of `.brand()`, and
  `generate_plain_enum_zod_schema_method` ahead of the `.meta` a plain enum writes itself.
- **A holder reads the binding.** `record_zod_binding_preprocess` notes which bindings call a
  function (`ZOD_PREPROCESSED`), an alias of one included, and `flattened_zod_branches` and
  `zod_member_leaves` write no member of such a type in its place: a flattened choice that names
  one is joined by its binding, the one schema that calls the function.
- **The tests.** `src/model_schema/preprocess/tests.rs` holds the refusals and the emitted read;
  `tests/pattern_preprocess_tests/` the models, the reads of every shape (`read.rs`), the chained
  hooks (`chained.rs`), the Zod text (`zod.rs`) and the README (`readme.rs`);
  `tests/recovering_decode_tests/preprocessed.rs` and
  `tests/recovering_decode_bson_tests/preprocessed.rs` the recovering decode, the second under
  each `bson` major; `tests/service_schema_dispatch_tests/corrected_request_service.rs` a
  dispatched request. `tests/generic_types_tests/declarations.rs` compiles the models in a
  bundle of their own (`corrections`) and holds what node prints to what the Rust read prints,
  from one table (`SAMPLE`, `POSITIONS`).

`validate()` is written in the type's own `impl`, beside the schema delegates
(`assemble_schema_output`, `assemble_branded_output`), so a type that declares a parameter has it
at every filling. `README.md`'s "A type that declares a parameter" section is the consumer's view;
this is what a change has to keep.

- A holder's `value.validate()` finds the method whatever fills the type it holds. The block-local
  fallback (`unpublished_validate_fallback`) answers only for a type that publishes none, and it
  answers `Ok(())` without a word: a `validate()` moved to an `impl` of one filling, or under a
  bound the holder's parameter lacks, is a check lost in every generic holder.
- A struct's and an enum's body builds under the type's own bounds: a bound is written on a
  string, a number, a list counted whole or a named type, and one on a bare parameter is refused.
- A generic brand's string checks are its declared default's. Its `validate()` hands the value
  to `validate_at_declared_default`, which `embedded_validate_at_declared_default` emits beside
  `type_identity`: where the two types are one it reads the value as the declared default
  through a pointer cast and runs `validate_value`, and at any other filling it answers `Ok(())`,
  so nothing asks `Display` of the parameter. A brand whose inner names no parameter takes the
  plain `validate_value` and none of this. `TypeId` does not tell two lifetimes apart, so the
  function declares `'held` for its borrow and `at_one_lifetime` writes the default with every
  lifetime it writes replaced by that one: the value outlives the borrow, and is never read at
  a lifetime longer than its own. It answers the result and no reference. A brand over a list
  counts its items at every filling and takes no such function.
- The macro adds no bound to a brand's declaration, and a brand's read asks of a parameter what
  serde's derive asks (`read_bounds`). Nothing asks `Display` of a parameter, on the read or in
  `validate()`.
- A consumer's own `validate` on one filling is a duplicate definition (E0592).
  `struct_validate_method`'s doc comment holds the run.
