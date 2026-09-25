//! The emitted clients put through a real runtime, with a real message object.
//!
//! Every other assertion about an emitted client here reads strings: which expression was written,
//! never what it computes. `String(sending)` is well-formed TypeScript that renders every object
//! as the constant `[object Object]`, so the only witness is a runtime.
//!
//! **No toolchain is required.** A runtime is looked up on `PATH`, or at whatever the group's own
//! variable names, and a build that finds none stands down rather than failing. `just
//! test-emitted` is the entry point that refuses to stand down.
//!
//! Gated on `serde` (which `#[service_schema]` requires), and on `zod` and `typescript`, since
//! `ts_http_client` is published only where the Zod surface a client validates against is.

#![cfg(all(feature = "serde", feature = "zod", feature = "typescript"))]

extern crate alloc;

#[cfg(test)]
#[macro_use]
#[path = "service_schema_emitted_client_tests/tests.rs"]
mod tests;

/// The `http_rest` dispatcher for `ThumbnailClientService`, the Rust twin the `bytes` body kind
/// is measured against — in a module of its own, the placement every dispatcher macro needs.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/thumbnail_http_rest_transport.rs"]
mod thumbnail_http_rest_transport;

/// The `http_rest` dispatcher for `ContentClientService`, the Rust twin the `stream` body kind
/// is measured against.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/content_http_rest_transport.rs"]
mod content_http_rest_transport;

/// The `http_rest` dispatcher for `UploadDocumentClientService`, the Rust twin the `multipart`
/// body kind is measured against.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/upload_document_http_rest_transport.rs"]
mod upload_document_http_rest_transport;

/// The `http_rest` dispatcher for `EchoClientService`, the Rust twin the `header_in` echo group
/// is measured against.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/echo_http_rest_transport.rs"]
mod echo_http_rest_transport;

/// The `http_rest` dispatcher for `PulseClientService`, the Rust twin the bodyless-empty-message
/// group is measured against.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/pulse_http_rest_transport.rs"]
mod pulse_http_rest_transport;

/// The `ws_rpc` dispatcher and client for `StampClientService`, the Rust twins the headers groups
/// are measured against.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/stamp_ws_rpc_transport.rs"]
mod stamp_ws_rpc_transport;

#[cfg(test)]
#[path = "service_schema_emitted_client_tests/stamp_ws_rpc_client.rs"]
mod stamp_ws_rpc_client;

#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_node.rs"]
mod run_node;

#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_node_ws_headers.rs"]
mod run_node_ws_headers;

#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_node_ws_server.rs"]
mod run_node_ws_server;

/// The emitted `http_rest` server: the seven design requests, the reader forms, the query-read
/// message, and the three body kinds compared against the Rust twins above.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_node_http_service.rs"]
mod run_node_http_service;

#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_dart.rs"]
mod run_dart;

#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_dart_ws.rs"]
mod run_dart_ws;

#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift.rs"]
mod run_swift;

#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_ws_headers.rs"]
mod run_swift_ws_headers;

#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_kotlin.rs"]
mod run_kotlin;

#[cfg(test)]
#[path = "service_schema_emitted_client_tests/runtime.rs"]
mod runtime;

// The transport's expansion reaches each service through `$crate`, which is this binary's root:
// every trait and its schema module are named here for it to resolve.
#[cfg(test)]
use tests::{
    ArchiveClientService, ContentClientService, ConversationClientService, EchoClientService,
    GateClientService, LabelClientService, PulseClientService, SealClientService,
    SearchClientService, StampClientService, ThumbnailClientService, UploadDocumentClientService,
    VaultClientService, archive_client_service_schema, content_client_service_schema,
    conversation_client_service_schema, echo_client_service_schema, gate_client_service_schema,
    label_client_service_schema, pulse_client_service_schema, seal_client_service_schema,
    search_client_service_schema, stamp_client_service_schema, thumbnail_client_service_schema,
    upload_document_client_service_schema, vault_client_service_schema,
};
#[cfg(all(test, feature = "dart"))]
use tests::{ShelfClientService, shelf_client_service_schema};
