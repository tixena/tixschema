### Preprocess and check

Two keys name functions of your own. [`preprocess`](preprocess.md#preprocessing-preprocess) names functions that correct a payload before a value is read from it. [`check`](checks.md#checks-check) names functions that judge a value. Each key is written on a type, in `#[model_schema(...)]`, and on a field, in `#[model_schema_prop(...)]`. A name is written once, in the attribute, and you write one function under it in each language the crate builds for. Rust calls the snake case of the name, and TypeScript, Dart, Swift and Kotlin call the name as written.

**`preprocess` is a last resort.** It is not encouraged. It is for a payload that cannot be read otherwise. A function of yours runs on every read of the type, on a payload no schema describes, and you write it again in each language that reads the type.

This section states the two keys for Rust, TypeScript with Zod, Dart, Swift and Kotlin side by side. The detail of each language is in the section of that language, which this section names.

```rust
#[model_schema(check = ["withinParcel"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Parcel {
    pub fragile_item_codes: Vec<String>,
    #[model_schema_prop(check = ["withoutDuplicates"])]
    pub item_code_list: Vec<String>,
    #[model_schema_prop(minLength = 1, preprocess = ["trimmed"])]
    pub label: String,
}
```

`trimmed` corrects the payload of `label`, `withoutDuplicates` judges `item_code_list`, and `withinParcel` judges the whole parcel. A Rust read of the parcel calls `trimmed`, builds the value, and judges it with the bound and the two checks:

```rust
serde_json::from_str::<Parcel>(r#"{"fragileItemCodes":["x"],"itemCodeList":["x"],"label":"  A  "}"#);
// Ok: `label` is "A"

serde_json::from_str::<Parcel>(r#"{"fragileItemCodes":["y"],"itemCodeList":["x","x"],"label":"   "}"#);
// Err: 'itemCodeList': duplicate item code x; 'label': too short: minimum length is 1, got 0; 'fragileItemCodes.0': y is outside of itemCodeList
```

#### What each language calls

| | A `preprocess` function is called in | The function a `preprocess` names | A `check` function is called in | The function a `check` names | A refusal is | A name with no function is |
|---|---|---|---|---|---|---|
| Rust | `model_schema_read_unjudged`, the read alone, which the `Deserialize` of the type reads through | `fn trimmed(payload: serde_json::Value) -> serde_json::Value` | `validate()`, which the `Deserialize` of the type calls on the value it read | `fn without_duplicates(value: &[String]) -> Result<(), Vec<String>>` on a field, `fn within_parcel(value: &ParcelDraft) -> Result<(), Vec<String>>` on a type | the error of the serde read, which holds the messages joined by `; ` | ``error[E0425]: cannot find function `within_parcel` in this scope`` |
| TypeScript with Zod | one stage per name that calls `trimmed`, around the schema, inside one `z.preprocess(function paired(held) { ... }, ...)` that hands the first function each number as a `number` and one `z.preprocess(function reboxed(pair) { ... }, ...)` that gives the schema a number written as a float back as it was written | `function trimmed(payload: unknown): unknown` | one `.check((ctx) => { ... })` per name, on the schema | `function withoutDuplicates(value: Array<string>, ctx: ParsePayload<Array<string>>): void` on a field, `function withinParcel(value: ParcelDraft, ctx: ParsePayload<ParcelDraft>): void` on a type | a `ZodError`, which `parse` throws and `safeParse` answers under `error`, with one issue per broken rule | `error TS2304: Cannot find name 'withinParcel'.` |
| Dart | `fromJsonUnjudged`, the read alone, which `fromJson` reads through | `dynamic trimmed(dynamic payload)` | `validate()`, which `fromJson` calls on the value it read | `List<String> withoutDuplicates(List<String> value)` on a field, `List<String> withinParcel(Parcel value)` on a type | a `FormatException`, whose `message` holds the messages joined by `; ` | `Error: Method not found: 'trimmed'.` for a `preprocess`, `Error: The method 'withinParcel' isn't defined for the type 'Parcel'.` for a `check` |
| Swift | `init(swiftSchemaUnjudged:)`, the read alone, which `init(from:)` reads through | `func trimmed(_ payload: Any) -> Any` | `validate()`, which `init(from:)` calls on the value it read | `func withoutDuplicates(_ value: [String]) -> [String]` on a field, `func withinParcel(_ value: Parcel) -> [String]` on a type | `DecodingError.dataCorrupted`, whose `debugDescription` holds the messages joined by `; ` | `error: cannot find 'withinParcel' in scope` |
| Kotlin | `{Type}Unjudged`, the serializer that reads alone, which `{Type}Serializer` reads through | `fun trimmed(payload: JsonElement): JsonElement` | `validate()`, which `{Type}Serializer` calls on the value it read | `fun withoutDuplicates(value: List<String>): List<String>` on a field, `fun withinParcel(value: Parcel): List<String>` on a type | a `SerializationException`, whose message holds the messages joined by `; ` | `error: unresolved reference 'withinParcel'.` |

- **A function a `preprocess` names takes a payload and answers a payload**, on a field and on a type alike. A field's function takes the payload of the field, and a type's function takes the whole payload.
- **A function a `check` names takes a value and answers its messages.** A field's function takes the field's own value, and a type's function takes the whole value.
- **A name with no function is one error for the two keys**, in every language but Dart, which reports each in its own words.
- **`ParsePayload` is zod's own type**, which `zod/v4/core` exports.

Where each row is stated in full:

| | The functions of a `preprocess` | The functions of a `check` and `validate()` | The refusal |
|---|---|---|---|
| Rust | [Preprocessing](preprocess.md#preprocessing-preprocess) | [Checks](checks.md#checks-check), [The `validate()` Method](validation.md#the-validate-method) | [A read refuses what `validate()` refuses](validation.md#a-read-refuses-what-validate-refuses) |
| TypeScript with Zod | [The order of one read](preprocess.md#the-order-of-one-read) | [Where a check runs](checks.md#where-a-check-runs) | [The messages](checks.md#the-messages) |
| Dart | [A `preprocess` in the Dart client](preprocess.md#a-preprocess-in-the-dart-client) | [`validate()` in Dart](dart.md#validate-in-dart) | [The Dart read refuses what `validate()` refuses](dart.md#the-dart-read-refuses-what-validate-refuses) |
| Swift | [`preprocess` in the Swift client](preprocess.md#preprocess-in-the-swift-client) | [`validate()` in the Swift client](swift.md#validate-in-the-swift-client) | [A Swift read refuses what `validate()` refuses](swift.md#a-swift-read-refuses-what-validate-refuses) |
| Kotlin | [`preprocess` in the Kotlin client](kotlin.md#preprocess-in-the-kotlin-client) | [`validate()` in the Kotlin client](kotlin.md#validate-in-the-kotlin-client) | [The read in the Kotlin client](kotlin.md#the-read-in-the-kotlin-client) |

**Each client has limits of its own**, and the section of the client states them: a type declared below the type that reads it from its own decoder in Swift, and a model declared below its holder and a member that reaches a type parameter in Kotlin.

#### The six shared cases

Six values run the bound and the two checks of the parcel. Cases 1 to 5 are parcels, and case 6 is a `Manifest` that holds the parcel of case 4 under `parcel`:

| Case | The payload |
|---|---|
| 1 | `{"label":"A","itemCodeList":["x"],"fragileItemCodes":["x"]}` |
| 2 | `{"label":"","itemCodeList":["x"],"fragileItemCodes":["x"]}` |
| 3 | `{"label":"A","itemCodeList":["x","x"],"fragileItemCodes":["x"]}` |
| 4 | `{"label":"A","itemCodeList":["x"],"fragileItemCodes":["x","y"]}` |
| 5 | `{"label":"","itemCodeList":["x","x"],"fragileItemCodes":["y"]}` |
| 6 | `{"parcel":{"label":"A","itemCodeList":["x"],"fragileItemCodes":["x","y"]}}` |

Every language prints the same 13 lines for them:

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

Rust, Dart, Swift and Kotlin print them from the `validate()` of each value, and TypeScript from the issues the Zod schema answers for each payload. This repository holds the five to the lines with the parcel of [`validate()` in Dart](dart.md#validate-in-dart), which declares the members of the parcel above in the same order, with the bound and the two checks and no `preprocess`. The Rust `validate()` of the parcel above answers the same lines. A member's lines come in the order the parcel declares its members.

#### What holds in every language

- **One name, written once.** The attribute holds the name, Rust calls its snake case, `withinParcel` as `within_parcel`, and every other language calls the name as written. A name is an identifier: ASCII letters, digits and `_`, not led by a digit.
- **The order of one read is the same.** The type's `preprocess` functions on the whole payload, each field's as that field is read, then the value is built, then its bounds and its checks judge it. A bound and a check judge what the functions answered.
- **Several functions of one key run in the order written.** Each `preprocess` function is called over the answer of the one before, and every `check` runs: none stops another, and none is stopped by a broken bound.
- **A message is led by the path of the value in the payload**, in single quotes: the keys serde writes and the index of each item, joined by dots, `'parcel.fragileItemCodes.1'`. A field's `check` names no path, and its messages are written under the field. A type's `check` leads each message with the path of what it refuses.
- **A `preprocess` is not called for a key the payload does not hold**, in Rust, Dart, Swift and Kotlin. The Zod schema calls the field's function for such a key, with `undefined`, or with the default of a key under `#[serde(default)]` ([A Key Read as Its Default](models.md#a-key-read-as-its-default-serdedefault)). A `null` the payload holds is a payload, and every language hands it to the function.
- **`validate()` is on every type the Dart, Swift and Kotlin emitters write.** In Rust a type has it where a field carries a bound, where the type or a field names a `check`, and where it holds a type that has one. It judges a value built or changed in code.
- **A read refuses what `validate()` refuses**, in Rust, Dart, Swift and Kotlin: the read of a type is its own read alone, followed by its `validate()`, and no value is handed back for a payload that breaks a rule. The refusal holds every message, joined by `; `.
- **The whole value is judged once.** A type that holds another reads it with no judgement, and the type the read was asked for judges the whole value, each message at its full path. A value behind a type parameter is not reached by the `validate()` of its holder: it judges itself as it is read, with its own messages.
- **The Dart, Swift and Kotlin service clients answer their fault for a reply that breaks a rule**, with the messages in the fault's detail: `undeserializable-payload` from an `http_rest` client, and `failed-validation` from a `ws_rpc` client. Dart spells the two `undeserializablePayload` and `failedValidation`.
