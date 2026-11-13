### Preprocessing (`preprocess`)

`preprocess = ["name", ...]` names functions of your own that correct a payload before a value is read from it: a label written with spaces at its ends, a key an older writer left out. It is written on a type, in `#[model_schema(...)]`, and on a field, in `#[model_schema_prop(...)]`. A name is written once, and every read calls a function under it: Rust the snake case of the name, `withoutFragile` as `without_fragile`, and the Zod schema, [the Dart client](#a-preprocess-in-the-dart-client), [the Swift client](#preprocess-in-the-swift-client) and [the Kotlin client](kotlin.md#preprocess-in-the-kotlin-client) the name as written. [Preprocess and check](preprocess-and-check.md#preprocess-and-check) states the functions of the five side by side.

**The key is a last resort.** A function of yours runs on every read of the type, on a payload no schema describes. Reach for it where the payload cannot be corrected where it is written, and where a serde `default`, an `Option` or an `alias` does not say what you mean.

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::{model_schema, model_schema_prop};

#[model_schema(preprocess = ["withoutFragile"])]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parcel {
    #[model_schema_prop(minLength = 1, preprocess = ["trimmed"])]
    pub label: String,
    pub item_code_list: Vec<String>,
    pub fragile_item_codes: Vec<String>,
}

#[model_schema()]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub parcel: Parcel,
}

// A field's function takes the payload of the field, and answers the payload the field is read from.
fn trimmed(payload: Value) -> Value {
    match payload {
        Value::String(text) => Value::String(text.trim().to_owned()),
        other => other,
    }
}

// A type's function takes the whole payload, ahead of every field's.
fn without_fragile(mut payload: Value) -> Value {
    if let Some(object) = payload.as_object_mut() {
        object
            .entry("fragileItemCodes")
            .or_insert_with(|| Value::Array(Vec::new()));
    }
    payload
}

