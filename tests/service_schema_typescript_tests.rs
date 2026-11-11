//! The TypeScript a `#[service_schema]` service publishes, and the bundle one registration line
//! per artifact produces.

#[cfg(test)]
#[macro_use]
#[path = "service_schema_typescript_tests/tests.rs"]
mod tests;

#[cfg(all(test, feature = "serde", feature = "typescript"))]
#[path = "service_schema_typescript_tests/amqp_transport.rs"]
mod amqp_transport;

// Both halves a transport contributes reach what the service declared through `$crate`, which is
// this binary's root. The gate is the declaration's own.
#[cfg(all(test, feature = "serde"))]
use tests::{ProbeService, probe_service_schema};

/// Five MCP services of one operation each, whose TypeScript the compiler checks one at a time.
#[cfg(test)]
#[cfg(all(feature = "serde", feature = "typescript", feature = "zod"))]
#[path = "service_schema_emitted_client_tests/mcp_note_services.rs"]
mod mcp_note_services;

#[cfg(test)]
#[cfg(all(feature = "serde", feature = "typescript", feature = "zod"))]
use mcp_note_services::{
    NoteBlocksService, NoteCountService, NoteGreetingService, NotePreviewService,
    NoteReviewService, note_blocks_service_schema, note_count_service_schema,
    note_greeting_service_schema, note_preview_service_schema, note_review_service_schema,
};
