# The Dart Client

The draft and the valid type, `validationIssues()`, the read and `preprocess` on the Dart types.

#### The draft and the valid type

A struct, a tuple struct, a unit struct and every enum, a plain enum included, are two classes
(`struct_dart_tokens`, `value_wrapper_tokens` and `enum_dart_tokens` write the pair). The class the
emitter wrote before is the draft, named `{Name}Draft` (`draft_name`): its constructor, its fields,
its one read, `toJson` and `validationIssues()` are the class as it was, and `validate_method` adds
`{Name} validate()`, which answers `{Name}._(this)` where `validationIssues()` is empty and throws a
`FormatException` of the issues joined by `; ` otherwise. `valid_class` writes `{Name}`: the only
constructor `const {Name}._(this._draft)`, a getter for each member of the draft
(`forwarding_getters`; `value` for a tuple struct, none for an enum), `toDraft()`, `fromJson`, which
is `judged_body_returning` over the draft's read, `fromJsonUnjudged`, which is `{Name}._` over it
and stays public for a holder in another library, `toJson`, which forwards the converters of a type
with a parameter (`GenericCodec::to_json_args`), and `validationIssues()`.

- **Every message stays as it was.** A class names the type in a message as the type is named
  (`ClassNames::message`), not as the draft is: `Unknown key of a Parcel: stray`. Only a name the
  consumer writes carries `Draft`: a constructor, a check function's argument, the type a Dart
  error prints. The `ReadForm::Draft` read of a draft is `draft_read`, one `fromJson`; `ReadForm::Whole`
  is `read_pair`, the pair a type without a draft keeps.
- **A holder calls `validationIssues()` on everything it holds.** `held_at` writes
  `expr.validationIssues()` for a model, a brand, an alias and a plain enum alike, so a type
  declared below its holder, or in another crate, is held without being known. A brand and an alias
  have no draft and list their messages under that name. `validate()` is the
  draft's alone.
- **A holder reads a held model through `{Name}.fromJsonUnjudged`**, public on the valid type, which
  builds the valid type with no check; the holder judges the whole value once, so each answer is
  byte-identical to the answer before the pair.
- **An enum's draft is the sealed base, and a variant keeps its subclass name.** `enum_base`,
  `enum_reads` and `enum_classes` write `sealed class {Name}Draft`, then `{Name}`, then each variant
  `{Name}{Variant} extends {Name}Draft`, whose read is `fromJson` (`VariantClass::form`). The base
  declares `validationIssues()` (`VALIDATE_DECLARED`), each variant overrides it, and `validate()` is
  written once on the base. A variant is a draft: it does not fill a member typed `{Name}`.
- **A check function takes the draft.** `validationIssues()` calls `withinParcel(this)` on the
  draft, so `List<String> withinParcel(ParcelDraft value)` and, for a type with a parameter,
  `withACode<CodeType>(CrateDraft<CodeType> value)`.
- **A type a service writes for itself has no draft** (`DartTypeArgs::undrafted`, from the
  `__tixschema_undrafted` argument): the pair's one-class form, so a client builds its requests and
  its results with the constructors it had, with `validationIssues()` in place of `validate()`.
- **`toDraft`, `validate` and `validationIssues` are taken names** in `DART_FIELD_NAMES_TAKEN`, so a
  member of that name is `toDraft_`, `validate_` or `validationIssues_`; `validationIssues` is also
  taken in a plain enum (`DART_ENUM_NAMES_TAKEN`).
- **The tests.** `tests/service_schema_emitted_client_tests/run_dart_valid_types.rs` analyzes a
  second library that writes `Cargo(...)`, `const Cargo(...)` and `Cargo._(draft)`, and runs one
  that builds through the draft: the getters, `toDraft()`, `toJson()`, a `validate()` whose message
  is the Rust `validation_issues()` joined by `; `, and a holder in another library that reads
  through `fromJsonUnjudged`.

#### `validationIssues()` on the Dart types

Every type `src/features/dart.rs` writes has `List<String> validationIssues()`, which runs what the
Rust `validation_issues()` of the type runs, in its order. `docs/guide/dart.md`'s "`validate()` in
Dart" section is the consumer's view; this is what a change has to keep.

