//! tixschema is a procedural macro crate that publishes Rust types and traits to the languages on
//! the other side of a wire.
//!
//! One declaration, read the way serde reads it, gives a TypeScript type, a Zod v4 schema, a JSON
//! Schema document, Dart, Swift and Kotlin types with their JSON codecs, the same field rules in
//! every one of them and, from a trait, a service with its dispatchers and clients.
//!
//! Nothing of tixschema runs in your program. The macros write code into your crate, and that code
//! names only crates your own `Cargo.toml` lists.
//!
//! # What it solves
//!
//! A Rust server and its clients describe the same payloads several times: serde types in Rust, a
//! type and a runtime check in TypeScript, a model class in each mobile app, and often a JSON
//! Schema for the documentation. Written by hand, the copies drift apart. A renamed field, a new
//! enum variant or a length rule added on one side fails at run time on another.
//!
//! tixschema writes every copy from the Rust declaration, with the serde attributes the server
//! reads and writes by, so the copies change when the declaration changes. A rule declared once,
//! such as a minimum length, is checked by the Rust read, by Zod and by the Dart, Swift and Kotlin
//! reads with the same message, and is written into the JSON Schema.
//!
//! Crates such as ts-rs and specta write TypeScript types from Rust types, typeshare writes
//! TypeScript, Swift and Kotlin types, and schemars writes JSON Schema. From one declaration,
//! tixschema writes TypeScript types and JSON Schema, and also:
//!
//! - Zod 4 schemas that check a payload at run time;
//! - Dart, Swift and Kotlin types with JSON codecs whose read calls a `validate()` that holds the
//!   Rust rules;
//! - each rule written once and refused with the same message on every side;
//! - from a trait, a service with Rust dispatchers, and clients in Rust, TypeScript, Dart, Swift
//!   and Kotlin.
//!
//! Use it when:
//!
//! - a Rust backend serves a TypeScript web app, a Flutter app, or native iOS and Android apps, and
//!   every side must agree on the JSON;
//! - a payload that breaks a rule (a length, a range, a pattern, a check across fields) must be
//!   refused at the boundary, with a message that names the field;
//! - a team publishes JSON Schema for its API or for its configuration files;
//! - a service talks over HTTP, a WebSocket or an AMQP broker, and its clients should be typed
//!   without being written by hand;
//! - records stored in `MongoDB` need typed filters and updates, and rows that older code wrote
//!   need repair as they are read.
//!
//! # What it generates
//!
//! In the table, `T` is a type declared with [`#[model_schema]`](attr.model_schema.html), and
//! `{type}` is its Rust name in snake case: `OrderLine` gives `order_line_dart`. `{service}` is the
//! snake case name of a trait declared with [`#[service_schema]`](attr.service_schema.html), and
//! `{Service}Schema` is the trait's name followed by `Schema`.
//!
//! | Output | Where it comes from | Feature |
//! |---|---|---|
//! | TypeScript types | `T::ts_definition()` | `typescript` |
//! | Zod v4 schemas | `T::zod_schema()` | `zod` |
//! | JSON Schema documents | `T::json_schema()`, a `serde_json::Value` | `jsonschema` |
//! | Dart classes with `fromJson`, `toJson` and `validate()` | `{type}_dart::dart_definition()` | `dart` |
//! | Swift `Codable` types with `validate()` | `{type}_swift::swift_definition()` | `swift` |
//! | Kotlin `kotlinx.serialization` classes with `validate()` | `{type}_kotlin::kotlin_definition()` | `kotlin` |
//! | Serde renames, tags, flattening and defaults, read on every surface | the `#[serde(...)]` attributes already on the type | `serde` |
//! | Field and type rules | `#[model_schema_prop(...)]` on a field and `#[model_schema(check = [...])]` on the type, giving `T::validate()` and a serde read that refuses what `validate()` refuses | `serde` with `typescript`, `zod` or `jsonschema`; a `check` in every build |
//! | Recovering decode of old or untrusted values | `#[model_schema(enable = ["decode_with"])]` and `tixschema::decode_with_foundation!()`, giving `T::from_value_with` and `T::from_value_piped` | `serde`; `bson` adds `T::from_bson_with` and `T::from_bson_piped` |
//! | `MongoDB` typed paths, filters, updates and operations | `enable = ["decode_with", "mongodb"]` and `tixschema::mongodb_foundation!()`, giving `T::MONGO_FIELDS`, `T::find_one`, `T::find`, `T::count`, `value.insert_one`, `T::update_one`, `T::delete_one` and their siblings | `mongodb` |
//! | `ObjectId` fields, chrono dates and times, `IndexMap` and `IndexSet` | a field of that type | `mongodb`, `chrono`, `indexmap` |
//! | Conversions to and from arky's frontmatter `Value` | `enable = ["arky"]`, giving `TryFrom` in both directions | `arky` |
//! | A service: the trait, its messages and its fault type | [`#[service_schema]`](attr.service_schema.html) and [`#[service_schema_op]`](attr.service_schema_op.html) on a trait | `serde` |
//! | Rust dispatchers and clients for `amqp_rpc`, `http_rest` and `ws_rpc` | `#[service_schema(transports = [...])]`, then the `{service}_{transport}_dispatcher!()` and `{service}_{transport}_client!()` macros it writes, and `{service}_amqp_rpc_server!()` | `serde` |
//! | TypeScript service clients and dispatchers | `{Service}Schema::ts_client()`, `ts_service()`, `ts_http_client()`, `ts_http_service()`, `ts_ws_client()`, `ts_ws_service()` and `ts_ws_server()` | `serde`, `typescript` and `zod` |
//! | Dart service messages, clients and a WebSocket dispatcher | `{Service}Schema::dart_definition()`, `dart_http_client()` and `dart_ws_client()` | `serde` and `dart` |
//! | Swift service clients | `{Service}Schema::swift_http_client()` and `swift_ws_client()` | `serde` and `swift` |
//! | Kotlin service clients and a WebSocket dispatcher | `{Service}Schema::kotlin_http_client()` and `kotlin_ws_client()` | `serde` and `kotlin` |
//!
//! The functions that write a surface's text return it as a `String`: `ts_definition()`,
//! `zod_schema()`, `dart_definition()`, `swift_definition()`, `kotlin_definition()` and every
//! function of `{Service}Schema`. `json_schema()` returns a `serde_json::Value`. The other items
//! are code your program calls: `T::validate()` returns `Result<(), Vec<String>>`, the recovering
//! reads return `Result<T, Unrecovered<_>>`, `T::MONGO_FIELDS` is a `const`, `T::find_one`,
//! `T::find` and `T::count` return a `Read` you await, and the other `MongoDB` operations return
//! futures of the driver's results. A type alias has no methods of its own: its surfaces are
//! `{type}_schema::Schema::ts_definition()` and its siblings.
//!
//! # Getting started
//!
//! Add tixschema and the crates the generated code names:
//!
//! ```toml
//! [dependencies]
//! tixschema = "0.2"
//! serde = { version = "1", features = ["derive"] }
//! serde_json = "1"
//! ```
//!
//! tixschema needs Rust 1.91.1 or later. The default features need `serde_json`, since
//! `json_schema()` returns a `serde_json::Value`. Add
//! `regex = "1"` when a type declares a `pattern` that is more than a fixed text: the Rust check
//! of such a pattern builds a `regex::Regex`. A fixed text, anchored or not, such as `^INV-`, is
//! checked with `str` comparisons and needs no `regex`. The TypeScript project installs Zod 4
//! with `npm install zod@^4`.
//!
//! Put `#[model_schema()]` on each type, above its `#[derive(...)]`, and write the TypeScript file
//! from a test of your crate:
//!
#![cfg_attr(
    all(feature = "serde", feature = "typescript", feature = "zod"),
    doc = r##"
```rust
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Customer {
    pub customer_id: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    pub order_count: u32,
}

#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderLine {
    pub sku: String,
    pub quantity: u32,
}

#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    pub customer: Customer,
    pub lines: Vec<OrderLine>,
}

/// The lines every bundle opens with: the names the generated code uses and does not declare.
const HEADER: &str = r#"import { z, type ZodType, type $brand } from "zod";
import type { SomeType } from "zod/v4/core";
"#;

/// The TypeScript file of the API: the header, then each type after the types it holds.
fn models_ts() -> String {
    let mut bundle = String::from(HEADER);
    for surface in [
        Customer::ts_definition(),
        Customer::zod_schema(),
        OrderLine::ts_definition(),
        OrderLine::zod_schema(),
        Order::ts_definition(),
        Order::zod_schema(),
    ] {
        bundle.push('\n');
        bundle.push_str(&surface);
        bundle.push('\n');
    }
    bundle
}

/// `cargo test write_models` writes the file the web project imports.
#[test]
fn write_models() {
    std::fs::write("web/src/api/models.ts", models_ts()).unwrap();
}

fn main() {
    let typescript = Customer::ts_definition();
    assert!(typescript.contains("export type Customer = {"));
    assert!(typescript.contains("customerId: string;"));
    assert!(typescript.contains("email: string | undefined;"));
    assert!(typescript.contains("orderCount: number;"));

    let zod = Customer::zod_schema();
    assert!(zod.contains("const Customer$RawSchema = z.strictObject({"));
    assert!(zod.contains("orderCount: z.number().int().min(0).max(4294967295)"));
    assert!(zod.contains("export const Customer$Schema: ZodType<Customer>"));

    let order = Order::zod_schema();
    assert!(order.contains("customer: Customer$Schema,"));
    assert!(order.contains("lines: z.array(OrderLine$Schema),"));

    let bundle = models_ts();
    let declared = |name: &str| bundle.find(&format!("export const {name}$Schema")).unwrap();
    assert!(declared("Customer") < declared("Order"));
    assert!(declared("OrderLine") < declared("Order"));
}
```
"##
)]
//!
//! `ts_definition()` returns the TypeScript and no Zod, and `zod_schema()` the Zod schema, so a
//! bundle calls both for each type. Write a type ahead of the types that hold it: `Order`'s schema
//! names `Customer$Schema` and `OrderLine$Schema` directly, and a JavaScript `const` cannot be
//! read before its declaration. A test or a small binary of your crate writes the file; a build
//! script runs before your crate is compiled, so it cannot call these functions. Commit the
//! written file, so that a change to a Rust type shows in review as a change to the TypeScript.
//!
//! The bundle declares every name it uses but these, which the header imports or your project
//! declares:
//!
//! - `z` and `ZodType`: every Zod schema.
//! - `$brand`: a branded newtype, such as `string & $brand<"InvoiceNumber">`.
//! - `SomeType`: an optional or `nullable` field that holds a generic type.
//! - `ObjectId`: an `ObjectId` field under the `mongodb` feature. Your project declares it, for
//!   example as `type ObjectId = { $oid: string };`.
//! - The functions a `check` or a `preprocess` names: Zod calls each by its name, so the bundle
//!   declares or imports it.
//!
//! The README's
//! [Generating TypeScript Files](https://github.com/tixena/tixschema/blob/master/README.md#generating-typescript-files)
//! shows other ways to run the generation.
//!
//! # Use cases
//!
//! ## One contract for an API and its web app
//!
//! An order service sends events to a web app, which narrows each one on its `type` key. The enum
//! is declared once with serde's tag. The web app gets a discriminated union and a Zod schema that
//! reads it, and the documentation gets a JSON Schema:
//!
#![cfg_attr(
    all(
        feature = "serde",
        feature = "typescript",
        feature = "zod",
        feature = "jsonschema"
    ),
    doc = r##"
```rust
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum OrderEvent {
    OrderPlaced { order_id: String, total_cents: u32 },
    OrderCancelled { order_id: String, reason: String },
}

let typescript = OrderEvent::ts_definition();
assert!(typescript.contains(r#"type: "orderPlaced";"#));
assert!(typescript.contains("orderId: string;"));
assert!(typescript.contains("export function OrderEvent$Variant(value: unknown): string"));

let zod = OrderEvent::zod_schema();
assert!(zod.contains(r#"z.discriminatedUnion("type", ["#));
assert!(zod.contains(r#"type: z.literal("orderCancelled"),"#));

let document = OrderEvent::json_schema();
assert_eq!(document["oneOf"].as_array().map(Vec::len), Some(2));
assert_eq!(document["oneOf"][0]["properties"]["type"]["const"], "orderPlaced");
```
"##
)]
//!
//! The web app reads an event through the schema, and TypeScript narrows it on `type`:
//!
//! ```typescript
//! import { OrderEvent$Schema } from "./api/models";
//!
//! const event = OrderEvent$Schema.parse(await response.json());
//! switch (event.type) {
//!   case "orderPlaced":
//!     showTotal(event.orderId, event.totalCents);
//!     break;
//!   case "orderCancelled":
//!     showReason(event.orderId, event.reason);
//!     break;
//! }
//! ```
//!
//! `parse` throws a `ZodError` for a payload the schema refuses. `rename_all` on an enum renames
//! its variants and `rename_all_fields` the fields inside them, as serde does.
//! `OrderEvent$Variant(value)` answers the Rust variant a value carries. The README's
//! [Enums](https://github.com/tixena/tixschema/blob/master/README.md#enums) covers every tagging,
//! and [Type Parameters](https://github.com/tixena/tixschema/blob/master/README.md#type-parameters)
//! and [Branded Newtypes](https://github.com/tixena/tixschema/blob/master/README.md#branded-newtypes)
//! cover generic envelopes such as `Page<T>` and ids that cannot be swapped.
//!
//! ## Rules checked at the boundary
//!
//! A sign-up service must refuse a user name shorter than three characters and an age over 120,
//! with a message that names the field. Each rule is written once, on its field:
//!
#![cfg_attr(
    all(feature = "serde", feature = "zod", feature = "jsonschema"),
    doc = r##"
```rust
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Signup {
    #[model_schema_prop(minLength = 3, maxLength = 30, pattern = "^[a-z0-9_]+$")]
    pub user_name: String,
    #[model_schema_prop(minimum = 13, maximum = 120)]
    pub age: u32,
}

// serde's own read refuses the payload, naming each field that broke a rule.
let refused = serde_json::from_str::<Signup>(r#"{"userName":"ab","age":150}"#).unwrap_err();
let said = refused.to_string();
assert!(said.contains("'userName': too short: minimum length is 3, got 2"));
assert!(said.contains("'age': too large: maximum is 120, got 150"));

// A value built in code is judged by `validate()`.
let built = Signup { user_name: "ada_l".to_owned(), age: 36 };
assert!(built.validate().is_ok());

// The same rules reach Zod and JSON Schema.
assert!(Signup::zod_schema().contains("userName: z.string().min(3, "));
assert_eq!(Signup::json_schema()["properties"]["userName"]["minLength"], 3);
```
"##
)]
//!
//! The macro writes the type's `Deserialize` as serde's read followed by `validate()`, so every
//! serde read refuses a value that breaks a rule, with every message joined by `; `. `validate()`
//! stays for a value built in code. `#[model_schema(check = ["withinBudget"])]` names a function
//! of yours for a rule across fields: Rust calls `within_budget`, and Zod, Dart, Swift and Kotlin
//! call `withinBudget`. See [`model_schema_prop`](attr.model_schema_prop.html) and the README's
//! [Field Validation](https://github.com/tixena/tixschema/blob/master/README.md#field-validation-model_schema_prop).
//!
//! ## Configuration files checked by JSON Schema
//!
//! A worker reads its settings from a JSON file that people edit by hand. The settings struct
//! carries the rules. Its JSON Schema lets an editor check the file as it is typed, and the serde
//! read refuses a file that breaks a rule when the worker starts:
//!
#![cfg_attr(
    all(feature = "serde", feature = "jsonschema"),
    doc = r##"
