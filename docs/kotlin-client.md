# The Kotlin Client

`validate()`, the read and `preprocess` in the Kotlin client.

#### The Kotlin client's `validate()`

Every Kotlin type `src/features/kotlin.rs` writes carries `fun validate(): List<String>`, written
by `src/features/kotlin/validate.rs`: a data class, a unit struct's `object`, a brand's value
class, a tuple struct, a sealed interface with each variant, and a plain enum. It answers what
Rust's `validate()` answers for the same value. `README.md`'s "`validate()` in the Kotlin client"
section is the consumer's view; this is what a change has to keep.

- **The emitter reads what the web surfaces read.** `kotlin_schema_dispatch` takes `KotlinArgs`,
  which `kotlin_suffix_tokens` builds: the `name` override, the type's `check` names,
  `default_types`, and a brand's `minLength`, `maxLength` and `pattern`. `KotlinArgs::member_meta`
  is `member_prop_meta`, so an enum's own string check is each reached member's. `patterns_of`
  reads `enum_pattern` under the gate that field is compiled under.
- **`Rules` is the one writer, and its order is Rust's.** `Rules::member` writes what the value a member
  holds reports (`held_statements`), then its bounds (`bound_statements`), then its checks
  (`check_statements`). What its type declares comes before the member's own bound, as
  `named_member_validate_body` does in Rust. `Rules::own` writes
  the type's checks last, `Rules::brand` a brand's own bounds and no walk, and `Rules::written` the
  method. A type with no statement writes `= emptyList()`.
- **The sentence is `kotlin_violation`'s**, in `src/bound_message.rs`, written from
  `Bound::stated()`. `kotlin_reports_each_bound_in_the_sentence_rust_reports_it_in` holds it to
  `rust_violation`, per bound.
- **The units.** A string's length is `codePointCount`, in Unicode code points, and a list's is
  `size`. `Number` compares a range in the Kotlin type the field is held as, and quotes a float
  back through `kotlinSchemaShown`, as Rust's `Display` writes one. `jvm_pattern` writes `$` as
  `\z`, `\v` as `\x0B` and a named group as a plain one, which is what the `regex` crate reads in
  the three; the message states the pattern as declared.
- **Every function a type calls is private to the type's own body** (`Helper`): `kotlinSchemaUnder`,
  `kotlinSchemaShown`, the `String.kotlinSchemaText()` a declared type is asked through, and the
  `Any?.validate()` a held type with no `validate()` answers with. The emitted types are joined
  into one file, where two file-scope declarations of one signature do not compile, and tixschema
  ships no runtime library.
- **A declared type is asked for its text, and the language picks.** A brand over a string or a
  path carries `kotlinSchemaText()` (`Rules::text_member`), and a brand over a brand or over an
  alias that answers one answers through it. A holder writes `held.kotlinSchemaText()`
  (`text_read`): a member is found ahead of the holder's own extension of `String`, so a brand
  declared above or below the holder and an alias of a string all build, and no expansion is
  refused. `NOTED` is read for four things alone: whether the declared type a brand holds was
  written above and answers its text, whether a held type is known to carry `validate()`,
  which spares its holder the fallback, and the two the read asks: whether the type has an
  unjudged serializer a holder can name, and whether it carries `kotlinSchemaBound()`.
- **A slot and a variant's member are named by the steps serde writes** (`Tagging`,
  `variant_body`, `slot_rules`): the variant's wire name where no tag is written, the content key
  under a tag and a content key, and nothing under a tag alone and under no tag; a slot's position
  in the array serde writes, `0` for the one slot `#[serde(skip)]` leaves of several, and nothing
  for the one slot of a struct or a variant that declares one.
- **A map's key is `key_step`'s**: the key itself for a string, a number and a `Boolean`, the text
  its serializer writes for a declared type, and a type parameter's only where
  `serialized_parameters` finds `Serialize` declared for it, as Rust writes it.
- **One wrapper is written per tuple shape per item.** `tuple_wrapper` queues a wrapper
  class the first time the item asks for a shape, and answers its name to every later ask of that
  shape (`KOTLIN_TUPLES`, cleared with the queue in `kotlin_module_tokens`). A shape is the
  class's body without its name: the type of each position and the serializer each is written
  and read with, so a walked and an unwalked read are two shapes where a position holds a model
  the walked one reads unjudged. The walk reads a position as `slot{N}` off the value, and names
  no wrapper. A wrapper whose positions reach type parameters of its holder declares them, in
  the order a walk first reaches them (`FieldDef::reach_type_parameters`, which Swift's tuple
  structs read too), and its serializer is a class over one serializer per parameter, bound to
  `Any`: `tuple_wrapper_typename` writes the reference with the parameters and
  `tuple_wrapper_serializer` hands the holder's serializers of them. A wrapper that reaches none
  is an `object`, written as it was. `Yoked` in `run_kotlin_lone_slots.rs` reads a tuple of a
  parameter filled at a text and at a model under kotlinc.
- **A brand with a parameter** runs its bounds behind `if (value is {Default})`, the Kotlin type
  of the parameter's declared default (`declared_default`). A default written with arguments
  writes no bound: the JVM keeps no argument to tell the filling by.
- **The tests.** `tests/kotlin_tests/validate.rs` pins the text in every build with `kotlin`, a
  build with no schema surface included: Rust holds no bound there and Kotlin holds every one.
  `tests/service_schema_emitted_client_tests/run_kotlin_validate.rs` compiles the emitted types
  with `-Werror`, runs them, and holds each row to what Rust's `validate()` answers, or, for a
  slot and a variant's member, to the name written in the row. `src/features/kotlin/tests.rs`
  holds the README's block to what the emitter writes for the documents' parcel.

#### The Kotlin client's read

The serializer of a Kotlin type throws for a payload whose value breaks a bound or a check.
`src/features/kotlin/read.rs` writes it. `README.md`'s "The read in the Kotlin client" section is
the consumer's view; this is what a change has to keep.