- **A class refuses a key no member reads**, as TypeScript, Zod and serde refuse it
  (`closed_keys`, `unknown_keys_refused`): `tag_checked_reads` opens the unjudged read of a struct
  and of a variant's class with a loop that throws `Unknown key of a {Class}: {key}`, the key its
  `source`, for a key of the object that is none of its members' names and aliases, its own tag, the tag of the enum it is
  a variant of and the keys of each member it flattens. A `nonstrict` type, a type that names a
  `preprocess`, which can rewrite the keys it reads, a struct under its own tag, which serde reads
  past a key beside, and a type that flattens a member whose keys are not known, such as a map or
  a type parameter, refuse nothing. A member whose type is declared below the holder or in another
  crate is listed by a placeholder (`member_keys::of`), which `dart_definition()` replaces with
  the keys the type publishes in `__TIXSCHEMA_KEYS` (`member_keys::filled`); the `const` beside a
  closed holder refuses a type no `#[model_schema]` expanded, so every such type publishes them. The unit variant of a strict enum under a tag alone refuses
  every key but the tag (`unit_beside_the_tag`), as the Rust read the macro writes for it does. The keys a type refuses a
  key beside are recorded per type (`DART_CLOSED`, `record_closed_keys`), for a struct and for an
  enum under a tag alone or untagged, which takes the union of its variants'. A holder hands each
  flattened member `handed`, the object without its own keys and those the members ahead of it
  took, as serde hands it, and a member that takes its own keys the entries of those keys alone
  of it (`only`, in `constructor_arguments`). An enum under a tag and a content key refuses a key
  beside the two (`beside_the_content`).
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
- **A member** is `member_elements`: what the value it holds reports (`held_elements`), then
  its own bounds (`bound_elements`), then its checks (`check_elements`). That is the order
  `named_member_validate_body` and `then_checks` give Rust.
  A type's own checks come last (`own_check_elements`).
- **A check is called under the name the attribute writes**, a function of the consumer's that
  answers its messages as a list: `List<String> withoutDuplicates(List<String> value)` for a
  field's, which takes the field's own value and names no place,
  `List<String> withinParcel(ParcelDraft value)` for a type's, which is handed `this`, the draft, and
  leads each message with a place, and `List<String> withACode<CodeType>(CrateDraft<CodeType> value)`
  for a type with a parameter, written once. A missing function is the Dart compiler's to report.
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
  checks, and no `validationIssues()` of what it wraps. Over a bare parameter it measures the declared
  default (`declared_default`) behind `if (value case final String held)`. An alias runs its
  target's `validationIssues()` where the target is a declared type written as itself.
- **A sealed enum** declares `List<String> validationIssues();` on its base (`VALIDATE_DECLARED`), and
  `variant_validate` writes each variant's override, the enum's checks last, each handed `this`.
  A plain enum draft runs its checks, each handed `this`, the draft.
- **`toJson` is untouched.** `validationIssues()` is the last member of each draft class but its
  helpers and `validate()`, after its codec. `the_codec_of_each_type_is_the_text_it_was` in
  `tests/dart_tests/validate.rs` holds every piece of the emitted text but each
  `validationIssues()`, in its order: the read, `toJson` and the valid type.
- **The tests.** `tests/dart_tests/validate.rs` pins the text of each shape and calls no Rust
  `validate()`. `tests/dart_tests/bounded_brands.rs` pins a brand's string check.
  `tests/service_schema_emitted_client_tests/run_dart_validate.rs` runs the emitted source under
  the Dart SDK: a value under a named field, in a list, in a map and in an optional is compared
  with what Rust's `validate()` answers of it, a slot and a variant's member are written out, and
  the guide's section is held to the text. `dart analyze` passes every shape whose codec it
  passed before. Each value is read with `fromJsonUnjudged`: `fromJson` refuses a broken one.

#### The Dart read refuses what `validate()` refuses

`fromJson` of every type `src/features/dart.rs` writes is its own unjudged read followed by its
`validationIssues()`, and throws a `FormatException` whose message is the list joined by `; `, as the Rust
read joins it. `docs/guide/dart.md`'s "The Dart read refuses what `validate()` refuses" section is the
consumer's view; this is what a change has to keep.