fn main() {
    // Row 1: the label is read without the spaces at its ends.
    let parcel: Parcel = serde_json::from_str(
        r#"{"label":"  A  ","itemCodeList":["x"],"fragileItemCodes":["x"]}"#,
    )
    .unwrap();
    assert_eq!(parcel.label, "A");

    // Row 2: the bound judges the label the function answered.
    let refused = serde_json::from_str::<Parcel>(
        r#"{"label":"   ","itemCodeList":["x"],"fragileItemCodes":["x"]}"#,
    )
    .unwrap_err();
    assert_eq!(
        refused.to_string(),
        "'label': too short: minimum length is 1, got 0"
    );

    // Row 3: the type's function gives the payload the key it leaves out.
    let parcel: Parcel = serde_json::from_str(r#"{"label":"A","itemCodeList":["x"]}"#).unwrap();
    assert!(parcel.fragile_item_codes.is_empty());

    // Row 4: the same payload as a BSON document.
    let stored = bson::doc! { "label": "A", "itemCodeList": ["x"] };
    let parcel: Parcel = bson::from_document(stored).unwrap();
    assert!(parcel.fragile_item_codes.is_empty());

    // Row 5: a type that holds a parcel reads it through the parcel's functions.
    let manifest: Manifest = serde_json::from_str(
        r#"{"parcel":{"label":"  A  ","itemCodeList":["x"],"fragileItemCodes":["x"]}}"#,
    )
    .unwrap();
    assert_eq!(manifest.parcel.label, "A");

    // A key the payload does not hold is read as serde reads it: no function is called for it.
    let missing = serde_json::from_str::<Parcel>(r#"{"itemCodeList":["x"]}"#).unwrap_err();
    assert_eq!(missing.to_string(), "missing field `label`");

    // A value built in code is never read, so nothing corrects it.
    let built = ParcelDraft {
        label: "  A  ".to_owned(),
        item_code_list: Vec::new(),
        fragile_item_codes: Vec::new(),
    };
    assert_eq!(built.label, "  A  ");
}
```

| # | Payload | A Rust read | The Zod schema |
|---|---|---|---|
| 1 | `{"label":"  A  ","itemCodeList":["x"],"fragileItemCodes":["x"]}` | read; `label` is `"A"` | the same |
| 2 | `{"label":"   ","itemCodeList":["x"],"fragileItemCodes":["x"]}` | refused: `'label': too short: minimum length is 1, got 0` | the same line |
| 3 | `{"label":"A","itemCodeList":["x"]}` | read; `fragile_item_codes` is `[]` | read; `fragileItemCodes` is `[]` |
| 4 | row 3 as a BSON document | read, the same value | not applicable |
| 5 | row 1 inside `Manifest`, under `parcel` | read; `parcel.label` is `"A"` | the same |

Row 4 is read by the `bson` library's own read of a document, `bson::from_document` in version 2 and `bson::deserialize_from_document` in version 3.

#### The functions you write

A Rust function is in scope where the type is declared, a type declared inside a function body included. A TypeScript function is imported by the file the generated schemas are written into.

| | Rust | TypeScript |
|---|---|---|
| A field's function | `fn trimmed(payload: serde_json::Value) -> serde_json::Value` | `function trimmed(payload: unknown): unknown` |
| A type's function | `fn without_fragile(payload: serde_json::Value) -> serde_json::Value` | `function withoutFragile(payload: unknown): unknown` |

The two functions of the example in TypeScript:

```ts
export function trimmed(payload: unknown): unknown {
  return typeof payload === "string" ? payload.trim() : payload;
}

export function withoutFragile(payload: unknown): unknown {
  if (typeof payload !== "object" || payload === null || Array.isArray(payload)) {
    return payload;
  }
  return "fragileItemCodes" in payload ? payload : { ...payload, fragileItemCodes: [] };
}
```

- **A field's function takes the payload of the field**, and a type's takes the whole payload. Each answers the payload the value is then read from, and answers a payload it has nothing to correct in as it is: the read takes it or refuses it in its own words.
- **The payload is a `serde_json::Value`** whatever it was read from, a JSON text, a `serde_json::Value` or a BSON document, so one function serves every read and a crate that names a `preprocess` lists `serde_json`. It is untyped: `{"label": 5}` reaches `trimmed` as the number it is, and the read refuses what the function leaves, ``invalid type: integer `5`, expected a string``.
- **Several functions run in the order written**, each over the answer of the one before: `preprocess = ["unwrapped", "withoutFragile"]` calls `without_fragile(unwrapped(payload))`.
- **A type with a parameter writes its function once**: the payload has no type, so one function serves every filling.
- **The function of a [branded newtype](generics-and-brands.md#branded-newtypes)** is written on the type, and takes the payload of the value the brand is written over.

#### The order of one read

1. The type's functions, on the whole payload.
2. Each field's functions, as that field is read.
3. serde builds the value.
4. [`validate()`](validation.md#the-validate-method) judges it, where the type publishes one: a bound and a [check](checks.md#checks-check) judge what the functions answered, which is why row 2 is refused.

The Zod schema keeps the same order. A type's wrap is around its own expression and its `.check(...)`, a field's around the field's schema and its bounds:

```ts
const ParcelDraft$RawSchema = z.preprocess(function paired(held) { if (held instanceof Number) return [held, held.valueOf()]; if (typeof held !== "object" || held === null || (!Array.isArray(held) && Object.getPrototypeOf(held) !== Object.prototype)) return [held, held]; const copied = Array.isArray(held) ? [...held] : { ...held }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, paired(item)[1]); return [held, copied]; }, z.preprocess(function (pair, ctx) { if (!Array.isArray(pair)) return pair; const answered = Reflect.apply(withoutFragile, undefined, [pair[1], ctx]); return answered instanceof Promise ? answered.then((plain) => [pair[0], plain]) : [pair[0], answered]; }, z.preprocess(function reboxed(pair) { if (!Array.isArray(pair)) return pair; const original = pair[0]; const out = pair[1]; if (original instanceof Number) return Object.is(out, original.valueOf()) ? original : out; if (typeof out !== "object" || out === null || typeof original !== "object" || original === null || Array.isArray(out) !== Array.isArray(original) || (!Array.isArray(out) && Object.getPrototypeOf(out) !== Object.prototype)) return out; const copied = Array.isArray(out) ? [...out] : { ...out }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, reboxed([Reflect.get(original, key), item])); return copied; }, z.strictObject({
  label: z.preprocess(function paired(held) { if (held instanceof Number) return [held, held.valueOf()]; if (typeof held !== "object" || held === null || (!Array.isArray(held) && Object.getPrototypeOf(held) !== Object.prototype)) return [held, held]; const copied = Array.isArray(held) ? [...held] : { ...held }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, paired(item)[1]); return [held, copied]; }, z.preprocess(function (pair, ctx) { if (!Array.isArray(pair)) return pair; const answered = Reflect.apply(trimmed, undefined, [pair[1], ctx]); return answered instanceof Promise ? answered.then((plain) => [pair[0], plain]) : [pair[0], answered]; }, z.preprocess(function reboxed(pair) { if (!Array.isArray(pair)) return pair; const original = pair[0]; const out = pair[1]; if (original instanceof Number) return Object.is(out, original.valueOf()) ? original : out; if (typeof out !== "object" || out === null || typeof original !== "object" || original === null || Array.isArray(out) !== Array.isArray(original) || (!Array.isArray(out) && Object.getPrototypeOf(out) !== Object.prototype)) return out; const copied = Array.isArray(out) ? [...out] : { ...out }; for (const [key, item] of Object.entries(copied)) Reflect.set(copied, key, reboxed([Reflect.get(original, key), item])); return copied; }, z.string().min(1, { error: (issue) => `too short: minimum length is 1, got ${[...String(issue.input)].length}` })))),
  itemCodeList: z.array(z.string()),
  fragileItemCodes: z.array(z.string()),
}))));
```

Several names nest in the order written, the first outermost, so it is the first one called. A type's wrap is inside `.brand()` on a brand, inside the `.meta({ ... })` every binding carries, and inside the builder of a type with a parameter, so every filling of its factory carries it.

#### Where a function is called

**In every serde read** of a type that derives `Deserialize`, in every build: `serde_json::from_str`, `serde_json::from_value`, a BSON document under either major of the `bson` library, and any other format that describes itself. `#[model_schema()]` writes the `Deserialize` of a type that names a `preprocess`, on itself or on a field, as it writes that of a type that [publishes `validate()`](validation.md#a-read-refuses-what-validate-refuses), whether or not the type publishes one. Every shape takes the key on the type: a struct with named fields, a tuple struct, a unit struct, an enum under every tagging and a plain one, a branded newtype, and a type with a parameter at every filling.

