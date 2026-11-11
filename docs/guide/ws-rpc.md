#### The `ws_rpc` Transport

`#[service_schema(transports = ["ws_rpc"])]` serves a service over one WebSocket as JSON text frames -- the same request, notify and reply terms `amqp_rpc` already answers through, carried over a socket instead of a queue. A service that only answers calls asks for it alone; a service the server calls instead -- pushing an event to whichever connection wants it -- asks for it too, notify travelling either direction over the same wire. The pair below is what a browser dashboard and its backend actually share: the browser calls `Ledger`, and the server pushes `LedgerEvents` back down the same connection.

```rust,ignore
/// What the browser or the Flutter app calls on the server.
#[service_schema(transports = ["ws_rpc"])]
pub trait Ledger<Ctx> {
    async fn create_transaction(
        &self,
        ctx: &Ctx,
        req: CreateTransactionRequest,
    ) -> Result<Transaction, CreateError>;

    async fn list_transactions(
        &self,
        ctx: &Ctx,
        req: ListTransactionsRequest,
    ) -> Result<TransactionList, ListError>;

    /// The app tells the server it showed a transaction; nothing comes back.
    #[service_schema_op(one_way)]
    async fn mark_seen(&self, ctx: &Ctx, req: MarkSeen);
}

/// What the server calls on the browser or the Flutter app.
#[service_schema(transports = ["ws_rpc"])]
pub trait LedgerEvents<Ctx> {
    #[service_schema_op(one_way)]
    async fn transaction_posted(&self, ctx: &Ctx, ev: TransactionPosted);
}
```

**The wire.** One JSON object per text frame, `kind` naming which:

| key | present on | value |
| --- | --- | --- |
| `kind` | every frame | `"request"`, `"notify"`, `"reply"`, `"ping"` or `"pong"` |
| `id` | `request`, `reply` | the id the caller's own request counter minted; a `reply` carries back whatever `id` its `request` named |
| `service` | `request`, `notify`, `reply` | the trait's own name, exactly as declared -- carried beside the payload rather than folded into it, since more than one service's frames can share one socket and no message type has to reserve a key for routing |
| `operation` | `request`, `notify` | the operation's own wire name |
| `payload` | `request`, `notify` | the operation's message, JSON-encoded |
| `headers` | `request`/`notify` claiming a `header_in`, `reply` claiming a `header_out` | one JSON-encoded entry per bound header -- the same encoding `header_in`/`header_out` already ride on `amqp_rpc`'s own headers table -- left off entirely where none are bound |
| `ok` | `reply` | `true` for a success, `false` for a declared error or a fault |
| `value` | `reply`, `ok: true` | the success value, or `null` for a unit success or a one-way operation answered through `request` |
| `error` | `reply`, `ok: false` | the operation's own declared error, or `{ isServiceFault: true, fault: <ServiceFault> }` |

`ping` and `pong` carry no field beside `kind`. A `ping` is sent by whichever side wants to probe liveness -- the generated TypeScript and Dart transports run their own heartbeat on the calling side; a hand-rolled interval does the same on the serving side, there being no `FrameSession` there to run one through -- and is answered with a `pong` by whichever side reads it: `answer` on the dispatcher, `FrameSession::deliver` on the client, and each generated transport's own socket listener.

**What the dispatcher macro emits.** `{service}_ws_rpc_dispatcher!()` emits everything every transport's dispatcher publishes -- `IncomingMessage`, `Reply`, `dispatch` and the panic guard and refused-payload reader, described under [Transports](services.md#transports) above -- plus this transport's own pieces: `SERVICE`, the constant this dispatcher's own `answer` matches a frame's own `service` field against, so a frame belonging to another service on the same connection is left alone -- the client macro publishes an identical copy of its own, which `FrameSession::deliver` matches the same way; `Frame`, the enum one inbound frame decodes into -- `Request`, `Notify` and `Ping` on this side, a `Reply` or a `Pong` decoding to `Frame::Ignored` rather than a refusal, since it belongs to the client macro's own half of the wire; `FrameReply`, the `Reply` a request frame is answered through, its `send` published only where the service declares at least one operation that answers -- a one-way-only service's `FrameReply` publishes `fault` alone; and `answer(text, &svc, &ctx) -> Option<String>`, the one call an adapter makes per inbound text frame: a request naming this service is dispatched and its reply frame returned, a notify naming this service is dispatched and nothing returned, a ping is answered with a pong, and anything else -- another service's frame, a reply, text this transport does not recognise -- answers `None`.