- **One writer of the read.** A draft holds one read, `draft_read`: `fromJson` over the body
  `unjudged_read` writes, with no judgement. `valid_class` writes the valid type's `fromJson` over
  it, and `fromJsonUnjudged`. `read_pair` writes `fromJson` over `fromJsonUnjudged` for what has no
  draft: a brand, an alias and a type a service writes. `judged_body` writes the judgement, which the
  static `fromJson` of a plain enum draft takes too. A generic type takes its converters in each read.
- **A brand is sealed.** `struct_dart_tokens` hands `value_wrapper_tokens` `Held::Brand` for a
  brand alone: the class writes `const X._(this.value)`, the only constructor, and `factory X.of(value)`
  (`judged_body` over `X._(value)`, so it throws the `FormatException` `fromJson` throws, the messages
  of `validationIssues()` joined by `; `). `fromJsonUnjudged` stays public under that name and builds through
  `X._`, as Rust keeps `model_schema_read_unjudged` public: a holder in another file, which is another
  library, or in another crate calls it, so a holder reads a brand as it reads any type. A name
  starting with an underscore is private to the library, so a second library builds a brand through
  `of` or a read alone. A tuple struct and an alias keep the public constructor.
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
  ``expected value`` for an externally tagged enum written as neither a string nor an object of
  one key, serde's words for text that is not the document; `tagged_variant_tokens` the same words
  for its content under a content key; `untagged_enum_dart_source` `No variant of {Enum}
  matched`; `plain_enum_dart_source` `Unknown {Enum}: {name}`; `range_decode` `Unknown key of a
  {kind}: {keys}`; `result_decode` ``expected value`` for a value that is neither a string nor an object of one key, serde's words for text that is not the document, and `Not a Result: {keys}` for a key no arm names; `tuple_decode`, the read of every
  tuple (a member's, a tuple struct's, a variant's slots), `invalid length {n}, expected {m}`
  for an array of another length, a longer one among them, as serde and Zod refuse it;
  `dart_decode_at` the same words for a level written as `[T; N]` (`FieldDef::fixed_length_at`),
  at each level, before it maps the items, where a level of no recorded length is read as it
  was; `own_tag_held` a struct's own tag; and `named_by_aliases` a member under two of its names. A
  cast's `TypeError` below the payload is refused with a `FormatException` of its words (`_readAt`
  below), and every other error a Dart operation throws on its own is the language's.
  `run_dart_refusals.rs` reads each refusal under the Dart SDK, beside serde's refusal of the same
  payload.