- **A type has two serializers, a plain enum apart.** `{Type}Unjudged` reads and writes the type
  and judges nothing. `{Type}Serializer` (`judged`) is the type's own, named by
  `@Serializable(with = ...)`: it reads through the first and throws
  `SerializationException(validate().joinToString("; "))`. It is also a `() ->
  KSerializer<{Type}>`, and called it answers the first, which a holder that cannot name it reads
  by (`read_or_judged`, below). A type with parameters writes both as classes over one serializer
  per parameter.
- **The plugin's serializer is kept where the plugin takes the annotation.** A class, a unit
  struct's `object` and a brand's value class carry `@KeepGeneratedSerializer` (`kept`,
  `kept_inline`), and their `{Type}Unjudged` delegates to `generatedSerializer()` (`generated`).
  A class under its own tag wraps it in a `JsonTransformingSerializer` instead
  (`read::transforming`), whose `transformSerialize` (`read::tag_first`) moves the tag the plugin
  writes after the members ahead of them, as serde writes it; `kept_unjudged` joins it to the
  correction of a type that names a function. The serializer the emitter writes for a struct with
  a flattened member, a tuple struct and an enum is the unjudged one, under `written_head`.
- **A struct under its own tag reads its name under the tag alone**, as TypeScript and Zod read
  it (`kind: z.literal("Flag")`), where serde reads past the tag whatever it holds, and where it
  is absent. `own_tag_held` in `src/features/kotlin.rs` writes the one check, inline: ``missing
  field `{key}` `` for an object without the key, and `expected tag "{name}", found {value}` for
  another value under it, each a `SerializationException`. `kept_unjudged` writes it into
  `transformDeserialize`, after the correction of the payload where it has one, over the object
  the plugin then reads, and `flatten_merging_serializer` right after `obj`, the corrected object
  every member is read from. A holder that flattens such a struct hands it the tag, so the
  holder refuses the same payloads. The write is serde's either way: the struct's name, ahead of
  its members. `tests/kotlin_tests/flattened_models.rs` holds `FLAG_TAGS`, `PENNANT_TAGS` and
  `DRESSING_TAGS`, `tests/kotlin_tests/tests.rs` pins serde's read of each and the Zod literal,
  and `run_kotlin_flattened.rs` reads them under kotlinc, the first row as serde writes it back
  and every other refused.