**What the client macro emits.** `{service}_ws_rpc_client!()` emits everything every transport's client publishes -- `Transport`, `{Service}Client`, the fault mirror and the envelope readers, described under [Transports](services.md#transports) above -- plus its own copy of the frame codec (`Frame` here reading `Reply`, `Ping` and `Pong` alone, a `Request` or a `Notify` decoding to `Frame::Ignored`), `ping_frame` -- published by the client macro alone, a dispatcher answering a ping rather than ever sending one -- and `pong_frame`, published by both; `request_frame` and `notify_frame`; and two ways to send them: `FrameWriter`, a one-way `Transport` over any function that puts a text frame on the wire, whose `request` always answers `Err` -- a writer keeps no correlation map to resolve one against; and `FrameSession`, a request-and-reply `Transport` over one socket in `core` and `std` alone, a correlation map keyed on an id from an atomic counter and settled by `deliver`, `close` failing everything still waiting once the socket goes away. A caller's own adapter shrinks to one send function plus one call to `deliver` per inbound frame.

**Placement follows the same doctrine as `amqp_rpc`'s three macros and `http_rest`'s two -- see "Where each goes, and why placement is fussy" above:** each invocation in a module of its own file, the `mod` declaration above the crate's `use` items, invoked by name (not by path) from inside the declaring crate:

```text
// the crate that serves it -- src/lib.rs
mod ledger_transport;

// src/ledger_transport.rs
declaring_crate::ledger_ws_rpc_dispatcher!();
```

```text
// a crate that calls it -- src/lib.rs
mod ledger_client;

// src/ledger_client.rs
use declaring_crate::{
    CreateError, CreateTransactionRequest, ListError, ListTransactionsRequest, MarkSeen,
    Transaction, TransactionList,
};

declaring_crate::ledger_ws_rpc_client!();
```

The dispatcher's crate names `serde`, `serde_json`, `serde_path_to_error` and `tracing` in its own manifest, the panic guard logging through `tracing`; the client's crate names `serde`, `serde_json` and `serde_path_to_error` and not `tracing` -- nothing in a client catches a panic -- and needs no runtime crate beyond those three for `FrameSession`'s own correlation map either, `core` and `std` being enough for a mutex and a waker.

**Refusals.** `amqp_rpc` and `ws_rpc` both answer every reply inside `{ ok, value, error }`, `ws_rpc` over its own frame rather than a queue message; `http_rest` does not, which is exactly why `body = "stream"` and `part(...)`-claimed file parts are `http_rest`-only grammar, and only meaningful on an operation that also carries an `http(...)` binding. A service that asks for `http_rest` beside `ws_rpc` (or `amqp_rpc`) on such an operation is refused at the declaration, naming the operation and the transport: a `body = "stream"` operation's `StreamedAnswer` and a `part(...)`-claimed file part are both a `Box<dyn BodySource + Send>`, publishing neither `Serialize` nor `Deserialize`, and neither has a carrier inside the envelope the other transport answers with, nor inside the tool result `mcp` answers with:

```rust,ignore
#[service_schema(transports = ["ws_rpc", "http_rest"])]
pub trait ContentService<Ctx> {
    #[service_schema_op(http(
        method = "GET",
        path = "/documents/{document_id}/content",
        body = "stream",
        error_status(NotFound = 404),
    ))]
    async fn get_content(
        &self,
        ctx: &Ctx,
        document_id: String,
    ) -> Result<content_service_schema::StreamedAnswer, ContentError>;
}
```

