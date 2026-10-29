# The Dart Client

`validate()`, the read and `preprocess` on the Dart types.

#### `validate()` on the Dart types

Every type `src/features/dart.rs` writes has `List<String> validate()`, which runs what the Rust
`validate()` of the type runs, in its order. `README.md`'s "`validate()` in Dart" section is the
consumer's view; this is what a change has to keep.

- **The dispatch is handed what it reads of the attribute.** `dart_suffix_tokens` builds a
  `DartTypeArgs`: the name, the type's `check`, a brand's or an enum's `minLength`, `maxLength` and
  `pattern`, `default_types`, and `member_meta`, which is `member_prop_meta` with the enum's
  arguments for a member of an enum. `ModelSchemaArgs` stays private to `model_schema.rs`. Every
  member's `FieldDef` takes its meta through `field_def_with_prop_meta`, so an enum's string check
  reaches in Dart the fields `written_as_text` says it reaches.
- **One sentence source.** `dart_violation` in `src/bound_message.rs` writes the Dart string from
  `Bound::stated()`, as `rust_violation` and `zod_error_arg` do, and
  `the_three_renderings_of_a_bound_hold_one_sentence` holds the three to it. `dart_escaped` writes
  a text between two quotes of a Dart string, which reads `$` as an interpolation.
- **A member** is `member_elements`: its own bounds (`bound_elements`), then what the value it
  holds reports (`held_elements`), then its checks (`check_elements`), the own bounds and the
  held ones in the order `OWN_BOUND_FIRST` names. That is the order `named_member_validate_body`
  and `then_checks` give Rust.
  A type's own checks come last (`own_check_elements`).
- **A check is called under the name the attribute writes**, a function of the consumer's that
  answers its messages as a list: `List<String> withoutDuplicates(List<String> value)` for a
  field's, which takes the field's own value and names no place,
  `List<String> withinParcel(Parcel value)` for a type's, which is handed `this` and leads each
  message with a place, and `List<String> withACode<CodeType>(Crate<CodeType> value)` for a type
  with a parameter, written once. A missing function is the Dart compiler's to report.
- **A length** is `runes.length` on a string, its Unicode code points, and `length` on a list. A
  member typed with a named type is measured on `_boundText(...)`, and a `double` is quoted back
  through `_number`, a whole one without `.0`. `_under` writes a place in front of a message, as
  `nested_under` does in Rust. `validate_members` writes each helper into the type that calls it
  and into no other: two emitted types share nothing.
- **What a value holds** is walked over its `FieldDef`: an optional (`present`, an
  `if (x case final held?)`), each level of a list and of a set (`item_elements`, the index a
  step of the place), a map's values (the key's text from `dart_map_key_text`), a tuple's
  positions (`slot_elements`) and a declared type, whose `validate()` is called with no knowledge
  of what it declares. A type parameter is not reached. A binder is named by its depth
  (`binder`): `held`, `index` and `entry`, then `held1`.
- **A place is the steps serde writes on the way to the value** (`Step`): a key, a position in an
  array, or nothing. `tagged_variant_steps` answers a variant's: the content key, nothing beside a
  tag, and otherwise the name the variant is written under. A tuple struct that declares one slot,
  an untagged variant and a `#[serde(flatten)]` member add no step, and a slot of several is its
  position on the wire.
- **A brand** (`brand_validate`) runs the type's string check on `value` under no place, then its
  checks, and no `validate()` of what it wraps. Over a bare parameter it measures the declared
  default (`declared_default`) behind `if (value case final String held)`. An alias runs its
  target's `validate()` where the target is a declared type written as itself.
- **A sealed enum** declares `List<String> validate();` on its base (`VALIDATE_DECLARED`), and
  `variant_validate` writes each variant's override, the enum's checks last, each handed `this`.
  A plain enum runs its checks.
- **`validate` is a taken name** in `DART_FIELD_NAMES_TAKEN` and `DART_ENUM_NAMES_TAKEN`.
- **`toJson` is untouched.** `validate()` is the last member of each class, after its codec.
  `the_codec_of_each_type_is_the_text_it_was` in `tests/dart_tests/validate.rs` holds every piece
  of the codec's text, in its order: the two reads, and `toJson` as it was before any type had
  `validate()`.
