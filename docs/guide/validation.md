## Field Validation (`model_schema_prop`)

Use `#[model_schema_prop(...)]` on individual fields to add validation constraints, name [checks](checks.md#checks-check) of your own, override types, or correct a payload before it is read ([`preprocess`](preprocess.md#preprocessing-preprocess)). A constraint written once is held in five languages: the Zod schema checks it in TypeScript, a generated `validate()` method checks it in Rust, and the `validate()` of each emitted [Dart](dart.md#validate-in-dart), [Swift](swift.md#validate-in-the-swift-client) and [Kotlin](kotlin.md#validate-in-the-kotlin-client) type checks it there. The JSON Schema states it. In Rust, Dart, Swift and Kotlin the read of a type runs its `validate()` on the value it read, and refuses a payload that breaks a rule. In Rust the type you declare is emitted as `{Name}Draft`: its `validate(self)` answers the valid `{Name}` or every message, and its `validation_issues(&self)` lists the same messages without consuming the draft ([A Draft and a Valid Type](models.md#a-draft-and-a-valid-type)). [Preprocess and check](preprocess-and-check.md#preprocess-and-check) states the two keys that name functions of your own, for the five languages side by side.

### String Constraints (minLength, maxLength, pattern)

```rust
use tixschema::{model_schema, model_schema_prop};
use serde::{Deserialize, Serialize};

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize)]
pub struct UserProfile {
    #[model_schema_prop(minLength = 3, maxLength = 30, pattern = "^[a-z0-9_]+$")]
    pub username: String,

    #[model_schema_prop(maxLength = 200)]
    pub bio: String,

    #[model_schema_prop(pattern = "^[0-9a-fA-F]{24}$")]
    pub external_id: String,
}
```

Generated Zod for `username` -- each check carrying the sentence the Rust validator reports for the same bound, so the two languages answer a bad value with the same words:

```text
z.string()
  .min(3, { error: (issue) => `too short: minimum length is 3, got ${[...String(issue.input)].length}` })
  .max(30, { error: (issue) => `too long: maximum length is 30, got ${[...String(issue.input)].length}` })
  .check(z.regex(/^[a-z0-9_]+$/, { error: "does not match pattern '^[a-z0-9_]+$'" }))
```

Generated JSON Schema for `username`: `{ "type": "string", "minLength": 3, "maxLength": 30, "pattern": "^[a-z0-9_]+$" }`

The TypeScript type is unchanged -- still just `string`.

`minLength` and `maxLength` on a string count Unicode characters, its code points, on every surface: Rust (`chars().count()`), Zod, the JSON Schema, Dart (`runes`), Swift (`unicodeScalars`) and Kotlin (`codePointCount`), in the check and in the number a message reports. `"é"` and `"😀"` count 1, `"e\u{301}"` and `"😀😀"` count 2; a grapheme cluster is not the unit. The same holds for a brand, for the name of a unit variant under a length written on its enum, and for the rendering of a path.

A value breaking more than one of a field's bounds is reported once per bound on both sides, in the order the bounds were declared: `validate()` collects every violation rather than stopping at the first, and every check on the Zod schema runs. So `"A!"` in the field above reads `'username': too short: minimum length is 3, got 2; 'username': does not match pattern '^[a-z0-9_]+$'` from a Rust service and from a TypeScript one alike.

A type whose schema this crate writes whole carries none of the five constraints, and writing one on
such a field is a compile error naming the keys and the type: `ObjectId` writes a `{"$oid": "..."}`
object rather than a string, and the chrono types (`NaiveDate`, `NaiveTime`, `NaiveDateTime`,
`DateTime<Tz>`) write their own ISO spellings, which no surface reads a length, a pattern or a range
beside. Carry the value in a `String` field if it needs one.

A `PathBuf` field carries the same three constraints, as does the `Path` borrow behind a wrapper (`Box<Path>`, `Cow<'_, Path>`, `Arc<Path>`, `Rc<Path>`): serde writes a path as a JSON string, which is what the three surfaces render a constrained string for. The checks measure that string -- the path's `to_string_lossy` rendering, which is the exact wire value for every path serde can write, a path that is not UTF-8 being one serde refuses to serialize at all.

A borrowed string carries them too: a `&'a str` or `&'static str` field, bare or under the wrappers a `String` is reached through (`Option<&'a str>`). serde writes it as the JSON string a `String` writes, so the Zod schema and the JSON Schema are the ones a `String` field gets, and `validate()` holds the borrowed value to the bound in the same words.

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Greeting<'text> {
    #[model_schema_prop(minLength = 3)]
    pub name: &'text str,
}

Greeting { name: "al" }.validate();
// Err(["'name': too short: minimum length is 3, got 2"])
```

`validate()` reads through every reference, whatever it borrows, as the Zod schema and the JSON Schema do. A bound on a `&'a String`, a `&'a u32`, a `&'a [String]`, an `Option<&'a String>` or a `&'a Option<String>` field holds the value it borrows, and `validate()` answers it in the words the owned field gets. A bound refused on the owned type is refused on a reference to it: a length on a `&'a bool` fails to build with the error a length on a `bool` gives, and a bound on a `&'a RefCell<String>` is refused at expansion, naming `RefCell`.

#### A field typed with a branded string

