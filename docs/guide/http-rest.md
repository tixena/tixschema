#### The `http_rest` Transport

`#[service_schema(transports = ["http_rest"])]` asks for a real REST surface instead of (or beside) `amqp_rpc`: owner-declared methods, paths, statuses and header bindings, served through whatever HTTP stack the hosting application already uses. It contributes **two** macros rather than three -- `{service_snake_case}_http_rest_dispatcher!()` and `{service_snake_case}_http_rest_client!()` -- because there is no third: `amqp_rpc`'s server macro exists because AMQP's own request-and-reply shape is fixed enough that one consumer loop over `lapin` serves every service that places it, and no such fixed loop exists for HTTP. What an HTTP server looks like is the hosting application's own router, so `http_rest` stops at the dispatcher and leaves registering routes on a real listener to a small hand-written adapter -- see "Adapters carry no tixschema runtime crate" below.

There is no envelope on this transport. `amqp_rpc` answers every reply through `{ ok, value?, error? }`; `http_rest` answers a success with the declared `ok_status` and the success type serialized directly as the body, a declared error with its mapped status and the error type itself as the body, and a fault -- a defect no operation declared -- through the fault catch-all described below. The status line carries what the `ok` flag carries on the bus.

**The `http(...)` grammar.** Every grammar arm at once, on one operation:

```rust,ignore
#[service_schema(transports = ["http_rest"])]
pub trait DocumentService<Ctx> {
    #[service_schema_op(http(
        method = "GET",
        path = "/documents/{document_id}/versions/{version_id}",
        ok_status = 200,
        header_in("range" = byte_range),
        header_out("etag"),
        error_status(NotFound = 404, VersionGone = 410),
    ))]
    async fn get_version(
        &self,
        ctx: &Ctx,
        document_id: String,
        version_id: String,
        byte_range: Option<String>,
    ) -> Result<(VersionResponse, String), GetVersionError>;
}
```

`method` is one of `"GET"`, `"POST"`, `"PUT"`, `"DELETE"`, `"PATCH"`; `path` is a template walked left to right, `{field}` placeholders included; `ok_status` is the success status (default 200, or 204 where the reply carries nothing -- see below); `error_status(Variant = code, ...)` maps the operation's own declared error variants to the statuses they answer at; `header_in`/`header_out`/`error_header_out` bind request and response headers (covered on its own below); `body` picks the body kind (`"json"`, the default; `"bytes"`; `"multipart"`; or `"stream"` -- covered on its own below); and `part("name" = parameter)` claims one multipart file part, only meaningful under `body = "multipart"`. Writing an argument this grammar does not recognise is refused naming the arguments it does:

```text
service_schema: unknown `http` argument
       the arguments are `method`, `path`, `ok_status`, `error_status`, `header_in`, `header_out`, `error_header_out`, `part` and `body`
```

**An operation naming no `http(...)` group at all** still gets one, defaulted rather than left unhandled: `POST /{wire-name}`, the same `ok_status` default every annotated operation gets (200, or 204 for nothing to serialize), no header bindings, and every declared error answered at one fixed status, `422`, rather than a per-variant table -- there being no annotation to read one from. So an internal service pays no attribute cost and still answers real HTTP:

```rust,ignore
    // Names no `http(...)` group: defaults to `POST /sweep-documents`, 200 on success,
    // every declared error of `SweepError` at 422.
    async fn sweep_documents(&self, ctx: &Ctx) -> Result<SweepReport, SweepError>;
```

**Path placeholders and the query string.** A `{field}` placeholder binds a same-named field on the operation's own message; naming one the message does not have is refused on the placeholder:

```text
service_schema: operation `get_widget`'s path names `{item_id}`, and its message has no field named `item_id`
              a path placeholder binds a same-named field on the message
```

On a message of its own type, a placeholder names the field as Rust writes it, whatever key serde writes it under: `{document_id}`, for a field `document_id` under `#[serde(rename_all = "camelCase")]` or `#[serde(rename = "doc-id")]`. Every client reads the field through the name its language gives it, and each dispatcher writes the path's value over the key serde reads the field from, so the implementation gets the path's value whatever the body carries. A struct `#[model_schema()]` wrote publishes those keys as `WIRE_KEYS`, a const on the type, as each member's name in Rust beside its key and the form its text is read in: `text`, `boolean`, `number`, `bigint` for a `u64` or an `i64`, and `list of ` ahead of the form of a list's item. Each dispatcher reads a placeholder's text in that form, as it reads a placeholder of a message the macro declares from the operation's arguments (below): with `pub page_size: u32`, `PUT /pages/7` reaches the implementation as the number `7`, and `PUT /pages/abc` is refused with ``'page_size': invalid type: string "abc", expected u32``. A placeholder that names anything else on such a struct stops the build: the key on the wire, a member of a `#[serde(flatten)]` field, a field serde does not read, and any placeholder on an enum, which holds no field a placeholder binds:

```text
error[E0080]: evaluation panicked: service_schema: operation `by_wire_name`'s path names `{documentId}`, and its message has no field named `documentId` that serde reads under a key of its own. A path placeholder names a field of a struct as Rust writes it, whatever key serde writes it under. An enum holds no field a placeholder binds: bind a struct, or declare the field as an argument of the operation.
```