- **Each value read under a step is read through `_readAt`** (`READ_AT_BODY`): a member under
  its key (`constructor_arguments`), an item under its index and a map's value under its key
  (`dart_decode_at`, `map_decode`), a variant's content under its entry or the content key, the
  tag under its key, a `Result`'s arm under its key (`result_decode`), and a tuple's position. A
  brand that keys a map is built and judged under the key's text, its value read from the text
  with no step of its own (`judged_key`, `brand_leaf`): a key a brand's `check` refuses is named
  under its map, `pitches.13`, a key that does not read at the map, `pitches`, and a value an arm
  refuses under the arm, `tally.Ok`, as the Rust dispatcher names each. A brand over a brand is
  judged by both checks under the key's text, as its `fromJson` reads the inner brand through the
  inner brand's own judged `fromJson`: `outer.mute` and `outer.soft` in `run_carillon.rs`. Each class whose read calls it declares it
  (`reading_at`, `with_read_at`); a text with no helper, a service's, names its own
  (`read_at_function`), and the read writes no step where none is declared. A refusal carries the
  path of the value refused as its `FormatException`'s `source`, a list of steps the helper puts
  its own ahead of, and keeps the `message`. A read that fails where the object does not hold the
  key is refused with ``missing field `{key}` ``, as serde words it; a cast's `TypeError` becomes a
  `FormatException` of its words, and a Dart parser's refusal one with the path. A flattened
  member is read through `_readFlattened` (`READ_FLATTENED`, which a class with one declares),
  which drops the path of its refusal but for the key a `missing field` or an unknown key names,
  so the holder's step is the place, as serde names the holder. The `ws_rpc` attachment and
  client name the joined path as the fault's `field`, or the place a `validate()` message opens
  with (`refused_at_fn` in `dart_ws_client.rs`).
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
  holds an integer to `FieldDefType::integer_range` on the read and on the write, and an `f32`
  below `F32_READ_BOUND` in size on a map's key: `expected an integer from 0 to 255, found 256`,
  `expected a 32-bit float, found 1e+300`. `float_sent` holds an `f32` value to the same bound on
  the write, but for one that is not finite, which `toJson` writes as the `double` it is, as it
  writes an `f64`: serde writes it as `null`, `jsonEncode` refuses it, and the `ws_rpc`
  attachment writes it as `null` (`_{service}WsWritten`), as the `mcp` server writes it in all it
  answers (`_{service}McpJson`, whose `jsonEncode` writes such a `double` through
  `_{service}McpEncodable`). `float_read` reads an `f32` or an
  `f64` value as serde does: an `f32` beyond `F32_READ_BOUND` as an infinity of its sign, and the
  infinity `jsonDecode` reads a number beyond an `f64` as refused with ``number out of range``.
  `number_key_read` reads a number key, alone or under a brand, as serde reads the text between
  its quotes: the number, then the type's refusal of it, then the text after it. A text that opens
  with neither a digit nor `-` is refused with ``invalid value: expected key to be a number in
  quotes``, one that breaks the number past its first character, `01` or `-`, with ``invalid
  number``, a float beyond an `f64` with ``number out of range`` and one that goes on after the
  number, `1x`, with ``expected `"` ``. A float for an integer key, `1e3` or `-0`, is refused at
  the map, and an integer beyond the type under the number, the `source` of its
  `FormatException`. These messages are `NOT_A_DOCUMENT`: the `ws_rpc` client and attachment answer a refusal of one of them with
  `undeserializable-payload` naming no field (`_{service}WsRefused`), as the Rust dispatcher
  answers what serde classifies as text that is not the document, and the transport's `_onFrame`
  reads a frame through `_{service}WsHeld`, a reviver that refuses an infinity, so a frame that
  holds a number beyond an `f64` is dropped, as the Rust transports drop it. It reads the frame
  after `_{service}WsSignedZeros` writes each `-0` outside a string as `-0.0` (`signed_zeros_fn`,
  `SIGNED_ZERO`): `jsonDecode` reads `-0` as the `int` 0, and serde as the float `-0.0`, so the
  read of an integer refuses it, as it refuses `1.0`, `1e2` and `-0.0`, which `jsonDecode`
  reads as a `double`, with ``invalid type: floating point `-0.0`, expected u32``, serde's words
  (`integer_read`: the float spelled as `serde_json` spells it, plainly from `0.00001` to below
  `1e16`, `1e+16` beyond, and `number out of range` for an infinity). Every Dart read of a JSON text passes it through such a function first, one
  declared in each text that reads JSON (`signed_zeros_call`): the `http_rest` client reads a
  response's body and a fault's through `_{service}HttpSignedZeros` (`response_json`), the `mcp`
  server each message through `_{service}WsSignedZeros`, and the `mcp` client each answer and the
  text of a tool error through `_{service}McpSignedZeros`. `fromJson` called by hand on what
  `jsonDecode` hands reads `-0` as 0. The `mcp` server reads each message through the same
  reviver, and answers one that holds such a number with `-32600`. It names the field of a refusal
  from the JSON schema of the message, and where the schema admits the value, `1.0` or `-0` for
  an integer, from the path the read refused at (`_{service}WsRefusedAt`). A `DateTime<Tz>` is written
  `toUtc().toIso8601String()` with `.000Z` written as `Z` (`CHRONO_WHOLE_SECONDS`), as a value and as a map's key; a map read by a date key refuses two keys that read as one with `repeated key`. The WebSocket client writes and reads
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
- **A plain enum with no `check`** is `undrafted_plain_enum_dart_source`: the enhanced enum `{Name}` alone, with its `fromJson`, `fromJsonUnjudged`, `toJson` and `validationIssues()`.
- **A plain enum that names a `check`** is `plain_enum_dart_source`: the enhanced enum `{Name}Draft`, whose static
  `fromJson` is the unjudged read and which declares `validationIssues()` and `validate_method`, and
  the `valid_class` `{Name}` with `Pair::by_draft`, which adds `operator ==` and `hashCode` over the
  draft. `{Name}.fromJson` judges through `judged_body_returning`, and `{Name}.fromJsonUnjudged`, the
  name a holder reads every declared type by, runs no check. A map key reads through
  `{Name}.fromJson` and writes through `toJson`.