A field typed with a [branded newtype](generics-and-brands.md#branded-newtypes) over a string carries the same three constraints, and the bound holds that field alone. The brand keeps none, so another field of the same type reads any value:

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct FullPath(String);

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct GroupTag {
    #[model_schema_prop(minLength = 1)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_field: Option<FullPath>,
    pub source: FullPath,
}
```

- `validate()` reports `'sort_field': too short: minimum length is 1, got 0` for `Some(FullPath::model_schema_read_unjudged(serde_json::json!("")).unwrap())`, a value `FullPath::new("")` refuses before a brand exists, and nothing for an empty `source`.
- The Zod schema writes the checks onto the brand's own schema, so the parsed value keeps its brand: `FullPath$Schema.check(z.minLength(1, { error: ... }))`.
- The JSON Schema narrows the brand's document from inside an `allOf`: `{ "allOf": [{ "type": "string" }, { "minLength": 1 }] }`.

The bound reaches the brand through the wrappers it reaches a `String` through -- `Option`, `Box`/`Rc`/`Arc`/`Cow` -- and whether the brand is declared above the field's type or below it. On a list of the brand it counts the list, as on [every list](#list-length-minlength-maxlength). The validator measures the text the brand publishes for it: `model_schema_bound_text()`, a method `#[model_schema]` adds to every brand whose inner is a `String`, a `str`, a `PathBuf` or a `Path`, directly or behind `Box`/`Rc`/`Arc`/`Cow`. A `#[model_schema]` alias of a `String` or a `PathBuf` that takes no parameter is measured as the type it names.

A length or a pattern on a field typed with anything else is a compile error at the field, because no surface has a string to measure there: a struct, an enum, a brand over a number, a `bool`, a `char`.

```text
error[E0277]: model_schema: field `latest`: `minLength` cannot apply to `Version`
   |
14 |     pub latest: Version,
   |                 ^^^^^^^ this type holds no string for the bound to measure
   |
   = note: a length or a pattern is measured on a `String`, on a path, or on a `#[serde(transparent)]` brand over one of them that `#[model_schema]` was written on, a brand over a brand needing the inner brand declared above it: declare the field as one of those, or drop the bound
```

Two more placements are refused as the field is expanded: a length or a pattern on a type written with arguments (`UserId<String>`), which writes whatever its arguments make it, and `minimum` or `maximum` on a field typed with any type declared elsewhere, a range being spelled against a number this crate writes itself.

#### What a `pattern` may contain

One `pattern` string reaches three readers: the generated Rust validator builds it with `regex::Regex::new`, the Zod schema splices it between `/` delimiters as a regex literal, and the JSON Schema `pattern` keyword is an ECMA-262 regex. A pattern is accepted only if all three read it, and read it the same way; otherwise the derive fails at expansion with the construct named.

**Engine baseline: ES2018.** The emitted Zod literals and JSON Schema patterns are written for an ECMA-262 engine at ES2018 or newer, and the guard's admissions are decided against that version rather than against whichever JavaScript runtime happens to be installed where the derive runs -- the schema is read wherever it is loaded, not where it was generated. ES2018 is what the crate's own output already assumes: it emits `(?<name>...)` named capture groups, which are ES2018. Nothing it emits needs anything newer.

Zod literals are spliced without flags, so the JavaScript side is always non-Unicode mode.

Some constructs are **translated** on the way out, to a spelling that means the same thing to all three readers. The translation is what every surface receives, the Rust validator included, so the three never validate different sets:

| You write | Every surface receives | Why |
|---|---|---|
| `(?P<name>...)` | `(?<name>...)` | One group under two spellings; the `regex` crate reads both, JavaScript reads only the second. |
| `\d` | `[0-9]` | The `regex` crate reads `\d` as the Unicode digit class, a flagless literal as ASCII -- so `^\d+$` would accept `١` in Rust and reject it in the browser. |
| `\w` | `[0-9A-Za-z_]` | Same divergence: the `regex` crate counts `é` and `α` as word characters, a flagless literal does not. |
| `\s` | `[\t\n\v\f\r ]` | The two whitespace sets are not even nested -- the `regex` crate spaces U+0085 and JavaScript does not, JavaScript spaces U+FEFF and the `regex` crate does not -- leaving the ASCII run as the only common ground. |

A class you already wrote out is left exactly as you wrote it, byte for byte.

Some constructs are **refused**, because no spelling makes the readers agree:

- `.`, and the negated classes `\D`, `\W`, `\S`. A flagless JavaScript literal matches one UTF-16 code unit where the `regex` crate matches one character, so a single character outside the Basic Multilingual Plane -- an emoji, say -- fills `^.$` in Rust and never in JavaScript. Writing the members out settles *which* characters are named and cannot settle how many code units one of them is. Name the characters you mean instead.
- `(?i:...)`, `(?m:...)` and `(?s:...)`, and their negated and combined forms. These are ECMA-262 regular expression modifiers, which post-date the ES2018 baseline; an engine at the baseline throws a `SyntaxError` where the schema loads. Recent runtimes do parse them, which is exactly why the baseline is a recorded decision rather than a measurement.
- `(?i)`, `(?x:...)`, `(?U:...)`, `(?u:...)`, `(?R:...)` -- inline flag directives and flags ECMA-262 never had.
- `\A`, `\z`, `\b{start}`, `\<`, `\>` -- anchors and boundaries JavaScript reads as escaped letters. Use `^` and `$`.
- `\p{...}`, `\pL`, `\P{...}` and POSIX `[:alpha:]` -- Unicode and POSIX classes a flagless literal reads as ordinary characters.
- `&&`, `--`, `~~` class operators and a class nested inside a class -- set operations JavaScript reads as class members.
- `\x{...}`, `\u{...}`, `\U...` and octal escapes -- code point escapes JavaScript reads differently or not at all. Write the character.
- An unescaped `]` opening a class, as in `[]-a]`. This one is refused rather than escaped because escaping it changes the meaning: `[]-a]` is the three members `]`, `-` and `a`, and `[\]-a]` is a range.

#### A `pattern` has to turn some value away

A pattern every string satisfies is refused too, for a different reason: it is a constraint that constrains nothing. `""`, `^`, `$`, `|`, `a*` and `^a*` all match at some position of every string, so the generated validator would reject no value and the Zod and JSON schemas would publish a check every payload passes -- an attribute that reads as a guarantee and is not one. The derive fails at expansion, naming the field or the brand.

The verdict is read off the parsed pattern rather than off its text, so the same shape written any way gets the same answer -- and the near misses keep theirs:

| Pattern | Verdict |
|---|---|
| `""`, `^`, `$`, `\|`, `()`, `(^)` | Refused -- matches at a position every string has |
| `a*`, `a?`, `a\|`, `^a*`, `a*$` | Refused -- the repeated part may run zero times, or the alternative may be skipped |
| `^$` | Accepted -- both ends at one position, which only the empty string has |
| `^a*$` | Accepted -- both ends pinned around a run of `a` |
| `\b`, `\B` | Constraining -- the empty string holds no word boundary -- and refused by the next rule |
| `\b\w+`, `\Ba` | Accepted -- a boundary with something beside it |
| `a+`, `^[a-z]+$` | Accepted -- a character has to be there |

#### A `pattern` cannot be one assertion and nothing else

`\b` and `\B` are refused for a third reason, and it is not about what they accept. The generated Rust validator builds the pattern with `regex::Regex::new`, and `clippy::trivial_regex` -- which a consumer denying `clippy::nursery` gets -- calls a lone assertion "unlikely to be useful as it is" and reports it against the `#[model_schema]` attribute, where the author has no edit to make. For the shapes that lint does name a replacement for -- `^abc`, `abc$`, `^abc$`, `^$`, `abc` -- this crate emits the `str` call instead of a regex, so they compile clean; for a lone boundary it names none, so there is nothing to put in the regex's place.

Write the boundary beside what has to sit next to it: `\b\w+` rather than `\b`, `\B\w` rather than `\B`. That keeps the regex, keeps the boundary check, and is no longer trivial to the lint. Only the lone assertion is refused -- `\ba`, `^\bfoo` and every other pattern with a boundary somewhere in it are untouched.

### Numeric Constraints (minimum, maximum)

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Product {
    #[model_schema_prop(minimum = 0, maximum = 120)]
    pub age_restriction: u32,

    #[model_schema_prop(minimum = 0.0)]
    pub price: f64,
}
```

A limit keeps the digits written, on every surface. `#[model_schema_prop(maximum = 9007199254740993)] pub size: i64` passes `9007199254740993` and refuses `9007199254740994` with `'size': too large: maximum is 9007199254740993, got 9007199254740994`, in `validate()` and the read, in Zod, Dart, Swift and Kotlin, and the JSON Schema writes `{"type":"integer","maximum":9007199254740993}`. An integer limit takes the type of its field where the generated code compiles, and the JSON Schema writes it as an integer: `maximum = 10` is `"maximum": 10` and `maximum = 10.0` is `"maximum": 10.0`. A separator and a suffix are not part of the number, so `1_000` and `1000i64` are `1000` everywhere. A limit on a float is compared as a float of the field's type, whatever its size: `#[model_schema_prop(maximum = 3000000000)] pub weight: f64`, and `maximum = 3e9`, pass `1.0` and `3000000000.0` and refuse `4e9` with `'weight': too large: maximum is 3000000000, got 4000000000`, and `maximum = 1e30` on an `f32` passes `2e9`.

### List Length (minLength, maxLength)

A key written on a field measures the field's own value. On a list, `minLength` and `maxLength` count its items, and no key written on a list reaches the items:

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    pub value: String,
}

#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Document {
    #[model_schema_prop(minLength = 1, maxLength = 3)]
    pub name_config: Vec<Part>,
}