A message `#[model_schema()]` did not write publishes no keys, and a placeholder on it stops the build, since no dispatcher can learn the key serde reads the field from:

```text
error[E0080]: evaluation panicked: service_schema: operation `unlisted`'s path names `{document_id}`, and its message `Unlisted` publishes no key of its fields, so no dispatcher can write the path's value over the key serde reads the field from. Put `#[model_schema()]` on `Unlisted`.
```

Put `#[model_schema()]` on the struct, or declare the fields as the operation's arguments, whose keys the macro reads.

Every client fills a placeholder the way the Rust client does, through the value's own codec: a string as itself, a number or boolean as its text, and a newtype such as `MediaId("abc123")` as its wire value, `/media/abc123`, each percent-encoded. The Rust and TypeScript dispatchers read a placeholder percent-decoded, as they read a query value, so `a b/c` is written `/media/a%20b%2Fc` and read back as `a b/c`; an adapter hands `IncomingRequest::new` the path as the request carried it, `Uri::path()`, not a decoded one. That includes a field of an author's own named message, whose type the macro never sees: the Dart, Kotlin and Swift clients render it at runtime through a generated helper.

A placeholder that binds a list is written as the list's items, each in the form above, percent-encoded as one text and joined by a comma: `pages: Vec<u32>` of `[1, 2, 3]` on `/listed/{pages}` is `/listed/1%2C2%2C3` on every client, whether the message is declared or generated, and both dispatchers read it back as the list. Both dispatchers percent-decode the text before they split it on `,`, so no form carries an item that holds a comma; every client refuses such a list before sending, answering the `failed-validation` fault on the placeholder's field with ``a list item holds a comma, which a path would read as two items`` (a reply operation's `Fault` result, a one-way operation's refusal). Carry such a list in the body, or as an argument the path does not bind.

`GET` and `DELETE` carry no request body, so a required field the path did not bind has nowhere left to go and is refused on the field rather than silently dropped:

```text
service_schema: operation `get_widget`'s field `filter` is required and is bound by no path placeholder
              `GET` carries no body, so a required field must appear in the path
```

An *optional* field a bodyless method leaves unbound becomes a query parameter instead, read out of the parsed query string on the way in and appended to it on the way out. A path placeholder, a query parameter and a `header_in` value are all read the same way, off the operation argument's own declared type: `true`/`false` become a JSON boolean, anything that parses as a number becomes a JSON number, anything else stays a JSON string, and a `Vec<T>` splits its raw text on `,` and coerces each piece the same way -- the dispatcher and the TypeScript client apply this judgement identically, from one shared rule, so the two sides of the wire cannot read a query string two different ways.