- **`validate()` stays the last member of a class**: `validateBound()` and `_bounds()` are
  written ahead of it.
- **A member serde never writes is read and has no entry in `toJson`.** `DartField::never_written`
  is `has_serde_skip_serializing`: `skip_serializing`, never `skip_serializing_if`.
  `class_body_parts` writes no entry for it, a flattened one spreading nothing, a variant with
  named members sharing it. `collect_dart_fields` pushes no nullable level on a flattened one,
  which has no key to leave out, so one that is no `Option` is read as any flattened member is
  and its constructor parameter is `required`. A member under `skip_serializing_if` is written
  wherever it holds a value: no client calls the Rust function, and the guide states the limit.
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
  holds one. serde refuses an answer its type does not read. Under a `nonstrict` holder,
  `corrected_option` writes `switch (answer) { null => null, final corrected => read }`, the
  member `null` exactly where the functions answer `null`, as serde and the holder's Zod schema
  read it, and `held_read` wraps the read of a type whose keys are there in `caught`, so a read
  that throws is `null` wherever a key is looked for. `run_dart_flattened_options.rs` holds each
  row of `CORRECTED` and `OPEN_CORRECTED`, in `tests/kotlin_tests/corrected_options_models.rs`, to
  what Zod reads. What a member takes is `taken_by`, the `Taken` recorded for its type in `DART_TAKEN_KEYS`, and nothing for a
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
    `value_wrapper_tokens` records, `$oid`: the guide's `ObjectId` refuses
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
    enum, an enum under a tag alone declared below its holder and one that names a function on
    itself have no key to be looked for by, and are read in the closure.
  - A type parameter: an optional one is looked for by the keys of what fills it, which the type
    that fills it hands. `record_flattenings` notes in `DART_LOOKED_UP` each parameter a type
    reads flattened under an `Option`, itself or through a type it hands the parameter to, with
    the name its two reads take the keys under, named and optional (`{List<String>? heldKeys}`,
    `generic_codec`; `DART_LOOKING` for the item being emitted). `argument_converters` hands the
    keys `argument_keys` answers: those an `Option` of the argument flattened in its place is
    looked for by, or the holder's own for one of its parameters. `flattened_read` writes
    `(heldKeys == null ? <the closure> : (handed.keys.any(heldKeys.contains) ? ... : null))`.

  A struct that names a `preprocess` on itself is read by serde as a value, and one with a
  flattened member of its own as a map: each records none. A type declared below its holder or in
  another crate publishes what it takes: `__TIXSCHEMA_TAKEN` (a struct's keys, an enum's tag and
  content key), `__TIXSCHEMA_ENTRIES` (the names each variant of an externally tagged enum is read
  under, one list per variant) and `__TIXSCHEMA_UNITS` (the first name of each unit variant, which
  the Swift read uses), written by `model_schema/flatten_order/published.rs`. The holder names each by a placeholder
  (`member_keys::taken_of`, `entries_of`, `units_of`) that `dart_definition()` replaces, and
  `late_flattened_decode` reads the member as the type's entries and keys say at run time: from the
  first entry under a variant's name where there are entries, from the keys taken where there are
  keys, and as a type whose keys are not known otherwise. So the read of such a member is the one
  a type declared above takes. `toJson` writes each
  flattened member at its place; a key two of them hold is refused at expansion
  (`model_schema/flatten_order.rs`), so no holder the emitter writes holds one. `run_dart_flattened_options.rs` pins the read of a flattened
  `Option` of a struct and of an enum of each tagging and runs it under the Dart SDK, over the
  models of `tests/swift_tests/flattened_options_models.rs`, which the `dart_tests` binary cannot
  declare beside the `Veering` of `kotlin_tests/aliased_models.rs`, and of `Cringle`, an enum
  under a tag that names a function on itself, which it declares. `src/features/dart/tests.rs`
  pins the read of an enum under each tag shape whose class was emitted above.
