//! The TypeScript a `#[service_schema]` service publishes, behind one registration line.
//!
//! # Registration rides with the service
//!
//! In the consuming codebase a type reaches the emitted TypeScript only by being named by hand in
//! a bundle's entity list. A message the macro declared has nobody to write that line: the author
//! never wrote the type and has no reason to know its name, so a forgotten line would leave a
//! Rust-only message and a client unable to call the operation at all.
//!
//! So `ts_definition()` answers for the service *and* for every message
//! [`parse`](crate::service_schema::parse) recorded on
//! [`ServiceDef::generated_messages`](crate::service_schema::parse::ServiceDef::generated_messages)
//! — the same list the message emitter writes the types from, read rather
//! than re-derived, so what is written and what is registered cannot disagree. A service is added
//! to a bundle once and nothing it declared can be left behind.
//!
//! # Why the registration hangs off `<Service>Schema`
//!
//! The design spells the bundle line `UsageService::ts_definition()`, and Rust does not allow it:
//! `UsageService` is a trait, an inherent `impl` on a trait is not a thing, and calling a trait's
//! associated function without naming an implementing type is `error[E0790]`. A struct of the same
//! name would collide with the trait in the type namespace. So the artifacts hang off a unit struct
//! named for the service, `UsageServiceSchema`, and the bundle line reads
//! `UsageServiceSchema::ts_definition()` — still one line per artifact, still nothing to remember
//! per message.
//!
//! # What each artifact is
//!
//! - `ts_definition()`: every generated message's type and schema, the fault type and the kind it
//!   reports, and one [`result`] type per operation that answers.
//! - `ts_client()`: the AMQP-shaped transport seam, the client type and the factory that binds one.
//! - `ts_http_client()`: the `http_rest` transport seam — a plain-terms request in, response out,
//!   nothing here naming the library that finally carries the call — the client type, and the
//!   factory that binds one.
//! - `ts_http_service()`: an `http_rest` server — the route table, the request and response
//!   shapes, a fault handler with the Rust transport's own defaults, and a dispatcher that matches,
//!   assembles the message the way the Rust dispatcher does, and drives `ts_service()`'s own
//!   `create{Service}Dispatcher`. Names no web framework; the hosting application binds it.
//! - `ts_service()`: the interface an implementation satisfies in full, the outcome types it
//!   answers with, and the dispatcher factory.
//! - `ts_ws_client()`: the `ws_rpc` transport seam — a socket seam a platform `WebSocket` satisfies
//!   as it is, the heartbeat options, the per-operation schema tables a reply is checked against,
//!   and the factory that binds one to the `ts_client()` seam.
//! - `ts_ws_service()`: the `ws_rpc` dispatcher attachment — reads every notify and request frame
//!   naming this service off the socket seam `ts_ws_client()` publishes, drives the dispatcher
//!   `ts_service()` publishes, and hands a refused notify's fault to a required `onFault`. A
//!   bundle names `ts_ws_client()` before this, so the socket type it attaches to is declared.
//! - `ts_ws_server()`: a `ws_rpc` server for Node that accepts connections — a context and an
//!   idle-armed heartbeat per connection, a cancellation signal, and a hook that hands one socket
//!   to a second service — wrapping `ts_ws_service()`'s attachment rather than re-emitting its
//!   frame rules. A bundle names `ts_ws_client()`, then `ts_ws_service()`, then this.
//! - `dart_definition()`: the Dart sibling of `ts_definition()` — every generated message's Dart
//!   type, the fault kind and fields, and one [`dart_result`] pair per operation that answers
//!   (published only where the `dart` feature is on).
//! - `dart_http_client()`: the Dart sibling of `ts_http_client()` — the same `http_rest` seam and
//!   per-operation client, in Dart, over the `dart` feature's own types and codec rather than Zod;
//!   a reply method answers [`dart_result`]'s own sealed pair rather than throwing (published only
//!   where the `dart` feature is on).
//! - `dart_ws_client()`: the Dart `ws_rpc` sibling — a transport over a sink and a stream, the
//!   per-operation client answering the same sealed pair as `dart_http_client()`, and a dispatcher
//!   attachment for a service the app implements (published only where the `dart` feature is on).
//! - `swift_http_client()`: the Swift `http_rest` sibling — the transport seam, one `async`
//!   method per operation over Swift's own `Result<Success, Failure>`, and the fault helpers
//!   every method reaches for (published only where the `swift` feature is on).
//! - `swift_ws_client()`: the Swift `ws_rpc` sibling — the socket seam, the heartbeat options, and
//!   one actor carrying the correlation and the liveness probe with one method per operation,
//!   naming `swift_http_client()`'s own `Failure`/`Refusal` types rather than redeclaring them
//!   (published only where the `swift` feature is on).
//! - `kotlin_http_client()`: the Kotlin sibling of `dart_http_client()` — request/response data
//!   classes, the transport seam, one sealed result per reply operation, and a `suspend`-method-
//!   per-operation client, over the `kotlin` feature's own types and codec (published only where
//!   the `kotlin` feature is on).
//! - `kotlin_ws_client()`: the Kotlin sibling of `dart_ws_client()` — a transport that owns the
//!   socket, a `suspend`-method-per-operation client answering `kotlin_http_client()`'s own
//!   sealed result, and a dispatcher attachment for a service the app implements, over the
//!   `kotlin` feature's own types and codec (published only where the `kotlin` feature is on). A
//!   bundle names `kotlin_http_client()` before this, so the sealed result type it answers with
//!   is declared.
//!
//! # The client and the dispatcher exist only where the Zod surface does
//!
//! A message validates when it is constructed, in both directions: the client parses what it is
//! about to send before a transport is reached, and the dispatcher parses what arrived before an
//! implementation is entered, so an implementation may assume its message is valid. On the
//! TypeScript side both checks are the same parse, against the `<Message>$Schema` const
//! `#[model_schema()]` publishes — and only a build with the `zod` feature publishes one.
//!
//! So a build with `typescript` on and `zod` off emits neither. The two artifacts it can still
//! write truthfully — the message types and the result envelopes — are published exactly as they
//! always are, because they describe what a Rust service puts on the wire and that half validates
//! either way. The two it cannot are absent rather than emitted without their check: a client that
//! forwards whatever it is handed and a dispatcher that narrows an unread payload with `as` would
//! both compile, both look like the checked ones, and neither would hold the guarantee every
//! caller of them is written against.
//!
//! A bundle naming `<Service>Schema::ts_client()` in such a build is refused where it names it,
//! which is the one place the choice of features can still be acted on.
//!
//! # Every emitted name carries the service
//!
//! TypeScript has no per-service scope. Rust puts each service's supporting types in a module of
//! its own, and a bundle is one flat file — so a consuming codebase with ten services in one
//! bundle would declare `ServiceFault` ten times and would not compile, and two services sharing
//! an operation name would collide on the result type the same way. Every name emitted here is
//! therefore prefixed with the service: `UsageServiceFault`, `UsageServiceGetBalanceResult`,
//! `UsageServiceClient`. The prefix makes TypeScript say what Rust already means.
//!
//! # The fault's TypeScript is generated, not written
//!
//! The Rust `ServiceFault` carries `#[model_schema()]`, so its TypeScript comes from the same
//! declaration as the Rust type and the two cannot drift. Nothing here writes a fault's fields; the
//! registration below asks the Rust type for them, exactly as it does for every message.
//!
//! What the registration adds is the seal. The fields publish under a name of their own,
//! `<Service>FaultFields`, and [`fault`] declares `<Service>Fault` over them as those fields plus a
//! brand keyed on a symbol the bundle exports nowhere. Rust refuses a fabricated fault with `E0451`
//! on the fields, and this is what TypeScript can be given in its place: a type a caller reads
//! exactly as before and an implementation cannot write.