A [brand](generics-and-brands.md#branded-newtypes), a struct of one slot or an alias over a number or a `bool`, declared above the service, is read the way the number or the `bool` it wraps is: `pub struct Berth(u32);` handed `7` in a path, a query, a header or a text part is read as the number `7`, a brand over an `i64` or a `u64` is read exactly, and every client writes it as that text, and reads a `header_out` of it back from that text. A brand over a `String` stays text, whatever its digits: `pub struct Label(String);` handed `007` is read as `"007"`. The kind is recorded as the type expands, so a type declared below the service, or in another crate, is read as text, and a brand over a number declared there is refused with `invalid type: string "7", expected u32`; declare it above the service. A member of a message of its own type is read in the form the message publishes, which is recorded as the message expands: declare a brand over a number such a member holds above the message.

A single-argument message -- the whole message already being the author's own type, or the argument list collapsed to one -- gives this macro no fields to read off the query string, so a bodyless method refuses it outright, naming what to write instead:

```text
service_schema: operation `download` takes its whole message as the one argument `req` on a `GET`
              a `GET` carries no body, and the path does not bind `req`; this macro cannot see its fields to read them off the query string
              declare each field as its own argument, so the message is generated and every field is read by its type
```

The one exception is a scalar bound whole by the path's one placeholder of its own name -- `get(&self, ctx: &Ctx, id: String)` on `/documents/{id}` stays legal, its value traveling in the path rather than the query.

**`header_in`/`header_out` on `http_rest` are the same grammar already introduced above for `amqp_rpc`'s headers-table channel, realized over real HTTP request and response headers instead.** `header_in("range" = byte_range)` binds the request's own `range` header to an ordinary argument beside the message; naming a parameter the operation does not take is refused on the parameter:

```text
service_schema: operation `get_widget` binds header "range" to a parameter named `byte_range`, and `get_widget` takes no argument by that name
              `header_in` binds one ordinary argument beside the message; name it in the signature, or remove the binding
```

`header_out("etag")` makes the operation's success type a tuple: the body first, then one element per declared `header_out` entry, in declaration order, each written out as a response header and read back the same way by the client. A tuple success type with no `header_out` to explain it is refused naming the requirement, and so is a `header_out` whose count does not match the tuple's own arity:

```text
service_schema: operation `get_widget` returns a tuple success type and declares no `header_out`
              name what each element after the first is with `header_out("name")`, or return the type directly
```

`error_header_out("name")` is the same tuple-splitting rule applied to the *declared error* instead of the success type: the error first, then one element per declared `error_header_out` entry, in declaration order, each written out as a response header when the error is the reply and read back the same way by the client. A tuple error type with no `error_header_out` to explain it, an `error_header_out` whose count does not match the tuple's own arity, and `error_header_out` on a `one_way` operation (which has no declared error to carry one in) are each refused naming the requirement:

```text
service_schema: operation `check_range` returns a tuple error type and declares no `error_header_out`
              name what each element after the first is with `error_header_out("name")`, or return the type directly
```

**Header names and values are checked, not trusted.** Across `header_in`, `header_out` and `error_header_out` together, on one operation: a header name must be a legal HTTP token (letters, digits and `` !#$%&'*+-.^_`|~ ``, RFC 9110's own `tchar` set); a name the transport already writes itself -- `content-type`, `content-length`, and `content-range` in `header_in` or `header_out` on a `body = "stream"` operation -- cannot be claimed by any of the three directives; and within any one of `header_in`, `header_out` or `error_header_out`, the same name cannot be declared twice -- the same name declared in two *different* lists, an echoed `x-request-id` read by `header_in` and written back by `header_out`, is legal. All three are refused at compile time, on the operation:

```text
service_schema: operation `get_widget` declares the header name "content-type", which `json_response`, and a `body = "bytes"` or `body = "stream"` reply's own content type, writes itself
              name a header this transport does not already control
```

A header *value* is checked at runtime instead, on every HTTP writer: `header_in`, `header_out` and `error_header_out` values are each rejected -- a fault on the server, a refusal before sending on the client -- unless every byte is visible ASCII (`0x21`-`0x7E`), a space, or a tab, which rules out a value carrying a line break or another control character before it ever reaches the wire.

Every REST client reads a `header_out` or `error_header_out` element back the way the Rust client does. An absent optional element reads as the language's own null. An absent required one answers the `undeserializable-payload` fault `a declared response header was missing`. A declared type -- a newtype such as `MediaId`, or an enum -- is read as a JSON string through its own codec, as the Rust client reads it. A present one that does not decode as its declared type -- a number that does not parse, a boolean other than `true`/`false`, a value its type's codec refuses, any piece of a list -- answers the same fault with `a response header did not match its declared type`, never an exception or a default value.

**Statuses.** `ok_status` and `error_status` are owner-chosen, exactly as `amqp_rpc`'s own message contract stays HTTP-free -- neither lives on a type, only on the operation. `error_status`'s completeness against the operation's own declared error type is checked unconditionally, by a `match` over exactly the declared arms and no wildcard: a variant the table leaves out is refused with rustc's own exhaustiveness check, naming it, and the refusal is spanned on the trait's own `#[service_schema]` attribute rather than invented by this crate:

```text
error[E0004]: non-exhaustive patterns: `&WidgetError::Gone` not covered
```

An operation whose reply carries nothing to serialize -- `one_way`, or a declared reply of `()` -- answers a bodyless status: 204 by default, or whatever `ok_status` the owner wrote (202 Accepted for queued work, say). Nothing about the *shape* of that case is special: the dispatcher decodes, validates and calls the implementation exactly as any other arm does, and only the answer has nothing in it.

**Body kinds.** `body = "json"` is the default: the message is the whole body on a bodied method, and the success/error types are the whole JSON body coming back. `body = "bytes"` says the response is not JSON: the reply's success type must be exactly `(Vec<u8>, String)` -- the raw bytes and their content type -- plus one further tuple element per declared `header_out` entry, in that order. A reply that still claims a JSON success type is refused naming the required shape:

```text
service_schema: `body = "bytes"` requires a success type of `(Vec<u8>, String)`
              the operation's signature still claims a JSON success type - answer `Result<(Vec<u8>, String), Error>`, the bytes and their content type
```

`body = "stream"` answers an undrained body instead of a materialized one: the reply's success type must name `StreamedAnswer` -- the seam type `#[service_schema]` publishes beside the trait wherever any operation needs it -- again plus one tuple element per `header_out` entry:

```rust,ignore
async fn get_content(
    &self,
    ctx: &Ctx,
    document_id: String,
    byte_range: Option<String>,
) -> Result<content_service_schema::StreamedAnswer, ContentError>;
```

`StreamedAnswer::Full { source, content_type }` answers the declared `ok_status` with the whole body; `StreamedAnswer::Partial { source, content_range, content_type }` answers `206` with `content-range` set to what the handler built (`bytes {start}-{end}/{total}`, RFC 9110 section 14.4), the declared headers (if any) riding beside it rather than being replaced by it. Both write `content_type` as the `content-type` header, checked like any other header value, as a `body = "bytes"` reply writes the second element of its pair; every client reads it back into the answer. A declared error never gets a `content-range` from the transport, so `error_header_out("content-range")` is how a `416` answers `bytes */<size>`, the complete length a client reads to learn the object's size (RFC 9110 section 15.5.17). `source` is a `Box<dyn BodySource + Send>` -- `BodySource::pull(&mut self, buf) -> io::Result<usize>` pulled one chunk at a time, blanket-implemented for every `std::io::Read` so a `File` or any other reader an author already has satisfies it for free, and named through no runtime crate beyond `std::io`. The dispatcher never buffers a streamed body whole; it hands `OutgoingBody::Stream(source)` to the adapter, which drains it onto the wire a chunk at a time.

`body = "multipart"` reads the *request* as named parts instead of one JSON object -- it says nothing about the response, whose success and error types stay ordinary JSON, `header_out` included, exactly like `body = "json"`. A scalar field is read straight off the same-named part, the same way a bodyless method's field is read off the query string; a `part("name" = parameter)` binding claims one file part and hands it through as the same `BodySource` handle a streamed response pulls from, undecoded -- a message field never carries one, `BodySource` publishing no `Deserialize` for it to fall back to. `GET` and `DELETE` carry no request body for parts to travel in, so `body = "multipart"` on either is refused naming the method:

```text
service_schema: operation `upload_document` declares `body = "multipart"` on a `GET`
              `GET` carries no request body for parts to travel in; write `POST`, `PUT` or `PATCH`
```

and a `part(...)` naming a parameter the operation does not take is refused exactly as an unclaimed `header_in` is:

```text
service_schema: operation `upload_document` binds part "file" to a parameter named `attachment`, and `upload_document` takes no argument by that name
              `part` binds one ordinary argument beside the message; name it in the signature, or remove the binding
```

A single-argument message under `body = "multipart"` is refused the same way, struct or lone scalar alike -- this macro cannot see its fields to send or read as parts:

```text
service_schema: operation `upload` takes its whole message as the one argument `req` under `body = "multipart"`
              a multipart request carries its fields as text parts, and this macro cannot see `req`'s fields to write or read them
              declare each field as its own argument beside the `part(...)` bindings
```

Verified end to end, no server involved: a multipart upload dispatched by hand decodes its scalar parts into the generated message and hands the file part through as an undrained `BodySource`, draining to exactly the bytes it was built with, before `validate()` runs; a streamed download answers `200` with the whole body pulled across more than one `BodySource::pull()` call for a body larger than the reader's own chunk cap, and a `range` header answers `206` with `content-range` and the sliced body, the declared `header_out` riding beside either arm.

**What the dispatcher macro emits.** `IncomingRequest` -- one plain-terms request, built with `IncomingRequest::new(method, path, query, headers, body)` by whichever adapter owns the socket, and read back through `method()`, `path()` (the path as the request carried it, percent-encoded, `dispatch` decoding each placeholder it matches), `query()` (the query string, unparsed, `dispatch` reading it itself), `header(name)` (case-insensitive) and `headers()`. `OutgoingResponse` -- `status()`, `headers()`, `body()` (or, on a service with at least one streamed operation, `into_body()` answering an `OutgoingBody` of `Bytes(Vec<u8>)` or `Stream(Box<dyn BodySource + Send>)` instead of a bare `body()`, since materializing every answer into bytes is exactly what a streamed body must not do). `Route`/`ROUTES` -- every operation's method, path template (`{field}` placeholders written exactly as declared), `ok_status()` and `error_statuses()`, for an adapter to iterate when it registers one handler per route. `FaultHandler` -- the fault catch-all, covered next -- and `dispatch(service, ctx, request, [parts,] handler) -> impl Future<Output = OutgoingResponse>`, generic over the implementing type (a trait with `async` methods is not `dyn`-compatible), which matches the method and path itself, decodes path, query, headers and body into the operation's own arguments, validates, calls the implementation behind the same panic guard every transport uses, and answers through `FaultHandler` for anything the operation itself did not declare. A service with at least one `body = "multipart"` operation gives `dispatch` one further argument, `parts: Vec<(String, IncomingPart)>` -- `IncomingPart::Text(String)` or `IncomingPart::File(Box<dyn BodySource + Send>)`, one entry per part the adapter already split off the wire.

**What the client macro emits.** `OutgoingRequest` -- `method()`, `path()` (placeholders already filled in), `query()`, `headers()`, `body()`, plus `into_parts()` on a service with a multipart operation, answering the `Vec<(String, OutgoingPart)>` a `body = "multipart"` method built (`OutgoingPart::Text`/`OutgoingPart::File`, the file variant carrying an owned `BodySource` handle). `IncomingResponse` -- `status()`, `header(name)`, `body()` (or `into_body()` answering `IncomingBody::Bytes`/`IncomingBody::Stream` where the service streams). `Transport` -- what an adapter implements: one method, `send(&self, request: OutgoingRequest) -> impl Future<Output = Result<IncomingResponse, String>> + Send`, sending a whole request and answering with a whole response or with what stopped it in words. `{Service}Client<T: Transport>` -- `new(transport)`, `transport()`, and one method per operation, over any `T: Transport + Sync`, each building the request from the operation's own validated message (path, query, headers filled from it) and decoding the answer by status exactly as the dispatcher answers it: the declared `ok_status` into the success type (or tuple, `header_out` elements read back off response headers; or the bytes and content type for `body = "bytes"`; or a `StreamedAnswer` pulling from the response body for `body = "stream"`, `Full` at `200`, `Partial` with `content-range` at `206`, both with their `content-type`), a mapped status into the declared error, and anything else -- including a fixed fault status this build did not expect at that code -- into a fault.

**Placement follows the same doctrine as `amqp_rpc`'s three macros** -- see "Where each goes, and why placement is fussy" above: each invocation in a module of its own file, the `mod` declaration above the crate's `use` items, invoked by name (not by path) from inside the declaring crate:

```text
// the crate that serves it -- src/lib.rs
mod http_transport;

// src/http_transport.rs
declaring_crate::document_service_http_rest_dispatcher!();
```

```text
// a crate that calls it -- src/lib.rs
mod http_client;

// src/http_client.rs
use declaring_crate::{GetVersionRequest, VersionResponse};

declaring_crate::document_service_http_rest_client!();
```

The invoking crate names its own subset of the runtime crates the macro's body reaches through `::`: the dispatcher's crate names `serde`, `serde_json`, `serde_path_to_error` and `tracing` (the panic guard logs through `tracing`); the client's crate names `serde`, `serde_json` and `serde_path_to_error` and not `tracing` -- nothing in a client catches a panic.

**The fault catch-all.** A payload that will not deserialize or fails `validate()`, a request no route answers to, and a handler that panicked are none of the things an operation declared it could fail at, so they answer through `FaultHandler` rather than through any declared error:

```rust,ignore
pub trait FaultHandler {
    fn on_fault(&self, fault: &ServiceFault) -> OutgoingResponse {
        // the provided default
    }
}

pub struct DefaultFaultHandler;
impl FaultHandler for DefaultFaultHandler {}
```

The provided default answers 404 for `ServiceFaultKind::UnknownOperation`, 500 for `ServiceFaultKind::HandlerPanic`, and 400 for everything else (an undeserializable payload, a validation failure), each as the fault itself serialized under `json_response`. `dispatch` takes a `&H: FaultHandler` argument, so an owner installs their own by implementing the trait on a type of their own and handing it to `dispatch` in place of `DefaultFaultHandler`; overriding `on_fault` replaces the default for every kind at once, `fault.kind()` being how an override tells one kind from another:

```rust,ignore
struct RecordingFaultHandler;

impl FaultHandler for RecordingFaultHandler {
    fn on_fault(&self, fault: &ServiceFault) -> OutgoingResponse {
        let status = match fault.kind() {
            ServiceFaultKind::UnknownOperation => 490,
            ServiceFaultKind::FailedValidation | ServiceFaultKind::UndeserializablePayload => 491,
            ServiceFaultKind::HandlerPanic => 492,
            ServiceFaultKind::TransportFailure => 493,
        };
        OutgoingResponse::new(
            status,
            vec![("x-fault-kind".to_owned(), format!("{}", fault.kind()))],
            format!("recorded: {}", fault.detail()).into_bytes(),
        )
    }
}
```

A `dispatch` built on `RecordingFaultHandler` answers a request to an unmatched route with status 490 and an `x-fault-kind: unknown operation` header instead of the default's 404 -- nothing about the numbers above is the contract; the seam is.

**Adapters carry no tixschema runtime crate.** `IncomingRequest`, `OutgoingResponse`, `Route`/`ROUTES`, `FaultHandler` and `dispatch` on one side, and `OutgoingRequest`, `IncomingResponse` and `Transport` on the other, are plain terms: a method, a path, headers and a body in, a status, headers and a body out. Nothing generated here names Axum, reqwest or fetch. A server adapter is a small hand-written loop, living with the hosting application, that iterates `ROUTES` to register one handler per route on its own router, and inside each handler builds an `IncomingRequest` from whatever its framework handed it, calls `dispatch`, and writes the answered `OutgoingResponse` back exactly as its own framework answers a request -- the same shape this crate's own dual-transport test harness proves by joining a client straight back to a dispatcher with no socket at all:

```rust,ignore
impl<H: FaultHandler + Sync> http_rest_client::Transport for HttpLoop<'_, H> {
    async fn send(
        &self,
        request: http_rest_client::OutgoingRequest,
    ) -> Result<http_rest_client::IncomingResponse, String> {
        let incoming = http_rest_transport::IncomingRequest::new(
            request.method().to_owned(),
            request.path().to_owned(),
            request.query().to_owned(),
            request.headers().to_vec(),
            request.body().to_vec(),
        );
        let answered = http_rest_transport::dispatch(self.service, &(), &incoming, &self.handler).await;
        Ok(http_rest_client::IncomingResponse::new(
            answered.status(),
            answered.headers().to_vec(),
            answered.body().to_vec(),
        ))
    }
}
```

A client adapter is the mirror: a small hand-written `Transport` implementation over a real HTTP stack (reqwest in Rust; a service-agnostic `fetch` helper in TypeScript; a service-agnostic `send` implementation in Dart), living with whichever codebase calls the service, doing exactly one job -- carry the plain-terms request across a real connection and hand the plain-terms response back. Timeouts, cancellation, retries, connection handling, mutual TLS and authentication are that adapter's own concerns, never the generated seam's, exactly as `amqp_rpc`'s own adapter split already works: the generator emits the seam, the application implements it against its own libraries, and a workspace that wants to share one adapter across services keeps it in its own crate under its own name -- never a `tixschema`-branded runtime dependency.

**The TypeScript REST server.** `<Service>Schema::ts_http_service()` emits the route table, the plain-terms request and response shapes, a fault handler with the Rust defaults, and `create{Service}HttpDispatcher(impl, onFault?)` -- the TypeScript twin of `ROUTES`, `IncomingRequest`/`OutgoingResponse`, `FaultHandler` and `dispatch` above, doing its own method and path matching, placeholder and query coercion, and message assembly rather than taking an already-matched operation name. It names no framework: the adapter that binds it to a real listener is the hosting application's, exactly as the client adapter above is.

Each bound `header_in` and `part(...)` arrives at the TypeScript implementation as its own argument after the message, in declaration order, the way the Rust trait method already receives it -- `create{Service}Dispatcher` looks it up, decodes it and refuses a missing required one through the same framed fault a bad payload gets, before the implementation is ever called, so an argument read there is as trustworthy as the message is. The REST server coerces each `header_in` value from its header text the way a query parameter is coerced, and hands it to the dispatcher JSON-encoded; a `header_out` or `error_header_out` value comes back from the dispatcher JSON-encoded and is written as header text again. `get_version`'s `byte_range` above reaches its TypeScript implementation this way:

```typescript
// not compiled here
export interface DocumentServiceImpl<Ctx> {
  getVersion(
    ctx: Ctx,
    req: GetVersionRequest,
    byteRange: string | undefined,
  ): Promise<DocumentServiceGetVersionOutcome>;
}
```

The REST server itself keeps no presence check of its own: it passes each `header_in` value it found and, on a service with at least one multipart operation, the request's own `parts` through to the dispatcher, exactly as it passes the assembled message.

<!-- read from tests/service_schema_emitted_client_tests/run_node_http_service.rs, the Node `http` adapter driving the emitted dispatcher -->
```typescript
const dispatch = createConversationClientServiceHttpDispatcher(impl);
const server = createServer(async (req, res) => {
  const chunks = [];
  for await (const chunk of req) chunks.push(chunk);
  const url = new URL(req.url ?? "/", "http://localhost");
  const answered = await dispatch(
    {},
    {
      body: new Uint8Array(Buffer.concat(chunks)),
      headers: Object.entries(req.headers).map(([name, value]) => [name, Array.isArray(value) ? value.join(",") : String(value ?? "")]),
      method: req.method ?? "GET",
      path: url.pathname,
      query: url.search.startsWith("?") ? url.search.slice(1) : url.search,
    },
  );
  res.writeHead(answered.status, Object.fromEntries(answered.headers));
  res.end(Buffer.from(answered.body));
});
```

The same dispatcher runs unchanged behind another web framework -- Express, as a catch-all handler that leaves the matching to the dispatcher:

```typescript
app.use(express.raw({ limit: "8mb", type: () => true }));
app.all(/.*/, async (req, res) => {
  writeResponse(res, await dispatch({ requestId: ++requestCounter }, toRequest(req)));
});
```

Run against seven requests -- a matched call, a declared error, a handler that throws, a one-way call, and three that match no route (a wrong path, a wrong method on a known path, an empty placeholder segment) -- both adapters answered identically: 200 with the success value as the bare JSON body, 404 with the declared error's own body, 500 with a `handler-panic` fault, 204 with nothing, and 404 with an `unknown-operation` fault for each of the last three. What decides each status:

| what answers | who decides it |
| --- | --- |
| the success status | the operation's own `ok_status`, else 204 for nothing to serialize and 200 otherwise |
| a declared error's status | `error_status(Variant = code)`, else 422 for an operation that declares no table |
| a fault's status | 404 unknown route, 400 refused payload, 500 handler threw -- replaceable through the dispatcher's own `onFault` argument |

**The TypeScript, Dart, Swift and Kotlin clients.** `<Service>Schema::ts_http_client()` publishes the `http_rest` half beside `ts_client()`'s AMQP-shaped one: a service-agnostic `{Service}HttpTransport` seam (`send(request): Promise<response>`, both the request and the response carrying `method`/`path`/`query`/`headers` as plain strings and `body` as a `Uint8Array`, a JSON body as its UTF-8 text, plus `parts` on the request where the service declares a multipart operation and `bodyStream: ReadableStream<Uint8Array>` on the response where the service declares a streamed operation -- the platform's own stream type, never a naming of `fetch`), the `{Service}HttpClient` interface, and `create{Service}HttpClient(transport)`. It needs the `zod` feature exactly as `ts_client()` and `ts_service()` do -- outbound validation before a byte goes out is what a `safeParse` against the message's own `$Schema` gives it, and a build without Zod cannot write that check truthfully, so it publishes none of the three rather than one without it. What comes back is parsed the same way: a success body and a declared error body are each read through their own `$Schema` rather than cast, and a body that is no JSON or not the declared type answers the `undeserializable-payload` fault, as every other REST client answers a body that will not decode. `<Service>Schema::dart_http_client()` is the Dart sibling -- the same seam and per-operation client, over the `dart` feature's own generated types and JSON codec rather than Zod, needing no separate outbound check: a Dart message is a real class with `required` constructor parameters, and its `toJson` refuses a value no form serde reads holds ([The Form Each Client Sends](client-forms.md#the-form-each-client-sends)). A message, success or declared error that is not a generated class -- a `String`, an `i64`, a `Vec<T>`, a plain enum -- crosses both Dart clients as its own JSON value, the way a model field of that type does. Unlike the TypeScript seam, the Dart request and response records carry `parts` and `bodyStream` on every service whatever it declares: Dart records are structural, so one shape is what lets one implementation satisfy every service's interface. Where TypeScript answers a reply with an `{ ok, value | error }` union, Dart answers a returned `{Service}{Operation}Result` instead: `<Service>Schema::dart_definition()` publishes every message, fault type and result pair a service needs beside `dart_http_client()`, one sealed class per reply operation carrying three members -- `{Service}{Operation}ResultOk(value)` for the success, `{Service}{Operation}ResultOperation(error)` for the declared error, `{Service}{Operation}ResultFault(fault)` for a fault the operation never declared -- so a reply method answers `Future<{Service}{Operation}Result>` and a caller reads the outcome through an exhaustive `switch` rather than a catch block:

```dart
// not compiled here
final result = await client.window(WindowRequest(conversation_id: '652f1a3b4c5d6e7f8a9b0c1d'));
switch (result) {
  case ConversationClientServiceWindowResultOk(:final value):
    print(value.items);
  case ConversationClientServiceWindowResultOperation(:final error):
    print('declared: $error');
  case ConversationClientServiceWindowResultFault(:final fault):
    print('fault ${fault.kind} in ${fault.operation}');
}
```

A one-way method still answers `Future<void>` and throws the fault-only `{Service}HttpRefusal` -- Dart's own idiom for a `Future`, mirroring exactly how its own one-way AMQP methods already throw, and having no reply arm to carry a fault through instead.

`<Service>Schema::swift_http_client()` is the Swift sibling, over the `swift` feature's own generated `Codable` types -- needing no separate outbound check for the same reason Dart needs none, a Swift message being a `Codable` struct with non-optional stored properties for every required field, whose `encode(to:)` refuses a value no form serde reads holds. A reply method answers Swift's own `Result<Success, Failure>` rather than a returned pair, `Failure` being `{Service}{Operation}Failure`, an enum of `.declared(Error)` for the operation's own declared error and `.fault({Service}Fault)` for one it never declared:

```swift
// adapted from tests/service_schema_emitted_client_tests/run_swift.rs, the http_rest group's own recording transport
let client = ConversationClientServiceHttpClient(transport: recorder)
let outcome = await client.window(WindowRequest(conversationId: "652f1a3b4c5d6e7f8a9b0c1d", limit: 10))
if case .failure(.declared(let error)) = outcome {
  report(error)
}
```

A one-way method still `throws` the fault-only `{Service}Refusal`, shared with `swift_ws_client()`'s own one-way methods rather than named apart from them.

`<Service>Schema::kotlin_http_client()` is the Kotlin sibling, over the `kotlin` feature's own `kotlinx.serialization` types. A reply method answers the same sealed `{Service}{Operation}Result` `kotlin_ws_client()` answers with -- one `sealed interface` per reply operation with `Ok`/`Declared`/`Fault` members, narrowed with `is` exactly as the Dart pair is narrowed with `switch`:

```kotlin
// adapted from tests/service_schema_emitted_client_tests/run_kotlin.rs, the http_rest group's own recording transport
val client = ConversationClientServiceHttpClient(transport)
val outcome = client.window(WindowRequest(conversationId = "652f1a3b4c5d6e7f8a9b0c1d", limit = 10u))
if (outcome is ConversationClientServiceWindowResult.Declared) {
  report(outcome.error)
}
```

A one-way method still throws the fault-only `{Service}Refusal`. A call whose coroutine is cancelled while the transport's `send` waits ends with that `CancellationException`, answering no fault and throwing no refusal.

All four non-Rust backends cover `body = "json"`, `body = "bytes"`, `body = "stream"` and `body = "multipart"` in full, each over the transports that carry it. A `bytes` reply answers the bytes beside their content type: the `http_rest` clients read the bytes off the response's binary body, the TypeScript one as the `Uint8Array` its result declares, and the TypeScript transport-agnostic and `ws_rpc` clients parse the pair of the bytes and their content type the TypeScript dispatcher answers. An operation only `http_rest` carries, a `body = "stream"` one and one with a `part(...)` file part, has a method on the `http_rest` clients alone: the `ws_rpc` clients of every language and the TypeScript transport-agnostic one hold no method for it, and the `ws_rpc` dispatchers and `create{Service}Dispatcher` no handler, since no server of those transports can answer it, as the refusals below state. Each client, the Dart and Kotlin handlers and `{Service}Impl` name the operations left out in their doc comments. `create{Service}Dispatcher` takes the implementation without those members, `Omit<{Service}Impl<Ctx>, "getContent">`, and `create{Service}HttpDispatcher` answers them itself. A streamed operation's TypeScript client answers `{ contentRange: string | undefined; contentType: string; body: ReadableStream<Uint8Array> }` -- `contentRange` left `undefined` at the operation's own `ok_status`, read back off the response ahead of naming the body and set to the range text at `206`, `contentType` read back as a `bytes` reply's is -- off the seam's own `bodyStream` field; its Dart client answers the same fields as a `({String? contentRange, String contentType, Stream<List<int>> body})` record off `bodyStream` there too; its Swift client answers them as `(contentRange: String?, contentType: String, body: AsyncThrowingStream<Data, Error>)` off `bodyStream` there too; its Kotlin client answers them wrapped in an aux `{Result}Streamed(contentRange, contentType, body: Flow<ByteArray>)` type off `bodyStream` there too. A declared `header_out` composes onto either answer exactly as it does for `json` and `bytes`. A multipart operation's TypeScript client builds `parts` from the message's own fields and the declared `part` bindings; its Dart client builds the same list, the file handles crossing as `dynamic` through the same path an unknown type already renders by; its Swift client builds the same list, the file handles crossing as `any Sendable`; its Kotlin client builds the same list, the file handles crossing as `Any?`.

##### OpenAPI

`#[service_schema(transports = ["http_rest"], openapi)]` asks for the service's OpenAPI 3.1 document. `openapi(title = "...", path = "...")` writes either key, or both: `title` is the document's `info.title`, the service's name where none is written, and `path` is the route the dispatchers answer the document at, `/openapi.json` where none is written.

```rust,ignore
use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub title: String,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum DocumentError {
    NotFound,
}

#[service_schema(transports = ["http_rest"], openapi(title = "Documents API"))]
pub trait DocumentService<Ctx> {
    #[service_schema_op(http(
        method = "GET",
        path = "/documents/{document_id}",
        error_status(NotFound = 404)
    ))]
    async fn get_document(&self, ctx: &Ctx, document_id: String)
    -> Result<Document, DocumentError>;
}
```

`DocumentServiceSchema::openapi()` answers the document as a `serde_json::Value`, built at run time in the crate that declares the service. It holds every operation's path and method; its path, query and header parameters; its request body; and each status it answers at, with the body and the headers of each: the success at `ok_status`, `206` beside it for a `body = "stream"` operation, each declared error's status, and the `400`, `404` and `500` faults. Every type the operations name is written once under `components.schemas` and referenced as `#/components/schemas/<Name>`, under the name it is published under. A type one of those holds is a component where the document references it two or more times, and is written in place where once, as [Nested Types](nested-recursive-and-aliased-types.md#nested-types) says of `$defs`. `info.version` is the declaring crate's version. An excerpt of the document above:

```json
{
  "openapi": "3.1.0",
  "info": { "title": "Documents API", "version": "0.1.0" },
  "paths": {
    "/documents/{document_id}": {
      "get": {
        "operationId": "get-document",
        "parameters": [
          { "in": "path", "name": "document_id", "required": true, "schema": { "type": "string" } }
        ],
        "responses": {
          "200": {
            "content": { "application/json": { "schema": { "$ref": "#/components/schemas/Document" } } },
            "description": "The operation's answer."
          },
          "404": {
            "content": {
              "application/json": {
                "schema": {
                  "anyOf": [
                    { "$ref": "#/components/schemas/DocumentError" },
                    { "$ref": "#/components/schemas/DocumentServiceFaultFields" }
                  ]
                }
              }
            },
            "description": "A declared error: `NotFound`. A fault: no route answers the request."
          }
        }
      }
    }
  }
}
```

Every model type an operation names lists `"jsonschema"` in its `enable` list, because the document reads each type's JSON Schema; the messages and the fault fields the service macro declares itself list it on their own. A type that does not list it stops the build where the operation names it:

```text
error[E0599]: no associated function or constant named `JSON_SCHEMA_NAME` found for struct `balance_schema::Schema` in the current scope
```

Both `http_rest` dispatchers, the Rust `dispatch` and the TypeScript `create{Service}HttpDispatcher`, answer `GET` at the document's path with `200`, `content-type: application/json` and the document as the body, ahead of every route. Any other method at that path answers as it would without `openapi`, and so does every request to a service that does not ask for it. `ts_http_service()` writes the text of the document `openapi()` builds into the text it answers, at run time, as a string literal, and the TypeScript dispatcher answers that text as it is: both dispatchers answer the same bytes, so a float written `1.0` and an integer past 2^53 keep the digits Rust wrote.

The Rust dispatcher reads the document from `$crate::<Service>Schema`, so a service that asks for `openapi` and is declared in a submodule re-exports its `<Service>Schema` at the crate root, beside its generated module and its trait. The declaration refuses a missing re-export in tixschema's own words, whether or not a dispatcher is placed:

```text
error[E0277]: `DocumentServiceSchema` is not re-exported at the crate root
  --> src/contract.rs:18:11
   |
17 | #[service_schema(transports = ["http_rest"], openapi)]
   | ------------------------------------------------------ in this attribute macro expansion
18 | pub trait DocumentService<Ctx> {
   |           ^^^^^^^^^^^^^^^ this service asks for `openapi`, and its dispatcher reads `$crate::DocumentServiceSchema`
   ...
   = note: add `pub use` of `DocumentServiceSchema` at the crate root, beside the service's module and trait
```

`openapi` is refused, and the build stops, in six cases.

A service that does not ask for `http_rest`:

```text
error: service_schema: `openapi` describes the `http_rest` transport, and this service does not ask for it
              add "http_rest" to `transports`, or remove `openapi`
```

A `path` that does not start with `/`:

```text
error: service_schema: `openapi(path = "...")` is a route, so it starts with `/`
```

A path an operation of the service answers with `GET`, written or the default. A route with a placeholder answers every path it matches, so `GET /{document_id}` answers `/openapi.json`:

```text
error: service_schema: operation `get_document` already answers `GET /openapi.json`, so the OpenAPI document cannot be served there
              write another `path` in `openapi(...)`
```

A key other than `title` and `path`:

```text
error: service_schema: unknown `openapi` argument
              the arguments are `title` and `path`, each written `name = "..."`
```

A key written twice:

```text
error: service_schema: `path` is written twice in `openapi(...)`
              write it once
```

`openapi` written with a value, as `openapi = true`:

```text
error: service_schema: `openapi` takes no value
              write `openapi`, or `openapi(title = "...", path = "...")`
```