```rust
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkerConfig {
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[model_schema_prop(minLength = 1)]
    pub queue_name: String,
    #[model_schema_prop(minimum = 1, maximum = 64)]
    pub worker_count: u32,
}

/// `cargo test write_config_schema` writes the schema the config file names.
#[test]
fn write_config_schema() {
    let schema = serde_json::to_string_pretty(&WorkerConfig::json_schema()).unwrap();
    std::fs::write("config.schema.json", schema).unwrap();
}

fn main() {
    let schema = WorkerConfig::json_schema();
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(schema["properties"]["workerCount"]["maximum"].as_f64(), Some(64.0));
    assert_eq!(schema["required"], serde_json::json!(["queueName", "workerCount"]));

    let read: WorkerConfig = serde_json::from_str(
        r#"{"$schema":"./config.schema.json","queueName":"invoices","workerCount":8}"#,
    )
    .unwrap();
    assert_eq!(read.worker_count, 8);

    let refused =
        serde_json::from_str::<WorkerConfig>(r#"{"queueName":"invoices","workerCount":0}"#)
            .unwrap_err();
    assert!(refused.to_string().contains("'workerCount': too small: minimum is 1, got 0"));
}
```
"##
)]
//!
//! The config file names its schema in its own `$schema` key, which editors read:
//!
//! ```json
//! {
//!   "$schema": "./config.schema.json",
//!   "queueName": "invoices",
//!   "workerCount": 8
//! }
//! ```
//!
//! The schema closes every object with `"additionalProperties": false`, so the struct declares
//! `$schema` as a field of its own, and the schema admits the key.
//!
//! ## Mobile apps that read what the server writes
//!
//! A Flutter app, an iOS app and an Android app read the invoices a Rust server writes. Each gets
//! the type in its own language, with a JSON codec that reads the keys serde writes and a
//! `validate()` that runs the server's rules:
//!
#![cfg_attr(
    all(
        feature = "serde",
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    ),
    doc = r#"
```rust
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Invoice {
    #[model_schema_prop(minLength = 1)]
    pub customer_name: String,
    pub total_cents: u32,
}

/// The Dart file a build step writes into the Flutter app: the imports, then each type.
fn dart_file() -> String {
    format!("import 'dart:convert';\n\n{}\n", invoice_dart::dart_definition())
}

/// The Swift file a build step writes into the iOS app.
fn swift_file() -> String {
    format!("import Foundation\n\n{}\n", invoice_swift::swift_definition())
}

/// The Kotlin file a build step writes into the Android app: its package, the imports the
/// serializers name, then each type.
fn kotlin_file() -> String {
    format!(
        "package com.example.billing\n\n\
         import kotlinx.serialization.*\n\
         import kotlinx.serialization.builtins.*\n\
         import kotlinx.serialization.descriptors.*\n\
         import kotlinx.serialization.encoding.*\n\
         import kotlinx.serialization.json.*\n\n\
         {}\n",
        invoice_kotlin::kotlin_definition()
    )
}

fn main() {
    let dart = dart_file();
    assert!(dart.contains("factory Invoice.fromJson(Map<String, dynamic> json)"));
    assert!(dart.contains("throw FormatException(violations.join('; '))"));

    let swift = swift_file();
    assert!(swift.contains("public struct Invoice: Codable, Sendable"));
    assert!(swift.contains("public func validate() -> [String]"));

    let kotlin = kotlin_file();
    assert!(kotlin.contains("data class Invoice("));
    assert!(kotlin.contains("fun validate(): List<String>"));
}
```
"#
)]
//!
//! The generated text holds no `import` and no `package` line: the build step that writes each
//! file adds them, as above. Dart imports `dart:convert`, and `dart:typed_data` or `dart:async`
//! where the code names them. Swift imports `Foundation`. Kotlin imports `kotlinx.serialization.*`
//! and its `builtins`, `descriptors`, `encoding` and `json` packages, and the build step writes
//! the `package` line the Android app expects.
//!
//! Each read calls `validate()` and refuses a broken payload with the messages joined by `; `:
//! Dart's `fromJson` throws a `FormatException`, Swift's `init(from:)` throws
//! `DecodingError.dataCorrupted`, and Kotlin's `InvoiceSerializer` throws a
//! `SerializationException`. The Flutter app reads an invoice like this:
//!
//! ```dart
//! import 'dart:convert';
//!
//! Invoice? readInvoice(String body) {
//!   try {
//!     return Invoice.fromJson(jsonDecode(body) as Map<String, dynamic>);
//!   } on FormatException catch (refused) {
//!     showError(refused.message);
//!     return null;
//!   }
//! }
//! ```
//!
//! The iOS app reads it with `try JSONDecoder().decode(Invoice.self, from: data)`, and the Android
//! app with `Json.decodeFromString<Invoice>(body)`. See the README's
//! [`validate()` in Dart](https://github.com/tixena/tixschema/blob/master/README.md#validate-in-dart),
//! [in the Swift client](https://github.com/tixena/tixschema/blob/master/README.md#validate-in-the-swift-client)
//! and [in the Kotlin client](https://github.com/tixena/tixschema/blob/master/README.md#validate-in-the-kotlin-client).
//!
//! ## A service with typed clients
//!
//! An orders service answers over a WebSocket. The trait is the contract. The macro writes the
//! service's fault type and, for each transport, macros that write the Rust dispatcher and client
//! into your crate. The dispatcher below answers frames with no socket and no runtime, which is
//! also how a test drives it:
//!
#![cfg_attr(
    all(
        feature = "serde",
        any(feature = "typescript", feature = "zod", feature = "jsonschema")
    ),
    doc = r##"
```rust
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::{model_schema, service_schema};

#[model_schema()]
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderLookup {
    #[model_schema_prop(minLength = 1)]
    pub order_id: String,
}

#[model_schema()]
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderSummary {
    pub order_id: String,
    pub total_cents: u32,
}

#[model_schema()]
#[derive(Deserialize, Serialize)]
#[serde(tag = "errorCode", rename_all = "kebab-case")]
pub enum OrderError {
    NotFound,
}

#[service_schema(transports = ["ws_rpc"])]
pub trait Orders<Ctx> {
    async fn get_order(&self, ctx: &Ctx, req: OrderLookup) -> Result<OrderSummary, OrderError>;
}

// The Rust dispatcher of the `ws_rpc` transport, written into a module of this crate.
mod orders_ws {
    orders_ws_rpc_dispatcher!();
}

pub struct OrderBook;

impl Orders<()> for OrderBook {
    async fn get_order(&self, _ctx: &(), req: OrderLookup) -> Result<OrderSummary, OrderError> {
        if req.order_id == "A-1" {
            Ok(OrderSummary { order_id: req.order_id, total_cents: 4250 })
        } else {
            Err(OrderError::NotFound)
        }
    }
}

/// Hands one text frame to the dispatcher. Nothing here waits, so one poll answers it.
fn reply_to(frame: &str) -> Value {
    use std::future::Future;
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};

    let mut answering = pin!(orders_ws::answer(frame, &OrderBook, &()));
    let Poll::Ready(Some(reply)) = answering.as_mut().poll(&mut Context::from_waker(Waker::noop()))
    else {
        panic!("the dispatcher did not answer {frame}");
    };
    serde_json::from_str(&reply).unwrap()
}

fn main() {
    let found = reply_to(
        r#"{"kind":"request","id":"1","service":"Orders","operation":"get-order","payload":{"orderId":"A-1"}}"#,
    );
    assert_eq!(
        found,
        json!({
            "id": "1",
            "kind": "reply",
            "ok": true,
            "service": "Orders",
            "value": { "orderId": "A-1", "totalCents": 4250 }
        })
    );

    // An error the operation declares travels as itself.
    let missing = reply_to(
        r#"{"kind":"request","id":"2","service":"Orders","operation":"get-order","payload":{"orderId":"B-7"}}"#,
    );
    assert_eq!(missing["error"], json!({ "errorCode": "not-found" }));

    // A message that breaks its own rule never reaches the implementation: it is a fault.
    let refused = reply_to(
        r#"{"kind":"request","id":"3","service":"Orders","operation":"get-order","payload":{"orderId":""}}"#,
    );
    assert_eq!(refused["error"]["fault"]["kind"], "failed-validation");
    assert_eq!(refused["error"]["fault"]["field"], "orderId");
}
```
"##
)]
//!
//! The dispatcher reads each message and checks it before it calls the implementation, and a
//! defect no operation declared reaches the caller as a fault, kept apart from the operation's own
//! errors. Over a real socket, the read loop calls `orders_ws::answer` for each text frame and
//! writes back the text it returns; a `None` writes nothing. The README's
//! [The `ws_rpc` Transport](https://github.com/tixena/tixschema/blob/master/README.md#the-ws_rpc-transport)
//! holds such a loop over axum.
//!
//! The same trait publishes the TypeScript, Dart, Swift and Kotlin clients through
//! `OrdersSchema`, and `http_rest` and `amqp_rpc` are the other transports. The
//! [`service_schema`](attr.service_schema.html#examples) examples call this service through a Rust
//! client, and the [`service_schema_op`](attr.service_schema_op.html#example) example serves
//! `http_rest` from an HTTP handler. See also the README's
//! [Services](https://github.com/tixena/tixschema/blob/master/README.md#services).
//!
//! ## Typed `MongoDB` filters and operations
//!
//! Invoices are stored in a `MongoDB` collection. Each field of `Invoice` gets a typed path, so a
//! misspelled field or a value of the wrong type is a compile error, and the operations read every
//! row through the recovering decode:
//!
#![cfg_attr(
    feature = "mongodb",
    doc = r##"
```rust
# extern crate bson2 as bson;
use bson::{Document, doc};
use mongodb::Collection;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use invoice_schema::{Filter, OperationError, WriteError};

tixschema::decode_with_foundation!();
tixschema::mongodb_foundation!();

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
#[serde(rename_all = "camelCase")]
pub struct Invoice {
    #[serde(rename = "_id")]
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paid_at: Option<String>,
    pub status: InvoiceStatus,
    pub total: f64,
}

/// Unpaid invoices that are past due or above `above`.
fn to_chase(above: f64) -> Result<Filter<Invoice>, WriteError> {
    let paths = Invoice::MONGO_FIELDS;
    Ok(paths
        .status()
        .eq(InvoiceStatus::PastDue)?
        .or(paths.total().gt(above)?)
        .and(paths.paid_at().exists(false)))
}

/// The largest of them, each row read as an `Invoice`. Nothing here calls it: it needs a server.
pub async fn largest_to_chase(
    invoices: &Collection<Document>,
) -> Result<Vec<Invoice>, OperationError> {
    let filter = to_chase(1000.5).map_err(OperationError::Unwritable)?;
    Invoice::find(invoices, filter).sort(doc! { "total": -1 }).limit(20).await
}

fn main() -> Result<(), WriteError> {
    assert_eq!(
        to_chase(1000.5)?.into_document().to_string(),
        r#"{ "$and": [{ "$or": [{ "status": { "$eq": "past-due" } }, { "total": { "$gt": 1000.5 } }] }, { "paidAt": { "$exists": false } }] }"#
    );
    Ok(())
}
```
"##
)]
//!
//! A key is the one serde writes, and a value is written by the field's own `Serialize`. The
//! options of a `Read` take no typed path: `sort` takes a `bson::Document` keyed by the names
//! serde writes, as `doc! { "total": -1 }` above, `hint` and `collation` take the driver's
//! `mongodb::options::Hint` and `mongodb::options::Collation`, `limit` an `i64` and `skip` a
//! `u64`. The operations take a `mongodb::Collection<bson::Document>`, and a row that does not
//! read as the type fails with `OperationError::Unreadable`, naming its `_id` and each issue. A
//! crate with such types writes `tixschema::decode_with_foundation!();` and
//! `tixschema::mongodb_foundation!();` once, at its root. See
//! [`mongodb_foundation`](macro.mongodb_foundation.html) and the README's
//! [`MongoDB` Operations and Typed Filters](https://github.com/tixena/tixschema/blob/master/README.md#mongodb-operations-and-typed-filters).
//!
//! ## Reading records an older writer stored
//!
//! An order line stored by older code holds its quantity as text and a key the type no longer
//! declares. serde stops at the first error and reads past an unknown key. The recovering decode
//! lists every issue at its path, hands the list to your code once, and reads the repaired value
//! again:
//!
#![cfg_attr(
    feature = "serde",
    doc = r#"