```text
service_schema: operation `get_content` declares a streamed body, and this service also declares `ws_rpc`
       a streamed body has no carrier inside the `{ ok, value, error }` envelope `ws_rpc` answers with - drop them from `transports`, or drop `body = "stream"` from this operation
```

A `part(...)` binding under `body = "multipart"` is refused the same way:

```rust,ignore
#[service_schema(transports = ["ws_rpc", "http_rest"])]
pub trait UploadService<Ctx> {
    #[service_schema_op(http(
        method = "POST",
        path = "/documents",
        body = "multipart",
        part("file" = attachment),
        error_status(TooLarge = 413),
    ))]
    async fn upload_document(
        &self,
        ctx: &Ctx,
        title: String,
        attachment: Box<dyn upload_service_schema::BodySource + Send>,
    ) -> Result<UploadResponse, UploadError>;
}
```

```text
service_schema: operation `upload_document` declares a multipart file part, and this service also declares `ws_rpc`
       a file part has no carrier inside the `{ ok, value, error }` envelope `ws_rpc` answers with - drop them from `transports`, or drop the `part(...)` binding and `body = "multipart"` from this operation
```

Both are checked at the declaration, ahead of every transport's own macros, so the mismatch never gets as far as a dispatcher whose `Reply::send` would not compile.

The emitted clients and dispatchers read the same question, whatever `transports` names: an operation with a streamed body or a file part has a method on the `http_rest` clients alone. The `ws_rpc` client of each language, the TypeScript transport-agnostic client, the `ws_rpc` dispatcher attachments and `create{Service}Dispatcher` hold no method and no handler for it, and name no `StreamedAnswer`; a service of no other operation has a `ws_rpc` client of no method. A call of it there is refused by the compiler, as the Rust declaration refuses the transport.

**The TypeScript client and dispatcher attachment.** `<Service>Schema::ts_ws_client()` publishes the socket half beside `ts_client()`'s AMQP-shaped one: a `{Service}WsSocket` seam naming the four members every platform `WebSocket` already has (`send`, `close`, `addEventListener`/`removeEventListener` for `"message"` and `"close"`), so `new WebSocket(url)` plugs into `create{Service}WsTransport(socket, options)` with nothing written in between -- nothing generated here names `WebSocket` itself. `options.heartbeat` defaults to `{ intervalMs: 30_000, timeoutMs: 10_000 }`; `heartbeat: false` turns the probe off entirely. The returned transport owns that heartbeat and a per-socket correlation map, and checks every reply against the operation's own declared success or error schema before a caller sees it -- a mismatch folds into a `failed-validation` fault naming the first offending key, a unit success (having no schema to check) normalizes to `{ ok: true, value: undefined }` whatever `value` the reply carried, and a header tuple is checked on its body alone -- and settles every request still waiting with a `transport-failure` fault on a missed `pong` or a closed socket, so no caller hangs. Binding is per socket: two request-and-reply services sharing one connection each call `create{Service}WsTransport(socket)` on their own, and each runs its own heartbeat.

```typescript
import { createProbeServiceClient, createProbeServiceWsTransport } from "./bundle";

declare const socket: WebSocket;

export async function read(): Promise<string> {
  const client = createProbeServiceClient(createProbeServiceWsTransport(socket));
  const answered = await client.getBalance({ customer_id: "acme" });
  if (answered.ok) {
    return `${answered.value.invoices}`;
  }
  return "failed";
}
```

Headers ride the frame's own `headers` object both ways: the transport writes the seam's JSON-encoded pairs into a `request` or `notify` frame as the JSON values they encode, leaving the key off where there are none, and hands a `reply` frame's `headers` back beside the envelope -- what the Rust `headers_table` and `headers_of` do.

