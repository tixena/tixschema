//! `{service}_mcp_dispatcher!()` and `{service}_mcp_client!()`, driven against a real expansion:
//! every sample of the wire, an endpoint shared by two services, the HTTP rules, and the client
//! read back through the dispatcher. Every message an assertion reads passes the official
//! 2026-07-28 schema.

#![cfg(feature = "serde")]

#[cfg(test)]
#[macro_use]
#[path = "mcp_dispatch_tests/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "mcp_dispatch_tests/document_mcp.rs"]
mod document_mcp;

#[cfg(test)]
#[path = "mcp_dispatch_tests/billing_mcp.rs"]
mod billing_mcp;

#[cfg(test)]
#[path = "mcp_dispatch_tests/document_client.rs"]
mod document_client;

#[cfg(test)]
#[path = "mcp_dispatch_tests/support.rs"]
mod support;

#[cfg(test)]
#[path = "mcp_dispatch_tests/serve_tests.rs"]
mod serve_tests;

#[cfg(test)]
#[path = "mcp_dispatch_tests/http_tests.rs"]
mod http_tests;

#[cfg(test)]
#[path = "mcp_dispatch_tests/resource_tests.rs"]
mod resource_tests;

#[cfg(test)]
#[path = "mcp_dispatch_tests/client_tests.rs"]
mod client_tests;

// What a service generates reaches the trait and its module through `$crate`, this binary's root.
#[cfg(test)]
use tests::{
    BillingService, BillingServiceSchema, DocumentService, DocumentServiceSchema,
    billing_service_schema, document_service_schema,
};