```rust
# extern crate bson2 as bson;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

use order_line_schema::{Expected, Issue, Stored as _, Verdict};

tixschema::decode_with_foundation!();

#[model_schema(enable = ["decode_with"])]
#[derive(Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderLine {
    pub sku: String,
    pub quantity: u32,
}

fn main() {
    let stored = json!({ "sku": "BK-101", "quantity": "3", "giftWrap": true });
    let read = OrderLine::from_value_with(stored, |raw, issues| {
        for issue in issues {
            let repaired = match issue {
                Issue::Invalid { path, expected: Expected::U32, found: Value::String(text), .. } => {
                    text.parse::<u32>()
                        .is_ok_and(|quantity| raw.set_at(path, Value::from(quantity)))
                }
                Issue::Unknown { path, .. } => raw.remove_at(path),
                _ => false,
            };
            if !repaired {
                return Verdict::Reject;
            }
        }
        Verdict::Fixed
    });
    assert_eq!(read.ok(), Some(OrderLine { sku: "BK-101".to_owned(), quantity: 3 }));
}
```
"#
)]
//!
//! `from_value_piped` takes small resolver functions in place of one callback, and under the `bson`
//! feature `from_bson_with` and `from_bson_piped` read a BSON document. See
//! [`decode_with_foundation`](macro.decode_with_foundation.html) and the README's
//! [Recovering Decode](https://github.com/tixena/tixschema/blob/master/README.md#recovering-decode-decode_with).
//!
//! # Features
//!
//! The default is `web`. Three sets group the plain features: `web` is `serde`, `zod`,
//! `jsonschema` and `typescript`; `mobile` is `serde`, `dart`, `swift` and `kotlin`; `mongo` is
//! `mongodb`, `chrono`, `indexmap` and `arky`. `mongodb` turns on `bson`, and `bson` and `arky`
//! turn on `serde`. A feature says what tixschema can generate, and a type's `enable` list says
//! what it generates for that type.
//!
//! | Feature | What it adds | What your crate lists |
//! |---|---|---|
//! | `serde` | Reads serde's attributes on every surface. With `typescript`, `zod` or `jsonschema` on, writes `validate()` and the read that refuses what it refuses. `#[service_schema]` and the recovering decode need it. | `serde` with its `derive` feature. |
//! | `typescript` | `T::ts_definition()`. | Nothing. |
//! | `zod` | `T::zod_schema()`, and `T::schema_examples()` on a type whose doc comment holds a code block fenced as `rust example`. | `serde_json` where a type holds such blocks. The TypeScript project installs `zod` at version 4. |
//! | `jsonschema` | `T::json_schema()`. | `serde_json`. |
//! | `dart` | `{type}_dart::dart_definition()`, and the Dart service clients. | Nothing in Rust. The Dart file imports `dart:convert`, and `dart:typed_data` or `dart:async` where the code names them; the code uses Dart 3 sealed classes, records and patterns. |
//! | `swift` | `{type}_swift::swift_definition()`, and the Swift service clients. | Nothing in Rust. The Swift file imports `Foundation`. |
//! | `kotlin` | `{type}_kotlin::kotlin_definition()`, and the Kotlin service clients. | Nothing in Rust. The Kotlin build applies the `kotlin("plugin.serialization")` plugin and depends on `kotlinx-serialization-json`. The Kotlin file imports `kotlinx.serialization.*` and its `builtins`, `descriptors`, `encoding` and `json` packages, and the build step writes its `package` line. `kotlin_ws_client()`, and the `kotlin_http_client()` of a service with a `body = "stream"` operation, also need `kotlinx-coroutines-core`. |
//! | `bson` | `T::from_bson_with` and `T::from_bson_piped` on a type whose `enable` list holds `"decode_with"`. | `bson`, under that name, in every crate with such a type. |
//! | `mongodb` | The `ObjectId` mapping, and `MONGO_FIELDS`, filters, updates and operations on a type whose `enable` list holds `"mongodb"`. | `mongodb`, and `bson` at the major version the driver is built for: `mongodb = "3.9"` with `bson = "2.15"`. |
//! | `chrono` | `NaiveDate`, `NaiveTime`, `NaiveDateTime` and `DateTime<Tz>` fields, and `#[model_schema_prop(as_number)]`. | `chrono` with its `serde` feature. |
//! | `indexmap` | `IndexMap` read as a map and `IndexSet` as a list. | `indexmap` with its `serde` feature. |
//! | `arky` | `TryFrom` conversions to and from `arky::parser::ast::Value` on a type whose `enable` list holds `"arky"`. | `arky`. |
//!
//! ```toml
//! # Web and mobile clients, and MongoDB.
//! tixschema = { version = "0.2", features = ["web", "mobile", "mongo"] }
//! ```
//!
//! Some crates are listed whatever the features. A type that names a `preprocess` or has a
//! `#[serde(default)]` member needs `serde_json`, and so does a member written as a bare `Value`
//! or `Map`. A `pattern` that is more than a fixed text needs `regex`. A crate that
//! invokes a transport macro lists what that macro's code names: `serde`, `serde_json` and
//! `tracing` for a dispatcher, `serde` and `serde_json` for a client, and for
//! `{service}_amqp_rpc_server!()` also `lapin`, `tokio` with its `macros` feature, and `futures`.
//!
//! # Limits
//!
//! - **Zod 4 only.** The schemas use the Zod 4 API (`z.strictObject`, `z.codec`, `.prefault`,
//!   `.check`), which Zod 3 does not have.
//! - **`u64` and `usize` stop the build under `swift` or `kotlin`**, in every `#[model_schema]`
//!   type of the crate and at any depth: `Vec<u64>`, a map's values, a tuple, a generic argument.
//!   The `mobile` set and `--all-features` turn both features on. Use `i64`, or `u32` where the
//!   range allows. A member serde neither writes nor reads, under `#[serde(skip)]` or under
//!   `skip_serializing` and `skip_deserializing` together, and every member of a variant serde
//!   skips, is not refused.
//! - **`u64` and `i64` are `bigint` in TypeScript.** Zod reads them through `z.codec`. A plain
//!   `JSON.parse` rounds a value above `2^53 - 1`, and the schema then refuses it. As a map's key,
//!   an `i64` or a `u64` is a `number` in TypeScript and `z.number().int()` in Zod.
//! - **Some types stop the build.** `i128`, `u128`, `f16` and `f128` are refused at expansion. A
//!   `serde_json::Value` or `serde_json::Map` member, at any depth, is refused at expansion when
//!   it is written by its path, such as `serde_json::Value`. Written by its bare name after
//!   `use serde_json::Value;`, it is refused by a check the macro writes beside the item and the
//!   compiler evaluates, as error `E0080`. That check names `::serde_json`, so a crate that writes
//!   a bare `Value` or `Map` in a member lists `serde_json`. A member serde neither writes nor
//!   reads is not refused for the JSON value it holds. See the README's
//!   [A JSON value is no member](https://github.com/tixena/tixschema/blob/master/README.md#a-json-value-is-no-member).
//! - **A named `Option<T>` field needs a key-dropping serde attribute** under `serde`:
//!   `skip_serializing_if`, `skip_serializing` or `skip`, or else `#[model_schema_prop(nullable)]`.
//!   `ts_optional` alone does not satisfy the rule. A field under `#[serde(flatten)]` is not held
//!   to it.
//! - **Declare a type at item scope, above the types that hold it, in the same crate.** The module
//!   written beside a type reaches the types it names through `use super::*`, so a referenced type
//!   inside a function body is not found. Several emitters also read what earlier expansions
//!   recorded, such as the untagged enum a later type flattens.
//! - **Names are matched as written.** A type named `ObjectId` is read as the BSON id under
//!   `mongodb`, and `IndexMap`, `IndexSet` and the chrono types are recognized only with their
//!   feature on. Two `#[model_schema]` types with one Rust name in two modules publish one
//!   TypeScript name, and nothing refuses them.
//! - **A `pattern` must read the same in every engine it reaches**: the Rust `regex` crate, the
//!   JavaScript regex Zod and JSON Schema use, and Dart, Swift and Kotlin. A construct they read
//!   differently, such as `.`, `\p{L}`, a negated class or an inline flag, is refused at expansion.
//! - **Lengths agree on ASCII text only.** Rust counts the UTF-8 bytes of a string, Zod its UTF-16
//!   code units, and Dart, Swift and Kotlin its code points.
//! - **The macro writes `Deserialize`** for a type that publishes `validate()` or names a
//!   `preprocess`. serde's `remote`, `from`, `try_from` and `into` are refused on such a type, and
//!   a method of your own named `validate` is a duplicate definition (`E0592`).
//! - **A generic type publishes a Zod factory**, `Page$SchemaFactory(...)` and
//!   `Page$SchemaDefault`, and no `Page$Schema`. Under `jsonschema` each type parameter needs a
//!   `default_types` entry.
//!
//! # Reference
//!
//! The macros:
//!
//! - [`model_schema`](attr.model_schema.html): the type attribute, its arguments (`name`,
//!   `default_types`, `enable`, `check`, `preprocess`, `pattern`, `minLength`, `maxLength`,
//!   `no_display`) and what each surface writes.
//! - [`model_schema_prop`](attr.model_schema_prop.html): the field attribute and its keys
//!   (`minLength`, `maxLength`, `pattern`, `minimum`, `maximum`, `literal`, `as`, `check`,
//!   `preprocess`, `ts_optional`, `nullable`, `as_number`).
//! - [`service_schema`](attr.service_schema.html): the service trait, its `transports` and
//!   `non_exhaustive` arguments, and what it generates.
//! - [`service_schema_op`](attr.service_schema_op.html): an operation's wire name, `one_way` and
//!   `http(...)`.
//! - [`decode_with_foundation`](macro.decode_with_foundation.html): the support library of the
//!   recovering decode.
//! - [`mongodb_foundation`](macro.mongodb_foundation.html): the support library of the `MongoDB`
//!   operations.
//!
//! The [README](https://github.com/tixena/tixschema/blob/master/README.md) is the full guide:
//!
//! - Models: [Optional Fields](https://github.com/tixena/tixschema/blob/master/README.md#optional-fields),
//!   [Collections and Maps](https://github.com/tixena/tixschema/blob/master/README.md#collections-and-maps),
//!   [Enums](https://github.com/tixena/tixschema/blob/master/README.md#enums),
//!   [Flatten](https://github.com/tixena/tixschema/blob/master/README.md#intersection-types-serdeflatten),
//!   [Untagged Enums](https://github.com/tixena/tixschema/blob/master/README.md#untagged-enums-serdeuntagged),
//!   [Type Parameters](https://github.com/tixena/tixschema/blob/master/README.md#type-parameters),
//!   [Branded Newtypes](https://github.com/tixena/tixschema/blob/master/README.md#branded-newtypes)
//!   and [Serde Attribute Support](https://github.com/tixena/tixschema/blob/master/README.md#serde-attribute-support).
//! - Rules: [Field Validation](https://github.com/tixena/tixschema/blob/master/README.md#field-validation-model_schema_prop),
//!   [The `validate()` Method](https://github.com/tixena/tixschema/blob/master/README.md#the-validate-method),
//!   [Checks](https://github.com/tixena/tixschema/blob/master/README.md#checks-check),
//!   [Preprocessing](https://github.com/tixena/tixschema/blob/master/README.md#preprocessing-preprocess),
//!   [What a `pattern` May Contain](https://github.com/tixena/tixschema/blob/master/README.md#what-a-pattern-may-contain)
//!   and [Compiler-Validated Examples](https://github.com/tixena/tixschema/blob/master/README.md#compiler-validated-examples).
//! - Services: [Services](https://github.com/tixena/tixschema/blob/master/README.md#services),
//!   [What a Service Generates](https://github.com/tixena/tixschema/blob/master/README.md#what-a-service-generates),
//!   [Service Faults](https://github.com/tixena/tixschema/blob/master/README.md#service-faults),
//!   [Transports](https://github.com/tixena/tixschema/blob/master/README.md#transports)
//!   and [The Form Each Client Sends](https://github.com/tixena/tixschema/blob/master/README.md#the-form-each-client-sends).
//! - Storage: [Recovering Decode](https://github.com/tixena/tixschema/blob/master/README.md#recovering-decode-decode_with),
//!   [`MongoDB` Operations and Typed Filters](https://github.com/tixena/tixschema/blob/master/README.md#mongodb-operations-and-typed-filters),
//!   [`MongoDB` `ObjectId` Support](https://github.com/tixena/tixschema/blob/master/README.md#mongodb-objectid-support),
//!   [Chrono Dates and Times](https://github.com/tixena/tixschema/blob/master/README.md#chrono-datetime-types)
//!   and [arky Frontmatter Conversions](https://github.com/tixena/tixschema/blob/master/README.md#arky-frontmatter-conversions-arky).
//! - Setup: [Feature Flags](https://github.com/tixena/tixschema/blob/master/README.md#feature-flags),
//!   [Generating TypeScript Files](https://github.com/tixena/tixschema/blob/master/README.md#generating-typescript-files),
//!   [`u64` or `usize` Under `swift` or `kotlin`](https://github.com/tixena/tixschema/blob/master/README.md#u64-or-usize-under-swift-or-kotlin)
//!   and [Error Handling and Troubleshooting](https://github.com/tixena/tixschema/blob/master/README.md#error-handling--troubleshooting).

mod bound_message;
#[cfg(test)]
mod emitted_names_tests;
mod features;
mod field_type;
mod foundation;
mod model_schema;
mod rename_rule;
mod service_schema;
mod utils;

use foundation::{exec_decode_with_foundation, exec_mongodb_foundation};
use model_schema::exec_model_schema;
use proc_macro::TokenStream;
use service_schema::exec_service_schema;