Document { name_config: Vec::new() }.validate();
// Err(["'nameConfig': too short: minimum length is 1, got 0"])
```

A list of four parts answers `'nameConfig': too long: maximum length is 3, got 4`, and a list of one to three passes whatever its items hold. Each key works alone.

- **Zod** writes the count on the array: `z.array(Part$Schema).min(1, { error: ... }).max(3, { error: ... })`, each check reporting the sentence `validate()` reports.
- **JSON Schema** writes `"minItems": 1, "maxItems": 3` on the array, which are the keywords a validator reads there.
- **`validate()`** reports the list once, under the field's key. On a member of an `#[serde(untagged)]` enum the read counts it too, and the [recovering decode](recovering-decode.md#recovering-decode-decode_with) lists one `Invalid` at the list. See [Where a constraint is checked](#where-a-constraint-is-checked).
- **TypeScript** stays `Array<Part>`, with `Minimum length: 1` and `Maximum length: 3` in the member's comment.

Every list takes the two keys: `Vec`, `VecDeque`, `HashSet`, `BTreeSet`, `BinaryHeap`, a slice (`Box<[T]>`, `&'a [T]`), a fixed-size array `[T; N]`, and `IndexSet<T>` under the `indexmap` feature, whatever `T` is -- a model, a string, a number, a map, one of the item's own type parameters, or the item itself. On `[T; N]` the JSON Schema states the declared length and the bound together. `Option` and the transparent wrappers (`Box`, `Rc`, `Arc`, `Cow`) are read through, each holding one value: an `Option<Vec<T>>` that is `None` passes, and `Some(vec![])` is counted. A list is a value of its own and is not read through, so on a `Vec<Vec<String>>` the keys count the outer list and say nothing of the inner ones.