- **A name serde writes is written between the quotes of a Kotlin string by `kotlin_quoted`**,
  over `kotlin_template_text`, which escapes a `$`, a `"` and a `\`: a member's wire name and
  aliases, a struct's own tag key and name, a variant's name, the tag and content keys, a plain
  enum's value, the key sets `kotlin_set_of` writes and a flattened enum's names. The service
  clients write each header name, part name and message name through it, as Dart writes them
  through `quoted` and Swift through `swift_quoted`. A name with none of those characters is
  written as before. `tests/kotlin_tests/quoted.rs` pins the text over
  `tests/dart_tests/quoted_models.rs`, `run_kotlin_quoted.rs` reads and writes back each payload
  against serde, and `run_quoted_service.rs` calls each mobile client of a service whose names hold
  them.
- **A tuple struct that declares one slot is the value of that slot**, as serde writes it whatever
  the slot's attributes say: `tuple_struct_kotlin_tokens` writes its `{Type}Unjudged` over the
  slot's value, the data class keeping `slot0`, and one of no slot or several over the array of
  the slots serde writes (`tuple_array`). `lone_slot_rows.rs`, in
  `tests/service_schema_emitted_client_tests/`, holds the table of such a struct in each position,
  which `run_kotlin_lone_slots.rs`, `run_dart_lone_slots.rs`, `run_swift_lone_slots.rs` and, for
  Zod, `run_node_lone_slots.rs` read and write back against serde.
- **Every refusal a serializer the emitter writes makes as it reads is a `SerializationException`**,
  checked inline in the read, with no declaration shared at file scope. Six writers in
  `src/features/kotlin.rs` word it: `read_as` and `payload_read_as` read a payload as an object, an
  array or a tag's string (`JsonKind`) and refuse any other kind with `expected {kind}, found
  {payload}`, `present` refuses a key the object does not hold with serde's ``missing field
  `{key}` ``, `tuple_array` refuses the array of a tuple of another length, a longer one among
  them, as serde and Zod refuse it, with `invalid length {n}, expected {m}`, `own_tag_held`
  refuses a struct's own tag of another value, or none (above), and `unknown_key_refused` refuses a
  key no member reads (below).
  `flatten_merging_serializer`, `own_field_plan`, `flatten_map_field_plan`,
  `tuple_wrapper_typename` and `tuple_struct_kotlin_tokens` call them, and so do
  `dispatched_tagged_enum_kotlin_source` and `dispatched_tagged_variant` for the tag and the
  content. An externally tagged object of no entry or of several is `expected an object of one
  key, found {payload}`, a tag no variant has `unknown tag {tag}` under every tagging, and a
  payload no variant of an untagged enum reads `No variant of {Enum} matched`. The `.jsonObject` of
  a write is the write's. `run_kotlin_refusals.rs` reads each refusal under the toolchain, beside
  serde's refusal of the same payload. The refusal of a text of another form is worded in
  `src/features/kotlin/texts.rs`, and that of a fixed array of another length in
  `src/features/kotlin/arrays.rs` (below).
- **The serializer `flatten_merging_serializer` writes refuses a key no member reads**, as the
  plugin's refuses it under the default `Json` and as TypeScript and Zod refuse it, where serde
  reads past it: after the members are read, `unknown_key_refused` throws `Encountered an unknown
  key '{key}'` for the first key of the object no member read, under a `Json` whose
  `configuration.ignoreUnknownKeys` is `false`, so a caller's `Json { ignoreUnknownKeys = true }`
  reads past it, as it reads past it in a class the plugin reads. A flattened struct whose keys
  `read_keys` knows is read under the caller's `Json`, from the object kept to those keys
  (`flatten_struct_field_plan`), and a flattened externally tagged enum from its one entry
  (`flatten_enum_field_plan`), so a model either holds, at any depth, refuses a key no member of it
  reads, as it does held under a key. Every other flattened member is read with `lenient`, which
  `FlattenedMember::lenient` declares only where one is, and every model below it reads past such
  a key: the README states the limit. `flatten_plan`
  answers what the members read in `FlattenPlan::read`, the terms subtracted from the object: the
  wire name of each own member (an alias is already read as it), the entry a flattened externally
  tagged enum took, and every key `KOTLIN_READ_KEYS` notes for a flattened struct the emitter
  wrote (`read_keys`): the name and each alias of every member, one serde never reads among them,
  and its own tag, so a holder reads the tag serde writes for it. `flatten_merging_serializer`
  adds the struct's own tag. `FlattenPlan::read` is `None`, and no key is refused, beside a
  flattened map, a member `preprocess::takes_no_key` answers for and a type `KOTLIN_READ_KEYS`
  does not note: a struct with a flattened member of its own or that names a function on itself,
  an enum `flattened_variant_names` answers no names for, and a type the emitter has not written.
  Zod refuses a key none of those reads, and the README states the limit. A payload serde writes
  holds no such key. The serializer of an enum under a tag key refuses a key beside its tag and its
  content key, under a tag and a content key, and beside the tag of a unit variant under a tag
  alone (`unit_variant_read`), through `unknown_key_refused`, where Zod's strict object of each arm
  refuses it; a variant of members under a tag alone is read by its class, which refuses it.
  `src/features/kotlin/tests.rs` pins the text, and `run_kotlin_unknown_keys.rs`
  reads each payload under kotlinc with both `Json`s, beside serde's read and what Zod answers
  under node; the rows of `run_kotlin_flattened.rs` that hold a key serde reads past are pinned to
  the Kotlin read, the Zod read.
- **A `char` and a chrono date, time and date and time are read in their one form, and written
  in the form serde writes**, by `src/features/kotlin/texts.rs`. Kotlin holds each as a `String`,
  and `TextForm` (`src/features/text_forms.rs`, section 14) names its form. `texts::serializer`
  declares, once per item and form, `object KotlinText{N} : KSerializer<String>` into the queue
  the tuple wrappers are written from (`KOTLIN_TEXTS`, cleared with them), which throws
  `SerializationException` for any other text: `expected a character, found "ab"` for a text of
  other than one code point, as serde refuses it, and `expected a date, found "x"` for a text
  Zod's `z.iso.date()`, `z.iso.time()` or `z.iso.datetime({ local: true })` refuses, written with
  the regexes of Zod 4.6.5. Where serde and Zod part on a date, the read follows Zod: serde reads
  more forms than Zod, years outside 0000 to 9999 and leap seconds among them, and refuses a date
  and time under `Z` or with no seconds, which Zod reads. The write drops that `Z`
  (`TextForm::unzoned`), writes the seconds (`TextForm::unsecond`), and throws in the words of
  the read for a text that is then not of `TextForm::sent`. A `DateTime<Tz>` held as its text is
  `TextForm::Zoned`: read as any text, as Zod reads any text a JavaScript `Date` reads, and
  written held to RFC 3339 with its zone, its seconds written, `expected a date and time with its
  zone, found "x"` for any other.
  - Where the plugin reads, the declaration names the serializer. `read::plugin_read` writes
    `@Serializable(with = KotlinText{N}::class)` at each place `texts::declared` finds a form, the
    value itself, inside a list, a nullable, a map's key and value, a type argument, a range's
    bound and a `Result`'s arm, through the `Annotated` of `annotated_base`, and
    `read::with_text_forms` writes it on a brand's value and on the type an alias names. The
    property keeps its type.
  - Where the emitter writes the read, `kotlin_serializer_expr` composes the serializer wherever
    `texts::reaches` finds a form, as it composes one for a type parameter: the reified
    `serializer<T>()` reads no annotation. The key of a flattened map of a form is read through it
    (`flatten_plan`). An alias written above with no type parameter is read as the type it names
    (`texts::note_alias`, `texts::named_by`). A type argument is handed to the serializer of a
    model the emitter wrote above (`validate::reads_unjudged`); an alias with type parameters and
    a type declared below keep `serializer<T>()`, which reads any text, and the README states the
    limit.
  - The service clients and the WebSocket dispatcher read a message, a reply and a header the same
    way: `kotlin_read_serializer` composes its serializer, `read_serializer` in
    `src/features/service_schema/kotlin_http_client.rs` hands it the message's type, and each
    emitter writes what `kotlin_declarations` drains of the queue at the end of its source. A
    header the HTTP client reads is read from its text as a JSON string through it
    (`kotlin_header_out_decode`). `run_kotlin_service_forms.rs` reads a reply of a date and a
    header of a `char` under kotlinc.
  - `tests/kotlin_tests/text_forms_models.rs` holds `TEXT_FORMS`, what serde, Zod and the Kotlin,
    Dart and Swift reads answer of each payload of a `char` and of each chrono type in each place.
    `tests/kotlin_tests/text_forms.rs` pins the text, `run_kotlin_text_forms.rs` reads each payload
    under kotlinc, `tests/generic_types_tests/declarations.rs` holds Zod to the table under `node`,
    and `src/features/kotlin/tests/texts.rs` pins each shape and each limit.
- **A number and a `bool` are read from an unquoted JSON value alone**, as serde, Zod and the other
  clients read them: `kotlinx.serialization` 1.11.0 reads `"7"` for a `Long`, `Int`, `Short`,
  `Byte`, `Double`, `Float`, `UByte`, `UShort` and `UInt`, and `"true"` for a `Boolean`.
  `texts::Primitive` names each (`Primitive::of`, with `Millis` for a `DateTime<Tz>` under
  `as_number`), and `texts::number_serializer` declares `object KotlinNumber{N} :
  KSerializer<T>` once per item and primitive into the queue the tuple wrappers are written from
  (`KOTLIN_NUMBERS`, cleared with it; the counter is not, so two types of one file share no name).
  It reads the element through `decodeJsonElement()` and throws `SerializationException("invalid
  type: string \"7\", expected i64")` for a string primitive, in the words serde uses with the
  Rust type of the member (`a boolean` for a `bool`, `a unix timestamp in milliseconds` under
  `as_number`), and hands anything else to the library's serializer. The places are the text
  forms': `texts::found` walks them (`Annotated::numbers` names the serializer where the plugin
  reads, `kotlin_serializer_expr` composes it where the emitter writes the read), and a map's key
  is the one place it is skipped (`texts::found`'s `key` flag, `key_serializer_expr`), since JSON
  writes a number key between quotes. `u64` and `usize` are refused before they are read.
  `tests/service_schema_emitted_client_tests/run_kotlin_refusals.rs` reads a quoted value in every
  place beside serde's refusal, and `run_kotlin_wide_integers.rs` pins the quoted `i64` refused.
- **A fixed array is read from an array of its length alone**, at each level, by
  `src/features/kotlin/arrays.rs`. `arrays::serializer` declares, once per item and length, `class
  KotlinArray{N}<Item>(item: KSerializer<Item>) : KSerializer<List<Item>>` into the queue the tuple
  wrappers are written from (`KOTLIN_ARRAYS`), which reads through `ListSerializer(item)` and throws
  `SerializationException("invalid length n, expected N")`, the words of `tuple_array`. Where the
  plugin reads, `annotated_base` writes `@Serializable(with = KotlinArray{N}::class)` on each level
  of one length, and the plugin hands the class the serializer of the items, a type parameter's
  among them. Where the emitter writes the read, `listed_serializer` composes `KotlinArray{N}(...)`
  for each such level, wherever `arrays::reaches` finds one, through the walk `texts::found` makes,
  so a type argument of a type declared below and of an alias with parameters reads any length,
  as a text does there. `FieldDef::fixed_length_at` is compiled under `kotlin`. `validate()` is
  unchanged. `tests/kotlin_tests/fixed_arrays_models.rs` holds `FIXED`, a fixed array in each
  place and serde's read of each payload, `tests/kotlin_tests/read.rs` pins the text, and
  `run_kotlin_refusals.rs` reads each payload under kotlinc through both serializers, and the
  `Clenching`, `Clasping` and `Grasping` of the Dart run.
- **A member serde never writes is read and has no `put`**, in a struct and in an enum's variant.
  `KotlinField::never_written` is `has_serde_skip_serializing`: `skip_serializing`, never
  `skip_serializing_if`. The plugin writes every property, so `struct_kotlin_tokens` hands a
  struct with such a member to `flatten_merging_serializer`, as it hands one with a flattened
  member, `written_by_hand` answers for a variant with one as for a variant with a flattened
  member (below), and `own_field_plan` answers no `serialize` statement for the member. A struct
  or a variant with neither keeps the plugin's serializer and its text. A flattened member has no
  `put` either: `flattened_merge` answers no statement for it, and `flatten_plan` pushes none for
  a flattened map. `collect_kotlin_fields` pushes the nullable level of a member whose key serde
  leaves out on no flattened one, which has no key: one that is no `Option` is read as any
  flattened member is, as serde reads it whatever its `default` says. A member under
  `skip_serializing_if` is written wherever it holds a value: no client calls the Rust function,
  and the README states the limit.
  `tests/service_schema_emitted_client_tests/unwritten_models.rs` holds the models, a struct, a
  flattened struct and a flattened map, and an enum under each tagging that writes a variant's
  named members, and serde's writes, and `run_kotlin_unwritten.rs`, `run_dart_unwritten.rs` and
  `run_swift_unwritten.rs` read and write each back under the toolchain, against serde.
- **Every flattened member is read at its place, in the order declared**, as serde reads it
  (`flatten_plan`): from `JsonObject(obj - setOf(...) - {member}Keys ...)`, the object without the
  keys the holder's own members take and those each member flattened ahead of it took, and from
  `obj` where nothing was taken. A struct, and an enum under a tag key, the emitter wrote that
  hands its object to no function (`reads_the_whole`: `KOTLIN_TAKEN_KEYS` or
  `KOTLIN_TAG_KEYS` records it, and `preprocess::takes_no_key` does not answer for it) is read
  from `obj`, the whole object, as TypeScript and Zod read it, so a key a member of its holder's
  own, a struct flattened ahead of it or the tag of the variant that flattens it reads too is
  read by both, where serde refuses the payload. A variant under a tag alone that is written by
  hand hands its class `obj`, the tag in it (`dispatched_tagged_variant`), and its serializer
  counts the tag among the keys its holder's own members take (`BesideTag`), so the unknown-key
  check passes it. `tests/swift_tests/shared_key_models.rs` holds `SHARED_KEYS`, and
  `run_kotlin_flattened.rs` reads each as Zod reads it. An optional one looks for its keys in the
  object it is read from (`flatten_struct_field_plan`): `null` where none is there, and read where
  one is, so a key that does not read refuses the payload, as TypeScript and Zod refuse it, where
  serde reads `None`.
  An optional struct `KOTLIN_TAKEN_KEYS` records with no key, one with a flattened member of its
  own, has none to be looked for by (`recorded_keys`): where the member names no function it is
  read inside `runCatching { ... }.getOrNull()`, held wherever its read succeeds and `null`
  wherever it throws, as serde's untagged read of an `Option` reads `None`, and it still takes no
  key. A member that names a function is read as the Zod schema of its holder reads it: its
  functions are called first, into `{member}Corrected`, the type is read from what they answer
  inside `runCatching`, and `getOrElse` answers `null` where the object handed holds none of its
  keys (`.keys.any({member}Keys::contains)`), or no key at all where `{member}Keys` is empty, and
  throws the read's error where it holds one. Zod's union tries the type over the function's
  answer first, and its branch without the type takes an object of no key beyond its holder's
  own, so serde's refusal of `{"note":"n"}` and its `None` for a function that answers `null` are
  not followed. `tests/kotlin_tests/corrected_options_models.rs` holds `Reeved`, `Wormed` and
  `Moused` with what serde, TypeScript and Zod read of each payload, which
  `tests/generic_types_tests/declarations.rs` measures, and `run_kotlin_flattened.rs` holds the
  Kotlin read to Zod's. An optional enum under a tag key the emitter wrote, whose member names no
  function, is looked for by the tag, then the content key, which `enum_kotlin_tokens` records in
  `KOTLIN_TAG_KEYS` (`tag_keys`, `recorded_tag_keys`): `if ({rest}.keys.any { it in setOf("kind",
  "body") }) ... else null`, so a tag or a content that does not read refuses the payload with
  what the enum's read throws, as TypeScript and Zod refuse it, where serde reads `None`, and a
  variant's member with no tag reads `null`, as serde reads it. `{member}Keys` stays the
  `elementNames` of its descriptor, which names none, so the members flattened after it are
  handed what they were. An untagged enum and one that names a function on itself record no key.
  A type neither registry records keys for, a struct or an enum the emitter has not written, an
  untagged enum, an enum that names a function on itself, an alias and a type parameter, is looked
  for by the `descriptor.elementNames` of its serializer where they name an element, and read
  inside `runCatching` where they name none, the check made at run time: `if
  ({member}Keys.isEmpty()) runCatching { ... }.getOrNull() else if (...)`. So a serializer written
  by hand is tried, where TypeScript and Zod refuse a key of the type that does not read, and the
  plugin's is looked for by its keys. A type parameter is looked for, where those elements are
  none, by the keys `filled_standing` writes as `{member}Standing`: a map by Kotlin name of the
  keys `KOTLIN_TAKEN_KEYS` and `KOTLIN_TAG_KEYS` hold for each type the emitter wrote above, read
  at the `descriptor.serialName` of the parameter's serializer, which one written by hand declares
  as the type's name. `Impress` and `Waxwork` in `object_id_models.rs` hold it. Where the type
  is read by a serializer that judges, one
  declared below its holder among them, the tried read is `read_or_judged`: where the judged read
  throws, the payload is read again by what the serializer answers called, `(serializer as?
  Function0<*>)?.invoke() as? DeserializationStrategy<*>`, its unjudged one, or by itself where it
  answers none, and the judged read's refusal is thrown where that read succeeds: a value that
  reads and breaks a rule is refused with the rule's message, as serde, TypeScript and Zod refuse
  it, and one that does not read is `null`. A type of the consumer's own is tried as before.
  `GROMMETING` in `tests/kotlin_tests/unrecorded_options_models.rs` holds `Grommeting` over
  `Gromet`, declared below it with a rule and a flattened member of its own, with serde's answer of
  each payload, and `run_kotlin_flattened.rs` holds the Kotlin read and Zod's to it.
  `tests/kotlin_tests/unrecorded_options_models.rs` holds `Fiddling` over `Crowned` declared below
  it and a holder of an enum under each tagging but serde's own, with serde's write-back of each
  payload and whether TypeScript and Zod take it, which `tests/generic_types_tests/declarations.rs`
  measures under `tsc --strict` and `node`, and `Clapping` over `Clapped`, which names a function
  on itself, with serde's write-back; `tests/kotlin_tests/tests.rs` pins the Kotlin text and
  `run_kotlin_flattened.rs` runs it against serde, beside `Ratline`, which keeps the key test.
  `run_kotlin_flattened.rs` also reads `Lowering`, `Squaring`, `Heeling` and `Pitching` of
  `tests/swift_tests/flattened_options_models.rs` against serde, each row TypeScript and Zod
  refuse held to what the Kotlin read throws. A struct takes the keys `flattened_struct_keys`
  answers, unless `preprocess::takes_no_key` answers for it: a member that names a function, and a
  type `HANDED` does not note as handing its object to none, so a type the emitter has not written
  takes none.
  `struct_kotlin_tokens` records in `KOTLIN_TAKEN_KEYS`, for every struct it writes, the keys
  `keys_taken_when_flattened` in `src/features/serde.rs` answers: the name and each alias of every
  member serde reads, one serde never writes among them, and neither the struct's own tag nor a
  member under `skip_deserializing`; none for a struct with a flattened member of its own, which
  serde reads as a map. The descriptor of a serializer the emitter writes declares no element,
  and the plugin's names the tag and a member serde never reads, and no alias. A struct that
  names a function on itself takes none, and is recorded with the keys its read reads, which an
  optional holder looks for. An enum under a tag and a content key takes both, as serde reads it
  as a struct of the two: `enum_kotlin_tokens` records them in `KOTLIN_TAKEN_TAG_KEYS` from
  `tag_and_content_when_flattened` (`taken_tag_keys`), none for an enum that names a function on
  itself, and a member flattened after it, and the unknown-key check, count them taken. A map is
  the one member of its object (section 4).
- **A flattened map's key is written and read through the key's own serializer**
  (`flatten_map_field_plan`), where the key's Kotlin type (`kotlin_typename`) is no `String`: it
  is written as `output.json.encodeToJsonElement(<key>, k).jsonPrimitive.content` and read over
  `entries.associate` through `input.json.decodeFromJsonElement(<key>, JsonPrimitive(k))`, the
  serializer `kotlin_serializer_expr` answers, so a key that serializer refuses refuses the entry
  and reads an optional map as `null`. A plain enum's key is the name serde writes, its `rename`
  and the enum's `rename_all` among them. A `String` key, a `char` and a date among them, is
  written and read as it was. A map keyed by a number or a `bool` is refused flattened (section
  4). `tests/kotlin_tests/flattened_models.rs` holds `Cathead`, `Hawsepipe` and `Snubbing` and
  serde's reads of `CHOCKED` and `SNUBBED`, and `run_kotlin_flattened.rs` holds the Kotlin read
  of each to serde's.
- **An optional flattened map is `null` where serde reads `None`** (`flatten_map_field_plan`):
  written through `value.{member}?.forEach`, so `null` writes no entry, and read inside
  `runCatching { ... }.getOrNull()`, so an entry that throws as the map's value reads the map as
  `null`, as serde's untagged read of an `Option` reads `None`. The decode of an entry throws
  `SerializationException`, and `IndexOutOfBoundsException` for a list or an object where a text
  or a number is read, so the read catches whatever is thrown, as the untagged enum's
  `recoverCatching` chain does. A map that is not optional is read as it was.
  `tests/kotlin_tests/flattened_models.rs` holds `ProbeLocker` and serde's reads of `LOCKER`, and
  `run_kotlin_flattened.rs` holds the Kotlin read of each to serde's.
- **A flattened externally tagged enum takes one entry** (`flatten_enum_field_plan`): the first of
  that object, in its order, under a name in the `setOf(...)` written from
  `variant_names_when_flattened` in `src/features/serde.rs`, which Dart reads too. The holder hands
  the enum the object of that one entry, throws `SerializationException` where there is none and
  reads `null` for an optional enum, and every member after it is read from the object without
  `{member}Entry.key`. Entries under two variants' names, or under a variant's name and its alias,
  throw `SerializationException("more than one variant of enum {Enum} found in flattened data: " +
  ...)`, naming each variant once, in the order the enum declares them, as TypeScript and Zod refuse
  the payload, where serde reads the first: `KOTLIN_VARIANT_NAMES` holds the names one list per
  variant. `enum_kotlin_tokens` records the names in `KOTLIN_VARIANT_NAMES`, and
  `flattened_variant_names` reads them: an enum under a tag key, an untagged one, one that names a
  function on itself or on its member, and one the emitter has not written are read as a struct is,
  and take no key, a limit the README states for one the emitter has not written. The names hold
  each `alias`, and the enum's own read takes a variant under each of them, so an entry under one is
  taken and read as that variant (`UNDER_AN_ALIAS`).
- **A flattened enum's unit variant is written as its name over `null`** (`flattened_merge`), as
  serde writes it, where the enum writes a bare name. `enum_kotlin_tokens` records in
  `KOTLIN_BARE_NAMED` each externally tagged enum with a unit variant, whatever function it
  names, and `writes_a_bare_name` reads it: the holder merges what such an enum writes through
  `when (val written = ...)`, a `JsonPrimitive` as `put(written.content, JsonNull)` and an object
  entry by entry, optional and under a function too. Every other flattened member merges its
  object as it did, so an enum the emitter has not written throws on a unit variant: the README
  states the limit. `alias_kotlin_tokens` records an alias of such an enum as well, and
  `value_class_tokens` and `tuple_struct_kotlin_tokens` a brand and a struct of one slot over one
  (`note_bare_named`), so each is written the same way, flattened and beside a tag.
  `run_kotlin_flattened.rs` writes each variant of `Tacking` against serde.
- **An externally tagged enum reads a unit variant's entry over `null` alone**, as serde reads
  the enum alone (`unit_variant_read`): the arm reads `if (data is JsonNull)` and throws
  `SerializationException("{Enum}: " + tag + " takes no value")` otherwise, the wording of the
  Dart read; a bare name reads `data` as `JsonNull`. `flatten_enum_field_plan` hands the enum its
  entry outside `runCatching`, so an optional enum whose unit variant's entry holds any other value
  refuses the payload, as TypeScript, Zod, Dart and Swift refuse it, where serde reads `None`.
  `run_kotlin_flattened.rs` reads `Thimble` and `Easing` of
  `tests/swift_tests/flattened_options_models.rs` against serde.
- **An enum under a tag and a content key reads a unit variant from an object with no content
  key**, as Zod's strict object of the tag alone reads it (`unit_variant_read`): the arm reads
  `if (data == null)` and throws the same `SerializationException` otherwise, a content of `null`
  among them, which serde reads. serde writes the variant as its tag alone.
  `run_kotlin_refusals.rs` reads `Becalming` under the toolchain, with serde's answer and the Zod
  schema's text pinned beside each payload.
- **An enum under a tag alone is written by hand as well.** The plugin refuses
  `@KeepGeneratedSerializer` on a sealed interface, so `dispatched_tagged_enum_kotlin_source`
  writes all three taggings, `Tagging::Unwrapped` being the tag alone there: the tag is taken off
  the object and the rest is read as the variant's own subclass. A lone slot is read from the rest
  as the value it holds, through the serializer the other taggings read it with
  (`read::declared_serializer`), and its members are written after the tag, as serde writes them
  (`merged_beside`, which writes an externally tagged enum's unit variant as an entry of its name
  over `null`). The subclass keeps its `value` property and carries its tag in `@SerialName`, and
  a tag no variant has throws `SerializationException`. `run_kotlin_tagged_slots.rs` reads and
  writes back a slot of each shape serde writes beside the tag, against serde and Zod, with
  `run_dart_tagged_slots.rs` and `run_swift_tagged_slots.rs` beside it.
- **A variant with a flattened member is written by hand too**, under every tagging, as serde
  reads and writes the member at the variant's own level, and so is a variant with a member serde
  never writes. `written_by_hand` asks it of the variant's payload, `variant_subclass` names
  `{Subclass}Unjudged` in the class's `@Serializable(with = ...)` and declares the members as the
  emitter's serializer reads them,
  and `variant_serializer` writes that serializer through `flatten_merging_serializer`, with no
  tag and no function of the type's. The enum hands it the variant's object as it is
  (`members_renamed` and `variant_keyed` answer nothing for it): the serializer reads each alias
  and calls each member's function itself. The enum judges the variant, so the subclass has no
  judged serializer. A variant with neither keeps the plugin's serializer and its text.
  `tests/kotlin_tests/flattened_models.rs` holds the models and serde's reads, and
  `run_kotlin_flattened.rs` reads and writes each under kotlinc, against serde.
- **A model is read unjudged exactly where `validate()` asks it, and nowhere else.** A model read
  unjudged where no `validate()` reaches it is a broken value handed back. `validate::asks_the_held`
  is the condition `Rules::member` walks under, asked of a scratch `Rules`, and
  `holds_an_unjudged` follows the walk of `held_statements`. `plugin_read` writes the annotation
  where the plugin reads, on the property and on the type inside a list, a nullable and a map,
  and `member_serializer` composes the serializer where the emitter writes the read. What a brand
  holds, a type argument and a member with a bound of its own on a brand are read by the type's
  own serializer.
- **A holder names only a serializer the emitter has written.** `reads_unjudged` answers for a
  type written above. A type declared below its holder, an alias and a type of the consumer's are
  read by their own serializer and judge themselves as they are read: an annotation names a class,
  and no fallback stands in for one. Rust reads a model declared below unjudged, so the two differ
  there, and `run_kotlin_read.rs` pins the Kotlin text.
- **A tuple a `validate()` walks reads its positions unjudged, by its walked wrapper wherever the
  emitter reads it.** `tuple_wrapper_typename` takes `walked`: `annotated_base` and
  `unjudged_serializer` ask for the wrapper of a walked tuple, at any depth of the member, and
  `kotlin_base` for every other. One wrapper is written per tuple shape per item, and a walked and
  an unwalked read are two shapes where a position holds a model, so a member is declared with the
  wrapper its read names. `plugin_read` declares what the plugin reads. Where the emitter writes
  the read through `member_serializer`, `read::declared_typename` declares the member: a member of
  a struct with a flattened member, a slot of a tuple struct and a member a serializer of its own
  reads (`read_whole`). The wrapper of a walked tuple declares each position with
  `read::position_typename`, the type `position_serializer` reads. A serializer that reads a tuple
  its holder has declared names the wrapper of that declaration (`read::declared_serializer`): the
  lone value of a tagged variant, a variant of several slots among them, is read by the type
  `variant_subclass` declared it with.
- **An untagged enum takes the first variant whose bounds hold.** `Rules::choosing` collects, per
  variant, each member's own bound and `kotlinSchemaBound()` of each brand it holds
  (`Asked::Bound`), and writes them into the variant's own `kotlinSchemaBound()`, which the chain
  of `untagged_enum_kotlin_source` asks after the variant decodes. `Rules::brand` writes the
  method on a brand with a bound. A check and what a held model reports choose nothing: the
  enum's own serializer refuses them after the choice, as serde's read does. `Helper::Unbounded`
  answers for a type the emitter has not written.
- **A name serde reads beside the one it writes is read too.** `aliases::written_on` reads the
  `#[serde(alias)]` names of a variant and of a field, and `KotlinField::aliases` holds a member's,
  none for a flattened member or one serde never reads. `dispatched_tagged_variant` writes each
  variant's arm under its written name and each alias, under every tagging. `aliases::keys_named`
  reads a member's alias as its wire name in the object the member is read from, and refuses two
  names of one member as serde refuses a duplicate field: in the `{Type}Unjudged` `kept_unjudged`
  writes for a class the plugin reads, a `JsonTransformingSerializer` over the plugin's, after the
  type's own functions and ahead of `preprocess::keyed`; in the object `flatten_merging_serializer`
  reads every member from; and in the object a tagged or an untagged enum hands a variant's class
  (`members_renamed`). A plain enum that names no function is read by
  `aliases::plain_enum_serializer`, a `{Type}Unjudged` written by hand that reads each variant under
  its name and each alias, and carries no opt-in. One that names a function keeps
  `read::kept_plain`, and `aliases::text_named` reads the alias after the functions. The write keeps
  the written name. A struct records in `KOTLIN_TAKEN_KEYS` the keys `keys_taken_when_flattened`
  answers, its aliases among them, which its descriptor's `elementNames` do not hold. `@JsonNames`
  is not used: it is experimental, and the build compiles under `-Werror` with no opt-in to it.