/// Publishes a struct, an enum or a type alias as TypeScript, Zod, JSON Schema, Dart, Swift and
/// Kotlin, each read from the serde attributes the type carries.
///
/// The macro keeps the item's fields and adds, on the type, one function per surface the build's
/// features turn on. In a build with `typescript`, `zod` or `jsonschema` on, it also adds a module
/// beside the type, `{type}_schema` (the Rust name in snake case). In a build with none of those
/// three, such as `default-features = false, features = ["mobile"]`, no `{type}_schema` module is
/// written, except the part `enable = ["decode_with"]` adds; the `{type}_dart`, `{type}_swift` and
/// `{type}_kotlin` modules are written under their own features. The [crate page](index.html) maps
/// each feature to its surface and lists the crates the generated code names.
///
/// # Where it goes
///
/// On each struct, enum or `type` alias that crosses the wire. Any other item is refused with
/// `model_schema: unsupported target for this attribute`.
///
/// - Write `#[model_schema]` above `#[derive(Serialize, Deserialize)]`, as every example here
///   does. The macro reads the derive list below it: where the type has `validate()` or names a
///   `preprocess`, it takes `Deserialize` off that list and writes the read itself. Attributes
///   expand from the top down, so a derive written above the macro has expanded before the macro
///   runs, from the item as written, and the macro does not see it.
/// - Declare the type at item scope, above the types that hold it, in the same crate, as the
///   crate page's [Limits](index.html#limits) say. A type declared inside a function body and named
///   by another model is `E0433`, naming the `{type}_schema` module that cannot be found.
/// - Field rules go in [`#[model_schema_prop(...)]`](attr.model_schema_prop.html) on the field.
///   This macro reads and removes them, so they need no import.
///
/// # Arguments
///
/// Every argument is optional, and `#[model_schema()]` with none is the usual form. An argument
/// outside this list is refused, and the refusal names the nine.
///
/// - `name = "Invoice"`: publishes the type under this name on every surface. The Rust name is
///   still published as an alias of it (`export type InvoiceRecord = Invoice;`, a Dart `typedef`,
///   a Swift and a Kotlin `typealias`), and it still names the `{type}_schema`, `{type}_dart`,
///   `{type}_swift` and `{type}_kotlin` modules. The value spells an identifier. Two types that
///   publish one name are refused, and the refusal names both.
/// - `default_types(T = String, ...)`: the type each type parameter is filled with where a surface
///   needs one filling, which is `json_schema()` and Zod's `T$SchemaDefault`. Under `jsonschema`
///   each type parameter needs an entry. An entry that names no parameter, and the argument on a
///   type with no type parameter, are refused in every build.
/// - `pattern = "regex"`, `minLength = N`, `maxLength = N`: rules on text, written on the type.
///   On a branded newtype they hold its value, and need `typescript`, `zod` or `jsonschema` on. On
///   an enum they hold each string field and the name of each variant that carries no data, and a
///   variant name that breaks them is refused at expansion. They are refused on any other struct
///   and on an alias.
/// - `no_display`, or `no_display = true` or `no_display = false`: drops the `Display` impl a
///   branded newtype gets, for a brand whose inner type has no `Display`.
/// - `check = ["withinBudget"]`: functions of yours that judge the whole value, each named once.
///   Rust's `validate()` calls `within_budget`, which takes `&T` and returns
///   `Result<(), Vec<String>>`, and Zod, Dart, Swift and Kotlin call `withinBudget`. A type that
///   names a check has `validate()` in every build.
/// - `preprocess = ["withoutLegacyKeys"]`: functions of yours that correct the whole payload before
///   it is read. Each Rust read calls `without_legacy_keys`, which takes and returns a
///   `serde_json::Value`, Zod wraps the schema in `z.preprocess`, and the Dart, Swift and Kotlin
///   reads call `withoutLegacyKeys`.
/// - `enable = ["decode_with", "mongodb", "arky"]`: code this type opts into. Each entry is written
///   once, and the list is never empty. An entry whose feature is off adds nothing.
///   - `"decode_with"`: the recovering decode, `T::from_value_with` and `T::from_value_piped`, and
///     under `bson` also `T::from_bson_with` and `T::from_bson_piped`. The crate writes
///     [`decode_with_foundation!()`](macro.decode_with_foundation.html) at its root. Refused on an
///     alias and on a type that borrows.
///   - `"mongodb"`: `T::MONGO_FIELDS` and the `MongoDB` operations, under the `mongodb` feature.
///     It needs `"decode_with"` beside it, and the crate writes
///     [`mongodb_foundation!()`](macro.mongodb_foundation.html) at its root.
///   - `"arky"`: under the `arky` feature, `TryFrom<&T>` for `arky::parser::ast::Value` and
///     `TryFrom<&arky::parser::ast::Value>` for `T`, both through serde. Refused on an alias.
///
/// The bare argument `decode_with` is refused with the `enable` form to write in its place.
///
/// # What it generates
///
/// For a struct or an enum `T`, each item under the features named beside it:
///
/// - `T::ts_definition() -> String` (`typescript`): the TypeScript declaration, with its doc
///   comments, and nothing of Zod.
/// - `T::zod_schema() -> String` (`zod`): `T$RawSchema` and `export const T$Schema`. A generic
///   type publishes the factory `T$SchemaFactory(...)` and `T$SchemaDefault` in place of
///   `T$Schema`.
/// - `T::json_schema() -> serde_json::Value` (`jsonschema`): one self-contained document, whose
///   objects are closed with `"additionalProperties": false` and list in `required` the keys serde
///   always writes. A generic type's document is built at its `default_types`, and
///   `{type}_schema::Schema::json_schema_with(&[...])` builds it at other fillings, one document
///   per type parameter in declaration order.
/// - `T::validate(&self) -> Result<(), Vec<String>>`, where a field or the type carries a rule, or
///   a field holds a type that does. It runs every bound, literal and check, and leads each message
///   with the value's path in the payload: `'lines.1.quantity': too large: maximum is 99, got 120`.
///   Bounds and literals are held under `serde` with `typescript`, `zod` or `jsonschema` on, and
///   checks in every build.
/// - The type's `Deserialize`, where `T` derives it and has `validate()` or names a `preprocess`:
///   serde's own read, then `validate()`. Every serde read then refuses a payload that breaks a
///   rule, with the messages joined by `; `. `T::model_schema_read_unjudged` is the read alone.
/// - `T::schema_examples()` and `T::schema_example()` (`zod`), on a type whose doc comment holds
///   code blocks fenced as `rust example`: each block is compiled as a value of `T`, and the values
///   are also written into Zod's `.meta({ examples })`.
/// - `T::enum_members() -> Vec<String>`, on an enum whose variants carry no data, in every build:
///   the names serde writes.
/// - `{type}_dart::dart_definition()`, `{type}_swift::swift_definition()` and
///   `{type}_kotlin::kotlin_definition()` (`dart`, `swift`, `kotlin`): functions in modules beside
///   the type, not methods on it. Each returns the type in that language, with its JSON codec and a
///   `validate()` that gives Rust's messages, and holds no import lines.
/// - A `Display` impl on a branded newtype that forwards to the inner value, unless `no_display`
///   is written.
///
/// A type alias has no methods: its surfaces are `{type}_schema::Schema::ts_definition()` and its
/// siblings. It publishes as its name followed by `Type` (`pub type Slug = String;` as `SlugType`),
/// unless `name` is written.
///
/// ## A struct
///
/// ```rust
/// use tixschema::model_schema;
/// use serde::{Deserialize, Serialize};
///
/// #[model_schema()]
/// #[derive(Serialize, Deserialize)]
/// #[serde(rename_all = "camelCase")]
/// pub struct User {
///     #[serde(skip_serializing_if = "Option::is_none")]
///     pub age: Option<u32>,
///     pub first_name: String,
///     pub id: String,
///     pub last_name: String,
///     pub roles: Vec<String>,
/// }
/// ```
///
/// Under the default features the declaration returns the text below. `rename_all` moves every
/// key, the `Option` field is `number | undefined` and Zod reads a `null` or a missing key there as
/// absent, and Zod holds the `u32` to its range.
///
/// `User::ts_definition()`:
///
#[doc = "```typescript
/**
 * User
 * 
 */
export type User = {
  /**
   * age
   * 
   */
  age: number | undefined;
  /**
   * firstName
   * 
   */
  firstName: string;
  /**
   * id
   * 
   */
  id: string;
  /**
   * lastName
   * 
   */
  lastName: string;
  /**
   * roles
   * 
   */
  roles: Array<string>;
};
```"]
///
/// `User::zod_schema()`:
///
#[doc = "```typescript
const User$RawSchema = z.strictObject({
  age: z.union([z.null().transform(() => undefined), z.number().int().min(0).max(4294967295), z.undefined()]).prefault(undefined),
  firstName: z.string(),
  id: z.string(),
  lastName: z.string(),
  roles: z.array(z.string()),
});

export const User$Schema: ZodType<User> = User$RawSchema.meta({
  examples: []
});
```"]
///
/// ## A plain enum
///
/// ```rust
/// use tixschema::model_schema;
/// use serde::{Deserialize, Serialize};
///
/// #[model_schema()]
/// #[derive(Serialize, Deserialize)]
/// #[serde(rename_all = "lowercase")]
/// pub enum Status {
///     Active,
///     Inactive,
///     Pending,
/// }
/// ```
///
/// An enum whose variants carry no data is a string union in TypeScript, `z.enum` in Zod and
/// `{ "type": "string", "enum": [...] }` in JSON Schema. `Status$Variant(value)` answers the Rust
/// name of the variant a value carries, or `""` for a value that carries none.
///
/// `Status::ts_definition()`:
///
#[doc = r#"```typescript
/**
 * Status
 * 
 */
export type Status =
  | "active"
  | "inactive"
  | "pending";

/** The Rust variant name `value` carries, or "" where it carries none. */
export function Status$Variant(value: unknown): string {
  switch (value) {
    case "active": return "Active";
    case "inactive": return "Inactive";
    case "pending": return "Pending";
    default: return "";
  }
}
```"#]
///
/// `Status::zod_schema()`:
///
#[doc = r#"```typescript
const Status$RawSchema = z.enum(["active", "inactive", "pending"]).meta({
  description: "Status",
  examples: [],
});

export const Status$Schema: ZodType<Status> = Status$RawSchema;
```"#]
///
/// ## A tagged enum
///
/// ```rust
/// use tixschema::model_schema;
/// use serde::{Deserialize, Serialize};
///
/// #[model_schema()]
/// #[derive(Serialize, Deserialize)]
/// #[serde(tag = "type", rename_all = "camelCase")]
/// pub enum Event {
///     UserCreated {
///         timestamp: String,
///         user_id: String,
///     },
///     UserDeleted {
///         #[serde(skip_serializing_if = "Option::is_none")]
///         reason: Option<String>,
///         user_id: String,
///     },
/// }
/// ```
///
/// `#[serde(tag = "type")]` gives a discriminated union in TypeScript, `z.discriminatedUnion` in
/// Zod and `oneOf` in JSON Schema. `rename_all` on an enum renames its variants, as serde does, so
/// the fields inside them keep their Rust names, `user_id` here; `rename_all_fields = "camelCase"`
/// renames those. `content` beside `tag` gives the adjacently tagged form, and an enum with neither
/// is externally tagged.
///
/// `Event::ts_definition()`:
///
#[doc = r#"```typescript
/**
 * Event
 * 
 */
export type Event = {
  /**
   * userCreated
   * 
   */
  type: "userCreated";
  /**
   * timestamp
   * 
   */
  timestamp: string;
  /**
   * user_id
   * 
   */
  user_id: string;
} | {
  /**
   * userDeleted
   * 
   */
  type: "userDeleted";
  /**
   * reason
   * 
   */
  reason: string | undefined;
  /**
   * user_id
   * 
   */
  user_id: string;
};

/** The Rust variant name `value` carries, or "" where it carries none. */
export function Event$Variant(value: unknown): string {
  if (typeof value !== "object" || value === null) return "";
  switch ((value as { type?: unknown }).type) {
    case "userCreated": return "UserCreated";
    case "userDeleted": return "UserDeleted";
    default: return "";
  }
}
```"#]
///
/// `Event::zod_schema()`:
///
#[doc = r#"```typescript
const Event$RawSchema = z.discriminatedUnion("type", [z.strictObject({
  type: z.literal("userCreated"),
  timestamp: z.string(),
  user_id: z.string(),
}), z.strictObject({
  type: z.literal("userDeleted"),
  reason: z.union([z.null().transform(() => undefined), z.string(), z.undefined()]).prefault(undefined),
  user_id: z.string(),
})]);

export const Event$Schema: ZodType<Event> = Event$RawSchema.meta({
  examples: []
});
```"#]
///
/// ## Other shapes
///
/// - **An untagged enum** (`#[serde(untagged)]`): `A | B` in TypeScript, `z.union` in Zod, `anyOf`
///   in JSON Schema. A variant with no data and a tuple variant of several slots are refused.
/// - **A flattened field** (`#[serde(flatten)]`): `{ ... } & B` in TypeScript,
///   `.and(z.lazy(() => B$Schema))` in Zod, and one closed object holding both in JSON Schema.
///   Only a value serde writes as an object can be flattened, and an untagged enum is declared
///   above the type that flattens it.
/// - **A unit struct**: `Record<string, never>`, written as `{}`.
/// - **A generic type**: TypeScript keeps the parameters (`export type Page<T>`). Zod publishes a
///   memoized factory, `Page$SchemaFactory(itemSchema)`, and `Page$SchemaDefault` at the declared
///   filling. JSON Schema is built at `default_types`.
/// - **A branded newtype**, a `#[serde(transparent)]` struct of one field: TypeScript
///   `string & $brand<"UserId">` and Zod `.brand<"UserId">()`, so two ids over one inner type
///   cannot be swapped. Its inner type implements `Display`, or the brand writes `no_display`; a
///   brand over a parameter asks it of the declared default alone, and of a path default not at all.
///
/// ## Numbers, options, dates and maps
///
/// - `u8`, `u16`, `u32`, `i8`, `i16`, `i32`, `usize`, `isize`, `f32` and `f64` are `number` in
///   TypeScript, and Zod holds an integer to its type's range.
/// - `u64` and `i64` are `bigint` in TypeScript, read in Zod through `z.codec`, and
///   `{ "type": "integer" }` in JSON Schema. As a map's key they are a TypeScript `number` and a
///   Zod `z.number().int()`. `u64` and `usize` are refused under `swift` or `kotlin`.
/// - `i128`, `u128`, `f16` and `f128` are refused at expansion. A `serde_json::Value` or
///   `serde_json::Map` member is refused at expansion when written by its path, and as error
///   `E0080`, by a check the compiler evaluates, when written by its bare name after a `use`.
/// - `Option<T>` is `field: T | undefined` in TypeScript, a key Zod reads as absent when it is
///   `null` or missing, and in JSON Schema a key left out of `required`. Under `serde` a named
///   `Option` field carries `skip_serializing_if`, `skip_serializing` or `skip`, or
///   `#[model_schema_prop(nullable)]`.
/// - Under `chrono`, `NaiveDate`, `NaiveTime` and `NaiveDateTime` are strings, and `DateTime<Tz>`
///   is a TypeScript `Date` that Zod coerces from a `Date`, a string or a number.
/// - Under `indexmap`, `IndexMap` is a map and `IndexSet` a list.
///
/// ## Arguments in use
///
/// A brand that holds its own pattern, a type published under another name, and a generic type
/// with its default filling. A bundle that holds the brand imports `$brand` from `zod`, as the
/// header of [Getting started](index.html#getting-started) does:
///
#[cfg_attr(
    all(
        feature = "serde",
        feature = "typescript",
        feature = "zod",
        feature = "jsonschema"
    ),
    doc = r##"
```rust
use serde::{Deserialize, Serialize};
use serde_json::json;
use tixschema::model_schema;

/// `INV-` and six digits.
#[model_schema(pattern = "^INV-[0-9]{6}$")]
#[derive(Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct InvoiceNumber(pub String);

#[model_schema(name = "Invoice")]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvoiceRecord {
    pub number: InvoiceNumber,
    pub total_cents: u32,
}

#[model_schema(default_types(T = String))]
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total_count: u32,
}

fn main() -> Result<(), serde_json::Error> {
    // The brand is a type of its own, and every read holds it to its pattern.
    assert!(
        InvoiceNumber::ts_definition()
            .contains(r#"export type InvoiceNumber = string & $brand<"InvoiceNumber">"#)
    );
    assert!(InvoiceNumber::zod_schema().contains(r#".brand<"InvoiceNumber">()"#));
    let read: InvoiceRecord =
        serde_json::from_str(r#"{"number":"INV-000042","totalCents":1250}"#)?;
    assert_eq!(read.number.0, "INV-000042");
    let refused = serde_json::from_str::<InvoiceRecord>(r#"{"number":"INV-42","totalCents":1}"#)
        .unwrap_err();
    assert!(refused.to_string().contains("does not match pattern '^INV-[0-9]{6}$'"));

    // `name` publishes `Invoice`, and keeps the Rust name as an alias of it.
    let typescript = InvoiceRecord::ts_definition();
    assert!(typescript.contains("export type Invoice = {"));
    assert!(typescript.contains("export type InvoiceRecord = Invoice;"));

    // `default_types` fills `T` where one filling is needed; any other is built on request.
    assert!(Page::<String>::ts_definition().contains("export type Page<T> = {"));
    let zod = Page::<String>::zod_schema();
    assert!(zod.contains("export function Page$SchemaFactory<T extends ZodType>("));
    assert!(zod.contains("export const Page$SchemaDefault"));
    let declared = Page::<String>::json_schema();
    assert_eq!(declared["properties"]["items"]["items"]["type"], "string");
    let counted = page_schema::Schema::json_schema_with(&[json!({ "type": "integer" })]);
    assert_eq!(counted["properties"]["items"]["items"]["type"], "integer");
    Ok(())
}
```
"##
)]
///
/// ## `ObjectId` under the `mongodb` feature
///
/// A type named `ObjectId`, by that name alone, is read as `MongoDB`'s object id. Zod and JSON
/// Schema describe the form serde writes for `bson::oid::ObjectId`,
/// `{ "$oid": "<24 hexadecimal digits>" }`, in lower case and closed to any other key. TypeScript,
/// Dart, Swift and Kotlin write a bare `ObjectId` reference, and your project declares that type
/// in each of those languages, such as `type ObjectId = { $oid: string };` in the TypeScript
/// bundle, as the README's
/// [`MongoDB` `ObjectId` Support](https://github.com/tixena/tixschema/blob/master/README.md#mongodb-objectid-support)
/// shows.
///
#[cfg_attr(
    feature = "mongodb",
    doc = r#"
