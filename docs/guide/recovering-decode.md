## Recovering Decode (`decode_with`)

A type that opts in implements `DecodeWith`, the crate's trait of the methods that read a value the caller already holds: `from_value_with` and `from_value_piped` read a `serde_json::Value`, and, in a build with tixschema's `bson` feature on, `from_bson_with` and `from_bson_piped` read a `bson::Document`. A caller imports `DecodeWith` to call them, from the `{type}_schema` module of any flagged type of the crate: `use invoice_schema::DecodeWith;`. Serde reads the value with the type's read alone, ahead of the `validate()` [a plain read of the type ends on](validation.md#a-read-refuses-what-validate-refuses), so a broken bound and a broken check are issues to list and never a refusal that hides them. When serde reads it, the value is held in the form the type writes and it breaks no bound a field declares, the decoded value comes back and nothing else happens. When serde refuses it, when it carries a key the type does not declare, when a value reads but is held in another form than its field writes, or when a value reads but breaks a bound its field declares, the raw value and every issue found in it are handed over, once. A `with` method hands them to a callback supplied with that call, which rejects the record or fixes the raw value. A `piped` method hands them to [resolvers](#resolvers) supplied with that call, small functions that each answer one issue at a time. Either way serde then gets one more read, the same way.

The entry `"decode_with"` implements the trait. The types the callback and the resolvers work with are the crate's [support library](#the-support-libraries), which the crate writes once, at its root, and every flagged type's module re-exports. A generated dispatcher reads a request with the read alone and then calls `validate()`, as [Service Validation](services.md#service-validation) says, and in a build without the `mongodb` feature nothing tixschema generates calls them. With `mongodb` on, a type that lists `"mongodb"` also reads and writes its own rows in a collection, and those reads go through `from_bson_piped`: see [MongoDB Operations and Typed Filters](mongodb-operations.md#mongodb-operations-and-typed-filters).

### Opting in

Write the `enable` list among the type's `#[model_schema(...)]` arguments. It takes four entries, `"decode_with"`, `"mongodb"`, `"arky"` and `"jsonschema"`:

- `#[model_schema(enable = ["decode_with"])]` gives the type the recovering decode of a JSON value, and, in a build with tixschema's `bson` feature on, of a BSON document. A type that is read and never stored lists this, whether it is read from a request or from a BSON document.
- `#[model_schema(enable = ["decode_with", "mongodb"])]` gives it, under tixschema's `mongodb` feature, the [MongoDB layer](mongodb-operations.md#mongodb-operations-and-typed-filters) as well. A type that is stored lists both. In a build without the `mongodb` feature the entry adds nothing and is accepted, so the declaration builds whatever features tixschema is built with.
- `#[model_schema(enable = ["arky"])]` gives it, under tixschema's `arky` feature, the [conversions to and from arky's frontmatter `Value`](arky.md#arky-frontmatter-conversions-arky). It needs no other entry, and is written beside them in any order.
- `#[model_schema(enable = ["jsonschema"])]` gives it `json_schema()` and `json_schema_within()`, its JSON Schema document. JSON Schema is no feature: a type asks for its document, and only a crate that asks depends on `serde_json` for it. Every model type it holds lists the entry too, since its document calls each held type's `json_schema_within`; a held type without it stops the build at the holder with `E0599`. Every type parameter of a type that lists it declares a default type ([Declaring the default type](generics-and-brands.md#declaring-the-default-type)). It needs no other entry.

A type without the list gains nothing: its expansion is what it was, with no method and no type added. This section calls a type that lists `"decode_with"` a flagged type, and the entry the flag. The crate that declares a flagged type writes [the initializers](#the-support-libraries) once, at its root.

```rust
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Stored: the BSON decode and the MongoDB layer.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Customer {
    pub name: String,
    pub open_invoices: u32,
}

/// Read and never stored: the recovering decode, from BSON as well under `bson`, and no MongoDB code.
#[model_schema(enable = ["decode_with"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceLookup {
    pub number: String,
}
```

What each entry generates on a type, by the features tixschema is built with:

| Entry | tixschema built with | Generates, per type |
|-------|----------------------|---------------------|
| `"decode_with"` | `serde` | An `impl` of `DecodeWith` with `from_value_with` and `from_value_piped`, their walkers, the type's own items in its `{type}_schema` module, and the re-export of the `decode_with` library there |
| `"decode_with"` | `bson`, which `mongodb` turns on | The row above, and `from_bson_with` and `from_bson_piped` in the same `impl`, the BSON walkers, and the BSON pieces of the `decode_with` library among the names the module re-exports. No operation, no `MONGO_FIELDS` and no typed path |
| `"mongodb"` | `mongodb` | The operations `find_one`, `find_one_with`, `find`, `find_with`, `count`, `insert_one`, `update_one`, `update_many`, `delete_one` and `delete_many`, `MONGO_FIELDS`, `MongoFields`, the typed paths, and the re-export of the MongoDB library: the query types and what the operations answer |

The BSON decode follows the `bson` feature alone: every flagged type has it in a build with `bson` on, whether it lists `"mongodb"` or not, so a type read from a BSON document and never stored lists `"decode_with"` alone. A type without `"mongodb"` gets none of the last row, under any feature, and a type that lists `"mongodb"` gets nothing of it in a build without the `mongodb` feature.

On a type `Record` that lists `"decode_with"`, in a build with `bson` on, the list generates this `impl`:

```rust
impl record_schema::DecodeWith for Record {
    fn from_value_with<F>(
        mut value: serde_json::Value,
        decide: F,
    ) -> core::result::Result<Self, record_schema::Unrecovered<serde_json::Value>>
    where
        F: FnOnce(&mut serde_json::Value, &[record_schema::Issue<serde_json::Value>]) -> record_schema::Verdict,
    { /* ... */ }

    fn from_value_piped(
        mut value: serde_json::Value,
        resolvers: &[record_schema::Resolver<'_, serde_json::Value, serde_json::Value>],
    ) -> core::result::Result<Self, record_schema::Unrecovered<serde_json::Value>>
    { /* ... */ }

    fn from_bson_with<F>(
        document: bson::Document,
        decide: F,
    ) -> core::result::Result<Self, record_schema::Unrecovered<bson::Bson>>
    where
        F: FnOnce(&mut bson::Document, &[record_schema::Issue<bson::Bson>]) -> record_schema::Verdict,
    { /* ... */ }

    fn from_bson_piped(
        document: bson::Document,
        resolvers: &[record_schema::Resolver<'_, bson::Document, bson::Bson>],
    ) -> core::result::Result<Self, record_schema::Unrecovered<bson::Bson>>
    { /* ... */ }
}
```

- The callback is the method's own type parameter, named `F` unless the type's declaration already writes that name, with its bound in a `where` clause. It is an `FnOnce`, so the compiler holds it to one run per read.
- The resolvers of a `piped` method are a slice, in the order they are to run: `&[&first, &second]`. An empty slice, `&[]`, is a read any issue refuses.
- `record_schema` is the module tixschema writes for `Record`. The callback's types and the resolvers' are the crate's support library, which `record_schema` re-exports name by name: `record_schema::Verdict` and `record_schema::Issue` are the crate's one `Verdict` and one `Issue` ([One library per crate](#one-library-per-crate)).
- `from_value_with` and `from_value_piped` are generated in a build with tixschema's `serde` feature on, and `from_bson_with` and `from_bson_piped` in a build with its `bson` feature on, which `mongodb` turns on, whether the type lists `"mongodb"` or not.
- `DecodeWith` is written once per crate, by `decode_with_foundation!()`, and every flagged type's module re-exports it. A flagged type of another crate implements that crate's `DecodeWith`, so a caller imports the trait from that crate's module. The reads are methods of the trait alone: a type has no inherent `from_value_with`, `from_value_piped`, `from_bson_with` or `from_bson_piped`, so a module that calls one imports the trait, and a call keeps its spelling, `Record::from_value_with(value, decide)`.

#### Reading several types through one function

`DecodeWith` is the bound that lets one function read every flagged type of a crate, through the same resolvers:

```rust
use bson::{Bson, Document};
use invoice_schema::{DecodeWith, Resolver};

const STORED: &[Resolver<'static, Document, Bson>] = &[&whole_number_as_text];

fn read<Row: DecodeWith>(document: Document) -> Row {
    Row::from_bson_piped(document, STORED).unwrap()
}

let invoice: Invoice = read(doc! { "number": "INV-0042", "openLines": "3" });
let order: Order = read(doc! { "lines": "3", "number": "ORD-0007" });
```

`whole_number_as_text` is a resolver as [Resolvers](#resolvers) writes one. `from_value_piped` reads a `serde_json::Value` through the same bound.

The list is read in every build. Each of these is refused, spanned on what is written, with a message that says what to write instead. The error opens with the type it is written on, ``model_schema: type `Record`:``, and goes on with the message:

| Written | The message |
|---------|-------------|
| `decode_with`, `decode_with = true` or `decode_with = false`, the flag as it was written before the list | ``tixschema: `decode_with` is now an entry of the `enable` list: write `#[model_schema(enable = ["decode_with"])]` `` |
| `enable = ["mongodb"]` | ``tixschema: `"mongodb"` requires `"decode_with"`: write `enable = ["decode_with", "mongodb"]` `` |
| `enable = ["decode_wth"]`, an entry the list does not read | ``tixschema: unknown entry `"decode_wth"`; the entries are `"arky"`, `"decode_with"`, `"jsonschema"` and `"mongodb"` `` |
| `enable = ["decode_with", "decode_with"]` | ``tixschema: `"decode_with"` is written twice`` |
| `enable = []` | ``tixschema: the list is empty; remove `enable` `` |
| `enable = "decode_with"`, `enable = [decode_with]`, or any other form but a list of string literals | `` `model_schema` argument `enable` takes a list of string literals, written `enable = ["decode_with"]` or `enable = ["decode_with", "mongodb"]` `` |

`"mongodb"` without `"decode_with"` is refused because the MongoDB layer reads every row through the recovering decode, which the first entry generates.

### What your crate lists

The generated code names `serde_json`, and, in a build with tixschema's `bson` feature on, the code of every flagged type and the library `decode_with_foundation!()` writes name the `bson` library, for `from_bson_with`. So the crate that declares a flagged type lists `serde_json` in its own manifest beside `serde`, and lists `bson` too wherever tixschema is built with `bson`:

```toml
serde_json = "1"
# for from_bson_with, at the major version your MongoDB driver uses
bson = "2.15"
```

List the `bson` library at the major version your MongoDB driver uses, so the `Document` the driver hands you is the `bson::Document` the method takes. The generated code calls only what both major versions of the library have, and this repository's own tests build it against each, beside a MongoDB driver built for that version: `bson = "2.15"` in the crate's own `tests/`, and `bson = { version = "3.1", features = ["serde"] }`, with version 3's `serde` feature turned on, in `bson3/`, a package of its own.

With tixschema's `mongodb` feature on, the generated code names the `mongodb` driver too, in every crate where a type lists `"mongodb"`, for the operations the type gains. [The driver your crate lists](mongodb-operations.md#the-driver-your-crate-lists) has the lines for each major version of `bson`.

A crate whose types list `"decode_with"` alone, and that writes `decode_with_foundation!()` alone, names no `mongodb` path and lists no driver, whatever features tixschema is built with. It names `bson` wherever tixschema's `bson` feature is on, and so lists it there. That includes a workspace where another crate turns tixschema's `mongodb` feature on, which Cargo then turns on, `bson` with it, for every crate of the build.

### The support libraries

What a callback, a resolver and a walker work with is the same for every flagged type, so a crate holds it once. Two initializers, each called once at the crate root, write it:

- `tixschema::decode_with_foundation!();` writes the module `__tixschema_decode_with`: `Issue`, `Expected`, `Path` and `Segment`, `Verdict`, `Resolution`, `Resolver`, `Stored`, `Unrecovered`, `unsettled`, and what one type's walker hands another's. Under `bson` it holds as well what reads a BSON value and the implementation of `Stored` for `bson::Document`, and it names no `bson` path in a build without `bson`. It writes nothing in a build without tixschema's `serde` feature.
- `tixschema::mongodb_foundation!();` writes the module `__tixschema_mongodb` under tixschema's `mongodb` feature: the query types (`Filter`, `Update`, `ArmFilter`, `ArmUpdate`, the path kinds) and what the operations answer (`Read`, `OperationError`, `WriteError`). It reads the names of the first library, so it is written beside `decode_with_foundation!()`. In a build without `mongodb` it writes nothing, so the call stays in a crate whatever features the crate builds with.

A crate whose types list `"decode_with"` writes the first, and a crate where one lists `"mongodb"` writes both. Each is written once per crate, at the crate root (`src/lib.rs` or `src/main.rs`), on its own line, anywhere outside a function, after the `mod` lines as well as before them:

```rust
// src/lib.rs
tixschema::decode_with_foundation!();
tixschema::mongodb_foundation!();

pub mod billing;
```

- **Neither takes an argument.** Anything written between the parentheses is refused: ``tixschema: `decode_with_foundation!` takes no arguments: write `tixschema::decode_with_foundation!();` at the crate root``, and the same for `mongodb_foundation!`.
- **Each is written once.** Written twice, an initializer declares its module and its marker twice, which rustc refuses (`E0428`, `` the name `__tixschema_decode_with` is defined multiple times ``).
- **Each crate has its own.** A library, a binary, each test binary, each example and each doctest is a crate, and writes the initializers at its own root. A doctest has a root of its own only where it writes its own `fn main`: one that does not is wrapped in a `fn main`, and an initializer inside a function writes no module at the crate root.
- **Every `{type}_schema` module re-exports its crate's libraries, name by name**: the `decode_with` library always, its BSON pieces included under `bson`, and the MongoDB library where the type lists `"mongodb"`, under `mongodb`. So every path written against a type's module resolves, `record_schema::Verdict::Reject`, `invoice_schema::Issue`, `invoice_schema::Filter<Invoice>`, from the crate and from another one. A name written out shadows the one the module reads from your scope: in a module of yours that writes `use std::path::Path;` or declares a struct `Filter` of its own beside a flagged type, `Path` and `Filter` stay yours, and `record_schema::Path` and `invoice_schema::Filter` are the library's.

Nothing tixschema emits into your crate carries `#[allow]`, `#[expect]` or `#[doc(hidden)]`, so your lint levels reach all of it, with two exceptions, both on the libraries:

- **Each re-export in a `{type}_schema` module carries `#[expect(clippy::pub_use, reason = "...")]`.** The re-export is a `pub use`, which clippy's `pub_use`, a lint of its restriction group, reports wherever that group is denied. It is what keeps every `{type}_schema::Name` path reachable from your crate and from another, so tixschema writes the expectation, with its reason, rather than leaving the lint to fire at every flagged type. Neither `pub_use` nor an unmet expectation is reported, in `cargo build`, in clippy at the lint's default level, or in clippy with the restriction group denied.
- **Each library module, `__tixschema_decode_with` and `__tixschema_mongodb`, carries `#[doc(hidden)]`**, so the library is not published in your crate's documentation as a module of its own. Its items are documented where you name them, under each `{type}_schema` module that re-exports them.

### A crate without its initializer

Each flagged type checks that its crate wrote the libraries it re-exports. In a crate that did not, the build fails at each such type with an error that names the line to add and where to add it. With no `decode_with_foundation!()`, beside rustc's own error for the re-export it cannot resolve (`E0432`, `` could not find `__tixschema_decode_with` in the crate root ``), each flagged type reports an `E0277` whose message, label and note are:

```text
message: tixschema: this crate has `#[model_schema(enable = ["decode_with"])]` types but no `tixschema::decode_with_foundation!();`
label:   this type needs the decode_with support library, which `tixschema::decode_with_foundation!();` writes once per crate
note:    Fix: add the line `tixschema::decode_with_foundation!();` at the crate root (src/lib.rs or src/main.rs), on its own line, anywhere outside a function. Add it once per crate; nothing else is needed.
```

With `decode_with_foundation!()` and no `mongodb_foundation!()`, in a build with `mongodb` on, each type that lists `"mongodb"` reports:

```text
message: tixschema: this crate has `#[model_schema(enable = ["decode_with", "mongodb"])]` types but no `tixschema::mongodb_foundation!();`
label:   this type needs the MongoDB support library, which `tixschema::mongodb_foundation!();` writes once per crate
note:    Fix: add the line `tixschema::mongodb_foundation!();` at the crate root (src/lib.rs or src/main.rs), on its own line, anywhere outside a function. Add it once per crate; nothing else is needed.
```

The fix is the line the note gives. The check is the same wherever the initializer is written at the crate root, so a crate whose `mod` lines come first and whose initializers come last builds.

### One library per crate

A crate holds its libraries once, and every flagged type of it shares them:

- **Every path written against a type's module compiles**: `invoice_schema::Verdict::Reject`, `invoice_schema::Issue`, `invoice_schema::OperationError`.
- **Within one crate, `invoice_schema::Issue` and `customer_schema::Issue` are one type**, and so is every other name of the libraries: `Verdict`, `Resolution`, `Resolver` and `OperationError` among them. A type of another crate has its own crate's.
- **One resolver serves every flagged type of its crate.** A resolver written against `invoice_schema::Issue` is handed to the `piped` read of any flagged type the crate declares, and to `find_one_with` and `find_with` of any that lists `"mongodb"`.
- **Two impls written for two modules' types are one impl written twice**, which rustc refuses. Keep one of them:

```rust
// two impls of one crate, each written for one module's `OperationError`
impl From<invoice_schema::OperationError> for StoreError {
    fn from(failed: invoice_schema::OperationError) -> Self { Self::Stored(failed.to_string()) }
}
impl From<customer_schema::OperationError> for StoreError {
    fn from(failed: customer_schema::OperationError) -> Self { Self::Stored(failed.to_string()) }
}
// error[E0119]: conflicting implementations of trait `From<OperationError>` for type `StoreError`
```

- **A crate with no stored type names no `mongodb` path, and names `bson` only where tixschema's `bson` feature is on.** Its types list `"decode_with"` alone and it writes `decode_with_foundation!()` alone. Built without `bson`, its manifest lists neither library:

```rust
// src/lib.rs of a crate whose Cargo.toml lists tixschema, serde, serde_json, bson where tixschema
// has `bson` on, and no mongodb
tixschema::decode_with_foundation!();

#[tixschema::model_schema(enable = ["decode_with"])]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct InvoiceLookup {
    pub number: String,
}
```

- **A doctest that declares a flagged type writes its own `fn main`**, with the initializers at its crate root beside it.

### One call, one chance

A read is one call. What it finds is handed over once, to a callback or to resolvers, and the value is then read one more time.

1. Serde reads the value, with the type's read alone, and the walker walks it. When serde reads it and the walk finds no issue, the decoded value is returned, and neither the callback nor any resolver is called.
2. Otherwise every issue the walk found is handed over, with the raw value, mutable. A record serde reads still gets there when it carries a key its type does not declare, a value held in another form than its field writes, or a value that breaks a bound its field declares.
3. `from_value_with` and `from_bson_with` call their callback once, with the whole list. `Verdict::Reject` fails the read with `Unrecovered`, carrying that list. `Verdict::Fixed` goes on to the second read.
4. `from_value_piped` and `from_bson_piped` run their resolvers in the order given, each over one issue at a time. A resolver answers an issue `Settled`, it repaired the raw value; `Rejected`, the issue is known and is not to be repaired; or `NotTouched`, the issue is not its own. An issue answered `Settled` or `Rejected` is handed to no later resolver, and one left `NotTouched` goes on to the next. Once the last resolver has run, any issue that is not `Settled` fails the read with `Unrecovered`, carrying only those issues, in the order the walk found them. With every issue settled, the second read follows.
5. The second read: serde reads the value as it was left, the same way, and the walker walks it again. A read with no issue returns the value. Anything else fails the read with `Unrecovered`, carrying the second walk's list. Neither the callback nor a resolver is called again: an issue a repair uncovers fails the read.

The result is the decoded value or the failure. Nothing reports whether the callback or a resolver ran, or what it changed: a caller that wants to know records it inside its own callback or resolver.

### The issues

The callback's types and the resolvers' are named through the flagged type's own module, `record_schema` for `Record`, which re-exports them from the crate's [support library](#the-support-libraries). An `Issue<V>` is generic over the raw value: `serde_json::Value` from `from_value_with` and `from_value_piped`, `bson::Bson` from `from_bson_with` and `from_bson_piped`. One callback receives every issue in the whole record as the outer type's own `Issue`, an issue inside a nested type included, at its full path, and so does each resolver, one issue at a time.

| `Issue` member | What it says | Example |
|----------------|--------------|---------|
| `Invalid { path, expected, found, reason }` | The value is there, and either serde refuses it for its field, or serde reads it and it breaks a `#[model_schema_prop]` bound its field declares, or a [check](checks.md#checks-check) refuses it. `reason` is serde's own message for the first, in the `bson` library's wording where the value is BSON, the bound's own sentence for the second, and the check's message after its path for the third | A version holding `"number": "2"` where `number` is an `i32`: `Invalid` at `versions[1].number`, expected `I32`. A `humanName` holding `""` under `minLength = 1`: `Invalid` at `humanName`, reason `too short: minimum length is 1, got 0` |
| `Missing { path, expected }` | The type requires the key, and it is not there | A record with no `name` where `name` is a `String`: `Missing` at `name`, expected `String` |
| `Unknown { path, found }` | The key is there, and the type declares no field by that name | A record holding `"legacyField": true`: `Unknown` at `legacyField` |
| `Mistyped { path, expected, found }` | serde reads the value, but it is held in another form than its field writes: another JSON shape, or another BSON type | An id held as the text `"6a7cc592ca0574e6efdfe217"` where an `ObjectId` writes `{ "$oid": "..." }`: `Mistyped` at `recordId`, expected `ObjectId` |
| `NoVariant { path, found, variants }` | An untagged enum none of whose variants reads the value. `variants` holds each variant's own list of issues, in the order declared | `{ "country": "one", "digits": "555" }` read as an untagged enum of `Email`, `Phone` and `Versioned`: one `NoVariant`, holding a list for each of the three |
| `Undescribed { reason }` | serde refuses the value and the walk found nothing that says why. `reason` is serde's own message | `{ "Circle": 5 }` read as an enum whose `Circle` variant holds named fields |

`Undescribed` is added when everything the walk listed is an `Unknown` or a `Mistyped`, or it listed nothing, and serde still refuses the value. A type whose `Deserialize` is written by hand earns it, and so does an undeclared key under `#[serde(deny_unknown_fields)]`, listed beside its `Unknown`.

`Expected` describes a field's type as tixschema classifies it:

- one member per plain category: `Boolean`, `Char`, `String`, `I8` to `I64`, `Isize`, `U8` to `U64`, `Usize`, `F32`, `F64`, `DateTime`, `NaiveDate`, `NaiveDateTime`, `NaiveTime`, `ObjectId`, `Unknown`, and the three literals `BooleanLiteral(bool)`, `NumberLiteral(f64)` and `StringLiteral(&'static str)`;
- `Model(&'static str)`, a `#[model_schema]` type by its Rust name as the field's type writes it, and `TypeParam(&'static str)`, one of the type's own type parameters by its name;
- `Array(Box<Expected>)`, `Optional(Box<Expected>)` and `Map(Box<Expected>)`, each carrying what it holds, a map the type of its values, and `Tuple(Vec<Expected>)`, carrying its positions in order: `Array(Model("Version"))`, `Optional(ObjectId)`, `Tuple([String, U32])`;
- `Range(&'static str, Box<Expected>)`, a std range by the name it is declared under in `core::ops` and the type of its bounds: `Range("RangeInclusive", Box::new(U32))`;
- `Variants(&'static [&'static str])`, the tag values a tagged enum accepts, for a tag naming none of them, and the two arms of a `Result`, `Variants(["Ok", "Err"])`, for an object that names neither or both.

`Path` is where an issue sits: `Path(pub Vec<Segment>)`, a `Segment` being `Key(String)` or `Index(usize)`. It displays as `versions[1].number`, and the empty path is the value itself.

`Stored` is the trait a callback or a resolver repairs the raw value through. It is implemented for `serde_json::Value` and, with the `bson` feature, for `bson::Document`, and each module of a type of the crate re-exports it. `Stored::Value` is what one place inside the raw value holds: `serde_json::Value`, or `bson::Bson`.

- `set_at(&mut self, path: &Path, value: Self::Value) -> bool` puts `value` at the path, inserting the last key when it is absent, and answers whether it changed anything. On a JSON value the empty path replaces the whole value. A parent that is not there, or an index past the end of its list, answers `false`. A path into a document starts at a key, so the empty path sets nothing there.
- `remove_at(&mut self, path: &Path) -> bool` removes the key or the item at the path, and answers whether it did.
- `at(&self, path: &Path) -> Option<&Self::Value>` is what is at the path.
- `text`, `whole` and `real` read a value as a `&str`, an `i64` and an `f64`, and `from_text`, `from_whole` and `from_real` build the value that holds one. A document holds a whole number as `Int32` or `Int64`, and `from_whole` builds an `Int64`.

A resolver written over `D: Stored` serves a list of JSON resolvers and a list of BSON resolvers alike:

```rust
use invoice_schema::{Expected, Issue, Resolution, Resolver, Stored};

fn count_as_text<D: Stored>(raw: &mut D, issue: &Issue<D::Value>) -> Resolution {
    let Issue::Invalid { path, expected: Expected::U32, found, .. } = issue else {
        return Resolution::NotTouched;
    };
    let Some(text) = D::text(found) else {
        return Resolution::NotTouched;
    };
    match text.trim().parse::<i64>() {
        Ok(number) if number >= 0 && raw.set_at(path, D::from_whole(number)) => Resolution::Settled,
        _ => Resolution::Rejected,
    }
}

const FROM_JSON: &[Resolver<'static, serde_json::Value, serde_json::Value>] =
    &[&count_as_text::<serde_json::Value>];
const FROM_BSON: &[Resolver<'static, bson::Document, bson::Bson>] =
    &[&count_as_text::<bson::Document>];
```

**Breaking change in 0.2.0.** The four methods of `Path` that wrote at a path, a setter and a remover for a JSON value and a setter and a remover for a BSON document, are gone: `Stored` is the one way to write at a path. A resolver moves from a method of its path to a method of its raw value. Before, the repair above was two functions, one per kind of raw value, each writing through the method of `path` made for that kind:

```rust
fn count_as_text_json(raw: &mut serde_json::Value, issue: &Issue<serde_json::Value>) -> Resolution {
    // ... the match and the parse, then a method of `path` that writes a JSON value into `raw`
}

fn count_as_text_bson(raw: &mut bson::Document, issue: &Issue<bson::Bson>) -> Resolution {
    // ... the same match and parse, then a method of `path` that writes a BSON value into `raw`
}
```

After, with `Stored` in scope, it is the one `count_as_text` above, which writes with `raw.set_at(path, D::from_whole(number))`. A resolver written against one kind changes its write to `raw.set_at(path, value)`, and its removal to `raw.remove_at(path)`.

`Verdict` is the callback's answer, `Reject` or `Fixed`. `Resolution` is a resolver's answer for one issue, `Settled`, `Rejected` or `NotTouched`. `Resolver<'r, D, V>` is the type a `piped` method takes each resolver as, `&'r (dyn Fn(&mut D, &Issue<V>) -> Resolution + Sync)`: `D` is the raw value, `serde_json::Value` or `bson::Document`, and `V` what an issue holds. `unsettled(raw, issues, resolvers)` is the pipe a `piped` method runs, a public function of the module: it runs the resolvers in order and answers the issues none settled.

`Unrecovered<V>` is the failed read, with its issues in `pub issues: Vec<Issue<V>>`. It implements `Debug`, `Display`, one line per issue with its path first, and `std::error::Error`, so `?` carries a failed read into the caller's own error type.

Each of these types is `#[non_exhaustive]`: inside the crate that declares the flagged type a `match` may list every member, and a callback written in another crate needs a wildcard arm.

### What each shape reports

The walker reads each type in the form serde writes it. A type held in another form than the one its walker walks, a struct as something other than an object or a tuple struct as something other than an array, is read whole with the type's own reader, and what is listed for it is serde's verdict: `Invalid` where serde refuses it, and `Mistyped` where serde reads it in a form the type does not write. A struct held as text is `Invalid`; one held as an array of its fields in order, which serde reads, is `Mistyped`.

**A struct with named fields** is an object, each field looked up under the name serde reads it under, after every `rename` and `rename_all`. A renaming written as a list counts by its `deserialize` side: every build that describes the type, a mobile client included, refuses a list whose two sides name two keys, and a build with no schema surface and no client takes it.

- A key that is there is read with the function serde's derive calls for the field: the field type's own `Deserialize`, or the field's `deserialize_with` function or `with` module. A refusal is `Invalid`.
- A key that is absent is `Missing`, unless serde reads its absence: an `Option` field, a `#[serde(default)]` field or container, or a `#[serde(skip_deserializing)]` field.
- A key stored under one of a field's `#[serde(alias)]` names counts as that field, and an issue with it is listed at the key it is stored under.
- A key the type does not declare is `Unknown`. A key the type writes and never reads back, under `skip_deserializing`, is one it declares, under the name serde writes it under.
- A field whose type is another flagged type is walked by that type's walker at the extended path, through a list, a map, an `Option` or a `Box`: `versions[1].number`, `byName.first.number`, `latest.draft`. An optional one is walked when its key is there and does not hold `null`.
- A value that reads is written back the way its field writes it, through the field's `serialize_with` function or `with` module when it has one, and compared with what is held. A mismatch is `Mistyped`.
- A `#[model_schema_prop]` bound on a field is no part of the read the recovering decode makes, the type's read alone, so serde reads a value that breaks it. The walker runs the field's validator on what serde read, and lists one `Invalid` at the value, with the bound's sentence as its `reason`: `humanName` holding `""` under `minLength = 1` lists `Invalid` at `humanName`, reason `too short: minimum length is 1, got 0`. A value that breaks two bounds lists one issue at its path, its reasons joined by `; `, a list that breaks its count is one issue at the list (`tags`, whatever its items are), and a field of a tagged enum's variant is listed the same way. So is a slot of a tagged tuple variant, at the slot's own path: `Pair[1]` for the second slot of an externally tagged `Pair`, and the key the variant's value sits under for a lone slot. And so is a slot of a tuple struct: `[1]` for the second slot of a `struct Pair(String, u32)` read as the value itself, `pair[1]` where a field `pair` holds it, and the path the struct sits at for the one slot of a single-slot struct. A callback or a resolver repairs the value at the issue's path. The walker runs the field's validator in every build.
- A [check](checks.md#checks-check) the type or one of its fields names takes the typed value, which the walker never holds, so the checks run once serde has read the value: after the first read where serde accepts it, and after the read that follows a `Fixed` verdict or the resolvers. Each message is an `Invalid` of its own, expected `Unknown`. With `Parcel` of [the example](checks.md#checks-check), `{ "label": "A", "itemCodeList": ["x"], "fragileItemCodes": ["x", "y"] }` lists `Invalid` at `fragileItemCodes[1]`, reason `y is outside of itemCodeList`, and the same parcel held by `Manifest` lists it at `parcel.fragileItemCodes[1]`. The path is what the message leads with, read against the raw value: a segment that is a number where the raw value holds a list is an index, and every other segment is a key. A message with no lead is at the value itself, and `found` is `null` where the path leads nowhere. A member's check is listed where serde writes the member under each tagging of an enum, and the checks of a flagged type held in a field, an `Option`, a list, a map or a `Box` are listed under the key, and the index or the map key, it is held at. A payload serde refuses runs no check, no value having been built: a callback that fixes it meets the checks on the second read, and a `Fixed` verdict that leaves a check broken fails the read with `Unrecovered`, as it does for a bound. A check is listed in every build, as a bound is. Nothing is listed for a member serde never reads back, nor for what a field read through a hook of your own holds. A path is told apart by its dots, so a map key that holds one is read as two segments.

Read as a struct holding `name: String` and `versions: Vec<Version>`, `{ "name": "Loan", "versions": [{ "number": 1 }, { "number": "2" }], "legacyField": true }` lists `Invalid` at `versions[1].number`, expected `I32`, and `Unknown` at `legacyField`.

**A plain value inside a list or a map** is read on its own, at its own path, with its own expected type. Read as a struct holding `tags: Vec<ObjectId>`, `owners: HashMap<String, ObjectId>` and `counts: Vec<i32>`, an id held as text is `Mistyped` at `tags[1]` and at `owners.alice`, each expecting `ObjectId`, and a number held as text is `Invalid` at `counts[1]`, expecting `I32`. A field holding something other than the list or the map it is written as is one `Invalid` at the field. An optional plain value is read whole at its field, expecting `Optional(ObjectId)`.

**A tuple struct and a tuple field** are walked by position, each position a `Segment::Index`. Read as `struct Pair(String, u32)`, `["a", "x"]` lists `Invalid` at `[1]`, expected `U32`, and `["a"]` lists `Missing` at `[1]`. A position the type does not declare is `Unknown`: serde refuses a JSON array longer than the tuple, so `["a", 1, true]` lists `Unknown` at `[2]` beside an `Undescribed`, and serde reads past the position in a BSON list, where the `Unknown` is the only issue. A field `spot: (String, u32)` holding `[5, 1]` lists `Invalid` at `spot[0]`, expected `String`.

**A brand and a single-slot tuple struct** are read as the one value they hold, at the path they sit at, and the issue names what they hold. With `id` a brand over `String`, a struct holding `"id": 5` lists `Invalid` at `id`, expected `String`. The read a brand is listed from holds the value to the brand's own bound, in every build: a brand declared `minLength = 3` lists `"ab"` as `Invalid`, and a [check](checks.md#checks-check) the brand names is listed beside it, from the value read. Over another flagged type it is walked as that type, whatever shape that type has: with `pinned` a brand over `Version`, `"pinned": { "number": "3" }` lists `Invalid` at `pinned.number`, and with `owner` a brand over a brand over `String`, `"owner": 5` lists `Invalid` at `owner`, expected `String`.

**A std `Result`** is walked as the externally tagged enum serde writes it as: an object whose one key, `Ok` or `Err`, names the arm, and what the arm holds is walked under that key. With `frontmatter: Option<Result<Headline, Garbled>>`, `"frontmatter": { "Ok": { "title": "" } }` lists `Invalid` at `frontmatter.Ok.title`, where `title` is under `minLength = 1`, and `"outcome": { "Ok": "seven" }` lists `Invalid` at `outcome.Ok`, expected `U32`. An object of both keys, of neither or of another key, and a value that is no object, is one `Invalid` at the field, expected `Variants(["Ok", "Err"])`. A `Result` an arm of which holds a value of one of the type's own parameters is read whole.

**A unit struct** is the `{}` tixschema makes it write, in which no key is its own: read as `struct Ping;`, `{ "x": 1 }` lists `Unknown` at `x`.

**A `#[serde(transparent)]` struct**, with a named field or with slots, is walked as the value of the one field serde reads it as, and no key is looked up under that field's name. Read as `struct Slug { text: String }` under `transparent`, `5` lists `Invalid` at the value itself, expected `String`. A slot serde does not read is no part of the value: with `tags` a `struct Tags(Vec<String>, #[serde(skip)] u8)` under `transparent`, `"tags": ["a", 7]` lists `Invalid` at `tags[1]`, expected `String`.

**A plain enum** is one value, read with its own reader. Read as `enum Status { Draft, Published }`, `"Archived"` lists `Invalid` at the value itself, expected `Model("Status")`, and `{ "Draft": null }`, which serde reads and the enum does not write, lists `Mistyped`. `from_bson_with` is generated on a plain enum as on every flagged type, and every call to it reaches the callback, a document never being the bare name the enum writes.

**A tagged enum** has its tag read first, and the variant the tag names is walked where that form writes it. The examples read a struct holding one enum of each form as `outline`, `fill` and `stroke`.

- Externally tagged, the enum is a unit variant's name as text, or an object whose one key names a variant over what it holds. A key naming no variant is `Invalid` at the enum's own path, holding the whole value: `"outline": { "Hexagon": {} }` lists it at `outline`, expected `Variants(["Circle", "Empty", "Label"])`. An issue inside a variant sits under the variant's key: `outline.Circle.radius`.
- Internally tagged, the tag is read from its key, and the variant's fields are walked in the same object. A tag naming no variant is `Invalid` at the tag's key: `"fill": { "color": "red", "kind": "Striped" }` lists it at `fill.kind`, expected `Variants(["Clear", "Solid", "Versioned"])`. An absent tag is `Missing` at that key, and an issue inside a variant sits beside the tag: `fill.number`. A variant that holds one flagged type hands that type the object without the tag's entry, which is what serde hands it.
- Adjacently tagged, the tag is read from one key and what the variant holds is walked under the other: `"stroke": { "data": {}, "kind": "Dashed" }` lists `Missing` at `stroke.data.gap`.
- A tag naming no variant that serde reads all the same, as the variant marked `#[serde(other)]`, is `Mistyped` at the tag.
- The content of a variant with named fields, held as something other than an object, is the one place the walk lists nothing: `{ "Circle": 5 }` carries serde's refusal alone, as an `Undescribed`.

**An untagged enum** is walked as the variant serde reads the value as, and that variant alone. Where serde reads it as none, the one issue is a `NoVariant` at the enum's path, holding each variant's own list in the order declared. A constrained member is read through the hook tixschema hangs on it, so a value its bound refuses takes that variant out.

**A variant's `alias`** is a tag serde reads the variant under, so a tag or a key stored as one names that variant: nothing is listed for the alias, an issue inside the variant is listed under the key it is stored under, `Round.radius` for a `Circle` stored as `Round`, and `Variants` lists each variant's name and then its aliases. **A variant serde never reads**, under `skip_deserializing` or `skip`, is no variant to the walker: a tag naming it is `Invalid`, `Variants` leaves it out, and an untagged enum holds no list for it in its `NoVariant`.

**A `#[serde(flatten)]` field** has its keys among the keys of the object that holds it, so its issues sit at that object's path, never under the name of the field. A flattened type is handed what serde hands it: that object without the keys the outer type's own fields are read under, and without the entries serde took for the flattened fields declared before it. A struct with named fields, a plain or externally tagged enum and an adjacently tagged enum take the entries serde reads for them; a struct that itself flattens a field, and an internally tagged or untagged enum are read from what is left and take nothing, so a type flattened after one is handed its keys as well. A map is the [one member of its object](flatten.md#intersection-types-serdeflatten). The examples read `struct Sheet`, which declares `title: String` and flattens an `audit: Audit` holding `createdBy` and `revision`, an `extra: Option<Extra>` holding `note` and `weight`, and a `paint: Paint`, an enum internally tagged under `kind`.

- A flattened flagged struct is walked by its own fields walker, and the keys it declares count as declared: `"createdBy": 7` lists `Invalid` at `createdBy`, and a key neither `Sheet` nor a type it flattens declares is `Unknown` once, `legacy` at `legacy`.
- A flattened `Option` of a flagged struct is read by serde as absent wherever serde does not read the struct from the object. With none of its keys there, nothing is listed. With `note` there and `weight` not, the object holds what the field would not write, and the one issue is a `Mistyped` at the object's own path, expected `Optional(Model("Extra"))`. Where serde reads it, it lists what its own walk finds. A flattened `Option` of an enum is absent when what names its variant is absent: the tag's key for an internally or adjacently tagged enum, every variant's key for an externally tagged one, and, for an untagged enum, whenever serde reads none of its variants from the object. The object's keys are then the outer type's to judge, so one nothing declares is `Unknown` as anywhere else.
- A flattened tagged enum is walked under its own keys in that object: a `kind` naming no variant is `Invalid` at `kind`, and an absent one is `Missing` there. An externally tagged enum is walked under the key naming its variant, and an adjacently tagged one under its tag and content keys.
- A flattened untagged enum whose variants are objects with keys of their own is walked as the variant serde reads from what it is handed, and is one `NoVariant` at the object's path, holding what it was handed, where serde reads none.
- A flattened map takes every key of its object, and each value is walked at its key: read as a struct that flattens a `HashMap<String, i32>`, `{ "a": "x", "b": 2 }` lists `Invalid` at `a`, and no key is `Unknown`.
- A flattened field its author's hook reads is read whole, through the hook, from the keys nothing else declares: one issue at the object's path. Where the hook reads an `Option`, serde reads it as absent wherever what it holds is refused, so nothing is listed for it.
- A flattened type that itself takes every key it is handed, an untagged enum with a map among its variants, takes none of the outer type's own. Read as `struct Scoreboard`, which declares `id: String` and flattens `enum Scores { Captioned(Captioned), Tallied(HashMap<String, i32>) }`, `{ "a": 1, "id": "i" }` lists nothing. A key the outer type and a type it flattens both declare is the outer type's: the flattened type is handed an object without it.
- Beside a struct and a map, serde flattens a single-slot struct or a brand as the value it holds, and a plain enum as the key naming its variant, holding `null`. Each is walked as that, and the keys serde reads for it are its own: a single-slot struct over a map, over an `Option` of a flagged type, over a type parameter or over a JSON value is walked as a flattened field of that value is, and a plain enum, which builds flattened only where it is declared below the type that flattens it or in another crate, as an externally tagged enum whose variants hold nothing. A single-slot struct whose slot carries a hook of the author's is read whole with the struct's own reader, which runs the hook, and every key is its own; where that slot is an `Option`, nothing is listed for it. A single-slot struct over an `ObjectId`, which serde writes as an object, takes every key, and nothing is listed for it. Read as a struct that flattens `struct Extras(HashMap<String, i32>)`, `{ "a": "x" }` lists `Invalid` at `a`, and no key is `Unknown`. serde refuses to flatten a tuple struct and a single-slot struct over text, a list or a tuple, which build flattened only where they are declared below the type that flattens them or in another crate: nothing is listed for one, no key is its own, and the read carries serde's refusal as an `Undescribed`.
- Some flattened fields are walked by nothing: an `ObjectId`, which serde writes as an object and so flattens, and a second field that takes the keys nothing else declares, after the first one. Every key then counts as declared, so none is `Unknown`, and serde's verdict is the read's: a value serde refuses there is one `Undescribed`.
- A flattened field of a variant is walked in the object the variant's fields sit in, wherever the enum's form puts that object.

**A generic type** gets its walkers on `impl`s of their own, one per source, which bound every type parameter, and the type itself, `serde::de::DeserializeOwned`, and `serde::Serialize` as well for the BSON one. Its one `impl` of `DecodeWith` is written under the bounds of the last source, so in a build with `bson` on its JSON reads ask `serde::Serialize` of the parameters too. A type that itself bounds a parameter `Serialize` or `DeserializeOwned` has that bound written a second time on these `impl`s, which clippy reports at the declaration as `trait_duplication_in_bounds` (a `nursery` lint), as it already does for the `Serialize` bound serde's own derive repeats; leave the bound off the type and neither is reported, serde's derive and these `impl`s each adding what they need. A value of a parameter's type is read whole, with that parameter's own reader, and so is a list, a map, an `Option` or a tuple that holds one: read as `Page<Version>`, where `Page<T>` declares `items: Vec<T>`, a bad number inside one item is a single `Invalid` at `items`, expected `Array(TypeParam("T"))`. From a JSON value such a value is never `Mistyped`, nothing there being bound to write it back. From a BSON document it is: an id filling a parameter and stored as text is `Mistyped` at its field.

### Nested types

Every model type a flagged type's fields reach lists `"decode_with"` too, whatever its shape. Where the model type is declared above the flagged one and has no flag, the build fails: the flagged type's walker calls the nested type's walker, so the error names the method the flag would have generated:

```rust
tixschema::decode_with_foundation!();

#[model_schema()]
#[derive(Deserialize, Serialize)]
pub struct Version {
    pub number: i32,
}

#[model_schema(enable = ["decode_with"])]
#[derive(Deserialize, Serialize)]
pub struct Record {
    pub versions: Vec<Version>,
}
```

```text
error[E0599]: no associated function or constant named `decode_with_value_issues` found for struct `Version` in the current scope
```

The error is reported at the `#[model_schema(...)]` of the type that names the unflagged one. A type flattened without the flag earns the same error twice, naming `decode_with_value_named` and `decode_with_value_fields`, and, in a build with `bson` on, each of these errors comes a second time, for `decode_with_bson_issues`, `decode_with_bson_named` and `decode_with_bson_fields`.

**A type that lists `"mongodb"` may hold a model type that lists `"decode_with"` alone**, declared above it, below it or in another crate. Its BSON walk calls the held type's BSON walker, which every flagged type has under `bson`, and lists an issue inside the held type at its full path. With `Record` listing both entries and holding `current: Version`, and `Version` listing `"decode_with"` alone with `minimum = 1` on its `number`, `from_bson_with` lists `{ "current": { "number": 0 } }` as one `Invalid` at `current.number`.

- **A model type filling a type parameter needs no flag.** It is read as one value, so `Page<Plain>` reads with no flag on `Plain`.
- **A field typed with an alias of a flagged model type is walked**, the alias being that type: with `#[model_schema()] type EntryAlias = Entry;`, a field `featured: EntryAlias` lists an issue at `featured.name`.
- **A field typed with an alias of a list or a map is walked as that list or map**, where `#[model_schema]` is written on the alias and the alias is declared above the type that names it. With `#[model_schema()] pub type Versions = Vec<Version>;` and a field `all: Versions`, `{ "all": [{ "number": 1 }, { "number": "2" }] }` lists `Invalid` at `all[1].number`, and with `#[model_schema()] pub type Counts = HashMap<String, i32>;` and a field `counts: Counts`, `{ "counts": { "a": "x", "b": 2 } }` lists `Invalid` at `counts.a`, expected `I32`. The field publishes under the alias's name on every schema surface, as it does without the flag. The alias may be declared in another module: what it holds is reached from the field's own type, so nothing the alias is written with has to be in scope beside the field. An alias that takes a type parameter, and one declared below the type that names it, are read whole, as the next point says.
- **A field of a type tixschema has not seen is read whole.** A name `#[model_schema]` was not written on above the flagged type may be a `type` alias with no `#[model_schema]`, a type from another crate, or a model type declared below, and tixschema cannot tell which. So the walker asks the type for its walker, and where the type has none it reads the value whole, with the type's own `Deserialize`: a value serde refuses is one `Invalid` at the field, expected `Unknown`, and flattened, the field takes every key nothing else declares. With `pub type Properties = HashMap<String, u8>;` and a field `properties: Properties`, `{ "properties": { "a": 5 } }` reads with no issue and `{ "properties": 5 }` lists `Invalid` at `properties`. A flagged model type declared below is walked by its own walker all the same. An unflagged one declared below is read whole, where one declared above fails the build. A field `timeout: Duration`, `Duration` being `std::time::Duration`, is read whole too; the `duration_schema` error the same field earns on a type that lists `enable = ["jsonschema"]` is no part of the flag and is unchanged.

### What is refused

The entry `"decode_with"` is refused where the methods cannot be written, with a `compile_error!`, in every feature combination. No other shape is refused. What the `enable` list itself refuses is in [Opting in](#opting-in).

**On a type alias.** An alias is another name for a type that already exists, so the methods would be that type's. For `pub type DecodeVersions = Vec<DecodeVersion>;`:

```text
model_schema: type `DecodeVersions`: `enable = ["decode_with"]` cannot be written on a type alias: `DecodeVersions` is another name for `Vec<DecodeVersion>`, and the methods would have to be added to that type. List it on the model types the alias reaches.
```

**On a type that declares a lifetime.** The methods own the value they read, and a decoded value cannot borrow from it. For `pub struct DecodeNamed<'a> { pub name: &'a str }`, and for an enum declaring `'a` alike:

```text
model_schema: type `DecodeNamed`: `enable = ["decode_with"]` cannot be written on a type that borrows: `from_value_with` owns the value it reads, so the decoded `DecodeNamed<'a>` cannot borrow from it for `'a`.
```

**On a type with a field that borrows.** A reference written in a field borrows from the owned value as a declared lifetime does. For `pub struct DecodeLabelled { pub label: &'static str }`:

```text
model_schema: type `DecodeLabelled`: `enable = ["decode_with"]` cannot be written on a type with a field that borrows: `label` is `&'static str`, and `from_value_with` owns the value it reads, so nothing decoded from it can be borrowed for `'static`. Write the field as an owned type.
```

The message names the first such field serde reads, a reference held inside an `Option` or a `Vec` included, a tuple position by its number and an enum's field behind its variant: `Named.label`. A field that names a lifetime and borrows nothing from the value is not refused: a `Cow<'static, str>`, which serde reads as an owned value, and a reference in a field serde never reads, under `skip` or `skip_deserializing`.

### Reading from MongoDB

- **A row's `_id` is a key like any other.** Read into a type that does not declare `_id`, it is `Unknown`, and every such read goes to the callback or the resolvers. A type read from MongoDB declares `_id`, or the query projects it out with `_id: 0`, or a resolver settles the key by taking it out, as [Rows that do not read as the type](mongodb-operations.md#rows-that-do-not-read-as-the-type) shows.
- **A value stored as another BSON type than its field writes is `Mistyped`**, serde reading it all the same: a MongoDB query for the field's own type does not match such a value. Numbers count as one type, `Int32`, `Int64`, `Double` and `Decimal128`, strings and symbols as one, and every other BSON type matches only itself. An id stored as its hex text where the field is an `ObjectId` is `Mistyped`; a number stored as an `Int64` where the field writes an `Int32` is no issue.
- **A `chrono` `DateTime<Tz>` field writes text, or a number under `as_number`**, so a row holding a BSON date in it is `Invalid`, serde refusing it.
- **A reader on an older model** sends every record a newer model wrote with an added field to the callback, the added key being one its type does not declare.
- **The `reason` of an issue is serde's refusal or a bound's sentence.** For a value serde refuses it is the `bson` library's own wording, which differs between its two major versions. For a value that breaks a bound it is the bound's own sentence, the same under both. A callback or a resolver matches on the member, the path, `expected` and `found`, never on `reason`.
- **A value that breaks two bounds is one `Invalid` issue at its path**, its `reason` every violation in the order `validate()` reports them, joined by `; `. One repair of the value at that path answers it.
- **With tixschema's `mongodb` feature on, a type that lists `"mongodb"` reads its own rows from a collection**, each through `from_bson_piped`: [MongoDB Operations and Typed Filters](mongodb-operations.md#mongodb-operations-and-typed-filters).

### Cost

Every read walks the record, one that decodes cleanly included: the walk is what finds a key the type does not declare, a value held in another form than its field writes and a value that breaks a bound its field declares. Each plain value is read a second time and written back, to compare it with what is held.

`from_bson_with` copies the document once for each read of it by serde, `bson::Deserializer::new` taking the value it reads by value: once for the read that decodes it, and once more for the read of what the callback left after it answers `Fixed`. The walk borrows the document. Whatever the walk reads with serde on its own, each plain value first of all, it copies as it reads it, for the same reason.

Measured on one machine in a release build, on the `Record` below holding two versions, a `from_value_with` read that calls no callback took about 2.7 times what `serde_json::from_value` takes to read the same value, and a `from_bson_with` read about 3.3 times what the `bson` library's own read takes to read the same document, `bson::from_document` in version 2 and `bson::deserialize_from_document` in version 3. Each read was handed its own copy of the value and consumed it. Plain serde reading the JSON value through a reference, `Record::deserialize(&value)`, leaves the value with its caller and took less than half of what `serde_json::from_value` takes: `from_value_with` took about six times that.

A type that flattens another flagged type beside keys of its own copies the entries its own fields did not take, once each time it is walked, and hands that copy to the first type it flattens: those entries are what serde hands a flattened type. A type with no key of its own hands over the object it is read from, and copies nothing. A flattened type declared after another is handed what serde left once it had read the earlier ones: one more copy each time an earlier one took entries, and the same object each time it took none. What an earlier one takes is asked of its own reader, which is called once more for it with a deserializer that reads nothing. An internally tagged variant that holds one flagged type copies the object without the tag's entry. Where a flattened value read through a hook, or of a parameter's type, is refused, its reader is called once more, with that same deserializer, to ask whether it reads an `Option`.

### A complete callback

A `Record` with an inner `Version`, read from the MongoDB rows older writers left. Both list `"decode_with"` and `"mongodb"`, as stored types do, and the crate that declares them writes both [initializers](#the-support-libraries) at its root. `from_bson_with` is what `"decode_with"` generates in a build with `bson` on. The example needs tixschema's `mongodb` and `chrono` features.

```rust
use bson::oid::ObjectId;
use bson::{Bson, Document};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub number: i32,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    #[serde(rename = "recordId")]
    pub id: ObjectId,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub versions: Vec<Version>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}
```

The callback, a function over `Record` and its inner `Version`. An issue inside a version reaches it as `record_schema::Issue`, at its full path:

```rust
use record_schema::{DecodeWith as _, Expected, Issue, Stored as _, Verdict};

/// Repairs what older writers left in a `Record` row, inner versions included, and rejects
/// anything else.
fn repair_record(row: &mut Document, issues: &[Issue<Bson>]) -> Verdict {
    for issue in issues {
        if let Issue::Mistyped {
            path,
            expected: Expected::ObjectId,
            found: Bson::String(hex),
        } = issue
        {
            // the record's id, stored as text by rows written before it was an `ObjectId`
            let Ok(id) = ObjectId::parse_str(hex) else {
                return Verdict::Reject;
            };
            row.set_at(path, Bson::ObjectId(id));
        } else if let Issue::Invalid {
            path,
            expected: Expected::DateTime,
            found: Bson::DateTime(date),
            reason: _reason,
        } = issue
        {
            // the creation date, stored as a BSON date by a driver that writes dates natively
            let Some(created) = DateTime::<Utc>::from_timestamp_millis(date.timestamp_millis())
            else {
                return Verdict::Reject;
            };
            row.set_at(path, Bson::String(created.to_rfc3339()));
        } else if let Issue::Invalid {
            path,
            expected: Expected::DateTime,
            found: Bson::Int64(millis),
            reason: _reason,
        } = issue
        {
            // the creation date, stored as epoch milliseconds
            let Some(created) = DateTime::<Utc>::from_timestamp_millis(*millis) else {
                return Verdict::Reject;
            };
            row.set_at(path, Bson::String(created.to_rfc3339()));
        } else if let Issue::Invalid {
            path,
            expected: Expected::I32,
            found: Bson::String(text),
            reason: _reason,
        } = issue
        {
            // a version's number, stored as text: the path is `versions[1].number`
            let Ok(number) = text.parse::<i32>() else {
                return Verdict::Reject;
            };
            row.set_at(path, Bson::Int32(number));
        } else if let Issue::Unknown {
            path,
            found: _found,
        } = issue
        {
            // a key no type declares, on the record or on one of its versions
            row.remove_at(path);
        } else {
            return Verdict::Reject;
        }
    }
    Verdict::Fixed
}
```

A stored row with problems at three levels: the record's id stored as text and its creation date as a BSON date, a version's number stored as text beside a key no version declares, and the row's own `_id`, which `Record` does not declare. Shown as MongoDB Extended JSON:

```json
{
  "_id": { "$oid": "6a7cc592ca0574e6efdfe299" },
  "recordId": "6a7cc592ca0574e6efdfe217",
  "name": "Loan",
  "createdAt": { "$date": "2025-10-04T17:46:40Z" },
  "versions": [{ "number": 1 }, { "number": "2", "draft": true }]
}
```

Read with the callback, `row` being that row as a `bson::Document`:

```rust
let record = Record::from_bson_with(row, repair_record)?;
```

The callback runs once and is handed five issues, `Found` being the held value as `bson::Bson` prints it. It can read the row's `_id` from the raw document it is handed, `row.get_object_id("_id")`, to record which row it repaired.

| Member | Path | Expected | Found |
|--------|------|----------|-------|
| `Mistyped` | `recordId` | `ObjectId` | `String("6a7cc592ca0574e6efdfe217")` |
| `Invalid` | `createdAt` | `DateTime` | `DateTime(2025-10-04 17:46:40.0 +00:00:00)` |
| `Invalid` | `versions[1].number` | `I32` | `String("2")` |
| `Unknown` | `versions[1].draft` | | `Boolean(true)` |
| `Unknown` | `_id` | | `ObjectId("6a7cc592ca0574e6efdfe299")` |

It answers `Fixed`, the second read finds no issue, and the call returns this `Record`, each version shown by its `number`:

| Field | Value |
|-------|-------|
| `id` | `6a7cc592ca0574e6efdfe217` |
| `name` | `Loan` |
| `created_at` | `2025-10-04T17:46:40+00:00` |
| `versions` | `[1, 2]` |
| `note` | `None` |

### Resolvers

`from_bson_piped` and `from_value_piped` take resolvers where `from_bson_with` and `from_value_with` take one callback. A resolver is a function over the raw value and one issue, and it answers that issue alone: `Settled`, `Rejected` or `NotTouched`. Each resolver knows one repair, so a read is handed the ones its rows need, in the order they are to run.

A resolver is written against `Issue`, which a crate holds once: within one crate, `invoice_schema::Issue` and `order_schema::Issue` are one type, so one resolver serves the reads of every flagged type the crate declares ([One library per crate](#one-library-per-crate)). A type of another crate has its own crate's `Issue`. An issue inside a nested type reaches the outer type's resolvers as the outer type's `Issue`, at its full path, so a resolver for `Invoice` repairs what is wrong inside its `Customer`.

An `Invoice` that holds a `Customer`, both read from the rows older writers left. Both list `"decode_with"` and `"mongodb"`, as stored types do. `from_bson_piped` is what `"decode_with"` generates in a build with `bson` on, and the example needs tixschema's `bson` feature.

```rust
use bson::{Bson, Document, doc};
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Customer {
    pub name: String,
    pub open_invoices: u32,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Invoice {
    pub customer: Customer,
    pub number: String,
}
```

Two resolvers. Each looks for the one issue it repairs and answers `NotTouched` for any other:

```rust
use invoice_schema::{DecodeWith as _, Expected, Issue, Resolution, Stored as _};

/// A customer an older writer stored as JSON text: parsed into a document.
fn customer_as_text(raw: &mut Document, issue: &Issue<Bson>) -> Resolution {
    let Issue::Invalid {
        path,
        expected: Expected::Model("Customer"),
        found: Bson::String(text),
        reason: _reason,
    } = issue
    else {
        return Resolution::NotTouched;
    };
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text) else {
        return Resolution::Rejected;
    };
    // Each major version of the library names its own function for this; both have the serializer.
    let Ok(Bson::Document(customer)) = parsed.serialize(bson::Serializer::new()) else {
        return Resolution::Rejected;
    };
    if raw.set_at(path, Bson::Document(customer)) {
        Resolution::Settled
    } else {
        Resolution::Rejected
    }
}

/// A whole number an older writer stored as text; a negative count is refused.
fn whole_number_as_text(raw: &mut Document, issue: &Issue<Bson>) -> Resolution {
    let Issue::Invalid {
        path,
        expected: Expected::U32,
        found: Bson::String(text),
        reason: _reason,
    } = issue
    else {
        return Resolution::NotTouched;
    };
    let settled = text
        .trim()
        .parse::<i64>()
        .is_ok_and(|number| number >= 0_i64 && raw.set_at(path, Bson::Int64(number)));
    if settled {
        Resolution::Settled
    } else {
        Resolution::Rejected
    }
}
```

Five stored rows, and the read of each with both resolvers:

```rust
/// Five rows older writers left.
fn stored_rows() -> [Document; 5] {
    [
        // a count stored as text
        doc! {
            "customer": { "name": "Acme", "openInvoices": "3" },
            "number": "INV-0042",
        },
        // a count stored as text that is no count
        doc! {
            "customer": { "name": "Acme", "openInvoices": "-3" },
            "number": "INV-0042",
        },
        // a key no type declares
        doc! {
            "customer": { "name": "Acme", "openInvoices": 3 },
            "legacy": true,
            "number": "INV-0042",
        },
        // a customer stored as text, beside a key no type declares
        doc! {
            "customer": "{\"name\":\"Acme\",\"openInvoices\":3}",
            "legacy": true,
            "number": "INV-0042",
        },
        // a customer stored as text, whose count is text too
        doc! {
            "customer": "{\"name\":\"Acme\",\"openInvoices\":\"3\"}",
            "number": "INV-0042",
        },
    ]
}

/// One stored row read with both resolvers, told in one line.
fn told(row: Document) -> String {
    match Invoice::from_bson_piped(row, &[&customer_as_text, &whole_number_as_text]) {
        Ok(invoice) => format!("read as {invoice:?}"),
        Err(refused) => format!("refused: {refused}"),
    }
}
```

The five rows told in order, under version 2 of the `bson` library:

```text
read as Invoice { customer: Customer { name: "Acme", open_invoices: 3 }, number: "INV-0042" }
refused: customer.openInvoices: invalid: expected U32, found String("-3"): invalid type: string "-3", expected u32
refused: legacy: unknown: found Boolean(true)
refused: legacy: unknown: found Boolean(true)
refused: customer.openInvoices: invalid: expected U32, found String("3"): invalid type: string "3", expected u32
```

1. The first row reads. `customer_as_text` leaves its one issue `NotTouched`, `whole_number_as_text` settles it, and the second read finds no issue.
2. The second is refused by the resolver that knows its issue: `whole_number_as_text` answers `Rejected` for a count below zero.
3. The third is refused over an issue no resolver knows. Both leave the key `legacy` `NotTouched`.
4. The fourth is refused over that same issue, and that one alone: `customer_as_text` settled the customer stored as text, and a settled issue is not among the ones a refused read holds.
5. The fifth is refused by the second read. `customer_as_text` settles the customer, every issue of the first read is then settled, and the read of what the resolvers left finds the count inside that customer held as text. No resolver is handed that issue: the resolvers run once.

The second and the fifth line end in the `bson` library's own wording, which its two major versions write differently. Under version 3 the same two lines read:

```text
refused: customer.openInvoices: invalid: expected U32, found String("-3"): BSON error. Kind: A deserialization-related error occurred. Message: invalid type: string "-3", expected u32.
refused: customer.openInvoices: invalid: expected U32, found String("3"): BSON error. Kind: A deserialization-related error occurred. Message: invalid type: string "3", expected u32.
```

What the pipe holds every read to:

- **Resolvers run in the order of the slice.** Each one is handed, one at a time and in the order the read found them, the issues every resolver before it left `NotTouched`.
- **`Settled` says the resolver repaired the raw value.** Nothing checks the repair but the second read, which lists the issue again where the value still does not read.
- **`Rejected` fails the read.** The issue is known and is not to be repaired. No later resolver is handed it, and the failed read holds it.
- **An issue every resolver leaves `NotTouched` fails the read** as well, and the failed read holds it.
- **A failed read holds the issues no resolver settled**, in the order they were found, as `Unrecovered`. With no resolver at all, `&[]`, that is every issue.
- **A resolver is `Sync`.** It is a function, or a closure that holds what it needs, taken by reference: `&[&customer_as_text, &counted]`. A closure that counts in a `Cell` or a `RefCell` is refused (`E0277`, `` `Cell<u32>` cannot be shared between threads safely``), and one that counts in an atomic or behind a `Mutex` is taken. It is what lets a read that waits for MongoDB hold its resolvers and still move to another thread.

`from_value_piped` is the same read of a `serde_json::Value`. Its resolvers take `&mut serde_json::Value` and `&Issue<serde_json::Value>`, and repair through `Stored` for `serde_json::Value`.