- **A serializer the emitter writes binds a parameter to `Any`** (`COMPOSED`), which
  `KSerializer<T>.nullable` requires, and one that delegates to the plugin's binds none. So
  `unjudged_serializer` composes a model written with a nullable argument through its own
  serializer.
- **The tests.** `src/features/kotlin/tests.rs` pins the text of each shape, a type written above
  and one written below in one test: the registries are thread-local, and a unit test has its
  own. `tests/kotlin_tests/read.rs` pins what the macro's own path writes, in every build with
  `kotlin`. `run_kotlin_validate.rs` reads each row with `{Type}Unjudged` and, in a run of its
  own, with the type's serializer, held to what `validate()` answered.
  `run_kotlin_read.rs` holds the README's rows to what the Rust read answers of the same payload,
  and both clients to their fault. `tests/kotlin_tests/flattened_models.rs` holds the models of a
  member flattened after a struct and beside a member of its holder's own, a struct under its own
  tag and one with a member serde never reads among them, and of an externally tagged enum
  flattened beside other keys, `tests/kotlin_tests/tests.rs` pins their reads, and
  `run_kotlin_flattened.rs` reads each under the toolchain, against serde, and writes back the
  structs, a variant's entry under an alias among them (`UNDER_AN_ALIAS`). It also reads
  `Scarfing`, and `Crowning`, `Fidding` and `Flaking` of
  `tests/swift_tests/flattened_options_models.rs`: an optional struct that takes no key against
  serde, and one that takes a key against the rule TypeScript and Zod hold. It writes back `Flag`,
  `Gonfalon` and `Semaphore`, a struct under its own tag alone, with a parameter and with an
  alias, and `Dressing`, which flattens `Ensign` ahead of a member, each with the tag first.
  `tests/kotlin_tests/aliased_models.rs` holds the models of a variant and a member under an
  alias, under each tagging, in a plain enum, a struct the plugin reads, one read by hand and one
  flattened, and pins what serde reads of each payload, which the Dart client is held to as well.
  `tests/kotlin_tests/aliased.rs` pins their Kotlin text, and `run_kotlin_aliased.rs` reads and
  writes back each payload under the toolchain, against serde.