```rust
# extern crate bson2 as bson;
use bson::oid::ObjectId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tixschema::model_schema;

#[model_schema()]
#[derive(Serialize, Deserialize)]
pub struct Document {
    pub author_id: ObjectId,
    pub id: ObjectId,
    pub metadata: HashMap<String, ObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<ObjectId>,
    pub tags: Vec<ObjectId>,
    pub title: String,
}

fn main() {}
```

`Document::ts_definition()`:

```typescript
/**
 * Document
 * 
 */
export type Document = {
  /**
   * author_id
   * 
   */
  author_id: ObjectId;
  /**
   * id
   * 
   */
  id: ObjectId;
  /**
   * metadata
   * 
   */
  metadata: Partial<Record<string, ObjectId>>;
  /**
   * parent_id
   * 
   */
  parent_id: ObjectId | undefined;
  /**
   * tags
   * 
   */
  tags: Array<ObjectId>;
  /**
   * title
   * 
   */
  title: string;
};
```

`Document::zod_schema()`:

```typescript
const Document$RawSchema = z.strictObject({
  author_id: z.strictObject({ $oid: z.string().regex(/^[a-f0-9]{24}$/, { message: "Invalid ObjectId" }) }),
  id: z.strictObject({ $oid: z.string().regex(/^[a-f0-9]{24}$/, { message: "Invalid ObjectId" }) }),
  metadata: z.record(z.string(), z.strictObject({ $oid: z.string().regex(/^[a-f0-9]{24}$/, { message: "Invalid ObjectId" }) })),
  parent_id: z.union([z.null().transform(() => undefined), z.strictObject({ $oid: z.string().regex(/^[a-f0-9]{24}$/, { message: "Invalid ObjectId" }) }), z.undefined()]).prefault(undefined),
  tags: z.array(z.strictObject({ $oid: z.string().regex(/^[a-f0-9]{24}$/, { message: "Invalid ObjectId" }) })),
  title: z.string(),
});

export const Document$Schema: ZodType<Document> = Document$RawSchema.meta({
  examples: []
});
```
"#
)]
///
/// # Errors met most
///
/// - **A named `Option` field with no key-dropping attribute**, under `serde`: refused, naming the
///   field. Add `#[serde(skip_serializing_if = "Option::is_none")]`, with `default` where the type
///   derives `Deserialize`, or `skip_serializing` or `skip`; or declare
///   `#[model_schema_prop(nullable)]` where the wire carries `null`.
/// - **`E0433`, a `{type}_schema` module that cannot be found**: a type another model names is
///   declared inside a function body. Declare it in a module.
/// - **`u64` or `usize` under `swift` or `kotlin`**: refused at the field, at any depth. Use `i64`,
///   or `u32` where the range allows.
/// - **A function of yours named `validate` or `model_schema_read_unjudged`** on a type that
///   publishes one: a duplicate definition (`E0592`). serde's `remote`, `from`, `try_from` and
///   `into` are refused on a type whose `Deserialize` the macro writes.
/// - **`E0277` at a brand's inner field, naming `Display`**: the inner type has no `Display`. Write
///   `no_display`; a brand over text that carries `pattern`, `minLength` or `maxLength` still needs
///   a `Display` inner.
/// - **A type parameter with no `default_types` entry**, under `jsonschema`: refused at the
///   parameter.
/// - **`pattern`, `minLength` or `maxLength` on a struct that is no brand**: refused; write them on
///   the field with `#[model_schema_prop(...)]`.
///
/// The README holds the full rules:
/// [Published Names](https://github.com/tixena/tixschema/blob/master/README.md#published-names-name),
/// [Optional Fields](https://github.com/tixena/tixschema/blob/master/README.md#optional-fields),
/// [Collections and Maps](https://github.com/tixena/tixschema/blob/master/README.md#collections-and-maps),
/// [Enums](https://github.com/tixena/tixschema/blob/master/README.md#enums),
/// [Flatten](https://github.com/tixena/tixschema/blob/master/README.md#intersection-types-serdeflatten),
/// [Untagged Enums](https://github.com/tixena/tixschema/blob/master/README.md#untagged-enums-serdeuntagged),
/// [Type Aliases](https://github.com/tixena/tixschema/blob/master/README.md#type-aliases),
/// [Type Parameters](https://github.com/tixena/tixschema/blob/master/README.md#type-parameters),
/// [Branded Newtypes](https://github.com/tixena/tixschema/blob/master/README.md#branded-newtypes),
/// [Compiler-Validated Examples](https://github.com/tixena/tixschema/blob/master/README.md#compiler-validated-examples),
/// [Chrono Dates and Times](https://github.com/tixena/tixschema/blob/master/README.md#chrono-datetime-types)
/// and [Serde Attribute Support](https://github.com/tixena/tixschema/blob/master/README.md#serde-attribute-support).
#[proc_macro_attribute]
pub fn model_schema(args: TokenStream, input: TokenStream) -> TokenStream {
    exec_model_schema(args.into(), input.into()).into()
}

/// Writes, once per crate, the support library of the recovering decode, which a type opts into
/// with `enable = ["decode_with"]`.
///
/// Write it at the crate root, on its own line and outside any function:
/// `tixschema::decode_with_foundation!();`. A crate needs it when any of its types lists
/// `"decode_with"` in [`#[model_schema(enable = [...])]`](attr.model_schema.html). Each such type
/// re-exports the library from its `{type}_schema` module, so within one crate
/// `invoice_schema::Issue` and `customer_schema::Issue` are one type, and one resolver serves every
/// type. It takes no arguments, and writes nothing in a build without the `serde` feature, so the
/// line stays whatever the crate builds with.
///
/// # What a type that lists `"decode_with"` gains
///
/// - `T::from_value_with(value: serde_json::Value, decide)`: reads `value` with the type's own read
///   and walks it. A clean value is returned at once. Otherwise `decide(&mut value, &issues)` is
///   called once and answers `Verdict::Reject`, which fails the read, or `Verdict::Fixed`, which
///   reads the repaired value once more and fails on anything found then.
/// - `T::from_value_piped(value, resolvers)`: the same read, with the issues handed to `resolvers`
///   in order. A resolver answers `Resolution::Settled`, `Rejected` or `NotTouched` for each issue,
///   and a later resolver sees only the issues every earlier one left `NotTouched`. An issue none
///   settles fails the read.
/// - `T::from_bson_with(document: bson::Document, decide)` and `T::from_bson_piped`, under the
///   `bson` feature, which `mongodb` turns on: the same two reads over a BSON document.
///
/// Each read fails with `Unrecovered { issues }`, which implements `Display`, one line per issue,
/// and `std::error::Error`. A `u64` or `i64` stored as the text of its digits is read as its number
/// before any issue is handed on.
///
/// # What the library holds
///
/// Each name is reached as `{type}_schema::Name`:
///
/// - `Issue<V>`, `V` being the type of the raw value, with one variant per kind of issue:
///   - `Invalid { path, expected, found, reason }`: a value the field cannot be read from, or one
///     that breaks a bound or a check.
///   - `Missing { path, expected }`: a key the type requires.
///   - `Unknown { path, found }`: a key the type does not declare.
///   - `Mistyped { path, expected, found }`: a value held in another form than the field writes.
///   - `NoVariant { path, found, variants }`: an untagged enum no variant reads; `variants` holds
///     each variant's name and its own issues.
///   - `Undescribed { reason }`: a refusal the walk found no reason for. It carries no `path`.
/// - `Expected`: what the field reads, such as `Expected::U32`, `Expected::String` or
///   `Expected::Model("Customer")`.
/// - `Path` and `Segment`: where an issue is, displayed as `lines[1].quantity`.
/// - `Stored`: the trait a callback repairs the raw value through, implemented for
///   `serde_json::Value` and, under `bson`, for `bson::Document`. `set_at` and `remove_at` write
///   at a `Path` and answer whether they changed anything; `at` reads there.
/// - `Verdict`, `Resolution`, `Resolver<'r, D, V>` (a reference to a `Sync` function over the raw
///   value `D`), `Unrecovered<V>`, and `unsettled`, which runs resolvers over a list of issues.
///
/// Every type of the library is `#[non_exhaustive]`: a `match` in another crate has a wildcard arm,
/// and builds a `Path` from `Path::default()`.
///
/// # What your crate lists
///
/// `serde` with its `derive` feature, and `serde_json`. Wherever tixschema's `bson` feature is on,
/// directly, through `mongodb`, or through Cargo's feature unification in a workspace, every crate
/// with such a type also lists `bson` under that name, at the major version its `MongoDB` driver
/// uses. The example below brings `bson` in through a hidden `extern crate` line, because
/// tixschema's own dependency on it is renamed.
///
/// # Example
///
/// Two resolvers repair what older writers left in stored invoices; the
/// [crate page](index.html#reading-records-an-older-writer-stored) shows the same read with one
/// callback:
///
#[cfg_attr(feature = "serde", doc = include_str!("decode_with_example.md"))]
///
/// # Errors met most
///
/// - **A type that lists `"decode_with"` in a crate with no `decode_with_foundation!();`**:
///   `E0432` for the library it re-exports, and an `E0277` at each such type whose note gives the
///   line to add. Writing it twice is `E0428`.
/// - **The line inside a function**, including a doctest with no `fn main` of its own, which
///   rustdoc wraps in one: the library is not at the crate root.
/// - **A model type declared above a flagged type and held by it, without the entry**: `E0599`,
///   naming `decode_with_value_issues`. A model declared below, or in another crate, is read whole
///   and needs no entry.
/// - **A resolver that counts in a `Cell` or a `RefCell`**: `E0277`, since a resolver is `Sync`.
///   Count in an atomic or a `Mutex`.
///
/// The README holds the full rules:
/// [Recovering Decode](https://github.com/tixena/tixschema/blob/master/README.md#recovering-decode-decode_with),
/// [The support libraries](https://github.com/tixena/tixschema/blob/master/README.md#the-support-libraries),
/// [The issues](https://github.com/tixena/tixschema/blob/master/README.md#the-issues)
/// and [Resolvers](https://github.com/tixena/tixschema/blob/master/README.md#resolvers).
#[proc_macro]
pub fn decode_with_foundation(input: TokenStream) -> TokenStream {
    exec_decode_with_foundation(&input.into()).into()
}

/// Writes, once per crate, the support library of the typed `MongoDB` filters, updates and
/// operations, which a type opts into with `enable = ["decode_with", "mongodb"]`.
///
/// Write it at the crate root, beside `tixschema::decode_with_foundation!();`:
/// `tixschema::mongodb_foundation!();`. A crate needs it when any of its types lists `"mongodb"`
/// in [`#[model_schema(enable = [...])]`](attr.model_schema.html), and each such type re-exports it
/// from its `{type}_schema` module. It takes no arguments, and writes nothing in a build without
/// the `mongodb` feature, so the line stays whatever the crate builds with. A crate with no stored
/// type writes `decode_with_foundation!()` alone, since this library names the `mongodb` driver.
///
/// # What a type that lists `"mongodb"` gains
///
/// Under the `mongodb` feature:
///
/// - `T::MONGO_FIELDS`, with one `const fn` per key serde writes, named after the Rust field. A
///   map field has none: its keys are data. Each
///   answers a typed path, keyed by the name serde writes. The field's type decides the kind of
///   path, and the kind decides its operators:
///   - `Field`, a value every row holds: `eq`, `ne`, `gt`, `gte`, `lt`, `lte`, `is_in`, `not_in`,
///     `set` and `set_on_insert`, and `regex` where the value is a `String`.
///   - `OptionalField`, an `Option`: every operator of `Field`, through `Deref`, and `exists` and
///     `unset`.
///   - `ListField`, a list of plain values: `contains`, `contains_any`, `contains_none`, `size`,
///     `element`, `elem_match`, `push`, `pull`, `set` and `set_on_insert`.
///   - `Model`, a nested model: `eq`, `ne`, `is_in`, `not_in`, `set` and `set_on_insert`.
///   - `OptionalModel`, an `Option` of a nested model: every operator of `Model`, through `Deref`,
///     and `exists` and `unset`.
///   - `ModelList`, a list of nested models: `elem_match`, `size`, `push`, `pull` and `set`.
///
///   A path that reaches a nested model also has `nest`, which takes a filter or an update built
///   from the held type's own paths. An enum under a serde tag also gets one function per arm,
///   such as `type__by_courier()`.
/// - `Filter<T>` and `Update<T>`. `and`, `or` and `negated` join filters, `and` merges updates,
///   and `into_document()` answers the `bson::Document` any driver call takes. An operator that
///   writes a value answers `Result<_, WriteError>`, the value written by the field's own
///   `Serialize`.
/// - Ten operations over a `mongodb::Collection<bson::Document>`: `T::find_one`,
///   `T::find_one_with`, `T::find`, `T::find_with` and `T::count`, which answer a `Read` that takes
///   `sort`, `limit`, `skip`, `hint` and `collation` and is awaited; and `value.insert_one`,
///   `T::update_one`, `T::update_many`, `T::delete_one` and `T::delete_many`. Each row is read
///   through `from_bson_piped`, with the resolvers a `_with` read is handed. None of them upserts.
/// - `OperationError`: `Database` for the driver's error, `Unreadable { row, issues }` for a row
///   that does not read as `T`, naming its `_id`, and `Unwritable` for a value that cannot be
///   written.
///
/// A misspelled field is `E0599`, a value of the wrong type `E0308`, and a filter over another
/// type's rows, or an update where a filter belongs, `E0277`.
///
/// # What your crate lists
///
/// The `mongodb` driver and `bson` at the major version that driver is built for, such as
/// `mongodb = "3.9"` with `bson = "2.15"`, beside `serde` and `serde_json`. The README's
/// [The driver your crate lists](https://github.com/tixena/tixschema/blob/master/README.md#the-driver-your-crate-lists)
/// has the lines for `bson` 3.
///
/// # Example
///
/// An update built from typed paths; the
/// [crate page](index.html#typed-mongodb-filters-and-operations) shows a filter and an operation:
///
#[cfg_attr(
    feature = "mongodb",
    doc = r##"
