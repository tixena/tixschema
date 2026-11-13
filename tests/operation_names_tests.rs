//! `Operation` and `OPERATIONS` in the module of every service, whatever its `transports` holds.

#![cfg(feature = "serde")]

#[cfg(test)]
#[path = "operation_names_tests/tests.rs"]
mod tests;

// A service's transports reach what it declared through `$crate`, which is this binary's root.
#[cfg(test)]
use tests::{
    ContentService, CounselService, EveryService, SocketService, TemplateService,
    content_service_schema, counsel_service_schema, every_service_schema, socket_service_schema,
    template_service_schema,
};