A type that holds another reads it through that type's functions wherever it holds it: in a field, an `Option`, a list, a map's value, a `#[serde(flatten)]` member, and in a variant under every tagging. A type that reaches itself is corrected at every level. A function of a flattened type, and a function a flattened member names, is handed the keys the holder's own members do not take, in Rust and in Zod alike: a holder of two flattened types hands each one the same keys, and a payload that is no object is handed on as it is. With a function that answers `null` where the object it is handed holds `note`, a `#[model_schema(nonstrict)]` holder of `note` and a flattened `Option<Tholepin>` under it reads `{"label":"north","note":"n"}` with that `Tholepin`, on serde, Zod and each client: the function is handed `{"label":"north"}`.

**In the [recovering decode](recovering-decode.md#recovering-decode-decode_with)** of a type whose `enable` list holds `"decode_with"`. `from_value_with`, `from_value_piped`, `from_bson_with` and `from_bson_piped` call the type's functions once, on the value as they take it, so the callback and the resolvers are handed the corrected value, the one their repairs are read from, and no issue is listed for what a function corrected. A type held by another is corrected where it is walked. A field that names a function is read whole through it by the read and by the listing of issues, each calling it once: a payload the read refuses once corrected is one `Invalid` at the field, and so is a bound the corrected value breaks. The value the callback is handed holds the field as it was written.

**In a [MongoDB operation](mongodb-operations.md#mongodb-operations-and-typed-filters)** that reads a row, each row being read through `from_bson_piped`, and in a [service's dispatcher](services.md#service-validation), which reads a request through the functions of its message.

**In the Zod schema**, as one stage per name that calls the function, inside one `z.preprocess(function paired(held) { ... }, ...)` that hands the first function each number as a `number` and around one `z.preprocess(function reboxed(pair) { ... }, ...)` that hands the schema after them a number as the `Number` object it was read as where a function answered it unchanged: the TypeScript service code reads a number written with a fraction or an exponent as a `Number` object (see [Type mappings](../type-mappings.md)), and the function is handed what `JSON.parse` reads, at any depth of a list or a plain object. An integer a function answers unchanged is refused when it was written as a float, as serde refuses it, so `1.0` there is refused for a `u32`; a number a function changed, or answered at another path, is read as the number it is.

**In [the Dart client](#a-preprocess-in-the-dart-client)**, in the read of every type `dart_definition()` writes, under the `dart` feature.

**In the Swift client**, in every read of a type `swift_definition()` writes: [`preprocess` in the Swift client](#preprocess-in-the-swift-client) is what it calls and what it is handed.

**In the read of the Kotlin client**, by the serializers of each type: see [`preprocess` in the Kotlin client](kotlin.md#preprocess-in-the-kotlin-client).

**Nowhere else.** Writing calls no function, `Serialize` being serde's derive, and neither does `validate()`: a value built in code is never read, so nothing corrects it. The JSON Schema describes the value the type is read to and calls no function. So beside a `nonstrict` holder it describes a flattened `Option` a function reads as absent or as its type, whatever the function answers: with `#[model_schema(nonstrict)] pub struct Reeved { pub note: String, #[model_schema_prop(preprocess = ["asGiven"])] #[serde(flatten, default, skip_serializing_if = "Option::is_none")] pub tholepin: Option<Tholepin> }` over a `Tholepin` that reads `label`, and an `asGiven` that answers the payload as it was handed, the JSON Schema takes `{"note":"n"}` and `{"label":5,"note":"n"}`. serde reads `Tholepin` from what `asGiven` answers and refuses both, the first with ``missing field `label` `` and the second with ``invalid type: integer `5`, expected a string``, and so do Zod and the Dart, Kotlin and Swift reads, which call the function.

#### A key the payload does not hold

A Rust read answers a key the payload does not hold as serde does, and calls no function for it: a `#[serde(default)]` or an `Option` supplies the value, and any other field is refused with ``missing field `label` ``. The Zod schema calls the field's function for such a key, with `undefined`, or with the default of a key under `#[serde(default)]` ([A Key Read as Its Default](models.md#a-key-read-as-its-default-serdedefault)).

So a field's function that turns `undefined` into a value makes the Zod schema read a payload the Rust read refuses. Where a key may be left out, say so on the field, with `#[serde(default)]` or an `Option`, or give the payload the key in the type's function, which both sides call on the whole payload, as `withoutFragile` does. A `null` the payload holds is a payload: both sides hand it to the field's function.

#### A payload that is not JSON

A function takes a `serde_json::Value`, so what is read crosses to one through serde, and what the function answers is read through serde again. A type's function makes the whole payload cross, and a field's makes that field's payload cross and no other. The format has to describe itself: the crossing asks it what each value is.

Under BSON, measured against versions 2 and 3 of the `bson` library:

| The document holds | The function is handed | The read answers |
|---|---|---|
| an `ObjectId` | `{"$oid": "0123456789abcdef01234567"}` | the same `ObjectId`, where the function leaves it as it is |
| a date | `{"$date": "2023-11-14T22:13:20Z"}` | the same date |
| an `Int32`, an `Int64`, a `Double`, a text, a list | the JSON value of the same content | the same value |
| a `Double` that is `NaN` or infinite | `null` | what the field reads `null` as: an `f64` is refused, `invalid type: null, expected f64` |
| a generic binary, a `Decimal128` | nothing: the crossing is refused | `invalid type: byte array, expected any valid JSON value` |

A type that holds a value of the last row keeps `preprocess` off itself and off that field. Version 3 of the `bson` library writes each refusal inside a sentence of its own, `BSON error. Kind: A deserialization-related error occurred. Message: ... .`

In the recovering decode the document the callback is handed has crossed back to BSON: an `ObjectId` and a date are the BSON values they were, and an `Int32` is an `Int64`. A document that cannot cross is answered `Unrecovered` with one `Undescribed` issue holding the refusal, and the callback is not called.

#### What is refused

The key is read in every build, and each refusal is spanned on what was written:

| Written | The refusal says |
|---|---|
| `preprocess = "trimmed"` | takes an array of string literals, each naming a function: `preprocess = ["trimmed"]` |
| `preprocess = [trimmed]` | takes string literals: each element is a string literal that names a function |
| `preprocess = ["(v) => v"]` | names `(v) => v`, and a name is an identifier: ASCII letters, digits and `_`, not led by a digit. The name is written as a function's name in every language |
| `preprocess = []` | names no function: write one name at least, or drop the key |
| `preprocess = ["trimmed", "trimmed"]` | names `trimmed` twice: the name is written twice, and a function runs once |

A type that derives `Deserialize` is refused the key in five more places, each naming what to write instead:

- **Beside a `deserialize_with` or a `with` of your own on the same field.** serde reads a field through one function, and the macro's own is the one that calls yours: call the function from your own reader, or take the attribute off.
- **On a field that borrows from the payload**, a `&'a str` or a `#[serde(borrow)]`, and on a type that holds one. What a function answers is a payload of the read's own, which nothing read from it can borrow from.
- **On a type alias**, which has no read of its own: write it on the type the alias names, or on a field that holds the alias.
- **On the slot of a [branded newtype](generics-and-brands.md#branded-newtypes)**, as every key written there is: a brand's `preprocess` is written on the type.
- **Beside [`as`](literals-and-type-names.md#type-names-as)** on the same field.

`#[serde(remote = ...)]`, `from`, `try_from` and `into` are refused on a type that names a `preprocess`, as they are on one that [publishes `validate()`](validation.md#a-read-refuses-what-validate-refuses): the macro writes the read, and cannot put it behind a conversion it does not write. A type that derives no `Deserialize` is never read: it calls no function and is refused none of these.

**A name with no function in scope does not build**, and the error is reported at the name:

```rust,compile_fail
#[model_schema(preprocess = ["withoutFragile"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    pub label: String,
}
```

```text
error[E0425]: cannot find function `without_fragile` in this scope
  |
4 | #[model_schema(preprocess = ["withoutFragile"])]
  |                              ^^^^^^^^^^^^^^^^ not found in this scope
```

What resolves the error is a function `without_fragile` in scope where `Parcel` is declared.

**What the macro writes.** The type's `Deserialize` is written as [that of a type that publishes `validate()`](validation.md#a-read-refuses-what-validate-refuses) is: serde's derive runs on a private copy of the item inside `model_schema_read_unjudged`, and a field's functions are called by a function local to that read, hung on the copy's member. A type that names a `preprocess` on itself gains `model_schema_read_corrected` as well, the read of a payload its functions have already corrected, and one declared with `decode_with` gains `decode_with_read_corrected`, `decode_with_value_corrected` and the walk of a corrected value under `decode_with_value_corrected_issues`, `_named` and `_fields`, with the `bson` twin of each under the `bson` feature. Each is a name of the type's own: a function of yours under one of them on the same type is a duplicate definition (E0592).

#### A `preprocess` in the Dart client

With the `dart` feature, the read of every type `dart_definition()` writes, `fromJson` of its draft, calls the functions the type and its members name, each under the name the attribute writes. [`fromJson` and `fromJsonUnjudged` of the valid type are that read](dart.md#the-dart-read-refuses-what-validate-refuses), the first followed by `validationIssues()`, so both reads correct the payload, in [the order of a Rust read](#the-order-of-one-read): the type's functions, each member's as it is read, the value, then `validationIssues()`. The parcel of the example is read by:

```dart
factory ParcelDraft.fromJson(dynamic payload) {
  final json = withoutFragile(payload) as Map<String, dynamic>;
  return ParcelDraft(
    label: _readAt(json, 'label', () => (json.containsKey('label') ? trimmed(json['label']) : null) as String),
    item_code_list: _readAt(json, 'itemCodeList', () => (json['itemCodeList'] as List<dynamic>).indexed.map((e) => _readAt(null, e.$1, () => e.$2 as String)).toList()),
    fragile_item_codes: _readAt(json, 'fragileItemCodes', () => (json['fragileItemCodes'] as List<dynamic>).indexed.map((e) => _readAt(null, e.$1, () => e.$2 as String)).toList()),
  );
}
```

The functions are yours, in scope in the file the emitted definitions are written into. Each takes a payload and answers the payload the value is read from:

| | Dart |
|---|---|
| A field's function | `dynamic trimmed(dynamic payload)` |
| A type's function | `dynamic withoutFragile(dynamic payload)` |

```dart
dynamic trimmed(dynamic payload) => payload is String ? payload.trim() : payload;

dynamic withoutFragile(dynamic payload) =>
    payload is Map<String, dynamic> ? {'fragileItemCodes': <dynamic>[], ...payload} : payload;
```

With those two functions, `Parcel.fromJson` and `Manifest.fromJson` answer for the rows of [the table above](#preprocessing-preprocess) what the Rust read answers. Row 4 is a BSON document, which no Dart client reads:

| # | A Dart read |
|---|---|
| 1 | read; `label` is `"A"` |
| 2 | refused with a `FormatException`: `'label': too short: minimum length is 1, got 0` |
| 3 | read; `fragile_item_codes` is `[]` |
| 5 | read; `parcel.label` is `"A"` |

- **The key stays a last resort.** Each name is one more function you write and keep in step, in every language you emit.
- **The payload is what `jsonDecode` holds**: a `Map<String, dynamic>`, a `List<dynamic>`, a `String`, a number, a `bool` or `null`. A function answers `dynamic`, and the read casts what it answers.
- **A field's function is called where the payload holds the key**, a `null` it holds included. A key the payload does not hold calls no function and is read as it was: `{"itemCodeList":["x"],"fragileItemCodes":["x"]}` is refused with ``missing field `label` ``, as the Rust read answers it. A key under `#[serde(default)]` that the payload does not hold is read as its default, and no function is called for it. A slot's function is called with what its position holds.
- **Several functions run in the order written**, each over the answer of the one before: `preprocess = ["unwrapped", "withoutFragile"]` calls `withoutFragile(unwrapped(payload))`. One read calls each function once.
- **A type's function is handed the payload as it arrives**, of any kind, as the Rust read hands it. A type that names one takes `dynamic` in its reads, `factory Sticker.fromJson(dynamic json)` on the valid type and `factory StickerDraft.fromJson(dynamic payload)` on its draft, and the cast after the functions refuses an answer of the wrong kind. A type that names none keeps the payload its read types.
- **A type that holds another reads it through that type's own read**, so it is corrected wherever it is held: in a field, an optional, a list, a map's value, a variant and a `#[serde(flatten)]` member. A holder that names no function and flattens no member is written as it was.
- **A flattened member is read from the entries the members beside it and the members flattened ahead of it do not take**, as serde hands them: `Tagged.fromJsonUnjudged(({...json}..remove('dock')))`. A function it names, and a function of the type it holds, is handed those keys. Under a tag key, a type flattened into a variant, the function of a variant's one slot and the type that slot holds are handed the object without the tag as well, as serde hands them.
- **A flattened `Option` of any type but a map is read from what its functions answer as the Zod schema of its holder reads it**, where serde parts from Zod: where the type does not read the answer, `null` among it, the member is `null` where the object its functions are handed holds none of its struct's keys, or no key at all for a type whose keys are not known, an enum among them, and the payload is refused where it holds one, with the error of that read, as [the Dart read](dart.md#the-dart-read-refuses-what-validate-refuses) states. serde refuses an answer its type does not read, and reads `null` as `None`. A struct that flattens such a member is marked `nonstrict` ([Order and strictness of flattened members](flatten.md#order-and-strictness-of-flattened-members)), and beside a `nonstrict` holder the member is `null` exactly where its functions answer `null`, and is read from any other answer, so the Dart read answers each payload as serde does: `#[model_schema(nonstrict)] struct Reeved { note: String, #[model_schema_prop(preprocess = ["asGiven"])] #[serde(flatten, default, skip_serializing_if = "Option::is_none")] tholepin: Option<Tholepin> }` over a `Tholepin` that reads `label` refuses `{"note":"n"}` and `{"label":5,"note":"n"}`, and under a function that answers `null` reads `{"label":"north","note":"n"}` with `tholepin` `null`.
- **A sealed enum calls its functions in its base**, once, ahead of the dispatch on the tag, and an `#[serde(untagged)]` one ahead of every attempt. A plain enum calls them on the text, in `fromJsonUnjudged`.
- **The service clients read a reply through `fromJson`**, so a reply is corrected as any other payload is.
- **A name with no function does not compile**, and the Dart compiler reports the call: `Error: Method not found: 'trimmed'.`
- **A function may share a name with a member of its type.** A `preprocess` named `label` on a type with a field `label` is called through a private forwarder declared ahead of the class, as a [check](dart.md#the-functions-you-write-in-dart) is. The names a read declares itself hide none: `json`, `payload`, `map`, `tag`, `read` and the converter it takes for a type parameter, `codeTypeFromJson`, are numbered where a function is called under one, `json2`.

#### `preprocess` in the Swift client

Every read of a type `swift_definition()` writes calls the functions the type and its members name: `init(from:)`, [`init(swiftSchemaUnjudged:)`](swift.md#a-swift-read-refuses-what-validate-refuses), and the service clients, which read a reply through `init(from:)`. Every shape takes the key on the type: a struct, a tuple struct, a unit struct, the wrapper of a [branded newtype](generics-and-brands.md#branded-newtypes), an enum under every tagging and a plain one, and a type with a parameter at every filling. A member takes it in a struct, in the named members and the slots of a variant, and in the slots of a tuple struct.

**The key is a last resort in Swift as it is in Rust.** A function of yours runs on every read of the type, on a payload no type describes, and you write it again in each language that reads the type.

The two functions of the example in Swift, each under the name the attribute writes:

```swift
func trimmed(_ payload: Any) -> Any {
  guard let text = payload as? String else { return payload }
  return text.trimmingCharacters(in: .whitespaces)
}

func withoutFragile(_ payload: Any) -> Any {
  guard var members = payload as? [String: Any], members["fragileItemCodes"] == nil else { return payload }
  members["fragileItemCodes"] = [String]()
  return members
}
```

| | Swift |
|---|---|
| A field's function | `func trimmed(_ payload: Any) -> Any` |
| A type's function | `func withoutFragile(_ payload: Any) -> Any` |

A function is any function of the module the generated types are compiled in, and a type that names one needs `import Foundation`. A name with no function does not build:

```text
error: cannot find 'trimmed' in scope
```

- **The payload arrives as the Swift values of a JSON value**: an object as a `[String: Any]`, a list as an `[Any]`, a text as a `String`, `true` and `false` as a `Bool`, a number written with no fraction and no exponent that an `Int64` holds as an `Int64`, any other number as a `Double`, and `null` as an `NSNull`. `5` is an `Int64`, and `2.0`, `1e3`, `-0` and `1.5` are each a `Double`, as serde's `Value` holds a float. An integer member refuses a float the payload holds at its path, one a function answered included, as serde refuses it (``invalid type: floating point `2.0`, expected u32``), and a float member reads an integer a function answered. It is untyped, as in Rust: `{"label": 5}` reaches `trimmed` as the `Int64` it is.
- **A function answers a payload in the same values**, and answers a payload it has nothing to correct in as it is. An answer that is no JSON value, a `Date` or a value of a type of your own, is thrown as `DecodingError.dataCorrupted`, `a preprocess answered a value that is not JSON`.
- **A function is not called for a key the payload does not hold.** The key is read as it is with no function named: `keyNotFound` for a member the type requires, the default for a member under `#[serde(default)]`, `nil` for an optional one. A `null` the payload holds is a payload, and the function is handed an `NSNull`.
- **A flattened `Option` of any type but a map is read from what its functions answer as the Zod schema of its holder reads it**, where serde parts from Zod: where the type does not read the answer, `null` among it, the member is `nil` where the object its functions are handed holds none of its struct's keys, or no key at all for a type whose keys are not known, an enum among them, and the payload is refused where it holds one, as [the Swift read](swift.md#a-swift-read-refuses-what-validate-refuses) states. serde refuses an answer its type does not read, and reads `null` as `None`. Beside a `nonstrict` holder the member is `nil` exactly where its functions answer `NSNull()`, and is read from any other answer, as serde reads it.
- **Several functions run in the order written**, each over the answer of the one before: `preprocess = ["unwrapped", "withoutFragile"]` calls `withoutFragile(unwrapped(payload))`.
- **A type with a parameter writes its function once**, the payload having no type.
- **The order of one read is [the Rust read's](#the-order-of-one-read)**: the type's functions on the whole payload, each member's as that member is read, then the value is built, and `init(from:)` calls `validate()` on it. A bound and a check judge what the functions answered.

The Swift read answers the sample as the Rust read does:

| # | Payload | A Swift read |
|---|---|---|
| 1 | `{"label":"  A  ","itemCodeList":["x"],"fragileItemCodes":["x"]}` | read; `label` is `"A"` |
| 2 | `{"label":"   ","itemCodeList":["x"],"fragileItemCodes":["x"]}` | thrown, `DecodingError.dataCorrupted`: `'label': too short: minimum length is 1, got 0` |
| 3 | `{"label":"A","itemCodeList":["x"]}` | read; `fragileItemCodes` is `[]` |
| 5 | row 1 inside `Manifest`, under `parcel` | read; `parcel.label` is `"A"` |

**What the emitter writes** for `Parcel`, beside its properties, its `CodingKeys`, its memberwise initializer and its `init(from:)`:

```swift
private init(swiftSchemaCorrected decoder: Decoder) throws {
  let container = try decoder.container(keyedBy: CodingKeys.self)
  self.fragileItemCodes = try container.decode([String].self, forKey: .fragileItemCodes)
  self.itemCodeList = try container.decode([String].self, forKey: .itemCodeList)
  let labelPayload = try container.decode(SwiftSchemaPayload.self, forKey: .label).value
  self.label = try SwiftSchemaPayload.read(String.self, from: trimmed(labelPayload), at: container.codingPath + [CodingKeys.label])
}

fileprivate init(swiftSchemaUnjudged decoder: Decoder, swiftSchemaWithout taken: [String]) throws {
  self = try SwiftSchemaPayload.read(SwiftSchemaRead.self, from: withoutFragile(try SwiftSchemaPayload(from: decoder, without: taken).value), at: decoder.codingPath).value
}

fileprivate init(swiftSchemaUnjudged decoder: Decoder) throws {
  try self.init(swiftSchemaUnjudged: decoder, swiftSchemaWithout: [])
}
```

- **`SwiftSchemaPayload` is a private type written into each type that names a function**, on itself or on a member, into each type that reads a member from its object without some of its keys, a flattened member or what a variant under a tag alone holds, and into a struct under its own tag, which quotes a tag of another value through it: nothing is shared between two emitted types. It reads any JSON value from a `Decoder`, and reads a type back from what a function answers, through `JSONSerialization` and a `JSONDecoder` of its own.
- **A member's function is called in the type's own read**, on the payload of the member's key, and the member is read from what it answers.
- **A type's function is called by `init(swiftSchemaUnjudged:)`**, which reads the value from what it answers through the private `init(swiftSchemaCorrected:)`, the read of a payload the functions have already corrected. `SwiftSchemaRead` is the private type that hands that initializer a decoder.
- **A type that holds another reads it through that type's functions** wherever it holds it: in a field, an optional, a list, a map's value, a tuple, a flattened member and the payload of a variant. A model the `validate()` of its holder walks is read with no judgement where its member names a function too, through a private type written for the member, `SwiftSchemaReadFirstParcel` for `firstParcel`, so the whole value is still judged once.
- **A payload the read refuses once corrected is reported at the coding path of the value**, under the path of whatever holds the type: `label` for a parcel read alone, and `1`, `parcel`, `label` for the parcel of the second manifest of a list.
- **A function is handed the keys no type around its own took**, as [the Rust read hands them](#where-a-function-is-called). The function of a type flattened into a holder, and the function a flattened member names, is not handed the keys of the holder's own members, those a struct flattened ahead of it took, the entry an externally tagged enum flattened ahead of it took, nor those of a holder of that holder, and the function of a type a variant holds under a tag alone is not handed the tag. `init(swiftSchemaUnjudged:swiftSchemaWithout:)` is the read that takes those keys. It is written for a type that names a function on itself, and for every type that reads one from its own decoder: a holder that flattens it, an enum that holds it under a tag alone or with no tag, and a brand over it. The type is declared above the type that reads it: declared below, or held under a type parameter, it is read from the whole object of its holder, flattened from that object without the keys of the holder's own members, those a struct flattened ahead of it took and those of a holder of that holder, and as the one slot of a variant under a tag alone from that object without the tag.
