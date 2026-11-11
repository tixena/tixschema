### `validate()` in Dart

With the `dart` feature, every Dart type `dart_definition()` writes has `List<String> validate()`: a class, the wrapper of a [branded newtype](generics-and-brands.md#branded-newtypes) and of a type alias, a tuple struct, a plain enum, and a sealed enum with each of its variants. It runs what the Rust [`validate()`](validation.md#the-validate-method) of the same type runs, in the same order, and answers the messages as a list, empty for a value that keeps every rule. A type with no rule answers `const []`.

```rust
#[model_schema(check = ["withinParcel"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Parcel {
    pub fragile_item_codes: Vec<String>,
    #[model_schema_prop(check = ["withoutDuplicates"])]
    pub item_code_list: Vec<String>,
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Manifest {
    pub parcel: Parcel,
}
```

`parcel_dart::dart_definition()` writes the class with its constructor, its fields, [its two reads](#the-dart-read-refuses-what-validate-refuses) and `toJson`, and after them:

```dart
List<String> validate() => [
  for (final violation in withoutDuplicates(item_code_list)) _under('itemCodeList', violation),
  if (label.runes.length < 1) "'label': too short: minimum length is 1, got ${label.runes.length}",
  ...withinParcel(this),
];

static String _under(String place, String violation) {
  if (violation.startsWith("'")) {
    final end = violation.indexOf("'", 1);
    if (end > 0) return "'$place.${violation.substring(1, end)}'${violation.substring(end + 1)}";
  }
  return "'$place': $violation";
}
```

`Manifest` holds a parcel, and runs it under the key it holds it at:

```dart
List<String> validate() => [
  for (final violation in parcel.validate()) _under('parcel', violation),
];
```

#### The functions you write in Dart

A check is called under the name the attribute writes, `withinParcel`, from inside the emitted class. The function is yours, in scope in the file the emitted definitions are written into. It takes the value and answers its messages, an empty list for a value it passes:

| | Dart |
|---|---|
| A field's check | `List<String> withoutDuplicates(List<String> value)` |
| A type's check | `List<String> withinParcel(Parcel value)` |
| The check of a type with a parameter, written once | `List<String> withACode<CodeType>(Crate<CodeType> value)` |

```dart
List<String> withoutDuplicates(List<String> value) {
  final seen = <String>{};
  for (final code in value) {
    if (!seen.add(code)) return ['duplicate item code $code'];
  }
  return const [];
}

List<String> withinParcel(Parcel value) => [
      for (final (index, code) in value.fragile_item_codes.indexed)
        if (!value.item_code_list.contains(code)) "'fragileItemCodes.$index': $code is outside of itemCodeList",
    ];
```

- **A field's check takes the field's own value** and names no place: `validate()` writes each message under the field's key. Under an `Option` it is called where a value is held.
- **A type's check takes the whole value** and leads each message with the place of what it refuses, in single quotes, as [the Rust function does](checks.md#the-messages). The check of an enum takes the enum, and each variant hands itself over. The check of a brand takes the wrapper.
- **A check may share a name with a member of its type.** A check named `label` on a type with a field `label` is called through a private forwarder declared ahead of the class (`final _parcelCheckLabel = label;`), which names your function where the class would read the member. A check that shares a name with no member is called by that name.

With those two functions, the `validate()` of six values, each read with `fromJsonUnjudged` from the payload serde writes for it, answers what the Rust `validate()` answers of the same values:

```text
case 1: passes
case 2: refused
    'label': too short: minimum length is 1, got 0
case 3: refused
    'itemCodeList': duplicate item code x
case 4: refused
    'fragileItemCodes.1': y is outside of itemCodeList
case 5: refused
    'itemCodeList': duplicate item code x
    'label': too short: minimum length is 1, got 0
    'fragileItemCodes.0': y is outside of itemCodeList
case 6: refused
    'parcel.fragileItemCodes.1': y is outside of itemCodeList
```

**A name with no function does not compile.** The emitted class calls it, and the Dart compiler reports the call:

```text
Error: The method 'withinParcel' isn't defined for the type 'Parcel'.
```

#### What Dart checks

| Written in Rust | Dart writes |
|---|---|
| `minLength`, `maxLength` on a string | `value.runes.length`, its Unicode code points: `"é"` and a character outside the Basic Multilingual Plane each count one |
| `minLength`, `maxLength` on a list | `value.length`, its items |
| `minimum`, `maximum` | `value < N`, `value > N` |
| `pattern`, and the second pattern [an enum's own check](enums.md#a-check-written-on-the-enum) adds | `RegExp("...").hasMatch(value)`, a match anywhere in the string |
| [`literal`](literals-and-type-names.md#literal-values) | `value != 214`, `value != 'fixed'`, `!value`: on each item of a list, under its index |
| `check` on a field and on a type | a call of the function under that name |
| a `HashSet`, a `BTreeSet` or an `IndexSet`, with no key | `_repeats(value)`, ahead of the set's count: two items are one where the JSON the codec writes for each is one text, and the message is [`repeats an item: a set holds each item once`](models.md#a-set-holds-each-item-once) |

- **The sentence is the one Rust and Zod write**, from the same source. A whole number is quoted back without `.0`: `too large: maximum is 3, got 4` for a `double` holding `4.0`.
- **The order is Rust's**: each member as declared, what the value it holds reports, then its bounds, then its checks, and the type's own checks last. The repeat of a set, which Rust never reports, runs ahead of the set's count.
- **An optional member is measured where it holds a value.** A member typed with a branded string is measured on the text the wrapper holds.
- **A holder runs the `validate()` of what it holds**, through an optional, each level of a list, a map's values and the positions of a tuple, and writes the place in front: `'parcelList.1.fragileItemCodes.1'`, `'parcelsByDock.north.label'`. An entry of a map is named by the text its key is written as, in the order the map holds its keys. A value behind one of the type's own parameters is not reached, as in Rust.
- **A brand runs its own string check on the value it wraps**, under no place. A brand over one of its own parameters holds its declared default and no other filling: `if (value case final String held) ...`. A brand over a list counts its items at every filling.
- **An enum's own string check** reaches in Dart the fields it reaches in Rust.

A message names a value by the steps serde writes on the way to it: a key, a position in an array, or nothing where serde writes the value itself.

| Member | serde writes it | The name |
|---|---|---|
| a named field of a struct | under its key | `'itemLabel'` |
| a slot of a tuple struct of several slots | at its position in the array | `'1'` |
| the one slot of a struct serde writes as the value itself | as the value itself | none: the sentence alone |
| a named member of a variant, no tag attribute | `{"Loose":{"freightLabel":...}}` | `'Loose.freightLabel'` |
| the one slot of a variant, no tag attribute | `{"Boxed":...}` | `'Boxed'`, and `'Boxed.label'` for a model it holds |
| a slot of a variant of several, no tag attribute | `{"Split":[...]}` | `'Split.1'` |
| a named member, tag and content key | `{"kind":"Loose","held":{...}}` | `'held.haulLabel'` |
| the one slot, tag and content key | `{"kind":"Boxed","held":...}` | `'held'` |
| a slot of several, tag and content key | `{"kind":"Split","held":[...]}` | `'held.1'` |
| a named member, tag alone | beside the tag | `'liftLabel'` |
| the one slot, tag alone | its own keys beside the tag | none |
| a member of an untagged variant | as the value itself, or under its key | none for the slot, its key for a named member |

The step of a variant is the name serde writes it under: its own `rename`, or the enum's `rename_all` over its name. A `#[serde(flatten)]` member writes no key and adds no step.

#### The Dart read refuses what `validate()` refuses

`fromJson` of every emitted type is its own unjudged read followed by its `validate()`, as [the Rust read](validation.md#a-read-refuses-what-validate-refuses) is. A payload whose value breaks a bound or a check is refused with a `FormatException`, and no object is handed back. Its `message` holds what `validate()` answers of the value read, every message, joined by `; `. Each class carries the two constructors:

```dart
factory Parcel.fromJson(Map<String, dynamic> json) {
  final read = Parcel.fromJsonUnjudged(json);
  final violations = read.validate();
  if (violations.isNotEmpty) throw FormatException(violations.join('; '));
  return read;
}

factory Parcel.fromJsonUnjudged(Map<String, dynamic> json) {
  for (final key in json.keys) {
    if (!const <String>{'fragileItemCodes', 'itemCodeList', 'label'}.contains(key)) throw FormatException('Unknown key of a Parcel: $key', <Object>[key]);
  }
  return Parcel(
    fragile_item_codes: _readAt(json, 'fragileItemCodes', () => (json['fragileItemCodes'] as List<dynamic>).indexed.map((e) => _readAt(null, e.$1, () => e.$2 as String)).toList()),
    item_code_list: _readAt(json, 'itemCodeList', () => (json['itemCodeList'] as List<dynamic>).indexed.map((e) => _readAt(null, e.$1, () => e.$2 as String)).toList()),
    label: _readAt(json, 'label', () => json['label'] as String),
  );
}
```

`_readAt`, a static helper of each class, reads a value under its step, a member's key or an item's index, and puts the step ahead of the path a refusal of the value carries.

A type that holds another reads it with `fromJsonUnjudged`, so the whole value is judged once, by the type the read was asked for:

```dart
factory Manifest.fromJsonUnjudged(Map<String, dynamic> json) {
  for (final key in json.keys) {
    if (!const <String>{'parcel'}.contains(key)) throw FormatException('Unknown key of a Manifest: $key', <Object>[key]);
  }
  return Manifest(
    parcel: _readAt(json, 'parcel', () => Parcel.fromJsonUnjudged(json['parcel'])),
  );
}
```

A terminal holds a parcel in a field, in a list, as a map's value and in an optional:

```rust
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Terminal {
    pub first_parcel: Parcel,
    pub lists_by_dock: BTreeMap<String, Vec<Parcel>>,
    pub parcel_list: Vec<Parcel>,
    pub parcels_by_dock: BTreeMap<String, Parcel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare_parcel: Option<Parcel>,
}
```

Five reads, each printing whether it was read and, where it was refused, the `message` of the exception broken after each `; `. A terminal with four broken parcels is refused once, with every message at its full place, and the Rust read answers the same four for the same payload:

```text
Parcel alone: refused (FormatException)
    'fragileItemCodes.1': y is outside of itemCodeList
Terminal, four broken parcels: refused (FormatException)
    'firstParcel.label': too short: minimum length is 1, got 0
    'parcelList.1.fragileItemCodes.1': y is outside of itemCodeList
    'parcelsByDock.north.label': too short: minimum length is 1, got 0
    'spareParcel.fragileItemCodes.1': y is outside of itemCodeList
List<Terminal>, the second broken: refused (FormatException)
    'firstParcel.label': too short: minimum length is 1, got 0
    'parcelList.1.fragileItemCodes.1': y is outside of itemCodeList
    'parcelsByDock.north.label': too short: minimum length is 1, got 0
    'spareParcel.fragileItemCodes.1': y is outside of itemCodeList
Crate<Parcel>, the second code broken: refused (FormatException)
    'fragileItemCodes.1': y is outside of itemCodeList
Terminal with no spareParcel key, all good: read
```

- **`fromJsonUnjudged` is the read alone.** It is public, because a holder in another file calls it. Call it yourself for the value and its messages together: `Parcel.fromJsonUnjudged(json).validate()`.
- **Every refusal the read writes itself is a `FormatException`**, the exception it throws for a value that breaks a rule, so a caller that catches it catches each. With `enum Drizzling { Calm, Gust { knots: u32 }, Held(String) }`, `"Gale"` and `{"Gale":{}}` are refused with `Unknown Drizzling: Gale`, and `{"Calm":1}` with `Drizzling: Calm takes no value`. A tag no variant has is refused in the same words under every tagging, and so is a name no variant of a plain enum has. A payload no variant of an untagged enum `Misting` reads is refused with `No variant of Misting matched`, a range of a key it does not write with `Unknown key of a Range: (start, end, step)`, and a `Result` of both its keys with `expected value`, serde's words for text that is not the document, which a `ws_rpc` attachment and client answer as `undeserializable-payload`, as serde refuses each. An externally tagged enum is read from an object of one key, and a tuple from an array of its length, as serde and Zod read each: with `enum Fogging { Boxed(String), Split(String, u32) }`, `{"Boxed":"a","Split":["b",1]}` is refused with `more than one key for Fogging`, `{}` with `empty object for Fogging`, `{"Split":["b",1,2]}` with `invalid length 3, expected 2` and `{"Split":["b"]}` with `invalid length 1, expected 2`, as serde and Zod refuse each. The array of a tuple struct, of a member's tuple and of a tuple a tuple holds is refused in the same words wherever its length is another. A fixed array `[T; N]` is read from an array of `N` items alone, at each level: with `struct Clenching { pair: [u8; 2] }`, `{"pair":[1,2,3]}` is refused with `invalid length 3, expected 2` and `{"pair":[1]}` with `invalid length 1, expected 2`, and a `[[u8; 2]; 3]` refuses an inner array of another length in the same words, as serde and Zod refuse each. A payload that is not the kind its type is read from, an array where an object is read, is refused by the Dart language with an error of its own, which is no `FormatException`: the `TypeError` of a cast. Either way no value was read for `validate()` to judge.
- **A refusal carries the path of the value refused**, as the `FormatException`'s `source`, a list of steps, and its `message` and what it prints are the words of the refusal alone. Each holder puts the step it read the value under ahead of the path: a member's key, an item's index, a map's key, under which its value is read and a brand that keys it is judged, a variant's name under serde's own tagging, the content key under a tag and a content key, a `Result`'s arm, and a tuple's position. With `pitches: BTreeMap<Pitch, u32>` over a brand whose `check` refuses a pitch above 12, `{"pitches":{"13":1}}` is refused with the source `[pitches, 13]`, and with `tally: Result<u32, String>`, `{"tally":{"Ok":"x"}}` with `[tally, Ok]`, as the Rust dispatcher names each. A key that does not read is refused at its map, `[pitches]` for `{"pitches":{"x":1}}`, as the Rust dispatcher names it. `{"parcel":{"fragileItemCodes":[],"itemCodeList":[],"label":"a","stray":1}}` read as a `Manifest` is refused with `Unknown key of a Parcel: stray` and the source `[parcel, stray]`. A key the payload does not hold, for a member whose read refuses the `null` it reads as, is refused with ``missing field `parcel` ``, as serde words it, and the source `[parcel]`. A value below the payload that Dart cannot take apart, such as a text where a number is read, is refused with a `FormatException` of the words of the cast's `TypeError` and its path, and so is a refusal a Dart parser writes, which keeps its message. A value inside a `#[serde(flatten)]` member is refused at its holder, as serde names the holder: with `pub struct Frame { #[serde(flatten)] pub beam: Beam, pub span: u32 }` over `pub struct Beam { pub oak: u32 }` held under `frame`, `{"frame":{"span":1,"oak":"x"}}` is refused with the source `[frame]`, and a key the member misses or a key it refuses keeps that key, `[frame, oak]` for `{"frame":{"span":1}}`. The Dart `ws_rpc` attachment and client name the path as the fault's `field`, and a `validate()` refusal by the place its first message opens with.
- **A value `validate()` does not reach judges itself as it is read**, and the first one that breaks a rule ends the read with its own messages, which name no place of the holder: a value behind one of the holder's own type parameters, read by the function the caller hands in (`Crate<Parcel>.fromJson(json, (e) => Parcel.fromJson(e))`), the key of a map, and what a brand wraps.
- **A sealed enum judges in its base.** The unjudged read of the base chooses the variant by the tag, and a variant with named members carries the two constructors as a class does.
- **An `#[serde(untagged)]` enum takes the variant serde takes.** Its unjudged read takes the first variant whose payload decodes and whose bounds hold: each member's own, and the bound a brand it holds carries. A brand with a `pattern`, a `minLength` or a `maxLength` answers for that bound alone in `validateBound()`. A check, and what a model it holds reports, choose no variant: the variant is taken, and `fromJson` then refuses the value.
- **A plain enum judges where it names a check**, and carries `fromJsonUnjudged` in every case.
- **A number, a `char` and a chrono date, time, and date and time are read in the form every target reads.** An integer outside its type's range is refused with `FormatException: expected an integer from 0 to 255, found 256` for a `u8`, as serde refuses it. An `f32` beyond what a 32-bit float holds is read as an infinity of its sign, `1e39` as `double.infinity`, as serde reads it, and a number beyond an `f64`, which `jsonDecode` reads as an infinity, is refused for an `f32` and an `f64` with `FormatException: number out of range`, serde's words. A `char` of other than one code point is refused with `FormatException: expected a character, found "ab"`, as serde refuses it, and a text `z.iso.date()`, `z.iso.time()` or `z.iso.datetime({ local: true })` refuses with `expected a date, found "2024-1-5"`, `expected a time, ...` or `expected a date and time, ...`, each value quoted as JSON. Where serde and Zod part on a date, the read follows Zod: it refuses `2024-1-5`, which serde reads, and reads `2024-01-05T10:20Z`, which serde refuses. A `DateTime<Tz>` is a Dart `DateTime`, read by `DateTime.parse` and written by `toUtc().toIso8601String()` with the `.000Z` of a date of whole seconds written as `Z`, as chrono writes it. Dart holds a microsecond: a date finer than that is read at the microsecond, and a map keyed by dates that read as one key is refused with `FormatException: repeated key ...` where it kept one entry.
- **`toJson` judges no bound and no check**: a value built in code that breaks one is written as it is. It writes each value in [the form every target reads](client-forms.md#the-form-each-client-sends): an integer outside its type's range, a `char` of other than one code point and a date or time text no rewrite turns into its form throw a `FormatException` in the words of the read, and an `f32` beyond what a 32-bit float holds, an infinity among them, one that says `expected a 32-bit float, found 1e+300`, a `NaiveDateTime` with a `Z`, and a `NaiveTime` or a `NaiveDateTime` without seconds, is written in the form serde writes, and a `DateTime` in UTC.
- **A struct's member under `#[serde(skip_serializing)]` is read, and `toJson` writes no key for it**, as serde writes none, and spreads nothing of a flattened one. A flattened one that is no `Option` is read as any flattened member is, its `default` aside, as serde reads it: a payload without a key its struct requires, or with an entry its map does not read, is refused. A member under `skip_serializing_if` is written wherever it holds a value: `toJson` cannot call the Rust function, so under any function but `Option::is_none`, `Vec::is_empty` say, it writes the key where serde leaves it out.
- **A `#[serde(flatten)]` member is read at its place, in the order declared**, as serde reads it: from the object without the keys the holder's own members take and the entries each member flattened ahead of it took. The function a flattened member names is handed that object, and so is a struct that hands the object it is read from to a function: one that names a [`preprocess`](preprocess.md#a-preprocess-in-the-dart-client) on itself, or flattens a member that names one. Every other member declared above its holder is handed the object without the keys of its holder's own members and those the members flattened ahead of it took, as serde hands it, and a struct reads its own keys of that object; a key two sources write, or one reads that a source after it writes, is refused at expansion ([Order and strictness of flattened members](flatten.md#order-and-strictness-of-flattened-members)). A struct takes the keys it reads off the object a member flattened after it is handed. A struct that names a [`preprocess`](preprocess.md#a-preprocess-in-the-dart-client) on itself, a flattened member that names one and a struct with a `#[serde(flatten)]` member of its own take no key, as serde reads each, so the function of a member flattened after one is handed its keys as well. A struct declared below its holder, or in another crate, is not known to take any: it is read from the object without the keys taken around it, and a member flattened after it is read from its keys as well, where serde reads it without them. A map is the [one member of its object](flatten.md#intersection-types-serdeflatten), and is read from every key of it, but the tag of an enum under a tag key. An optional map is refused at expansion, as serde reads its `None` back as a map of no entry ([Order and strictness of flattened members](flatten.md#order-and-strictness-of-flattened-members)). A model a map holds is read with no judgement, so one that breaks its rule still refuses the payload, as serde refuses it. Under a [`preprocess`](preprocess.md#a-preprocess-in-the-dart-client) it is read from what the function answers, and an entry there that does not read refuses the payload, as serde refuses it. `toJson` writes each flattened member at its place.
- **A plain enum under `#[serde(flatten)]` declared below its holder does not build.** serde writes it as the name of its variant over `null`, `{"Sound":null,"width":1}`, and reads it back. The holder is written before the enum is, so it cannot tell a plain enum from a struct, and reads the member from the object, as it reads a struct, where the `fromJsonUnjudged` of a plain enum takes a `String`: with `pub struct Casement { pub width: u32, #[serde(flatten)] pub weathering: Weathering }` over `pub enum Weathering { Sound, Worn }` declared below it, `dart analyze` answers `error - The argument type 'Map<String, dynamic>' can't be assigned to the parameter type 'String'.  - argument_type_not_assignable`. Declare the enum above its holder: the macro then refuses it with its own message, which names the field and says to write a named member (see [Intersection Types](flatten.md#intersection-types-serdeflatten)), and a run of the analyzer pins the answer above.
- **A flattened `Option` of a struct is `null` where the object it is handed holds none of the struct's keys, and a key there that does not read refuses the payload**, as TypeScript and Zod refuse it, where serde reads `None` and drops the key with no error. With `struct Lapping { note: String, #[serde(flatten, default, skip_serializing_if = "Option::is_none")] tabling: Option<Tabling> }` over a `Tabling` that reads `label`, `{"note":"n"}` is read with `tabling` `null` and `{"label":"north","note":"n"}` with the `Tabling` it holds, and `{"label":5,"note":"n"}` is refused with the words of its cast, `type 'int' is not a subtype of type 'String' in type cast`, where serde reads `None`. A struct of two keys is refused where one of them is there and the other is not. The keys are those serde reads the struct under, each member's name and each alias, and they are looked for in the object the struct is read from: the whole object for a struct declared above its holder that hands it to no function, a key a member of its holder's own reads too among them, as Zod looks for them, where serde reads `None`. An optional enum under a tag key is looked for the same way, by its tag and its content key: it is `null` where neither is there, and is read where one is, so a tag or a content that does not read refuses the payload, as TypeScript and Zod refuse it, where serde reads `None`. With `#[serde(tag = "kind")] enum Grommet { Calm, Round { turns: u32 } }` flattened beside `note: String`, `{"kind":"Round","note":"n","turns":2}` reads `Round`, `{"note":"n"}` reads `null`, and so does `{"note":"n","turns":2}`, a variant's member with no tag, as serde reads it, and `{"kind":"Round","note":"n","turns":"x"}` is refused with the words of its cast and `{"kind":"Gale","note":"n"}` with `FormatException: Unknown Grommet: Gale`. Under `#[serde(tag = "kind", content = "body")]`, `{"body":{"turns":2},"note":"n"}`, a content with no tag, is refused as well. Under a tag alone the enum takes no key off the object a member flattened after it is handed, and under a tag and a content key it takes both. A type whose keys are not known is read as `null` wherever its read throws, as serde reads `None`: a struct declared below its holder or in another crate, a struct with a `#[serde(flatten)]` member of its own, a struct with no member serde reads, a type that names a [`preprocess`](preprocess.md#a-preprocess-in-the-dart-client) on itself, an untagged enum, an enum under a tag key declared below its holder or in another crate, an alias, and a type parameter filled at a type whose keys are not known where it is filled. A type parameter filled at a type whose keys are known there, an `ObjectId`, a type over one, or a struct or an enum under a tag key declared above, is looked for by those keys: the read of a type that flattens an `Option` of its parameter takes them under a named argument, `{List<String>? heldKeys}`, and the type that fills the parameter hands them, so with `struct Waxing<Held> { #[serde(flatten)] annotation: Notation, #[serde(flatten)] held: Option<Held> }`, `Waxing<ObjectId>` reads `{"note":"n"}` with `held` `null` and refuses `{"$oid":"507f1f77bcf86cd79943901z","note":"n"}`, as TypeScript and Zod refuse it, where serde reads `None`. A model the holder's `validate()` walks is read with no judgement where it reads, so one that breaks its rule still refuses the payload, as serde refuses it. A flattened `Option` of any type but a map that names a `preprocess` is read from what its functions answer, as the Zod schema of its holder reads it, where serde parts from Zod: where the type does not read the answer, `null` among it, the member is `null` where the object its functions are handed holds none of its struct's keys, or no key at all for a type whose keys are not known, an enum among them, and the payload is refused where it holds one. serde refuses an answer its type does not read, and reads `null` as `None`. An externally tagged enum takes one entry, below, and an optional map is `null` wherever an entry does not read, above.
- **Beside a `nonstrict` holder, a flattened `Option` whose type does not read is `null`**, as serde reads `None` and the Zod schema of the holder reads it absent, and the holder reads past its keys: with `Grommet` above flattened under an `Option` into a `nonstrict` holder, `{"kind":"Round","note":"n","turns":"x"}` and `{"kind":"Gale","note":"n"}` read with the member `null`, and so does a struct's key, an externally tagged enum's entry and an `ObjectId` that does not read. One that names a [`preprocess`](preprocess.md#a-preprocess-in-the-dart-client) is `null` exactly where its functions answer `null`, and is read from any other answer, so an answer its type does not read refuses the payload, as serde refuses it: with `#[model_schema(nonstrict)] struct Reeved { note: String, #[model_schema_prop(preprocess = ["asGiven"])] #[serde(flatten, default, skip_serializing_if = "Option::is_none")] tholepin: Option<Tholepin> }` over a `Tholepin` that reads `label`, `{"note":"n"}` and `{"label":5,"note":"n"}` are refused, and under a function that answers `null`, `{"label":"north","note":"n"}` reads `tholepin` as `null`. The type refuses a key it does not declare in what the functions answer, as serde refuses it: `{"label":"north","note":"n","stray":1}` is refused under `asGiven`, and a member flattened with no `Option` is read the same way.
- **A flattened externally tagged enum takes one entry**, as serde reads it: the first, in the object's order, under the name a variant is read under, its `rename` or the enum's `rename_all` included, or an alias of one, wherever it is in the object, and the enum is read from that entry. With `enum Hatching { Bay { label: String }, Dock { label: String } }` flattened beside `note: String`, `{"north":2,"Bay":{"label":"n"},"note":"a"}` reads `Bay` and the note `a`, and a payload with no such entry is refused, as serde refuses it. A payload with entries under two of its names, `{"note":"a","Bay":{"label":"n"},"Dock":{"label":"s"}}`, is refused with `FormatException: more than one variant of enum Hatching found in flattened data: Bay, Dock`, each variant named once, in the order the enum declares them, a variant's entry beside one under its alias naming that variant alone, as TypeScript and Zod refuse it, where serde reads the first. An optional enum, `#[serde(default, flatten, skip_serializing_if = "Option::is_none")] hatching: Option<Hatching>`, reads `null` from a payload with no such entry, `{"north":2,"note":"a"}` among them, as serde reads `None`, and an entry that does not read, `{"Bay":{"label":5},"note":"a"}`, refuses the payload, as TypeScript and Zod refuse it, where serde reads `None` and drops the entry with no error. Flattened ahead of a member that names a function, `{"Bay":{"label":"n"},"north":2}` reads `Bay`, and the function is handed `north`. The enum is written as serde writes it, its variant's entry beside the holder's members, and a unit variant as its name over `null`: with `Calm` among `Hatching`'s variants, the holder writes `{"note":"a","Calm":null}`, the enum optional or under a `preprocess` as well, and reads `Calm` from that entry in either order. The entry over any other value is refused, as the Zod schema, which takes `null` alone, refuses it: serde refuses it too, but for an empty object, which it reads as `Calm`. An enum under a tag key alone and an untagged enum take no key, as serde reads each, and so does an enum that names a `preprocess` on itself. An enum under a tag and a content key takes both, as serde reads it as a struct of the two: the function of a member flattened after it is handed neither. An enum declared below its holder, or in another crate, is not known to take an entry: it is read from the object it is handed, which is refused where it holds any other key, as serde refuses it, the holder refusing a key none of its members declares: with `struct Overcast { note: String, #[serde(flatten)] stratus: Hazing }` declared above `enum Hazing { Bay { label: String } }`, `{"note":"a","Bay":{"label":"n"},"north":2}` is refused with `more than one key for Hazing`, where serde answers ``unknown field `north` ``. The write of the unit variant of such an enum throws a `TypeError`.
- **An externally tagged enum held by a newtype variant under a tag alone is read from the object without the tag and written beside the tag**, as serde writes it, a unit variant as its name over `null`: with `#[serde(tag = "kind")] enum Docketed { External(Palleted) }` over `enum Palleted { Loose, Stacked { layers: u32 } }`, `{"kind":"External","Loose":null}` reads `External(Loose)` and is written back as it was read. The tag alone, `{"kind":"External"}`, is refused with `FormatException: empty object for Palleted`, and the entries of two variants with `FormatException: more than one key for Palleted`, as serde refuses each. An alias of the enum declared above is written the same way, held there or flattened.
- **An externally tagged enum reads a unit variant from its entry over `null` too**, as serde reads it, where the Zod schema of the enum takes the name alone: `Hatching.fromJson(jsonDecode('{"Calm":null}'))` reads `Calm`, beside `"Calm"`, and writes it back `"Calm"`. The entry over any other value is refused, as serde refuses it.
- **An enum under a tag and a content key reads a unit variant from an object with no content key**, as TypeScript and Zod read it: with `#[serde(tag = "kind", content = "held")] enum Becalming { Breeze(bool), Lull }`, `Becalming.fromJson(jsonDecode('{"kind":"Lull"}'))` reads `Lull`, and `{"kind":"Lull","held":5}` is refused with `FormatException: Becalming: Lull takes no value`, as serde refuses it. `{"kind":"Lull","held":null}` is refused with the same exception, as TypeScript and Zod refuse it, where serde reads `Lull`. serde writes the variant as its tag alone, so every payload serde writes is read.
- **A struct under its own `#[serde(tag = "...")]` reads a payload whose tag is the struct's name, and refuses any other**, as TypeScript and the generated Zod schema refuse it, whose tag is the literal of that name. serde reads the struct whatever the tag holds, and where it is absent, and writes the name back. With `#[serde(tag = "kind")] struct Flag { label: String }`, `{"kind":"Flag","label":"n"}` reads, `{"label":"n"}` is refused with ``FormatException: missing field `kind` ``, `{"kind":"Other","label":"n"}` with `FormatException: expected tag "Flag", found "Other"`, and `{"kind":5,"label":"n"}` with `FormatException: expected tag "Flag", found 5`, where serde reads each as `Flag { label: "n" }`. A struct with a `#[serde(flatten)]` member reads its tag the same way, and so does a struct flattened into a holder, from the object the holder hands it, so the holder refuses the same payloads. `toJson` writes the tag ahead of the members, as serde writes it.
- **`()` is read from `null` alone and written as `null`**, as serde reads and writes it, wherever it is held: a field, the slot of a brand, a list's item, a tuple's position, an `Option` and a variant's one value. With `struct ProbeUnits { listed: Vec<()>, nothing: ProbeNothing, paired: (u32, ()), unit: () }` and `#[serde(transparent)] struct ProbeNothing(())`, `{"listed":[null],"nothing":null,"paired":[1,null],"unit":null}` is read and written back as serde writes it. Any other value in its place is refused with `FormatException: invalid type: [], expected unit`, the value quoted as JSON, and an absent key that holds `()`, or a brand over `()` or an untagged enum with a member over `()` declared above the holder, with ``FormatException: missing field `unit` ``, as serde refuses each. Such a type declared below its holder reads an absent key as `()`. An `Option<()>` reads `null` as `null`, as serde reads it as `None`. A tuple struct whose every slot serde skips is no `()`: it is read from `[]` and written as `[]`, as serde writes it.
- **A name serde writes is read and written as serde writes it, whatever it holds.** A member's, an alias's, a struct's own tag key and name, an enum's variant, tag key and content key, and a plain enum's value are each written between the quotes of a Dart string, a `$`, a `'` and a `\` escaped: with `#[serde(tag = "ki$nd", rename = "Price's$Tag\\")] struct Tariffed { #[serde(rename = "back\\slash")] backslash: String, #[serde(rename = "cost$usd", alias = "cost's")] cost: String, #[serde(rename = "say \"hi\"")] quoted: String }`, `{"ki$nd":"Price's$Tag\\","back\\slash":"b","cost$usd":"c","say \"hi\"":"s"}` is read and written back as serde writes it.
- **A name serde reads beside the one it writes is read too.** A variant is read under each of its `#[serde(alias = "...")]` names, under every tagging, flattened, and in a plain enum, and a member under each of its aliases, in a struct, flattened or not, and in a variant. Each is written under the name serde writes. With `#[serde(alias = "Quay")] Dock { #[serde(alias = "name")] label: String }`, `{"Quay":{"name":"s"}}` reads `Dock` and is written back `{"Dock":{"label":"s"}}`. A member under two of its names, `{"Dock":{"label":"s","name":"t"}}`, is refused with ``FormatException: duplicate field `label` ``, as serde refuses it. A flattened struct takes each alias of its members off its holder's object, as serde does. A class with a member under an alias reads its members from a copy of the payload, each alias read as the name it stands for, and a type that names no alias is written as it was.
- **A tuple struct that declares one slot is read from the value of its slot and written as that value**, as serde writes it, whatever the slot's attributes say, and its class holds the slot's value in `value`, as a brand's class does. With `pub struct Wafer(pub ObjectId);` held under the key `wafer`, `{"wafer":{"$oid":"507f1f77bcf86cd799439011"}}` is read and written back as it was, and so is such a struct in a list, as a map's value, as a type argument, flattened and in the one slot of a variant. A tuple struct that declares no slot or several is the array of the slots serde writes.
- **A field named `fromJsonUnjudged` keeps its name**, Dart taking a field and a constructor under one. A member of a plain enum under that name is `fromJsonUnjudged_`, the enum having the function, and keeps the value serde writes.
- **The service clients read a reply through `fromJson`**, so a reply that breaks a rule is the fault each client answers for a reply it cannot read, with the messages in its `detail`: `undeserializablePayload` from the HTTP client, and `failedValidation` from the WebSocket client.

#### What stays yours

- **`validate()` is for a value built in code**, before it is sent, and for a value changed after it was read: [`fromJson` judges what it reads](#the-dart-read-refuses-what-validate-refuses). It calls no [`preprocess`](preprocess.md#a-preprocess-in-the-dart-client): a read does, ahead of it.
- **A class of your own that a model holds** declares `List<String> validate()` and the constructor `fromJsonUnjudged` as the emitted ones do: a holder reads it and calls it with no knowledge of what the held type declares, whether that type is declared above the holder or below it. [`ObjectId`](object-id.md#mongodb-objectid-support) is the one exception, read through its `fromJson` and asked nothing.
- **A field named `validate`** is the member `validate_`, and keeps its key, as [a reserved word does](features.md#a-member-named-after-a-word-the-client-language-reserves).
- **The Dart `validate()` holds each bound and calls each check whatever the other features**, and a brand's own `minLength`, `maxLength` and `pattern` in every build with `serde`, the feature a brand needs.
