//! `<Service>Schema::mcp_tools()`, `wrapped_{operation}()` and `MCP_NAMES`: the tools of services
//! that ask for `mcp`, each held to the `Tool` definition of the official 2026-07-28 schema, and
//! `needs_wrapper()` on every kind of model type, held to what serde writes, and `mcp_form_value()`
//! and `mcp_form_flat()` beside it.

#[cfg(test)]
#[path = "mcp_declaration_tests/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "mcp_declaration_tests/needs_wrapper.rs"]
mod needs_wrapper;

#[cfg(test)]
#[path = "mcp_declaration_tests/mcp_form.rs"]
mod mcp_form;

// What a service generates reaches the trait and its module through `$crate`, this binary's root.
#[cfg(all(test, feature = "serde"))]
use tests::{
    AnnotatedService, AnnotatedServiceSchema, DocumentService, DocumentServiceSchema, RouteService,
    TwoTransportService, TwoTransportServiceSchema, annotated_service_schema,
    document_service_schema, route_service_schema, two_transport_service_schema,
};