- **No cast is written of a value Dart types `Map<String, dynamic>` already**, a cast `dart
  analyze` reports as unnecessary. `joined_entries` spreads what a flattened member, and the lone
  slot of a variant under a tag alone, writes as it is where `writes_an_object` answers: a map, the
  guide's `ObjectId`, and a struct `DART_WRITES_AN_OBJECT` notes, a class whose `toJson` is
  `Map<String, dynamic>`. An optional one is cast to the map it holds. A brand, an alias, a tuple struct and an enum answer `dynamic` from
  `toJson` and are spread through `as Map<String, dynamic>`. A type the emitter has not written, one
  declared below its holder or in another crate, is spread through a `switch` over an `Object?`
  that writes a `String` as `{name: null}` and a map as it is, which holds no cast. A
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
  member writes the spread it wrote. The enum's map branch
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
  serde takes, a reply that breaks a rule through both clients, and the guide's section.
  `tests/dart_tests/flattened.rs` pins the read of a flattened member at each place, over the
  models of `tests/dart_tests/flattened_models.rs`, which also pin what serde reads of each, and
  `run_dart_flattened.rs` reads and writes each back under the Dart SDK, against serde. A map of
  models with a rule (`Limber`) is held to serde in every build with `serde`, and the Dart read
  refuses what it breaks in every build.
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
functions, each member's as it is read, the value, then `validate()`. `docs/guide/preprocess.md`'s "A
`preprocess` in the Dart client" section is the consumer's view, which calls the key a last
resort; this is what a change has to keep.

- **The dispatch is handed the type's functions** in `DartTypeArgs::preprocess`, and a member's
  are in the meta `member_meta` answers. A type that names none, on itself or on a member, is
  written as it was, a holder of a flattened member apart (below).
- **A type's functions are called by `unjudged_read`**, the one writer of the body of every
  unjudged read: a class, a wrapper (a brand, a tuple struct), the base of a sealed enum and a
  plain enum. Where the type names functions the read takes its payload as `payload` and holds
  what they answer as `json`, cast as the read types it (`final json = withoutFragile(payload)
  as Map<String, dynamic>;`), and a read of no member holds it under no name. `read_pair` and
  `valid_class` write `fromJson` as it was: it calls no function. A sealed enum calls them in its base alone
  (`Corrects::of_a_variant`), and the draft of a plain enum calls them in its one `fromJson`.
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
  signature of both reads and where a value behind the parameter is read. A function of the
  same name as a member of the type, a check or a `preprocess`, is called through a private
  top-level forwarder declared ahead of the item (`called`, `DART_MEMBERS`, `DART_FORWARDERS`,
  written by `dart_module_tokens`): `final _parcelCheckLabel = label;`. `record_members` holds
  the names a member may be written as, and a function that names none is called by its own name.
- **The payload a type's function takes is typed as its read types it**: a
  `Map<String, dynamic>` for a class and an enum under a tag key, a `String` for a plain enum,
  `dynamic` for a wrapper and any other enum. A payload of another kind is refused ahead of the
  function, where Rust hands it over.
- **The tests.** `tests/dart_tests/preprocess.rs` pins the text of each shape.
  `tests/service_schema_emitted_client_tests/run_dart_preprocess.rs` compiles the models of
  `tests/pattern_preprocess_tests/models.rs` and runs them under the Dart SDK: the sample and the
  shared table of positions print what the Rust read prints, two rows apart (`NOT_READ`), with
  the keys a flattened type is handed, the calls of one read counted, a function named after a
  name of the read, a reply through both clients, and the guide's section. Its module is
  declared last in the binary: its models take names the modules above it declare, and an
  expansion reads a type by its name.

#### The `mcp` server and client

`src/features/service_schema/dart_mcp.rs` writes `dart_mcp_service()` and `dart_mcp_client()` on the registry of a service that asks for `mcp`.

