### `validate()` in the Kotlin client

Every type `kotlin_definition()` writes carries `fun validate(): List<String>`: a data class, the `object` of a unit struct, the value class of a brand, a tuple struct, a sealed interface with each of its variants, and a plain enum. It answers the messages Rust's [`validate()`](validation.md#the-validate-method) answers for the same value, in the same order, and an empty list for a value that keeps every rule. A type with no rule answers `emptyList()`.

The serializer of a type calls it on the value it read: [the read refuses what `validate()` refuses](#the-read-in-the-kotlin-client). You call it yourself on a value built in code, and on one changed after it was read.

This is what the `Parcel` and the `Manifest` of [Checks](checks.md#checks-check) emit. The emitter writes each type on one line, and each line break below stands for one space of it:

```kotlin
@OptIn(ExperimentalSerializationApi::class) @Serializable(with = ParcelSerializer::class) @KeepGeneratedSerializer
data class Parcel(val label: String, val itemCodeList: List<String>, val fragileItemCodes: List<String>) {
  fun validate(): List<String> {
    val violations = mutableListOf<String>();
    if (label.codePointCount(0, label.length) < 1) violations.add("'label': too short: minimum length is 1, got ${label.codePointCount(0, label.length)}");
    violations.addAll(withoutDuplicates(itemCodeList).map { kotlinSchemaUnder("itemCodeList", it) });
    violations.addAll(withinParcel(this));
    return violations
  };
  private fun kotlinSchemaUnder(place: String, violation: String): String {
    val end = violation.indexOf('\'', 1);
    return if (violation.startsWith("'") && end > 0) { "'$place.${violation.substring(1, end)}'${violation.substring(end + 1)}" } else { "'$place': $violation" }
  }
}
@OptIn(ExperimentalSerializationApi::class) object ParcelUnjudged : KSerializer<Parcel> by Parcel.generatedSerializer()
object ParcelSerializer : KSerializer<Parcel>, () -> KSerializer<Parcel> {
  override fun invoke(): KSerializer<Parcel> = ParcelUnjudged;
  override val descriptor: SerialDescriptor = ParcelUnjudged.descriptor;
  override fun serialize(encoder: Encoder, value: Parcel) = ParcelUnjudged.serialize(encoder, value);
  override fun deserialize(decoder: Decoder): Parcel {
    val read = ParcelUnjudged.deserialize(decoder);
    val violations = read.validate();
    if (violations.isNotEmpty()) throw SerializationException(violations.joinToString("; "), IllegalArgumentException("validate()"));
    return read
  }
}

@OptIn(ExperimentalSerializationApi::class) @Serializable(with = ManifestSerializer::class) @KeepGeneratedSerializer
data class Manifest(@Serializable(with = ParcelUnjudged::class) val parcel: Parcel) {
  fun validate(): List<String> {
    val violations = mutableListOf<String>();
    violations.addAll(parcel.validate().map { kotlinSchemaUnder("parcel", it) });
    return violations
  };
  private fun kotlinSchemaUnder(place: String, violation: String): String {
    val end = violation.indexOf('\'', 1);
    return if (violation.startsWith("'") && end > 0) { "'$place.${violation.substring(1, end)}'${violation.substring(end + 1)}" } else { "'$place': $violation" }
  }
}
@OptIn(ExperimentalSerializationApi::class) object ManifestUnjudged : KSerializer<Manifest> by Manifest.generatedSerializer()
object ManifestSerializer : KSerializer<Manifest>, () -> KSerializer<Manifest> {
  override fun invoke(): KSerializer<Manifest> = ManifestUnjudged;
  override val descriptor: SerialDescriptor = ManifestUnjudged.descriptor;
  override fun serialize(encoder: Encoder, value: Manifest) = ManifestUnjudged.serialize(encoder, value);
  override fun deserialize(decoder: Decoder): Manifest {
    val read = ManifestUnjudged.deserialize(decoder);
    val violations = read.validate();
    if (violations.isNotEmpty()) throw SerializationException(violations.joinToString("; "), IllegalArgumentException("validate()"));
    return read
  }
}
```

The `validate()` of the parcel `{"label":"","itemCodeList":["x","x"],"fragileItemCodes":["y"]}` answers the three lines Rust answers, and that of a manifest that holds a parcel with `"y"` outside its codes answers the fourth:

```text
'label': too short: minimum length is 1, got 0
'itemCodeList': duplicate item code x
'fragileItemCodes.0': y is outside of itemCodeList
'parcel.fragileItemCodes.1': y is outside of itemCodeList
```

- **The order is Rust's.** Each member in the order declared: what the value it holds reports, then its bounds, then the checks it names. The type's own checks run last. Every rule runs, and none stops another.
- **A member with a bound of its own** is asked for what its type declares too, its own bound after the type's, as in Rust.
- **Every function a type calls is in the type's own body.** `kotlinSchemaUnder` writes a holder's key into a message, and is written into each type that names a place. The emitted types are joined into one file, so no function is written beside a type, and tixschema ships no runtime library. The two serializers of a type are the only declarations beside it, each under the type's own name.
- **A member named `validate` builds and keeps its key**, a property and a method being two names to Kotlin. A member named `violations` is read as `this.violations`.
- **Every build with the `kotlin` feature writes the same `validate()`.** The emitter reads the declaration itself, and writes each bound a field declares into the Kotlin client whatever the other features.

#### The bounds

| Key | On | Kotlin writes |
|---|---|---|
| `minLength`, `maxLength` | a string | `value.codePointCount(0, value.length)`: Unicode code points |
| `minLength`, `maxLength` | a list | `value.size` |
| `minimum`, `maximum` | a number | `value < 1u`, `value > 120.0`: a comparison in the number's own type |
| `pattern`, and the second pattern an enum writes | a string | `Regex("...").containsMatchIn(value)`: a match anywhere |
| [`literal`](literals-and-type-names.md#literal-values) | a string, a number or a `Boolean` | `value != 214u`, `value != "fixed"`, `!value`: in the number's own type, and on each item of a list, under its index |
| none | a `HashSet`, a `BTreeSet` or an `IndexSet` | `value.toSet().size != value.size`, ahead of the set's count: two equal items repeat one, and the message is [`repeats an item: a set holds each item once`](models.md#a-set-holds-each-item-once) |

- **A length is counted in Unicode code points**, so `"é"` and a character outside the Basic Multilingual Plane each count one. The Rust validator counts the bytes of the UTF-8 text (`len()`) and the Zod schema counts UTF-16 code units (`length`), so the three agree on ASCII text and can differ on any other.
- **A nullable member is measured only where it holds a value**: `spareLabel?.let { held -> ... }`.
- **A number is compared in its own type.** An unsigned one is compared with an unsigned literal (`1u`), a `Float` with a `Float` (`0.5f`), a `Double` with a `Double`. A bound that is no value of the type, `minimum = 0.5` on a `u32`, is compared as a `Double`.
- **A float is quoted back as Rust writes it.** Kotlin prints `150.0` and `2.5E8` where Rust prints `150` and `250000000`, so a type that bounds a float carries `kotlinSchemaShown`, which writes a whole float without `.0` and any float without an exponent.
- **A pattern is written for the JVM, and stated as you wrote it.** The JVM reads `$` as the end of the text or the place before a last line break, `\v` as any vertical space, and refuses a named group whose name holds `_`. The emitted regex writes `$` as `\z`, `\v` as `\x0B` and a named group as a plain one, which is what the `regex` crate reads in the three. The message still states the pattern of the declaration: `does not match pattern '^[a-z]+$'`.

#### What a holder runs

A holder calls the `validate()` of each value of a declared type it holds, and writes each message under the place the payload holds the value at:

```kotlin
violations.addAll(firstParcel.validate().map { kotlinSchemaUnder("firstParcel", it) });
parcelList.forEachIndexed { index, held -> violations.addAll(held.validate().map { kotlinSchemaUnder("parcelList.$index", it) }) };
spareParcel?.let { held -> violations.addAll(held.validate().map { kotlinSchemaUnder("spareParcel", it) }) };
for ((key, held) in parcelsByDock) { violations.addAll(held.validate().map { kotlinSchemaUnder("parcelsByDock.$key", it) }) }
```

- **A list** names each item by its index at each level, and **a tuple** written as a field's type each position: `'parcelGrid.0.1.label'`, `'deckPair.1.label'`.
- **A map** is walked in the order it holds its entries, and names each by its key as serde writes it. A key that is a string, a number or a `Boolean` is written as it is. A key of a declared type is written as its serializer writes it, so a plain enum goes by the name serde writes the variant under: `'byWing.north.label'`.
- **A `#[serde(flatten)]` member** writes no key, and what it reports is written under no name. A flattened map names each entry by its key alone.
- **A value behind a type parameter is not reached**, as in Rust. A key typed with a type parameter is written where the Rust type declares `Serialize` for the parameter, as the serializer of the key's class writes it (`'byKey.north-dock.itemLabel'` for a plain enum under `rename_all`, the string of a brand), and left out where the type declares none. A key whose class declares a type parameter of its own, a generic brand, has no serializer to find from the key alone and is written as Kotlin prints it.
- **A type the emitter has not written** when it writes the holder has no `validate()` the holder can count on: a type of your own you supply to Kotlin, an alias of a string or of a list, a type declared below. The holder then carries `private fun Any?.validate(): List<String> = emptyList()` in its own body. A member is found ahead of an extension, so a type that has `validate()` answers with its own.

#### A bound on a field typed with a declared type

A length or a pattern on [a field typed with a branded string](validation.md#a-field-typed-with-a-branded-string) is run on the string the type holds. The value class of a brand over a string or a path carries `fun kotlinSchemaText(): String = value`, and a brand over such a brand answers through the brand it holds. The holder asks the field for its text, and carries the answer of a string in its own body:

```kotlin
if (slug.kotlinSchemaText().codePointCount(0, slug.kotlinSchemaText().length) < 3) violations.add("'slug': too short: minimum length is 3, got ${slug.kotlinSchemaText().codePointCount(0, slug.kotlinSchemaText().length)}");
private fun String.kotlinSchemaText(): String = this
```

A member is found ahead of an extension, so a brand answers with its own text and an alias of a string with the string. The brand and the alias may be declared above the holder or below it. A field typed with a type that holds no string does not compile in Kotlin, at the holder: `error: candidate 'fun String.kotlinSchemaText(): String' is inapplicable because of a receiver type mismatch.` A brand over a brand answers through the brand it holds where that brand is declared above it.

A string bound on a field typed with a brand over a brand measures the text of the string at the bottom, in Rust and in Kotlin alike, where the inner brand is declared above the outer one; with the inner brand declared below it, the field is refused.

#### A brand

The value class of a brand runs the brand's own `minLength`, `maxLength` and `pattern` over `value`, then the checks the brand names, and names no place: `too short: minimum length is 2, got 1`. A brand over a list counts its items. A brand with such a bound carries `fun kotlinSchemaBound(): List<String>` beside `validate()`: what the bounds alone answer, the checks apart, which is what [an untagged enum chooses a variant by](#the-read-in-the-kotlin-client).

A brand that holds one of its type parameters bare runs its bounds where the value is of the type declared for the parameter, and at no other filling: `if (value is String) { ... }`. Where the declared type is itself a brand written with arguments, `default_types(WrapType = StrictDocumentId<String>)`, the JVM keeps no argument to tell the filling by, so the bounds run where the value is a `StrictDocumentId<*>` that holds a `String`, the type the declared default gives its parameter: `if (value is StrictDocumentId<*>) { val kotlinSchemaHeld = value.value; if (kotlinSchemaHeld is String) { ... } }`. That is the one difference from Rust, which runs the bounds at exactly the declared default: a `StrictDocumentId<CharSequence>` built by hand, which holds a `String`, is judged as well. A `StrictDocumentId<UInt>` and a `WrapType` filled with a `String` are not.

A brand whose declared default is a list, `default_types(T = Vec<String>)` with a `minLength`, counts its items only at that filling, and the JVM erases the item type, so the class holds no test of it: a holder passes the type it writes, `carried.kotlinSchemaAtFilling("List<String>")`, the brand's own serializer passes the default where the serializer it was handed for the parameter has the default's descriptor, and the class counts when it is told the default. A `Carried<List<Int>>` is then not counted in a holder or in a read of the brand, as in Rust. A `Carried(emptyList<String>())` built in Kotlin and checked with its own `validate()` is not counted either; a holder, a read of the brand and the Rust side count it.

#### The name of a slot and of a variant's member

A message names a value by the steps serde writes on the way to it: a key, a position in an array, or nothing where serde writes the value itself.

| Member | serde writes it | The name |
|---|---|---|
| a slot of a tuple struct that declares several slots | at its position in the array, `0` for the one slot `#[serde(skip)]` leaves | `'1'` |
| the one slot of a tuple struct | as the value itself | none: the sentence alone, or the holder's key |
| a named member of a variant, no tag attribute | `{"Loose":{"freightLabel":…}}` | `'Loose.freightLabel'` |
| the one slot of a variant, no tag attribute | `{"Boxed":…}` | `'Boxed'`, and `'Boxed.label'` for a model it holds |
| a slot of a variant that declares several, no tag attribute | `{"Split":[…]}` | `'Split.1'`, and `'Split.0'` for the one slot `#[serde(skip)]` leaves |
| a named member, tag and content key | `{"kind":"Loose","held":{…}}` | `'held.haulLabel'` |
| the one slot, tag and content key | `{"kind":"Boxed","held":…}` | `'held'` |
| a slot of several, tag and content key | `{"kind":"Split","held":[…]}` | `'held.1'` |
| a named member, tag alone | beside the tag | `'liftLabel'` |
| the one slot, tag alone | its own keys beside the tag | none |
| a member of an `#[serde(untagged)]` variant | as the value itself, or under its key | none for the slot, its key for a named member |

The variant's step is the name serde writes it under: the variant's own `rename`, or the enum's `rename_all` applied to its ident. Each variant of a sealed interface overrides `validate()` with the rules of its own members, and the enum's own checks run last in each. A string check written on the enum reaches in Kotlin the fields [it reaches in Rust](enums.md#a-check-written-on-the-enum).

#### The checks

A check is called under the name the attribute writes, on the value Rust hands its function, and answers the messages as a list: empty where the value keeps the rule. A top-level function of the package, or one the file imports, is in scope.

| | Kotlin |
|---|---|
| A field's check | `fun withoutDuplicates(value: List<String>): List<String>` |
| A type's check | `fun withinParcel(value: Parcel): List<String>` |
| The check of a type with a parameter, written once | `fun <CodeType> withACode(value: Crate<CodeType>): List<String>` |

```kotlin
fun withoutDuplicates(value: List<String>): List<String> {
    val seen = mutableSetOf<String>()
    for (code in value) {
        if (!seen.add(code)) return listOf("duplicate item code $code")
    }
    return emptyList()
}

fun withinParcel(value: Parcel): List<String> =
    value.fragileItemCodes.withIndex()
        .filter { it.value !in value.itemCodeList }
        .map { "'fragileItemCodes.${it.index}': ${it.value} is outside of itemCodeList" }
```

A field's check names no path, and `validate()` writes its messages under the field. A type's check leads each message with the path of what it refuses, as [in Rust](checks.md#the-messages). The check of a nullable member is called only where the member holds a value, and the check of a brand takes the value class.

**A name with no Kotlin function does not compile**, and the Kotlin compiler says so at the call:

```text
error: unresolved reference 'withinParcel'.
```

#### The read in the Kotlin client

The serializer of a type throws for a payload whose value breaks a bound or a check, and hands no object back. `Json.decodeFromString<Parcel>(payload)`, `serializer<Parcel>()` and a property typed `Parcel` in a class of your own all read through `ParcelSerializer`, the type's own serializer: it reads the value and calls `validate()` on it. What it throws is `SerializationException`, the exception of `kotlinx.serialization`, and its message is what `validate()` answers of the value read, every message, joined by `; `:

```kotlin
Json.decodeFromString<Parcel>("""{"label":"","itemCodeList":["x"],"fragileItemCodes":["x","y"]}""")
// SerializationException: 'label': too short: minimum length is 1, got 0; 'fragileItemCodes.1': y is outside of itemCodeList
```

**Every type has two serializers**, a plain enum that names no check apart:

| | What it reads | Who names it |
|---|---|---|
| `{Type}Unjudged` | the payload, with no judgement | a holder, for each model it holds, and you, to read a value that breaks a rule |
| `{Type}Serializer` | through the first, and then throws what `validate()` answers | the type: `@Serializable(with = ParcelSerializer::class)` |

Both write the same payload, and writing judges nothing: a broken value built in code is written as it is. A type with parameters writes the two as classes over one serializer per parameter, `CrateUnjudged(serializer<String>())`. The type's own serializer is a function too: called, `ParcelSerializer()` answers `ParcelUnjudged`, which is how a read tixschema writes reaches the unjudged serializer of a type it holds but cannot name, one declared below it.

- **A class, the `object` of a unit struct and the value class of a brand keep the serializer the plugin writes**, with `@KeepGeneratedSerializer`, and `{Type}Unjudged` is that serializer under a name: `object ParcelUnjudged : KSerializer<Parcel> by Parcel.generatedSerializer()`. A struct under its own `#[serde(tag = "...")]` holds the tag in a property the plugin reads and writes after the struct's members, so its `{Type}Unjudged` wraps the plugin's serializer, refuses a payload whose tag is not the struct's name (below), and writes the tag ahead of the members, as serde writes it: `#[serde(tag = "kind")] struct Flag { label: String }` is written `{"kind":"Flag","label":"n"}`, alone and flattened into a holder. The annotation is an experimental one of `kotlinx.serialization`, so each such type carries `@OptIn(ExperimentalSerializationApi::class)`. This repository compiles the emitted source with `kotlinx.serialization` 1.11.0, the version `just kotlin-libs` installs.
- **A struct with a `#[serde(flatten)]` member, a struct with a member serde never writes, a tuple struct and an enum under every tagging** have a serializer tixschema writes, and it is the unjudged one. So does the class of a variant with a `#[serde(flatten)]` member or a member serde never writes, `{Enum}{Variant}Unjudged`, its one serializer: the enum's own serializer judges the variant.
- **Every refusal of a serializer tixschema writes is a `SerializationException`**, the exception the plugin's own serializers throw, so a caller that catches it catches each. Its message says what the read expected and found: ``missing field `held` `` for a key serde requires, `expected an object, found [1]` for a value of another kind, `expected an object of one key, found {}` for an externally tagged enum read from an object of no entry or of several, `unknown tag Gale` for a tag no variant has, and `invalid length 3, expected 2` for the array of a tuple, or of a fixed array, of another length, a longer one among them, as serde refuses each.
- **A refusal states the path of the value refused, from the root of the payload**, as kotlinx states a path: an `i64` item out of range is refused with `Failed to parse literal '-9223372036854775809' as a long value at path: $.listed[1]`, and the fixed array below with `invalid length 3, expected 2 at path: $.pair`. A serializer tixschema writes reads each part of the value it took off the decoder from the part's text, where kotlinx states the part's path, and places a refusal below it at the path the decoder read the value at, the part's step ahead. What it throws is a `SerializationException` over the refusal, its message the refusal's with that path and without the `Unexpected JSON token at offset N: ` kotlinx writes ahead of the words of a text it refuses. A map keyed by a [brand](generics-and-brands.md#branded-newtypes) is read through `class {Brand}MapSerializer<KotlinSchemaValue>`, which the brand declares when it holds a text, a number or a `bool` and the member names on the map, `val pitches: @Serializable(with = PitchMapSerializer::class) Map<@Serializable(with = PitchKeySerializer::class) Pitch, UInt>`: it reads each key from its text at the map's path, as serde names a key that does not read, and each value from its text under the key's text, where kotlinx would state the key as the brand's `toString()`, so `{"pitches":{"3":-1}}` is refused at `$.pitches.3`, and writes the map as kotlinx writes it.
- **A `char`, a `NaiveDate`, a `NaiveTime` and a `NaiveDateTime`, which Kotlin holds as a `String`, are read in their one form wherever they are held**: a field, an item of a list, a key and a value of a map, a type argument, a range's bound, an arm of a `Result`, a position of a tuple, a variant's value, the key of a flattened map, the value of a brand and the type an alias names. Each is read through a serializer of the type's own, `object KotlinText1 : KSerializer<String>`, which the property names, `val letter: @Serializable(with = KotlinText1::class) String`, so the property keeps its type, and which throws `SerializationException` for any other text. A `char` is one code point, as serde reads it: `"ab"` and `""` are refused, the first with `SerializationException: expected a character, found "ab"`, and `"😀"` is read. A date, a time and a date and time are read where the generated Zod schema reads them, `z.iso.date()`, `z.iso.time()` and `z.iso.datetime({ local: true })`: `"x"` is refused with `SerializationException: expected a date, found "x"`, and with `expected a time` and `expected a date and time` where it is read as the other two. Where serde and Zod part, the read follows Zod: `"2024-1-6"`, a year before 0000 or after 9999, `"+12024-01-06"`, which serde writes with its sign, and the leap second `"23:59:60"`, all of which serde reads, are refused, and `"2024-01-05T11:20:30Z"` and `"2024-01-05T11:20"`, which serde refuses, are read as a `NaiveDateTime`. A `DateTime<Tz>` without `as_number` is read through a serializer of its own as any text: Zod reads any text a JavaScript `Date` reads, which no Kotlin check follows, and serde reads an RFC 3339 date and time. A read tixschema writes, of a struct with a `#[serde(flatten)]` member or a member serde never writes, of a tuple and of a variant's value, reads as any text an alias declared below its holder, and a `char` or a date that fills the type parameter of an alias, or of a generic type declared below its holder. The generated service clients and the WebSocket dispatcher read a request, a reply and a header that is itself a `char` or a date, or a list or a map of one, in its one form as well: a reply of another is answered with the client's fault, `undeserializable-payload` from the `http_rest` client and `failed-validation` from the `ws_rpc` one, and a header of another with the dispatcher's `failed-validation`, the refusal in its `detail`. The same serializer writes each in [the form every target reads](client-forms.md#the-form-each-client-sends): a `NaiveDateTime` with a `Z`, and a `NaiveTime` or a `NaiveDateTime` without seconds, in the form serde writes, `"2024-01-05T11:20:30Z"` as `"2024-01-05T11:20:30"`, and a text no such rewrite turns into its form is refused with the `SerializationException` of the read. A `DateTime<Tz>` is written held to RFC 3339 with its zone, `"2024-01-05T10:20Z"` as `"2024-01-05T10:20:00Z"`, and any other text is refused with `SerializationException: expected a date and time with its zone, found "x"`. Kotlin holds the text as written, so a date of whole seconds, of a fraction of any length and of an offset crosses unchanged.
- **A number and a `bool` are read from an unquoted JSON value alone, wherever they are held**, as serde, TypeScript and the generated Zod schema read them: `kotlinx.serialization` reads `"7"` as a `Long`, an `Int`, a `Short`, a `Byte`, a `Double`, a `Float`, an unsigned integer and `"true"` as a `Boolean`, and each is read here through an `object KotlinNumber{N}` declared once for each item and type that throws `SerializationException` in serde's words, ``invalid type: string "7", expected i64`` and ``invalid type: string "true", expected a boolean``, naming the Rust type of the member. An integer's refuses a number written with a fraction or an exponent, or `-0`, as serde refuses the float it reads it as, ``invalid type: floating point `100.0`, expected i64`` for `1e2` and ``invalid type: floating point `-0.0`, expected u32`` for `-0`, where kotlinx reads `1e2` and `-0` as a `Long` and refuses each as an unsigned integer in words of its own, `Failed to parse type 'UInt' for input '1e2'`. It is named where the plugin reads, on a field, an item of a list, a map's value, a type argument, a range's bound, an arm of a `Result`, a position of a tuple and the value of a brand, and composed where tixschema writes the read. A map's key is read as it was, since JSON writes a number key between quotes, and a `DateTime<Tz>` under `as_number` is read as epoch milliseconds the same way.
- **A fixed array `[T; N]` is read from an array of `N` items alone, at each level**, as serde and the generated Zod schema (`z.array(...).length(N)`) read it, wherever it is held: a field, beside a `#[serde(flatten)]` member, an item of a list, a map's value, under an `Option`, a position of a tuple, a variant's value, a type argument, the fixed array of a type parameter and the type an alias names. Each level is read through a class of the type's own, `class KotlinArray1<Item>(item: KSerializer<Item>) : KSerializer<List<Item>>`, which the property names, `val pair: @Serializable(with = KotlinArray1::class) List<UByte>`, so the property keeps its type, and which throws `SerializationException` for an array of another length: with `struct Clenching { pair: [u8; 2] }`, `{"pair":[1,2,3]}` is refused with `SerializationException: invalid length 3, expected 2 at path: $.pair` and `{"pair":[1]}` with `invalid length 1, expected 2 at path: $.pair`, and a `[[u8; 2]; 3]` refuses an inner array of another length in the same words, at the inner array's path, through the type's own serializer and its unjudged one. A list, `Vec<u8>`, is read at any length. A read tixschema writes, of a struct with a `#[serde(flatten)]` member or a member serde never writes, of a tuple and of a variant's value, reads at any length a fixed array an alias declared below its holder names, and one that fills the type parameter of an alias, or of a generic type declared below its holder, as it reads any text there.
- **A struct under its own `#[serde(tag = "...")]` reads a payload whose tag is the struct's name, and refuses any other**, as TypeScript and the generated Zod schema refuse it, whose tag is the literal of that name. serde reads the struct whatever the tag holds, and where it is absent, and writes the name back. With `#[serde(tag = "kind")] struct Flag { label: String }`, `{"kind":"Flag","label":"n"}` reads, `{"label":"n"}` is refused with ``SerializationException: missing field `kind` ``, `{"kind":"Other","label":"n"}` with `SerializationException: expected tag "Flag", found "Other"`, and `{"kind":5,"label":"n"}` with `SerializationException: expected tag "Flag", found 5`, where serde reads each as `Flag { label: "n" }`. A struct with a `#[serde(flatten)]` member reads its tag the same way, and so does a struct flattened into a holder, from the object the holder hands it, so the holder refuses the same payloads.
- **A member of a struct or of an enum's variant under `#[serde(skip_serializing)]` is read, and written into no object**, as serde writes it into none, and a flattened one merges nothing into its holder's. A flattened one that is no `Option` is read as any flattened member is, its `default` aside, as serde reads it: a payload without a key its struct requires, or with an entry its map does not read, is refused. The plugin writes every property, so a struct that holds such a member is written by the serializer tixschema writes, and so is the class of a variant that holds one, under every tagging. A member under `skip_serializing_if` is written wherever it holds a value: the serializer cannot call the Rust function, so under any function but `Option::is_none`, `Vec::is_empty` say, it writes the key where serde leaves it out.
- **The serializer tixschema writes for a struct or a variant's class refuses a key no member reads**, as TypeScript, Zod, serde and the plugin's own serializer refuse it: with `struct Furling { label: String, #[serde(default, skip_serializing)] spare: Option<u32> }`, `{"label":"north","other":1}` is refused with `SerializationException: Encountered an unknown key 'other'`. A `Json { ignoreUnknownKeys = true }` reads past the key, as it does in a class the plugin reads, and so does the class of a [`nonstrict`](models.md#unknown-keys-strict-reads-and-nonstrict) type, annotated `@JsonIgnoreUnknownKeys`, whose serializer refuses nothing. The keys the members read are each member's name and alias, the struct's own tag, every key a flattened struct tixschema wrote reads, its own tag and a member under `#[serde(skip_deserializing)]` among them, and the one entry a flattened externally tagged enum takes: a second variant's entry is refused. A payload serde writes holds no other key. A model held at any depth inside a flattened struct whose keys tixschema knows, or inside the entry of a flattened externally tagged enum, refuses such a key too, as it does held under a key: the member is read under the `Json` the read was asked with, from the keys it reads alone. With `struct Thimbling { held: Ringbolt, label: String, rail: Pinrail }` flattened beside `note: String`, over `Pinrail { ringbolt: Ringbolt }` and `Ringbolt { label: String }`, `{"note":"a","held":{"label":"x"},"label":"n","rail":{"ringbolt":{"label":"y","other":1}}}` is refused with the `SerializationException` of the plugin's serializer, whose message opens `Encountered an unknown key 'other' at path: $`. Beside a flattened map, and beside a flattened member whose keys tixschema does not know, no key is refused, and every model inside such a member reads past one, where TypeScript and Zod refuse one none of the members reads: a struct with a `#[serde(flatten)]` member of its own, a type or a member that names a [`preprocess`](#preprocess-in-the-kotlin-client), an enum under a tag key or untagged, and a type declared below its holder, in another crate, an alias or a type parameter. The serializer of an enum under a tag key refuses a key beside its tag and its content, as TypeScript and Zod refuse it: under `#[serde(tag = "kind", content = "load")]`, `{"kind":"Gust","load":{"speed":1},"other":1}` and `{"kind":"Lull","other":1}` are refused, and under `#[serde(tag = "kind")]` a unit variant, `{"kind":"Lull","other":1}`, as a variant of members is by the class that reads it. A lenient `Json` reads past each.
- **A `#[serde(flatten)]` member is read at its place, in the order declared**, as serde reads it: from the object without the keys the holder's own members take and those each member flattened ahead of it took. A struct, and an enum under a tag key, declared above its holder that hands its object to no function is read from the whole object, as TypeScript and Zod read it; a key it reads that a member of its holder's own, a struct flattened ahead of it or the tag of the variant that flattens it reads too is refused at expansion ([Order and strictness of flattened members](flatten.md#order-and-strictness-of-flattened-members)). A struct that hands its object to a function, one that names a [`preprocess`](#preprocess-in-the-kotlin-client) on itself or flattens a member that names one, is handed the object without those keys, as serde hands it. An optional one looks for its keys in the object it is read from: it is `null` where none of them is there, and is read where one is, a key that does not read refusing the payload, as TypeScript and Zod refuse it, where serde reads `None`. A struct takes the keys serde reads it from: the name and each alias of every member it reads, one serde never writes among them. Its own `#[serde(tag = "...")]` and a member under `#[serde(skip_deserializing)]` are not among them, so a member flattened after it is handed both, as serde hands them, and an optional one whose tag alone is in the object is `null`. A struct that names a [`preprocess`](#preprocess-in-the-kotlin-client) on itself, a flattened member that names one and a struct with a `#[serde(flatten)]` member of its own take no key, as serde reads each. An optional struct with a `#[serde(flatten)]` member of its own has no key to be looked for by: it is read wherever its read succeeds and is `null` wherever that read throws, as serde reads `None`. With `struct Crowning { note: String, #[serde(flatten)] tabling: Tabling }` over a `Tabling` that reads `label`, `#[serde(flatten, default, skip_serializing_if = "Option::is_none")] crowning: Option<Crowning>` reads `{"label":"north","note":"n"}` as that `Crowning`, written back as `{"note":"n","label":"north"}`, and reads `{}`, `{"note":"n"}` and `{"label":5,"note":"n"}` as `null`, written back as `{}`. A struct declared below its holder, or in another crate, is not known to take any: a member flattened after it is read from its keys as well, where serde reads it without them, and an optional one is looked for by the properties its serializer declares, its tag and a member serde never reads among them and no alias. Where its serializer declares none, as the serializer tixschema writes for a struct with a `#[serde(flatten)]` member of its own declares none, the optional one whose member names no `preprocess` has no key to be looked for by, and is read wherever its read succeeds and is `null` wherever that read throws, as serde reads `None`: with `Crowning` declared below its holder, `{"label":"north","note":"n"}` reads that `Crowning`, and `{"note":"n"}` reads `null`, where TypeScript and Zod refuse it. Where the read throws for a value that reads and breaks a rule, the payload is refused with the rule's message, as serde, TypeScript and Zod refuse it: the read is tried again through the unjudged serializer the type's own answers when called, and the refusal stands where that read succeeds. With `struct Gromet { #[serde(flatten)] earing: Earing, #[model_schema_prop(minLength = 2)] note: String }` declared below its holder, `{"label":"north","note":"n"}` is refused with `SerializationException: 'note': too short: minimum length is 2, got 1`, and `{"label":5,"note":"nn"}` reads `null`. A type of your own is read as before. An optional untagged enum is read the same way, and so is an enum under a tag key declared below its holder or that names a [`preprocess`](#preprocess-in-the-kotlin-client) on itself, and an alias or a type parameter that names a type whose serializer declares no property. A type parameter filled at a struct or an enum under a tag key that tixschema wrote above the type that flattens an `Option` of it is looked for by that type's keys, told by the name its serializer declares, where the serializer declares no property: with `struct Waxing<Held> { #[serde(flatten)] annotation: Notation, #[serde(flatten)] held: Option<Held> }` and `#[serde(tag = "kind")] enum Waxwork { Plain, Red { shade: String } }` above it, `Waxing<Waxwork>` reads `{"note":"n"}` with `held` `null` and refuses `{"kind":"Gale","note":"n"}`, as TypeScript and Zod refuse it, where serde reads `None`. An optional one whose member names a [`preprocess`](#preprocess-in-the-kotlin-client) is read as the Zod schema of its holder reads it: its functions are called first, on the object the member is handed, and the type is read from what they answer. Where that read throws, it is `null` where that object holds none of the type's keys, or no key at all where those keys are not known, and the payload is refused where the object holds one. With `struct Reeved { note: String, #[model_schema_prop(preprocess = ["asGiven"])] #[serde(flatten, default, skip_serializing_if = "Option::is_none")] tholepin: Option<Tholepin> }` over a `Tholepin` that reads `label`, and `fun asGiven(payload: JsonElement): JsonElement = payload`, `{"label":"north","note":"n"}` reads that `Tholepin`, `{"note":"n"}` reads `null`, where serde refuses it with `missing field`, and `{"label":5,"note":"n"}` is refused. Under a function that answers `JsonNull`, `{"label":"north","note":"n"}` is refused, as Zod refuses it, where serde reads `None`. Beside a `nonstrict` holder, whose Zod schema reads a flattened `Option` as absent wherever its type does not read, as serde reads `None`, an optional member whose keys are there is `null` where its read throws, a tag, an externally tagged enum's entry and an `ObjectId` that do not read among them, and the holder reads past those keys; one whose member names a `preprocess` is `null` exactly where its functions answer `JsonNull`, and is read from any other answer, so `Reeved` marked `#[model_schema(nonstrict)]` refuses `{"note":"n"}` and `{"label":5,"note":"n"}`, as serde refuses them, and under a function that answers `JsonNull` reads `{"label":"north","note":"n"}` with `tholepin` `null`. Beside a `nonstrict` holder a member that names a `preprocess`, under an `Option` or not, is read from the whole answer under the caller's `Json`, so its type refuses a key it does not declare, as serde refuses it: `{"label":"north","note":"n","stray":1}` is refused under `asGiven`. A map is the [one member of its object](flatten.md#intersection-types-serdeflatten), and is read from every key of it. A key Kotlin holds as no `String`, a plain enum or a [brand](generics-and-brands.md#branded-newtypes) among them, is written as the text its own serializer writes and read back through that serializer: with `enum Chock { Port, Starboard }`, `#[serde(flatten)] lines: BTreeMap<Chock, u8>` reads `{"Port":2}` and writes it back as it was, and refuses `{"Bow":2}` with `SerializationException`, as serde refuses it, a variant under its `rename` or the enum's `rename_all` read under the name serde reads. A map keyed by a number or a `bool` [does not build](flatten.md#intersection-types-serdeflatten) flattened, and a `char` or a date key is read in its one form, as every `char` and date is (above). An optional map is refused at expansion, as serde reads its `None` back as a map of no entry ([Order and strictness of flattened members](flatten.md#order-and-strictness-of-flattened-members)). Under a [`preprocess`](#preprocess-in-the-kotlin-client) it is read from what the function answers, as serde reads it: `null` is `null`, and an entry that does not read as its value is refused. A value inside a flattened member that the read refuses is refused at the path of its holder, as serde names the holder: with `pub struct Frame { #[serde(flatten)] pub beam: Beam, pub span: u32 }` over `pub struct Beam { pub oak: u32 }` held under `frame`, `{"frame":{"span":1,"oak":"x"}}` is refused at `$.frame`, and a key the member misses or a key it refuses keeps the words that name it, ``Field 'oak' is required`` at `$.frame`.
- **A plain enum under `#[serde(flatten)]` declared below its holder is read from the one entry named for its variant, over `null`, and written as that entry**, as serde reads and writes it. The holder is written before the enum is, so it cannot tell a plain enum from a struct: where the serializer of the member is an enum's, the one tixschema writes for an enum with an `alias` included, the holder hands it the key of the one entry whose key it reads. With `pub struct Casement { pub width: u32, #[serde(flatten)] pub weathering: Weathering }` over `pub enum Weathering { Sound, Worn }` declared below it, `{"Sound":null,"width":1}` reads `Sound` into the member and is written back as it was, and so are a variant read under its `alias`, the enum under a `rename_all`, the enum optional, and the member ahead of the holder's own. A payload with no entry named for a variant is refused with `SerializationException` and `no variant of enum Weathering found in flattened data`, one whose entry holds a value other than `null` with `variant sound of enum Weathering is read over null`, and one with entries under two variants' names with `more than one variant of enum Weathering found in flattened data: sound, worn`, where serde refuses the first two and reads the first of the third. A plain enum declared above its holder is refused by the macro, as [Intersection Types](flatten.md#intersection-types-serdeflatten) says.
- **A variant reads and writes a `#[serde(flatten)]` member at its own level**, as serde does: with no tag attribute, under a tag and a content key, untagged, and under a tag alone, where a struct is flattened beside the tag and a map [stops the build](flatten.md#intersection-types-serdeflatten). The variant is read, as a struct with such a member is, from the object its enum hands it: its entry, its content, the whole payload under a tag alone, the tag in it, so a struct it flattens that reads a key named as the tag reads it, as TypeScript and Zod read it, where serde hands the variant the object without the tag, or the whole payload untagged. It is written into that object. Over a `Tabling` that reads `label`, `#[model_schema(nonstrict)] enum Gaffing { Counted { #[serde(flatten)] counts: BTreeMap<String, u8> }, Labelled { #[serde(flatten)] tabling: Tabling, note: String } }`, `nonstrict` as the build asks of an enum whose variant flattens a map beside another struct variant, reads `{"Counted":{"north":2}}` as the count `north` and writes it back as it was, and reads `{"Labelled":{"note":"a","label":"n"}}` as the label `n` and the note `a`, written back `{"Labelled":{"label":"n","note":"a"}}`. A member of such a variant is read under each of its aliases and through each of its [`preprocess`](#preprocess-in-the-kotlin-client) functions, as a struct's is.
- **A flattened externally tagged enum takes one entry**, as serde reads it: the first, in the object's order, under the name a variant is read under, its `rename` or the enum's `rename_all` included, or an alias of one, and every member flattened after it is handed the object without that entry. With `enum Hatching { Bay { label: String }, Dock { label: String } }` flattened beside `note: String`, `{"note":"a","Bay":{"label":"n"}}` reads `Bay` and the note `a`, and `{"note":"a","Bay":{"label":"n"},"Dock":{"label":"s"}}` is refused with `SerializationException: more than one variant of enum Hatching found in flattened data: Bay, Dock`, each variant named once, in the order the enum declares them, a variant's entry beside one under its alias naming that variant alone, as TypeScript and Zod refuse it, where serde reads `Bay` and reads past `Dock`. Flattened ahead of a member that names a function, `{"Bay":{"label":"n"},"north":2}` reads `Bay`, and the function is handed `north`. A payload with no such entry is refused with `SerializationException`, and an optional enum reads `null` from it. The enum is written as serde writes it, its variant's entry beside the holder's members, and a unit variant as its name over `null`: with `Calm` among `Hatching`'s variants, the holder writes `{"note":"a","Calm":null}`, the enum optional or under a `preprocess` as well. An enum under a tag key alone and an untagged enum take no key, as serde reads each, and so does an enum that names a `preprocess` on itself or on its member. An enum under a tag and a content key takes both, as serde reads it as a struct of the two: a member flattened after it that names a function is handed neither. An optional enum under a tag key, whose member names no `preprocess`, is looked for by its tag and its content key in the object it is handed: it is `null` where neither is there, and is read where one is, so a tag or a content that does not read refuses the payload, as TypeScript and Zod refuse it, where serde reads `None`. With `#[serde(tag = "kind")] enum Hitched { Bay { label: String } }` flattened beside `note: String`, `{"kind":"Bay","label":"n","note":"a"}` reads `Bay`, `{"kind":"Bay","note":"a"}` is refused, and `{"note":"a"}` reads `null`, and so does `{"label":"n","note":"a"}`, a variant's member with no tag, as serde reads it, where TypeScript and Zod refuse it. Under `#[serde(tag = "kind", content = "held")]`, `{"held":{"label":"n"},"note":"a"}`, a content with no tag, is refused as well. The refusal is what the enum's read throws, a `SerializationException`: under a tag and a content key, ``missing field `kind` `` for a content with no tag and ``missing field `held` `` for a tag with no content. An optional untagged enum, and one under a tag key declared below its holder or that names a `preprocess` on itself, has no key to be looked for by: it is read wherever its read succeeds and is `null` wherever that read throws, as serde reads `None`. Where the member of an optional enum under a tag key, or untagged, names a `preprocess`, it is read from what the functions answer, as an optional struct that names one is, and has no key to be looked for by: with `asGiven`, `{"kind":"Bay","label":"n","note":"a"}` reads `Bay`, `{"note":"a"}` reads `null`, and `{"kind":"Bay","note":"a"}` and `{"label":"n","note":"a"}` are refused, as Zod reads each, where serde refuses all three. An enum declared below its holder, or in another crate, is not known to take an entry: it is read from the object it is handed, and is refused where that object holds any other key, and the write of its unit variant throws `IllegalArgumentException`.
- **An externally tagged enum reads a unit variant from its entry over `null` alone**, as serde reads it: with `Calm` among `Hatching`'s variants, `{"Calm":null}` reads `Calm`, beside `"Calm"`, and is written back `"Calm"`. The entry over any other value, `{"Calm":1}`, is refused with `SerializationException: Hatching: Calm takes no value`, as serde refuses it. Flattened, the enum is read from that entry, so a holder of an optional `Hatching` refuses `{"Calm":1,"note":"a"}` with the same exception, as TypeScript and Zod refuse it, where serde reads `None`.
- **An enum under a tag and a content key reads a unit variant from an object with no content key**, as TypeScript and Zod read it: with `#[serde(tag = "kind", content = "held")] enum Becalming { Breeze(bool), Lull }`, `{"kind":"Lull"}` reads `Lull`, and `{"kind":"Lull","held":5}` is refused with `SerializationException: Becalming: Lull takes no value`, as serde refuses it. `{"kind":"Lull","held":null}` is refused with the same exception, as TypeScript and Zod refuse it, where serde reads `Lull`. serde writes the variant as its tag alone, so every payload serde writes is read.
- **`()` is an `object` of its own, `KotlinUnit{N}`, read from `null` alone and written as `null`**, as serde reads and writes it, by a serializer declared beside it, once for each item that holds `()`: a field, the slot of a brand, a list's item, a tuple's position, an `Option` and a variant's one value. With `struct ProbeUnits { listed: Vec<()>, nothing: ProbeNothing, paired: (u32, ()), unit: () }` and `#[serde(transparent)] struct ProbeNothing(())`, `{"listed":[null],"nothing":null,"paired":[1,null],"unit":null}` is read and written back as serde writes it. Any other value in its place is refused with `SerializationException: invalid type: [], expected unit`, the value quoted as JSON, and an absent key that holds `()` with the `MissingFieldException` of the plugin's serializer, as serde refuses each. An `Option<()>` reads `null` as `null`, as serde reads it as `None`. A tuple struct whose every slot serde skips is no `()`: it is a `class`, which kotlinc takes where it refuses a `data class` of no property, read from `[]` and written as `[]`, as serde writes it.
- **A name serde writes is read and written as serde writes it, whatever it holds.** A member's, an alias's, a struct's own tag key and name, an enum's variant, tag key and content key, a plain enum's value and the names a flattened member is read under are each written between the quotes of a Kotlin string, a `$`, a `"` and a `\` escaped: with `#[serde(tag = "ki$nd", rename = "Price's$Tag\\")] struct Tariffed { #[serde(rename = "back\\slash")] backslash: String, #[serde(rename = "cost$usd", alias = "cost's")] cost: String, #[serde(rename = "say \"hi\"")] quoted: String }`, the property is declared `@SerialName("cost\$usd") val cost: String`, and `{"ki$nd":"Price's$Tag\\","back\\slash":"b","cost$usd":"c","say \"hi\"":"s"}` is read and written back as serde writes it. The service clients write each header name, part name and message name the same way, in Dart and Swift as well.
- **A name serde reads beside the one it writes is read too.** A variant is read under each of its `#[serde(alias = "...")]` names, under every tagging, flattened, and in a plain enum, and a member under each of its aliases, in a struct, flattened or not, and in a variant. Each is written under the name serde writes. With `#[serde(alias = "Quay")] Dock { #[serde(alias = "name")] label: String }`, `{"Quay":{"name":"s"}}` reads `Dock` and is written back `{"Dock":{"label":"s"}}`. A member under two of its names, `{"Dock":{"label":"s","name":"t"}}`, is refused with ``SerializationException: duplicate field `label` ``, as serde refuses it. A flattened struct takes each alias of its members off its holder's object, as serde does. A class the plugin reads, with a member under an alias, has a `{Type}Unjudged` that hands the plugin's serializer the payload with each alias read as the name it stands for, `object ParcelUnjudged : KSerializer<Parcel> { private val held = Parcel.generatedSerializer(); ... }`, and a plain enum with a variant under an alias has a `{Type}Unjudged` written by hand, its one serializer.
- **A tuple is a data class of its own, `KotlinTuple{N}{Crate}` in a model type's definition, and `KotlinTupleOf{hash}` in the service accessors (named by its shape, so `kotlin_http_client()` declares it once and `kotlin_ws_client()`, `kotlin_mcp_service()` and `kotlin_mcp_client()` name it and declare none; a bundle of them compiles with no edit), generic over each type parameter of its holder its positions reach**, read from and written as the array serde writes: with `pub struct Yoked<Stowed> { pub pair: (u8, Stowed) }`, `pair` is a `KotlinTuple{N}<Stowed>`, and `Yoked<String>` reads `{"pair":[1,"a"]}` and writes it back as serde does. An array of another length is refused with `SerializationException: invalid length 1, expected 2`, as serde refuses it.
- **A tuple struct that declares one slot is read from the value of its slot and written as that value**, as serde writes it, whatever the slot's attributes say, and its data class keeps the slot as `slot0`. With `pub struct Wafer(pub ObjectId);` held under the key `wafer`, `{"wafer":{"$oid":"507f1f77bcf86cd799439011"}}` is read and written back as it was, and so is such a struct in a list, as a map's value, as a type argument, flattened and in the one slot of a variant. A tuple struct that declares no slot or several is the array of the slots serde writes.
- **An enum under a tag alone is read by a serializer tixschema writes as well.** The plugin refuses `@KeepGeneratedSerializer` on a sealed interface, so the enum carries no `@JsonClassDiscriminator`: its unjudged serializer reads the tag key, takes it off the object and reads the rest as the variant, and writes the variant with the tag ahead of its members. A tag no variant has, and a payload with no tag, throw `SerializationException`. A newtype variant reads the rest as the value it holds, and writes that value's members after the tag, as serde writes them and as TypeScript and Zod read them: with `#[serde(tag = "kind")] enum Docketed { Weighed(Tonnage) }` over `struct Tonnage { cost: String }`, `{"kind":"Weighed","cost":"c"}` reads `Weighed` and is written back as it was read, and `{"kind":"Weighed","value":{"cost":"c"}}` is refused with `SerializationException: Encountered an unknown key 'value' at path: $`, as serde and Zod refuse it. The variant's class keeps its `value` property. An externally tagged enum held there writes a unit variant as an entry of its name over `null`, as serde writes it and as TypeScript and Zod describe it.
- **A plain enum that names a check has the two serializers every type has**, so `Json.decodeFromString<Rating>("\"Unrated\"")` throws what `validate()` answers, as the Rust read does, and a type that holds it reads it through `RatingUnjudged` and names it once. One that names no check keeps the one serializer the plugin writes, and its read judges nothing. One that names no check but a [`preprocess`](#preprocess-in-the-kotlin-client), or reads a variant under an alias, is read by `{Type}Unjudged`, the one serializer it then has.
- **A refusal the decoder raises itself, where no serializer tixschema writes reads the value, is its own, unchanged**: a missing key, a value of another type. No value was read for `validate()` to judge.

**The file the definitions are written into imports what they name.** The two serializers of every type name `KSerializer` and `SerializationException` of `kotlinx.serialization`, `SerialDescriptor` of its `descriptors` package, and `Encoder` and `Decoder` of its `encoding` package. A serializer tixschema writes names the `json` package as well, and `builtins` where it composes the serializer of a list, a map or a nullable:

```kotlin
import kotlinx.serialization.*
import kotlinx.serialization.builtins.*
import kotlinx.serialization.descriptors.*
import kotlinx.serialization.encoding.*
import kotlinx.serialization.json.*
```

**A holder reads what it holds unjudged, and the whole value is judged once**, by the type the read was asked for. It names the unjudged serializer of each model where the model is written, on the property and inside a list, a nullable and a map:

```kotlin
data class Manifest(
  @Serializable(with = ParcelUnjudged::class) val firstParcel: Parcel,
  val parcelList: List<@Serializable(with = ParcelUnjudged::class) Parcel>,
  val parcelsByDock: Map<String, @Serializable(with = ParcelUnjudged::class) Parcel>,
  @Serializable(with = ParcelUnjudged::class) val spareParcel: Parcel? = null,
)
```

A manifest that holds a broken parcel in each of the four places is refused once, with every message at its full place. The message is one line, broken here after each `; `, and it is the text the [Rust read](validation.md#a-read-refuses-what-validate-refuses) answers of the same payload:

```text
'firstParcel.label': too short: minimum length is 1, got 0;
'parcelList.1.fragileItemCodes.1': y is outside of itemCodeList;
'parcelsByDock.north.label': too short: minimum length is 1, got 0;
'spareParcel.fragileItemCodes.1': y is outside of itemCodeList
```

A model is read unjudged where the holder's `validate()` asks it for what it reports, and nowhere else. Held any other way it is read by its own serializer and judges itself as it is read: the payload is refused there too, by the first broken model, with that model's own messages and no place in front of them.

| Held | Judged |
|---|---|
| in a field, a nullable, a list, a map's values, a tuple written as a field's type, a `#[serde(flatten)]` member, a slot of a tuple struct or of a variant | once, with its holder, each message at its full place |
| behind one of the holder's own type parameters | by itself: `Crate<Parcel>` with a broken parcel answers `'fragileItemCodes.1': y is outside of itemCodeList` |
| by a brand, and by a member that writes a length or a pattern of its own on a brand | by itself: `too short: minimum length is 2, got 1`, where Rust writes the member's key in front, `'code': too short: ...` |
| by a brand over a text, a number, a date or a `bool` as a map's key | by itself, as the map reads the key, each message under the key's text at the path of the map: `'13': 13 is outside of the scale at path: $.pitches`, where Rust writes `'pitches.13': 13 is outside of the scale`, and over a text `'mute': mute is silent at path: $.tones`; a brand over a brand over a text writes the inner brand's messages the same way, `'mute': mute is silent at path: $.outer` |
| where its type is declared below its holder, is an alias, or is a type of your own | by itself: the emitter has written no `{Type}Unjudged` under that name when it writes the holder |

Rust reads a model declared below its holder unjudged, so the two differ there: a holder that breaks a bound of its own and holds such a model that breaks one answers both messages in Rust, and the model's alone in Kotlin. Declare the model above the type that holds it, and the two answer the same text.

**An `#[serde(untagged)]` enum takes the variant serde takes.** Its unjudged serializer tries each variant in the order declared and takes the first whose payload decodes and whose bounds hold: each member's own, and the `minLength`, `maxLength` and `pattern` of each brand the variant holds. A [check](checks.md#checks-check) and what a held model reports choose no variant: the variant is chosen first, and the enum's own serializer then refuses the value. A variant with such a bound, and the value class of a brand with one, carry `fun kotlinSchemaBound(): List<String>` for that: what the bounds alone answer.

```rust
#[model_schema(pattern = "^[A-Z]+$", check = ["unreserved"])]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Tag(pub String);

#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum Mark {
    Brief(#[model_schema_prop(maxLength = 3)] String),
    Coded(Tag),
    Plain(String),
}
```

| Payload | Read |
|---|---|
| `"ab"` | `MarkBrief` |
| `"ABCD"` | `MarkCoded`: past the bound of the first variant |
| `"abcd"` | `MarkPlain`: past that bound, and outside the pattern of the brand |
| `"NONE"` | refused with `a tag is not NONE`: `MarkCoded` is chosen, and the check of the brand refuses it |

A payload no variant takes throws `SerializationException: No variant of Mark matched`.

**The generated clients answer their fault for a reply that breaks a rule**, with the messages in the fault's detail: `undeserializable-payload` from the `http_rest` client, and `failed-validation` from the `ws_rpc` one, which names the field the Rust `ws_rpc` dispatcher names for the same payload ([the field a refused message names](ws-rpc.md)).

**To read a value that breaks a rule**, name the unjudged serializer, and call `validate()` for what is wrong with it:

```kotlin
val parcel = Json.decodeFromString(ParcelUnjudged, payload)
parcel.validate()
// ['label': too short: minimum length is 1, got 0, 'fragileItemCodes.1': y is outside of itemCodeList]
```

#### `preprocess` in the Kotlin client

The Kotlin read calls each function a [`preprocess`](preprocess.md#preprocessing-preprocess) names, under the name the attribute writes, on the payload the Rust read hands its own function. The payload is a `JsonElement` of `kotlinx.serialization`, and the function answers the payload the value is then read from: a payload it has nothing to correct in, as it is. A top-level function of the package, or one the file imports, is in scope. **The key is a last resort** here as it is in Rust: a function of yours runs on every read of the type, on a payload no schema describes.

| | Kotlin |
|---|---|
| A field's function | `fun trimmed(payload: JsonElement): JsonElement` |
| A type's function | `fun withoutFragile(payload: JsonElement): JsonElement` |

The two functions of the [example](preprocess.md#preprocessing-preprocess) in Kotlin:

```kotlin
fun trimmed(payload: JsonElement): JsonElement =
    if (payload is JsonPrimitive && payload.isString) JsonPrimitive(payload.content.trim()) else payload

fun withoutFragile(payload: JsonElement): JsonElement =
    if (payload is JsonObject && "fragileItemCodes" !in payload) {
        JsonObject(payload + ("fragileItemCodes" to JsonArray(emptyList())))
    } else {
        payload
    }
```

This is what the `Parcel` of that example emits. The emitter writes it on one line, and each line break below stands for one space of it:

```kotlin
@OptIn(ExperimentalSerializationApi::class) @Serializable(with = ParcelSerializer::class) @KeepGeneratedSerializer
data class Parcel(@Serializable(with = ParcelLabelPreprocess::class) val label: String, val itemCodeList: List<String>, val fragileItemCodes: List<String>) {
  fun validate(): List<String> {
    val violations = mutableListOf<String>();
    if (label.codePointCount(0, label.length) < 1) violations.add("'label': too short: minimum length is 1, got ${label.codePointCount(0, label.length)}");
    return violations
  }
}
@OptIn(ExperimentalSerializationApi::class) object ParcelUnjudged : KSerializer<Parcel> {
  private val held = Parcel.generatedSerializer();
  override val descriptor: SerialDescriptor = held.descriptor;
  override fun serialize(encoder: Encoder, value: Parcel) = held.serialize(encoder, value);
  private fun kotlinSchemaRebased(refused: SerializationException, at: String): SerializationException {
    if (refused.cause?.message == "validate()") return refused;
    val said = refused.message.orEmpty().replaceFirst(Regex("^Unexpected JSON token at offset \\d+: "), "");
    return SerializationException(if (" at path: \$" in said) said.replaceFirst(" at path: \$", " at path: " + at) else said + " at path: " + at, refused)
  };
  @OptIn(ExperimentalSerializationApi::class) private inline fun <KotlinSchemaRead> kotlinSchemaAt(input: Decoder, read: () -> KotlinSchemaRead): KotlinSchemaRead = try { read() } catch (refused: SerializationException) {
    val at = try { (input as? JsonDecoder)?.decodeJsonElement(); null } catch (probe: JsonDecodingException) { probe.path } ?: throw refused;
    throw kotlinSchemaRebased(refused, at)
  };
  private inline fun <KotlinSchemaRead> kotlinSchemaUnder(step: String, read: () -> KotlinSchemaRead): KotlinSchemaRead = try { read() } catch (refused: SerializationException) { throw kotlinSchemaRebased(refused, "\$" + step) };
  override fun deserialize(decoder: Decoder): Parcel {
    val input = decoder as JsonDecoder;
    val element = input.decodeJsonElement();
    return kotlinSchemaAt(input) { input.json.decodeFromString(held, transformDeserialize(element).toString()) }
  };
  private fun transformDeserialize(element: JsonElement): JsonElement = withoutFragile(element)
}
object ParcelSerializer : KSerializer<Parcel>, () -> KSerializer<Parcel> {
  override fun invoke(): KSerializer<Parcel> = ParcelUnjudged;
  override val descriptor: SerialDescriptor = ParcelUnjudged.descriptor;
  override fun serialize(encoder: Encoder, value: Parcel) = ParcelUnjudged.serialize(encoder, value);
  override fun deserialize(decoder: Decoder): Parcel {
    val read = ParcelUnjudged.deserialize(decoder);
    val violations = read.validate();
    if (violations.isNotEmpty()) throw SerializationException(violations.joinToString("; "), IllegalArgumentException("validate()"));
    return read
  }
}
object ParcelLabelPreprocess : KSerializer<String> {
  private val held = serializer<String>();
  override val descriptor: SerialDescriptor = held.descriptor;
  override fun serialize(encoder: Encoder, value: String) = held.serialize(encoder, value);
  private fun kotlinSchemaKinded(serializer: DeserializationStrategy<*>, element: JsonElement, at: String): JsonElement {
    var held = serializer.descriptor;
    while (held.isInline && held.elementsCount == 1) held = held.getElementDescriptor(0);
    val kind = held.kind;
    if ((kind is PrimitiveKind && element !is JsonPrimitive) || (kind == StructureKind.LIST && element !is JsonArray) || (kind == StructureKind.MAP && element !is JsonObject))
      throw SerializationException("'" + at + "': " + kind + " cannot be read from " + when (element) { is JsonArray -> "an array"; is JsonObject -> "an object"; else -> "a primitive" });
    return element
  };
  private fun <KotlinSchemaRead> kotlinSchemaDecoded(json: Json, serializer: DeserializationStrategy<KotlinSchemaRead>, element: JsonElement, at: String, step: String): KotlinSchemaRead = kotlinSchemaUnder(step) { json.decodeFromString(serializer, kotlinSchemaKinded(serializer, element, at).toString()) };
  private fun kotlinSchemaRebased(refused: SerializationException, at: String): SerializationException {
    if (refused.cause?.message == "validate()") return refused;
    val said = refused.message.orEmpty().replaceFirst(Regex("^Unexpected JSON token at offset \\d+: "), "");
    return SerializationException(if (" at path: \$" in said) said.replaceFirst(" at path: \$", " at path: " + at) else said + " at path: " + at, refused)
  };
  @OptIn(ExperimentalSerializationApi::class) private inline fun <KotlinSchemaRead> kotlinSchemaAt(input: Decoder, read: () -> KotlinSchemaRead): KotlinSchemaRead = try { read() } catch (refused: SerializationException) {
    val at = try { (input as? JsonDecoder)?.decodeJsonElement(); null } catch (probe: JsonDecodingException) { probe.path } ?: throw refused;
    throw kotlinSchemaRebased(refused, at)
  };
  private inline fun <KotlinSchemaRead> kotlinSchemaUnder(step: String, read: () -> KotlinSchemaRead): KotlinSchemaRead = try { read() } catch (refused: SerializationException) { throw kotlinSchemaRebased(refused, "\$" + step) };
  override fun deserialize(decoder: Decoder): String {
    val input = decoder as JsonDecoder;
    val element = input.decodeJsonElement();
    return kotlinSchemaAt(input) { input.json.decodeFromString(held, transformDeserialize(element).toString()) }
  };
  private fun transformDeserialize(element: JsonElement): JsonElement = kotlinSchemaKinded(serializer<String>(), trimmed(element), "label")
}
```

The serializer of the type answers each row of the [sample](preprocess.md#preprocessing-preprocess) as the Rust read answers it:

| # | Payload | The Kotlin read |
|---|---|---|
| 1 | `{"label":"  A  ","itemCodeList":["x"],"fragileItemCodes":["x"]}` | read; `label` is `"A"` |
| 2 | `{"label":"   ","itemCodeList":["x"],"fragileItemCodes":["x"]}` | refused: `'label': too short: minimum length is 1, got 0` |
| 3 | `{"label":"A","itemCodeList":["x"]}` | read; `fragileItemCodes` is `[]` |
| 5 | row 1 inside `Manifest`, under `parcel` | read; `parcel.label` is `"A"` |

- **The order of one read is Rust's.** The type's functions on the whole payload, each field's as that field is read, the value, then `validate()`: a bound and a [check](#the-checks) judge what the functions answered, which is why row 2 is refused. Several functions run in the order written, each over the answer of the one before: `preprocess = ["unwrapped", "withoutFragile"]` calls `withoutFragile(unwrapped(element))`.
- **A type's functions are called by `{Type}Unjudged`**, so [both serializers of the type](#the-read-in-the-kotlin-client) read the corrected payload, and so does a type that holds it, wherever it holds it: in a field, a nullable, a list, a map's value, a `#[serde(flatten)]` member, a variant under every tagging, and behind a type parameter. Where the plugin writes the serializer, of a class, of the `object` of a unit struct and of the value class of a brand, `{Type}Unjudged` is a `KSerializer` over it, as above: it takes the payload off the decoder, corrects it with `transformDeserialize`, and reads the plugin's serializer from the text of what that answers, so a refusal states its path from the root of the payload. The serializer tixschema writes for a struct with a `#[serde(flatten)]` member, for a tuple struct and for an enum under every tagging calls the functions on the payload it decodes, and an `#[serde(untagged)]` enum does so once, ahead of the choice of a variant.
- **A brand's functions take the payload of the value the brand is written over.** The value class of a brand with a parameter keeps the delegation to the plugin's serializer and reads through it in a `deserialize` of its own: the Kotlin compiler hands such a value class unboxed to the `serialize` a `JsonTransformingSerializer` inherits, and the write then throws `ClassCastException`.
- **A plain enum that names a function has `{Type}Unjudged` as its one serializer**, over the plugin's: `@Serializable(with = GradeUnjudged::class) @KeepGeneratedSerializer enum class Grade`. Its read judges nothing, as before.
- **A field's functions are called by a serializer of the field's own**, `{Type}{Field}Preprocess`, which the property names, in a class and in a variant's class alike. It reads through the serializer the field has without the key: the plugin's, the unjudged one of a model the holder's `validate()` asks, a nullable one for an `Option`, so the corrected payload is read, and judged, exactly as the payload was.
- **A field whose type reaches a type parameter has no serializer of its own.** The plugin takes one on a property only where its constructor takes no parameter, or one per argument of the property's own type, so it cannot be handed the serializer of the parameter. `{Type}Unjudged` calls that field's functions at its key in the object, after the type's own, and an enum's serializer does the same for such a member of a variant.
- **Where tixschema writes the read, a field's functions are called in place**: a member of a struct with a `#[serde(flatten)]` member, a slot of a tuple struct, and the slot of a variant. `val dock = kotlinSchemaDecoded(input.json, serializer<String>(), trimmed(obj.getValue("dock")), "dock", ".dock")`, the last argument the step the refusal of the member is placed under.
- **A key the payload does not hold calls no function**, as in Rust: `kotlinx.serialization` asks a property's serializer only for a key the payload holds, and a property with a default keeps it. A `null` the payload holds is a payload, and is handed to the function.
- **A `#[serde(flatten)]` member a function reads is handed the keys no member ahead of it took**, as the Rust read hands them, and as [every flattened member is](#the-read-in-the-kotlin-client): `{"dock":"north","tag":"t"}` hands the function of the flattened type `{"tag":"t"}`. A member flattened ahead of it, of a type that names no function, takes its own keys too, as serde's read of it does, and one that names a function takes none. An optional one is read from what its functions answer, and where that read throws is `null` or refuses the payload as the Zod schema of its holder does, as [the read](#the-read-in-the-kotlin-client) states.
- **Writing calls no function.** `{Type}Unjudged` and the serializer of a preprocessed field write a value as it is written without the key, and a value built in code is never read, so nothing corrects it.
- **A refusal below a function states the path the payload was read at.** A field that names a function, and a type that names one, are each read from the text of the value the function answered, and a refusal there is placed below the path the decoder read the payload at: `{"label":5}`, where the function left the number as it was, is refused with `Expected quotation mark '"', but had 'EOF' instead at path: $.label`, the words kotlinx writes of a text it refuses. A list or an object where the field reads a text or a number is refused with a `SerializationException` that names the field, `'label': STRING cannot be read from an array at path: $.label`, as it is by every read tixschema writes itself: each such read goes through a private `kotlinSchemaDecoded` that asks the serializer's descriptor for its kind before `kotlinx.serialization` 1.11.0 reads the element, which throws `IndexOutOfBoundsException` for a text read from a list or an object. The generated clients answer their fault for it, as they do for any refusal of a reply.
- **The definitions name more types of the `json` package**, `JsonElement`, `JsonDecoder`, `JsonEncoder` and `JsonDecodingException`, which the [imports of the read](#the-read-in-the-kotlin-client) already bring in.

**A name with no Kotlin function does not compile**, and the Kotlin compiler says so at the call:

```text
error: unresolved reference 'trimmed'.
```
