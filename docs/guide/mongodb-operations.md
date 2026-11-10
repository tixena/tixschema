## MongoDB Operations and Typed Filters

With tixschema's `mongodb` feature on, a type declared with [`#[model_schema(enable = ["decode_with", "mongodb"])]`](recovering-decode.md#opting-in) reads and writes its own rows in a MongoDB collection, and names its own fields in the filters and updates it does so with:

- **Typed paths.** `MONGO_FIELDS`, a const on the type, answers one path per key serde writes for it, each from a function named after the field. A path builds a filter or an update from a value of its field's own type, so a field that does not exist and a value of another type are compile errors. `nest` writes what is asked of a nested model's own paths under the key that holds the model.
- **Operations.** `find_one`, `find`, `count`, `insert_one`, `update_one`, `update_many`, `delete_one` and `delete_many`, over a `mongodb::Collection<bson::Document>`. Every row a read is handed goes through the recovering decode, so a row that does not read as the type is reported by its `_id` and its issues, and `find_one_with` and `find_with` take the [resolvers](recovering-decode.md#resolvers) that repair one.

The entry `"mongodb"` is what turns both on, under tixschema's `mongodb` feature, and it needs `"decode_with"` beside it. A type whose list holds `"decode_with"` alone has neither in any build. In a build without the `mongodb` feature the entry adds nothing and is accepted: a type that lists it has the recovering decode `"decode_with"` gives it, from JSON, and from BSON under `bson`, and no operation, no `MONGO_FIELDS` and no typed path. A type that is never stored lists `"decode_with"` alone, whether it is read from BSON or not, and none of this section's code is generated for it.

The query types and what the operations answer are the crate's MongoDB [support library](recovering-decode.md#the-support-libraries), which `mongodb_foundation!()` writes once under the `mongodb` feature, and which the `<type>_schema` module of every type that lists `"mongodb"` re-exports name by name, beside the issue types. So `invoice_schema::Filter`, `invoice_schema::OperationError` and `invoice_schema::Read` are the crate's one `Filter`, `OperationError` and `Read`, and `customer_schema` names the same three. A compiler error names such a type where the library declares it, `__tixschema_mongodb::Filter<Invoice>`. What each type writes for itself is its typed paths, `MongoFields` in its own module and `MONGO_FIELDS` on the type, and its operations, on the type. A filter and an update still belong to the rows of one type, which their first parameter names: a `Filter<Invoice>` is over the rows of `Invoice`, and is refused where a filter over the rows of `Customer` is asked. `nest` takes a filter or an update built from a nested model's own paths, and answers one over the outer type's rows ([Filters and updates](#filters-and-updates)).

No line of this section is the answer of a MongoDB server. A document shown is the one a call writes, as `bson::Document` displays it, and an answer is the driver's own result type.

### The driver your crate lists

The code generated for a type that lists `"mongodb"`, and the library `mongodb_foundation!()` writes, name the `mongodb` driver, so a crate that declares such a type lists the driver beside `bson`, `serde` and `serde_json`, whether or not it calls an operation. Without it the build fails with ``error[E0433]: cannot find module or crate `mongodb` in this scope``. A crate whose types list `"decode_with"` alone lists no driver, and lists `bson` wherever tixschema's `bson` feature is on, as [What your crate lists](recovering-decode.md#what-your-crate-lists) says.

The crate's `bson` is the major version its driver is built for. The driver's default features build it for version 2 of the `bson` library:

```toml
mongodb = "3.9"
bson = "2.15"
```

With its default features off and `bson-3` on, it is built for version 3:

```toml
mongodb = { version = "3.9", default-features = false, features = ["bson-3", "compat-3-3-0", "rustls-tls", "dns-resolver"] }
bson = { version = "3.1", features = ["serde"] }
```

`compat-3-3-0`, `rustls-tls` and `dns-resolver` are part of the driver's defaults, written out because turning the defaults off turns them off too. The crate's own `tests/` build against the driver at its default features and `bson` 2.15, and `bson3/`, this repository's package for version 3, against the second pair.

A `bson` of the other major version fails the build at each type that lists `"mongodb"`, the generated operations handing the driver a `Document` it does not take:

```text
error[E0308]: mismatched types
 --> src/lib.rs:8:1
  |
8 | #[model_schema(enable = ["decode_with", "mongodb"])]
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  | |
  | expected `mongodb::bson::Document`, found `bson::Document`
  | arguments to this method are incorrect
  |
note: there are multiple different versions of crate `bson` in the dependency graph
```

The note goes on to name the two versions, each at its place in the Cargo registry of the machine that compiled it.

### The model of the examples

The examples of this section share one model: an `Invoice`, and the types it holds. Every one of them lists `"decode_with"` and `"mongodb"`, and each is declared above the type that holds it.

```rust
use std::collections::HashMap;

use bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

tixschema::decode_with_foundation!();
tixschema::mongodb_foundation!();

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    pub city: String,
    pub postal_code: String,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Customer {
    pub address: Address,
    pub name: String,
    pub open_invoices: u32,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum InvoiceStatus {
    Draft,
    Paid,
    PastDue,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct LineItem {
    pub price: f64,
    pub quantity: u32,
    pub sku: String,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Payment {
    Card { last4: String },
    Cash,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Invoice {
    #[serde(rename = "_id")]
    pub id: ObjectId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub billing: Option<Address>,
    pub customer: Customer,
    pub details: HashMap<String, String>,
    pub items: Vec<LineItem>,
    pub number: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paid_at: Option<String>,
    pub payment: Payment,
    pub status: InvoiceStatus,
    pub tags: Vec<String>,
    pub total: f64,
}
```

### The paths of a type

`Invoice::MONGO_FIELDS` is an `invoice_schema::MongoFields<Invoice>`. It holds no path: it has one function per field of `Invoice`, under the field's Rust name, and each answers the path of the key serde writes that field under. The type of the field decides the kind of the path:

| Function | The kind it answers |
|----------|---------------------|
| `id()` | `Field<Invoice, ObjectId>` |
| `billing()` | `OptionalModel<Invoice, Address>` |
| `customer()` | `Model<Invoice, Customer>` |
| `details` | none: a map's keys are data |
| `items()` | `ModelList<Invoice, LineItem>` |
| `number()` | `Field<Invoice, String>` |
| `paid_at()` | `OptionalField<Invoice, String>` |
| `payment()` | `Model<Invoice, Payment>` |
| `status()` | `Field<Invoice, InvoiceStatus>` |
| `tags()` | `ListField<Invoice, String>` |
| `total()` | `Field<Invoice, f64>` |

Each kind is a type of `invoice_schema`. The first parameter is the row a filter is over, and the second the value an operator takes. A path is one key, the one its own type writes: the paths of `Invoice` hold no key of `Customer`, and those of `Customer` none of `Address`.

**`nest` reaches a nested model.** `Customer` has paths of its own, `Customer::MONGO_FIELDS`. `nest`, on the path that holds a customer, takes a filter or an update built from those and answers the same filter or update over the rows of `Invoice`, each key written under the path's own. `paths.customer().nest(customer.address().nest(address.city().eq("Moca".to_owned())?))` asks for a city three types down, and the key it writes is `customer.address.city`.

A path builds a `Filter<Invoice>` or an `Update<Invoice>`, the two documents an operation takes. Building one asks nothing of MongoDB:

```rust
use bson::doc;

use invoice_schema::{Filter, Update, WriteError};

/// Unpaid invoices of one city that are past due or above an amount.
fn to_chase(city: &str, above: f64) -> Result<Filter<Invoice>, WriteError> {
    let (paths, customer) = (Invoice::MONGO_FIELDS, Customer::MONGO_FIELDS);
    let in_city = Address::MONGO_FIELDS.city().eq(city.to_owned())?;
    Ok(paths
        .status()
        .eq(InvoiceStatus::PastDue)?
        .or(paths.total().gt(above)?)
        .and(paths.customer().nest(customer.address().nest(in_city)))
        .and(paths.paid_at().exists(false)))
}

/// What paying an invoice changes.
fn paid(at: &str) -> Result<Update<Invoice>, WriteError> {
    let (paths, customer) = (Invoice::MONGO_FIELDS, Customer::MONGO_FIELDS);
    Ok(paths
        .status()
        .set(InvoiceStatus::Paid)?
        .and(paths.paid_at().set(at.to_owned())?)
        .and(paths.customer().nest(customer.open_invoices().set(0)?))
        .and(paths.tags().push("settled".to_owned())?))
}
```

| Written | The document it builds |
|---------|------------------------|
| `to_chase("Moca", 1000.5)?` | `{ "$and": [{ "$or": [{ "status": { "$eq": "past-due" } }, { "total": { "$gt": 1000.5 } }] }, { "customer.address.city": { "$eq": "Moca" } }, { "paidAt": { "$exists": false } }] }` |
| `paid("2026-10-07T17:40:00Z")?` | `{ "$set": { "status": "paid", "paidAt": "2026-10-07T17:40:00Z", "customer.openInvoices": 0 }, "$push": { "tags": "settled" } }` |

- **A value is of the field's own type, taken by value.** `paths.status().eq` takes an `InvoiceStatus`, and the text serde writes for one is refused: `paths.status().eq("past-due")` is ``error[E0308]: mismatched types``, ``expected `InvoiceStatus`, found `&str` ``. `paths.total().gt` takes an `f64`.
- **A field the type does not declare is a function that does not exist.** `paths.amount()` is ``error[E0599]: no method named `amount` found for struct `invoice_schema::MongoFields<Root>` in the current scope``.
- **A value is written the way its field is stored**, by the type's own `Serialize` through `bson::Serializer::new()`: `InvoiceStatus::PastDue` as the text `past-due`, a `u32` as a 64-bit integer, an `ObjectId` as one.
- **A key is the name serde writes**: `paid_at()` is the key `paidAt`, and `open_invoices()` nested under `customer()` is `customer.openInvoices`.
- **An operator that writes a value answers a `Result`**, since a value may be one BSON cannot hold; the error is `WriteError`, the `bson` library's own serialization error, `bson::ser::Error` in version 2 and `bson::error::Error` in version 3. An operator that writes none, such as `exists`, `unset`, `size`, `regex` and `is_<variant>`, answers the filter or the update itself. `nest` answers what it is handed, a filter or an update.
- **A path is a value, and `MONGO_FIELDS` a const.** Each function is a `const fn`, so a `const` item holds a path: `const TOTAL: invoice_schema::Field<Invoice, f64> = Invoice::MONGO_FIELDS.total();`. `MongoFields` holds no path and a path holds one key, so neither grows with what the type holds.

### Operators

Every operator, once, with the document it writes. `paths` is `Invoice::MONGO_FIELDS`, and `customer`, `address`, `item` and `payment` are `Customer::MONGO_FIELDS`, `Address::MONGO_FIELDS`, `LineItem::MONGO_FIELDS` and `Payment::MONGO_FIELDS`.

**A value every row holds, a `Field`.** The comparisons, membership in a list of values, and the two operators that set it. `regex` is there where the value is a `String`, and takes MongoDB's options as text:

| Written | The document it builds |
|---------|------------------------|
| `paths.number().eq("INV-0042".to_owned())?` | `{ "number": { "$eq": "INV-0042" } }` |
| `paths.number().ne("INV-0042".to_owned())?` | `{ "number": { "$ne": "INV-0042" } }` |
| `paths.total().gt(1000.5)?` | `{ "total": { "$gt": 1000.5 } }` |
| `paths.total().gte(1000.5)?` | `{ "total": { "$gte": 1000.5 } }` |
| `paths.total().lt(1000.5)?` | `{ "total": { "$lt": 1000.5 } }` |
| `paths.total().lte(1000.5)?` | `{ "total": { "$lte": 1000.5 } }` |
| `paths.number().is_in(["INV-0042".to_owned(), "INV-0043".to_owned()])?` | `{ "number": { "$in": ["INV-0042", "INV-0043"] } }` |
| `paths.number().not_in(["INV-0042".to_owned()])?` | `{ "number": { "$nin": ["INV-0042"] } }` |
| `paths.number().regex("^INV-", "i")` | `{ "number": { "$regex": "^INV-", "$options": "i" } }` |
| `paths.total().set(99.5)?` | `{ "$set": { "total": 99.5 } }` |
| `paths.number().set_on_insert("INV-0099".to_owned())?` | `{ "$setOnInsert": { "number": "INV-0099" } }` |

**A value a row may leave out, an `OptionalField`.** Every operator of a `Field`, and two of its own: `exists`, which takes whether the key is to be there, and `unset`:

| Written | The document it builds |
|---------|------------------------|
| `paths.paid_at().exists(false)` | `{ "paidAt": { "$exists": false } }` |
| `paths.paid_at().unset()` | `{ "$unset": { "paidAt": "" } }` |
| `paths.paid_at().lt("2026-10-01".to_owned())?` | `{ "paidAt": { "$lt": "2026-10-01" } }` |

`unset` is on a path a row may leave out and on no other: `paths.number().unset()` is ``error[E0599]: no method named `unset` found for struct `__tixschema_mongodb::Field<Root, V, N>` in the current scope``.

**A type serde writes as one value**, here the plain enum `InvoiceStatus`. Held by another type it is a `Field`, with every operator of one:

| Written | The document it builds |
|---------|------------------------|
| `paths.status().eq(InvoiceStatus::PastDue)?` | `{ "status": { "$eq": "past-due" } }` |
| `paths.status().is_in([InvoiceStatus::Draft, InvoiceStatus::PastDue])?` | `{ "status": { "$in": ["draft", "past-due"] } }` |
| `paths.status().set(InvoiceStatus::Paid)?` | `{ "$set": { "status": "paid" } }` |

**A list of plain values, a `ListField`.** A list is matched through its elements: `contains` asks for one value, `contains_any` and `contains_none` for several, and `elem_match` holds one element to every condition of an `Element`, which `element()` starts and `eq`, `ne`, `gt`, `gte`, `lt` and `lte` add to. `size` takes how many elements the list holds. `push` and `pull` change one element, and `set` and `set_on_insert` take the whole list:

| Written | The document it builds |
|---------|------------------------|
| `paths.tags().contains("export".to_owned())?` | `{ "tags": { "$eq": "export" } }` |
| `paths.tags().contains_any(["export".to_owned(), "priority".to_owned()])?` | `{ "tags": { "$in": ["export", "priority"] } }` |
| `paths.tags().contains_none(["void".to_owned()])?` | `{ "tags": { "$nin": ["void"] } }` |
| `paths.tags().size(0)` | `{ "tags": { "$size": 0 } }` |
| `paths.tags().elem_match(paths.tags().element().gte("a".to_owned())?.lt("n".to_owned())?)` | `{ "tags": { "$elemMatch": { "$gte": "a", "$lt": "n" } } }` |
| `paths.tags().push("reviewed".to_owned())?` | `{ "$push": { "tags": "reviewed" } }` |
| `paths.tags().pull("draft".to_owned())?` | `{ "$pull": { "tags": "draft" } }` |
| `paths.tags().set(["new".to_owned()])?` | `{ "$set": { "tags": ["new"] } }` |
| `paths.tags().set_on_insert(["new".to_owned()])?` | `{ "$setOnInsert": { "tags": ["new"] } }` |

A comparison of the list itself is no operator: `paths.tags().gt("export".to_owned())` does not compile (`E0599`).

**A nested model, a `Model`.** `nest` writes what is asked of the model's own paths under its key, a filter and an update alike. The model is one whole value as well: `eq`, `ne`, `is_in`, `not_in`, `set` and `set_on_insert` take the model itself. `moca()` is an `Address`:

```rust
/// The address the examples write.
fn moca() -> Address {
    Address {
        city: "Moca".to_owned(),
        postal_code: "56000".to_owned(),
    }
}
```

| Written | The document it builds |
|---------|------------------------|
| `paths.customer().nest(customer.name().eq("Acme".to_owned())?)` | `{ "customer.name": { "$eq": "Acme" } }` |
| `paths.customer().nest(customer.address().nest(address.city().eq("Moca".to_owned())?))` | `{ "customer.address.city": { "$eq": "Moca" } }` |
| `paths.customer().nest(customer.name().set("John".to_owned())?)` | `{ "$set": { "customer.name": "John" } }` |
| `paths.customer().nest(customer.address().eq(moca())?)` | `{ "customer.address": { "$eq": { "city": "Moca", "postalCode": "56000" } } }` |
| `paths.customer().nest(customer.address().ne(moca())?)` | `{ "customer.address": { "$ne": { "city": "Moca", "postalCode": "56000" } } }` |
| `paths.customer().nest(customer.address().is_in([moca()])?)` | `{ "customer.address": { "$in": [{ "city": "Moca", "postalCode": "56000" }] } }` |
| `paths.customer().nest(customer.address().not_in([moca()])?)` | `{ "customer.address": { "$nin": [{ "city": "Moca", "postalCode": "56000" }] } }` |
| `paths.customer().nest(customer.address().set(moca())?)` | `{ "$set": { "customer.address": { "city": "Moca", "postalCode": "56000" } } }` |
| `paths.customer().nest(customer.address().set_on_insert(moca())?)` | `{ "$setOnInsert": { "customer.address": { "city": "Moca", "postalCode": "56000" } } }` |

**A nested model a row may leave out, an `OptionalModel`.** Every operator of a `Model`, `nest` among them, and `exists` and `unset`:

| Written | The document it builds |
|---------|------------------------|
| `paths.billing().nest(address.city().eq("Moca".to_owned())?)` | `{ "billing.city": { "$eq": "Moca" } }` |
| `paths.billing().exists(true)` | `{ "billing": { "$exists": true } }` |
| `paths.billing().unset()` | `{ "$unset": { "billing": "" } }` |
| `paths.billing().set(moca())?` | `{ "$set": { "billing": { "city": "Moca", "postalCode": "56000" } } }` |

**A list of nested models, a `ModelList`.** What `nest` writes under it matches where any element does. `elem_match` holds one element to a filter over the rows of the element's own type, and `pull` takes out every element such a filter matches. `size`, `push` and `set` are those of a list. `bolts()` is a `LineItem`:

```rust
/// The line item the examples write.
fn bolts() -> LineItem {
    LineItem {
        price: 4.5,
        quantity: 9,
        sku: "C-3".to_owned(),
    }
}
```

| Written | The document it builds |
|---------|------------------------|
| `paths.items().nest(item.price().gt(100.5)?)` | `{ "items.price": { "$gt": 100.5 } }` |
| `paths.items().elem_match(item.quantity().gte(2)?.and(item.sku().eq("B-7".to_owned())?))` | `{ "items": { "$elemMatch": { "$and": [{ "quantity": { "$gte": 2 } }, { "sku": { "$eq": "B-7" } }] } } }` |
| `paths.items().size(2)` | `{ "items": { "$size": 2 } }` |
| `paths.items().push(bolts())?` | `{ "$push": { "items": { "price": 4.5, "quantity": 9, "sku": "C-3" } } }` |
| `paths.items().pull(item.quantity().lt(1)?)` | `{ "$pull": { "items": { "quantity": { "$lt": 1 } } } }` |
| `paths.items().set([bolts()])?` | `{ "$set": { "items": [{ "price": 4.5, "quantity": 9, "sku": "C-3" }] } }` |

**An enum under a tag**, here `Payment`. `is_<variant>()` asks which variant a row holds, and what a variant holds is reached through a function named after the variant in snake case. Both are functions of the enum's own paths, which `nest` writes under the key that holds the enum. The enum is one whole value as any nested model is:

| Written | The document it builds |
|---------|------------------------|
| `paths.payment().nest(payment.is_card())` | `{ "payment.kind": { "$eq": "card" } }` |
| `paths.payment().nest(payment.is_cash())` | `{ "payment.kind": { "$eq": "cash" } }` |
| `paths.payment().nest(payment.card().last4().eq("4242".to_owned())?)` | `{ "payment.last4": { "$eq": "4242" } }` |
| `paths.payment().set(Payment::Cash)?` | `{ "$set": { "payment": { "kind": "cash" } } }` |

**The arms of an enum under a tag.** `<tag>__<value>()` is a function of the enum's own paths, one per variant serde writes, and it answers the rows of that arm. Its name is the tag key, `__`, and the value serde writes for the variant, each in snake case. What it answers is an arm filter, an `ArmFilter<Root, M>`: a filter over the rows of the enum that holds the arm's tag from the start, and whose `and` takes a filter over the rows of `M`, the model the arm holds, and over no other. So a field of an arm is asked only together with that arm's tag, and a row of another arm that writes the same key is not matched. The function is a `const fn`, so a `const` item holds an arm filter, and each use of the `const` is a new value:

```rust
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Courier {
    pub name: String,
    pub region: String,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum Transport {
    #[serde(rename = "by-courier")]
    ByCourier(Courier),
    Collected,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Booking {
    pub depot_code: String,
    #[serde(flatten)]
    pub transport: Transport,
}

/// The bookings a courier carries, held in a `const`.
const BY_COURIER: transport_schema::ArmFilter<Transport, Courier> =
    Transport::MONGO_FIELDS.type__by_courier();

/// The bookings one courier carries out of one depot.
fn couriered(depot: &str, courier: &str) -> Result<booking_schema::Filter<Booking>, WriteError> {
    let booking = Booking::MONGO_FIELDS;
    let by_name = BY_COURIER.and(Courier::MONGO_FIELDS.name().eq(courier.to_owned())?);
    Ok(booking
        .transport()
        .nest(by_name)
        .and(booking.depot_code().eq(depot.to_owned())?))
}
```

| Written | The document it builds |
|---------|------------------------|
| `couriered("SDQ-1", "Acme Freight")?` | `{ "$and": [{ "type": { "$eq": "by-courier" } }, { "name": { "$eq": "Acme Freight" } }, { "depotCode": { "$eq": "SDQ-1" } }] }` |
| `Booking::count(&collection, couriered("SDQ-1", "Acme Freight").map_err(booking_schema::OperationError::Unwritable)?).await?` | the count of those rows |

`collection` is a `mongodb::Collection<bson::Document>` of bookings. `Booking` flattens the enum, so `nest` writes the tag and the condition among the row's own keys. Where a row holds the enum under a key, `nest` writes both under that key. With the model in another crate nothing changes: `Courier::MONGO_FIELDS` is that crate's, and `and` takes its filter by the two standard traits ([Filters and updates](#filters-and-updates)).

An arm filter is a filter over the rows of its enum. It turns into the document of its tag alone, `and` adds each condition to one `$and`, and `Filter::from` makes it the module's own `Filter`, which has `negated`:

| Written | The document it builds |
|---------|------------------------|
| `BY_COURIER` | `{ "type": { "$eq": "by-courier" } }` |
| `BY_COURIER.and(Courier::MONGO_FIELDS.name().eq("Acme".to_owned())?)` | `{ "$and": [{ "type": { "$eq": "by-courier" } }, { "name": { "$eq": "Acme" } }] }` |
| `transport_schema::Filter::from(BY_COURIER).negated()` | `{ "$nor": [{ "type": { "$eq": "by-courier" } }] }` |

The model `M` an arm is over follows from what its variant holds:

| The variant holds | `M` |
|-------------------|-----|
| One slot serde writes | The slot's type, read through `Option`, a list and the wrappers `nest` reads through: what `nest` reaches below a path over that slot. `ByCourier(Courier)` is over `Courier`, and an arm that holds a `Box<Courier>` is over `Courier` too. A map is `()` |
| Fields written in the variant, or several slots | The arm's marker, `mongo_arms::Card` for `Card { last4: String }`: `with` hands its closure the variant's paths |
| Nothing, or one slot serde never writes | `()` |

The snake case of a tag key and of a value is its words in lower case, joined by one `_`. A word ends at a character that is no ASCII letter or digit, before a capital that follows a small letter or a digit, and before a capital a small letter follows where a capital comes before it: `docType` is `doc_type`, `HTTPServer` is `http_server`, `v2Beta` is `v2_beta`, and `@type` is `type`.

| The enum | The variant | Its arm function |
|----------|-------------|------------------|
| `#[serde(tag = "type")]` | `#[serde(rename = "by-courier")] ByCourier(Courier)` | `type__by_courier()` |
| `#[serde(tag = "type")]` | `Collected` | `type__collected()` |
| `#[serde(tag = "docType", content = "body", rename_all = "kebab-case")]` | `PastDue(Notice)` | `doc_type__past_due()` |

**An arm whose fields are written in the variant takes conditions on those fields alone, through `with`.** `with` hands its closure the paths of the variant, rooted at the arm's marker, and joins the condition the closure answers to the arm's tag in one `$and`. It answers a `Result`, as the operators do. Here `payment` is the paths of `Payment`, and the last row is the plain `and`, which still takes any condition over the enum:

| Written | The document it builds |
|---------|------------------------|
| `payment.kind__cash()` | `{ "kind": { "$eq": "cash" } }` |
| `paths.payment().nest(payment.kind__cash())` | `{ "payment.kind": { "$eq": "cash" } }` |
| `payment.kind__card().with(\|card\| card.last4().eq("4242".to_owned()))?` | `{ "$and": [{ "kind": { "$eq": "card" } }, { "last4": { "$eq": "4242" } }] }` |
| `payment.card().last4().eq("4242".to_owned())?.and(payment.kind__card())` | `{ "$and": [{ "last4": { "$eq": "4242" } }, { "kind": { "$eq": "card" } }] }` |

Under `#[serde(tag = "docType", content = "body")]`, a variant with named fields writes each field under the content key (`body.clerk`), and one of several slots writes each slot there (`body.1`). `with` refuses a condition of another arm, and one of the enum's plain paths such as `payment.card().last4()`: ``error[E0277]: the trait bound `__tixschema_mongodb::Filter<Payment>: AsRef<PhantomData<mongo_arms::Card>>` is not satisfied``.

**An arm whose `M` is `()` answers its tag alone**: no model and no field is there for a condition to be over.

**`and` on an arm filter refuses a filter over another type.** With `Wagon` a flagged struct that declares a `number`, `BY_COURIER.and(Wagon::MONGO_FIELDS.number().eq("W-9".to_owned())?)` is ``error[E0277]: the trait bound `__tixschema_mongodb::Filter<Wagon>: AsRef<PhantomData<Courier>>` is not satisfied``.

**An arm takes an update of the model it holds.** `update` on an arm filter takes an update of the rows of `M`, the model the arm holds, and of no other, and answers an arm update, an `ArmUpdate<Root, M>`: an update of the rows of the enum, with each key of the model written where the arm holds it. `and` on an arm update merges one more update of `M`, as `and` on an update does. `tag_on_insert` adds the arm's tag to `$setOnInsert`, which an upsert writes where it inserts the row. The tag is written only where it is asked for: an update of a row that is there needs none.

```rust
/// A booking's courier renamed, with the arm's tag written where an upsert inserts the row.
fn recouriered(courier: &str) -> Result<booking_schema::Update<Booking>, WriteError> {
    let renamed = BY_COURIER
        .update(Courier::MONGO_FIELDS.name().set(courier.to_owned())?)
        .tag_on_insert();
    Ok(Booking::MONGO_FIELDS.transport().nest(renamed))
}
```

| Written | The document it builds |
|---------|------------------------|
| `recouriered("Acme Freight")?` | `{ "$set": { "name": "Acme Freight" }, "$setOnInsert": { "type": "by-courier" } }` |
| `Booking::update_one(&collection, couriered("SDQ-1", "Acme")?, recouriered("Acme Freight")?).await?` | the driver's `UpdateResult` |
| `collection.update_one(couriered("SDQ-1", "Acme")?.into_document(), recouriered("Acme Freight")?.into_document()).upsert(true).await?` | the same, as an upsert: the driver's own call |

`Booking` flattens the enum, so `nest` writes the model's key and the tag among the row's own keys. Where a row holds the enum under a key, `nest` writes every key under it, the tag included. The model may be of another crate, or declared below the enum: no function of the variant takes its update there, and the arm does.

**An update carries no condition.** `update` leaves behind the conditions `and` added to the arm filter: an update selects no row, and the filter the operation takes beside it does, as `couriered` does above. No operation upserts. An upsert is the driver's own call ([A query the operations do not cover](#a-query-the-operations-do-not-cover)), which `into_document()` hands the filter and the update.

The tag is written at the enum's own level under either tagging, and what the arm holds is written under the content key where the enum has one. `Update::from` makes an arm update the module's own `Update`. `courier` and `notice` are `Courier::MONGO_FIELDS` and `Notice::MONGO_FIELDS`, and `paper` is the paths of the enum of the last row of the table of arm functions above, whose `PastDue(Notice)` holds a `Notice` that declares a `code`:

| Written | The document it builds |
|---------|------------------------|
| `BY_COURIER.update(courier.name().set("Renamed".to_owned())?)` | `{ "$set": { "name": "Renamed" } }` |
| `BY_COURIER.update(courier.name().set("Renamed".to_owned())?).and(courier.region().set_on_insert("north".to_owned())?).tag_on_insert()` | `{ "$set": { "name": "Renamed" }, "$setOnInsert": { "region": "north", "type": "by-courier" } }` |
| `paper.doc_type__past_due().update(notice.code().set("N-8".to_owned())?).tag_on_insert()` | `{ "$set": { "body.code": "N-8" }, "$setOnInsert": { "docType": "past-due" } }` |
| `transport_schema::Update::from(BY_COURIER.tag_on_insert())` | `{ "$setOnInsert": { "type": "by-courier" } }` |

**An arm whose fields are written in the variant, or whose `M` is `()`, writes its tag all the same.** No update is over `()` or over a marker, so `update` has nothing to take there, and `tag_on_insert` is on the arm filter itself. What it answers joins a plain update of the fields written in the variant:

| Written | The document it builds |
|---------|------------------------|
| `payment.kind__cash().tag_on_insert()` | `{ "$setOnInsert": { "kind": "cash" } }` |
| `payment.card().last4().set("4242".to_owned())?.and(payment.kind__card().tag_on_insert())` | `{ "$set": { "last4": "4242" }, "$setOnInsert": { "kind": "card" } }` |

**`update` on an arm filter takes an update of the arm's model and nothing else.** Handed a filter over the rows of that model, `BY_COURIER.update(Courier::MONGO_FIELDS.name().eq("Acme".to_owned())?)` is ``error[E0277]: the trait bound `__tixschema_mongodb::Filter<Courier>: AsRef<PhantomData<fn(Courier) -> Courier>>` is not satisfied``. Handed an update of another type, with `Wagon` a flagged struct that declares a `number`, `BY_COURIER.update(Wagon::MONGO_FIELDS.number().set("W-1".to_owned())?)` is ``error[E0277]: the trait bound `__tixschema_mongodb::Update<Wagon>: AsRef<PhantomData<fn(Courier) -> Courier>>` is not satisfied``. An arm update is no filter: it carries an update's marker and no other, so `BY_COURIER.and(BY_COURIER.tag_on_insert())` does not compile (`E0277`).

### Filters and updates

`and` and `or` join two filters over the same rows, and `negated` turns one around. `and` on an update merges two updates of the same rows: the keys of each operator are put together, and a key both write keeps the later value. `Filter::raw` and `Update::raw` take a document written by hand, for what has no typed form:

| Written | The document it builds |
|---------|------------------------|
| `paths.total().gt(1000.5)?.and(paths.paid_at().exists(false))` | `{ "$and": [{ "total": { "$gt": 1000.5 } }, { "paidAt": { "$exists": false } }] }` |
| `paths.total().gt(1000.5)?.or(paths.paid_at().exists(false))` | `{ "$or": [{ "total": { "$gt": 1000.5 } }, { "paidAt": { "$exists": false } }] }` |
| `paths.paid_at().exists(true).negated()` | `{ "$nor": [{ "paidAt": { "$exists": true } }] }` |
| `Filter::<Invoice>::raw(doc! { "details.costCenter": "CC-7" })` | `{ "details.costCenter": "CC-7" }` |
| `paths.total().set(99.5)?.and(paths.paid_at().unset())` | `{ "$set": { "total": 99.5 }, "$unset": { "paidAt": "" } }` |
| `Update::<Invoice>::raw(doc! { "$inc": { "total": 12.5 } })` | `{ "$inc": { "total": 12.5 } }` |

- **`nest` takes a filter or an update over the nested model, whichever module it comes from, and answers one of the module it was called in.** `address.city().eq(..)` is an `address_schema::Filter<Address>`. `customer.address().nest(..)` makes it a `customer_schema::Filter<Customer>`, and `paths.customer().nest(..)` an `invoice_schema::Filter<Invoice>`, which `to_chase` above joins with `and`. An update goes the same way, and `nest` tells the two apart by what it is handed: nothing is imported for it.
- **`nest` joins two keys with a dot, and leaves an empty one out with its dot.** `customer` and `address.city` are `customer.address.city`. A model serde flattens into the type that holds it has no key there, so its keys are written as they are. The one path of a type serde writes as one value is the empty key, so under `status` it is `status`.
- **A filter is nested whole.** Each of its keys is written under the path's, inside `$and`, `$or` and `$nor` too. Any other key that starts with `$`, which a `Filter::raw` document may write at its top, is kept as it is written. In an update, each key of each operator is written under the path's, and its value is kept.
- **A filter joins a filter over the same rows, whichever module it comes from.** What `and` and `or` ask of their argument is that it turn into a `bson::Document` and carry the row type, which every module's `Filter` does through two standard traits, `Into<bson::Document>` and `AsRef<PhantomData<Root>>`. `nest` asks the same of what it is handed, over the nested model. The answer is of the type `and` was called on.
- **An arm filter goes where a filter over its enum goes.** A `transport_schema::ArmFilter<Transport, Courier>` turns into a `bson::Document` and carries the row type `Transport` through the same two traits, so `nest` on a path that holds a `Transport` takes one, and so do `and`, `or`, `elem_match` on a list of the enum, and every operation. It is no `Filter` itself: `transport_schema::Filter::from(arm)` makes it the module's own `Filter<Transport>`, which starts a join and has `negated`. Its own `and` asks the two traits of its argument over the model the arm holds ([The arms of an enum under a tag](#operators)).
- **An arm update goes where an update of its enum goes.** A `transport_schema::ArmUpdate<Transport, Courier>` turns into a `bson::Document` and carries `AsRef<PhantomData<fn(Transport) -> Transport>>`, an update's marker over the rows of the enum, and no other. So `nest` on a path that holds a `Transport` takes one, and so do `and` on an update of the enum, `update_one` and `update_many`, and nothing that asks for a filter does. `transport_schema::Update::from(arm)` makes it the module's own `Update<Transport>`. Its own `and`, and `update` on the arm filter, ask the two traits of their argument over the model the arm holds ([The arms of an enum under a tag](#operators)).
- **A filter over another type's rows is refused.** Handed `Order::MONGO_FIELDS.placed().eq(true)?`, `and` on a filter over `Invoice` is ``error[E0277]: the trait bound `__tixschema_mongodb::Filter<Order>: AsRef<PhantomData<Invoice>>` is not satisfied``, and `nest` on `paths.customer()`, which holds a `Customer`, is ``error[E0277]: the trait bound `__tixschema_mongodb::Filter<Order>: Nests<Invoice, Customer, _>` is not satisfied``.
- **An update is refused where a filter is asked, and a filter where an update is.** An update carries `AsRef<PhantomData<fn(Root) -> Root>>` in place of a filter's marker. `paths.total().gt(1000.0)?.and(paths.total().set(0.0)?)` is ``error[E0277]: the trait bound `__tixschema_mongodb::Update<Invoice>: AsRef<PhantomData<Invoice>>` is not satisfied``, and `paths.total().set(0.0)?.and(paths.total().gt(1000.0)?)` is ``error[E0277]: the trait bound `__tixschema_mongodb::Filter<Invoice>: AsRef<PhantomData<fn(Invoice) -> Invoice>>` is not satisfied``.
- **A document is not a filter until `Filter::raw` makes it one.** `and` handed a `doc! { .. }` as it stands is ``error[E0277]: the trait bound `bson::Document: AsRef<PhantomData<Invoice>>` is not satisfied``. `raw` gives the document the row type of what it joins, and nothing checks what it holds: its keys and its values are the caller's.
- **`into_document()` hands over the document** of a filter or an update, and so does `bson::Document::from`, for a call made on the driver itself.

A map has no path, its keys being data, and an operator the typed paths do not write has no method. Both are written by hand and joined to the typed ones:

```rust
/// Invoices of one cost center that are no draft. `details` is a map: its keys are data.
fn of_cost_center(code: &str) -> Result<Filter<Invoice>, WriteError> {
    let paths = Invoice::MONGO_FIELDS;
    Ok(paths
        .status()
        .ne(InvoiceStatus::Draft)?
        .and(Filter::raw(doc! { "details.costCenter": code })))
}

/// A surcharge: `$inc`, an operator the typed paths do not write, beside one they do.
fn surcharged(by: f64) -> Result<Update<Invoice>, WriteError> {
    let paths = Invoice::MONGO_FIELDS;
    Ok(paths
        .tags()
        .push("surcharged".to_owned())?
        .and(Update::raw(doc! { "$inc": { "total": by } })))
}
```

| Written | The document it builds |
|---------|------------------------|
| `of_cost_center("CC-7")?` | `{ "$and": [{ "status": { "$ne": "draft" } }, { "details.costCenter": "CC-7" }] }` |
| `surcharged(12.5)?` | `{ "$push": { "tags": "surcharged" }, "$inc": { "total": 12.5 } }` |

### What each field becomes

A function is there for every key serde writes, and the kind it answers follows from how serde writes the field:

| The field | What its function answers |
|-----------|---------------------------|
| A plain value `T` | `Field<Root, T>` |
| `Option<T>` | `OptionalField<Root, T>` |
| `Vec<T>`, `VecDeque<T>`, `HashSet<T>`, `BTreeSet<T>`, `BinaryHeap<T>`, `LinkedList<T>`, `[T; N]`, `[T]`, and `IndexSet<T>` under the `indexmap` feature | `ListField<Root, T>` |
| A struct or an enum `M` declared with `#[model_schema(enable = ["decode_with", "mongodb"])]` above the type, which serde writes under keys | `Model<Root, M>`, whose `nest` reaches `M`'s own paths |
| `Option<M>` | `OptionalModel<Root, M>` |
| A list of `M` | `ModelList<Root, M>` |
| A flagged type serde writes as one value: a brand, a plain enum, a unit struct | `Field<Root, M>`, and `OptionalField<Root, M>` in an `Option`. A list of one is a `ListField<Root, M>` |
| `Box`, `Rc`, `Arc`, `Cow`, `Cell`, `Mutex`, `RefCell` or `RwLock` around any of these | The kind of what it holds, with a third parameter: `Model<Root, Box<Customer>, Customer>`. An operator over the whole value takes the type as written, `Box<Customer>`, and `nest` a filter over what it holds, `Customer` |
| `#[serde(flatten)]` of `M`, or of `Option<M>` | `Nested<Root, M>` at the empty key: `nest` writes `M`'s keys as they are, among those of the type that flattens it |
| A flattened field of any other type | No function: it has no key of its own |
| A map: bare, in an `Option`, behind one of those wrappers, flattened | No function |
| A type tixschema has not seen as a flagged model: another crate's, one declared below, a `type` alias, the type itself | One whole value: `Field`, `OptionalField` or `ListField` over it, whose `nest` reaches its paths |
| A member typed with one of the type's own parameters | One whole value of what fills it |
| `Option<Vec<T>>` | `OptionalField<Root, Vec<T>>`: one whole list a row may leave out |
| A list of lists | `ListField<Root, Vec<T>>`: each element is one whole list |
| A tuple | `Field<Root, (A, B)>`: one whole value |
| A std range: `Range<T>`, `RangeInclusive<T>`, `RangeFrom<T>`, `RangeTo<T>` | `Field<Root, Range<T>>`: one whole value |
| A std `Result<T, E>` | `Field<Root, Result<T, E>>`: one whole value |
| A field serde never writes, under `skip` or `skip_serializing` | No function |
| A field with a hook that writes it: `with`, `serialize_with`, the one `as_number` adds | `Field`, or `OptionalField` for an `Option`, whose values are written through the hook |
| A field serde writes and never reads back, under `skip_deserializing`, or reads through `deserialize_with` alone | One whole value, of the kind its `Option` or its list is |

What a flagged type's own paths are follows from the shape serde writes the type in:

| The flagged type | Its paths |
|------------------|-----------|
| A struct with named fields | One function per field, under the field's Rust name |
| A tuple struct with several slots | A tuple of paths, each slot's key its position: with `Point` a `struct Point(f64, f64)`, `Point::MONGO_FIELDS.0` writes the key `0`, and nested under a field `origin`, `origin.0`. A slot serde never writes keeps its place as `()` |
| An internally tagged enum | `is_<variant>()` is `$eq` on the tag's key, and `<tag>__<value>()` answers the arm filter of each variant. A variant's fields sit beside the tag: a function named after the variant answers a struct with one function per field. A variant that holds one flagged model answers a `Nested` at the empty key |
| An adjacently tagged enum | The same `is_<variant>()` and `<tag>__<value>()`. What a variant holds is under the content key: its fields, one value, or a tuple of positions, and each condition of its arm filter |
| An externally tagged enum | What a variant holds is under the name serde writes the variant as. `is_<variant>()` is `$exists` on that key, and `$eq` with the name for a variant that holds nothing |
| An untagged enum | One function per variant that holds a value, its paths at the enum's own level. No `is_<variant>()`: nothing stored names the variant |
| A brand, a single-slot tuple struct, a `#[serde(transparent)]` struct, a plain enum, a unit struct | One path at the empty key, which `MONGO_FIELDS` dereferences to, with the operators of a `Field`, and `regex` where the one value is a `String` |

- **A type serde writes as one value is one path.** `InvoiceStatus::MONGO_FIELDS.eq(InvoiceStatus::Paid)?` is a filter over an `InvoiceStatus`, at the empty key, which `nest` writes at the key that holds one. Held by another type the value is a plain one, a `Field` or an `OptionalField`, so every comparison is a plain call: `paths.status().eq(InvoiceStatus::PastDue)`. `regex` on one written as text is asked of the type's own path: with `callsign` a field typed with `Callsign`, a brand over a `String`, `paths.callsign().nest(Callsign::MONGO_FIELDS.regex("^mav", "i"))`. A list of one is a `ListField`, a list of plain values matched through `contains` and `element()`. A filter over the rows of such a type has no key to write its operator under, so `elem_match` on that list takes an `Element` and refuses one (``error[E0308]: `?` operator has incompatible types``).
- **A path is under the name serde writes.** `rename` on a field, `rename_all` on the type, `rename` and `rename_all` on a variant and `rename_all_fields` on an enum all count. A renaming written as a list counts by its `serialize` side, `rename(serialize = "ref", deserialize = "ref")` being the key `ref`. A list whose two sides name two keys is refused by every build that describes the type, a mobile client included; a build with no schema surface and no client takes it, and the path is the key that is written.
- **A function of an enum is named after its variant, in snake case**: a variant `OverLand` is the function `over_land()` and the method `is_over_land()`. A variant serde never writes has neither.
- **`is_<variant>()` keeps its name.** With variants `Card` and `IsCard`, `is_card()` asks for `Card` and `is_is_card()` for `IsCard`, and what `IsCard` holds is reached through `is_card2()`: the function of a variant that meets a taken name is numbered, as those of two variants with one snake-case name are.
- **An arm function is named `<tag>__<value>()`**: the tag key and the value serde writes for the variant, each in snake case, with `__` between them. An enum serde writes under a tag key has one per variant serde writes, whatever the variant holds. A tag key whose snake case starts with a digit gives its functions a leading `_`, since no Rust name starts with one: `#[serde(tag = "2fa")]` on `Gate { Open, Shut }` gives `Gate::MONGO_FIELDS._2fa__open()`, which writes `{ "2fa": { "$eq": "Open" } }`, and the refusals below name a function in that form, `_2fa__open()`. An externally tagged, an untagged and a plain enum have none, since they store no tag key. Beside the functions, the schema module of the enum declares `Arm`, an enum with one unit arm per variant that has a function: `transport_schema::Arm::ByCourier`.
- **No two functions of an enum's paths take one name.** `__` is in the name of an arm function and of no other, and what would break that is refused where the `enable` list is read, whether the enum lists `"mongodb"` or not, with the `mongodb` feature on and off. A variant whose Rust name holds `__` is ``model_schema: type `Transport`: variant `By__Courier` holds `__` in its name. Under a tag and `decode_with`, `__` parts the tag from the value in the function of each arm, `type__<value>()`. Rename the variant``. Two values that read as one in snake case are ``model_schema: type `Transport`: variants `ByCourier` and `Couriered` are written `by-courier` and `by_courier` under the tag `type`, and both read `by_courier` in snake case, so `type__by_courier()` would answer for both. Rename one of the two values``. An arm function that meets an `is_<variant>()`, which takes a tag key that reads `is` and a variant whose name starts with `_`, is ``model_schema: type `Transport`: the arm function `is__parcel()` of variant `Parcel` and `is__parcel()`, which asks for variant `_Parcel`, would be one function. Rename the variant `_Parcel` ``. An enum that lists `"decode_with"` alone has no arm function and is refused all the same, and an enum with no `enable` list is refused none of the three.
- **A generic type has its paths at each filling**: with `Wrapper<T>` declaring `inner: T` and `label: String`, `Wrapper::<u32>::MONGO_FIELDS.inner()` is a `Field<Wrapper<u32>, u32>`. The arms of a generic enum are at each filling too.

**A field with a serde hook is compared the way it is stored.** A `chrono` date writes text, so a date kept as a BSON date carries the `bson` library's hook, and a filter on it has to write a BSON date too. The path writes its values through the field's own hook, here bound as `date_hook`:

```rust
// version 2 of the `bson` library
use bson::serde_helpers::chrono_datetime_as_bson_datetime as date_hook;
// version 3
use bson::serde_helpers::datetime::FromChrono04DateTime as date_hook;
```

```rust
use window_schema::{Filter, WriteError};

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Window {
    #[serde(with = "date_hook")]
    pub closes_at: DateTime<Utc>,
    pub opens_at: DateTime<Utc>,
}

/// Windows that close before an instant, and windows that open before it.
fn closing_before(instant: DateTime<Utc>) -> Result<[Filter<Window>; 2], WriteError> {
    let paths = Window::MONGO_FIELDS;
    Ok([
        paths.closes_at().lt(instant)?,
        paths.opens_at().lt(instant)?,
    ])
}
```

For the instant `2026-10-07T17:40:00Z`, the two filters, the hooked date as a BSON date and the plain one as text:

```text
{ "closesAt": { "$lt": DateTime("2026-10-07 17:40:00.0 +00:00:00") } }
{ "opensAt": { "$lt": "2026-10-07T17:40:00Z" } }
```

Each hook lives behind the `bson` library's `chrono-0_4` feature. The path of the attribute is called as it is written, type arguments included, so a hook written `::serde_with::As::<Option<FromChrono04DateTime>>` on an `Option` of a date is the one its path writes through. A hook on a field that holds a list writes the whole list, so that field is one whole value, a `Field<Root, Vec<T>>`.

### A type tixschema has not seen

A field typed with a flagged model, one that lists `"mongodb"` or `"decode_with"` alone, is a `Model`, an `OptionalModel` or a `ModelList` only where tixschema has already expanded the model: a struct or an enum `#[model_schema]` is written on **above** the type that holds it, in the same crate. A type of another crate, one declared below, a plain `type` alias and the type itself are held as one whole value, a `Field`, an `OptionalField` or a `ListField` over the type, with the operators of that kind over the whole value. This is the reason every model of the section's examples is declared above `Invoice`.

`nest` reaches the paths below such a member as it does below a model: a `Field` and a `ListField` have it too.

```rust
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Shipment {
    pub carrier: Carrier,
    pub code: String,
}

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Carrier {
    pub name: String,
}

/// Shipments of one carrier. `Carrier` is declared below `Shipment`, so `carrier` is one whole
/// value there, and `nest` reaches the paths below it as it does below any model.
fn carried_by(name: &str) -> Result<shipment_schema::Filter<Shipment>, WriteError> {
    let shipment = Shipment::MONGO_FIELDS;
    let named = Carrier::MONGO_FIELDS.name().eq(name.to_owned())?;
    Ok(shipment
        .code()
        .regex("^S-", "")
        .and(shipment.carrier().nest(named)))
}
```

| Written | The document it builds |
|---------|------------------------|
| `carried_by("Acme Freight")?` | `{ "$and": [{ "code": { "$regex": "^S-", "$options": "" } }, { "carrier.name": { "$eq": "Acme Freight" } }] }` |

`Shipment::MONGO_FIELDS.carrier()` is a `Field<Shipment, Carrier>` there. `named` is a `carrier_schema::Filter<Carrier>`, and what `nest` answers is a `shipment_schema::Filter<Shipment>` under the key `carrier`.

- **A kind that wraps a model names the model as a third parameter.** `parent: Option<Box<Category>>` on `Category` is an `OptionalField<Category, Box<Category>, Category>`: `set` takes a `Box<Category>`, and `nest` a filter over the rows of `Category`. Where the field wraps nothing the third parameter is the second, so `Field<Invoice, f64>` names the kind whole.
- **`MongoPath` is one key**, as the type that declares the path writes it: `MongoPath::at("total")`, and `MongoPath::ROOT` for the empty key. `key()` reads it. A key holds a dot where the type's own shape puts two keys, `list.0` being the first slot of a tuple variant under the content key `list`. The module's `Path` is another type: where an issue of the recovering decode sits.
- **A path can be built by hand** through the constructors the generated ones are built with, `Field::plain(MongoPath::at("total"))` and its twins on `OptionalField` and `ListField`. Such a path writes whatever key it is given: nothing checks it against the type.

### The operations

Each operation is a function on a type that lists `"mongodb"`, over a `mongodb::Collection<bson::Document>`. The collection is one of documents, never of the type: a row is read through the recovering decode and written through the type's own `Serialize`, not by the driver's serde.

| Operation | What it sends | What it answers |
|-----------|---------------|-----------------|
| `Invoice::find_one(collection, filter)` | The driver's `find_one` | `Option<Invoice>`: the first row the filter matches, read as the type, and `None` where it matches none |
| `Invoice::find_one_with(collection, filter, resolvers)` | The same | The same, with the resolvers run over the row's issues |
| `Invoice::find(collection, filter)` | The driver's `find` | `Vec<Invoice>`: every row the filter matches |
| `Invoice::find_with(collection, filter, resolvers)` | The same | The same, with the resolvers run over each row's issues |
| `Invoice::count(collection, filter)` | The driver's `count_documents` | `u64`: how many rows the filter matches. No row is read |
| `invoice.insert_one(collection)` | The driver's `insert_one`, with the value written as a document | The driver's `mongodb::results::InsertOneResult` |
| `Invoice::update_one(collection, filter, update)` | The driver's `update_one` | The driver's `mongodb::results::UpdateResult` |
| `Invoice::update_many(collection, filter, update)` | The driver's `update_many` | The same |
| `Invoice::delete_one(collection, filter)` | The driver's `delete_one` | The driver's `mongodb::results::DeleteResult` |
| `Invoice::delete_many(collection, filter)` | The driver's `delete_many` | The same |

Every one of them fails with the crate's one [`OperationError`](#operationerror). Each bound to what it answers, in a function that compiles:

```rust
use bson::{Bson, Document, doc};
use mongodb::Collection;
use mongodb::options::{Collation, Hint};
use mongodb::results::{DeleteResult, InsertOneResult, UpdateResult};
use tokio::task::JoinHandle;

use invoice_schema::DecodeWith as _;
use invoice_schema::Stored as _;
use invoice_schema::{Expected, Filter, Issue, OperationError, Read, Resolution, Resolver};

/// Every operation, each bound to what it answers.
async fn every_operation(
    invoices: &Collection<Document>,
    invoice: &Invoice,
) -> Result<(), OperationError> {
    let paths = Invoice::MONGO_FIELDS;
    let drafts = || {
        paths
            .status()
            .eq(InvoiceStatus::Draft)
            .map_err(OperationError::Unwritable)
    };
    let voided = || {
        paths
            .total()
            .set(0.0_f64)
            .map_err(OperationError::Unwritable)
    };
    let resolvers: [Resolver<'_, Document, Bson>; 1] = [&whole_number_as_text];

    let _stored: InsertOneResult = invoice.insert_one(invoices).await?;
    let _first: Option<Invoice> = Invoice::find_one(invoices, drafts()?).await?;
    let _repaired: Option<Invoice> =
        Invoice::find_one_with(invoices, drafts()?, &resolvers).await?;
    let _every: Vec<Invoice> = Invoice::find(invoices, drafts()?).await?;
    let _every_repaired: Vec<Invoice> = Invoice::find_with(invoices, drafts()?, &resolvers).await?;
    let _how_many: u64 = Invoice::count(invoices, drafts()?).await?;
    let _changed: UpdateResult = Invoice::update_one(invoices, drafts()?, voided()?).await?;
    let _all_changed: UpdateResult = Invoice::update_many(invoices, drafts()?, voided()?).await?;
    let _deleted: DeleteResult = Invoice::delete_one(invoices, drafts()?).await?;
    let _all_deleted: DeleteResult = Invoice::delete_many(invoices, drafts()?).await?;
    Ok(())
}
```

`whole_number_as_text` is the resolver of [Resolvers](recovering-decode.md#resolvers), written there against another `Invoice` and here against this one: the same text, over `invoice_schema::Issue`.

- **A filter is any filter over the type's rows**, whichever module's `Filter` it is. A filter over another type's rows and an update are refused, by the bound a filter joins another under.
- **A filter and an update cannot change places.** `Invoice::update_one(invoices, paths.status().set(InvoiceStatus::Paid).map_err(OperationError::Unwritable)?, paths.number().eq("INV-0042".to_owned()).map_err(OperationError::Unwritable)?)` does not compile: ``error[E0277]: the trait bound `__tixschema_mongodb::Update<Invoice>: AsRef<PhantomData<Invoice>>` is not satisfied`` for the update handed as the filter, and ``error[E0277]: the trait bound `__tixschema_mongodb::Filter<Invoice>: AsRef<PhantomData<fn(Invoice) -> Invoice>>` is not satisfied`` for the filter handed as the update.
- **A read of many rows fails as a whole.** The rows are read one at a time as the driver hands them over, and the first that does not read ends the read with that row's `_id` and its issues. No row is answered.
- **`insert_one` writes the row when it is called**, before anything is awaited. A value BSON cannot hold, and a type serde writes as anything but a document, a plain enum or a brand, answer `OperationError::Unwritable` without MongoDB being asked anything. What is awaited holds no borrow of the value.
- **The updates and the deletes answer what the driver answers**: `UpdateResult` with its `matched_count`, `modified_count` and `upserted_id`, `DeleteResult` with its `deleted_count`, and `InsertOneResult` with the `inserted_id` of the row.
- **Every operation can be moved to another thread.** What each answers is `Send`, the resolvers of a read being `Sync`. A read of many rows asks that the row type be `Send`, the rows read so far being held while the next is awaited.

### Read options

`find_one`, `find`, `count` and the two that take resolvers do not answer a future. They answer a `<type>_schema::Read<'c, T, O>`, a read that asks nothing of MongoDB until it is awaited, and that takes the options it is to be run under first. `T` is the row type, and `O` what the read answers: `Vec<T>`, `Option<T>` or `u64`.

```rust
/// The largest unpaid invoices of one customer, largest first, and how many it has unpaid.
async fn largest_unpaid(
    invoices: &Collection<Document>,
    customer: &str,
    at_most: i64,
) -> Result<(Vec<Invoice>, u64), OperationError> {
    let paths = Invoice::MONGO_FIELDS;
    let unpaid = || {
        Customer::MONGO_FIELDS
            .name()
            .eq(customer.to_owned())
            .map(|named| paths.customer().nest(named))
            .map(|of_customer| of_customer.and(paths.paid_at().exists(false)))
            .map_err(OperationError::Unwritable)
    };
    let largest = Invoice::find_with(invoices, unpaid()?, &[&whole_number_as_text])
        .sort(doc! { "total": -1_i32 })
        .limit(at_most)
        .await?;
    let how_many = Invoice::count(invoices, unpaid()?).await?;
    Ok((largest, how_many))
}

/// One page of invoices in the order of their numbers: a read under every option it takes,
/// which asks nothing of MongoDB until it is awaited.
fn page(invoices: &Collection<Document>, at: u64) -> Read<'_, Invoice, Vec<Invoice>> {
    Invoice::find(invoices, Filter::raw(doc! {}))
        .sort(doc! { "number": 1_i32 })
        .skip(at * 20)
        .limit(20)
        .hint(Hint::Name("number_1".to_owned()))
        .collation(Collation::builder().locale("en").build())
}
```

| Option | Takes | Sent with |
|--------|-------|-----------|
| `sort` | A `bson::Document`, each key with `1` or `-1` | `find` and `find_one` |
| `limit` | An `i64`, as the driver's `find` takes it: `0` is every row | `find`, as given. A count is held to the same number: it is sent the limit's absolute value, and none for `0` |
| `skip` | A `u64`, the rows to pass over | `find`, `find_one` and `count` |
| `hint` | A `mongodb::options::Hint`, the index to use | `find`, `find_one` and `count` |
| `collation` | A `mongodb::options::Collation` | `find`, `find_one` and `count` |

- **An option the driver's own method does not have is not sent**: `limit` on a read of one row, which answers the first row whatever the limit, and `sort` on a count, which is the same in any order.
- **A sort is a document written by hand**, its keys the ones serde writes. There is no projection: a read answers the whole type.
- **What `unpaid` hands `nest` in `largest_unpaid` is a `customer_schema::Filter<Customer>`**, a filter over a customer. What `nest` answers is an `invoice_schema::Filter<Invoice>`, which `find_with` and `count` take.
- **An option is handed to the driver as it is given.** What a collation folds, and which hint MongoDB takes or refuses, are MongoDB's own to answer.
- **A read is `#[must_use]`.** One that is never awaited asks nothing, and the compiler says so: ``unused `__tixschema_mongodb::Read` that must be used``, with the note `a read asks nothing of MongoDB until it is awaited`.
- **A read is turned into a future by `.await`**, and is not one before. It implements `IntoFuture`, not `Future`, so something that takes a future is handed `.into_future()`, and a read handed over as it stands is ``error[E0277]: `__tixschema_mongodb::Read<'_, Invoice, Vec<Invoice>>` is not a future``:

```rust
/// A count handed to something that takes a future, which a read is not until it is turned
/// into one.
fn counted_elsewhere(
    invoices: &'static Collection<Document>,
) -> JoinHandle<Result<u64, OperationError>> {
    tokio::spawn(Invoice::count(invoices, Filter::raw(doc! {})).into_future())
}
```

### Rows that do not read as the type

`find_one` and `find` take no resolver, so any issue in a row refuses it: a key the type does not declare, a value held in another form, a value serde refuses, and a value that breaks a bound its field declares. `find_one_with` and `find_with` take the resolvers that repair such a row, and run them over each row's issues as [`from_bson_piped`](recovering-decode.md#resolvers) does, once.

A row older writers left, an invoice whose customer holds `"openInvoices": "3"`, is refused by the plain read with `OperationError::Unreadable`. Under version 2 of the `bson` library it displays as:

```text
the row ObjectId("6a7cc592ca0574e6efdfe217") does not read as expected: customer.openInvoices: invalid: expected U32, found String("3"): invalid type: string "3", expected u32
```

and under version 3, whose own wording the end of the line is:

```text
the row ObjectId("6a7cc592ca0574e6efdfe217") does not read as expected: customer.openInvoices: invalid: expected U32, found String("3"): BSON error. Kind: A deserialization-related error occurred. Message: invalid type: string "3", expected u32.
```

Read with `&[&whole_number_as_text]`, the same row reads.

**A type that declares no `_id` is refused on every stored row.** MongoDB stores every row under an `_id`, and to a type that does not declare one it is a key like any other, `Unknown` at `_id`. `Customer` declares none. A stored customer read by `Customer::find_one` fails with:

```text
the row ObjectId("6a7cc592ca0574e6efdfe217") does not read as expected: _id: unknown: found ObjectId("6a7cc592ca0574e6efdfe217")
```

A type whose rows are stored declares the `_id`, as `Invoice` does with `#[serde(rename = "_id")]`, or reads through a resolver that settles the key by taking it out:

```rust
/// The `_id` MongoDB stores every row under, which `Customer` does not declare: taken out.
fn id_dropped(
    raw: &mut Document,
    issue: &customer_schema::Issue<Bson>,
) -> customer_schema::Resolution {
    use customer_schema::{Issue, Resolution, Stored as _};

    let Issue::Unknown {
        path,
        found: _found,
    } = issue
    else {
        return Resolution::NotTouched;
    };
    if path.to_string() == "_id" && raw.remove_at(path) {
        Resolution::Settled
    } else {
        Resolution::NotTouched
    }
}

/// A customer by name, from a collection whose rows are customers.
async fn customer_named(
    customers: &Collection<Document>,
    name: &str,
) -> Result<Option<Customer>, customer_schema::OperationError> {
    let named = Customer::MONGO_FIELDS
        .name()
        .eq(name.to_owned())
        .map_err(customer_schema::OperationError::Unwritable)?;
    Customer::find_one_with(customers, named, &[&id_dropped]).await
}
```

With that resolver the same row reads as:

```text
Customer { address: Address { city: "Moca", postal_code: "56000" }, name: "Acme", open_invoices: 3 }
```

`insert_one` on such a type sends a row with no `_id`. MongoDB stores every row under one, so the stored row is one this type's own `find_one` refuses.

### `OperationError`

Every operation fails with `OperationError`, which the crate's MongoDB [support library](recovering-decode.md#the-support-libraries) declares once and the `<type>_schema` module of every type that lists `"mongodb"` re-exports:

| Member | When | It displays as |
|--------|------|----------------|
| `Database(mongodb::error::Error)` | MongoDB refused the operation, or could not be reached | `MongoDB refused the operation or could not be reached: `, then the driver's own error |
| `Unreadable { row, issues }` | A row did not read as the type | `the row `, the row's `_id`, ` does not read as expected: `, then one line per issue |
| `Unwritable(WriteError)` | A value of a filter, an update or an insert could not be written as BSON | `a value could not be written as BSON: `, then the `bson` library's own error |

- **`row` is the row's `_id` as `bson::Bson` displays it**, which keeps its BSON type in the text, so that the text `3` and the number `3` are told apart:

| The row's `_id` | `row` |
|-----------------|-------|
| an `ObjectId` | `ObjectId("6a7cc592ca0574e6efdfe217")` |
| the text `abc` | `"abc"` |
| the text `3` | `"3"` |
| the number `3` | `3` |
| no `_id` | `without an _id` |

- **`issues` holds the issues no resolver settled**, as the crate's `Issue<bson::Bson>`, in the order they were found. The line of a value serde refuses ends in the `bson` library's own wording, which differs between its major versions, and the line of a value that breaks a bound ends in the bound's own sentence, the same under both: match on the member, the path, `expected` and `found`.
- **`?` converts neither error into it.** A filter or an update the `bson` library could not write is turned by name, `.map_err(OperationError::Unwritable)`, and a call made on the driver itself by `.map_err(OperationError::Database)`, as the examples of this section do. With `?` alone, either is ``error[E0277]: `?` couldn't convert the error to `OperationError` ``.
- **It is an error like any other**: `Debug`, `Display`, `std::error::Error`, `Send`, `Sync` and `'static`. `source()` is the driver's error for `Database` and the `bson` library's for `Unwritable`, and `Unreadable` has none. It is `#[non_exhaustive]`, as each type the flag adds is.

### What a row stores

A row is written by the type's own `Serialize`, and a value of a filter or an update the same way, so what a filter writes is what the field stores. Two cases stand apart: a member whose serde form depends on the format, whose type reads and writes as JSON as before, and a map whose key is not text, whose type does not build.

**A member whose serde form depends on the format has two stored forms.** The generated reads and writes go through `bson::Serializer::new()` and `bson::Deserializer::new(..)`, which call themselves human-readable. A typed `Collection<T>` of the driver writes through the `bson` library's raw serializer, which does not. `std::net::IpAddr` is such a member: no schema surface describes it, so the example is one of a build with tixschema's `mongo` features alone.

```rust
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Host {
    pub address: IpAddr,
}
```

`insert_one` stores a host at `127.0.0.1` as:

```text
{ "address": "127.0.0.1" }
```

The same value written through a typed `Collection<Host>` is stored as `{ "address": { "V4": [127, 0, 0, 1] } }`, and a row in that form is refused by the generated read, as an `Invalid` at `address`. Under version 2 of the `bson` library, and then under version 3:

```text
the row without an _id does not read as expected: address: invalid: expected Unknown, found Document({"V4": Array([Int32(127), Int32(0), Int32(0), Int32(1)])}): invalid type: map, expected IP address
the row without an _id does not read as expected: address: invalid: expected Unknown, found Document({"V4": Array([Int32(127), Int32(0), Int32(0), Int32(1)])}): BSON error. Kind: A deserialization-related error occurred. Message: invalid type: map, expected IP address.
```

`ObjectId`, `bson::DateTime` and `bson::Uuid` are stored in one form by both serializers.

**A `u64` and an `i64` are stored as the integer** serde writes: a row `insert_one` writes, `mongo_written_row()`, `bson::to_document` of a model, and the value of a filter and of an update hold an `Int64`. The generated read takes the `Int64`, and the string of its digits a stored row may hold, alike, and a `u64` beyond `i64::MAX` is refused as a value BSON cannot hold.

**A map whose key is not text does not build.** tixschema takes a map keyed by a number or a `bool`, because serde writes such a key as text into JSON, and into JSON only. BSON document keys are always text: the `bson` library refuses to write such a key, and hands a stored one back to the key's type as text, which the type refuses. So a type that lists `"mongodb"` and holds such a map is refused where it is declared, in every build:

```rust
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Roster {
    pub by_number: HashMap<u32, String>,
}
```

```text
error: model_schema: type `Roster`: field `by_number`: a map keyed by `u32` cannot be stored in MongoDB — BSON document keys are always text, and the `bson` library refuses to write this key and to read it back. Key the map by a `String`, or do not list "mongodb" in `enable`.
```

- **The keys refused** are the integer types, `f32`, `f64` and `bool`, and a brand or an alias declared above over one of them, each also under `Box`, `Rc`, `Arc` and `Cow`. A generic brand or alias declared above whose value is its own type parameter, `#[serde(transparent)] pub struct Ordinal<IdType>(pub IdType);` or `pub type Slot<T> = T;`, is refused where that parameter is filled at one of them: `Ordinal<u32>` and `Slot<bool>` are refused, and `Ordinal<String>` is not. So is one whose value reaches its parameter under `Box`, `Rc`, `Arc` or `Cow`, `#[serde(transparent)] pub struct Boxed<T>(pub Box<T>);` or `pub type Held<T> = Box<T>;`, or through such a brand or alias declared above it, `#[serde(transparent)] pub struct Rank<T>(pub Ordinal<T>);`: `Boxed<u32>`, `Held<u32>` and `Rank<u32>` are refused, and `Rank<String>` is not. A brand reached through one declared below it is not refused. The error names the field and the key as written, once for each such map the field holds.
- **The map is found in every member serde writes or reads**, of a struct and of each variant of an enum, behind an `Option`, a list, a tuple, a wrapper, another map's value, either arm of a `Result` and the bound of a range: a field written `Option<Vec<BTreeMap<bool, u32>>>` or `Result<HashMap<u32, String>, String>` is refused as `by_number` is. A type of the crate's own declared above under the name `Result`, `Range`, `RangeInclusive`, `RangeFrom` or `RangeTo` is read as the model it is, and the std type written in full under `std` or `core`, `std::result::Result` or `core::ops::Range`, is read as the std type. A member under `#[serde(skip)]` is not, and neither is a member or a variant written and read by functions of its author's, under `#[serde(with = "...")]` or under both `serialize_with` and `deserialize_with`: what those functions store is theirs to answer for. A member or a variant under one of the two alone is refused, serde still writing or reading the map itself.
- **A model or an alias declared above is looked into.** A member that holds a `#[model_schema]` struct, enum or alias declared above the type, behind the same `Option`, list, tuple, wrapper, map's value, arm of a `Result` and bound of a range, is refused where that model or alias holds such a map, at any depth and whatever the model's own `enable` list. The error names the member, the model or the alias, and the first such map it holds. The members and the variants the bullet above reads past are read past here too, in the type and in the model. A model or an alias declared below the type, or of another crate, is not looked into.
- **A model or an alias declared above is read at the arguments it is held with.** A model that keys a map by one of its own type parameters, `pub struct Keyed<KeyType> { pub by_key: HashMap<KeyType, String> }`, is refused where that parameter is filled at a refused key: a member `keyed: Keyed<u32>` is refused, and `Keyed<String>` is not. An alias that keys a map by one of its own type parameters, `pub type ByKey<KeyType> = HashMap<KeyType, String>;`, is refused the same way: a member `by_key: ByKey<u32>` is refused, and `ByKey<String>` is not. A model or an alias that hands its parameter on to such a model is refused the same way, at what fills it: `pub type Rekeyed<K> = Keyed<K>;` held as `Rekeyed<u32>` is refused. A map an argument holds is refused as one the member holds itself: `Wrapper<HashMap<u32, String>>` is refused, and `Wrapper<HashMap<String, String>>` is not. The error names the member, the model or the alias as written, and the map:

  ```text
  error: model_schema: type `Arena`: field `keyed`: `Keyed<u32>` holds a map that cannot be stored in MongoDB (field `by_key`, a map keyed by `u32`) — BSON document keys are always text, and the `bson` library refuses to write this key and to read it back. Key that map by a `String`, or do not list "mongodb" in `enable`.
  ```

  Every argument is read as one the model writes into its row: a map an argument holds is refused even where the model holds that argument only in a `PhantomData`, or only in a member serde neither writes nor reads. A model or an alias declared below the type, or of another crate, is not read at its arguments.
- **A type that lists `"mongodb"` and keys a map by its own type parameter builds, whatever fills the parameter.** The filling is written where the type is used, which no expansion sees, so nothing is refused:

  ```rust
  #[model_schema(enable = ["decode_with", "mongodb"], default_types(KeyType = String))]
  #[derive(Debug, Deserialize, Serialize)]
  pub struct Keyed<KeyType: Hash + Eq> {
      pub by_key: HashMap<KeyType, String>,
  }
  ```

  `Keyed<String>` is stored and read back: a value that maps `"8"` to `"eight"` is stored as `{ "by_key": { "8": "eight" } }`. `Keyed<u32>` builds as well, and so does a filling at any other integer type or at `bool`, and each fails at run time: every write of a value whose map holds an entry, `insert_one` among them, and every read of a row whose map holds one, `find_one` and `find` among them. A value whose map is empty is stored and read back. Under version 2 of the `bson` library, the write of a `Keyed<u32>` that maps `8` to `"eight"`, and then the read of the row above as `Keyed<u32>`:

  ```text
  a value could not be written as BSON: Invalid map key type: 8
  the row without an _id does not read as expected: undescribed: invalid type: string "8", expected u32
  ```

  Under version 3:

  ```text
  a value could not be written as BSON: BSON error. Kind: A serialization error occurred. Message: invalid document key type: int64.
  the row without an _id does not read as expected: undescribed: BSON error. Kind: A deserialization-related error occurred. Message: invalid type: string "8", expected u32.
  ```
- **A key serde writes as text is stored as that text and read back**: a `String`, a `char`, a `chrono` date or time, a plain enum, and a brand over a `String`. A key the expansion cannot see into, a type of another crate or a brand declared below the type, is not refused.
- **A type that lists `"decode_with"` alone builds with such a map** and reads it from JSON. Its `from_bson_piped` refuses a stored row that holds the map, with one `Undescribed` issue.

### A query the operations do not cover

The operations cover a filter, an update and the five read options. An aggregation, an upsert, a find-one-and-update and a projection are calls on the driver itself, on the same `Collection<bson::Document>`:

- a typed filter or update is handed to the driver as a document, through `into_document()`;
- each row the driver answers is read with `from_bson_piped` or `from_bson_with`, into a flagged type;
- a row is read as the driver's cursor hands it over, with `advance` and `deserialize_current`, so reading many rows asks no `futures` crate of the caller, as the generated `find` asks none.

An aggregation answers rows that are no rows of `Invoice`, so they are read as a type of their own. `"decode_with"` gives it `from_bson_piped`, and `Billed` lists `"mongodb"` as well, so that `billed_schema` re-exports `OperationError`:

```rust
/// What one customer was billed: no row of `Invoice`, so no read of `Invoice` answers it.
#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
pub struct Billed {
    #[serde(rename = "_id")]
    pub customer: String,
    pub total: f64,
}

/// What each customer was billed, by an aggregation run on the driver itself. Each row it
/// answers is read as the operations read one, and fails as they fail.
async fn billed(
    invoices: &Collection<Document>,
) -> Result<Vec<Billed>, billed_schema::OperationError> {
    use billed_schema::OperationError;

    let pipeline = [
        doc! { "$group": { "_id": "$customer.name", "total": { "$sum": "$total" } } },
        doc! { "$sort": { "_id": 1_i32 } },
    ];
    let mut cursor = invoices
        .aggregate(pipeline)
        .await
        .map_err(OperationError::Database)?;
    let mut rows = Vec::new();
    while cursor.advance().await.map_err(OperationError::Database)? {
        let row = cursor
            .deserialize_current()
            .map_err(OperationError::Database)?;
        let id = row
            .get("_id")
            .map_or_else(|| "without an _id".to_owned(), ToString::to_string);
        let read =
            Billed::from_bson_piped(row, &[]).map_err(|refused| OperationError::Unreadable {
                row: id,
                issues: refused.issues,
            })?;
        rows.push(read);
    }
    Ok(rows)
}
```

The function fails with `billed_schema::OperationError`, the crate's one `OperationError`, built as the generated reads build it: `Database` by name for each call on the driver, and `Unreadable` from the row's `_id` and the issues of the refused read.

### Limits

- **A type that lists `enable = ["jsonschema"]` and holds a type declared in the same function body does not build.** Its JSON Schema names the held type's schema module, which no module beside the holder reaches: ``error[E0433]: cannot find module or crate `inner_schema` in this scope``. Declare the held type at module level, as [Function-Local Types](troubleshooting.md#function-local-types) says.
- **An operation is a function of the type itself, under its own name.** A type that lists `"mongodb"` and already has a function of its own called `find_one`, `find_one_with`, `find`, `find_with`, `count`, `insert_one`, `update_one`, `update_many`, `delete_one` or `delete_many` builds without the `mongodb` feature and stops building with it. For a type with a `count` of its own: ``error[E0592]: duplicate definitions with name `count` ``.
- **A slot of a tuple struct under `skip_serializing_if` moves the positions after it.** The paths are written for a value that holds every slot. With `pair` a field typed with `Pair`, a tuple struct of an `Option<u32>` under `skip_serializing_if` and then a `String`, `Pair::MONGO_FIELDS.1` nested under `paths.pair()` writes the key `pair.1`, and a value whose first slot is `None` is stored as `{ "pair": ["x"] }`, its text at position `0`.

### This repository's checks against a server

The checks that run the operations against a real collection are in `tests/mongodb_operation_tests/`: `live.rs`, and this section's own examples beside it. Both major versions of the `bson` library compile them.

- **They read `TIXSCHEMA_MONGODB_URI`**, a MongoDB connection string. Each check works in a collection of its own in the database `tixschema_live`, named after the check, the `bson` major and the process, and drops it when it is done.
- **Without the variable every one of them stands down**, and says so once on stderr: a `cargo test` that passes with no server named has run none of them. With the variable set, a value that is no address and a server that does not answer are failures.
- **`just test-mongodb` refuses to stand down.** It stops with `No MongoDB server: set TIXSCHEMA_MONGODB_URI to one's address, as in mongodb://127.0.0.1:27017.` where the variable is not set, and runs the checks under both major versions where it is.
- **CI does not run them**: no MongoDB server comes with the CI runner. What `cargo test` proves with no server is what needs none: the documents the paths write, the read of a row once the driver has handed it over, and a row refused before it is sent.