#[cfg(all(feature = "typescript", feature = "zod"))]
mod client;
#[cfg(feature = "dart")]
mod dart_http_client;
#[cfg(feature = "dart")]
mod dart_result;
#[cfg(feature = "dart")]
mod dart_ws_client;
#[cfg(feature = "typescript")]
mod fault;
#[cfg(all(feature = "typescript", feature = "zod"))]
mod http_client;
#[cfg(all(feature = "typescript", feature = "zod"))]
mod http_service;
#[cfg(feature = "kotlin")]
mod kotlin_http_client;
#[cfg(feature = "kotlin")]
mod kotlin_ws_client;
#[cfg(all(feature = "typescript", feature = "zod"))]
mod message;
mod result;
#[cfg(all(feature = "typescript", feature = "zod"))]
mod service;
#[cfg(feature = "swift")]
mod swift_http_client;
#[cfg(feature = "swift")]
mod swift_type;
#[cfg(feature = "swift")]
mod swift_ws_client;
#[cfg(all(feature = "typescript", feature = "zod"))]
mod ws_client;
#[cfg(all(feature = "typescript", feature = "zod"))]
mod ws_server;
#[cfg(all(feature = "typescript", feature = "zod"))]
mod ws_service;

use crate::service_schema::parse::ServiceDef;
use crate::service_schema::support::exhaustiveness;
#[cfg(any(feature = "typescript", feature = "dart"))]
use crate::service_schema::support::{fault_fields_typescript_name, module_ident};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub fn emit(service: &ServiceDef, non_exhaustive: bool) -> TokenStream {
    let named = service.ident.to_string();
    let registry = format_ident!("{named}Schema", span = service.ident.span());
    let rustdoc = registry_rustdoc(&named);
    let ts_seam = ts_seam(service);
    let dart_seam = dart_seam(service);
    let swift_seam = swift_seam(service);
    let kotlin_seam = kotlin_seam(service);
    let sealed = exhaustiveness(non_exhaustive);
    quote! {
        #(#[doc = #rustdoc])*
        #sealed
        pub struct #registry;

        impl #registry {
            #ts_seam
            #dart_seam
            #swift_seam
            #kotlin_seam
        }
    }
}

