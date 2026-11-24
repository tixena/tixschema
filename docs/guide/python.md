### The Python target

With the `python` feature, a type declared with `#[model_schema]` writes `{type}_python::python_definition()`: the text of its Python declaration, with full type hints and a JSON codec written against the standard library alone, for Python 3.12 or later. The read takes what serde reads and refuses what serde refuses, in serde's words, and the write sends what serde writes, byte for byte. No package is installed beside it.

The target is new, and writes some shapes so far (below). A type of any other shape writes no `{type}_python` module, so a bundle that names its `python_definition()` stops at build time rather than holding a class that reads otherwise than serde.

#### Joining the types into a module

The module's first line is `from __future__ import annotations`. Each type's text follows, in any order:

```rust
let module = [
    parcel_python::python_definition(),
    size_python::python_definition(),
]
.into_iter()
.fold(String::from("from __future__ import annotations\n"), |mut joined, text| {
    joined.push_str("\n\n");
    joined.push_str(&text);
    joined
});
```

Each text imports the modules it uses under a name of its own, `import json as _Parcel_json`, and every other name it binds at module scope carries the type's published name, so the texts of every type of a crate join into one module with no name bound twice. The module passes `pyright` in strict mode.

#### A draft and a valid type

A struct is two classes, as in the other clients. With

```rust
#[model_schema()]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Parcel {
    pub label: String,
    pub item_count: u32,
    #[serde(default, skip_serializing_if = "::core::option::Option::is_none")]
    pub note: Option<String>,
    pub tags: Vec<String>,
    pub size: Size,
}

#[model_schema()]
#[derive(Debug, Serialize, Deserialize)]
pub enum Size {
    Small,
    Large,
}
```

`ParcelDraft` is the class you build: a frozen dataclass whose members are keyword-only, each under its Rust name, `note` defaulting to `None`. It has `from_json`, `from_json_value`, `to_json`, `to_json_value`, `validation_issues()` and `validate()`, which answers the valid type or raises `ParcelRefusal`, whose `issues` are what `validation_issues()` listed. `Parcel` is the valid type, which holds a draft that passed: `ParcelDraft(...).validate()`, `Parcel.of(...)` and `Parcel.from_json(...)` make one, and `Parcel()` raises `TypeError`. It has a property for each member, `to_draft()`, `to_json()`, `to_json_value()`, `from_json_value_unjudged`, which a holder reads it with, and `validation_issues()`.

```python
parcel = Parcel.of(label="crate", item_count=7, tags=["a"], size=Size.Large)
parcel.item_count              # 7
parcel.to_json()               # '{"label":"crate","itemCount":7,"tags":["a"],"size":"Large"}'
```

A plain enum, every variant of which serde writes as its name, is an `enum.Enum` whose members are the variants under their Rust names, each holding the name serde writes: `Size.Large.value == "Large"`. It reads the name, and an object of the name over `null`, as serde reads a unit variant.

A type `name = "..."` publishes under another name is written under that name, and its Rust ident is bound to it after it, `ParcelRecord = Parcel` and `ParcelRecordDraft = ParcelDraft`, so a type written above the rename resolves.

#### The read

`from_json(text)` reads JSON text, and `from_json_value(value)` a value already parsed. A payload serde refuses raises `ValueError(words, place)`: `words` are serde's, and `place` lists the key or index of each step to the value refused.

```python
Parcel.from_json('{"label":"box","itemCount":1.0,"tags":[],"size":"Small"}')
# ValueError('invalid type: floating point `1.0`, expected u32', ['itemCount'])
```

Where Python's `json` reads a text otherwise than serde, the read follows serde:

| Payload | serde, and the Python read |
|---|---|
| `1.0`, `1e2` or `-0` for an integer | ``invalid type: floating point `1.0`, expected u32``: serde reads each as a float |
| `true` for an integer | ``invalid type: boolean `true`, expected u32``, where Python holds `True` as an `int` |
| An integer beyond the type | ``invalid value: integer `4294967296`, expected u32`` |
| An integer beyond `i64` and `u64` | the float serde reads it as, refused as one for an integer and read for a float |
| A number beyond an `f64`, `1e400` | `number out of range`, at its place |
| `NaN` or `Infinity` | `expected value`, at its place, and `-Infinity` `invalid number` |
| A field written twice | ``duplicate field `label` ``, before its second value is read; a map reads each value in turn and keeps the last, as serde's does |
| A key no member is read under | ``unknown field `stray`, expected one of `label`, `itemCount`, ...``, unless the type is `nonstrict` |
| A member left out | ``missing field `label` ``, placed at the member; an `Option` reads `None` |
| An `f32` | the number rounded to an `f32`, held as the shortest digits that read back as it, which is what Rust displays; one of `3.4028235677973366e38` or more in size reads as an infinity of its sign |

The read walks an object's keys in the order the text writes them, as serde's derive does, so of two faults in one payload the one the text writes first is refused, wherever its member is declared, and a member left out is refused only once every key is read, the first in the order declared. With `{"tags":5,"label":5}`, both refuse `tags`. A key a `nonstrict` struct reads past is skipped as serde skips it: refused at the key where its value holds `NaN`, `Infinity` or `-Infinity`, and read past where it holds a number beyond an `f64`.

A struct is read from an object alone, as the Dart, Swift and Kotlin reads read it, where serde also reads one from an array of its members in order.

#### The write

`to_json()` writes what serde writes: no space, each text escaped as serde escapes it, each float spelled as serde spells it, `0.00001` where Python's `json` writes `1e-05`, and a `BTreeMap` in the order of its keys, whatever order it was read or built in. A `HashMap` or an `IndexMap` is written in the order the `dict` holds its keys. `to_json_value()` answers the same value, unwritten. A value no form serde reads holds is refused before it is written, with `ValueError`: an integer outside its type's range, a `bool` where an integer is declared, and a float that is not finite, or for an `f32` one of `3.4028235677973366e38` or more in size.

#### Names

A member keeps its Rust name, which is already snake case. One named after a word Python refuses as a name, one of its 35 hard keywords, takes a trailing underscore, as PEP 8 spells it, and so does one named after a member every class writes for itself (`draft`, `from_json`, `from_json_value`, `from_json_value_unjudged`, `of`, `to_draft`, `to_json`, `to_json_value`, `to_valid`, `validate`, `validation_issues`). A soft keyword, `match`, `case`, `type` or `_`, names a member as it is. The key on the wire stays the one serde writes: `pub r#class: String` is `class_: str`, read and written under `class`. A variant named after a keyword is moved the same way: `None` is `Mode.None_`, written `"None"`.

#### The shapes it writes so far

- A struct of named members, with no type parameter, whose members are a `String`, a `bool`, any integer up to 64 bits, an `f32` or an `f64`, a `Vec` of one, an `Option` of one, a `HashMap` or `BTreeMap` keyed by `String`, or a type the crate declares that writes Python, at any depth of those.
- A plain enum, every variant of which is a unit variant serde writes as its name.
- On a struct, `rename_all`, `rename` and `deny_unknown_fields`, and `nonstrict`; on a member, `rename`, `skip`, `#[model_schema_prop(nullable)]`, and `default` and `skip_serializing_if` on an `Option`; on a variant, `rename`.

Every other shape writes no module yet: a rule (a bound, a `check`, a `literal`, an `as`) or a `preprocess`, a type parameter, a tuple struct, a unit struct, a struct with no member, a brand, an alias, an enum with a variant that holds a value or written under a tag, `flatten`, `alias`, a `default` on a member that is no `Option`, a map keyed by any other type, a set, an array of a fixed length, a `Vec<u8>`, a `char`, a jiff or chrono date or time, an `ObjectId`, a `Result`, a range, and the types a service writes for itself. A type declared after one of these that holds it writes none either.
