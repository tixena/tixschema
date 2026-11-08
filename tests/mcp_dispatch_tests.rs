//! `{service}_mcp_dispatcher!()` and `{service}_mcp_client!()`, driven against a real expansion:
//! every sample of the wire, an endpoint shared by two services, the HTTP rules, and the client
//! read back through the dispatcher. Every message an assertion reads passes the official
//! 2026-07-28 schema.

#![cfg(feature = "serde")]

extern crate alloc;

#[cfg(test)]
#[macro_use]
#[path = "mcp_dispatch_tests/tests.rs"]
mod tests;

#[cfg(test)]
#[macro_use]
#[path = "mcp_dispatch_tests/approval.rs"]
mod approval;

#[cfg(test)]
#[path = "mcp_dispatch_tests/approval_mcp.rs"]
mod approval_mcp;

#[cfg(test)]
#[path = "mcp_dispatch_tests/approval_client.rs"]
mod approval_client;

#[cfg(test)]
#[macro_use]
#[path = "mcp_dispatch_tests/counsel.rs"]
mod counsel;

#[cfg(test)]
#[path = "mcp_dispatch_tests/counsel_mcp.rs"]
mod counsel_mcp;

#[cfg(test)]
#[path = "mcp_dispatch_tests/counsel_client.rs"]
mod counsel_client;

#[cfg(test)]
#[path = "mcp_dispatch_tests/prompt_tests.rs"]
mod prompt_tests;

#[cfg(test)]
#[path = "mcp_dispatch_tests/document_mcp.rs"]
mod document_mcp;

#[cfg(test)]
#[path = "mcp_dispatch_tests/skill_tests.rs"]
mod skill_tests;

#[cfg(test)]
#[path = "mcp_dispatch_tests/billing_mcp.rs"]
mod billing_mcp;

#[cfg(test)]
#[path = "mcp_dispatch_tests/export_mcp.rs"]
mod export_mcp;

#[cfg(test)]
#[path = "mcp_dispatch_tests/document_client.rs"]
mod document_client;

#[cfg(test)]
#[path = "mcp_dispatch_tests/support.rs"]
mod support;

#[cfg(test)]
#[path = "mcp_fault_cases/cases.rs"]
mod cases;

#[cfg(test)]
#[path = "mcp_dispatch_tests/serve_tests.rs"]
mod serve_tests;

#[cfg(test)]
#[path = "mcp_dispatch_tests/http_tests.rs"]
mod http_tests;

#[cfg(test)]
#[path = "mcp_dispatch_tests/auth_tests.rs"]
mod auth_tests;

#[cfg(test)]
#[path = "mcp_dispatch_tests/resource_tests.rs"]
mod resource_tests;

#[cfg(test)]
#[path = "mcp_dispatch_tests/writer_tests.rs"]
mod writer_tests;

#[cfg(test)]
#[macro_use]
#[path = "mcp_dispatch_tests/listens_through.rs"]
mod listens_through;

#[cfg(test)]
#[path = "mcp_dispatch_tests/listen_tests.rs"]
mod listen_tests;

#[cfg(test)]
#[path = "mcp_dispatch_tests/client_tests.rs"]
mod client_tests;

#[cfg(test)]
#[path = "mcp_dispatch_tests/elicitation_tests.rs"]
mod elicitation_tests;

// What a service generates reaches the trait and its module through `$crate`, this binary's root.
#[cfg(test)]
use approval::{ApprovalService, ApprovalServiceSchema, approval_service_schema};
#[cfg(test)]
use counsel::{CounselService, CounselServiceSchema, counsel_service_schema};
#[cfg(test)]
use tests::{
    BillingService, BillingServiceSchema, DocumentService, DocumentServiceSchema, ExportService,
    ExportServiceSchema, billing_service_schema, document_service_schema, export_service_schema,
};
