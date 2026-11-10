### Services

`#[service_schema()]` declares a service as a trait: one trait, one context, and one message in and one message out per operation. Failing to implement an operation is a compile error, which is the whole reason the construct is a trait rather than a table of handlers.

**Services need the `serde` feature.** A service's messages derive `Serialize` and `Deserialize`, a dispatcher reads payloads through `serde_json`, and the camelCase and kebab-case keys it puts on the wire come from serde attributes the macro writes. A build without the feature is refused at the declaration rather than left to publish TypeScript that names Rust fields the wire never carries:

```text
service_schema: a service needs tixschema's `serde` feature, and this build does not have it
       without it the TypeScript a service publishes names Rust fields rather than the camelCase
       and kebab-case keys its own dispatcher writes, so the two halves disagree about the wire
       add `features = ["serde"]` to the tixschema dependency in Cargo.toml
```

**Only the `typescript` feature publishes TypeScript at all.** `<Service>Schema::ts_definition()`
and every other `ts_*` accessor live behind it, independently of `dart`, `swift` and `kotlin`. A
build with `serde` and `dart` alone publishes `dart_definition()`, `dart_http_client()` and
`dart_ws_client()` with no TypeScript in sight, and the reverse holds too — `serde` and `typescript`
with none of the mobile features publishes TypeScript alone. Each language earns its own accessors
under `serde` plus that language's own feature; none of the four needs any of the others.

**A service that publishes TypeScript needs the `zod` feature too.** A message validates when it is
constructed, in both directions: the generated client parses what it is about to send before it
reaches a transport, and the generated dispatcher parses what arrived before it enters an
implementation, so an implementation may assume its message is valid. On the TypeScript side both
checks are the same parse, against the `<Message>$Schema` const `#[model_schema()]` publishes — and
only a build with the `zod` feature publishes one.

So a build with `typescript` on and `zod` off publishes the service's **types** and none of the
three seam artifacts: no `<Service>Schema::ts_client()`, no `<Service>Schema::ts_http_client()`, and
no `<Service>Schema::ts_service()`. The types are published as they always are, because they
describe what the Rust dispatcher and the Rust client put on the wire, and that half validates in
either build. The three that are withheld are withheld rather than emitted without their check: a
client that forwards whatever it is handed and a dispatcher that narrows an unread payload with `as`
both compile and both read exactly like the checked ones, while the Rust half of the same service
goes on validating — two halves of one service disagreeing about what they accept, with nothing to
say so.

A bundle that names either one in such a build is refused where it names it, which is the one place
the choice of features can still be acted on:

```text
error[E0599]: no associated function or constant named `ts_client` found for struct `BillingServiceSchema` in the current scope
   |
23 | #[service_schema()]
   | ------------------- associated function or constant `ts_client` not found for this struct
...
34 |     let _ = BillingServiceSchema::ts_client();
   |                                 ^^^^^^^^^ associated function or constant not found in `BillingServiceSchema`
```

**A service is declared at module scope, never inside a function body.** The module the macro
generates opens with `use super::*;`, which is how the client and the per-operation validators
reach the trait and the message types declared beside it. A module written inside a function body
has the enclosing *module* as its parent rather than the function, so `super` from there reaches
past every name that function declared. `#[model_schema()]` carries the same requirement, for the
same reason.

The macro cannot refuse the placement: an attribute macro is handed the annotated item's own tokens
and nothing about the scope it was written in, and a trait written inside a function is the same
tokens as one written beside it. A function-scoped declaration earns the compiler's own resolution
errors instead — one for the trait, and one for each type an operation named:

```text
error[E0405]: cannot find trait `BillingService` in module `super`
error[E0425]: cannot find type `BalanceRequest` in this scope
error[E0425]: cannot find type `BalanceResponse` in this scope
error[E0425]: cannot find type `BalanceError` in this scope
```

One consequence is worth knowing before it costs anyone an afternoon: rustdoc wraps a doctest with
no explicit `fn main` in one, so a doctest that declares a service writes `fn main() {}` of its own.

**`#[service_schema(non_exhaustive)]` marks every type the service generates `#[non_exhaustive]`:** the fault kind, `CallError`, the messages the macro writes, the `{Service}Schema` struct, and with `mcp` the `McpTask`, `McpTaskState`, `McpContent`, `McpResourceContents`, `McpRole` and `McpPromptMessage` types. A type you declare yourself keeps your own attributes. Another crate that matches on one of them needs a wildcard arm. The flag is bare: `non_exhaustive = true` is refused.

#### The Shape of a Service

A service names a **context** type parameter, and every operation takes it as its first argument after `&self`. An operation receives one message and answers `Result<Success, Error>`.

