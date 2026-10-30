# The Swift Client

`validate()`, the read and `preprocess` in the Swift client.

### 10. The Swift `validate()`

Every Swift type the emitter writes has `public func validate() -> [String]`, which runs what the
Rust `validate()` of the type runs, in its order and in its words. `README.md`'s "`validate()` in
the Swift client" section is the consumer's view; this is what a change to the emitter has to keep.

- **The emitter is handed what it reads.** `swift_suffix_tokens` in `src/model_schema.rs` builds a
  `SwiftTypeArgs`: the `name` override, the type's `check` names, a brand's or an enum's
  `minLength`, `maxLength` and `pattern`, the declared `default_types`, and `member_meta`, which is
  `member_prop_meta` with the enum's arguments where the item is an enum. Every member's bounds
  are read through it (`member_def`), so an enum's string check reaches in Swift the fields it
  reaches in Rust.
- **`src/features/swift/validate.rs` writes the method.** `member_rules` is one member: what the value it
  holds reports (`held_walk`), then its bounds (`bound_rules`), then the checks it names
  (`check_rules`). The type's message comes before the member's own, which is
  `named_member_validate_body`'s and `slot_validate_body`'s rule. `own_checks` are the type's, last. `method` writes the method
  and the helpers its statements call, and `[]` for a type with no rule.
- **One sentence source.** `swift_violation` in `src/bound_message.rs` is written from
  `Bound::stated()`, as `rust_violation` is, and
  `the_swift_sentence_is_the_one_rust_writes_for_each_bound` holds the two together.
- **The unit of a length is the Unicode code point** (`unicodeScalars.count`), and a list is
  counted by `count`. Rust measures a string with `len()`, its bytes: a test of a non-ASCII string
  pins this client's own text.
- **A range is written where a value can break it.** `Number::least` and `Number::most` answer no
  literal for a bound outside the range of the field's integer type: `x < 0` on an unsigned
  integer is a warning in Swift. A float is quoted back through `swiftSchemaPlain`, which drops
  the `.0` Rust does not write.
- **A pattern is respelled for Foundation.** `swift_pattern` writes `$` as `\z` and `\v` as
  `\u000B`, which is what makes the engine read the portable pattern as the `regex` crate does. The
  message quotes the pattern as written.
