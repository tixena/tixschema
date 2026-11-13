### Checks (`check`)

`check = ["name", ...]` names functions of your own that judge a value: a rule that reads one field against another, or one no bound spells. It is written on a type, in `#[model_schema(...)]`, and on a field, in `#[model_schema_prop(...)]`. A name is written once, and every language calls a function under it: Rust the snake case of the name, `withinParcel` as `within_parcel`, and TypeScript, Dart, Swift and Kotlin the name as written. [Preprocess and check](preprocess-and-check.md#preprocess-and-check) states the functions of the five side by side.

```rust
use serde::{Deserialize, Serialize};
use serde_json::json;
use tixschema::{model_schema, model_schema_prop};
use parcel_schema::DecodeWith as _;

tixschema::decode_with_foundation!();

#[model_schema(enable = ["decode_with"], check = ["withinParcel"])]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parcel {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
    #[model_schema_prop(check = ["withoutDuplicates"])]
    pub item_code_list: Vec<String>,
    pub fragile_item_codes: Vec<String>,
}

#[model_schema(enable = ["decode_with"])]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub parcel: Parcel,
}

// A field's check takes the field's own value, and names no path.
fn without_duplicates(value: &[String]) -> Result<(), Vec<String>> {
    for (index, code) in value.iter().enumerate() {
        if value[..index].contains(code) {
            return Err(vec![format!("duplicate item code {code}")]);
        }
    }
    Ok(())
}

// A type's check takes the whole value, and leads each message with the path of what it refuses.
fn within_parcel(value: &ParcelDraft) -> Result<(), Vec<String>> {
    let outside: Vec<String> = value
        .fragile_item_codes
        .iter()
        .enumerate()
        .filter(|(_, code)| !value.item_code_list.contains(code))
        .map(|(index, code)| {
            format!("'fragileItemCodes.{index}': {code} is outside of itemCodeList")
        })
        .collect();
    if outside.is_empty() { Ok(()) } else { Err(outside) }
}

fn main() {
    let parcel = |label: &str, codes: &[&str], fragile: &[&str]| ParcelDraft {
        label: label.to_string(),
        item_code_list: codes.iter().map(|code| code.to_string()).collect(),
        fragile_item_codes: fragile.iter().map(|code| code.to_string()).collect(),
    };
    let lines = |answered: Vec<String>| answered;

    // The draft is what is built, and `validate()` makes the valid `Parcel` of it.
    let valid: Parcel = parcel("A", &["x"], &["x"]).validate().unwrap();
    assert_eq!(valid.label, "A");
    assert!(parcel("A", &["x"], &["x"]).validation_issues().is_empty());
    assert_eq!(
        lines(parcel("", &["x"], &["x"]).validation_issues()),
        ["'label': too short: minimum length is 1, got 0"]
    );
    assert_eq!(
        lines(parcel("A", &["x", "x"], &["x"]).validation_issues()),
        ["'itemCodeList': duplicate item code x"]
    );
    assert_eq!(
        lines(parcel("A", &["x"], &["x", "y"]).validation_issues()),
        ["'fragileItemCodes.1': y is outside of itemCodeList"]
    );
    // Every bound and every check runs, and none stops another.
    assert_eq!(
        lines(parcel("", &["x", "x"], &["y"]).validation_issues()),
        [
            "'label': too short: minimum length is 1, got 0",
            "'itemCodeList': duplicate item code x",
            "'fragileItemCodes.0': y is outside of itemCodeList",
        ]
    );
    // A holder runs the `validation_issues()` of what it holds, under the key it holds it at. A
    // value that breaks a rule is a valid `Parcel` only as it is read with no check.
    let held = Parcel::model_schema_read_unjudged(
        json!({ "label": "A", "itemCodeList": ["x"], "fragileItemCodes": ["x", "y"] }),
    )
    .unwrap();
    let manifest = ManifestDraft { parcel: held };
    assert_eq!(
        lines(manifest.validation_issues()),
        ["'parcel.fragileItemCodes.1': y is outside of itemCodeList"]
    );

    // The recovering read lists a broken check as an issue, at the place its message names.
    let read = Parcel::from_value_with(
        json!({ "label": "A", "itemCodeList": ["x"], "fragileItemCodes": ["x", "y"] }),
        |_, issues| {
            assert_eq!(
                format!("{:?}", issues[0]),
                r#"Invalid { path: Path([Key("fragileItemCodes"), Index(1)]), expected: Unknown, found: String("y"), reason: "y is outside of itemCodeList" }"#
            );
            parcel_schema::Verdict::Reject
        },
    );
    assert!(read.is_err());
}
```

#### The functions you write

A Rust function is in scope where the type is declared, a type declared inside a function body included. A TypeScript function is imported by the file the generated schemas are written into, as a [`preprocess`](preprocess.md#preprocessing-preprocess) function is.

| | Rust | TypeScript |
|---|---|---|
| A field's check | `fn without_duplicates(value: &[String]) -> Result<(), Vec<String>>` | `function withoutDuplicates(value: Array<string>, ctx: ParsePayload<Array<string>>): void` |
| A type's check | `fn within_parcel(value: &ParcelDraft) -> Result<(), Vec<String>>` | `function withinParcel(value: ParcelDraft, ctx: ParsePayload<ParcelDraft>): void` |

`ParsePayload` is zod's own type, `z.core.ParsePayload`, which `zod/v4/core` exports. The two functions of the example in TypeScript:

```ts
import type { ParsePayload } from "zod/v4/core";

export function withoutDuplicates(value: Array<string>, ctx: ParsePayload<Array<string>>): void {
  const seen = new Set<string>();
  for (const code of value) {
    if (seen.has(code)) {
      ctx.issues.push({ code: "custom", message: `duplicate item code ${code}`, path: [], input: value });
      return;
    }
    seen.add(code);
  }
}

export function withinParcel(value: ParcelDraft, ctx: ParsePayload<ParcelDraft>): void {
  value.fragileItemCodes.forEach((code, index) => {
    if (!value.itemCodeList.includes(code)) {
      ctx.issues.push({
        code: "custom",
        message: `${code} is outside of itemCodeList`,
        path: ["fragileItemCodes", index],
        input: value,
      });
    }
  });
}
```

- **A field's check takes the field's own value**, read as a [bound reads it](validation.md#constraints-under-option-and-wrappers): under an `Option` it is called only where a value is held, under `Box`, `Rc`, `Arc`, `Cow` and a reference it takes what they hold, and a list or a map is taken whole. The call passes a reference, so `&[String]` and `&Vec<String>` both take a `Vec<String>`, and `&str` takes a `String`.
- **A type's check takes the whole value**, `&Parcel`, and an enum's takes the enum, whichever variant it holds.
- **A type with a parameter writes its function once, over every filling**: `fn with_a_code<CodeType>(value: &CrateDraft<CodeType>) -> Result<(), Vec<String>>`, and `function withACode<CodeType>(value: CrateDraft<CodeType>, ctx: ParsePayload<CrateDraft<CodeType>>): void`.
- **The check of a [branded newtype](generics-and-brands.md#branded-newtypes)** takes the brand in Rust, `&Code`, and the value the brand is written over in TypeScript, a `string` for a brand over a `String`: the Zod schema writes the check ahead of `.brand()`.

#### The messages

Rust answers `Err` with one message per broken rule, and TypeScript pushes one issue per broken rule to `ctx.issues`: `{ code: "custom", message, path, input: value }`.

- **A field's check names no path.** `validate()` writes each message under the name a bound of that field is reported under, [the path serde writes the field at](validation.md#the-path-a-message-names): its key, a slot's position among several, the variant's name or the content key ahead of a variant's member, and nothing for a `#[serde(flatten)]` member and for a slot serde writes as the value itself. A message that leads with a path below the field's value is written into that name: a check of `item_code_list` answering `'1': ...` is reported as `'itemCodeList.1': ...`. In TypeScript the issue's `path` is `[]`, or the path below the field's value, `[1]`, and zod writes the field's own key ahead of it.
- **A type's check leads each message with the path of what it refuses**, in single quotes, written with the keys of the payload and the index of each item, joined by dots: `'fragileItemCodes.1': y is outside of itemCodeList`. In TypeScript that is `path: ["fragileItemCodes", 1]`. tixschema cannot check a path you write: a key under `rename_all` is the renamed one, as in every generated message. A message with no lead is about the value itself.

#### Where a check runs

**In `validate()`, in every build.** A type that names a check, on itself or on a field, publishes [`validate()`](validation.md#the-validate-method) whether or not a field of it carries a bound, whatever the features: a function you wrote is called wherever the type is declared. Each member's bounds run first and its checks after them, member by member in the order declared, and the type's checks run last. Several checks of one key run in the order written, and every one runs: none stops another, and none is stopped by a broken bound.

Every shape runs its checks: a struct with named fields, a tuple struct (a slot's check where [a slot's bound](validation.md#a-slot-of-a-tuple-struct) is, reported where that bound is), a unit struct, an enum under every tagging and a plain one (a member's check runs for the variant the value holds, the enum's own after it), a branded newtype (after the brand's own string checks, with no path), and a type with a parameter at every filling. A type that holds another runs the `validate()` of what it holds, as it does [for a bound](validation.md#a-bound-the-fields-own-type-declares), so `Manifest` reports `'parcel.fragileItemCodes.1': y is outside of itemCodeList`.

**In the Zod schema**, as one `.check(...)` per name:

```ts
.check((ctx) => {
  const own = { value: ctx.value, issues: ctx.issues.slice(0, 0) };
  withinParcel(own.value, own);
  for (const issue of own.issues) ctx.issues.push({ ...issue, continue: true });
})
```

A field's check is written on the schema of the value the Rust function takes: after the field's bounds, inside the union an `Option` writes, on the array of a list after its count, and inside a `z.preprocess` wrap. A type's check is written on the type's own expression, ahead of `.brand()` on a brand and of the `.meta({ ... })` every binding carries, and inside the builder of a type with a parameter, so every filling of its factory carries it. Each issue is copied with `continue: true`: zod skips a later check once an issue without that flag is listed, and the Rust side runs every one.

**In the [recovering decode](recovering-decode.md#recovering-decode-decode_with)** of a type whose `enable` list holds `"decode_with"`. serde builds the value, the checks run on it, and each message is one `Issue::Invalid` of the read: `path` is what the message leads with, read against the raw value, `expected` is `Unknown`, `found` is the raw value at that path, and `reason` is the text after the lead. See [What each shape reports](recovering-decode.md#what-each-shape-reports). A check of a model a holder holds is listed at its full path, the holder's place in front, and so it is below a generic holder and a recursive one: a `Truck<CodeType>` holding `load: Crate<CodeType>` lists a broken check of the crate at `load.fragileItemCodes[1]`, and a `Carton` holding `cartons: Vec<Self>` lists one of its first inner carton at `cartons[0].fragileItemCodes[1]`.

**In a serde read** of a type that derives `Deserialize`. The read runs `validate()` on the value it read, checks included, and [refuses the payload](validation.md#a-read-refuses-what-validate-refuses) with what it answers. The hook an `#[serde(untagged)]` member is read through runs no check: a check chooses no variant.

**In the Dart `validationIssues()`** every emitted Dart type has, under the name as written, on the draft. See [`validate()` in Dart](dart.md#validate-in-dart).

**In the `validate()` of the Kotlin client.** Every Kotlin type carries one, which runs the bounds and calls each check under the name written, and the serializer of the type calls it on the value it read. See [`validate()` in the Kotlin client](kotlin.md#validate-in-the-kotlin-client) and [The read in the Kotlin client](kotlin.md#the-read-in-the-kotlin-client).

**In the Swift client**, in the [`validate()`](swift.md#validate-in-the-swift-client) every emitted type has: a member's checks after its bounds and after what the value it holds reports, the type's own last, each a call of the Swift function of that name.

**Nowhere else.** The JSON Schema calls no function, and a document is written as it is without the key. The Zod schema of a type that flattens an `#[serde(untagged)]` enum writes that enum's variants out one by one, with each member's check and without the check written on the enum itself, which the holder's `validate()` does run.

#### What is refused

The key is read in every build, and each refusal is spanned on what was written:

| Written | The refusal says |
|---|---|
| `check = "withinParcel"` | takes an array of string literals, each naming a function: `check = ["withinParcel"]` |
| `check = [withinParcel]` | takes string literals: each element is a string literal that names a function |
| `check = ["within-parcel"]` | names `within-parcel`, and a name is an identifier: ASCII letters, digits and `_`, not led by a digit. The name is written as a function's name in every language |
| `check = []` | names no function: write one name at least, or drop the key |
| `check = ["a", "a"]` | names `a` twice: the name is written twice, and a function runs once |

A `check` on a type alias is refused, an alias having no `impl` of its own to call a function from: write it on the type the alias names, or on a field that holds the alias. A `check` on the slot of a [branded newtype](generics-and-brands.md#branded-newtypes) is refused as every key written there is: a brand's check is written on the type.

**A name with no function in scope does not build**, and the error is reported at the name:

```rust,compile_fail
#[model_schema(check = ["withinParcel"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Parcel {
    pub label: String,
}
```

```text
error[E0425]: cannot find function `within_parcel` in this scope
  |
4 | #[model_schema(check = ["withinParcel"])]
  |                         ^^^^^^^^^^^^^^ help: try calling `within_parcel` as a method: `self.within_parcel()`
```

The `help` is the compiler's own. What resolves the error is a function `within_parcel` in scope where `Parcel` is declared.

**A function that asks more of a type parameter than the type declares does not build** until the bound is written on the type, the call being made at every filling:

```rust,compile_fail
#[model_schema(default_types(CodeType = String), check = ["withoutRepeats"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Crate<CodeType> {
    pub codes: Vec<CodeType>,
}

fn without_repeats<CodeType>(value: &CrateDraft<CodeType>) -> Result<(), Vec<String>>
where
    CodeType: PartialEq,
{
    for (index, code) in value.codes.iter().enumerate() {
        if value.codes[..index].contains(code) {
            return Err(vec![format!("'codes.{index}': a code is listed once")]);
        }
    }
    Ok(())
}
```

```text
error[E0277]: can't compare `CodeType` with `CodeType`
   |
 4 | #[model_schema(default_types(CodeType = String), check = ["withoutRepeats"])]
   |                                                           ^^^^^^^^^^^^^^^^ no implementation for `CodeType == CodeType`
   |
help: consider restricting type parameter `CodeType` with trait `PartialEq`
   |
 6 | pub struct Crate<CodeType: std::cmp::PartialEq> {
   |                          +++++++++++++++++++++
```