```rust
use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

#[model_schema()]
#[derive(Deserialize, Serialize)]
pub struct OpenBalanceRequest {
    pub customer_id: String,
}

#[model_schema()]
#[derive(Deserialize, Serialize)]
pub struct OpenBalanceResponse {
    pub is_post_paid: bool,
    pub open_invoices: u32,
}

#[model_schema()]
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum BalanceError {
    DbError,
    InsufficientBalance { shortfall: u32 },
}

#[service_schema()]
pub trait BillingService<Ctx> {
    /// No reply, so no return type and no error arm.
    #[service_schema_op(one_way)]
    async fn apply_bundle(&self, ctx: &Ctx, req: OpenBalanceRequest);

    /// Carried on the wire as `billing-generation-request` rather than `can-generate`.
    #[service_schema_op(message = "billing-generation-request")]
    async fn can_generate(
        &self,
        ctx: &Ctx,
        req: OpenBalanceRequest,
    ) -> Result<OpenBalanceResponse, BalanceError>;

    /// One argument after the context: that argument already is the message.
    async fn get_open_balance(
        &self,
        ctx: &Ctx,
        req: OpenBalanceRequest,
    ) -> Result<OpenBalanceResponse, BalanceError>;

    /// None at all, so the macro declares an empty message.
    async fn sweep(&self, ctx: &Ctx) -> Result<OpenBalanceResponse, BalanceError>;

    /// Several arguments after the context: the macro declares the message.
    async fn void_invoice(
        &self,
        ctx: &Ctx,
        customer_id: String,
        invoice_id: String,
    ) -> Result<OpenBalanceResponse, BalanceError>;
}
```

The context is whatever an implementation needs and no caller may see -- a logger, a correlation id. The implementation chooses the type, and it reaches no message, no schema and no wire:

```rust
pub struct BillingContext {
    pub logger_name: String,
}

pub struct BillingBackEnd;

impl BillingService<BillingContext> for BillingBackEnd {
    async fn apply_bundle(&self, ctx: &BillingContext, req: OpenBalanceRequest) {
        // ...
    }

    // ... and the other four, or this does not compile.
}
```

The trait is emitted as declared, except that `async fn` is desugared to `-> impl Future<Output = ...> + Send`, the desugaring the compiler's own `async_fn_in_trait` warning recommends. A trait with `async fn` is not dyn compatible, so the generated dispatcher is generic over the implementing type rather than taking `&dyn BillingService`.

#### Service Messages

Every operation receives exactly one message. How many arguments it takes after the context decides only who declares that message:

| arguments after the context | the incoming message is | who declares it |
| --- | --- | --- |
| exactly one | that argument, as-is | you, with `#[model_schema()]`, like any other type |
| more than one | a message declared from the argument list | the macro, named `<Operation>Request` |
| none | an empty message | the macro, named `<Operation>Request`, with no fields |

**One argument is the normal case and the one to reach for.** The argument *is* the message: nothing is synthesized, nothing is renamed, and the type is one you wrote and can reuse, version and reference from elsewhere.

**The multi-argument form costs something, and the cost is on the wire.** Each argument's name becomes a field name on the declared message, so renaming a parameter -- an invisible refactor in Rust -- moves a key on the wire, and no compiler will flag it. Reach for it where the input is one identifier and a named type would be ceremony; reach for a declared type everywhere else. Every generated message repeats this in its own rustdoc and in the JSDoc it publishes.

`void_invoice` takes two arguments, so `VoidInvoiceRequest` is declared from them, in declaration order and with camelCase keys:

```typescript
export type VoidInvoiceRequest = {
  customerId: string;
  invoiceId: string;
};
```

```typescript
const VoidInvoiceRequest$RawSchema = z.strictObject({
  customerId: z.string(),
  invoiceId: z.string(),
});

export const VoidInvoiceRequest$Schema: ZodType<VoidInvoiceRequest> = VoidInvoiceRequest$RawSchema.meta({
  examples: []
});
```

`sweep` takes none, so its message is empty rather than absent. An operation that later needs a field gains one here, instead of changing from carrying no payload to carrying one and breaking every caller:

```typescript
export type SweepRequest = Record<string, never>;

const SweepRequest$RawSchema = z.strictObject({
});

export const SweepRequest$Schema: ZodType<SweepRequest> = SweepRequest$RawSchema.meta({
  examples: []
});
```

`get_open_balance` takes one argument, so nothing is declared for it at all -- `OpenBalanceRequest` is the message, exactly as you wrote it.

#### Operation Names

An operation's name is written once, in Rust, and spelled per language by convention. Nothing is repeated and no attribute is involved:

| where | spelling | derived as |
| --- | --- | --- |
| Rust | `get_open_balance` | the trait method as declared |
| TypeScript | `getOpenBalance` | camelCase, what a TypeScript caller expects to type |
| the wire | `get-open-balance` | kebab-case of the method name |

`#[service_schema_op(message = "...")]` moves the **wire name only**. Rust still calls the operation by its method name and TypeScript by the camelCased one:

```rust
#[service_schema_op(message = "billing-generation-request")]
async fn can_generate(
    &self,
    ctx: &Ctx,
    req: OpenBalanceRequest,
) -> Result<OpenBalanceResponse, BalanceError>;
```

It exists for one reason: services already ship wire names nobody would derive from the method. An operation a caller knows as `canGenerate` may dispatch today as `billing-generation-request`, the two sharing no substring, so without the override no such service could adopt this construct without changing bytes. A greenfield operation writes no attribute at all. The override replaces the derived name rather than adding a second one -- after it, nothing answers to `can-generate`.

The TypeScript client and dispatcher carry a wire name, a path, a part name and a header name exactly as declared, whatever they hold: each is written into the generated code as an escaped string literal, so `message = "le$vy'it\"q\\x"` with `path = "/le$vy'it\"q\\x/{label}"`, and a multipart operation's `part("fi$le'x\"q\\" = attachment)`, compile under `tsc --strict`, and the client sends the path `/le$vy'it"q\x/a` and the part `fi$le'x"q\`, which the Rust dispatcher reads.