- **A place is the steps serde writes.** A named member goes by `SwiftField::wire_name`, a
  flattened one by none, a slot of a tuple by its position (`slot_rules`), the one slot of a tuple
  struct by none (`tuple_struct_rules`), and a variant's payload by the variant's name with no tag
  attribute, by the content key under one, and by none under a tag alone and in an untagged enum
  (`resolve_variant_payload`'s `step`). `held_walk` adds the index of an item and the key of an
  entry, a map being walked by its sorted keys.
- **Nothing is shared between two emitted types, and nothing is declared beside them.**
  `swiftSchemaUnder` and `swiftSchemaPlain` are private static functions written into each type
  whose statements call them (`method`). A name a second emitted type would declare too does not
  compile once the types are joined into one file.
- **A named type is measured by what the language finds.** A brand over a string or a path
  declares `swiftSchemaBoundText` (`brand_text_member`), a brand over a brand answering through the
  one it holds, and a bound on a field typed with any named type reads that member, declared above
  or below. The registry (`note`, `Text`) decides one thing: an alias of a string the emitter has
  written is measured as the string. No bound is refused as the type is expanded: a type with no
  such member is the Swift compiler's to refuse.
- **A holder calls `validate()` on every named type it holds** but one the registry has noted as
  running no rule (`validates`): a type declared below is called, the holder having no way to tell
  it from one with a rule.
- **A local of the method is never read as a property.** `property_read` reads a property named
  `errors`, `held`, `index` or `key` through `self` (`TAKEN_BY_VALIDATE`), and `swift_property`
  declares a field named `validate` as `validate_`, with its key kept. `swift_field_member` is
  the same seam, so a service client reads the property under the name it was declared with.

### 11. The Swift read

Every Swift type the emitter writes has two reads: `init(swiftSchemaUnjudged:)` is the type's own,
and `init(from:)` runs it and then `validate()`, throwing `DecodingError.dataCorrupted` with every
message joined by `; ` under the decoder's own coding path. `README.md`'s "A Swift read refuses
what `validate()` refuses" section is the consumer's view; this is what a change to the emitter has
to keep.

- **`src/features/swift/read.rs` writes both.** `both` takes the statements of the type's own read
  and writes the pair, `JUDGED` being one text for every type. A struct, the struct written for a
  variant's members and for a tuple, a brand's wrapper, a tuple struct and an enum under every
  tagging call it. A plain enum that names no check keeps the `init(from:)` Swift synthesizes, and
  its unjudged initializer is that read.
- **A member is read unjudged exactly where `validate()` walks it.** `validate::walks` is the one
  question: `held_walk` ends on a value whose `validate()` it runs (`reaches`). `read::unjudged` then takes the member apart as `held_walk` does: an `Option`,
  each level of a list, a map's values, the struct written for a tuple, a named type. A step added
  to one is added to the other: a value the read leaves unjudged and the walk does not reach is
  judged by nothing.
- **Any other value is read through `Decodable` and judges itself**: one behind a type parameter,
  one in a field with a bound of its own, and the value a brand wraps, a brand's `validate()`
  running no `validate()` of what it holds.
- **A held type is handed a decoder of its own** (`Source`): `superDecoder(forKey:)` under a key,
  `superDecoder()` in a list, and the holder's decoder for a flattened member, the tuple of a
  tuple struct and a payload beside a tag.
- **An absent key is what it was** (`Presence`). `superDecoder(forKey:)` answers a decoder over
  `null` for an absent key, where `decode` answers `keyNotFound`, so `Refused` writes
  `_ = try container.decodeNil(forKey:)` ahead of the read of a held type. `Nil` is an optional
  member's: `contains`, then `decodeNil`. `decode_statement` is told whether Swift synthesized the
  read before one was written (`synthesized`): a synthesized read took the absent key of every
  optional member as `nil`, a `nullable` one included, and a written one refused that key. A
  member under `#[serde(default)]` is read as its default where `contains` answers no key
  (section 13).
- **Every struct writes its memberwise initializer**, public, the struct written for a tuple
  included: Swift synthesizes none beside a written `init(from:)`.
- **`encode(to:)` does not move.** `struct_body_content` writes it where it wrote it before
  (`needs_encode`), and Swift synthesizes it everywhere else. A member of a `char` or of a chrono
  date or time held as a `String` is a `LeafConversion::Text`, so its type writes it.
- **A brand and a struct of one slot write their value through their own encoder**
  (`value_wrapper_tokens`): `try value.encode(to: encoder)`, as serde writes a newtype, so a
  flattened one joins its value's keys to the holder's object, which a single value written over
  it replaced. `Laden` in `tests/swift_tests/lone_slots_models.rs` flattens one of each.
- **A `char` and a chrono date, time and date and time are read and written in their one form**
  (section 14), by `src/features/swift/forms.rs`. `forms::read_call` reads the text through
  `swiftSchemaRead{Char|Date|Time|Stamp}`, which throws `DecodingError.dataCorrupted` and
  `expected {form}, found ...` for a text not of `TextForm::read`, and `forms::sent_call` writes it
  through `swiftSchemaSent…`, which drops the `Z`, writes the seconds and throws
  `EncodingError.invalidValue` in the same words for a text not of `TextForm::sent`, each under
  the coding path of the decoder or the encoder, and a map's key under the path of its entry. `forms::helpers` writes the functions a type calls as private static
  functions of that type, with `swiftSchemaQuoted`, and `leaf_helpers_for` writes them beside the
  date helpers; a map's wrapper reads and writes a key of a form through them. A pattern is
  written with `[0-9]` for `\d`: Foundation's `\d` takes a digit of any script. A `DateTime<Tz>`
  is a `Date`, read by a formatter with a fraction of a second or by one without
  (`swiftSchemaWholeIso8601Formatter`, `isoWholeFormatterForKeys`), and written by the first with the `.000Z` of a date of whole seconds written as `Z`, as chrono writes it (`CHRONO_WHOLE_SECONDS`; the formatter options are `FRACTIONAL_OPTIONS` and `WHOLE_OPTIONS`, shared by the field helpers, the key wrapper and `validate()`'s `key_text`). A wrapper of a date key refuses a second key that reads as the first, which Swift's millisecond makes one; under
  `as_number` it is an `Int64` read and written as the number serde writes.
  `run_swift_text_forms.rs` reads `TEXT_FORMS` under the toolchain, and `run_swift_wire_forms.rs`
  writes `WIRE_FORMS`.
- **A member serde never writes is read and encoded under no key.** `SwiftField::never_written`
  is `has_serde_skip_serializing`: `skip_serializing`, never `skip_serializing_if`. Swift's own
  `encode(to:)` encodes every property, so `needs_encode` holds for a struct with such a member,
  a variant's payload sharing `struct_body_content`, and the written one encodes no statement for
  it, a flattened one included, which `collect_swift_fields` pushes no nullable level on, having
  no key to leave out: one that is no `Option` is read as any flattened member is. Where no member
  is encoded under a key, it opens the object with
  `_ = encoder.container(keyedBy: CodingKeys.self)`. A member under `skip_serializing_if` is
  encoded wherever it holds a value: no client calls the Rust function, and the README states the
  limit.
- **The container of a struct's keys is declared where a statement uses it.**
  `struct_body_content` writes `let container` ahead of the read's statements and
  `var container` ahead of `encode(to:)`'s only where `names_container` finds a statement that
  reads or writes through it, so a struct and a variant's payload whose every member is flattened
  declare neither, which `-warnings-as-errors` would refuse unused.
- **Nothing is shared between two emitted types.** `SwiftSchemaAnyKey`, the key type a map is read
  entry by entry under, is private to each type whose read or written `encode(to:)` names it
  (`any_key`, `any_key_written`). `Locals` numbers what one initializer declares (`listed`,
  `items`, `keyed`, `entries`, `entry`, `number`), and a property is assigned through `self.`
  alone, so one named after a local is never read as it. A written `encode(to:)` reads each
  property through `self.` too (`encode_statement`), so a member named after one of its locals
  (`container`, `tagged`, `held`, `entry`, a `wire{Member}`) or its parameter, `encoder`, is
  never read as either. `run_swift_flattened.rs` runs a member named
  `container`, of its holder's own and flattened, against serde.
- **A map under a key Swift reads through a wrapper** (`MapKeys::Wrapped`) is read by the wrapper,
  which gains an `init(swiftSchemaUnjudged:)` where its values are walked.
- **A flattened map is read entry by entry** (`flattened_map`, from `member_reads`,
  `read::read_entries`), under no key the types around the holder took, a `where` clause over
  `taken` that `names_taken` counts, so the holder takes those keys: the tag of a variant under a
  tag key among them. A map is the one member of its object (section 4), so the holder's own
  members and the members flattened ahead of it take no key of it. A map under a key Swift wraps
  is read by its wrapper, through the `init(swiftSchemaUnjudged:swiftSchemaWithout:)`
  `flattened_wrapper_read` writes in an extension of the wrapper for the flattened member alone.
  An optional map is read behind `do { ... } catch { self.{member} = nil }`, a wrapped one too, so
  it is `nil` wherever an entry does not read as its key and value, as
  `FlatMapDeserializer::deserialize_option` answers `None` for any error of the map's read; a
  model it holds is read unjudged, and one that breaks its rule refuses the payload through
  `validate()`, as the Rust read refuses it. `run_swift_flattened.rs` runs a map alone in each
  shape, optional or not, and `tests/swift_tests/flattened.rs` pins its text. What a struct takes
  is `preprocess::keys_taken_by`: the keys `struct_swift_tokens` records for its type
  (`preprocess::note_keys`), serde's own list from
  `keys_taken_when_flattened`, and none for a member that names a function. A struct that names a
  `preprocess` on itself is read by serde as a value, and one with a flattened member of its own
  as a map: each records none. A type the emitter has not written takes none, and the README
  states the limit. `encode(to:)` writes each flattened member at its place, through its shape
  (`written_from`): a key two of them hold is written once, with the last value, where serde
  writes it twice. A flattened `Option` is written under `if let held` (`encode_statement`): the
  keys of the value it holds, and nothing for `nil`, where `Optional`'s own `encode(to:)` writes
  one value over its holder's keys. A holder with no member written under a key opens its object
  all the same. `tests/swift_tests/flattened_options.rs` pins the text, and
  `run_swift_flattened_options.rs` runs it against serde.
- **A flattened `Option` is read as `nil` where the object it is handed holds none of its type's
  keys, and a key there that does not read refuses the payload**, as TypeScript and Zod refuse
  it and Kotlin's `flatten_struct_field_plan` reads it. serde reads `None` there,
  `FlatMapDeserializer::deserialize_option` answering `None` for any error of the type's read.
  `flattened_option`, from `present_statement`, reads the type under the `Option` as a flattened
  member of that type is read, unjudged where `validate()` walks it and handed the keys taken
  where it takes them, behind `if {test} { ... } else { self.{member} = nil }`. The test is
  `preprocess::holds_one_of`: one of the keys `preprocess::keys_standing_for` answers for the type,
  in `decoder.container(keyedBy: SwiftSchemaAnyKey.self).allKeys`, less the `Handed` keys where
  the member is withheld and its type does not read the whole object
  (`preprocess::reads_the_whole`), so the key type is the one `read::both` writes into the
  holder. Those
  keys are a struct's, `preprocess::keys_taken_by`, and an enum under a tag key's: its tag, then
  its content key where it has one, which `enum_swift_tokens` records for an enum written above
  its holder (`preprocess::note_tag_keys`, from `tag_keys`), none for one that names a function
  on itself. So an enum under a tag key alone is `nil` where its tag is absent, a member of a
  variant there or not, and one under a content key too where the content is absent, and is
  refused where one is there and the enum does not read, as Zod refuses it. A type
  `keys_standing_for` answers no key for keeps `do { ... } catch { self.{member} = nil }`, which
  reads `nil` wherever its read throws: a struct declared below its holder or in another crate,
  one with a flattened member of its own or that names a function on itself, an untagged enum, an
  enum declared below its holder or that names a function on itself, an alias and a type
  parameter `preprocess::filled_keys` tells no keys for. A type parameter is tested by the keys of
  what fills it: `filled_keys` writes `{member}Standing`, a dictionary by Swift name of the keys
  `keys_standing_for` answers for each type written above whose keys are known (an `ObjectId`, a
  brand and a struct of one slot over one, a struct, an enum under a tag key), read at the name
  `String(describing:)` writes for the parameter with its arguments cut off, and `nil` keeps the
  `catch`. An externally tagged enum written above is read from its one entry (below). A model
  it holds that breaks its rule still refuses the payload through `validate()`, as the Rust read
  refuses it. A member that names a function is read as the Zod schema of its holder reads it
  (`corrected_option`, section 12), and a flattened map, optional or not, by `flattened_map`.
  `tests/swift_tests/flattened_options.rs` pins the text, and `run_swift_flattened_options.rs`
  reads each payload against serde, the rows serde reads as `None` and the Swift read refuses held
  to the refusal it prints, an enum of each tagging and one declared below its holder among them.
- **Every flattened member is handed the keys its holder withholds, the first one included.**
  `member_reads` hands each a `Handed`: the `taken + [...]` of the holder's own keys and those
  each member flattened ahead took, and `withheld`, whether any key can be withheld: the holder
  has a member of its own, a member ahead took keys, or the types around the holder hand some. A
  type that takes keys (`preprocess::beside`) and a member that names a function read that
  expression. A struct, and an enum under a tag key, written above that names no function
  (`preprocess::reads_the_whole`: `TAKEN_KEYS` or `TAG_KEYS` records it) is read from its
  holder's decoder, the whole object, as TypeScript and Zod read it, so it reads a key the types
  around it took, where serde refuses the payload (`tests/swift_tests/shared_key_models.rs`). Of
  those, a struct with a flattened member of its own takes keys: it hands them on to the members
  it flattens alone (`handed_reads`) and reads its own members from the whole object. Where
  `withheld` holds, `present_statement` reads any other flattened member through
  `corrected_field` with no function: `SwiftSchemaPayload(from:without:)`, then the type, a model
  `validate()` walks through `SwiftSchemaRead{Member}`, unjudged: a type declared below its
  holder, an untagged enum and an alias among them. A
  holder of no member of its own is handed keys where the types around it hand some (`around`,
  which `struct_declaration` takes) or where one of its reads names `taken`, so `handed_reads`
  writes its reads twice there, the second time with every member withheld from; one whose
  reads name none, and that nothing around hands keys, reads its members from its decoder.
- **A flattened `ObjectId` is read from its `$oid` key alone**, as the README's `ObjectId` refuses
  any other key, where serde reads past one flattened. `preprocess::only_keys` answers `$oid` for
  an `ObjectId` and for a brand, a struct of one slot or an alias over one written above
  (`ONLY_KEYS`, `preprocess::note_only_keys`), and `present_statement` reads such a member from
  `read::Source::Keeping`, the decoder's object filtered to those keys. `keys_taken_by` answers the
  same keys, so an optional one is looked for by `$oid` in the whole object it is read from, and
  the members after it are handed the object without it. A flattened member that is a type
  parameter itself (`preprocess::filled_alone`) is read behind a test at run time of the name
  `String(describing:)` writes for its filling: `ObjectId`, and each brand and struct of one slot
  over one written above that declares no parameter (`ONLY_NAMED`), is read from `$oid` alone,
  and any other filling as before. The test names no type, so a crate with no `ObjectId` builds.
  A type declared below the one that reads it is read as any other flattened member is, and the
  README states the limit.
- **A flattened externally tagged enum is read from its one entry.** `enum_swift_tokens` records,
  for an enum written above its holder, the names serde reads its variants under flattened
  (`preprocess::note_entries`, from `flattened_entries`): `variant_names_when_flattened`, none for
  an enum under a tag key, an untagged one and one that names a function on itself, each beside the
  name the enum's own read takes the variant under and whether the variant holds no value
  (`preprocess::Entry`). `preprocess::entries_of` answers for a member that names no function, under
  an `Option` or not, and `present_statement` hands such a member to `flattened_enum`, which writes
  `preprocess::entry`: the object without the `Handed` keys (`{member}Object`), the names it holds
  (`{member}Found`), in the order the enum declares its variants, and the read of the one entry from
  a payload of that entry (`entry_payload`): a unit variant's name where the entry holds `null`, an
  alias's entry under its variant's name, then the type, a model `validate()` walks through
  `SwiftSchemaRead{Member}`, unjudged. Entries under two names are refused with
  `DecodingError.dataCorrupted` and ``more than one variant of enum {Enum} found in flattened data:
  {variants}``, each variant named once (`{member}Variants`), in the order the enum declares them,
  an entry under a variant's name beside one under its alias naming that variant alone, as
  TypeScript and Zod refuse the payload, and as the Dart and Kotlin reads word it, where serde reads
  the first in the object's order: Swift keeps no order of an object's keys (`allKeys` and
  `JSONSerialization` answer them in another order on each run). No entry is refused with ``no
  variant of enum {Enum} found in flattened data``, as serde refuses it; an optional enum is `nil`
  there, and an entry the enum does not read refuses the payload, as TypeScript and Zod refuse it,
  where serde reads `None`. `handed_reads` adds `{member}Found` to the keys each member flattened
  after it is handed (`taken + [...] + {member}Found`). An enum declared below its holder is read as
  any other flattened member is, and is refused where the object it is handed holds another key.
  `tests/swift_tests/flattened.rs` pins the text of `Anchoring`, `Dredging` and `Fending`, and
  `run_swift_flattened_enums.rs` reads the holders of `tests/kotlin_tests/flattened_models.rs` and
  of `Careen` against serde, three times, the rows with entries under two names held to the refusal.
- **A flattened externally tagged enum's unit variant is written as its name over `null`**, as
  serde writes it. The enum writes it as a bare name through `singleValueContainer()`, which the
  holder's object drops with no error. `enum_swift_tokens` records, for an externally tagged enum,
  the case and the written name of each unit variant (`unit_entries`, `variant_names`, which
  `external_tagged_enum_swift_source` names its variants by, into `UNIT_ENTRIES`), and none for an
  enum of any other shape. `encode_statement` hands a flattened member to `flattened_write`, which
  writes, for a member `unit_entries_of` answers for, optional or under a function too, a `switch`
  whose case of each unit variant writes `encodeNil(forKey:)` under `SwiftSchemaAnyKey` and whose
  `default` writes the value as before.
  `struct_body_content` declares the key type where the reads declare none
  (`read::any_key_written`). An enum declared below its holder writes no entry for its unit
  variant, and the README states the limit. A member under a function reads the entry back
  through the enum's own read, which refuses it, as the Zod schema refuses it. `alias_swift_tokens`
  records an alias of such an enum with the enum's `UNIT_ENTRIES` and `ENTRIES`, so the alias is
  read and written the same way, and `value_wrapper_tokens` a brand and a struct of one slot over
  one (`note_value_entries`), whose unit variants a holder tells by `value` (`entries_held`,
  `SlotEntries::value`).
- **The lone slot of a newtype variant under a tag alone that holds an externally tagged enum
  written above is read and written as serde writes it beside the tag**: `slot_payload` hands the
  payload `SlotEntries`, the enum's unit variants. The slot is read from the object without the
  tag, a unit variant's entry over `null` read as the variant's name (`preprocess::Member::units`,
  through `unit_entry_read`), so the enum's own read takes it and refuses the tag alone and two
  entries, as serde does, and `VariantPayload::written_beside` writes a unit variant as its entry
  through `unit_entries_write`, which `flattened_write` shares; `internal_tagged_enum_swift_source`
  declares the key type for it. `run_swift_tagged_slots.rs` pins the text and runs `Docketed` of
  `tagged_slot_models.rs` against serde, its `ENTRIES_REFUSED` among it.
  `tests/swift_tests/flattened.rs` and `flattened_options.rs` pin the text, and
  `run_swift_flattened_enums.rs` writes each variant of `Tacking` in `Stowing`, `Belting` and
  `Yawing` against serde, and reads serde's write back.
- **An externally tagged enum is read from an object of one key.** The read
  `external_tagged_enum_swift_source` writes takes a unit variant from a bare string, then guards
  `container.allKeys.count == 1` beside the first key, as the read of a `Result` does, and refuses
  an object of no key with ``empty object for {Enum}`` and one of several with ``more than one key
  for {Enum}``, as serde and Zod refuse each: `allKeys` keeps no order, so no first key is known.
  `tests/swift_tests/read.rs` pins the guard, and `run_swift_flattened_enums.rs` reads `Tacking`
  alone and `Luffing`, which flattens an enum declared below it beside another key, against
  serde, three times.
- **An enum under a tag and a content key reads a unit variant from an object with no content
  key**, as Zod's strict object of the tag alone reads it (`adjacent_unit_arm`): the arm guards
  `!container.contains(.swiftSchemaContent)` and throws `DecodingError.dataCorrupted` with
  ``{Enum}: \(tag) takes no value`` otherwise, a content of `null` among it, which serde reads.
  serde writes the variant as its tag alone.
  `tests/swift_tests/tests.rs` pins the arm, and `run_swift_flattened_enums.rs` reads `Becalming`,
  of `run_kotlin_refusals.rs`, under the toolchain, with serde's answer and the Zod schema's text
  pinned beside each payload.
- **A struct under its own tag reads its name under the tag alone**, as TypeScript and Zod read
  it (`kind: z.literal("Flag")`), where serde reads past the tag whatever it holds, and where it
  is absent. `own_tag_held` in `src/features/swift.rs` writes the check, inline, ahead of the
  statements `struct_body_content` hands `read::both`: `DecodingError.dataCorrupted` with
  ``missing field `{key}` `` for an object without the key under `SwiftSchemaTagCodingKeys`, and
  `expected tag "{name}", found {value}` for another value under it, `null` among it, the value
  read through `SwiftSchemaPayload` and written by `JSONSerialization`. A type that names a
  function on itself checks the object its functions answered. A holder that flattens such a
  struct hands it the tag, so the holder refuses the same payloads. `encode(to:)` is unchanged.
  `run_swift_own_tags.rs` reads `FLAG_TAGS`, `PENNANT_TAGS` and `DRESSING_TAGS` of
  `tests/kotlin_tests/flattened_models.rs`, the same rows of its own `Bunted`, which names a
  function on itself, and a tag of `null` under the toolchain, with serde's write-back pinned
  beside each row, and pins the text of each read.
- **What a variant under a tag alone holds is handed the object without the tag**, as serde
  hands it, but a struct or an enum under a tag key `preprocess::reads_the_whole` answers for,
  which reads the whole object, the tag in it, as TypeScript and Zod read it.
  `EnumTagAttrs::tag_alone` is the one question. `named_payload` hands
  `struct_declaration` `around` under it, so the struct written for a variant whose every member
  is flattened withholds keys from each, takes the keys itself and is read by `read::beside` with
  `taken + [tag]`; a struct with a member of its own is written as it was. `slot_payload` reads a
  slot whose type takes no keys (`preprocess::beside`) through `VariantPayload::corrected` with no
  function: `corrected_member` reads it from `SwiftSchemaPayload(from:without:)`, a model
  `validate()` walks through `SwiftSchemaRead{Case}`, unjudged; one `reads_the_whole` answers for
  is read from the enum's decoder. A slot whose type takes keys is read through its taking
  initializer, as before. `tests/swift_tests/flattened.rs` pins the text
  of `Grafting` and `Pointing`, and `run_swift_flattened.rs` runs both against serde.
- **An untagged enum takes the variant serde takes.** `validate::bound_alone` is what chooses: a
  member's own bounds, and `held_walk` under `Walk::Bound`, which asks each brand the member
  holds for `swiftSchemaValidateBound()`, the member the wrapper of a bounded brand declares
  (`bound_member`) and every other generated type declares as `[]` (`validate::method`). The enum writes one `swiftSchemaBound{Variant}` per variant that has such a
  rule, and one `swiftSchemaRead{Variant}` where a slot is read in several statements. A check and
  a model choose nothing. The registry says which name is a bounded brand (`note_bounded`); a name it
  does not hold yet, a type declared below the enum, is asked as well, so a brand declared below
  chooses as one declared above. A variant whose payload Swift holds as an optional
  (`VariantPayload::optional`: an `Option`, or an alias `note_optional_alias` noted as one) is
  tried in `do { let value = try ...; ... } catch {}`, where `try?` would take a read that
  answered `nil` for one that threw, so `null` takes it, as serde reads it as `None`. Every other
  variant keeps `if let value = try? ...`. `tests/untagged_tests/null_models.rs` holds
  `NULL_ROWS`, and `run_untagged_nulls.rs` holds what Zod takes, and what each client reads and
  writes back, to serde.
- **A name serde reads beside the one it writes is read too.** `src/features/swift/aliases.rs` reads
  the `#[serde(alias)]` names of a variant and of a field (`aliases::written_on`), and
  `SwiftField::aliases` holds a member's, none for a flattened member or one serde never reads. A
  tagged enum takes a variant under its written name and each alias in one `case`
  (`aliases::case_names`), under every tagging, a unit variant of an externally tagged enum from its
  bare name. A plain enum with a variant under an alias declares `init?(rawValue:)`
  (`aliases::raw_value_init`), which Swift then synthesizes no more: the read Swift synthesizes, the
  read written for a check or a function and a map's key all read through it. A struct with a member
  under an alias declares `let aliased`, a container of every key under `SwiftSchemaAnyKey`
  (`aliases::CONTAINER`, in `struct_body_content`), and `decode_statement` writes `aliases::lookup`
  ahead of the member's read: the names among the member's own and its aliases the object holds
  (`{member}Names`), refused with ``duplicate field `{wire}` `` where two are, as serde refuses
  them, and the one it holds, or its own where it holds none (`{member}Key`). `SwiftField::keyed_at`
  answers the container and the key every read of the member is written over, `keyed_read` and a
  default's among them, so the member is read as before under the name found. `handed_reads` adds
  each alias of a member of the holder's own to the keys a flattened member is not handed.
  `CodingKeys` and `encode(to:)` do not move: the write keeps the written name, and a type that
  names no alias is written as it was. An externally tagged enum read alone still refuses a unit
  variant's entry over `null`, under its name or an alias, as TypeScript and the generated Zod
  schema refuse it, where serde reads it. `tests/swift_tests/aliased.rs` pins the text over the
  models of `tests/kotlin_tests/aliased_models.rs` and of
  `tests/swift_tests/aliased_members_models.rs`, which holds a member under an alias in each way the
  read reads a member under its key, and `run_swift_aliased.rs` reads and writes back each payload
  of the Kotlin and the Dart runs and of `Tholing` under the toolchain, against serde, those of
  `tests/dart_tests/flattened_aliased_models.rs` among them.
- **A fixed array is read item by item and held to its length, and so is a tuple.**
  `read::holds_a_length` is the one question: a level written as `[T; N]`
  (`FieldDef::fixed_length_at`), an alias of one written above (`through_alias`, over
  `COUNTED_ALIASES`, which `alias_swift_tokens` notes for an alias that declares no parameter), or
  the values of a map Swift keys natively that hold one. `read::counted` reads such a value: each
  level through a nested unkeyed container, refused by `refusing_another_length` after its items
  with `DecodingError.dataCorrupted` and serde's `invalid length {n}, expected {N}` at the coding
  path of that array, a map's entries one by one, and the rest in one call
  (`Shaped::read_whole`). `read::unjudged` refuses the same way at each level it walks. It is
  reached from every read of a value: `present_statement` for a member, `Shaped::read` for a slot
  of a tuple, of a variant and of a `Result`, a map's value and a flattened map's entry, the brand
  and the lone slot (`wrapper_read`, `lone_slot_read`), and `preprocess::Plain::Counted`, which
  `plain_read` chooses, for a member a function corrects and a default. A fixed array that fills a
  type parameter is held in `{Holder}{Field}Argument{N}Fixed`, written by `held_argument`, which
  writes `{hint}Date` too: the type it fills reads it through `Decodable`. A value of any length
  is read as it was. `tuple_struct` refuses an array of another length ahead of the slots, where
  the decoder knows its `count`. `tests/swift_tests/read.rs` pins the text, and
  `run_swift_refusals.rs` reads each place under the toolchain, judged and unjudged, against serde,
  with the fixed arrays and the tuples of `run_dart_refusals.rs`.
- **A name serde writes is written escaped wherever it sits between quotes.** `swift_quoted` writes
  it as a Swift string literal through `swift_literal_text`, a `"` and a `\` escaped: a member's
  key in `CodingKeys`, a struct's own tag key and name, a variant's name, an enum's tag key and
  content key, a plain enum's value, and an alias (`aliases::case_names`). A name with no such
  character is written as it was. `tests/swift_tests/quoted.rs` pins the text over the models of
  `tests/dart_tests/quoted_models.rs`, and `run_swift_quoted.rs` reads and writes back each
  payload under the toolchain, against serde.
- **A name the emitter has not written is read as a model**, as `validate()` calls it: a type
  declared below its holder, and a type of the consumer's own, which declares
  `init(swiftSchemaUnjudged:)` beside `validate()`.
- **A std sequence is the array serde writes.** `arrayed` in `src/features/swift.rs` reads a
  `VecDeque`, a `BinaryHeap`, a `HashSet`, a `BTreeSet` and an `IndexSet` as one array level, as
  `Vec` is read, through a sequence of sequences. `member_def` and the target of an alias call
  it, so the declaration, the read, the write and `validate()` read one def, and `swift_resolve`
  calls it for what no member is collected into: a type argument, a map's value, a tuple's
  position and a service's message. `tests/swift_tests/sequences.rs` holds the text of each
  shape to the text of its twin written with `Vec`, and `run_swift_sequences.rs` reads and writes
  each under the toolchain.
- **The tests.** `src/features/swift/read/tests.rs` holds each `Source` and `Presence`, and the
  README's block to what the emitter writes for the documents' parcel;
  `tests/swift_tests/read.rs` pins the text of each shape;
  `tests/service_schema_emitted_client_tests/run_swift_read.rs` compiles the emitted types with
  `-warnings-as-errors`, holds each read to the Rust `validate()` of the value and to the Rust
  read of the payload, an untagged enum to the variant serde takes, and each client to its fault.
  `an_absent_key_and_a_null_are_read_as_they_were` passed on the emitter before a read was
  written. The `validate()` runs read a payload through the unjudged initializer, and build in
  code the three values no read hands back. `tests/swift_tests/flattened.rs` pins the read of a
  flattened map alone in each shape, of a member that names a function at each place, and of
  each other member flattened after a struct, over the models of
  `tests/swift_tests/flattened_models.rs`, which also pin what serde reads of each, and
  `run_swift_flattened.rs` reads and writes each back under the toolchain, against serde, a holder
  whose every member is flattened among them.

### 12. The Swift `preprocess`

The read of a Swift type calls the functions the type and its members name, each on a payload,
ahead of the value read from it. A function is called under the name the attribute writes,
`func trimmed(_ payload: Any) -> Any`, on the Swift values of a JSON value: a `[String: Any]`, an
`[Any]`, a `String`, a `Bool`, an `Int64` for a number one holds, a `Double` for any other, and an
`NSNull`. The documents call the key a last resort, and a change keeps them saying so.
`README.md`'s "`preprocess` in the Swift client" section is the consumer's view; this is what a
change to the emitter has to keep.

- **The emitter is handed the type's functions.** `swift_suffix_tokens` puts `args.preprocess` in
  `SwiftTypeArgs::preprocess`, and a member's are in the meta `member_def` reads
  (`preprocess::functions`). Nothing reads the item's derives: the Swift type is read whether or
  not the Rust one derives `Deserialize`.
- **`src/features/swift/preprocess.rs` holds what is written.** `PAYLOAD` is the private
  `SwiftSchemaPayload`, written into each type that names a function, into each that reads a
  member from its object without some keys, a flattened member or what a variant under a tag
  alone holds, and into a struct under its own tag, which quotes a tag of another value through
  it (`read::both` adds it where the statements or `Written::own` name it). It reads any JSON
  value from a `Decoder`, reads a type back from one through `JSONSerialization` and a
  `JSONDecoder` of its own, and writes the coding path of the value into each `DecodingError` of
  that read. An answer `JSONSerialization.isValidJSONObject` refuses
  is a `dataCorrupted`, never an exception.
- **A member's functions are called in the read the member had.** `preprocess::member` is the one
  writer, called from `decode_statement` (a field), `tuple_shape` (a slot of a tuple) and
  `VariantPayload::read` (the one slot of a variant). `Source::payload` answers the payload and
  the coding path of each source: `decode(SwiftSchemaPayload.self, forKey:)` under a key, which
  refuses an absent key as a plain `decode` does, a decoder of its own for a slot, and the
  decoder itself. `Source::absent_as_nil` asks an optional member for its key first, so no
  function is called for a key the payload does not hold. The value is then read as it was read:
  as its declared type, as its wire type with the conversion it had (`Plain::Wire`), or, where
  `validate()` walks it, through a private type written for it (`SwiftSchemaRead{Member}`),
  whose `init(from:)` holds the statements of `read::unjudged`. A model read from a corrected
  payload is therefore still read with no judgement.
- **A flattened `Option` of any type but a map is read as the Zod schema of its holder reads
  it.** `present_statement` hands it to `corrected_option`, which reads the type under the
  `Option` (`flattened_under_option`) through `preprocess::optional`: the payload as
  `Source::payload` answers it, then `do { ... } catch { ... }` around the read of the type from
  what the functions answer, `null` among it. The `catch` throws the read's error where
  `{member}Payload`, the object the functions were handed, holds one of the struct's keys
  (`preprocess::struct_keys`, what `TAKEN_KEYS` records whatever function the member names), or
  any key where none are known, and assigns `nil` otherwise. The tag and the content key of an
  enum stand for none of its variants' members, so an enum's keys are not known. Zod's union
  tries the type over the function's answer first, and its branch without the type takes an
  object of no key beyond its holder's own, so serde's refusal of `{"note":"n"}` and its `None`
  for a function that answers `null` are not followed, as the Kotlin read does not follow them.
  `corrected_field` writes it, handed the keys as `absent`.
  `tests/swift_tests/flattened_options.rs` pins the text of `Veering` and of `Reeved`, `Wormed`
  and `Moused` of `tests/kotlin_tests/corrected_options_models.rs`, and
  `run_swift_flattened_options.rs` holds the Swift read of each payload of `CORRECTED` to Zod's.
- **A flattened externally tagged enum under a member's function reads a unit variant's entry
  over `null`**, as the Zod schema of its holder reads it and serde writes it. `corrected_field`
  hands `preprocess::Member::units` the enum's unit variants (`units_of`, from the entries
  `preprocess::note_entries` records), and `unit_entry_read` reads what the functions answer,
  where it is an object of one such entry over `null`, as the name the enum's own read takes the
  variant under. `run_swift_flattened_enums.rs` reads `Yawing`, of
  `tests/kotlin_tests/flattened_models.rs`, against serde.
- **A type's functions are called by its own read.** With `Written::own`, `read::both` writes the
  statements into the private `init(swiftSchemaCorrected:)`, and `init(swiftSchemaUnjudged:)`
  hands the functions the payload, then reads `SwiftSchemaRead` from what they answer, a private
  type whose `init(from:)` calls that initializer. Every caller of `both` passes the type's own
  functions, so every shape takes the key: a struct, a unit struct, a brand's wrapper, a tuple
  struct, an enum under each tagging and a plain one, which then writes its read as one that
  names a check does. `JUDGED` does not move: `init(from:)` judges what the functions answered.
- **A type written inside another names its owner.** In a private type nested in `Parcel`, `Self`
  is the nested type, so `both` writes each member's type through `naming_its_owner`.
- **A function is handed the keys no type around its own took**, as the Rust read hands them.
  `Source::Beside` is the decoder of a type that reads keys of its own beside the value: a
  flattened member (`decode_statement`, with the wire names of the struct's own members and the
  keys each struct flattened ahead of it took), a
  payload under a tag alone (the tag) and an untagged one. It holds the keys as a Swift
  expression, `taken + ["dock"]` (`taken_with`). A type whose statements name `taken`
  (`names_taken`) has its own read twice: `init(swiftSchemaUnjudged:swiftSchemaWithout:)`, and
  `init(swiftSchemaUnjudged:)`, which calls it with no key. A type that names a function on
  itself always has both, and removes the keys from the payload it hands its functions
  (`SwiftSchemaPayload(from:without:)`); the statements of its `init(swiftSchemaCorrected:)` then
  read `let taken: [String] = []`.
- **The registry says which type takes the keys.** `preprocess::note` records each written type
  whose reads hold the taking one (`read::takes_keys`), an alias of one included, and
  `preprocess::beside` is what a holder asks. A flattened member, the slot of a variant read
  from the enum's decoder and the value of a brand are read through that initializer where the
  type takes keys, and through the read they had everywhere else, so a type that names no
  function and reads none is written as it was, a flattened member its holder withholds keys from
  and the slot of a variant under a tag alone apart. A struct written for a variant's members
  answers through `VariantPayload::takes_keys`. A type declared below its holder is not known
  to take keys, and is read from the holder's whole object, or, flattened where its holder
  withholds keys and as the slot of a variant under a tag alone, from the payload without them.
- **An untagged enum hands its readers the keys.** A `swiftSchemaRead{Variant}` whose statements
  name `taken` takes it as a second parameter, and `Written::beside` hands `both` the readers,
  which name the payload type and the key type the enum declares ahead of them.
- **Names.** `swiftSchemaCorrected`, `swiftSchemaWithout`, `SwiftSchemaPayload`, `SwiftSchemaRead`
  and `SwiftSchemaRead{Member}` are the ones this adds. A local is `{member}Payload`,
  `{member}Decoder` for a slot, and `wire{Member}` where the member is converted.
- **The tests.** `src/features/swift/preprocess/tests.rs` holds the call, each source and the
  registry; `tests/swift_tests/preprocess.rs` pins the text of each shape, and that a type with
  no function is written as it was;
  `tests/service_schema_emitted_client_tests/run_swift_preprocess.rs` compiles the emitted types
  with `-warnings-as-errors` and holds what they read to `SAMPLE` and `POSITIONS` of
  `tests/pattern_preprocess_tests/models.rs`, the table the Rust read and the Zod schemas are
  held to, to the Rust read of each payload of its own models, and each client to its reply.

- **A function named after a member of the type is called through a forwarder.** A check or a
  `preprocess` whose name is that of a property, a case, `validate`, `encode` or `value` of the
  emitted type is read by Swift as the member. `swift/forwarded.rs` records the names a member may
  be written as and, where a function meets one, declares a `private func` ahead of the item
  (`_parcelCheckLabel(_ swiftSchemaChecked: Parcel) -> [String]`, or
  `_parcelPreprocessNote(_ swiftSchemaPayload: Any) -> Any`) that names the module's function.
  A check on a member is typed by the member's Swift type, and carries the type's parameters it
  names. A function that meets no member is called by its own name.