#### The Kotlin client's `preprocess`

The Kotlin read calls each function a `preprocess` names, under the name the attribute writes:
`fun trimmed(payload: JsonElement): JsonElement`, on a field and on a type alike.
`src/features/kotlin/preprocess.rs` writes each call, inside the unjudged read, so both
serializers of a type make it. `README.md`'s "`preprocess` in the Kotlin client" section is the
consumer's view; this is what a change has to keep. The key is a last resort, and the documents
say so.

- **The emitter is handed the type's functions.** `KotlinArgs::preprocess` is the type's own,
  from `kotlin_suffix_tokens`, and a member's are in the meta `KotlinArgs::member_meta` answers
  (`preprocess::of`). `preprocess::corrected` is the one writer of a call: each function over the
  answer of the one before, in the order written.
- **The order of one read is Rust's**: the type's functions on the whole payload, each member's
  on the member's own, the value, then `validate()` in the type's own serializer. Nothing is
  called on a write.
- **A type's functions are in `{Type}Unjudged`.** Where the plugin writes the serializer,
  `kept_unjudged` hands `read::transforming` the correction: a `JsonTransformingSerializer` over
  `generatedSerializer()`, in place of the delegation `read::generated` writes. A serializer the
  emitter writes calls the functions on `input.decodeJsonElement()`: the flatten-merging one, a
  tuple struct's, a tagged enum's, and an untagged enum's ahead of the choice of a variant. A
  type that names no function is written token for token as before.