```rust
# extern crate bson2 as bson;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use order_schema::{Update, WriteError};

tixschema::decode_with_foundation!();
tixschema::mongodb_foundation!();

#[model_schema(enable = ["decode_with", "mongodb"])]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    #[serde(rename = "_id")]
    pub id: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shipped_at: Option<String>,
}

/// Marks an order shipped on `day`.
fn shipped(day: &str) -> Result<Update<Order>, WriteError> {
    let paths = Order::MONGO_FIELDS;
    Ok(paths
        .status()
        .set("shipped".to_owned())?
        .and(paths.shipped_at().set(day.to_owned())?))
}

fn main() -> Result<(), WriteError> {
    assert_eq!(
        shipped("2026-10-01")?.into_document().to_string(),
        r#"{ "$set": { "status": "shipped", "shippedAt": "2026-10-01" } }"#
    );
    assert_eq!(
        Order::MONGO_FIELDS.shipped_at().unset().into_document().to_string(),
        r#"{ "$unset": { "shippedAt": "" } }"#
    );
    Ok(())
}
```
"##
)]
///
/// # Errors met most
///
/// - **`?` from a `WriteError` or a driver error into `OperationError`**: `E0277`, since neither
///   converts on its own. Write `.map_err(OperationError::Unwritable)` and
///   `.map_err(OperationError::Database)`.
/// - **Every row refused as `Unreadable`**: the type does not declare `_id`, so `_id` is `Unknown`.
///   Declare it with `#[serde(rename = "_id")]`, or read with `find_one_with` or `find_with` and a
///   resolver that removes it.
/// - **A `Read` handed to something that takes a `Future`**, such as `tokio::spawn`: `Read`
///   implements `IntoFuture`, so call `.into_future()` first.
/// - **A map keyed by an integer, a float or a `bool`** on a type that lists `"mongodb"`: refused,
///   since the `bson` library writes no such key.
/// - **A function of yours named `find_one`, `count` or another operation**: `E0592` once the
///   feature is on.
/// - **No `mongodb_foundation!();` at the crate root**: an `E0277` at each such type whose note
///   gives the line to add.
///
/// The README holds the full rules:
/// [`MongoDB` Operations and Typed Filters](https://github.com/tixena/tixschema/blob/master/README.md#mongodb-operations-and-typed-filters),
/// [The paths of a type](https://github.com/tixena/tixschema/blob/master/README.md#the-paths-of-a-type),
/// [Operators](https://github.com/tixena/tixschema/blob/master/README.md#operators),
/// [The operations](https://github.com/tixena/tixschema/blob/master/README.md#the-operations)
/// and [`OperationError`](https://github.com/tixena/tixschema/blob/master/README.md#operationerror).
#[proc_macro]
pub fn mongodb_foundation(input: TokenStream) -> TokenStream {
    exec_mongodb_foundation(&input.into()).into()
}

/// Puts a rule or a rendering on one field of a `#[model_schema]` type: a length, a range, a
/// pattern, a literal, a check, a correction, or how TypeScript writes the key.
///
/// [`#[model_schema]`](attr.model_schema.html) reads this attribute and removes it from the field,
/// so it needs no import, and on its own it does nothing. It goes on a named field, on a slot of a
/// tuple struct or of a tuple variant, or on a member of an enum variant. It is refused on the slot
/// of a branded newtype, whose rules are written on the type with
/// `#[model_schema(pattern = ..., minLength = ..., maxLength = ...)]`.
///
/// # Keys
///
/// A key that names a bound is camelCase (`minLength`), and a flag is snake case (`ts_optional`).
/// An unknown key, `min_length` among them, is refused with the list of the twelve.
///
/// - `minLength = N`, `maxLength = N`: on a string or a path, the shortest and the longest text, as
///   Zod `.min(N)` and `.max(N)` and JSON Schema `minLength` and `maxLength`. On a list (`Vec`,
///   `VecDeque`, a set, `BinaryHeap`, a slice or `[T; N]`), the fewest and the most items, as
///   `minItems` and `maxItems`; no key written on a list reaches its items.
/// - `pattern = "regex"`: on a string or a path, a regex the text matches, as Zod
///   `.check(z.regex(...))` and JSON Schema `pattern`. See
///   [What a pattern may contain](#what-a-pattern-may-contain).
/// - `minimum = N`, `maximum = N`: on a number, an integer or a float literal, as Zod `.min(N)` and
///   `.max(N)` and JSON Schema `minimum` and `maximum`.
/// - `literal = value`: a string, `true` or `false`, an integer or a float, of the field's own
///   kind. TypeScript writes the literal type, Zod `z.literal(...)` and JSON Schema a `const`, and
///   `validate()` and every read refuse any other value. No other rule may sit beside it.
/// - `check = ["name", ...]`: functions of yours that judge the field's value. See
///   [Checks](#checks).
/// - `preprocess = ["name", ...]`: functions of yours that correct the field's payload before the
///   field is read. See [Preprocessing](#preprocessing).
/// - `as = Type`: the type the field renders, which is the field's own type or the value under its
///   wrappers (`as = String` on a `Vec<String>`). Any other type, and `as` beside `preprocess`, is
///   refused.
/// - `ts_optional`: on an `Option<T>` field, the TypeScript key `field?: T` in place of
///   `field: T | undefined`. Zod and JSON Schema are unchanged, and under `serde` the field still
///   carries `skip_serializing_if` or `skip_serializing`. On a field serde takes off the wire in
///   both directions, with `skip` or with `skip_serializing` and `skip_deserializing` together,
///   `ts_optional` is refused in every build: no key is written for it to make optional.
/// - `nullable`: on an `Option<T>` field, `T | null` with the key always written, on TypeScript,
///   Zod and JSON Schema. Under `serde` the field carries no key-dropping attribute, and
///   `ts_optional` beside it is refused.
/// - `as_number`: on a `DateTime<Tz>` field under the `chrono` feature, epoch milliseconds as a
///   `number` in place of a `Date`. Under `serde` the macro hangs `chrono::serde::ts_milliseconds`
///   on a `DateTime<Utc>` field and `chrono::serde::ts_milliseconds_option` on an `Option` of one,
///   unless the field carries its own `#[serde(with = ...)]`.
///
/// A key written where it means nothing, such as `minimum` on a string, `pattern` on a list or
/// `ts_optional` on a field that is no `Option`, is refused at the key.
///
/// # Where a rule is held
///
/// A bound or a literal is held by Zod and JSON Schema, by Rust's `validate()` and the serde read
/// that runs it (under `serde` with `typescript`, `zod` or `jsonschema` on), and by the
/// `validate()` and the read of the Dart, Swift and Kotlin types, which write Rust's sentences.
/// Lengths agree on ASCII text only: Rust counts UTF-8 bytes, Zod UTF-16 code units, and Dart,
/// Swift and Kotlin code points.
///
/// Each Rust message opens with where the value sits in the payload, in single quotes: the keys
/// serde writes on the way to it, joined by dots, with the index of a list item and the key of a
/// map entry as steps. A broken label in the second item of `parcel_list`, under
/// `rename_all = "camelCase"`, reads
/// `'parcelList.1.itemLabel': too short: minimum length is 1, got 0`, and a broken member of the
/// variant `Loose` of an enum with no tag attribute reads `'Loose.freightLabel': ...`. The other
/// sentences are `too long: maximum length is 30, got 31`, `too small: minimum is 1, got 0`,
/// `does not match pattern '^[a-z]+$'` and `not the literal "EUR", got USD`.
///
/// `validate()` judges a value built in code:
///
#[cfg_attr(
    all(
        feature = "serde",
        any(feature = "typescript", feature = "zod", feature = "jsonschema")
    ),
    doc = r#"
```rust
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Deserialize, Serialize)]
pub struct Registration {
    #[model_schema_prop(minimum = 0, maximum = 120)]
    pub age: u32,
    #[model_schema_prop(minLength = 3, maxLength = 30)]
    pub username: String,
}

let built = Registration { age: 150, username: "ab".to_owned() };
let messages = built.validate().unwrap_err();
for message in &messages {
    println!("{message:?}");
}
// "'age': too large: maximum is 120, got 150"
// "'username': too short: minimum length is 3, got 2"
assert_eq!(messages.len(), 2);
```
"#
)]
///
/// The rules on every surface, and the read that refuses a payload:
///
#[cfg_attr(
    all(
        feature = "serde",
        feature = "typescript",
        feature = "zod",
        feature = "jsonschema"
    ),
    doc = r##"
```rust
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderLine {
    #[model_schema_prop(minLength = 4, maxLength = 20, pattern = "^[A-Z]{2}-[0-9]+$")]
    pub sku: String,
    #[model_schema_prop(minimum = 1, maximum = 99)]
    pub quantity: u32,
    #[model_schema_prop(maxLength = 3)]
    pub gift_notes: Vec<String>,
    #[model_schema_prop(literal = "EUR")]
    pub currency: String,
}

assert!(OrderLine::zod_schema().contains(
    r#".check(z.regex(/^[A-Z]{2}-[0-9]+$/, { error: "does not match pattern '^[A-Z]{2}-[0-9]+$'" }))"#
));
assert!(OrderLine::ts_definition().contains(r#"currency: "EUR";"#));

let document = OrderLine::json_schema();
assert_eq!(document["properties"]["sku"]["maxLength"].as_u64(), Some(20));
assert_eq!(document["properties"]["giftNotes"]["maxItems"].as_u64(), Some(3));
assert_eq!(document["properties"]["currency"]["const"], "EUR");

// serde's own read refuses the payload, with every message.
let refused = serde_json::from_str::<OrderLine>(
    r#"{"sku":"AB-1","quantity":120,"giftNotes":[],"currency":"USD"}"#,
)
.unwrap_err()
.to_string();
assert!(refused.contains("'quantity': too large: maximum is 99, got 120"));
assert!(refused.contains(r#"'currency': not the literal "EUR", got USD"#));
```
"##
)]
///
/// # What a pattern may contain
///
/// One `pattern` runs in the Rust `regex` crate, in the JavaScript regex Zod and JSON Schema use,
/// and in Dart, Swift and Kotlin, so it is held to what every engine reads the same way:
///
/// - Rewritten: `\d` as `[0-9]`, `\w` as `[0-9A-Za-z_]`, `\s` as `[\t\n\v\f\r ]`, and
///   `(?P<name>...)` as `(?<name>...)`.
/// - Refused at expansion, naming the construct: `.`, `\D`, `\W`, `\S`, a negated class `[^...]`,
///   a Unicode class such as `\p{L}`, a POSIX class such as `[[:alpha:]]`, the class operators
///   `&&`, `--` and `~~`, a class inside a class, `\A`, `\z`, `\b{start}`, `\<`, `\>`, a braced
///   escape such as `\x{41}`, `\a`, an inline flag such as `(?i)`, and an unescaped `]` that opens
///   a class.
/// - Refused because it checks nothing: a pattern every string matches, such as `^`, `$`, `|` or
///   `a*`.
/// - Refused because the Rust check would draw a lint in your crate: a pattern that is a lone
///   `\b` or `\B`. The `regex::Regex` built from it trips `clippy::trivial_regex`, which a crate
///   that denies `clippy::nursery` gets. Write the boundary beside what sits next to it, such as
///   `\b\w+`.
///
/// A pattern that is more than a fixed text is checked in Rust through the `regex` crate, which
/// your crate then lists. A fixed text, anchored or not, such as `^INV-`, `-x$` or `^abc$`, is
/// checked with `str` comparisons and names no `regex`.
///
/// # Checks
///
/// `check = ["withoutDuplicates"]` names functions of yours that judge the field's value, each
/// named once and run after the field's bounds, in the order written. The same key on the type,
/// `#[model_schema(check = ["withinParcel"])]`, judges the whole value. Rust calls the snake case
/// of each name, in scope where the type is declared, and every other language calls the name as
/// written:
///
/// ```text
/// // Rust
/// fn without_duplicates(value: &[String]) -> Result<(), Vec<String>>
/// fn within_parcel(value: &Parcel) -> Result<(), Vec<String>>
/// // TypeScript
/// function withoutDuplicates(value: Array<string>, ctx: ParsePayload<Array<string>>): void
/// function withinParcel(value: Parcel, ctx: ParsePayload<Parcel>): void
/// // Dart
/// List<String> withoutDuplicates(List<String> value)
/// List<String> withinParcel(Parcel value)
/// // Swift
/// func withoutDuplicates(_ value: [String]) -> [String]
/// func withinParcel(_ value: Parcel) -> [String]
/// // Kotlin
/// fun withoutDuplicates(value: List<String>): List<String>
/// fun withinParcel(value: Parcel): List<String>
/// ```
///
/// A field's check receives `&T` for a field of type `T`. For `Option<T>` it receives `&T` and runs
/// only when the value is `Some`. For `Vec<T>` it receives the whole list, a `&Vec<T>`, which a
/// function that takes `&[T]` accepts. Its messages carry no path: `validate()` puts the field's
/// path in front of each. A type's check leads each message with the path of what it refuses, in
/// single quotes.
/// Every check runs and none stops another, and a type that names a check has `validate()` in every
/// build. A TypeScript check pushes `{ code: "custom", message, path, input }` to `ctx.issues`.
/// JSON Schema calls no function. A type with `enable = ["decode_with"]` lists each message as an
/// `Issue::Invalid` at the path the message names.
///
#[cfg_attr(
    all(
        feature = "serde",
        any(feature = "typescript", feature = "zod", feature = "jsonschema")
    ),
    doc = include_str!("check_example.md")
)]
///
/// The key is refused where it is not an array of string literals, where a name is no identifier or
/// is written twice, on a type alias, and on the slot of a branded newtype. A name with no function
/// in scope is `E0425` at the string literal.
///
/// # Preprocessing
///
/// `preprocess = ["trimmed"]` names functions of yours that correct the field's payload before the
/// field is read from it, each over the answer of the one before. The same key on the type,
/// `#[model_schema(preprocess = ["withoutFragile"])]`, corrects the whole payload first. Rust calls
/// the snake case of each name, and every other language calls the name as written:
///
/// ```text
/// // Rust
/// fn trimmed(payload: serde_json::Value) -> serde_json::Value
/// fn without_fragile(payload: serde_json::Value) -> serde_json::Value
/// // TypeScript
/// function trimmed(payload: unknown): unknown
/// function withoutFragile(payload: unknown): unknown
/// // Dart
/// dynamic trimmed(dynamic payload)
/// dynamic withoutFragile(dynamic payload)
/// // Swift
/// func trimmed(_ payload: Any) -> Any
/// func withoutFragile(_ payload: Any) -> Any
/// // Kotlin
/// fun trimmed(payload: JsonElement): JsonElement
/// fun withoutFragile(payload: JsonElement): JsonElement
/// ```
///
/// One read runs the type's functions, then each field's as the field is read; serde then builds
/// the value, and `validate()` judges it. Every serde read calls them, from JSON text, a
/// `serde_json::Value`, a BSON document or any other format that describes itself, and so do a
/// holder's read, the recovering decode, a `MongoDB` operation and a service dispatcher. Zod wraps
/// the schema in `z.preprocess(name, ...)`, the first name outermost. JSON Schema calls no
/// function, and a value built in code is never corrected. A key the payload does not hold is read
/// as serde reads an absent key, without the field's Rust function, where Zod calls the function
/// with `undefined`.
///
/// A function takes a `serde_json::Value` whatever the payload was read from, so a crate that names
/// a `preprocess` lists `serde_json`. Row 4 of the example reads a BSON document, the one row that
/// needs the `bson` crate:
///
#[cfg_attr(
    all(
        feature = "serde",
        any(feature = "typescript", feature = "zod", feature = "jsonschema")
    ),
    doc = include_str!("preprocess_example.md")
)]
///
/// The key is refused where it is not an array of string literals, where a name is no identifier
/// or is written twice, beside `with`, `deserialize_with` or `as` on the same field, on a field
/// that borrows from the payload and on a type that holds one, on a type alias, and on the slot of
/// a branded newtype. serde's `remote`, `from`, `try_from` and `into` are refused on a type that
/// names one. A name with no function in scope is `E0425` at the string literal.
///
/// # Errors met most
///
/// - **An unknown key**, often a snake case spelling such as `min_length`: refused, with the list
///   of keys.
/// - **A key on a field it cannot hold**, such as `minimum` on a string: refused at the key.
/// - **A `literal` beside another rule**: refused, naming the literal and the rule.
/// - **A `check` or `preprocess` name with no function in scope**: `E0425` at the string literal.
/// - **A `pattern` that is more than a fixed text, in a crate that does not list `regex`**: the
///   generated check names `regex::Regex`, which does not resolve.
///
/// The README holds the full rules:
/// [Field Validation](https://github.com/tixena/tixschema/blob/master/README.md#field-validation-model_schema_prop),
/// [The `validate()` Method](https://github.com/tixena/tixschema/blob/master/README.md#the-validate-method),
/// [What a `pattern` may contain](https://github.com/tixena/tixschema/blob/master/README.md#what-a-pattern-may-contain),
/// [Literal Values](https://github.com/tixena/tixschema/blob/master/README.md#literal-values),
/// [Checks](https://github.com/tixena/tixschema/blob/master/README.md#checks-check),
/// [Preprocessing](https://github.com/tixena/tixschema/blob/master/README.md#preprocessing-preprocess),
/// [What each language calls](https://github.com/tixena/tixschema/blob/master/README.md#what-each-language-calls),
/// [Optional TypeScript Keys](https://github.com/tixena/tixschema/blob/master/README.md#optional-typescript-keys-ts_optional)
/// and [Nullable Object Keys](https://github.com/tixena/tixschema/blob/master/README.md#nullable-object-keys-nullable).
#[proc_macro_attribute]
pub fn model_schema_prop(_args: TokenStream, input: TokenStream) -> TokenStream {
    input
}