`pattern`, `minimum` and `maximum` on a list are compile errors, a list having no text and no number. A rule for each item goes on the item's own type, a [branded newtype](generics-and-brands.md#branded-newtype-validation-constraints) that carries it, and `validate()` runs it on each item beside the list's own count, reporting each item under [its index](#the-path-a-message-names):

```rust
#[model_schema(pattern = "^[a-z]+$", minLength = 1, maxLength = 3)]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Tag(String);

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Article {
    #[model_schema_prop(minLength = 1)]
    pub tags: Vec<Tag>, // at least one tag, each held by `Tag`
}
```

A brand carries `pattern`, `minLength` and `maxLength` and no range, so a range for each number of a list has no place to be written.

The validator and the hook a list publishes, `validate_{field}_value(&list)` and `deserialize_{field}`, are written over any list: they name no type the list holds, so a `Vec<T>` of the item's own parameter and a `Vec<Self>` get them as a `Vec<String>` does.

**A list that is named.** A length written where the field's type names a list counts the list's items, as it does where the list is written. That holds for a field typed with a [branded newtype](generics-and-brands.md#branded-newtypes) over a list, with a `#[model_schema()]` alias of a list, with a brand over such a brand, and for a generic brand whose declared default is a list:

```rust
#[model_schema(no_display)]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Tags(Vec<String>);

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Article {
    #[model_schema_prop(minLength = 1)]
    pub tags: Tags,
}

Article { tags: Tags(Vec::new()) }.validate();
// Err(["'tags': too short: minimum length is 1, got 0"])
```

The name is resolved by the compiler, so the type may be declared below the field's. A brand over a text or a list publishes the count as `model_schema_bound_length()`, a brand over a brand answers through the brand it holds when that brand is declared above it, and an alias of a `String`, a path or a std list is counted as the type it names. Zod writes `Tags$Schema.check(z.minLength(1, { error: ... }))`, whose sentence counts an array by its items and anything else by its characters; the JSON Schema narrows the name from inside an `allOf` with `minLength` beside `minItems` (and both maxima), since each keyword constrains only the instance type it is for. `pattern` on any of them stays a compile error, as does a length on a type that holds neither a text nor a list, in words that name a string, a path and a list. Dart, Swift and Kotlin count the same: `_boundLength`, `swiftSchemaBoundLength` and `kotlinSchemaLength()`. An alias of an `IndexSet` counts as an alias of a list, under the `indexmap` feature, when it is declared above the field. A brand over a brand that writes no bound of its own publishes no count, and a length on a field typed with one is refused. A length written on a brand over a brand declared below it is refused at the lower brand, in words that name the order that builds: move the lower brand above the one that holds it.

**What changed for a declaration that compiled.** A key written on a list used to be carried to each of its items. `minLength` or `maxLength` on a list of strings, of paths or of brands still compiles and now counts the items: `#[model_schema_prop(minLength = 3)] pub tags: Vec<String>` held each tag to three characters, and holds the list to three tags. The compiler does not report that change. `pattern`, `minimum` or `maximum` on a list no longer compiles.

### A bound of the wrong kind

A length or a pattern is measured on a string, a range on a number, and a length alone on a list. A key written on a field whose value it does not measure is a compile error as the field is expanded, in every feature configuration, naming the field and the keys:

| Keys | Field | What happens |
|---|---|---|
| `minLength`, `maxLength`, `pattern` | a number: `u8` to `u64`, `i8` to `i64`, `usize`, `isize`, `f32`, `f64` | Refused |
| `minimum`, `maximum` | a string: `String`, `str`, `PathBuf`, `Path` | Refused |
| `minimum`, `maximum` | a `bool`, a `char`, and a value every surface describes as unknown | Refused |
| `pattern`, `minimum`, `maximum` | a list | Refused |
| `minLength`, `maxLength`, `pattern` | a string, one read through a reference included | Enforced by Zod, the JSON Schema and `validate()` |
| `minimum`, `maximum` | a number | Enforced by Zod, the JSON Schema and `validate()` |
| `minLength`, `maxLength` | a list | Enforced by Zod, the JSON Schema and `validate()`: they [count its items](#list-length-minlength-maxlength) |

`Option` and a transparent wrapper do not change which kind a field is: `Option<u32>` and `Box<u32>` are numbers, `Option<String>`, `Box<str>`, `Cow<'_, str>` and `&'a str` are strings, and `Option<bool>` and `Box<char>` are refused a range as `bool` and `char` are. A list is a list whatever it holds: `Vec<u32>`, `[u32; 4]` and `Option<Vec<String>>` are lists.

```text
error: model_schema: field `age_restriction`: `minLength` cannot apply to a `u32` field — a length or a pattern is measured on a string, and this field writes a number: the constraint would reach neither Zod, nor the JSON schema, nor the generated validator. Bound a number with `minimum` or `maximum`, or drop it.
```

```text
error: model_schema: field `username`: `minimum` cannot apply to a `String` field — a range is measured on a number, and this field writes a string: the constraint would reach neither Zod, nor the JSON schema, nor the generated validator. Bound a string with `minLength`, `maxLength` or `pattern`, or drop it.
```

```text
error: model_schema: field `tags`: `pattern` cannot apply to a `Vec<String>` field — a key measures the field's own value, and this field writes a list, which has no text for a pattern and no number for a range. `minLength` and `maxLength` count the list's items, and no key written here reaches them. A rule for each item goes on the item's own type: declare the item as a `#[serde(transparent)]` brand that carries the `pattern`.
```

The refusal names the keys of the wrong kind and no other: `minimum` written beside a refused `minLength` on a number is not named, and neither is a `minLength` written beside a refused `pattern` on a list.

A `minimum` or `maximum` that is a whole number outside the range of the field's integer type is refused too, naming the field, the bound and the range: `#[model_schema_prop(minimum = -1)] pub count: u32` answers ``error: model_schema: field `count`: `minimum = -1` is outside what `u32` holds (0 to 4294967295). Write a bound inside that range, or take it off.`` It holds under `Option` and `Box`, on a variant's field and on a slot of a tuple.

A `minimum` or `maximum` with a fraction on an integer field is refused too, naming the whole limit that states the same bound: `#[model_schema_prop(minimum = 1.5)] pub least: i32` answers ``error: model_schema: field `least`: `minimum = 1.5` holds a fraction, and `i32` holds whole numbers only. Write `minimum = 2`.``, and `maximum = -1.5` answers ``Write `maximum = -2`.`` A limit written whole, `maximum = 10.0`, is not a fraction. A limit no float of the field's type holds is refused as well: `#[model_schema_prop(maximum = 1e39)] pub single: f32` answers ``error: model_schema: field `single`: `maximum = 1e39` is beyond what an `f32` holds (3.4028235e38 in size). Write a bound inside that range, or take it off.`` An `f64` is held to `1.7976931348623157e308`, and `maximum = 3e38` on an `f32` and `maximum = 1e39` on an `f64` build.

A bound no value meets is refused too, on a field and on a brand's own `minLength` and `maxLength`: a lower bound above its upper bound answers ``error: model_schema: field `name`: `minLength = 5` is above `maxLength = 2`, so no value passes``. On a fixed array `[T; N]`, a `maxLength` below `N` or a `minLength` above `N` is refused (``field `pair`: `maxLength = 1` cannot hold a `[String; 2]`, which always has 2 items``), and so is a count that only repeats the length (``field `pair`: `minLength = 2` repeats the length of `[String; 2]`, which always has 2 items. Take it off.``). An equal pair, a count the length always meets, and a length the expansion cannot read build as before.

### The `validate()` Method

When any field carries a constraint, or the type or one of its fields names a [check](checks.md#checks-check), the macro generates a `validate(&self) -> Result<(), Vec<String>>` method. It is what checks a field's constraints. A serde read of the type calls it on the value it read and [refuses what it refuses](#a-read-refuses-what-validate-refuses), and you call it on an instance built or changed in Rust.

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Registration {
    #[model_schema_prop(minLength = 3, maxLength = 30)]
    pub username: String,

    #[model_schema_prop(minimum = 0, maximum = 120)]
    pub age: u32,
}

let reg = Registration { username: "ab".to_string(), age: 150 };

match reg.validate() {
    Ok(()) => println!("valid"),
    Err(errors) => {
        for e in &errors {
            println!("Error: {e}");
        }
        // "'username': too short: minimum length is 3, got 2"
        // "'age': too large: maximum is 120, got 150"
    }
}
```

The macro also generates into the type's schema module:
- `validate_{field}_value(&FieldType) -> Result<(), Vec<String>>` -- pure static validator per field, answering every bound the value broke in the order they were declared
- `deserialize_{field}(D) -> Result<FieldType, E>` -- serde hook that calls the static validator; a field read through a reference gets none

A constrained field of an enum variant is named for its variant too -- `validate_{variant}_{field}_value` and `deserialize_{variant}_{field}`, with `{variant}` in `snake_case`. One schema module holds every variant's helpers, and a field name is unique only within the variant that declares it, so two variants naming one field carry their own constraints.

An enum whose members carry constraints publishes the same `validate(&self) -> Result<(), Vec<String>>`, under every tagging serde offers (externally, internally and adjacently tagged, and `#[serde(untagged)]`). A value holds one variant at a time, so the check runs that variant's members and no other's, collecting every violation in the words a struct's field is answered in:

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Action {
    Delete {
        #[model_schema_prop(minLength = 5)]
        note: String,
    },
    Upload {
        #[model_schema_prop(minLength = 3)]
        note: String,
    },
}

let deleting = Action::Delete { note: "abc".to_string() };
assert_eq!(
    deleting.validate().unwrap_err(),
    vec!["'note': too short: minimum length is 5, got 3".to_string()],
);
// The same value read as an `Upload` is held to that variant's own minimum, and passes.
assert!(Action::Upload { note: "abc".to_string() }.validate().is_ok());
```

Parity with structs runs both ways: an enum no member of which carries a constraint publishes no `validate()`, exactly as a constraint-free struct publishes none. A set, a deque and a heap are walked only where they hold a declared type, so a struct whose only member is a `HashSet<String>` holds no rule and publishes none either: a direct call of its `validate()` stops building, and a `validate` of your own on it builds.

#### A type that declares a parameter

`validate()` is written in the type's own `impl`, so a struct, a tuple struct and an enum that declare a type parameter have it at every filling, and a type that holds one at any filling runs it:

```rust
#[model_schema(default_types(CodeType = String))]
#[derive(Serialize, Deserialize)]
pub struct Crate<CodeType> {
    pub codes: Vec<CodeType>,
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Depot {
    pub numbered: Crate<u32>,
    pub named: Crate<String>,
}

Crate::<u32> { codes: vec![], label: String::new() }.validate();
// Err(["'label': too short: minimum length is 1, got 0"])

Depot {
    numbered: Crate { codes: vec![], label: String::new() },
    named: Crate { codes: vec![], label: String::new() },
}
.validate();
// Err(["'numbered.label': too short: minimum length is 1, got 0", "'named.label': too short: minimum length is 1, got 0"])
```

A [branded newtype](generics-and-brands.md#branded-newtype-validation-constraints) that declares a parameter has `validate()` at every filling as well, and runs its string checks where the filling is its declared default.

**`validate()` reads nothing behind a type parameter.** The `validate()` of a `Crate<CodeType>` runs nothing on its `codes`, whatever fills `CodeType`, a type that has a `validate()` of its own included. A read of the same payload is refused: what fills the parameter is read by its own `Deserialize`, and judges itself [as it is read](#a-read-refuses-what-validate-refuses), with its own messages and no place of the holder in front of them:

```rust
let outer = Crate {
    codes: vec![Crate::<u32> { codes: vec![], label: String::new() }],
    label: "outer".to_string(),
};
outer.validate();
// Ok(())

serde_json::from_str::<Crate<Crate<u32>>>(r#"{"codes":[{"codes":[],"label":""}],"label":"outer"}"#);
// Err: 'label': too short: minimum length is 1, got 0 at line 1 column 34
```

**A `validate` of your own on one filling does not build.** A type has one inherent function under a name, and the generated one is there at every filling:

```rust,compile_fail
impl Crate<u32> {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        if self.codes.contains(&0) {
            return Err(vec!["'codes': no code is zero".to_owned()]);
        }
        Ok(())
    }
}
```

```text
error[E0592]: duplicate definitions with name `validate`
```

#### The path a message names

Each message opens with where the broken value sits in the payload, in single quotes: every step serde writes on the way to the value, joined by dots. A step is the key of a member, the name or the content key serde writes a variant under, a position in an array, the index of an item of a list or the key of an entry of a map. Where serde writes the value itself, there is no step. The path is the one an issue of the Zod schema of the same declaration holds, and the one an issue of the [recovering decode](recovering-decode.md#recovering-decode-decode_with) holds.

- **A member** is named by the key serde writes it under. `rename_all` on a struct, `rename_all_fields` on an enum and a variant's own `rename_all` each move the name, and a field's own `rename` wins over the three. A type that renames nothing names each field as it was declared.
- **An item of a list** is named by its index, counted from zero, at each level of the list. A `grid: Vec<Vec<Package>>` answers `'grid.0.1.itemLabel': too short: minimum length is 1, got 0`, and a `code_list: Vec<Code>` answers `'codeList.1': too short: minimum length is 2, got 1`. A count written on the list itself is reported at the list, with no index: a `parcel_list` under `minLength = 1` answers `'parcelList': too short: minimum length is 1, got 0` when it is empty.
- **An entry of a map** is named by its key, as serde writes it. A `parcels_by_dock: BTreeMap<String, Parcel>` answers `'parcelsByDock.north.itemLabel': too short: minimum length is 1, got 0`. A key that is no string is the text serde writes for it: `'byNumber.7.itemLabel'` for a `u32`, `'byFlag.true.itemLabel'` for a `bool`, and for a plain enum the name serde writes the variant under, `'byDock.north.itemLabel'`. A map of lists and a list of maps name the key and the index in the order the payload nests them: `'listsByDock.north.1.itemLabel'` and `'docksInOrder.1.north.itemLabel'`. See [A model held in a map, in a slot or in a tuple](#a-model-held-in-a-map-in-a-slot-or-in-a-tuple).
- **A slot among several** is named by its position in the array serde writes, `'0'`, and so is a position of a tuple written as a field's type: `'deckPair.1.itemLabel'`. A slot under `#[serde(skip)]` is in no array, so the slots after it move up.
- **The one slot of a struct** is the value serde writes for the struct, and adds no step. `Sealed(Parcel)` answers `'itemLabel': too short: minimum length is 1, got 0` for the payload `{"itemLabel":""}`, a field `sealed_load: Sealed` answers `'sealedLoad.itemLabel': ...`, and a field `code: Lone`, over a `Lone(String)` held to `minLength = 1`, answers `'code': too short: minimum length is 1, got 0` for the payload `{"code":""}`.
- **A member of a variant** is named below what serde writes ahead of it:

  | The enum is written | serde writes | A named member | The one slot | A slot among several |
  |---|---|---|---|---|
  | with no tag attribute | `{"Loose":{"freightLabel":""}}` | `'Loose.freightLabel'` | `'Boxed.itemLabel'` | `'Split.1.itemLabel'` |
  | under `tag = "kind", content = "held"` | `{"kind":"Loose","held":{"haulLabel":""}}` | `'held.haulLabel'` | `'held.itemLabel'` | `'held.1.itemLabel'` |
  | under `tag = "kind"` | `{"kind":"Loose","liftLabel":""}` | `'liftLabel'` | `'itemLabel'` | |
  | under `untagged` | `{"loadLabel":"","loadParcels":[]}` | `'loadLabel'` | `'itemLabel'` | |

  The one slot holds a `Parcel` in each row, and the variant's step is the name serde writes it under: the variant's own `rename`, or the enum's `rename_all`. A slot that holds a string is named by the variant alone, `'Stamped': too short: minimum length is 1, got 0` for the payload `{"Stamped":""}`.
- **A `#[serde(flatten)]` member and an `Option`** write no key and add no step.
- **A value serde reads from a buffer** is named by no path below the buffered value: under `tag` alone, under `untagged` and below a `#[serde(flatten)]` member, a payload serde refuses answers `failed-validation` with serde's own sentence and no field. `{"kind":"Loose","label":5}` under `tag = "kind"` answers `invalid type: integer \`5\`, expected a string`, and `{"label":5}` under `untagged` answers `data did not match any variant of untagged enum`.

```rust
#[model_schema(minLength = 2)]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Code(String);

#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Package {
    #[model_schema_prop(minLength = 1)]
    pub item_label: String,
    #[serde(rename = "codes")]
    #[model_schema_prop(minLength = 1)]
    pub listed_item_codes: Vec<String>,
    pub parcel_code: Code,
}

#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Base {
    #[model_schema_prop(minLength = 1)]
    pub base_name: String,
}

#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub first_parcel: Package,
    pub parcel_list: Vec<Package>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spare_parcel: Option<Package>,
    #[serde(flatten)]
    pub starting_base: Base,
}

let broken = || Package {
    item_label: String::new(),
    listed_item_codes: Vec::new(),
    parcel_code: Code::model_schema_read_unjudged(serde_json::json!("a")).unwrap(),
};
let manifest = Manifest {
    first_parcel: broken(),
    parcel_list: vec![broken(), broken()],
    spare_parcel: Some(broken()),
    starting_base: Base { base_name: String::new() },
};
println!("{}", manifest.validate().unwrap_err().join("\n"));
```

```text
'firstParcel.itemLabel': too short: minimum length is 1, got 0
'firstParcel.codes': too short: minimum length is 1, got 0
'firstParcel.parcelCode': too short: minimum length is 2, got 1
'parcelList.0.itemLabel': too short: minimum length is 1, got 0
'parcelList.0.codes': too short: minimum length is 1, got 0
'parcelList.0.parcelCode': too short: minimum length is 2, got 1
'parcelList.1.itemLabel': too short: minimum length is 1, got 0
'parcelList.1.codes': too short: minimum length is 1, got 0
'parcelList.1.parcelCode': too short: minimum length is 2, got 1
'spareParcel.itemLabel': too short: minimum length is 1, got 0
'spareParcel.codes': too short: minimum length is 1, got 0
'spareParcel.parcelCode': too short: minimum length is 2, got 1
'baseName': too short: minimum length is 1, got 0
```

What a program is written with keeps its Rust name: the helpers of `item_label` are `validate_item_label_value` and `deserialize_item_label`, and a refusal at build time names the field as its author wrote it.

#### A slot of a tuple variant

A slot of a tuple variant takes the five constraints as a named member of the variant does, with one slot or several, and the variant's wire and TypeScript type do not change. The Zod schema and the JSON Schema write the checks on the slot's own member, and the Rust side enforces them where it enforces a named member's in the same position:

| The slot of | The read | `validate()` reports a violation |
|---|---|---|
| a newtype variant of an `#[serde(untagged)]` enum, such as the `Other(String)` arm of [`MimeType`](untagged-enums.md#untagged-enums-serdeuntagged) | checks it, which is what chooses the variant | under no name: `too short: minimum length is 1, got 0` |
| a tuple variant of a tagged enum | reads it, and the enum's read refuses the value once it is whole | at [the path serde writes the slot at](#the-path-a-message-names): `'Code': ...` for the one slot of a variant, and `'Pair.0': ...` and `'Pair.1': ...` for two slots |

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub enum Entry {
    Code(#[model_schema_prop(pattern = "^[A-Z]{3}$")] String),
    Pair(
        #[model_schema_prop(minLength = 2)] String,
        #[model_schema_prop(minimum = 1)] u32,
    ),
}

Entry::Code("abc".to_string()).validate();
// Err(["'Code': does not match pattern '^[A-Z]{3}$'"])
Entry::Pair("a".to_string(), 0).validate();
// Err(["'Pair.0': too short: minimum length is 2, got 1", "'Pair.1': too small: minimum is 1, got 0"])
MimeType::Other(String::new()).validate();
// Err(["too short: minimum length is 1, got 0", "does not match pattern '^[a-z]+/[a-z0-9.+-]+$'"])
```

serde refuses `{"Code":"abc"}` and `{"Pair":["a",0]}` with those messages, the two of the pair joined by `; `. It reads `{"Code":"ABC"}` and `{"Pair":["ab",1]}` as it did, and `Entry::Code("ABC".to_string())` still writes `{"Code":"ABC"}`. The helpers are named by the slot's place among the ones its variant declares, `validate_{variant}_{index}_value` and `deserialize_{variant}_{index}`: `validate_code_0_value` and `validate_pair_1_value` here, `validate_other_0_value` and `deserialize_other_0` for `MimeType`. An untagged slot whose type is a brand that declares a bound of its own runs both, the slot's check first.

#### A slot of a tuple struct

A slot of a tuple struct takes the five constraints as a named field of a struct does, and the struct's wire and TypeScript type do not change. `validate()` reports a violation under the slot's position in the array serde writes, the struct's read refuses the value with it, and the Zod schema and the JSON Schema write the checks on the slot's member of the tuple:

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Pair(
    #[model_schema_prop(minLength = 1)] pub String,
    #[model_schema_prop(minimum = 1)] pub u32,
);

Pair(String::new(), 0).validate();
// Err(["'0': too short: minimum length is 1, got 0", "'1': too small: minimum is 1, got 0"])
```

serde refuses `["",0]` with those two messages joined by `; `, reads `["a",1]` as it did, and `Pair("a".to_string(), 1)` still writes `["a",1]`. The Zod schema is `z.tuple([z.string().min(1, { error: ... }), z.number().int().max(4294967295).min(1, { error: ... })])`, the JSON Schema writes `{ "type": "string", "minLength": 1 }` and `{ "type": "integer", "minimum": 1 }` under `prefixItems`, and the TypeScript type stays `[string, number]`. The helpers are named by the slot's place among the ones declared, `validate_{index}_value` and `deserialize_{index}`: `validate_0_value` and `validate_1_value` here. A slot typed as a list is [counted](#list-length-minlength-maxlength), as a field is.

A struct of one slot is written as that slot's value, so its check is reported under no name: a `Lone(String)` held to `minLength = 1` answers `too short: minimum length is 1, got 0`, and under the key of a field that holds it. A `#[serde(transparent)]` struct of one slot is a [branded newtype](generics-and-brands.md#branded-newtype-validation-constraints), whose checks are written on the type: a `#[model_schema_prop]` on its slot is refused.

#### Constraints under `Option` and wrappers

A constraint describes the field's own value. `validate()` therefore reaches through everything that holds one value -- an `Option`, a transparent wrapper (`Box`, `Rc`, `Arc`, `Cow`), and every reference, whatever it borrows -- and checks the value under them, which is the same place the Zod, TypeScript and JSON Schema surfaces put the bound. A list is a value of its own: the walk stops at it, and [its length](#list-length-minlength-maxlength) is what a key written on it measures.

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Article {
    #[model_schema_prop(minLength = 3)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,   // checked inside the Some; a None is nothing to check

    #[model_schema_prop(minLength = 3)]
    pub tags: Vec<String>,          // at least three tags, each of any length
}
```

### Where a constraint is checked

As the value is read, and by `validate()`.

A bound written on a member is held beside the bounds its type declares, the type's reported first and the member's after, as the Zod schema reports them: a `Slug` brand with `minLength = 3` on a field that writes `pattern = "^[a-z]+$"` reports `'slug': too short: minimum length is 3, got 2` and then `'slug': does not match pattern '^[a-z]+$'` for `Slug("AB")`.

#### A read refuses what `validate()` refuses

A type that publishes [`validate()`](#the-validate-method) and derives `Deserialize` is read in two steps: serde reads its members, and `validate()` judges the value read. `#[model_schema()]` writes that `Deserialize` itself, so every serde read answers an error for a payload whose value breaks a bound or a [check](checks.md#checks-check): `serde_json::from_str`, `serde_json::from_value`, a BSON document under either major of the `bson` library, and any other format. No object is handed back. The error holds what `validate()` answers of the value read, every message, joined by `; `:

```rust
#[model_schema(check = ["withinParcel"])]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parcel {
    pub fragile_item_codes: Vec<String>,
    #[model_schema_prop(minLength = 1)]
    pub item_label: String,
}

#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub boxed_parcel: Box<Parcel>,
    pub first_parcel: Parcel,
    pub parcel_list: Vec<Parcel>,
    pub parcels_by_dock: BTreeMap<String, Parcel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spare_parcel: Option<Parcel>,
}

serde_json::from_str::<Parcel>(r#"{"fragileItemCodes":["x","y"],"itemLabel":""}"#);
// Err: 'itemLabel': too short: minimum length is 1, got 0; 'fragileItemCodes.1': y is outside of itemCodeList
```

A type that holds another reads it with no judgement of its own, and the whole value is judged once, by the type the read was asked for. A manifest that holds that parcel in each of its five places is refused with every message of every parcel, each at its full path. The error is one line, broken here after each `; `:

```text
'boxedParcel.itemLabel': too short: minimum length is 1, got 0;
'boxedParcel.fragileItemCodes.1': y is outside of itemCodeList;
'firstParcel.itemLabel': too short: minimum length is 1, got 0;
'firstParcel.fragileItemCodes.1': y is outside of itemCodeList;
'parcelList.1.itemLabel': too short: minimum length is 1, got 0;
'parcelList.1.fragileItemCodes.1': y is outside of itemCodeList;
'parcelsByDock.north.itemLabel': too short: minimum length is 1, got 0;
'parcelsByDock.north.fragileItemCodes.1': y is outside of itemCodeList;
'spareParcel.itemLabel': too short: minimum length is 1, got 0;
'spareParcel.fragileItemCodes.1': y is outside of itemCodeList
```

The messages come in the order `validate()` reports them: member by member as declared, a type's own check after its members. The text is the same from a string, from a `serde_json::Value` and from a BSON document. What each library writes around it is its own: `serde_json` adds no position to a refusal of the value it was asked for, and version 3 of the `bson` library writes it inside a sentence of its own, `BSON error. Kind: A deserialization-related error occurred. Message: ... .`

- **A refusal serde raises itself is serde's, unchanged.** A parcel holding `"itemLabel": 5` is refused with `invalid type: integer `5`, expected a string at line 1 column 39`, and with the same sentence at its own position where a manifest holds it: no value was read for `validate()` to judge.
- **A payload that keeps every rule** is read to the value it was read to before, and written back to the same text. Writing judges nothing: `Serialize` is still serde's derive, and a broken value built in code is written as it is.
- **A type that publishes no `validate()`** is left as it was written, with serde's own `Deserialize`, unless it names a [`preprocess`](preprocess.md#preprocessing-preprocess), on itself or on a member: its `Deserialize` is written too, and calls each function ahead of the read. A type that derives `Serialize` alone is left as it was written.

**Where a held model is judged.** A holder reads a model unjudged where `validate()` walks it: held itself, or under `Option`, `Vec`, `Box`, `Rc`, `Arc`, a fixed array, a set, a deque, a heap, as a map's value and as a position of a tuple, in any nesting of those, in a named field, a slot or a `#[serde(flatten)]` member. That holds where the member's type is written with the holder's own type parameters, `entries: Vec<Entry<IdType>>` in a `Ledger<IdType>`, and where it is written with `Self`, `children: Vec<Self>`. A model held any other way judges itself as it is read, and the payload is refused there too, with that model's own messages, the first broken model ending the read:

| Held | Judged |
|---|---|
| in a field, an `Option`, a `Vec`, a `Box`, an `Rc`, an `Arc`, a fixed array, a set, a deque, a heap, a position of a tuple, a map's value, whether its type is written with a concrete filling, with the holder's own type parameter (`Vec<Entry<IdType>>`) or as `Self` | once, with its holder, each message at its full path |
| filling one of the holder's own type parameters, as what fills `Held` in `held: Vec<Held>` | by itself, as it is read |
| in a member whose type names a lifetime | by itself, as it is read |
| in a member written with a type parameter that a bound of the holder ties to another of its parameters or to a lifetime (`where IdType: From<Held>`), or with one of a holder read at `'static` | by itself, as it is read |
| in a field read through a `deserialize_with` or a `with` of your own | by whatever that function calls |
| by a type with no `#[model_schema()]` | by itself, as it is read: two broken parcels in a `Vec<Parcel>` of such a type answer the first one's messages, `... at line 1 column 58` |

**A branded newtype is read the same way.** A [brand](generics-and-brands.md#branded-newtypes) that carries a `pattern`, a `minLength` or a `maxLength`, or names a [check](checks.md#checks-check), publishes `validate()`, and its read is its own read followed by `validate()`. A type that holds the brand reads it with no judgement, in the places of the table above, and names each broken one at its full path:

```rust
#[model_schema(minLength = 2, check = ["labelInUpperCase"])]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Label(String);

fn label_in_upper_case(value: &Label) -> Result<(), Vec<String>> {
    if value.0.chars().any(char::is_lowercase) {
        return Err(vec!["a label is upper case".to_owned()]);
    }
    Ok(())
}

#[model_schema(no_display, minLength = 3)]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Dir(PathBuf);

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Listing {
    pub listed: Vec<Dir>,
}

serde_json::from_str::<Label>(r#""a""#);
// Err: too short: minimum length is 2, got 1; a label is upper case

serde_json::from_str::<Listing>(r#"{"listed":["/a","/etc","/b"]}"#);
// Err: 'listed.0': too short: minimum length is 3, got 2; 'listed.2': too short: minimum length is 3, got 2
```

A brand that carries no bound and names no check publishes no `validate()`, and keeps the `Deserialize` serde's derive writes it. One position reads a brand held to its bound while the members are still being read: [a member of an `#[serde(untagged)]` enum](#what-is-checked-while-the-members-are-read).

**What `validate()` is for.** A value built in code, and a value changed after it was read:

```rust
let mut parcel: Parcel = serde_json::from_str(payload)?;
parcel.fragile_item_codes.push("y".to_string());
parcel.validate();
// Err(["'fragileItemCodes.1': y is outside of itemCodeList"])
```

**What the macro writes.** `#[derive(Deserialize)]` is taken off the item, and serde's derive runs on a private copy of it, declared inside a function the type gains: `model_schema_read_unjudged`, the read alone, public so that a holder in another module can call it. The `Deserialize` of the type calls it and then `validate()`. The copy keeps the item's `serde` and `cfg` attributes and no other, so an attribute of another macro written on the item or on a member is not on the copy.

- `model_schema_read_unjudged` is a name of the type's own: a function of yours under that name on the same type is a duplicate definition (E0592). A type whose [`enable` list](recovering-decode.md#opting-in) holds `"decode_with"` gains `decode_with_read_unjudged` beside it.
- A brand that carries a `pattern`, a `minLength` or a `maxLength` gains `model_schema_validate_bound` beside `validate()`: what the bound alone answers of the value, the checks the brand names apart. A member of an `#[serde(untagged)]` enum is held to it as it is read, and the recovering decode lists the brand from a read held to it. It is a name of the type's own as well.
- The `Deserialize` is written under the bounds serde's derive writes. A container `#[serde(bound = ...)]` is used as written. Without one, each type parameter a read field names is asked `Deserialize<'de>`, a parameter used only in `PhantomData`, in a skipped field or in a field read through `with` or `deserialize_with` is asked nothing to read, a `default` asks `Default` of what it builds, and `'de` outlives each lifetime serde borrows for: a `&'a str`, a `&'a [u8]`, and what a `#[serde(borrow)]` names. A type that borrows for `'static` is read at `'static`. The bounds are those of the type as you wrote it: the hook the macro hangs on a constrained member of an `#[serde(untagged)]` enum, and the `default` it writes beside one, ask nothing more. So `listed: Vec<(u32, Key)>` under a `minLength`, in an enum that declares `Key`, asks `Key: Deserialize<'de>`, and `Option<Vec<Key>>` asks no `Default` of `Key`.
- A unit struct with a check gains the `validate()` call in the `Deserialize` the macro already writes for it.
- `#[serde(remote = ...)]`, `from`, `try_from` and `into` on a type that publishes `validate()` and derives `Deserialize` are refused, the macro having no place to put a judgement behind a conversion it does not write:

```text
error: model_schema: type `Sealed`: `#[serde(try_from = ...)]` puts a conversion of its author's between the type and what serde reads or writes for it, and `Sealed` publishes `validate()`, so `#[model_schema()]` writes its `Deserialize`: the read of its members, then `validate()`. It cannot put that judgement behind a conversion it does not write. Take `try_from` off the type and convert where the value is read, or take off the bounds and the checks that make the type publish `validate()`.
```

#### What is checked while the members are read

A field still generates both helpers into its schema module: `validate_{field}_value()`, which `validate()` calls, and `deserialize_{field}`, a serde hook an author may hang on a field of their own accord. A field read through a reference generates the validator alone. Two positions are checked while the members are read, ahead of the type's `validate()`, and both have to be:

- **A member of an `#[serde(untagged)]` enum**, a struct variant's named member and a newtype variant's slot alike, where whether the member is admissible is what chooses which variant the payload is. The check is part of reading the value rather than part of judging it, so the member's own bound chooses the variant as it does under `anyOf` and `z.union` on the two schema surfaces the same type publishes, and `validate()` cannot stand in for it, since by the time it runs the variant has already been chosen. A wrapped member's hook deserializes the member's own declared type and then runs the same walk `validate()` runs over it; the two differ in that `validate()` answers with every violation in the instance while a `Deserializer` answers with one line, so the read stops at the first value that broke a bound and hands serde that value's violations joined by `; `. A member read through a reference, a borrowed `&str` or a borrowed list among them, and a member its author reads through a `deserialize_with` or a `with` of their own, serde admitting one reader per field, are the constrained members that choose no variant: each is read with no bound written, the first variant whose shape fits taking the payload, and the enum's read then refuses the value where it breaks the bound.
- **A constrained brand held by a member of an `#[serde(untagged)]` enum**, where the same reasoning applies: the member is held to the brand's `pattern`, `minLength` and `maxLength` as it is read, so a value the brand's bound refuses takes that variant out of the running. A [check](checks.md#checks-check) the brand names chooses no variant, as no check does: the variant is chosen first, and the enum's read then refuses the value with the check's messages. Held anywhere else, a brand is read with no judgement, as a model is, and the holder's `validate()` judges it.

A rule of a model a member holds, its checks included, chooses no variant. The first variant whose shape reads the payload takes it, and the enum's read then refuses the value where the model breaks its rule. Zod's `z.union` and the JSON Schema's `anyOf` try the next variant there instead, so each of the two schema surfaces reads a payload serde refuses:

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Bale {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Loose {
    pub label: String,
}

#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum Picker {
    A(Bale),
    B(Loose),
}
```

`{"label":""}` read as `Picker` is `Picker::A`, the first variant whose shape reads it, and serde refuses it with `'label': too short: minimum length is 1, got 0`. `Picker$Schema.safeParse({ label: "" })` answers `{"success":true,"data":{"label":""}}`, the payload read as `B`, and the second branch of the JSON Schema's `anyOf` admits it. serde writes `Picker::B(Loose { label: String::new() })` as that payload, and does not read it back. A struct of one slot is such a model too: serde writes it as the slot's value, as it writes a brand, but a bound on its slot chooses no variant, so `0` read as `#[serde(untagged)] pub enum Chooser { A(Count), B(u32) }`, over `pub struct Count(#[model_schema_prop(minimum = 1)] pub u32);`, is refused with `too small: minimum is 1, got 0`, where Zod reads it as `B`. The [Dart](dart.md#the-dart-read-refuses-what-validate-refuses), [Swift](swift.md#a-swift-read-refuses-what-validate-refuses) and [Kotlin](kotlin.md#the-read-in-the-kotlin-client) reads take the variant serde takes, and refuse the value as serde does.

The [recovering decode](recovering-decode.md#recovering-decode-decode_with) lists what a plain read refuses. `from_value_with` and `from_value_piped`, `from_bson_with` and `from_bson_piped` read with the type's read alone, run each field's validator on the value serde read, and the validator of each slot of a tuple struct and of a tagged tuple variant, and list the violations of each value as one issue, which is what lets a callback or a resolver repair a stored value. The [MongoDB reads](mongodb-operations.md#rows-that-do-not-read-as-the-type) of a type that lists `"mongodb"` read each row the same way, so a row that breaks a bound is refused until a resolver repairs it. See [What each shape reports](recovering-decode.md#what-each-shape-reports). A [service's dispatcher](services.md#service-validation) reads a request with the read alone as well and calls `validate()` next, so a broken bound is a `failed-validation` fault with its field.

A field whose key may be left out keeps that reading. Where a member is read through a hook -- an untagged variant's, and a member whose model is read unjudged -- an `Option` written outermost (under any number of transparent wrappers) is given `#[serde(default)]` alongside it, since a `deserialize_with` otherwise turns a missing key into an error; a field that writes its own `default` keeps the one it wrote. Off the hook there is nothing to put back and no `default` is written, so a required key that goes missing is still the error it always was.

#### A bound the field's own type declares

A field carrying no `model_schema_prop` of its own may still hold a type that declares one -- a constrained brand, or a nested `#[model_schema()]` type. The enclosing `validate()` runs whatever validator that type published and writes its report under the field that held it:

```rust
#[model_schema(minLength = 3)]
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Slug(String);

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Enrolment {
    pub slug: Slug,
}
```

```rust
Enrolment { slug: Slug("a".to_string()) }.validate();
// Err(["'slug': too short: minimum length is 3, got 1"])
```

The brand's own report says `too short: ...` and names nothing, the brand being the value rather than a field of anything -- so the field's key is the message's to supply, written first and in single quotes, which is where a reader of these reports already looks for one: `'slug': too short: ...`. A nested type's report already names its own member, and the field's key is written *into* that name rather than in front of it: `'holds.name': too short: ...`. One quoted run carries the whole path, so a reader taking the first one gets the member that was actually wrong rather than the hop above it -- and it is the string the TypeScript schema published from the same declaration reports for the same payload.

A `#[serde(flatten)]` hop writes no key, so it contributes no segment: a bound under a flattened `claims` inside `account` reads `'account.jti': too short: ...`, which is what the wire carries and what TypeScript names.

This reaches through `Option`, `Box`/`Cow`/`Rc`/`Arc`, every reference and every level of a list, a fixed array included, to each item a list holds -- beside the count a key written on the list holds it to, and with the item's [index](#the-path-a-message-names) written after the field's key at each level -- and applies to a struct's field, to a named member of an enum variant under any tagging, and to the lone slot of an `#[serde(untagged)]` newtype variant. That last one contributes no segment either, for the same reason a flattened hop contributes none: what an untagged newtype member writes on the wire *is* the inner value, so a violation beneath it is already one of the enclosing object's own keys. So a union of newtype members publishes a `validate()` that dispatches to whichever variant the value holds, and a field holding such a union reports the path the payload spells -- `'account.jti': too short: ...` whether the account is a union or a plain struct.

#### A model held in a map, in a slot or in a tuple

`validate()` runs what each of these holds as well, and names it by the place the payload holds it at:

| Held | Reported under |
|---|---|
| as a value of a `HashMap`, of a `BTreeMap` and, under the [`indexmap` feature](features.md#feature-flags), of an `IndexMap` | the entry's key, as serde writes it |
| in a slot of a tuple struct, and of a tuple variant of a tagged enum | [the path serde writes the slot at](#the-path-a-message-names), as the check [written on the slot itself](#a-slot-of-a-tuple-struct) is: its position among several slots, nothing for the one slot of a struct, and the variant's name or the content key ahead of either |
| in a tuple written as a field's type, `(String, Parcel)` | its position in the tuple |

```rust
#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parcel {
    #[model_schema_prop(minLength = 1)]
    pub item_label: String,
}

#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Terminal {
    pub first_parcel: Parcel,
    pub parcels_by_dock: BTreeMap<String, Parcel>,
}

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Consignment(#[model_schema_prop(minLength = 1)] pub String, pub Parcel);

let unlabelled = || Parcel { item_label: String::new() };

Terminal {
    first_parcel: Parcel { item_label: "A".to_string() },
    parcels_by_dock: BTreeMap::from([("north".to_string(), unlabelled())]),
}
.validate();
// Err(["'parcelsByDock.north.itemLabel': too short: minimum length is 1, got 0"])

Consignment(String::new(), unlabelled()).validate();
// Err(["'0': too short: minimum length is 1, got 0", "'1.itemLabel': too short: minimum length is 1, got 0"])
```

The walk goes on below each of the three as it does below a field: through an `Option`, the transparent wrappers, every reference and every level of a list, and from a map into a list or another map. A model held behind a reference -- `&'a Parcel`, `Option<&'a Parcel>`, `&'a [Parcel]` or `&'a Option<Parcel>` -- is walked as the owned model is and named by the same path: a `first_parcel: &'a Parcel` answers `'firstParcel.itemLabel': too short: minimum length is 1, got 0`. serde reads no reference to a model, so no read hook is written for such a member. The entries of a map are reported in the order the map yields them: sorted for a `BTreeMap`, as inserted for an `IndexMap`, and in no fixed order for a `HashMap`. An empty map reports nothing. A map, a slot and a tuple that hold no declared type add nothing to `validate()`, and a type that holds nothing else publishes none.

**A key is the text serde writes for it**, read through the key's own `Serialize` with the serializer serde has kept for a `Formatter` since 1.0.108: a string as it is, a number and a `bool` as serde stringifies them, a plain enum under the name serde writes for the variant, a [brand](generics-and-brands.md#branded-newtypes) as its inner value, and a `chrono::NaiveDateTime` as serde writes it, `2026-10-13T04:17:00`, where the type prints `2026-10-13 04:17:00`. A map written as the whole value -- a `#[serde(flatten)]` map, and the lone slot of an `#[serde(untagged)]` variant -- names each entry by its key alone, `'north.itemLabel': ...`.

`validate()` is written under the bounds the type declares, so a map of models keyed by one of the type's own parameters is refused at expansion unless the type declares `Serialize` for that parameter: `struct Shelf<Key: Ord>` over a `by_key: BTreeMap<Key, Parcel>` does not build, and the message says `Key` declares no `Serialize` on `Shelf` and to write `Key: Serialize` on the type. Write it in the generics or in a `where` clause, and tell the derive not to write it a second time with `#[serde(bound(serialize = ""))]`, which clippy's `trait_duplication_in_bounds` asks for. `struct Rack<Key: Ord + Serialize>` over the same map answers `'byKey.north.itemLabel': ...`. A map whose values are no declared type is not walked, so `BTreeMap<Key, u32>` needs nothing.

A brand's rules and a nested `#[model_schema()]` type's bound are both the holder's `validate()` to report. The holder reads each with no judgement of its own, and its read then runs `validate()` over the whole value, which is also what refuses a message built in Rust, where no read ever ran, as every outbound message a generated client sends is.