- **A brand with a parameter is read through `read::transforming_inline`.** kotlinc hands a
  value class with a parameter unboxed to the `serialize` a `JsonTransformingSerializer` inherits,
  and the type's own serializer then throws `ClassCastException` on a write. So that one keeps
  the delegation and overrides `deserialize`.
- **A plain enum that names a function carries `{Type}Unjudged` as its one serializer**
  (`read::kept_plain`). `note_plain_enum` still says no holder names it: a holder reads the enum
  by its own serializer, which is that one.
- **A member the plugin reads is read through a serializer of its own.** `Plugin::declared`
  queues `object {Owner}{Member}Preprocess : JsonTransformingSerializer<T>(...)`
  (`read::corrected_member`) and answers the annotation that names it. It reads through
  `read::read_whole`: `member_serializer`, the serializer the member has without the key, so a
  model is still judged once. A tuple is asked for its name once there. `kotlin_module_tokens`
  writes the queue after the type's source (`preprocess::serializers`).
- **A member that reaches a type parameter is corrected at its key** (`Plugin::keyed`,
  `preprocess::keyed`). The plugin takes a serializer on a property only where its constructor
  takes no parameter, or one per argument of the property's own type, so none can be handed the
  parameter's serializer. `{Type}Unjudged` corrects such a member of a class, and the enum's
  serializer the object it hands a variant's class (`variant_keyed`).