#### Service Errors

An operation's `Result` names **one** type in its error position. That type is normally an enum, which is a union, so "an operation's errors" is one `#[model_schema()]` type with several variants rather than several types.

What is per-operation is *which* type is named, and two operations on one service may name entirely unrelated ones. A service that genuinely has one error surface pays nothing for this -- it writes the same type in every signature. What it buys is that a read operation is never handed a write-conflict variant to match on, and a caller's `switch` over an operation's errors is exhaustive over exactly what that operation can return.

A variant may carry data of its own, which the tag sits beside:

```typescript
export type BalanceError = {
  errorCode: "db-error";
} | {
  errorCode: "insufficient-balance";
  shortfall: number;
};
```

#### One-Way Operations

An operation marked `#[service_schema_op(one_way)]` expects no reply, and therefore declares no return type:

```rust
#[service_schema_op(one_way)]
async fn apply_bundle(&self, ctx: &Ctx, req: OpenBalanceRequest);
```

The flag and the return type must agree, and the macro checks both directions, so a forgotten `Result` cannot become a silent fire-and-forget. A missing return type with no flag is refused naming both choices:

```text
service_schema: operation `apply_bundle` has no return type
       add `#[service_schema_op(one_way)]` if it expects no reply,
       or give it a `Result<Success, Error>` return
```

And the flag with a return type is refused too:

```text
service_schema: operation `apply_bundle` is marked `one_way` but returns a value
       a one-way operation produces no reply
```

There is no error arm on a one-way operation, because there is no reply to carry one. An operation that needs to report failure is a request-and-reply operation declared wrong. In TypeScript a one-way method answers `Promise<void>`; in Rust it answers nothing beyond the send.

The one failure it can still report is its own. A message that fails its own validation never reaches the transport, and the fault naming the key has nowhere to go but out of band: in Rust it is `Err(ServiceFault)`, the failure arm the method already has, and in TypeScript `Promise<void>` has no failure arm at all, so it is **thrown**. What is thrown is published rather than left to be discovered from the body -- `<Service>Refusal`, an `Error` carrying the fault -- and the method's own `JSDoc` names it:

```typescript
export type BillingServiceRefusal = Error & { fault: BillingServiceFault };
```

Silently dropping a refused message was the alternative and is worse: a fault is a defect meant to page a human, and a dropped one pages nobody.

#### Service Validation

A message is validated when it is constructed, in both directions, by the validator `#[model_schema()]` already generates for it -- `validate()` on the Rust side, the Zod schema on the TypeScript side. Nothing new is written for a service. A generated Rust dispatcher reads a request with the message's read alone, `model_schema_read_unjudged`, through the `unjudged_<operation>` function the service's module publishes for each operation, and calls `validate()` next, so a request that breaks a bound or a check is a `failed-validation` fault naming its field, with every message in its detail.

Inbound, the dispatcher deserializes the payload into the operation's message and validates it *before* the operation body is entered. **A service implementation may therefore assume its incoming message is valid, because an invalid one never reaches it.**

Outbound, the generated client validates the message it is about to send, so a malformed request fails at the caller with the field named rather than as a remote error a round trip later -- and the transport is never touched:

```typescript
    async voidInvoice(req) {
      const validated = billingServiceChecked(VoidInvoiceRequest$Schema, req);
      if (!validated.success) {
        return {
          ok: false,
          error: {
            isServiceFault: true,
            fault: billingServiceOutboundFault("void-invoice", validated.error.issues),
          },
        };
      }
      const { answered } = await transport.request<BillingServiceVoidInvoiceResult>("void-invoice", validated.sent, []);
      if (answered.ok) {
        const parsed = OpenBalanceResponse$Schema.safeParse(answered.value);
        if (!parsed.success) {
          return {
            ok: false,
            error: {
              isServiceFault: true,
              fault: billingServiceReplyFault("void-invoice", parsed.error.issues),
            },
          };
        }
        return { ok: true, value: parsed.data };
      }
      const error = answered.error;
      if (typeof error === "object" && error !== null && "isServiceFault" in error) {
        return { ok: false, error };
      }
      const declared = BalanceError$Schema.safeParse(error);
      if (!declared.success) {
        return {
          ok: false,
          error: {
            isServiceFault: true,
            fault: billingServiceReplyFault("void-invoice", declared.error.issues),
          },
        };
      }
      return { ok: false, error: declared.data };
    },
```

A one-way method runs the same check and has nowhere to put the result of it, so it throws the refusal the section above names -- still before the transport is reached:

```typescript
    async applyBundle(req) {
      const validated = billingServiceChecked(OpenBalanceRequest$Schema, req);
      if (!validated.success) {
        throw billingServiceRefused(billingServiceOutboundFault("apply-bundle", validated.error.issues));
      }
      await transport.notify("apply-bundle", validated.sent, []);
    },
```

A replying method checks what comes back as well, in the tail of `voidInvoice` above. A success is parsed against the operation's declared success and a failed reply against its declared error before the caller reads either: a fault the far side already sealed passes through, a reply that parses is returned as parsed, and one that does not -- a declared field missing, an undeclared key, another shape altogether -- answers a `failed-validation` fault naming the first offending key rather than a value typed as that error. A bundle therefore publishes the Zod schema of every declared error type beside its type.