`<Service>Schema::ts_ws_service()` publishes the other half: `attach{Service}WsDispatcher(socket, ctx, impl, onFault)` reads every `notify` and `request` frame naming the service off `socket`, drives `impl` through the generated dispatcher with the frame's own `headers`, and answers a `request` with a `reply` frame carrying the headers the dispatcher answered -- `{ ok: true, value: null }` where the dispatcher answered nothing, which is what lets a `request` naming a one-way operation still get an answer instead of leaving its caller waiting. `onFault` is required rather than optional: a `notify` that fails inside the dispatcher has nobody waiting on a reply to carry the fault, so it has nowhere else to go.

```typescript
import {
  attachProbeServiceWsDispatcher,
  type ProbeServiceVoidInvoiceOutcome,
  type ProbeServiceFaultKind,
  type ProbeServiceGetBalanceOutcome,
  type ProbeServiceSettleOutcome,
  type ProbeServiceSweepOutcome,
} from "./bundle";

type ProbeContext = { loggerName: string };

declare const socket: WebSocket;

attachProbeServiceWsDispatcher<ProbeContext>(socket, { loggerName: "probe" }, {
  async applyBundle(ctx, req): Promise<void> {
    void `${ctx.loggerName}:${req.customerId}:${req.bundleId}`;
  },
  async voidInvoice(ctx, req): Promise<ProbeServiceVoidInvoiceOutcome> {
    void `${ctx.loggerName}:${req.customerId}:${req.invoiceId}`;
    return { ok: false, error: { errorCode: "conflict" } };
  },
  async getBalance(ctx, req): Promise<ProbeServiceGetBalanceOutcome> {
    void `${ctx.loggerName}:${req.customer_id}`;
    return { ok: true, value: { invoices: 1 } };
  },
  async settle(ctx, req): Promise<ProbeServiceSettleOutcome> {
    void `${ctx.loggerName}:${req.customer_id}`;
    return { ok: true, value: { applied: true } };
  },
  async sweep(ctx, req): Promise<ProbeServiceSweepOutcome> {
    void `${ctx.loggerName}:${Object.keys(req).length}`;
    return { ok: false, error: { errorCode: "db-error" } };
  },
}, (fault) => {
  const kind: ProbeServiceFaultKind = fault.kind;
  void kind;
});
```

Both need the `zod` feature exactly as `ts_client()` and `ts_service()` do -- checking an inbound reply against a declared schema is what a `safeParse` against the message's own `$Schema` gives them, and a build without Zod cannot write that check truthfully, so a build with no Zod surface publishes neither.

**The Node WebSocket server.** `<Service>Schema::ts_ws_server()` publishes `create{Service}WsServer(impl, contextFor, options)`, which accepts connections rather than attaching to one socket somebody else already opened. `accept(socket)` builds a context per connection through the caller's own synchronous `contextFor`, wraps the existing `attach{Service}WsDispatcher` around a write-guarded view of the socket, and returns a `{Service}WsConnection` carrying an `AbortSignal` aborted when the connection closes, a `close()`, and `share(attach)` for handing the same guarded socket and signal to a second service's own attachment:

<!-- read from tests/service_schema_emitted_client_tests/run_node_ws_server.rs, the server accepting connections from the `ws` package -->
```typescript
const server = createConversationClientServiceWsServer(
  impl,
  (connection) => ({ n: ++n, signal: connection.signal }),
  { onFault: () => {} },
);
const wss = new WebSocketServer({ port: 0 });
wss.on("connection", (socket) => server.accept(socket));
```

`options.heartbeat` defaults to the client's own `{ intervalMs: 30_000, timeoutMs: 10_000 }`: the idle timer arms only once no frame of any kind has arrived for `intervalMs`, a `ping` then goes out, and the connection closes if no `pong` answers within `timeoutMs` -- a busy connection is never probed. The accepted-socket type requires an `error` listener the single-socket seam does not carry, because the `ws` package throws an unhandled `error` event out of `emit`; `options.onSocketError` is called, and the connection closed, when one fires. A second service reaches the same socket through the connection's own hook rather than through a second listener:

<!-- read from tests/service_schema_emitted_client_tests/run_node_ws_server.rs, scenario 8 -->
```typescript
wss.on("connection", (socket) => {
  connection = server.accept(socket);
  connection.share(attachInventoryServiceStub);
});
```

Each service answers only the frames naming it, exactly one `pong` answers one `ping` because the accepting server owns the heartbeat, and `share` throws once the connection has closed.

**The Dart client.** `<Service>Schema::dart_ws_client()` is the Dart sibling, over the `dart` feature's own generated types and JSON codec rather than Zod -- needing no separate outbound check, a Dart message being a real class with `required` constructor parameters whose `toJson` refuses a value no form serde reads holds. `{Service}WsTransport` takes the sink and stream a `WebSocketChannel` already exposes (`StreamSink<dynamic>`/`Stream<dynamic>`) rather than naming `web_socket_channel` or `dart:io` itself, plus an optional `{Service}WsHeartbeat` (`{Service}WsHeartbeat.off()` turning liveness checking off, the same thing `heartbeat: false` does in TypeScript). `WebSocketChannel.stream` is single-subscription, so the transport is the only thing that ever calls `.listen` on it; an attachment for a second, browser-implemented service sharing the same connection reaches the transport's own `frames` instead -- a record pairing `inbound` (every frame the transport did not itself correlate to a pending request) with `send`, structurally identical for every service, so `attach{Service}WsDispatcher` composes over it without naming the calling service's transport class at all.

`{Service}WsClient` answers a returned `{Service}{Operation}Result` rather than throwing, exactly as `dart_http_client()` now does: a reply method answers `Future<{Service}{Operation}Result>` from the same sealed pair `<Service>Schema::dart_definition()` publishes, its `Ok`/`Operation`/`Fault` members carrying the success, the declared error and an undeclared fault respectively. A one-way method still answers `Future<void>` and throws the fault-only `{Service}WsRefusal`, having no reply arm to carry a fault through. A reply that will not decode answers the pair's `Fault` member carrying `failedValidation` rather than `undeserializablePayload` -- `ws_rpc` has no status line to draw that distinction with, so a reply failing the check the transport was always going to make against the declared type reads the same as any other failed validation. `{Service}Handlers` carries one handler per declared operation, and `attach{Service}WsDispatcher(frames, ctx, handlers, onFault: ...)` is the dispatcher attachment's own mirror: a decode failure, an unknown operation, or a handler throwing anything but its own declared error reaches `onFault` rather than vanishing, and a request left waiting on any of them is answered with the fault instead of hanging.

A `header_in` binding is one more argument after `req`, a `None` value still sent as JSON `null` rather than left out. A `header_out`/`error_header_out` tuple is rejoined on the way back: an absent optional element reads `null`, a missing or undecodable required one answers the pair's `Fault` member. `{Service}Handlers` takes the same `header_in` arguments, and `attach{Service}WsDispatcher` splits a header-tuple answer the same way it reads one -- a `null` element not written.

```dart
// not compiled here
final transport = LedgerWsTransport(sink: channel.sink, stream: channel.stream);
final ledger = LedgerWsClient(transport);
final created = await ledger.createTransaction(
  CreateTransactionRequest(account_id: 'acc-1', amount_cents: 1250, memo: 'coffee'),
);

attachLedgerEventsWsDispatcher(
  transport.frames,
  screen,
  LedgerEventsHandlers(
    transactionPosted: (ctx, ev) async => ctx.render(ev.transaction),
  ),
  onFault: (fault) => report(fault),
);
```