/// Declares a service as a trait, and generates its messages, its fault type, the Rust dispatchers
/// and clients it asks for, and its TypeScript, Dart, Swift and Kotlin clients.
///
/// Each operation is an `async fn` that takes `&self`, then the context, then its message, and
/// returns `Result<Success, Error>`, or nothing under `#[service_schema_op(one_way)]`. Each half
/// checks a message before it crosses: a client before it sends, a dispatcher before it calls the
/// implementation. A defect no operation declared reaches the caller as a fault, apart from the
/// operation's own errors. Every declaration needs tixschema's `serde` feature, and is refused
/// without it.
///
/// # Where it goes
///
/// On a trait with one type parameter, the context: a logger, a connection, whatever an
/// implementation needs that is not on the wire. Every operation takes it as `ctx: &Ctx` right
/// after `&self`, and no message carries it. The message, success and error types are
/// `#[model_schema]` types, declared at item scope in the same crate. A service that asks for a
/// transport is declared at the crate root, or below it with a `pub use` of the trait and of its
/// `{service}_schema` module at the crate root: the transport macros reach both from there.
///
/// # Arguments
///
/// - `transports = ["amqp_rpc", "http_rest", "ws_rpc"]`: the transports whose Rust dispatcher and
///   client the macro writes, as macros you invoke. `transports = []`, `#[service_schema()]` and
///   `#[service_schema]` ask for none.
/// - `non_exhaustive`: a bare flag that puts `#[non_exhaustive]` on the messages the macro
///   declares, on the fault kind, on `CallError` and on `{Service}Schema`.
///
/// The two may come in either order. Any other argument, and a transport name not in the list, is
/// refused. Per-operation directives go in
/// [`#[service_schema_op(...)]`](attr.service_schema_op.html).
///
/// # Operations and messages
///
/// - One argument after the context is the message itself, a type you declared.
/// - Several arguments, or none, make the macro declare `<Operation>Request` beside the trait, with
///   one field per argument and camelCase keys: `void_invoice(&self, ctx, invoice_id, reason)` gets
///   `VoidInvoiceRequest { invoice_id, reason }`. Its keys are the parameter names, so renaming a
///   parameter moves a key on the wire. The name carries no service prefix, so two services in one
///   module that both leave a `sweep` operation to the macro collide.
/// - The wire name is the method name in kebab-case, `void-invoice`, and TypeScript, Dart, Swift
///   and Kotlin call the method in camelCase, `voidInvoice`. A Rust client takes the arguments of a
///   several-argument operation one by one; the other languages take the generated message.
/// - The emitted trait turns each `async fn` into `-> impl Future<Output = ...> + Send`, so an
///   implementation holds nothing across an `.await` that is not `Send`. The trait is not dyn
///   compatible, and every dispatcher is generic over the implementing type.
///
/// # What it generates
///
/// Beside the trait, in every build with `serde`:
///
/// - The `<Operation>Request` messages above, each a `#[model_schema]` type that publishes on every
///   surface.
/// - The module `{service}_schema` (the trait's name in snake case). It holds `ServiceFault`, the
///   fault for a defect no operation declared, with `kind()`, `operation()`, `field()` and
///   `detail()` and one public constructor per kind; `ServiceFaultKind`, written on the wire as
///   `failed-validation`, `handler-panic`, `transport-failure`, `undeserializable-payload` or
///   `unknown-operation`; and `CallError<E>`, the failure of a Rust client call, which is
///   `Fault(ServiceFault)` or `Operation(E)`. For a dispatcher written by hand it also holds, per
///   operation, `{Operation}Message`, `unjudged_{operation}` (the message's read alone) and
///   `validated_{operation}` (the message's `validate()`, or `Ok(())` where it has none).
/// - `{Service}Schema`, whose functions return the text of each language's side. None of them
///   holds a message, success or error type you declared, so a bundle adds each one's own surfaces:
///   - `ts_definition()` under `typescript`: the messages the macro declared, with their Zod
///     schemas under `zod`, the fault, and one `{Service}{Operation}Result` per operation that
///     replies.
///   - `ts_client()`, `ts_service()`, `ts_http_client()`, `ts_http_service()`, `ts_ws_client()`,
///     `ts_ws_service()` and `ts_ws_server()` under `typescript` and `zod`: the clients and
///     dispatchers of each transport. `ts_ws_client()` binds the transport `ts_client()` declares,
///     and `ts_http_service()` and `ts_ws_service()` drive the dispatcher `ts_service()` declares,
///     so a bundle holds those as well.
///   - `dart_definition()`, `dart_http_client()` and `dart_ws_client()` under `dart`.
///   - `swift_http_client()` and `swift_ws_client()` under `swift`, and `kotlin_http_client()` and
///     `kotlin_ws_client()` under `kotlin`. The WebSocket client reuses the result and fault types
///     the HTTP client declares, so a bundle holds both. Swift and Kotlin have no service-level
///     definition: a bundle holds each message's own `{type}_swift::swift_definition()` or
///     `{type}_kotlin::kotlin_definition()`, and the fault's from
///     `{service}_schema::{service}_fault_kind_swift` and
///     `{service}_schema::{service}_fault_fields_swift`, or their `_kotlin` twins.
/// - For each transport asked for, `#[macro_export]` macros named
///   `{service}_{transport}_dispatcher!()` and `{service}_{transport}_client!()`, and for
///   `amqp_rpc` also `{service}_amqp_rpc_server!()`. Each takes no arguments and writes its items
///   into the module it is invoked in:
///   - A dispatcher: `dispatch`, which reads a message, validates it, calls the implementation
///     behind a panic guard and answers. `amqp_rpc` adds `IncomingMessage` and the `Reply` trait,
///     `http_rest` adds `IncomingRequest`, `OutgoingResponse`, `ROUTES` and `FaultHandler`, and
///     `ws_rpc` adds `answer(text, &service, &ctx)`, which answers one text frame.
///   - A client: the `Transport` trait it sends through, and `{Service}Client<T: Transport>` with
///     one method per operation, which validates the message before it sends. `ws_rpc` adds
///     `FrameSession`, a request and reply transport over one socket written with `std` alone, and
///     `FrameWriter`, a one-way one.
///   - `{service}_amqp_rpc_server!()`: the dispatcher, plus `serve_until`, a consumer loop over a
///     `lapin` channel, and `serve_deliveries`, the same loop over any stream of deliveries.
///
/// # What your crate lists
///
/// The crate that declares the service lists `tixschema` under that name with `serde` on, and
/// `serde` with its `derive` feature. A crate that invokes a dispatcher macro lists `serde`,
/// `serde_json` and `tracing`; a client macro, `serde` and `serde_json`; and
/// `{service}_amqp_rpc_server!()`, also `lapin`, `tokio` with its `macros` feature, and `futures`.
/// The TypeScript side needs Zod 4. The Dart, Swift and Kotlin clients name no networking library:
/// the app implements the transport each one sends through, as the examples below show. A Kotlin
/// build that holds `kotlin_ws_client()` depends on `kotlinx-coroutines-core`, and its file imports
/// `kotlinx.coroutines.*` and `kotlinx.coroutines.flow.*`. `kotlin_http_client()` names
/// `kotlinx.coroutines.flow` only for a service with a `body = "stream"` operation.
///
/// # Placing a transport macro
///
/// Invoke each macro in a module of its own file, with the `mod` line above the crate's `use`
/// items: an inline `mod name { ... }` meets `clippy::inline_modules` in a crate with strict lints.
/// The examples on this page are one file each, so they write each module inline.
///
/// The crate that declares the service calls a macro by its bare name, below the declaration:
/// `#[macro_use]` on the `mod` that declares the service carries the macro out of it, and
/// `crate::orders_http_rest_dispatcher!()` is refused there. A service declared below the crate
/// root is re-exported at the root, with its `{service}_schema` module:
///
/// ```text
/// // src/lib.rs
/// #[macro_use]
/// mod orders;
/// mod orders_http;
///
/// pub use orders::{Orders, orders_schema};
///
/// // src/orders.rs: the model types and the trait
/// #[service_schema(transports = ["http_rest"])]
/// pub trait Orders<Ctx> { ... }
///
/// // src/orders_http.rs
/// use crate::orders::OrderError;
///
/// orders_http_rest_dispatcher!();
/// ```
///
/// Any other crate calls the macro by its path, such as
/// `the_contract_crate::orders_http_rest_dispatcher!()`, in a module of its own file the same way.
///
/// The macros write some types by the name the trait's signatures spell, so the invoking module
/// brings those names into scope with `use`:
///
/// - A client module brings in every type the operations' signatures name: each message, success
///   and error type, and the type of each `header_in` and `part` argument.
/// - An `http_rest` dispatcher module brings in the error type of each operation that writes
///   `error_status`, which the dispatcher matches by variant, and the type of each `header_in` and
///   `part` argument. That is why the `http_rest` example on
///   [`service_schema_op`](attr.service_schema_op.html#example) imports `OrderError`.
/// - The `amqp_rpc` and `ws_rpc` dispatchers name no type of the trait's, so their modules need
///   no `use`.
///
/// A message the macro declares from several arguments, and the fault types, are reached through
/// `{service}_schema` and need no import.
///
/// # What a transport macro writes
///
/// The items your code calls, with `Orders` as the service:
///
/// ```text
/// // orders_http_rest_dispatcher!()
/// pub const ROUTES: &[Route]; // method(), path(), ok_status(), error_statuses(), operation()
/// impl IncomingRequest {
///     pub const fn new(method: String, path: String, query: String,
///                      headers: Vec<(String, String)>, body: Vec<u8>) -> Self;
/// }
/// impl OutgoingResponse {
///     pub const fn new(status: u16, headers: Vec<(String, String)>, body: Vec<u8>) -> Self;
///     pub const fn status(&self) -> u16;
///     pub fn headers(&self) -> &[(String, String)];
///     pub fn body(&self) -> &[u8];
/// }
/// pub trait FaultHandler {
///     // By default: 404 for an unknown route, 500 for a panic, 400 for any other fault.
///     fn on_fault(&self, fault: &orders_schema::ServiceFault) -> OutgoingResponse;
/// }
/// pub struct DefaultFaultHandler;
/// pub fn dispatch<S, Ctx, H>(svc: &S, ctx: &Ctx, request: &IncomingRequest, handler: &H)
///     -> impl Future<Output = OutgoingResponse> + Send
/// where S: Orders<Ctx> + Sync, Ctx: Sync, H: FaultHandler + Sync;
///
/// // orders_ws_rpc_dispatcher!()
/// pub async fn answer<S, Ctx>(text: &str, svc: &S, ctx: &Ctx) -> Option<String>
/// where S: Orders<Ctx> + Sync, Ctx: Sync;
///
/// // orders_amqp_rpc_client!() and orders_ws_rpc_client!()
/// pub trait Transport {
///     fn notify<T>(&self, operation: &str, payload: T, headers: Vec<(String, String)>)
///         -> impl Future<Output = Result<(), String>> + Send
///     where T: Serialize + Send;
///     fn request<T>(&self, operation: &str, payload: T, headers: Vec<(String, String)>)
///         -> impl Future<Output = Result<(Vec<u8>, Vec<(String, String)>), String>> + Send
///     where T: Serialize + Send;
/// }
/// impl<T: Transport> OrdersClient<T> {
///     pub const fn new(transport: T) -> Self;
/// }
///
/// // orders_http_rest_client!(): the same OrdersClient, over this Transport
/// pub trait Transport {
///     fn send(&self, request: OutgoingRequest)
///         -> impl Future<Output = Result<IncomingResponse, String>> + Send;
/// }
///
/// // orders_ws_rpc_client!() also writes two transports over one socket
/// #[derive(Clone)]
/// pub struct FrameSession; // request and reply
/// impl FrameSession {
///     pub fn new<F, Fut>(send: F) -> Self
///     where F: Fn(String) -> Fut + Send + Sync + 'static,
///           Fut: Future<Output = Result<(), String>> + Send + 'static;
///     pub async fn deliver(&self, text: &str); // each frame read off the socket
///     pub fn close(&self, detail: &str);       // fails every call still waiting
/// }
/// pub struct FrameWriter; // one way, built with FrameWriter::new(send)
///
/// // orders_amqp_rpc_server!()
/// pub async fn serve_until<S, F>(channel: &lapin::Channel, queue: &str, service: &S,
///                                prefetch: NonZeroU16, shutdown: F)
///     -> Result<Stopped, lapin::Error>
/// where S: Orders<Context> + Sync, F: Future<Output = ()> + Send;
/// ```
///
/// A client method takes the operation's message, or its arguments one by one, and answers
/// `Result<Success, CallError<Error>>`; a one-way method answers `Result<(), ServiceFault>`. A
/// `Transport` answers `Err` with words when the call did not travel, and the client turns that
/// into a fault of kind `transport-failure`.
///
/// # Examples
///
/// The three shapes of an operation, and what the macro declares beside them:
///
#[cfg_attr(feature = "serde", doc = include_str!("service_schema_example.md"))]
///
/// A Rust client over `ws_rpc`. Its transport here has no socket: it hands each request frame to
/// the dispatcher's `answer` and returns the reply, which is also how a test drives a client.
/// `FrameSession` is the transport the client macro writes for one real socket.
///
#[cfg_attr(
    all(
        feature = "serde",
        any(feature = "typescript", feature = "zod", feature = "jsonschema")
    ),
    doc = r#"