#### Service Faults

A payload that will not deserialize, a message that fails validation, an operation name nothing recognises, a handler that panicked and a call the transport could not carry are none of the things an operation declared it could fail at. Folding them into a service's domain errors would put transport concerns in every signature, so they come back as a **fault**: a defect rather than a condition, meant to be logged at error level and to page a human.

A result therefore keeps two arms, with the fault riding inside the failure arm behind a literal a caller can narrow on:

```typescript
export type BillingServiceGetOpenBalanceResult =
  | { ok: true; value: OpenBalanceResponse }
  | { ok: false; error: BalanceError | { isServiceFault: true; fault: BillingServiceFault } };
```

A caller that only handles the declared errors still compiles; one that wants to distinguish narrows on `isServiceFault` and then on whatever the error type discriminates on, with no `default` arm to write:

```typescript
if (result.ok) {
  render(result.value.openInvoices);
} else if ("isServiceFault" in result.error) {
  reportUnexpected(result.error.fault);
} else {
  switch (result.error.errorCode) {
    case "db-error":
      return retryLater();
    case "insufficient-balance":
      return showUpgradePrompt();
  }
}
```

**No service implementation can answer with a fault.** In Rust an operation's signature admits only its own error type, so there is nowhere to put one. In TypeScript the interface an implementation satisfies is typed against an *outcome*, which has no fault member in it:

```typescript
export type BillingServiceGetOpenBalanceOutcome =
  | { ok: true; value: OpenBalanceResponse }
  | { ok: false; error: BalanceError };
```

**And the fault type itself refuses to be written.** An outcome with no fault member stops a fault being *returned*; it does not stop one being *built*, and a plain structural object type is writable by anyone who can name it. So the fault's fields publish under a name of their own and the name a caller reads is those fields under a brand keyed on a symbol the bundle declares and exports nowhere:

```typescript
export type BillingServiceFaultFields = {
  detail: string;
  field: string | undefined;
  kind: BillingServiceFaultKind;
  operation: string;
};

declare const billingServiceFaultSeal: unique symbol;

export type BillingServiceFault = BillingServiceFaultFields & {
  readonly [billingServiceFaultSeal]: true;
};
```

The fields still come from the Rust declaration, so nothing about the wire moved and nothing a caller reads changed -- narrow on `isServiceFault`, then read `kind`, `detail`, `field` and `operation`, or switch over `kind` exhaustively. The brand is a type and never a value: `declare const` emits nothing, and a fault carries no extra key.

What stops compiling is writing one. An object literal under a `BillingServiceFault` annotation cannot carry the branded property, and neither can a structurally-equal value assigned into a `BillingServiceFault` position -- a module outside the bundle cannot name the symbol, and a module inside has no value to write. What it does **not** stop is a deliberate type assertion: `built as BillingServiceFault` compiles, as does anything laundered through `any` or `unknown`, and TypeScript has no construct that refuses those. The generated constructors mint through exactly that assertion, from the fields type, which is the one direction an assertion is unambiguously sound in. So the seal makes fabricating a fault a deliberate, greppable act rather than something an annotated literal does silently -- it is not the `E0451` Rust gives, because TypeScript has none to give.

Exactly two generated places build one: the dispatcher, when an incoming message cannot be turned into a valid request or names an operation nothing recognises, and the client, when the message *it* is about to send fails its own validation or the transport reports that the call never landed. In Rust a client call answers `Result<Success, CallError<Error>>`, `CallError` being `Operation(E)` for the error the operation declared and `Fault(ServiceFault)` for a defect that reached the caller.

Which kind a refused payload is reported under is `serde_json`'s own classification of the refusal, not the shape of the sentence it wrote. `undeserializable-payload` means the bytes are not a document at all -- not JSON, or a document that ends early -- which is a sender whose serialization is broken, and such a refusal names no field because nothing was read far enough for a key to be what went wrong. `failed-validation` means the bytes read as a document and did not match the message: a field carrying the wrong type of value, a key that is missing, a value a bound refuses. That is a value someone supplied. A refusal of serde's own names the full dotted path of the value it refused, as a broken bound does, and its detail opens with that path in single quotes: `'entities.1.name': missing field \`name\``, with the field `entities.1.name`. Where serde reads a value from a buffer (`tag` alone, `untagged` and `#[serde(flatten)]`), the path reaches no deeper than the buffered value. The byte offset serde appends is dropped, it being a position inside an encoding the caller never saw.

The TypeScript dispatcher raises only the second of the two, and that is the same line rather than a different one: it is handed a payload somebody already read out of the bytes, so by the time it runs the bytes *were* a document and the only thing left to be wrong is what the document said. Bytes that are no document at all fail before it, in whatever turns them into the value it takes. So one defect reaches a call site under one kind whichever language served it.

The field is named wherever the refusal named one: a validator's report names it in single quotes, and serde names it in backticks for a missing or unknown key. A bound declared on a field's *type* rather than on the field -- a constrained brand -- is the validator's to report as every other is: the brand's own report names nothing, the brand being the value rather than a member of anything, and the message holding it writes the field's wire key into the report, so `'customerId': too short: minimum length is 3, got 2` is what a caller reads, one quoted run holding the whole name, beside every other message of the request. A type mismatch is the one refusal left carrying no field: serde says what it expected and not where, and inventing a name for it would send a caller to the wrong key.