**The Swift client.** `<Service>Schema::swift_ws_client()` is the Swift sibling: `{Service}WsTransport(socket:, options:)` is an `actor` that owns the socket -- a `{Service}WsSocket` protocol of four members (`send`, `close`, `onMessage`, `onClose`), none naming a networking type, so an app's own `URLSessionWebSocketTask` wrapper satisfies it without an adapter -- correlates a `request` to its reply, answers an inbound `ping` with one `pong`, and runs its own heartbeat (`options.heartbeat` defaulting to `.init(intervalMs: 30_000, timeoutMs: 10_000)`, `nil` turning it off). A reply method answers `Result<Success, Failure>` exactly as `swift_http_client()`'s own methods do, rather than a `{Service}{Operation}Result` pair -- Swift's own `Result` already carries the two arms. `{Service}WsClient` is a `typealias` of `{Service}WsTransport`, so a caller constructs one and calls its methods directly, mirroring `{Service}HttpClient`'s own shape rather than wrapping a second object around it. A one-way method still `throws` the shared `{Service}Refusal`, having no reply arm to carry a fault through.

```swift
// adapted from tests/service_schema_emitted_client_tests/run_swift.rs, the ws_rpc group's own driver
let transport = ConversationClientServiceWsTransport(socket: socket, options: .init(heartbeat: nil))
let outcome = await transport.window(WindowRequest(conversationId: "652f1a3b4c5d6e7f8a9b0c1d", limit: nil))
if case .failure(.fault(let fault)) = outcome {
  report(fault)
}
```

A method takes one argument per `header_in` binding after the message, exactly as `swift_http_client()`'s own methods do; an `Optional` holding `nil` is sent as JSON `null` rather than left out. A `header_out` or `error_header_out` reply answers the declared tuple: an absent optional element reads `nil`, and a missing required one answers a `failedValidation` fault.

**The Kotlin client and mini server.** `<Service>Schema::kotlin_ws_client()` publishes `{Service}WsTransport(socket, options)` under the same preferred ownership shape: it correlates a `request` to its reply through a `CompletableDeferred`, answers an inbound `ping` with one `pong`, and runs its own heartbeat on a `CoroutineScope` it owns -- closing the transport cancels that scope, and with it every dispatcher reading the transport's own `frames`. `{Service}WsClient(transport)` wraps it and answers a reply method with the same sealed `{Service}{Operation}Result` `kotlin_http_client()` publishes; a one-way method still throws the fault-only `{Service}WsRefusal`, having no reply arm to carry a fault through. Where the service declares a `header_in`, a client method takes one parameter per binding after `req`, sent in the frame's own `headers` (`None` as JSON `null`); a `header_out`/`error_header_out` reply is rejoined into the `{Result}Value`/`{Result}Declared` class `kotlin_http_client()` publishes, an absent optional element reading `null` and a missing or undecodable required one answering a `failed-validation` fault naming the header.

Kotlin is the one target that also emits a server-side surface for this transport: `{Service}WsFrames` (`inbound`, `send`, `scope`) is the structural seam `attach{Service}WsDispatcher(frames, handlers, onFault)` -- the mini server -- dispatches over rather than the socket itself, so a second service sharing the same connection reaches the same frames without naming the first service's transport class at all. A handler takes the same `header_in` parameters the client sends and, for a header tuple, answers the same `{Result}Value`/`{Result}Declared` class; the attachment writes its `value`/`error` and each header under its own name in the reply's `headers`, a `null` element left out. The attachment's own `share(attach)` hands a second service's `attach*WsDispatcher` the transport's `send` and `scope`, and `detach()` tears down every service it shared the socket with along with itself:

```kotlin
// adapted from tests/service_schema_emitted_client_tests/run_kotlin.rs, the mini-server group's own driver
val serverTransport = ConversationClientServiceWsTransport(serverSocket, ConversationClientServiceWsOptions(heartbeat = null))
val attachment = attachConversationClientServiceWsDispatcher(serverTransport.frames, Handlers, onFault = { faults.add(it) })

attachment.share { send, scope ->
    val pulseFrames = PulseClientServiceWsFrames(serverTransport.frames.inbound, send, scope)
    attachPulseClientServiceWsDispatcher(pulseFrames, PulseHandlers, onFault = {})
    ({ pulseDetachRan = true })
}
```