- **Two halves of the text.** The emitted text holds markers for what only a compiled type knows: the tool list (`mcp_tools()`, one Dart string literal per tool with `$` written `\$`), the names (`MCP_NAMES`), the instructions, the crate version (`env!("CARGO_PKG_VERSION")`) and `wrapped_{operation}()`. The accessor writes them over the markers when it runs in the consuming program, as `ts_http_service()` writes `openapi()`. An operation whose wrapping depends on the type it names reads a `const` table of its answers; one whose message is generated, or which binds a `header_in`, has it fixed at expansion.
- **A draft a handler throws is answered as its valid type.** Every class and enum `dart_definition()` writes declares `static Object $thrown<T>(Object thrown)`: a draft answers `validate()`, a type with no draft and any other object answers what it was handed. `guarded_call` in `dart_ws_client.rs` wraps the handler's call in a `catch` that rethrows `Error.throwWithStackTrace(DeclaredError.$thrown(thrown), trace)`, with the type arguments the declared error is written with and, for a `header_out` tuple, the first element converted and the rest kept, so the `on` clause that answers the declared error answers the draft as well. The dispatcher names only the declared error's class, which exists in the library that declares it whatever its kind, so no registry of the expansion decides what it writes, and a type declared below the service or in another crate compiles. The `mcp` server and the attachment share it.