#### What a Service Generates

On the Rust side, beside the trait, in a module named for the service (`billing_service_schema`):

- `ServiceFault` and `ServiceFaultKind` -- readable by anyone, constructible by nothing outside the module. `ServiceFault` is the alias; the struct is declared as `<Service>FaultFields`, which is the name its *fields* publish under in TypeScript, `<Service>Fault` there being the sealed type written over them.
- `CallError<E>` -- `Operation(E)` or `Fault(ServiceFault)`.
- `<Operation>Message`, `unjudged_<operation>` and `validated_<operation>` -- the name each operation's message is republished under, the read of that message alone, which a dispatcher reads a request with, and the validator that message runs. Which of the two answers a check gives depends on the message's concrete type, so both halves of a service ask this one function rather than answering for themselves, from wherever they were expanded.
- `Answered<T, E>` -- the envelope a request-and-reply operation answers in, written by a dispatcher and read back by a client.
- `VoidInvoiceRequest` and `SweepRequest` -- the messages declared for the operations that named none, each an ordinary `#[model_schema()]` type.

Beside the trait, the `macro_rules!` each transport the service asked for contributes -- three for `amqp_rpc`, two each for `http_rest`, `ws_rpc` and `mcp`; see [Transports](#transports) -- and nothing else transport-shaped. A service that asks for no transport is emitted none of them.

#### The `amqp_rpc` walkthrough

`#[service_schema(transports = ["amqp_rpc"])]` says which transports the service wants. Each one contributes `#[macro_export]` macros named `{service_snake_case}_{transport}_dispatcher` and `{service_snake_case}_{transport}_client`, and `amqp_rpc` a third, `{service_snake_case}_{transport}_server`, all emitted at the trait's own scope, whose bodies hold that transport's halves as *tokens*: nothing inside any of them is compiled where the service is declared.

Two macros rather than one, and a third for `amqp_rpc`, because the halves of a service usually live in different crates -- a crate that calls the service can see the contract but has no business seeing the server's backend, a server crate has no use for a client, and a crate that only wants `dispatch` -- a hand-rolled adapter, or a test with no broker in reach -- has no business seeing `lapin`, `tokio` or `futures` either. `amqp_rpc` earns the third because one consumer loop over `lapin` serves every service that places it; `http_rest`, `ws_rpc` and `mcp` have no such fixed loop, a server there being the hosting application's own router, socket handler or stdio loop. Each is invoked and placed by the half that wants it, none drags in another, and a crate that wants more than one places more than one.

```rust,ignore
// In the crate that declares the service: nothing below is built here.
#[service_schema(transports = ["amqp_rpc"])]
pub trait BillingService<Ctx> { /* ... */ }
```

Each invocation goes in a module of its own **file**, and those `mod` declarations go above the crate's `use` items. A `macro_rules!` body is linted under the levels of the crate that *invokes* it, and this is the placement that measures clean: an inline `mod amqp_transport { ... }` earns `clippy::inline_modules`, a `mod` written below a `use` earns `clippy::arbitrary_source_item_ordering`, and reaching the author's own types through `use declaring_crate::*;` earns `clippy::wildcard_imports`.

```text
// In the crate that serves it -- src/lib.rs
mod amqp_transport;

// src/amqp_transport.rs
declaring_crate::billing_service_amqp_rpc_dispatcher!();
```

```text
// In a crate that calls it -- src/lib.rs: no backend, no bus library.
mod amqp_client;

// src/amqp_client.rs
use declaring_crate::{OpenBalanceRequest, OpenBalanceResponse};

declaring_crate::billing_service_amqp_rpc_client!();
```

```text
// In the crate that serves it for real, over a real `lapin::Channel` -- src/lib.rs
mod amqp;

// src/amqp.rs
declaring_crate::billing_service_amqp_rpc_server!();
```

Each macro takes no arguments and emits bare items rather than a module of its own, so the caller names the module and two transports in one crate cannot collide. The dispatcher emits `IncomingMessage` -- built with `IncomingMessage::new(operation, payload, headers)` by whichever crate owns the bus and read through `operation()`, `payload()` and, where the service claims at least one `header_in` binding, `headers()`; `headers` carries the request's own headers table, and a service claiming none is left with the earlier two-field shape and no accessor for a field nothing would read -- `Reply`, the handle a transport implements to answer one message, with `send(value, headers)` and `fault`, and `dispatch(svc, ctx, message, reply)`, generic over the implementing type. Beside them go the panic guard an arm calls its implementation behind and the reader that classifies a refused payload, both emitted only where the service declares an operation for them to serve. The client emits:

- `Transport` -- the seam a client is bound to, with `notify(operation, payload, headers)` and `request(operation, payload, headers)`, `request` answering the encoded reply *and* the headers it carried back. Both answer a `Result`, whose failure arm carries in words what stopped a call from travelling; the client turns it into a fault of kind `transport-failure`.
- `BillingServiceClient` -- one method per operation, over any `Transport`. Each takes that operation's arguments -- one more per `header_in` binding, in declaration order -- and no context: a context is what an implementation needs, and a caller has nothing to hand one to. Every method answers a `Result`, and every method's generated documentation says under an `# Errors` heading what its failure arm can hold.
- The mirror that reads a fault back off the wire and the reader that turns one envelope into three outcomes, emitted only where the service declares an operation that answers. A service declaring only one-way operations is emitted neither, there being no reply for either to read.

**`header_in`/`header_out`/`error_header_out` ride the headers channel, not the payload.** `#[service_schema_op(http(header_in("name" = parameter), header_out("name")))]` binds an ordinary argument beside the message to a named header on the way in, and names the tuple element after the response on the way out; `error_header_out("name")` names the tuple element after the *declared error* the same way, read and written only when the error is the reply. On `amqp_rpc` that channel is the message's own basic-properties headers table: the dispatcher decodes each `header_in` value (JSON-encoded) off the incoming headers before calling the implementation, naming the header rather than a payload field if a value will not decode; a `header_out`-bound success type and an `error_header_out`-bound error type are each a tuple, and the arm sends the value alone while writing every element after it into the reply's own headers. The client mirrors both directions, encoding each `header_in` argument and decoding `request`'s returned headers back into whichever declared tuple its outcome answers. An operation naming no `http(...)` group carries an empty headers list either way -- the channel exists on every operation, but nothing reads or writes it there.

The server emits everything the dispatcher does, plus the pieces that turn a real delivery into a call on an implementation:

- `Context { pub logger: tracing::Span }` -- what the service is handed beside every message.
- `ReplyHandle` -- implements the dispatcher's own `Reply` over a `lapin::Channel`: `fault` and `send` both publish through the wire framing below; `send`'s headers are written into the outgoing `lapin::BasicProperties`' own headers table, JSON-encoded, beside whatever the transport merges in; a handle whose delivery carried no `reply_to` publishes nothing, and a publish the channel refuses is logged at error and dropped rather than propagated.
- **A declared error and a fault reply both carry the header `is_error` = `true`, an AMQP boolean field; a success reply carries none.** The payload is unchanged -- this rides the same headers table `header_out` writes into, as a boolean rather than the JSON text a `header_out` value is. `header_out("is_error")` is refused at the declaration on a service that asks for `amqp_rpc`, so a bound value never collides with it. A crate that places only `{service}_amqp_rpc_dispatcher!()` builds no `ReplyHandle` and does its own framing over `Reply` by hand, so it is untouched by this.
- **A reply's body is the payload alone, and everything else is AMQP metadata.** The body is the value, the declared error or the fault, exactly as `dispatch` wrote it -- `null` for a unit success -- with nothing added and nothing removed. Which of the three it is rides in the AMQP `type` property (`response`, `error` or `fault`), the correlation id in `correlation_id`, and `is_error` and every `header_out` value in the headers table. A transport that hands the reply to a generated client rebuilds `{ ok, value | error }` from the `type` property and the body.
- `reply`, `framed_fault` and `outgoing_headers` -- the wire framing a reply and a fault are built through: `reply` answers the `type` property and the body for one answer.
- `serve_until(channel: &lapin::Channel, queue: &str, service: &S, prefetch: NonZeroU16, shutdown: F) -> Result<Stopped, lapin::Error>` -- the consumer loop itself: declares the queue, tells the broker this consumer holds `prefetch` unacknowledged deliveries, and dispatches up to that many at once, acknowledging each when its own dispatch finishes, in whatever order they finish. The prefetch has no default: `basic.qos` reads `0` as unlimited, which no bounded loop can honour, and a fixed one would park deliveries behind a busy consumer. The deliveries run concurrently on the task awaiting `serve_until`, not in parallel, so a handler that blocks its thread holds up every other; a service that wants threads runs more than one loop, each spawned on a task of its own. Once `shutdown` completes the consumer is cancelled at once, so the broker stops pushing, no delivery is started, and every delivery already started is dispatched and acknowledged before `serve_until` returns `Stopped::ShutdownRequested`; a consumer the broker closes is drained the same way before it returns `Stopped::ConsumerClosed`. The queue is a runtime `&str`; the span the loop opens is `tracing::info_span!("amqp_service", queue, operation = operation.as_str())`. Every delivery's headers table is read into `IncomingMessage` regardless of whether the service claims any of them, `header_in` being a per-operation question the loop itself does not answer.
- `serve_deliveries(deliveries, limit, shutdown, stop, handle) -> Stopped` -- the loop `serve_until` runs, generic over the stream of deliveries, the shutdown, the action that stops the consumer and the handler one delivery is given to, so it runs with no broker at all: in a test, over `futures::stream::iter`.

A crate that invokes the server macro names all six of `lapin`, `tokio`, `futures`, `serde`, `serde_json` and `tracing` in its own manifest, the emitted consumer loop calling every one of them.

Paths inside a `macro_rules!` body resolve where the macro is *invoked*, so the two kinds are spelled apart. Everything tixschema generated is reached through `$crate::`, which resolves in the crate that declared the service; every runtime crate is reached through a leading `::` and resolves in the invoking crate, **which is therefore the one that names `serde`, `serde_json` and `tracing` in its own manifest**. That is the point of the shape: a crate that only declares services names serde alone. A crate that places only a client names `serde` and `serde_json` and not `tracing`, nothing in a client catching a panic; the `use` in the client module above is what resolves the message types the author declared, which the expansion spells exactly as they were written.

A crate that declares a service *and* places one of its halves invokes that macro by name rather than by path -- `billing_service_amqp_rpc_dispatcher!()` -- after the declaration and inside it or a module below it, `#[macro_use]` carrying it out of a submodule. Rust refuses an absolute path to a `macro_export` macro that a macro produced, from within the crate that produced it -- `crate::billing_service_amqp_rpc_dispatcher!()` and `use crate::billing_service_amqp_rpc_dispatcher;` both earn a deny-by-default future-incompatibility:

```text
error: macro-expanded `macro_export` macros from the current crate cannot be referred to by absolute paths
   = note: `#[deny(macro_expanded_macro_exports_accessed_by_absolute_paths)]` (part of `#[deny(future_incompatible)]`) on by default
```

Another crate reaches either macro by path as above.

`#[macro_export]` places each macro at the declaring crate's root whatever module it was written in, so each name has to be unique across that crate, and `$crate` reads from that same root. A service declared at the crate root needs nothing further; one declared in a submodule re-exports at the crate root its generated module and its trait, which is all either half reaches by name, beside the `<Service>Schema` the `http_rest` dispatcher of a service that asks for `openapi` reads its document from (see [OpenAPI](http-rest.md#openapi)) -- every message a dispatcher deserializes, and every message a client builds, is reached as `$crate::{service}_schema::<Operation>Message`. A message the *author* named is spelled as they wrote it where a client method takes it, and resolves in the module the macro was invoked in.

Both of those the declaring crate checks for itself. A service that asked for at least one transport resolves its generated module and its trait against the crate root at the declaration, so leaving either unreachable fails **that** crate's own `cargo build` rather than every crate that goes on to invoke a macro. The caret sits on the trait, with the attribute macro named as the origin:

```text
error[E0433]: cannot find `billing_service_schema` in `crate`
  --> src/services/billing.rs:15:11
   |
14 | #[service_schema(transports = ["amqp_rpc"])]
   | -------------------------------------------- in this attribute macro expansion
15 | pub trait BillingService<Ctx> {
   |           ^^^^^^^^^^^^ unresolved import

error[E0405]: cannot find trait `BillingService` in the crate root
```

Each is earned on its own, so re-exporting one leaves the other standing rather than repeating both. Write them as **`pub use`**, not `use`: a private import at the root satisfies an invocation inside the declaring crate -- which is what this crate's own test harnesses rely on -- while a crate invoking the macro from outside earns ``error[E0603]: module `billing_service_schema` is private``. A service that asked for no transport publishes no macro, is checked against no root, and compiles below the crate root with nothing re-exported.

On the TypeScript side, three artifacts, reached through a unit struct named `<Service>Schema`:

```rust
pub fn get_entities() -> (String, Vec<String>) {
    (
        "service entities".to_string(),
        vec![
            BillingServiceSchema::ts_definition(),
            BillingServiceSchema::ts_client(),
            BillingServiceSchema::ts_service(),
        ],
    )
}
```

`ts_definition()` answers with every message the macro declared for the service, the fault type and the kind it reports, and one result type per operation that answers -- **so a generated message needs no registration line of its own.** The author never wrote those types and has no reason to know their names; a forgotten line would leave a Rust-only message and a client unable to call the operation.

`ts_client()` answers with the transport seam, the client type and the factory that binds one:

```typescript
export type BillingServiceClient = {
  /**
   * Sends `apply-bundle` on `BillingService`, which expects no reply.
   *
   * @throws {BillingServiceRefusal} when the message fails its own schema. The operation
   * answers `Promise<void>`, so there is no failure arm to put the fault in; the transport
   * is still never reached.
   */
  applyBundle(req: OpenBalanceRequest): Promise<void>;
  /** Calls `billing-generation-request` on `BillingService` and waits for the answer. */
  canGenerate(req: OpenBalanceRequest): Promise<BillingServiceCanGenerateResult>;
  /** Calls `get-open-balance` on `BillingService` and waits for the answer. */
  getOpenBalance(req: OpenBalanceRequest): Promise<BillingServiceGetOpenBalanceResult>;
  /** Calls `sweep` on `BillingService` and waits for the answer. */
  sweep(req: SweepRequest): Promise<BillingServiceSweepResult>;
  /** Calls `void-invoice` on `BillingService` and waits for the answer. */
  voidInvoice(req: VoidInvoiceRequest): Promise<BillingServiceVoidInvoiceResult>;
};
```

`ts_service()` answers with the interface an implementation satisfies in full, the outcome types it answers with, and the dispatcher factory:

```typescript
export interface BillingServiceImpl<Ctx> {
  /** Handles `apply-bundle` on `BillingService`, which expects no reply. */
  applyBundle(ctx: Ctx, req: OpenBalanceRequest): Promise<void>;
  /** Handles `billing-generation-request` on `BillingService` and answers it. */
  canGenerate(ctx: Ctx, req: OpenBalanceRequest): Promise<BillingServiceCanGenerateOutcome>;
  /** Handles `get-open-balance` on `BillingService` and answers it. */
  getOpenBalance(ctx: Ctx, req: OpenBalanceRequest): Promise<BillingServiceGetOpenBalanceOutcome>;
  /** Handles `sweep` on `BillingService` and answers it. */
  sweep(ctx: Ctx, req: SweepRequest): Promise<BillingServiceSweepOutcome>;
  /** Handles `void-invoice` on `BillingService` and answers it. */
  voidInvoice(ctx: Ctx, req: VoidInvoiceRequest): Promise<BillingServiceVoidInvoiceOutcome>;
}
```

```typescript
export function createBillingServiceDispatcher<Ctx>(
  impl: BillingServiceImpl<Ctx>,
): (
  ctx: Ctx,
  operation: string,
  payload: unknown,
  headers?: ReadonlyArray<readonly [string, string]>,
  parts?: ReadonlyArray<readonly [string, unknown]>,
) => Promise<BillingServiceDispatched | undefined> {
```

Every member is required, so an implementation missing one is refused where it reaches `createBillingServiceDispatcher`. Every emitted name carries the service -- `BillingServiceFault`, `BillingServiceGetOpenBalanceResult`, `BillingServiceClient`, and the fault's own brand symbol `billingServiceFaultSeal` -- because TypeScript has no per-service scope and a bundle is one flat file. Rust needs no such prefix, the generated module being the scope TypeScript lacks.

**Headers cross the TypeScript seam both ways, each value JSON-encoded.** The client's transport seam carries them beside the payload, and `request` answers the reply's own headers beside the envelope:

```typescript
export type BillingServiceTransport = {
  notify(
    operation: string,
    payload: unknown,
    headers: ReadonlyArray<readonly [string, string]>,
  ): Promise<void>;
  request<Answered>(
    operation: string,
    payload: unknown,
    headers: ReadonlyArray<readonly [string, string]>,
  ): Promise<{ answered: Answered; headers: ReadonlyArray<readonly [string, string]> }>;
};
```

The header text is the JSON encoding the Rust `amqp_rpc` transport writes into the AMQP headers table, so an AMQP transport hands the pairs over as they are. A transport written against the earlier seam -- `request` answering the reply alone -- answers `{ answered, headers }` now, with an empty list where the reply carried none. A client method takes one argument per `header_in` binding after the message and sends each JSON-encoded, an optional one holding `undefined` as `null`, which is what the Rust client writes for a `None`. A `header_out` or `error_header_out` reply is rejoined into the tuple the operation declared: the body off the envelope, each header off the reply's headers, checked against the element's own schema. An absent optional header reads as `null`, the value its tuple slot holds; a required one that is absent, is not JSON, or does not match its schema answers a `failed-validation` fault naming the header.

The dispatcher reads each `header_in` value off the same JSON text, checked against the argument's own schema, and refuses a missing required one or a malformed one through the same framed fault a bad payload gets. It answers `BillingServiceDispatched` -- `{ answered, headers }`, the envelope that goes on the wire and the headers written beside it -- or `undefined` for a one-way operation that ran. A header tuple is split there: the body answers as the envelope's `value` or `error`, and each header element is written under its name, one holding `null` written nowhere. A transport that drove the dispatcher before writes `dispatched.answered` where it wrote the whole answer, and `dispatched.headers` into its own headers channel.

Every reader ignores `value` for a `Result<(), E>` operation, and for a `Result<T, E>` operation whose declared success `T` is a unit struct declared above the service. `ts_client()`'s result type reads `{ ok: true; value: undefined }`, and `ts_service()`'s outcome type reads `{ ok: true }` with no `value` member at all.

#### Transports

tixschema generates no transport of its own -- what it emits is the seam either side of one -- with one exception: the `amqp_rpc` transport's server macro also emits a real consumer loop over `lapin`, since AMQP's own request-and-reply shape is fixed enough that the loop is the same for every service that places it. The seam below is what `amqp_rpc`, `http_rest` and `ws_rpc` are built on; `mcp`, whose wire is JSON-RPC under the Model Context Protocol, has its own, under [The `mcp` Transport](mcp.md#the-mcp-transport):

- **The operation name arrives from the transport**, beside the payload and never inside it, so no message type has to reserve a key for routing. `dispatch` is handed the name and matches on it; a gRPC method name or an HTTP path serves the same role on those transports.
- **The encoding lives behind `Reply`.** `Reply::send(value, headers)` is handed a value rather than bytes, because a transport merges its own fields -- a correlation id, an error flag -- into the object before serializing it, and neither is reachable behind an encoded buffer.
- **Queues, retries, timeouts, acknowledgement, connection handling and reconnection belong to the caller.** `dispatch` returns nothing, so the adapter that called it still holds the delivery when the arm finishes, and acknowledges there -- for every message, including a one-way one. That placement is what lets the handle carry replying and nothing else: a path that never replies names nothing about replying.
- **A call that never landed has a place to be reported.** `Transport::notify` and `Transport::request` both answer a `Result`, and `Err` carries whatever the transport wants to say about a message that did not go out or a reply that is not coming. Without it a transport that hit its own deadline could only panic or hang, and a caller would be left holding a call that never completes and no fault to report. Whether there is a deadline, and how long it is, stays the transport's decision -- the arm is where the answer is reported, not where it is decided.
- **A header-shaped channel rides beside the payload on both legs.** `notify`/`request` take a `headers: Vec<(String, String)>` beside the payload -- one JSON-encoded entry per `header_in` binding, empty for an operation that claims none -- and `request`'s answer carries the reply's own headers back the same way, which is where a `header_out` value is read from. On `amqp_rpc` this is the AMQP message's own basic-properties headers table; another transport maps it onto whatever header-shaped channel it has.

A request-and-reply arm answers exactly once, through `send` or `fault`, whichever way it goes. A one-way arm that reached its implementation calls neither; one whose payload was refused before it got there answers with a fault, that being the only thing it can say about a message it never ran.