- **The tests.** `tests/dart_tests/validate.rs` pins the text of each shape and calls no Rust
  `validate()`, so it holds in a build with no schema surface. `tests/dart_tests/bounded_brands.rs`
  pins a brand's string check, which such a build refuses.
  `tests/service_schema_emitted_client_tests/run_dart_validate.rs` runs the emitted source under
  the Dart SDK: a value under a named field, in a list, in a map and in an optional is compared
  with what Rust's `validate()` answers of it, a slot and a variant's member are written out, and
  the README's section is held to the text. `dart analyze` passes every shape whose codec it
  passed before. Each value is read with `fromJsonUnjudged`: `fromJson` refuses a broken one.

#### The Dart read refuses what `validate()` refuses

`fromJson` of every type `src/features/dart.rs` writes is its own unjudged read followed by its
`validate()`, and throws a `FormatException` whose message is the list joined by `; `, as the Rust
read joins it. `README.md`'s "The Dart read refuses what `validate()` refuses" section is the
consumer's view; this is what a change has to keep.

- **One writer of the pair.** `read_pair` writes `fromJson` over `fromJsonUnjudged` for a class, a
  wrapper (a brand, a tuple struct, an alias), a variant with named members and the base of a
  sealed enum. `judged_body` writes the judgement, which the static `fromJson` of a plain enum
  takes too. A generic type takes its converters in both reads.
- **A tuple struct that declares one slot is the value of that slot**, as serde writes it whatever
  the slot's attributes say: `tuple_struct_dart_tokens` hands `value_wrapper_tokens` the slot as
  `Held::Value`, the class a brand is written as, and the slot is read through `member_decode`,
  which calls the functions it names. So a slot over an `ObjectId` takes `$oid` alone flattened
  (`Taken::Only`). One of no slot or several is `Held::Slots`, the record of the slots serde
  writes, read from their array.