/// The service's generated TypeScript: `ts_definition()` plus the client and dispatcher artifacts
/// [`seam`] publishes where `zod` is also on. Published only where the `typescript` feature is on.
#[cfg(feature = "typescript")]
fn ts_seam(service: &ServiceDef) -> TokenStream {
    let published = published(service);
    let seam = seam(service);
    quote! {
        #[doc = " Every TypeScript type this service publishes: the messages the macro declared"]
        #[doc = " for it, the fault a caller can receive, and one result type per operation that"]
        #[doc = " answers."]
        pub fn ts_definition() -> std::string::String {
            [#(#published),*].join("\n\n")
        }

        #seam
    }
}

#[cfg(not(feature = "typescript"))]
fn ts_seam(_service: &ServiceDef) -> TokenStream {
    TokenStream::new()
}

/// The service's generated Dart clients: the `http_rest` transport seam, the exceptions a call
/// throws, the client class, and the fault helpers every method reaches for; and the `ws_rpc`
/// transport over a sink and a stream, its own client and exceptions, and a dispatcher attachment
/// for a service the app implements — published only where the `dart` feature publishes the Dart
/// types and codec this client's messages, successes and errors are written in.
#[cfg(feature = "dart")]
fn dart_seam(service: &ServiceDef) -> TokenStream {
    let definition = dart_published(service);
    let client = dart_http_client::emit(service).join("\n\n");
    let ws_client = dart_ws_client::emit(service).join("\n\n");
    quote! {
        #[doc = " Every Dart type this service publishes: the messages the macro declared for it,"]
        #[doc = " the fault kind and fields, and one result pair per operation that answers — the"]
        #[doc = " Dart twin of `ts_definition()`."]
        pub fn dart_definition() -> std::string::String {
            [#(#definition),*].join("\n\n")
        }

        #[doc = " The service's generated Dart `http_rest` client: the transport seam, the client"]
        #[doc = " class, and the one-way refusal a call still throws."]
        pub fn dart_http_client() -> std::string::String {
            #client.to_owned()
        }

        #[doc = " The service's generated Dart `ws_rpc` client: the transport over a sink and a"]
        #[doc = " stream, the client class, the one-way refusal a call still throws, and the"]
        #[doc = " dispatcher attachment."]
        pub fn dart_ws_client() -> std::string::String {
            #ws_client.to_owned()
        }
    }
}

#[cfg(not(feature = "dart"))]
fn dart_seam(_service: &ServiceDef) -> TokenStream {
    TokenStream::new()
}

/// The service's generated Swift clients: the `http_rest` transport seam with one `async` method
/// per operation, and the `ws_rpc` transport with one correlating actor — both published only
/// where the `swift` feature publishes the Swift types and codec.
#[cfg(feature = "swift")]
fn swift_seam(service: &ServiceDef) -> TokenStream {
    let http_client = swift_http_client::emit(service).join("\n\n");
    let ws_client = swift_ws_client::emit(service).join("\n\n");
    quote! {
        #[doc = " The service's generated Swift `http_rest` client: the transport seam, one"]
        #[doc = " `async` method per operation, and the fault helpers every method reaches for."]
        pub fn swift_http_client() -> std::string::String {
            #http_client.to_owned()
        }

        #[doc = " The service's generated Swift `ws_rpc` client: the socket seam, the heartbeat"]
        #[doc = " options, and the actor that correlates requests to their replies and answers"]
        #[doc = " one method per operation."]
        pub fn swift_ws_client() -> std::string::String {
            #ws_client.to_owned()
        }
    }
}

/// The service's generated Kotlin `http_rest` client and its `ws_rpc` sibling, both over the
/// `kotlin` feature's own types and codec, published only where that feature is on.
#[cfg(feature = "kotlin")]
fn kotlin_seam(service: &ServiceDef) -> TokenStream {
    let client = kotlin_http_client::emit(service).join("\n\n");
    let ws_client = kotlin_ws_client::emit(service).join("\n\n");
    quote! {
        #[doc = " The service's generated Kotlin `http_rest` client: request/response data"]
        #[doc = " classes, the transport seam, one sealed result per reply operation, and the"]
        #[doc = " client class with one `suspend` method per operation."]
        pub fn kotlin_http_client() -> std::string::String {
            #client.to_owned()
        }

        #[doc = " The service's generated Kotlin `ws_rpc` client: the transport that owns the"]
        #[doc = " socket, the client class answering `kotlin_http_client()`'s own sealed result,"]
        #[doc = " and the dispatcher attachment for a service the app implements."]
        pub fn kotlin_ws_client() -> std::string::String {
            #ws_client.to_owned()
        }
    }
}

#[cfg(not(feature = "swift"))]
fn swift_seam(_service: &ServiceDef) -> TokenStream {
    TokenStream::new()
}

#[cfg(not(feature = "kotlin"))]
fn kotlin_seam(_service: &ServiceDef) -> TokenStream {
    TokenStream::new()
}

/// The two artifacts that carry the validation decision D11 binds: the client that checks a
/// message before it reaches a transport, and the dispatcher that checks one before it reaches an
/// implementation. Both parse against the Zod schema `#[model_schema()]` publishes for the message,
/// so a build without the Zod surface has nothing for either of them to check against and publishes
/// neither.
#[cfg(all(feature = "typescript", feature = "zod"))]
fn seam(service: &ServiceDef) -> TokenStream {
    let client = client::emit(service).join("\n\n");
    let http_client = http_client::emit(service).join("\n\n");
    let http_service = http_service::emit(service).join("\n\n");
    let service_side = service::emit(service).join("\n\n");
    let ws_client = ws_client::emit(service).join("\n\n");
    let ws_service = ws_service::emit(service).join("\n\n");
    let ws_server = ws_server::emit(service).join("\n\n");
    quote! {
        #[doc = " The service's generated TypeScript client: the transport seam it is bound"]
        #[doc = " to, the type its methods are declared on, and the factory that binds one."]
        pub fn ts_client() -> std::string::String {
            #client.to_owned()
        }

        #[doc = " The service's generated `http_rest` TypeScript client: the plain-terms request"]
        #[doc = " and response seam, the client type, and the factory that binds one to it."]
        pub fn ts_http_client() -> std::string::String {
            #http_client.to_owned()
        }

        #[doc = " The service's generated `http_rest` TypeScript server: the route table, the"]
        #[doc = " request and response shapes, a fault handler with the Rust transport's own"]
        #[doc = " defaults, and the dispatcher that matches, assembles the message the way the"]
        #[doc = " Rust dispatcher does, and drives `create{Service}Dispatcher`."]
        pub fn ts_http_service() -> std::string::String {
            #http_service.to_owned()
        }

        #[doc = " The service's implementable TypeScript interface, the outcome types an"]
        #[doc = " implementation answers with, and the dispatcher factory that drives one."]
        pub fn ts_service() -> std::string::String {
            #service_side.to_owned()
        }

        #[doc = " The service's generated `ws_rpc` TypeScript transport: the socket seam a platform"]
        #[doc = " `WebSocket` satisfies, the heartbeat options, and the factory that binds one to"]
        #[doc = " the `ts_client()` seam."]
        pub fn ts_ws_client() -> std::string::String {
            #ws_client.to_owned()
        }

        #[doc = " The service's generated `ws_rpc` dispatcher attachment: serves a service the"]
        #[doc = " browser implements off one socket, handing a refused push to the required"]
        #[doc = " `onFault`."]
        pub fn ts_ws_service() -> std::string::String {
            #ws_service.to_owned()
        }

        #[doc = " The service's generated `ws_rpc` server for Node: accepts sockets a listener"]
        #[doc = " produced, builds a context per connection, answers probes, and hands one socket"]
        #[doc = " to a second service. Wraps `ts_ws_service()`'s attachment, so a bundle names that"]
        #[doc = " first."]
        pub fn ts_ws_server() -> std::string::String {
            #ws_server.to_owned()
        }
    }
}

#[cfg(all(feature = "typescript", not(feature = "zod")))]
fn seam(_service: &ServiceDef) -> TokenStream {
    TokenStream::new()
}

/// One expression per published artifact, each answering with a `String`, in the order they are
/// written into the bundle: every declared message first, so the types the result envelopes name
/// are read before the envelopes themselves, then the fault, then the results.
///
/// The fault's fields and the kind it reports are asked for by name rather than written here. Both
/// are ordinary `#[model_schema()]` types inside the service's own module, so their TypeScript
/// comes from the declarations the Rust dispatcher and the Rust client build faults from — the one
/// thing that keeps the type a caller narrows on and the value the wire carries from drifting
/// apart. What [`fault`] adds beside them is the seal and nothing else: no field, no kind, no
/// spelling of either.
///
/// A message's Zod schema is one of those artifacts and is registered here for the same reason its
/// type is — nobody else has a line to write it on. It is asked for only in a build that writes
/// Zod at all.
#[cfg(feature = "typescript")]
fn published(service: &ServiceDef) -> Vec<TokenStream> {
    let module = module_ident(service);
    let mut collected = Vec::new();
    for declared in &service.generated_messages {
        let message = &declared.ident;
        collected.push(quote! { #message::ts_definition() });
        #[cfg(feature = "zod")]
        collected.push(quote! { #message::zod_schema() });
    }
    let fields = format_ident!(
        "{}",
        fault_fields_typescript_name(&service.ident.to_string())
    );
    let kind = format_ident!("{}FaultKind", service.ident);
    collected.push(quote! { #module::#kind::ts_definition() });
    collected.push(quote! { #module::#fields::ts_definition() });
    collected.extend(
        fault::emit(service)
            .iter()
            .map(|rendered| quote! { #rendered.to_owned() }),
    );
    collected.extend(
        result::emit(service)
            .iter()
            .map(|rendered| quote! { #rendered.to_owned() }),
    );
    collected
}

/// The Dart twin of [`published`], with no `fault::emit` counterpart: Dart names no sealed fault
/// type of its own, so the result pair's `Fault` member and every fault helper reach for
/// `{Service}FaultFields` directly.
#[cfg(feature = "dart")]
fn dart_published(service: &ServiceDef) -> Vec<TokenStream> {
    use crate::features::dart::dart_module_ident;

    let module = module_ident(service);
    let mut collected = Vec::new();
    for declared in &service.generated_messages {
        let message_dart = dart_module_ident(&declared.ident.to_string(), declared.ident.span());
        collected.push(quote! { #message_dart::dart_definition() });
    }
    let fields = fault_fields_typescript_name(&service.ident.to_string());
    let fields_dart = dart_module_ident(&fields, service.ident.span());
    let kind = format!("{}FaultKind", service.ident);
    let kind_dart = dart_module_ident(&kind, service.ident.span());
    collected.push(quote! { #module::#kind_dart::dart_definition() });
    collected.push(quote! { #module::#fields_dart::dart_definition() });
    collected.extend(
        dart_result::emit(service)
            .iter()
            .map(|rendered| quote! { #rendered.to_owned() }),
    );
    collected
}

fn registry_rustdoc(service: &str) -> Vec<String> {
    let mut written = vec![format!(
        " What `{service}` publishes, in one place per artifact."
    )];
    written.extend(ts_registry_rustdoc(service));
    written
}

/// What the registry's own rustdoc says about `ts_definition()`, the client and the dispatcher.
/// Folded on both `typescript` and `zod` rather than composed from two calls, so a build missing
/// either has nothing left over to warn about being unused.
#[cfg(all(feature = "typescript", feature = "zod"))]
fn ts_registry_rustdoc(service: &str) -> Vec<String> {
    vec![
        String::new(),
        format!(
            " A bundle names `{service}Schema::ts_definition()` once and receives the service's own \
             types together with every message the macro declared for it, so no generated message \
             needs a registration line of its own."
        ),
    ]
}

#[cfg(all(feature = "typescript", not(feature = "zod")))]
fn ts_registry_rustdoc(service: &str) -> Vec<String> {
    vec![
        String::new(),
        format!(
            " A bundle names `{service}Schema::ts_definition()` once and receives the service's own \
             types together with every message the macro declared for it, so no generated message \
             needs a registration line of its own."
        ),
        String::new(),
        format!(
            " This build publishes no `{service}Schema::ts_client()`, no \
             `{service}Schema::ts_http_client()`, no `{service}Schema::ts_http_service()`, no \
             `{service}Schema::ts_service()`, no `{service}Schema::ts_ws_client()`, no \
             `{service}Schema::ts_ws_service()`, and no `{service}Schema::ts_ws_server()`. The \
             first six parse a message against the schema `#[model_schema()]` writes for it, \
             and the seventh wraps the one that does; only a build with tixschema's `zod` \
             feature writes one — so rather than a client, a transport and a dispatcher that \
             check nothing, this build publishes the service's types and leaves the seven seam \
             artifacts out. Add `features = [\"zod\"]` to the tixschema dependency to get them."
        ),
    ]
}

#[cfg(not(feature = "typescript"))]
fn ts_registry_rustdoc(service: &str) -> Vec<String> {
    vec![
        String::new(),
        format!(
            " This build publishes no TypeScript for `{service}`: no `ts_definition()` and none of \
             the client or dispatcher artifacts. Add `features = [\"typescript\"]` to the \
             tixschema dependency to get them, or enable a language feature of its own (`dart`, \
             `swift`, `kotlin`) if a mobile or server target is what this build wants instead."
        ),
    ]
}

#[cfg(test)]
mod tests;