**The field a refused message names.** The Dart and Kotlin attachments answer a message their read refuses with `failed-validation` and the field the Rust `ws_rpc` dispatcher names, and call no handler, and the Dart, Kotlin and Swift `ws_rpc` clients answer a reply their read refuses with the same fault and field. The field is the path of the value refused, an item's index, a map's key and a variant's name or content key among its steps, then the key the read names as unknown or missing; a value `validate()` refuses names the place its message opens with. With `pub struct BellRope { pub length: u32 }` and `pub struct BellToll { pub rope: BellRope, pub strokes: u32 }`, `{"rope":{"length":3},"strokes":2,"stray":1}` answers the field `stray`, `{"rope":{"length":3,"stray":1},"strokes":2}` the field `rope.stray`, and `{"rope":{"length":3}}` the field `strokes`. A message whose `bells: Vec<CarillonBell>` holds `[{"weight":3},{"weight":"x"}]` answers `bells.1.weight`, and one whose externally tagged `peal` holds `{"Swung":{"arc":true}}` answers `peal.Swung.arc`. The Dart reads join the path [their refusal carries](dart.md#the-dart-read-refuses-what-validate-refuses). The Kotlin reads decode the message from its text, where kotlinx states the path of the value it refuses, and each serializer tixschema writes places a refusal at the path it read the value at ([the Kotlin read](kotlin.md#the-read-in-the-kotlin-client)). The Swift client names the steps of the coding path of the `DecodingError`, and reads an integer Foundation refuses with no coding path, one outside the range of its type or with a fraction, again at its own path, so `-1` read as a `u32` at `towers.north.weight` answers that field. serde reads a `#[serde(flatten)]` member from the keys its holder buffers, and names the holder for a value inside the member it refuses, as every surface does: `{"frame":{"span":1,"oak":"x"}}`, where `oak` belongs to the flattened member, answers the field `frame`. A key the member misses keeps its step, `{"frame":{"span":1}}` answering `frame.oak`, and so does a value `validate()` refuses inside it. A key of a map that a brand's `check` refuses is named under its map: with `pitches: BTreeMap<CarillonPitch, u32>` over a `#[serde(transparent)]` brand of a `u32` whose check refuses a pitch above 12, `{"pitches":{"13":1}}` answers `pitches.13`. A value refused in an arm of a `Result` is named under its arm, `{"tally":{"Ok":"x"}}` answering `tally.Ok` for `tally: Result<u32, String>`, and a key no arm names at the `Result`, `{"tally":{"Maybe":1}}` answering `tally`. Three answers differ from the Rust dispatcher's. The Dart attachment and client name such a key at its map alone, `pitches`, and such a value at the `Result` alone, `tally`; the Swift client reads the key; and Kotlin names a value refused in a map keyed by a brand under the key as the brand's `toString()` writes it, `{"pitches":{"3":-1}}` answering `pitches.CarillonPitch(value=3)`, which is the step kotlinx states.

**What a developer types.** Five seats share the pair declared above, each typing only what its own role needs:

1. **The crate that declares them** -- the two traits shown at the top of this section, and nothing transport-shaped beside the macro invocations every other seat places. `ServiceFault` and the rest of what [What a Service Generates](services.md#what-a-service-generates) lists are generated once, from the declaration alone.
2. **The crate that serves `Ledger` and pushes `LedgerEvents`** -- invokes both macros, `ledger_ws_rpc_dispatcher!()` to answer inbound calls and `ledger_events_ws_rpc_client!()` to push over the same connection, and reads every frame off the socket through `answer`, forwarding whatever it returns back onto the wire. The reference loop, over axum, is `ledger_socket` in `tests/service_schema_ws_rpc_live_tests/tests.rs`:

   ```rust,ignore
   async fn ledger_socket(
       ws: WebSocketUpgrade,
       State(back_end): State<Arc<LedgerBackEnd>>,
   ) -> impl IntoResponse {
       ws.on_upgrade(move |socket| async move {
           use axum::extract::ws::Message;

           let (mut sink, mut stream) = socket.split();
           let (out, mut outbox) = mpsc::channel::<String>(64);
           tokio::spawn(async move {
               while let Some(text) = outbox.recv().await {
                   let _: Result<(), _> = sink.send(Message::Text(text.into())).await;
               }
           });
           let push = out.clone();
           let session = Session {
               user_id: "u-1".to_owned(),
               events: LedgerEventsClient::new(ledger_events_ws::FrameWriter::new(move |text| {
                   let sender = push.clone();
                   async move {
                       sender
                           .send(text)
                           .await
                           .map_err(|refused| refused.to_string())
                   }
               })),
           };
           while let Some(Ok(message)) = stream.next().await {
               let Message::Text(text) = message else {
                   continue;
               };
               if let Some(reply) = ledger_ws::answer(&text, &*back_end, &session).await {
                   let _: Result<(), _> = out.send(reply).await;
               }
           }
       })
   }
   ```

3. **A crate that calls `Ledger` and receives `LedgerEvents` pushes** -- invokes the other two macros, `ledger_ws_rpc_client!()` and `ledger_events_ws_rpc_dispatcher!()`, and drives them with a matching loop. The reference, over tokio-tungstenite, is `connect` in the same file:

   ```rust,ignore
   async fn connect(
       url: &str,
   ) -> (
       LedgerClient<FrameSession>,
       mpsc::Receiver<TransactionPosted>,
   ) {
       let (socket, _response) = connect_async(url).await.unwrap();
       let (mut sink, mut stream) = socket.split();
       let (out, mut outbox) = mpsc::channel::<String>(64);
       tokio::spawn(async move {
           while let Some(text) = outbox.recv().await {
               let _: Result<(), _> = sink.send(Message::Text(text.into())).await;
           }
       });

       let session = FrameSession::new(move |text| {
           let sender = out.clone();
           async move {
               sender
                   .send(text)
                   .await
                   .map_err(|refused| refused.to_string())
           }
       });
       let ledger = LedgerClient::new(session.clone());
       let reader = session;
       let (posted_out, posted_in) = mpsc::channel(16);
       let screen = Screen(posted_out);
       tokio::spawn(async move {
           while let Some(Ok(message)) = stream.next().await {
               let Message::Text(text) = message else {
                   continue;
               };
               reader.deliver(&text).await; // replies to my requests, and pongs
               screen_ws::answer(&text, &screen, &()).await; // pushes from the server
           }
           reader.close("the socket closed");
       });

       (ledger, posted_in)
   }
   ```

   Both loops read every frame off the socket and act on text frames alone (`let Message::Text(text) = message else { continue };`); a binary frame is left unread, there being no binary framing this transport defines.
4. **The browser** -- `createLedgerWsTransport(socket)` to call `Ledger`, `attachLedgerEventsWsDispatcher(socket, ctx, impl, onFault)` to answer `LedgerEvents` pushes on the same socket, exactly as shown above.
5. **The Flutter app** -- `LedgerWsTransport(sink: ..., stream: ...)` to call `Ledger`, `attachLedgerEventsWsDispatcher(transport.frames, ctx, handlers, onFault: ...)` to answer `LedgerEvents` pushes on the same connection, exactly as shown above.

There is no subscription operation, no topic and no broker: a push travels exactly as far as the one connection it was written against, over the `LedgerEventsClient` the serving crate built from that connection's own `FrameWriter` -- `Session` above holds one `events` client per connection, not a registry of every connection there is. "Tell this user" is nowhere generated, because it is the application's own question to answer: a registry from a user id to the connections currently open for it, built and kept by whatever code accepts the upgrade, exactly as `Session.user_id` above stands in for one connection's own identity. `ws_rpc` gives that registry a wire to push over once it has decided who to tell; deciding who that is stays the application's.