- **Every refusal the read writes itself is a `FormatException`**, thrown inline in the emitted
  text, with no declaration shared between two types. In `src/features/dart.rs`,
  `tagged_enum_dart_source` writes `Unknown {Enum}: {tag}` for a tag no variant has, under every
  tagging, `{Enum}: {tag} takes no value` for a unit variant's entry over a value, and
  `empty object for {Enum}` and `more than one key for {Enum}` for an externally tagged object
  of no key and of several, as serde and Zod refuse each; `tagged_variant_tokens` the same words
  for its content under a content key; `untagged_enum_dart_source` `No variant of {Enum}
  matched`; `plain_enum_dart_source` `Unknown {Enum}: {name}`; `range_decode` `Unknown key of a
  {kind}: {keys}`; `result_decode` `Not a Result: {keys}`; `tuple_decode`, the read of every
  tuple (a member's, a tuple struct's, a variant's slots), `invalid length {n}, expected {m}`
  for an array of another length, a longer one among them, as serde and Zod refuse it;
  `dart_decode_at` the same words for a level written as `[T; N]` (`FieldDef::fixed_length_at`),
  at each level, before it maps the items, where a level of no recorded length is read as it
  was; `own_tag_held` a struct's own tag; and `named_by_aliases` a member under two of its names. A
  cast's `TypeError`, and every other error a Dart operation throws on its own, are the
  language's. So an enum declared below its holder, handed the object without the holder's own
  keys, is refused where that object holds another key beside its entry, which serde reads.
  `run_dart_refusals.rs` reads each refusal under the Dart SDK, beside serde's refusal of the same
  payload.
- **A number, a `char` and a chrono date, time and date and time are read and written in their
  one form** (section 14), by `src/features/dart/forms.rs`, each an inline expression, so two
  types share nothing. `text_read` reads a `char` and a date or time held as a `String` wherever
  `dart_decode_at` reads one, and a map's key: a `switch` that answers the text where it
  matches `TextForm::read`, or is one code point for a `char`, and throws `FormatException('expected
  {form}, found ...')` otherwise, the text quoted by `jsonEncode`. `text_sent`, wherever `toJson`
  writes one, a map's key among them, drops the `Z` (`TextForm::unzoned`), writes the seconds
  (`TextForm::unsecond`) and throws in the same words for a text not of `TextForm::sent`. A map's
  key held as a `String` is a `String` already, read off the object and written by `toJson`, so
  `dart_map_key_decode` and `dart_map_key_encode` cast none, a cast `dart analyze` reports as
  unnecessary: a `String` key is the key itself, a `char` and a date or time key `text_read` and
  `text_sent` of it. A key of a declared type and an unknown one keep the cast of what `toJson`
  answers. `run_dart_text_keys.rs` reads and writes back a `String` key held itself, under an
  `Option`, in a list and as a map's value, against serde, and analyzes the type, and
  `run_dart_text_forms.rs` analyzes the types keyed by a `char` and a date or time. `number_held`
  holds an integer to `FieldDefType::integer_range` and an `f32` below `F32_READ_BOUND` in size,
  on the read and on the write, a map's key included: `expected an integer from 0 to 255, found
  256`, `expected a 32-bit float, found 1e+300`. A `DateTime<Tz>` is written
  `toUtc().toIso8601String()`, as a value and as a map's key. The WebSocket client writes and reads
  a header through the same codec. `run_dart_text_forms.rs` reads `TEXT_FORMS` under the Dart SDK,
  and `run_dart_wire_forms.rs` writes `WIRE_FORMS`.
- **A declared type is read unjudged exactly where the holder's `validate()` runs it.** `Read` is
  threaded through `dart_decode_expr` and `dart_decode_at`, and `member_elements` walks every
  member's type, so a member's own bound is held beside what its type declares. A value read unjudged where no `validate()`
  walks is a rule no read runs, so each of these is read judged and ends the read with its own
  messages: a converter handed for a type argument, the key of a map (`dart_map_key_decode`),
  what a brand wraps, and an alias of anything but a declared type itself. A flattened member is read unjudged whatever it writes, as `field_elements`
  walks it.
- **`dart_json_decode` stays judged**, so the clients' text is what it was: a reply is read
  through `fromJson` inside the `try` each client already writes. The HTTP client answers
  `undeserializablePayload` and the WebSocket client `failedValidation`, the fault each answers
  for a reply it cannot read.
- **A tagged enum** dispatches in the base's unjudged read: to `fromJsonUnjudged` of a variant
  with named members, and to the constructor of a variant that holds a value or none, its
  payload read as a member is.
- **An untagged enum takes the variant serde takes.** `variant_bounds` writes `_bounds()` ahead
  of a variant's `validate()`, and the attempt takes the variant where it answers nothing.
  `Asked` is what one walk is asked: `Rules` for `validate()`, and `Bound` for the member's own
  bounds under no place, then `_heldBound(...)` of each declared type the walk reaches, with no
  check. `_heldBound` calls `validateBound()` through `dynamic` and answers nothing for a value
  with no such method. `brand_validate` writes that method on a brand that carries a bound: the
  bound alone. So a brand's bound chooses a variant whether the brand is declared above the
  enum, below it or in another file, and nothing a check or a model reports does. A variant
  with nothing to ask keeps `try { return ...; } catch (_) {}`.
- **A plain enum** keeps its `fromJson` as it was where it names no check, and judges where it
  names one. Both carry `fromJsonUnjudged`, the name a holder reads every declared type by.
- **`validate()` stays the last member of a class**: `validateBound()` and `_bounds()` are
  written ahead of it.
- **A member serde never writes is read and has no entry in `toJson`.** `DartField::never_written`
  is `has_serde_skip_serializing`: `skip_serializing`, never `skip_serializing_if`.
  `class_body_parts` writes no entry for it, a flattened one spreading nothing, a variant with
  named members sharing it. `collect_dart_fields` pushes no nullable level on a flattened one,
  which has no key to leave out, so one that is no `Option` is read as any flattened member is
  and its constructor parameter is `required`. A member under `skip_serializing_if` is written
  wherever it holds a value: no client calls the Rust function, and the README states the limit.
- **Every flattened member is read at its place, in the order declared**, as serde reads it
  (`constructor_arguments`): from the object without the tag, the keys the holder's own members
  take and the entries each member flattened ahead of it took. That is what a function a member
  names is handed, and a struct that hands its object to a function: `DART_HANDED` notes, per
  type the emitter wrote, whether it does, its own `preprocess` or a member it flattens that is
  handed one, and `hands_to_none` reads it. Every other struct, and an enum under a tag key, the
  emitter wrote above is read from `json`, the whole object, as TypeScript and Zod read it
  (`reads_the_whole`): a key a member of its holder's own, a struct flattened ahead of it or the
  tag of the variant that flattens it reads too is read by both, where serde refuses the payload.
  `tests/swift_tests/shared_key_models.rs` holds `SHARED_KEYS`, and `run_dart_flattened.rs`
  reads each as Zod reads it. A map is the one member of its
  object (section 4), and is handed every key but the tag of a variant under a tag key. An
  optional map that names no function takes no key, and is read in the closure below, so it is
  `null` wherever an entry does not read as its key and value, as serde's untagged read of an
  `Option` reads `None`; one that names a function is read from what the function answers, where
  an entry that does not read refuses the payload, as the Rust read refuses it. Any other optional
  member that names a function is read by `corrected_option`, as the holder's Zod schema reads
  it: the type from what the functions answer, held under the name `declared("corrected")`
  answers, inside a closure called in place that answers `null` where that read throws, an answer
  of `null` among it, and the object the functions are handed holds none of the struct's keys
  (`struct_keys`, the `Taken::Keys` recorded for its type whatever function the member names), or
  no key at all for a type whose keys are not known, an enum among them, and rethrows where it
  holds one. serde refuses an answer its type does not read. `run_dart_flattened_options.rs`
  holds each row of `CORRECTED`, in `tests/kotlin_tests/corrected_options_models.rs`, to what Zod
  reads. What a member takes is `taken_by`, the `Taken` recorded for its type in `DART_TAKEN_KEYS`, and nothing for a
  member that names a function:
  - `Taken::Keys` of a struct, which `struct_dart_tokens` records from
    `keys_taken_when_flattened` in `src/features/serde.rs`, which the Swift and Kotlin emitters
    read too: serde's own list of the name and each alias of every member it reads. `without`
    removes them. An optional one that names no function is read by `flattened_decode`: where
    `taken_by` answers keys, `null` where the object it is handed holds none of them, and its
    type's read where one is there, which refuses a key that does not read, as TypeScript and Zod
    refuse it, where serde reads `None`; where neither it nor `DART_TAG_KEYS` (below) answers
    keys, a map among them, its type's read inside a closure called in place, `(() { try { return ...; } catch (_) { return null; }
    })()`, which declares no name and answers `null` wherever the read throws, as serde reads
    `None`.
  - `Taken::FirstOf` of an externally tagged enum, which `enum_dart_tokens` records from
    `variant_names_when_flattened` in `src/features/serde.rs`: the names each variant serde reads
    is read under, its own and each alias, one list per variant, serde's `VARIANTS`. serde's
    `FlatMapDeserializer::deserialize_enum` takes the first entry, in the object's order, under one
    of them, wherever it is. `first_of` hands the enum an object of that one entry, and of none
    where no entry is, and throws `FormatException('more than one variant of enum {Enum} found in
    flattened data: ' + ...)` where entries under two variants' names, or under a variant's name
    and its alias, are there, naming each variant once, in the order the enum declares them, as
    TypeScript and Zod refuse the payload, where serde reads the first. `without_first` removes the
    entry from the object a `jsonDecode` answers in
    the order written, for the members after the enum: each a `switch` of one arm whose variable,
    `whole` or `rest`, calls no function. The entry depends on what is left to the enum, so the
    keys taken ahead of it are removed first and the keys taken after it from what
    `without_first` answers (`source`, `removed`); a holder with no flattened enum is written as
    it was. An optional enum is read by `first_entry_decode`: `null` where the object `first_of`
    answers is empty, and the enum's read of the entry where it holds one, which refuses an entry
    that does not read, as TypeScript and Zod refuse it, where serde reads `None`. Its variable,
    `entry`, calls no function either.
  - `Taken::Only` of an `ObjectId` (`object_id_keys`), and of a brand or an alias over one that
    `value_wrapper_tokens` records, `$oid`: the README's `ObjectId` refuses
    any other key, where serde reads past one flattened, so `only` hands it an object of that key
    alone, and `without` removes it from what the members after it are handed. An optional one
    is looked for by the key, as a struct is. A holder hands such a value the same way through
    the converter of a type parameter the generic type reads flattened (`argument_converters`).
    `record_flattenings` records, per type, how it reads each parameter (`DART_FLATTENINGS`,
    `parameter_reads`, a `Flattening`), the parameter itself flattened or an argument handed to a
    type that reads it flattened. Read flattened alone (`Flattening::Alone`), the converter is
    filtered. Read flattened and otherwise (`Flattening::Also`), the type's two reads take a
    second converter, named optional (`{Held Function(dynamic)? heldFromFlattenedJson}`,
    `generic_codec`), which its flattened reads call where it is handed and the converter of
    the parameter otherwise (`parameter_read`, through `DART_FLATTENED_READ`, which
    `flattened_decode` sets while it reads such a member), and the holder hands the filtered
    converter there, and a parameter of its own its own converter of that kind. Every other
    filling is handed what it was.
  - `Taken::Tagged` of an enum under a tag and a content key, which serde reads as a struct of
    the two, both of which it takes: `tag_and_content_when_flattened` in `src/features/serde.rs`,
    which the Swift and Kotlin emitters read too, answers them, none for an enum that names a
    function on itself. `without` removes them.
  - Nothing, for an enum under a tag alone, an untagged one and one that names a function on
    itself. serde reads an enum under a tag alone and an untagged one as a map, which takes no
    key. An optional enum under a tag key is still looked for by
    keys: `enum_dart_tokens` records its tag, then its content key where it has one, in
    `DART_TAG_KEYS` (`tag_keys`, `record_tag_keys`), none for an untagged enum and one that
    names a function on itself, and `flattened_decode` tests it by them (`looked_for_by`) with
    the test a struct takes, `(handed.keys.any(const ['kind', 'body'].contains) ? ... : null)`.
    So a tag naming no variant, a member or a content that does not read, and a content with no
    tag refuse the payload, as TypeScript, Zod and the Swift and Kotlin clients refuse it, where
    serde reads `None`, and a variant's member with no tag reads `null`, as serde reads it. An
    enum under a tag alone takes no key: a member flattened after it is handed what it was. An
    untagged
    enum, one declared below its holder and one that names a function on itself have no key to
    be looked for by, and are read in the closure.
  - A type parameter: an optional one is looked for by the keys of what fills it, which the type
    that fills it hands. `record_flattenings` notes in `DART_LOOKED_UP` each parameter a type
    reads flattened under an `Option`, itself or through a type it hands the parameter to, with
    the name its two reads take the keys under, named and optional (`{List<String>? heldKeys}`,
    `generic_codec`; `DART_LOOKING` for the item being emitted). `argument_converters` hands the
    keys `argument_keys` answers: those an `Option` of the argument flattened in its place is
    looked for by, or the holder's own for one of its parameters. `flattened_read` writes
    `(heldKeys == null ? <the closure> : (handed.keys.any(heldKeys.contains) ? ... : null))`.

  A struct that names a `preprocess` on itself is read by serde as a value, and one with a
  flattened member of its own as a map: each records none. A type the emitter has not written
  takes none, so a member after a struct declared below its holder is handed the struct's keys,
  and an optional one is read in the closure, and the README states both. `toJson` writes each
  flattened member at its place: a key two of them hold is written once, with the last value,
  where serde writes it twice. `run_dart_flattened_options.rs` pins the read of a flattened
  `Option` of a struct and of an enum of each tagging and runs it under the Dart SDK, over the
  models of `tests/swift_tests/flattened_options_models.rs`, which the `dart_tests` binary cannot
  declare beside the `Veering` of `kotlin_tests/aliased_models.rs`, and of `Cringle`, an enum
  under a tag that names a function on itself, which it declares. `src/features/dart/tests.rs`
  pins the read of an enum under each tag shape whose class was emitted above.
- **No cast is written of a value Dart types `Map<String, dynamic>` already**, a cast `dart
  analyze` reports as unnecessary. `joined_entries` spreads what a flattened member, and the lone
  slot of a variant under a tag alone, writes as it is where `writes_an_object` answers: a map, the
  README's `ObjectId`, and a struct `DART_WRITES_AN_OBJECT` notes, a class whose `toJson` is
  `Map<String, dynamic>`. An optional one is cast to the map it holds. A brand, an alias, a tuple struct and an enum answer `dynamic` from
  `toJson` and are spread through `as Map<String, dynamic>`, and so is a type the emitter has not
  written: a struct declared below its holder keeps the cast, which the analyzer reports. A
  flattened map is read off the object it is handed (`map_decode`), which the read types
  `Map<String, dynamic>`, and a map under a key off `json['key'] as Map<String, dynamic>`.
  `run_dart_validate.rs`, `run_dart_ranges.rs`, `run_dart_results.rs`, `run_dart_object_ids.rs`,
  `run_dart_quoted.rs`, `run_dart_own_tags.rs`, `run_dart_flattened_options.rs` and
  `run_dart_text_forms.rs` analyze every type they read, a flattened struct, `ObjectId` and map and
  a slot beside a tag among them.
- **A unit variant of an externally tagged enum is an entry of its name over `null`** where the
  enum is flattened, as serde writes and reads it. The enum's `toJson` writes the bare name, so
  `tagged_enum_dart_source` records every externally tagged enum with a unit variant in
  `DART_WRITES_A_NAME`, and `class_body_parts` spreads a member `writes_a_name` answers for
  through a `switch` that writes a `String` as `{name: null}` and a map as it was
  (`joined_entries`), the member optional or under a function too, and so does the lone slot of a
  newtype variant under a tag alone, which serde writes beside the tag as it writes it flattened.
  `value_wrapper_tokens` records an alias, a brand and a struct of one slot over such an enum as
  well. A holder of any other flattened
  member, and of an enum declared below it, writes the spread it wrote. The enum's map branch
  reads a unit variant from its entry, under each of its names, where the value is `null`, and
  refuses any other value: the Zod schema takes the entry over `null` alone, where serde's
  flattened read takes an empty object too. So does the enum read alone, as serde reads it.
- **An enum under a tag and a content key reads a unit variant from an object with no content
  key**, as Zod's strict object of the tag alone reads it (`tagged_variant_tokens`): the arm reads
  `json.containsKey('{content}')` and throws `FormatException('{Enum}: ' + tag + ' takes no
  value')` where it holds, a content of `null` among it, which serde reads. serde writes the
  variant as its tag alone. `run_dart_unit_entries.rs` reads `Becalming`, of
  `run_kotlin_refusals.rs`, under the toolchain, with serde's answer and the Zod schema's text
  pinned beside each payload.
- **A struct under its own tag reads its name under the tag alone**, as TypeScript and Zod read
  it (`kind: z.literal("Flag")`), where serde reads past the tag whatever it holds, and where it
  is absent. `own_tag_held` in `src/features/dart.rs` writes the check, inline, ahead of the
  constructor: ``FormatException('missing field `{key}`')`` for an object without the key, and
  `FormatException('expected tag "{name}", found ' + jsonEncode(...))` for another value under it,
  `null` among it. `tag_checked_reads`, the struct's `class_reads`, writes it over the object the
  members are read from: what the type's functions answer where it names one, and `named` where a
  member has an alias. A holder that flattens such a struct hands it the tag, so the holder
  refuses the same payloads. `toJson` writes the tag ahead of the members either way, as serde
  writes it. `run_dart_own_tags.rs` reads `FLAG_TAGS`, `PENNANT_TAGS` and `DRESSING_TAGS` of
  `tests/kotlin_tests/flattened_models.rs` under the Dart SDK, with serde's write-back pinned
  beside each row, and pins the text of each read; `src/features/dart/tests.rs` pins the check
  under a function and an alias.
- **A name serde writes is written escaped wherever it sits between quotes.** `quoted` writes it
  as a Dart string literal through `dart_escaped`, a `$`, a `'` and a `\` escaped: a member's wire
  name in the read and in `toJson`, an alias, a struct's own tag key and name, a variant's name,
  an enum's tag key and content key, a plain enum's value, and every list of names a flattened
  member is read by (`listed_names`, `without`). A name with no such character is written as it
  was. `tests/dart_tests/quoted.rs` pins the text over the models of
  `tests/dart_tests/quoted_models.rs`, and `run_dart_quoted.rs` reads and writes back each
  payload under the Dart SDK, against serde.
- **A name serde reads beside the one it writes is read too.** `read_aliases` reads the
  `#[serde(alias)]` names of a field and of a variant, none for one serde never reads, and
  `DartField::aliases` holds a member's. A tagged enum takes a variant under its written name or
  any alias (`any_of`, `case 'Berth' || 'Pier':`), under every tagging, and an externally tagged
  variant read under an alias too reads its value from the entry it took, `map[tag]`
  (`VariantClass::aliased`); a plain enum's `switch` takes each alias the same way. Where a member
  of a class has an alias, `class_reads` reads the members from `named`, the object copied with
  each alias moved to the member's wire name in its place (`named_by_aliases`), and refuses a
  member under two of its names with `FormatException('duplicate field ...')`, as serde refuses
  it: a member flattened after them is handed `named` without their keys. A type that names no
  alias is written as it was, and `toJson` writes the written name.
- **`fromJsonUnjudged` is a taken name of a plain enum** (`DART_ENUM_NAMES_TAKEN`), which holds
  it as a static function. It is not in `DART_FIELD_NAMES_TAKEN`: Dart takes a field and a
  constructor under one name, so such a field is emitted as it was.
- **The tests.** `tests/dart_tests/read.rs` pins the text of each read, and
  `src/features/dart/tests.rs` that of a field named `fromJsonUnjudged`, which no integration
  test can declare, the name being no snake case.
  `tests/service_schema_emitted_client_tests/run_dart_read.rs` runs the reads under the Dart
  SDK: the sample beside what the Rust read answers, every case of `run_dart_validate.rs` read
  with `fromJson` and held to what its `validate()` answers, an untagged enum beside the variant
  serde takes, a reply that breaks a rule through both clients, and the README's section.
  `tests/dart_tests/flattened.rs` pins the read of a flattened member at each place, over the
  models of `tests/dart_tests/flattened_models.rs`, which also pin what serde reads of each, and
  `run_dart_flattened.rs` reads and writes each back under the Dart SDK, against serde. A map of
  models with a rule (`Limber`) is held to serde under a schema surface alone: a build with none
  holds no bound in Rust, and the Dart read refuses what it breaks in every build.
  `tests/dart_tests/aliased.rs` pins the read of a variant and a member under an alias, over the
  models of `tests/kotlin_tests/aliased_models.rs` and of
  `tests/dart_tests/flattened_aliased_models.rs`, which flattens each tagged enum of the first
  beside a member and pins what serde reads of it, and `run_dart_aliased.rs` reads and writes
  each payload back under the Dart SDK, against serde. `run_dart_unit_entries.rs` reads and
  writes back each variant of `Tacking`, of `tests/kotlin_tests/flattened_models.rs`, flattened
  in `Stowing`, `Belting` and `Yawing`, in either order, against serde: those models are compiled
  with `kotlin`, so the run is too.

#### A `preprocess` in the Dart read

`fromJsonUnjudged` of every type `src/features/dart.rs` writes calls the functions the type and
its members name, under the names the attribute writes, each a function of the consumer's own:
`dynamic trimmed(dynamic payload)`. `fromJson` calls that read, so both correct the payload, and
`validate()` judges what the functions answered. The order of one read is Rust's: the type's
functions, each member's as it is read, the value, then `validate()`. `README.md`'s "A
`preprocess` in the Dart client" section is the consumer's view, which calls the key a last
resort; this is what a change has to keep.

- **The dispatch is handed the type's functions** in `DartTypeArgs::preprocess`, and a member's
  are in the meta `member_meta` answers. A type that names none, on itself or on a member, is
  written as it was, a holder of a flattened member apart (below).
- **A type's functions are called by `unjudged_read`**, the one writer of the body of every
  unjudged read: a class, a wrapper (a brand, a tuple struct), the base of a sealed enum and a
  plain enum. Where the type names functions the read takes its payload as `payload` and holds
  what they answer as `json`, cast as the read types it (`final json = withoutFragile(payload)
  as Map<String, dynamic>;`), and a read of no member holds it under no name. `read_pair` writes
  `fromJson` as it was: it calls no function. A sealed enum calls them in its base alone
  (`Corrects::of_a_variant`), and a plain enum that names one and no check delegates `fromJson`
  to `fromJsonUnjudged`.
- **A member's functions are called by `member_decode`**, around the payload its own read is
  written over: `constructor_arguments` for a named member, the tuple arm of `dart_decode_at` for
  a slot, and the arm of a variant that holds one value. `corrected` folds the calls, the first
  name innermost, so it is the first one called.
- **No function is called for a key the payload does not hold.** A named member and a slot
  under a content key are read from `json.containsKey('key') ? f(json['key']) : null`, so an
  absent key is read as it was, and a `null` the payload holds is handed over. A slot of a tuple
  and the value under a variant's name are held by every payload that is read.
- **One read calls each function once.** A read that names its payload more than once, an
  optional, holds the corrected payload under `corrected`, the variable of a `switch` of one arm;
  any other read is written around the call itself, a tuple's among them, which binds its array
  once.
- **A flattened member is read from the keys serde hands it** (`without`, `without_first`,
  `first_of`): the object without the keys of the members beside it and the entries the members
  flattened ahead of it took, and without the tag where the holder is a variant under a tag key
  (`constructor_arguments`' `tag_key`), and an externally tagged enum the one entry of it under
  a variant's name. A function the member names, and one the type it holds names, is handed
  those keys. A holder with no member of its own hands its first flattened member `json` as it
  was, an externally tagged enum apart. A struct's own
  tag is not taken out: serde hands it on too. The slot of a variant under a
  tag alone, and the function it names, are handed the object without the tag, as serde hands
  them, so an externally tagged enum there reads its one entry (`run_dart_tagged_slots.rs`).
- **A flattened `Option` of any type but a map that names a function is read as the Zod schema
  of its holder reads it** (`corrected_option`), where serde parts from Zod: where the type does
  not read what the functions answer, `null` among it, the member is `null` where the object they
  are handed holds none of its struct's keys, or no key at all for a type whose keys are not
  known, an enum among them, and the read's error is thrown again where it holds one.
- **Every name a read declares is its own.** `dart_schema_dispatch` records the name of each
  function the item's reads call (`called_by`, `DART_CALLED`), and `declared` numbers a name
  where one is called under it (`json2`): `json`, `payload`, `map`, `tag` and `read`, which
  `Corrects` holds, `corrected` where a flattened `Option` holds what its functions answer
  (`corrected_option`), `range`, `result`, `tuple` and `array`, the value a range, a `Result`, a
  tuple and a fixed array are read from (`range_decode`, `result_decode`, `tuple_decode`,
  `dart_decode_at`), and the converter of a type parameter (`from_json_argument`), in the
  signature of both reads and where a value behind the parameter is read. A member of the type
  still hides a function of its name, as it hides a check's, and the documents say so.
- **The payload a type's function takes is typed as its read types it**: a
  `Map<String, dynamic>` for a class and an enum under a tag key, a `String` for a plain enum,
  `dynamic` for a wrapper and any other enum. A payload of another kind is refused ahead of the
  function, where Rust hands it over.
- **The tests.** `tests/dart_tests/preprocess.rs` pins the text of each shape.
  `tests/service_schema_emitted_client_tests/run_dart_preprocess.rs` compiles the models of
  `tests/pattern_preprocess_tests/models.rs` and runs them under the Dart SDK: the sample and the
  shared table of positions print what the Rust read prints, two rows apart (`NOT_READ`), with
  the keys a flattened type is handed, the calls of one read counted, a function named after a
  name of the read, a reply through both clients, and the README's section. Its module is
  declared last in the binary: its models take names the modules above it declare, and an
  expansion reads a type by its name.