- **A member the emitter reads is corrected in place**: `own_field_plan`, a slot of a tuple
  struct, a slot of a variant read by the enum's serializer, under a tag alone from the object
  without the tag, a slot of an untagged variant as the variant is tried, and a position of a
  tuple's wrapper (`tuple_wrapper_typename`).
- **No function is called for a key the payload does not hold.** The plugin asks a property's
  serializer only for a key that is there, and `own_field_plan` corrects inside `?.let`. A `null`
  is handed to the function: the serializer of a nullable member is a
  `JsonTransformingSerializer<T?>`.
- **A flattened member a function reads is handed what serde hands it**, as every flattened
  member is ("The Kotlin client's read", above), and takes none of its keys
  (`preprocess::takes_no_key`). `HANDED` notes, per type the emitter wrote, whether it hands its
  object to a function, its own or one of a member it flattens: a type known to hand it to none
  takes its own keys.
- **A flattened map is read from every key of the object** (`flatten_map_field_plan`): it is
  the one member of its object (section 4), and the enum's serializer takes the tag off the
  object before it hands it a variant's class. A function the map names is handed the whole
  object. An optional map is read from what the function answers as the Rust read of a
  corrected payload reads it: `null` where it answers `JsonNull`, and refused where an entry
  does not read as the map's value (`CorrectedLocker`, `EmptiedLocker`).
- **The path of a refusal below a function starts at the value the function answered**: a
  member and a type that name one are each read from a JSON value of their own. A list or an
  object where a text or a number is read is thrown by `kotlinx.serialization` 1.11.0 as
  `IndexOutOfBoundsException`, as it is by every read the emitter writes itself.
  `run_kotlin_preprocess.rs` pins both, and that each client answers its fault for the second.
- **The tests.** `src/features/kotlin/tests/preprocess.rs` pins the text of each shape, and the
  README's block to what the emitter writes for the documents' parcel.
  `tests/kotlin_tests/preprocess.rs` pins what the macro's own path writes, in every build with
  `kotlin`. `tests/service_schema_emitted_client_tests/run_kotlin_preprocess.rs` compiles the
  emitted types with `-Werror`, runs them, and holds each row to what the Rust read prints of
  the same payload: the README's sample, every position a type is held in, every shape, and
  every place a member is read. Rows whose words are Kotlin's own are pinned beside them.