- **The server reuses the WebSocket attachment's seams.** Each tool is a private function that reads the message with `message_decode`, the read of `dart_ws_client.rs` (the generated type's `fromJson`, which judges), then the `header_in` arguments, calls the handler, and renders the outcome with `split_reply_record` as the attachment does. The read's refusal is handed whole to `_McpUnreadFault`, which answers it as the Rust dispatcher answers a refusal of the arguments it reads out of a `Value`: ``invalid value: expected key to be a number in quotes`` is `undeserializable-payload` naming no field, ``expected value`` (a `Result` of other than one key) names the path its `FormatException` carries (`_WsRefusedAt`) ahead of the walk's, and so does a refusal inside a flattened member, whose walk path (`frame.oak`) is below the holder the read refused at (`frame`) as serde names it, the walk's entries below the holder dropped (`run_carillon_mcp.rs` holds each to the Rust dispatcher, beside a refusal under each shape of tag and under an untagged enum), and any other answers `failed-validation`: `_McpMismatches` walks the tool's JSON schema from the catalog (`_McpMessageSchema`) beside the arguments, past the `header_in` keys and the form's answers, and collects every value that does not match in the order met: the dotted path of the first (`drafts.1.title`) is the field, and the detail opens with that path in single quotes before the decoder's words. A key an object's schema does not declare under `"additionalProperties": false` is such a value, so a stray key is the field. Where the walk finds two or more, `_McpUnreadFault(..., every: true)` joins the others to the detail in the walk's words and lists each in `issues`, as the Rust dispatcher does, for a tool, a prompt (`_McpPromptSchema`) and a resource template (`_McpReadSchema`, over the `mcp_read_inputs()` the text embeds); a completion names the first alone. Where the walk finds none, the fault names no field. The walk follows `$ref`, the first alternative of an `anyOf` that has no mismatch (else the first one's), `enum`, an object's `properties`, `required` and `additionalProperties`, an array's `items`, and the type of a leaf. It does not follow `oneOf` or `const`, which the schema of an enum under a tag, adjacently tagged, externally tagged or untagged holds, so a value inside such an enum reaches no mismatch of the walk and the fault names the path the read refused at, as serde does (`run_carillon_mcp.rs` holds the field of each shape to the Rust dispatcher's). A wrapped message missing its `value` key answers `failed-validation` with the field `value`. The client's methods write the attachment's `_WsRefused`, and the `_WsFailedValidation` that names a header (`header_refused_fault`), as `_McpReadFailed`, so a success, a declared error, and a `header_out` or `error_header_out` value beside either that its read refuses answer `undeserializable-payload` naming no field, as the Rust client answers them.
- **Keys are sorted.** `_McpJson` writes every object with its keys in order, the bytes a `serde_json` build without `preserve_order` writes. Tool, resource and prompt texts are embedded as they were handed in, so a schema keeps the digits Rust wrote; `mcp_tools()`, `mcp_resources()` and `mcp_resource_templates()` return their members in key order for that reason.
- **Resources.** `resource_functions` writes one private function for each resource, template, listing and completion, built as a tool's is (`message_decode`, the handler, the outcome rendered by `dart_json_encode`) and answering a `-32602` where a tool answers a tool error: `_McpUnread` for a fault, `_McpRefused` for a declared error. `_McpRead` tries a fixed URI ahead of the templates, `_McpCaptured` matches a template as the Rust dispatcher does, and a `(Vec<u8>, String)` success is `base64Encode` of the record's bytes. `{Service}McpResource` and `{Service}McpResourcePage` are written once per service that lists, named after it.
- **`serve` is the whole of the protocol around a route**: `-32600`, a notification, the `_meta` keys (`-32602`), the version (`-32022`), `server/discover`, the four list methods, `resources/list` with its `"<service>:<cursor>"` cursor (asking each service from the one the cursor names until one lists), and the routed methods, which ask each service's route in order; a routed request names nothing to answer with `-32602`, as the Rust `serve` does. `serveHttp` adds the rules of the Streamable HTTP transport in front, and answers the status each rule names: the metadata document first, then the method and `Origin` rules, then `admit`, then the rest. A denial is a `DocumentServiceMcpNoValidCredentials`, `DocumentServiceMcpInsufficientScope` or `DocumentServiceMcpForbidden`, written as the Rust dispatcher writes it.
- **The writer.** `DocumentServiceMcpCall` holds the `progressToken`, the writer and a cancelled flag; `progress` writes `notifications/progress` itself through `_McpJson` and is awaited, and a write that throws sets the flag. `serve` reads the message ahead of `_McpRespond`, marks the request a `notifications/cancelled` names in `DocumentServiceMcpInFlight`, and builds the `Ctx` and lists the request in the table on the first route call (`tools/call`, `resources/read`, `resources/list`, `prompts/get`, `completion/complete`, `skills/get`, the three task methods and `subscriptions/listen` alike), so a request answered ahead of any service (`server/discover`, the list methods, a refusal) never builds one. `serveHttp` takes the `DocumentServiceMcpHttpStream` of the host and hands `admit` a call that holds what it writes until the stream has begun; a routed request that carries a token (or a `subscriptions/listen`) calls `begin` on its first route call, writes each event as it happens, and answers `null`.
- **Asking and content.** A tool that asks takes `params` as well as the arguments: `answers_stmt` reads `inputResponses` into `members` ahead of the message's read, and the declared error is held as `declaredBody` for `_McpAsking`, which answers `input_required` when the tag key `mcp_variant_tag` publishes for the arm holds the name serde writes it under. A content tool answers `_McpBlocks` (a list of `McpContent.toJson()`) or `_McpMedia` (an image or audio pair); the client reads a pair from the structured value or from its one block with `_McpPair`, and a list through `McpContent.fromJson`, which `dart_mcp_content()` publishes.
- **The endpoint is a `const` class over a `Set<String>`**: Dart evaluates a `const` set at compile time and refuses a repeated element.
- **The client** numbers its requests with `_nextId`, sends `tools/call` and `prompts/get` through `send`, and reads the answer with the decode statements of the WebSocket client (`success_decode_block`, `declared_error_decode_stmts`), whose fault helper is replaced by one that answers `undeserializable-payload` (the client's own read of a reply, which names no argument).
- **Names.** A header argument is named through `dart_parameter`; the locals the server and client write around it (`arguments`, `members`, `source`, `result`, `parsed`, `failure`, `structured`, `text`) are in `TAKEN_BY_A_METHOD`.
- **The tests.** `tests/service_schema_emitted_client_tests/run_dart_mcp.rs` runs the server and the client under the Dart SDK on the services of `mcp_dart_services.rs`; every answer is validated against the vendored 2026-07-28 schema, and the shared-name `const` endpoint is compiled and refused. `run_dart_mcp_resources.rs` sends the requests of `mcp_library_rows.rs` to the server and holds each answer to the Rust dispatcher's, on the services of `mcp_library_service.rs`.