```rust
use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

use orders_schema::{CallError, ServiceFaultKind};

#[model_schema()]
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderLookup {
    #[model_schema_prop(minLength = 1)]
    pub order_id: String,
}

#[model_schema()]
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderSummary {
    pub order_id: String,
    pub total_cents: u32,
}

#[model_schema()]
#[derive(Deserialize, Serialize)]
#[serde(tag = "errorCode", rename_all = "kebab-case")]
pub enum OrderError {
    NotFound,
}

#[service_schema(transports = ["ws_rpc"])]
pub trait Orders<Ctx> {
    async fn get_order(&self, ctx: &Ctx, req: OrderLookup) -> Result<OrderSummary, OrderError>;
}

mod orders_ws {
    orders_ws_rpc_dispatcher!();
}

mod orders_client {
    use super::{OrderError, OrderLookup, OrderSummary};

    orders_ws_rpc_client!();
}

pub struct OrderBook;

impl Orders<()> for OrderBook {
    async fn get_order(&self, _ctx: &(), req: OrderLookup) -> Result<OrderSummary, OrderError> {
        if req.order_id == "A-1" {
            Ok(OrderSummary { order_id: req.order_id, total_cents: 4250 })
        } else {
            Err(OrderError::NotFound)
        }
    }
}

/// A transport with no socket: each frame goes to the dispatcher in the same process.
pub struct InMemory;

impl orders_client::Transport for InMemory {
    async fn notify<T>(
        &self,
        operation: &str,
        payload: T,
        headers: Vec<(String, String)>,
    ) -> Result<(), String>
    where
        T: Serialize + Send,
    {
        let frame = orders_client::notify_frame(operation, &payload, headers)?;
        orders_ws::answer(&frame, &OrderBook, &()).await;
        Ok(())
    }

    async fn request<T>(
        &self,
        operation: &str,
        payload: T,
        headers: Vec<(String, String)>,
    ) -> Result<(Vec<u8>, Vec<(String, String)>), String>
    where
        T: Serialize + Send,
    {
        let frame = orders_client::request_frame("1", operation, &payload, headers)?;
        let reply = orders_ws::answer(&frame, &OrderBook, &())
            .await
            .ok_or("the dispatcher sent no reply")?;
        let Ok(orders_client::Frame::Reply { envelope, .. }) = orders_client::Frame::decode(&reply)
        else {
            return Err(format!("not a reply frame: {reply}"));
        };
        let encoded = serde_json::to_vec(&envelope).map_err(|refused| refused.to_string())?;
        Ok((encoded, Vec::new()))
    }
}

/// Runs a call that never waits, as every call here is: one poll finishes it.
fn run<F: std::future::Future>(call: F) -> F::Output {
    use std::future::Future;
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};

    let mut call = pin!(call);
    match call.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(answered) => answered,
        Poll::Pending => panic!("the call waited"),
    }
}

fn main() {
    let client = orders_client::OrdersClient::new(InMemory);

    let found = run(client.get_order(OrderLookup { order_id: "A-1".to_owned() }));
    assert!(matches!(found, Ok(OrderSummary { total_cents: 4250, .. })));

    let missing = run(client.get_order(OrderLookup { order_id: "B-7".to_owned() }));
    assert!(matches!(missing, Err(CallError::Operation(OrderError::NotFound))));

    // The client checks the message before the transport is reached.
    let refused = run(client.get_order(OrderLookup { order_id: String::new() }));
    let Err(CallError::Fault(fault)) = refused else {
        panic!("the empty id was sent");
    };
    assert_eq!(fault.kind(), ServiceFaultKind::FailedValidation);
}
```
"#
)]
///
/// The service's TypeScript, which a test or a small binary of the crate writes into the web
/// project:
///
#[cfg_attr(
    all(feature = "serde", feature = "typescript", feature = "zod"),
    doc = r#"
```rust
use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

#[model_schema()]
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Receipt {
    pub receipt_id: String,
}

#[model_schema()]
#[derive(Deserialize, Serialize)]
#[serde(tag = "errorCode", rename_all = "kebab-case")]
pub enum PaymentError {
    Declined,
}

#[service_schema()]
pub trait Payments<Ctx> {
    async fn capture(
        &self,
        ctx: &Ctx,
        order_id: String,
        amount_cents: u32,
    ) -> Result<Receipt, PaymentError>;
}

/// The service's own part of a bundle: its types, then the clients.
fn service_typescript() -> String {
    [
        PaymentsSchema::ts_definition(),
        PaymentsSchema::ts_client(),
        PaymentsSchema::ts_http_client(),
    ]
    .join("\n\n")
}

fn main() {
    let typescript = service_typescript();
    assert!(typescript.contains("export type CaptureRequest = {"));
    assert!(typescript.contains("export type PaymentsCaptureResult ="));
    assert!(typescript.contains("export type PaymentsTransport = {"));
    assert!(typescript.contains(
        "export function createPaymentsClient(transport: PaymentsTransport): PaymentsClient {"
    ));
    assert!(typescript.contains(
        "export type PaymentsHttpTransport = {\n  send(request: {\n    method: string;\n    path: string;\n    query: string;\n    headers: ReadonlyArray<readonly [string, string]>;\n    body: Uint8Array;\n  }): Promise<{\n    status: number;\n    headers: ReadonlyArray<readonly [string, string]>;\n    body: Uint8Array;\n  }>;\n};"
    ));
    assert!(typescript.contains(
        "export function createPaymentsHttpClient(\n  transport: PaymentsHttpTransport,\n): PaymentsHttpClient {"
    ));
}
```
"#
)]
///
/// `ts_http_client()` declares the transport the web app implements over `fetch`. A capture is a
/// `POST /capture`, the default route of an operation with no `http(...)`:
///
/// ```typescript
/// export type PaymentsHttpTransport = {
///   send(request: {
///     method: string;
///     path: string;
///     query: string;
///     headers: ReadonlyArray<readonly [string, string]>;
///     body: Uint8Array;
///   }): Promise<{
///     status: number;
///     headers: ReadonlyArray<readonly [string, string]>;
///     body: Uint8Array;
///   }>;
/// };
/// ```
///
/// ```typescript
/// const transport: PaymentsHttpTransport = {
///   async send(request) {
///     const headers = new Headers();
///     for (const [name, value] of request.headers) headers.append(name, value);
///     const query = request.query === "" ? "" : `?${request.query}`;
///     const response = await fetch(`/api${request.path}${query}`, {
///       method: request.method,
///       headers,
///       body: request.body.length === 0 ? undefined : request.body.slice(),
///     });
///     return {
///       status: response.status,
///       headers: [...response.headers],
///       body: new Uint8Array(await response.arrayBuffer()),
///     };
///   },
/// };
///
/// const payments = createPaymentsHttpClient(transport);
/// const result = await payments.capture({ orderId: "A-1", amountCents: 4250 });
/// if (result.ok) {
///   showReceipt(result.value.receiptId);
/// } else if ("isServiceFault" in result.error) {
///   reportDefect(result.error.fault);
/// } else {
///   showDeclined(result.error.errorCode);
/// }
/// ```
///
/// A thrown `fetch` answers the `transport-failure` fault. Over a WebSocket,
/// `createPaymentsWsTransport(new WebSocket(url))` from `ts_ws_client()` returns a
/// `PaymentsTransport`, the type `ts_client()` declares, and `createPaymentsClient(transport)`
/// binds the client to it.
///
/// The Dart, Swift and Kotlin clients take a transport the app implements over its own HTTP
/// stack:
///
/// - Dart: `PaymentsHttpClient(transport)`, where `transport` implements the abstract class
///   `PaymentsHttpTransport` and its one method, which takes the record
///   `({String method, String path, String query, List<(String, String)> headers, List<int> body, List<(String, dynamic)> parts})`
///   and returns a `Future` of
///   `({int status, List<(String, String)> headers, List<int> body, Stream<List<int>> bodyStream})`.
/// - Swift: `PaymentsHttpClient(transport:)`, over a type that conforms to `PaymentsHttpTransport`
///   with `func send(_ request: PaymentsHttpRequest) async throws -> PaymentsHttpResponse`.
/// - Kotlin: `PaymentsHttpClient(transport)`, over a class that implements `PaymentsHttpTransport`
///   with `suspend fun send(request: PaymentsHttpRequest): PaymentsHttpResponse`.
///
/// Over a WebSocket, each language has a `PaymentsWsTransport`. Dart's takes the `sink` and
/// `stream` of a `WebSocketChannel`. Swift's and Kotlin's take a `PaymentsWsSocket` the app
/// implements, with `send`, `close`, `onMessage` and `onClose`.
///
/// The [crate page](index.html#a-service-with-typed-clients) drives a `ws_rpc` dispatcher with no
/// socket and no runtime, and the [`service_schema_op`](attr.service_schema_op.html#example)
/// example serves `http_rest` from an HTTP handler.
///
/// # Errors met most
///
/// - **The `serde` feature off**: the declaration is refused, naming the feature to add.
/// - **An operation without `&self`, without `ctx: &Ctx` after it, or returning no `Result` without
///   `one_way`**: refused, saying what to write.
/// - **An implementation that leaves out an operation**: `E0046`.
/// - **`ts_client()` and the other TypeScript client functions with `zod` off**: `E0599`.
/// - **A trait with a second type parameter, or a service below the crate root with no `pub use`**,
///   once a transport is asked for: refused at the declaration.
/// - **Two services in one module whose generated messages share a name**: duplicate definitions.
///   Rename an operation, or have it take a message you declared.
///
/// The README holds the full rules:
/// [Services](https://github.com/tixena/tixschema/blob/master/README.md#services),
/// [The Shape of a Service](https://github.com/tixena/tixschema/blob/master/README.md#the-shape-of-a-service),
/// [Service Messages](https://github.com/tixena/tixschema/blob/master/README.md#service-messages),
/// [Service Faults](https://github.com/tixena/tixschema/blob/master/README.md#service-faults),
/// [What a Service Generates](https://github.com/tixena/tixschema/blob/master/README.md#what-a-service-generates),
/// [Transports](https://github.com/tixena/tixschema/blob/master/README.md#transports),
/// [The `amqp_rpc` walkthrough](https://github.com/tixena/tixschema/blob/master/README.md#the-amqp_rpc-walkthrough),
/// [The `ws_rpc` Transport](https://github.com/tixena/tixschema/blob/master/README.md#the-ws_rpc-transport)
/// and [The Form Each Client Sends](https://github.com/tixena/tixschema/blob/master/README.md#the-form-each-client-sends).
#[proc_macro_attribute]
pub fn service_schema(args: TokenStream, input: TokenStream) -> TokenStream {
    exec_service_schema(args.into(), input.into()).into()
}

/// Sets how one operation of a `#[service_schema]` trait travels: its name on the wire, whether it
/// replies, and its binding on the `http_rest` transport.
///
/// [`#[service_schema]`](attr.service_schema.html) reads this attribute and removes it from the
/// trait; written anywhere else, it leaves the item unchanged. An operation with none travels under
/// its method name in kebab-case, replies with `Result<Success, Error>`, and over `http_rest`
/// answers `POST /{wire-name}`, with 200 for a success (204 where it carries nothing) and 422 for
/// every declared error.
///
/// # Directives
///
/// - `message = "wire-name"`: the operation's name on the wire, in place of the method name in
///   kebab-case. Rust and the other languages still call the operation by its method name. Two
///   operations of one service may not share a wire name.
/// - `one_way`: the operation sends no reply, so it declares no return type and no error. An
///   operation without it returns `Result<Success, Error>`, and each choice is refused without the
///   other.
/// - `http(...)`: the operation's binding on `http_rest`, made of the arguments below. `method` and
///   `path` are required.
///   - `method = "GET"`: `GET`, `POST`, `PUT`, `PATCH` or `DELETE`.
///   - `path = "/orders/{order_id}"`: the path template, where each `{field}` binds the field of
///     the message with that name. `GET` and `DELETE` carry no body, so every other field of their
///     message is an `Option`, read from the query string. On those two methods a single message
///     argument is refused, unless it is a scalar the path's one placeholder binds whole.
///   - `ok_status = 200`: the status of a success.
///   - `error_status(NotFound = 404, ...)`: the status of each variant of the error enum. Once
///     written it names every variant, and a variant left out is rustc's `E0004`. Without it,
///     every declared error answers 422.
///   - `header_in("name" = argument)`: fills one argument of the operation from a request header,
///     and leaves that argument out of the message.
///   - `header_out("name")`: writes a response header. The success type becomes a tuple, the
///     response first and then one value per `header_out`, in the order written.
///   - `error_header_out("name")`: the same, on a response that answers a declared error.
///   - `body = "json"`, `"bytes"`, `"multipart"` or `"stream"`: `json` is the default. `bytes`
///     answers `(Vec<u8>, String)`, the bytes and their content type. `stream` answers the
///     `StreamedAnswer` the service's module declares, the whole body or a 206 range. `multipart`
///     reads the request as named parts.
///   - `part("name" = argument)`: on a `multipart` operation, fills one argument from a file part.
///
/// `body = "stream"` and file parts travel only over `http_rest`, so a service that also asks for
/// `amqp_rpc` or `ws_rpc` is refused. A service that asks for `amqp_rpc` may not declare
/// `header_out("is_error")` or `error_header_out("is_error")`: `amqp_rpc` writes the `is_error`
/// header itself.
///
/// # Example
///
/// Three operations over `http_rest`, the route table its Rust dispatcher answers with, and one
/// HTTP handler that serves them. The handler takes and returns the `http` crate's `Request` and
/// `Response`, which axum and hyper hand a handler, here through axum's re-export: it builds an
/// `IncomingRequest` from the request, awaits `dispatch`, and turns the `OutgoingResponse` into the
/// response. Nothing here opens a socket.
///
#[cfg_attr(
    all(
        feature = "serde",
        any(feature = "typescript", feature = "zod", feature = "jsonschema")
    ),
    doc = include_str!("service_schema_op_example.md")
)]
///
/// The README holds the full rules:
/// [Operation Names](https://github.com/tixena/tixschema/blob/master/README.md#operation-names),
/// [One-Way Operations](https://github.com/tixena/tixschema/blob/master/README.md#one-way-operations)
/// and [The `http_rest` Transport](https://github.com/tixena/tixschema/blob/master/README.md#the-http_rest-transport).
#[proc_macro_attribute]
pub fn service_schema_op(_args: TokenStream, input: TokenStream) -> TokenStream {
    input
}
